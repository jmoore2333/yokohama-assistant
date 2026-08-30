//! The strength court (Phase 3.2, spec
//! `docs/plans/2026-07-25-phase3.2-arena-spec.md` §4): honest per-seat play,
//! typed hard errors, and the design units a strength claim is tabulated over.
//!
//! WHAT MAKES THIS DIFFERENT FROM `game::agent`'s driver. `run_honest_game` is
//! a 3.0 surface and stays untouched, but the arena cannot build on it for two
//! reasons that are the whole point of this module:
//!
//!   1. **It materializes.** `HonestAgent::choose_move` takes `&[Move]`, so
//!      every decision pays the exhaustive union — the 442,368-member retreat
//!      product included. The arena drives the FACTORED surface through a
//!      [`DecisionHandle`], which is what makes a weighted policy affordable.
//!   2. **It falls back.** `TrivialHonestAgent` silently plays from the TRUE
//!      legal list when `BeliefSampler::from_view` errors, recording the fact
//!      only in a tally. That is privileged play, and an arena inheriting it
//!      would emit privileged games into the corpus 3.3 trains on, invisibly to
//!      the leakage suite. **Here a sampler refusal is a typed HARD error and
//!      there is no fallback arm to reach** — not a counter, an absence.
//!
//! THE HONEST CHAIN, per decision (spec §4): `observe(true_state, seat)` →
//! the seat's agent determinizes ONE world from its own view →
//! [`DecisionHandle`] over that world → [`RolloutPolicy::pick`] →
//! `move_legality::is_move_legal(true_state, .., PathMode::Canonical)` (false =
//! [`ArenaError::IllegalPick`], an arena bug, never a retry) → a
//! `debug_assertions`-only membership cross-check against `get_legal_moves` →
//! `apply_move_unchecked` → record the event. The volume metric is read from
//! the SAME handle the policy sampled, so a decision point costs exactly one
//! route enumeration (L10).
//!
//! WHY THE DEBUG CROSS-CHECK EXISTS. `PathMode::Canonical` is deliberately
//! WIDER than list membership — it does not verify lex-least witness selection
//! — so a non-canonical `MovePresident` witness would pass validation here and
//! then be REJECTED by the checked `apply_move` at strict replay. The debug
//! assert catches that regression per-edit; the release-side closing authority
//! is the corpus strict-replay check (spec §5), which pushes every emitted move
//! through the checked boundary.
//!
//! NO L10 COUNTER ASSERTIONS LIVE HERE (T2 review, carried). The membership
//! cross-check is itself a `get_legal_moves` call and the volume cross-check
//! (T6 review F3) builds a fresh true-state handle, so at an on-board Movement
//! point this driver legitimately runs THREE route DFSes in debug — the
//! agent's, the membership assert's, and the volume assert's. In RELEASE both
//! asserts compile out and the count is exactly the agent's one.
//! `tests/l10_enumeration_pin.rs` owns the counter property over its own
//! harness loop; arming it against this driver would over-read and be wrong.
//!
//! SEED KEYING (spec §4, E2's CRN rule) is the trap this module is shaped
//! around, and the two streams are keyed DIFFERENTLY on purpose:
//!
//!   * **Setup: keyed by (world seed, seat).** Both seatings of a unit must
//!     reach a bit-identical post-`finalize_setup` state, which is only true if
//!     the starting-order choice at a seat does not depend on WHICH contestant
//!     sits there. [`PolicyAgent::choose_starting_order`] therefore draws from
//!     a stream derived from `(seat, its own deal)` — a deterministic function
//!     of `(world seed, seat)` that the agent can compute without ever being
//!     handed the world seed, which IS the whole hidden set.
//!   * **Play: keyed by (world seed, seat, contestant slot)** —
//!     [`agent_stream_seed`]. Keying by seat ALONE is E2's measured trap: the
//!     two games of a pair become the same game, the null cell reads
//!     ρ = −1.0000 with SE 0.000, and the instrument reports "perfectly
//!     calibrated" while measuring nothing.
//!
//! **PAIRING SCOPE — WHAT SURVIVES A MIXED-KEEP CELL (Phase 3.3, spec §5.5,
//! ledger row 15).** The setup bullet above says "bit-identical post-setup
//! state", and from 3.3 that holds only in a **Keep-HOMOGENEOUS** cell — one
//! whose two contestants share a [`KeepKind`]. The rungs all draw the Keep from
//! the `(seat, deal)`-keyed stream; the evaluator contestants take
//! `greedy::score_starting_order`'s argmax instead, because the map mandates an
//! evaluator-informed Keep and the rejected alternative — every rung adopting
//! `score_starting_order` — would silently change every 3.2 gate cell's
//! trajectories and invalidate the recorded figures. So a **mixed-Keep cell's
//! unit is seed-paired and seat-SWAPPED at `new_game` but NOT setup-identical**:
//! the two seatings resolve the same deal two ways.
//!
//! That is acceptable, and on ledger row 15's own measurement it costs nothing
//! the design was relying on: **pairing is BOOKKEEPING** (variance reduction
//! ~0 at 2P, negative at 4P), and it is the SWAP — each contestant at each seat
//! — that removes the seat effect the unit exists to remove. The unit bootstrap
//! over units stays valid either way. What is scoped rather than repealed:
//!
//!   * `paired_seatings_share_a_bit_identical_starting_state` is KEPT and now
//!     asserts its own Keep-homogeneity precondition first;
//!   * `the_deal_at_each_seat_is_seating_invariant_in_a_mixed_keep_cell` holds
//!     the surviving half in mixed cells — the DEAL at a seat is a function of
//!     `(world seed, seat)`, so it does not move when the contestants swap, even
//!     though the KEEPS taken from it may;
//!   * every `CellReport`'s `spec_echo` prints `setup-identical=yes|no`, so a
//!     quoted figure carries which kind of unit produced it.
//!
//! The §10.2 null cell is Keep-homogeneous by construction, so the standing
//! self-test keeps its full-strength property.
//!
//! SUB-SEED DERIVATION: every stream seed is `mix(salt, components)` —
//! `acc = salt`, then `acc = splitmix64(acc ^ component)` per component in
//! order (`seeds::splitmix64`, the same primitive `SeedSchedule` streams with,
//! so there is one mixing authority and not a re-typed copy of its constants).
//! Each step is a bijection in `acc`, so the derivation is INJECTIVE IN ANY ONE
//! COMPONENT with the others held fixed — the property every keying test needs.
//! Two lists differing in SEVERAL components can collide at the ~2^-64 density
//! of any 64-bit hash (the T6 review computed a two-component counterexample),
//! so cross-component collision-freedom is probabilistic, not constructed.
//! Distinct salts keep the setup and play streams in separate domains.
//!
//! WHAT THE DERIVATION DOES NOT BUY. It is public arithmetic over the world
//! seed, so an ADVERSARIAL agent handed its own seed could invert it and
//! recover the world seed. That grants nothing the public board does not
//! already grant — `new_game` builds `board_layout` from the setup seed, and
//! `t8f_red_team_the_public_board_still_recovers_a_small_setup_seed` recovers
//! it in milliseconds (map §8 row 8's PERMANENT residual). Stated so nobody
//! mistakes seed derivation for a confidentiality mechanism.
//!
//! BOTS NEVER RECEIVE [`ArenaGameStats`]. It carries a `GameRecord`, and a
//! record carries the seed and every choice — i.e. the whole hidden set. Stats
//! flow to the harness that tabulates them, never back into an agent; that is
//! the same review-enforced rule `game::agent`'s doc states for records.
//!
//! TABULATION LIVES IN [`stats`] — cells, design units, and the interval
//! discipline that turns these games into a strength claim. It is a submodule
//! rather than more of this file because the two halves answer different
//! questions and are reviewed against different sources: this file against the
//! rules and the honesty boundary, [`stats`] against spec §4's tabulation block
//! and §0-R3. Everything it exports is re-exported here, so callers see one
//! `arena` surface.
//!
//! EMISSION LIVES IN [`corpus`], on the same split and for the same reason: the
//! corpus layout, the compact-JSON writer, and the rule that keeps privileged
//! play out of training data are reviewed against spec §5. What this file owns
//! is the STAMP — [`drive`] fills `GameRecord.run` with one
//! [`SeatProvenance`](crate::game::record::SeatProvenance) per seat, taking each
//! controller's name and version from THAT CONTROLLER'S OWN descriptor (never a
//! hand-typed string): [`RolloutPolicy::descriptor`] for a rung,
//! `greedy::controller_provenance` / `lookahead::controller_provenance` for the
//! evaluator contestants, each reading its own name and impl-version const per
//! spec §0.5. Each seat's `observation` comes from WHICH DRIVER built it. That
//! last split is the honesty boundary showing up in the artifact: an agent
//! states what it is, and the driver — the only party that knows whether it
//! passed a `PlayerView` or the true state — states what it could see. The
//! evaluator contestants run the honest chain through [`AgentBench`], so their
//! seats are labelled `view` and their records file into the corpus like any
//! other seat's.

pub mod corpus;
/// The V4 step-11 diagnosis wave's read surfaces (W-D1) — the `world` and
/// `caprate` instruments and their help text. PUBLIC because `examples/arena`
/// is print-only glue over them (the `driven_runner` precedent): the parse and
/// render logic is lib-tested here, so the runner needs no test target of its
/// own.
pub mod diagnostic;
pub mod stats;
/// The contestant name grammar (Phase 3.5c W1-B). PRIVATE, deliberately: the
/// public entry is [`Contestant::from_str_name`], so there is exactly ONE way to
/// turn a typed spec into a contestant and no second spelling to drift from it.
/// (`grammar` at the crate root would also be far too generic a name — the same
/// reason `eval`/`greedy`/`lookahead`/`mcts` are namespaced in `game/mod.rs`.)
mod grammar;
pub use corpus::*;
pub use diagnostic::*;
pub use grammar::ATOMIC_DIAG_ALIAS;
pub use stats::*;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::game::agent::HONEST_MAX_TURNS;
use crate::game::apply_move::apply_move_unchecked;
use crate::game::belief::BeliefSampler;
use crate::game::end_game::is_game_over;
use crate::game::invariants::check_invariants;
use crate::game::metrics::{decision_volume, CoverageTally, VolumeObs};
use crate::game::move_legality::{is_move_legal, PathMode};
use crate::game::observe::observe;
use crate::game::policy::RolloutPolicy;
use crate::game::record::{
    GameRecord, SeatProvenance, OBSERVATION_FULL_STATE_DIAGNOSTIC, OBSERVATION_VIEW,
};
use crate::game::rules::Rules;
use crate::game::scoring::{final_result, GameResult};
use crate::game::search_actions::DecisionHandle;
use crate::game::seeds::{splitmix64, WorldSeed};
use crate::game::setup::{finalize_setup, new_game, PendingSetup, StartingOrderChoice};
use crate::types::{EndGameTrigger, GameConfig, GameState, Move, OrderCardId, PlayerId, PlayerView};

// ---------------------------------------------------------------------------
// Sub-seed derivation
// ---------------------------------------------------------------------------

/// Domain salt for the play stream (world seed, seat, contestant slot).
const SALT_PLAY_STREAM: u64 = 0xA3E1_5701_0000_0001;
/// Domain salt for the setup stream (world seed, seat — carried as the deal).
const SALT_SETUP_STREAM: u64 = 0xA3E1_5E70_0000_0002;
/// Domain salt for the raw diagnostic driver's per-seat stream.
const SALT_RAW_STREAM: u64 = 0xA3E1_4157_0000_0003;
/// Domain salt for the DRIVEN adjudicator's per-replication world (design
/// `docs/plans/2026-08-17-phase-b-harness-design.md` §4):
/// `mix(SALT_DRIVEN_REPLICATION, [master, replication_index])` is the world one
/// replication of `game::driven::adjudicate_root` plays in, and every seat's
/// stream then descends from THAT world through [`agent_stream_seed`] — so the
/// harness invents no third seed discipline, it re-enters this one at a derived
/// world. `pub(crate)` for [`mix`]'s own reason: the deriving code is a sibling
/// module, and "one mixing authority, never a re-typed constant" is a rule
/// across the crate rather than inside this file. The serial is this family's
/// next free one (0004 is `arena::stats`' bootstrap, 0005 `eval_screen`'s,
/// 0006 `SALT_DRIVEN_REPLICATION` below, 0007 `driven::SALT_DRIVEN_RACE_PROBE`,
/// 0008 `driven_families::SALT_DRIVEN_FAMILY_READ` — the family home's index,
/// swept complete at the Phase-C close).
pub(crate) const SALT_DRIVEN_REPLICATION: u64 = 0xA3E1_D21E_0000_0006;

/// Fold `components` into one full-width sub-seed under `salt` — the module
/// doc's derivation. Injective in any single component (others fixed); see the
/// module doc for what is NOT claimed across several.
///
/// `pub(crate)` rather than private since Phase 3.5c: `game::eval_screen` is a
/// SIBLING module that derives its own bootstrap resample stream, and the module
/// doc's "one mixing authority and not a re-typed copy of its constants" rule
/// applies across the crate, not merely inside this file. It stays crate-private
/// — sub-seed derivation is not a public surface.
pub(crate) fn mix(salt: u64, components: &[u64]) -> u64 {
    let mut acc = salt;
    for &component in components {
        let mut step = acc ^ component;
        acc = splitmix64(&mut step);
    }
    acc
}

