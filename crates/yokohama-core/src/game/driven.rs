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
//! committed diagnostic one. **The reproduction pin rests on one further
//! property of the shipped agents, stated because it is a property and not a
//! guarantee**: a Keep rule never touches the PLAY stream — every evaluator
//! contestant decides its deal from the deal alone, and `PolicyAgent` draws
//! from a fresh `setup_stream_seed(seat, deal)` rng — so the arena's setup half
//! does not advance the stream its play half then samples from, which is what
//! lets a rooted run starting at stream position 0 reproduce it. An agent that
//! broke this would fail the pin rather than drift past it.
//!
//! **NO `GameRecord` IS EMITTED, ever** (design §3). A rooted run is not a
//! from-setup game: it has no `new_game` seed, no setup choices of its own, and
//! nothing that would let `replay` reconstruct it from the top. Stamping one
//! would put a file in the corpus that claims to be reproducible from setup and
//! is not. What travels instead is [`DrivenOutcome`] — moves with their movers,
//! the final state, the result, and the per-seat counters.
//!
//! **What T2 adds**: [`root_table`], the ONE DECISION analyzed — the mechanical
//! separator's instrument, and the surface on which D7's step-one check reads
//! all fifteen search counters rather than the two `why` prints.
//!
//! **What T3 adds**: WHERE a root comes from and WHAT a run says about itself.
//! [`RootSource`] resolves the three origins the design names — a state handed
//! in, the prefix of a record ([`RootSource::RecordPrefix`], `why n`'s own
//! index), and a seeded scan for a position satisfying a [`RootPredicate`] —
//! each into a [`RootDescriptor`] that identifies the root as DATA.
//! [`RootedProvenance`] rides EVERY [`DrivenOutcome`]: the descriptor, the
//! running `semantics_rev` and `rules_hash`, the master, and one
//! [`SeatDescriptor`] per seat carrying the RESOLVED contestant echo and the
//! observation label the driver witnessed. [`registry`] names the session's own
//! roots, absent-tolerantly. And the ROOT-ELIGIBILITY precheck refuses a
//! live-tracked root by name before a move is played, instead of letting it
//! surface as a mid-run `observe` failure at some seat's first decision.
//!
//! **What this module still deliberately does NOT ship**: `adjudicate_root`
//! (T4) and the runner (T5). Neither is stubbed here — a stub is a surface a
//! reader has to check the emptiness of.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::game::agent::HONEST_MAX_TURNS;
use crate::game::arena::{
    agent_stream_seed, play_from_state, require_supported_player_count, run_arena_game, AgentBench,
    ArenaAgent, Contestant, ExplainHandle, PlayError, PlayLimits, SeatBench, Slot,
};
use crate::game::belief::BeliefSampler;
use crate::game::end_game::is_game_over;
use crate::game::eval::WeightTable;
use crate::game::invariants::check_invariants;
use crate::game::lookahead::{LookaheadAgent, RootScore, SearchScheme, SearchStats};
use crate::game::observe::observe;
use crate::game::policy::RolloutPolicy;
use crate::game::record::{replay_from, state_at, GameRecord, RecordedEvent, SeatProvenance};
use crate::game::rules::Rules;
use crate::game::scoring::{final_result, GameResult};
use crate::game::seeds::{SeedSchedule, WorldSeed};
use crate::game::semantics::{record_digest, state_digest, SEMANTICS_REV};
use crate::types::{AreaId, GameState, Move, PlayerId};

/// The decision guard's default: how many decisions a rooted run may apply
/// before it is refused as non-terminating.
///
/// GENEROUS by design — organic games run ~1,200 applied moves at 2P and ~1,700
/// at 4P (the `policy_diversity` gate's own totals), and a rooted run is a
/// SUFFIX of one, so this is roughly 60× the longest thing an organic root can
/// produce. It is not a performance budget and nothing tunes against it: it
/// exists so that a HAND-BUILT fixture with an absorbing no-op loop — the T3
/// null-placement pathology, which loops WITHOUT advancing `turn_number` and so
/// slips under the turn cap — becomes a typed refusal instead of a hang.
///
/// BOUNDED is the property, not FAST: at this default an uncapped
/// `lookahead-v1` seat can spend minutes reaching the bound. A caller who wants
/// a quick refusal sets a smaller one (which is what the guard's own test
/// does); what the default promises is that the run ends.
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
///
/// **The two `*AtFinal` arms are PROVISIONAL, and the word is load-bearing** —
/// the same obligation `yokohama-cli`'s `bot::PROVISIONAL_DEFAULT_BOT` carries,
/// written here because this is core's own copy of it. Today they bind to the
/// SHIPPED defaults ([`Contestant::GREEDY_V1_DEFAULT`] and
/// [`Contestant::LOOKAHEAD_V1_DEFAULT`]); at D1's identity freeze they re-point
/// at the frozen champion constructed from the EXPLICIT knobs that
/// pre-registration records — never from a `*_DEFAULT` constant, which is D1's
/// own rule — in the same commit that re-points the CLI's const, so the seat you
/// play against and the seat a rooted question is measured against cannot drift
/// apart. Until then the shared defaults are themselves frozen identity (D1:
/// they may not move without an owner ruling), so the binding cannot drift
/// silently either. What a report quotes is the RESOLVED echo and never the menu
/// name: the name is a symbol whose meaning moves at the freeze.
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
    ///
    /// THE single materialization path — [`drive_to_terminal`] builds every seat
    /// through this one call — which is what makes the sugar-equivalence of
    /// `Reply(r)` and `Contestant(r.contestant())` hold BY CONSTRUCTION rather
    /// than by a test agreeing with a second construction site.
    pub fn contestant(self) -> Contestant {
        match self {
            Self::Contestant(c) => c,
            Self::Reply(r) => r.contestant(),
        }
    }

    /// What the caller ASKED FOR, as provenance text: `"contestant"` when a
    /// contestant was named directly, `"reply:<menu name>"` when a menu slot
    /// was.
    ///
    /// It rides [`SeatDescriptor::requested`] BESIDE the resolved echo, and the
    /// pair is the point (design §3, review S-1): the echo says what actually
    /// played, and this says which vocabulary chose it. At D1's identity freeze
    /// a `reply:lookahead-at-final` seat's echo MOVES while this string does
    /// not, so an artifact carrying both makes the re-pointing visible instead
    /// of leaving two runs looking equally like "the reply policy".
    pub fn requested_label(self) -> String {
        match self {
            Self::Contestant(_) => "contestant".to_string(),
            Self::Reply(r) => format!("reply:{}", r.as_str()),
        }
    }
}

// ---------------------------------------------------------------------------
// Roots: where a rooted run starts, and what it says about itself (T3)
// ---------------------------------------------------------------------------

/// WHICH POSITIONS a [`RootSource::Generated`] scan looks for.
///
/// v1 ships the TYPE plus exactly ONE predicate (design §3), and the restraint
/// is deliberate: a menu of predicates invented before a consumer asks for them
/// is a menu of fixtures chosen after seeing what each one selects. Layer 2 adds
/// variants at family freeze, with the family that needs them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RootPredicate {
    /// The final round is UNDER WAY and a decision still faces a mover:
    /// `end_game_triggered && final_round && !is_game_over`.
    ///
    /// All three conjuncts are load-bearing, and two of them are easy to get
    /// wrong. `end_game_triggered` ALONE is the TRIGGER round — the round the
    /// trigger fired in, which still has a whole final round after it
    /// (`apply_move`'s phase advance sets `final_round` only when the turn
    /// passes back to the start player) — so it would select positions with
    /// several turns of play left, which is not what family 1 asks about.
    /// `!is_game_over` excludes the terminal state: a terminal root is a
    /// legitimate thing to hand [`drive_to_terminal`] (it answers with a
    /// zero-move run, deliberately) but it is not a POSITION anyone can
    /// adjudicate a candidate at, and a scan that returned one would hand the
    /// adjudicator a root with no decision in it.
    FinalRoundAlreadyTriggered,
}

impl RootPredicate {
    /// The stable name, for descriptors and error text.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FinalRoundAlreadyTriggered => "final-round-already-triggered",
        }
    }

    /// Whether `state` is one of the positions this predicate names.
    pub fn holds(self, state: &GameState) -> bool {
        match self {
            Self::FinalRoundAlreadyTriggered => {
                state.end_game_triggered && state.final_round && !is_game_over(state)
            }
        }
    }
}

/// The v1 generating configuration's NAME. Bump the version in the same commit
/// as ANY change to a [`GeneratingEcology::v1`] field.
pub const GENERATED_ECOLOGY_V1: &str = "driven-generated-v1";

/// How many games a [`RootSource::Generated`] scan may play before it reports
/// the supply exhausted.
///
/// A BOUND, not a budget: the scan must terminate on an ordinal nothing
/// satisfies, and the honest way to say that is a refusal naming how far it
/// looked. MEASURED at this bound (the exhaustion cell's own master, 2026-08-18,
/// WSL debug): **54 final-round positions across the four games, ~14 per game**,
/// which is far more ordinals than family 1 asks for at a wall a debug test can
/// pay (0.30s for the whole four-game scan). A caller wanting more raises the
/// bound and re-records the ecology — which is why the bound rides
/// [`GeneratingEcology::games`] rather than living only here.
pub const GENERATED_SCAN_GAMES: u32 = 4;

/// The seat count v1's generating ecology plays at.
const GENERATED_PLAYERS: u8 = 2;

/// The contestant every seat of v1's generating ecology plays.
///
/// The CHEAPEST honest seat there is — the shipped rollout rung — chosen so a
/// scan costs games rather than searches. It is not a claim that final-round
/// positions reached by rungs resemble the ones a bot reaches; that is exactly
/// why the ecology is RECORDED on the descriptor rather than assumed, so a later
/// stronger ecology is a visible change of identity instead of a silent
/// re-pointing of every `(predicate, master, ordinal)` triple ever quoted.
const GENERATED_SEAT: Contestant = Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT);

/// The play that PRODUCED a generated root, recorded as data.
///
/// Review S-5's finding made a type: `(predicate, master, ordinal)` alone does
/// NOT identify a position. The ordinal-th root satisfying a predicate is a fact
/// about the games the scan played, so a later change to the generating seats,
/// the player count, the seed derivation or the scan bound re-points every
/// recorded triple at a different state while every recorded triple still reads
/// exactly the same. A descriptor carrying this cannot do that silently: two
/// runs whose ecologies differ are visibly two different questions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratingEcology {
    /// The configuration's name and version — [`GENERATED_ECOLOGY_V1`].
    pub id: String,
    /// What played the games.
    pub driver: String,
    /// The seat count.
    pub players: u8,
    /// The generating seats' RESOLVED echoes, in seat order — never a bare
    /// name, for [`SeatDescriptor::echo`]'s reason.
    pub seats: Vec<String>,
    /// How a game's world seed and each seat's stream are derived.
    pub seed_derivation: String,
    /// The scan bound this run used ([`GENERATED_SCAN_GAMES`]) — recorded
    /// because an `Exhausted` refusal and a found root are answers about the
    /// same bound, and a later bound change makes an old "exhausted" reading
    /// non-comparable.
    pub games: u32,
}

impl GeneratingEcology {
    /// The v1 configuration — THE single spelling of it.
    ///
    /// Every string here is prose a reader acts on, not a key anything parses:
    /// what changes an identity is the FIELD SET plus [`GENERATED_ECOLOGY_V1`],
    /// and a reworded driver line that left the play unchanged would be a
    /// spurious identity change. Reword freely; re-point nothing.
    pub fn v1() -> Self {
        Self {
            id: GENERATED_ECOLOGY_V1.to_string(),
            driver: "arena::run_arena_game (honest bench, per-move invariants ON)".to_string(),
            players: GENERATED_PLAYERS,
            seats: vec![GENERATED_SEAT.echo(); GENERATED_PLAYERS as usize],
            seed_derivation: "SeedSchedule::diagnostic(master).next_world() once per game in \
                              scan order; agent_stream_seed(world, seat, Slot(seat)) per seat"
                .to_string(),
            games: GENERATED_SCAN_GAMES,
        }
    }
}

/// WHERE a root came from, as data that travels.
///
/// The descriptor half of [`RootSource`]: no `GameState`, no `GameRecord`, so it
/// serializes into a sidecar and compares cheaply, and it identifies the root by
/// what a reader can act on — a document digest, or the seeded scan that found
/// it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RootOrigin {
    /// A `GameState` handed straight to the driver — a fixture, a hand-built
    /// position, a state a caller derived some other way. There is nothing to
    /// point AT but the state itself, which is what
    /// [`RootDescriptor::root_digest`] is for.
    Constructed,
    /// The prefix of a recorded game, identified by the record's own document
    /// digest ([`record_digest`]).
    RecordPrefix { record_digest: String },
    /// The ordinal-th position a seeded scan found, with the ecology that
    /// produced it.
    Generated {
        predicate: RootPredicate,
        master: u64,
        ordinal: u32,
        /// Which game of the scan held it — reproduction and diagnosis only
        /// (the generated games are not saved anywhere; re-running the same
        /// master rebuilds them).
        game_index: u32,
        ecology: GeneratingEcology,
    },
}

impl RootOrigin {
    /// The stable kind name, for one-line digests and report headers.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Constructed => "constructed",
            Self::RecordPrefix { .. } => "record-prefix",
            Self::Generated { .. } => "generated",
        }
    }
}

/// A root, identified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootDescriptor {
    pub origin: RootOrigin,
    /// The event index the root sits at, when its origin has one.
    ///
    /// `state_at`'s convention EXACTLY — `k` counts events APPLIED, so this is
    /// the position the mover decided IN before the move that became event `k`,
    /// which is the index `why <n>` prints. `None` for a constructed root, which
    /// has no log to be an index into. For a GENERATED root it indexes the
    /// scan's own game, which lives nowhere but the master that rebuilds it.
    pub event_index: Option<usize>,
    /// [`state_digest`] of the root state itself — the identity every origin
    /// has, and the one thing two descriptions of the same position must agree
    /// on.
    pub root_digest: String,
}

impl RootDescriptor {
    /// The descriptor for a state handed straight in.
    ///
    /// Not a stub for a missing provenance: a caller who built a fixture IS the
    /// provenance, and the honest record of that is the state's own digest plus
    /// the statement that nothing else was claimed.
    pub fn constructed(root: &GameState) -> Self {
        Self {
            origin: RootOrigin::Constructed,
            event_index: None,
            root_digest: state_digest(root),
        }
    }
}

