//! The DRIVEN-FIXTURE harness (Phase B item 1, design
//! `docs/plans/2026-08-17-phase-b-harness-design.md`): play from a supplied
//! `GameState` to terminal with named seats, honest per-seat observation, and
//! per-seat counters.
//!
//! **What this is FOR.** The arena answers "who wins from SETUP"; the questions
//! Phase B asks are rooted — "from THIS position, is the move the bot picked
//! actually dead?", "does the band structure at this root move with the beam?".
//! Those need a driver that starts where a record (or a fixture) already is. It
//! is the instrument D7's mechanical separator and family 1's adjudication run
//! on, and it is deliberately OFF the fifteen-gate roster: a graduating fixture
//! joins `tests/scenario_suite.rs`'s never-delete registry, never this module.
//!
//! **THE PLAY LOOP IS THE ARENA'S, NOT A COPY OF IT.** [`drive_to_terminal`]
//! builds agents, seats them in the arena's own [`AgentBench`], and hands them to
//! [`play_from_state`] — the same `observe` → agent → `is_move_legal(Canonical)`
//! → `apply_move_unchecked` chain, the same debug-tier membership and volume
//! cross-checks, the same "an agent refusal is a HARD error with no fallback
//! arm" posture. A second loop would be a second thing that can drift, and the
//! drift would be invisible: a privileged-play fallback produces games that look
//! exactly like honest ones. The executable statement of that claim is
//! `a_driven_run_from_an_arena_root_reproduces_the_arena_game_move_for_move`
//! below — from a `new_game`-built root at a diagnostic master, this driver
//! reproduces `run_arena_game`'s game MOVE FOR MOVE.
//!
//! **SEEDING is the ARENA discipline** (design §4): each seat plays on ONE
//! long-lived stream keyed `agent_stream_seed(master, seat, slot)`. A rooted run
//! has no seating rotation, so the slot component is the seat's own index — the
//! derivation stays injective in the seat, and the reproduction pin above is
//! what verifies the whole discipline rather than a comment claiming it. The
//! master is recorded on the outcome; per standing policy a caller quoting
//! figures draws it from OS entropy and records it, and a fixture cell uses a
//! committed diagnostic one.
//!
//! **NO `GameRecord` IS EMITTED, ever** (design §3). A rooted run is not a
//! from-setup game: it has no `new_game` seed, no setup choices of its own, and
//! nothing that would let `replay` reconstruct it from the top. Stamping one
//! would put a file in the corpus that claims to be reproducible from setup and
//! is not. What travels instead is [`DrivenOutcome`] — moves with their movers,
//! the final state, the result, and the per-seat counters.
//!
//! **What T1 deliberately does NOT ship**: `RootSource`/provenance/the registry
//! (T3), `root_table` (T2), `adjudicate_root` (T4), the runner (T5). None is
//! stubbed here — a stub is a surface a reader has to check the emptiness of.

use crate::game::agent::HONEST_MAX_TURNS;
use crate::game::arena::{
    agent_stream_seed, play_from_state, require_supported_player_count, AgentBench, ArenaAgent,
    Contestant, ExplainHandle, PlayError, PlayLimits, Slot,
};
use crate::game::policy::RolloutPolicy;
use crate::game::rules::Rules;
use crate::game::scoring::{final_result, GameResult};
use crate::game::seeds::WorldSeed;
use crate::game::semantics::state_digest;
use crate::types::{GameState, Move, PlayerId};

/// The decision guard's default: how many decisions a rooted run may apply
/// before it is refused as non-terminating.
///
/// GENEROUS by design — organic games run ~1,200 applied moves at 2P and ~1,700
/// at 4P (the `policy_diversity` gate's own totals), and a rooted run is a
/// SUFFIX of one, so this is roughly 60× the longest thing an organic root can
/// produce. It is not a performance budget and nothing tunes against it: it
/// exists so that a HAND-BUILT fixture with an absorbing no-op loop — the T3
/// null-placement pathology, which loops WITHOUT advancing `turn_number` and so
/// slips under the turn cap — becomes a typed refusal in seconds instead of a
/// hang.
pub const DEFAULT_MAX_DECISIONS: u64 = 100_000;

// ---------------------------------------------------------------------------
// Seats
// ---------------------------------------------------------------------------

/// The declared REPLY menu (design §3): the named policies an opponent seat
/// plays under while a rooted question is being answered.
///
/// Sugar over [`Contestant`], never a second agent implementation — each arm
/// spells one contestant and [`ReplyPolicy::contestant`] is the only place the
/// spelling lives. The point of naming them at all is that an adjudication is
/// only as meaningful as the reply it was measured against, so the reply belongs
/// in the vocabulary rather than in a caller's argument.
///
/// **The fourth menu slot — one adversarial policy declared as a RULE OVER
/// STATE — is deliberately absent in v1.** It is declared at family freeze
/// (Phase C); shipping a placeholder would invite a hand-picked adversary chosen
/// after seeing the roots it is applied to, which is the shape of a fixture that
/// proves whatever its author wanted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyPolicy {
    /// The shipped rollout rung — `RolloutPolicy::FAMILY_WEIGHTED_DEFAULT`.
    FamilyWeighted,
    /// Greedy-v1 at its shipped retreat-sample knob.
    GreedyAtFinal,
    /// Lookahead-v1 at its shipped knobs, UNCAPPED — the provisional production
    /// contestant.
    LookaheadAtFinal,
}