/// The seed for the agent playing `slot` at `seat` in the game `seed`.
///
/// All THREE components enter (spec §4). The slot component is what makes E2's
/// trap structurally unreachable: without it, swapping the seating leaves every
/// seat's stream unchanged, so both games of a pair replay identically and the
/// unit's outcome carries no information. The seat component keeps the two
/// games' streams independent rather than merely permuted — pairing is kept for
/// bias removal and bit-identical starts and is explicitly NOT claimed as
/// variance reduction (E2: 0–2.5% at 2P, −13% at 4P), so correlating a
/// contestant's own randomness across the pair would buy nothing and inflate
/// the unit variance.
pub fn agent_stream_seed(seed: WorldSeed, seat: PlayerId, slot: Slot) -> u64 {
    mix(SALT_PLAY_STREAM, &[seed.0, u64::from(seat.0), u64::from(slot.0)])
}

/// The seat's setup stream, derived from what the agent legitimately holds at
/// that moment: its seat and its own two dealt cards.
///
/// The deal is a function of `(world seed, seat)`, so this IS spec §4's
/// seat-keyed setup stream — computed WITHOUT handing the agent the world seed,
/// which would hand it the hidden set outright. It is not injective in the
/// world seed (two games dealing a seat the same two cards share the stream),
/// which is harmless: the only decision it drives is which of exactly those two
/// cards to keep.
fn setup_stream_seed(seat: PlayerId, deal: &[OrderCardId; 2]) -> u64 {
    mix(
        SALT_SETUP_STREAM,
        &[u64::from(seat.0), u64::from(deal[0].0), u64::from(deal[1].0)],
    )
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why an arena game did not produce a completed, recorded result — and, for
/// the two counting-mode arms A-S11-1 added, why a whole CELL stopped.
///
/// Every variant is a HARD stop that emits nothing — spec §4's failure
/// taxonomy. A driver error is a bug and fails the run loudly;
/// [`CapReached`](Self::CapReached) is a DISTINGUISHED outcome that also fails
/// an asserting cell but names its seed so the run is reproducible (uniform
/// tails run to 5,364 of the 10,000 cap in E1's held-out cells).
///
/// **AMENDED at V4 step 11 by A-S11-1 (freeze doc
/// `docs/plans/2026-08-30-phase3.5c-v4-identity-freeze.md` §9.6), and the
/// amendment is narrower than it looks.** `CapReached` is the REFUSE mode's
/// arm, which is what every public driver and every unflagged cell still
/// raises. A cell run in the opt-in COUNTING mode
/// ([`CapPolicy::Count`](stats::CapPolicy::Count)) does not raise it at all: a
/// capped game is a typed, counted event there, and the two arms that can end
/// such a cell — [`CappedCeilingExceeded`](Self::CappedCeilingExceeded) and
/// [`NoReadUnits`](Self::NoReadUnits) — are INSTRUMENT GUARDS that carry the
/// counted games with them. Every other variant still refuses a whole cell,
/// emitting nothing, in either mode.
///
/// CLASSIFICATION RULE, stated because it is not inferable: [`ArenaAgent`]'s
/// error is an untyped `String`, so the driver cannot re-classify what an agent
/// refused about. **Every agent refusal in [`run_arena_game`] lands in
/// [`SamplerFailed`](Self::SamplerFailed) with its detail preserved verbatim**,
/// including [`PolicyAgent`]'s own dead-end string (whose text says so).
/// [`DeadEnd`](Self::DeadEnd) is raised where the driver IS the picker —
/// [`run_raw_game`], whose `pick` returning `None` is a dead end it observed
/// itself rather than one it was told about.
#[derive(Debug)]
pub enum ArenaError {
    /// A seat's agent refused to produce a move. The honest chain's typed hard
    /// error; the commonest cause is `BeliefSampler::from_view`.
    SamplerFailed { seat: PlayerId, detail: String },
    /// A seat proposed a move that is not legal in the TRUE state. An arena
    /// bug: the move never reaches `apply_move_unchecked`.
    IllegalPick { seat: PlayerId, mv: Box<Move> },
    /// The decision point offered nothing to play.
    DeadEnd { seat: PlayerId },
    /// The turn cap was hit before the game ended. Carries the seed so the run
    /// reproduces.
    CapReached { seed: WorldSeed, turns: u32 },
    /// **Counting mode only** (V4 step 11, amendment A-S11-1; freeze doc
    /// `docs/plans/2026-08-30-phase3.5c-v4-identity-freeze.md` §9.6 item 2): a
    /// cell's capped games passed its registered ceiling.
    ///
    /// An INSTRUMENT GUARD, outcome-independent, and raised the moment the
    /// count passes the ceiling rather than at the schedule's end — so the run
    /// is INVALID and the rest of it is not spent. Deliberately NOT
    /// [`CapReached`](Self::CapReached): a reader of a failed run must be able
    /// to tell a ceiling refusal from the un-amended refuse-at-the-first-cap
    /// rule, which a shared arm would hide. `capped` carries every capped game
    /// so far, the (c+1)-th included.
    CappedCeilingExceeded { ceiling: u32, capped: Vec<stats::CappedGame> },
    /// **Counting mode only** (A-S11-1): every scheduled design unit is void,
    /// so no unit outcome constrains θ.
    ///
    /// Typed rather than a report of NaNs: a number-shaped answer to a question
    /// the data cannot answer is the one thing this module's refusals exist to
    /// never print.
    NoReadUnits { scheduled: u32, capped: Vec<stats::CappedGame> },
    /// Setup, observation, or application failed — always a bug here, since
    /// every input was engine-produced.
    Driver(String),
}

impl std::fmt::Display for ArenaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SamplerFailed { seat, detail } => {
                write!(f, "seat {} produced no move: {detail}", seat.0)
            }
            Self::IllegalPick { seat, mv } => {
                write!(f, "seat {} proposed a move illegal in the true state: {mv:?}", seat.0)
            }
            Self::DeadEnd { seat } => write!(f, "seat {} reached a dead end", seat.0),
            Self::CapReached { seed, turns } => {
                write!(f, "turn cap reached at turn {turns} (world seed {:#018x})", seed.0)
            }
            Self::CappedCeilingExceeded { ceiling, capped } => write!(
                f,
                "capped games ({}) exceeded the registered ceiling of {ceiling}: {}",
                capped.len(),
                capped_roll(capped),
            ),
            Self::NoReadUnits { scheduled, capped } => write!(
                f,
                "no read units: all {scheduled} scheduled units are void ({} capped games), \
                 so nothing constrains theta: {}",
                capped.len(),
                capped_roll(capped),
            ),
            Self::Driver(message) => write!(f, "arena driver: {message}"),
        }
    }
}

impl std::error::Error for ArenaError {}

/// THE spelling of one capped game's figures, shared by the two surfaces that
/// print them as a FIGURE LIST: [`diagnostic::render_cap_rate`]'s per-world
/// line and [`capped_roll`] below.
///
/// One function rather than two `format!`s that happen to agree — an operator
/// carries such a line into `arena world`, and a second spelling free to drift
/// from the first is a second thing a reader has to recognize.
///
/// **A THIRD surface names a capped game and deliberately does not use this**
/// (review N1): [`stats::CellReport::capped_lines`] prints `capped: arena world
/// <x> <y> <seed> --arrangement <arr> · turns=<n>`, i.e. the reproduce COMMAND
/// with its arguments filled in, because `render_cap_rate` carries that
/// template in a group header a per-line helper has no room for. The two shapes
/// answer different questions — a figure list versus a command to paste — and
/// §9.6 does not require them identical. It takes the
/// three FIGURES rather than either surface's struct
/// ([`stats::CappedGame`] / [`diagnostic::CappedWorld`]) because the two types
/// are deliberately separate: one is a counted cell event, the other a
/// cap-rate observation, and neither should have to name the other to print.
pub(crate) fn capped_figures(seed: WorldSeed, arrangement: u8, turns: u32) -> String {
    format!("seed={:#018x} arrangement={arrangement} turns={turns}", seed.0)
}

/// The `(seed, arrangement, turns)` roll of a counting cell's capped games,
/// through [`capped_figures`] — the shared spelling.
fn capped_roll(capped: &[stats::CappedGame]) -> String {
    capped
        .iter()
        .map(|game| capped_figures(game.seed, game.arrangement, game.turns))
        .collect::<Vec<_>>()
        .join(" · ")
}

// ---------------------------------------------------------------------------
// The agent seam
// ---------------------------------------------------------------------------

/// One decision, as an agent reports it: the move plus the volume observation
/// taken at the SAME handle that produced it.
///
/// Bundling them is what holds L10 across the seam. If the driver measured the
/// volume itself it would need a handle over the true state — a second
/// enumeration per Movement decision point, and a second thing that can
/// disagree with the first.
#[derive(Debug, Clone)]
pub struct DecisionReport {
    pub mv: Move,
    pub volume: VolumeObs,
}

/// What a seat's CONTROLLER says about itself for the record's provenance
/// (spec §5) — everything a
/// [`SeatProvenance`](crate::game::record::SeatProvenance) carries EXCEPT the
/// observation label.
///
/// The observation is absent on purpose. It is the one claim in provenance that
/// must not come from the thing being labelled: only the driver knows whether it
/// handed this seat a `PlayerView` or the true `GameState`, so only the driver
/// may say. Splitting the type makes that structural instead of a convention a
/// future agent could quietly break by filling the field in itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerProvenance {
    /// The controller's stable name — a policy's `as_str()`, or `"human"`.
    pub name: String,
    /// Implementation version under that name.
    pub version: u32,
    /// Integer knobs in their declared units (ppm for a probability).
    pub params: std::collections::BTreeMap<String, i64>,
    /// The seat's own RNG stream seed, when it has one.
    pub agent_seed: Option<u64>,
}

impl ControllerProvenance {
    /// The provenance of a seat played by `policy` on stream `agent_seed`.
    ///
    /// THE construction path for a policy-driven seat: the name, the version and
    /// the knob map all come from [`RolloutPolicy::descriptor`], so no call site
    /// hand-types a controller name or re-derives a params map that could drift
    /// from the policy it claims to describe.
    pub fn from_policy(policy: RolloutPolicy, agent_seed: Option<u64>) -> Self {
        let (name, version, params) = policy.descriptor();
        Self { name, version, params, agent_seed }
    }

    /// Complete this into a `SeatProvenance` under the DRIVER's observation
    /// label (`OBSERVATION_VIEW` or `OBSERVATION_FULL_STATE_DIAGNOSTIC`).
    pub fn observed_as(self, observation: &str) -> SeatProvenance {
        SeatProvenance {
            controller: self.name,
            version: self.version,
            params: self.params,
            agent_seed: self.agent_seed,
            observation: observation.to_string(),
        }
    }
}

/// A seat in the arena. Deliberately NOT [`crate::game::agent::HonestAgent`]:
/// that trait's `choose_move` takes a materialized `&[Move]`, which is the
/// ceiling this driver exists to avoid.
///
/// The signatures are the honesty boundary — neither method can name
/// `GameState` or `PendingSetup`, so an implementation cannot read the deck
/// order, the boxed pile, or another seat's hidden starting order. The
/// construction channel is closed the same way `run_honest_game` closes it: the
/// driver invokes the factory itself, after `new_game`, passing only
/// `(seat, player_count)`.
///
/// **Spec deviation, recorded (plan Task 6; fold back at close-out).** Spec §4
/// sketched `choose_move(&mut self, view, handle: &DecisionHandle, rules) ->
/// Move`. That signature contradicts the same section's flow — the agent
/// determinizes its OWN world and builds the handle over it, so a driver-passed
/// handle has no state to be over — and it cannot express the mandated typed
/// sampler hard error. `choose(view, rules) -> Result<DecisionReport, String>`
/// implements the section's flow exactly. K>1 stays open: an implementation may
/// sample several worlds internally, which is 3.3's business. The PR #8
/// `sample_family -> Option<Move>` precedent applies.
pub trait ArenaAgent {
    /// Setup: keep dealt Order card 0 or 1. The agent sees ONLY its own deal.
    fn choose_starting_order(&mut self, seat: PlayerId, deal: &[OrderCardId; 2]) -> u8;

    /// Play: one move for `view`'s seat, or the reason there is none.
    ///
    /// An `Err` is a HARD error at the driver — never a fallback to privileged
    /// play. See [`ArenaError`]'s classification rule for where it lands.
    fn choose(&mut self, view: &PlayerView, rules: &Rules) -> Result<DecisionReport, String>;

    /// What this agent IS, for the record's provenance (spec §5).
    ///
    /// Deliberately without a default implementation. A default would let an
    /// agent nobody thought about play a game into the corpus under a generic
    /// label — and an unlabelled game in training data is worse than no game,
    /// because it is indistinguishable from a labelled one until someone tries
    /// to reproduce it. The compiler asking every implementor to answer is the
    /// cheapest possible enforcement.
    fn controller(&self) -> ControllerProvenance;
}

/// A cloneable handle on one [`PolicyAgent`]'s honest-path counter.
///
/// Clone it out BEFORE the agent is moved into the driver: a game's agents are
/// built by the driver's factory and never handed back, so this is the only way
/// to read the counter afterwards ([`crate::game::agent::AgentPathTally`] is
/// the in-tree precedent it mirrors).
///
/// There is deliberately no fallback counter to pair with it. `TrivialHonestAgent`
/// needed one because it HAS a fallback arm; here the honest chain either
/// produces a move or the run dies, so "fallback == 0" is a structural fact and
/// counting it would suggest otherwise.
#[derive(Clone, Default)]
pub struct HonestPathTally {
    sampled: Arc<AtomicU64>,
}