/// WHERE a rooted run starts.
///
/// The three origins design §3 names. `Constructed` and `RecordPrefix` carry
/// their material (a state, a record) because resolving them is pure; `Generated`
/// carries only a question, because resolving it is a seeded SCAN — which is why
/// its descriptor records the ecology that answered it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootSource {
    /// A state a caller built or holds — the `scenario_suite` grain.
    Constructed(GameState),
    /// The state event `event_index` was decided in, replayed from `record`.
    ///
    /// `event_index` is `state_at`'s `k` and `why <n>`'s `n` — the SAME number,
    /// deliberately, so a root taken from a session's own `why` output needs no
    /// arithmetic to become a fixture here.
    RecordPrefix { record: GameRecord, event_index: usize },
    /// The `ordinal`-th position satisfying `predicate` in a seeded scan at
    /// `master`.
    ///
    /// **`(predicate, master, ordinal)` is not the whole identity** — see
    /// [`GeneratingEcology`], which the resolved descriptor carries for exactly
    /// that reason.
    Generated { predicate: RootPredicate, master: u64, ordinal: u32 },
}

impl RootSource {
    /// The root this source names, with the descriptor that identifies it.
    ///
    /// Every arm is TOTAL and every failure is typed: an out-of-range prefix
    /// index, a generating game that did not complete, an ordinal beyond the
    /// scan's supply. None of them is a state — a caller that cannot get its
    /// root gets a sentence naming what it asked for.
    ///
    /// **Eligibility is NOT checked here.** A root whose seats carry hidden
    /// deals resolves fine and is refused by [`drive_to_terminal`] (the one
    /// site, so a bare-state caller gets the same refusal a resolved one does);
    /// see `require_eligible_root`.
    pub fn resolve(&self, rules: &Rules) -> Result<(GameState, RootDescriptor), RootError> {
        match self {
            Self::Constructed(state) => {
                Ok((state.clone(), RootDescriptor::constructed(state)))
            }
            Self::RecordPrefix { record, event_index } => {
                let state = state_at(record, *event_index, rules).map_err(|e| RootError::Prefix {
                    event_index: *event_index,
                    events: record.events.len(),
                    detail: e.to_string(),
                })?;
                let root_digest = state_digest(&state);
                Ok((
                    state,
                    RootDescriptor {
                        origin: RootOrigin::RecordPrefix { record_digest: record_digest(record) },
                        event_index: Some(*event_index),
                        root_digest,
                    },
                ))
            }
            Self::Generated { predicate, master, ordinal } => {
                generate_root(*predicate, *master, *ordinal, rules)
            }
        }
    }
}

/// Why a [`RootSource`] produced no root.
#[derive(Debug)]
pub enum RootError {
    /// The record does not carry the asked-for prefix.
    Prefix { event_index: usize, events: usize, detail: String },
    /// A generating game did not complete, so the scan cannot continue: the
    /// ordinal is counted over a sequence, and skipping a game would silently
    /// re-point every ordinal after it.
    Generate { game_index: u32, detail: String },
    /// The scan reached its bound with fewer than `ordinal + 1` hits. Names what
    /// it FOUND and how far it looked, so a caller can tell "no such position"
    /// from "not in this many games".
    Exhausted { predicate: RootPredicate, ordinal: u32, found: u32, games: u32 },
}

impl std::fmt::Display for RootError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prefix { event_index, events, detail } => write!(
                f,
                "no root at event {event_index}: the record carries {events} events ({detail})"
            ),
            Self::Generate { game_index, detail } => write!(
                f,
                "the scan's game {game_index} did not complete, so no ordinal after it is \
                 well defined: {detail}"
            ),
            Self::Exhausted { predicate, ordinal, found, games } => write!(
                f,
                "no root {ordinal} for {}: {found} found in {games} generated games",
                predicate.as_str()
            ),
        }
    }
}

impl std::error::Error for RootError {}

/// What ONE seat of a rooted run was, recorded so a report never has to look it
/// up.
///
/// **The echo and the observation label are the two load-bearing fields**
/// (design §3, review S-1/N-5). A menu name is a symbol whose meaning moves at
/// D1's identity freeze, so an artifact carrying only `reply:lookahead-at-final`
/// would describe two different opponents identically; the resolved echo carries
/// the version and every knob. The observation label is single-sourced from the
/// DRIVER'S OWN BENCH rather than from the contestant — `SeatProvenance`'s rule,
/// for its reason: a controller labelling its own information access is exactly
/// the claim that needs an independent witness.
///
/// [`controller`](Self::controller) + [`params`](Self::params) are the record's
/// own two halves, so `Contestant::from_parts` reconstructs the seat from this
/// descriptor alone — a property the provenance test pins, and the reason the
/// pair rides here beside the (display-only) echo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatDescriptor {
    pub seat: u8,
    /// What the caller ASKED for — see [`SeatSpec::requested_label`].
    pub requested: String,
    /// What actually played — `Contestant::echo`.
    pub echo: String,
    /// The controller's stable wire name, as the seat's own agent reported it.
    pub controller: String,
    /// That controller's implementation version.
    pub version: u32,
    /// Its knobs in their declared integer units, identity keys included.
    pub params: BTreeMap<String, i64>,
    /// What this seat could SEE, witnessed by the driver's bench.
    pub observation: String,
    /// The seat's play stream, from `agent_stream_seed`.
    pub agent_seed: u64,
}

/// What a rooted run WAS — stamped on every [`DrivenOutcome`].
///
/// **Never a `GameRecord`** (design §3): a rooted run is not a from-setup game.
/// It has no `new_game` seed of its own, no setup choices, and nothing that
/// would let `replay` reconstruct it from the top, so a record claiming
/// otherwise would put a file in the corpus that says it is reproducible from
/// setup and is not. This says what a rooted run can honestly say: which
/// position, under which engine semantics and rules data, with which seats, off
/// which master.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootedProvenance {
    /// Which position, and where it came from.
    pub root: RootDescriptor,
    /// The RUNNING engine's `SEMANTICS_REV` — the comparability key. A figure
    /// from another rev is not this figure, whatever else matches.
    pub semantics_rev: u32,
    /// The RUNNING `Rules::content_hash()`.
    pub rules_hash: String,
    /// One entry per seat, in seat order.
    pub seats: Vec<SeatDescriptor>,
    /// The master every seat's stream descends from.
    pub master: u64,
}

