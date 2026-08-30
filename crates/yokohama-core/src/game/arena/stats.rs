//! Tabulation — turning arena games into a strength claim with an honest
//! interval (Phase 3.2, spec `docs/plans/2026-07-25-phase3.2-arena-spec.md` §4's
//! tabulation block, governed by §0-R3).
//!
//! THE UNIT IS NOT THE GAME. Every interval here is computed over the DESIGN
//! UNIT — the pair at 2P, the rotation block at 3–4P — because the games inside
//! a unit share a world seed and (in a Keep-HOMOGENEOUS cell) a bit-identical
//! starting state, so they are not independent observations. Since Phase 3.3's
//! mixed-Keep cells (spec §5.5; the report's `setup-identical=no`), a unit's
//! games may resolve the same deal by two different Keep rules — still
//! seed-paired at `new_game` and swap-balanced, no longer setup-identical; the
//! unit bootstrap is valid either way (ledger row 15: pairing is bookkeeping,
//! the swap is what removes the seat effect — `game/arena.rs`'s module doc
//! carries the full scoping). Treating them as independent is not a rounding
//! error: E2 measured 4P within-block correlation ρ = +0.051, which drags a
//! nominal 95% interval down to ~93.2% true coverage. That is why
//! Wilson-on-pooled-games is PRINTED as a cross-check and is NEVER the primary
//! interval at 3–4P.
//!
//! WHAT θ̂ MEANS, AND WHAT ITS NULL IS. `theta_hat` is contestant X's share of
//! the games in the cell, averaged over units. The cell seats ONE X against
//! `players − 1` copies of Y (the single-champion shape), so **the null value is
//! 1/players, not 0.5** — 0.5 at 2P, 0.333… at 3P, 0.25 at 4P. The reference is
//! printed in every report's cross-checks rather than left for a reader to
//! assume, and [`pairs_needed_for_mme`] is a 2P-PAIR formula whose (θ − 0.5)
//! denominator says so in its own name.
//!
//! **3P CARRIES E2's CAVEAT.** 3P cells run and tabulate exactly like 4P ones,
//! but 3P SEAT SYMMETRY IS UNTESTED (spec §4) — the rotation removes a seat
//! effect nobody has measured at that count, so a 3P figure travels with that
//! caveat until someone does. 2P and 4P symmetry WERE measured (unswapped
//! arrangement gap up to 3.6pp, which is why the swap is mandatory rather than
//! an option).
//!
//! ZERO WIDTH IS A REFUSAL, NEVER A NUMBER. The two rungs that exist today are
//! a blowout — retreat-averse beat uniform 4000/4000 games at 2P and 400/400
//! across 100 rotation blocks at 4P — so every unit reads the same value and the
//! bootstrap and t intervals both collapse to ±0.000. A printed ±0.000 is an
//! instrument claiming infinite precision from zero information, so a constant
//! unit vector produces [`IntervalOutcome::RefusedZeroWidth`] carrying a
//! one-sided bound instead (spec §0-R3(iv); the uniform-vs-retreat-averse
//! bracket is the standing test vector).
//!
//! FIXED N, AND NO PEEKING MACHINERY. `CellSpec.units` is chosen BEFORE the cell
//! runs, from a pre-specified minimum meaningful edge ([`pairs_needed_for_mme`])
//! — never from a pilot point estimate, which measurably cost 20 points of power
//! at a 0.28pp pilot error. There is deliberately no interim-analysis,
//! early-stopping, or run-until-significant surface in this module: eight naive
//! looks measured 19.0% type-I error. Adding one is a spec change that must
//! spend alpha explicitly, not an API convenience.
//!
//! REPRODUCIBILITY IS BY CONSTRUCTION. [`run_cell`] is single-threaded and every
//! stream it uses — per-unit world seeds, per-seat agent streams, the bootstrap
//! resample — descends from the one master, which the report records. On
//! [`SeedScheduleKind::Diagnostic`] the whole report is therefore byte-identical
//! across runs (§0-R3(v)); on [`SeedScheduleKind::OsEntropy`] the DRAWN master is
//! recorded, and re-running it as a diagnostic master is the reproduction path
//! (spec §3). Gate cells use `Diagnostic` — an OS-entropy gate would flake at
//! the nominal rate by construction.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use super::{
    agent_stream_seed, mix, run_arena_game_capped, run_arena_game_diagnostic_capped, unit_for,
    ArenaAgent, ArenaError, ArenaGameStats, ControllerProvenance, DecisionReport,
    DiagnosticOutcome, PolicyAgent, Slot,
};
use crate::game::agent::HONEST_MAX_TURNS;
use crate::game::eval::WeightTable;
use crate::game::greedy::{self, ExplanationTally, GreedyAgent, DEFAULT_RETREAT_SAMPLES};
use crate::game::lookahead::{
    self, LookaheadAgent, SearchScheme, SearchTally, DEFAULT_ATOMIC_DEPTH, DEFAULT_BEAM,
    DEFAULT_WORLDS,
};
use crate::game::mcts::{
    self, LeafRule, McKnobs, McTally, MctsAgent, DEFAULT_EXPLORATION_PPM, DEFAULT_SQUASH_SCALE,
};
use crate::game::metrics::{nearest_rank_index, wilson_95, TriggerTally, Z_95};
use crate::game::policy::RolloutPolicy;
use crate::game::rules::Rules;
use crate::game::seeds::{SeedSchedule, WorldSeed};
use crate::types::{EndGameTrigger, OrderCardId, PlayerId, PlayerView};

/// Domain salt for the bootstrap resample stream.
pub(crate) const SALT_BOOTSTRAP: u64 = 0xA3E1_B007_0000_0004;

/// The contestant under test. Every other slot in the unit is Y — the
/// single-champion shape [`unit_for`]'s rotations are built for, and the reason
/// the null value is `1/players` rather than 0.5.
///
/// `pub(crate)` since the V4 step-11 diagnosis wave: `arena::diagnostic` seats
/// one world EXACTLY as a cell seats it, and re-typing `Slot(0)` there would be
/// a second spelling of the rule deciding which contestant is X — free to drift
/// from this one, and a drift no test would name.
pub(crate) const X_SLOT: Slot = Slot(0);

/// Bootstrap replicates for the primary interval (spec §4: B = 10,000).
pub const BOOTSTRAP_REPLICATES: u32 = 10_000;

/// The two-sided-α=0.05, power-80% sample-size constant
/// `(z_0.975 + z_0.80)² = (1.959964 + 0.841621)² = 7.848878`, quoted to the
/// precision spec §4's sizing figures were computed at.
const MME_CONSTANT: f64 = 7.848878;

/// Where a cell's master seed comes from — spec §3's rule made a type.
///
/// The NAME of the diagnostic arm is the waiver, exactly as
/// [`SeedSchedule::diagnostic`]'s is: a committed fixed master is correct for a
/// regression cell and wrong for a published evaluation run, and nothing but the
/// name distinguishes the two at the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SeedScheduleKind {
    /// A committed master: reproduction, fixtures, and every asserting gate cell.
    Diagnostic(u64),
    /// A fresh master per run, drawn from OS entropy and RECORDED in the report
    /// — the rule for bot-facing evaluation runs whose figures get quoted.
    OsEntropy,
}

/// WHO can sit in a cell's seat — the Phase-3.3 widening of the 3.2 arena, which
/// keyed a cell on a [`RolloutPolicy`] and so could only ever run rungs against
/// rungs (spec §5.1).
///
/// **The Policy arm is behaviour-preserving BY CONSTRUCTION, and that claim is
/// under test.** [`Contestant::Policy`] builds exactly the [`PolicyAgent`] the
/// 3.2 factory built, on exactly the [`agent_stream_seed`] keying, so every 3.2
/// figure recorded in map §11 A2 still describes this code —
/// `tests/contestant_golden.rs` holds two PRE-WIDENING `CellReport`s (a 2P pair
/// and a 4P rotation block) against that claim and is the acceptance test for
/// this type's existence.
///
/// **EVERY VARIANT'S KNOBS ARE THE AGENT'S OWN TYPES**, not a widened `u32` set:
/// `b` is [`DEFAULT_BEAM`]'s `u16`, `k`/`m`/`d` are the `u8`s their constructors
/// take. A knob set that cannot express something the agent can (or vice versa)
/// is a silent divergence between what a cell says it ran and what it ran.
///
/// The evaluator contestants carry NO weight knob: they run the frozen
/// [`WeightTable::V2`] the shipped agents run (`V1` until the 2026-08-10 v2
/// freeze), and a weight change is a version bump (spec §0.5 / L12), not a cell
/// parameter — which is exactly what that freeze did, moving all four
/// controller version consts in the commit that moved the table.
/// `weights=<identity>` still rides the params map so a corpus reader can tell
/// two tables apart without consulting a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contestant {
    /// A 3.2 rollout rung, played by [`PolicyAgent`] exactly as before.
    Policy(RolloutPolicy),
    /// Greedy-v1 (spec §3): one-ply argmax of the evaluator. `m` is the retreat
    /// sample count.
    GreedyV1 { m: u8 },
    /// Lookahead-v1 (spec §4.2), the S1 turn-boundary rung: beam `b`, `k`
    /// sampled worlds, `m` retreat samples.
    ///
    /// **`cap` is Phase 3.4's shared scored-successor budget (spec §3.4),
    /// `None` by default and `None` in [`LOOKAHEAD_V1_DEFAULT`](Self::LOOKAHEAD_V1_DEFAULT).**
    /// An absent cap is byte-identical to the pre-3.4 rung all the way out to
    /// the echo: the descriptor emits NO `cap=` token for it, which is what
    /// keeps every recorded 3.3 figure and every committed golden describing
    /// this code.
    LookaheadV1 { b: u16, k: u8, m: u8, cap: Option<u32> },
    /// The S2 DIAGNOSTIC arm (spec §4.1): atomic depth `d` in moves, with
    /// quiescence. Named `-diag` in provenance because it is preserved as a rung
    /// only if T7 flips.
    AtomicDiag { d: u8, k: u8, m: u8 },
    /// **MCTS-v1** (spec §3), Phase 3.4's candidate architecture: a UCT tree over
    /// one sampled world, sized by the SAME scored-successor budget the capped
    /// S1 arm is sized by — which is what makes a node-matched cell possible.
    ///
    /// `cap` is that budget (`None` = uncapped, and then the agent runs
    /// [`DEFAULT_SIMULATIONS`](crate::game::mcts::DEFAULT_SIMULATIONS); a capped
    /// seat runs [`SIMULATION_CEILING`](crate::game::mcts::SIMULATION_CEILING)
    /// and lets the cap decide — [`mcts::simulations_for_cap`] is the one site
    /// that says so). `c_ppm` is UCB1's exploration constant in ppm, `scale` the
    /// value squash's scale in `eval::score` units, `m` the retreat sample
    /// count, `leaf` the leaf rule — spec §4's screening grid is `leaf × c ×
    /// scale`, so all three are cell parameters. There is deliberately no weight
    /// knob (spec §0.5).
    MctsV1 { cap: Option<u32>, c_ppm: u32, scale: i64, m: u8, leaf: LeafRule },
    /// **A NAMED EVALUATOR IDENTITY** (V4 §7 step 8, spec §13's step-8
    /// blast-radius addition): a [`LookaheadAgent`] seat at the shipped knobs
    /// whose WEIGHT TABLE is named rather than hardwired, so the §11 arms are
    /// runnable identities rather than names. See [`NamedIdentity`].
    NamedTable(NamedIdentity),
}

/// The named weight-table identities the arena can seat — spec §13's
/// explicit-table surface, as NAMES rather than raw weight dumps.
///
/// **Why a distinct contestant rather than a weight KNOB on `lookahead-v1`.**
/// This type's own enclosing doc states the house rule: "a weight change is a
/// version bump (spec §0.5 / L12), not a cell parameter". A `weights=` knob on
/// the shipped arm would break that rule outright; a distinct NAMED contestant
/// does not — it is a new identity, which is exactly the thing §13 sanctioned
/// adding. The `weights=<identity>` token still rides the params map, so the
/// echo carries both halves: the name a reader types and the digest of the
/// thirty-four numbers it resolved to.
///
/// **Knobless, deliberately** — `eval_screen`'s stated rule for the same job
/// ("`ScreenRung` has no knob fields: each variant runs its controller's
/// SHIPPED knobs … The screen's subject is the WEIGHTS; a knob that moved in
/// the same cell would confound the only thing it measures"). Both arms
/// therefore run ONE knob set, and a cell pitting them against each other
/// varies the table alone.
///
/// **The knob literals are TRANSCRIBED, not tracked** —
/// [`build_pinned_v2_seat`](crate::game::driven)'s discipline, adopted for its
/// reason: an arm is an IDENTITY, and a later move of the shipped default must
/// not silently re-point figures already recorded against these names.
/// `the_named_identity_arms_transcribe_todays_shipped_knobs` states the
/// present equality as a transcription fact and is DELETED (never edited) in
/// the commit that legitimately moves the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedIdentity {
    /// [`WeightTable::V2`] — THE shipped table, named. Post-padding this is
    /// spec §10's `V2-padded`, so this arm's play is the shipped seat's play
    /// and `the_named_identity_arms_walk_their_own_tables` holds it there.
    V2Padded,
    /// [`WeightTable::V2_REFIT_PADDED`] — the §8 B-cell refit (the sixteen
    /// incumbents refit with no candidate columns admitted). NOT a shipped
    /// table; that const's doc carries the provenance and the never-move rule.
    V2RefitPadded,
    /// [`WeightTable::V4_STEP9_DRAFT1`] — the V4 step-9 CANDIDATE, draft 1's
    /// table: the first arm here whose candidate slots are nonzero, which is
    /// the whole of what the sighted evaluator is. NOT a shipped table; that
    /// const's doc carries the provenance, the never-move rule, and the note
    /// that this WIRE NAME is proposed rather than spec-registered.
    V4Step9Draft1,
}

/// The shipped lookahead knobs every named arm runs, TRANSCRIBED (see
/// [`NamedIdentity`]): beam 4, one sampled world, eight retreat samples, no cap.
const NAMED_IDENTITY_BEAM: u16 = 4;
const NAMED_IDENTITY_WORLDS: u8 = 1;
const NAMED_IDENTITY_RETREAT_SAMPLES: u8 = 8;

impl NamedIdentity {
    /// Every named identity, for a grammar or a test that must cover them all.
    pub const ALL: [Self; 3] =
        [Self::V2Padded, Self::V2RefitPadded, Self::V4Step9Draft1];

    /// The WIRE name — what a runner accepts and an echo carries. Read from
    /// here at every site, so the grammar and provenance cannot drift into two
    /// spellings (`greedy::GREEDY_CONTROLLER`'s rule for the shipped arms).
    pub fn name(self) -> &'static str {
        match self {
            Self::V2Padded => "v2-padded",
            Self::V2RefitPadded => "v2-refit-padded",
            Self::V4Step9Draft1 => "v4-step9-draft1",
        }
    }

    /// The thirty-four numbers this name resolves to — the ONE site that
    /// decides an arm's table, so `build` and `provenance` cannot disagree
    /// about what a seat played.
    pub fn table(self) -> WeightTable {
        match self {
            Self::V2Padded => WeightTable::V2,
            Self::V2RefitPadded => WeightTable::V2_REFIT_PADDED,
            Self::V4Step9Draft1 => WeightTable::V4_STEP9_DRAFT1,
        }
    }

    /// The name's inverse — `None` for anything else, [`RolloutPolicy::from_str_name`]'s
    /// shape because an arm name either is or is not one of two literals and
    /// the caller composes its own refusal around that.
    pub fn from_str_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|arm| arm.name() == name)
    }
}

/// How a contestant resolves its dealt starting-order pair — the property spec
/// §5.5 scopes the pairing doctrine on.
///
/// Two contestants sharing a `KeepKind` make a cell **Keep-homogeneous**, and
/// only then are a unit's two seatings setup-IDENTICAL. A mixed cell is still
/// seed-paired and swap-balanced; it just resolves the same deal two ways. See
/// [`Contestant::keep_kind`] and the arena module doc's pairing paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepKind {
    /// [`PolicyAgent`]'s `(seat, deal)`-keyed uniform draw — every rung's, since
    /// 3.2 ships no starting-order policy to differentiate them with.
    SeatKeyedUniform,
    /// `greedy::score_starting_order`'s argmax over the two dealt cards — every
    /// evaluator contestant's, since [`LookaheadAgent`] shares Greedy's table.
    EvaluatorScored,
}

/// A cloneable read-out on ONE built contestant's explanation and budget
/// counters.
///
/// The three evaluator tallies are deliberately different types (a beam has
/// continuation lengths, a tree has simulations, a one-ply argmax has neither),
/// so this is the narrow thing the driver actually needs from all of them: spec
/// §0.3(b)'s count and spec §6's budget meters. The Policy rungs explain nothing
/// and produce no handle at all — `Option::None` is the honest shape, not a zero
/// counter that would suggest the rung had an explanation path and simply never
/// took it.
///
/// **Every accessor below returns `Option`, and `None` always means "this
/// contestant has no such quantity", never "zero".** The distinction is what
/// lets a gate assert an identity on a contestant that has the counter without
/// the assertion passing vacuously on one that does not.
#[derive(Clone)]
pub enum ExplainHandle {
    Greedy(ExplanationTally),
    Search(SearchTally),
    /// MCTS-v1's ONE handle: [`McTally`] carries the explanation count beside
    /// the budget meters (that type's doc argues for the merge — an MCTS
    /// explanation is a projection of the search that just ran, so there is no
    /// second counter set to split off).
    Mcts(McTally),
}

impl ExplainHandle {
    /// Decisions this contestant explained. Equal to its applied-move count on a
    /// completed game — no evaluator agent can return a move without explaining
    /// it, and a failed `choose` fails the run.
    pub fn explained(&self) -> u64 {
        match self {
            Self::Greedy(t) => t.explained(),
            Self::Search(t) => t.explained(),
            Self::Mcts(t) => t.explained(),
        }
    }

    /// Continuations [`MAX_TURN_PLIES`](crate::game::lookahead::MAX_TURN_PLIES)
    /// stopped — `None` for a contestant with NO SEARCH, on
    /// [`explained`](Self::explained)'s own `Option` grounds one level down: a
    /// one-ply argmax has no continuation to truncate, and a `0` would read as
    /// "searched and never hit the guard".
    ///
    /// **The MCTS arm answers with its hand-off completions' hits** — the SAME
    /// guard and the same constant (`McTally::completion_cap_hits`), and the
    /// same meaning: a line stopped there is valued mid-turn.
    pub fn ply_cap_hits(&self) -> Option<u64> {
        match self {
            Self::Greedy(_) => None,
            Self::Search(t) => Some(t.ply_cap_hits()),
            Self::Mcts(t) => Some(t.completion_cap_hits()),
        }
    }

    /// Free extensions
    /// [`MAX_FREE_EXTENSION`](crate::game::lookahead::MAX_FREE_EXTENSION)
    /// stopped, same `None` rule. `Some(0)` for an S1 contestant is structural
    /// rather than evidence — the turn-boundary scheme never enters the
    /// quiescence arm. `None` for MCTS-v1, which has no quiescence arm to enter
    /// at all.
    pub fn free_cap_hits(&self) -> Option<u64> {
        match self {
            Self::Greedy(_) => None,
            Self::Search(t) => Some(t.free_cap_hits()),
            Self::Mcts(_) => None,
        }
    }

    /// Successors applied, observed and scored — **spec §0.3's budget
    /// currency**, and `Some` on every explaining contestant: all three pay it
    /// and all three count it.
    pub fn candidates_scored(&self) -> Option<u64> {
        match self {
            Self::Greedy(t) => Some(t.candidates_scored()),
            Self::Search(t) => Some(t.candidates_scored()),
            Self::Mcts(t) => Some(t.candidates_scored()),
        }
    }

    /// The MANDATORY ROOT PASS's share of
    /// [`candidates_scored`](Self::candidates_scored) — spec §0.3's realized
    /// root share.
    ///
    /// `None` for the one-ply argmax, which has no root/deeper SPLIT to take a
    /// share of: its whole pass is one ply, so a share would be 1 by
    /// construction and would carry no information a reader could act on.
    pub fn root_scored(&self) -> Option<u64> {
        match self {
            Self::Greedy(_) => None,
            Self::Search(t) => Some(t.root_scored()),
            Self::Mcts(t) => Some(t.root_scored()),
        }
    }

    /// Decisions in which the cap DENIED work (spec §3.4). `None` for a
    /// contestant with no budget at all — and `Some(0)` for a capable-but-
    /// UNCAPPED one, which is the reading the experiment's validity condition
    /// needs (a cap that never binds is not a matched budget).
    pub fn budget_binds(&self) -> Option<u64> {
        match self {
            Self::Greedy(_) => None,
            Self::Search(t) => Some(t.budget_binds()),
            Self::Mcts(t) => Some(t.budget_binds()),
        }
    }

    /// Decisions whose MANDATORY ROOT PASS alone spent the cap, so no deeper
    /// work could start — a sub-case of [`budget_binds`](Self::budget_binds),
    /// and the counter spec §0.1's S2 completion-failure surface routes on. Same
    /// `None` rule.
    pub fn budget_starved_decisions(&self) -> Option<u64> {
        match self {
            Self::Greedy(_) => None,
            Self::Search(t) => Some(t.budget_starved_decisions()),
            Self::Mcts(t) => Some(t.budget_starved_decisions()),
        }
    }
}

/// One SIDE's deterministic budget meters, summed over a cell's seats and games
/// — spec §6's `x_*`/`y_*` additions in the shape the driver folds them.
///
/// Every field is `Option` on [`ExplainHandle`]'s rule, and the fold is
/// `get_or_insert(0) +=`: a side made of contestants that HAVE a counter
/// reports the sum, a side made of ones that do not reports `None`.
#[derive(Debug, Clone, Copy, Default)]
struct BudgetMeters {
    candidates_scored: Option<u64>,
    root_scored: Option<u64>,
    binds: Option<u64>,
    starved: Option<u64>,
}

impl BudgetMeters {
    fn fold(&mut self, handle: &ExplainHandle) {
        if let Some(v) = handle.candidates_scored() {
            *self.candidates_scored.get_or_insert(0) += v;
        }
        if let Some(v) = handle.root_scored() {
            *self.root_scored.get_or_insert(0) += v;
        }
        if let Some(v) = handle.budget_binds() {
            *self.binds.get_or_insert(0) += v;
        }
        if let Some(v) = handle.budget_starved_decisions() {
            *self.starved.get_or_insert(0) += v;
        }
    }
}

/// `name@vN` / `name@vN{k=v,…}` — THE one echo composition, shared by
/// [`Contestant::echo`] and the driven generating-ecology's pinned seat (V4
/// step 8), so a pinned construction's recorded echo and a contestant's cannot
/// drift into two spellings of the same identity.
pub(crate) fn compose_echo(name: &str, version: u32, params: &BTreeMap<String, i64>) -> String {
    if params.is_empty() {
        return format!("{name}@v{version}");
    }
    let knobs: Vec<String> = params.iter().map(|(k, v)| format!("{k}={v}")).collect();
    format!("{name}@v{version}{{{}}}", knobs.join(","))
}

impl Contestant {
    /// Greedy-v1 at its shipped knob.
    pub const GREEDY_V1_DEFAULT: Self = Self::GreedyV1 { m: DEFAULT_RETREAT_SAMPLES };
    /// Lookahead-v1 (S1) at its shipped knobs — UNCAPPED, which is what makes it
    /// byte-identical to the pre-3.4 rung the goldens pin.
    pub const LOOKAHEAD_V1_DEFAULT: Self =
        Self::LookaheadV1 { b: DEFAULT_BEAM, k: DEFAULT_WORLDS, m: DEFAULT_RETREAT_SAMPLES, cap: None };
    /// The S2 diagnostic arm at its shipped knobs.
    pub const ATOMIC_DIAG_DEFAULT: Self =
        Self::AtomicDiag { d: DEFAULT_ATOMIC_DEPTH, k: DEFAULT_WORLDS, m: DEFAULT_RETREAT_SAMPLES };
    /// MCTS-v1 at its shipped knobs, UNCAPPED — so it runs
    /// [`DEFAULT_SIMULATIONS`](crate::game::mcts::DEFAULT_SIMULATIONS) and
    /// terminates. Every registered cell in spec §5 names a cap instead; this
    /// const is the knobless spelling the grammar and the fixtures start from.
    pub const MCTS_V1_DEFAULT: Self = Self::MctsV1 {
        cap: None,
        c_ppm: DEFAULT_EXPLORATION_PPM,
        scale: DEFAULT_SQUASH_SCALE,
        m: DEFAULT_RETREAT_SAMPLES,
        leaf: LeafRule::HandOff,
    };

    /// Parse a contestant SPEC — `<name>` or `<name>:<key>=<value>,…` — into the
    /// contestant it names. **THE single-sourced grammar** (Phase 3.5c W1-B):
    /// `examples/arena.rs` parses its `<x>`/`<y>` through here, the probe's
    /// contestant arm delegates here too since W1-C (narrowing after the parse),
    /// and the CLI's bot seats JOIN at their W5 retype — so a spec typed at any
    /// consumer means the same thing at all of them. The body and its reasoning
    /// live in [`super::grammar`].
    ///
    /// **UNKNOWN IS AN ERROR at every level** — an unknown contestant name, an
    /// unknown knob, an unknown knob VALUE, or any knob at all on a contestant
    /// that has none. Nothing is silently defaulted or silently ignored,
    /// because a swallowed knob would put a published figure under knobs the
    /// cell did not run. Every error NAMES what it saw.
    ///
    /// The signature deliberately differs from [`RolloutPolicy::from_str_name`]'s
    /// `Option`: a rung name either is or is not one of three literals, while
    /// this grammar has many distinct ways to be wrong and the caller cannot
    /// reconstruct which one fired — so the refusal carries its own text, which
    /// is user-facing in the runner.
    ///
    /// ```
    /// use yokohama_core::Contestant;
    /// // A bare name is that controller's SHIPPED default…
    /// assert_eq!(Contestant::from_str_name("lookahead-v1"), Ok(Contestant::LOOKAHEAD_V1_DEFAULT));
    /// // …and for S1 that means UNCAPPED: an absent cap is not any set cap.
    /// assert!(matches!(
    ///     Contestant::from_str_name("lookahead-v1"),
    ///     Ok(Contestant::LookaheadV1 { cap: None, .. })
    /// ));
    /// assert!(Contestant::from_str_name("lookahead-v1:cap=0").is_err());
    /// assert!(Contestant::from_str_name("greedy-v1:zzz=1").is_err());
    /// ```
    pub fn from_str_name(spec: &str) -> Result<Self, String> {
        super::grammar::parse_contestant(spec)
    }