impl HonestPathTally {
    /// Decisions this agent resolved through the sampler chain — every decision
    /// it was asked for, since there is no other way through.
    pub fn sampled_decisions(&self) -> u64 {
        self.sampled.load(Ordering::Relaxed)
    }
}

/// The arena's shipped agent: sample one world from the seat's view, then let a
/// [`RolloutPolicy`] pick over the factored surface.
pub struct PolicyAgent {
    policy: RolloutPolicy,
    rng: ChaCha8Rng,
    /// Kept alongside the stream it seeded so provenance can RECORD it —
    /// `ChaCha8Rng` does not hand its seed back, and a corpus record that names
    /// the policy but not the stream cannot reproduce the seat.
    agent_seed: u64,
    tally: HonestPathTally,
}

impl PolicyAgent {
    /// `agent_seed` is the PLAY stream — key it with [`agent_stream_seed`]. The
    /// setup decision deliberately does not draw from it (see
    /// [`choose_starting_order`](ArenaAgent::choose_starting_order)).
    pub fn new(policy: RolloutPolicy, agent_seed: u64) -> Self {
        Self {
            policy,
            rng: ChaCha8Rng::seed_from_u64(agent_seed),
            agent_seed,
            tally: HonestPathTally::default(),
        }
    }

    /// This agent's rung — provenance reads it rather than hand-typing a name.
    pub fn policy(&self) -> RolloutPolicy {
        self.policy
    }

    /// Decisions resolved so far through the sampler chain.
    pub fn sampled_decisions(&self) -> u64 {
        self.tally.sampled_decisions()
    }

    /// A cloneable handle on the counter above, readable after the agent has
    /// been moved into the driver.
    pub fn tally(&self) -> HonestPathTally {
        self.tally.clone()
    }
}

impl ArenaAgent for PolicyAgent {
    /// Drawn from a SEAT-keyed stream, not from `self.rng`.
    ///
    /// This is spec §4's keying rule made structural rather than documented:
    /// `self.rng` is keyed by the contestant SLOT, so drawing here would make
    /// the starting-order choice depend on which contestant sits at the seat,
    /// and the two seatings of a unit would no longer share a bit-identical
    /// post-`finalize_setup` state. The stream is derived from `(seat, deal)` —
    /// see [`setup_stream_seed`].
    ///
    /// The rung does not enter: all three rungs are MOVE policies, and 3.2
    /// ships no starting-order policy to differentiate them with.
    fn choose_starting_order(&mut self, seat: PlayerId, deal: &[OrderCardId; 2]) -> u8 {
        let mut setup_rng = ChaCha8Rng::seed_from_u64(setup_stream_seed(seat, deal));
        setup_rng.gen_range(0..2u8)
    }

    fn choose(&mut self, view: &PlayerView, rules: &Rules) -> Result<DecisionReport, String> {
        // The sampler seed comes from THIS AGENT'S OWN stream and nothing else.
        // Deriving it from any state-flavoured value would correlate the sample
        // with the truth it must be blind to (`game::agent`'s rule, restated
        // because this is the second implementation of it).
        let sampler_seed: u64 = self.rng.gen();
        let mut sampler = BeliefSampler::from_view(view, rules, sampler_seed).map_err(|e| {
            format!("belief sampler refused seat {}'s view: {e}", view.viewer.0)
        })?;
        let world = sampler.sample();
        let handle = DecisionHandle::new(&world, rules);

        // ONE enumeration for the whole decision: `decision_volume` forces the
        // routes memo, and every branch of `pick` below reads that same memo.
        // Taking the volume FIRST is what lets both `None` arms say something
        // true — this one is "not a decision point at all", the next is "a
        // structurally empty family".
        let volume = decision_volume(&handle).ok_or_else(|| {
            format!(
                "seat {} was asked to move at {:?}, which offers no families at all",
                view.viewer.0,
                handle.turn_phase(),
            )
        })?;
        let mv = self.policy.pick(&handle, &mut self.rng).ok_or_else(|| {
            format!(
                "dead end: policy {} found no move at {:?} for seat {}",
                self.policy.as_str(),
                handle.turn_phase(),
                view.viewer.0,
            )
        })?;

        self.tally.sampled.fetch_add(1, Ordering::Relaxed);
        Ok(DecisionReport { mv, volume })
    }

    fn controller(&self) -> ControllerProvenance {
        ControllerProvenance::from_policy(self.policy, Some(self.agent_seed))
    }
}

// ---------------------------------------------------------------------------
// Pairing and rotation
// ---------------------------------------------------------------------------

/// A contestant's slot in a cell, stable across every seating of a unit.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Slot(pub u8);

/// One design unit: ONE world seed played under every seating in `seatings`.
///
/// `seatings[k][seat]` is the slot sitting at `seat` in arrangement `k`. The
/// unit — not the game — is what intervals are computed over (spec §4): the
/// pair at 2P, the rotation block at 3–4P.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UnitSpec {
    pub seed: WorldSeed,
    pub players: u8,
    pub seatings: Vec<Vec<Slot>>,
}

/// The design unit for `players` contestants at `seed`: every cyclic rotation,
/// so each slot occupies each seat exactly once.
///
/// At 2P that is the pair `[[0,1],[1,0]]`; at 4P the four rotations. Seat swap
/// is BIAS removal and is mandatory (unswapped arrangement gap measured up to
/// 3.6pp); 3P is supported but carries E2's caveat that 3P seat symmetry is
/// UNTESTED.
pub fn unit_for(players: u8, seed: WorldSeed) -> UnitSpec {
    let n = players as usize;
    let seatings = (0..n)
        .map(|k| (0..n).map(|seat| Slot(((seat + k) % n) as u8)).collect())
        .collect();
    UnitSpec { seed, players, seatings }
}

// ---------------------------------------------------------------------------
// Drivers
// ---------------------------------------------------------------------------

/// One completed arena game. See the module doc: this carries a `GameRecord`,
/// so it never travels back to an agent.
///
/// **The arena's typed COMPLETED-game shape**, which is why emission filters on
/// it rather than on a flag: a game that did not complete has no
/// `ArenaGameStats` in existence to be filed, tabulated, or observed.
/// "Completed games only" is therefore structural here rather than checked.
///
/// The ROUTE to that fact has two shapes since A-S11-1 (freeze doc §9.6), and
/// the conclusion is the same on both: in refuse mode every failure in spec
/// §4's taxonomy — cap reached, dead end, driver bug — is an [`ArenaError`]; in
/// a counting cell a capped game is a typed event carrying the partial run's
/// per-seat moves and NOTHING of this type.
/// (Cross-reference, NOT a change: `metrics::TriggerTally::observe` takes an
/// `Option<&EndGameTrigger>` and states the same rule in prose, so it could one
/// day take this type instead and get the rule from the compiler. Out of scope
/// for 3.2 — noted so the option is not rediscovered from scratch.)
#[derive(Debug, Clone)]
pub struct ArenaGameStats {
    pub seed: WorldSeed,
    pub players: u8,
    pub turns: u32,
    pub moves_applied: u64,
    /// Applied moves per SEAT, indexed by seat — spec §5.4's driver extension,
    /// and the only place the per-seat split exists (the record's event list
    /// carries moves, not movers).
    ///
    /// Sums to [`moves_applied`](Self::moves_applied) by construction: every
    /// applied move has exactly one mover, and the loop increments both in the
    /// same step. What it BUYS is spec §0.3(b): "every move the evaluator bot
    /// played was explained" needs a per-contestant denominator, and a cell's
    /// contestant occupies a different seat in every arrangement.
    pub moves_by_seat: Vec<u64>,
    pub result: GameResult,
    pub trigger: Option<EndGameTrigger>,
    /// One observation per decision point, in play order. Never carries a
    /// non-decision point: `decision_volume`'s `None` is not a measurement.
    pub volumes: Vec<VolumeObs>,
    pub coverage: CoverageTally,
    pub record: GameRecord,
}

/// Typed guard shared by both public drivers (T6 review F4): every other bad
/// input here is a typed `ArenaError`, and Task 9 wires these entry points to
/// argv, where a `--players 5` panic would be user-facing.
///
/// `pub(crate)` since Phase 3.5c so `game::eval_screen`'s own unit loop asks the
/// SAME authority what the engine plays: its loop calls `unit_for` before it
/// calls any driver, and `unit_for(0, ..)` yields an empty seating list, so a
/// screen that re-derived the bound could tabulate zero games into a report full
/// of NaNs instead of erroring.
pub(crate) fn require_supported_player_count(players: u8) -> Result<(), ArenaError> {
    if (2..=4).contains(&players) {
        Ok(())
    } else {
        Err(ArenaError::Driver(format!("unsupported player count {players} (the engine plays 2-4)")))
    }
}

/// Play one honest game: one [`ArenaAgent`] per seat, each acting on its own
/// view alone.
///
/// `make_agent` is invoked BY THIS FUNCTION, after `new_game`, with only
/// `(seat, player_count)` — the caller never gets a chance to construct an
/// agent from privileged setup data. `invariants` runs `check_invariants` after
/// every applied move (ON in tests and gates; bulk corpus generation may
/// disable it, since those records are verified by strict replay instead).
pub fn run_arena_game(
    seed: WorldSeed,
    players: u8,
    rules: &Rules,
    make_agent: &mut dyn FnMut(PlayerId, u8) -> Box<dyn ArenaAgent>,
    invariants: bool,
) -> Result<ArenaGameStats, ArenaError> {
    run_arena_game_capped(seed, players, rules, make_agent, invariants, HONEST_MAX_TURNS)
}

/// [`run_arena_game`] with the non-termination guard as a parameter.
///
/// The public driver passes [`HONEST_MAX_TURNS`], deliberately the same
/// constant `game::agent`'s loop and the fuzz driver use — the two loops must
/// not disagree about what "stuck" means, which is the fold `game::agent`'s
/// `the_honest_loop_shares_the_fuzz_drivers_termination_guard` said 3.2 would
/// do. The parameter exists so a test can reach the
/// [`CapReached`](ArenaError::CapReached) arm in milliseconds instead of
/// playing 10,000 no-op turns.
pub fn run_arena_game_capped(
    seed: WorldSeed,
    players: u8,
    rules: &Rules,
    make_agent: &mut dyn FnMut(PlayerId, u8) -> Box<dyn ArenaAgent>,
    invariants: bool,
    max_turns: u32,
) -> Result<ArenaGameStats, ArenaError> {
    require_supported_player_count(players)?;
    let pending = new_game(GameConfig::for_player_count(players), rules, seed.0)
        .map_err(|e| ArenaError::Driver(format!("new_game failed ({players}P): {e}")))?;

    // The factory runs HERE — after `new_game`, before anything else — and
    // sees only (seat, player_count).
    let agents: Vec<Box<dyn ArenaAgent>> =
        (0..players).map(|i| make_agent(PlayerId(i), players)).collect();

    let mut bench = AgentBench::new(agents);
    drive(seed, rules, pending, &mut bench, invariants, max_turns)
}

/// Play one game with each seat's policy reading the TRUE state — the fuzzer's
/// shape, for screening cells and labelled diagnostics only.
///
/// No `observe`, no sampler: this is privileged play by construction, which is
/// why spec §4 requires the label to ride the ARTIFACT rather than the prose:
/// every seat's provenance says `observation: "full-state-diagnostic"`, and
/// [`corpus::emit_record`] refuses to file such a record as ordinary training
/// data (see [`corpus::corpus_refusal`]). Validation,
/// recording, and the failure taxonomy are identical to the honest driver's —
/// the two share one loop.
pub fn run_raw_game(
    seed: WorldSeed,
    players: u8,
    rules: &Rules,
    seat_policies: &[RolloutPolicy],
    rng_master: u64,
    invariants: bool,
) -> Result<ArenaGameStats, ArenaError> {
    require_supported_player_count(players)?;
    if seat_policies.len() != players as usize {
        return Err(ArenaError::Driver(format!(
            "run_raw_game got {} seat policies for {players} seats",
            seat_policies.len(),
        )));
    }
    let pending = new_game(GameConfig::for_player_count(players), rules, seed.0)
        .map_err(|e| ArenaError::Driver(format!("new_game failed ({players}P): {e}")))?;

    let seats = seat_policies
        .iter()
        .enumerate()
        .map(|(i, &policy)| {
            // The WORLD seed is folded in (T6 review F1): a sweep holding
            // `rng_master` fixed across games — the natural reading of a
            // "master" — must not hand seat i the identical stream in every
            // game, or the §7 cells' distributional figures sit on cross-game
            // common random numbers.
            let stream = mix(SALT_RAW_STREAM, &[seed.0, rng_master, i as u64]);
            RawSeat { policy, rng: ChaCha8Rng::seed_from_u64(stream), stream }
        })
        .collect();

    let mut bench = RawBench { seats };
    drive(seed, rules, pending, &mut bench, invariants, HONEST_MAX_TURNS)
}