impl RootedProvenance {
    /// Stamp a run. Private: provenance is what the DRIVER witnessed, so it is
    /// assembled where the witnessing happened and nowhere else.
    fn stamp(
        root: RootDescriptor,
        rules: &Rules,
        master: WorldSeed,
        specs: &[SeatSpec],
        witnessed: &[SeatProvenance],
        agent_seeds: &[u64],
    ) -> Self {
        let seats = specs
            .iter()
            .enumerate()
            .map(|(index, spec)| {
                let seen = &witnessed[index];
                // PLUMBING PIN: the stream this descriptor records is the one
                // the seat's own agent says it holds. The two come from
                // different places on purpose — the driver derived
                // `agent_seeds[index]` and the AGENT reports what it was built
                // on — so a driver that ever built a seat on one stream and
                // recorded another would produce provenance that reproduces
                // nothing, silently. Once per seat per run, and debug-only
                // because a runner should not abort on it while every test run
                // is a debug run.
                debug_assert_eq!(
                    seen.agent_seed,
                    Some(agent_seeds[index]),
                    "seat {index} was built on a different stream than the run recorded",
                );
                SeatDescriptor {
                    seat: index as u8,
                    requested: spec.requested_label(),
                    echo: spec.contestant().echo(),
                    controller: seen.controller.clone(),
                    version: seen.version,
                    params: seen.params.clone(),
                    observation: seen.observation.clone(),
                    agent_seed: agent_seeds[index],
                }
            })
            .collect();
        Self {
            root,
            semantics_rev: SEMANTICS_REV,
            rules_hash: rules.content_hash(),
            seats,
            master: master.0,
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// path.
    ///
    /// `None` means NO SEAT EXPLAINS (every seat a rung) — the counter-absence
    /// distinction, not a zero. **`Some(0)` is a real and reachable reading**: a
    /// seat that DOES explain and was asked for no decisions, which is exactly
    /// what an already-terminal root produces. The two are different statements
    /// and a reader must not collapse them.
    pub explanations: Option<u64>,
    /// What this run WAS (T3): the root's identity, the engine's semantics and
    /// rules stamps, the resolved seats, the master.
    ///
    /// Stamped on EVERY outcome, including one from [`drive_to_terminal`]'s
    /// bare-state entry point — which stamps [`RootOrigin::Constructed`],
    /// because that is what a state handed straight in honestly is.
    pub provenance: RootedProvenance,
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
    ///
    /// **It names the ROOT since T3** — the origin KIND and the root state's own
    /// digest, off [`provenance`](Self::provenance). Before that it did not, and
    /// the gap was real: two different roots that happen to reach the same
    /// terminal state under the same master digested alike, so a quoted digest
    /// identified the run without identifying the question it answered. The
    /// FULL provenance (rules hash, seat knobs, the record digest a prefix root
    /// came from) is deliberately not folded in here — it rides
    /// [`RootedProvenance`], which serializes; this line stays a line.
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
            "driven-v1 rev={} root={}@{} master={:#018x} decisions={} state={} winner=p{} \
             seats=[{}] standings=[{}]",
            self.provenance.semantics_rev,
            self.provenance.root.origin.kind(),
            self.provenance.root.root_digest,
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
    /// The [`RootSource`] produced no root at all — there was nothing to play
    /// from. Kept as ONE variant wrapping [`RootError`] rather than flattened:
    /// this taxonomy answers "why is there no outcome", and "the root could not
    /// be resolved" is one answer whose own reasons are that type's business.
    Root(RootError),
    /// The seat list does not describe the root's seats.
    SeatMismatch { seats: usize, players: usize },
    /// The root, or the options, are not something this driver can play.
    UnsupportedRoot(String),
    /// The root violates an engine invariant — refused BEFORE the first
    /// decision, so a corrupt fixture is never misattributed to a move.
    CorruptRoot(String),
    /// A seat cannot observe the root. The reachable case is a live-tracked
    /// seat whose own deal was never revealed.
    ///
    /// **Since T3 this is normally an UP-FRONT refusal about ANY seat**, not
    /// only the mover: `require_eligible_root` scans every seat's
    /// `starting_order_hidden` before a move is played, so a live-tracked root
    /// is refused as the ineligible ROOT it is instead of surfacing at whichever
    /// seat happened to decide first. The mid-run arm behind it is retained as
    /// defence in depth — it is what `classify` maps `observe`'s own refusal to
    /// — and the DETAIL is the discriminator: the precheck names
    /// `starting_order_hidden`, `observe`'s refusal names its own view error.
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
            Self::Root(e) => write!(f, "no root: {e}"),
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
        // Through an invariant-clean root only `ViewError::ViewerDealUnrecorded`
        // is reachable here (`SeatOutOfRange` needs a `current_player` no clean
        // state carries), and only for the seat ON THE MOVE — a hidden deal on
        // a NON-mover seat refuses mid-run at that seat's first decision instead
        // of up front, which is why design §5's all-seats eligibility precheck
        // is T3's obligation rather than something this arm quietly covers.
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
///
/// **An ALREADY-TERMINAL root is an `Ok`, not a refusal** — `decisions == 0`,
/// `applied` empty, `result` the root's own `final_result`, and an explaining
/// seat reading `Some(0)`. That is deliberate and load-bearing: T4's
/// adjudicator forces a candidate move and drives the REMAINDER, and a
/// candidate that ends the game leaves a remainder of exactly nothing. Refusing
/// it would make "this move wins immediately" the one outcome the instrument
/// could not report.
pub fn drive_to_terminal(
    root: &GameState,
    seats: &[SeatSpec],
    rules: &Rules,
    master: WorldSeed,
    opts: DrivenOpts,
) -> Result<DrivenOutcome, DrivenError> {
    drive_from_descriptor(root, RootDescriptor::constructed(root), seats, rules, master, opts)
}

/// Resolve `source` and play it to terminal — [`drive_to_terminal`] with the
/// root's PROVENANCE carried through instead of reconstructed.
///
/// The two entry points share one driver ([`drive_from_descriptor`]) and differ
/// only in what the resulting [`RootedProvenance`] can honestly say: a bare
/// state is [`RootOrigin::Constructed`], and a resolved source names the record
/// or the seeded scan it came from. That difference is the whole reason this
/// function exists — an outcome whose root descriptor was rebuilt by the driver
/// from a state it was handed could never say more than "someone gave me this
/// position".
pub fn drive_rooted(
    source: &RootSource,
    seats: &[SeatSpec],
    rules: &Rules,
    master: WorldSeed,
    opts: DrivenOpts,
) -> Result<DrivenOutcome, DrivenError> {
    let (root, descriptor) = source.resolve(rules).map_err(DrivenError::Root)?;
    drive_from_descriptor(&root, descriptor, seats, rules, master, opts)
}

/// Refuse a root whose seats carry HIDDEN DEALS, before anything is played
/// (design §5, review N-3).
///
/// The reachable source is a LIVE-TRACKED record: an opponent who kept their
/// dealt starting order face-down carries `starting_order_hidden` with the
/// card's identity recorded nowhere in the state, and `observe` refuses to
/// derive that seat's own view (a seat may not be ignorant of its own deal).
/// Left to the run, that refusal surfaces at the seat's FIRST DECISION — after
/// the other seats have already played, as an error about a seat the caller
/// never chose, at a position the caller never asked about.
///
/// **EVERY seat is checked, not only the mover**, and the non-mover is the whole
/// point: the mover's own hidden deal is the case `observe` catches on the first
/// decision anyway, while a non-mover's is the one it catches too late.
///
/// The refusal reuses [`DrivenError::Unobservable`] rather than adding a second
/// variant, because it is the same fact — this seat cannot be observed here —
/// learned earlier. What tells them apart is the DETAIL, which names
/// `starting_order_hidden` and the eligibility rule.
fn require_eligible_root(root: &GameState) -> Result<(), DrivenError> {
    for player in &root.players {
        if player.starting_order_hidden {
            return Err(DrivenError::Unobservable {
                seat: player.id,
                detail: format!(
                    "seat {} carries a hidden starting-order deal (starting_order_hidden), so its \
                     own view cannot be derived: a live-tracked root is not driveable",
                    player.id.0,
                ),
            });
        }
    }
    Ok(())
}

/// The ONE driver both entry points run, with the root's descriptor supplied
/// rather than assumed.
fn drive_from_descriptor(
    root: &GameState,
    descriptor: RootDescriptor,
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
    // an outcome whose `final_result` nobody should read). The narrowing runs
    // through `try_from` rather than an `as` cast: `as` would WRAP a nonsense
    // seat count into a supported one, and the guard would then pass on a state
    // it should have refused.
    let seat_count = u8::try_from(players)
        .map_err(|_| DrivenError::UnsupportedRoot(format!("root carries {players} seats")))?;
    // The GUARD is the arena's; the RENDERING is this module's. Forwarding
    // `ArenaError`'s own `Display` would nest "arena driver:" inside "unsupported
    // root:" — an error naming a driver this run is not — so the count is
    // formatted here and the arena answer is used only as the verdict.
    require_supported_player_count(seat_count).map_err(|_| {
        DrivenError::UnsupportedRoot(format!(
            "root carries {seat_count} seats (the engine plays 2-4)"
        ))
    })?;
    if opts.max_decisions == 0 {
        return Err(DrivenError::UnsupportedRoot(
            "max_decisions is 0: a run that may apply no decision is not a run".to_string(),
        ));
    }
    // ELIGIBILITY before agents: a root nobody can play is not worth building
    // agents for, and the refusal must not depend on which seat moves first.
    require_eligible_root(root)?;

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

    // WITNESSED, not asserted: the bench states each seat's observation label
    // (it IS the mode — `AgentBench` passes views and nothing else can) and each
    // agent states its own controller descriptor. Read here rather than derived
    // from the seat list so the provenance records what the driver actually
    // seated, which is the independent witness `SeatProvenance`'s own rule asks
    // for.
    let witnessed = bench.seat_provenance();

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
    // `None` when NO seat explains — `ExplainHandle`'s counter-absence rule,
    // carried up one level: a rung has no explanation count at all, and a zero
    // would read as "it had one and never used it". `Some(0)` says exactly
    // that latter thing and is reachable on purpose (a terminal root), so the
    // fold seeds from `None` and only an explaining seat can start the sum.
    let explanations = seat_outcomes
        .iter()
        .filter_map(|s| s.counters.explained)
        .fold(None, |acc: Option<u64>, e| Some(acc.unwrap_or(0) + e));

    let result = final_result(&outcome.final_state, rules);
    let provenance =
        RootedProvenance::stamp(descriptor, rules, master, seats, &witnessed, &agent_seeds);
    Ok(DrivenOutcome {
        master,
        provenance,
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

// ---------------------------------------------------------------------------
// The root table: ONE decision, analyzed
// ---------------------------------------------------------------------------

/// The width of the top BAND [`RootTable::top_band`] reports, in evaluator
/// points.
///
/// **One point, and the number is the e113 lesson rather than a taste.** At that
/// root the shipped rung ranked 139 candidates and 100 of them valued within ONE
/// point of the best, while only a handful were tied AT it. A report carrying
/// [`tied_at_top`](RootTable::tied_at_top) alone describes that decision as "a
/// few candidates were equal" when the honest statement is that the evaluator
/// separated almost nothing — and the mechanical separator's question (does a
/// wider beam SEE more here?) is a question about the second number.
pub const TOP_BAND_POINTS: i64 = 1;

/// WHICH WORLD the analyzed decision was taken in.
///
/// A searching contestant does not decide in the true state: it decides in one
/// world its belief sampler determinized from the seat's view. The separator's
/// premise — that b=4, b=8 and b=16 at one root differ only in how much of the
/// SAME world they searched — is therefore a claim about the sampler, and this
/// is the claim RECORDED rather than inferred (design §3, review N-1). It holds
/// because the agent draws its three stream words (sampler seed, tie key,
/// candidate seed) before any beam-dependent work, so equal decision seeds
/// sample equal worlds at every beam; two tables that disagree here were not
/// comparable and the numbers say so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampledWorld {
    /// The seed the AGENT DREW from its own play stream and handed to
    /// [`BeliefSampler::from_view`] — read off the agent, never re-derived from
    /// the decision seed by a second copy of its draw order.
    pub sampler_seed: u64,
    /// Worlds this contestant samples per decision (`K`), read off the agent.
    /// 1 at the shipped knobs.
    pub worlds: u8,
    /// [`state_digest`] of the FIRST sampled world.
    ///
    /// **REBUILT from [`sampler_seed`](Self::sampler_seed), not read out of the
    /// agent** — which keeps a `GameState` clone per decision off the agent's hot
    /// path. It is the same world by CONSTRUCTION and not an inference about it:
    /// `BeliefSampler::from_view` is a deterministic function of `(view, rules,
    /// seed)` and the first `sample()` off a fresh sampler is the first world the
    /// agent searched. At `K > 1` the later worlds are not digested — one
    /// identity, stated, beats a vector nothing reads.
    pub digest: String,
}

/// ONE decision point, with every root candidate the contestant valued.
///
/// The instrument D7's mechanical separator runs on, and the answer to a
/// question `why` cannot ask: `why` renders ONE decision's chosen move with 2 of
/// the 15 search counters, while a separator needs the whole ranking's SHAPE
/// (how much did the evaluator actually separate?) and the whole counter set
/// (did the search do more work, and where?) at a root it re-runs at several
/// beams.
///
/// **Every number here comes off the agent's own surface** —
/// [`LookaheadAgent::last_roots`] and [`LookaheadAgent::last_stats`] — never
/// from a second evaluation this module runs. A re-derivation could rank a
/// different move first and there would be no fact of the matter about which
/// answer the contestant gave (`suggest`'s rule, spec §0.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootTable {
    /// The seat that was on the move, and whose view the decision was taken
    /// from.
    pub seat: PlayerId,
    /// What was asked for.
    pub contestant: Contestant,
    /// What it RESOLVES to — `Contestant::echo`, so a quoted table names its
    /// version and knobs instead of a bare name a reader would have to look up.
    pub echo: String,
    /// The seed the agent was built on. A fresh agent per decision (design §4's
    /// second discipline, W5's shape), so this and the root determine everything
    /// below.
    pub decision_seed: u64,
    /// The move the contestant PICKED — one of the maximisers of
    /// [`ranked`](Self::ranked), chosen by the keyed tie draw, so the ranking
    /// alone does not determine it.
    pub winner: Move,
    /// Every root candidate with its folded value, BEST FIRST (stable, so tied
    /// candidates keep the candidate scheme's own order — `suggest`'s and
    /// `bot::decide`'s rule, adopted verbatim so three readers of one decision
    /// cannot disagree about which tie is listed first).
    pub ranked: Vec<RootScore>,
    /// The decision's FULL counter set — all fifteen fields readable, which is
    /// the whole reason this type exists beside `why` (D7's step-one surface).
    pub stats: SearchStats,
    /// Which world it was decided in.
    pub world: SampledWorld,
}

impl RootTable {
    /// How many root candidates the contestant valued.
    pub fn candidates(&self) -> usize {
        self.ranked.len()
    }

    /// The best value on the table — `None` only for an empty ranking, which
    /// [`root_table`] refuses to produce.
    pub fn top_value(&self) -> Option<i64> {
        self.ranked.iter().map(|r| r.value).max()
    }

    /// Candidates tied AT the best value. 1 means the evaluator picked a
    /// strict argmax.
    pub fn tied_at_top(&self) -> usize {
        self.band_within(0)
    }

    /// Candidates valued within `points` of the best — the top set INCLUDED, so
    /// this is never smaller than [`tied_at_top`](Self::tied_at_top).
    ///
    /// Counted over the whole vector rather than as a prefix: the vector IS
    /// sorted, and a count that depends on that is a count that breaks silently
    /// if a caller ever hands this type an unsorted ranking.
    pub fn band_within(&self, points: i64) -> usize {
        // Saturating because a floor below `i64::MIN` is not a smaller floor,
        // it is a wrapped one — and a wrapped floor would count the whole table
        // as "outside the band" on a ranking of very negative values.
        let Some(floor) = self.top_value().map(|top| top.saturating_sub(points)) else {
            return 0;
        };
        self.ranked.iter().filter(|r| r.value >= floor).count()
    }

    /// [`band_within`](Self::band_within) at [`TOP_BAND_POINTS`] — the e113
    /// number.
    pub fn top_band(&self) -> usize {
        self.band_within(TOP_BAND_POINTS)
    }

    /// How many DISTINCT values the ranking holds. The degeneracy read from the
    /// other side: a 139-candidate table with 4 distinct values separated almost
    /// nothing however its band reads.
    pub fn distinct_values(&self) -> usize {
        let mut seen: Vec<i64> = self.ranked.iter().map(|r| r.value).collect();
        seen.sort_unstable();
        seen.dedup();
        seen.len()
    }

    /// Whether [`winner`](Self::winner) is one of the maximisers — a contract
    /// the agent owes and this type can check, since a winner outside the top
    /// set would mean the ranking and the pick came from different decisions.
    pub fn winner_is_a_maximiser(&self) -> bool {
        let Some(top) = self.top_value() else { return false };
        self.ranked.iter().any(|r| r.mv == self.winner && r.value == top)
    }
}

/// Why a contestant has no root table in v1.
///
/// Typed rather than a message, because each arm is a different KIND of answer:
/// two are permanent facts about what the contestant is, and one is a scope
/// statement this module can revisit with a consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedReason {
    /// A [`RolloutPolicy`] rung: it SAMPLES a move from a distribution and
    /// values nothing, so there is no ranking to report and never will be.
    /// `why`'s own posture at the same question.
    Sampled,
    /// **Greedy-v1: a one-ply argmax that reports no per-candidate vector.** It
    /// scores its candidates and keeps only the chosen successor's explanation —
    /// `GreedyAgent` has no `last_roots`, no `last_stats`, and no
    /// [`SearchStats`] to carry at all — which is exactly what the CLI's own
    /// greedy arm says by serving `ranking: Vec::new(), stats: None` and
    /// printing a one-ply account in place of both blocks (`bot::decide`,
    /// `why::ONE_PLY_ACCOUNT`). Mirroring that precedent HERE would produce a
    /// table with no candidates, no band and no counters; producing a real one
    /// would mean re-running the evaluator over the candidate set outside the
    /// agent, which is the second derivation this module exists not to have.
    /// The honest v1 answer is a refusal that names the missing surface.
    Unranked,
    /// **MCTS-v1 and the S2 atomic-diagnostic arm — a SCOPE refusal, and the
    /// two arms are refused for different reasons.** MCTS-v1 genuinely has no
    /// ranked-root surface: `MctsAgent` is a tree over visit counts and carries
    /// no `last_roots`/`last_stats` at all. The atomic-diag arm is the opposite
    /// case and the honest wording matters — it IS a [`LookaheadAgent`]
    /// (`Contestant::build` routes it through the same `build_search` as
    /// `lookahead-v1`, differing only in `SearchScheme::AtomicDiag`), so
    /// mirroring it here would be one more match arm and it would work. It is
    /// refused because it is a LABELLED DIAGNOSTIC that no v1 consumer asks
    /// about — the separator reads S1 beams — and a table nobody reads is a
    /// surface whose correctness nothing checks. Revisited with a consumer, not
    /// with a plumbing change (design §3).
    Diagnostic,
}

impl UnsupportedReason {
    /// The sentence a report prints.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sampled => "it samples its move from a distribution and values no candidate",
            Self::Unranked => {
                "a one-ply argmax reports no per-candidate vector and no search counters"
            }
            Self::Diagnostic => "a labelled diagnostic arm no v1 consumer asks a root table about",
        }
    }
}

/// Why a decision produced no table.
///
/// [`DrivenError`]'s posture at one decision: every variant is a hard stop that
/// produces nothing. A table computed on a corrupt root, at a seat that is not
/// on the move, or by a contestant that valued nothing would be a wrong answer
/// wearing the shape of a right one, and the analysis riding on it would have no
/// way to notice.
#[derive(Debug)]
pub enum RootTableError {
    /// The root violates an engine invariant — refused before the decision, so
    /// a corrupt fixture is never reported as a search result.
    CorruptRoot(String),
    /// The seat asked about is not the seat to move. The decision analyzed is
    /// the one the MOVER faces; valuing another seat's "candidates" at this
    /// position would enumerate the mover's moves and attribute them to someone
    /// else.
    NotOnTheMove { seat: PlayerId, mover: PlayerId },
    /// The seat cannot observe the root (a live-tracked seat whose own deal was
    /// never revealed).
    Unobservable { seat: PlayerId, detail: String },
    /// This contestant has no root table in v1.
    Unsupported { echo: String, reason: UnsupportedReason },
    /// The agent refused the decision — a sampler refusal, or a decision point
    /// that offered it nothing (which is what an ALREADY-TERMINAL root arrives
    /// as: no candidates, hence a dead end, hence no table).
    Agent { seat: PlayerId, detail: String },
    /// The agent returned a move without the accounting it owes — a broken
    /// contract rather than a state to report around.
    Unaccounted { echo: String, missing: &'static str },
}

impl std::fmt::Display for RootTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CorruptRoot(m) => write!(f, "corrupt root: {m}"),
            Self::NotOnTheMove { seat, mover } => write!(
                f,
                "seat {} is not on the move at this root (seat {} is)",
                seat.0, mover.0
            ),
            Self::Unobservable { seat, detail } => {
                write!(f, "seat {} cannot observe the root: {detail}", seat.0)
            }
            Self::Unsupported { echo, reason } => {
                write!(f, "{echo} has no root table: {}", reason.as_str())
            }
            Self::Agent { seat, detail } => {
                write!(f, "seat {} produced no decision: {detail}", seat.0)
            }
            Self::Unaccounted { echo, missing } => {
                write!(f, "{echo} decided without reporting {missing}")
            }
        }
    }
}

impl std::error::Error for RootTableError {}