impl ReplyPolicy {
    /// The contestant this arm names. THE single spelling of the menu.
    pub fn contestant(self) -> Contestant {
        match self {
            Self::FamilyWeighted => Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
            Self::GreedyAtFinal => Contestant::GREEDY_V1_DEFAULT,
            Self::LookaheadAtFinal => Contestant::LOOKAHEAD_V1_DEFAULT,
        }
    }

    /// The stable name, for reports and error text.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FamilyWeighted => "family-weighted",
            Self::GreedyAtFinal => "greedy-at-final",
            Self::LookaheadAtFinal => "lookahead-at-final",
        }
    }
}

/// Who sits at one seat of a driven run.
///
/// Both arms materialize to the SAME honest chain — the [`Reply`](Self::Reply)
/// arm is a named shorthand, not a different kind of seat — so a mixed run is
/// not mixing two agent implementations, only two ways of writing one down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatSpec {
    /// Any contestant the arena grammar can express, knobs included.
    Contestant(Contestant),
    /// One of the declared reply policies.
    Reply(ReplyPolicy),
}

impl SeatSpec {
    /// The contestant this seat plays.
    pub fn contestant(self) -> Contestant {
        match self {
            Self::Contestant(c) => c,
            Self::Reply(r) => r.contestant(),
        }
    }
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Knobs on a driven run. Every field has a stated default and none of them
/// changes what a decision IS — they bound the run and choose how loudly the
/// engine is checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrivenOpts {
    /// The non-termination guard — see [`DEFAULT_MAX_DECISIONS`]. Zero is
    /// refused: a run that may apply no decisions is not a run.
    pub max_decisions: u64,
    /// The turn guard, `HONEST_MAX_TURNS` by default — deliberately the same
    /// constant every driver in the repo uses, so the two loops cannot disagree
    /// about what "stuck" means.
    pub max_turns: u32,
    /// Per-move `check_invariants`. ON by default, which is what every driver in
    /// the repo contracts for; the off-switch exists for bulk generation, whose
    /// states are checked another way.
    pub invariants: bool,
}

impl Default for DrivenOpts {
    fn default() -> Self {
        Self {
            max_decisions: DEFAULT_MAX_DECISIONS,
            max_turns: HONEST_MAX_TURNS,
            invariants: true,
        }
    }
}

// ---------------------------------------------------------------------------
// The outcome
// ---------------------------------------------------------------------------

/// One applied move with its MOVER. The pair is what a rooted run reports
/// instead of a record: a `GameRecord` carries moves without movers (the mover
/// is re-derived by replay), and per-seat attribution is exactly what a rooted
/// question asks about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedMove {
    pub seat: PlayerId,
    pub mv: Move,
}

/// One seat's counters at the end of a run — a SNAPSHOT of its
/// [`ExplainHandle`], never the live handle.
///
/// Snapshotted because the handle is an `Arc` on counters the agent keeps
/// mutating: a value that changes after the run ended cannot be compared, hashed
/// into a digest, or trusted in a report. **Every field is `Option` on
/// `ExplainHandle`'s own rule — `None` means "this contestant has no such
/// quantity", never "zero"** — so an assertion about a counter cannot pass
/// vacuously on a contestant that does not have it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SeatCounters {
    /// Decisions this seat explained (spec §0.3(b)'s left-hand side).
    pub explained: Option<u64>,
    /// Continuations `MAX_TURN_PLIES` stopped — `None` without a search.
    pub ply_cap_hits: Option<u64>,
    /// Free extensions `MAX_FREE_EXTENSION` stopped — `None` without a
    /// quiescence arm.
    pub free_cap_hits: Option<u64>,
    /// Successors applied, observed and scored — the budget currency.
    pub candidates_scored: Option<u64>,
    /// The mandatory root pass's share of `candidates_scored`.
    pub root_scored: Option<u64>,
    /// Decisions in which a cap denied work.
    pub budget_binds: Option<u64>,
    /// Decisions whose root pass alone spent the cap.
    pub budget_starved_decisions: Option<u64>,
}

impl SeatCounters {
    /// A seat that counts nothing — every rung's shape.
    pub const NONE: Self = Self {
        explained: None,
        ply_cap_hits: None,
        free_cap_hits: None,
        candidates_scored: None,
        root_scored: None,
        budget_binds: None,
        budget_starved_decisions: None,
    };

    /// Read one built contestant's handle. Private: the handle is built inside
    /// this module and never escapes it.
    fn from_handle(handle: &ExplainHandle) -> Self {
        Self {
            explained: Some(handle.explained()),
            ply_cap_hits: handle.ply_cap_hits(),
            free_cap_hits: handle.free_cap_hits(),
            candidates_scored: handle.candidates_scored(),
            root_scored: handle.root_scored(),
            budget_binds: handle.budget_binds(),
            budget_starved_decisions: handle.budget_starved_decisions(),
        }
    }
}

/// What one seat did, and what it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatOutcome {
    pub seat: PlayerId,
    /// What the caller ASKED for.
    pub spec: SeatSpec,
    /// What actually played — `Contestant::echo`, so the outcome states the
    /// version and knobs rather than a bare name a reader would have to look up.
    pub echo: String,
    /// This seat's play stream, from `agent_stream_seed`.
    pub agent_seed: u64,
    pub applied_moves: u64,
    pub counters: SeatCounters,
}