/// The per-seat half of the driver: everything the two modes do DIFFERENTLY.
///
/// Every step after `decide` returns — validation, application, recording,
/// tallying, the failure taxonomy — is shared below, which is the point: the
/// honest path and the diagnostic path cannot drift in what they consider a
/// legal, recorded move.
///
/// `pub(crate)` since Phase 3.5c Phase-B T1: [`play_from_state`] is the half of
/// the driver a ROOTED run shares with the arena, and a `pub(crate)` function
/// cannot name a private trait in its signature. It stays crate-private — the
/// bench is an implementation seam, not a public surface (the public seam is
/// [`ArenaAgent`]).
pub(crate) trait SeatBench {
    /// SETUP is the from-setup driver's half alone — a rooted run never calls
    /// this — so it keeps reporting [`ArenaError`] while [`decide`](Self::decide)
    /// reports the shared [`PlayError`].
    fn starting_order(&mut self, seat: PlayerId, deal: &[OrderCardId; 2]) -> Result<u8, ArenaError>;
    fn decide(
        &mut self,
        state: &GameState,
        seat: PlayerId,
        rules: &Rules,
    ) -> Result<DecisionReport, PlayError>;

    /// Per-seat provenance in seat order (spec §5).
    ///
    /// The `observation` label is stated HERE and not by the agent, because the
    /// bench IS the mode: `AgentBench` passes views and `RawBench` passes the
    /// true state, so each has exactly one honest answer and neither can give
    /// the other's.
    fn seat_provenance(&self) -> Vec<SeatProvenance>;
}

pub(crate) struct AgentBench {
    agents: Vec<Box<dyn ArenaAgent>>,
}

impl AgentBench {
    /// One bench over `agents`, indexed by seat.
    ///
    /// `pub(crate)` since Phase 3.5c Phase-B T1 so `game::driven` seats a rooted
    /// run through THE arena's own honest bench rather than a copy of it: the
    /// `observe` call, the view/seat assertion and the refusal classification in
    /// [`AgentBench::decide`] are the honesty chain, and a second implementation
    /// of them is a second thing that can drift.
    pub(crate) fn new(agents: Vec<Box<dyn ArenaAgent>>) -> Self {
        Self { agents }
    }
}

impl SeatBench for AgentBench {
    fn starting_order(&mut self, seat: PlayerId, deal: &[OrderCardId; 2]) -> Result<u8, ArenaError> {
        Ok(self.agents[seat.0 as usize].choose_starting_order(seat, deal))
    }

    fn seat_provenance(&self) -> Vec<SeatProvenance> {
        self.agents.iter().map(|a| a.controller().observed_as(OBSERVATION_VIEW)).collect()
    }

    fn decide(
        &mut self,
        state: &GameState,
        seat: PlayerId,
        rules: &Rules,
    ) -> Result<DecisionReport, PlayError> {
        let view = observe(state, seat)
            .map_err(|e| PlayError::Observe { seat, detail: e.to_string() })?;
        // Plumbing pin: an agent must never be handed another seat's view.
        // Unconditional, like `run_honest_game`'s — one `u8` compare against a
        // fault that would silently hand a seat someone else's information.
        assert_eq!(view.viewer, seat, "view/seat mismatch in the arena driver");
        self.agents[seat.0 as usize]
            .choose(&view, rules)
            .map_err(|detail| PlayError::Agent { seat, detail })
    }
}

/// One seat of the diagnostic driver. `stream` is kept beside the `rng` it
/// seeded for the same reason [`PolicyAgent`] keeps its own: provenance records
/// the stream, and a `ChaCha8Rng` will not give it back.
struct RawSeat {
    policy: RolloutPolicy,
    rng: ChaCha8Rng,
    stream: u64,
}

struct RawBench {
    seats: Vec<RawSeat>,
}

impl SeatBench for RawBench {
    fn starting_order(
        &mut self,
        seat: PlayerId,
        _deal: &[OrderCardId; 2],
    ) -> Result<u8, ArenaError> {
        Ok(self.seats[seat.0 as usize].rng.gen_range(0..2u8))
    }

    fn seat_provenance(&self) -> Vec<SeatProvenance> {
        self.seats
            .iter()
            .map(|seat| {
                ControllerProvenance::from_policy(seat.policy, Some(seat.stream))
                    .observed_as(OBSERVATION_FULL_STATE_DIAGNOSTIC)
            })
            .collect()
    }

    fn decide(
        &mut self,
        state: &GameState,
        seat: PlayerId,
        rules: &Rules,
    ) -> Result<DecisionReport, PlayError> {
        let RawSeat { policy, rng, .. } = &mut self.seats[seat.0 as usize];
        let handle = DecisionHandle::new(state, rules);
        let volume = decision_volume(&handle).ok_or(PlayError::DeadEnd { seat })?;
        let mv = policy.pick(&handle, rng).ok_or(PlayError::DeadEnd { seat })?;
        Ok(DecisionReport { mv, volume })
    }
}

// ---------------------------------------------------------------------------
// The play half (Phase 3.5c Phase-B T1)
// ---------------------------------------------------------------------------

/// Why [`play_from_state`] stopped short of a terminal state.
///
/// **Not [`ArenaError`], and the difference is the point.** `ArenaError` is the
/// FROM-SETUP driver's taxonomy: it names a `WorldSeed` in its cap arm and folds
/// observe/apply/invariant faults into one `Driver(String)`. The play half is
/// shared with a ROOTED run that has no world seed and wants those faults
/// classified rather than stringified, so it reports structurally and each
/// caller maps into its own taxonomy — [`arena_error`] rebuilds `ArenaError`'s
/// messages VERBATIM (which is what keeps the arena's behaviour unmoved by the
/// split), and `game::driven` maps to its own typed error.
#[derive(Debug)]
pub(crate) enum PlayError {
    /// The state handed in violates an engine invariant before any move.
    CorruptRoot(String),
    /// `observe` refused to derive the mover's view (a live-tracked seat with an
    /// unrecorded deal is the reachable case).
    Observe { seat: PlayerId, detail: String },
    /// A seat's agent refused to produce a move — the honest chain's typed hard
    /// error. There is no fallback arm to reach.
    Agent { seat: PlayerId, detail: String },
    /// A seat proposed a move that is not legal in the TRUE state.
    IllegalPick { seat: PlayerId, mv: Box<Move> },
    /// The decision point offered nothing to play.
    DeadEnd { seat: PlayerId },
    /// A validated move failed to apply.
    Apply { seat: PlayerId, detail: String },
    /// An invariant broke after an applied move.
    Invariant { seat: PlayerId, reason: String },
    /// The turn guard tripped.
    ///
    /// `partial` is what the loop HAD when it stopped — every applied move in
    /// play order and the state it stopped in — and is `Some` from
    /// [`play_from_state`] alone. It exists for the V4 step-11 diagnosis wave:
    /// the arena's public drivers drop it (in refuse mode a capped game is a
    /// failure and emits nothing, unchanged), and
    /// `run_arena_game_diagnostic` keeps it, because a loop nobody can look at
    /// cannot be diagnosed — which is also how a COUNTING cell gets a capped
    /// game's per-seat move split since A-S11-1 (freeze doc §9.6). A rooted turn-driver has no
    /// from-setup outcome to hand back and passes `None`, which is why the field
    /// is an `Option` rather than a promise this error cannot keep.
    TurnCap { turns: u32, partial: Option<Box<PlayOutcome>> },
    /// The decision guard tripped — only reachable when a caller SETS one
    /// ([`PlayLimits::max_decisions`]); the arena passes `None`.
    DecisionCap { decisions: u64 },
}

/// The two non-termination guards the shared loop enforces.
///
/// `max_decisions` is `None` for the arena — deliberately, so the split adds no
/// guard the from-setup driver did not already have, and its behaviour is
/// unchanged by construction rather than by measurement. A rooted run sets it,
/// because a HAND-BUILT root can loop without ever advancing `turn_number` and
/// the turn cap alone would spin forever on it.
pub(crate) struct PlayLimits {
    pub(crate) max_turns: u32,
    pub(crate) max_decisions: Option<u64>,
}

/// What one run of the shared loop produced. Deliberately record-free: the
/// arena stamps its own [`GameRecord`] from `applied`, and a rooted run emits no
/// record at all (a rooted run is not a from-setup game and must not masquerade
/// as one).
#[derive(Debug)]
pub(crate) struct PlayOutcome {
    /// The state the loop stopped in — terminal by construction on `Ok`, and
    /// merely the state the turn guard tripped in when this value rides out
    /// inside [`PlayError::TurnCap`]'s `partial` (which is the ONLY non-`Ok`
    /// route it takes).
    pub(crate) final_state: GameState,
    /// Every applied move with its MOVER, in play order. The mover is
    /// `state.current_player` read BEFORE the apply.
    pub(crate) applied: Vec<(PlayerId, Move)>,
    /// Applied moves per seat, indexed by seat — sums to `applied.len()`.
    pub(crate) moves_by_seat: Vec<u64>,
    pub(crate) volumes: Vec<VolumeObs>,
    pub(crate) coverage: CoverageTally,
}

/// Play from `root` to terminal: validate, apply, tally, one decision at a time.
///
/// **This is the arena's own loop** — [`drive`] is this function plus the setup
/// half, and `game::driven` is this function plus a rooted seat list. Nothing in
/// here knows how the state was reached.
pub(crate) fn play_from_state(
    root: &GameState,
    rules: &Rules,
    bench: &mut dyn SeatBench,
    limits: PlayLimits,
    invariants: bool,
) -> Result<PlayOutcome, PlayError> {
    let mut state = root.clone();
    // Guard the INCOMING state so corruption is not misattributed to the first
    // move (every driver in the repo does this). UNCONDITIONAL, not gated on
    // `invariants`: that flag is about the per-move cost of a bulk run, and a
    // corrupt starting state is not a cost, it is a wrong answer waiting.
    if let Err(reason) = check_invariants(&state, rules) {
        return Err(PlayError::CorruptRoot(reason));
    }

    let mut volumes: Vec<VolumeObs> = Vec::new();
    let mut coverage = CoverageTally::default();
    let mut applied: Vec<(PlayerId, Move)> = Vec::new();
    let mut moves_by_seat: Vec<u64> = vec![0; state.players.len()];

    loop {
        if is_game_over(&state) {
            return Ok(PlayOutcome {
                final_state: state,
                applied,
                moves_by_seat,
                volumes,
                coverage,
            });
        }
        if state.turn_number > limits.max_turns {
            // The partial run rides OUT with the refusal (V4 step-11). Nothing
            // downstream of the public drivers reads it — they map this arm to
            // `ArenaError::CapReached` exactly as before — so the arena's
            // behaviour is unmoved; the diagnostic driver is the one caller that
            // looks.
            let turns = state.turn_number;
            return Err(PlayError::TurnCap {
                turns,
                partial: Some(Box::new(PlayOutcome {
                    final_state: state,
                    applied,
                    moves_by_seat,
                    volumes,
                    coverage,
                })),
            });
        }
        // The decision guard, checked BEFORE the decision it would be the
        // (max+1)-th of, so the refusal names the bound the caller set rather
        // than one past it. `None` — the arena's own setting — cannot trip.
        if let Some(max) = limits.max_decisions {
            if applied.len() as u64 >= max {
                return Err(PlayError::DecisionCap { decisions: applied.len() as u64 });
            }
        }

        let seat = state.current_player;
        let report = bench.decide(&state, seat, rules)?;
        let mv = report.mv;

        // The validation boundary. `PathMode::Canonical` is design A doing the
        // job open decision #3 settled it for; a false here is a driver bug, so
        // it is a hard error and never a redraw.
        if !is_move_legal(&state, &mv, rules, PathMode::Canonical) {
            return Err(PlayError::IllegalPick { seat, mv: Box::new(mv) });
        }
        // Debug-tier membership cross-check (spec §4): `Canonical` is wider
        // than list membership — it does not verify lex-least witness selection
        // — so this is what catches a non-canonical witness per-edit, before
        // strict replay would reject it through the checked `apply_move`.
        debug_assert!(
            crate::game::legal_moves::get_legal_moves(&state, rules).contains(&mv),
            "seat {} played a move that passed PathMode::Canonical but is not in \
             get_legal_moves — a non-canonical witness: {mv:?}",
            seat.0,
        );

        // The agent-attested volume is cross-checked against the TRUE state in
        // the same debug tier as the membership assert (T6 review F3): the
        // report's figure crosses the agent seam unvalidated otherwise, and a
        // per-family-invariance failure should die here, loudly, not surface
        // as a skewed published quantile. (Reviewer-verified to hold across
        // 14,572 decisions before this assert existed.) This is the THIRD
        // debug-tier route DFS at an on-board Movement point — see the module
        // doc's counter paragraph.
        #[cfg(debug_assertions)]
        {
            let truth = crate::game::metrics::decision_volume(
                &crate::game::search_actions::DecisionHandle::new(&state, rules),
            );
            debug_assert_eq!(
                truth.map(|v| (v.total, v.has_upper_bound_term)),
                Some((report.volume.total, report.volume.has_upper_bound_term)),
                "seat {}'s attested volume disagrees with the true state's",
                seat.0,
            );
        }

        state = apply_move_unchecked(&state, &mv, rules)
            .map_err(|e| PlayError::Apply { seat, detail: e.to_string() })?;
        if invariants {
            if let Err(reason) = check_invariants(&state, rules) {
                return Err(PlayError::Invariant { seat, reason });
            }
        }

        volumes.push(report.volume);
        coverage.observe(&mv);
        // `seat` is `state.current_player` read BEFORE the apply, i.e. the mover
        // — not the seat whose turn it is afterwards.
        applied.push((seat, mv));
        moves_by_seat[seat.0 as usize] += 1;
    }
}