    /// **The INVERSE of [`descriptor`](Self::descriptor)** over a record's two
    /// stored halves — the controller `name` and its typed `params` map
    /// (`SeatProvenance`'s own fields) — Phase 3.5c W5, design §1.5.
    ///
    /// WHY this exists rather than a bare-name reparse: a name carries no
    /// knobs, so reconstructing a recorded seat from `controller` alone
    /// silently resets every one of them — a recorded `lookahead-v1` at `b=8`
    /// would resume at `b=4` and go on playing under knobs the record does not
    /// name. The CLI's resume path (W5 task 3) is the first consumer; the
    /// deferred replay viewer and the agreement instrument are the next.
    ///
    /// **UNKNOWN IS AN ERROR at every level**, [`from_str_name`](Self::from_str_name)'s
    /// discipline adopted from birth: an unknown controller name, an unknown
    /// params key, or a value outside the agent's own type is `Err`, and every
    /// refusal NAMES what it saw.
    ///
    /// Three rules worth stating in full, because each is a silent defect if
    /// mis-read:
    ///
    /// - **An absent `cap` key is `cap: None`** — absence IS the uncapped
    ///   identity on the provenance preserve-list, and the descriptor emits no
    ///   token for it. A `cap` of 0 is refused with [`super::grammar`]'s own
    ///   sentence.
    /// - **The identity keys are accepted and NOT consumed** — see
    ///   [`IDENTITY_PARAM_KEYS`]; the caller compares them.
    /// - **A missing CONSTRUCTION key is an error, never a default.** Every key
    ///   read below is one the matching arm's descriptor ALWAYS emits, so its
    ///   absence means the map did not come from a descriptor — and defaulting
    ///   it would invent a knob value the record never carried. (`cap` is the
    ///   one optional key, and its absence is a stated value rather than a
    ///   default.)
    ///
    /// **Names are WIRE names.** The four controller consts and
    /// [`RolloutPolicy::from_str_name`]'s rung names are accepted; the runner
    /// grammar's short `atomic-diag-v1` alias is NOT, because no descriptor
    /// ever emits it and this function inverts descriptors.
    ///
    /// **Core stays neutral about eligibility**: `mcts-v1` and the S2
    /// diagnostic parse here without complaint. Refusing a diagnostic
    /// contestant a particular consumer will not drive is that consumer's
    /// narrowing to make (the probe's W1-C precedent, and the CLI's at W5).
    pub fn from_parts(name: &str, params: &BTreeMap<String, i64>) -> Result<Self, String> {
        if name == greedy::GREEDY_CONTROLLER {
            reject_unknown_params(name, params, &["m"], &[])?;
            return Ok(Self::GreedyV1 { m: param_u8(name, params, "m")? });
        }

        if name == lookahead::LOOKAHEAD_CONTROLLER {
            reject_unknown_params(name, params, &["b", "k", "m", "cap"], &[])?;
            let b = param_u16(name, params, "b")?;
            if b == 0 {
                return Err(format!("b=0 is not a beam ({name} descriptors floor it at 1)"));
            }
            return Ok(Self::LookaheadV1 {
                b,
                k: param_worlds(name, params)?,
                m: param_u8(name, params, "m")?,
                cap: param_cap(name, params)?,
            });
        }

        if name == lookahead::ATOMIC_DIAG_CONTROLLER {
            reject_unknown_params(name, params, &["d", "k", "m"], &[])?;
            let d = param_u8(name, params, "d")?;
            if d == 0 {
                return Err(format!("d=0 searches nothing ({name} descriptors floor it at 1)"));
            }
            return Ok(Self::AtomicDiag {
                d,
                k: param_worlds(name, params)?,
                m: param_u8(name, params, "m")?,
            });
        }

        if name == mcts::MCTS_CONTROLLER {
            reject_unknown_params(
                name,
                params,
                &["c", "scale", "m", "leaf", "cap"],
                MCTS_DERIVED_PARAM_KEYS,
            )?;
            let scale = required_param(name, params, "scale")?;
            if scale <= 0 {
                return Err(format!("scale={scale} is not a squash scale"));
            }
            return Ok(Self::MctsV1 {
                cap: param_cap(name, params)?,
                c_ppm: param_u32(name, params, "c")?,
                scale,
                m: param_u8(name, params, "m")?,
                leaf: param_leaf(name, params)?,
            });
        }

        // A named identity's descriptor emits the LOOKAHEAD knob keys under the
        // ARM's name, so the inverse reads them back and CHECKS them rather
        // than defaulting: the knobs are transcribed literals, not knobs a
        // record may vary, so a record naming this arm at other knobs did not
        // come from this arm and must not be silently resumed as if it had.
        if let Some(arm) = NamedIdentity::from_str_name(name) {
            reject_unknown_params(name, params, &["b", "k", "m"], &[])?;
            let (b, k, m) = (
                param_u16(name, params, "b")?,
                param_worlds(name, params)?,
                param_u8(name, params, "m")?,
            );
            if (b, k, m)
                != (NAMED_IDENTITY_BEAM, NAMED_IDENTITY_WORLDS, NAMED_IDENTITY_RETREAT_SAMPLES)
            {
                return Err(format!(
                    "{name} is a knobless named identity at b={NAMED_IDENTITY_BEAM}, \
                     k={NAMED_IDENTITY_WORLDS}, m={NAMED_IDENTITY_RETREAT_SAMPLES}, but the \
                     record carries b={b}, k={k}, m={m}",
                ));
            }
            return Ok(Self::NamedTable(arm));
        }

        let Some(policy) = RolloutPolicy::from_str_name(name) else {
            return Err(format!(
                "unknown controller {name:?} (rungs: uniform | retreat-averse | family-weighted; \
                 evaluator: {} | {} | {}; architecture: {}; named identities: {} | {})",
                greedy::GREEDY_CONTROLLER,
                lookahead::LOOKAHEAD_CONTROLLER,
                lookahead::ATOMIC_DIAG_CONTROLLER,
                mcts::MCTS_CONTROLLER,
                NamedIdentity::V2Padded.name(),
                NamedIdentity::V2RefitPadded.name(),
            ));
        };
        match policy {
            RolloutPolicy::FamilyWeighted { .. } => {
                reject_unknown_params(name, params, &["retreat_ppm", "recovery_min_of"], &[])?;
                let retreat_ppm = param_u32(name, params, "retreat_ppm")?;
                // The policy's own stated bound: `Rng::gen_ratio` asserts it in
                // every profile, so a record above it would panic at play rather
                // than at load.
                if retreat_ppm > 1_000_000 {
                    return Err(format!("retreat_ppm={retreat_ppm} exceeds 1_000_000 ppm"));
                }
                Ok(Self::Policy(RolloutPolicy::FamilyWeighted {
                    retreat_ppm,
                    recovery_min_of: param_u8(name, params, "recovery_min_of")?,
                }))
            }
            knobless => {
                reject_unknown_params(name, params, &[], &[])?;
                Ok(Self::Policy(knobless))
            }
        }
    }

    /// What this contestant CALLS itself, for provenance and the report echo —
    /// routed to the controller's own descriptor and never composed here.
    ///
    /// Spec §5.2 under §0.5's per-controller version rule: `Policy` answers from
    /// [`RolloutPolicy::descriptor`], the evaluator arms from
    /// [`greedy::controller_provenance`] / [`lookahead::controller_provenance`],
    /// each of which reads its own name and version const. No controller name is
    /// hand-typed at any call site in the arena, this one included.
    pub fn provenance(&self, agent_seed: Option<u64>) -> ControllerProvenance {
        match *self {
            Self::Policy(policy) => ControllerProvenance::from_policy(policy, agent_seed),
            Self::GreedyV1 { m } => {
                greedy::controller_provenance(&WeightTable::V2, m, agent_seed)
            }
            // The CAPPED form for S1, on the same function: an absent cap emits
            // no token there, so the uncapped rung's descriptor is what it
            // always was and the 3.3 goldens keep meaning what they meant.
            Self::LookaheadV1 { b, k, m, cap } => lookahead::controller_provenance_capped(
                SearchScheme::TurnBoundary { beam: b },
                k,
                m,
                cap,
                &WeightTable::V2,
                agent_seed,
            ),
            Self::AtomicDiag { d, k, m } => lookahead::controller_provenance(
                SearchScheme::AtomicDiag { depth: d },
                k,
                m,
                &WeightTable::V2,
                agent_seed,
            ),
            Self::MctsV1 { cap, c_ppm, scale, m, leaf } => mcts::controller_provenance(
                McKnobs::sized_by_cap(cap, c_ppm, scale, m, leaf),
                &WeightTable::V2,
                agent_seed,
            ),
            // The controller's OWN descriptor over the arm's OWN table, with
            // one field replaced: the NAME. Everything a reader needs to price
            // the seat — the knobs, `eval_impl`, and the `weights=` digest of
            // the thirty-four numbers — is the lookahead controller's own
            // answer, so this arm cannot echo a search it did not run; and the
            // name is the arm's, because "lookahead-v1" would leave two
            // different tables echoing under one identity, which is the exact
            // confusion §13's surface exists to remove. The name is read from
            // `NamedIdentity::name` rather than typed here, so there is still
            // one spelling of it (`greedy::GREEDY_CONTROLLER`'s rule).
            Self::NamedTable(arm) => {
                let mut p = lookahead::controller_provenance(
                    SearchScheme::TurnBoundary { beam: NAMED_IDENTITY_BEAM },
                    NAMED_IDENTITY_WORLDS,
                    NAMED_IDENTITY_RETREAT_SAMPLES,
                    &arm.table(),
                    agent_seed,
                );
                p.name = arm.name().to_string();
                p
            }
        }
    }

    /// `(name, version, params)` — [`provenance`](Self::provenance) without the
    /// stream seed, in [`RolloutPolicy::descriptor`]'s shape so the echo
    /// formatter is one function for every contestant.
    pub fn descriptor(&self) -> (String, u32, BTreeMap<String, i64>) {
        let p = self.provenance(None);
        (p.name, p.version, p.params)
    }

    /// `name@vN` or `name@vN{k=v,…}` — ONE display form for every contestant,
    /// fed by [`descriptor`](Self::descriptor) so a rung's echo is byte-for-byte
    /// what it was before the 3.3 widening (`tests/contestant_golden.rs` pins
    /// the strings).
    ///
    /// Public since Phase 3.5c W5 (design §4.2) because the CLI needs the same
    /// sentence in three places — the wizard's sim summary, the `why` header,
    /// and the resume warnings — and a second composer in the CLI is exactly
    /// the drift the W1 grammar promotion exists to prevent. `spec_echo` reads
    /// it here, so a report header and a CLI header cannot disagree about what
    /// a seat is.
    ///
    /// **This is a DISPLAY form, deliberately NOT the parse grammar**
    /// (`name:k=v,…`): the knob separator differs, the version rides the string,
    /// and the params map carries identity keys that are not knobs at all — so
    /// `from_str_name(echo(..))` does not round-trip and is not meant to.
    /// [`from_parts`](Self::from_parts) is the inverse that does, over the
    /// record's two halves rather than over this string.
    pub fn echo(&self) -> String {
        let (name, version, params) = self.descriptor();
        compose_echo(&name, version, &params)
    }

    /// Which starting-order rule this contestant plays (spec §5.5).
    pub fn keep_kind(&self) -> KeepKind {
        match self {
            Self::Policy(_) => KeepKind::SeatKeyedUniform,
            // MCTS-v1 joins the evaluator group because it shares
            // `greedy::score_starting_order` VERBATIM (spec §3.6 / map row 20),
            // which is what makes an mcts-vs-lookahead cell setup-identical —
            // and every §5 cell depends on that.
            // A named arm joins the evaluator group for the SAME reason, and
            // the reason is worth stating precisely because it is the one an
            // identity leak would hide in: `greedy::score_starting_order` takes
            // NO weight table (`eval_screen`'s doc says so in as many words —
            // "the KEEP function … takes NO weight table — so a screen is
            // Keep-HOMOGENEOUS and setup-IDENTICAL by construction, at any pair
            // of tables"). So an arm's table cannot reach its Keep at all: the
            // two named arms resolve every deal identically, to each other and
            // to the shipped seat, and a §11 cell is setup-identical
            // STRUCTURALLY. `the_named_identity_arms_keep_by_a_weight_blind_rule`
            // pins that, so a future weight-sensitive Keep has to answer for
            // itself here instead of quietly making two arms' setups differ.
            Self::GreedyV1 { .. }
            | Self::LookaheadV1 { .. }
            | Self::AtomicDiag { .. }
            | Self::MctsV1 { .. }
            | Self::NamedTable(_) => KeepKind::EvaluatorScored,
        }
    }

    /// Build the agent this contestant names, on the PLAY stream `agent_seed`,
    /// plus its explanation read-out if it has one.
    ///
    /// **`agent_seed` must come from [`agent_stream_seed`].** The evaluator
    /// contestants inherit the CRN rule unchanged — their play streams are keyed
    /// by `(world seed, seat, slot)` exactly as a rung's, because the trap the
    /// slot component excludes (both games of a pair becoming one game) is a
    /// property of the KEYING and not of what sits at the seat. Their SETUP
    /// decisions take no stream at all (`score_starting_order` is a function of
    /// the deal), which is why they are a different [`KeepKind`] and not a
    /// different keying.
    ///
    /// The handle is cloned out HERE, before the agent is boxed: a game's agents
    /// are built by the driver's factory and never handed back
    /// ([`HonestPathTally`](super::HonestPathTally)'s precedent).
    pub fn build(
        &self,
        rules: &Rules,
        agent_seed: u64,
    ) -> (Box<dyn ArenaAgent>, Option<ExplainHandle>) {
        match *self {
            Self::Policy(policy) => (Box::new(PolicyAgent::new(policy, agent_seed)), None),
            Self::GreedyV1 { m } => {
                // THE SHIPPED TABLE, and the reason all three arms below name it
                // explicitly rather than calling `::new`: a contestant's weights
                // are part of what its NAME means (L12 — the grammar carries no
                // weight knob), so the one place a cell's table is decided is
                // here. The v2 freeze moved all three from `V1` to `V2` in one
                // commit and bumped every controller const with them.
                let agent = GreedyAgent::with_weights(rules, WeightTable::V2, m, agent_seed);
                let handle = ExplainHandle::Greedy(agent.tally());
                (Box::new(agent), Some(handle))
            }
            Self::LookaheadV1 { b, k, m, cap } => Self::build_search(
                rules,
                SearchScheme::TurnBoundary { beam: b },
                k,
                m,
                cap,
                WeightTable::V2,
                agent_seed,
            ),
            Self::AtomicDiag { d, k, m } => Self::build_search(
                rules,
                SearchScheme::AtomicDiag { depth: d },
                k,
                m,
                None,
                WeightTable::V2,
                agent_seed,
            ),
            // The SAME construction path as the shipped rung, differing in the
            // one thing the arm names: its table. Routing it through
            // `build_search` rather than a private constructor is what makes
            // "a named arm is a lookahead seat" true by construction rather
            // than by comment — a divergence would have to be authored here.
            Self::NamedTable(arm) => Self::build_search(
                rules,
                SearchScheme::TurnBoundary { beam: NAMED_IDENTITY_BEAM },
                NAMED_IDENTITY_WORLDS,
                NAMED_IDENTITY_RETREAT_SAMPLES,
                None,
                arm.table(),
                agent_seed,
            ),
            Self::MctsV1 { cap, c_ppm, scale, m, leaf } => {
                // ONE knob value builds the agent and (in `provenance` above)
                // the descriptor, so a cell cannot echo a search it did not run
                // — including the simulation count, which is DERIVED from the
                // cap rather than named separately (`mcts::simulations_for_cap`).
                let knobs = McKnobs::sized_by_cap(cap, c_ppm, scale, m, leaf);
                let agent = MctsAgent::with_knobs(rules, agent_seed, knobs)
                    .with_weights(WeightTable::V2);
                let handle = ExplainHandle::Mcts(agent.tally());
                (Box::new(agent), Some(handle))
            }
        }
    }

    /// `weights` is a PARAMETER rather than the hardwired shipped table since
    /// V4 step 8: the two shipped arms pass [`WeightTable::V2`] explicitly (so
    /// their construction is byte-identical to what it always was, and the
    /// caller above still reads as "the one place a cell's table is decided"),
    /// and the named arms pass their own. One search constructor, one place a
    /// table enters it.
    fn build_search(
        rules: &Rules,
        scheme: SearchScheme,
        worlds: u8,
        retreat_samples: u8,
        cap: Option<u32>,
        weights: WeightTable,
        agent_seed: u64,
    ) -> (Box<dyn ArenaAgent>, Option<ExplainHandle>) {
        let agent =
            LookaheadAgent::with_knobs(rules, scheme, worlds, retreat_samples, weights, agent_seed)
                .with_cap(cap);
        let handle = ExplainHandle::Search(agent.tally());
        (Box::new(agent), Some(handle))
    }
}

// ---------------------------------------------------------------------------
// `from_parts`: reading a descriptor back
// ---------------------------------------------------------------------------

/// Descriptor keys that IDENTIFY the build rather than CONSTRUCT the
/// contestant — accepted by [`Contestant::from_parts`] on every arm, consumed
/// by none.
///
/// **WHY they are skipped rather than checked.** `eval_impl` says which
/// evaluator implementation produced the record, `weights` which thirty
/// numbers, and greedy's `placement_cap` which enumeration ceiling. None is an
/// input to any constructor here, and all three may legitimately differ from
/// the running build's when an OLDER record is resumed — which is the whole
/// case this function exists to serve. Design §1.5 puts the CHECKING in the
/// caller: it compares every recorded param against a freshly-derived
/// [`Contestant::descriptor`] and warns loudly per differing field, so the game
/// stays playable and the drift stays visible. Consuming (or verifying) them
/// here would instead make a cross-build record unloadable, which is the
/// failure the preserve-list forbids.
const IDENTITY_PARAM_KEYS: &[&str] = &["eval_impl", "weights", "placement_cap"];

/// Descriptor keys the MCTS arm DERIVES rather than carries.
///
/// [`Contestant::MctsV1`] names five knobs; `McKnobs::sized_by_cap` turns them
/// into a knob set that additionally reports `k` (worlds, floored at 1) and —
/// on an UNCAPPED seat only — `sims`, the simulation count derived from the cap
/// by `mcts::simulations_for_cap`. Neither has a field to be read back into, so
/// both are accepted and ignored on this arm exactly as the identity keys are;
/// the `cap` they are derived FROM is what round-trips. They are a separate
/// list from [`IDENTITY_PARAM_KEYS`] because `k` is a real construction knob on
/// the two lookahead arms, and a shared list would silently stop consuming it
/// there.
const MCTS_DERIVED_PARAM_KEYS: &[&str] = &["k", "sims"];

/// Every key must be one this arm constructs from, derives, or identifies with
/// — anything else is refused rather than ignored, because a swallowed param
/// resumes a seat under knobs the record does not name.
fn reject_unknown_params(
    name: &str,
    params: &BTreeMap<String, i64>,
    construction: &[&str],
    derived: &[&str],
) -> Result<(), String> {
    for key in params.keys() {
        let known = construction.contains(&key.as_str())
            || derived.contains(&key.as_str())
            || IDENTITY_PARAM_KEYS.contains(&key.as_str());
        if !known {
            let takes = if construction.is_empty() {
                "it takes no construction params".to_string()
            } else {
                construction.join(" | ")
            };
            return Err(format!("unknown param {key:?} for {name} ({takes})"));
        }
    }
    Ok(())
}

/// A CONSTRUCTION key's raw value. Absence is an error, never a default: every
/// caller below reads a key its arm's descriptor always emits, so a missing one
/// means the map did not come from a descriptor.
fn required_param(name: &str, params: &BTreeMap<String, i64>, key: &str) -> Result<i64, String> {
    params.get(key).copied().ok_or_else(|| {
        format!("{name} params are missing {key:?}, which every {name} descriptor emits")
    })
}

fn param_u8(name: &str, params: &BTreeMap<String, i64>, key: &str) -> Result<u8, String> {
    let raw = required_param(name, params, key)?;
    u8::try_from(raw).map_err(|e| format!("{key}={raw} is out of range for {name}: {e}"))
}

fn param_u16(name: &str, params: &BTreeMap<String, i64>, key: &str) -> Result<u16, String> {
    let raw = required_param(name, params, key)?;
    u16::try_from(raw).map_err(|e| format!("{key}={raw} is out of range for {name}: {e}"))
}

fn param_u32(name: &str, params: &BTreeMap<String, i64>, key: &str) -> Result<u32, String> {
    let raw = required_param(name, params, key)?;
    u32::try_from(raw).map_err(|e| format!("{key}={raw} is out of range for {name}: {e}"))
}

/// `k`, the sampled-world count. Zero is REFUSED rather than floored: the
/// descriptor emits `worlds.max(1)`, so a recorded `k=0` never came from one,
/// and flooring it here would let `from_parts` and `descriptor` disagree about
/// the map they just exchanged.
fn param_worlds(name: &str, params: &BTreeMap<String, i64>) -> Result<u8, String> {
    let k = param_u8(name, params, "k")?;
    if k == 0 {
        return Err(format!("k=0 is not a world count ({name} descriptors floor it at 1)"));
    }
    Ok(k)
}

/// The shared scored-successor budget (spec §3.4): an ABSENT key is the
/// uncapped identity, and `cap=0` is refused with [`super::grammar`]'s own
/// sentence so the two surfaces cannot drift into two rules.
fn param_cap(name: &str, params: &BTreeMap<String, i64>) -> Result<Option<u32>, String> {
    let Some(&raw) = params.get("cap") else { return Ok(None) };
    let cap = u32::try_from(raw)
        .map_err(|e| format!("cap={raw} is out of range for {name}: {e}"))?;
    if cap == 0 {
        return Err(super::grammar::CAP_ZERO_REFUSAL.to_string());
    }
    Ok(Some(cap))
}

/// The leaf rule, read back off its WIRE CODE — the params map is `i64`-valued
/// and cannot carry a name. The inverse is searched over `LeafRule::code`
/// itself rather than re-tabulated, so the two directions cannot renumber
/// apart; an unrecognized code is an error on the grammar's own grounds (an
/// unknown knob VALUE is as dangerous as an unknown knob NAME).
fn param_leaf(name: &str, params: &BTreeMap<String, i64>) -> Result<LeafRule, String> {
    let code = required_param(name, params, "leaf")?;
    [LeafRule::HandOff, LeafRule::MidTurnDiag, LeafRule::ConstantDiag]
        .into_iter()
        .find(|rule| rule.code() == code)
        .ok_or_else(|| {
            let known: Vec<String> = [LeafRule::HandOff, LeafRule::MidTurnDiag, LeafRule::ConstantDiag]
                .into_iter()
                .map(|rule| format!("{} = {}", rule.code(), rule.as_str()))
                .collect();
            format!("unknown leaf code {code} for {name} ({})", known.join(" | "))
        })
}

/// What a cell does when one of its games reaches the turn guard — V4 step 11,
/// amendment A-S11-1 (freeze doc
/// `docs/plans/2026-08-30-phase3.5c-v4-identity-freeze.md` §9.6).
///
/// **[`Refuse`](Self::Refuse) is the behaviour every figure recorded in this
/// repo was measured under, and it is what an unflagged cell still does**: the
/// first capped game refuses the WHOLE cell with
/// [`ArenaError::CapReached`], because a skipped game would read as a different
/// N. Every existing entry point passes it, through the same call chain it
/// always used, so the choice is opt-in at the CALLER and no recorded figure
/// moves.
///
/// [`Count`](Self::Count) is the amendment's counting mode: a capped game
/// becomes a typed, COUNTED event, the design unit containing it is VOID, and
/// the cell refuses only when the count passes the arm's registered `ceiling`.
/// `ceiling: 0` is legal and refuses at the first capped game — through
/// [`ArenaError::CappedCeilingExceeded`], deliberately distinct from refuse
/// mode's `CapReached`, so the two refusals are never read as the same event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapPolicy {
    /// Today's rule: the first capped game refuses the cell.
    Refuse,
    /// Count capped games, void their units, and refuse only above `ceiling`.
    Count { ceiling: u32 },
}

/// One capped game of a counting cell, named so it reproduces.
///
/// `arrangement` is the index into [`unit_for`]'s seatings — the same number
/// `arena world --arrangement` takes — so `arena world <x> <y> <seed>
/// --arrangement <arr>` replays exactly this game. `turns` is the driver's own
/// number, one past the guard (10,001 at the shipped [`HONEST_MAX_TURNS`]).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CappedGame {
    pub seed: WorldSeed,
    pub arrangement: u8,
    pub turns: u32,
}

/// One cell of the arena: contestant X against contestant Y at a player count,
/// over a fixed number of design units.
///
/// `units` is the DESIGN unit count, not a game count: the cell plays
/// `units × players` games, because every unit is played under every rotation so
/// each contestant occupies each seat exactly once.
#[derive(Debug, Clone)]
pub struct CellSpec {
    /// Names the matchup for the report echo and the corpus path.
    pub name: String,
    pub players: u8,
    /// The contestant under test — one seat per game.
    pub x: Contestant,
    /// The field — `players − 1` seats per game.
    pub y: Contestant,
    /// Design units. Size it from [`pairs_needed_for_mme`] BEFORE running.
    pub units: u32,
    pub master: SeedScheduleKind,
}

/// The primary interval on θ, or the typed refusal that replaces it when the
/// unit outcomes carry no variation to resample.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum IntervalOutcome {
    /// Percentile bootstrap over the design units — the primary interval at
    /// every player count (spec §4).
    Bootstrap {
        lo: f64,
        hi: f64,
        /// Replicates drawn. Recorded because the endpoints are order
        /// statistics of exactly this many replicate means.
        b: u32,
        /// The resample stream's seed, recorded so the interval reproduces.
        resample_seed: u64,
    },
    /// Every unit read the same value, so there is nothing to resample.
    ///
    /// `one_sided_lower` is always a TRUE 95% one-sided lower bound on θ, and
    /// `method` says how informative it is — which is the honest shape when the
    /// data is degenerate in a direction the field cannot express. For an
    /// all-won vector it is the rule-of-three bound `1 − 3/n`; for an all-lost
    /// or interior-constant vector the only bound the data supports without a
    /// model is 0, and `method` names the informative statement instead (the
    /// report's cross-checks carry the complementary `3/n` figure).
    RefusedZeroWidth { one_sided_lower: f64, method: &'static str },
}

/// Every unit outcome read the same, and that value was a win.
const ALL_UNITS_WON: &str =
    "rule-of-three one-sided 95% lower bound (every unit won; no variation to resample)";
/// Every unit outcome read the same, and that value was a loss.
const ALL_UNITS_LOST: &str = "every unit lost; 0 is the only lower bound this data supports \
     — the informative statement is the complementary rule-of-three upper bound in cross_checks";
/// Constant, but at neither extreme (e.g. every 2P pair split 1–1).
const CONSTANT_INTERIOR: &str = "every unit read the same interior value; there is no unit-level \
     variation to resample and rule-of-three does not apply — 0 is the only lower bound this \
     data supports";
/// No units at all. Caller misuse (`run_cell` refuses a zero-unit spec before
/// reaching here); this exists so the release path names what happened instead
/// of borrowing a message about constant data that does not exist.
const NO_UNITS: &str = "no units were tabulated; nothing constrains theta";