/// A completed rooted run.
#[derive(Debug, Clone, PartialEq)]
pub struct DrivenOutcome {
    /// The master every seat's stream descends from.
    pub master: WorldSeed,
    /// Every applied move with its mover, in play order.
    pub applied: Vec<AppliedMove>,
    /// Decision points resolved — equal to `applied.len()`, since a decision
    /// that produced no applied move is a hard error and there is no outcome.
    pub decisions: u64,
    pub final_state: GameState,
    pub result: GameResult,
    /// One entry per seat, in seat order.
    pub seats: Vec<SeatOutcome>,
    /// Spec §0.3(b)'s count, summed over the seats that HAVE an explanation
    /// path — `None` when no seat does (every seat a rung), never `Some(0)`.
    pub explanations: Option<u64>,
}

impl DrivenOutcome {
    /// Applied moves per seat, indexed by seat.
    pub fn moves_by_seat(&self) -> Vec<u64> {
        self.seats.iter().map(|s| s.applied_moves).collect()
    }

    /// Spec §0.3(b)'s IDENTITY: every explaining seat explained exactly the
    /// moves it applied. A seat with no explanation path is vacuously fine —
    /// which is why the count is `Option` and this reads it rather than
    /// comparing a sum.
    pub fn explanation_identity_holds(&self) -> bool {
        self.seats
            .iter()
            .all(|s| s.counters.explained.is_none_or(|e| e == s.applied_moves))
    }

    /// A stable one-line identity for this run.
    ///
    /// Deterministic in `(root, seats, master, opts)` and nothing else, which is
    /// what makes it a reproduction check rather than a summary: the final
    /// state's own content digest closes over every card, pawn and track, the
    /// per-seat counters close over the search work, and the master is printed
    /// so a quoted digest names the run that produced it. It is NOT a
    /// serialization format — nothing parses it back.
    pub fn digest(&self) -> String {
        let seats: Vec<String> = self
            .seats
            .iter()
            .map(|s| {
                format!(
                    "p{}={} moves={} explained={} scored={}",
                    s.seat.0,
                    s.echo,
                    s.applied_moves,
                    opt(s.counters.explained),
                    opt(s.counters.candidates_scored),
                )
            })
            .collect();
        let standings: Vec<String> = self
            .result
            .standings
            .iter()
            .map(|st| format!("p{}={}", st.player.0, st.breakdown.total))
            .collect();
        format!(
            "driven-v1 master={:#018x} decisions={} state={} winner=p{} seats=[{}] standings=[{}]",
            self.master.0,
            self.decisions,
            state_digest(&self.final_state),
            self.result.winner.0,
            seats.join(" | "),
            standings.join(" "),
        )
    }
}

/// `Some(n)` as `n`, `None` as `-` — the digest's rendering of the
/// counter-absence distinction.
fn opt(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_string(), |v| v.to_string())
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why a rooted run produced no outcome.
///
/// Every variant is a HARD stop that produces nothing — the arena's posture,
/// adopted for the same reason: a harness that silently degraded (played on with
/// privileged information, clamped a bound, skipped a seat) would produce
/// fixtures indistinguishable from honest ones, and the analysis riding on them
/// would be wrong in a way no later check could find.
#[derive(Debug)]
pub enum DrivenError {
    /// The seat list does not describe the root's seats.
    SeatMismatch { seats: usize, players: usize },
    /// The root, or the options, are not something this driver can play.
    UnsupportedRoot(String),
    /// The root violates an engine invariant — refused BEFORE the first
    /// decision, so a corrupt fixture is never misattributed to a move.
    CorruptRoot(String),
    /// The seat to move cannot observe the root. The reachable case is a
    /// live-tracked seat whose own deal was never revealed.
    Unobservable { seat: PlayerId, detail: String },
    /// A seat's agent refused to produce a move — the honest chain's typed hard
    /// error, with NO fallback arm behind it.
    AgentRefused { seat: PlayerId, detail: String },
    /// A seat proposed a move illegal in the true state (a driver bug).
    IllegalPick { seat: PlayerId, mv: Box<Move> },
    /// The decision point offered nothing to play.
    DeadEnd { seat: PlayerId },
    /// The engine rejected a validated move, or an invariant broke after one.
    Engine(String),
    /// The turn guard tripped.
    TurnCapReached { turns: u32 },
    /// The decision guard tripped: this root did not terminate within the bound
    /// — a REFUSAL naming the bound, never a hang and never a truncated run
    /// reported as a result.
    DecisionCapReached { decisions: u64 },
}

impl std::fmt::Display for DrivenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SeatMismatch { seats, players } => write!(
                f,
                "{seats} seat specs for a {players}-seat root: a driven run names every seat"
            ),
            Self::UnsupportedRoot(m) => write!(f, "unsupported root: {m}"),
            Self::CorruptRoot(m) => write!(f, "corrupt root: {m}"),
            Self::Unobservable { seat, detail } => {
                write!(f, "seat {} cannot observe the root: {detail}", seat.0)
            }
            Self::AgentRefused { seat, detail } => {
                write!(f, "seat {} produced no move: {detail}", seat.0)
            }
            Self::IllegalPick { seat, mv } => {
                write!(f, "seat {} proposed a move illegal in the true state: {mv:?}", seat.0)
            }
            Self::DeadEnd { seat } => write!(f, "seat {} reached a dead end", seat.0),
            Self::Engine(m) => write!(f, "engine: {m}"),
            Self::TurnCapReached { turns } => write!(f, "turn cap reached at turn {turns}"),
            Self::DecisionCapReached { decisions } => write!(
                f,
                "the root did not terminate within {decisions} decisions (the guard is a refusal, \
                 never a truncated run)"
            ),
        }
    }
}