/// Analyze ONE decision: what `contestant` would play for `seat` at `root`, with
/// every candidate it valued.
///
/// **Construction MIRRORS [`Contestant::build`]'s arms** (design §3, review
/// MUST-1) — same constructor, same knob order, same [`WeightTable::V2`], and no
/// weight parameter anywhere on this surface (L12: a weight change is a
/// versioned controller change, not a cell parameter). It cannot go through
/// `build` itself: that returns `Box<dyn ArenaAgent>`, whose `choose` answers
/// `DecisionReport { mv, volume }`, and the ranked values live on the CONCRETE
/// agent's `last_roots`/`last_stats` — while `ExplainHandle` carries counters
/// only. This is the CLI's own W5 pattern (`bot::decide` mirrors build for
/// exactly this reason) and it carries the same obligation: the mirror is
/// PINNED equal to `build` at the same view and seed, per supported arm, by
/// `the_mirrored_agent_picks_what_contestant_build_picks`.
///
/// **`decision_seed` is design §4's second discipline** — the explicit
/// single-decision seed, a fresh agent per decision (W5's shape), NOT the
/// arena's long-lived per-seat stream. A caller reproducing a recorded CLI
/// decision passes the seed that record implies; core does not re-implement the
/// CLI's `splitmix64(bot_seed ^ event_count)` derivation.
///
/// **Knob-varied contestants come through the existing grammar** —
/// `Contestant::from_str_name("lookahead-v1:b=8")` — so the separator's b=4 /
/// b=8 / b=16 runs are three parses and one function, with no beam parameter
/// added here.
pub fn root_table(
    root: &GameState,
    seat: PlayerId,
    contestant: Contestant,
    rules: &Rules,
    decision_seed: u64,
) -> Result<RootTable, RootTableError> {
    // The ARM first: an unsupported contestant is a fact about the question, not
    // about the position, so it is answered before the root is examined at all.
    let mut agent = mirror_build(contestant, rules, decision_seed)?;
    // Then the root, on `drive_to_terminal`'s posture: a table computed on a
    // corrupt state is a wrong answer wearing the shape of a right one.
    check_invariants(root, rules).map_err(RootTableError::CorruptRoot)?;
    if root.current_player != seat {
        return Err(RootTableError::NotOnTheMove { seat, mover: root.current_player });
    }

    let view = observe(root, seat)
        .map_err(|e| RootTableError::Unobservable { seat, detail: e.to_string() })?;
    // The agent's OWN honest chain, unchanged: it samples its world from the
    // view and the seed it drew, and nothing here hands it anything else.
    let report = agent
        .choose(&view, rules)
        .map_err(|detail| RootTableError::Agent { seat, detail })?;

    let echo = contestant.echo();
    let mut ranked = agent.last_roots().to_vec();
    if ranked.is_empty() {
        return Err(RootTableError::Unaccounted { echo, missing: "its root candidates" });
    }
    // Stable, so tied candidates keep the candidate scheme's own order —
    // `suggest`'s and `bot::decide`'s rule adopted verbatim, so three readers of
    // one decision cannot disagree about which tie is listed first.
    ranked.sort_by_key(|r| Reverse(r.value));
    let Some(stats) = agent.last_stats().copied() else {
        return Err(RootTableError::Unaccounted { echo, missing: "its search counters" });
    };
    let Some(sampler_seed) = agent.last_sampler_seed() else {
        return Err(RootTableError::Unaccounted { echo, missing: "its sampler seed" });
    };

    // The world identity. REBUILT from the seed the agent recorded rather than
    // read out of it (the agent keeps no world), which is the same world by
    // construction: `from_view` is deterministic in `(view, rules, seed)` and
    // the first `sample()` off a fresh sampler is the first world the search
    // ran in. A refusal here cannot happen after the agent's own sampler
    // accepted the same view at the same seed — and if it somehow did, it is an
    // accounting failure rather than an answer.
    let digest = BeliefSampler::from_view(&view, rules, sampler_seed)
        .map(|mut sampler| state_digest(&sampler.sample()))
        .map_err(|_| RootTableError::Unaccounted {
            echo: contestant.echo(),
            missing: "a reproducible sampled world",
        })?;

    Ok(RootTable {
        seat,
        contestant,
        echo,
        decision_seed,
        winner: report.mv,
        ranked,
        stats,
        world: SampledWorld { sampler_seed, worlds: agent.worlds(), digest },
    })
}

/// THE mirror of [`Contestant::build`]'s arms, and the ONE place a supported
/// contestant is decided.
///
/// Exhaustive with no `_` arm — `bot::eligible`'s discipline: a contestant added
/// to core lands here as a compile error to be ANSWERED, never as a default that
/// silently starts producing tables nobody checked.
fn mirror_build(
    contestant: Contestant,
    rules: &Rules,
    decision_seed: u64,
) -> Result<LookaheadAgent, RootTableError> {
    let refuse = |reason| {
        Err(RootTableError::Unsupported { echo: contestant.echo(), reason })
    };
    match contestant {
        // `Contestant::build`'s `build_search` arm, verbatim: same constructor,
        // same knob ORDER (worlds then retreat samples — adjacent `u8`s, so a
        // transposition compiles clean and mislabels everything downstream),
        // the same frozen `WeightTable::V2` named explicitly rather than taken
        // from a `::new` default, and the same `with_cap` builder afterwards.
        Contestant::LookaheadV1 { b, k, m, cap } => Ok(LookaheadAgent::with_knobs(
            rules,
            SearchScheme::TurnBoundary { beam: b },
            k,
            m,
            WeightTable::V2,
            decision_seed,
        )
        .with_cap(cap)),
        Contestant::Policy(_) => refuse(UnsupportedReason::Sampled),
        Contestant::GreedyV1 { .. } => refuse(UnsupportedReason::Unranked),
        Contestant::MctsV1 { .. } | Contestant::AtomicDiag { .. } => {
            refuse(UnsupportedReason::Diagnostic)
        }
    }
}

// ---------------------------------------------------------------------------
// The generated scan
// ---------------------------------------------------------------------------

/// Find the `ordinal`-th position satisfying `predicate` in a seeded scan at
/// `master`, under [`GeneratingEcology::v1`].
///
/// **The scan order IS the ordinal**, and it is stated here because every
/// recorded triple depends on it: games in `SeedSchedule::diagnostic(master)`
/// draw order, and within a game the positions in PLAY order — position `k`
/// being the state event `k` was decided in, `state_at`'s own index. Hits are
/// counted across games, so ordinal 0 is the first hit of game 0 and the
/// counting never restarts.
///
/// **Each game is walked ONE EVENT AT A TIME rather than by repeated
/// `state_at`**: `state_at(record, k)` is O(k) applies, so scanning a
/// thousand-event game with it is a million applies for the same information a
/// single fold gives once. The fold is `replay_from` over one event, which is
/// the same public path `state_at` itself runs — no second event-application
/// implementation appears here.
fn generate_root(
    predicate: RootPredicate,
    master: u64,
    ordinal: u32,
    rules: &Rules,
) -> Result<(GameState, RootDescriptor), RootError> {
    let ecology = GeneratingEcology::v1();
    let mut schedule = SeedSchedule::diagnostic(master);
    let mut found: u32 = 0;

    for game_index in 0..ecology.games {
        let world = schedule.next_world();
        // The ecology's own seating, on the arena's own keying — the same
        // derivation `drive_to_terminal` uses, so a generated root is reachable
        // by a run anyone can reproduce from the master alone.
        let mut make = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
            GENERATED_SEAT.build(rules, agent_stream_seed(world, seat, Slot(seat.0))).0
        };
        let stats = run_arena_game(world, GENERATED_PLAYERS, rules, &mut make, true)
            .map_err(|e| RootError::Generate { game_index, detail: e.to_string() })?;

        let events = &stats.record.events;
        let mut state = state_at(&stats.record, 0, rules)
            .map_err(|e| RootError::Generate { game_index, detail: e.to_string() })?;
        // INCLUSIVE of the past-the-last position, for totality: `state_at(record,
        // len)` is a real position (the terminal state) and a future predicate may
        // want it. The shipped one excludes it — its `!is_game_over` conjunct —
        // so that position is unreachable today rather than dead, and stopping the
        // range one short would make the scan quietly predicate-specific.
        for position in 0..=events.len() {
            if predicate.holds(&state) {
                if found == ordinal {
                    let root_digest = state_digest(&state);
                    return Ok((
                        state,
                        RootDescriptor {
                            origin: RootOrigin::Generated {
                                predicate,
                                master,
                                ordinal,
                                game_index,
                                ecology,
                            },
                            event_index: Some(position),
                            root_digest,
                        },
                    ));
                }
                found += 1;
            }
            if let Some(event) = events.get(position) {
                state = replay_from(&state, std::slice::from_ref(event), rules)
                    .map_err(|e| RootError::Generate { game_index, detail: e.to_string() })?;
            }
        }
    }

    Err(RootError::Exhausted { predicate, ordinal, found, games: ecology.games })
}

// ---------------------------------------------------------------------------
// The registry: named roots
// ---------------------------------------------------------------------------

/// The workspace root, resolved from THIS CRATE'S manifest directory rather
/// than from the working directory (review N-5).
///
/// A registry path resolved against the CWD would name a different file
/// depending on where a runner was launched from, so an absent-file miss would
/// fire (or not) for reasons that have nothing to do with the file. `env!` is a
/// compile-time constant, so this is the same anchor wherever the binary runs.
///
/// The search looks for the ancestor carrying BOTH a `Cargo.toml` and a
/// `crates/` directory — the workspace's own shape. If no ancestor has it (a
/// vendored copy with another layout), the manifest directory itself is
/// returned, which makes a registry miss name an obviously-wrong path rather
/// than silently resolving somewhere plausible.
pub fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .find(|dir| dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir())
        .unwrap_or(manifest)
        .to_path_buf()
}

/// The bot seat in BOTH session games (the CLI seats the human at 0).
pub const SESSION_BOT_SEAT: PlayerId = PlayerId(1);

/// One NAMED root the runner and an analysis session can ask for by name.
///
/// **Runner and analysis material, never a test dependency** (design §3): the
/// session entries point at gitignored saves, so the test suite must be green
/// with those files ABSENT — which is why loading is absent-tolerant with a
/// typed miss and why no pin here reads one.
///
/// The recorded fields are what make a separator run reproduce BY DEFAULT
/// instead of on hand-copied hex (review S-3): `decision_seed` is the seed the
/// CLI's own `splitmix64(bot_seed ^ event_count)` derivation produced for that
/// decision (derived in `yokohama-cli`, which core deliberately does not
/// re-implement — see [`root_table`]), and `recorded_move` is the move the bot
/// actually played there, so [`RegistryEntry::load`] can CHECK the file it
/// opened is the game the entry describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    /// The name a runner asks for.
    pub name: &'static str,
    /// WORKSPACE-ROOT-RELATIVE path to the record, resolved by
    /// [`workspace_root`].
    pub path: &'static str,
    /// `state_at`'s `k` and `why <n>`'s `n` — the position the mover decided in.
    pub event_index: usize,
    /// Which seat the bot played.
    pub bot_seat: PlayerId,
    /// The decision seed the recorded decision was taken at.
    pub decision_seed: u64,
    /// The move the record carries at [`event_index`](Self::event_index), where
    /// it is known.
    pub recorded_move: Option<Move>,
    /// What this root is FOR, in one line.
    pub note: &'static str,
}

impl RegistryEntry {
    /// This entry's record path, anchored at [`workspace_root`].
    pub fn resolved_path(&self) -> PathBuf {
        workspace_root().join(self.path)
    }

    /// Load this entry's record and name the root it points at.
    ///
    /// **An absent file is a typed MISS naming the path** — not a panic, not a
    /// skip, not an env var: the runner prints it and moves on, and the test
    /// suite never asks. Everything else this function refuses is a
    /// TRANSCRIPTION check on the entry itself: an event index past the log, or
    /// a recorded move the record does not carry at that index. Those cannot
    /// fire on a matching file and both fire loudly on the wrong one — an entry
    /// pointed at a re-saved or replaced game is exactly the silent failure a
    /// hand-transcribed literal invites.
    pub fn load(&self) -> Result<RootSource, RegistryError> {
        let path = self.resolved_path();
        // `read_to_string` would report the absence too, but as one io error
        // among many — and THE absence is the expected outcome here, so it gets
        // its own arm rather than a message a caller has to match on.
        if !path.is_file() {
            return Err(RegistryError::Missing { name: self.name, path });
        }
        let text = std::fs::read_to_string(&path).map_err(|e| RegistryError::Read {
            name: self.name,
            path: path.clone(),
            detail: e.to_string(),
        })?;
        let record: GameRecord = serde_json::from_str(&text).map_err(|e| RegistryError::Parse {
            name: self.name,
            path,
            detail: e.to_string(),
        })?;

        let events = record.events.len();
        if self.event_index >= events {
            return Err(RegistryError::EventOutOfRange {
                name: self.name,
                event_index: self.event_index,
                events,
            });
        }
        if let Some(expected) = &self.recorded_move {
            let found = &record.events[self.event_index];
            let matches = matches!(found, RecordedEvent::Move(mv) if mv == expected);
            if !matches {
                return Err(RegistryError::MoveMismatch {
                    name: self.name,
                    event_index: self.event_index,
                    expected: Box::new(expected.clone()),
                    found: format!("{found:?}"),
                });
            }
        }
        Ok(RootSource::RecordPrefix { record, event_index: self.event_index })
    }
}