/// One tabulated cell. Everything a strength claim is allowed to quote, plus
/// the provenance that makes it re-runnable.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CellReport {
    /// The spec as run, including the master and the unit shape — so a quoted
    /// figure carries its measurement policy rather than relying on prose.
    pub spec_echo: String,
    /// Games played: `units × players`.
    pub games: u32,
    /// X's win share within each unit, in play order. The interval's input.
    pub x_unit_shares: Vec<f64>,
    /// Mean of `x_unit_shares`. Its null value is `1/players` (see the module
    /// doc), which the cross-checks print.
    pub theta_hat: f64,
    pub interval: IntervalOutcome,
    /// Printed cross-checks — the unit-t interval, Wilson-on-pooled-games, the
    /// null reference, and the tie accounting. **Never primary**, and the Wilson
    /// row says so with the clustering figure attached.
    pub cross_checks: Vec<String>,
    /// Games won by seat 0 across the cell. Under a balanced design this should
    /// sit near `games/players`; a drift is the seat effect the rotation exists
    /// to remove.
    pub seat0_wins: u32,
    /// Games in which the top VP total was SHARED and the clockwise tie-break
    /// awarded the win to seat 0 — the +0.63pp seat-0 award, which is half the
    /// tie rate and scales with it one-for-one (spec §4). Recorded per cell
    /// because it will move under stronger bots.
    pub ties_to_seat0: u32,
    /// Mean over games of (X's VP total − the mean of the Y seats' totals).
    /// The screening/triage statistic (1.4–2.1× fewer units for equal power —
    /// a 2P PAIR-UNIT measurement, E2; nothing has measured it for the 3-4P
    /// block unit);
    /// the WIN RATE above is the gate statistic, per spec §4.
    pub vp_margin_mean: f64,
    /// Applied moves per SEAT over the whole cell, indexed by seat (spec §5.4).
    ///
    /// A cell's seats are filled by different contestants in different
    /// arrangements, so this is a balance read on the DESIGN and not a
    /// per-contestant figure — the per-contestant one is
    /// [`x_applied_moves`](Self::x_applied_moves), which follows X through the
    /// rotation. Under a balanced rotation these should sit close together; a
    /// drift is the seat effect the rotation exists to remove, showing up in
    /// move volume rather than in wins.
    pub applied_by_seat: Vec<u64>,
    /// Moves applied by the X seat, summed over the cell's games — X's seat
    /// changes with the arrangement, so this follows the CONTESTANT.
    ///
    /// Spec §0.3(b)'s right-hand side: every move the evaluator bot plays must
    /// be explained, and `x_explanations == x_applied_moves` is that claim made
    /// countable at cell level.
    pub x_applied_moves: u64,
    /// Decisions X EXPLAINED, summed over the cell's games — spec §0.3(b)'s
    /// left-hand side, read off the tally handles the factory cloned out.
    ///
    /// **`None` when X HAS NO EXPLANATION PATH** (a [`Contestant::Policy`] rung),
    /// which is a different fact from an explaining contestant that explained
    /// nothing — and a `0` would conflate the two, letting an evaluator that
    /// silently stopped explaining pass the same check a rung passes by design.
    /// The gate assertion is therefore the whole equality
    /// `x_explanations == Some(x_applied_moves)`: total, and non-vacuous without
    /// a second guard.
    pub x_explanations: Option<u64>,
    /// Continuations X's search had stopped by
    /// [`MAX_TURN_PLIES`](crate::game::lookahead::MAX_TURN_PLIES), summed over
    /// the cell's games. **Expected 0.**
    ///
    /// `None` when X HAS NO SEARCH (a rung or the one-ply argmax), on
    /// [`x_explanations`](Self::x_explanations)'s rule. Surfaced at the six-lens
    /// review: the counter was already being cloned out per X seat per game and
    /// then dropped, so the gate deciding 3.3's promotion checked nothing about
    /// the guard across its 1,472 games. A cap hit is not a wrong answer, but a
    /// capped line is valued MID-TURN, so a cell that starts hitting the guard
    /// is measuring something slightly different from the one that set the
    /// figures — which is a thing a report should say rather than a thing a
    /// reader should assume.
    pub x_ply_cap_hits: Option<u64>,
    /// Free extensions X's search had stopped by
    /// [`MAX_FREE_EXTENSION`](crate::game::lookahead::MAX_FREE_EXTENSION), same
    /// `None` rule and same expectation. `Some(0)` from an S1 contestant is
    /// structural (it never runs the quiescence arm); the figure has content
    /// only for the S2 diagnostic.
    pub x_free_cap_hits: Option<u64>,
    /// **Spec §6's deterministic budget meters, X side** — successors X applied,
    /// observed and scored over the whole cell (spec §0.3's currency).
    ///
    /// `None` when X has no such counter (a [`Contestant::Policy`] rung), on
    /// [`x_explanations`](Self::x_explanations)'s rule. Deterministic, hence on
    /// the report at all: the byte-identity pins hold over it. The WALL half of
    /// §0.3's two-axis table is not here and cannot be — see [`CellWall`].
    pub x_candidates_scored: Option<u64>,
    /// The MANDATORY ROOT PASS's share of
    /// [`x_candidates_scored`](Self::x_candidates_scored) — spec §0.3's realized
    /// root share, `None` for a contestant with no root/deeper split.
    pub x_root_scored: Option<u64>,
    /// Decisions in which X's cap DENIED work. `Some(0)` from a capable but
    /// UNCAPPED contestant is the experiment's validity read (a cap that never
    /// binds is not a matched budget); `None` means the contestant has no budget.
    pub x_budget_binds: Option<u64>,
    /// Decisions whose mandatory root pass alone spent X's cap — spec §0.1's S2
    /// surface routes on this counter, so a cell carries it rather than leaving
    /// an adjudicator to re-derive it.
    pub x_budget_starved_decisions: Option<u64>,
    /// **The Y-SIDE MIRRORS** (lens 1 M11 / lens 2 M2). §0.3 requires the
    /// two-axis table to report realized budget for BOTH sides, and every
    /// pre-3.4 per-contestant counter on this report is X-side — so a
    /// node-matched cell could not state its own matched-ness through the seam
    /// it ran on.
    ///
    /// Summed over the `players − 1` Y seats per game and over every game, so at
    /// 2P it is one opponent's total and at 3–4P it is the FIELD's. Divide by
    /// the Y applied-move count (`applied_by_seat` sum minus
    /// [`x_applied_moves`](Self::x_applied_moves)) for a per-decision figure.
    pub y_candidates_scored: Option<u64>,
    /// Y's realized root share, same summation and same `None` rule.
    pub y_root_scored: Option<u64>,
    /// Y's cap-bind count, same summation and same `None` rule.
    pub y_budget_binds: Option<u64>,
    /// Y's starved-decision count, same summation and same `None` rule.
    pub y_budget_starved_decisions: Option<u64>,
    /// End-trigger counts over the cell's games — one player count per tally,
    /// as [`TriggerTally`] requires.
    pub trigger: TriggerTally,
    /// The master this cell actually ran on: the committed one for
    /// [`SeedScheduleKind::Diagnostic`], the DRAWN one for
    /// [`SeedScheduleKind::OsEntropy`].
    pub master: u64,
    /// The registered void CEILING this cell ran under, or `None` in REFUSE
    /// mode — V4 step 11, amendment A-S11-1 (freeze doc §9.6).
    ///
    /// `None` is the shape of every figure recorded before the amendment and of
    /// every unflagged run since: it says "the first capped game would have
    /// refused this cell", which is why the four fields below then carry no
    /// information (`units_void` is 0 and `capped` is empty BY CONSTRUCTION,
    /// not by observation).
    pub capped_ceiling: Option<u32>,
    /// Design units the cell SCHEDULED — `spec.units`, always.
    pub units_scheduled: u32,
    /// Scheduled units containing at least one capped game. Such a unit is VOID
    /// and contributes NO outcome: every outcome statistic on this report reads
    /// over the READ units alone.
    pub units_void: u32,
    /// Units that contributed an outcome — `x_unit_shares.len()`, and the `n`
    /// of the interval.
    pub units_read: u32,
    /// Every capped game, in schedule order. Empty in refuse mode.
    pub capped: Vec<CappedGame>,
}

impl CellReport {
    /// The VOID LINE a counting cell prints, or `None` in refuse mode — V4 step
    /// 11, amendment A-S11-1 (freeze doc §9.6).
    ///
    /// `None` and not an empty string: the runner prints this line and
    /// [`capped_lines`](Self::capped_lines) ONLY when this is `Some`, which is
    /// what keeps an unflagged `cell` run's output byte-for-byte what it always
    /// was. The refuse-mode fields carry no information to print — `units_void`
    /// is 0 and `capped` is empty by CONSTRUCTION there, not by observation —
    /// so a line stating them would read as a measurement.
    ///
    /// It lives in core, beside the fields it reads, for
    /// [`super::diagnostic::render_cap_rate`]'s reason: the runner is
    /// print-only glue, and text composed there is text no library test covers.
    pub fn void_summary(&self) -> Option<String> {
        let ceiling = self.capped_ceiling?;
        Some(format!(
            "units scheduled={} · void={} · read={} · capped games={} (ceiling {ceiling})",
            self.units_scheduled,
            self.units_void,
            self.units_read,
            self.capped.len(),
        ))
    }

    /// ONE line per capped game, each carrying the command that replays it.
    ///
    /// **The `<x>`/`<y>` placeholders are deliberate and are
    /// [`super::diagnostic::render_cap_rate`]'s own spelling:** a `CellReport`
    /// carries no contestant fields (they live in
    /// [`spec_echo`](Self::spec_echo), printed directly above), so this line
    /// cannot honestly fill them in. The seed and the arrangement — the two
    /// things that identify WHICH game, and the two a reader cannot reconstruct
    /// — are concrete.
    ///
    /// Empty in refuse mode, where no capped game exists.
    pub fn capped_lines(&self) -> Vec<String> {
        self.capped
            .iter()
            .map(|game| {
                format!(
                    "capped: arena world <x> <y> {:#018x} --arrangement {} · turns={}",
                    game.seed.0, game.arrangement, game.turns,
                )
            })
            .collect()
    }
}

/// One side's in-cell WALL — nanoseconds spent inside `choose`, and the
/// decisions that spent them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SideWall {
    pub nanos: u64,
    pub decisions: u64,
}

impl SideWall {
    /// µs per decision, or `NaN` when the side took none (a side that never
    /// decided has no rate, and 0 would read as "instant").
    pub fn micros_per_decision(&self) -> f64 {
        if self.decisions == 0 {
            return f64::NAN;
        }
        self.nanos as f64 / 1_000.0 / self.decisions as f64
    }
}

/// **Per-side in-cell wall clock, and deliberately NOT a [`CellReport`] field**
/// (spec §0.3's deliverability repair / §6).
///
/// §0.3 requires the ADR's two-axis table to report realized µs/decision for
/// both sides, and the per-unit cost is NOT arm-symmetric — a node-matched cell
/// hands the tree arm a wall advantage BY CONSTRUCTION, which is why the wall
/// cell C3 exists. So the number has to be produced.
///
/// It cannot live on the report: `CellReport` is byte-identity-pinned across
/// runs on a diagnostic master (`same_master_reproduces_the_report_byte_identically`,
/// the goldens, T7's determinism cell), and a wall figure is load-sensitive by
/// construction. Putting it there would either break those pins or force them to
/// be weakened, which is the worse trade. It is therefore accumulated by the
/// driver, handed back BESIDE the report, and printed by the runner — the
/// T7-wall-line convention: recorded, never asserted, and always carrying its
/// conditions.
///
/// It measures time inside [`ArenaAgent::choose`] ONLY — not setup, not
/// `apply_move`, not the invariant checks — because µs/DECISION is what §0.3's
/// table compares, and mixing driver overhead into it would make two arms'
/// figures differ by how much game each happened to play.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CellWall {
    pub x: SideWall,
    pub y: SideWall,
}

/// The per-decision clock, shared by every agent on one SIDE of a cell.
///
/// `Arc<AtomicU64>` rather than a borrow because `Box<dyn ArenaAgent>` is
/// `'static`: the driver's factory hands the agent away and the cell must still
/// be able to read the counter afterwards — the clone-out rule every tally in
/// this module already follows.
#[derive(Clone, Default)]
struct WallClock {
    nanos: Arc<AtomicU64>,
    decisions: Arc<AtomicU64>,
}

impl WallClock {
    fn read(&self) -> SideWall {
        SideWall {
            nanos: self.nanos.load(Ordering::Relaxed),
            decisions: self.decisions.load(Ordering::Relaxed),
        }
    }
}

/// An agent wearing a stopwatch. PURE DELEGATION in every other respect —
/// provenance, the setup decision and the move itself all pass straight through,
/// so wrapping cannot move a cell's play (which the committed goldens then
/// verify rather than take on trust).
struct TimedAgent {
    inner: Box<dyn ArenaAgent>,
    clock: WallClock,
}

impl ArenaAgent for TimedAgent {
    fn choose_starting_order(&mut self, seat: PlayerId, deal: &[OrderCardId; 2]) -> u8 {
        self.inner.choose_starting_order(seat, deal)
    }

    fn choose(&mut self, view: &PlayerView, rules: &Rules) -> Result<DecisionReport, String> {
        let started = Instant::now();
        let out = self.inner.choose(view, rules);
        // Charged whether or not the decision succeeded: a failing `choose` is a
        // hard error that ends the run, and time it spent is still time this
        // side spent.
        self.clock.nanos.fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
        self.clock.decisions.fetch_add(1, Ordering::Relaxed);
        out
    }

    fn controller(&self) -> ControllerProvenance {
        self.inner.controller()
    }
}

/// What one scheduled game of a cell produced.
///
/// In [`CapPolicy::Refuse`] only [`Completed`](Self::Completed) is ever built —
/// the driver's cap is an `Err` there, exactly as it always was. The variant
/// split exists so the counting mode cannot hand a partial run to anything that
/// takes an [`ArenaGameStats`]: a capped game has none, and the corpus sink,
/// the trigger tally and the outcome buffer all take one.
enum PlayedGame {
    Completed(Box<ArenaGameStats>),
    /// The turn guard tripped. `moves_by_seat` is the PARTIAL run's per-seat
    /// split, which the volume counters fold exactly as a completed game's.
    Capped { turns: u32, moves_by_seat: Vec<u64> },
}

/// One design unit's OUTCOME contributions, held back until the unit is known
/// READ — V4 step 11, amendment A-S11-1 (freeze doc §9.6).
///
/// **Why a buffer and not a running total.** A unit is VOID as soon as ANY of
/// its games caps, and a cap can land in the unit's SECOND game — after the
/// first has already completed, been emitted and been tabulated. An outcome
/// folded the moment its game finished cannot be taken back, so the outcome
/// half of every game waits here and folds into the cell only once the whole
/// unit has been played and read.
///
/// **The VOLUME half is deliberately NOT here.** `applied_by_seat`,
/// `x_applied_moves`, the explanation tally, the two cap counters and the
/// budget meters fold over EVERY played game, capped ones included, so
/// `x_explanations == Some(x_applied_moves)` stays a claim about every move X
/// played rather than about the moves that happened to land in a read unit.
///
/// `vp_margins` is a per-GAME vector rather than a running `f64` subtotal so
/// the cell's float addition sequence is EXACTLY the one it always was — units
/// in schedule order, games in arrangement order. A per-unit subtotal would
/// re-associate the sum, and the goldens compare that figure byte for byte.
#[derive(Default)]
struct UnitOutcome {
    games: u32,
    x_wins: u32,
    seat0_wins: u32,
    top_vp_ties: u32,
    ties_to_seat0: u32,
    vp_margins: Vec<f64>,
    triggers: Vec<Option<EndGameTrigger>>,
}

/// Run one cell with per-move invariant checking ON — the default every driver
/// in the repo contracts for, and what every asserting gate cell wants.
///
/// Single-threaded by construction: see the module doc's reproducibility note.
pub fn run_cell(spec: &CellSpec, rules: &Rules) -> Result<CellReport, ArenaError> {
    run_cell_with(spec, rules, true)
}

/// [`run_cell`] with per-move `check_invariants` as a parameter.
///
/// The off-switch spec §4 documents for BULK generation, whose records are
/// verified by strict replay instead. The split mirrors
/// [`super::run_arena_game`] / [`super::run_arena_game_capped`]: the public
/// default carries the contract, the parameterized form exists for callers that
/// have a stated reason to differ.
pub fn run_cell_with(
    spec: &CellSpec,
    rules: &Rules,
    invariants: bool,
) -> Result<CellReport, ArenaError> {
    run_cell_emitting(spec, rules, invariants, &mut |_stats, _arr| Ok(()))
}

/// [`run_cell_with`] with a per-game SINK — the emission seam (spec §5).
///
/// `sink(stats, arr)` sees every COMPLETED game as it finishes, where `arr` is
/// the arrangement index within its design unit — the two things
/// `corpus::corpus_path` needs that a `CellReport` cannot carry. Completed only
/// is structural rather than checked: on THIS entry point a cap hit or a driver
/// error is an `Err` from the driver and short-circuits the loop, so a failed
/// game has no [`ArenaGameStats`] to hand anyone (spec §4's failure taxonomy —
/// see that type's doc).
///
/// **The rule survives A-S11-1's counting mode, by a different route** (freeze
/// doc §9.6): there a capped game does not short-circuit anything, but it
/// produces no `ArenaGameStats` either, so the sink still sees every completed
/// game and nothing else — including the completed games of a VOID unit. A
/// record is a record; voiding is a unit-level statistic.
///
/// Streaming rather than returning the records: a 200-unit 4P cell is 800 games
/// of event log, which is a corpus, not a report.
///
/// This is also where the RUN half of provenance is filled in. The driver stamps
/// each seat's controller but knows nothing above the game; the cell knows the
/// master its world seeds descend from and its own name, so it says so on every
/// record — which is what makes a corpus file self-describing rather than
/// meaningful only next to the invocation that produced it.
pub fn run_cell_emitting(
    spec: &CellSpec,
    rules: &Rules,
    invariants: bool,
    sink: &mut dyn FnMut(&ArenaGameStats, u8) -> Result<(), ArenaError>,
) -> Result<CellReport, ArenaError> {
    run_cell_emitting_timed(spec, rules, invariants, sink).map(|(report, _wall)| report)
}

/// [`run_cell_emitting`] handing back the per-side [`CellWall`] beside the
/// report — spec §0.3's both-sides wall axis, at the ONE layer that can produce
/// it.
///
/// Separate from the four entry points above rather than folded into them
/// because the wall is a DIFFERENT KIND of number: every other output of this
/// module is deterministic in the master and is pinned as such, and a load-
/// sensitive figure travelling in the same value would eventually be quoted as
/// if it were. The runner prints it; nothing asserts it.
pub fn run_cell_emitting_timed(
    spec: &CellSpec,
    rules: &Rules,
    invariants: bool,
    sink: &mut dyn FnMut(&ArenaGameStats, u8) -> Result<(), ArenaError>,
) -> Result<(CellReport, CellWall), ArenaError> {
    run_cell_loop(spec, rules, invariants, CapPolicy::Refuse, HONEST_MAX_TURNS, sink)
}

/// [`run_cell_emitting_timed`] in A-S11-1's COUNTING mode: a capped game is a
/// typed, counted event, its design unit is VOID, and the cell refuses only
/// when the count passes `ceiling` (freeze doc §9.6).
///
/// A SEPARATE entry point rather than a parameter on the four above, for the
/// reason the amendment gives: the counting mode is opt-in per cell, and every
/// caller that does not ask for it must keep reaching the refuse path through
/// the call chain it always used. `ceiling` travels as an argument rather than
/// on [`CellSpec`] for the same reason — the spec is the DESIGN, and 47
/// constructor sites across the gate binaries and the committed goldens spell
/// it.
pub fn run_cell_counting_caps_timed(
    spec: &CellSpec,
    rules: &Rules,
    invariants: bool,
    ceiling: u32,
    sink: &mut dyn FnMut(&ArenaGameStats, u8) -> Result<(), ArenaError>,
) -> Result<(CellReport, CellWall), ArenaError> {
    run_cell_loop(spec, rules, invariants, CapPolicy::Count { ceiling }, HONEST_MAX_TURNS, sink)
}

/// THE cell loop — the one every entry point above reaches.
///
/// `max_turns` is a parameter for [`super::run_arena_game_capped`]'s reason
/// exactly: a test must reach a capping cell in milliseconds instead of playing
/// 10,000 no-op turns. Every public entry point passes [`HONEST_MAX_TURNS`], the
/// shared non-termination guard, which is therefore untouched by the widening.
///
/// `pub(crate)` and not `pub`: the guard is a TEST affordance, and a public
/// surface that let a caller shorten the guard would let a published figure be
/// measured against a different definition of "stuck".
pub(crate) fn run_cell_loop(
    spec: &CellSpec,
    rules: &Rules,
    invariants: bool,
    policy: CapPolicy,
    max_turns: u32,
    sink: &mut dyn FnMut(&ArenaGameStats, u8) -> Result<(), ArenaError>,
) -> Result<(CellReport, CellWall), ArenaError> {
    // Both guards run BEFORE any game, because both failure modes are silent
    // otherwise: `unit_for(0, ..)` yields an empty seating list, so a zero or
    // one player count would tabulate zero games into a report full of NaNs
    // instead of erroring. The player-count check is `run_arena_game`'s own, so
    // there is one authority for what the engine plays.
    super::require_supported_player_count(spec.players)?;
    if spec.units == 0 {
        return Err(ArenaError::Driver(format!(
            "cell {:?} asked for 0 units: N is fixed before the run, and 0 is not a design",
            spec.name,
        )));
    }
    let (mut schedule, master) = match spec.master {
        SeedScheduleKind::Diagnostic(master) => (SeedSchedule::diagnostic(master), master),
        SeedScheduleKind::OsEntropy => SeedSchedule::from_os_entropy(),
    };

    let players = spec.players;
    // The registered ceiling, read off the policy ONCE: it is both a report
    // field and the echo's token, and deriving it twice is two places to
    // disagree about which mode a printed figure was measured in.
    let capped_ceiling = match policy {
        CapPolicy::Refuse => None,
        CapPolicy::Count { ceiling } => Some(ceiling),
    };
    let mut x_unit_shares: Vec<f64> = Vec::with_capacity(spec.units as usize);
    // Every capped game, in schedule order. Stays empty in refuse mode, where
    // the driver's cap is an `Err` that never reaches this loop.
    let mut capped: Vec<CappedGame> = Vec::new();
    let mut games = 0u32;
    let mut x_game_wins = 0u32;
    let mut seat0_wins = 0u32;
    let mut ties_to_seat0 = 0u32;
    let mut top_vp_ties = 0u32;
    let mut vp_margin_sum = 0.0f64;
    let mut trigger = TriggerTally::default();
    let mut applied_by_seat = vec![0u64; players as usize];
    let mut x_applied_moves = 0u64;
    // `None` until an X seat hands back an explanation handle — which is how the
    // report distinguishes "this contestant has no explanation path" from "it
    // explained nothing" (see `CellReport::x_explanations`). The two cap
    // counters follow the same rule one level down: `None` until a SEARCHING X
    // hands one back.
    let mut x_explanations: Option<u64> = None;
    let mut x_ply_cap_hits: Option<u64> = None;
    let mut x_free_cap_hits: Option<u64> = None;
    // Spec §6's `x_*`/`y_*` budget meters. The Y side is folded over the
    // `players − 1` Y seats of every game — a FIELD total at 3–4P, which the
    // field's doc says rather than leaving a reader to divide by the wrong
    // denominator.
    let mut x_meters = BudgetMeters::default();
    let mut y_meters = BudgetMeters::default();
    // Spec §0.3's wall axis — one clock per SIDE, shared by every agent that
    // side builds across every game of the cell.
    let (x_clock, y_clock) = (WallClock::default(), WallClock::default());

    for _ in 0..spec.units {
        let seed = schedule.next_world();
        let unit = unit_for(players, seed);
        // The unit's outcome, held back until the unit is known READ, and the
        // flag that decides whether it ever folds.
        let mut unit_outcome = UnitOutcome::default();
        let mut unit_void = false;

        for (arr, seating) in unit.seatings.iter().enumerate() {
            // Which seat X is sitting in THIS arrangement. `unit_for` puts every
            // slot at every seat exactly once across the unit, so this is a
            // different seat each time and the seat effect cancels at the unit
            // level rather than being assumed away.
            let x_seat = seating
                .iter()
                .position(|&slot| slot == X_SLOT)
                .map(|i| PlayerId(i as u8))
                .ok_or_else(|| {
                    ArenaError::Driver(format!(
                        "seating {seating:?} has no slot {} to seat contestant X in",
                        X_SLOT.0,
                    ))
                })?;

            let seating = seating.clone();
            let (x, y) = (spec.x, spec.y);
            // The factory clones each X seat's explanation handle OUT before the
            // agent is boxed and moved into the driver, which is the only moment
            // it is reachable (`HonestPathTally`'s precedent). The keying is
            // untouched by the widening: every contestant, rung or evaluator,
            // plays on `agent_stream_seed(seed, seat, slot)`.
            let mut x_handles: Vec<ExplainHandle> = Vec::new();
            let mut y_handles: Vec<ExplainHandle> = Vec::new();
            let mut played = {
                let (xh, yh) = (&mut x_handles, &mut y_handles);
                let (xc, yc) = (&x_clock, &y_clock);
                let mut make_agent = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
                    let slot = seating[seat.0 as usize];
                    let is_x = slot == X_SLOT;
                    let contestant = if is_x { x } else { y };
                    let (agent, handle) =
                        contestant.build(rules, agent_stream_seed(seed, seat, slot));
                    if is_x {
                        xh.extend(handle);
                    } else {
                        yh.extend(handle);
                    }
                    // The stopwatch goes on LAST and delegates everything else,
                    // so the seat plays exactly what the contestant built.
                    Box::new(TimedAgent {
                        inner: agent,
                        clock: if is_x { xc.clone() } else { yc.clone() },
                    })
                };
                match policy {
                    // REFUSE MODE'S PER-GAME DRIVER CALL IS UNCHANGED: the
                    // public `run_arena_game` IS this call at
                    // `HONEST_MAX_TURNS`, so an unflagged cell reaches the same
                    // driver through the same chain it always did — a
                    // construction fact, which is what freeze doc §9.6 asks for
                    // ahead of any pin. Its cap is still an `Err`, so no
                    // `PlayedGame::Capped` can exist in this mode.
                    CapPolicy::Refuse => PlayedGame::Completed(Box::new(run_arena_game_capped(
                        seed, players, rules, &mut make_agent, invariants, max_turns,
                    )?)),
                    // COUNTING MODE plays the SAME game through the diagnostic
                    // driver — same setup half, same shared loop, same keying
                    // (W-D1 pinned the equivalence) — which is the only driver
                    // that hands a capped game's partial run back instead of
                    // dropping it.
                    CapPolicy::Count { .. } => {
                        let game = run_arena_game_diagnostic_capped(
                            seed, players, rules, &mut make_agent, invariants, max_turns,
                        )?;
                        match game.outcome {
                            DiagnosticOutcome::Terminated(stats) => PlayedGame::Completed(stats),
                            DiagnosticOutcome::Capped { turns, moves_by_seat, .. } => {
                                PlayedGame::Capped { turns, moves_by_seat }
                            }
                        }
                    }
                }
            };
            // The RUN half of provenance (spec §5): the driver stamped who
            // played each seat, and this is the only layer that knows which
            // schedule and which cell they played for. A capped game reaches
            // neither line — it has no `ArenaGameStats`, and the sink sees every
            // COMPLETED game and nothing else, void unit or not (a record is a
            // record; voiding is a unit-level statistic).
            if let PlayedGame::Completed(stats) = &mut played {
                stats.record.set_run_context(master, &spec.name);
                sink(stats, arr as u8)?;
            }

            // Spec §5.4's driver extension, aggregated two ways because they
            // answer two questions: `applied_by_seat` follows the SEAT (a design
            // balance read), `x_applied_moves` follows the CONTESTANT through the
            // rotation (spec §0.3(b)'s right-hand side). BOTH fold over every
            // played game — a capped game's per-seat split comes off the partial
            // run the diagnostic driver hands back.
            let moves_by_seat: &[u64] = match &played {
                PlayedGame::Completed(stats) => &stats.moves_by_seat,
                PlayedGame::Capped { moves_by_seat, .. } => moves_by_seat,
            };
            for (seat, moves) in moves_by_seat.iter().enumerate() {
                applied_by_seat[seat] += moves;
            }
            x_applied_moves += moves_by_seat[x_seat.0 as usize];
            for handle in &x_handles {
                *x_explanations.get_or_insert(0) += handle.explained();
                if let Some(hits) = handle.ply_cap_hits() {
                    *x_ply_cap_hits.get_or_insert(0) += hits;
                }
                if let Some(hits) = handle.free_cap_hits() {
                    *x_free_cap_hits.get_or_insert(0) += hits;
                }
                x_meters.fold(handle);
            }
            for handle in &y_handles {
                y_meters.fold(handle);
            }

            // THE OUTCOME HALF — buffered, never folded here. See
            // [`UnitOutcome`]: a cap in this unit's later game would otherwise
            // leave a contribution that cannot be taken back.
            match played {
                PlayedGame::Completed(stats) => {
                    unit_outcome.games += 1;
                    unit_outcome.triggers.push(stats.trigger);

                    if stats.result.winner == x_seat {
                        unit_outcome.x_wins += 1;
                    }
                    if stats.result.winner == PlayerId(0) {
                        unit_outcome.seat0_wins += 1;
                    }

                    // `final_result` (game::scoring) is a TOTAL order: it sorts
                    // on `(Reverse(total), clockwise distance from
                    // start_player)`, and the distance `(seat + n - start) % n`
                    // is a bijection from seat onto 0..n, so no two players can
                    // share a sort key. A game therefore NEVER reports a tie —
                    // the clockwise tie-break has already decided it, and
                    // `winner` is always exactly one seat. What is recorded here
                    // is the tie-break's WORK: how often the top VP total was
                    // shared at all, and how often seat 0 was the one it
                    // awarded.
                    let standings = &stats.result.standings;
                    let top_shared = standings.len() >= 2
                        && standings[0].breakdown.total == standings[1].breakdown.total;
                    if top_shared {
                        unit_outcome.top_vp_ties += 1;
                        if stats.result.winner == PlayerId(0) {
                            unit_outcome.ties_to_seat0 += 1;
                        }
                    }

                    let mut x_total = 0.0f64;
                    let mut y_total = 0.0f64;
                    for standing in standings {
                        if standing.player == x_seat {
                            x_total = f64::from(standing.breakdown.total);
                        } else {
                            y_total += f64::from(standing.breakdown.total);
                        }
                    }
                    unit_outcome.vp_margins.push(x_total - y_total / f64::from(players - 1));
                }
                PlayedGame::Capped { turns, .. } => {
                    // The counted event. The unit is VOID from here — and the
                    // rest of its schedule is still PLAYED, because the ceiling
                    // counts capped games over the full schedule, which is what
                    // the measured per-game rates priced.
                    unit_void = true;
                    capped.push(CappedGame { seed, arrangement: arr as u8, turns });
                    // THE CEILING, checked the moment the count passes it: the
                    // (c+1)-th capped game refuses the cell AT ONCE rather than
                    // at the schedule's end, because an instrument guard that
                    // waited would spend the rest of a run it has already
                    // invalidated (freeze doc §9.6 item 2).
                    if let Some(ceiling) = capped_ceiling {
                        if capped.len() as u64 > u64::from(ceiling) {
                            return Err(ArenaError::CappedCeilingExceeded { ceiling, capped });
                        }
                    }
                }
            }
        }

        if unit_void {
            // A VOID unit contributes NO OUTCOME. Its games were played, its
            // completed ones reached the sink, and its volume is counted — the
            // outcome, and only the outcome, is dropped.
            continue;
        }
        games += unit_outcome.games;
        x_game_wins += unit_outcome.x_wins;
        seat0_wins += unit_outcome.seat0_wins;
        top_vp_ties += unit_outcome.top_vp_ties;
        ties_to_seat0 += unit_outcome.ties_to_seat0;
        for game_trigger in &unit_outcome.triggers {
            trigger.observe(game_trigger.as_ref());
        }
        // In game order, so the cell's float sum is the sequence it always was.
        for margin in &unit_outcome.vp_margins {
            vp_margin_sum += margin;
        }
        x_unit_shares.push(f64::from(unit_outcome.x_wins) / unit.seatings.len() as f64);
    }

    // NO READ UNIT: nothing constrains theta, and a report of NaNs would be a
    // number-shaped answer to a question the data cannot answer. Unreachable in
    // refuse mode — the zero-unit guard above runs before any game and every
    // unit is read there — so this is the counting mode's own refusal.
    if x_unit_shares.is_empty() {
        return Err(ArenaError::NoReadUnits { scheduled: spec.units, capped });
    }

    let x_unit_shares_len = x_unit_shares.len() as u32;
    let theta_hat = mean(&x_unit_shares);
    // Derived from the design, never from the outcomes — a resample stream that
    // read the unit vector would make the interval a function of the data twice
    // over. Two cells sharing (master, units, players) share this stream, which
    // is harmless: they resample different unit vectors, and the only property
    // required of it is that it be fixed and RECORDED.
    let resample_seed = mix(
        SALT_BOOTSTRAP,
        &[master, u64::from(spec.units), u64::from(players)],
    );
    let interval = interval_for_units(&x_unit_shares, BOOTSTRAP_REPLICATES, resample_seed);

    let cross_checks = cross_checks(
        &x_unit_shares,
        theta_hat,
        players,
        games,
        x_game_wins,
        top_vp_ties,
        ties_to_seat0,
        &interval,
    );

    Ok((
        CellReport {
            spec_echo: spec_echo(spec, master, resample_seed, capped_ceiling),
            games,
            x_unit_shares,
            theta_hat,
            interval,
            cross_checks,
            seat0_wins,
            ties_to_seat0,
            vp_margin_mean: vp_margin_sum / f64::from(games),
            applied_by_seat,
            x_applied_moves,
            x_explanations,
            x_ply_cap_hits,
            x_free_cap_hits,
            x_candidates_scored: x_meters.candidates_scored,
            x_root_scored: x_meters.root_scored,
            x_budget_binds: x_meters.binds,
            x_budget_starved_decisions: x_meters.starved,
            y_candidates_scored: y_meters.candidates_scored,
            y_root_scored: y_meters.root_scored,
            y_budget_binds: y_meters.binds,
            y_budget_starved_decisions: y_meters.starved,
            trigger,
            master,
            capped_ceiling,
            units_scheduled: spec.units,
            units_void: spec.units - x_unit_shares_len,
            units_read: x_unit_shares_len,
            capped,
        },
        CellWall { x: x_clock.read(), y: y_clock.read() },
    ))
}