impl std::error::Error for DrivenError {}

/// [`PlayError`] in this module's taxonomy — total, and structural rather than
/// string-matched.
///
/// The two `Engine` arms are folded deliberately: an apply that rejects a
/// validated move and an invariant that breaks after one are both "the engine
/// disagreed with itself", they are both bugs rather than answers, and a caller
/// that wants to tell them apart reads the message. Everything a rooted CALLER
/// can cause — a corrupt fixture, an unobservable seat, a refusing agent, a
/// bound reached — gets its own arm, because those are the ones a caller acts on.
fn classify(error: PlayError) -> DrivenError {
    match error {
        PlayError::CorruptRoot(reason) => DrivenError::CorruptRoot(reason),
        PlayError::Observe { seat, detail } => DrivenError::Unobservable { seat, detail },
        PlayError::Agent { seat, detail } => DrivenError::AgentRefused { seat, detail },
        PlayError::IllegalPick { seat, mv } => DrivenError::IllegalPick { seat, mv },
        PlayError::DeadEnd { seat } => DrivenError::DeadEnd { seat },
        PlayError::Apply { seat, detail } => DrivenError::Engine(format!(
            "apply_move_unchecked rejected a validated move from seat {}: {detail}",
            seat.0
        )),
        PlayError::Invariant { seat, reason } => DrivenError::Engine(format!(
            "invariant violated after seat {}'s move: {reason}",
            seat.0
        )),
        PlayError::TurnCap { turns } => DrivenError::TurnCapReached { turns },
        PlayError::DecisionCap { decisions } => DrivenError::DecisionCapReached { decisions },
    }
}

// ---------------------------------------------------------------------------
// The driver
// ---------------------------------------------------------------------------