/// Why a registry entry produced no root.
#[derive(Debug)]
pub enum RegistryError {
    /// The record file is not there. THE expected miss — see
    /// [`RegistryEntry::load`].
    Missing { name: &'static str, path: PathBuf },
    /// The file is there and could not be read.
    Read { name: &'static str, path: PathBuf, detail: String },
    /// The file is there and is not a `GameRecord`.
    Parse { name: &'static str, path: PathBuf, detail: String },
    /// The record is shorter than the entry's event index.
    EventOutOfRange { name: &'static str, event_index: usize, events: usize },
    /// The record carries something else at that index — the entry and the file
    /// describe different games.
    MoveMismatch { name: &'static str, event_index: usize, expected: Box<Move>, found: String },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { name, path } => write!(
                f,
                "{name}: no record at {} (session roots live in gitignored saves; this is a \
                 MISS, not a failure)",
                path.display()
            ),
            Self::Read { name, path, detail } => {
                write!(f, "{name}: {} could not be read: {detail}", path.display())
            }
            Self::Parse { name, path, detail } => {
                write!(f, "{name}: {} is not a GameRecord: {detail}", path.display())
            }
            Self::EventOutOfRange { name, event_index, events } => write!(
                f,
                "{name}: event {event_index} is past the log ({events} events) — the entry and \
                 the file describe different games"
            ),
            Self::MoveMismatch { name, event_index, expected, found } => write!(
                f,
                "{name}: event {event_index} is {found}, not the recorded {expected:?} — the \
                 entry and the file describe different games"
            ),
        }
    }
}

impl std::error::Error for RegistryError {}

/// The NAMED roots this session produced (design §3, plan §T5's list).
///
/// Six entries over the two owner diagnostic games, every one a
/// [`RootSource::RecordPrefix`] at the position the bot decided in. Their
/// content is FACTS ABOUT GITIGNORED MATERIAL — the seeds were derived by the
/// CLI's own rule from each save's recorded bot seed, and the moves were read
/// out of the saves' event logs — transcribed here because a runner cannot read
/// them from a file it may not have, and because a transcription the loader
/// CHECKS (see [`RegistryEntry::load`]) is better than a hand-copied hex string
/// on a command line.
///
/// **The root-index rule** (plan §T5, review S-4): each root is the turn's
/// FIRST BRANCHING decision. The `EndAdditionalPhase` events immediately before
/// the two final-turn roots were verified forced (one root candidate, no
/// runner-up) with the CLI's own `why`, so those two skip nothing.
pub fn registry() -> Vec<RegistryEntry> {
    const G1: &str = "docs/temp/owner-games/game1.json";
    const G3: &str = "docs/temp/owner-games/game3.json";
    vec![
        RegistryEntry {
            name: "g1-e113-tiemass",
            path: G1,
            event_index: 113,
            bot_seat: SESSION_BOT_SEAT,
            decision_seed: 0x15e8_0120_dfcb_f8c3,
            recorded_move: Some(Move::PlaceThreeAreas {
                placements: vec![AreaId(0), AreaId(1)],
            }),
            note: "the 36-way tie — the branch-0 headline root of the D7 triage",
        },
        RegistryEntry {
            name: "g3-e63-tiemass",
            path: G3,
            event_index: 63,
            bot_seat: SESSION_BOT_SEAT,
            decision_seed: 0x08d4_02cb_00f7_0b0e,
            recorded_move: Some(Move::PlaceThreeAreas {
                placements: vec![AreaId(1), AreaId(2), AreaId(3)],
            }),
            note: "a 26-way tie — the second tie-mass seed, in the other game",
        },
        RegistryEntry {
            name: "g3-e93-separated",
            path: G3,
            event_index: 93,
            bot_seat: SESSION_BOT_SEAT,
            decision_seed: 0x15eb_620d_98dc_105b,
            recorded_move: Some(Move::PlaceTwoSameArea { area: AreaId(2), adjacent_bonus: None }),
            note: "a +116 unique argmax — the POSITIVE CONTROL: the instrument must read a \
                   decision the evaluator genuinely separated as separated",
        },
        RegistryEntry {
            name: "g1-e228-deadrecall",
            path: G1,
            event_index: 228,
            bot_seat: SESSION_BOT_SEAT,
            decision_seed: 0x5453_844f_397d_538a,
            recorded_move: Some(Move::ReturnPresidentToHand {
                recover: BTreeMap::from([(AreaId(4), 1), (AreaId(6), 1)]),
            }),
            note: "the dead-recall 4-way tie — family 1's adjudication root",
        },
        RegistryEntry {
            name: "g3-e178-finalturn",
            path: G3,
            event_index: 178,
            bot_seat: SESSION_BOT_SEAT,
            decision_seed: 0x0192_0e62_30f9_dcaf,
            recorded_move: Some(Move::PlaceThreeAreas {
                placements: vec![AreaId(4), AreaId(6), AreaId(9)],
            }),
            note: "the bot's genuinely-final turn, first branching decision (e177's \
                   EndAdditionalPhase was verified forced)",
        },
        RegistryEntry {
            name: "g1-e239-finalturn",
            path: G1,
            event_index: 239,
            bot_seat: SESSION_BOT_SEAT,
            decision_seed: 0x18e9_4f6a_7620_fcab,
            recorded_move: Some(Move::PlaceThreeAreas { placements: vec![AreaId(0)] }),
            note: "game 1's true final turn, first branching decision (e238's \
                   EndAdditionalPhase was verified forced)",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::arena::{
        run_arena_game, ArenaGameStats, ControllerProvenance, DecisionReport,
    };
    use crate::game::end_game::is_game_over;
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
    ///
    /// The SECOND arm is the one that pins the guard's shape rather than its
    /// existence: `invariants: false` is about the per-move cost of a bulk run,
    /// and the ROOT check is unconditional — a corrupt starting state is not a
    /// cost, it is a wrong answer waiting. Nothing else in the repo exercises
    /// that flag's `false` setting on this driver, so a guard quietly moved
    /// inside the `if invariants` block would otherwise ship silently.
    #[test]
    fn a_corrupt_root_is_refused_before_the_first_decision() {
        let rules = Rules::standard();
        let mut root = root_2p(&rules, world(14).0);
        root.players[0].assistants.in_hand += 1;
        assert!(check_invariants(&root, &rules).is_err(), "the fixture must actually be corrupt");

        let seats = [
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
        ];
        let err = drive_to_terminal(&root, &seats, &rules, world(15), DrivenOpts::default())
            .expect_err("a corrupt root must refuse");
        match err {
            DrivenError::CorruptRoot(reason) => {
                assert!(reason.contains("assistants total"), "the reason is lost: {reason}");
            }
            other => panic!("expected CorruptRoot, got {other:?}"),
        }

        let unchecked = DrivenOpts { invariants: false, ..DrivenOpts::default() };
        let err = drive_to_terminal(&root, &seats, &rules, world(15), unchecked)
            .expect_err("the ROOT guard does not answer to the per-move flag");
        match err {
            DrivenError::CorruptRoot(reason) => {
                assert!(reason.contains("assistants total"), "the reason is lost: {reason}");
            }
            other => panic!("expected CorruptRoot with invariants off, got {other:?}"),
        }
    }

    /// BOTH player-count guards, on the two shapes that reach them — and they
    /// are two guards, not one property with two inputs.
    ///
    /// A rooted driver is handed states nobody generated, which is the whole
    /// point of the module, so "the engine plays 2-4" has to be enforced on the
    /// ROOT rather than inherited from a `new_game` call that never happened.
    /// The 5-seat root is refused by `require_supported_player_count`; the
    /// 258-seat root is refused BEFORE it, by the `u8::try_from` narrowing,
    /// because `258 as u8` is 2 and the supported-count check would then wave
    /// through a root it exists to refuse.
    ///
    /// Both fixtures are seat CLONES: the guards run before any invariant check
    /// or agent construction, so a state needs only to carry the seat count.
    #[test]
    fn a_root_with_an_unsupported_seat_count_is_refused_before_it_is_played() {
        let rules = Rules::standard();
        let base = root_2p(&rules, world(18).0);
        let filler = base.players[0].clone();

        let mut five = base.clone();
        five.players.resize(5, filler.clone());
        let five_seats = [SeatSpec::Reply(ReplyPolicy::FamilyWeighted); 5];
        let err = drive_to_terminal(&five, &five_seats, &rules, world(19), DrivenOpts::default())
            .expect_err("a 5-seat root must refuse");
        let rendered = err.to_string();
        assert!(
            matches!(err, DrivenError::UnsupportedRoot(_)),
            "expected UnsupportedRoot, got {err:?}",
        );
        assert!(rendered.contains("5 seats"), "the refusal must name the count: {rendered}");
        assert!(
            !rendered.contains("arena driver"),
            "a rooted refusal must not nest the arena driver's own prefix: {rendered}",
        );

        // The narrowing arm. 258 is chosen because it WRAPS to 2 — a supported
        // count — so a cast here would produce an outcome for a root the engine
        // cannot play, rather than an error.
        let mut many = base;
        many.players.resize(258, filler);
        let many_seats = vec![SeatSpec::Reply(ReplyPolicy::FamilyWeighted); 258];
        let err = drive_to_terminal(&many, &many_seats, &rules, world(19), DrivenOpts::default())
            .expect_err("a 258-seat root must refuse rather than narrow to 2");
        match err {
            DrivenError::UnsupportedRoot(m) => assert!(
                m.contains("258 seats"),
                "the refusal must name the count it SAW, not the one it wrapped to: {m}",
            ),
            other => panic!("expected UnsupportedRoot, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // The zero-decision contract, and the explanation fold
    // -----------------------------------------------------------------------

    /// An ALREADY-TERMINAL root is an `Ok` with nothing in it, never a refusal.
    ///
    /// T4's adjudicator forces a candidate and drives the REMAINDER, and a
    /// candidate that ends the game leaves a remainder of exactly nothing —
    /// refusing here would make "this move wins immediately" the one answer the
    /// instrument could not report.
    ///
    /// Seat 0 is an EVALUATOR deliberately: this is also the executable
    /// statement that `explanations` reads `Some(0)` — a seat that HAS an
    /// explanation path and was asked for no decisions — which is the reading
    /// the field's own doc used to deny.
    #[test]
    fn an_already_terminal_root_drives_to_an_empty_ok_run() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(20).0);
        let rungs = [
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
        ];
        let played = drive_to_terminal(&root, &rungs, &rules, world(21), DrivenOpts::default())
            .expect("the run that MAKES the fixture must complete");
        let terminal = played.final_state.clone();
        assert!(is_game_over(&terminal), "anti-vacuity: the root under test is not terminal");

        let seats = [
            SeatSpec::Contestant(Contestant::GREEDY_V1_DEFAULT),
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
        ];
        let outcome = drive_to_terminal(&terminal, &seats, &rules, world(22), DrivenOpts::default())
            .expect("a terminal root is an Ok, not a refusal");

        assert_eq!(outcome.decisions, 0, "a terminal root has nothing to decide");
        assert!(outcome.applied.is_empty());
        assert_eq!(outcome.moves_by_seat(), vec![0, 0]);
        assert_eq!(
            outcome.result,
            final_result(&terminal, &rules),
            "the run reported a result other than the root's own",
        );
        assert_eq!(outcome.result, played.result, "the same terminal state scored differently");
        assert_eq!(
            outcome.explanations,
            Some(0),
            "an explaining seat with zero decisions reads Some(0) — None would say NO seat explains",
        );
        assert_eq!(outcome.seats[0].counters.explained, Some(0));
        assert_eq!(outcome.seats[1].counters, SeatCounters::NONE, "a rung explains nothing");
        assert!(outcome.explanation_identity_holds());
    }

    /// TWO explaining seats: `explanations` is their SUM.
    ///
    /// The accumulation is a fold seeded from `None`, and a fold that dropped
    /// the accumulator would still produce a plausible `Some(n)` — one seat's
    /// count, on a run where both seats searched. Nothing else in the module
    /// has two explaining seats, so this is the only cell that can tell the two
    /// apart. Driven from a LATE root, and `greedy-v1` on both sides, so the
    /// search is paid for a tail rather than a whole game.
    #[test]
    fn two_explaining_seats_sum_into_the_explanation_count() {
        let rules = Rules::standard();
        let seed = world(23);
        let rung = Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT);
        let stats = arena_game(&rules, seed, rung, rung);
        let events = stats.record.events.len();
        let root = state_at(&stats.record, events.saturating_sub(30), &rules)
            .expect("a late root replays");
        assert!(!is_game_over(&root), "anti-vacuity: the chosen root is already terminal");

        let greedy = SeatSpec::Contestant(Contestant::GREEDY_V1_DEFAULT);
        let outcome =
            drive_to_terminal(&root, &[greedy, greedy], &rules, seed, DrivenOpts::default())
                .expect("the driven run must complete");

        let e0 = outcome.seats[0].counters.explained.expect("seat 0 explains");
        let e1 = outcome.seats[1].counters.explained.expect("seat 1 explains");
        assert!(
            e0 > 0 && e1 > 0,
            "anti-vacuity: a sum over one moving seat is not a sum (e0={e0}, e1={e1})",
        );
        assert_eq!(outcome.explanations, Some(e0 + e1), "the fold dropped a seat's count");
        assert_eq!(
            outcome.explanations,
            Some(outcome.decisions),
            "with every seat explaining, the sum IS the decision count",
        );
        assert!(outcome.explanation_identity_holds());
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

    // =======================================================================
    // T2: the root table
    // =======================================================================

    /// T2's committed diagnostic DECISION seed — design §4's second discipline
    /// (an explicit single-decision seed, a fresh agent per decision), not an
    /// arena stream. Diagnostic BY NAME, per `game::seeds`: a table pin needs a
    /// fixed decision, and a fixture cell is exactly the case spec §3's rule
    /// carves out.
    const DECISION_SEED: u64 = 0x0B02_2026_0817_9991;

    /// A cheap 2P game to take ORGANIC roots out of. BOTH seats are rungs, so
    /// the game itself pays for no search at all — the searching is what
    /// `root_table` does, once, at whichever root a cell picks (design §6's wall
    /// discipline).
    fn rung_game(rules: &Rules, seed: WorldSeed) -> ArenaGameStats {
        let rung = Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT);
        arena_game(rules, seed, rung, rung)
    }

    /// `contestant`'s table at the state event `k` was decided in, at
    /// [`DECISION_SEED`].
    fn table_at(
        rules: &Rules,
        stats: &ArenaGameStats,
        k: usize,
        contestant: Contestant,
    ) -> Result<RootTable, RootTableError> {
        let state = state_at(&stats.record, k, rules).expect("the prefix replays");
        let seat = state.current_player;
        root_table(&state, seat, contestant, rules, DECISION_SEED)
    }

    /// The first root in `from .. from + window` whose table satisfies `want`.
    ///
    /// Deterministic in `(record, from, window, DECISION_SEED)` and nothing
    /// else, so a cell that scans is as reproducible as one that names an index
    /// — and a scan that finds nothing PANICS rather than quietly asserting
    /// against whatever it last looked at.
    fn scan(
        rules: &Rules,
        stats: &ArenaGameStats,
        from: usize,
        window: usize,
        want: impl Fn(&RootTable) -> bool,
    ) -> (usize, RootTable) {
        let end = (from + window).min(stats.record.events.len());
        (from..end)
            .find_map(|k| {
                let table = table_at(rules, stats, k, Contestant::LOOKAHEAD_V1_DEFAULT).ok()?;
                want(&table).then_some((k, table))
            })
            .unwrap_or_else(|| panic!("no root in {from}..{end} satisfied the cell's predicate"))
    }

    /// A table with the given values and nothing else real — the ARITHMETIC
    /// fixture. Its moves are distinct but arbitrary (the band functions read
    /// values, never moves) and its winner is the FIRST entry.
    fn table_of(values: &[i64]) -> RootTable {
        RootTable {
            seat: PlayerId(0),
            contestant: Contestant::LOOKAHEAD_V1_DEFAULT,
            echo: Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
            decision_seed: DECISION_SEED,
            winner: Move::BuildShophouse { slot: 0 },
            ranked: values
                .iter()
                .enumerate()
                .map(|(i, v)| RootScore { mv: Move::BuildShophouse { slot: i as u8 }, value: *v })
                .collect(),
            stats: SearchStats::default(),
            world: SampledWorld {
                sampler_seed: 0,
                worlds: 1,
                digest: "0000000000000000".to_string(),
            },
        }
    }

    // -----------------------------------------------------------------------
    // The band arithmetic
    // -----------------------------------------------------------------------

    /// The tie/band/distinct fields are EXACT, on rankings whose answers are
    /// hand-computable.
    ///
    /// They are pure functions of the ranked vector, so this is the cell that
    /// says what they MEAN — and the one that can state the boundary, which no
    /// organic root can: `TOP_BAND_POINTS` is inclusive, so a candidate exactly
    /// one point below the best is INSIDE the band and one two points below is
    /// outside. An off-by-one here would silently re-scale every degeneracy
    /// figure the separator reports.
    #[test]
    fn the_band_arithmetic_is_exact_on_a_hand_computed_ranking() {
        let t = table_of(&[10, 10, 9, 9, 5]);
        assert_eq!(t.candidates(), 5);
        assert_eq!(t.top_value(), Some(10));
        assert_eq!(t.tied_at_top(), 2, "two candidates sit AT 10");
        assert_eq!(t.top_band(), 4, "10, 10, 9, 9 are within one point of the best; 5 is not");
        assert_eq!(t.band_within(0), 2, "a zero-wide band IS the tied set");
        assert_eq!(t.band_within(5), 5, "a five-point band swallows the whole table");
        assert_eq!(t.distinct_values(), 3, "10, 9 and 5");
        assert!(t.winner_is_a_maximiser());

        // The boundary, on negatives so a sign error cannot hide in it.
        let edge = table_of(&[0, -1, -2]);
        assert_eq!(edge.tied_at_top(), 1);
        assert_eq!(edge.top_band(), 2, "exactly one point below is INSIDE the band");
        assert_eq!(edge.distinct_values(), 3);

        // A strict argmax and a total tie — the two ends.
        let lone = table_of(&[7]);
        assert_eq!((lone.candidates(), lone.tied_at_top(), lone.top_band()), (1, 1, 1));
        assert_eq!(lone.distinct_values(), 1);
        let flat = table_of(&[3, 3, 3]);
        assert_eq!((flat.tied_at_top(), flat.top_band(), flat.distinct_values()), (3, 3, 1));

        // The winner check is a real check: a winner outside the top set means
        // the ranking and the pick came from different decisions.
        let mut liar = table_of(&[10, 4]);
        liar.winner = Move::BuildShophouse { slot: 1 };
        assert!(!liar.winner_is_a_maximiser(), "a winner valued 4 under a top of 10 is not one");
    }

    // -----------------------------------------------------------------------
    // A real root
    // -----------------------------------------------------------------------

    /// An organic root's table reports the AGENT'S OWN ranking, and its shape
    /// fields agree with it.
    ///
    /// Two cells rather than one, because "known ties" has two ends a real game
    /// supplies and a literal fixture cannot vouch for: a FORCED decision point,
    /// where every field is exactly 1 and the arithmetic is hand-computable
    /// against a real search, and a TIED one, where the evaluator genuinely
    /// separated less than the candidate count suggests.
    ///
    /// The load-bearing cross-check is `candidates() == stats.root_candidates`:
    /// the ranking and the counters come off two different accessors of the same
    /// decision, and a table that stitched a ranking to another decision's
    /// counters would pass every other assertion here.
    #[test]
    fn a_real_roots_table_reports_the_agents_own_ranking() {
        let rules = Rules::standard();
        let stats = rung_game(&rules, world(30));

        let (k, forced) = scan(&rules, &stats, 40, 40, |t| t.candidates() == 1);
        assert_eq!(forced.tied_at_top(), 1);
        assert_eq!(forced.top_band(), 1, "one candidate is one band");
        assert_eq!(forced.distinct_values(), 1);
        assert_eq!(forced.winner, forced.ranked[0].mv, "the only candidate is the winner");
        assert_eq!(
            forced.candidates() as u64,
            forced.stats.root_candidates,
            "event {k}: the ranking and the counters describe different decisions",
        );

        let (k, tied) = scan(&rules, &stats, 40, 60, |t| t.tied_at_top() >= 2);
        assert!(tied.candidates() >= tied.tied_at_top());
        assert!(
            tied.top_band() >= tied.tied_at_top(),
            "event {k}: the band must contain the tied set",
        );
        assert!(tied.distinct_values() <= tied.candidates());
        assert!(tied.winner_is_a_maximiser(), "event {k}: the pick is one of the maximisers");
        assert!(
            tied.ranked.windows(2).all(|w| w[0].value >= w[1].value),
            "event {k}: the ranking is not sorted best-first",
        );
        assert!(
            tied.ranked.iter().any(|r| r.mv == tied.winner),
            "event {k}: the winner is not on its own table",
        );
        assert_eq!(tied.candidates() as u64, tied.stats.root_candidates);
        assert_eq!(tied.seat, state_at(&stats.record, k, &rules).unwrap().current_player);
        assert_eq!(tied.echo, Contestant::LOOKAHEAD_V1_DEFAULT.echo());
        assert_eq!(tied.decision_seed, DECISION_SEED);
    }

    /// ALL FIFTEEN counters ride the table, and an UNCAPPED S1 arm reads the
    /// zero shape on every counter that has one.
    ///
    /// This is the reason the type exists beside `why`, which prints 2 of the 15
    /// — so the cell reads every field by name rather than spot-checking three:
    /// a field that stopped being populated would otherwise be invisible here
    /// exactly as it is in the CLI.
    #[test]
    fn the_full_counter_set_rides_the_table_and_an_uncapped_arm_reads_zeros() {
        let rules = Rules::standard();
        let stats = rung_game(&rules, world(31));
        let (k, table) = scan(&rules, &stats, 30, 40, |t| t.candidates() >= 3);
        let s = table.stats;

        // The uncapped shape: nothing was truncated and no budget existed to
        // bind. `evaluator_gate`'s own assertions, at one decision.
        assert_eq!(s.ply_cap_hits, 0, "event {k}: a line stopped at MAX_TURN_PLIES is valued mid-turn");
        assert_eq!(s.free_cap_hits, 0, "the turn-boundary scheme never enters the quiescence arm");
        assert_eq!(s.free_extension_steps, 0, "S1 takes no free extensions at all");
        assert_eq!(s.budget_binds, 0, "an UNCAPPED arm has no budget to bind");
        assert_eq!(s.budget_starved, 0);

        // The work counters, and the identities the type documents.
        assert!(s.root_candidates > 0, "a search that offered nothing did not search");
        assert_eq!(s.root_candidates, table.candidates() as u64);
        assert_eq!(s.root_scored, s.root_candidates, "each root candidate is scored exactly once");
        assert!(s.scored >= s.root_scored, "the root share cannot exceed the total");
        assert_eq!(s.nodes, s.handles, "one DecisionHandle per expandable node, by construction");
        assert!(s.nodes > 0 && s.leaves > 0, "event {k}: {s:?}");
        assert!(s.max_line >= 1, "a completed continuation is at least the root move");
        // Read, never asserted: shape observations whose values are properties
        // of the position rather than of the code under test.
        let _ = (s.route_dfs_nodes, s.forced_nodes, s.placement_cap_binds);
    }

    // -----------------------------------------------------------------------
    // Determinism, and the sampled world
    // -----------------------------------------------------------------------

    /// Same root, same seat, same contestant, same decision seed → the same
    /// table, twice — and a DIFFERENT seed reaches a different world, which is
    /// the anti-vacuity half (a table that ignored its seed would satisfy the
    /// first assertion no matter what it did).
    ///
    /// **The last arm is `worlds`' own anti-vacuity** (review probe J): every
    /// other cell in this module runs the shipped `K = 1` knobs, where a
    /// `worlds` field HARDCODED to 1 at the construction site reads exactly like
    /// one read off the agent. The only assertion that can tell those apart is
    /// one at a contestant whose `K` is not 1, and it costs a single `K = 2`
    /// decision.
    #[test]
    fn the_same_root_and_seed_produce_the_same_table_twice() {
        let rules = Rules::standard();
        let stats = rung_game(&rules, world(32));
        let (k, first) = scan(&rules, &stats, 30, 40, |t| t.candidates() >= 3);
        let state = state_at(&stats.record, k, &rules).expect("the prefix replays");
        let seat = state.current_player;

        let second =
            root_table(&state, seat, Contestant::LOOKAHEAD_V1_DEFAULT, &rules, DECISION_SEED)
                .expect("the table must reproduce");
        assert_eq!(first, second, "two identical asks produced different tables");

        let elsewhere = root_table(
            &state,
            seat,
            Contestant::LOOKAHEAD_V1_DEFAULT,
            &rules,
            DECISION_SEED ^ 0x5555_5555_5555_5555,
        )
        .expect("the table must reproduce at another seed");
        assert_ne!(
            first.world.digest, elsewhere.world.digest,
            "the seed does not reach the sampled world, so the world identity witnesses nothing",
        );
        assert_ne!(first.world.sampler_seed, elsewhere.world.sampler_seed);
        assert_eq!(first.world.worlds, 1, "the shipped knobs sample ONE world");

        // The digest's RECIPE, pinned: it is the world the SAMPLER SEED reaches,
        // and the sampler seed is a word the agent DREW rather than the decision
        // seed it was built on. A digest taken from the decision seed instead
        // would name a world the search never entered while passing every
        // assertion above — same determinism, same seed-sensitivity, wrong
        // world.
        let view = observe(&state, seat).expect("the mover observes its own root");
        let mut sampler = BeliefSampler::from_view(&view, &rules, first.world.sampler_seed)
            .expect("the recorded seed rebuilds the sampler the agent used");
        assert_eq!(
            first.world.digest,
            state_digest(&sampler.sample()),
            "the world digest is not the world its own sampler seed reaches",
        );
        assert_ne!(
            first.world.sampler_seed, first.decision_seed,
            "the sampler seed is a DRAW from the stream, not the seed the stream started at",
        );

        // `worlds` is READ OFF THE AGENT (`LookaheadAgent::worlds`), which only a
        // contestant whose K is not the shipped 1 can witness. The knob comes
        // through the existing grammar, so this arm adds no surface — one K=2
        // decision, at the root the cell already found.
        let two_worlds = Contestant::from_str_name("lookahead-v1:k=2").expect("k=2 parses");
        let doubled = root_table(&state, seat, two_worlds, &rules, DECISION_SEED)
            .expect("a K=2 arm is the same supported S1 arm at another knob");
        assert_eq!(
            doubled.world.worlds, 2,
            "worlds is not the agent's own K: a K=2 arm reported {}",
            doubled.world.worlds,
        );
        assert!(doubled.echo.contains("k=2"), "the K=2 arm must echo its knob: {}", doubled.echo);
        // And the world IDENTITY is still world 0's at either K: the agent draws
        // its sampler seed BEFORE the K loop and samples the loop off that one
        // sampler, so K moves how many worlds were searched and not which world
        // the digest names.
        assert_eq!(
            doubled.world.sampler_seed, first.world.sampler_seed,
            "K moved the sampler seed, so two K's tables are not comparable at one root",
        );
        assert_eq!(
            doubled.world.digest, first.world.digest,
            "the digest names world 0, and world 0 is the same world at either K",
        );
    }

    /// **The separator's premise, recorded rather than inferred** (design §3,
    /// review N-1): at one root and one decision seed, the THREE beams the
    /// separator runs — b=4, b=8, b=16 — search the SAME sampled world, because
    /// the agent draws its three stream words before any beam-dependent work.
    /// The beam is therefore the only thing that differs between their tables,
    /// which is the claim the separator's whole reading rests on.
    ///
    /// The second half is the ANTI-VACUITY the mechanical separator needs: an
    /// instrument that reported the same band structure at every beam could not
    /// separate anything, so a root at which the tables DIFFER must exist and be
    /// findable. No claim is made about WHICH way it moves — that is the
    /// separator's finding to make, not this cell's.
    #[test]
    fn a_wider_beam_searches_the_same_world_and_can_move_the_band() {
        let rules = Rules::standard();
        let stats = rung_game(&rules, world(33));
        let wide = Contestant::from_str_name("lookahead-v1:b=8").expect("the grammar parses b=8");
        let widest = Contestant::from_str_name("lookahead-v1:b=16").expect("b=16 parses");

        let mut moved: Option<(usize, RootTable, RootTable)> = None;
        for k in 30..90.min(stats.record.events.len()) {
            let Ok(narrow) = table_at(&rules, &stats, k, Contestant::LOOKAHEAD_V1_DEFAULT) else {
                continue;
            };
            let Ok(broad) = table_at(&rules, &stats, k, wide) else { continue };
            let Ok(broadest) = table_at(&rules, &stats, k, widest) else { continue };
            // The premise holds at EVERY root, differing bands or not.
            assert_eq!(
                narrow.world, broad.world,
                "event {k}: two beams sampled different worlds, so their tables are not comparable",
            );
            assert_eq!(narrow.world, broadest.world, "event {k}: b=16 left the shared world");
            let shape = |t: &RootTable| (t.tied_at_top(), t.top_band(), t.distinct_values());
            if shape(&narrow) != shape(&broad) || narrow.ranked != broad.ranked {
                moved = Some((k, narrow, broad));
                break;
            }
        }

        let (k, narrow, broad) = moved.expect(
            "no root in the scanned window read differently at b=4 and b=8 — the instrument \
             cannot see a beam difference at all",
        );
        assert_eq!(narrow.candidates(), broad.candidates(), "event {k}: the ROOT family is the beam's");
        assert_eq!(narrow.echo, Contestant::LOOKAHEAD_V1_DEFAULT.echo());
        assert!(broad.echo.contains("b=8"), "the wider arm must echo its own beam: {}", broad.echo);
    }

    // -----------------------------------------------------------------------
    // The mirror-vs-build drift pin
    // -----------------------------------------------------------------------

    /// **THE DRIFT PIN** (design §3, review MUST-1), in the two halves one root
    /// cannot supply on its own. This module constructs the concrete agent
    /// itself, because `Contestant::build`'s `Box<dyn ArenaAgent>` erases the
    /// `last_roots`/`last_stats` surface the ranked values live on — so the
    /// mirror has to be held equal to what it mirrors, or it would go on
    /// producing plausible tables about a contestant nobody runs.
    ///
    /// **(a) CONSTRUCTION identity**, through the one surface a boxed agent does
    /// expose: `ArenaAgent::controller()`. It carries the WHOLE construction —
    /// name and version from the scheme, `b`/`k`/`m`/`cap`, `eval_impl`, the
    /// WEIGHT TABLE identity, and the stream seed — so a mirror that drifted in
    /// any of them fails here even at a position where the drift happens not to
    /// move the pick.
    ///
    /// **(b) BEHAVIOURAL identity** over a WINDOW of roots rather than at one.
    /// Measured while writing this: a `WeightTable::V1` mirror against a V2
    /// build disagrees at only 3 of 40 consecutive roots here, so a single-root
    /// cell passes a weight drift roughly 9 times in 10 — which is exactly the
    /// drift half (a) alone would also catch, and precisely why the behavioural
    /// half is not left at one root. The default arm sweeps the full window; the
    /// two knob-varied arms sweep a shorter one, since their construction drift
    /// is what (a) is for and a searching decision is not free in debug.
    ///
    /// **The knob-arm window is 12, widened from 8 at the T2 fix wave (review
    /// N-1), and the reason is the SLACK.** At 8 both knob arms read branching
    /// == floor == 2 — the anti-vacuity check passed with nothing to spare, so a
    /// single root turning forced would have failed the cell for a reason that
    /// is a property of the game rather than of the code under test. At 12 the
    /// floor is 3 and both read 4 (roots 34/35/38/39 of this record), i.e. slack
    /// 1, and four more decisions are compared. The default arm is unchanged at
    /// 40 (branching 15, floor 10, slack 5). The floor FORMULA is untouched:
    /// widening the window raises the bar with the sample, which is the point.
    #[test]
    fn the_mirrored_agent_picks_what_contestant_build_picks() {
        let rules = Rules::standard();
        let stats = rung_game(&rules, world(34));
        let arms = [
            (Contestant::LOOKAHEAD_V1_DEFAULT, 40),
            (Contestant::from_str_name("lookahead-v1:b=8").expect("b=8 parses"), 12),
            (
                Contestant::from_str_name("lookahead-v1:b=2,cap=750").expect("a capped arm parses"),
                12,
            ),
        ];

        for (arm, window) in arms {
            // (a) the construction, compared through the trait object.
            let (built, _) = arm.build(&rules, DECISION_SEED);
            let mirrored = mirror_build(arm, &rules, DECISION_SEED)
                .unwrap_or_else(|e| panic!("{} is a supported arm: {e}", arm.echo()));
            assert_eq!(
                built.controller(),
                mirrored.controller(),
                "MIRROR DRIFT: {} is CONSTRUCTED differently here than by Contestant::build",
                arm.echo(),
            );

            // (b) the decision, over a window.
            let mut compared = 0;
            let mut branching = 0;
            for k in 30..30 + window {
                let Ok(state) = state_at(&stats.record, k, &rules) else { continue };
                let seat = state.current_player;
                let Ok(view) = observe(&state, seat) else { continue };
                let Ok(table) = root_table(&state, seat, arm, &rules, DECISION_SEED) else {
                    continue;
                };
                let (mut built, _) = arm.build(&rules, DECISION_SEED);
                let report = built
                    .choose(&view, &rules)
                    .unwrap_or_else(|e| panic!("{} through build at event {k}: {e}", arm.echo()));
                assert_eq!(
                    table.winner,
                    report.mv,
                    "MIRROR DRIFT at event {k}: {} decided differently through Contestant::build",
                    arm.echo(),
                );
                compared += 1;
                if table.candidates() >= 3 {
                    branching += 1;
                }
            }
            assert_eq!(compared, window, "{}: the window did not compare cleanly", arm.echo());
            assert!(
                branching * 4 >= window,
                "anti-vacuity: only {branching} of {compared} roots offered a real choice ({}) — \
                 the floor is {} and this window read at authoring 15/40 (slack 5) on the default \
                 arm and 4/12 (slack 1) on each knob arm",
                arm.echo(),
                window / 4,
            );
        }
    }

    // -----------------------------------------------------------------------
    // The refusals
    // -----------------------------------------------------------------------

    /// Every contestant with no ranked-root surface is refused by its OWN
    /// reason, and refused BEFORE the root is touched.
    ///
    /// The second half is what the corrupt root proves: a `Policy` rung asked
    /// about a broken state comes back `Unsupported`, not `CorruptRoot`, so the
    /// arm check genuinely precedes every other question rather than happening
    /// to be reached first on a healthy state.
    #[test]
    fn every_contestant_without_a_ranked_root_surface_is_refused_by_its_own_reason() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(35).0);
        let seat = root.current_player;

        let expected = [
            (Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT), UnsupportedReason::Sampled),
            (Contestant::GREEDY_V1_DEFAULT, UnsupportedReason::Unranked),
            (Contestant::MCTS_V1_DEFAULT, UnsupportedReason::Diagnostic),
            (Contestant::ATOMIC_DIAG_DEFAULT, UnsupportedReason::Diagnostic),
        ];
        for (contestant, reason) in expected {
            let err = root_table(&root, seat, contestant, &rules, DECISION_SEED)
                .expect_err("v1 supports the S1 turn-boundary arm and nothing else");
            match err {
                RootTableError::Unsupported { echo, reason: got } => {
                    assert_eq!(got, reason, "{echo} was refused for the wrong reason");
                    assert_eq!(echo, contestant.echo(), "the refusal must name what it refused");
                    assert!(
                        err_text(&RootTableError::Unsupported { echo, reason: got })
                            .contains(reason.as_str()),
                        "the rendered refusal drops its reason",
                    );
                }
                other => panic!("expected Unsupported for {}, got {other:?}", contestant.echo()),
            }
        }

        // The arm check precedes the root check.
        let mut corrupt = root;
        corrupt.players[0].assistants.in_hand += 1;
        assert!(check_invariants(&corrupt, &rules).is_err(), "the fixture must be corrupt");
        assert!(matches!(
            root_table(
                &corrupt,
                seat,
                Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
                &rules,
                DECISION_SEED,
            ),
            Err(RootTableError::Unsupported { .. }),
        ));
        assert!(matches!(
            root_table(&corrupt, seat, Contestant::LOOKAHEAD_V1_DEFAULT, &rules, DECISION_SEED),
            Err(RootTableError::CorruptRoot(_)),
        ));
    }

    /// `Display`, as a function, so a cell can assert a refusal SAYS what it is.
    fn err_text(e: &RootTableError) -> String {
        e.to_string()
    }

    /// The two remaining guards: a seat that is not on the move, and a root the
    /// seat cannot observe.
    ///
    /// Both are wrong ANSWERS rather than errors if they are let through — a
    /// table for a seat that is not to move would enumerate the mover's moves
    /// and attribute them to someone else, which is a report no reader could
    /// tell from a real one.
    #[test]
    fn a_table_is_refused_for_a_seat_that_is_not_the_mover_or_cannot_observe() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(36).0);
        let mover = root.current_player;
        let other = PlayerId(1 - mover.0);