/// The 95% percentile bootstrap over `units`: resample `units` with replacement
/// `b` times, and take the 2.5th and 97.5th percentiles of the replicate means.
///
/// Endpoints are picked with the crate's ONE nearest-rank rule
/// ([`nearest_rank_index`], the same one every quantile row uses): at
/// `b = 10_000` that is the 250th and 9,750th order statistics, leaving 2.5% of
/// replicates strictly outside each end.
///
/// Deterministic in `resample_seed` — the report records it, so an interval is
/// re-derivable from a published cell without replaying a single game.
///
/// An empty `units` or `b == 0` is caller misuse: both are `debug_assert`ed, and
/// in release both return `(NaN, NaN)` rather than a plausible number, so a
/// downstream containment check fails instead of silently passing (the honest
/// direction, matching `wilson_95`'s `n == 0` reasoning).
pub fn bootstrap_percentile(units: &[f64], b: u32, resample_seed: u64) -> (f64, f64) {
    debug_assert!(!units.is_empty(), "bootstrap_percentile over an empty unit vector");
    debug_assert!(b > 0, "bootstrap_percentile with B = 0");
    let n = units.len();
    if n == 0 || b == 0 {
        return (f64::NAN, f64::NAN);
    }

    let mut rng = ChaCha8Rng::seed_from_u64(resample_seed);
    let mut replicate_means: Vec<f64> = Vec::with_capacity(b as usize);
    for _ in 0..b {
        let mut sum = 0.0f64;
        for _ in 0..n {
            sum += units[rng.gen_range(0..n)];
        }
        replicate_means.push(sum / n as f64);
    }
    replicate_means.sort_by(|a, b| {
        a.partial_cmp(b).expect("unit outcomes are finite, so replicate means are too")
    });

    let b_len = b as usize;
    (
        replicate_means[nearest_rank_index(b_len, 25, 1000)],
        replicate_means[nearest_rank_index(b_len, 975, 1000)],
    )
}

/// The primary interval for a unit vector, or the typed refusal when there is
/// nothing to resample.
///
/// The degeneracy test is exact equality against the first unit, not
/// `sd < epsilon`: spec §4's condition is literally `sd(units) == 0`, and a
/// structural "every entry is the same value" cannot be fooled by accumulation
/// order the way a computed sd can. A vector with even one unit moved off the
/// constant gets a real bootstrap interval.
pub fn interval_for_units(units: &[f64], b: u32, resample_seed: u64) -> IntervalOutcome {
    let n = units.len();
    debug_assert!(n > 0, "interval_for_units over an empty unit vector");
    if n == 0 {
        return IntervalOutcome::RefusedZeroWidth { one_sided_lower: 0.0, method: NO_UNITS };
    }
    if units.iter().all(|&u| u == units[0]) {
        let (one_sided_lower, method) = if units[0] >= 1.0 {
            // Clamped: rule-of-three is meaningful only at n >= 3, and below
            // that the raw formula is a NEGATIVE probability — the exact class
            // of dishonest number this refusal exists to never print (T7
            // review F1; reachable from T9's argv at `units 1`).
            ((1.0 - 3.0 / n as f64).max(0.0), ALL_UNITS_WON)
        } else if units[0] <= 0.0 {
            (0.0, ALL_UNITS_LOST)
        } else {
            (0.0, CONSTANT_INTERIOR)
        };
        return IntervalOutcome::RefusedZeroWidth { one_sided_lower, method };
    }
    let (lo, hi) = bootstrap_percentile(units, b, resample_seed);
    IntervalOutcome::Bootstrap { lo, hi, b, resample_seed }
}

/// Design units needed to detect a true win share of `mme_theta` against the
/// 0.5 null with 80% power at two-sided α = 0.05:
/// `ceil(7.848878 · var_pair / (mme_theta − 0.5)²)`.
///
/// A **2P PAIR formula** — the 0.5 in the denominator is the two-contestant null
/// (see the module doc; at 3–4P the single-champion null is `1/players` and this
/// function does not apply). `var_pair` is the variance of the PAIR outcome,
/// which spec §4 supplies from measurement (0.1225) rather than from theory.
///
/// SIZE THE CELL FROM A PRE-SPECIFIED EDGE, NOT A PILOT. Feeding this a pilot
/// point estimate is the measured hazard the module doc names: a 0.28pp pilot
/// error cost 20 points of power.
///
/// `mme_theta == 0.5` asks for the units needed to detect no edge at all and
/// honestly returns `u64::MAX`; a non-finite or negative `var_pair` is caller
/// misuse and is `debug_assert`ed.
pub fn pairs_needed_for_mme(mme_theta: f64, var_pair: f64) -> u64 {
    debug_assert!(
        var_pair.is_finite() && var_pair >= 0.0,
        "pairs_needed_for_mme: var_pair must be a non-negative variance, got {var_pair}",
    );
    debug_assert!(
        mme_theta.is_finite() && (0.0..=1.0).contains(&mme_theta),
        "pairs_needed_for_mme: mme_theta is a win share in [0, 1], got {mme_theta}",
    );
    let delta = mme_theta - 0.5;
    if delta == 0.0 || !delta.is_finite() {
        return u64::MAX;
    }
    let needed = MME_CONSTANT * var_pair / (delta * delta);
    if !needed.is_finite() {
        return u64::MAX;
    }
    // .max(1.0): a zero variance would size a 0-unit design that only fails
    // later and further from the mistake (T7 review F6) — one unit is the
    // smallest runnable design and still fails honestly at tabulation.
    needed.ceil().max(1.0) as u64
}

// ---------------------------------------------------------------------------
// Printed cross-checks and echoes
// ---------------------------------------------------------------------------

pub(crate) fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Sample standard deviation (n − 1 denominator), or `NaN` under two units.
/// `pub(crate)` since Phase 3.5c W3 so `eval_screen`'s sd means what a cell's
/// does by SHARING the function rather than by copying it (its review, SHOULD-4).
pub(crate) fn sample_sd(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return f64::NAN;
    }
    let m = mean(values);
    let ss: f64 = values.iter().map(|v| (v - m) * (v - m)).sum();
    (ss / (values.len() - 1) as f64).sqrt()
}

/// The 97.5th percentile of Student's t with `df` degrees of freedom, by the
/// Cornish–Fisher expansion around [`Z_95`].
///
/// Accurate to better than 1e-4 for `df ≥ 10` (spot-checked against the
/// published table: df 30 → 2.04227 exact, this expansion 2.042271; df 39 →
/// 2.02269 both). Good enough BECAUSE the unit-t interval is a printed
/// cross-check that nothing gates on — the primary interval is the bootstrap,
/// which needs no distributional quantile at all. `df == 0` (one unit) has no
/// finite quantile and returns infinity.
fn student_t_975(df: u64) -> f64 {
    if df == 0 {
        return f64::INFINITY;
    }
    let v = df as f64;
    let z = Z_95;
    let (z2, z3) = (z * z, z * z * z);
    let (z5, z7, z9) = (z3 * z2, z3 * z2 * z2, z3 * z2 * z2 * z2);
    z + (z3 + z) / (4.0 * v)
        + (5.0 * z5 + 16.0 * z3 + 3.0 * z) / (96.0 * v * v)
        + (3.0 * z7 + 19.0 * z5 + 17.0 * z3 - 15.0 * z) / (384.0 * v.powi(3))
        + (79.0 * z9 + 776.0 * z7 + 1482.0 * z5 - 1920.0 * z3 - 945.0 * z)
            / (92160.0 * v.powi(4))
}

fn spec_echo(
    spec: &CellSpec,
    master: u64,
    resample_seed: u64,
    capped_ceiling: Option<u32>,
) -> String {
    let unit_shape = if spec.players == 2 {
        "pair (2 seatings)".to_string()
    } else {
        format!("rotation block ({} seatings)", spec.players)
    };
    let master_kind = match spec.master {
        SeedScheduleKind::Diagnostic(_) => "diagnostic",
        SeedScheduleKind::OsEntropy => "os-entropy (drawn)",
    };
    // Spec §5.5, printed per cell because a unit's meaning depends on it: `yes`
    // says the two seatings of a unit start from a BIT-IDENTICAL state, `no` says
    // they are seed-paired and swap-balanced but resolve the same deal by two
    // different Keep rules. The unit bootstrap is valid either way (ledger row
    // 15: pairing is bookkeeping, the swap is what removes the seat effect) —
    // what changes is what a reader may claim about the units.
    //
    // SPELLED `key=value`, not the spec's prose `setup-identical: yes|no`,
    // matching every other token in this echo (`cell=`, `master=`, `fixed-N=`).
    // The T6 review corrected this comment's original grounds, which were
    // FALSE: the echo already carries interior-space VALUES (`unit=pair (2
    // seatings)`), and the golden's token checks are substring `contains` that
    // a colon spelling would not have disturbed. The `key=value` spelling is
    // kept for consistency, not necessity. Recorded formatting deviation.
    let setup_identical = if spec.x.keep_kind() == spec.y.keep_kind() { "yes" } else { "no" };
    // A-S11-1's ONE trailing token, and only in COUNTING mode: an absent
    // `--capped-ceiling` emits no token at all, exactly as an absent `cap` knob
    // emits none on `lookahead-v1` (freeze doc §9.6). TRAILING, so a counting
    // echo is the refuse-mode echo plus a suffix — a property a test can state
    // in one line, rather than a second echo format to keep in step.
    let capped_ceiling = match capped_ceiling {
        Some(ceiling) => format!(" capped-ceiling={ceiling}"),
        None => String::new(),
    };
    format!(
        "cell={} players={} x={} y={} units={} unit={} games={} \
         master={master_kind}:{master:#018x} bootstrap_B={BOOTSTRAP_REPLICATES} \
         resample_seed={resample_seed:#018x} single-threaded=yes fixed-N=yes \
         setup-identical={setup_identical}{capped_ceiling}",
        spec.name,
        spec.players,
        // [`Contestant::echo`], the one composer — public since W5 so the CLI
        // reads the same sentence rather than growing a second one.
        spec.x.echo(),
        spec.y.echo(),
        spec.units,
        unit_shape,
        u32::from(spec.players) * spec.units,
    )
}