/// Play `root` to terminal with `seats`, on streams derived from `master`.
///
/// One agent per seat, built by [`Contestant::build`] — the SHIPPED agents, at
/// their own knobs, through their own honest chains. Nothing about the root is
/// privileged: an agent sees `observe(state, its seat)` and nothing else, and a
/// sampler refusal is a typed hard error rather than a fallback.
pub fn drive_to_terminal(
    root: &GameState,
    seats: &[SeatSpec],
    rules: &Rules,
    master: WorldSeed,
    opts: DrivenOpts,
) -> Result<DrivenOutcome, DrivenError> {
    let players = root.players.len();
    if seats.len() != players {
        return Err(DrivenError::SeatMismatch { seats: seats.len(), players });
    }
    // ONE authority for what the engine plays — the arena's own guard, not a
    // re-typed range (a rooted run that tabulated a 1-seat "game" would produce
    // an outcome whose `final_result` nobody should read).
    require_supported_player_count(players as u8)
        .map_err(|e| DrivenError::UnsupportedRoot(e.to_string()))?;
    if opts.max_decisions == 0 {
        return Err(DrivenError::UnsupportedRoot(
            "max_decisions is 0: a run that may apply no decision is not a run".to_string(),
        ));
    }

    // The agents are built HERE, from the seat list alone. Each gets the seat's
    // own long-lived stream and nothing else — no state, no root, no master —
    // which is the arena's construction channel unchanged.
    let mut agents: Vec<Box<dyn ArenaAgent>> = Vec::with_capacity(players);
    let mut handles: Vec<Option<ExplainHandle>> = Vec::with_capacity(players);
    let mut agent_seeds: Vec<u64> = Vec::with_capacity(players);
    for (index, spec) in seats.iter().enumerate() {
        let seat = PlayerId(index as u8);
        // A rooted run has no seating rotation to identify a contestant across,
        // so the slot component IS the seat index: the derivation stays
        // injective in the seat, and two seats never share a stream.
        let agent_seed = agent_stream_seed(master, seat, Slot(seat.0));
        let (agent, handle) = spec.contestant().build(rules, agent_seed);
        agents.push(agent);
        handles.push(handle);
        agent_seeds.push(agent_seed);
    }

    let mut bench = AgentBench::new(agents);
    let outcome = play_from_state(
        root,
        rules,
        &mut bench,
        PlayLimits { max_turns: opts.max_turns, max_decisions: Some(opts.max_decisions) },
        opts.invariants,
    )
    .map_err(classify)?;

    // The handles are read AFTER the run — they are `Arc`s on live counters, so
    // reading them earlier would report a search that had not happened yet.
    let seat_outcomes: Vec<SeatOutcome> = seats
        .iter()
        .enumerate()
        .map(|(index, spec)| SeatOutcome {
            seat: PlayerId(index as u8),
            spec: *spec,
            echo: spec.contestant().echo(),
            agent_seed: agent_seeds[index],
            applied_moves: outcome.moves_by_seat[index],
            counters: handles[index].as_ref().map_or(SeatCounters::NONE, SeatCounters::from_handle),
        })
        .collect();
    // `None` when NO seat explains, never `Some(0)` — `ExplainHandle`'s rule,
    // carried up one level: a rung field has no explanation count, and a zero
    // would read as "it had one and never used it".
    let explanations = seat_outcomes
        .iter()
        .filter_map(|s| s.counters.explained)
        .fold(None, |acc: Option<u64>, e| Some(acc.unwrap_or(0) + e));

    let result = final_result(&outcome.final_state, rules);
    Ok(DrivenOutcome {
        master,
        decisions: outcome.applied.len() as u64,
        applied: outcome
            .applied
            .into_iter()
            .map(|(seat, mv)| AppliedMove { seat, mv })
            .collect(),
        final_state: outcome.final_state,
        result,
        seats: seat_outcomes,
        explanations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::arena::{
        run_arena_game, ArenaGameStats, ControllerProvenance, DecisionReport,
    };
    use crate::game::invariants::check_invariants;
    use crate::game::record::{state_at, RecordedEvent};
    use crate::game::seeds::SeedSchedule;
    use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
    use crate::types::{GameConfig, PlayerView};
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    /// This task's committed diagnostic master (Phase B, T1). The NAME of the
    /// constructor is the waiver, per `game::seeds`: an asserting test wants
    /// reproducibility, and a rooted fixture cell is exactly the diagnostic case
    /// spec §3's rule carves out.
    const MASTER: u64 = 0x0B01_2026_0817_9990;

    fn world(index: u64) -> WorldSeed {
        let mut schedule = SeedSchedule::diagnostic(MASTER);
        (0..=index).map(|_| schedule.next_world()).last().unwrap()
    }

    /// A plain 2P post-setup state — the cheapest legal root there is.
    fn root_2p(rules: &Rules, seed: u64) -> GameState {
        let pending = new_game(GameConfig::for_player_count(2), rules, seed)
            .expect("new_game builds a 2P setup");
        finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)])
            .expect("the setup finalizes")
    }

    /// One arena game at the IDENTITY seating — slot k at seat k — so its play
    /// streams are `agent_stream_seed(seed, seat, Slot(seat))`, which is exactly
    /// what a driven run at `master = seed` derives.
    fn arena_game(
        rules: &Rules,
        seed: WorldSeed,
        x: Contestant,
        y: Contestant,
    ) -> ArenaGameStats {
        let mut make = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
            let slot = Slot(seat.0);
            let contestant = if slot == Slot(0) { x } else { y };
            contestant.build(rules, agent_stream_seed(seed, seat, slot)).0
        };
        run_arena_game(seed, 2, rules, &mut make, true)
            .unwrap_or_else(|e| panic!("the arena game must complete: {e}"))
    }

    /// Every `Move` a record's event log carries, in order.
    fn recorded_moves(stats: &ArenaGameStats) -> Vec<Move> {
        stats
            .record
            .events
            .iter()
            .filter_map(|e| match e {
                RecordedEvent::Move(mv) => Some(mv.clone()),
                _ => None,
            })
            .collect()
    }

    /// Assert two move sequences are equal, reporting the FIRST divergence
    /// rather than dumping a thousand-move diff.
    fn assert_same_moves(driven: &[Move], arena: &[Move], what: &str) {
        for (i, (d, a)) in driven.iter().zip(arena.iter()).enumerate() {
            assert_eq!(d, a, "{what}: first divergence at applied move {i}");
        }
        assert_eq!(driven.len(), arena.len(), "{what}: the runs differ in LENGTH");
    }

    // -----------------------------------------------------------------------
    // The seat vocabulary
    // -----------------------------------------------------------------------

    /// The declared menu spells the contestants the design names, and the
    /// spelling lives in ONE place. `m = 8` is asserted literally as well as
    /// through the const: the plan names the number, and a const silently
    /// re-pointed at another value would keep this test green while changing
    /// what every adjudication was measured against.
    #[test]
    fn the_reply_menu_spells_the_declared_contestants() {
        assert_eq!(
            ReplyPolicy::FamilyWeighted.contestant(),
            Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
        );
        assert_eq!(ReplyPolicy::GreedyAtFinal.contestant(), Contestant::GREEDY_V1_DEFAULT);
        assert_eq!(ReplyPolicy::GreedyAtFinal.contestant(), Contestant::GreedyV1 { m: 8 });
        assert_eq!(ReplyPolicy::LookaheadAtFinal.contestant(), Contestant::LOOKAHEAD_V1_DEFAULT);
        assert!(
            matches!(
                ReplyPolicy::LookaheadAtFinal.contestant(),
                Contestant::LookaheadV1 { cap: None, .. }
            ),
            "the shipped S1 arm is UNCAPPED: an absent cap is not any set cap",
        );

        // The two arms of `SeatSpec` are one vocabulary, not two agent kinds.
        assert_eq!(
            SeatSpec::Reply(ReplyPolicy::GreedyAtFinal).contestant(),
            SeatSpec::Contestant(Contestant::GREEDY_V1_DEFAULT).contestant(),
        );
        assert_eq!(ReplyPolicy::FamilyWeighted.as_str(), "family-weighted");
    }

    // -----------------------------------------------------------------------
    // Determinism
    // -----------------------------------------------------------------------

    /// Same root, same seats, same master → the same run, twice.
    ///
    /// The third cell is the anti-vacuity half: a digest that never moves would
    /// satisfy the first two assertions no matter what the driver did, so a
    /// DIFFERENT master must produce a different digest.
    #[test]
    fn driving_the_same_root_at_the_same_master_twice_is_byte_identical() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(5).0);
        let seats = [
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
        ];
        let master = world(6);

        let first = drive_to_terminal(&root, &seats, &rules, master, DrivenOpts::default())
            .expect("the run must complete");
        let second = drive_to_terminal(&root, &seats, &rules, master, DrivenOpts::default())
            .expect("the run must complete");

        assert_eq!(first.digest(), second.digest(), "the digest moved between two identical runs");
        assert_eq!(first, second, "the outcomes differ beyond what the digest covers");
        assert!(
            first.decisions > 100,
            "anti-vacuity: a real 2P game is ~1,200 decisions, this one ran {}",
            first.decisions,
        );
        assert_eq!(first.decisions, first.applied.len() as u64);

        let elsewhere = drive_to_terminal(
            &root,
            &seats,
            &rules,
            WorldSeed(master.0 ^ 0x5555_5555_5555_5555),
            DrivenOpts::default(),
        )
        .expect("the run must complete");
        assert_ne!(
            first.digest(),
            elsewhere.digest(),
            "the digest does not depend on the master, so it cannot witness determinism",
        );
    }

    /// The seed discipline, asserted rather than commented: every seat's stream
    /// is `agent_stream_seed(master, seat, Slot(seat))`, and the two seats' are
    /// distinct (keying by master alone would hand both seats one stream).
    #[test]
    fn every_seat_plays_on_the_arena_keyed_stream() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(7).0);
        let master = world(8);
        let outcome = drive_to_terminal(
            &root,
            &[
                SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
                SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
            ],
            &rules,
            master,
            DrivenOpts::default(),
        )
        .expect("the run must complete");

        for (i, seat) in outcome.seats.iter().enumerate() {
            assert_eq!(seat.seat, PlayerId(i as u8));
            assert_eq!(
                seat.agent_seed,
                agent_stream_seed(master, PlayerId(i as u8), Slot(i as u8)),
                "seat {i} did not play on the arena-keyed stream",
            );
        }
        assert_ne!(
            outcome.seats[0].agent_seed, outcome.seats[1].agent_seed,
            "both seats drew from one stream",
        );
        assert_eq!(outcome.master, master);
    }

    // -----------------------------------------------------------------------
    // The arena-reproduction pin (the load-bearing honesty test)
    // -----------------------------------------------------------------------

    /// **THE DRIFT PIN** (design §4). From the state `run_arena_game` reached
    /// after setup, with the same contestants on the same streams, this driver
    /// plays the SAME GAME — move for move, seat for seat, to the same terminal
    /// digest.
    ///
    /// It is the executable form of "the play loop is the arena's": if the two
    /// ever stop sharing a loop, or the rooted seeding stops matching the
    /// arena's keying, or a fallback arm appears on one side, this test fails —
    /// and nothing else in the repo would notice, because a divergent driven run
    /// still produces a legal, plausible-looking game.
    ///
    /// One seat SEARCHES (`greedy-v1` — an evaluator contestant with its own
    /// Keep rule and its own explanation path), because a rung-only cell would
    /// only exercise the `PolicyAgent` chain the arena's own tests already cover.
    #[test]
    fn a_driven_run_from_an_arena_root_reproduces_the_arena_game_move_for_move() {
        let rules = Rules::standard();
        let seed = world(0);
        let x = Contestant::GREEDY_V1_DEFAULT;
        let y = Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT);

        let stats = arena_game(&rules, seed, x, y);
        // `state_at(record, 0)` is the post-setup state with nothing folded onto
        // it — the arena's own root, rebuilt through the public replay path
        // rather than smuggled out of the driver.
        let root = state_at(&stats.record, 0, &rules).expect("the setup preamble replays");

        let outcome = drive_to_terminal(
            &root,
            &[SeatSpec::Contestant(x), SeatSpec::Contestant(y)],
            &rules,
            seed,
            DrivenOpts::default(),
        )
        .expect("the driven run must complete");

        let arena_moves = recorded_moves(&stats);
        assert!(
            arena_moves.len() > 100,
            "anti-vacuity: the arena game was only {} moves, so agreeing with it says little",
            arena_moves.len(),
        );
        let driven_moves: Vec<Move> = outcome.applied.iter().map(|a| a.mv.clone()).collect();
        assert_same_moves(&driven_moves, &arena_moves, "the driven run left the arena's line");

        assert_eq!(
            outcome.moves_by_seat(),
            stats.moves_by_seat,
            "the same moves were attributed to different seats",
        );
        assert_eq!(outcome.result, stats.result, "the same game scored differently");
        assert_eq!(
            Some(state_digest(&outcome.final_state)),
            stats.record.terminal_digest,
            "the driven run ended in a different state than the arena's record digests",
        );

        // The explanation identity, on the seat that HAS one — and its absence
        // on the seat that does not, which is what keeps the identity from
        // passing vacuously.
        assert!(outcome.explanation_identity_holds());
        assert_eq!(outcome.seats[0].counters.explained, Some(outcome.seats[0].applied_moves));
        assert_eq!(outcome.seats[1].counters, SeatCounters::NONE, "a rung explains nothing");
        assert_eq!(outcome.explanations, Some(outcome.seats[0].applied_moves));
        assert_eq!(outcome.seats[0].echo, x.echo());
        assert_eq!(outcome.seats[1].echo, y.echo());
    }

    /// A searching seat's counters ride the outcome — the surface D7's step-one
    /// check reads. Driven from a LATE root so the search is paid for a handful
    /// of decisions rather than a whole game.
    #[test]
    fn a_searching_seats_counters_ride_the_outcome() {
        let rules = Rules::standard();
        let seed = world(1);
        let rung = Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT);
        let stats = arena_game(&rules, seed, rung, rung);
        let events = stats.record.events.len();

        // The first late position where SEAT 0 is on the move, so the searching
        // seat is guaranteed at least one decision. Deterministic: same record,
        // same k.
        let start = events.saturating_sub(30);
        let (root, k) = (start..events)
            .find_map(|k| {
                let state = state_at(&stats.record, k, &rules).ok()?;
                (state.current_player == PlayerId(0)).then_some((state, k))
            })
            .expect("seat 0 moves at least once in the last 30 events");
        assert!(k < events, "the chosen root is not terminal");

        let outcome = drive_to_terminal(
            &root,
            &[
                SeatSpec::Reply(ReplyPolicy::LookaheadAtFinal),
                SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
            ],
            &rules,
            seed,
            DrivenOpts::default(),
        )
        .expect("the driven run must complete");

        let searching = &outcome.seats[0];
        assert_eq!(searching.spec, SeatSpec::Reply(ReplyPolicy::LookaheadAtFinal));
        assert!(searching.applied_moves > 0, "anti-vacuity: the searching seat never moved");
        assert_eq!(
            searching.counters.explained,
            Some(searching.applied_moves),
            "the searching seat applied a move it did not explain",
        );
        assert_eq!(
            searching.counters.ply_cap_hits,
            Some(0),
            "a line stopped at MAX_TURN_PLIES is valued mid-turn (evaluator_gate's own assertion)",
        );
        assert_eq!(
            searching.counters.free_cap_hits,
            Some(0),
            "the turn-boundary scheme never enters the quiescence arm",
        );
        assert!(
            searching.counters.candidates_scored.unwrap_or(0) > 0,
            "a search that scored nothing did not search",
        );
        assert!(searching.counters.root_scored.is_some(), "an S1 arm has a root share");
        assert_eq!(
            searching.counters.budget_binds,
            Some(0),
            "an UNCAPPED arm has no budget to bind",
        );
        assert_eq!(searching.counters.budget_starved_decisions, Some(0));
        // The rung opposite it still counts nothing — the `None`-is-not-zero
        // rule, on the same outcome.
        assert_eq!(outcome.seats[1].counters, SeatCounters::NONE);
        assert_eq!(outcome.explanations, Some(searching.applied_moves));
    }

    // -----------------------------------------------------------------------
    // The refusals
    // -----------------------------------------------------------------------

    /// The non-termination guard is a REFUSAL naming its bound, never a hang and
    /// never a truncated run reported as a result.
    ///
    /// Driven from an ORGANIC root with a tiny bound rather than from a fixture
    /// that actually loops: a test that needs a genuinely non-terminating state
    /// to prove the guard would hang if the guard were missing, which is the one
    /// failure mode a test may not have. The bound is what is under test, and it
    /// is reached identically either way.
    #[test]
    fn a_root_that_does_not_terminate_within_the_bound_is_a_typed_refusal() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(9).0);
        let seats = [
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
        ];
        let opts = DrivenOpts { max_decisions: 5, ..DrivenOpts::default() };

        let err = drive_to_terminal(&root, &seats, &rules, world(10), opts)
            .expect_err("a run that cannot finish inside the bound must refuse");
        match err {
            DrivenError::DecisionCapReached { decisions } => {
                assert_eq!(decisions, 5, "the refusal must name the bound it reached");
                assert!(
                    err.to_string().contains("did not terminate within 5 decisions"),
                    "the message hides the bound: {err}",
                );
            }
            other => panic!("expected DecisionCapReached, got {other:?}"),
        }

        // Zero is not a run.
        let none = DrivenOpts { max_decisions: 0, ..DrivenOpts::default() };
        assert!(matches!(
            drive_to_terminal(&root, &seats, &rules, world(10), none),
            Err(DrivenError::UnsupportedRoot(_)),
        ));
    }

    /// An agent refusal is a HARD error with no fallback arm — the arena's
    /// posture, inherited structurally because the loop IS the arena's.
    ///
    /// **Why this drives the shared loop directly instead of
    /// [`drive_to_terminal`].** The reachable cause of a refusal is
    /// `BeliefSampler::from_view`, and all five of its refusals are shapes an
    /// invariant-clean `GameState` cannot produce through `observe`: a hand deep
    /// enough to overflow (`check_invariants` caps hands at 3), a hidden slot on
    /// the viewer's own seat or two in one hand (`observe` masks at most one
    /// opponent card per seat), and the two conservation mismatches (the order
    /// and tech conservation invariants). `belief.rs` pins all five by doctoring
    /// a VIEW; this surface takes a `&GameState`, so no root reaches them. What
    /// is under test here is therefore the DRIVER's half of the contract — the
    /// refusal stops the run at the first decision, applies nothing, and arrives
    /// typed — and that is exercised end to end with a refusing agent.
    #[test]
    fn an_agent_refusal_is_a_typed_hard_error_with_no_fallback() {
        struct Refuser {
            calls: Arc<AtomicU64>,
        }
        impl ArenaAgent for Refuser {
            fn choose_starting_order(
                &mut self,
                _seat: PlayerId,
                _deal: &[crate::types::OrderCardId; 2],
            ) -> u8 {
                0
            }
            fn choose(
                &mut self,
                view: &PlayerView,
                _rules: &Rules,
            ) -> Result<DecisionReport, String> {
                self.calls.fetch_add(1, Ordering::Relaxed);
                Err(format!("belief sampler refused seat {}'s view: doctored", view.viewer.0))
            }
            fn controller(&self) -> ControllerProvenance {
                ControllerProvenance {
                    name: "test:refuser".to_string(),
                    version: 0,
                    params: BTreeMap::new(),
                    agent_seed: None,
                }
            }
        }

        let rules = Rules::standard();
        let root = root_2p(&rules, world(11).0);
        let calls = Arc::new(AtomicU64::new(0));
        let mut bench = AgentBench::new(vec![
            Box::new(Refuser { calls: Arc::clone(&calls) }) as Box<dyn ArenaAgent>,
            Box::new(Refuser { calls: Arc::clone(&calls) }) as Box<dyn ArenaAgent>,
        ]);

        let err = play_from_state(
            &root,
            &rules,
            &mut bench,
            PlayLimits { max_turns: HONEST_MAX_TURNS, max_decisions: Some(64) },
            true,
        )
        .expect_err("a refusal must fail the run");

        let mover = root.current_player;
        match classify(err) {
            DrivenError::AgentRefused { seat, detail } => {
                assert_eq!(seat, mover, "the refusal named the wrong seat");
                assert!(detail.contains("belief sampler refused"), "detail lost: {detail}");
            }
            other => panic!("expected AgentRefused, got {other:?}"),
        }
        // Had a fallback arm existed, the driver would have applied a move and
        // asked again — which is the corpus-poisoning failure, not a lesser one.
        assert_eq!(calls.load(Ordering::Relaxed), 1, "the loop kept playing past a refusal");
    }

    /// A root the MOVER cannot observe is refused before anything is applied.
    ///
    /// The fixture is the live-tracking shape and it is invariant-CLEAN (asserted
    /// below, so the refusal is about observability and not about corruption):
    /// the seat's dealt card sits in the removed pile with `starting_order_hidden`
    /// set and no identity recorded, exactly as `reveal_starting_order` expects
    /// to find it. A rooted harness will be handed such states by anyone pointing
    /// it at a live-tracked record, so the refusal is a real path, not a
    /// contrivance.
    #[test]
    fn a_root_the_mover_cannot_observe_is_a_typed_refusal() {
        let rules = Rules::standard();
        let mut root = root_2p(&rules, world(12).0);
        let mover = root.current_player;
        let seat = mover.0 as usize;

        let kept = root.players[seat].starting_order.take().expect("a dealt starting order");
        root.players[seat].order_cards_in_hand.retain(|c| *c != kept);
        root.orders_removed.push(kept);
        root.players[seat].starting_order_hidden = true;
        check_invariants(&root, &rules).expect("the live-tracking root must be invariant-clean");

        let err = drive_to_terminal(
            &root,
            &[
                SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
                SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
            ],
            &rules,
            world(13),
            DrivenOpts::default(),
        )
        .expect_err("a root the mover cannot observe must refuse");
        match err {
            DrivenError::Unobservable { seat, .. } => assert_eq!(seat, mover),
            other => panic!("expected Unobservable, got {other:?}"),
        }
    }

    /// A corrupt root is refused BEFORE the first decision, so a broken fixture
    /// is never misattributed to a move — every driver in the repo guards its
    /// incoming state, and a rooted one has more reason to than most.
    #[test]
    fn a_corrupt_root_is_refused_before_the_first_decision() {
        let rules = Rules::standard();
        let mut root = root_2p(&rules, world(14).0);
        root.players[0].assistants.in_hand += 1;
        assert!(check_invariants(&root, &rules).is_err(), "the fixture must actually be corrupt");

        let err = drive_to_terminal(
            &root,
            &[
                SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
                SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
            ],
            &rules,
            world(15),
            DrivenOpts::default(),
        )
        .expect_err("a corrupt root must refuse");
        match err {
            DrivenError::CorruptRoot(reason) => {
                assert!(reason.contains("assistants total"), "the reason is lost: {reason}");
            }
            other => panic!("expected CorruptRoot, got {other:?}"),
        }
    }

    /// The seat list must describe the root's seats — a shorter list would
    /// silently leave a seat unnamed, and a longer one would name a seat that
    /// does not exist.
    #[test]
    fn the_seat_list_must_name_every_seat_of_the_root() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(16).0);
        let err = drive_to_terminal(
            &root,
            &[SeatSpec::Reply(ReplyPolicy::FamilyWeighted)],
            &rules,
            world(17),
            DrivenOpts::default(),
        )
        .expect_err("a one-seat list for a two-seat root must refuse");
        match err {
            DrivenError::SeatMismatch { seats, players } => {
                assert_eq!((seats, players), (1, 2));
            }
            other => panic!("expected SeatMismatch, got {other:?}"),
        }
    }
}
