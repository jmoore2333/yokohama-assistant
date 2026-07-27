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

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use super::{
    agent_stream_seed, mix, run_arena_game, unit_for, ArenaAgent, ArenaError, ArenaGameStats,
    ControllerProvenance, PolicyAgent, Slot,
};
use crate::game::eval::WeightTable;
use crate::game::greedy::{self, ExplanationTally, GreedyAgent, DEFAULT_RETREAT_SAMPLES};
use crate::game::lookahead::{
    self, LookaheadAgent, SearchScheme, SearchTally, DEFAULT_ATOMIC_DEPTH, DEFAULT_BEAM,
    DEFAULT_WORLDS,
};
use crate::game::metrics::{nearest_rank_index, wilson_95, TriggerTally, Z_95};
use crate::game::policy::RolloutPolicy;
use crate::game::rules::Rules;
use crate::game::seeds::SeedSchedule;
use crate::types::PlayerId;

/// Domain salt for the bootstrap resample stream.
const SALT_BOOTSTRAP: u64 = 0xA3E1_B007_0000_0004;

/// The contestant under test. Every other slot in the unit is Y — the
/// single-champion shape [`unit_for`]'s rotations are built for, and the reason
/// the null value is `1/players` rather than 0.5.
const X_SLOT: Slot = Slot(0);

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
/// [`WeightTable::V1`] the shipped agents run, and a weight change is a version
/// bump (spec §0.5 / L12), not a cell parameter. `weights=<identity>` still
/// rides the params map so a corpus reader can tell two tables apart without
/// consulting a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contestant {
    /// A 3.2 rollout rung, played by [`PolicyAgent`] exactly as before.
    Policy(RolloutPolicy),
    /// Greedy-v1 (spec §3): one-ply argmax of the evaluator. `m` is the retreat
    /// sample count.
    GreedyV1 { m: u8 },
    /// Lookahead-v1 (spec §4.2), the S1 turn-boundary rung: beam `b`, `k`
    /// sampled worlds, `m` retreat samples.
    LookaheadV1 { b: u16, k: u8, m: u8 },
    /// The S2 DIAGNOSTIC arm (spec §4.1): atomic depth `d` in moves, with
    /// quiescence. Named `-diag` in provenance because it is preserved as a rung
    /// only if T7 flips.
    AtomicDiag { d: u8, k: u8, m: u8 },
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

/// A cloneable read-out on ONE built contestant's explanation counter.
///
/// The two evaluator tallies are deliberately different types (a search has
/// nodes and continuation lengths; a one-ply argmax has neither), so this is the
/// narrow thing the driver actually needs from both: spec §0.3(b)'s count. The
/// Policy rungs explain nothing and produce no handle at all — `Option::None` is
/// the honest shape, not a zero counter that would suggest the rung had an
/// explanation path and simply never took it.
#[derive(Clone)]
pub enum ExplainHandle {
    Greedy(ExplanationTally),
    Search(SearchTally),
}

impl ExplainHandle {
    /// Decisions this contestant explained. Equal to its applied-move count on a
    /// completed game — neither evaluator agent can return a move without
    /// explaining it, and a failed `choose` fails the run.
    pub fn explained(&self) -> u64 {
        match self {
            Self::Greedy(t) => t.explained(),
            Self::Search(t) => t.explained(),
        }
    }
}