#[allow(clippy::too_many_arguments)]
fn cross_checks(
    units: &[f64],
    theta_hat: f64,
    players: u8,
    games: u32,
    x_game_wins: u32,
    top_vp_ties: u32,
    ties_to_seat0: u32,
    interval: &IntervalOutcome,
) -> Vec<String> {
    let n = units.len();
    let mut rows = Vec::new();

    rows.push(format!(
        "null reference for this cell: 1/{players} = {:.6} (ONE X against {} Y seats — \
         the single-champion shape; 0.5 is the null only at 2P)",
        1.0 / f64::from(players),
        players - 1,
    ));

    let sd = sample_sd(units);
    if sd.is_finite() && sd > 0.0 {
        let t = student_t_975(n as u64 - 1);
        let half = t * sd / (n as f64).sqrt();
        rows.push(format!(
            "unit-t 95% cross-check: [{:.6}, {:.6}] (n={n} units, sd={sd:.6}, \
             t(0.975, {})≈{t:.4}) — printed, never primary; asymptotic expansion, \
             ~11% narrow at df=1 and unreliable below df≈10 (T7 review F5)",
            theta_hat - half,
            theta_hat + half,
            n - 1,
        ));
    } else {
        rows.push(format!(
            "unit-t 95% cross-check: refused (sd={sd:.6} over n={n} units) — see the \
             interval's zero-width refusal",
        ));
    }

    let wilson = wilson_95(u64::from(x_game_wins), u64::from(games));
    rows.push(format!(
        "Wilson-on-pooled-games 95% cross-check: [{:.6}, {:.6}] ({x_game_wins}/{games} games) \
         — CROSS-CHECK ONLY and NEVER primary at 3-4P: games within a unit are clustered \
         (E2 measured 4P rho=+0.051, dragging a nominal 95% interval to ~93.2% true coverage), \
         so this row understates the width the unit bootstrap reports",
        wilson.lo, wilson.hi,
    ));

    rows.push(format!(
        "top-VP ties: {top_vp_ties}/{games} games ({:.4}), {ties_to_seat0} decided for seat 0 — \
         `final_result` is a total order and never reports a tie, so this is how often the \
         clockwise tie-break DECIDED the game{}",
        f64::from(top_vp_ties) / f64::from(games),
        if players == 2 {
            "; at 2P seat 0 takes every tie, so its award is half this rate (spec §4's +0.63pp \
             at the bracket rate of 1.26%)"
        } else {
            "; at 3-4P the award depends on which seats tied (the recorded seat-0 count above is \
             the realized figure — the 2P half-the-rate derivation does not apply)"
        },
    ));

    // Near-degeneracy diagnostic (T7 review F3): percentile-bootstrap coverage
    // degrades toward the boundary (independently computed: 0.39 at theta=0.99
    // N=50; 0.87 at N=200), and the refusal above fires only on EXACT
    // constancy — so a saturated endpoint or a nearly-constant unit vector
    // gets its caveat printed rather than silently looking like a clean read.
    if let IntervalOutcome::Bootstrap { lo, hi, .. } = interval {
        let distinct = {
            let mut vals: Vec<u64> = units.iter().map(|u| u.to_bits()).collect();
            vals.sort_unstable();
            vals.dedup();
            vals.len()
        };
        if *lo <= 0.0 || *hi >= 1.0 || distinct < 3 {
            rows.push(format!(
                "NEAR-BOUNDARY CAVEAT: interval endpoint saturated at 0/1 or unit vector has \
                 only {distinct} distinct value(s) — percentile-bootstrap coverage degrades \
                 toward the boundary (measured 0.39-0.87 at theta=0.99 for N=50-200); read this \
                 interval as directional, not calibrated",
            ));
        }
    }

    if let IntervalOutcome::RefusedZeroWidth { method, .. } = interval {
        if n >= 3 {
            rows.push(format!(
                "zero-width refusal ({method}); complementary rule-of-three bound over n={n} \
                 units: 3/n = {:.6}",
                3.0 / n as f64,
            ));
        } else {
            rows.push(format!(
                "zero-width refusal ({method}); n={n} is below rule-of-three's n >= 3 floor, so \
                 no complementary bound is printed (T7 review F1)",
            ));
        }
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::greedy::{GREEDY_CONTROLLER, GREEDY_IMPL_VERSION};
    use crate::types::OrderCardId;
    use crate::game::policy::POLICY_IMPL_VERSION;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    /// This task's committed diagnostic master. The constructor's NAME is the
    /// waiver (`game::seeds`): an asserting cell wants reproducibility, and an
    /// OS-entropy gate would flake at the nominal rate by construction.
    const MASTER: u64 = 0xA3E1_2026_0726_0007;

    /// Does the percentile bootstrap actually cover the truth it claims to?
    ///
    /// §0-R3(iii)'s free half — synthetic units with a KNOWN θ, no games played.
    /// Units are Bernoulli(θ), so the true mean is θ exactly and coverage is
    /// countable.
    ///
    /// PARAMETERS, AND WHY THEY ARE NOT THE PLANNED ONES. The plan's full-scale
    /// check is R=2,000 replicates per θ at N=200 units. Measured with THIS
    /// estimator at B=400: **1.16s (θ=0.5) / 1.12s (θ=0.6) in RELEASE, but
    /// ~66.5s / ~66.7s in DEBUG** (extrapolated from a measured R=60 run at
    /// 2.00s each) — a ~57× profile gap, because every resample index is a
    /// ChaCha8 draw and ChaCha8's block function is that much slower
    /// unoptimized. 133s for both θ is not a per-edit test at any useful
    /// coverage resolution, and shrinking R until it fits leaves a band too wide
    /// to mean anything. This per-edit slice therefore runs **R=200 per θ at
    /// N=200, B=200** (measured 6.7s debug, off the suite's 11.1s critical
    /// path), and the FULL-SCALE run is handed to Task 9's
    /// `tests/arena_gate.rs` release binary, which is where the plan already
    /// sites §0-R3(iii)'s other half (`real_game_replicates_match_reported_se`)
    /// — one release binary owning both halves of one requirement.
    ///
    /// RECORDED FOR THAT TASK (master `0xA3E1_2026_0726_0007`, release,
    /// R=2,000 · N=200 · B=400): coverage **0.9540 at θ=0.5**, **0.9600 at
    /// θ=0.6**. What this slice itself measured at HEAD: coverage **0.9650 /
    /// 0.9450**, width ratios **0.9901 / 0.9914**.
    ///
    /// THE BAND: 0.95 ± 3.29·√(0.95·0.05/200) = [0.899, 1.0], the 99.9%
    /// binomial band at R=200 around the nominal rate — rounded out to 0.89.
    ///
    /// ANTI-VACUITY IS LOAD-BEARING HERE. A coverage band that is open at the
    /// top passes trivially for an estimator that returns [0, 1] every time, so
    /// the mean interval WIDTH is pinned to the theoretical
    /// 2·z·√(θ(1−θ)/N) within ±15% as well. Coverage alone would not catch a
    /// bootstrap that resampled with too much spread; width alone would not
    /// catch one centred wrong.
    #[test]
    fn bootstrap_covers_a_known_bernoulli_theta() {
        const N_UNITS: usize = 200;
        const B: u32 = 200;
        const R: u32 = 200;

        for (t_index, theta) in [0.5f64, 0.6].into_iter().enumerate() {
            let mut rng = ChaCha8Rng::seed_from_u64(MASTER ^ (t_index as u64 + 1));
            let mut covered = 0u32;
            let mut width_sum = 0.0f64;

            for _ in 0..R {
                let units: Vec<f64> = (0..N_UNITS)
                    .map(|_| if rng.gen_bool(theta) { 1.0 } else { 0.0 })
                    .collect();
                let (lo, hi) = bootstrap_percentile(&units, B, rng.gen());
                if lo <= theta && theta <= hi {
                    covered += 1;
                }
                width_sum += hi - lo;
            }

            let coverage = f64::from(covered) / f64::from(R);
            assert!(
                (0.89..=1.0).contains(&coverage),
                "theta {theta}: coverage {coverage} outside the R={R} binomial band [0.89, 1.0]",
            );

            let nominal = 2.0 * Z_95 * (theta * (1.0 - theta) / N_UNITS as f64).sqrt();
            let ratio = (width_sum / f64::from(R)) / nominal;
            assert!(
                (0.92..=1.08).contains(&ratio),
                "theta {theta}: mean interval width is {ratio:.3}x the theoretical \
                 2z*sqrt(theta(1-theta)/N) = {nominal:.6} — coverage alone cannot catch this",
            );
        }
    }

    /// §0-R3(iv): a degenerate cell produces a typed refusal with a one-sided
    /// bound, NEVER a printed ±0.000.
    ///
    /// This is not a hypothetical corner. The two rungs that exist are a
    /// blowout — retreat-averse won 4000/4000 games at 2P — so the bracket cell
    /// hands this path an all-constant vector every time it runs.
    #[test]
    fn zero_width_is_refused_with_a_one_sided_bound() {
        // All 60 units won: the rule-of-three one-sided 95% lower bound.
        let all_wins = vec![1.0f64; 60];
        match interval_for_units(&all_wins, BOOTSTRAP_REPLICATES, 1) {
            IntervalOutcome::RefusedZeroWidth { one_sided_lower, method } => {
                assert!(
                    (one_sided_lower - (1.0 - 3.0 / 60.0)).abs() < 1e-12,
                    "expected the rule-of-three bound 1 - 3/60 = 0.95, got {one_sided_lower}",
                );
                assert_eq!(method, ALL_UNITS_WON);
            }
            other => panic!("a constant unit vector must be refused, got {other:?}"),
        }

        // The mirror image. 0 is the only lower bound this data supports, and
        // the method string says where the information actually is.
        match interval_for_units(&vec![0.0f64; 60], BOOTSTRAP_REPLICATES, 1) {
            IntervalOutcome::RefusedZeroWidth { one_sided_lower, method } => {
                assert_eq!(one_sided_lower, 0.0);
                assert_eq!(method, ALL_UNITS_LOST);
            }
            other => panic!("an all-lost vector must be refused, got {other:?}"),
        }

        // Constant but interior — every pair split 1-1. sd is still 0, so there
        // is still nothing to resample, but rule-of-three does not apply and
        // claiming theta >= 0.5 from a point mass would be exactly the false
        // comfort the refusal exists to prevent.
        match interval_for_units(&vec![0.5f64; 60], BOOTSTRAP_REPLICATES, 1) {
            IntervalOutcome::RefusedZeroWidth { one_sided_lower, method } => {
                assert_eq!(one_sided_lower, 0.0);
                assert_eq!(method, CONSTANT_INTERIOR);
            }
            other => panic!("an interior-constant vector must be refused, got {other:?}"),
        }

        // ANTI-VACUITY: the refusal must be triggered by degeneracy, not by
        // being the only path implemented. One unit moved off the constant is
        // enough to buy a real two-sided interval.
        let mut one_moved = vec![1.0f64; 60];
        one_moved[7] = 0.5;
        match interval_for_units(&one_moved, BOOTSTRAP_REPLICATES, 1) {
            IntervalOutcome::Bootstrap { lo, hi, b, resample_seed } => {
                assert!(hi > lo, "a non-degenerate vector must report a positive width");
                assert_eq!(b, BOOTSTRAP_REPLICATES);
                assert_eq!(resample_seed, 1);
                assert!(lo <= 1.0 && hi <= 1.0);
            }
            other => panic!("a varying unit vector must get a real interval, got {other:?}"),
        }
    }

    /// Spec §4's sizing table: MME 0.60 against the 0.5 null at the measured
    /// pair variance 0.1225 needs ~97 pairs. The spec's promotion cells round
    /// that UP to 200 pairs — that rounding is policy and is asserted where it
    /// is used, not here.
    #[test]
    fn pairs_needed_matches_e2s_table() {
        let needed = pairs_needed_for_mme(0.60, 0.1225);
        assert!(
            (94..=100).contains(&needed),
            "MME 0.60 at var_pair 0.1225 should need ~97 pairs, got {needed}",
        );

        // Sharper than the band, because the arithmetic is hand-checkable:
        // 7.848878 * 0.1225 / 0.01 = 96.149, and the rule rounds UP to whole
        // units. Spec §4's promotion cells then round 97 up again to 200 pairs;
        // that second rounding is POLICY and is asserted where it is used.
        assert_eq!(needed, 97);

        // NOT the same rule as spec §4's milestone-precision cells (h = 0.02 ->
        // ~1,189 pairs, lens-re-derived 1,182-1,207). Those size for a CI
        // HALF-WIDTH, `z^2 * var / h^2`, which answers "how precisely will I
        // measure theta" — a different question from this function's "how small
        // an edge will I detect at 80% power", and it lands ~2.0x apart at the
        // same numbers (2,404 here vs ~1,189 there). 3.5 owns half-width sizing;
        // conflating the two is why this note is executable rather than prose.
        assert_eq!(pairs_needed_for_mme(0.52, 0.1225), 2_404);

        // A tighter edge costs more, and detecting NO edge is not financeable.
        assert!(pairs_needed_for_mme(0.55, 0.1225) > needed);
        assert_eq!(pairs_needed_for_mme(0.5, 0.1225), u64::MAX);
    }

    /// The instrument's self-test at per-edit scale (§0-R3(ii)): a cell where
    /// both contestants run the SAME rung must read near its null with a
    /// nonzero width.
    ///
    /// This is the test E2's CRN trap fails loudly: keying the play stream by
    /// seat alone makes the two games of a pair the same game, every pair reads
    /// exactly 0.5, and the interval collapses to a refusal — an instrument
    /// reporting "perfectly calibrated" while measuring nothing. The assertion
    /// that the interval is a real `Bootstrap` is what catches that.
    ///
    /// SIZE AND BAND: 40 units = 80 retreat-averse 2P games, measured at
    /// **0.078 s/game debug with per-move invariants ON** (mean 99 turns) for a
    /// measured **6.79s** — below the debug suite's 11.1s critical path, i.e.
    /// free in wall clock. The band [0.30, 0.70] is deliberately wide: the
    /// realized unit sd is 0.375, so the standard error over 40 units is 0.059
    /// and ±0.20 is ~3.4 SE. The TIGHT null cell (200 units) is the release
    /// gate's, per spec §0-R3(ii).
    ///
    /// MEASURED AT HEAD: θ̂ = 0.475 over 80 games, bootstrap interval
    /// [0.3625, 0.5875], 40/80 games won by seat 0, and 1/80 games (1.25%)
    /// decided by the clockwise tie-break — which independently reproduces E2's
    /// 1.26% bracket tie rate on a different policy pair.
    #[test]
    fn a_null_cell_smoke_reads_near_half() {
        let rules = Rules::standard();
        let spec = CellSpec {
            name: "null-ra-vs-ra-smoke".to_string(),
            players: 2,
            x: Contestant::Policy(RolloutPolicy::RetreatAverse),
            y: Contestant::Policy(RolloutPolicy::RetreatAverse),
            units: 40,
            master: SeedScheduleKind::Diagnostic(MASTER),
        };

        let report = run_cell(&spec, &rules).unwrap_or_else(|e| panic!("null cell failed: {e}"));

        assert_eq!(report.games, 80, "40 pairs is 80 games");
        assert_eq!(report.x_unit_shares.len(), 40);
        assert_eq!(report.trigger.games(), 80, "every completed game is a trigger observation");
        assert_eq!(report.master, MASTER);
        assert!(
            (0.30..=0.70).contains(&report.theta_hat),
            "a same-rung cell read theta_hat = {} — far from its 0.5 null",
            report.theta_hat,
        );

        match report.interval {
            IntervalOutcome::Bootstrap { lo, hi, b, .. } => {
                assert_eq!(b, BOOTSTRAP_REPLICATES);
                assert!(hi > lo, "a null cell must report a NONZERO width, got [{lo}, {hi}]");
                assert!(
                    lo <= report.theta_hat && report.theta_hat <= hi,
                    "the point estimate {} fell outside its own interval [{lo}, {hi}]",
                    report.theta_hat,
                );
                assert!(
                    lo <= 0.5 && 0.5 <= hi,
                    "the interval [{lo}, {hi}] misses the 0.5 null of a same-rung cell",
                );
            }
            other => panic!("a null cell must report a real interval, got {other:?}"),
        }

        // The unit outcomes are pair win shares, so they can only be 0, 0.5 or 1
        // — and if they were all identical the interval above would have been a
        // refusal, which is the shape this cell exists to distinguish from.
        assert!(report.x_unit_shares.iter().all(|s| [0.0, 0.5, 1.0].contains(s)));

        // The printed rows a reader is entitled to, with the clustering caveat
        // riding the Wilson row rather than living in prose somewhere else.
        let joined = report.cross_checks.join("\n");
        assert!(joined.contains("null reference for this cell: 1/2"), "{joined}");
        assert!(joined.contains("unit-t 95% cross-check"), "{joined}");
        assert!(joined.contains("NEVER primary at 3-4P"), "{joined}");
        assert!(joined.contains("rho=+0.051"), "{joined}");
        assert!(joined.contains("top-VP ties"), "{joined}");
    }

    /// §0-R3(v): same master, same spec, byte-identical report.
    ///
    /// Byte-identity over the SERIALIZED report is the strong form — it covers
    /// the per-unit outcome vector, the interval endpoints, the drawn resample
    /// seed, the tie accounting and the trigger tally at once, so any stream
    /// that quietly picked up an unrecorded input shows up here.
    #[test]
    fn same_master_reproduces_the_report_byte_identically() {
        let rules = Rules::standard();
        let spec = CellSpec {
            name: "determinism".to_string(),
            players: 2,
            x: Contestant::Policy(RolloutPolicy::RetreatAverse),
            y: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
            units: 4,
            master: SeedScheduleKind::Diagnostic(MASTER),
        };

        let first = serde_json::to_string(&run_cell(&spec, &rules).unwrap()).unwrap();
        let second = serde_json::to_string(&run_cell(&spec, &rules).unwrap()).unwrap();
        assert_eq!(first, second, "the same master produced two different reports");

        // ANTI-VACUITY: byte-identity is worthless if the report does not
        // depend on the master at all. A different master must move the bytes.
        let mut other = spec.clone();
        other.master = SeedScheduleKind::Diagnostic(MASTER ^ 0xFFFF_FFFF);
        let third = serde_json::to_string(&run_cell(&other, &rules).unwrap()).unwrap();
        assert_ne!(first, third, "the report did not depend on the master");
    }

    /// The report's own echo carries the measurement policy, so a quoted figure
    /// cannot be separated from the conditions that produced it.
    #[test]
    fn the_spec_echo_carries_the_measurement_policy() {
        let spec = CellSpec {
            name: "fw-knob-edge".to_string(),
            players: 4,
            x: Contestant::Policy(RolloutPolicy::FamilyWeighted { retreat_ppm: 50_000, recovery_min_of: 5 }),
            y: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
            units: 100,
            master: SeedScheduleKind::Diagnostic(MASTER),
        };
        let echo = spec_echo(&spec, MASTER, 0xDEAD_BEEF, None);

        assert!(echo.contains("cell=fw-knob-edge players=4"), "{echo}");
        assert!(echo.contains("units=100"), "{echo}");
        assert!(echo.contains("games=400"), "{echo}");
        assert!(echo.contains("rotation block (4 seatings)"), "{echo}");
        assert!(echo.contains("master=diagnostic:0xa3e1202607260007"), "{echo}");
        assert!(echo.contains("bootstrap_B=10000"), "{echo}");
        assert!(echo.contains("fixed-N=yes"), "{echo}");
        // The two knob settings of the diagnostic edge cell are DIFFERENT
        // contestants, so the echo must distinguish them — `as_str()` alone
        // would print "family-weighted" for both and make the cell unreadable.
        assert!(echo.contains("retreat_ppm=50000"), "{echo}");
        assert!(echo.contains("retreat_ppm=166667"), "{echo}");
        assert!(echo.contains("recovery_min_of=5"), "{echo}");
    }

    /// **Spec §0.3(b) at CELL level, plus the §5.4 driver extensions that make
    /// it expressible at all**, plus the corpus half of §5.2 — one cell, because
    /// they are three readings of the same two games and the games are what this
    /// test costs (MEASURED **0.64 s debug** for the whole test — the greedy pair
    /// plus the rung control — at HEAD; the release-scale cell is T11's).
    ///
    /// WHY THE DRIVER NEEDED EXTENDING. Before this task the claim "every move
    /// the evaluator bot played was explained" was unstatable here: the emission
    /// sink is per-GAME and `ArenaGameStats` carried only a total
    /// `moves_applied`, so there was no per-contestant denominator to compare a
    /// tally against — and a contestant occupies a different SEAT in every
    /// arrangement, so a per-seat total is not one either. `moves_by_seat`
    /// (driver) plus `x_applied_moves` (cell, following X through the rotation)
    /// are the two halves, and the equality below is what they buy.
    ///
    /// THE `Option` IS LOAD-BEARING and is asserted from both sides: an
    /// evaluator X reports `Some(n)`, a rung X reports `None`. A plain `0` would
    /// let an evaluator that silently stopped explaining pass the identity a rung
    /// passes by design.
    ///
    /// MUTATION-PROVEN (reverted by inverse edit, never `git checkout` — L7):
    /// attributing each applied move to `(mover + 1) % players` in the driver —
    /// a plausible off-by-one in exactly the new code — fails the identity at
    /// `Some(511)` vs `Some(586)`. The two sides genuinely come from independent
    /// places: the left from the agent's own atomic counter, the right from the
    /// driver's per-seat tally followed through the rotation.
    #[test]
    fn a_greedy_x_cell_counts_and_labels_every_move_it_explained() {
        let rules = Rules::standard();
        let spec = CellSpec {
            name: "t6-greedy-vs-fw-counters".to_string(),
            players: 2,
            x: Contestant::GREEDY_V1_DEFAULT,
            y: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
            units: 1,
            master: SeedScheduleKind::Diagnostic(MASTER),
        };

        /// One game's per-seat provenance, as the sink saw it.
        type SeatLabels = Vec<(String, u32, String)>;
        let mut per_game: Vec<SeatLabels> = Vec::new();
        let mut total_moves = 0u64;
        let report = run_cell_emitting(&spec, &rules, true, &mut |stats, arr| {
            let run = stats.record.run.as_ref().expect("a cell's records carry provenance");
            per_game.push(
                run.seats
                    .iter()
                    .map(|s| (s.controller.clone(), s.version, s.observation.clone()))
                    .collect(),
            );
            total_moves += stats.moves_applied;
            assert_eq!(
                stats.moves_by_seat.iter().sum::<u64>(),
                stats.moves_applied,
                "every applied move has exactly one mover",
            );
            // §5.2's corpus half: an evaluator seat runs the HONEST chain, so
            // its record is ordinary training data and the containment rule must
            // not refuse it at an ordinary cell path. Checked through the pure
            // predicate — no I/O, nothing left in the repo's corpus directory.
            let path = crate::game::arena::corpus::corpus_path(
                &crate::game::arena::corpus::default_corpus_dir(),
                crate::game::semantics::SEMANTICS_REV,
                2,
                "t6-greedy-vs-fw-counters",
                stats.seed,
                arr,
            );
            assert!(
                crate::game::arena::corpus::corpus_refusal(&path, &stats.record).is_none(),
                "an evaluator seat plays the honest chain, so its record is corpus-admissible",
            );
            Ok(())
        })
        .unwrap_or_else(|e| panic!("the greedy cell failed: {e}"));

        assert_eq!(report.games, 2, "one 2P unit is the swapped pair");

        // -- §5.4: per-seat applied-move counts, both aggregations --------------
        assert_eq!(report.applied_by_seat.len(), 2, "one entry per seat");
        assert_eq!(report.applied_by_seat.iter().sum::<u64>(), total_moves);
        assert!(
            report.applied_by_seat.iter().all(|&n| n > 0),
            "a seat never moved: {:?}",
            report.applied_by_seat,
        );
        // X is ONE seat of two, so its count must be a strict part of the whole
        // — the check that `x_applied_moves` follows the contestant rather than
        // quietly being the cell total.
        assert!(
            report.x_applied_moves > 0 && report.x_applied_moves < total_moves,
            "x_applied_moves {} is not a strict part of the cell's {total_moves} moves",
            report.x_applied_moves,
        );

        // -- §0.3(b): the identity ---------------------------------------------
        assert_eq!(
            report.x_explanations,
            Some(report.x_applied_moves),
            "Greedy-v1 played a move it did not explain (or explained one it did not play)",
        );

        // -- the cap counters: a one-ply argmax has NO search, so `None` -------
        // The three-way distinction the Options carry: no explanation path
        // (rung), an explanation path but no search (this cell), and a search
        // whose guards did not fire (the lookahead cell below).
        assert_eq!(
            (report.x_ply_cap_hits, report.x_free_cap_hits),
            (None, None),
            "Greedy-v1 has no continuation to truncate, and a Some(0) would claim it searched",
        );

        // -- §5.2: provenance, from the controllers' own descriptors ------------
        let greedy_seats: Vec<usize> = per_game
            .iter()
            .flat_map(|seats| seats.iter().enumerate())
            .filter(|(_, (name, _, _))| name == GREEDY_CONTROLLER)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            greedy_seats,
            vec![0, 1],
            "X must sit at each seat exactly once across the pair, and be labelled greedy at both",
        );
        for seats in &per_game {
            for (name, version, observation) in seats {
                assert_eq!(observation, "view", "every arena seat here plays the honest chain");
                let expected = if name == GREEDY_CONTROLLER {
                    GREEDY_IMPL_VERSION
                } else {
                    assert_eq!(name, "family-weighted", "unexpected controller {name:?}");
                    crate::game::policy::POLICY_IMPL_VERSION
                };
                assert_eq!(*version, expected, "{name} reported the wrong impl version");
            }
        }

        // -- §5.5: a mixed-Keep cell says so in its own echo --------------------
        assert!(report.spec_echo.contains("setup-identical=no"), "{}", report.spec_echo);
        assert!(
            report.spec_echo.contains(&format!("x={GREEDY_CONTROLLER}@v{GREEDY_IMPL_VERSION}{{")),
            "the echo must name X's controller and its knob group: {}",
            report.spec_echo,
        );

        // -- the `None` side of the Option, on a rung X -------------------------
        let rung_x = CellSpec {
            name: "t6-rung-x-explains-nothing".to_string(),
            x: Contestant::Policy(RolloutPolicy::RetreatAverse),
            ..spec.clone()
        };
        let rung_report = run_cell(&rung_x, &rules).unwrap_or_else(|e| panic!("rung cell: {e}"));
        assert_eq!(
            rung_report.x_explanations, None,
            "a rollout rung has no explanation path, which is not the same fact as explaining zero",
        );
        assert_eq!(
            (rung_report.x_ply_cap_hits, rung_report.x_free_cap_hits),
            (None, None),
            "a rung produces no handle at all, so there is nothing to read a cap counter off",
        );
        assert!(rung_report.x_applied_moves > 0, "the rung still played");
        assert!(rung_report.spec_echo.contains("setup-identical=yes"), "{}", rung_report.spec_echo);
    }

    /// §5.2 / §0.5: every contestant's name, version and knobs come from THAT
    /// controller's own descriptor, and the per-controller version consts are
    /// distinct where the controllers are.
    ///
    /// Literals on the right-hand side, not `descriptor()` on both — comparing a
    /// call against itself would pin nothing. These strings are the WIRE FORMAT a
    /// corpus reader matches on.
    #[test]
    fn every_contestant_reports_its_own_controllers_descriptor() {
        use crate::game::lookahead::{
            ATOMIC_DIAG_CONTROLLER, ATOMIC_DIAG_IMPL_VERSION, LOOKAHEAD_CONTROLLER,
            LOOKAHEAD_IMPL_VERSION,
        };

        let (name, version, params) =
            Contestant::Policy(RolloutPolicy::RetreatAverse).descriptor();
        assert_eq!((name.as_str(), version), ("retreat-averse", POLICY_IMPL_VERSION));
        assert!(params.is_empty(), "a knobless rung reports no knobs");

        let (name, version, params) = Contestant::GREEDY_V1_DEFAULT.descriptor();
        assert_eq!((name.as_str(), version), (GREEDY_CONTROLLER, GREEDY_IMPL_VERSION));
        assert_eq!(params["m"], i64::from(DEFAULT_RETREAT_SAMPLES));
        assert!(params.contains_key("eval_impl") && params.contains_key("weights"));

        let (name, version, params) = Contestant::LOOKAHEAD_V1_DEFAULT.descriptor();
        assert_eq!((name.as_str(), version), (LOOKAHEAD_CONTROLLER, LOOKAHEAD_IMPL_VERSION));
        assert_eq!(params["b"], i64::from(DEFAULT_BEAM));
        assert_eq!(params["k"], i64::from(DEFAULT_WORLDS));
        assert!(!params.contains_key("d"), "S1 carries no atomic depth");

        let (name, version, params) = Contestant::ATOMIC_DIAG_DEFAULT.descriptor();
        assert_eq!((name.as_str(), version), (ATOMIC_DIAG_CONTROLLER, ATOMIC_DIAG_IMPL_VERSION));
        assert_eq!(params["d"], i64::from(DEFAULT_ATOMIC_DEPTH));
        assert!(!params.contains_key("b"), "S2 carries no beam");

        // §0.5's whole point: the two search schemes are separately versionable,
        // and at HEAD they have in fact moved apart.
        assert_ne!(
            ATOMIC_DIAG_IMPL_VERSION, LOOKAHEAD_IMPL_VERSION,
            "the per-controller consts are separate so they CAN differ; if they \
             ever coincide again, this pin should be re-read, not deleted",
        );

        // `with_knobs` raises K to 1, so a descriptor that printed the raw 0
        // would name knobs the seat did not play on.
        let (_, _, params) = Contestant::LookaheadV1 { b: 2, k: 0, m: 3, cap: None }.descriptor();
        assert_eq!(params["k"], 1, "the descriptor must report the knobs the agent will run");

        // The agent an evaluator contestant BUILDS reports the same descriptor,
        // with the stream seed filled in — the drift this routing exists to
        // prevent (a cell echo that says one thing and a record that says
        // another).
        let rules = Rules::standard();
        let (agent, handle) = Contestant::GREEDY_V1_DEFAULT.build(&rules, 0xABCD);
        let built = agent.controller();
        let declared = Contestant::GREEDY_V1_DEFAULT.provenance(Some(0xABCD));
        assert_eq!(built, declared, "a built agent and its contestant disagreed about what it is");
        assert_eq!(built.agent_seed, Some(0xABCD));
        assert_eq!(handle.expect("an evaluator hands back a tally").explained(), 0);
        assert!(
            Contestant::Policy(RolloutPolicy::Uniform).build(&rules, 1).1.is_none(),
            "a rung has no explanation path to hand back",
        );
    }

    /// **Spec §3.6 at the SEAM**: the 3.4 contestants' descriptors, and the
    /// absent-optional-knob rule that keeps the shipped S1 rung's echo unchanged
    /// by the cap knob's existence.
    ///
    /// The absent-cap clause is what the committed goldens rest on: they were
    /// captured on a tree where `Contestant::LookaheadV1` had no `cap` field at
    /// all, and they stay green only because `LOOKAHEAD_V1_DEFAULT` is `None`
    /// AND `None` emits no token. Both halves are asserted here, so a future
    /// edit that defaults the knob to `Some(..)` fails HERE — naming the cause —
    /// rather than only in a golden diff that says "the arm moved".
    #[test]
    fn the_mcts_contestant_reports_its_descriptor_and_an_absent_cap_emits_no_token() {
        use crate::game::mcts::{MCTS_CONTROLLER, MCTS_IMPL_VERSION};

        // -- the shipped S1 rung is UNCAPPED, and says nothing about a cap -----
        let (_, _, params) = Contestant::LOOKAHEAD_V1_DEFAULT.descriptor();
        assert!(
            !params.contains_key("cap"),
            "the shipped rung's echo gained a cap token — every 3.3 figure and every \
             committed golden describes the descriptor WITHOUT one",
        );
        let (_, _, capped) = Contestant::LookaheadV1 {
            b: DEFAULT_BEAM,
            k: DEFAULT_WORLDS,
            m: DEFAULT_RETREAT_SAMPLES,
            cap: Some(750),
        }
        .descriptor();
        assert_eq!(capped["cap"], 750, "a capped S1 seat must say so");
        assert_eq!(
            capped.len(),
            params.len() + 1,
            "the cap knob moved something other than the cap token",
        );

        // -- MCTS-v1's own descriptor -----------------------------------------
        let (name, version, params) = Contestant::MCTS_V1_DEFAULT.descriptor();
        assert_eq!((name.as_str(), version), (MCTS_CONTROLLER, MCTS_IMPL_VERSION));
        assert_eq!(params["c"], i64::from(DEFAULT_EXPLORATION_PPM));
        assert_eq!(params["scale"], DEFAULT_SQUASH_SCALE);
        assert_eq!(params["m"], i64::from(DEFAULT_RETREAT_SAMPLES));
        assert_eq!(params["leaf"], LeafRule::HandOff.code());
        assert_eq!(params["sims"], i64::from(mcts::DEFAULT_SIMULATIONS));
        assert!(!params.contains_key("cap"), "the knobless spelling is uncapped");
        assert!(!params.contains_key("b") && !params.contains_key("d"), "a tree has no beam");

        // The SIZING RULE at the seam (spec §5's `mcts-v1@<cap>` cells): naming a
        // cap raises the simulation count to the ceiling, so the budget is the
        // only bound and no report has to say which of two bounds bit first.
        let sized = Contestant::MctsV1 {
            cap: Some(750),
            c_ppm: DEFAULT_EXPLORATION_PPM,
            scale: DEFAULT_SQUASH_SCALE,
            m: DEFAULT_RETREAT_SAMPLES,
            leaf: LeafRule::HandOff,
        };
        let (_, _, params) = sized.descriptor();
        assert_eq!(params["cap"], 750);
        assert!(!params.contains_key("sims"), "a capped seat emits no sims token (T5 review S1)");

        // The agent it BUILDS reports the same thing, stream seed filled in —
        // the drift this routing exists to prevent.
        let rules = Rules::standard();
        let (agent, handle) = sized.build(&rules, 0x5EED);
        assert_eq!(
            agent.controller(),
            sized.provenance(Some(0x5EED)),
            "a built mcts agent and its contestant disagreed about what it is",
        );
        let handle = handle.expect("an mcts contestant hands back a tally");
        assert_eq!(handle.explained(), 0, "nothing has been decided yet");
        // The `Option` SHAPE of the new meters on an unrun handle: present where
        // the contestant has the quantity, `None` where it does not.
        assert_eq!(handle.candidates_scored(), Some(0));
        assert_eq!(handle.root_scored(), Some(0));
        assert_eq!(handle.budget_binds(), Some(0));
        assert_eq!(handle.budget_starved_decisions(), Some(0));
        assert_eq!(handle.ply_cap_hits(), Some(0), "the completion guard is the ply analogue");
        assert_eq!(handle.free_cap_hits(), None, "a tree has no quiescence arm to guard");

        // And MCTS-v1 is Keep-homogeneous with the evaluator arms, which is what
        // makes every §5 cell setup-identical.
        assert_eq!(Contestant::MCTS_V1_DEFAULT.keep_kind(), KeepKind::EvaluatorScored);
        let echo = spec_echo(
            &CellSpec {
                name: "keep".to_string(),
                players: 2,
                x: Contestant::MCTS_V1_DEFAULT,
                y: Contestant::LOOKAHEAD_V1_DEFAULT,
                units: 1,
                master: SeedScheduleKind::Diagnostic(MASTER),
            },
            MASTER,
            0,
            None,
        );
        assert!(echo.contains("setup-identical=yes"), "{echo}");
        assert!(echo.contains(&format!("x={MCTS_CONTROLLER}@v{MCTS_IMPL_VERSION}{{")), "{echo}");
    }

    /// **[`Contestant::from_parts`] is the INVERSE of
    /// [`Contestant::descriptor`]** (W5 design §1.5) — the property the CLI's
    /// resume path rests on, since a recorded seat stores exactly those two
    /// halves and reparsing the bare NAME would silently reset every knob (a
    /// recorded `lookahead-v1` at `b=8` would resume at `b=4`).
    ///
    /// Round-tripped over every arm AND over knob-presence: BOTH cap-carrying
    /// contestants appear once uncapped and once capped, because absence IS the
    /// uncapped identity on the provenance preserve-list — and the capped tree
    /// is additionally the one descriptor shape where `cap` is present while
    /// the derived `sims` is absent, so it pins `MCTS_DERIVED_PARAM_KEYS`
    /// behaviour nothing else reaches (T1 review SHOULD-1).
    #[test]
    fn from_parts_round_trips_every_descriptor() {
        let capped_mcts = {
            let Contestant::MctsV1 { c_ppm, scale, m, leaf, .. } = Contestant::MCTS_V1_DEFAULT
            else {
                unreachable!("MCTS_V1_DEFAULT is the MctsV1 arm");
            };
            Contestant::MctsV1 { cap: Some(750), c_ppm, scale, m, leaf }
        };
        for contestant in [
            Contestant::Policy(RolloutPolicy::Uniform),
            Contestant::Policy(RolloutPolicy::RetreatAverse),
            Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
            Contestant::Policy(RolloutPolicy::FamilyWeighted {
                retreat_ppm: 50_000,
                recovery_min_of: 5,
            }),
            Contestant::GREEDY_V1_DEFAULT,
            Contestant::LOOKAHEAD_V1_DEFAULT,
            Contestant::LookaheadV1 { b: 8, k: 1, m: 8, cap: Some(750) },
            Contestant::ATOMIC_DIAG_DEFAULT,
            Contestant::MCTS_V1_DEFAULT,
            capped_mcts,
        ] {
            let (name, _version, params) = contestant.descriptor();
            assert_eq!(
                Contestant::from_parts(&name, &params),
                Ok(contestant),
                "the descriptor of {contestant:?} did not invert",
            );
        }
    }

    /// The preserve-list clause made executable on BOTH sides of the shared
    /// scored-successor budget: an absent `cap` key is not any set cap, and it
    /// survives the round trip as `None` rather than as some default number.
    #[test]
    fn from_parts_absent_cap_is_uncapped_and_present_cap_is_capped() {
        let (name, _, params) = Contestant::LOOKAHEAD_V1_DEFAULT.descriptor();
        assert!(!params.contains_key("cap"), "the shipped S1 rung emits no cap token");
        assert_eq!(
            Contestant::from_parts(&name, &params),
            Ok(Contestant::LookaheadV1 {
                b: DEFAULT_BEAM,
                k: DEFAULT_WORLDS,
                m: DEFAULT_RETREAT_SAMPLES,
                cap: None,
            }),
        );

        let capped = Contestant::LookaheadV1 {
            b: DEFAULT_BEAM,
            k: DEFAULT_WORLDS,
            m: DEFAULT_RETREAT_SAMPLES,
            cap: Some(750),
        };
        let (name, _, params) = capped.descriptor();
        assert_eq!(params.get("cap"), Some(&750_i64));
        assert_eq!(Contestant::from_parts(&name, &params), Ok(capped));

        let (name, _, params) = Contestant::MCTS_V1_DEFAULT.descriptor();
        assert!(!params.contains_key("cap"), "the knobless tree spelling is uncapped");
        assert!(matches!(
            Contestant::from_parts(&name, &params),
            Ok(Contestant::MctsV1 { cap: None, .. }),
        ));

        // The capped tree, full equality — a recorded `mcts-v1@<cap>` seat that
        // silently resumed uncapped would be a different contestant under the
        // same name (T1 review SHOULD-1's escaped mutation).
        let capped_mcts = {
            let Contestant::MctsV1 { c_ppm, scale, m, leaf, .. } = Contestant::MCTS_V1_DEFAULT
            else {
                unreachable!("MCTS_V1_DEFAULT is the MctsV1 arm");
            };
            Contestant::MctsV1 { cap: Some(750), c_ppm, scale, m, leaf }
        };
        let (name, _, params) = capped_mcts.descriptor();
        assert_eq!(params.get("cap"), Some(&750_i64));
        assert_eq!(Contestant::from_parts(&name, &params), Ok(capped_mcts));
    }

    /// Unknown is an error, and the error names what it saw — the grammar's
    /// contract, adopted here from birth.
    ///
    /// The short runner alias `atomic-diag-v1` is deliberately among the
    /// REFUSED names: this function inverts a descriptor, and no descriptor ever
    /// emits that spelling (`ATOMIC_DIAG_CONTROLLER` is the wire name).
    #[test]
    fn from_parts_refuses_an_unknown_name_naming_it() {
        let params = BTreeMap::new();
        for name in ["", "human", "bga-human", "greedy-v2", "atomic-diag-v1", "UNIFORM"] {
            let message = Contestant::from_parts(name, &params)
                .expect_err("an unknown controller name must be refused");
            assert!(
                message.contains(&format!("{name:?}")),
                "the refusal of {name:?} did not name it: {message}",
            );
        }
    }

    /// A key no arm knows is an error rather than a silent ignore — a swallowed
    /// param would resume a seat under knobs the record did not name.
    #[test]
    fn from_parts_refuses_an_unknown_key_naming_it() {
        let (name, _, mut params) = Contestant::LOOKAHEAD_V1_DEFAULT.descriptor();
        params.insert("zzz".to_string(), 1);
        let message =
            Contestant::from_parts(&name, &params).expect_err("an unknown param must be refused");
        assert!(message.contains("zzz"), "the refusal did not name the key: {message}");
        assert!(message.contains(&name), "the refusal did not name the contestant: {message}");

        // A construction knob belonging to ANOTHER arm is still unknown here.
        let (name, _, mut params) = Contestant::GREEDY_V1_DEFAULT.descriptor();
        params.insert("b".to_string(), 4);
        let message =
            Contestant::from_parts(&name, &params).expect_err("a one-ply argmax has no beam");
        assert!(message.contains("\"b\""), "the refusal did not name the key: {message}");
    }

    /// A value outside the agent's own type (or outside the grammar's stated
    /// range) is refused, not truncated.
    #[test]
    fn from_parts_refuses_an_out_of_range_value() {
        let (name, _, mut params) = Contestant::GREEDY_V1_DEFAULT.descriptor();
        params.insert("m".to_string(), 300);
        let message =
            Contestant::from_parts(&name, &params).expect_err("300 does not fit the agent's u8");
        assert!(message.contains("300"), "the refusal did not name the value: {message}");
        assert!(message.contains('m'), "the refusal did not name the key: {message}");

        // `cap=0` carries the GRAMMAR's own refusal text, single-sourced: the two
        // surfaces must not disagree about what a zero budget means.
        let (name, _, mut params) = Contestant::LOOKAHEAD_V1_DEFAULT.descriptor();
        params.insert("cap".to_string(), 0);
        let message = Contestant::from_parts(&name, &params).expect_err("cap=0 must be refused");
        assert!(message.contains("omit the knob"), "{message}");

        // A negative count is not a count.
        let (name, _, mut params) = Contestant::LOOKAHEAD_V1_DEFAULT.descriptor();
        params.insert("b".to_string(), -1);
        assert!(
            Contestant::from_parts(&name, &params).is_err(),
            "a negative beam width was accepted",
        );
    }

    /// The descriptor carries IDENTITY keys beyond the construction knobs —
    /// `eval_impl`, the `weights` table digest, greedy's `placement_cap`. They
    /// are ACCEPTED here and never checked: design §1.5 puts the checking in the
    /// CALLER's field-by-field comparison against a freshly-derived descriptor,
    /// which warns loudly per differing field and never refuses, so a record
    /// from another build still loads and still plays.
    #[test]
    fn from_parts_accepts_identity_keys_without_consuming_them() {
        let (name, _, params) = Contestant::LOOKAHEAD_V1_DEFAULT.descriptor();
        assert!(params.contains_key("eval_impl") && params.contains_key("weights"));
        let (name_g, _, params_g) = Contestant::GREEDY_V1_DEFAULT.descriptor();
        assert!(params_g.contains_key("placement_cap"), "greedy's third identity key");
        assert_eq!(Contestant::from_parts(&name, &params), Ok(Contestant::LOOKAHEAD_V1_DEFAULT));
        assert_eq!(Contestant::from_parts(&name_g, &params_g), Ok(Contestant::GREEDY_V1_DEFAULT));

        // The skip clause ALTERED on every arm that emits identity keys — a
        // single-arm pin lets the other arms start CHECKING a key undetected,
        // which is exactly the cross-build unloadability `IDENTITY_PARAM_KEYS`'
        // doc forbids (T1 review SHOULD-2's escaped mutation).
        for contestant in [
            Contestant::GREEDY_V1_DEFAULT,
            Contestant::LOOKAHEAD_V1_DEFAULT,
            Contestant::ATOMIC_DIAG_DEFAULT,
            Contestant::MCTS_V1_DEFAULT,
        ] {
            let (name, _, params) = contestant.descriptor();
            let mut altered = params.clone();
            for key in IDENTITY_PARAM_KEYS {
                if altered.contains_key(*key) {
                    altered.insert((*key).to_string(), 0x0BAD_0BAD);
                }
            }
            assert_ne!(altered, params, "the alteration must actually move a key on {name}");
            assert_eq!(
                Contestant::from_parts(&name, &altered),
                Ok(contestant),
                "an identity key was consumed as a construction input on {name}",
            );
        }
    }

    /// The echo composer is a PUBLIC method on the type (W5 design §4.2) — the
    /// CLI's wizard summary, `why` header and resume warnings all need it, and a
    /// second spelling in the CLI is exactly the drift the W1 grammar promotion
    /// exists to prevent.
    ///
    /// The literals are what the module-private `contestant_echo` produced at
    /// HEAD `a61d642`, captured BEFORE the body moved onto the type. Both format
    /// branches are covered: knobless, and knob-bearing.
    #[test]
    fn the_echo_is_public_and_matches_the_spec_echo_format() {
        // The `weights=` identity moved ONCE at the V4 step-8 padding (the
        // table hashes 68 numbers instead of 32): -7339724794018772210 was the
        // sixteen-slot V2, -1160323518167071950 is the padded table whose
        // sixteen live slots are byte-identical to it.
        assert_eq!(
            Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
            "lookahead-v1@v2{b=4,eval_impl=3,k=1,m=8,weights=-1160323518167071950}",
        );
        assert_eq!(Contestant::Policy(RolloutPolicy::Uniform).echo(), "uniform@v1");

        // …and the cell echo is composed from that same method, so the report
        // header and a CLI header cannot disagree about what a seat is.
        let echo = spec_echo(
            &CellSpec {
                name: "echo".to_string(),
                players: 2,
                x: Contestant::LOOKAHEAD_V1_DEFAULT,
                y: Contestant::Policy(RolloutPolicy::Uniform),
                units: 1,
                master: SeedScheduleKind::Diagnostic(MASTER),
            },
            MASTER,
            0,
            None,
        );
        assert!(
            echo.contains(&format!("x={}", Contestant::LOOKAHEAD_V1_DEFAULT.echo())),
            "{echo}",
        );
        assert!(
            echo.contains(&format!("y={}", Contestant::Policy(RolloutPolicy::Uniform).echo())),
            "{echo}",
        );
    }

    /// §5.5: the echo states whether the cell's two contestants resolve a deal
    /// the same way, so a reader of a quoted figure knows whether its units were
    /// setup-identical or merely swapped-balanced.
    #[test]
    fn the_spec_echo_states_whether_the_cell_is_setup_identical() {
        let homogeneous = CellSpec {
            name: "keep-homogeneous".to_string(),
            players: 2,
            x: Contestant::Policy(RolloutPolicy::Uniform),
            y: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
            units: 4,
            master: SeedScheduleKind::Diagnostic(MASTER),
        };
        let echo = spec_echo(&homogeneous, MASTER, 0, None);
        assert!(echo.contains("setup-identical=yes"), "{echo}");

        let mixed = CellSpec { x: Contestant::GREEDY_V1_DEFAULT, ..homogeneous.clone() };
        let echo = spec_echo(&mixed, MASTER, 0, None);
        assert!(echo.contains("setup-identical=no"), "{echo}");
    }

    /// The bootstrap endpoints are order statistics under the crate's ONE
    /// nearest-rank rule, and the resample seed is the only thing that moves
    /// them.
    #[test]
    fn bootstrap_endpoints_are_nearest_rank_order_statistics() {
        let units: Vec<f64> = (0..100).map(|i| f64::from(i) / 99.0).collect();

        let (lo, hi) = bootstrap_percentile(&units, 1_000, 42);
        assert!(lo < hi);
        // The replicate means concentrate around the sample mean, so both
        // endpoints sit strictly inside the data's range.
        let m = mean(&units);
        assert!(lo < m && m < hi, "[{lo}, {hi}] does not bracket the sample mean {m}");

        // Deterministic in the seed, and genuinely driven by it.
        assert_eq!(bootstrap_percentile(&units, 1_000, 42), (lo, hi));
        assert_ne!(bootstrap_percentile(&units, 1_000, 43), (lo, hi));

        // More replicates do not shift the interval materially; the ENDPOINT
        // RULE is what this pins, and it is the same rule the quantile rows use.
        assert_eq!(nearest_rank_index(10_000, 25, 1000), 249);
        assert_eq!(nearest_rank_index(10_000, 975, 1000), 9_749);
        assert_eq!(nearest_rank_index(200, 25, 1000), 4);
        assert_eq!(nearest_rank_index(200, 975, 1000), 194);
    }

    /// The unit-t cross-check's quantile, against the published table.
    #[test]
    fn the_printed_t_quantile_matches_the_table() {
        for (df, table) in [(10u64, 2.2281), (30, 2.0423), (39, 2.0227), (199, 1.9720)] {
            let got = student_t_975(df);
            assert!(
                (got - table).abs() < 1e-3,
                "t(0.975, {df}) = {got}, table says {table}",
            );
        }
        // The limit is the normal quantile the crate already owns. The residual
        // at df = 1e6 is the expansion's leading (z^3+z)/(4*df) term, 2.4e-6 —
        // so the tolerance is 1e-5, not 1e-6.
        assert!((student_t_975(1_000_000) - Z_95).abs() < 1e-5);
        assert!(student_t_975(1_000_000) > Z_95, "t is always wider than z");
        assert!(student_t_975(0).is_infinite(), "one unit has no finite t quantile");
    }

    /// A 4P cell is a rotation block: X sits in every seat exactly once, and the
    /// report's null reference says 0.25 rather than silently reusing 0.5.
    #[test]
    fn a_four_player_cell_is_a_rotation_block_against_the_quarter_null() {
        let seed = crate::game::seeds::WorldSeed(0x1234_5678_9ABC_DEF0);
        let unit = unit_for(4, seed);
        assert_eq!(unit.seatings.len(), 4, "a 4P unit is a four-game rotation block");

        let x_seats: Vec<usize> = unit
            .seatings
            .iter()
            .map(|s| s.iter().position(|&slot| slot == X_SLOT).unwrap())
            .collect();
        let mut sorted = x_seats.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![0, 1, 2, 3], "X must occupy each seat exactly once: {x_seats:?}");

        // Every other seat in every arrangement is a Y seat — the
        // single-champion shape the 1/players null belongs to.
        for seating in &unit.seatings {
            assert_eq!(seating.iter().filter(|&&slot| slot == X_SLOT).count(), 1);
        }

        let rows = cross_checks(
            &[0.25, 0.5, 0.0, 0.25],
            0.25,
            4,
            16,
            4,
            0,
            0,
            &IntervalOutcome::Bootstrap { lo: 0.1, hi: 0.4, b: 10, resample_seed: 0 },
        );
        assert!(rows[0].contains("1/4 = 0.250000"), "{}", rows[0]);
        assert!(rows[0].contains("3 Y seats"), "{}", rows[0]);
    }

    /// A cell that cannot be tabulated is a typed error, never an empty report.
    ///
    /// Both arms would otherwise fail SILENTLY rather than loudly: zero units
    /// and an unsupported player count each produce zero games, and a report
    /// built from zero games is a full set of NaNs that no downstream
    /// containment check would reject. Task 9 wires these entry points to argv,
    /// where both are one typo away.
    #[test]
    fn an_untabulatable_cell_is_refused_before_any_game() {
        let rules = Rules::empty_for_tests();
        let base = CellSpec {
            name: "empty".to_string(),
            players: 2,
            x: Contestant::Policy(RolloutPolicy::Uniform),
            y: Contestant::Policy(RolloutPolicy::Uniform),
            units: 0,
            master: SeedScheduleKind::Diagnostic(MASTER),
        };
        match run_cell(&base, &rules) {
            Err(ArenaError::Driver(message)) => assert!(message.contains("0 units"), "{message}"),
            other => panic!("expected a typed driver error, got {other:?}"),
        }

        for players in [0u8, 1, 5] {
            let spec = CellSpec { players, units: 4, ..base.clone() };
            match run_cell(&spec, &rules) {
                Err(ArenaError::Driver(message)) => {
                    assert!(message.contains("unsupported player count"), "{message}");
                }
                other => panic!("{players}P should be refused, got {other:?}"),
            }
        }
    }

    /// Task 8: a cell's games know which RUN produced them.
    ///
    /// The per-seat half of provenance is stamped in the driver, which knows the
    /// policies but not the schedule above it — so `master_seed` and `run_id`
    /// are the cell's to fill, and a corpus file is self-describing only if it
    /// does. Pinned through the emission sink because that is the only place a
    /// cell's records are observable: `CellReport` deliberately does not carry
    /// them (400 games of event log is not a report).
    #[test]
    fn cell_games_carry_the_master_and_the_cell_name() {
        let rules = Rules::standard();
        let spec = CellSpec {
            name: "ra-vs-fw-provenance".to_string(),
            players: 2,
            x: Contestant::Policy(RolloutPolicy::RetreatAverse),
            y: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
            units: 2,
            master: SeedScheduleKind::Diagnostic(MASTER),
        };

        /// What the sink saw for one game.
        struct Seen {
            master_seed: Option<u64>,
            run_id: Option<String>,
            controllers: Vec<String>,
            arr: u8,
        }

        let mut seen: Vec<Seen> = Vec::new();
        let report = run_cell_emitting(&spec, &rules, true, &mut |stats, arr| {
            let run = stats.record.run.as_ref().expect("a cell's records carry provenance");
            seen.push(Seen {
                master_seed: run.master_seed,
                run_id: run.run_id.clone(),
                controllers: run.seats.iter().map(|s| s.controller.clone()).collect(),
                arr,
            });
            Ok(())
        })
        .unwrap_or_else(|e| panic!("cell failed: {e}"));

        assert_eq!(report.games, 4, "2 pairs is 4 games");
        assert_eq!(seen.len(), 4, "the sink must see every completed game");
        for game in &seen {
            assert_eq!(game.master_seed, Some(MASTER), "a cell game must name its master");
            assert_eq!(game.run_id.as_deref(), Some("ra-vs-fw-provenance"), "...and its cell");
        }
        // Arrangement indices run 0..seatings within each unit, which is what
        // `corpus_path` files on — two units of two seatings each.
        assert_eq!(seen.iter().map(|g| g.arr).collect::<Vec<_>>(), vec![0, 1, 0, 1]);
        // X rotates through the seats, so the per-seat controller vector must
        // SWAP between the two games of a pair. A run-level field could not say
        // this, which is why provenance is per seat.
        assert_eq!(seen[0].controllers, ["retreat-averse", "family-weighted"]);
        assert_eq!(seen[1].controllers, ["family-weighted", "retreat-averse"]);

        // The sink is a seam, not a semantic change: `run_cell` is this function
        // with a sink that does nothing.
        let plain = run_cell(&spec, &rules).unwrap();
        assert_eq!(
            serde_json::to_string(&plain).unwrap(),
            serde_json::to_string(&report).unwrap(),
            "emitting changed the report",
        );
    }

    /// T6-review should-fix 2: the §0.3(b) cell-level identity was tested only
    /// for the GREEDY arm while spec §0.1's contestant under test is
    /// Lookahead-v1 — `build_search` and the `ExplainHandle::Search` arm ran
    /// under no committed test. Same shape as the greedy cell above at
    /// deliberately SMALL knobs (b=1, m=2: the identity is about the tally
    /// path, not strength, and a B=4 lookahead game is ~14x dearer in debug).
    /// Covers S1; S2 shares `build_search` and the same tally type.
    #[test]
    fn a_lookahead_x_cell_counts_and_labels_every_move_it_explained() {
        let rules = Rules::standard();
        let spec = CellSpec {
            name: "t6r-lookahead-vs-fw-counters".to_string(),
            players: 2,
            x: Contestant::LookaheadV1 { b: 1, k: 1, m: 2, cap: None },
            y: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
            units: 1,
            master: SeedScheduleKind::Diagnostic(MASTER),
        };
        let report = run_cell(&spec, &rules).expect("the lookahead cell runs");
        assert_eq!(
            report.x_explanations,
            Some(report.x_applied_moves),
            "every move the search rung applied must carry an explanation \
             (§0.3(b) at cell level, S1 arm)",
        );
        assert!(report.x_applied_moves > 0, "the cell must actually have played");
        // The SEARCHING side of the same Options (six-lens review): a search
        // contestant reports `Some`, and the guards are expected not to fire.
        // The free counter is `Some(0)` structurally here — S1 never enters the
        // quiescence arm — so what this pins is the plumbing, not S2's guard.
        assert_eq!(
            report.x_ply_cap_hits,
            Some(0),
            "MAX_TURN_PLIES stopped a continuation, so some move was explained mid-turn",
        );
        assert_eq!(report.x_free_cap_hits, Some(0), "S1 cannot reach the quiescence guard at all");
        assert!(
            report.spec_echo.contains("x=lookahead-v1@v2{"),
            "the X seat's provenance must be the search rung's descriptor: {}",
            report.spec_echo,
        );
    }

    /// **The T5 cell pin: spec §0.3(b)'s identity for an MCTS X, and spec §6's
    /// `x_*`/`y_*` budget meters on BOTH sides of one cell.**
    ///
    /// One cell, because the two claims are two readings of the same two games
    /// and the games are what the test costs (MEASURED **~1.3 s debug** at HEAD
    /// for the pair).
    ///
    /// KNOBS, AND WHY THESE. Both sides are capped small so a per-edit pin can
    /// afford a whole 2P pair, and the meters still have content at that size:
    /// a small cap makes the root pass a large share, so `root_scored` is a
    /// strict positive fraction of `candidates_scored` on both sides rather than
    /// a degenerate 0 or 1. Y's cap is **16 and deliberately not 32–128**: spec
    /// §5 records a MEASURED non-terminating 2P band there for the capped S1
    /// arm, and a per-edit test sitting in it would hang for a reason the
    /// experiment's completion-failure surface already owns. `b=1`/`m=2` are the
    /// lookahead cell's own reasoning above — the identity is about the tally
    /// path, not about strength.
    ///
    /// WHAT IS NOT ASSERTED: any direction, any rate, and the WALL. The wall is
    /// not on the report at all (see [`CellWall`]) and is load-sensitive; this
    /// test reads it only to check the driver actually clocked BOTH sides, which
    /// is a wiring fact and not a figure.
    #[test]
    fn an_mcts_x_cell_counts_every_move_it_explained_and_meters_both_sides() {
        let rules = Rules::standard();
        let spec = CellSpec {
            name: "t5-mcts-vs-capped-s1-counters".to_string(),
            players: 2,
            x: Contestant::MctsV1 {
                cap: Some(24),
                c_ppm: DEFAULT_EXPLORATION_PPM,
                scale: DEFAULT_SQUASH_SCALE,
                m: 2,
                leaf: LeafRule::HandOff,
            },
            y: Contestant::LookaheadV1 { b: 1, k: 1, m: 2, cap: Some(16) },
            units: 1,
            master: SeedScheduleKind::Diagnostic(MASTER),
        };
        let (report, wall) = run_cell_emitting_timed(&spec, &rules, true, &mut |_s, _a| Ok(()))
            .unwrap_or_else(|e| panic!("the mcts cell failed: {e}"));

        // -- §0.3(b): the identity, on the new contestant ----------------------
        assert!(report.x_applied_moves > 0, "the cell must actually have played");
        assert_eq!(
            report.x_explanations,
            Some(report.x_applied_moves),
            "MCTS-v1 played a move it did not explain (or explained one it did not play)",
        );
        // The cap counters' three-way distinction, on the arm that answers
        // (Some, None): the tree HAS a ply guard (its completions') and has no
        // quiescence arm at all.
        assert_eq!(report.x_ply_cap_hits, Some(0), "a stopped completion is valued mid-turn");
        assert_eq!(report.x_free_cap_hits, None, "a tree cannot reach a quiescence guard");

        // -- §6: the budget meters, X side -------------------------------------
        let x_scored = report.x_candidates_scored.expect("an mcts X meters its spend");
        let x_root = report.x_root_scored.expect("and its root share");
        assert!(x_scored > 0, "the currency meter counted nothing");
        assert!(
            x_root > 0 && x_root < x_scored,
            "the mandatory root pass must be a real fraction of a real total ({x_root}/{x_scored})",
        );
        assert!(
            report.x_budget_binds.expect("a capped X binds") > 0,
            "anti-vacuity: at cap 24 the budget must have denied work somewhere",
        );
        // Starvation is a SUB-CASE of binding, and the S2 surface reads it.
        let x_starved = report.x_budget_starved_decisions.expect("a capped X reports starvation");
        assert!(x_starved <= report.x_budget_binds.unwrap(), "starvation is a bind");

        // -- §6: the Y-SIDE MIRRORS (lens 1 M11 / lens 2 M2) -------------------
        // Without these the cell could not state its own matched-ness through
        // the seam it ran on — every pre-3.4 per-contestant counter is X-side.
        let y_decisions = report.applied_by_seat.iter().sum::<u64>() - report.x_applied_moves;
        assert!(y_decisions > 0, "the field must have played too");
        let y_scored = report.y_candidates_scored.expect("a capped S1 Y meters its spend");
        let y_root = report.y_root_scored.expect("and its root share");
        assert!(
            y_root > 0 && y_root < y_scored,
            "Y's root pass must be a real fraction of a real total ({y_root}/{y_scored})",
        );
        assert!(report.y_budget_binds.expect("a capped Y binds") > 0);
        assert!(
            report.y_budget_starved_decisions.expect("a capped Y reports starvation")
                <= report.y_budget_binds.unwrap(),
        );
        // The two sides are INDEPENDENT quantities, not one number written
        // twice — the exact defect a mirrored-field seam can ship silently.
        assert_ne!((x_scored, x_root), (y_scored, y_root), "the two sides read identically");

        // Structural companion (T5 review N2): meter PROVENANCE, not magnitude.
        // A Greedy X has no root/deeper split BY CONSTRUCTION while an MCTS Y
        // meters its own, so no coincidence of values can satisfy this the way
        // it theoretically could the assert_ne above.
        let greedy_x = CellSpec {
            name: "t5-greedy-x-mcts-y-provenance".to_string(),
            x: Contestant::GREEDY_V1_DEFAULT,
            y: spec.x,
            ..spec.clone()
        };
        let g = run_cell(&greedy_x, &rules).unwrap_or_else(|e| panic!("greedy-X cell: {e}"));
        assert!(g.x_root_scored.is_none(), "a one-ply argmax has no root/deeper split to meter");
        assert!(g.y_root_scored.is_some(), "while the MCTS field meters its own");

        // -- the `None` side of the new Options, on a rung Y --------------------
        let rung_y = CellSpec {
            name: "t5-mcts-vs-rung-meters".to_string(),
            y: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
            ..spec.clone()
        };
        let rung_report = run_cell(&rung_y, &rules).unwrap_or_else(|e| panic!("rung-Y cell: {e}"));
        assert_eq!(
            (
                rung_report.y_candidates_scored,
                rung_report.y_root_scored,
                rung_report.y_budget_binds,
                rung_report.y_budget_starved_decisions,
            ),
            (None, None, None, None),
            "a rollout rung has no budget at all, which is not the same fact as spending zero",
        );
        assert!(
            rung_report.x_candidates_scored.is_some(),
            "the X side must still meter when Y does not",
        );

        // -- the wall axis is WIRED for both sides (never a figure here) --------
        assert_eq!(wall.x.decisions, report.x_applied_moves, "X's clock missed decisions");
        assert_eq!(wall.y.decisions, y_decisions, "Y's clock missed decisions");
        assert!(wall.x.nanos > 0 && wall.y.nanos > 0, "a side was never clocked at all");
        println!(
            "t5 mcts cell: X {x_scored} scored ({x_root} root) over {} decisions at {:.1} us/dec · \
             Y {y_scored} ({y_root} root) over {y_decisions} at {:.1} us/dec — RECORDED, \
             load-sensitive, never asserted",
            report.x_applied_moves,
            wall.x.micros_per_decision(),
            wall.y.micros_per_decision(),
        );
    }

    /// The determinism pin, extended to the new contestant: an MCTS cell on a
    /// diagnostic master reproduces BYTE-IDENTICALLY, with the flipped-master
    /// anti-vacuity control the Policy pin above carries.
    ///
    /// It is a separate cell from that one because MCTS-v1 adds two whole new
    /// randomness surfaces to the report's dependency set — a per-decision keyed
    /// tie stream and a per-node candidate seed — and a keying defect in either
    /// would leave every Policy determinism test green.
    ///
    /// Y is a RUNG here rather than the capped S1 arm: this cell runs three
    /// times, and the claim under test is about X's reproducibility, so the
    /// cheaper opponent buys the same evidence.
    #[test]
    fn same_master_reproduces_an_mcts_cell_byte_identically() {
        let rules = Rules::standard();
        let spec = CellSpec {
            name: "t5-mcts-determinism".to_string(),
            players: 2,
            x: Contestant::MctsV1 {
                cap: Some(24),
                c_ppm: DEFAULT_EXPLORATION_PPM,
                scale: DEFAULT_SQUASH_SCALE,
                m: 2,
                leaf: LeafRule::HandOff,
            },
            y: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
            units: 1,
            master: SeedScheduleKind::Diagnostic(MASTER),
        };

        let first = serde_json::to_string(&run_cell(&spec, &rules).unwrap()).unwrap();
        let second = serde_json::to_string(&run_cell(&spec, &rules).unwrap()).unwrap();
        assert_eq!(first, second, "the same master produced two different mcts reports");
        assert!(
            first.contains("\"x_candidates_scored\":"),
            "the byte-identity claim must cover the new meters: {first}",
        );

        // ANTI-VACUITY: byte-identity is worthless if the report does not depend
        // on the master at all.
        let mut other = spec.clone();
        other.master = SeedScheduleKind::Diagnostic(MASTER ^ 0xFFFF_FFFF);
        let third = serde_json::to_string(&run_cell(&other, &rules).unwrap()).unwrap();
        assert_ne!(first, third, "the mcts report did not depend on the master");
    }

    /// T6-review should-fix 3: `KeepKind::EvaluatorScored` groups all three
    /// evaluator contestants on the CLAIM that they share one Keep function —
    /// and `setup-identical=yes` on an evaluator-vs-evaluator cell rests
    /// entirely on it. The rung side of the same claim is pinned at
    /// `arena.rs` over 72 (seat x deal) cases; this is the evaluator analog.
    /// Seeds differ MAXIMALLY so agreement cannot come from a shared stream —
    /// the Keep must be a function of the deal alone.
    #[test]
    fn every_evaluator_contestant_shares_one_keep_function() {
        let rules = Rules::standard();
        let (mut greedy, _) = Contestant::GREEDY_V1_DEFAULT.build(&rules, 7);
        let (mut s1, _) = Contestant::LOOKAHEAD_V1_DEFAULT.build(&rules, u64::MAX / 5);
        let (mut s2, _) = Contestant::ATOMIC_DIAG_DEFAULT.build(&rules, u64::MAX - 11);
        // The FOURTH EvaluatorScored contestant (3.4): every §5 experiment
        // cell's `setup-identical=yes` rests on this agreement holding for it
        // too — structurally unguarded until the six-lens review added it.
        let (mut mcts, _) = Contestant::MCTS_V1_DEFAULT.build(&rules, u64::MAX / 3);
        for seat in 0..4u8 {
            for card in 1..=18u8 {
                let deal = [OrderCardId(card), OrderCardId(card + 18)];
                let g = greedy.choose_starting_order(PlayerId(seat), &deal);
                let a = s1.choose_starting_order(PlayerId(seat), &deal);
                let b = s2.choose_starting_order(PlayerId(seat), &deal);
                let m = mcts.choose_starting_order(PlayerId(seat), &deal);
                assert!(
                    g == a && a == b && b == m,
                    "the four evaluator contestants' Keeps diverged at seat {seat}, \
                     deal {deal:?}: greedy {g}, s1 {a}, s2 {b}, mcts {m} — \
                     KeepKind::EvaluatorScored (and every setup-identical=yes it \
                     prints) rests on their agreement",
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // V4 §7 STEP 8 T3 — the named-identity arms (spec §13)
    // -----------------------------------------------------------------------

    /// The arms' knob literals equal the shipped default's TODAY — a
    /// TRANSCRIPTION FACT, not a coupling, exactly as
    /// `the_pinned_v2_seat_transcribes_todays_shipped_knobs` is for the
    /// generating ecology. If the shipped default legitimately moves later,
    /// DELETE this assertion in the moving commit; the literals stay, because
    /// an arm is an identity and figures already recorded against these names
    /// must not silently re-point.
    #[test]
    fn the_named_identity_arms_transcribe_todays_shipped_knobs() {
        assert_eq!(
            Contestant::LOOKAHEAD_V1_DEFAULT,
            Contestant::LookaheadV1 {
                b: NAMED_IDENTITY_BEAM,
                k: NAMED_IDENTITY_WORLDS,
                m: NAMED_IDENTITY_RETREAT_SAMPLES,
                cap: None,
            },
            "the named arms' knob literals transcribe the shipped default of 2026-08-24",
        );
    }

    /// **THE GOLDEN TRANSCRIPTION PIN.** All sixty-eight literals of
    /// [`WeightTable::V4_STEP9_DRAFT1`] against the draft-1 ledger row, written
    /// out here rather than computed, because the risk this wave carries is a
    /// MIS-TRANSCRIPTION and a pin that derived the numbers from the same place
    /// the const did would catch nothing.
    ///
    /// Source of record: the execution ledger's draft-1 row (2026-08-29, "DRAFT
    /// 1 (1 of ≤ 3), LEDGER-RECORDED BEFORE ITS READ"), whose vectors are
    /// themselves verified against the `t4-entry-run.txt` artifact of record.
    ///
    /// The row's own per-change provenance is re-derived here as a SECOND,
    /// independent check: the moves it names must be exactly the coordinates
    /// where this table differs from `V2`, and the displacements it records
    /// (14 early over 11 moves, 102 late over 12) must be what those
    /// differences sum to. A transcription that hit the literals but broke the
    /// arithmetic, or vice versa, fails one of the two.
    #[test]
    fn the_v4_step9_draft1_table_is_the_ledger_recorded_draft_one_vectors() {
        const LEDGER_EARLY: [i64; 34] = [
            1, 20, 14, -4, 3, 30, 5, -10, 6, 12, 15, 8, 5, 0, 3, -1, //
            1, 1, 0, 0, 1, 1, 1, 2, 2, 0, 0, 1, 0, 0, 0, 1, 1, 0,
        ];
        const LEDGER_LATE: [i64; 34] = [
            40, 40, 20, 8, 20, 20, 8, -10, 10, 20, 25, 14, 8, 4, 4, -2, //
            14, 1, -6, -2, 20, 0, 5, 21, 15, 0, 1, 0, 0, 1, 0, 0, 0, 0,
        ];
        use crate::game::eval::Feature;
        let table = WeightTable::V4_STEP9_DRAFT1;
        assert_eq!(table.early, LEDGER_EARLY, "EARLY differs from the draft-1 ledger row");
        assert_eq!(table.late, LEDGER_LATE, "LATE differs from the draft-1 ledger row");

        // SECOND CHECK, from the row's provenance rather than its vectors: the
        // moves it names, and the displacements it records.
        let moved = |from: &[i64; 34], to: &[i64; 34]| -> Vec<(&'static str, i64)> {
            Feature::ALL
                .iter()
                .enumerate()
                .filter(|(i, _)| from[*i] != to[*i])
                .map(|(i, f)| (f.name(), to[i] - from[i]))
                .collect()
        };
        // **NAMED AND SIGNED.** A count-and-magnitude check has a
        // duplicated-sign-flip blind spot: flipping one coordinate in BOTH the
        // const and the ledger literals above moves no count and no `abs()`
        // sum, so it would pass while shipping the opposite weight. The row
        // records which coordinates moved AND which way, so both are asserted
        // against literals here.
        assert_eq!(
            moved(&WeightTable::V2.early, &table.early),
            vec![
                ("achievement_progress_sum", -2),
                ("production_access", 1),
                ("track_access", 1),
                ("bank_access", 1),
                ("employment_agency_access", 1),
                ("president_reachable_areas", 1),
                ("money_stock", 2),
                ("trade_goods_stock", 2),
                ("order_matched_goods", 1),
                ("lab_takeable", 1),
                ("port_takeable", 1),
            ],
            "the draft-1 row's EARLY moves, by name and signed delta",
        );
        assert_eq!(
            moved(&WeightTable::V2.late, &table.late),
            vec![
                ("country_bonus_now", 10),
                ("assistants_in_hand_excess", 6),
                ("production_access", 14),
                ("track_access", 1),
                ("assistant_areas_distinct", -6),
                ("president_local_access", -2),
                ("bank_access", 20),
                ("president_reachable_areas", 5),
                ("money_stock", 21),
                ("trade_goods_stock", 15),
                ("held_order_value", 1),
                ("flag_icon_marginal", 1),
            ],
            "the draft-1 row's LATE moves, by name and signed delta",
        );
        // The displacements those signed deltas must sum to, kept as the third
        // independent reading of the same row.
        assert_eq!(
            moved(&WeightTable::V2.early, &table.early).iter().map(|(_, d)| d.abs()).sum::<i64>(),
            14,
            "the row records early displacement 14 over 11 moves",
        );
        assert_eq!(
            moved(&WeightTable::V2.late, &table.late).iter().map(|(_, d)| d.abs()).sum::<i64>(),
            102,
            "the row records late displacement 102 over 12 moves",
        );
        // The two findings the row calls out by name.
        assert_eq!(
            table.early[Feature::AchievementProgressSum.index()],
            0,
            "the forced incumbent zeroing, priced by 6(b)-early",
        );
        assert_eq!(
            (
                table.late[Feature::AssistantAreasDistinct.index()],
                table.late[Feature::PresidentLocalAccess.index()],
            ),
            (-6, -2),
            "the B1' forced-set members at their certified doses",
        );
        // B3's registered exclusion: X carries EARLY sight only.
        for x in [Feature::LabTakeable, Feature::PortTakeable, Feature::MatchedProductionAccess] {
            assert_eq!(table.late[x.index()], 0, "{} is zero at late (B3)", x.name());
        }
    }

    /// **THE LOCK RECORD** (spec §9's closing clause, lines 1026-1028): the
    /// digests and FULL echoes of all three identities — candidate, baseline,
    /// control — printed in a form the close row can transcribe.
    ///
    /// Printed rather than asserted, except for the one property that must
    /// hold: the three `weights=` identities are DISTINCT. Two arms echoing one
    /// identity could trade places in a corpus and nothing downstream could
    /// tell.
    #[test]
    fn the_three_locked_identities_print_their_digests_and_echoes() {
        let identity_of = |table: &WeightTable| crate::game::greedy::weight_table_identity(table);
        println!("\n=== T5 LOCK RECORD (spec §9:1026-1028) ===");
        println!("SEMANTICS_REV     {}", crate::game::semantics::SEMANTICS_REV);
        println!("rules hash        {}", Rules::standard().content_hash());
        for (role, arm) in [
            ("candidate", NamedIdentity::V4Step9Draft1),
            ("baseline ", NamedIdentity::V2Padded),
            ("control  ", NamedIdentity::V2RefitPadded),
        ] {
            let table = arm.table();
            let id = identity_of(&table);
            println!(
                "{role}  {:<16}  weights={id:#018x} ({id})\n             echo  {}",
                arm.name(),
                Contestant::NamedTable(arm).echo(),
            );
        }
        // **THE DIGESTS, PINNED AS LITERALS.** Nothing else in the tree pins
        // `weight_table_identity`'s OUTPUT — it is a fixed-seed splitmix64
        // chain over the sixty-eight numbers, so these do not drift across
        // platforms or builds. A failure here means one of exactly two things,
        // and both need a human: a named table MOVED (forbidden by the
        // never-move rule on each const), or `weight_table_identity` itself
        // changed (a deliberate act that re-points every figure ever recorded
        // against these names, and owes a ledger row).
        assert_eq!(
            identity_of(&WeightTable::V4_STEP9_DRAFT1),
            0x608e_8453_3717_4fc8_u64 as i64,
            "the T5-locked CANDIDATE identity",
        );
        assert_eq!(
            identity_of(&WeightTable::V2),
            0xefe5_b3e8_6b1e_c332_u64 as i64,
            "the T5-locked BASELINE identity",
        );
        assert_eq!(
            identity_of(&WeightTable::V2_REFIT_PADDED),
            0x128a_84d3_51d0_9230_u64 as i64,
            "the T5-locked CONTROL identity",
        );

        let ids: Vec<_> = NamedIdentity::ALL.iter().map(|a| identity_of(&a.table())).collect();
        let mut distinct = ids.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), ids.len(), "the three arms' identities must DIFFER: {ids:?}");
    }

    /// The WIRE names are the arms' own consts, and the grammar's literals are
    /// checks on them rather than a second source.
    #[test]
    fn the_named_identity_wire_names_round_trip_through_their_own_consts() {
        assert_eq!(NamedIdentity::ALL.len(), 3, "a new arm must be dispositioned in these pins");
        for arm in NamedIdentity::ALL {
            assert_eq!(NamedIdentity::from_str_name(arm.name()), Some(arm));
            assert_eq!(Contestant::from_str_name(arm.name()), Ok(Contestant::NamedTable(arm)));
        }
        assert_eq!(NamedIdentity::V2Padded.name(), "v2-padded");
        assert_eq!(NamedIdentity::V2RefitPadded.name(), "v2-refit-padded");
        assert_eq!(NamedIdentity::V4Step9Draft1.name(), "v4-step9-draft1");
        assert_eq!(NamedIdentity::from_str_name("v2"), None);
        assert_eq!(NamedIdentity::from_str_name("V2-PADDED"), None);
    }

    /// **THE ECHO CARRIES THE NAME AND THE ARM'S OWN TABLE IDENTITY**, composed
    /// through the one [`compose_echo`] every contestant echoes through.
    ///
    /// The two `weights=` values must DIFFER (the T2 review's carry): two arms
    /// that echoed one identity could trade places in a corpus and nothing
    /// downstream could tell. And `v2-padded` must resolve `WeightTable::V2`'s
    /// identity exactly — an arm whose construction drifted off its named table
    /// would echo some third digest, which is the always-on shape of V3 §5.1's
    /// behavior-identity pin.
    #[test]
    fn the_named_identity_arms_echo_their_name_and_their_own_table_identity() {
        let identity_of = |table: &WeightTable| crate::game::greedy::weight_table_identity(table);
        // Recorded for the wave's commit message; a digest is only useful if it
        // is written down somewhere a reader can compare against.
        println!(
            "named-identity weights: v2-padded={} v2-refit-padded={} v4-step9-draft1={}",
            identity_of(&WeightTable::V2),
            identity_of(&WeightTable::V2_REFIT_PADDED),
            identity_of(&WeightTable::V4_STEP9_DRAFT1),
        );

        for arm in NamedIdentity::ALL {
            let echo = Contestant::NamedTable(arm).echo();
            assert!(
                echo.starts_with(&format!("{}@v", arm.name())),
                "{} must echo under its own name: {echo}",
                arm.name(),
            );
            assert!(
                echo.contains(&format!("weights={}", identity_of(&arm.table()))),
                "{} must echo its OWN table's identity: {echo}",
                arm.name(),
            );
            // The knobs are the controller's own answer, so an arm cannot echo
            // a search it did not run.
            for token in ["b=4", "k=1", "m=8", "eval_impl="] {
                assert!(echo.contains(token), "{} lost {token} from its echo: {echo}", arm.name());
            }
            assert!(!echo.contains("cap="), "an uncapped arm must emit no cap token: {echo}");
        }

        assert_ne!(
            identity_of(&WeightTable::V2),
            identity_of(&WeightTable::V2_REFIT_PADDED),
            "the two named arms must resolve DIFFERENT tables, or every cell between them is \
             a comparison of one table with itself",
        );
        // **THE DISTINCT-IDENTITIES PIN** (the T2 review's carry), read off the
        // RESOLVED echoes rather than off the consts: a name that resolved to
        // the wrong table would still echo a well-formed `weights=` token, and
        // only comparing what the two arms ACTUALLY resolved catches it.
        let weights_token = |arm: NamedIdentity| {
            let echo = Contestant::NamedTable(arm).echo();
            let token = echo
                .split("weights=")
                .nth(1)
                .unwrap_or_else(|| panic!("{} echoed no weights token: {echo}", arm.name()))
                .trim_end_matches('}')
                .to_string();
            token
        };
        assert_ne!(
            weights_token(NamedIdentity::V2Padded),
            weights_token(NamedIdentity::V2RefitPadded),
            "the two named arms RESOLVED the same table, so the arms can be conflated and a \
             cell between them would report one arm's figures under the other's name",
        );
        assert_ne!(
            Contestant::NamedTable(NamedIdentity::V2Padded).echo(),
            Contestant::NamedTable(NamedIdentity::V2RefitPadded).echo(),
        );
        // `v2-padded` IS the shipped seat, so the two differ in NAME and in
        // nothing else — the identity claim spec §10 makes, as a string compare.
        let padded = Contestant::NamedTable(NamedIdentity::V2Padded).echo();
        let shipped = Contestant::LOOKAHEAD_V1_DEFAULT.echo();
        assert_eq!(
            padded.replace("v2-padded@", "lookahead-v1@"),
            shipped,
            "v2-padded must be the shipped seat under another name: {padded} vs {shipped}",
        );
    }

    /// **THE KEEP IS WEIGHT-BLIND, AND THAT IS WHY THERE IS NO IDENTITY LEAK.**
    ///
    /// A named arm's Keep cannot come off the wrong table, because
    /// `greedy::score_starting_order` reads NO table at all — `eval_screen`'s
    /// doc states it in as many words ("the KEEP function … takes NO weight
    /// table — so a screen is Keep-HOMOGENEOUS and setup-IDENTICAL by
    /// construction, at any pair of tables"). So the honest pin is not "each
    /// arm keeps by its own table" (nothing does) but the structural fact that
    /// makes a §11 cell setup-identical: both arms and the shipped seat resolve
    /// EVERY deal identically.
    ///
    /// It is pinned rather than assumed so a future weight-sensitive Keep
    /// cannot land silently: it would fail here, and the arm-identity question
    /// it raises would have to be answered in that commit.
    #[test]
    fn the_named_identity_arms_keep_by_a_weight_blind_rule() {
        let rules = Rules::standard();
        for arm in NamedIdentity::ALL {
            assert_eq!(
                Contestant::NamedTable(arm).keep_kind(),
                KeepKind::EvaluatorScored,
                "{} must keep by the evaluator rule",
                arm.name(),
            );
        }
        // Maximally different streams, `every_evaluator_contestant_shares_one_keep_function`'s
        // rule: agreement must come from the deal, never from a shared stream.
        let (mut padded, _) =
            Contestant::NamedTable(NamedIdentity::V2Padded).build(&rules, 3);
        let (mut refit, _) =
            Contestant::NamedTable(NamedIdentity::V2RefitPadded).build(&rules, u64::MAX / 7);
        let (mut shipped, _) = Contestant::LOOKAHEAD_V1_DEFAULT.build(&rules, u64::MAX - 29);
        for seat in 0..4u8 {
            for card in 1..=18u8 {
                let deal = [OrderCardId(card), OrderCardId(card + 18)];
                let p = padded.choose_starting_order(PlayerId(seat), &deal);
                let r = refit.choose_starting_order(PlayerId(seat), &deal);
                let s = shipped.choose_starting_order(PlayerId(seat), &deal);
                assert!(
                    p == r && r == s,
                    "the Keep became table-sensitive at seat {seat}, deal {deal:?}: \
                     v2-padded {p}, v2-refit-padded {r}, shipped {s} — a §11 cell's \
                     setup-identical=yes rests on these agreeing",
                );
            }
        }
    }

    /// **EACH ARM PLAYS ITS OWN TABLE** — the play-side routing, where an
    /// identity leak WOULD be possible, driven rather than asserted about.
    ///
    /// Six decisions over three deterministic positions (the
    /// `the_pinned_v2_seat_walk_reproduces_the_recorded_pre_padding_decisions`
    /// shape), built through `Contestant::build` so a table swapped there has
    /// to move a played move rather than escaping through a direct helper.
    /// Three claims, and the third is what keeps the other two from being
    /// vacuous:
    ///
    /// 1. `v2-padded` plays exactly what `lookahead-v1` plays (it IS that seat);
    /// 2. `v2-refit-padded` plays exactly what a DIRECT `with_knobs`
    ///    construction on `WeightTable::V2_REFIT_PADDED` plays (the named table
    ///    reaches the agent);
    /// 3. the two arms DIVERGE somewhere on this walk — without which every
    ///    table pin here would pass on two tables that play identically.
    #[test]
    fn the_named_identity_arms_walk_their_own_tables() {
        use crate::game::policy::RolloutPolicy;
        use crate::game::search_actions::DecisionHandle;
        use crate::types::GameConfig;
        use crate::StartingOrderChoice;

        let rules = Rules::standard();
        // (setup seed, family-weighted steps forward, agent stream seed)
        const POSITIONS: [(u64, usize, u64); 3] =
            [(0xD1, 0, 0xA11CE0), (0xD2, 60, 0xA11CE1), (0xD3, 140, 0xA11CE2)];
        const DECISIONS: usize = 2;

        let walk = |build: &mut dyn FnMut(&Rules, u64) -> Box<dyn ArenaAgent>| -> Vec<String> {
            let mut moves = Vec::new();
            for (seed, steps, agent_seed) in POSITIONS {
                let pending = crate::game::new_game(GameConfig::for_player_count(2), &rules, seed)
                    .expect("setup");
                let mut state =
                    crate::game::finalize_setup(pending, &[StartingOrderChoice::Keep(0); 2])
                        .expect("finalize");
                let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x3F3F);
                let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
                for step in 0..steps {
                    let mv = {
                        let handle = DecisionHandle::new(&state, &rules);
                        policy
                            .pick(&handle, &mut rng)
                            .unwrap_or_else(|| panic!("dead end at seed {seed:#x} step {step}"))
                    };
                    state = crate::game::apply_move_unchecked(&state, &mv, &rules)
                        .expect("policy move applies");
                }
                let mut agent = build(&rules, agent_seed);
                for _ in 0..DECISIONS {
                    let mover = state.current_player;
                    let view = crate::game::observe(&state, mover).expect("the mover's view");
                    let report = agent.choose(&view, &rules).expect("the seat decides");
                    moves.push(format!("{:?}", report.mv));
                    state = crate::game::apply_move(&state, &report.mv, &rules)
                        .expect("the seat's move is legal at the checked boundary");
                }
            }
            moves
        };

        let contestant = |c: Contestant| {
            move |rules: &Rules, seed: u64| -> Box<dyn ArenaAgent> { c.build(rules, seed).0 }
        };
        let padded = walk(&mut contestant(Contestant::NamedTable(NamedIdentity::V2Padded)));
        let refit = walk(&mut contestant(Contestant::NamedTable(NamedIdentity::V2RefitPadded)));
        let shipped = walk(&mut contestant(Contestant::LOOKAHEAD_V1_DEFAULT));
        // The DIRECT construction on the named table, bypassing `build`
        // entirely: if `build` handed the agent some other table, this walk and
        // the arm's would part company.
        let direct = walk(&mut |rules: &Rules, seed: u64| -> Box<dyn ArenaAgent> {
            Box::new(LookaheadAgent::with_knobs(
                rules,
                SearchScheme::TurnBoundary { beam: NAMED_IDENTITY_BEAM },
                NAMED_IDENTITY_WORLDS,
                NAMED_IDENTITY_RETREAT_SAMPLES,
                WeightTable::V2_REFIT_PADDED,
                seed,
            ))
        });
        assert_eq!(padded.len(), POSITIONS.len() * DECISIONS, "the walk must reach every decision");

        assert_eq!(
            padded, shipped,
            "v2-padded must play the shipped seat's game — it names the shipped table",
        );
        assert_eq!(
            refit, direct,
            "v2-refit-padded's built agent must play the table its name resolves to",
        );
        assert_ne!(
            refit, padded,
            "the two arms played an IDENTICAL walk, so this walk cannot tell their tables \
             apart and every table pin resting on it is vacuous",
        );
        println!("named-identity walk: v2-padded {padded:?}\n                     refit {refit:?}");
    }

    /// A named arm survives the descriptor round trip — and a record naming an
    /// arm at knobs the arm does not have is REFUSED rather than resumed as if
    /// it were that arm.
    #[test]
    fn a_named_identity_round_trips_through_its_descriptor() {
        for arm in NamedIdentity::ALL {
            let contestant = Contestant::NamedTable(arm);
            let (name, _version, params) = contestant.descriptor();
            assert_eq!(name, arm.name());
            assert_eq!(
                Contestant::from_parts(&name, &params),
                Ok(contestant),
                "{} must reconstruct from its own descriptor",
                arm.name(),
            );
            // The identity keys ride the map and are NOT consumed — the
            // preserve-list rule, so a record from a build whose table moved
            // stays loadable and the drift stays visible to the caller.
            assert!(params.contains_key("weights") && params.contains_key("eval_impl"));

            let mut wrong = params.clone();
            wrong.insert("b".to_string(), 8);
            let message = Contestant::from_parts(&name, &wrong)
                .map(|c| format!("{c:?}"))
                .expect_err("a knobbed record must not resume as a knobless arm");
            assert!(message.contains("knobless"), "{}: {message}", arm.name());
        }
    }

    // -----------------------------------------------------------------------
    // W-A1 — the void-unit instrument (V4 step 11, amendment A-S11-1; the
    // operational definitions are freeze doc
    // `docs/plans/2026-08-30-phase3.5c-v4-identity-freeze.md` §9.6, registered
    // before this code existed).
    // -----------------------------------------------------------------------

    /// W-A1's OWN committed diagnostic master. The constructor's NAME is the
    /// waiver (`game::seeds`): these cells assert, and an OS-entropy master
    /// would flake by construction.
    ///
    /// A FRESH literal, deliberately none of the step-11 masters that carry a
    /// read: not the registered `0x0B0120260831990{1..5}` of the D2 cap-rate
    /// wave, not the `999{7,8,9}` scratch masters the freeze doc already spent,
    /// and not `arena::diagnostic`'s own `0x0B0120260831_9999`. A test that
    /// played a registered schedule would consume a read the orchestrator has
    /// not taken.
    const W_A1_MASTER: u64 = 0x0B01_2026_0830_A1A1;

    /// A cell of two RUNGS at [`W_A1_MASTER`] — cheap enough to play whole 2P
    /// games in the per-edit tier, which every cell below does.
    fn w_a1_spec(name: &str, units: u32) -> CellSpec {
        w_a1_spec_at(name, W_A1_MASTER, units)
    }

    /// [`w_a1_spec`] at a stated master — the wave ships two fixtures on two
    /// masters, and the CONTESTANTS are the same pair in both so the two differ
    /// only in the schedule they draw.
    fn w_a1_spec_at(name: &str, master: u64, units: u32) -> CellSpec {
        CellSpec {
            name: name.to_string(),
            players: 2,
            x: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
            y: Contestant::Policy(RolloutPolicy::RetreatAverse),
            units,
            master: SeedScheduleKind::Diagnostic(master),
        }
    }

    /// The turn guard at which EVERY game of a one-unit [`w_a1_spec`] caps —
    /// the all-void shape. (Measured: unit 0's two games end naturally at turns
    /// 128 and 104.)
    const W_A1_ALL_VOID_MAX_TURNS: u32 = 50;

    /// **THE MIXED FIXTURE'S GUARD.** Under it exactly TWO of
    /// [`W_A1_MIXED_UNITS`]'s twelve games pass the guard — and both are their
    /// unit's SECOND game, after a first game that COMPLETED. That is the shape
    /// the void semantics can get wrong: an outcome folded when its game
    /// finished cannot be taken back once the unit turns out to be void, so the
    /// loop has to buffer a unit's outcome and fold it only when the unit is
    /// known READ.
    const W_A1_MIXED_MAX_TURNS: u32 = 130;

    /// The `(unit index, arrangement)` of every capped game of the mixed
    /// fixture, in schedule order — recorded the way the contestant goldens
    /// record their probed seeds, and asserted below rather than trusted.
    const W_A1_MIXED_CAPPED: [(usize, u8); 2] = [(1, 1), (3, 1)];

    /// **THE TIE FIXTURE** — a second master, added by the fix wave for review
    /// S1: the wave's first fixture reads ZERO for both tie counters, so every
    /// assertion about them compared 0 with 0 and a mutant that leaked
    /// `top_vp_ties`/`ties_to_seat0` out of a void unit survived the suite.
    ///
    /// FOUND BY A BOUNDED SEARCH, recorded so it is reproducible: 20 masters
    /// `0x0B01_2026_0830_A101..=A114` × 40 units of [`w_a1_spec_at`], every game
    /// played to the shipped guard through `diagnostic::run_world_capped` and
    /// scored for `standings[0].total == standings[1].total`. 27 shared-top
    /// games in 1,600 (1.7%, the repo's own ~1.26% figure at this size), ALL of
    /// them decided for seat 0 — which is `cross_checks`'s "at 2P seat 0 takes
    /// every tie", measured. This master is the one shape in that search
    /// carrying a shared top total on BOTH sides of the void/read line.
    ///
    /// At `A10D`, 19 units, guard 94, the 38 scheduled games end naturally at
    /// `[114,40] [168,58] [118,58] [94,100] [58,98] [38,154] [94,80] [82,86]
    /// [58,106] [76,84] [80,228] [148,134] [84,110] [138,140] [68,72]
    /// [102,134] [82,88] [178,152] [66,48]` (unit 0 first, arrangement 0
    /// first). Seventeen pass the guard, leaving 13 void units and 6 read
    /// (units 6, 7, 9, 14, 16, 18):
    ///
    ///   * **unit 3 arrangement 0** completes at 94 turns with a SHARED top VP
    ///     total, and its sibling caps at 100 — so the tie sits INSIDE a void
    ///     unit and must not reach either counter;
    ///   * **unit 18 arrangement 1** completes at 48 turns with a shared top
    ///     total in a READ unit — so the healthy report reads ONE, not zero,
    ///     and a mutation that DROPPED a read unit's tie is caught too.
    const W_A1_TIE_MASTER: u64 = 0x0B01_2026_0830_A10D;
    const W_A1_TIE_UNITS: u32 = 19;
    const W_A1_TIE_MAX_TURNS: u32 = 94;
    /// Every capped game of the tie fixture, `(unit index, arrangement)`, in
    /// schedule order.
    const W_A1_TIE_CAPPED: [(usize, u8); 17] = [
        (0, 0),
        (1, 0),
        (2, 0),
        (3, 1),
        (4, 1),
        (5, 1),
        (8, 1),
        (10, 1),
        (11, 0),
        (11, 1),
        (12, 1),
        (13, 0),
        (13, 1),
        (15, 0),
        (15, 1),
        (17, 0),
        (17, 1),
    ];
    /// The completed game of a VOID unit whose top VP total was SHARED.
    const W_A1_TIE_IN_A_VOID_UNIT: (usize, u8) = (3, 0);

    /// The greedy fixture: three units at the same master and the same guard,
    /// where unit 0's FIRST game caps and its second COMPLETES — the mirror of
    /// the rung fixture's shape, and the one that proves a void unit's other
    /// arrangement is still played.
    const W_A1_GREEDY_UNITS: u32 = 3;

    /// The scheduled world seeds of a cell at [`W_A1_MASTER`], re-derived from
    /// the DESIGN exactly as the loop derives them. Never transcribed: a
    /// schedule change must fail here rather than leave a stale literal that
    /// still passes.
    fn w_a1_scheduled_seeds(units: u32) -> Vec<WorldSeed> {
        w_a1_scheduled_seeds_at(W_A1_MASTER, units)
    }

    /// [`w_a1_scheduled_seeds`] at a stated master.
    fn w_a1_scheduled_seeds_at(master: u64, units: u32) -> Vec<WorldSeed> {
        let mut schedule = SeedSchedule::diagnostic(master);
        (0..units).map(|_| schedule.next_world()).collect()
    }

    /// The design size of the wave's fixture cell — six units, twelve games.
    ///
    /// MEASURED at [`W_A1_MASTER`] before any of this code existed, the way the
    /// contestant goldens record their probed seeds: the twelve scheduled games
    /// end naturally at turns `[128, 104] [108, 136] [86, 72] [68, 168]
    /// [102, 60] [120, 86]` (unit 0 first, arrangement 0 first), so at the
    /// shipped guard the cell caps NOTHING and under a guard in the low
    /// hundreds it caps some units and not others.
    const W_A1_MIXED_UNITS: u32 = 6;

    /// **Pin (i), the construction half made a test.** Every existing entry
    /// point reaches ONE loop at `CapPolicy::Refuse` and `HONEST_MAX_TURNS`, so
    /// a report produced through the OLD public entry point and one produced
    /// through the parameterized loop are the SAME report — serialized-equal,
    /// which covers the unit vector, the interval, the drawn resample seed, the
    /// tie accounting and every counter at once.
    ///
    /// The void fields' refuse-mode values are pinned here too, and the echo is
    /// pinned NEGATIVELY: an absent `--capped-ceiling` emits no token at all
    /// (the `cap`-knob precedent, freeze doc §9.6).
    #[test]
    fn refuse_mode_reaches_the_same_report_through_the_parameterized_loop() {
        let rules = Rules::standard();
        let spec = w_a1_spec("w-a1-refuse-inert", 3);

        let old = run_cell(&spec, &rules).expect("a rung cell completes");
        let (looped, _wall) = run_cell_loop(
            &spec,
            &rules,
            true,
            CapPolicy::Refuse,
            HONEST_MAX_TURNS,
            &mut |_stats, _arr| Ok(()),
        )
        .expect("the same cell through the parameterized loop");

        assert_eq!(
            serde_json::to_string(&old).unwrap(),
            serde_json::to_string(&looped).unwrap(),
            "the old entry point and the parameterized loop are the same loop",
        );

        assert_eq!(old.capped_ceiling, None, "refuse mode registers no ceiling");
        assert_eq!(old.units_scheduled, spec.units);
        assert_eq!(old.units_void, 0, "no unit can be void in refuse mode");
        assert_eq!(old.units_read, spec.units);
        assert_eq!(old.units_read as usize, old.x_unit_shares.len());
        assert!(old.capped.is_empty(), "refuse mode counts no capped game");
        assert_eq!(old.games, spec.units * u32::from(spec.players));
        assert!(
            !old.spec_echo.contains("capped-ceiling"),
            "an absent ceiling emits NO token at all: {}",
            old.spec_echo,
        );
    }

    /// **THE VOID SEMANTICS.** The design UNIT containing a capped game is
    /// void: it contributes no outcome, and every outcome statistic on the
    /// report — the unit vector, θ̂, the interval, `games`, the trigger tally —
    /// reads over the READ units alone (freeze doc §9.6).
    ///
    /// The resample stream is the one thing that does NOT move with the read
    /// count: it is derived from the DESIGN (master, SCHEDULED units, players)
    /// exactly as before, so an interval stays a function of the data once.
    #[test]
    fn a_mixed_cell_voids_the_units_that_capped_and_reads_the_rest() {
        use std::collections::BTreeSet;

        let rules = Rules::standard();
        let spec = w_a1_spec("w-a1-mixed", W_A1_MIXED_UNITS);
        let (report, _wall) = run_cell_loop(
            &spec,
            &rules,
            true,
            CapPolicy::Count { ceiling: W_A1_MIXED_CAPPED.len() as u32 },
            W_A1_MIXED_MAX_TURNS,
            &mut |_stats, _arr| Ok(()),
        )
        .expect("a cell at its own ceiling completes");

        let seeds = w_a1_scheduled_seeds(spec.units);
        let expected: Vec<CappedGame> = W_A1_MIXED_CAPPED
            .iter()
            .map(|&(unit, arrangement)| CappedGame {
                seed: seeds[unit],
                arrangement,
                // The driver's own number, one past the guard.
                turns: W_A1_MIXED_MAX_TURNS + 1,
            })
            .collect();
        assert_eq!(report.capped, expected, "the counted events, in schedule order");

        let void: BTreeSet<usize> = W_A1_MIXED_CAPPED.iter().map(|&(unit, _)| unit).collect();
        assert_eq!(report.units_scheduled, spec.units);
        assert_eq!(report.units_void, void.len() as u32);
        assert_eq!(report.units_read, spec.units - void.len() as u32);
        assert_eq!(report.x_unit_shares.len(), report.units_read as usize, "units_read IS n");
        assert_eq!(
            report.games,
            report.units_read * u32::from(spec.players),
            "`games` on the report is the READ count",
        );
        assert_eq!(
            report.trigger.games(),
            u64::from(report.games),
            "a void unit's completed game is not an observation of the trigger distribution",
        );
        assert_eq!(report.theta_hat, mean(&report.x_unit_shares));

        let resample_seed = mix(
            SALT_BOOTSTRAP,
            &[W_A1_MASTER, u64::from(spec.units), u64::from(spec.players)],
        );
        assert_eq!(
            report.interval,
            interval_for_units(&report.x_unit_shares, BOOTSTRAP_REPLICATES, resample_seed),
            "the interval is the one the READ vector gives at the DESIGN-derived seed",
        );

        // The echo's `games=` is the SCHEDULED count — item 4's two clauses
        // answered by two numbers.
        assert!(report.spec_echo.contains(&format!("units={} ", spec.units)), "{}", report.spec_echo);
        assert!(
            report
                .spec_echo
                .contains(&format!("games={} ", u32::from(spec.players) * spec.units)),
            "{}",
            report.spec_echo,
        );
    }

    /// **THE CROSS-SURFACE PIN (pin (iii)).** The cell's figures are the fold of
    /// independent world reads over the RIGHT games: the outcome figures over
    /// the READ units only, the volume counters over ALL games — the capped
    /// ones' partial `moves_by_seat` included.
    ///
    /// The reads come from `arena::diagnostic::run_world_capped`, which seats
    /// and keys a world exactly as a cell does but tabulates nothing, so this
    /// compares the tabulation against a surface that shares only the play.
    ///
    /// ANTI-VACUITY is load-bearing on the volume half: the fold over ALL games
    /// and the fold over READ games alone must actually DIFFER at this fixture,
    /// or "over every played game" would be indistinguishable from "over the
    /// read units".
    #[test]
    fn a_mixed_cells_figures_are_the_fold_of_the_world_reads_over_the_right_games() {
        use crate::game::arena::diagnostic::{run_world_capped, WorldSpec};

        let rules = Rules::standard();
        let spec = w_a1_spec("w-a1-mixed-cross", W_A1_MIXED_UNITS);
        let (report, _wall) = run_cell_loop(
            &spec,
            &rules,
            true,
            CapPolicy::Count { ceiling: W_A1_MIXED_CAPPED.len() as u32 },
            W_A1_MIXED_MAX_TURNS,
            &mut |_stats, _arr| Ok(()),
        )
        .expect("a cell at its own ceiling completes");

        let players = spec.players;
        let mut applied_by_seat = vec![0u64; players as usize];
        let mut applied_by_seat_read_only = vec![0u64; players as usize];
        let mut x_applied_moves = 0u64;
        let mut seat0_wins = 0u32;
        let mut ties_to_seat0 = 0u32;
        let mut trigger = TriggerTally::default();
        let mut vp_margin_sum = 0.0f64;
        let mut shares: Vec<f64> = Vec::new();
        let mut games = 0u32;

        for seed in w_a1_scheduled_seeds(spec.units) {
            let mut completed = Vec::new();
            let mut unit_void = false;
            for arrangement in 0..players {
                let run = run_world_capped(
                    &WorldSpec { x: spec.x, y: spec.y, seed, players, arrangement },
                    &rules,
                    true,
                    W_A1_MIXED_MAX_TURNS,
                )
                .expect("the world plays");
                let x_seat = run.x_seat;
                match run.game.outcome {
                    DiagnosticOutcome::Capped { moves_by_seat, .. } => {
                        for (seat, moves) in moves_by_seat.iter().enumerate() {
                            applied_by_seat[seat] += moves;
                        }
                        x_applied_moves += moves_by_seat[x_seat.0 as usize];
                        unit_void = true;
                    }
                    DiagnosticOutcome::Terminated(stats) => {
                        for (seat, moves) in stats.moves_by_seat.iter().enumerate() {
                            applied_by_seat[seat] += moves;
                        }
                        x_applied_moves += stats.moves_by_seat[x_seat.0 as usize];
                        completed.push((stats, x_seat));
                    }
                }
            }
            if unit_void {
                continue;
            }
            let mut unit_x_wins = 0u32;
            for (stats, x_seat) in completed {
                games += 1;
                for (seat, moves) in stats.moves_by_seat.iter().enumerate() {
                    applied_by_seat_read_only[seat] += moves;
                }
                trigger.observe(stats.trigger.as_ref());
                if stats.result.winner == x_seat {
                    unit_x_wins += 1;
                }
                if stats.result.winner == PlayerId(0) {
                    seat0_wins += 1;
                }
                let standings = &stats.result.standings;
                let top_shared = standings.len() >= 2
                    && standings[0].breakdown.total == standings[1].breakdown.total;
                if top_shared && stats.result.winner == PlayerId(0) {
                    ties_to_seat0 += 1;
                }
                let (mut x_total, mut y_total) = (0.0f64, 0.0f64);
                for standing in standings {
                    if standing.player == x_seat {
                        x_total = f64::from(standing.breakdown.total);
                    } else {
                        y_total += f64::from(standing.breakdown.total);
                    }
                }
                vp_margin_sum += x_total - y_total / f64::from(players - 1);
            }
            shares.push(f64::from(unit_x_wins) / f64::from(players));
        }

        assert_ne!(
            applied_by_seat, applied_by_seat_read_only,
            "ANTI-VACUITY: this fixture's capped games must move the volume fold",
        );

        // (a) OUTCOME — over the READ units only.
        assert_eq!(report.games, games, "games");
        assert_eq!(report.x_unit_shares, shares, "the unit vector");
        assert_eq!(report.seat0_wins, seat0_wins, "seat-0 wins");
        assert_eq!(report.ties_to_seat0, ties_to_seat0, "ties decided for seat 0");
        assert_eq!(report.trigger, trigger, "the end-trigger tally");
        assert_eq!(
            report.vp_margin_mean,
            vp_margin_sum / f64::from(games),
            "the VP margin, summed in the same order the cell sums it",
        );

        // (b) VOLUME — over EVERY played game, capped ones included.
        assert_eq!(report.applied_by_seat, applied_by_seat, "applied moves per seat");
        assert_eq!(report.x_applied_moves, x_applied_moves, "X's applied moves");
    }

    /// **Pin (v)'s library half.** The two lines a counting cell prints are
    /// composed HERE and lib-tested, so `examples/arena` stays print-only glue
    /// — and `void_summary` returns `None` in refuse mode, which is what makes
    /// an unflagged run print byte-for-byte what it always printed.
    #[test]
    fn the_render_helpers_are_none_in_refuse_mode_and_exact_in_counting_mode() {
        const CEILING: u32 = 7;
        let rules = Rules::standard();

        let refused =
            run_cell(&w_a1_spec("w-a1-render-refuse", 1), &rules).expect("a rung cell completes");
        assert_eq!(refused.void_summary(), None, "refuse mode prints no void line at all");
        assert!(refused.capped_lines().is_empty());

        let spec = w_a1_spec("w-a1-render", W_A1_MIXED_UNITS);
        let (report, _wall) = run_cell_loop(
            &spec,
            &rules,
            true,
            CapPolicy::Count { ceiling: CEILING },
            W_A1_MIXED_MAX_TURNS,
            &mut |_stats, _arr| Ok(()),
        )
        .expect("under its ceiling the cell completes");

        assert_eq!(
            report.void_summary(),
            Some("units scheduled=6 · void=2 · read=4 · capped games=2 (ceiling 7)".to_string()),
        );

        let lines = report.capped_lines();
        assert_eq!(lines.len(), report.capped.len(), "ONE line per capped game");
        for (line, game) in lines.iter().zip(&report.capped) {
            assert_eq!(
                *line,
                format!(
                    "capped: arena world <x> <y> {:#018x} --arrangement {} · turns={}",
                    game.seed.0, game.arrangement, game.turns,
                ),
            );
        }
    }

    /// **Pin (iv), the ceiling.** The (c+1)-th capped game refuses the cell AT
    /// ONCE — not at the schedule's end — with a typed arm naming the ceiling
    /// and every capped game so far, and the run is INVALID (freeze doc §9.6
    /// item 2).
    ///
    /// The EARLY half is what the sink's call count proves: the fixture's
    /// second cap is its eighth scheduled game, so a cell that finished the
    /// schedule and refused afterwards would have emitted all ten completed
    /// games instead of six.
    #[test]
    fn the_ceiling_refuses_at_the_next_capped_game_and_cuts_the_schedule_short() {
        let rules = Rules::standard();
        let spec = w_a1_spec("w-a1-ceiling", W_A1_MIXED_UNITS);
        let caps = W_A1_MIXED_CAPPED.len() as u32;

        // The whole schedule's completed games, for the comparison below.
        let mut all_calls = 0u32;
        run_cell_loop(
            &spec,
            &rules,
            true,
            CapPolicy::Count { ceiling: caps },
            W_A1_MIXED_MAX_TURNS,
            &mut |_stats, _arr| {
                all_calls += 1;
                Ok(())
            },
        )
        .expect("at its own ceiling the cell completes");
        assert_eq!(
            all_calls,
            W_A1_MIXED_UNITS * 2 - caps,
            "the full schedule emits every completed game",
        );

        let mut calls = 0u32;
        let err = run_cell_loop(
            &spec,
            &rules,
            true,
            CapPolicy::Count { ceiling: caps - 1 },
            W_A1_MIXED_MAX_TURNS,
            &mut |_stats, _arr| {
                calls += 1;
                Ok(())
            },
        )
        .expect_err("one cap above the ceiling refuses");

        let ArenaError::CappedCeilingExceeded { ceiling, capped } = &err else {
            panic!("the ceiling has its OWN typed arm, got {err:?}");
        };
        assert_eq!(*ceiling, caps - 1);
        assert_eq!(
            capped.len() as u32,
            caps,
            "the refusal carries every capped game so far, the (c+1)-th included",
        );
        assert!(
            calls < all_calls,
            "the refusal must cut the schedule SHORT: {calls} emitted, {all_calls} in the full run",
        );

        let message = err.to_string();
        assert!(message.contains(&format!("{}", caps - 1)), "{message}");
        assert!(message.contains(&format!("{caps}")), "{message}");
        for game in capped {
            assert!(
                message.contains(&format!(
                    "seed={:#018x} arrangement={} turns={}",
                    game.seed.0, game.arrangement, game.turns,
                )),
                "every capped game must be named: {message}",
            );
        }
    }

    /// **Pin (iv), `Some(0)` is not `None`.** A registered ceiling of zero is
    /// legal and refuses at the FIRST capped game — through the ceiling arm,
    /// deliberately distinct from an absent flag's `CapReached`, so a reader of
    /// a failed run can tell an instrument-guard refusal from the un-amended
    /// rule.
    #[test]
    fn a_zero_ceiling_and_an_absent_flag_are_different_refusals() {
        let rules = Rules::standard();
        let spec = w_a1_spec("w-a1-zero-ceiling", W_A1_MIXED_UNITS);

        let counted = run_cell_loop(
            &spec,
            &rules,
            true,
            CapPolicy::Count { ceiling: 0 },
            W_A1_MIXED_MAX_TURNS,
            &mut |_stats, _arr| Ok(()),
        )
        .expect_err("ceiling 0 refuses at the first capped game");
        let ArenaError::CappedCeilingExceeded { ceiling, capped } = &counted else {
            panic!("expected the ceiling arm, got {counted:?}");
        };
        assert_eq!(*ceiling, 0);
        assert_eq!(capped.len(), 1, "the FIRST capped game, and no more of the schedule");

        let refused = run_cell_loop(
            &spec,
            &rules,
            true,
            CapPolicy::Refuse,
            W_A1_MIXED_MAX_TURNS,
            &mut |_stats, _arr| Ok(()),
        )
        .expect_err("refuse mode refuses at the first capped game too");
        assert!(
            matches!(refused, ArenaError::CapReached { .. }),
            "an absent flag keeps its OWN refusal, got {refused:?}",
        );
    }

    /// **Pin (iv), the all-void refusal.** A cell whose every scheduled unit is
    /// void has no read unit, so nothing constrains θ — and it refuses, typed,
    /// rather than reporting a table of NaNs.
    #[test]
    fn a_cell_with_no_read_units_refuses_instead_of_reporting_nans() {
        let rules = Rules::standard();
        let spec = w_a1_spec("w-a1-all-void", 1);

        let err = run_cell_loop(
            &spec,
            &rules,
            true,
            // At or above the game count, so the ceiling cannot be what refuses.
            CapPolicy::Count { ceiling: u32::from(spec.players) },
            W_A1_ALL_VOID_MAX_TURNS,
            &mut |_stats, _arr| Ok(()),
        )
        .expect_err("a cell with no read unit cannot report a theta");

        let ArenaError::NoReadUnits { scheduled, capped } = &err else {
            panic!("expected the no-read-units arm, got {err:?}");
        };
        assert_eq!(*scheduled, spec.units);
        assert_eq!(capped.len(), usize::from(spec.players), "both games of the only unit capped");
        let message = err.to_string();
        for game in capped {
            assert!(
                message.contains(&format!(
                    "seed={:#018x} arrangement={} turns={}",
                    game.seed.0, game.arrangement, game.turns,
                )),
                "{message}",
            );
        }
    }

    /// **THE TIE COUNTERS' void-exclusion, MEASURED** (fix wave, review S1).
    ///
    /// `top_vp_ties` and `ties_to_seat0` are the two buffered outcome
    /// quantities the wave's first fixture could not cover: they read ZERO
    /// there, so every assertion about them compared 0 with 0 and a mutant
    /// leaking them out of a void unit survived the whole suite. This cell runs
    /// the TIE FIXTURE, where a shared top VP total sits on BOTH sides of the
    /// void/read line, and asserts both counters against an independent fold
    /// over the READ units alone.
    ///
    /// `top_vp_ties` is not a [`CellReport`] field — it reaches a reader only
    /// through the `cross_checks` top-VP row — so that row is asserted by its
    /// printed text, which is the surface a reader actually quotes.
    ///
    /// MUTATION-PROVEN, and recorded because the cell's healthy values are
    /// small: with the review's probe (g) applied — `top_vp_ties` and
    /// `ties_to_seat0` folded in the void `continue` instead of dropped — this
    /// cell FAILS (`ties_to_seat0` reads 2 where the read-unit fold says 1) and
    /// the whole rest of the suite still passes.
    #[test]
    fn a_void_units_shared_top_vp_total_reaches_neither_tie_counter() {
        use crate::game::arena::diagnostic::{run_world_capped, WorldSpec};

        let rules = Rules::standard();
        let spec = w_a1_spec_at("w-a1-tie", W_A1_TIE_MASTER, W_A1_TIE_UNITS);
        let (report, _wall) = run_cell_loop(
            &spec,
            &rules,
            true,
            CapPolicy::Count { ceiling: W_A1_TIE_CAPPED.len() as u32 },
            W_A1_TIE_MAX_TURNS,
            &mut |_stats, _arr| Ok(()),
        )
        .expect("at its own ceiling the cell completes");

        let seeds = w_a1_scheduled_seeds_at(W_A1_TIE_MASTER, spec.units);
        let expected: Vec<CappedGame> = W_A1_TIE_CAPPED
            .iter()
            .map(|&(unit, arrangement)| CappedGame {
                seed: seeds[unit],
                arrangement,
                turns: W_A1_TIE_MAX_TURNS + 1,
            })
            .collect();
        assert_eq!(report.capped, expected, "the fixture's recorded capped set");

        let void: std::collections::BTreeSet<usize> =
            W_A1_TIE_CAPPED.iter().map(|&(unit, _)| unit).collect();
        assert_eq!(report.units_void, void.len() as u32);
        assert_eq!(report.units_read, spec.units - void.len() as u32);

        // THE INDEPENDENT FOLD, over the READ units alone — the units the
        // recorded capped set does not name.
        let mut read_top_vp_ties = 0u32;
        let mut read_ties_to_seat0 = 0u32;
        for (unit, &seed) in seeds.iter().enumerate() {
            if void.contains(&unit) {
                continue;
            }
            for arrangement in 0..spec.players {
                let run = run_world_capped(
                    &WorldSpec { x: spec.x, y: spec.y, seed, players: spec.players, arrangement },
                    &rules,
                    true,
                    W_A1_TIE_MAX_TURNS,
                )
                .expect("the world plays");
                let DiagnosticOutcome::Terminated(stats) = run.game.outcome else {
                    panic!("unit {unit} arrangement {arrangement} is a READ unit's game");
                };
                let standings = &stats.result.standings;
                if standings.len() >= 2
                    && standings[0].breakdown.total == standings[1].breakdown.total
                {
                    read_top_vp_ties += 1;
                    if stats.result.winner == PlayerId(0) {
                        read_ties_to_seat0 += 1;
                    }
                }
            }
        }

        // ANTI-VACUITY: the whole point of this fixture is that these are NOT
        // zero, so the equalities below compare something.
        assert!(
            read_top_vp_ties > 0 && read_ties_to_seat0 > 0,
            "the tie fixture must carry a shared top VP total in a READ unit \
             (top_vp_ties {read_top_vp_ties}, ties_to_seat0 {read_ties_to_seat0})",
        );

        assert_eq!(report.ties_to_seat0, read_ties_to_seat0, "ties decided for seat 0");
        let rows = report.cross_checks.join("\n");
        assert!(
            rows.contains(&format!("top-VP ties: {read_top_vp_ties}/{} games", report.games)),
            "the top-VP row reads over the READ units: {rows}",
        );
        assert!(
            rows.contains(&format!(", {read_ties_to_seat0} decided for seat 0")),
            "{rows}",
        );

        // THE EXCLUSION ITSELF: a VOID unit's completed game had a shared top VP
        // total too. Replaying it is what turns "the counters happen to agree"
        // into "a tie was dropped BECAUSE its unit was void" — without this the
        // cell would pass on a fixture where no void unit ever tied.
        let (void_unit, void_arrangement) = W_A1_TIE_IN_A_VOID_UNIT;
        assert!(void.contains(&void_unit), "the named unit must be VOID");
        let run = run_world_capped(
            &WorldSpec {
                x: spec.x,
                y: spec.y,
                seed: seeds[void_unit],
                players: spec.players,
                arrangement: void_arrangement,
            },
            &rules,
            true,
            W_A1_TIE_MAX_TURNS,
        )
        .expect("the world plays");
        let DiagnosticOutcome::Terminated(stats) = run.game.outcome else {
            panic!("the void unit's OTHER game is the one that caps");
        };
        let standings = &stats.result.standings;
        assert!(
            standings.len() >= 2 && standings[0].breakdown.total == standings[1].breakdown.total,
            "the void unit's completed game must SHARE its top VP total — that is the \
             fixture's whole content",
        );
        assert_eq!(stats.result.winner, PlayerId(0), "and be decided for seat 0");
    }

    /// **The explanation identity stays TOTAL over a capped game.**    /// **The explanation identity stays TOTAL over a capped game.** Spec
    /// §0.3(b)'s `x_explanations == Some(x_applied_moves)` is a claim about
    /// every move X played, and a capped game's moves are moves X played.
    ///
    /// This cell is the decisive pin that the two halves are folded TOGETHER:
    /// `greedy-v1` explains exactly once per move it plays, so a loop that
    /// folded the capped game's `moves_by_seat` but not its explanation handle
    /// (or the reverse) breaks the equality. Its capped game is the unit's
    /// FIRST — the mirror of the rung fixture — so the unit's SECOND game
    /// completes after the cap and proves a void unit's other arrangement is
    /// still played.
    #[test]
    fn the_explanation_identity_covers_a_mixed_cells_capped_game() {
        let rules = Rules::standard();
        let spec = CellSpec {
            name: "w-a1-mixed-greedy".to_string(),
            players: 2,
            x: Contestant::GREEDY_V1_DEFAULT,
            y: Contestant::Policy(RolloutPolicy::RetreatAverse),
            units: W_A1_GREEDY_UNITS,
            master: SeedScheduleKind::Diagnostic(W_A1_MASTER),
        };
        let mut sink_calls = 0u32;
        let (report, _wall) = run_cell_loop(
            &spec,
            &rules,
            true,
            CapPolicy::Count { ceiling: 1 },
            W_A1_MIXED_MAX_TURNS,
            &mut |_stats, _arr| {
                sink_calls += 1;
                Ok(())
            },
        )
        .expect("a cell at its own ceiling completes");

        assert_eq!(
            report.capped,
            vec![CappedGame {
                seed: w_a1_scheduled_seeds(spec.units)[0],
                arrangement: 0,
                turns: W_A1_MIXED_MAX_TURNS + 1,
            }],
        );
        assert_eq!(report.units_void, 1);
        assert_eq!(report.units_read, 2);
        assert_eq!(
            sink_calls, 5,
            "the sink sees every COMPLETED game, void unit or not — five of the six scheduled",
        );
        assert_eq!(
            report.x_explanations,
            Some(report.x_applied_moves),
            "spec §0.3(b): every move X played was explained, the capped game's included",
        );
        assert!(report.x_applied_moves > 0);
    }

    /// **Pin (ii).** With the flag PRESENT and no game capped, a counting cell
    /// is the refuse-mode cell: same games, same units, same interval, same
    /// counters. The only declared differences are the echo's ONE trailing
    /// token and the five void fields, which now state the mode instead of
    /// carrying refuse mode's constants.
    ///
    /// The comparison strips those six keys and compares the rest as a
    /// serialized value rather than field by field, so it keeps its meaning the
    /// next time [`CellReport`] grows.
    #[test]
    fn counting_mode_with_zero_capped_games_is_field_equal_to_refuse_mode() {
        const CEILING: u32 = 4;
        let rules = Rules::standard();
        let spec = w_a1_spec("w-a1-zero-caps", W_A1_MIXED_UNITS);

        let refused = run_cell(&spec, &rules).expect("no game of this cell caps at the shipped guard");
        let (counted, _wall) =
            run_cell_counting_caps_timed(&spec, &rules, true, CEILING, &mut |_stats, _arr| Ok(()))
                .expect("the same cell, counting");

        assert!(
            counted.capped.is_empty(),
            "this fixture caps NOTHING at the shipped guard — without that the \
             comparison below would be measuring the void semantics, not their absence",
        );
        assert_eq!(counted.capped_ceiling, Some(CEILING));
        assert_eq!(counted.units_scheduled, spec.units);
        assert_eq!(counted.units_void, 0);
        assert_eq!(counted.units_read, spec.units);
        assert_eq!(
            counted.spec_echo,
            format!("{} capped-ceiling={CEILING}", refused.spec_echo),
            "the flag adds exactly ONE trailing token to the echo",
        );

        let strip = |report: &CellReport| {
            let mut value = serde_json::to_value(report).unwrap();
            let map = value.as_object_mut().expect("a CellReport serializes as an object");
            for key in [
                "spec_echo",
                "capped_ceiling",
                "units_scheduled",
                "units_void",
                "units_read",
                "capped",
            ] {
                assert!(map.remove(key).is_some(), "{key} is not a CellReport field");
            }
            value
        };
        assert_eq!(
            strip(&refused),
            strip(&counted),
            "counting a cell that caps nothing must not move a single figure",
        );
    }

    /// **Pin (i), the refusal half.** Refuse mode is not merely the default: a
    /// cell that caps under it still fails whole, through the SAME
    /// [`ArenaError::CapReached`] whose operator-facing message the V4 step-11
    /// record quotes verbatim.
    #[test]
    fn a_refuse_mode_cell_that_caps_still_refuses_with_cap_reached() {
        let rules = Rules::standard();
        let spec = w_a1_spec("w-a1-refuse-caps", 1);

        let err = run_cell_loop(
            &spec,
            &rules,
            true,
            CapPolicy::Refuse,
            W_A1_ALL_VOID_MAX_TURNS,
            &mut |_stats, _arr| Ok(()),
        )
        .expect_err("a 50-turn guard cannot contain this cell's games");

        let (seed, turns) = match err {
            ArenaError::CapReached { seed, turns } => (seed, turns),
            other => panic!("refuse mode must still refuse through CapReached, got {other:?}"),
        };
        assert_eq!(turns, W_A1_ALL_VOID_MAX_TURNS + 1, "the cap fires one past the guard");
        assert_eq!(
            ArenaError::CapReached { seed, turns }.to_string(),
            format!("turn cap reached at turn {turns} (world seed {:#018x})", seed.0),
            "the message every cell, artifact and ledger row already quotes",
        );
    }
}