        match root_table(&root, other, Contestant::LOOKAHEAD_V1_DEFAULT, &rules, DECISION_SEED) {
            Err(RootTableError::NotOnTheMove { seat, mover: m }) => {
                assert_eq!((seat, m), (other, mover));
            }
            other => panic!("expected NotOnTheMove, got {other:?}"),
        }

        // The live-tracking shape, invariant-CLEAN so the refusal is about
        // observability rather than corruption (T1's own fixture).
        let mut hidden = root;
        let seat = mover.0 as usize;
        let kept = hidden.players[seat].starting_order.take().expect("a dealt starting order");
        hidden.players[seat].order_cards_in_hand.retain(|c| *c != kept);
        hidden.orders_removed.push(kept);
        hidden.players[seat].starting_order_hidden = true;
        check_invariants(&hidden, &rules).expect("the live-tracking root is invariant-clean");
        match root_table(&hidden, mover, Contestant::LOOKAHEAD_V1_DEFAULT, &rules, DECISION_SEED) {
            Err(RootTableError::Unobservable { seat, .. }) => assert_eq!(seat, mover),
            other => panic!("expected Unobservable, got {other:?}"),
        }
    }

    // =======================================================================
    // T3: roots, provenance, and the registry
    // =======================================================================

    /// T3's committed diagnostic master for the GENERATED scan — diagnostic BY
    /// NAME per `game::seeds`, the same waiver [`MASTER`] carries: a scan pin
    /// needs a fixed sequence of games, and a fixture cell is exactly the case
    /// spec §3's rule carves out.
    const GEN_MASTER: u64 = 0x0B03_2026_0817_9992;

    /// One REPLY seat and one named contestant that EXPLAINS — the seat list the
    /// provenance cells use, because the two arms of `SeatSpec` must both appear
    /// in a `seats` vector for the descriptor's `requested`/`echo` pair to be
    /// worth anything.
    fn mixed_seats() -> [SeatSpec; 2] {
        [
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
            SeatSpec::Contestant(Contestant::GREEDY_V1_DEFAULT),
        ]
    }

    /// Every seat descriptor says what the driver actually seated.
    ///
    /// Shared by the constructed and record-prefix cells: the seat half of
    /// provenance is the same claim whatever the root's origin, and stating it
    /// twice by hand would let the two drift.
    fn assert_seats_describe(seats: &[SeatSpec], p: &RootedProvenance, master: WorldSeed) {
        assert_eq!(p.seats.len(), seats.len(), "a seat played that provenance does not name");
        for (index, spec) in seats.iter().enumerate() {
            let d = &p.seats[index];
            assert_eq!(d.seat, index as u8, "the descriptors are not in seat order");
            assert_eq!(d.requested, spec.requested_label());
            assert_eq!(d.echo, spec.contestant().echo(), "seat {index} echoes another contestant");
            assert_eq!(
                d.observation,
                crate::game::record::OBSERVATION_VIEW,
                "seat {index} played from something other than a view",
            );
            assert_eq!(
                d.agent_seed,
                agent_stream_seed(master, PlayerId(index as u8), Slot(index as u8)),
                "seat {index}'s recorded stream is not the one it played on",
            );
            // The RECONSTRUCTION property: the two record halves this descriptor
            // carries are enough to rebuild the contestant, knobs and all — which
            // is what makes the descriptor an identity rather than a label.
            assert_eq!(
                Contestant::from_parts(&d.controller, &d.params),
                Ok(spec.contestant()),
                "seat {index}'s controller+params do not reconstruct what played",
            );
            // And the AGENT's own descriptor is the CONTESTANT's: these come from
            // two different places (the built agent reported the first, the seat
            // spec claims the second), so a contestant whose `build` ever routed
            // to a differently-versioned or differently-knobbed agent would show
            // up here rather than in a report nobody cross-checks.
            let (name, version, params) = spec.contestant().descriptor();
            assert_eq!(
                (d.controller.as_str(), d.version, &d.params),
                (name.as_str(), version, &params),
                "seat {index}'s agent describes itself differently than its contestant does",
            );
        }
    }

    /// A bare state handed to [`drive_to_terminal`] is stamped for what it is —
    /// CONSTRUCTED, with its own digest and nothing claimed beyond it.
    ///
    /// The second half is the equivalence that keeps the two entry points from
    /// describing one root two ways: `RootSource::Constructed(state).resolve()`
    /// must produce the descriptor the driver stamps by itself.
    #[test]
    fn a_constructed_root_stamps_its_own_identity_on_every_outcome() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(40).0);
        let seats = [
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
        ];
        let master = world(41);

        let outcome = drive_to_terminal(&root, &seats, &rules, master, DrivenOpts::default())
            .expect("the run must complete");
        let p = &outcome.provenance;

        assert_eq!(p.root.origin, RootOrigin::Constructed);
        assert_eq!(p.root.origin.kind(), "constructed");
        assert_eq!(p.root.event_index, None, "a constructed root has no log to index into");
        assert_eq!(p.root.root_digest, state_digest(&root), "the stamped root is another state");
        assert_eq!(p.semantics_rev, SEMANTICS_REV);
        assert_eq!(p.rules_hash, rules.content_hash());
        assert_eq!(p.master, master.0);
        assert_seats_describe(&seats, p, master);

        // The one-line digest NAMES the root since T3.
        let line = outcome.digest();
        assert!(line.contains("root=constructed@"), "the digest does not name the origin: {line}");
        assert!(line.contains(&p.root.root_digest), "the digest does not name the root: {line}");
        assert!(line.contains(&format!("rev={SEMANTICS_REV}")), "the digest hides the rev: {line}");

        // The entry points agree about what this root is.
        let (resolved, descriptor) = RootSource::Constructed(root.clone())
            .resolve(&rules)
            .expect("a constructed source resolves to itself");
        assert_eq!(resolved, root, "resolving a constructed source moved the state");
        assert_eq!(descriptor, p.root, "the driver and the source describe one root differently");
    }

    /// A RECORD-PREFIX root: `state_at`'s position, driven to terminal, with the
    /// document it came from named on the outcome.
    ///
    /// The event index is `why <n>`'s own number, which is the point of the
    /// variant — a root taken from a session's `why` output needs no arithmetic
    /// — so the cell pins the resolved state EQUAL to `state_at`'s rather than
    /// merely plausible.
    ///
    /// It does NOT pin the source game's tail (design §4, review N-2): fresh
    /// agents start their streams at position 0 where the recorded game's were
    /// mid-stream, so a continuation is a different line and never claims to be
    /// the same one. What is pinned is that it TERMINATES and says what it was.
    #[test]
    fn a_record_prefix_root_drives_to_terminal_with_its_provenance_stamped() {
        let rules = Rules::standard();
        let stats = rung_game(&rules, world(42));
        let events = stats.record.events.len();
        let k = events.saturating_sub(30);
        let source = RootSource::RecordPrefix { record: stats.record.clone(), event_index: k };

        let (root, descriptor) = source.resolve(&rules).expect("the prefix replays");
        assert_eq!(
            root,
            state_at(&stats.record, k, &rules).expect("the prefix replays"),
            "the resolved root is not the state event {k} was decided in",
        );
        assert!(!is_game_over(&root), "anti-vacuity: the chosen root is already terminal");
        assert_eq!(descriptor.event_index, Some(k));
        assert_eq!(descriptor.root_digest, state_digest(&root));
        assert_eq!(
            descriptor.origin,
            RootOrigin::RecordPrefix { record_digest: record_digest(&stats.record) },
            "the descriptor names another document",
        );

        let seats = mixed_seats();
        let master = world(43);
        let outcome = drive_rooted(&source, &seats, &rules, master, DrivenOpts::default())
            .expect("the rooted run must complete");

        assert!(is_game_over(&outcome.final_state), "a driven run must reach a terminal state");
        assert!(outcome.decisions > 0, "anti-vacuity: the run applied nothing");
        assert_eq!(outcome.decisions, outcome.applied.len() as u64);
        let p = &outcome.provenance;
        assert_eq!(p.root, descriptor, "the run stamped a different root than it resolved");
        assert_eq!(p.semantics_rev, SEMANTICS_REV);
        assert_eq!(p.rules_hash, rules.content_hash());
        assert_eq!(p.master, master.0);
        assert_seats_describe(&seats, p, master);
        // The explaining seat is the named contestant, and the reply rung
        // explains nothing — the `None`-is-not-zero rule, on a mixed vocabulary.
        assert_eq!(outcome.seats[1].counters.explained, Some(outcome.seats[1].applied_moves));
        assert_eq!(outcome.seats[0].counters, SeatCounters::NONE);
        assert!(outcome.explanation_identity_holds());

        // Real provenance survives the trip to a sidecar and back.
        let json = serde_json::to_string(p).expect("provenance serializes");
        let back: RootedProvenance = serde_json::from_str(&json).expect("provenance parses");
        assert_eq!(&back, p, "a real run's provenance did not survive serde");
    }

    /// An out-of-range prefix index is a typed refusal naming the bound — never
    /// a clamp to the terminal state, `state_at`'s own rule carried up.
    ///
    /// Costs no game: a zero-event record still has a position 0 (the post-setup
    /// state), which is exactly what makes the boundary statable.
    #[test]
    fn a_prefix_index_past_the_event_log_is_a_typed_refusal() {
        let rules = Rules::standard();
        let record = GameRecord::new(
            world(44).0,
            GameConfig::for_player_count(2),
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            &rules,
        );

        let (state, descriptor) = RootSource::RecordPrefix { record: record.clone(), event_index: 0 }
            .resolve(&rules)
            .expect("position 0 of any record is its post-setup state");
        assert_eq!(descriptor.event_index, Some(0));
        assert_eq!(descriptor.root_digest, state_digest(&state));

        let err = RootSource::RecordPrefix { record, event_index: 1 }
            .resolve(&rules)
            .expect_err("a record with no events has no position 1");
        match err {
            RootError::Prefix { event_index, events, .. } => {
                assert_eq!((event_index, events), (1, 0));
                let rendered = RootError::Prefix {
                    event_index,
                    events,
                    detail: "x".to_string(),
                }
                .to_string();
                assert!(rendered.contains('1'), "the refusal hides what was asked: {rendered}");
                assert!(rendered.contains('0'), "the refusal hides the bound: {rendered}");
            }
            other => panic!("expected Prefix, got {other:?}"),
        }
    }

    /// The provenance shape survives serde at EVERY origin, and a reader gets
    /// the fields a sidecar exists for.
    ///
    /// Hand-built rather than driven, deliberately: the `Generated` origin
    /// carries the whole ecology, which no cheap run produces, and an enum with
    /// data is exactly where a serde rename or a missing `derive` bites.
    #[test]
    fn provenance_round_trips_through_serde_at_every_origin() {
        let ecology = GeneratingEcology::v1();
        let origins = [
            RootOrigin::Constructed,
            RootOrigin::RecordPrefix { record_digest: "0123456789abcdef".to_string() },
            RootOrigin::Generated {
                predicate: RootPredicate::FinalRoundAlreadyTriggered,
                master: GEN_MASTER,
                ordinal: 3,
                game_index: 1,
                ecology: ecology.clone(),
            },
        ];

        for origin in origins {
            let kind = origin.kind();
            let p = RootedProvenance {
                root: RootDescriptor {
                    origin,
                    event_index: Some(113),
                    root_digest: "fedcba9876543210".to_string(),
                },
                semantics_rev: SEMANTICS_REV,
                rules_hash: "rules-hash".to_string(),
                seats: vec![SeatDescriptor {
                    seat: 1,
                    requested: "reply:lookahead-at-final".to_string(),
                    echo: Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
                    controller: "lookahead-v1".to_string(),
                    version: 2,
                    params: BTreeMap::from([("b".to_string(), 4)]),
                    observation: crate::game::record::OBSERVATION_VIEW.to_string(),
                    agent_seed: 0x1234_5678_9abc_def0,
                }],
                master: GEN_MASTER,
            };

            let json = serde_json::to_string(&p).expect("provenance serializes");
            let back: RootedProvenance =
                serde_json::from_str(&json).expect("provenance parses back");
            assert_eq!(back, p, "the {kind} origin did not survive serde");

            // The fields a sidecar READER needs are actually in the document.
            assert!(json.contains("fedcba9876543210"), "{kind}: the root digest is not written");
            assert!(json.contains(&Contestant::LOOKAHEAD_V1_DEFAULT.echo()), "{kind}: no echo");
            assert!(json.contains(crate::game::record::OBSERVATION_VIEW), "{kind}: no label");
        }

        // The ecology is v1's, spelled once.
        assert_eq!(ecology.id, GENERATED_ECOLOGY_V1);
        assert_eq!(ecology.players, 2);
        assert_eq!(ecology.games, GENERATED_SCAN_GAMES);
        assert_eq!(ecology.seats, vec![GENERATED_SEAT.echo(); 2]);
        assert!(!ecology.driver.is_empty() && !ecology.seed_derivation.is_empty());
    }

    /// A LIVE-TRACKED root is refused UP FRONT, by name, whichever seat carries
    /// the hidden deal (design §5, review N-3).
    ///
    /// The fixture doctors the seat that is NOT on the move, which is the case
    /// the driver could otherwise only discover mid-run — and `max_decisions: 1`
    /// is the discriminator that makes "up front" executable rather than
    /// asserted: the same root undoctored reaches the decision cap, so a refusal
    /// that arrived only when the hidden seat's turn came would show up here as
    /// `DecisionCapReached` instead.
    #[test]
    fn a_root_whose_seats_carry_hidden_deals_is_refused_up_front_by_name() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(45).0);
        let mover = root.current_player;
        let other = PlayerId(1 - mover.0);
        let seats = [
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
        ];
        let one_decision = DrivenOpts { max_decisions: 1, ..DrivenOpts::default() };

        // The control: undoctored, this root plays and hits the cap.
        assert!(matches!(
            drive_to_terminal(&root, &seats, &rules, world(46), one_decision),
            Err(DrivenError::DecisionCapReached { .. }),
        ));

        // The live-tracking shape on the NON-MOVER, invariant-clean so the
        // refusal is about eligibility and not corruption.
        let mut hidden = root;
        let index = other.0 as usize;
        let kept = hidden.players[index].starting_order.take().expect("a dealt starting order");
        hidden.players[index].order_cards_in_hand.retain(|c| *c != kept);
        hidden.orders_removed.push(kept);
        hidden.players[index].starting_order_hidden = true;
        check_invariants(&hidden, &rules).expect("the live-tracking root is invariant-clean");

        let err = drive_to_terminal(&hidden, &seats, &rules, world(46), one_decision)
            .expect_err("a live-tracked root must refuse");
        match err {
            DrivenError::Unobservable { seat, ref detail } => {
                assert_eq!(seat, other, "the refusal named the wrong seat");
                assert!(
                    detail.contains("starting_order_hidden"),
                    "the refusal does not name the field it read: {detail}",
                );
            }
            ref other => panic!("expected Unobservable, got {other:?}"),
        }
        assert!(err.to_string().contains(&format!("seat {}", other.0)));
    }

    /// The GENERATED arm: a seeded scan finds the ordinal-th final-round root,
    /// and finds the SAME one twice.
    ///
    /// Three anti-vacuity halves, because a scan that ignored any of its three
    /// inputs would still reproduce: the ORDINAL must move the root, the MASTER
    /// must move it, and the predicate must actually HOLD at what comes back
    /// (a scan returning position 0 of game 0 would be perfectly deterministic
    /// and perfectly useless).
    #[test]
    fn the_generated_arm_finds_the_same_final_round_root_twice() {
        let rules = Rules::standard();
        let predicate = RootPredicate::FinalRoundAlreadyTriggered;
        let source = RootSource::Generated { predicate, master: GEN_MASTER, ordinal: 0 };

        let (first, descriptor) = source.resolve(&rules).expect("the scan must find a root");
        let (second, again) = source.resolve(&rules).expect("the scan must reproduce");
        assert_eq!(descriptor, again, "two identical scans described different roots");
        assert_eq!(state_digest(&first), state_digest(&second), "the scan is not deterministic");
        assert_eq!(first, second);

        // What came back is what was asked for.
        assert!(predicate.holds(&first), "the scan returned a position the predicate rejects");
        assert!(first.end_game_triggered && first.final_round, "not a final-round position");
        assert!(!is_game_over(&first), "a terminal state is not a root anyone can decide at");
        assert_eq!(descriptor.root_digest, state_digest(&first));
        assert!(descriptor.event_index.is_some(), "a scanned root sits at a position");
        assert!(check_invariants(&first, &rules).is_ok(), "the scan produced a corrupt root");

        match &descriptor.origin {
            RootOrigin::Generated { predicate: p, master, ordinal, ecology, .. } => {
                assert_eq!(*p, predicate);
                assert_eq!(*master, GEN_MASTER);
                assert_eq!(*ordinal, 0);
                assert_eq!(ecology, &GeneratingEcology::v1(), "the ecology is not v1's");
            }
            other => panic!("expected a Generated origin, got {other:?}"),
        }

        // The ordinal is READ.
        let (next, _) = RootSource::Generated { predicate, master: GEN_MASTER, ordinal: 1 }
            .resolve(&rules)
            .expect("the scan supplies more than one root");
        assert_ne!(
            state_digest(&next),
            state_digest(&first),
            "ordinal 1 returned ordinal 0's root, so the ordinal selects nothing",
        );
        assert!(predicate.holds(&next));

        // So is the master.
        let (elsewhere, _) = RootSource::Generated {
            predicate,
            master: GEN_MASTER ^ 0x5555_5555_5555_5555,
            ordinal: 0,
        }
        .resolve(&rules)
        .expect("another master supplies its own roots");
        assert_ne!(
            state_digest(&elsewhere),
            state_digest(&first),
            "two masters produced one root, so the scan does not depend on its master",
        );
    }

    /// An ordinal beyond the scan's supply is a typed error naming how far it
    /// looked — never a hang, and never a nearest-available root quietly served
    /// in place of the one that was asked for.
    #[test]
    fn an_ordinal_beyond_the_generated_supply_is_a_typed_error() {
        let rules = Rules::standard();
        let predicate = RootPredicate::FinalRoundAlreadyTriggered;
        let err = RootSource::Generated { predicate, master: GEN_MASTER, ordinal: 1_000_000 }
            .resolve(&rules)
            .expect_err("no scan of four games holds a millionth final-round root");

        match err {
            RootError::Exhausted { predicate: p, ordinal, found, games } => {
                assert_eq!(p, predicate);
                assert_eq!(ordinal, 1_000_000);
                assert_eq!(games, GENERATED_SCAN_GAMES, "the refusal misnames the bound");
                assert!(
                    found > 0,
                    "anti-vacuity: the scan found NO final-round position in {games} games, so \
                     this refusal says nothing about the ordinal",
                );
                let rendered =
                    RootError::Exhausted { predicate: p, ordinal, found, games }.to_string();
                assert!(rendered.contains(predicate.as_str()), "no predicate: {rendered}");
                assert!(rendered.contains(&found.to_string()), "no found count: {rendered}");
                assert!(rendered.contains(&games.to_string()), "no bound: {rendered}");
            }
            other => panic!("expected Exhausted, got {other:?}"),
        }
    }

    /// The registry names this session's six roots — asserted WITHOUT opening a
    /// single file, which is the constraint that makes the suite corpus-free.
    ///
    /// The names are pinned literally because a runner and a session note both
    /// address roots by them: a renamed entry is a change to an interface, not
    /// an implementation detail.
    #[test]
    fn the_registry_names_the_session_roots_without_touching_a_file() {
        let entries = registry();
        let names: Vec<&str> = entries.iter().map(|e| e.name).collect();
        assert_eq!(
            names,
            vec![
                "g1-e113-tiemass",
                "g3-e63-tiemass",
                "g3-e93-separated",
                "g1-e228-deadrecall",
                "g3-e178-finalturn",
                "g1-e239-finalturn",
            ],
        );
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), entries.len(), "two entries share a name");

        for entry in &entries {
            assert!(
                entry.path.starts_with("docs/temp/owner-games/"),
                "{}: the path is not workspace-root-relative session material: {}",
                entry.name,
                entry.path,
            );
            assert!(entry.event_index > 0, "{}: event 0 is the post-setup state", entry.name);
            assert_eq!(entry.bot_seat, SESSION_BOT_SEAT, "{}: the bot sat elsewhere", entry.name);
            assert_ne!(entry.decision_seed, 0, "{}: no recorded decision seed", entry.name);
            assert!(entry.recorded_move.is_some(), "{}: no recorded move", entry.name);
            assert!(!entry.note.is_empty(), "{}: no note", entry.name);

            // The path is anchored at the WORKSPACE, not at the working
            // directory — so a runner launched from anywhere names one file.
            let resolved = entry.resolved_path();
            assert!(resolved.is_absolute(), "{}: {resolved:?} is relative", entry.name);
            assert!(resolved.ends_with(entry.path), "{}: {resolved:?}", entry.name);
        }

        // The anchor is the workspace itself (this file always exists in a
        // source checkout; no gitignored material is touched).
        let root = workspace_root();
        assert!(root.join("Cargo.toml").is_file(), "the anchor is not a workspace: {root:?}");
        assert!(root.join("crates").is_dir(), "the anchor holds no crates dir: {root:?}");
    }

    /// An absent record is a TYPED MISS naming the path — the runner prints it
    /// and moves on.
    ///
    /// Pointed at a path that cannot exist rather than at a real entry, so the
    /// cell asserts the same thing whether or not this checkout carries the
    /// gitignored saves. A pin that depended on their absence would be as
    /// corpus-coupled as one that depended on their presence.
    #[test]
    fn an_absent_registry_file_is_a_typed_miss_naming_the_path() {
        let mut entry = registry().into_iter().next().expect("the registry is not empty");
        entry.path = "docs/temp/owner-games/__no-such-record__.json";
        assert!(!entry.resolved_path().exists(), "the fixture path must not exist");

        let err = entry.load().expect_err("an absent record must be a miss, not a root");
        match err {
            RegistryError::Missing { name, ref path } => {
                assert_eq!(name, entry.name);
                assert_eq!(path, &entry.resolved_path());
            }
            ref other => panic!("expected Missing, got {other:?}"),
        }
        let rendered = err.to_string();
        assert!(rendered.contains("__no-such-record__.json"), "the miss hides its path: {rendered}");
        assert!(rendered.contains(entry.name), "the miss hides which entry: {rendered}");
    }
}