/// [`PlayError`] as the from-setup driver reports it — the messages
/// [`ArenaError`] carried before the T1 split, rebuilt VERBATIM.
///
/// The three `Driver` arms are the reason this function exists rather than a
/// `From` impl: `ArenaError` folds observe/apply/invariant faults into one
/// string, and those strings are USER-VISIBLE driver output (the runner and
/// `examples/arena` print them). No test matches on them — verified at the T1
/// fix wave, so nothing here is load-bearing for the suite — and they are
/// rebuilt verbatim anyway, because a message change inside a refactor billed
/// as behavior-preserving is a change, and an operator reading a failure should
/// not have to work out whether the driver moved or only its prose did.
fn arena_error(seed: WorldSeed, error: PlayError) -> ArenaError {
    match error {
        PlayError::CorruptRoot(reason) => {
            ArenaError::Driver(format!("corrupt starting state: {reason}"))
        }
        PlayError::Observe { seat, detail } => {
            ArenaError::Driver(format!("observe failed for seat {}: {detail}", seat.0))
        }
        PlayError::Agent { seat, detail } => ArenaError::SamplerFailed { seat, detail },
        PlayError::IllegalPick { seat, mv } => ArenaError::IllegalPick { seat, mv },
        PlayError::DeadEnd { seat } => ArenaError::DeadEnd { seat },
        PlayError::Apply { seat, detail } => ArenaError::Driver(format!(
            "apply_move_unchecked rejected a validated move from seat {}: {detail}",
            seat.0
        )),
        PlayError::Invariant { seat, reason } => ArenaError::Driver(format!(
            "invariant violated after seat {}'s move: {reason}",
            seat.0
        )),
        // The partial run is DROPPED here, deliberately: `ArenaError` is the
        // public drivers' taxonomy and, in REFUSE mode, a capped game is a
        // failure that emits nothing. `drive_capturing` is the one caller that
        // reads it instead — and a COUNTING cell reaches this loop through
        // that caller, so it never passes here (A-S11-1, freeze doc §9.6).
        PlayError::TurnCap { turns, .. } => ArenaError::CapReached { seed, turns },
        // Unreachable from here BY CONSTRUCTION — `drive` passes
        // `max_decisions: None`, which is what makes the split behaviour-inert
        // for the arena. Mapped rather than `unreachable!()` because a panic in
        // a driver is a worse answer than a typed error, whatever a future
        // caller of this mapping does.
        PlayError::DecisionCap { decisions } => ArenaError::Driver(format!(
            "decision cap reached after {decisions} decisions (the arena sets no decision cap)"
        )),
    }
}

/// The setup half of [`drive`], factored so the diagnostic driver enters the
/// SAME setup rather than a second transcription of it (V4 step-11 W-D1).
///
/// Everything up to and including `finalize_setup`: the keep choices, the
/// record with its seat provenance, and the post-setup state to play from.
struct DriveStart {
    players: u8,
    record: GameRecord,
    start: GameState,
}

fn drive_setup(
    seed: WorldSeed,
    rules: &Rules,
    pending: PendingSetup,
    bench: &mut dyn SeatBench,
) -> Result<DriveStart, ArenaError> {
    let players = pending.state.players.len() as u8;
    // From `pending.state`, not the caller's config: `new_game` stamps
    // `setup_seed` into it, so `record.config.setup_seed == record.seed` holds
    // by construction rather than by a caller remembering to.
    let config = pending.state.config.clone();

    let mut choices: Vec<StartingOrderChoice> = Vec::with_capacity(players as usize);
    for i in 0..players {
        let seat = PlayerId(i);
        let deal = pending.deals[i as usize];
        let keep = bench.starting_order(seat, &deal)?;
        if keep > 1 {
            return Err(ArenaError::Driver(format!(
                "seat {} returned keep index {keep}: must be 0 or 1",
                seat.0
            )));
        }
        choices.push(StartingOrderChoice::Keep(keep));
    }

    let mut record = GameRecord::new(seed.0, config, choices.clone(), rules);
    // Provenance (spec §5): `controller` comes from each seat's own
    // `RolloutPolicy::descriptor()` and `observation` from the bench that built
    // it, so neither is hand-typed here. `master_seed`/`run_id` stay empty — one
    // game has no schedule or cell above it to name; the harness that does calls
    // `set_run_context` (see `stats::run_cell_emitting`).
    record.set_run_seats(bench.seat_provenance());

    let start = finalize_setup(pending, &choices)
        .map_err(|e| ArenaError::Driver(format!("finalize_setup failed: {e}")))?;

    Ok(DriveStart { players, record, start })
}

/// The COMPLETED half of [`drive`]'s tail: fold the applied moves into the
/// record, stamp the terminal digest, and tabulate. Factored for `drive_setup`'s
/// reason — one transcription, two callers.
fn finish_game(
    seed: WorldSeed,
    players: u8,
    mut record: GameRecord,
    outcome: PlayOutcome,
    rules: &Rules,
) -> ArenaGameStats {
    let PlayOutcome { final_state, applied, moves_by_seat, volumes, coverage } = outcome;
    let moves_applied = applied.len() as u64;
    // The record is stamped HERE rather than inside the loop, because a record is
    // the from-setup driver's artifact and a rooted run must never produce one
    // (design §3). `push_move` is a pure append and the log is not read during
    // play, so appending in play order after the fact is the same record.
    for (_mover, mv) in applied {
        record.push_move(mv);
    }
    record.stamp_terminal_digest(&final_state);

    ArenaGameStats {
        seed,
        players,
        turns: final_state.turn_number,
        moves_applied,
        moves_by_seat,
        result: final_result(&final_state, rules),
        trigger: final_state.trigger_reason,
        volumes,
        coverage,
        record,
    }
}

/// The shared loop: setup choices, then validate/apply/record per decision.
fn drive(
    seed: WorldSeed,
    rules: &Rules,
    pending: PendingSetup,
    bench: &mut dyn SeatBench,
    invariants: bool,
    max_turns: u32,
) -> Result<ArenaGameStats, ArenaError> {
    let DriveStart { players, record, start } = drive_setup(seed, rules, pending, bench)?;

    // THE SHARED HALF (Phase 3.5c Phase-B T1). Everything from the incoming-state
    // guard to the terminal check lives in `play_from_state`, which a ROOTED run
    // drives too — so the two cannot drift in what they consider a legal,
    // recorded move. `max_decisions: None` keeps this driver's guard set exactly
    // as it was: the turn cap and nothing else.
    let outcome = play_from_state(
        &start,
        rules,
        bench,
        PlayLimits { max_turns, max_decisions: None },
        invariants,
    )
    .map_err(|e| arena_error(seed, e))?;

    Ok(finish_game(seed, players, record, outcome, rules))
}

// ---------------------------------------------------------------------------
// The DIAGNOSTIC driver (V4 step-11 diagnosis wave, W-D1 surface A)
// ---------------------------------------------------------------------------

/// How a DIAGNOSTIC run of one arena game ended.
///
/// The contrast with [`ArenaGameStats`]'s doc is the whole point of this type:
/// there, "completed" is STRUCTURAL because every non-completion is an
/// [`ArenaError`] and a failed game has no stats in existence. That rule is what
/// keeps a corpus honest, and it is unchanged — [`run_arena_game`] still refuses
/// a capped game and emits nothing, and so does an unflagged cell. It also
/// means a capped game leaves NO
/// evidence, so the V4 step-11 field's `cell failed: turn cap reached at turn
/// 10001` could name a world and nothing about what the world was doing.
///
/// This enum is the diagnosis-only exception, and it says so in its shape: a
/// [`Capped`](Self::Capped) game is a DIFFERENT variant carrying a DIFFERENT
/// payload, so no caller can mistake a partial run for a completed one and no
/// [`ArenaGameStats`] — the type every tabulator in `arena::stats` takes — is
/// ever built from one.
///
/// **WHAT THAT DOES AND DOES NOT BUY, stated precisely (W-D1 review), because
/// the obvious stronger reading is false.** The variant split protects the
/// TABULATORS, which take `ArenaGameStats`. It does NOT protect the corpus by
/// typing: [`corpus::emit_record`] takes a `&GameRecord`, and a capped game's
/// partial record is a perfectly ordinary `GameRecord` that could be handed to
/// it. Nor does the missing terminal digest make such a record self-refusing:
/// `replay_strict` SKIPS its terminal check when a record carries no digest
/// (`record.rs`, `if let Some(expected_digest)`), so a partial record replays
/// strict-clean. **The real protection is a CODE-PATH fact, not a type fact —
/// no path in `arena::diagnostic` or in `examples/arena`'s two diagnostic
/// subcommands calls `emit_record` at all** — and a future caller that wanted to
/// file one of these would have to write that call itself, deliberately.
#[derive(Debug)]
pub enum DiagnosticOutcome {
    /// The game reached a terminal state. Byte-for-byte the game
    /// [`run_arena_game`] plays at the same world and bench.
    Terminated(Box<ArenaGameStats>),
    /// The turn guard tripped. `turns` is the turn number the guard read (one
    /// past the limit), `record` carries every applied move in play order and
    /// nothing else, `state` is the state the guard tripped in, and
    /// `moves_by_seat` is the per-seat split.
    ///
    /// **The record carries NO terminal digest**, deliberately: the state is not
    /// terminal, so there is no terminal digest to honestly stamp — a digest
    /// here would ASSERT a claim about a game that has not ended. It does not
    /// follow that the digest-less record is thereby rejected anywhere:
    /// `replay_strict` skips its terminal check when there is no digest to check
    /// against (see this enum's doc). What is true is narrower and is what the
    /// wave relies on — lenient [`replay`](crate::game::record::replay)
    /// reproduces `state` exactly, the property
    /// `the_diagnostic_driver_surfaces_the_partial_record_and_state_on_a_cap`
    /// pins and `diagnostic::render_world` re-checks on every report.
    ///
    /// Both payloads are BOXED, matching [`Terminated`](Self::Terminated)'s
    /// indirection: a `GameRecord` of a capped game holds tens of thousands of
    /// events, and an unboxed variant would make every `DiagnosticGame` that
    /// size (`clippy::large_enum_variant`).
    Capped {
        turns: u32,
        record: Box<GameRecord>,
        state: Box<GameState>,
        moves_by_seat: Vec<u64>,
    },
}

/// One DIAGNOSTIC game: the world it was played at, its player count, and how
/// it ended.
#[derive(Debug)]
pub struct DiagnosticGame {
    pub seed: WorldSeed,
    pub players: u8,
    pub outcome: DiagnosticOutcome,
}

/// [`run_arena_game`] that KEEPS what a capped game had instead of dropping it.
///
/// Identical play: same setup half, same shared loop, same validation, same
/// keying — a terminating world produces the same moves, the same terminal
/// digest and the same stats. The ONLY difference is at the turn guard, where
/// this returns [`DiagnosticOutcome::Capped`] rather than
/// [`ArenaError::CapReached`].
///
/// **It does not replace either public driver.** An unflagged cell refuses at
/// the first capped game by design (a skipped game would read as a different
/// N); that rule is what keeps every recorded figure honest and is untouched.
///
/// **AMENDED by A-S11-1 (freeze doc §9.6): it IS how a cell runs in the opt-in
/// COUNTING mode.** `stats::run_cell_loop` plays every game of a
/// [`CapPolicy::Count`](stats::CapPolicy::Count) cell through this driver,
/// because it is the only one that hands a capped game's partial run back
/// instead of dropping it — the same setup half, the same shared loop and the
/// same keying, which is what makes a counted game reproduce here. In refuse
/// mode the cell still calls [`run_arena_game_capped`] and this driver is not
/// on its path at all. Its other use is unchanged: the V4 step-11 diagnosis
/// wave reproduces one world and looks at the loop.
pub fn run_arena_game_diagnostic(
    seed: WorldSeed,
    players: u8,
    rules: &Rules,
    make_agent: &mut dyn FnMut(PlayerId, u8) -> Box<dyn ArenaAgent>,
    invariants: bool,
) -> Result<DiagnosticGame, ArenaError> {
    run_arena_game_diagnostic_capped(
        seed,
        players,
        rules,
        make_agent,
        invariants,
        HONEST_MAX_TURNS,
    )
}

/// [`run_arena_game_diagnostic`] with the non-termination guard as a parameter —
/// [`run_arena_game_capped`]'s reason exactly: a test must reach the cap arm in
/// milliseconds instead of playing 10,000 no-op turns.
pub fn run_arena_game_diagnostic_capped(
    seed: WorldSeed,
    players: u8,
    rules: &Rules,
    make_agent: &mut dyn FnMut(PlayerId, u8) -> Box<dyn ArenaAgent>,
    invariants: bool,
    max_turns: u32,
) -> Result<DiagnosticGame, ArenaError> {
    require_supported_player_count(players)?;
    let pending = new_game(GameConfig::for_player_count(players), rules, seed.0)
        .map_err(|e| ArenaError::Driver(format!("new_game failed ({players}P): {e}")))?;

    // The factory runs HERE — after `new_game`, before anything else — and sees
    // only (seat, player_count). The honesty rule `run_arena_game_capped` states
    // is not relaxed for a diagnostic.
    let agents: Vec<Box<dyn ArenaAgent>> =
        (0..players).map(|i| make_agent(PlayerId(i), players)).collect();

    let mut bench = AgentBench::new(agents);
    drive_capturing(seed, rules, pending, &mut bench, invariants, max_turns)
}