impl Contestant {
    /// Greedy-v1 at its shipped knob.
    pub const GREEDY_V1_DEFAULT: Self = Self::GreedyV1 { m: DEFAULT_RETREAT_SAMPLES };
    /// Lookahead-v1 (S1) at its shipped knobs.
    pub const LOOKAHEAD_V1_DEFAULT: Self =
        Self::LookaheadV1 { b: DEFAULT_BEAM, k: DEFAULT_WORLDS, m: DEFAULT_RETREAT_SAMPLES };
    /// The S2 diagnostic arm at its shipped knobs.
    pub const ATOMIC_DIAG_DEFAULT: Self =
        Self::AtomicDiag { d: DEFAULT_ATOMIC_DEPTH, k: DEFAULT_WORLDS, m: DEFAULT_RETREAT_SAMPLES };

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
                greedy::controller_provenance(&WeightTable::V1, m, agent_seed)
            }
            Self::LookaheadV1 { b, k, m } => lookahead::controller_provenance(
                SearchScheme::TurnBoundary { beam: b },
                k,
                m,
                &WeightTable::V1,
                agent_seed,
            ),
            Self::AtomicDiag { d, k, m } => lookahead::controller_provenance(
                SearchScheme::AtomicDiag { depth: d },
                k,
                m,
                &WeightTable::V1,
                agent_seed,
            ),
        }
    }

    /// `(name, version, params)` — [`provenance`](Self::provenance) without the
    /// stream seed, in [`RolloutPolicy::descriptor`]'s shape so the echo
    /// formatter is one function for every contestant.
    pub fn descriptor(&self) -> (String, u32, BTreeMap<String, i64>) {
        let p = self.provenance(None);
        (p.name, p.version, p.params)
    }

    /// Which starting-order rule this contestant plays (spec §5.5).
    pub fn keep_kind(&self) -> KeepKind {
        match self {
            Self::Policy(_) => KeepKind::SeatKeyedUniform,
            Self::GreedyV1 { .. } | Self::LookaheadV1 { .. } | Self::AtomicDiag { .. } => {
                KeepKind::EvaluatorScored
            }
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
                let agent = GreedyAgent::with_weights(rules, WeightTable::V1, m, agent_seed);
                let handle = ExplainHandle::Greedy(agent.tally());
                (Box::new(agent), Some(handle))
            }
            Self::LookaheadV1 { b, k, m } => {
                Self::build_search(rules, SearchScheme::TurnBoundary { beam: b }, k, m, agent_seed)
            }
            Self::AtomicDiag { d, k, m } => {
                Self::build_search(rules, SearchScheme::AtomicDiag { depth: d }, k, m, agent_seed)
            }
        }
    }

    fn build_search(
        rules: &Rules,
        scheme: SearchScheme,
        worlds: u8,
        retreat_samples: u8,
        agent_seed: u64,
    ) -> (Box<dyn ArenaAgent>, Option<ExplainHandle>) {
        let agent = LookaheadAgent::with_knobs(
            rules,
            scheme,
            worlds,
            retreat_samples,
            WeightTable::V1,
            agent_seed,
        );
        let handle = ExplainHandle::Search(agent.tally());
        (Box::new(agent), Some(handle))
    }
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
    /// End-trigger counts over the cell's games — one player count per tally,
    /// as [`TriggerTally`] requires.
    pub trigger: TriggerTally,
    /// The master this cell actually ran on: the committed one for
    /// [`SeedScheduleKind::Diagnostic`], the DRAWN one for
    /// [`SeedScheduleKind::OsEntropy`].
    pub master: u64,
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
/// is structural rather than checked: a cap hit or a driver error is an `Err`
/// from the driver and short-circuits this loop, so a failed game has no
/// [`ArenaGameStats`] to hand anyone (spec §4's failure taxonomy — see that
/// type's doc).
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
    let mut x_unit_shares: Vec<f64> = Vec::with_capacity(spec.units as usize);
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
    // explained nothing" (see `CellReport::x_explanations`).
    let mut x_explanations: Option<u64> = None;

    for _ in 0..spec.units {
        let seed = schedule.next_world();
        let unit = unit_for(players, seed);
        let mut unit_x_wins = 0u32;

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
            let mut stats = {
                let handles = &mut x_handles;
                let mut make_agent = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
                    let slot = seating[seat.0 as usize];
                    let contestant = if slot == X_SLOT { x } else { y };
                    let (agent, handle) =
                        contestant.build(rules, agent_stream_seed(seed, seat, slot));
                    if slot == X_SLOT {
                        handles.extend(handle);
                    }
                    agent
                };
                run_arena_game(seed, players, rules, &mut make_agent, invariants)?
            };
            // The RUN half of provenance (spec §5): the driver stamped who
            // played each seat, and this is the only layer that knows which
            // schedule and which cell they played for.
            stats.record.set_run_context(master, &spec.name);
            sink(&stats, arr as u8)?;

            games += 1;
            trigger.observe(stats.trigger.as_ref());

            // Spec §5.4's driver extension, aggregated two ways because they
            // answer two questions: `applied_by_seat` follows the SEAT (a design
            // balance read), `x_applied_moves` follows the CONTESTANT through the
            // rotation (spec §0.3(b)'s right-hand side).
            for (seat, moves) in stats.moves_by_seat.iter().enumerate() {
                applied_by_seat[seat] += moves;
            }
            x_applied_moves += stats.moves_by_seat[x_seat.0 as usize];
            for handle in &x_handles {
                *x_explanations.get_or_insert(0) += handle.explained();
            }

            if stats.result.winner == x_seat {
                unit_x_wins += 1;
                x_game_wins += 1;
            }
            if stats.result.winner == PlayerId(0) {
                seat0_wins += 1;
            }

            // `final_result` (game::scoring) is a TOTAL order: it sorts on
            // `(Reverse(total), clockwise distance from start_player)`, and the
            // distance `(seat + n - start) % n` is a bijection from seat onto
            // 0..n, so no two players can share a sort key. A game therefore
            // NEVER reports a tie — the clockwise tie-break has already decided
            // it, and `winner` is always exactly one seat. What is recorded here
            // is the tie-break's WORK: how often the top VP total was shared at
            // all, and how often seat 0 was the one it awarded.
            let standings = &stats.result.standings;
            let top_shared = standings.len() >= 2
                && standings[0].breakdown.total == standings[1].breakdown.total;
            if top_shared {
                top_vp_ties += 1;
                if stats.result.winner == PlayerId(0) {
                    ties_to_seat0 += 1;
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
            vp_margin_sum += x_total - y_total / f64::from(players - 1);
        }

        x_unit_shares.push(f64::from(unit_x_wins) / unit.seatings.len() as f64);
    }

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

    Ok(CellReport {
        spec_echo: spec_echo(spec, master, resample_seed),
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
        trigger,
        master,
    })
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

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Sample standard deviation (n − 1 denominator), or `NaN` under two units.
fn sample_sd(values: &[f64]) -> f64 {
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

/// `name@vN` or `name@vN{k=v,...}` — one formatter for every contestant, fed by
/// [`Contestant::descriptor`] so a rung's echo is byte-for-byte what it was
/// before the widening (`tests/contestant_golden.rs` pins the strings).
fn contestant_echo(contestant: Contestant) -> String {
    let (name, version, params) = contestant.descriptor();
    if params.is_empty() {
        return format!("{name}@v{version}");
    }
    let knobs: Vec<String> = params.iter().map(|(k, v)| format!("{k}={v}")).collect();
    format!("{name}@v{version}{{{}}}", knobs.join(","))
}

fn spec_echo(spec: &CellSpec, master: u64, resample_seed: u64) -> String {
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
    format!(
        "cell={} players={} x={} y={} units={} unit={} games={} \
         master={master_kind}:{master:#018x} bootstrap_B={BOOTSTRAP_REPLICATES} \
         resample_seed={resample_seed:#018x} single-threaded=yes fixed-N=yes \
         setup-identical={setup_identical}",
        spec.name,
        spec.players,
        contestant_echo(spec.x),
        contestant_echo(spec.y),
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
        let echo = spec_echo(&spec, MASTER, 0xDEAD_BEEF);

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
        let (_, _, params) = Contestant::LookaheadV1 { b: 2, k: 0, m: 3 }.descriptor();
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
        let echo = spec_echo(&homogeneous, MASTER, 0);
        assert!(echo.contains("setup-identical=yes"), "{echo}");

        let mixed = CellSpec { x: Contestant::GREEDY_V1_DEFAULT, ..homogeneous.clone() };
        let echo = spec_echo(&mixed, MASTER, 0);
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
            x: Contestant::LookaheadV1 { b: 1, k: 1, m: 2 },
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
        assert!(
            report.spec_echo.contains("x=lookahead-v1@v1{"),
            "the X seat's provenance must be the search rung's descriptor: {}",
            report.spec_echo,
        );
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
        for seat in 0..4u8 {
            for card in 1..=18u8 {
                let deal = [OrderCardId(card), OrderCardId(card + 18)];
                let g = greedy.choose_starting_order(PlayerId(seat), &deal);
                let a = s1.choose_starting_order(PlayerId(seat), &deal);
                let b = s2.choose_starting_order(PlayerId(seat), &deal);
                assert!(
                    g == a && a == b,
                    "the three evaluator contestants' Keeps diverged at seat {seat}, \
                     deal {deal:?}: greedy {g}, s1 {a}, s2 {b} — KeepKind::EvaluatorScored \
                     (and every setup-identical=yes it prints) rests on their agreement",
                );
            }
        }
    }
}