/// [`drive`], with the turn cap read as an OUTCOME instead of an error.
fn drive_capturing(
    seed: WorldSeed,
    rules: &Rules,
    pending: PendingSetup,
    bench: &mut dyn SeatBench,
    invariants: bool,
    max_turns: u32,
) -> Result<DiagnosticGame, ArenaError> {
    let DriveStart { players, mut record, start } = drive_setup(seed, rules, pending, bench)?;

    match play_from_state(
        &start,
        rules,
        bench,
        PlayLimits { max_turns, max_decisions: None },
        invariants,
    ) {
        Ok(outcome) => Ok(DiagnosticGame {
            seed,
            players,
            outcome: DiagnosticOutcome::Terminated(Box::new(finish_game(
                seed, players, record, outcome, rules,
            ))),
        }),
        Err(PlayError::TurnCap { turns, partial: Some(partial) }) => {
            let PlayOutcome { final_state, applied, moves_by_seat, .. } = *partial;
            for (_mover, mv) in applied {
                record.push_move(mv);
            }
            // NO `stamp_terminal_digest`: see `DiagnosticOutcome::Capped`.
            Ok(DiagnosticGame {
                seed,
                players,
                outcome: DiagnosticOutcome::Capped {
                    turns,
                    record: Box::new(record),
                    state: Box::new(final_state),
                    moves_by_seat,
                },
            })
        }
        // `play_from_state` always attaches the partial (it is the only
        // constructor of this arm that has one to attach). A `None` here would
        // mean the loop refused a cap without the run behind it, which this
        // driver cannot honestly present as a diagnosis — so it reports the
        // ordinary cap failure rather than inventing an empty one.
        Err(e) => Err(arena_error(seed, e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::policy::POLICY_IMPL_VERSION;
    use crate::game::record::{replay, replay_strict, RUN_PROVENANCE_SCHEMA};
    use crate::game::semantics::state_digest;
    use crate::game::seeds::SeedSchedule;
    use crate::game::semantics::SEMANTICS_REV;
    use crate::types::TurnPhase;
    use std::sync::Mutex;

    /// The provenance a test agent that is not a policy reports. Its NAME says
    /// what it is: a fixture, never something a corpus should treat as a rung.
    fn test_controller(name: &'static str) -> ControllerProvenance {
        ControllerProvenance {
            name: name.to_string(),
            version: 0,
            params: std::collections::BTreeMap::new(),
            agent_seed: None,
        }
    }

    /// This task's committed diagnostic master. The NAME of the constructor is
    /// the waiver (`game::seeds`): an asserting test wants reproducibility.
    const MASTER: u64 = 0xA3E1_2026_0726_0006;

    fn world(index: u64) -> WorldSeed {
        let mut schedule = SeedSchedule::diagnostic(MASTER);
        (0..=index).map(|_| schedule.next_world()).last().unwrap()
    }

    /// A factory that seats one policy everywhere, keyed exactly as the pairing
    /// layer keys it.
    fn uniform_bench(
        policy: RolloutPolicy,
        seed: WorldSeed,
        seating: Vec<Slot>,
    ) -> impl FnMut(PlayerId, u8) -> Box<dyn ArenaAgent> {
        move |seat, _players| {
            let slot = seating[seat.0 as usize];
            Box::new(PolicyAgent::new(policy, agent_stream_seed(seed, seat, slot)))
        }
    }

    /// **SCOPED at Phase 3.3 to KEEP-HOMOGENEOUS cells, and KEPT (spec §5.5,
    /// ledger row 10).** Bit-identical starts were a whole-arena property in 3.2
    /// because every contestant was a rung and every rung takes its Keep from
    /// the same `(seat, deal)`-keyed stream. From 3.3 the evaluator contestants
    /// take `score_starting_order`'s argmax instead, so a MIXED cell's two
    /// seatings resolve the same deal two ways and this property is false there
    /// BY DESIGN — the doctrine is scoped, not repealed. The cell under test
    /// asserts its own homogeneity precondition below rather than relying on a
    /// reader noticing that both contestants happen to be rungs, and the
    /// surviving half in mixed cells has its own pin
    /// (`the_deal_at_each_seat_is_seating_invariant_in_a_mixed_keep_cell`).
    ///
    /// MUTATION-PROVEN (reverted by inverse edit, never `git checkout` — L7):
    /// drawing the setup choice from `self.rng` — the slot-keyed PLAY stream —
    /// fails the 72-pair assertion below at the first deal (left 1, right 0).
    #[test]
    fn paired_seatings_share_a_bit_identical_starting_state() {
        // E2's CRN rule, first half: the setup stream is keyed by (world seed,
        // seat), so swapping which contestant sits where cannot move the
        // starting state. If it could, a pair would not be a paired design at
        // all -- the two games would differ in the setup as well as the
        // seating, and the swap would stop removing the seat effect.
        let rules = Rules::standard();
        let seed = world(0);
        let unit = unit_for(2, seed);
        assert_eq!(unit.seatings, vec![vec![Slot(0), Slot(1)], vec![Slot(1), Slot(0)]]);

        // THE SCOPE, asserted rather than assumed: this cell is self-play of one
        // rung, hence Keep-homogeneous, hence entitled to the property below.
        let contestant = Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT);
        assert_eq!(
            contestant.keep_kind(),
            KeepKind::SeatKeyedUniform,
            "this pin holds only for Keep-homogeneous cells (spec §5.5)",
        );

        let starts: Vec<GameState> = unit
            .seatings
            .iter()
            .map(|seating| {
                let mut make = uniform_bench(
                    RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
                    seed,
                    seating.clone(),
                );
                let stats = run_arena_game(seed, 2, &rules, &mut make, true)
                    .unwrap_or_else(|e| panic!("arena game failed: {e}"));
                // Rebuild the driver's OWN starting state from the record it
                // emitted: `new_game` + `finalize_setup` over (seed, config,
                // choices), which is exactly the preamble the driver ran.
                let mut setup_only = stats.record.clone();
                setup_only.events.clear();
                setup_only.terminal_digest = None;
                replay(&setup_only, &rules).expect("the record's setup preamble must replay")
            })
            .collect();

        assert_eq!(starts[0], starts[1], "the two seatings started from different states");

        // The property the comparison above RESTS on, pinned directly because
        // two seats' worth of binary choices is weak evidence: a driver that
        // (wrongly) drew the setup choice from the slot-keyed PLAY stream would
        // still produce matching states ~25% of the time at 2P. Here two
        // contestants whose play streams share nothing agree on every one of 72
        // (seat, deal) pairs, which they can only do by not reading that stream.
        let mut a = PolicyAgent::new(RolloutPolicy::Uniform, 1);
        let mut b = PolicyAgent::new(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT, u64::MAX / 3);
        for seat in 0..4u8 {
            for card in 1..=18u8 {
                let deal = [OrderCardId(card), OrderCardId(card + 18)];
                assert_eq!(
                    a.choose_starting_order(PlayerId(seat), &deal),
                    b.choose_starting_order(PlayerId(seat), &deal),
                    "the setup choice moved with the agent's play stream (seat {seat}, deal {deal:?})",
                );
            }
        }
    }

    /// One seat's setup decision as observed: who was asked, what they were
    /// dealt, which card they kept.
    type SetupLog = Vec<(PlayerId, [OrderCardId; 2], u8)>;

    /// A seat's agent that delegates the SETUP decision to a real contestant and
    /// then stops the game — so a setup property costs one `new_game` instead of
    /// two full evaluator games.
    struct SetupSpy {
        inner: Box<dyn ArenaAgent>,
        log: Arc<Mutex<SetupLog>>,
    }

    impl ArenaAgent for SetupSpy {
        fn choose_starting_order(&mut self, seat: PlayerId, deal: &[OrderCardId; 2]) -> u8 {
            let keep = self.inner.choose_starting_order(seat, deal);
            self.log.lock().unwrap().push((seat, *deal, keep));
            keep
        }
        fn choose(&mut self, _v: &PlayerView, _r: &Rules) -> Result<DecisionReport, String> {
            Err("setup-only probe".to_string())
        }
        fn controller(&self) -> ControllerProvenance {
            self.inner.controller()
        }
    }

    /// **The NEW pin spec §5.5 requires: what survives in a MIXED-Keep cell.**
    ///
    /// `paired_seatings_share_a_bit_identical_starting_state` is false here by
    /// design — Greedy-v1 keeps by `score_starting_order`, a rung keeps by its
    /// `(seat, deal)`-keyed draw, so the same deal resolves two ways depending on
    /// who is sitting there. What does NOT move is the DEAL itself: it is a
    /// function of `(world seed, seat)` alone, decided inside `new_game` before
    /// any agent exists. That is the half the unit's pairing still rests on, and
    /// it is why a mixed cell is still seed-paired and swap-balanced (ledger
    /// row 15: pairing is bookkeeping, the swap is the point).
    ///
    /// BOTH HALVES ARE ASSERTED, because either alone is vacuous: the invariance
    /// half would pass on a cell where the keeps also happened to agree, so the
    /// run also counts the seeds whose keeps DIVERGED and requires at least one
    /// — i.e. this is demonstrably a mixed cell and not a homogeneous one wearing
    /// two names.
    ///
    /// Setup-only by construction (see [`SetupSpy`]): the property is about
    /// `new_game`'s deal and the two Keep rules, and playing evaluator games out
    /// would cost seconds to observe nothing extra.
    #[test]
    fn the_deal_at_each_seat_is_seating_invariant_in_a_mixed_keep_cell() {
        let rules = Rules::standard();
        let x = Contestant::GREEDY_V1_DEFAULT;
        let y = Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT);
        assert_ne!(
            x.keep_kind(),
            y.keep_kind(),
            "anti-vacuity: this pin is about a cell whose contestants keep DIFFERENTLY",
        );

        let deals_of = |log: &SetupLog| -> Vec<(PlayerId, [OrderCardId; 2])> {
            log.iter().map(|(seat, deal, _)| (*seat, *deal)).collect()
        };
        let keeps_of = |log: &SetupLog| -> Vec<u8> { log.iter().map(|(_, _, k)| *k).collect() };

        let mut seeds_whose_keeps_diverged = 0u32;
        for i in 0..8u64 {
            let seed = world(30 + i);
            let unit = unit_for(2, seed);

            let mut per_seating: Vec<SetupLog> = Vec::new();
            for seating in &unit.seatings {
                let log: Arc<Mutex<SetupLog>> = Arc::new(Mutex::new(Vec::new()));
                let collected = Arc::clone(&log);
                let err = run_arena_game(seed, 2, &rules, &mut |seat, _n| {
                    let slot = seating[seat.0 as usize];
                    let contestant = if slot == Slot(0) { x } else { y };
                    let (inner, _tally) =
                        contestant.build(&rules, agent_stream_seed(seed, seat, slot));
                    Box::new(SetupSpy { inner, log: Arc::clone(&collected) })
                }, true)
                .expect_err("the setup probe stops the game at its first decision");
                assert!(matches!(err, ArenaError::SamplerFailed { .. }), "{err}");
                let seen = log.lock().unwrap().clone();
                per_seating.push(seen);
            }

            assert_eq!(per_seating[0].len(), 2, "one setup decision per seat");
            assert_eq!(
                deals_of(&per_seating[0]),
                deals_of(&per_seating[1]),
                "seed {seed:?}: swapping the contestants moved a SEAT's deal — the half a mixed \
                 cell's pairing still rests on",
            );
            // ...and the deals are not a constant the equality above could hold
            // over trivially.
            assert_ne!(
                per_seating[0][0].1, per_seating[0][1].1,
                "two seats were handed the same deal",
            );

            if keeps_of(&per_seating[0]) != keeps_of(&per_seating[1]) {
                seeds_whose_keeps_diverged += 1;
            }
        }

        assert!(
            seeds_whose_keeps_diverged > 0,
            "no seed in the probe produced differing KEEPS across the two seatings — either the \
             two contestants share a Keep rule after all (and this cell is not mixed), or the \
             driver stopped asking the agents",
        );
    }

    /// MUTATION-PROVEN (reverted by inverse edit): dropping the slot component
    /// from `agent_stream_seed` — E2's trap verbatim — makes seat 0 hand both
    /// contestants the same stream (both drew 2586029867533633687) and fails
    /// the first assertion below.
    #[test]
    fn agent_rng_is_not_keyed_by_seat_alone() {
        // E2's CRN rule, second half, and the trap it exists to exclude: keying
        // the PLAY stream by seat alone makes both games of a pair the same
        // game, so the null cell reads rho = -1.0000 with SE 0.000 -- an
        // instrument that reports "perfectly calibrated" while measuring
        // nothing.
        //
        // DEVIATION NOTE (plan Task 6's parenthetical): the plan glossed this
        // test as "a slot's first draw identical across seatings", which
        // describes keying by (world seed, slot) with the seat DROPPED. Spec
        // §4's rule names all three components and this implementation keys by
        // all three, so a slot's stream does move with its seat; what the test
        // asserts instead is the property its NAME states and the trap
        // requires -- the stream is a function of the SLOT, not of the seat
        // alone. Dropping the seat would also correlate a contestant's own
        // randomness across the unit's games, and E2 keeps pairing for bias
        // removal while explicitly NOT claiming variance reduction from it.
        let seed = world(1);
        let unit = unit_for(2, seed);
        let first_draw = |seat: PlayerId, slot: Slot| -> u64 {
            ChaCha8Rng::seed_from_u64(agent_stream_seed(seed, seat, slot)).gen()
        };

        // At a FIXED seat, swapping which contestant sits there changes the
        // stream. This is the assertion keying by seat alone cannot pass.
        for seat in [PlayerId(0), PlayerId(1)] {
            assert_ne!(
                first_draw(seat, Slot(0)),
                first_draw(seat, Slot(1)),
                "seat {} drew the same stream for two different contestants",
                seat.0,
            );
        }

        // ...and the same read through the unit: seat 0 runs a different stream
        // in the two seatings, so the pair's two games are not one game twice.
        let seat0_streams: Vec<u64> = unit
            .seatings
            .iter()
            .map(|seating| first_draw(PlayerId(0), seating[0]))
            .collect();
        assert_ne!(seat0_streams[0], seat0_streams[1]);

        // The seat component is load-bearing too: one contestant's two seats
        // are independent streams, not the same one replayed.
        assert_ne!(first_draw(PlayerId(0), Slot(0)), first_draw(PlayerId(1), Slot(0)));

        // And the whole key moves with the world seed.
        let other = world(2);
        assert_ne!(
            agent_stream_seed(seed, PlayerId(0), Slot(0)),
            agent_stream_seed(other, PlayerId(0), Slot(0)),
        );
    }

    /// MUTATION-PROVEN (reverted by inverse edit): giving `AgentBench::decide`
    /// the `TrivialHonestAgent`-shaped fallback — on `Err`, play
    /// `get_legal_moves(true_state)[0]` — does not merely weaken this test, it
    /// turns the refusal into 10,001 turns of silent privileged play
    /// (`CapReached`), which is exactly the corpus-poisoning failure spec §4
    /// names. `the_driver_hands_each_agent_only_its_own_deal` falls with it.
    #[test]
    fn sampler_failure_is_a_hard_error_never_a_fallback() {
        // The `TrivialHonestAgent` precedent falls back to the TRUE legal list
        // when `from_view` errors -- privileged play, recorded only in a tally.
        // Here the chain has no second arm: the run dies, nothing is applied,
        // and no record escapes to be trained on.
        struct DoctoredView {
            calls: Arc<AtomicU64>,
        }
        impl ArenaAgent for DoctoredView {
            fn choose_starting_order(&mut self, _seat: PlayerId, _deal: &[OrderCardId; 2]) -> u8 {
                0
            }
            fn choose(
                &mut self,
                view: &PlayerView,
                rules: &Rules,
            ) -> Result<DecisionReport, String> {
                self.calls.fetch_add(1, Ordering::Relaxed);
                // Break the reconciliation identity the sampler validates.
                let mut doctored = view.clone();
                doctored.order_deck_len += 1;
                // `BeliefSampler` is not `Debug`, so this is a `let else`
                // rather than `expect_err`.
                let Err(err) = BeliefSampler::from_view(&doctored, rules, 7) else {
                    panic!("a view whose sinks do not partition the pool must be refused")
                };
                Err(format!("belief sampler refused seat {}'s view: {err}", view.viewer.0))
            }
            fn controller(&self) -> ControllerProvenance {
                test_controller("test:doctored-view")
            }
        }

        let rules = Rules::standard();
        let calls = Arc::new(AtomicU64::new(0));
        let shared = Arc::clone(&calls);
        let err = run_arena_game(world(3), 2, &rules, &mut |_seat, _n| {
            Box::new(DoctoredView { calls: Arc::clone(&shared) })
        }, true)
        .expect_err("a sampler refusal must fail the run");

        match err {
            ArenaError::SamplerFailed { seat, detail } => {
                assert_eq!(seat, PlayerId(0), "the first mover's refusal is the one reported");
                assert!(detail.contains("belief sampler refused"), "detail lost: {detail}");
            }
            other => panic!("expected SamplerFailed, got {other:?}"),
        }
        // NO move was applied: the driver asked exactly once and stopped. Had
        // it fallen back it would have applied that move and asked again.
        assert_eq!(calls.load(Ordering::Relaxed), 1, "the driver kept playing past a refusal");
    }

    #[test]
    fn the_driver_hands_each_agent_only_its_own_deal() {
        // `PendingSetup` carries every seat's deal AND the seeded deck; the
        // factory runs inside the driver and sees only (seat, player_count), so
        // no caller can build an agent from privileged setup data.
        type DealLog = Arc<Mutex<Vec<(PlayerId, [OrderCardId; 2])>>>;
        struct Recorder {
            log: DealLog,
        }
        impl ArenaAgent for Recorder {
            fn choose_starting_order(&mut self, seat: PlayerId, deal: &[OrderCardId; 2]) -> u8 {
                self.log.lock().unwrap().push((seat, *deal));
                0
            }
            fn choose(&mut self, _v: &PlayerView, _r: &Rules) -> Result<DecisionReport, String> {
                // Setup is all this test is about; stop the game here rather
                // than pay a full one.
                Err("stop after setup".to_string())
            }
            fn controller(&self) -> ControllerProvenance {
                test_controller("test:deal-recorder")
            }
        }

        let rules = Rules::standard();
        let deals: DealLog = Arc::new(Mutex::new(Vec::new()));
        let built: Arc<Mutex<Vec<(PlayerId, u8)>>> = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&deals);
        let seen_args = Arc::clone(&built);
        let err = run_arena_game(world(4), 2, &rules, &mut |seat, players| {
            seen_args.lock().unwrap().push((seat, players));
            Box::new(Recorder { log: Arc::clone(&log) })
        }, true)
        .expect_err("the recorder stops the game at its first decision");
        assert!(matches!(err, ArenaError::SamplerFailed { .. }));

        assert_eq!(
            *built.lock().unwrap(),
            vec![(PlayerId(0), 2u8), (PlayerId(1), 2u8)],
            "the factory saw something other than (seat, player_count) per seat",
        );
        let entries = deals.lock().unwrap();
        assert_eq!(entries.len(), 2, "one setup decision per seat");
        assert_eq!(entries[0].0, PlayerId(0));
        assert_eq!(entries[1].0, PlayerId(1));
        assert_ne!(entries[0].1, entries[1].1, "two seats were handed the same deal");
    }

    #[test]
    fn honest_path_tally_counts_every_decision() {
        // Anti-vacuity for the honest chain: every applied move came out of a
        // sampled world. A completed game proves nothing on its own -- the
        // fallback arm this driver refuses to have would also have completed
        // it (the trap `AgentPathTally` was built for).
        let rules = Rules::standard();
        let seed = world(5);
        let seating = unit_for(2, seed).seatings[0].clone();
        let tallies: Arc<Mutex<Vec<HonestPathTally>>> = Arc::new(Mutex::new(Vec::new()));
        let collected = Arc::clone(&tallies);

        let stats = run_arena_game(seed, 2, &rules, &mut |seat, _n| {
            let slot = seating[seat.0 as usize];
            let agent = PolicyAgent::new(
                RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
                agent_stream_seed(seed, seat, slot),
            );
            collected.lock().unwrap().push(agent.tally());
            Box::new(agent)
        }, true)
        .unwrap_or_else(|e| panic!("arena game failed: {e}"));

        let per_seat: Vec<u64> =
            tallies.lock().unwrap().iter().map(|t| t.sampled_decisions()).collect();
        assert_eq!(per_seat.len(), 2);
        assert!(per_seat.iter().all(|&n| n > 0), "a seat never played: {per_seat:?}");
        assert_eq!(
            per_seat.iter().sum::<u64>(),
            stats.moves_applied,
            "some move reached the board without a sampled decision behind it",
        );
        assert_eq!(
            stats.volumes.len() as u64,
            stats.moves_applied,
            "one volume observation per applied move",
        );
    }

    #[test]
    fn an_illegal_pick_is_a_hard_error() {
        // Church spaces are faith 4..=11, so `space: u8::MAX` is in no legal
        // list in any phase of any state -- the premise stays true whatever a
        // future phase makes broadly legal (`game::agent`'s substitution, same
        // reasoning).
        struct Cheater;
        impl ArenaAgent for Cheater {
            fn choose_starting_order(&mut self, _s: PlayerId, _d: &[OrderCardId; 2]) -> u8 {
                0
            }
            fn choose(&mut self, _v: &PlayerView, _r: &Rules) -> Result<DecisionReport, String> {
                Ok(DecisionReport {
                    mv: Move::ChooseChurchSpace { space: u8::MAX },
                    volume: VolumeObs {
                        phase: TurnPhase::Placement,
                        total: 1,
                        has_upper_bound_term: false,
                    },
                })
            }
            fn controller(&self) -> ControllerProvenance {
                test_controller("test:cheater")
            }
        }

        let rules = Rules::standard();
        let err = run_arena_game(world(6), 2, &rules, &mut |_s, _n| Box::new(Cheater), true)
            .expect_err("an illegal pick must fail the run");
        match err {
            ArenaError::IllegalPick { seat, mv } => {
                assert_eq!(seat, PlayerId(0));
                assert_eq!(*mv, Move::ChooseChurchSpace { space: u8::MAX });
            }
            other => panic!("expected IllegalPick, got {other:?}"),
        }
    }

    #[test]
    fn cap_reached_is_distinguished() {
        // A cap hit is not a driver bug: it fails the cell, emits nothing, and
        // names its seed so the run reproduces (uniform tails reach 5,364 of
        // the 10,000 cap in E1's held-out cells). The cap is a parameter only
        // so this assertion costs milliseconds; `run_arena_game` passes
        // HONEST_MAX_TURNS.
        let rules = Rules::standard();
        let seed = world(7);
        let seating = unit_for(2, seed).seatings[0].clone();
        let mut make = uniform_bench(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT, seed, seating);
        let err = run_arena_game_capped(seed, 2, &rules, &mut make, false, 2)
            .expect_err("a 2-turn cap cannot contain a full game");
        match err {
            ArenaError::CapReached { seed: reported, turns } => {
                assert_eq!(reported, seed, "the cap error must name its own world seed");
                assert!(turns > 2, "the cap fires past the limit, got turn {turns}");
            }
            other => panic!("expected CapReached, got {other:?}"),
        }
        assert_eq!(HONEST_MAX_TURNS, 10_000, "the shared termination guard moved");
    }

    /// **W-D1 (A), the diagnostic driver's cap path.** The arena's public driver
    /// DROPS the partial record and the state it capped in: a capped cell can
    /// therefore report games-until-first-hit and nothing about what the game
    /// was doing. This is the surface that keeps them, and these are its two
    /// honesty pins — the record carries every applied move and nothing else,
    /// and replaying it from setup reproduces the state handed back.
    #[test]
    fn the_diagnostic_driver_surfaces_the_partial_record_and_state_on_a_cap() {
        let rules = Rules::standard();
        let seed = world(7);
        let seating = unit_for(2, seed).seatings[0].clone();
        let mut make = uniform_bench(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT, seed, seating);
        // The cap is a parameter for the same reason `cap_reached_is_distinguished`
        // takes one: reaching this arm must cost milliseconds, not 10,000 turns.
        let game = run_arena_game_diagnostic_capped(seed, 2, &rules, &mut make, false, 20)
            .expect("a cap is an OUTCOME for the diagnostic driver, never an error");
        assert_eq!(game.seed, seed);
        assert_eq!(game.players, 2);
        let DiagnosticOutcome::Capped { turns, record, state, moves_by_seat } = game.outcome else {
            panic!("a 20-turn cap cannot contain a full 2P game");
        };
        assert!(turns > 20, "the cap fires past the limit, got turn {turns}");
        assert_eq!(
            record.events.len() as u64,
            moves_by_seat.iter().sum::<u64>(),
            "the partial record must carry EVERY applied move and nothing else",
        );
        assert!(
            record.terminal_digest.is_none(),
            "a capped game is NOT terminal — stamping a terminal digest would let a partial \
             record masquerade as a completed one",
        );
        let replayed = replay(&record, &rules).expect("the partial record replays from setup");
        assert_eq!(
            state_digest(&replayed),
            state_digest(&state),
            "the returned state must be the one the returned record folds to",
        );
    }

    /// **W-D1 (A), the diagnostic driver's terminating path IS the same game.**
    /// A separate driver that played a different game would make every tail read
    /// taken off it a read of something else.
    #[test]
    fn the_diagnostic_driver_plays_the_same_game_as_the_public_driver() {
        let rules = Rules::standard();
        let seed = world(3);
        let seating = unit_for(2, seed).seatings[0].clone();
        let mut public_make =
            uniform_bench(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT, seed, seating.clone());
        let public = run_arena_game(seed, 2, &rules, &mut public_make, false)
            .expect("the rung plays this world to a trigger");
        let mut diag_make = uniform_bench(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT, seed, seating);
        let game = run_arena_game_diagnostic(seed, 2, &rules, &mut diag_make, false)
            .expect("the diagnostic driver plays the same world");
        let DiagnosticOutcome::Terminated(stats) = game.outcome else {
            panic!("this world terminates under the public driver, so it must here too");
        };
        assert_eq!(stats.turns, public.turns, "turns");
        assert_eq!(stats.moves_by_seat, public.moves_by_seat, "moves per seat");
        assert_eq!(stats.trigger, public.trigger, "end trigger");
        assert_eq!(stats.record.events, public.record.events, "the applied move sequence");
        assert_eq!(
            stats.record.terminal_digest, public.record.terminal_digest,
            "the terminal-state digest",
        );
    }

    /// **W-D1's byte-inertness pin on the PUBLIC driver.** The diagnostic
    /// variant exists BESIDE `run_arena_game`, never inside its answer: a cap
    /// stays an `ArenaError::CapReached` whose message is the one every cell,
    /// artifact and ledger row already quotes.
    #[test]
    fn the_public_drivers_cap_error_text_is_unmoved_by_the_diagnostic_variant() {
        let rules = Rules::standard();
        let seed = world(7);
        let seating = unit_for(2, seed).seatings[0].clone();
        let mut make = uniform_bench(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT, seed, seating);
        let err = run_arena_game_capped(seed, 2, &rules, &mut make, false, 2)
            .expect_err("a 2-turn cap cannot contain a full game");
        let ArenaError::CapReached { seed: reported, turns } = err else {
            panic!("expected CapReached, got {err:?}");
        };
        assert_eq!(reported, seed);
        assert_eq!(
            ArenaError::CapReached { seed, turns }.to_string(),
            format!("turn cap reached at turn {turns} (world seed {:#018x})", seed.0),
            "the operator-facing cap message is what the V4 step-11 record quotes verbatim",
        );
    }

    #[test]
    fn a_full_2p_and_4p_game_completes_under_each_rung() {
        // Every rung plays both drivers to a real end trigger with per-move
        // invariants on, and every emitted record replays STRICT-clean -- which
        // is the property that says the debug membership cross-check and the
        // checked `apply_move` boundary agree about every move played.
        let rules = Rules::standard();
        let rungs = [
            RolloutPolicy::Uniform,
            RolloutPolicy::RetreatAverse,
            RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
        ];

        for (i, rung) in rungs.iter().enumerate() {
            for (j, players) in [2u8, 4].into_iter().enumerate() {
                let seed = world(10 + (i * 2 + j) as u64);
                let seating = unit_for(players, seed).seatings[0].clone();
                let mut make = uniform_bench(*rung, seed, seating);
                let stats = run_arena_game(seed, players, &rules, &mut make, true)
                    .unwrap_or_else(|e| panic!("{} {players}P failed: {e}", rung.as_str()));

                assert_eq!(stats.players, players);
                assert!(stats.moves_applied > 0);
                assert!(stats.trigger.is_some(), "a completed game names its end trigger");
                assert_eq!(stats.result.standings.len(), players as usize);
                assert_eq!(stats.coverage.applied(), stats.moves_applied);
                assert_eq!(stats.record.events.len() as u64, stats.moves_applied);
                assert_eq!(stats.record.seed, seed.0);
                assert_eq!(
                    stats.record.config.setup_seed, stats.record.seed,
                    "the record's config must be the one new_game stamped",
                );
                replay_strict(&stats.record, &rules).unwrap_or_else(|e| {
                    panic!("{} {players}P record failed strict replay: {e}", rung.as_str())
                });

                // The diagnostic driver, same rung, same shared loop.
                let raw = run_raw_game(
                    seed,
                    players,
                    &rules,
                    &vec![*rung; players as usize],
                    0xD1A6_0000_0000_0006,
                    true,
                )
                .unwrap_or_else(|e| panic!("{} {players}P raw failed: {e}", rung.as_str()));
                assert!(raw.trigger.is_some());
                replay_strict(&raw.record, &rules).unwrap_or_else(|e| {
                    panic!("{} {players}P raw record failed strict replay: {e}", rung.as_str())
                });
            }
        }
    }

    // -- Task 8: provenance --------------------------------------------------

    /// A record the arena emitted carries WHO PLAYED IT, per seat, and is still
    /// a record the checked replay boundary accepts. The two halves belong in
    /// one test because either alone is the failure spec §5 is guarding: a
    /// strict-clean record with no provenance is an unlabelled corpus entry, and
    /// a richly-labelled record that will not replay is not a corpus entry at
    /// all.
    ///
    /// The cell is MIXED (uniform at one seat, family-weighted at the other) —
    /// the per-seat shape is the sub-phase's headline capability, and a
    /// self-play cell could not tell a per-seat field from a per-run one.
    ///
    /// MUTATION-PROVEN (reverted by inverse edit, never `git checkout` — L7):
    /// labelling `AgentBench`'s seats `full-state-diagnostic` — an honest driver
    /// claiming privileged access, the mislabel the whole containment rule rests
    /// on not happening — fails the observation assertion below.
    #[test]
    fn arena_records_replay_strict_clean() {
        let rules = Rules::standard();
        let seed = world(20);
        let seating = unit_for(2, seed).seatings[0].clone();
        let rung_of = |slot: Slot| {
            if slot == Slot(0) {
                RolloutPolicy::Uniform
            } else {
                RolloutPolicy::FAMILY_WEIGHTED_DEFAULT
            }
        };

        let stats = run_arena_game(seed, 2, &rules, &mut |seat, _n| {
            let slot = seating[seat.0 as usize];
            Box::new(PolicyAgent::new(rung_of(slot), agent_stream_seed(seed, seat, slot)))
        }, true)
        .unwrap_or_else(|e| panic!("arena game failed: {e}"));

        replay_strict(&stats.record, &rules)
            .unwrap_or_else(|e| panic!("an arena record must replay strict-clean: {e}"));

        let run = stats.record.run.as_ref().expect("an arena record carries provenance");
        assert_eq!(run.schema, RUN_PROVENANCE_SCHEMA);
        assert_eq!(run.seats.len(), 2, "one entry per seat, in seat order");
        // A bare `run_arena_game` has no schedule above it to name.
        assert_eq!(run.master_seed, None);
        assert_eq!(run.run_id, None);

        for (i, seat_run) in run.seats.iter().enumerate() {
            let seat = PlayerId(i as u8);
            let slot = seating[i];
            // Literals, not `as_str()`: the code single-sources the name from
            // the policy, so comparing against the same call would pin nothing.
            // These strings are the WIRE FORMAT a 3.3 corpus reader will match.
            let (expected_name, expected_params): (&str, Vec<(&str, i64)>) = match rung_of(slot) {
                RolloutPolicy::Uniform => ("uniform", vec![]),
                _ => ("family-weighted", vec![("recovery_min_of", 5), ("retreat_ppm", 166_667)]),
            };
            assert_eq!(seat_run.controller, expected_name, "seat {i}'s controller");
            assert_eq!(seat_run.version, POLICY_IMPL_VERSION, "seat {i}'s policy impl version");
            assert_eq!(
                seat_run.params,
                expected_params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
                "seat {i}'s knobs",
            );
            assert_eq!(seat_run.observation, "view", "an honest seat sees a view");
            assert_eq!(
                seat_run.agent_seed,
                Some(agent_stream_seed(seed, seat, slot)),
                "seat {i} must record the stream it actually played on",
            );
        }
    }

    /// The label rides the ARTIFACT, not the prose (spec §4/§5, map §1.1): a
    /// raw-state game is privileged play by construction, so its record says so
    /// per seat, and the corpus emitter refuses to file it as ordinary training
    /// data. Both halves are here because the label is only worth something if
    /// something reads it.
    ///
    /// MUTATION-PROVEN (reverted by inverse edit — L7): inverting
    /// `corpus_refusal`'s diagnostic-seat test so it admits privileged records
    /// fails the refusal assertion below.
    #[test]
    fn raw_state_records_are_labelled_diagnostic() {
        let rules = Rules::standard();
        let seed = world(21);
        let raw = run_raw_game(
            seed,
            2,
            &rules,
            &[RolloutPolicy::Uniform, RolloutPolicy::FAMILY_WEIGHTED_DEFAULT],
            0xD1A6_0000_0000_0008,
            true,
        )
        .unwrap_or_else(|e| panic!("raw game failed: {e}"));

        let run = raw.record.run.as_ref().expect("a raw record carries provenance");
        assert_eq!(run.seats.len(), 2);
        for (i, seat_run) in run.seats.iter().enumerate() {
            assert_eq!(
                seat_run.observation, "full-state-diagnostic",
                "seat {i} of a raw-state game must be labelled diagnostic",
            );
            assert!(seat_run.agent_seed.is_some(), "seat {i} must record its stream");
        }
        assert_eq!(run.seats[0].controller, "uniform");
        assert_eq!(run.seats[1].controller, "family-weighted");
        replay_strict(&raw.record, &rules)
            .unwrap_or_else(|e| panic!("a raw record must still replay strict-clean: {e}"));

        // Containment (spec §5). Checked through the pure predicate so the
        // assertion costs no I/O and cannot leave a stray file inside the
        // repo's own corpus directory.
        let corpus = default_corpus_dir();
        let ordinary = corpus_path(&corpus, SEMANTICS_REV, 2, "fw-selfplay", seed, 0);
        let labelled = corpus_path(&corpus, SEMANTICS_REV, 2, "fw-selfplay-diagnostic", seed, 0);
        assert!(
            corpus_refusal(&ordinary, &raw.record).is_some(),
            "a diagnostic record must not be filed under an ordinary corpus cell",
        );

        // The T8-review absolutization (6d48c2e) pinned executably (branch
        // review, should-fix 4): the NATURAL relative spelling of --emit from
        // the crate root must refuse too — every other call site here is
        // absolute by construction, so this is the production spelling's only
        // coverage.
        let rel = ordinary
            .strip_prefix(std::env::current_dir().expect("cwd"))
            .expect("test cwd is the crate root");
        assert!(
            corpus_refusal(rel, &raw.record).is_some(),
            "the relative --emit spelling into the corpus dir must refuse as the absolute one does",
        );
        assert!(
            corpus_refusal(&labelled, &raw.record).is_none(),
            "a -diagnostic cell is exactly where a diagnostic record belongs",
        );

        // ...and the refusal is enforced at the writer, before it touches disk.
        // Compared against the BEFORE state rather than asserted absent: a T9
        // `--emit` run into the real corpus can legitimately have created this
        // cell directory, and what this pins is that the refusal changed
        // nothing, not that the tree was empty to begin with.
        let cell_dir = ordinary.parent().unwrap().to_path_buf();
        let existed_before = cell_dir.exists();
        let err = emit_record(&ordinary, &raw.record)
            .expect_err("the emitter must refuse a diagnostic record in an ordinary cell");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(
            cell_dir.exists(),
            existed_before,
            "the refusal must precede directory creation, not follow it",
        );
        assert!(!ordinary.exists(), "the refusal wrote the record anyway");

        // An honest record is admitted at the same ordinary path — so the
        // refusal above is about the LABEL and not about the path shape.
        let honest_seating = unit_for(2, seed).seatings[0].clone();
        let honest = run_arena_game(seed, 2, &rules, &mut |seat, _n| {
            let slot = honest_seating[seat.0 as usize];
            Box::new(PolicyAgent::new(
                RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
                agent_stream_seed(seed, seat, slot),
            ))
        }, true)
        .unwrap_or_else(|e| panic!("arena game failed: {e}"));
        assert!(corpus_refusal(&ordinary, &honest.record).is_none());
    }

    /// The kickoff's `StartingOrderChoice` pin (spec §4): the setup decision is
    /// a real per-seat decision the record carries, so two agents that decide it
    /// differently produce different records — and both still replay strict.
    ///
    /// Without this, `choose_starting_order` could be ignored by the driver (or
    /// overwritten with a default) and every test above would still pass:
    /// `PolicyAgent`'s own choice is seat-keyed, so a self-play cell's records
    /// look identical whether the agent was asked or not.
    #[test]
    fn distinguishable_setup_agents_produce_distinguishable_choices() {
        /// A `PolicyAgent` that plays normally but always keeps a FIXED card.
        struct FixedSetup {
            keep: u8,
            inner: PolicyAgent,
        }
        impl ArenaAgent for FixedSetup {
            fn choose_starting_order(&mut self, _seat: PlayerId, _deal: &[OrderCardId; 2]) -> u8 {
                self.keep
            }
            fn choose(&mut self, view: &PlayerView, rules: &Rules) -> Result<DecisionReport, String> {
                self.inner.choose(view, rules)
            }
            fn controller(&self) -> ControllerProvenance {
                self.inner.controller()
            }
        }

        let rules = Rules::standard();
        let seed = world(22);
        let seating = unit_for(2, seed).seatings[0].clone();
        let play_one = |keeps: [u8; 2]| {
            run_arena_game(seed, 2, &rules, &mut |seat, _n| {
                let slot = seating[seat.0 as usize];
                Box::new(FixedSetup {
                    keep: keeps[seat.0 as usize],
                    inner: PolicyAgent::new(
                        RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
                        agent_stream_seed(seed, seat, slot),
                    ),
                })
            }, true)
            .unwrap_or_else(|e| panic!("arena game failed: {e}"))
        };

        let left = play_one([0, 1]);
        let right = play_one([1, 0]);
        assert_eq!(
            left.record.choices,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(1)],
        );
        assert_eq!(
            right.record.choices,
            vec![StartingOrderChoice::Keep(1), StartingOrderChoice::Keep(0)],
        );
        // Distinguishable in the SETUP, and the difference propagates: the two
        // games have different starting hands, so they are not one game twice.
        assert_ne!(left.record.choices, right.record.choices);
        for stats in [&left, &right] {
            replay_strict(&stats.record, &rules)
                .unwrap_or_else(|e| panic!("a record with a chosen setup must replay strict: {e}"));
        }
    }
}
