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
//! **What T4 adds**: [`adjudicate_root`], family 1's question made executable —
//! force EVERY root candidate in turn, play the remainder out under the named
//! seats, K seeded replications, and report each candidate's final margin for
//! the ROOT MOVER. It is a **ROBUSTNESS READ and never exact ground truth**
//! ([`ROBUSTNESS_READ`] is the sentence a report prints): the numbers say what
//! the named policies reach from here, so a table whose candidates all read
//! alike says the position is dead AGAINST THAT REPLY. Replication worlds derive
//! through [`replication_world`] — design §4's `mix` under a named salt, with
//! the arena's per-seat keying re-entered at the derived world, so no third seed
//! discipline is invented.
//!
//! **What PHASE C's T1 adds** (design
//! `docs/plans/2026-08-18-phase-c-families-design.md` §1–§2): the layer-2
//! GENERATOR vocabulary. [`RootPredicate`] grows five variants — one nonterminal
//! control for family 1 and one per layer-2 family — and its `holds` takes
//! `&Rules`, because those arms probe the LEGAL LIST rather than reading flags.
//! Their shared conjuncts ship beside them as named, separately-pinned helpers:
//! [`true_margin_now`] (full-state truth, a SELECTION-side helper that must
//! never reach the evaluator), [`firing_moves`] (short-circuited on a
//! precondition set DERIVED from an exhaustive match over `EndGameTrigger`, so a
//! sixth trigger kind is a compile error rather than a silent scan bias), and
//! the [`MAX_ROOT_CANDIDATES`] width bound, which is a conjunct of the
//! predicates and therefore a recorded generator constraint rather than a
//! post-hoc filter. [`GeneratingEcology`] gains the per-game STRIDE CAP and a
//! second configuration, [`GeneratingEcology::v2_lookahead`], under which the
//! layer-2 predicates scan the subject's OWN reachable positions.
//!
//! **What PHASE C's T2 adds**: the TURN GRAIN. [`drive_turn`] plays ONE mover's
//! turn from a root — the grain between [`root_table`] (one decision) and
//! [`drive_to_terminal`] (a whole game), and the one families 2–4 read, because
//! their pass rules are statements about what a TURN did and the move that fires
//! a trigger is rarely the move that starts the turn. It stops on a DUAL
//! condition (hand-off OR game over) because the engine's end-of-turn arm and
//! its game-end arm write different fields, it runs ONE agent at the caller's
//! `decision_seed` so the stream advances across the turn, and it reports the
//! mover's WHOLE-TURN [`SeatCounters`] so a mid-turn cap hit reaches D7's
//! step-one filter. It is a third per-decision loop — [`play_from_state`] runs
//! to terminal and exposes no stop hook — and what keeps it from drifting is
//! stated where it is enforced: a prefix-equivalence pin against the play half,
//! and a failure taxonomy routed through [`classify`] over the arena's own
//! `PlayError` rather than re-typed.
//!
//! **What PHASE C's T3 adds**: the reply menu's fourth and ADVERSARIAL slot,
//! [`ReplyPolicy::RaceRusher`] — a rule over state, not a contestant, so
//! [`ReplyPolicy::contestant`] becomes partial ([`NotAContestant`]) and
//! [`SeatSpec::build`] grows a SECOND ARM that returns the boxed
//! [`RaceRusherAgent`] directly. It is still ONE materialization site: what that
//! ever bought was that no caller decides what a seat is, and the three drivers
//! still all ask here. The agent probes only the small families
//! ([`is_probed_family`]), classifies by apply-probe on its OWN sampled probe
//! world, and DELEGATES everything else to an inner greedy@final it owns for the
//! seat's lifetime — with the probe world derived from `(agent_seed, decision
//! ordinal)` under [`SALT_DRIVEN_RACE_PROBE`] rather than drawn from that
//! greedy's stream, which is what makes the no-race fallback byte-identical to a
//! bare greedy seat BY CONSTRUCTION rather than by measurement.
//!
//! **What this module still deliberately does NOT ship**: the runner (T5). It is
//! not stubbed here — a stub is a surface a reader has to check the emptiness
//! of.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::actions::{
    calculate_church_faith, customs_max_imported_goods, eligible_church_spaces,
    eligible_customs_spaces_for_cost, exported_reward, has_church_or_customs_assistant,
};
use crate::game::agent::HONEST_MAX_TURNS;
use crate::game::apply_move::apply_move_unchecked;
use crate::game::arena::{
    agent_stream_seed, mix, play_from_state, require_supported_player_count, run_arena_game,
    AgentBench, ArenaAgent, Contestant, ControllerProvenance, DecisionReport, ExplainHandle,
    PlayError, PlayLimits, SeatBench, Slot, SALT_DRIVEN_REPLICATION,
};
use crate::game::belief::BeliefSampler;
use crate::game::can_fulfill_order;
use crate::game::end_game::is_game_over;
use crate::game::eval::{realizable_power, WeightTable};
use crate::game::greedy::{GreedyAgent, DEFAULT_RETREAT_SAMPLES};
use crate::game::invariants::check_invariants;
use crate::game::legal_moves::{calculate_placement_cost, get_legal_moves};
use crate::game::lookahead::{LookaheadAgent, RootScore, SearchScheme, SearchStats};
use crate::game::metrics::decision_volume;
use crate::game::move_legality::{is_move_legal, PathMode};
use crate::game::observe::observe;
use crate::game::policy::RolloutPolicy;
use crate::game::power::calculate_power;
use crate::game::record::{replay_from, state_at, GameRecord, RecordedEvent, SeatProvenance};
use crate::game::rules::Rules;
use crate::game::scoring::{final_result, GameResult};
use crate::game::search_actions::{DecisionHandle, MoveFamily};
use crate::game::seeds::{SeedSchedule, WorldSeed};
use crate::game::semantics::{record_digest, state_digest, SEMANTICS_REV};
use crate::types::{
    AreaId, AreaType, ChurchTrack, CustomsTrack, EndGameTrigger, GameState, Move, OrderCardId,
    PendingChoice, PlayerId, PlayerView, PresidentLocation, TurnPhase,
};

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
/// **The fourth menu slot — [`RaceRusher`](Self::RaceRusher), one adversarial
/// policy declared as a RULE OVER STATE — landed at Phase C's T3** (design
/// `docs/plans/2026-08-18-phase-c-families-design.md` §2). It was deliberately
/// absent in v1 for a reason that its declaration had to answer rather than
/// outgrow: shipping a placeholder would have invited a hand-picked adversary
/// chosen after seeing the roots it is applied to, which is the shape of a
/// fixture that proves whatever its author wanted. What it ships as instead is a
/// STATED RULE over state — the rule is in this file, in full, and it was
/// written before any family root was drawn.
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
    /// **The declared adversarial arm** (Phase C, design §2): a positional
    /// OPPORTUNIST that takes a race step when one is in front of it and plays
    /// [`GreedyAtFinal`](Self::GreedyAtFinal) otherwise. See
    /// [`RaceRusherAgent`] for the rule in full — it is a HARNESS INSTRUMENT and
    /// has no [`Contestant`], which is why [`contestant`](Self::contestant) is
    /// partial. Its FALLBACK is provisional in exactly the sense the two
    /// `*AtFinal` arms are, and for the same reason: it IS
    /// [`GreedyAtFinal`](Self::GreedyAtFinal)'s binding
    /// ([`RACE_RUSHER_FALLBACK`]), so D1's identity freeze moves the two
    /// together or the pin fails.
    RaceRusher,
}

/// Why a [`ReplyPolicy`] arm names no [`Contestant`].
///
/// Typed rather than a bare `Option`, on [`UnsupportedReason`]'s grounds: the
/// absence is a FACT ABOUT THE ARM that a report should be able to print, and a
/// `None` says only that a lookup failed. It is also what keeps
/// [`SeatSpec::build`]'s two arms exhaustive — a second harness rule lands here
/// as a compile error to be answered rather than as a `_` that silently
/// materializes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotAContestant {
    /// [`ReplyPolicy::RaceRusher`] is a RULE OVER STATE built out of the shipped
    /// pieces (a belief sample, the factored family surface, an apply-probe, and
    /// a greedy fallback) — never an entrant. It must never become a
    /// [`Contestant`]: the grammar's names are things a CELL may pit against
    /// each other and a gate may quote, and this one exists only to be a hostile
    /// REPLY inside an adjudication.
    RaceRusher,
}

impl NotAContestant {
    /// The sentence a report prints.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RaceRusher => {
                "a harness reply rule over state, deliberately not a contestant"
            }
        }
    }
}

impl ReplyPolicy {
    /// The contestant this arm names. THE single spelling of the menu — PARTIAL
    /// since Phase C's T3, because [`RaceRusher`](Self::RaceRusher) names none.
    pub fn contestant(self) -> Result<Contestant, NotAContestant> {
        match self {
            Self::FamilyWeighted => Ok(Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT)),
            Self::GreedyAtFinal => Ok(Contestant::GREEDY_V1_DEFAULT),
            Self::LookaheadAtFinal => Ok(Contestant::LOOKAHEAD_V1_DEFAULT),
            Self::RaceRusher => Err(NotAContestant::RaceRusher),
        }
    }

    /// The stable name, for reports and error text.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FamilyWeighted => "family-weighted",
            Self::GreedyAtFinal => "greedy-at-final",
            Self::LookaheadAtFinal => "lookahead-at-final",
            Self::RaceRusher => "race-rusher",
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
    /// The contestant this seat plays, or the typed reason it names none.
    ///
    /// Sugar-equivalence still holds BY CONSTRUCTION for every arm that HAS a
    /// contestant: `Reply(r)` and `Contestant(r.contestant()?)` are one seat
    /// written two ways, because this is the only place the mapping lives.
    pub fn contestant(self) -> Result<Contestant, NotAContestant> {
        match self {
            Self::Contestant(c) => Ok(c),
            Self::Reply(r) => r.contestant(),
        }
    }

    /// What actually plays, as provenance/report text.
    ///
    /// [`Contestant::echo`] for every seat that names one, and
    /// [`race_rusher_echo`]'s composed sentence for the one that does not — so
    /// a descriptor's `echo` field is always a resolved identity and never a
    /// menu name (the rule [`requested_label`](Self::requested_label) states).
    pub fn echo(self) -> String {
        match self.contestant() {
            Ok(c) => c.echo(),
            Err(NotAContestant::RaceRusher) => race_rusher_echo(),
        }
    }

    /// Build the agent this seat plays, plus its explanation read-out if it has
    /// one — **THE single materialization site, now with TWO ARMS**.
    ///
    /// **The single-site argument survives the second arm, and restating it is
    /// the point** (design §2's SHOULD-7). What made one site valuable was never
    /// that it contained one `match` — it is that NO CALLER decides what a seat
    /// is: [`drive_to_terminal`], [`drive_turn`] and [`adjudicate_root`] all
    /// hand a [`SeatSpec`] here and receive an agent, so a seat cannot be
    /// materialized one way in a whole-game run and another way in a turn. The
    /// two arms are the two KINDS of thing a seat can be — an entrant, or a
    /// harness rule — and they are exhaustive over [`NotAContestant`], so a
    /// third kind is a compile error here rather than a silent omission at one
    /// of the three drivers.
    ///
    /// **The race-rusher arm hands back NO [`ExplainHandle`], deliberately.** It
    /// explains the decisions it DELEGATES (its inner greedy does) and not the
    /// ones it takes itself, so a handle here would report an `explained` count
    /// a reader would compare against the seat's applied moves and read as a
    /// defect. `None` is the closest true statement at this granularity — a seat
    /// that carries no explanation account — and it is the same posture a rung
    /// gets for the same kind of reason (see [`SeatCounters::NONE`]).
    pub fn build(
        self,
        rules: &Rules,
        agent_seed: u64,
    ) -> (Box<dyn ArenaAgent>, Option<ExplainHandle>) {
        match self.contestant() {
            // ARM 1 — an entrant: the arena's own constructor, untouched.
            Ok(contestant) => contestant.build(rules, agent_seed),
            // ARM 2 — a harness rule: built here, because there is no
            // `Contestant` to route it through and there must never be one.
            Err(NotAContestant::RaceRusher) => {
                (Box::new(RaceRusherAgent::new(rules, agent_seed)), None)
            }
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
// The RACE-RUSHER reply (Phase C, T3)
// ---------------------------------------------------------------------------

/// Domain salt for the RACE-RUSHER's PROBE WORLD.
///
/// **The derivation, stated where the constant is**:
/// `race_probe_seed(agent_seed, decision_ordinal) = mix(SALT_DRIVEN_RACE_PROBE,
/// [agent_seed, decision_ordinal])`, where `agent_seed` is the seat's own play
/// stream (from `agent_stream_seed`, exactly as every other seat's) and
/// `decision_ordinal` counts this seat's `choose` calls from 0.
///
/// **Why it is a DERIVATION and not a draw** — the property the whole T3
/// fallback claim rests on: a probe that drew its seed from the inner greedy's
/// `ChaCha8Rng` would advance that stream, and the no-race fallback would then
/// differ from a bare greedy seat at the same seed by however many probes
/// happened to run. Deriving the probe world from `(agent_seed, ordinal)`
/// instead means the classification consumes ZERO randomness from the inner
/// agent, so "a no-race decision is byte-identical to greedy@final" holds BY
/// CONSTRUCTION rather than by a test noticing. It is still a function of the
/// seat's own stream seed, so two seats never probe the same worlds, and it is
/// injective in the ordinal, so one seat's successive decisions do not either.
///
/// **Serial provenance (the A3E1_D21E family)**: this family's HOME is
/// `arena::SALT_DRIVEN_REPLICATION` (serial 0006), whose own doc records 0004
/// and 0005 as taken elsewhere. 0007 is the next free one. It is declared HERE
/// rather than there because Phase C's T3 moves `driven.rs` alone and this
/// constant has exactly one consumer in this file — the cross-pointer above is
/// what keeps the family's serial ledger readable from either end, and
/// `the_race_probe_salt_is_its_own_serial` pins that the two do not collide.
const SALT_DRIVEN_RACE_PROBE: u64 = 0xA3E1_D21E_0000_0007;

/// The probe world's sampler seed for one decision — see
/// [`SALT_DRIVEN_RACE_PROBE`] for the derivation and what it buys.
fn race_probe_seed(agent_seed: u64, decision_ordinal: u64) -> u64 {
    mix(SALT_DRIVEN_RACE_PROBE, &[agent_seed, decision_ordinal])
}

/// The contestant [`ReplyPolicy::RaceRusher`] DELEGATES to when no candidate
/// races — greedy@final, i.e. [`ReplyPolicy::GreedyAtFinal`]'s own binding.
///
/// Spelled from the same const that arm is spelled from
/// ([`Contestant::GREEDY_V1_DEFAULT`]'s definition), and PINNED equal to both
/// that const and to `GreedyAtFinal.contestant()`, so D1's identity freeze
/// re-points the fallback with the menu arm it is named after rather than
/// leaving this instrument on a stale binding.
///
/// **PROVISIONAL until D1, exactly as the other `*AtFinal` bindings are** — the
/// echo a report quotes carries the RESOLVED fallback (`fallback=<greedy
/// echo>`), never the word "greedy@final", for that reason.
const RACE_RUSHER_FALLBACK: Contestant = Contestant::GreedyV1 { m: DEFAULT_RETREAT_SAMPLES };

/// The race-rusher's stable controller name, in the repo's `name-vN` shape
/// (`greedy-v1`'s convention: the DESIGN version rides the name, the
/// IMPLEMENTATION version rides [`RACE_RUSHER_IMPL_VERSION`]).
pub const RACE_RUSHER_CONTROLLER: &str = "race-rusher-v1";

/// Implementation version under [`RACE_RUSHER_CONTROLLER`]. Bump it in the same
/// commit as ANY change to the decision rule, the probe scope, or the probe
/// world's derivation — the same obligation `GREEDY_IMPL_VERSION` carries.
pub const RACE_RUSHER_IMPL_VERSION: u32 = 1;

/// `reply:race-rusher-v1{fallback=<the resolved greedy echo>}` — what a
/// race-rusher seat ECHOES.
///
/// The `reply:` prefix is deliberate and is not a mistake about what an echo
/// field holds: every other echo names a contestant, and this one must say out
/// loud that it names none. The fallback is INTERPOLATED rather than described,
/// so a report carries the greedy identity (version, knobs, weight-table digest)
/// that the seat actually delegated to.
fn race_rusher_echo() -> String {
    format!("reply:{RACE_RUSHER_CONTROLLER}{{fallback={}}}", RACE_RUSHER_FALLBACK.echo())
}

/// What a probed candidate RACES, highest priority FIRST.
///
/// **Declaration order IS the priority order** and the derived `Ord` is what
/// enforces it: `Claim < Church < Customs`, so the best class is the MINIMUM.
/// Writing the priority as a type rather than as an integer means the ordering
/// cannot drift from the rule the design states (design §2: claims, then the
/// church race, then the customs race).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RaceClass {
    /// The move IS a claim (`Move::FulfillAchievement`) — the one class read
    /// off the move's SHAPE rather than off an apply-probe, because a claim is
    /// not a state change this classifier would have to look for: it is the
    /// move.
    Claim,
    /// The probed clone advances the mover on the CHURCH track — occupancy
    /// grew, or a church space-placement choice is queued for the mover.
    Church,
    /// The same on the CUSTOMS track.
    Customs,
}

/// Whether the race-rusher PROBES this family (design §2's probe scope, with the
/// real family-tag names).
///
/// Exhaustive with no `_` arm: a new [`MoveFamily`] must be ANSWERED here rather
/// than silently join (or silently miss) the probe set.
///
/// The three excluded movement/placement families are excluded because the rule
/// is a POSITIONAL one — the race-rusher races from where it stands, never by
/// walking somewhere first — and because those are precisely the families whose
/// size makes an apply-probe per candidate a six-figure cost (the retreat
/// product alone reaches 442,368). [`MoveFamily::FivePower`] and
/// [`MoveFamily::Building`] are excluded BY DECISION rather than by cost:
/// neither advances a track and neither claims, so probing them could only ever
/// return `None`.
fn is_probed_family(family: &MoveFamily) -> bool {
    match family {
        MoveFamily::AdditionalAction | MoveFamily::AreaAction | MoveFamily::ChoiceResolution => {
            true
        }
        MoveFamily::Placement | MoveFamily::MovementRoutes | MoveFamily::MovementRetreat { .. } => {
            false
        }
        MoveFamily::FivePower | MoveFamily::Building => false,
    }
}

/// Classify ONE candidate by apply-probe on `world`.
///
/// **QUEUE-AWARE** through [`mover_reachable_steps`], which is the single
/// authority for "did this move advance the mover on a track" and is shared
/// verbatim with [`RootPredicate::ExclusiveRaceOneStep`]'s mover half: a Church
/// or Customs action with two or more eligible spaces places NOTHING at apply
/// time (it queues a `PendingChoice::ChurchSpace`/`CustomsSpace`), so an
/// occupancy-only classifier would be blind to exactly the race-INITIATING moves
/// this policy exists to model. Sharing the helper rather than restating it is
/// what keeps the predicate and the reply from disagreeing about what a race
/// step is.
///
/// **Class 1 is read off the MOVE and costs no probe**, deliberately: a claim is
/// not a state change this classifier has to go looking for, it is the move
/// itself — and checking it first is what keeps the common case (a claim in the
/// additional-action list) from paying for a clone.
///
/// A move that reaches BOTH tracks classifies CHURCH, on the declared priority.
/// It is not a hypothetical: a track action taken while a track-space choice is
/// already queued reaches steps on both, and
/// `the_race_rule_takes_claim_then_church_then_customs` asserts exactly that
/// shape so the tie-break is exercised rather than merely written down.
fn race_class(world: &GameState, mv: &Move, rules: &Rules) -> Option<RaceClass> {
    if matches!(mv, Move::FulfillAchievement { .. }) {
        return Some(RaceClass::Claim);
    }
    let steps = mover_reachable_steps(world, std::slice::from_ref(mv), rules);
    if steps.iter().any(|s| matches!(s, RaceStep::Church(_))) {
        return Some(RaceClass::Church);
    }
    if steps.iter().any(|s| matches!(s, RaceStep::Customs(_))) {
        return Some(RaceClass::Customs);
    }
    None
}

/// The first candidate of the HIGHEST nonempty class, in the order `candidates`
/// are given.
///
/// Deterministic and stated: the caller supplies the probed families' moves in
/// the engine's own legal-list order (see [`RaceRusherAgent::choose`]), so "the
/// first of the best class" is a fact about the engine's enumeration and not
/// about a sort this function performs. A [`RaceClass::Claim`] short-circuits —
/// nothing outranks it, so the scan stops rather than probing the rest.
fn race_pick(world: &GameState, rules: &Rules, candidates: &[Move]) -> Option<(RaceClass, Move)> {
    let mut best: Option<(RaceClass, &Move)> = None;
    for mv in candidates {
        let Some(class) = race_class(world, mv, rules) else { continue };
        // STRICTLY better replaces, so the FIRST candidate of each class is the
        // one that survives its own class — "first of the best class", not "last
        // of it".
        if best.is_none_or(|(held, _)| class < held) {
            best = Some((class, mv));
        }
        if class == RaceClass::Claim {
            break;
        }
    }
    best.map(|(class, mv)| (class, mv.clone()))
}

/// The declared adversarial REPLY: take a race step when one is in front of it,
/// and play greedy@final otherwise.
///
/// **The rule, in full** (design §2, and this is the whole of it):
/// 1. sample a PROBE WORLD from the seat's own view, at
///    [`race_probe_seed`]`(agent_seed, decision_ordinal)` — the honest chain,
///    and a stream that is NOT the inner greedy's;
/// 2. collect the candidates of the probed families only —
///    [`is_probed_family`], i.e. additional actions, area actions and choice
///    resolutions;
/// 3. classify each by apply-probe on that world ([`race_class`]);
/// 4. play the first candidate of the highest nonempty class ([`race_pick`]);
/// 5. otherwise DELEGATE the whole decision to the inner greedy.
///
/// **It is a positional OPPORTUNIST and not a planner**, and the freeze document
/// says so in its robustness caveat: movement is never probed, so it races only
/// from where it already stands. Read as "the hostile arm of the reply menu", it
/// is a threat MODEL — a reply that competes for the tracks — and never a claim
/// about how well the tracks can be raced.
///
/// **It OWNS its fallback for the seat's lifetime.** One [`GreedyAgent`],
/// constructed once at the seat's own `agent_seed`, whose stream advances
/// exactly as a bare greedy seat's would — because the probes above draw from a
/// derived stream and never from it. That is what makes the no-race fallback
/// byte-identical to greedy@final at the same seed BY CONSTRUCTION, and
/// `the_no_race_fallback_is_byte_identical_to_a_bare_greedy_seat` is the pin.
///
/// **Never a [`Contestant`], never gate-eligible**: it is an instrument for
/// asking what a position looks like against a hostile reply, and an instrument
/// that could enter a cell is one whose numbers someone will eventually quote.
pub(crate) struct RaceRusherAgent {
    /// The MIRRORED greedy@final agent — see [`Self::new`] for the mirror and
    /// what pins it.
    inner: GreedyAgent,
    /// The seat's own play stream seed, kept because the probe derivation and
    /// the provenance both need it and `GreedyAgent` will not hand its own back.
    agent_seed: u64,
    /// How many decisions this seat has been asked for. The probe world's second
    /// derivation component; incremented on EVERY `choose`, probed or not, so
    /// the ordinal means "this seat's n-th decision" rather than "this seat's
    /// n-th probe".
    decisions: u64,
    /// Candidates COLLECTED for classification across this seat's life — the
    /// probe-scope instrument. It counts what the scope rule admitted rather
    /// than what [`race_class`] was actually called on, because the two differ
    /// only by [`race_pick`]'s claim short-circuit and it is the SCOPE this
    /// counter exists to pin. Test-only: it makes "a movement decision probes
    /// NOTHING" an assertion rather than an argument, and a harness instrument
    /// does not need it in release.
    #[cfg(test)]
    probed_candidates: u64,
    /// Decisions this seat resolved by taking a race candidate rather than by
    /// delegating. Test-only, for the same reason.
    #[cfg(test)]
    race_picks: u64,
    /// The probe seed this seat's LAST `choose` actually derived — the value
    /// handed to [`BeliefSampler::from_view`], recorded where it is built rather
    /// than recomputed by a cell.
    ///
    /// [`race_probe_seed`] has its own pin, but a pin on the FUNCTION says
    /// nothing about the argument the agent passes it: freezing `ordinal` at 0
    /// — one seat, ONE probe world for its whole life — leaves every other cell
    /// in this module green. This is the field that makes the live ordinal
    /// observable, and `successive_decisions_probe_at_successive_ordinals` is
    /// what fails when it stops advancing. Test-only, like the two counters
    /// above.
    #[cfg(test)]
    last_probe_seed: Option<u64>,
}

impl RaceRusherAgent {
    /// Build the seat, including its own fallback.
    ///
    /// **The inner agent MIRRORS [`Contestant::build`]'s greedy arm** — the same
    /// constructor, the same explicitly-named frozen [`WeightTable::V2`] (L12: a
    /// weight is part of what a controller's name MEANS, so it is never a
    /// parameter here), the same knob, at the SAME `agent_seed` the seat plays
    /// on. It is a mirror rather than a `build` call for [`mirror_build`]'s own
    /// reason: `build` erases the agent behind `Box<dyn ArenaAgent>`, and this
    /// type owns a concrete `GreedyAgent` so the fallback's own surfaces stay
    /// reachable. The obligation that comes with mirroring is discharged the
    /// same way: `the_inner_greedy_is_contestant_builds_greedy_arm` pins that a
    /// delegated decision equals what `Contestant::build`'s greedy agent plays
    /// at the same seed on the same view.
    pub(crate) fn new(rules: &Rules, agent_seed: u64) -> Self {
        Self {
            inner: GreedyAgent::with_weights(
                rules,
                WeightTable::V2,
                DEFAULT_RETREAT_SAMPLES,
                agent_seed,
            ),
            agent_seed,
            decisions: 0,
            #[cfg(test)]
            probed_candidates: 0,
            #[cfg(test)]
            race_picks: 0,
            #[cfg(test)]
            last_probe_seed: None,
        }
    }
}

impl ArenaAgent for RaceRusherAgent {
    /// Delegated to the fallback, unconditionally.
    ///
    /// The race rule is about PLAY and says nothing about a Keep, so inventing a
    /// second Keep rule here would make a cell seating this reply
    /// setup-different from one seating greedy@final for no stated reason.
    /// Greedy's Keep draws no stream at all, so the delegation also cannot
    /// disturb the fallback identity.
    fn choose_starting_order(&mut self, seat: PlayerId, deal: &[OrderCardId; 2]) -> u8 {
        self.inner.choose_starting_order(seat, deal)
    }

    /// The rule, in the order [`RaceRusherAgent`]'s doc states it.
    ///
    /// **The ordinal advances on EVERY decision, probed or not.** It names "this
    /// seat's n-th decision", so a turn whose first decision is a movement step
    /// and whose second is an area action probes the second at ordinal 1 — which
    /// keeps the probe stream a function of the seat's position in the game
    /// rather than of how often the scope rule happened to fire.
    ///
    /// **A probe-sampler refusal is a HARD error and never a quiet delegation.**
    /// The sampler refuses a VIEW, not a seed, so the inner greedy would refuse
    /// the same view moments later; swallowing it here would turn a typed
    /// honest-chain failure into a silently different policy. It names the probe
    /// so the two refusals stay distinguishable in a report.
    fn choose(&mut self, view: &PlayerView, rules: &Rules) -> Result<DecisionReport, String> {
        let ordinal = self.decisions;
        self.decisions += 1;

        // The PROBE WORLD — the agent's own, from its own view, on a stream that
        // is not the inner greedy's (see `SALT_DRIVEN_RACE_PROBE`).
        let probe_seed = race_probe_seed(self.agent_seed, ordinal);
        #[cfg(test)]
        {
            self.last_probe_seed = Some(probe_seed);
        }
        let mut sampler = BeliefSampler::from_view(view, rules, probe_seed).map_err(|e| {
            format!(
                "{RACE_RUSHER_CONTROLLER}'s probe sampler refused seat {}'s view: {e}",
                view.viewer.0,
            )
        })?;
        let world = sampler.sample();

        // ONE handle for the whole decision (L10): the family tags, the probed
        // candidates and the volume observation all come off it, so a raced
        // decision pays for one enumeration exactly as a rung's does.
        let handle = DecisionHandle::new(&world, rules);
        // The probed families in `families()`' own order, and within each the
        // order `iter_family` yields — which IS the engine's legal-list order
        // restricted to those families (`tests/action_source_equivalence.rs`
        // pins the ordered union against `get_legal_moves`). So "the first
        // candidate of the best class" is a fact about the engine's enumeration
        // and not about any ordering this agent imposes.
        let candidates: Vec<Move> = handle
            .families()
            .iter()
            .filter(|family| is_probed_family(family))
            .flat_map(|family| handle.iter(family))
            .collect();
        #[cfg(test)]
        {
            self.probed_candidates += candidates.len() as u64;
        }

        if let Some((_class, mv)) = race_pick(&world, rules, &candidates) {
            // Unreachable in the same sense `root_table`'s accounting refusals
            // are: a candidate was served, so this decision point HAS families
            // and `decision_volume` answers. It is an error rather than a
            // fallback because a report that silently degraded to greedy here
            // would be a report about a different policy.
            let volume = decision_volume(&handle).ok_or_else(|| {
                format!(
                    "{RACE_RUSHER_CONTROLLER} raced at a decision point reporting no volume, at \
                     {:?} for seat {}",
                    handle.turn_phase(),
                    view.viewer.0,
                )
            })?;
            #[cfg(test)]
            {
                self.race_picks += 1;
            }
            return Ok(DecisionReport { mv, volume });
        }

        // NOTHING RACES: the whole decision goes to the fallback, whose stream
        // the probes above did not touch.
        self.inner.choose(view, rules)
    }

    /// **The fallback's identity rides the params; the fallback's NAME rides the
    /// echo.** `ControllerProvenance::params` is `i64`-valued, so a delegate's
    /// name cannot travel in it — what travels instead is the delegate's whole
    /// knob map VERBATIM (its `m`, its placement cap, its evaluator version, its
    /// weight-table digest) plus `fallback_version`, all read OFF the inner
    /// agent rather than re-derived, and the composed [`race_rusher_echo`] on the
    /// seat descriptor beside it carries the name.
    fn controller(&self) -> ControllerProvenance {
        let fallback = self.inner.controller();
        let mut params = fallback.params;
        params.insert("fallback_version".to_string(), i64::from(fallback.version));
        ControllerProvenance {
            name: RACE_RUSHER_CONTROLLER.to_string(),
            version: RACE_RUSHER_IMPL_VERSION,
            params,
            agent_seed: Some(self.agent_seed),
        }
    }
}

// ---------------------------------------------------------------------------
// Roots: where a rooted run starts, and what it says about itself (T3)
// ---------------------------------------------------------------------------

/// WHICH POSITIONS a [`RootSource::Generated`] scan looks for.
///
/// v1 shipped the TYPE plus exactly ONE predicate (design §3), and the restraint
/// was deliberate: a menu of predicates invented before a consumer asks for them
/// is a menu of fixtures chosen after seeing what each one selects. **Phase C's
/// layer 2 adds five, each with the family that asks for it** (design
/// `docs/plans/2026-08-18-phase-c-families-design.md` §1): one nonterminal
/// CONTROL for family 1, a positive/control pair for family 2, a pilot probe for
/// family 3, and family 4's positive.
///
/// **PREDICATES SELECT ON TRUTH** (design §2). Every arm below reads the whole
/// `GameState` — hidden decks included — because a generator is not a player: it
/// is the thing that CHOOSES the position, and choosing it from a seat's view
/// would make the population a function of what one seat happened to know. The
/// SUBJECT at the resulting root still plays its own honest chain, which is the
/// property that matters and the one [`drive_to_terminal`] enforces.
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
    /// FAMILY 1's NONTERMINAL CONTROL — the trigger has fired but the final
    /// round has NOT begun: `end_game_triggered && !final_round &&
    /// !is_game_over`.
    ///
    /// The amendment's "nonterminal control earlier in the round" (design §1),
    /// and the exact COMPLEMENT of [`Self::FinalRoundAlreadyTriggered`] on the
    /// `final_round` conjunct — which is the point: the two predicates partition
    /// the triggered, non-terminal positions, so a family-1 result that holds on
    /// one and not the other is a statement about the final round rather than
    /// about the trigger. The session's g1:228 shape.
    TriggerRoundNotFinal,
    /// FAMILY 2's POSITIVE — the mover can END THE GAME this move and is AHEAD
    /// in truth: `!end_game_triggered && !firing_moves(..).is_empty() &&
    /// true_margin_now(..) >= TRUE_MARGIN_BAND && width_ok`.
    ///
    /// Canon 15's inverted brake, made a selection rule: the family supplies
    /// true-arithmetic inputs (the F9 lesson) rather than testing whether the
    /// subject can reconstruct them. [`TRUE_MARGIN_BAND`] carries the +10
    /// calibration and its reasoning.
    EndFirableAheadInTruth,
    /// FAMILY 2's CONTROL — the same shape with the mover BEHIND in truth
    /// (`true_margin_now(..) <= -TRUE_MARGIN_BAND`).
    ///
    /// Canon 15's own sentence: ending the game when you know you are going to
    /// lose is a bad idea, so the behind mover should hand off WITHOUT firing.
    EndFirableBehindInTruth,
    /// FAMILY 3's PILOT PROBE — an opponent is ONE COMPLETED ACTION from a
    /// contested track step and the mover holds a move that takes it.
    ///
    /// The exact arithmetic is [`opponent_race_steps`] (the opponent side) and
    /// [`mover_reachable_steps`] (the mover side); the predicate is their
    /// INTERSECTION plus `width_ok`. Read those two doc comments before quoting
    /// this predicate's supply — the design accepts that this arm may read
    /// `Exhausted` at the scan bound (family 3's honest source is T5's builder;
    /// this is the generated-arm probe whose supply is MEASURED and recorded),
    /// so it is written to favour PRECISION over recall throughout.
    ExclusiveRaceOneStep,
    /// FAMILY 4's POSITIVE — a legal `Move::FulfillAchievement` exists (plus
    /// `width_ok`).
    ///
    /// F8's live-green shape. No proximity clause: the leg-completable-within-
    /// the-turn corner is a BUILDER block (design §1 family 4), because a claim
    /// two decisions away is not a claim in the legal list and a predicate that
    /// tried to see it would be searching, not selecting.
    AchievementClaimAvailable,
    /// **V4 STEP 2, the S-group's development/escrow generator** (spec
    /// `docs/plans/2026-08-20-phase3.5c-v4-spec-prereg.md` §6): the mover's turn
    /// STARTS with ≥ 2 same-total placements into different area sets.
    ///
    /// The registration is one sentence and every word of it is a conjunct — see
    /// [`is_mover_turn_start`] for "turn starts", [`turn_start_placements`] for
    /// which placements are meant, and [`same_count_fork`] for "≥ 2 same-total
    /// into different area sets". What is NOT here is as load-bearing:
    ///
    /// * **no width conjunct.** Families 2–4 carry [`MAX_ROOT_CANDIDATES`]
    ///   because their reads ADJUDICATE (every root candidate, K playouts, the
    ///   whole reply menu). A V4 development root is read by evaluating feature
    ///   COLUMNS over the fork, which is cents per candidate — and the width a
    ///   bound would measure at a turn-start state is the ADDITIONAL-ACTION
    ///   list's, not the placement list's, so the conjunct would exclude the
    ///   rich placement positions for a number computed at the wrong position.
    /// * **no `!is_game_over`.** It is implied: `is_game_over` keys on
    ///   [`TurnPhase::TurnEnd`] and nothing else ever assigns that phase, so a
    ///   state in `AdditionalActionFirst` is never over. The pin says so rather
    ///   than a redundant conjunct claiming to.
    ///
    /// **DISCLOSED LOOSENESS, measured rather than argued.** The literal rule is
    /// satisfied by any turn start where the mover holds an assistant and can
    /// afford two different single-area placements — which is most turn starts.
    /// That is the registration, and this predicate implements the registration;
    /// [`scan_capacity`] is the instrument that says how loose it is at a master,
    /// and step 2's capacity measurement is where that number belongs.
    ///
    /// # MEASURED SUPPLY (2026-08-21, step-2 capacity read)
    ///
    /// `capacity placement-fork-same-count --master 0x0401_2026_0821_9999`
    /// (the declared dev-smoke throwaway) reached its `--stop-after 4` bound in
    /// TWO generated games — ordinals 0,1 from game 0 and 2,3 from game 1, i.e.
    /// the stride cap was spent in both. So the supply is ample and the
    /// looseness above is confirmed rather than merely suspected: this predicate
    /// selects near the first turn start it sees. What makes a development root
    /// worth reading is therefore the step-5 gate (the S columns take ≥ 2
    /// distinct values across the fork's same-count set), not this selection.
    PlacementForkSameCount,
    /// **V4 STEP 2, the E-group's development/escrow generator** (spec §6): the
    /// NON-MOVER holds ≥ 1 order, VISIBLE to the mover, whose requirements the
    /// non-mover's own stocks satisfy.
    ///
    /// **The visibility qualifier is the whole point and it is not decorative.**
    /// An unfulfilled hidden STARTING order is a card the mover cannot see, and
    /// an E-column read over a card no seat could read would be a read about
    /// privileged information. [`order_visible_to_others`] is the exact masking
    /// rule `observe::visible_player` applies, mirrored here and PINNED against
    /// `observe` itself rather than restated and hoped about.
    ///
    /// **The predicate still selects on TRUTH** (design §2), and the two facts
    /// are not in tension: a generator reads the whole `GameState` because it is
    /// choosing the position, and what it chooses here is a position where a
    /// MOVER-VISIBLE fact holds. The stocks half is `can_fulfill_order`'s own
    /// arithmetic on the holder — public goods, so nothing hidden enters either
    /// half.
    ///
    /// `!is_game_over` IS a conjunct here and is a judgment call named as one:
    /// spec §6's sentence does not carry it, but the §6 read is
    /// "≥ 1 for the opponent WITH the do-nothing-successor contrast", and a
    /// terminal state has no successor to contrast against. Every other layer-2
    /// arm excludes the terminal position for the same reason.
    ///
    /// # MEASURED SUPPLY: ZERO (2026-08-21, step-2 capacity read)
    ///
    /// `capacity opponent-sitting-executable --master 0x0401_2026_0821_9999`
    /// (the declared dev-smoke throwaway) walked the WHOLE registered ecology —
    /// all 240 games of `driven-generated-v2.2-lookahead`, EXHAUSTIVE, not
    /// truncated — and found **0 strided ordinals**. This predicate has no
    /// development band and no escrow band to draw at that master, and spec §6's
    /// supply-miss recovery (a prefix-stable scan-games raise) cannot help a
    /// rate that is not small but ZERO.
    ///
    /// **The zero is the POPULATION, not the implementation**, and the
    /// diagnostic that separates them is recorded here because it is the whole
    /// finding. It was taken over THE WALK'S OWN FIRST EIGHT GAMES — the same
    /// master, the same `SeedSchedule::diagnostic` draw order, the same
    /// `LOOKAHEAD_V1_DEFAULT` at both seats on the same
    /// `agent_stream_seed(world, seat, Slot(seat.0))` keying — instrumented to
    /// count instead of to select. It is a PREFIX of the 240-game walk and not an
    /// independent sample, and quoting it as corroboration would be quoting the
    /// same games twice. Across those 2,399 non-terminal positions:
    ///
    /// | quantity (position × held card)                   | count |
    /// |---------------------------------------------------|-------|
    /// | non-mover holds a VISIBLE card                     |   472 |
    /// | non-mover holds a PAYABLE card (visible or not)    |    24 |
    /// | non-mover holds a payable VISIBLE card (this arm)  | **0** |
    /// | MOVER holds a payable visible card                 |    48 |
    ///
    /// Visible non-mover cards are common and payable visible cards exist — but
    /// only ever in the hand of the seat ON THE MOVE. Every payable card a
    /// NON-mover holds is its unfulfilled hidden STARTING order, which this arm
    /// correctly excludes. The mechanism is not subtle: `Move::FulfillOrder` is
    /// an additional action in BOTH of a turn's phases, a visible held card is a
    /// CHOSEN Port take rather than a dealt one, and a non-mover's trade-good
    /// stocks cannot change during the opponent's turn (the mover's actions move
    /// no goods across seats; Trading House passive income is yen). So a seat
    /// that fulfils what it can before handing off is a seat that never sits on
    /// a visible executable order — and `lookahead-v1` does, at every one of
    /// those positions.
    ///
    /// **The consequence reaches past root supply**: `O3(opponent)` — spec §3.2's
    /// `ExecutableOrders` over KNOWN held orders, which is this arm's quantity
    /// exactly — is a CONSTANT ZERO on this ecology's production bytes, which is
    /// §8's business and not a generator's. Two scope limits keep that honest:
    /// the ecology seats the SHIPPED `lookahead-v1` at `WeightTable::V2`, so a
    /// fitted V4 identity may play differently; and hand-built pair fixtures
    /// (family 4(b)) declare their own O3 and are untouched by any of this.
    ///
    /// **RULED 2026-08-22 (owner; the guidingpath four-flags row): this
    /// predicate's generated bands are STRUCK and its master (M-OSE) is closed
    /// permanently undrawn.** The step-5/step-10 opponent-sitting reads re-scope
    /// to the game-3 (table 896199544) opponent-pending RECORDED root plus the
    /// constructed fixtures — the owner's grounds, recorded as design evidence:
    /// human players deliberately sit on executable orders (batching
    /// fulfillments, masking an import race, banking action-bonus icons), so the
    /// phenomenon is real in the target distribution and this ecology's zero
    /// only measures what the shipped never-declining policy cannot express.
    /// The variant stays in the vocabulary: its refusal text and this record are
    /// what a future ecology amendment would build from.
    OpponentSittingExecutable,
}

impl RootPredicate {
    /// Every variant, for callers that enumerate the vocabulary (the runner's
    /// `gen:` grammar names this set in its refusal, and the tests sweep it).
    ///
    /// HAND-MAINTAINED: adding a variant is a compile error in [`Self::as_str`],
    /// [`Self::holds`] and [`Self::config`] — all three are exhaustive matches —
    /// but NOT here, so a new variant must be added to this array by hand. The
    /// name-distinctness pin is what catches a copy-paste that forgot to.
    pub const ALL: [Self; 8] = [
        Self::FinalRoundAlreadyTriggered,
        Self::TriggerRoundNotFinal,
        Self::EndFirableAheadInTruth,
        Self::EndFirableBehindInTruth,
        Self::ExclusiveRaceOneStep,
        Self::AchievementClaimAvailable,
        Self::PlacementForkSameCount,
        Self::OpponentSittingExecutable,
    ];

    /// The stable name, for descriptors and error text.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FinalRoundAlreadyTriggered => "final-round-already-triggered",
            Self::TriggerRoundNotFinal => "trigger-round-not-final",
            Self::EndFirableAheadInTruth => "end-firable-ahead-in-truth",
            Self::EndFirableBehindInTruth => "end-firable-behind-in-truth",
            Self::ExclusiveRaceOneStep => "exclusive-race-one-step",
            Self::AchievementClaimAvailable => "achievement-claim-available",
            Self::PlacementForkSameCount => "placement-fork-same-count",
            Self::OpponentSittingExecutable => "opponent-sitting-executable",
        }
    }

    /// Whether `state` is one of the positions this predicate names.
    ///
    /// **The signature carries `&Rules` since Phase C** and the shipped v1
    /// variants ignore it: the layer-2 arms probe the LEGAL LIST (a firing move,
    /// a claim, a step the mover can take), which no state can answer on its
    /// own. The two rules-free arms are pinned byte-identical across two
    /// different `Rules` values, so "ignores it" is a property rather than a
    /// reading of the code.
    ///
    /// **Cheap conjuncts come first, deliberately.** The scan evaluates this at
    /// EVERY position of every generated game, so each arm is ordered so that
    /// the flag reads and the short-circuited [`firing_moves`] precondition
    /// reject the overwhelming majority of positions before any enumeration or
    /// apply-probe runs.
    pub fn holds(self, state: &GameState, rules: &Rules) -> bool {
        match self {
            Self::FinalRoundAlreadyTriggered => {
                state.end_game_triggered && state.final_round && !is_game_over(state)
            }
            Self::TriggerRoundNotFinal => {
                state.end_game_triggered && !state.final_round && !is_game_over(state)
            }
            Self::EndFirableAheadInTruth => {
                !state.end_game_triggered
                    && !firing_moves(state, rules).is_empty()
                    && true_margin_now(state, rules) >= TRUE_MARGIN_BAND
                    && width_ok(state, rules)
            }
            Self::EndFirableBehindInTruth => {
                !state.end_game_triggered
                    && !firing_moves(state, rules).is_empty()
                    && true_margin_now(state, rules) <= -TRUE_MARGIN_BAND
                    && width_ok(state, rules)
            }
            Self::ExclusiveRaceOneStep => {
                if is_game_over(state) {
                    return false;
                }
                let contested = opponent_race_steps(state);
                if contested.is_empty() {
                    return false;
                }
                let legal = get_legal_moves(state, rules);
                if !width_ok_len(legal.len()) {
                    return false;
                }
                let reachable = mover_reachable_steps(state, &legal, rules);
                contested.iter().any(|step| reachable.contains(step))
            }
            Self::AchievementClaimAvailable => {
                let legal = get_legal_moves(state, rules);
                width_ok_len(legal.len())
                    && legal.iter().any(|mv| matches!(mv, Move::FulfillAchievement { .. }))
            }
            Self::PlacementForkSameCount => {
                if !is_mover_turn_start(state) {
                    return false;
                }
                // The cheapest conjunct there is, and it comes before the
                // clone: a mover holding nothing reaches exactly ONE placement
                // (the always-legal empty one), which no total can fork on.
                if state.player(state.current_player).assistants.in_hand == 0 {
                    return false;
                }
                turn_start_placements(state, rules)
                    .is_some_and(|placements| same_count_fork(&placements))
            }
            Self::OpponentSittingExecutable => {
                if is_game_over(state) {
                    return false;
                }
                let mover = state.current_player;
                state.players.iter().any(|holder| {
                    holder.id != mover
                        && holder.order_cards_in_hand.iter().any(|&order| {
                            order_visible_to_others(holder, order)
                                && can_fulfill_order(state, holder.id, order, &rules.orders)
                        })
                })
            }
        }
    }

    /// Which generating configuration a scan for this predicate runs under.
    ///
    /// The family-1 pair scans the CHEAP rung ecology (a final-round position is
    /// a fact about the rules' own clock, and a rung reaches it for the price of
    /// a game); the four layer-2 predicates scan the LOOKAHEAD ecology, which
    /// generates the SUBJECT'S OWN reachable positions — the right population
    /// for an on-policy checkpoint, and the reason the ecology is recorded on
    /// every descriptor rather than assumed.
    ///
    /// The two V4 arms scan the LOOKAHEAD ecology too, and that is spec §6's own
    /// registration (`driven-generated-v2.3-lookahead`), not an inference: the
    /// V4 read is about positions the SUBJECT reaches, so the population has to
    /// be the subject's.
    pub fn ecology(self) -> GeneratingEcology {
        self.config().ecology
    }

    /// [`Self::ecology`] plus the live parts a scan needs — see
    /// [`GeneratingConfig`], which exists so the recorded seat ECHOES cannot
    /// drift from the contestants the scan actually seats.
    fn config(self) -> GeneratingConfig {
        match self {
            Self::FinalRoundAlreadyTriggered | Self::TriggerRoundNotFinal => v1_config(),
            Self::EndFirableAheadInTruth
            | Self::EndFirableBehindInTruth
            | Self::ExclusiveRaceOneStep
            | Self::AchievementClaimAvailable
            | Self::PlacementForkSameCount
            | Self::OpponentSittingExecutable => v2_lookahead_config(),
        }
    }
}

/// The TWO predicates spec §6 declares ORDINAL BANDS for.
///
/// A named array rather than a method, so every consumer of the bands (the burn
/// guard, the disjointness pins, the capacity report) reads ONE list and a third
/// V4 predicate cannot be added to the bands by being added to
/// [`RootPredicate::ALL`]. The other six arms have no band at all — family
/// blocks run confirmatory ordinals 4..28 by declaration, so a band applied to
/// them would refuse the frozen blocks.
pub const V4_BANDED_PREDICATES: [RootPredicate; 2] =
    [RootPredicate::PlacementForkSameCount, RootPredicate::OpponentSittingExecutable];

/// Spec §6's DEVELOPMENT band, half-open, per predicate.
pub const V4_DEVELOPMENT_ORDINALS: std::ops::Range<u32> = 0..4;

/// Spec §6's DEVELOPMENT EXTENSION band — drawn only on a development supply
/// miss, ledger-recorded, and still development.
pub const V4_DEVELOPMENT_EXTENSION_ORDINALS: std::ops::Range<u32> = 4..8;

/// Spec §6's ESCROW band, never re-purposed. The BURN RULE fences it: no
/// `table` / `adjudicate` / read of any kind here before step 10.
pub const V4_ESCROW_ORDINALS: std::ops::Range<u32> = 8..16;

/// Are ordinals `a` and `b` GUARANTEED to come from different generated games,
/// whatever any scan at any master happens to find?
///
/// **The stride lemma, and it is arithmetic rather than a hope.**
/// [`generate_roots`] walks games in draw order and positions in play order,
/// increments the ordinal once per kept hit, and stops walking a game the moment
/// `per_game_cap` hits have been kept there. So the ordinals one game
/// contributes are CONSECUTIVE and number at most `per_game_cap` — which makes
/// two ordinals from one game differ by at most `per_game_cap - 1`, and any pair
/// at least `per_game_cap` apart provably different games.
///
/// **This is a GUARANTEE and not a test.** A pair closer than the cap MAY still
/// land in different games (they usually do); what this says is that nothing
/// about the population can make a separated pair share one. Spec §6's
/// independence claim is exactly this lemma at `GENERATED_STRIDE_CAP = 2`, and
/// the place it does NOT reach is the development-EXTENSION boundary: ordinals 7
/// and 8 are one apart, so a game contributing a TWO-ordinal run can land
/// exactly `{7, 8}` and hold both. (The straddler is a two-hit game, never a
/// one-hit one — a game contributing a single ordinal cannot hold two. What
/// one-hit games do is shift the run boundaries: with every game yielding two
/// hits the runs are `{0,1} {2,3} {4,5} {6,7} {8,9}` and 7/8 never share, and it
/// takes an ODD number of ordinals ahead of a two-hit game to line it up on
/// `{7, 8}`.)
/// `the_stride_separates_the_development_band_from_escrow_but_not_the_extension`
/// pins both halves of that, and [`scan_capacity`] is what an orchestrator
/// checks the straddle with if the extension is ever drawn.
pub const fn ordinals_are_stride_separated(a: u32, b: u32, per_game_cap: u32) -> bool {
    a.abs_diff(b) >= per_game_cap
}

// ---------------------------------------------------------------------------
// V4 step 2: the two predicates' shared conjuncts (spec §6)
// ---------------------------------------------------------------------------

/// Is the MOVER'S TURN at its first phase, with nothing queued and no nested
/// sub-turn running?
///
/// **This is "the turn starts" as a state can express it, and the gap is named
/// rather than papered over.** `GameState` is HISTORY-FREE by design, so no
/// state-local rule can tell the FIRST decision of phase A from a later one:
/// the mover may already have fulfilled an order or claimed an achievement
/// there, and neither writes a flag ([`AdditionalActionState`] records only the
/// mercenary / University / Spinning Mill once-per-turn locks — see
/// `crate::types::AdditionalActionState`, which is the whole of it). A test that
/// demanded a pristine `additional_action_state` would therefore be a PARTIAL
/// rule wearing a total name — it would reject a mover who used University and
/// accept one who fulfilled three orders. What this predicate actually needs is
/// the property that IS total: the turn's PLACEMENT is still ahead of the mover,
/// which is exactly `AdditionalActionFirst`.
///
/// `pending_choices.is_empty()` is load-bearing twice over — a queued choice is
/// a different decision from the one the root is about, and
/// `EndAdditionalPhase` (the probe [`turn_start_placements`] applies) refuses
/// mid-queue by its own self-defence.
///
/// `!is_mercenary_action` is UNREACHABLE under legal play — a mercenary sub-turn
/// runs in the AreaAction..Recovery phases and restores `AdditionalActionFirst`
/// only as it clears the flag — and it is still checked, because `holds` must be
/// total over hand-built states too, and a probe that ended the phase inside a
/// nested sub-turn would enumerate placements the mover never reaches.
fn is_mover_turn_start(state: &GameState) -> bool {
    state.turn_phase == TurnPhase::AdditionalActionFirst
        && state.pending_choices.is_empty()
        && !state.is_mercenary_action
}

/// The placements the mover reaches by DECLINING every additional action —
/// `None` at a position that is not a turn start.
///
/// **The probe applies the engine's own move and reads the engine's own list,
/// and both halves are the point.** The placement family is not in the legal
/// list at a turn start (phase A offers additional actions and
/// `EndAdditionalPhase`), so a predicate that wanted to see it had two choices:
/// re-derive the placement enumeration from hand count and board, or ADVANCE the
/// state one move and ask `get_legal_moves`. The first is a second
/// implementation of `generate_placement_moves_with_tech` — including the
/// Newspaper 4-area case, the Telegram adjacency, the Canal exclusion and the
/// per-President placement toll — and a generator whose population depended on a
/// second opinion about legality is a generator nobody can check. This is the
/// second.
///
/// `EndAdditionalPhase` is ALWAYS in the legal list at
/// `AdditionalActionFirst` with no queue (`generate_additional_action_moves`
/// pushes it unconditionally, first), so `apply_move_unchecked` is being handed
/// a move drawn from the list exactly as its contract requires, and the `.ok()`
/// is not a fallback: the only failure it can express is the pending-choice
/// self-defence that [`is_mover_turn_start`] has already excluded.
///
/// **What this DOES fix is which line the fork is measured on**: the placements
/// a mover reaches after using University (one more assistant) or a mercenary
/// are a different set. Selecting on the plain line is a selection rule, stated;
/// it is not a claim that the mover will take it.
fn turn_start_placements(state: &GameState, rules: &Rules) -> Option<Vec<Move>> {
    if !is_mover_turn_start(state) {
        return None;
    }
    let placing = apply_move_unchecked(state, &Move::EndAdditionalPhase, rules).ok()?;
    Some(get_legal_moves(&placing, rules))
}

/// One placement move's SHAPE: how many assistants it places, and the SET of
/// areas it places them into.
///
/// The two encodings are not symmetric and the arithmetic says so:
/// `PlaceThreeAreas` places ONE assistant per listed area (Option A's "1 in each
/// of 1–N DIFFERENT areas", so the list is already distinct and the generator
/// emits it in ascending area order — there is no `[A,B]` / `[B,A]` pair to
/// collapse), while `PlaceTwoSameArea` places TWO into its area and, with
/// Telegram, one more into an adjacent one. So the totals are `len`, 2 and 3,
/// and the sets are 1-, 1- and 2-element.
///
/// `None` for anything else. Nothing else is reachable — the Placement phase
/// generates only these two variants — and the caller SKIPS rather than panics,
/// because a predicate is not the place to discover that the phase machine
/// changed.
///
/// **`pub` is a VISIBILITY-ONLY unblock (V4 §7 step 5), on `visible_power`'s
/// precedent — the body is untouched and the frozen predicate above computes
/// exactly what it computed before.** The step-5 e113 exemplar registration has
/// to ask a candidate placement for its total and its area set to apply its own
/// selection criterion, and the only two honest options were to call THIS or to
/// restate the `PlaceThreeAreas` / `PlaceTwoSameArea` arithmetic in the probe —
/// where a Telegram-swap arm quietly disagreeing with the predicate's reading
/// would make "same count" mean two different things in two files.
pub fn placement_shape(mv: &Move) -> Option<(usize, BTreeSet<AreaId>)> {
    match mv {
        Move::PlaceThreeAreas { placements } => {
            Some((placements.len(), placements.iter().copied().collect()))
        }
        Move::PlaceTwoSameArea { area, adjacent_bonus } => {
            let mut areas = BTreeSet::new();
            areas.insert(*area);
            match adjacent_bonus {
                Some(adjacent) => {
                    areas.insert(*adjacent);
                    Some((3, areas))
                }
                None => Some((2, areas)),
            }
        }
        _ => None,
    }
}

/// Spec §6's "≥ 2 same-total placements into different area sets": does any ONE
/// total carry two distinct area sets?
///
/// **SETS, because "sets" is what the registration says** — and the reading
/// costs nothing, which is worth showing rather than asserting. The only pair a
/// MULTISET reading would admit and this one would not is the Telegram swap:
/// `{A:2, B:1}` against `{B:2, A:1}`, same total 3, same SET `{A, B}`. Wherever
/// that pair is offered, the position also offers the two single-area placements
/// `[A]` and `[B]` — total 1 with sets `{A}` and `{B}`, trivially different —
/// because the Telegram move's own price is `cost(A,A) + cost(B)`, which
/// dominates each of `cost(A)` and `cost(B)`, so a mover who can afford the swap
/// can afford both singles. The two readings therefore never DISAGREE about a
/// position; they differ only in which pair witnesses. The sweep in
/// `the_set_reading_and_the_multiset_reading_never_disagree_about_a_position`
/// is that argument made executable over organic states.
///
/// The scan runs this at every turn start of every generated game, so it returns
/// the moment a total reaches two sets rather than tabulating the whole family.
fn same_count_fork(placements: &[Move]) -> bool {
    let mut by_total: BTreeMap<usize, BTreeSet<BTreeSet<AreaId>>> = BTreeMap::new();
    for mv in placements {
        let Some((total, areas)) = placement_shape(mv) else {
            continue;
        };
        let sets = by_total.entry(total).or_default();
        sets.insert(areas);
        if sets.len() >= 2 {
            return true;
        }
    }
    false
}

/// Is `order`, held by `holder`, a card the OTHER seats can see?
///
/// **A MIRROR of `observe::visible_player`'s hand-masking branch, and the mirror
/// is drift-pinned rather than trusted** (`the_visibility_mirror_agrees_with_
/// observe_on_every_held_order`). The masking rule is one of the five hidden
/// members, so the honest options were to CALL `observe` — which clones the
/// whole board layout and every seat, at every position of a 240-game scan — or
/// to restate the branch and hold it equal to its authority. This is the second,
/// on `root_table`-mirrors-`Contestant::build`'s precedent.
///
/// The two arms are `visible_player`'s own, in its own order:
///
/// * `starting_order_hidden` — the LIVE-tracking fiction, where the dealt card
///   is recorded NOWHERE and the view SYNTHESIZES a masked slot ahead of the
///   hand. Every card actually in `order_cards_in_hand` is a witnessed Port take
///   there, so every one of them is visible. (The invariant net makes the arm
///   exclusive: a seat may not carry the flag AND an identity.)
/// * otherwise the unfulfilled starting order is masked IN PLACE and nothing
///   else is. Once fulfilled it has left the hand for the face-up completed
///   pile, so the equality below stops masking exactly when the view does.
fn order_visible_to_others(holder: &crate::types::PlayerState, order: OrderCardId) -> bool {
    if holder.starting_order_hidden {
        true
    } else {
        holder.starting_order != Some(order)
    }
}

// ---------------------------------------------------------------------------
// V4 step 2: the §6 ESCROW RECIPE (spec §6's placement-fork pair recipe)
// ---------------------------------------------------------------------------

/// Which side of the §6 recipe one placement candidate is on.
///
/// **THREE outcomes, not two, and the third is the load-bearing one.** §6's
/// recipe wants a pair whose value distinctness is RULES-DERIVED and
/// unambiguous, so a candidate qualifies as a side only when its whole placed
/// set agrees:
///
/// * every placed area is a production area and every one of them exports a
///   nonzero reward → [`Productive`](Self::Productive);
/// * every placed area is a production area and every one of them exports zero
///   → [`Dead`](Self::Dead);
/// * anything else → [`Ineligible`](Self::Ineligible).
///
/// The `Ineligible` bucket holds three genuinely different positions, all of
/// which would corrupt the read if admitted:
///
/// 1. **A placement touching a NON-production area.** The dispatcher is silent
///    outside the four production types, and silence is not zero: a Bank
///    placement exports 1..5 Yen through a different helper, so calling it dead
///    would state a rules falsehood.
/// 2. **A MIXED production placement** — some placed areas exporting, some not.
///    §6 asks for the candidate placing into "areas whose exported action
///    reward … is nonzero" against "one where it is zero"; a mixed candidate
///    answers to neither description, and admitting it would make the pair's
///    direction a matter of which reading a reader picked.
/// 3. **The EMPTY placement.** `PlaceThreeAreas { placements: vec![] }` is
///    always in the list — it is how a mover declines to place — and both "all
///    exporting" and "all not exporting" are VACUOUSLY true of an empty set. A
///    classifier that let the vacuous truth decide would pair a placement
///    against a non-placement and call the difference a value distinction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeSide {
    /// Every placed area is a production area and every one of them exports a
    /// NONZERO reward at the post-placement arriving power; the payload is the
    /// summed reward.
    Productive(u32),
    /// Every placed area is a production area and every one of them exports
    /// ZERO.
    Dead,
    /// Neither side of a rules-distinct pair.
    Ineligible,
}

/// One placement candidate from the fork, classified.
///
/// Carried as DATA so the step-10 ledger row can record what the recipe saw
/// rather than only what it chose — a miss is then diagnosable from the same
/// call that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeCandidate {
    /// The placement move itself, as the engine enumerated it.
    pub mv: Move,
    /// How many assistants it places.
    pub total: usize,
    /// The SET of areas it places into.
    pub areas: BTreeSet<AreaId>,
    /// Which side of the recipe it is on.
    pub side: RecipeSide,
}

/// The §6 recipe PAIR: two same-total placements from one fork, separated by
/// the ENGINE-EXPORTED rules quantity and by nothing else.
///
/// **It fixes the recipe's QUANTITIES and no more than that** (spec §13's
/// driven-side clause). It carries no feature column, no weight, no evaluator
/// reading and no direction claim: the three things §6's escrow read then asks
/// of this pair — that the frozen raw definitions separate it, that no blocking
/// collision sits between its sides, and that the locked production identity's
/// weighted preference agrees with the rules-derived direction — are asked of
/// it by the READ, at step 10, over an object that was fixed before any of them
/// could influence it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipePair {
    /// The seat whose turn is starting.
    pub mover: PlayerId,
    /// The shared placement total both sides carry.
    pub total: usize,
    /// The placement into areas that export a nonzero reward.
    pub productive: Move,
    /// That placement's area set.
    pub productive_areas: BTreeSet<AreaId>,
    /// The summed exported reward of the productive side, > 0.
    pub productive_reward: u32,
    /// The same-total placement into areas that export zero.
    pub dead: Move,
    /// That placement's area set.
    pub dead_areas: BTreeSet<AreaId>,
}

/// Why a root yields no §6 recipe pair.
///
/// **Typed, and every arm is a STOP.** The recipe's whole claim is that its
/// pair is rules-distinct; a nearest-available pair would be a pair whose
/// distinctness nobody established, and the escrow read riding it would be
/// wrong with no way to notice. So there is no fallback arm anywhere below —
/// the ordinal simply does not carry the read, which is a fact the step-10
/// ledger row records rather than a failure to work around.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecipeMiss {
    /// The root is not a mover turn start.
    NotATurnStart,
    /// A placement the engine's own legal list offered did not apply.
    ///
    /// # Unreachable today, and the argument is NOT "it came from the legal list"
    ///
    /// [`classify_recipe_candidates`] enumerates on the state AFTER
    /// `EndAdditionalPhase` (phase `Placement`) and then applies each candidate
    /// to the ROOT (phase `AdditionalActionFirst`). The two applications agree
    /// BYTE FOR BYTE, but not because a move drawn from a legal list always
    /// applies — it is because of three facts about those particular arms, and
    /// a step-10 debugger reading this arm deserves the real ones:
    ///
    /// 1. The successor differs from the root in `turn_phase` and in NOTHING
    ///    ELSE. `advance_phase`'s `AdditionalActionFirst` arm is the single
    ///    assignment `AdditionalActionFirst => Placement`
    ///    (apply_move.rs:252–254); every other side effect of that function —
    ///    the mover, the turn number, the additional-action locks, the
    ///    turn-start tech effects — lives in the `AdditionalActionSecond` arm
    ///    and is not on this path.
    /// 2. **Both placement arms are PHASE-BLIND.** Neither `PlaceThreeAreas`
    ///    nor `PlaceTwoSameArea` reads `turn_phase` — their self-defence is the
    ///    count cap, distinctness, Telegram ownership and adjacency, and
    ///    per-area membership/placeability/supply/fees — and each ENDS by
    ///    overwriting `turn_phase` with `Movement`. So the one field that
    ///    differs is neither read nor preserved.
    /// 3. The queue guard is pre-satisfied: [`is_mover_turn_start`] already
    ///    required `pending_choices.is_empty()`, and the placement arms do not
    ///    consult the queue at all.
    ///
    /// **So the unreachability DEPENDS on fact 2 holding.** If a placement arm
    /// ever grew a `turn_phase` conjunct, every root would refuse loudly
    /// through this arm rather than silently returning a recipe computed from a
    /// thinned candidate set — which is the designed failure mode and the
    /// reason this is a typed variant instead of a `continue`.
    ///
    /// Typed on
    /// [`BuilderError::MissingArea`](crate::game::driven_families::BuilderError)'s
    /// precedent: an engine change must surface HERE, naming itself, rather
    /// than as a candidate quietly missing from a frozen recipe's population.
    /// Mapping it onto one of the other arms would have the recipe report a
    /// fact about the POSITION when what moved was the engine.
    CandidateDidNotApply(String),
    /// The mover's own view is unavailable.
    MoverViewUnavailable,
    /// The turn-start probe found no placement-shaped move at all.
    NoPlacementCandidates,
    /// Candidates exist and not one of them is a side: the mover can only
    /// DECLINE to place (the empty placement is the whole list — no assistant in
    /// hand, or no placement it can afford), or every candidate it can afford
    /// touches a non-production area or mixes paying and non-paying ones.
    NoEligibleCandidate,
    /// Some candidate is eligible, but none places into production areas that
    /// all export nonzero.
    NoProductiveSide,
    /// No candidate places into production areas that all export zero.
    NoDeadSide,
    /// Both sides exist, but never at the same placement total.
    NoSharedTotal,
}

impl std::fmt::Display for RecipeMiss {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotATurnStart => write!(f, "the root is not a mover turn start"),
            Self::CandidateDidNotApply(detail) => {
                write!(f, "a legal placement did not apply: {detail}")
            }
            Self::MoverViewUnavailable => write!(f, "the mover's own view is unavailable"),
            Self::NoPlacementCandidates => write!(f, "the turn start offers no placement"),
            Self::NoEligibleCandidate => {
                write!(f, "no candidate is a recipe side at all")
            }
            Self::NoProductiveSide => {
                write!(f, "no candidate places into all-nonzero production areas")
            }
            Self::NoDeadSide => write!(f, "no candidate places into all-zero production areas"),
            Self::NoSharedTotal => {
                write!(f, "the two sides exist but never at the same placement total")
            }
        }
    }
}

/// Every placement the fork offers, with the exported production reward it
/// would realize.
///
/// **Two engine calls do all the work, and both are deliberate.**
///
/// * The candidate list is [`turn_start_placements`]' — the SAME probe the
///   frozen §6 predicate selects the position with, so the recipe keys on
///   exactly the population the generator drew rather than on a second opinion
///   about which placements a turn start offers.
/// * Each candidate is APPLIED, and the reward is read on the successor. The
///   placement is what creates the access being priced, so a pre-placement
///   reading would call every empty area dead however many assistants the
///   candidate puts there; and applying is also what makes the Telegram swap,
///   the multi-area spreads and the per-President tolls come out right without
///   this function knowing anything about them.
///
/// The reward itself is [`exported_reward`] at
/// [`realizable_power`](crate::game::eval::realizable_power) — the §13
/// dispatcher and §3.2's shared arithmetic, CALLED rather than restated, which
/// is why this helper can be frozen without freezing a copy of the reward table
/// beside it. It observes the MOVER's own view to do so: every term of the
/// power sum is public, so the seat the view belongs to cannot change the
/// answer, and taking the reading through a view is what keeps the recipe's
/// quantity the same object a seated evaluator would compute.
pub fn classify_recipe_candidates(
    root: &GameState,
    rules: &Rules,
) -> Result<Vec<RecipeCandidate>, RecipeMiss> {
    let placements = turn_start_placements(root, rules).ok_or(RecipeMiss::NotATurnStart)?;
    let mover = root.current_player;

    let mut candidates = Vec::new();
    for mv in &placements {
        let Some((total, areas)) = placement_shape(mv) else {
            continue;
        };
        let placed = apply_move_unchecked(root, mv, rules)
            .map_err(|e| RecipeMiss::CandidateDidNotApply(e.to_string()))?;
        let view = observe(&placed, mover).map_err(|_| RecipeMiss::MoverViewUnavailable)?;
        let player =
            view.players.iter().find(|p| p.id == mover).ok_or(RecipeMiss::MoverViewUnavailable)?;

        // The three-way classification of the WHOLE placed set. `exporting`
        // stays true only while every area is production AND pays; `dead` only
        // while every area is production and pays nothing. An empty set leaves
        // both true, which is why emptiness is refused before they are read.
        let mut sum = 0u32;
        let mut all_exporting = true;
        let mut all_zero = true;
        for area in &areas {
            let area_type = view.board_layout.areas.get(area).map(|config| config.area_type);
            match area_type.and_then(|t| exported_reward(t, realizable_power(&view, player, *area)))
            {
                Some(0) => all_exporting = false,
                Some(reward) => {
                    all_zero = false;
                    sum += u32::from(reward);
                }
                // Outside the four production helpers — whatever the area pays
                // through some other one — or (unreachably, since a placement
                // names a board area) an area the layout does not carry.
                // Neither side, either way.
                None => {
                    all_exporting = false;
                    all_zero = false;
                }
            }
        }

        let side = if areas.is_empty() {
            RecipeSide::Ineligible
        } else if all_exporting {
            RecipeSide::Productive(sum)
        } else if all_zero {
            RecipeSide::Dead
        } else {
            RecipeSide::Ineligible
        };
        candidates.push(RecipeCandidate { mv: mv.clone(), total, areas, side });
    }

    Ok(candidates)
}

/// §6's pair, chosen from classified candidates.
///
/// **The rule, stated once because it is FROZEN**: scan placement totals
/// ASCENDING and take the first total carrying both a productive and a dead
/// candidate; within it take each side's LOWEST area set.
///
/// Both halves are selection, not optimization, and that is the point. The
/// smallest total is the least confounded pair available — fewest assistants
/// moved, smallest fee, smallest collateral change to everything else the
/// escrow read will look at — and a total carrying only one side is SKIPPED
/// rather than refused, so a position offering its pair only at two assistants
/// still carries the read. The within-total tie-break is the area set's own
/// order, which has no value content in it; picking (say) the largest reward
/// instead would be the recipe quietly optimizing the very quantity the read
/// exists to test.
///
/// The refusals are a LATTICE, ordered so the name says the most specific true
/// thing: no candidates at all, then none of them a side, then no side of one
/// kind anywhere, then both kinds but never together. A miss recorded in a
/// step-10 ledger row is then a fact about the position rather than a shrug.
fn select_recipe_pair(
    mover: PlayerId,
    candidates: &[RecipeCandidate],
) -> Result<RecipePair, RecipeMiss> {
    if candidates.is_empty() {
        return Err(RecipeMiss::NoPlacementCandidates);
    }
    let is_productive = |c: &RecipeCandidate| matches!(c.side, RecipeSide::Productive(_));
    let is_dead = |c: &RecipeCandidate| c.side == RecipeSide::Dead;
    if candidates.iter().all(|c| c.side == RecipeSide::Ineligible) {
        return Err(RecipeMiss::NoEligibleCandidate);
    }
    if !candidates.iter().any(is_productive) {
        return Err(RecipeMiss::NoProductiveSide);
    }
    if !candidates.iter().any(is_dead) {
        return Err(RecipeMiss::NoDeadSide);
    }

    let totals: BTreeSet<usize> = candidates.iter().map(|c| c.total).collect();
    for total in totals {
        let at_total = || candidates.iter().filter(move |c| c.total == total);
        // `min_by` keeps the FIRST minimum, so equal area sets fall back to the
        // engine's own enumeration order — deterministic either way.
        let live = at_total().filter(|c| is_productive(c)).min_by(|a, b| a.areas.cmp(&b.areas));
        let gone = at_total().filter(|c| is_dead(c)).min_by(|a, b| a.areas.cmp(&b.areas));
        let (Some(live), Some(gone)) = (live, gone) else {
            continue;
        };
        let RecipeSide::Productive(reward) = live.side else {
            unreachable!("filtered to the productive side");
        };
        return Ok(RecipePair {
            mover,
            total,
            productive: live.mv.clone(),
            productive_areas: live.areas.clone(),
            productive_reward: reward,
            dead: gone.mv.clone(),
            dead_areas: gone.areas.clone(),
        });
    }
    Err(RecipeMiss::NoSharedTotal)
}

/// **THE §6 ESCROW RECIPE** (spec `docs/plans/2026-08-20-phase3.5c-v4-spec-prereg.md`
/// §6, the escrow-read paragraph): from a placement-fork root, the pair of
/// same-count placements that the ENGINE-EXPORTED rules quantity separates —
/// one placing into areas whose exported action reward at realizable power is
/// nonzero, one where it is zero.
///
/// # What this object is, and what it is deliberately not
///
/// **FROZEN as of the commit that lands it**, and INSTANTIATED only at step 10.
/// Nothing here reads a feature column, a weight or an evaluator: §6 requires
/// value-distinctness to be rules-derived BEFORE the read, so this helper is
/// independent of `Feature::ALL` and stays independent of it — a later feature
/// list, a later fit, a later shipping identity all leave the pair it names
/// unmoved. That independence is what lets the step-10 read be a test of the
/// features rather than of itself.
///
/// It fixes the recipe's QUANTITIES only. The direction claim, the collision
/// check and the weighted-preference clause are the READ's, over the pair this
/// returns.
///
/// # Selection is by RULES, tie-broken structurally
///
/// The pair comes from [`classify_recipe_candidates`] and
/// [`select_recipe_pair`]; between them they call the §13 reward dispatcher and
/// §3.2's shared power arithmetic and derive nothing of their own. A root that
/// carries no such pair gets a TYPED [`RecipeMiss`] naming which half is
/// missing — never a nearest-available pair, because a pair whose distinctness
/// nobody established is exactly the thing the recipe exists to rule out.
pub fn escrow_recipe_pair(root: &GameState, rules: &Rules) -> Result<RecipePair, RecipeMiss> {
    let candidates = classify_recipe_candidates(root, rules)?;
    select_recipe_pair(root.current_player, &candidates)
}

// ---------------------------------------------------------------------------
// V4 step 10: the SHAPE-F FEE-FLOW RECIPE (guidingpath 2026-08-23, "THE STEP-10
// RECORDED-BAND RECIPE IS REGISTERED", item (1); the execution ledger's reading
// R-S10-4)
// ---------------------------------------------------------------------------

/// One placement the turn start offers, with the OPPONENT FEE FLOW it prices.
///
/// Carried as DATA on [`RecipeCandidate`]'s precedent: a step-10 row records
/// what the recipe SAW and not only what it chose, so a miss at a reserved
/// ordinal is diagnosable from the same call that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeFlowCandidate {
    /// The placement move itself, as the engine enumerated it.
    pub mv: Move,
    /// How many assistants it places.
    pub total: usize,
    /// The SET of areas it places into.
    pub areas: BTreeSet<AreaId>,
    /// The Yen this placement moves from the mover to the opponent — the
    /// engine's own placement pricing, CROSS-CHECKED against the applied
    /// successor's money deltas before the candidate is admitted (see
    /// [`classify_fee_flow_candidates`]). Zero for a placement that touches no
    /// area the opponent's President stands in.
    pub fee: u32,
}

/// **THE SHAPE-F PAIR**: two same-total placements from one turn start,
/// separated by the OPPONENT FEE FLOW the engine's own pricing exports and by
/// nothing else — `F+` the minimal NONZERO flow at that total, `F0` a
/// same-total ZERO flow.
///
/// The registration's words for the quantity (guidingpath 2026-08-23, item
/// (1)): "Fee flow = the engine's own placement pricing
/// (`calculate_placement_cost`/`credit_placement_fees`, an exact mirror; at 2P
/// payer cost == opponent income) — **the Trading House term is STRUCK**
/// (passive income fires only inside `apply_area_action`, never at placement)."
///
/// # What this object is, and what it is deliberately not
///
/// [`RecipePair`]'s posture, verbatim: it fixes the recipe's QUANTITIES and no
/// more. It carries no feature column, no weight, no evaluator reading and no
/// direction claim. The three things the step-10 read then asks of this pair —
/// the gated raw arm (opponent-E1 differs by EXACTLY the exported fee delta,
/// every other opponent column invariant, X1 recomputed), the collision check,
/// and clause (iii)'s RECORDED weighted preference under its mandatory parity
/// gate — are asked of it by the READ, over an object fixed before any of them
/// could influence it.
///
/// The direction is the RULES': the fee is 1 Yen per assistant placed into an
/// area an opponent President occupies, paid to that President's owner, and at
/// 2P the non-dummy opponent is the only President owner there — so the mover's
/// purse falls by exactly what the opponent's purse rises by. That is why the
/// raw arm can gate a magnitude and not merely a sign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeFlowPair {
    /// The seat whose turn is starting.
    pub mover: PlayerId,
    /// The ONE non-dummy other seat — the fee's payee.
    pub opponent: PlayerId,
    /// The shared placement total both sides carry.
    pub total: usize,
    /// `F+`: the minimal NONZERO opponent fee flow at that total.
    pub fee_side: Move,
    /// That placement's area set.
    pub fee_side_areas: BTreeSet<AreaId>,
    /// `F+`'s priced fee, > 0.
    pub fee: u32,
    /// `F0`: the same-total placement whose opponent fee flow is ZERO.
    pub free_side: Move,
    /// That placement's area set.
    pub free_side_areas: BTreeSet<AreaId>,
}

/// Why a root yields no Shape-F pair.
///
/// **Typed, and the arms are of TWO kinds** — the distinction is the
/// registration's own and the step-10 composition table reads it:
///
/// * The VOID lattice — [`NotATurnStart`](Self::NotATurnStart),
///   [`NotTwoPlayer`](Self::NotTwoPlayer),
///   [`NoOpponentPresidentOnBoard`](Self::NoOpponentPresidentOnBoard),
///   [`NoPlacementCandidates`](Self::NoPlacementCandidates),
///   [`NoFeeSide`](Self::NoFeeSide), [`NoFreeSide`](Self::NoFreeSide),
///   [`NoSharedTotal`](Self::NoSharedTotal) — each a FACT ABOUT THE POSITION,
///   ordered most-specific-first so the name says the most specific true thing.
///   These are the recipe's non-instantiation, which the 2026-08-23 row rules
///   "VOID ≠ miss, the remaining arms still read".
/// * The INSTRUMENT arms — [`CandidateDidNotApply`](Self::CandidateDidNotApply)
///   and [`PricingMismatch`](Self::PricingMismatch) — which are facts about the
///   ENGINE or this helper, never about the position. The composition table
///   routes them to precedence (0): "typed harness/identity failure ⇒ REFUSE
///   classification until resolved, never auto-classified". Mapping either onto
///   a void would have the recipe report a fact about the POSITION when what
///   moved was the code.
///
/// There is no fallback arm anywhere below, on [`RecipeMiss`]'s reasoning: a
/// nearest-available pair is a pair whose fee separation nobody established,
/// and the read riding it would be wrong with no way to notice.
///
/// **`MoverViewUnavailable` is deliberately ABSENT** (R-S10-4 lists it; this
/// helper cannot reach it). Fee flow is ENGINE PRICING — `president_location`,
/// the placed areas, and the two purses — so nothing here calls `observe` and
/// no view can be unavailable to it. Viewability is a SEPARATE step-10
/// determination over the root (R-S10-5: "VIEWABILITY = `observe(root, mover)`
/// succeeds"), made by the read surface, not by the recipe; an arm this module
/// can never construct would be an arm no cell could reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeeFlowMiss {
    /// The root is not a mover turn start.
    NotATurnStart,
    /// The root does not seat exactly one non-dummy opponent.
    ///
    /// The recorded band is 2P and the registered raw arm names ONE seat
    /// ("opponent-E1 differs by EXACTLY the exported fee delta"), so a
    /// three- or four-seat root is refused rather than answered about some
    /// arbitrarily chosen rival: at 3P+ two different opponents can both hold a
    /// President in the placed area (`calculate_placement_cost` sums over the
    /// pairs and `credit_placement_fees` credits each owner), and "the
    /// opponent's fee flow" stops naming one number.
    ///
    /// The count is the seat count. **The 2P dummy is not among them**:
    /// `PlayerId::DUMMY` "appears ONLY as a track-slot owner", and
    /// `place_setup_dummies` touches only the Church faith-7 and Customs
    /// space-4 slots — it holds no President and no purse, so it can neither
    /// charge a fee nor receive one. The `!is_dummy` filter below is therefore
    /// a total statement rather than a live branch.
    NotTwoPlayer { players: usize },
    /// The opponent's President is not on the board.
    ///
    /// **The instantiation condition, NAMED and checked BEFORE classification**
    /// (guidingpath 2026-08-23, item (1): "Instantiation conditions NAMED:
    /// opponent president ON BOARD + a mover assistant held + the fee
    /// affordable"). With the President in hand no placement can pay anything,
    /// so every candidate would price zero and the position would refuse as
    /// [`NoFeeSide`](Self::NoFeeSide) — true, but it would blame the mover's
    /// wallet for the opponent's geometry. Naming this arm makes the void
    /// INFORMATIVE, which is the whole point of a void that the step-10 row
    /// records rather than a shrug.
    ///
    /// It is also the arm the registration expects to see: "Recovery never
    /// recalls the president — the sole recall is the voluntary
    /// `ReturnPresidentToHand` — so organic instantiation is realistic but
    /// UNCERTIFIABLE; no census fact covers president positions; the floor does
    /// real work."
    NoOpponentPresidentOnBoard,
    /// A placement the engine's own legal list offered did not apply.
    ///
    /// **An INSTRUMENT arm, and unreachable today for exactly the three reasons
    /// [`RecipeMiss::CandidateDidNotApply`] spells out** — this helper
    /// enumerates on the post-`EndAdditionalPhase` state and applies to the
    /// ROOT, and the two agree byte for byte because (1) the successor differs
    /// only in `turn_phase`, (2) both placement arms are PHASE-BLIND, and (3)
    /// the queue guard is pre-satisfied by [`is_mover_turn_start`]. **So the
    /// unreachability DEPENDS on fact (2) holding.** If a placement arm ever
    /// grew a `turn_phase` conjunct, every root would refuse loudly here rather
    /// than silently pricing a thinned candidate set.
    CandidateDidNotApply(String),
    /// The priced fee and the APPLIED successor's money deltas disagree.
    ///
    /// **The exact-mirror cross-check's failure arm — an INSTRUMENT failure,
    /// never a void and never a miss** (R-S10-4: "a disagreement between the
    /// two is TYPED"). The registration calls `calculate_placement_cost` and
    /// `credit_placement_fees` "an exact mirror"; this arm is that claim made
    /// executable at every candidate, at every root the read touches.
    ///
    /// # Unreachable while the mirror holds, and here is what that depends on
    ///
    /// `apply_placement` charges `calculate_placement_cost(state, player,
    /// placements)` and then credits `credit_placement_fees` over the SAME
    /// slice, whose owner filter is `calculate_placement_cost`'s pairing
    /// restated; `apply_placement_same_area` does the same over `[area, area]`;
    /// and `PlaceTwoSameArea`'s Telegram leg runs `apply_placement(&[adj])`,
    /// which prices that leg the same way again. Placement moves NOTHING else
    /// through either purse — the Trading House term the registration STRUCK
    /// fires only inside `apply_area_action`. So the payer's deduction, the
    /// opponent's credit and the price below are three readings of one number.
    ///
    /// If a re-pricing ever moved one of them without the others — a
    /// per-President cap, a tech that waives the toll, a fee routed to the bank
    /// — this arm fires by name at the root that shows it, carrying both deltas
    /// and the price, instead of the read quietly gating a magnitude the engine
    /// never charged.
    PricingMismatch {
        /// The candidate, `Debug`-formatted (the type is not `Display`).
        candidate: String,
        /// What [`calculate_placement_cost`] priced it at.
        priced: u32,
        /// What the mover's purse actually fell by.
        payer_delta: i64,
        /// What the opponent's purse actually rose by.
        opponent_delta: i64,
    },
    /// The turn start offers no NON-EMPTY placement at all: the mover can only
    /// decline to place (no assistant in hand, or nothing it can afford).
    NoPlacementCandidates,
    /// No candidate pays the opponent anything.
    ///
    /// The generator has already dropped every placement the mover cannot
    /// afford, so this arm covers BOTH "no placement touches the opponent's
    /// President area" and "the ones that do are unaffordable" — a distinction
    /// the candidate list cannot express, because an unaffordable placement was
    /// never in it. The Yen-less mover and the far-from-the-President mover
    /// therefore record the same void, and that is honest: neither carries the
    /// read.
    NoFeeSide,
    /// Every non-empty candidate pays the opponent, so no zero-fee control
    /// exists at any total.
    NoFreeSide,
    /// Both kinds exist, but never at the same placement total.
    NoSharedTotal,
}

impl std::fmt::Display for FeeFlowMiss {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotATurnStart => write!(f, "the root is not a mover turn start"),
            Self::NotTwoPlayer { players } => {
                write!(f, "the fee-flow recipe is 2P and this root seats {players}")
            }
            Self::NoOpponentPresidentOnBoard => {
                write!(f, "the opponent's president is not on the board")
            }
            Self::CandidateDidNotApply(detail) => {
                write!(f, "a legal placement did not apply: {detail}")
            }
            Self::PricingMismatch { candidate, priced, payer_delta, opponent_delta } => write!(
                f,
                "placement pricing disagrees with the applied successor at {candidate}: \
                 priced {priced}, payer paid {payer_delta}, opponent received {opponent_delta}"
            ),
            Self::NoPlacementCandidates => write!(f, "the turn start offers no placement"),
            Self::NoFeeSide => write!(f, "no affordable placement pays the opponent"),
            Self::NoFreeSide => write!(f, "every placement pays the opponent"),
            Self::NoSharedTotal => {
                write!(f, "the two sides exist but never at the same placement total")
            }
        }
    }
}

/// The ONE non-dummy seat other than `mover`.
///
/// See [`FeeFlowMiss::NotTwoPlayer`] for why one is the only workable count and
/// why the dummy cannot be it.
fn sole_opponent(root: &GameState, mover: PlayerId) -> Result<PlayerId, FeeFlowMiss> {
    let others: Vec<PlayerId> =
        root.players.iter().map(|p| p.id).filter(|id| *id != mover && !id.is_dummy()).collect();
    match others.as_slice() {
        [only] => Ok(*only),
        _ => Err(FeeFlowMiss::NotTwoPlayer { players: root.players.len() }),
    }
}

/// What a placement costs its mover, PER ENCODING, exactly as `apply_move`
/// prices it.
///
/// The two encodings are priced by two different call shapes and the asymmetry
/// is the engine's, not this function's (R-S10-4: "`PlaceThreeAreas` → its
/// list; `PlaceTwoSameArea` → `[area, area]` then the Telegram `[adjacent]`"):
///
/// * `PlaceThreeAreas { placements }` — one assistant per listed area, so the
///   list IS the per-assistant slice `apply_placement` prices.
/// * `PlaceTwoSameArea { area, adjacent_bonus }` — TWO assistants land in
///   `area`, which `apply_placement_same_area` prices as the two-element slice
///   `[area, area]` ("pay 1 Yen to each President's owner PER ASSISTANT
///   PLACED"), and the Telegram leg then runs `apply_placement(&[adj])`, a
///   second, separately-priced call. Summing the two mirrors both the
///   generator's own `cost + adj_cost` affordability test and the applied
///   arm's two charges.
///
/// **Both legs are priced on the ROOT, and that is not an approximation.**
/// `calculate_placement_cost` reads exactly one thing about the other seats —
/// `president_location` — and a placement moves no President: the
/// `PlaceTwoSameArea` arm mutates purses, assistants and `turn_phase`, and
/// `set_president_start_area` RECORDS where the President already is. So
/// pricing the adjacent leg on the root and pricing it on the post-same-area
/// state are the same computation, and the applied cross-check in
/// [`classify_fee_flow_candidates`] is what holds that argument to account
/// rather than leaving it as a comment.
///
/// `None` for anything that is not a placement — the same two encodings
/// [`placement_shape`] admits, so a caller that got a shape always gets a
/// price.
fn placement_fee(root: &GameState, mover: PlayerId, mv: &Move) -> Option<u32> {
    match mv {
        Move::PlaceThreeAreas { placements } => {
            Some(calculate_placement_cost(root, mover, placements))
        }
        Move::PlaceTwoSameArea { area, adjacent_bonus } => Some(
            calculate_placement_cost(root, mover, &[*area, *area])
                + adjacent_bonus
                    .map_or(0, |adj| calculate_placement_cost(root, mover, &[adj])),
        ),
        _ => None,
    }
}

/// Every placement the turn start offers, with the opponent fee flow it prices
/// and the applied cross-check already paid.
///
/// **The ply is crossed by [`turn_start_placements`] — THAT function, not a
/// second walk** (R-S10-4: "crosses the ply through the SAME
/// `turn_start_placements`"). It applies `EndAdditionalPhase` and reads
/// `get_legal_moves`, so this recipe keys on exactly the population the frozen
/// §6 recipe keys on, and a re-derivation of the placement enumeration — the
/// Newspaper 4-area case, the Telegram adjacency, the Canal exclusion, the
/// per-President toll — never exists to drift.
///
/// **The order of the three refusals is the void lattice's own**: not a turn
/// start, then not 2P, then no opponent President on the board — each checked
/// before the work whose answer it would make meaningless.
///
/// **Every candidate is APPLIED and cross-checked.** The fee is priced by
/// [`placement_fee`] and then held against the successor from BOTH sides — the
/// mover's purse must fall by it and the opponent's must rise by it — because
/// the registration's claim is that the two engine functions are "an exact
/// mirror", and a claim a helper depends on is a claim the helper should test
/// at every root it is used on. A disagreement is
/// [`FeeFlowMiss::PricingMismatch`], an instrument failure, never a void.
///
/// **Nothing here reads a candidate column, a weight or an evaluator** — no
/// `observe`, no `Feature`, no `eval` (R-S10-5: "INSTANTIABILITY reads no
/// candidate column ... burn-free by construction"). Determinism follows:
/// the answer is a function of the root and the rules alone.
pub fn classify_fee_flow_candidates(
    root: &GameState,
    rules: &Rules,
) -> Result<Vec<FeeFlowCandidate>, FeeFlowMiss> {
    let placements = turn_start_placements(root, rules).ok_or(FeeFlowMiss::NotATurnStart)?;
    let mover = root.current_player;
    let opponent = sole_opponent(root, mover)?;
    if !matches!(root.player(opponent).president_location, PresidentLocation::OnBoard(_)) {
        return Err(FeeFlowMiss::NoOpponentPresidentOnBoard);
    }

    let mut candidates = Vec::new();
    for mv in &placements {
        let Some((total, areas)) = placement_shape(mv) else {
            continue;
        };
        // Unreachable after the shape matched: both functions admit the same
        // two encodings, so a shaped move is always a priced one. Skipping
        // rather than unwrapping keeps that a structural fact instead of a
        // panic waiting for a third placement encoding.
        let Some(fee) = placement_fee(root, mover, mv) else {
            continue;
        };
        let after = apply_move_unchecked(root, mv, rules)
            .map_err(|e| FeeFlowMiss::CandidateDidNotApply(e.to_string()))?;
        let payer_delta =
            i64::from(root.player(mover).money) - i64::from(after.player(mover).money);
        let opponent_delta =
            i64::from(after.player(opponent).money) - i64::from(root.player(opponent).money);
        if payer_delta != i64::from(fee) || opponent_delta != i64::from(fee) {
            return Err(FeeFlowMiss::PricingMismatch {
                candidate: format!("{mv:?}"),
                priced: fee,
                payer_delta,
                opponent_delta,
            });
        }
        candidates.push(FeeFlowCandidate { mv: mv.clone(), total, areas, fee });
    }

    Ok(candidates)
}

/// The Shape-F pair, chosen from priced candidates.
///
/// **The rule is [`select_recipe_pair`]'s, VERBATIM** (guidingpath 2026-08-23,
/// item (1): "scan totals ASCENDING, first total carrying both sides, ties by
/// lowest area set — the `select_recipe_pair` convention VERBATIM, never
/// maximal-fee (selecting by maximizing the read quantity would be the recipe
/// optimizing the very quantity the read exists to test)"). So:
///
/// * totals ASCENDING, first total carrying BOTH a paying and a free
///   candidate; a total carrying only one side is SKIPPED, not refused;
/// * `F+` = the MINIMAL nonzero fee at that total, ties broken by the lowest
///   area set — never the largest fee, which is exactly the optimization the
///   registration forbids;
/// * `F0` = the lowest area set among that total's zero-fee candidates.
///
/// `min_by` keeps the FIRST minimum, so a tie in both keys falls back to the
/// engine's own enumeration order — deterministic either way.
///
/// **The EMPTY placement is ineligible for either side.**
/// `PlaceThreeAreas { placements: vec![] }` is always in the list — it is how a
/// mover declines to place — and it prices at zero for the arithmetic reason
/// that it places nothing, not because it avoids the opponent's President.
/// Admitting it as `F0` would pair a placement against a NON-placement and call
/// the difference a fee-flow distinction, which is
/// `the_empty_placement_is_never_a_side_of_the_recipe`'s objection at the §6
/// recipe and is the same objection here. It is filtered by total and by
/// emptiness together: two readings of one fact, and neither alone is a
/// definition of "declines to place" a future encoding could not slip past.
fn select_fee_flow_pair(
    mover: PlayerId,
    opponent: PlayerId,
    candidates: &[FeeFlowCandidate],
) -> Result<FeeFlowPair, FeeFlowMiss> {
    let placing: Vec<&FeeFlowCandidate> =
        candidates.iter().filter(|c| c.total > 0 && !c.areas.is_empty()).collect();
    if placing.is_empty() {
        return Err(FeeFlowMiss::NoPlacementCandidates);
    }
    if !placing.iter().any(|c| c.fee > 0) {
        return Err(FeeFlowMiss::NoFeeSide);
    }
    if !placing.iter().any(|c| c.fee == 0) {
        return Err(FeeFlowMiss::NoFreeSide);
    }

    let totals: BTreeSet<usize> = placing.iter().map(|c| c.total).collect();
    for total in totals {
        let at_total = || placing.iter().filter(move |c| c.total == total);
        let paying = at_total()
            .filter(|c| c.fee > 0)
            .min_by(|a, b| (a.fee, &a.areas).cmp(&(b.fee, &b.areas)));
        let free = at_total().filter(|c| c.fee == 0).min_by(|a, b| a.areas.cmp(&b.areas));
        let (Some(paying), Some(free)) = (paying, free) else {
            continue;
        };
        return Ok(FeeFlowPair {
            mover,
            opponent,
            total,
            fee_side: paying.mv.clone(),
            fee_side_areas: paying.areas.clone(),
            fee: paying.fee,
            free_side: free.mv.clone(),
            free_side_areas: free.areas.clone(),
        });
    }
    Err(FeeFlowMiss::NoSharedTotal)
}

/// **THE SHAPE-F FEE-FLOW RECIPE** (guidingpath 2026-08-23, "THE STEP-10
/// RECORDED-BAND RECIPE IS REGISTERED — CHOSEN, NOT FORCED", item (1); the
/// execution ledger's R-S10-4): from a turn-start root, the pair of same-total
/// placements the ENGINE'S OWN placement pricing separates — one exporting a
/// minimal nonzero fee flow to the opponent, one exporting none.
///
/// # Why a SECOND copy of the §6 machinery, and not a shared one
///
/// The §6 recipe ([`escrow_recipe_pair`] and its two halves) is a FROZEN
/// artifact: its digests are pinned and its band is escrowed, so factoring a
/// shared selector out of it would move frozen bytes to serve a helper written
/// two steps later. The structure below is therefore MIRRORED — the same ply
/// crossing (which IS shared: [`turn_start_placements`] is called, never
/// re-walked), the same [`placement_shape`] reading (also shared), the same
/// classify/select split, the same ascending-totals convention, the same
/// lowest-area-set tie-break, the same lattice discipline — while the selection
/// body is a second copy. The copy is ACCEPTED and named here so a later reader
/// does not mistake it for drift: what the two recipes share is called, what
/// would have required editing the frozen artifact is duplicated.
///
/// # Independence
///
/// Like §6's, this recipe reads no feature column, no weight and no evaluator,
/// and unlike §6's it reads no VIEW either — fee flow is engine pricing over
/// `president_location` and the two purses. That is R-S10-5's "burn-free by
/// construction": instantiability at a reserved ordinal can be decided without
/// any candidate column being computed, so deciding it cannot burn the ordinal.
///
/// # What rides on the pair, and what does not
///
/// The registration's item (2) gates the RAW arm over these two successors
/// (opponent-E1 differs by exactly the exported fee delta; the rest of the
/// opponent's columns exactly invariant; X1 recomputed at the successor purse),
/// and its item (3) RECORDS clause (iii)'s weighted preference under a
/// mandatory production-vs-probe parity gate. Neither is this function's
/// business. It fixes the pair, and it fixes it out of quantities that were
/// settled before any of those reads existed.
pub fn fee_flow_recipe_pair(root: &GameState, rules: &Rules) -> Result<FeeFlowPair, FeeFlowMiss> {
    let candidates = classify_fee_flow_candidates(root, rules)?;
    let mover = root.current_player;
    // Re-derived rather than threaded out of the classifier: it is two seat-list
    // comparisons, and keeping `classify_fee_flow_candidates`' return type the
    // candidate list ALONE is what lets a caller record what the recipe saw
    // without also being handed a decision it did not ask for.
    let opponent = sole_opponent(root, mover)?;
    select_fee_flow_pair(mover, opponent, &candidates)
}

// ---------------------------------------------------------------------------
// Phase C layer 2: the shared conjuncts (design §1 families 1–2, §3, §4)
// ---------------------------------------------------------------------------

/// The design §4 CANDIDATE-WIDTH bound: the widest position a layer-2 family
/// will draw as a root.
///
/// A RECORDED GENERATOR CONSTRAINT, never a post-hoc filter — it is a CONJUNCT
/// of the predicates themselves, so a 139-wide placement position was never in
/// the ordinal space rather than dropped after being looked at. The reason is
/// compute: families 2–4 sit mid/late game where one adjudication playout is
/// about a half game, and the admission adjudication forces EVERY root
/// candidate under the whole reply menu. The families' questions do not need a
/// wide placement root.
pub const MAX_ROOT_CANDIDATES: usize = 60;

/// The design §1 family-2 band: how far AHEAD (or BEHIND) in truth a mover must
/// be for the end-trigger question to be the one canon 15 asks.
///
/// +10 VP ≈ one majority bonus ≈ "one good turn of value" — C-A's own
/// calibration, adopted so the family's roots are positions where the answer is
/// not a coin flip. It is a SELECTION parameter and nothing evaluates against
/// it: terminal-margin awareness stays off the evaluator (the STOP fence).
pub const TRUE_MARGIN_BAND: i64 = 10;

/// Whether this position is narrow enough to be a layer-2 root.
fn width_ok(state: &GameState, rules: &Rules) -> bool {
    width_ok_len(get_legal_moves(state, rules).len())
}

/// [`width_ok`]'s arithmetic, over a legal-list length the caller already holds.
///
/// The bound is spelled ONCE, here: two of the layer-2 arms need the legal list
/// anyway (they probe it), so they call this rather than paying a second
/// enumeration, and one spelling is what stops the two paths drifting to
/// different widths.
const fn width_ok_len(len: usize) -> bool {
    len <= MAX_ROOT_CANDIDATES
}

/// The mover's TRUE margin at this position: its final-score total minus the
/// best-ranked rival's, scored AS IF the game ended now.
///
/// **Full-state truth, hidden cards included** — a SELECTION-side helper
/// (design §1 family 2). It exists so family 2's generator can supply the
/// arithmetic canon 15 is about instead of testing whether the subject can
/// reconstruct it; nothing in the evaluator may read it.
///
/// "Best rival" is the highest-RANKED other seat in [`final_result`]'s official
/// standings, not the highest raw total: the standings already carry the
/// clockwise tie-break, so on level totals the rival is the one the rulebook
/// says beats the mover. **Margin 0 is therefore NOT a win** — it is exactly the
/// case where the ranking decides — and the pins say so.
///
/// Both lookups are total on any state the engine can produce (`current_player`
/// indexes `state.players`, and a supported game has 2–4 seats); a state that
/// fails either is corrupt in a way `check_invariants` already refuses, so the
/// panic names the corruption rather than a wrong margin being returned quietly.
pub fn true_margin_now(state: &GameState, rules: &Rules) -> i64 {
    let result = final_result(state, rules);
    let mover = state.current_player;
    let mover_total = result
        .standings
        .iter()
        .find(|s| s.player == mover)
        .expect("the mover is one of the state's own seats")
        .breakdown
        .total as i64;
    let rival_total = result
        .standings
        .iter()
        .find(|s| s.player != mover)
        .expect("a supported game has at least two seats")
        .breakdown
        .total as i64;
    mover_total - rival_total
}

/// How many Trading Houses complete the set —
/// `end_game::check_end_game_triggers`' own `>= 4`, restated where the
/// precondition needs it.
const TRADING_HOUSES_COMPLETE: usize = 4;

/// The same for Shophouses — that function's `>= 8`.
const SHOPHOUSES_COMPLETE: usize = 8;

/// The most Order-card slots ONE applied move can leave needing a refill.
///
/// `cards::refill_port` pops once per EMPTY slot of the ONE port board an action
/// refills — `actions/cards.rs`'s single refill site, reached once per applied
/// move — and a port board has six slots, so six is the structural bound and no
/// other code path takes a card off `order_deck` at all (verified by grep over
/// `game/`: setup builds the deck, this refill is the only consumer).
const MAX_ORDER_DRAWS_PER_MOVE: usize = 6;

/// The first [`EndGameTrigger`] kind; [`trigger_kind_after`] walks the rest.
///
/// The `PlayerId` is a PLACEHOLDER — this enumerates trigger KINDS, and every
/// precondition arm below ignores the player component.
const FIRST_TRIGGER_KIND: EndGameTrigger = EndGameTrigger::TradingHousesComplete(PlayerId::DUMMY);

/// The next [`EndGameTrigger`] kind after `trigger`, in
/// `check_end_game_triggers`' own order — `None` after the last.
///
/// The enumeration is an exhaustive `match` rather than a hand-written array so
/// that a SIXTH variant is a COMPILE ERROR here instead of a silently narrower
/// precondition set. That matters more than it looks: a precondition list that
/// quietly missed a trigger would bias every layer-2 scan — the family-2
/// generator would simply never see the positions that trigger fires from — and
/// nothing would fail.
fn trigger_kind_after(trigger: &EndGameTrigger) -> Option<EndGameTrigger> {
    match trigger {
        EndGameTrigger::TradingHousesComplete(_) => {
            Some(EndGameTrigger::ShophousesComplete(PlayerId::DUMMY))
        }
        EndGameTrigger::ShophousesComplete(_) => Some(EndGameTrigger::OrderDeckEmpty),
        EndGameTrigger::OrderDeckEmpty => Some(EndGameTrigger::ChurchTrackFull),
        EndGameTrigger::ChurchTrackFull => Some(EndGameTrigger::CustomsTrackFull),
        EndGameTrigger::CustomsTrackFull => None,
    }
}

/// Whether ONE applied move could make `trigger`'s own firing condition true.
///
/// Every arm mirrors `end_game::check_end_game_triggers`' condition with the
/// largest single-move delta subtracted, and each delta is a verified fact about
/// the engine's moves rather than an estimate:
/// - a build move places exactly ONE building (`Move::BuildShophouse` /
///   `Move::BuildTradingHouse` are one token each), so a player at
///   `COMPLETE - 1` is the one-move-away case;
/// - a Church/Customs placement writes exactly ONE track slot
///   (`place_church_assistant` / `place_customs_assistant` are the only two
///   writers, one slot per call), so `total + 1 >= track_trigger()` is exact;
/// - the order deck's delta is [`MAX_ORDER_DRAWS_PER_MOVE`], which is a bound
///   rather than an exact figure and is deliberately loose.
///
/// The direction that matters is that this NEVER says "no" where a firing move
/// exists — an over-eager arm costs a wasted apply-probe, an under-eager one
/// silently biases a scan. The soundness pins exercise both a clock shape and a
/// building shape through the short circuit.
fn trigger_is_within_one_move(trigger: &EndGameTrigger, state: &GameState) -> bool {
    match trigger {
        EndGameTrigger::TradingHousesComplete(_) => state
            .players
            .iter()
            .any(|p| p.trading_houses.total_built() + 1 >= TRADING_HOUSES_COMPLETE),
        EndGameTrigger::ShophousesComplete(_) => {
            state.players.iter().any(|p| p.shophouses.total_built() + 1 >= SHOPHOUSES_COMPLETE)
        }
        EndGameTrigger::OrderDeckEmpty => {
            // BOTH halves of that trigger's condition, one move ahead. It needs
            // an empty deck AND an empty port slot afterwards; a slot is empty
            // afterwards only if one is empty NOW, or if the move empties slots
            // the deck cannot refill — which needs the deck strictly shorter
            // than the slots one action can empty. Without the port half this
            // arm is true from move one at 2 players, where the whole deck is
            // ten cards after the setup removal, and the short circuit stops
            // circuiting exactly where the layer-2 scans run.
            port_has_empty_slot(state) || state.order_deck.len() < MAX_ORDER_DRAWS_PER_MOVE
        }
        EndGameTrigger::ChurchTrackFull => {
            state.church_track.total_assistants() + 1 >= state.config.track_trigger()
        }
        EndGameTrigger::CustomsTrackFull => {
            state.customs_track.total_assistants() + 1 >= state.config.track_trigger()
        }
    }
}

/// `end_game::check_port_empty_slot`'s condition, restated (that helper is
/// private to its module): does any port board hold an empty slot right now.
fn port_has_empty_slot(state: &GameState) -> bool {
    state.port_state.port_a.has_empty_slot()
        || state.port_state.port_b.as_ref().is_some_and(|b| b.has_empty_slot())
}

/// Whether ANY end-game trigger is within one applied move of firing — the
/// short circuit [`firing_moves`] pays before it clones anything.
fn any_trigger_within_one_move(state: &GameState) -> bool {
    let mut kind = Some(FIRST_TRIGGER_KIND);
    while let Some(k) = kind {
        if trigger_is_within_one_move(&k, state) {
            return true;
        }
        kind = trigger_kind_after(&k);
    }
    false
}

/// The legal moves whose application SETS `end_game_triggered` — family 2's
/// "the mover can end the game this move", as data.
///
/// **Transition semantics, and the word "sets" is load-bearing**: on a state
/// where the trigger has ALREADY fired this is EMPTY, because every applied
/// state inherits the flag and a bare "the result has it true" test would answer
/// "every legal move" at exactly the positions where the question is
/// meaningless.
///
/// **The short circuit is the whole reason this is affordable.** Probing every
/// legal move at every position of every generated game is ~10^5 `GameState`
/// clones per game; [`any_trigger_within_one_move`] rejects the overwhelming
/// majority of positions on integer reads alone, and it is derived from an
/// exhaustive match over [`EndGameTrigger`] so it cannot silently miss a trigger
/// kind. What it must never do is skip a position where a firing move exists —
/// that is the property its pins are about, not speed.
pub(crate) fn firing_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    if state.end_game_triggered {
        return Vec::new();
    }
    if !any_trigger_within_one_move(state) {
        return Vec::new();
    }
    get_legal_moves(state, rules)
        .into_iter()
        .filter(|mv| {
            apply_move_unchecked(state, mv, rules)
                .map(|next| next.end_game_triggered)
                .unwrap_or(false)
        })
        .collect()
}

/// One CONTESTED track step — the race currency family 3's predicate is written
/// in.
///
/// The read instrument's own declaration type (`driven_families::ContestedStep`,
/// T4) is a WIDER vocabulary that also names the tech-majority arm; this one is
/// deliberately narrower, because it is what the GENERATED predicate can decide
/// exactly (see [`opponent_race_steps`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RaceStep {
    /// A Church faith space, 4..=11.
    Church(u8),
    /// A Customs space, 1..=7.
    Customs(u8),
}

/// The one area of `wanted` type, if the board has one.
fn area_of_type(state: &GameState, wanted: AreaType) -> Option<AreaId> {
    state.board_layout.areas.iter().find(|(_, c)| c.area_type == wanted).map(|(a, _)| *a)
}

/// The track steps an OPPONENT is ONE COMPLETED ACTION away from taking.
///
/// **The arithmetic, exactly** (design §1 family 3; the plan's "write the exact
/// state arithmetic in-source"). For each seat other than `state.current_player`
/// and each of the two track areas:
/// 1. the opponent's President is NOT already on that area — a President may not
///    return to the area it starts its turn on (rulebook p.11), so an opponent
///    standing there cannot take that area's action next turn;
/// 2. the opponent has at least one ASSISTANT on that area — the durable claim.
///    This deliberately does NOT call `is_possible_action_area_full`: that
///    predicate answers "can this player act here NOW", and it counts every
///    other President as a block, but the race question is about NEXT turn, by
///    which time the mover's President will have moved (it moves every turn, and
///    at an area-action root it is standing on the very area being contested —
///    so the strict predicate would make this whole family structurally empty);
/// 3. the power the opponent would have on arrival —
///    `(calculate_power + 1).min(5)`, the +1 being the President that must
///    travel there — is enough, together with the goods the opponent actually
///    holds, to reach the step:
///    - CHURCH: `calculate_church_faith(power, …)` at every payment type the
///      opponent can afford (yen counts 2 Yen, ruling D1) gives its maximum
///      Faith, and `eligible_church_spaces` turns that into the unoccupied
///      spaces it could place on;
///    - CUSTOMS: `customs_max_imported_goods(power)` capped by the imported
///      goods held gives the largest printed cost it can pay exactly, and
///      `eligible_customs_spaces_for_cost` turns each affordable cost into
///      unoccupied spaces;
/// 4. an assistant is available for the placement at all
///    (`has_church_or_customs_assistant`, the apply-side guard restated).
///
/// **PRECISION over recall, as the plan directs.** Reachability of the area (a
/// walk the opponent can afford, fees included) and the mover's own turn — which
/// happens BEFORE the opponent's and can take goods, spaces or position away —
/// are NOT modelled, and both omissions can only make this predicate say YES
/// where a full turn simulation would say no. That is the recall/precision trade
/// stated honestly: the set is a superset in exactly those two ways and a strict
/// subset in the president-blocking and payment-arithmetic ways above, and it is
/// the BUILDER arm (T5), not this probe, that family 3 rests on.
///
/// **Deliberately ABSENT: the TECH-MAJORITY arm.** `ContestedStep` names three
/// race steps and this enumerates two. A tech-majority step ("the opponent's one
/// affordable lab purchase would rank it past the mover") needs the lab
/// affordability model — board-position surcharge, gear/yen/imported split,
/// `validate_lab_action`'s payment search — and an approximate version of it
/// would favour RECALL against the plan's explicit instruction. The tech arm
/// lives in the builder's declaration vocabulary instead.
fn opponent_race_steps(state: &GameState) -> Vec<RaceStep> {
    let mover = state.current_player;
    let church_area = area_of_type(state, AreaType::Church);
    let customs_area = area_of_type(state, AreaType::Customs);
    let mut steps: Vec<RaceStep> = Vec::new();

    for p in &state.players {
        if p.id == mover {
            continue;
        }
        if let Some(area) = church_area {
            if opponent_could_act_next_turn(p, area) && has_church_or_customs_assistant(p, area) {
                let power = arriving_power(state, p.id, area);
                let max_faith = calculate_church_faith(
                    power,
                    p.copper >= 1,
                    p.silk >= 1,
                    p.tea >= 1,
                    p.seafood >= 1,
                    // D1 (2026-08-08): the yen payment type costs 2 Yen.
                    p.money >= 2,
                    p.imported_goods >= 1,
                );
                for space in eligible_church_spaces(&state.church_track, max_faith) {
                    steps.push(RaceStep::Church(space));
                }
            }
        }
        if let Some(area) = customs_area {
            if opponent_could_act_next_turn(p, area) && has_church_or_customs_assistant(p, area) {
                let power = arriving_power(state, p.id, area);
                let payable = customs_max_imported_goods(power).min(
                    u8::try_from(p.imported_goods).unwrap_or(u8::MAX),
                );
                for cost in 1..=payable {
                    for space in eligible_customs_spaces_for_cost(&state.customs_track, cost) {
                        steps.push(RaceStep::Customs(space));
                    }
                }
            }
        }
    }

    steps.sort_unstable();
    steps.dedup();
    steps
}

/// Clause 1+2 of [`opponent_race_steps`]: the President is elsewhere (so it can
/// travel to `area` next turn) and the assistants that give it power are there.
fn opponent_could_act_next_turn(p: &crate::types::PlayerState, area: AreaId) -> bool {
    p.president_location != PresidentLocation::OnBoard(area) && p.assistants.in_area(area) >= 1
}

/// The power `player` would have at `area` with its President standing there —
/// `calculate_power`'s figure plus the President that has to travel, re-capped.
///
/// Adding after the cap and re-capping is correct because the cap is idempotent:
/// a raw total already above 5 reads 5 either way.
fn arriving_power(state: &GameState, player: PlayerId, area: AreaId) -> u8 {
    (calculate_power(state, player, area) + 1).min(5)
}

/// The track steps the MOVER would hold — or would have QUEUED the choice for —
/// after ONE of `legal`, by apply-probe.
///
/// **QUEUE-AWARE, and that is not an optimisation.** A Church or Customs action
/// with two or more eligible spaces places NOTHING at apply time: it queues a
/// `PendingChoice::ChurchSpace` / `CustomsSpace` and the pawn lands when the
/// choice resolves (the choice-encoding shape). An occupancy-only classifier
/// would therefore be blind to exactly the race-INITIATING moves this predicate
/// exists to find, and would see only the already-narrowed singleton case.
///
/// The queued arm is credited to the mover only while `current_player` still IS
/// the mover in the probed clone — a queue belongs to the seat that will resolve
/// it — and it credits every space the queued choice could still reach, since
/// which one it takes is the mover's decision and not yet made.
fn mover_reachable_steps(state: &GameState, legal: &[Move], rules: &Rules) -> Vec<RaceStep> {
    let mover = state.current_player;
    let mut steps: Vec<RaceStep> = Vec::new();

    for mv in legal {
        let Ok(next) = apply_move_unchecked(state, mv, rules) else {
            continue;
        };
        for (index, slot) in next.church_track.slots.iter().enumerate() {
            if state.church_track.slots[index].is_none()
                && slot.is_some_and(|s| s.player == mover)
            {
                steps.push(RaceStep::Church(ChurchTrack::index_to_faith(index)));
            }
        }
        for (index, slot) in next.customs_track.slots.iter().enumerate() {
            if state.customs_track.slots[index].is_none()
                && slot.is_some_and(|s| s.player == mover)
            {
                steps.push(RaceStep::Customs(CustomsTrack::index_to_space(index)));
            }
        }
        if next.current_player != mover {
            continue;
        }
        for choice in &next.pending_choices {
            match choice {
                PendingChoice::ChurchSpace { total_faith } => {
                    for space in eligible_church_spaces(&next.church_track, *total_faith) {
                        steps.push(RaceStep::Church(space));
                    }
                }
                PendingChoice::CustomsSpace { cost } => {
                    for space in eligible_customs_spaces_for_cost(&next.customs_track, *cost) {
                        steps.push(RaceStep::Customs(space));
                    }
                }
                _ => {}
            }
        }
    }

    steps.sort_unstable();
    steps.dedup();
    steps
}

/// The v1 generating configuration's NAME. Bump the version in the same commit
/// as ANY change to a [`GeneratingEcology::v1`] field.
///
/// **AMENDED to `v1.1` in Phase C** (design §2/§4): the ecology gained the
/// per-game STRIDE CAP and its `games` bound was raised 4 → 16 to cover the
/// families' block arithmetic. Both are identity, so both ride one version bump.
/// No committed artifact holds a serialized `Generated` triple, so nothing
/// recorded is re-pointed by this (verified by grep at the amending commit) —
/// the bump exists so that a triple quoted from a session NOTE cannot be
/// silently re-resolved.
pub const GENERATED_ECOLOGY_V1: &str = "driven-generated-v1.1";

/// How many games a [`RootSource::Generated`] scan may play before it reports
/// the supply exhausted.
///
/// A BOUND, not a budget: the scan must terminate on an ordinal nothing
/// satisfies, and the honest way to say that is a refusal naming how far it
/// looked. MEASURED at the PREVIOUS bound of 4 (the exhaustion cell's own
/// master, 2026-08-18, WSL debug): **54 final-round positions across the four
/// games, ~14 per game**, at 0.30s for the whole four-game scan.
///
/// **RAISED 4 → 16 in Phase C** for the block arithmetic: at the stride cap of
/// 2 hits per game, one family-1 predicate demands pilot 4 + confirmatory 6 +
/// escrow 6 = 16 strided ordinals, and 4 games × cap 2 = 8 could strand a frozen
/// confirmatory block on `Exhausted`; 16 × 2 = 32 covers the demand plus
/// dead-root slack. Raising the bound at a FIXED master and cap is
/// PREFIX-STABLE — the strided ordinal sequence is identical, merely longer — so
/// this is an extension and not a re-draw.
///
/// **RE-VERIFIED AT THE FREEZE WAVE and left at 16.** The baked family-1 blocks
/// are wider than that draft — pilot `0..4` plus confirmatory `4..20` is **20**
/// strided ordinals per predicate, not 16 — and both of family 1's arms
/// (`FinalRoundAlreadyTriggered` and its `TriggerRoundNotFinal` control) scan
/// this ecology, each in its OWN ordinal space. 16 × 2 = 32 still covers 20 with
/// 12 ordinals of slack for the escrow the freeze DOCUMENT holds, so no bound
/// moved. The check is executable, not a comment:
/// `driven_runner::every_frozen_block_fits_the_supply_its_generator_declares`.
pub const GENERATED_SCAN_GAMES: u32 = 16;

/// How many hits per generated game may enter the ordinal sequence.
///
/// **The STRIDE** (design §4, review MUST-6). Consecutive scan ordinals are NOT
/// independent draws: family 1's supply is ~14 hits per game, which are the
/// successive decision points of ONE final round, so an unstrided block of 6
/// ordinals would quote "six roots" that are near-duplicate positions of at most
/// two games. Hits past the cap are SKIPPED and consume no ordinal, so the
/// ordinal space is defined over strided hits and ordinal `cap` necessarily
/// comes from a later game than ordinal 0.
///
/// It is an ECOLOGY FIELD and therefore IDENTITY — see
/// [`GeneratingEcology::per_game_cap`].
pub const GENERATED_STRIDE_CAP: u32 = 2;

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

/// The v2 generating configuration's NAME — Phase C's layer-2 ecology. Bump the
/// version in the same commit as ANY change to a
/// [`GeneratingEcology::v2_lookahead`] field, exactly as
/// [`GENERATED_ECOLOGY_V1`]'s rule says.
///
/// **EXTENDED to `v2.1` (2026-08-18)**: the `games` bound rose 24 → 120 when
/// family 2's confirmatory blocks exhausted the supply the predicates REALIZE
/// (the measured rates are on [`GENERATED_V2_SCAN_GAMES`]). The bound is an
/// ecology field and therefore identity, so the raise rides a version bump. The
/// bump is a NAMING obligation, not a re-pointing: raising `games` at a FIXED
/// master and cap is PREFIX-STABLE, so every ordinal the old bound could supply
/// resolves to exactly the state it resolved to before — the strided sequence at
/// 120 games is the sequence at 24 with more on the end. The version moves so
/// that a triple quoted from a session note cannot be read as an answer about a
/// different scan than the one that produced it.
///
/// **EXTENDED AGAIN to `v2.2` (2026-08-18)**: 120 → 240, when the same family's
/// CONTROL arm exhausted a SECOND time — `EndFirableBehindInTruth` realized 26
/// strided hits in 120 games against the 28 its block demands, a measured
/// 0.217/game where the 24-game pilot had read 0.29. The recovery is the SAME
/// one, taken the same way and for the same reason: a prefix-stable extension at
/// a fixed master and cap, version-bumped because the bound is identity. The
/// strided sequence at 240 games is the sequence at 120 with more on the end,
/// which is the sequence at 24 with more on the end — so a triple recorded under
/// `v2` or `v2.1` still resolves to exactly the state it resolved to then, and
/// the version moves only so that a quoted reading names the scan that produced
/// it.
///
/// **RE-PINNED to `v2.3` (V4 step 8, the `Feature::ALL` adoption commit).**
/// The generating SEATS moved from the live shipped contestant to the PINNED
/// explicit-V2 construction ([`build_pinned_v2_seat`]) — a seat-field change,
/// so the id bumps per this const's own any-field-change rule. The bump is
/// again a NAMING obligation, not a re-pointing: pinned-V2 is the padded live
/// default byte-for-byte at the adoption commit (the walk pin proves the
/// decisions did not move), so every `v2`/`v2.1`/`v2.2` triple still resolves
/// to exactly the state it resolved to — and, from this commit on, KEEPS
/// resolving there through every later controller re-point, which is what the
/// pin buys.
pub const GENERATED_ECOLOGY_V2_LOOKAHEAD: &str = "driven-generated-v2.3-lookahead";

/// The v2 ecology's scan bound.
///
/// Sized by the SAME arithmetic as v1's (pilot 4 + confirmatory 6 + escrow 6 per
/// predicate, at cap 2) with slack, and with NO supply guarantee: the layer-2
/// predicates are selective by design, and an `Exhausted` refusal at this bound
/// is RECORDED EVIDENCE about the family rather than a failure of the scan
/// (design §1 family 3 says so in as many words). A games raise at a fixed
/// master and cap is prefix-stable, which is the recovery if a frozen block runs
/// short.
///
/// **RE-VERIFIED AT THE FREEZE WAVE and left at 24.** The baked layer-2 blocks
/// demand pilot `0..4` plus confirmatory `4..28` = **28** strided ordinals per
/// predicate, against a supply of 24 × 2 = 48 — 20 ordinals of slack, and the
/// three predicates that scan here (`EndFirableAheadInTruth`, its
/// `EndFirableBehindInTruth` control, and `AchievementClaimAvailable`) each get
/// their own space at their own master. No bound moved; the check is executable
/// in `driven_runner::every_frozen_block_fits_the_supply_its_generator_declares`.
/// The no-supply-guarantee sentence above still stands — a bound that FITS is
/// not a promise that the predicate hits that often.
///
/// **RAISED 24 → 120 (2026-08-18) — that last sentence, collected.** Family 2's
/// confirmatory blocks (`4..28`, both arms) EXHAUSTED at the old bound: the
/// positive predicate `EndFirableAheadInTruth` yielded **10** strided hits in 24
/// games and its `EndFirableBehindInTruth` control **7**, against the 28 each
/// block demands. The capacity arithmetic was never the binding constraint —
/// 24 × 2 = 48 covered 28 on paper the whole time — and the distinction is what
/// this raise records: `games × per_game_cap` is a CEILING on the ordinal space,
/// while the supply a predicate REALIZES inside it is a fact about the predicate
/// that only a scan can measure. At the measured rates, 120 games yield ≈50 and
/// ≈35 strided hits, so ordinal 27 is reached with slack on both arms, and
/// `AchievementClaimAvailable` gains the same headroom without being re-sized
/// separately.
///
/// **RAISED AGAIN 120 → 240 (2026-08-18) — and the ≈35 projection above is why.**
/// The CONTROL arm exhausted a second time: `EndFirableBehindInTruth` realized
/// **26** strided hits in 120 games (typed refusal `draw 26: no root 26 for
/// end-firable-behind-in-truth: 26 found in 120 generated games`) against the 28
/// its block demands — two short. The 24-game pilot's 7 hits had implied
/// 0.29/game; 120 games measure **0.217/game**, so the projection extrapolated a
/// rate the longer scan did not sustain, and the correction is to size off the
/// LONGER measurement rather than the pilot. At 0.217/game, 240 games yield ≈52
/// strided hits against a demand of 28 — 1.8× the demand on the arm that has
/// actually been measured twice, which is what makes this raise the last one
/// sizing is expected to need. The positive arm and `AchievementClaimAvailable`,
/// both denser than this control, gain the same headroom without being re-sized
/// separately.
///
/// **The raise is an EXTENSION, never a re-draw** (design §4 and freeze §4
/// pre-declare exactly this recovery): at a FIXED master and cap the scan walks
/// the same games in the same draw order and strides them the same way, so the
/// strided ordinal sequence is PREFIX-STABLE — identical, merely longer. Every
/// reading already taken at this ecology stands at the ordinal it was taken at;
/// the frozen blocks, their masters, the cap and the seed derivation are all
/// untouched. Only the version string moves ([`GENERATED_ECOLOGY_V2_LOOKAHEAD`],
/// now `v2.2`), because the bound is identity. Both raises are that same shape,
/// so `v2` ⊂ `v2.1` ⊂ `v2.2` as strided sequences.
///
/// It stays a BOUND and not a budget: [`generate_roots`] stops the moment the
/// highest requested ordinal is collected, so a block that was already served
/// inside 24 or 120 games costs after these raises exactly what it cost before
/// them.
///
/// The block-fits-supply check remains executable in
/// `driven_runner::every_frozen_block_fits_the_supply_its_generator_declares`,
/// now reading a capacity of 240 × 2 = **480** against the layer-2 demand of 28.
pub const GENERATED_V2_SCAN_GAMES: u32 = 240;

/// The seat count v2's generating ecology plays at — 2P throughout (the canon
/// priors are 2P priors).
const GENERATED_V2_PLAYERS: u8 = 2;

/// **THE PINNED v2 SEAT (V4 step 8; V3 spec §5.1's fix, adopted verbatim).**
/// The contestant every seat of v2's generating ecology plays: lookahead at
/// the shipped knobs, with `WeightTable::V2` EXPLICIT — an in-crate weighted
/// construction, never the live default.
///
/// This const WAS `Contestant::LOOKAHEAD_V1_DEFAULT`, whose doc promised to
/// re-point at the identity freeze "because the ecology is identity" — and
/// that promise is exactly the failure V3 §5.1 named: at any later controller
/// re-point both scanning seats would play differently and every recorded
/// `(predicate, master, ordinal)` triple would silently resolve to a DIFFERENT
/// state. The pin inverts the promise: the seats are frozen HERE, at explicit
/// V2 values, so ordinal semantics survive every controller bump including the
/// identity-freeze era — and the version string moved `v2.2` → `v2.3` because
/// the seat field is identity.
///
/// The knobs are TRANSCRIBED LITERALS (beam 4, one sampled world, eight
/// retreat samples, no cap), equal to `Contestant::LOOKAHEAD_V1_DEFAULT`'s
/// today — `the_pinned_v2_seat_transcribes_todays_shipped_knobs` states that
/// equality as a transcription fact. If the shipped default legitimately moves
/// later, THAT assertion is deleted in the moving commit and these literals
/// stay, exactly as `PRE_FREEZE_2026_07_27`'s doc prescribes for a historical
/// record.
///
/// This ecology generates the SUBJECT'S OWN reachable positions, which is the
/// right population for an on-policy checkpoint and is said out loud here
/// because it is also the ecology's whole cost: both seats search.
const PINNED_V2_SEAT_BEAM: u16 = 4;
const PINNED_V2_SEAT_WORLDS: u8 = 1;
const PINNED_V2_SEAT_RETREAT_SAMPLES: u8 = 8;

/// Build one pinned v2 generating seat on `agent_seed` — the `eval_screen`
/// precedent (`LookaheadAgent::with_knobs`, public in-crate weighted
/// construction), used only by the scan, never the runner grammar.
fn build_pinned_v2_seat(rules: &Rules, agent_seed: u64) -> Box<dyn ArenaAgent> {
    Box::new(crate::game::lookahead::LookaheadAgent::with_knobs(
        rules,
        crate::game::lookahead::SearchScheme::TurnBoundary {
            beam: PINNED_V2_SEAT_BEAM,
        },
        PINNED_V2_SEAT_WORLDS,
        PINNED_V2_SEAT_RETREAT_SAMPLES,
        WeightTable::V2,
        agent_seed,
    ))
}

/// The pinned v2 seat's RESOLVED echo — composed from the same
/// `controller_provenance` + `compose_echo` pair every contestant echoes
/// through, over the pinned construction's own explicit inputs, so the
/// recorded seat identity carries `WeightTable::V2`'s table identity forever
/// (the always-on half of V3 §5.1's two-tier behavior-identity pin).
fn pinned_v2_seat_echo() -> String {
    let p = crate::game::lookahead::controller_provenance(
        crate::game::lookahead::SearchScheme::TurnBoundary {
            beam: PINNED_V2_SEAT_BEAM,
        },
        PINNED_V2_SEAT_WORLDS,
        PINNED_V2_SEAT_RETREAT_SAMPLES,
        &WeightTable::V2,
        None,
    );
    crate::game::arena::compose_echo(&p.name, p.version, &p.params)
}

/// The one driver line both ecologies record — spelled once so a reworded
/// sentence cannot make the two ecologies look like different drivers.
const GENERATED_DRIVER: &str = "arena::run_arena_game (honest bench, per-move invariants ON)";

/// The one seed-derivation line both ecologies record.
const GENERATED_SEED_DERIVATION: &str =
    "SeedSchedule::diagnostic(master).next_world() once per game in scan order; \
     agent_stream_seed(world, seat, Slot(seat)) per seat";

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
    /// How many hits per generated game entered the ordinal sequence — the
    /// STRIDE ([`GENERATED_STRIDE_CAP`]).
    ///
    /// IDENTITY, like every other field here, and for the sharpest version of
    /// this type's own reason: changing the cap re-points every
    /// `(predicate, master, ordinal)` triple past the first game while every
    /// triple still reads the same. It carries **no `#[serde(default)]`
    /// deliberately** — no committed artifact holds a serialized `Generated`
    /// triple, so a sidecar that lacks this field is one written before the
    /// stride existed and should fail loudly rather than be silently read as
    /// "cap 0" (which is not even a meaningful scan).
    pub per_game_cap: u32,
}

impl GeneratingEcology {
    /// The v1 configuration — THE single spelling of it.
    ///
    /// Every string here is prose a reader acts on, not a key anything parses:
    /// what changes an identity is the FIELD SET plus [`GENERATED_ECOLOGY_V1`],
    /// and a reworded driver line that left the play unchanged would be a
    /// spurious identity change. Reword freely; re-point nothing.
    pub fn v1() -> Self {
        v1_config().ecology
    }

    /// The v2 LOOKAHEAD configuration — Phase C's layer-2 ecology, both seats
    /// the step-8 pinned explicit-V2 construction.
    pub fn v2_lookahead() -> Self {
        v2_lookahead_config().ecology
    }
}

/// A generating configuration: the recorded [`GeneratingEcology`] plus the live
/// parts a scan needs to play the games.
///
/// How a generating ecology CONSTRUCTS its seats — a named contestant built
/// through `Contestant::build` (v1's rung), or the V4 step-8 PINNED explicit-V2
/// lookahead construction (v2.3's seats; see [`build_pinned_v2_seat`] and V3
/// spec §5.1). An enum rather than a `Contestant` because the pin's whole
/// point is that the pinned seat is NOT a name a later freeze can re-point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GeneratingSeat {
    /// Built through [`Contestant::build`], echoing [`Contestant::echo`].
    Contestant(Contestant),
    /// The pinned v2 seat: `WeightTable::V2` explicit at transcribed knobs.
    PinnedLookaheadV2,
}

impl GeneratingSeat {
    fn build(&self, rules: &Rules, agent_seed: u64) -> Box<dyn ArenaAgent> {
        match self {
            GeneratingSeat::Contestant(c) => c.build(rules, agent_seed).0,
            GeneratingSeat::PinnedLookaheadV2 => build_pinned_v2_seat(rules, agent_seed),
        }
    }

    fn echo(&self) -> String {
        match self {
            GeneratingSeat::Contestant(c) => c.echo(),
            GeneratingSeat::PinnedLookaheadV2 => pinned_v2_seat_echo(),
        }
    }
}

/// The live construction and its recorded echo travel together: each config
/// builds both from one [`GeneratingSeat`], so the descriptor cannot drift from
/// the seats the scan actually benches.
struct GeneratingConfig {
    /// The construction every seat plays.
    seat: GeneratingSeat,
    /// The seat count.
    players: u8,
    /// What gets recorded on the descriptor.
    ecology: GeneratingEcology,
}

/// The v1 (rung) generating configuration — family 1's.
fn v1_config() -> GeneratingConfig {
    let seat = GeneratingSeat::Contestant(GENERATED_SEAT);
    GeneratingConfig {
        seat,
        players: GENERATED_PLAYERS,
        ecology: GeneratingEcology {
            id: GENERATED_ECOLOGY_V1.to_string(),
            driver: GENERATED_DRIVER.to_string(),
            players: GENERATED_PLAYERS,
            seats: vec![seat.echo(); GENERATED_PLAYERS as usize],
            seed_derivation: GENERATED_SEED_DERIVATION.to_string(),
            games: GENERATED_SCAN_GAMES,
            per_game_cap: GENERATED_STRIDE_CAP,
        },
    }
}

/// The v2 (lookahead) generating configuration — the layer-2 families'.
fn v2_lookahead_config() -> GeneratingConfig {
    let seat = GeneratingSeat::PinnedLookaheadV2;
    GeneratingConfig {
        seat,
        players: GENERATED_V2_PLAYERS,
        ecology: GeneratingEcology {
            id: GENERATED_ECOLOGY_V2_LOOKAHEAD.to_string(),
            driver: GENERATED_DRIVER.to_string(),
            players: GENERATED_V2_PLAYERS,
            seats: vec![seat.echo(); GENERATED_V2_PLAYERS as usize],
            seed_derivation: GENERATED_SEED_DERIVATION.to_string(),
            games: GENERATED_V2_SCAN_GAMES,
            per_game_cap: GENERATED_STRIDE_CAP,
        },
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
    /// A root a caller has ALREADY resolved, with the descriptor that
    /// identified it — resolving it again returns both verbatim.
    ///
    /// **Why this is not [`Constructed`](Self::Constructed) with extra steps**
    /// (Phase C's T5): a block of family roots is resolved ONCE, in a single
    /// scan walk, and then adjudicated under every policy of a reply menu.
    /// Handing `Generated` to each of those adjudications would re-run the scan
    /// per policy per root — at the v2 ecology, where both generating seats
    /// search, that is the block's entire cost paid over and over. Handing
    /// `Constructed` instead would work and would LIE: every table would record
    /// a root whose origin is "someone gave me this position" when the origin is
    /// a seeded scan a reader can reproduce. This variant is the honest third
    /// option — resolve once, carry the identity.
    ///
    /// **It is the ONLY arm whose descriptor the caller supplies, so it is the
    /// only arm whose pair can disagree — and since Phase C's T5 fix wave that
    /// disagreement is CHECKED**: [`resolve`](Self::resolve) re-derives
    /// [`state_digest`] and refuses a descriptor that describes another position
    /// ([`RootError::ResolvedMismatch`]). A caller pairing a descriptor with a
    /// different state is describing a root it did not resolve, and NOTHING
    /// downstream could tell — every report, digest and sidecar would name the
    /// wrong position perfectly consistently. The check costs one digest of a
    /// state the arm is about to clone anyway, and it is the Phase-B registry
    /// loader's `RegistryError::Stale` posture one level down: not a corruption
    /// report, but the identity key, refused before anything reads the state.
    /// The discipline is still that the pair comes from ONE `resolve` call; this
    /// makes departing from it a typed error instead of a silent one.
    Resolved { state: GameState, descriptor: RootDescriptor },
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
            Self::Resolved { state, descriptor } => {
                // The one integrity check this arm needs: the caller supplied
                // BOTH halves, so the pair is the only thing that can be wrong.
                let digest = state_digest(state);
                if digest != descriptor.root_digest {
                    return Err(RootError::ResolvedMismatch {
                        origin: descriptor.origin.kind(),
                        descriptor_digest: descriptor.root_digest.clone(),
                        state_digest: digest,
                    });
                }
                Ok((state.clone(), descriptor.clone()))
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
    /// A [`RootSource::Resolved`] pair whose descriptor describes a DIFFERENT
    /// position than the state it travels with.
    ///
    /// The comparability key, not a corruption report — the `RegistryError::Stale`
    /// shape at the one place a caller supplies both halves of an identity. The
    /// descriptor's ORIGIN kind rides along because it is what a reader acts on:
    /// a mis-paired `generated` descriptor points at a scan someone will re-run,
    /// while a mis-paired `constructed` one points at nothing at all.
    ResolvedMismatch { origin: &'static str, descriptor_digest: String, state_digest: String },
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
            Self::ResolvedMismatch { origin, descriptor_digest, state_digest } => write!(
                f,
                "the pre-resolved pair describes two positions: the {origin} descriptor names \
                 root {descriptor_digest} and the state it travels with digests to \
                 {state_digest}"
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
    ///
    /// Defaulted on read, mirroring `SeatProvenance::params` (record.rs:133) for
    /// its reason: a KNOBLESS controller's empty map is the commonest value this
    /// field takes, and a sidecar written without the key — by a writer that
    /// skipped it, or by an older build — must read back as "no knobs" rather
    /// than fail at parse.
    #[serde(default)]
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
                    echo: spec.echo(),
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
pub(crate) fn require_eligible_root(root: &GameState) -> Result<(), DrivenError> {
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

/// Everything a rooted call is refused for BEFORE an agent is built — the seat
/// list, the seat count, the options, and the root's own eligibility.
///
/// Extracted at T4 rather than duplicated: [`adjudicate_root`] has to run these
/// checks ITSELF, because a TERMINAL root produces no playout at all and a guard
/// left to the first playout would simply never run — an ill-formed call on a
/// finished game would come back `Ok` with an empty table. Same checks, same
/// order, one authority.
///
/// Deliberately NOT including `check_invariants`: [`play_from_state`] guards its
/// own incoming state unconditionally, so a driven run gets that for free, while
/// the adjudicator needs it a move EARLIER (before a candidate is forced) and
/// asks for it there.
///
/// **The SEAT-LIST arity is this function's only own business** since Phase C's
/// T2: everything after it is [`require_driveable_root`], which [`drive_turn`]
/// calls directly. A turn names ONE seat rather than a list, so it has no arity
/// to check and no `SeatMismatch` to report — but it refuses the same seat
/// counts, the same option, and the same live-tracked roots the whole-game
/// driver does, which is what keeps "driveable" one property rather than two.
/// The order is preserved exactly (arity, then count, then the option, then
/// eligibility) so no existing refusal moves.
fn preflight(root: &GameState, seats: &[SeatSpec], opts: DrivenOpts) -> Result<(), DrivenError> {
    let players = root.players.len();
    if seats.len() != players {
        return Err(DrivenError::SeatMismatch { seats: seats.len(), players });
    }
    require_driveable_root(root, opts)
}

/// The part of [`preflight`] that is about the ROOT and the OPTIONS rather than
/// about a seat list — shared verbatim with [`drive_turn`].
fn require_driveable_root(root: &GameState, opts: DrivenOpts) -> Result<(), DrivenError> {
    let players = root.players.len();
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
    require_eligible_root(root)
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
    preflight(root, seats, opts)?;
    let players = root.players.len();

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
        let (agent, handle) = spec.build(rules, agent_seed);
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
            echo: spec.echo(),
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
// The turn-grain driver (Phase C, T2)
// ---------------------------------------------------------------------------

/// ONE MOVER'S TURN, driven from a root: what it played, where it handed off,
/// and what its search cost across the WHOLE turn.
///
/// The grain between [`root_table`] (one decision) and [`DrivenOutcome`] (a
/// whole game), and the one families 2–4 read: their pass rules are statements
/// about what a TURN did — the trigger fired, an achievement was claimed, a
/// contested step was taken — and none of those is decidable at a single
/// decision, because the move that fires the trigger is rarely the move that
/// starts the turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnOutcome {
    /// The mover's applied moves, in order. Every entry's seat is the ROOT's
    /// mover — an opponent move inside a turn is structurally impossible (see
    /// [`drive_turn`]'s stop rule), not merely filtered out here.
    pub moves: Vec<AppliedMove>,
    /// The state the turn ENDED in: the first state whose `current_player` is
    /// not the root's, or the terminal state when the turn ended the game.
    pub handoff: GameState,
    /// True when the stop was the GAME ENDING rather than a hand-off — read as
    /// `is_game_over(handoff)` at the stop, never inferred from the mover.
    ///
    /// The two are not interchangeable and one of them is a trap: the game-end
    /// arm of `advance_phase` (`apply_move.rs:259-264`) sets `TurnPhase::TurnEnd`
    /// and NEVER writes `current_player`, so a game that ends on the mover's own
    /// turn hands off to nobody.
    pub game_over: bool,
    /// The mover's counters ACROSS THE WHOLE TURN — one agent, one handle, read
    /// after the last decision. Never the last decision's alone: design §2's
    /// SHOULD-6 is that a mid-turn cap hit or starvation must reach D7's
    /// step-one filter, and a per-decision snapshot would drop exactly the
    /// decisions that hit it.
    pub counters: SeatCounters,
    /// Decision points resolved — equal to `moves.len()`, on
    /// [`DrivenOutcome::decisions`]'s own grounds.
    pub decisions: u64,
}

/// Play ONE TURN: the mover's decisions from `root` until it hands off or the
/// game ends.
///
/// **THE STOP IS DUAL, and both halves are mandatory** (design §2's verified
/// premise, `apply_move.rs:252-279`). `current_player` has exactly ONE
/// production writer — `advance_phase`'s end-of-turn arm at
/// `apply_move.rs:276` — and mercenary sub-turns return through
/// `finish_recovery_transition` (`apply_move.rs:1166-1183`) without touching it,
/// which is what makes a turn cheap and reply-policy-free: the opponent never
/// moves inside one, so ONE seat spec names the whole run. But the arm BESIDE
/// that write (`apply_move.rs:259-264`) ends the game by setting
/// `TurnPhase::TurnEnd` and writing no seat at all, so a `current_player`-only
/// loop would spin at game end until the decision bound refused it, and an
/// `is_game_over`-only loop would run straight through every hand-off. The stop
/// is therefore `current_player != root.current_player || is_game_over(state)`,
/// with [`TurnOutcome::game_over`] READ from the second condition rather than
/// inferred from the first. The two reasons are disjoint by that code shape, and
/// the game-over corner is pinned as such.
///
/// **SEED DISCIPLINE — the arena's, scoped to a turn, and no third discipline is
/// invented** (design §2, review SHOULD-6). ONE agent is constructed at
/// `decision_seed` for the mover's seat and its stream ADVANCES across the
/// turn's decisions, exactly as a seat's long-lived stream advances across a
/// game in [`drive_to_terminal`]. At the FIRST decision that is byte-identical
/// to [`root_table`]'s fresh-agent-at-the-same-seed — which is what joins D7's
/// counter check (a table at replication 0's seed) to the read (a turn at the
/// same seed) — and the join is pinned rather than asserted in prose. Beyond
/// decision 0 the loop is pinned against the play half itself: at
/// `decision_seed = agent_stream_seed(world, mover, Slot(mover))` a turn's move
/// list EQUALS the mover's own prefix of `drive_to_terminal` at `world`.
///
/// **This is a THIRD per-decision loop, said out loud.** [`play_from_state`]
/// runs to terminal and exposes no stop hook, so a turn cannot borrow it; what
/// keeps the two from drifting is the prefix-equivalence pin above plus the
/// failure taxonomy, which is routed through [`classify`] over the arena's own
/// [`PlayError`] rather than re-typed — so a turn and a game refuse the same
/// root with the same variant and the same message BY CONSTRUCTION. The one
/// deliberate omission is `play_from_state`'s debug-tier VOLUME cross-check: a
/// turn reports no volume, so no attested figure crosses this seam, and adding
/// a fourth debug route DFS at every on-board Movement point would cost every
/// caller for a quantity nothing here reads. The membership `debug_assert` IS
/// kept — it guards the canonical-witness gap, which reaches an applied move.
///
/// **An ALREADY-TERMINAL root is an `Ok`**, not a refusal: zero moves,
/// `handoff` the root itself, `game_over` true, and an explaining seat's
/// `explained` reading `Some(0)`. That is [`drive_to_terminal`]'s contract at
/// this grain and for the same reason — a turn with nothing in it is a real
/// answer, and refusing it would make "the position was already finished" the
/// one reading the instrument could not report.
///
/// **`mover` names WHAT plays, never WHICH SEAT.** The seat is
/// `root.current_player` and there is nothing to reconcile: a [`SeatSpec`]
/// carries a contestant, not an index, and a turn has exactly one mover by the
/// stop rule above — which is also why this takes ONE spec where
/// [`drive_to_terminal`] takes a list, and why no reply policy appears anywhere
/// in a turn.
///
/// The root hygiene is [`drive_to_terminal`]'s, through the SHARED guard
/// [`require_driveable_root`] (seat count, `max_decisions`, hidden-deal
/// eligibility) plus a `check_invariants` guard that names the ROOT before the
/// first decision — so a corrupt fixture is never misattributed to a move. The
/// eligibility check is the one that would be easiest to drop and hardest to
/// miss the loss of: `observe` is only ever called for the MOVER inside a turn,
/// so a live-tracked root whose OPPONENT hid its deal would otherwise drive
/// perfectly and report a turn nobody could have played.
pub fn drive_turn(
    root: &GameState,
    mover: SeatSpec,
    rules: &Rules,
    decision_seed: u64,
    opts: &DrivenOpts,
) -> Result<TurnOutcome, DrivenError> {
    require_driveable_root(root, *opts)?;
    // The ROOT's own guard, run HERE rather than left to the first decision —
    // `drive_to_terminal` gets this from `play_from_state`, which a turn cannot
    // borrow, and `adjudicate_root`'s reason applies unchanged: a state computed
    // on a corrupt root is a wrong answer wearing the shape of a right one, and
    // it must be refused as the ROOT it is rather than misattributed to a move.
    // Unconditional in `opts.invariants`, which is about the PER-MOVE cost of a
    // bulk run and never about the state handed in.
    check_invariants(root, rules).map_err(DrivenError::CorruptRoot)?;

    // ONE agent, at `decision_seed`, for the whole turn — the seed discipline
    // this function's doc states, and the reason the handle below reads the
    // TURN's counters rather than a decision's.
    let seat = root.current_player;
    let (mut agent, handle) = mover.build(rules, decision_seed);

    let mut state = root.clone();
    let mut moves: Vec<AppliedMove> = Vec::new();
    let game_over = loop {
        // THE DUAL STOP, in the order that makes `game_over` a FACT about the
        // state rather than an inference: a game that ended on this turn never
        // wrote `current_player` (`apply_move.rs:259-264`), and a hand-off is
        // never terminal, so the two arms are disjoint and each is checked for
        // itself.
        if is_game_over(&state) {
            break true;
        }
        if state.current_player != seat {
            break false;
        }
        if state.turn_number > opts.max_turns {
            return Err(classify(PlayError::TurnCap { turns: state.turn_number }));
        }
        // The decision guard, checked BEFORE the decision it would be the
        // (max+1)-th of, so the refusal names the bound the caller set rather
        // than one past it — `play_from_state`'s rule, kept identical.
        let decisions = moves.len() as u64;
        if decisions >= opts.max_decisions {
            return Err(classify(PlayError::DecisionCap { decisions }));
        }

        // The honest chain, mirroring `AgentBench::decide`: the agent sees
        // `observe(state, its seat)` and nothing else, and a refusal is a typed
        // hard error with no fallback arm behind it. It is mirrored rather than
        // called because that bench is indexed by SEAT and a turn seats exactly
        // one — but every FAILURE is routed through `classify` over the arena's
        // own `PlayError`, so the two loops cannot drift in their taxonomy or
        // their messages.
        let view = observe(&state, seat)
            .map_err(|e| classify(PlayError::Observe { seat, detail: e.to_string() }))?;
        // Plumbing pin, unconditional exactly as the bench's is: one `u8`
        // compare against a fault that would hand a seat someone else's
        // information.
        assert_eq!(view.viewer, seat, "view/seat mismatch in the turn driver");
        let mv = agent
            .choose(&view, rules)
            .map_err(|detail| classify(PlayError::Agent { seat, detail }))?
            .mv;

        // The validation boundary, `play_from_state`'s verbatim: a false here is
        // a driver bug, so it is a hard error and never a redraw.
        if !is_move_legal(&state, &mv, rules, PathMode::Canonical) {
            return Err(classify(PlayError::IllegalPick { seat, mv: Box::new(mv) }));
        }
        // The debug-tier membership cross-check, kept: `Canonical` is wider than
        // list membership — it does not verify lex-least witness selection — so
        // this is what catches a non-canonical witness before it reaches an
        // applied move. The bench's OTHER debug-tier check, the attested-volume
        // comparison, is deliberately NOT here: a turn reports no volume, so no
        // attested figure crosses this seam, and a fourth debug route DFS at
        // every on-board Movement point would cost every caller for a quantity
        // nothing reads.
        debug_assert!(
            get_legal_moves(&state, rules).contains(&mv),
            "seat {} played a move that passed PathMode::Canonical but is not in \
             get_legal_moves — a non-canonical witness: {mv:?}",
            seat.0,
        );

        state = apply_move_unchecked(&state, &mv, rules)
            .map_err(|e| classify(PlayError::Apply { seat, detail: e.to_string() }))?;
        if opts.invariants {
            if let Err(reason) = check_invariants(&state, rules) {
                return Err(classify(PlayError::Invariant { seat, reason }));
            }
        }
        moves.push(AppliedMove { seat, mv });
    };

    // Read AFTER the loop: the handle is an `Arc` on live counters, so reading
    // it earlier would report a turn that had not happened yet.
    let counters = handle.as_ref().map_or(SeatCounters::NONE, SeatCounters::from_handle);
    Ok(TurnOutcome {
        decisions: moves.len() as u64,
        moves,
        handoff: state,
        game_over,
        counters,
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
    /// **A NAMED IDENTITY — an alternate-table arena arm (V4 §7 step 8).** Like
    /// the atomic-diag arm this one WOULD work: `Contestant::build` routes it
    /// through the same `build_search` as `lookahead-v1` and it carries the
    /// same ranked-root surface, so mirroring it here would be one more match
    /// arm. It is refused on SCOPE, and the scope is a standing rule rather
    /// than this wave's preference: V3 spec §5.1 admitted in-crate weighted
    /// construction "used only by the scan, **never the runner grammar**", and
    /// this harness's rooted reads are keyed to ONE pinned generating identity
    /// (`GENERATED_ECOLOGY_V2_LOOKAHEAD`). Admitting a seat whose table is a
    /// runner argument would put a weight identity into that grammar and make
    /// two `--contestant` spellings resolve to two different evaluators over
    /// one recorded ecology. The §11 arms are the ARENA's cells; run them
    /// through `examples/arena`.
    AlternateTable,
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
            Self::AlternateTable => {
                "an alternate-table arena identity; this harness's rooted reads are keyed to one \
                 pinned generating identity, and the weight arms are cells for examples/arena"
            }
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
        Contestant::NamedTable(_) => refuse(UnsupportedReason::AlternateTable),
    }
}

// ---------------------------------------------------------------------------
// The generated scan
// ---------------------------------------------------------------------------

/// Find the `ordinal`-th position satisfying `predicate` in a seeded scan at
/// `master`, under the ecology `predicate.ecology()` names.
///
/// **The scan order IS the ordinal**, and it is stated here because every
/// recorded triple depends on it: games in `SeedSchedule::diagnostic(master)`
/// draw order, and within a game the positions in PLAY order — position `k`
/// being the state event `k` was decided in, `state_at`'s own index. Hits are
/// counted across games, so ordinal 0 is the first hit of game 0 and the
/// counting never restarts.
///
/// **The ordinal space is STRIDED** (design §4): at most
/// [`GeneratingEcology::per_game_cap`] hits per generated game enter it, and
/// hits past that cap are skipped WITHOUT consuming an ordinal. Consecutive hits
/// inside one game are successive decision points of one round rather than
/// independent draws, so an unstrided block would quote a root count it does not
/// have. The scan therefore stops walking a game the moment its cap is spent —
/// every remaining hit there is one it would skip anyway.
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
    // ONE implementation, so "the batch resolves the same roots as the
    // per-ordinal path" is true BY CONSTRUCTION rather than by two walks
    // agreeing. The pin verifies it; this is what makes the pin cheap to keep.
    generate_roots(predicate, master, &[ordinal], rules)
        .map(|mut roots| roots.pop().expect("one ordinal asked for, one root returned"))
}

/// [`generate_root`] for a WHOLE BLOCK of ordinals, in ONE scan walk (Phase C's
/// T5, plan review NOTE-8).
///
/// **Why a batch entry point exists at all.** The scan is defined over a
/// SEQUENCE — ordinal `n` is the n-th strided hit across the generated games —
/// so resolving ordinals 0..6 one at a time re-plays the same games six times
/// and throws five of the walks away. On the rung ecology that is a second of
/// waste; on the v2 lookahead ecology, where BOTH generating seats search, it is
/// the whole cost of a family block paid six times over. The walk is identical
/// to the single-ordinal one in every other respect — same games in the same
/// draw order, same in-game position order, same stride, same descriptors — and
/// [`generate_root`] is a call to THIS function, which is what keeps the two
/// from drifting.
///
/// Returns one root per requested ordinal, IN THE REQUESTED ORDER (a repeated
/// ordinal is answered twice). A missing ordinal is
/// [`RootError::Exhausted`] naming the FIRST one the scan could not supply,
/// with the total it did find — the same refusal a single-ordinal call gives.
///
/// The scan stops the moment the HIGHEST requested ordinal is collected, so
/// asking for a block costs no more than asking for its last member.
pub fn generate_roots(
    predicate: RootPredicate,
    master: u64,
    ordinals: &[u32],
    rules: &Rules,
) -> Result<Vec<(GameState, RootDescriptor)>, RootError> {
    let wanted: BTreeSet<u32> = ordinals.iter().copied().collect();
    let Some(&highest) = wanted.iter().next_back() else {
        return Ok(Vec::new());
    };
    let ecology = predicate.ecology();
    let mut collected: BTreeMap<u32, (GameState, RootDescriptor)> = BTreeMap::new();
    let walk = walk_scan(predicate, master, rules, &mut |hit| {
        if wanted.contains(&hit.ordinal) {
            let root_digest = state_digest(hit.state);
            collected.insert(
                hit.ordinal,
                (
                    hit.state.clone(),
                    RootDescriptor {
                        origin: RootOrigin::Generated {
                            predicate,
                            master,
                            ordinal: hit.ordinal,
                            game_index: hit.game_index,
                            ecology: ecology.clone(),
                        },
                        event_index: Some(hit.position),
                        root_digest,
                    },
                ),
            );
            if hit.ordinal == highest {
                return ScanStep::Stop;
            }
        }
        ScanStep::Continue
    })?;

    ordinals
        .iter()
        .map(|ordinal| {
            collected.get(ordinal).cloned().ok_or(RootError::Exhausted {
                predicate,
                ordinal: *ordinal,
                found: walk.found,
                games: ecology.games,
            })
        })
        .collect()
}

/// One hit a [`walk_scan`] kept — the strided ordinal, the game and position it
/// was found at, and the state itself, BORROWED.
///
/// The lifetime is the honest part: a visitor that wants the state keeps it by
/// CLONING, and one that only counts pays nothing. That is what lets
/// [`scan_capacity`] measure a predicate's supply without any position ever
/// leaving the walk.
struct ScanHit<'a> {
    ordinal: u32,
    game_index: u32,
    position: usize,
    state: &'a GameState,
}

/// Whether a [`walk_scan`] visitor has seen enough.
enum ScanStep {
    Continue,
    Stop,
}

/// What a completed [`walk_scan`] counted.
struct ScanWalk {
    /// Strided hits COUNTED — the number a [`RootError::Exhausted`] reports.
    /// A visitor that stopped the walk early leaves this at the ordinal it
    /// stopped on, exactly as the pre-split loop did.
    found: u32,
    /// Generated games actually played.
    games_walked: u32,
}

/// THE SCAN WALK — one implementation, two consumers.
///
/// [`generate_roots`] resolves ordinals to roots and [`scan_capacity`] measures
/// how many ordinals there are; both are the SAME sequence of games, positions,
/// predicate evaluations and stride decisions, and one walk is what makes that a
/// property rather than two functions agreeing today. The visitor sees every
/// kept hit and decides whether the walk goes on.
///
/// **Every ordering rule the sequence depends on lives here**: games in
/// `SeedSchedule::diagnostic(master)` draw order, positions within a game in
/// PLAY order (`state_at`'s own index), the ordinal incremented once per KEPT
/// hit and never restarted across games, and the STRIDE — at most
/// `per_game_cap` hits per game, the rest skipped without consuming an ordinal,
/// with the game abandoned the moment its cap is spent.
///
/// The `Stop` arm short-circuits BEFORE the ordinal increment and before the
/// stride bookkeeping, which is where the pre-split loop's `break 'games` sat;
/// the split is behaviour-preserving and `generate_roots`' own reproduction pins
/// are what say so.
fn walk_scan(
    predicate: RootPredicate,
    master: u64,
    rules: &Rules,
    on_hit: &mut dyn FnMut(ScanHit<'_>) -> ScanStep,
) -> Result<ScanWalk, RootError> {
    #[cfg(test)]
    scan_probe::record_walk();
    let config = predicate.config();
    let ecology = config.ecology;
    let games = ecology.games;
    let per_game_cap = ecology.per_game_cap;
    let mut schedule = SeedSchedule::diagnostic(master);
    let mut found: u32 = 0;
    let mut games_walked: u32 = 0;

    'games: for game_index in 0..games {
        let world = schedule.next_world();
        // The ecology's own seating, on the arena's own keying — the same
        // derivation `drive_to_terminal` uses, so a generated root is reachable
        // by a run anyone can reproduce from the master alone.
        let mut make = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
            config
                .seat
                .build(rules, agent_stream_seed(world, seat, Slot(seat.0)))
        };
        let stats = run_arena_game(world, config.players, rules, &mut make, true)
            .map_err(|e| RootError::Generate { game_index, detail: e.to_string() })?;
        games_walked += 1;

        let events = &stats.record.events;
        let mut state = state_at(&stats.record, 0, rules)
            .map_err(|e| RootError::Generate { game_index, detail: e.to_string() })?;
        let mut hits_this_game: u32 = 0;
        // INCLUSIVE of the past-the-last position, for totality: `state_at(record,
        // len)` is a real position (the terminal state) and a future predicate may
        // want it. The shipped one excludes it — its `!is_game_over` conjunct —
        // so that position is unreachable today rather than dead, and stopping the
        // range one short would make the scan quietly predicate-specific.
        for position in 0..=events.len() {
            if predicate.holds(&state, rules) {
                let step = on_hit(ScanHit { ordinal: found, game_index, position, state: &state });
                if matches!(step, ScanStep::Stop) {
                    break 'games;
                }
                found += 1;
                hits_this_game += 1;
                if hits_this_game >= per_game_cap {
                    // The STRIDE. Every remaining hit in this game would be
                    // skipped, so walking on would only cost predicate
                    // evaluations — which, for the apply-probing layer-2 arms,
                    // is the scan's whole cost.
                    break;
                }
            }
            if let Some(event) = events.get(position) {
                state = replay_from(&state, std::slice::from_ref(event), rules)
                    .map_err(|e| RootError::Generate { game_index, detail: e.to_string() })?;
            }
        }
    }

    Ok(ScanWalk { found, games_walked })
}

/// A CAPACITY measurement: how many strided ordinals a predicate's scan realizes
/// at a master, and which generated game each one came from.
///
/// **THIS TYPE IS THE NO-BURN ARGUMENT.** Spec §6 permits — and requires —
/// capacity measurement at step 2 while fencing every root-table read at ordinal
/// ≥ 8 until step 10, and the fence is only worth something if "measuring" and
/// "reading" are different acts. Here they are different by CONSTRUCTION: this
/// struct carries counts and game indices and nothing else. There is no
/// `GameState`, no [`RootDescriptor`], no digest, and no field a position could
/// hide in — so a capacity report cannot tell anybody what an escrow root LOOKS
/// like, whatever they do with it afterwards. [`scan_capacity`] builds no root
/// table and clones no state; the borrow in [`ScanHit`] is what makes that
/// cheap, and this type is what makes it checkable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanCapacity {
    /// The predicate measured.
    pub predicate: RootPredicate,
    /// The master measured at. RECORDED BY THE CALLER before use, as ever.
    pub master: u64,
    /// The generating ecology the scan ran under — without which a count is not
    /// an identity, on the descriptor's own reasoning.
    pub ecology: GeneratingEcology,
    /// `game_of_ordinal[n]` is the generated game strided ordinal `n` was found
    /// in. Its LENGTH is the realized supply (or the stop bound, if
    /// [`truncated`](Self::truncated)).
    pub game_of_ordinal: Vec<u32>,
    /// Generated games actually played.
    pub games_walked: u32,
    /// The walk stopped at the caller's bound rather than exhausting the
    /// ecology's games — so the supply is AT LEAST `game_of_ordinal.len()`,
    /// never exactly it.
    pub truncated: bool,
}

impl ScanCapacity {
    /// Strided ordinals found (`>=` the true supply's lower bound; exactly the
    /// supply when not [`truncated`](Self::truncated)).
    pub fn found(&self) -> u32 {
        self.game_of_ordinal.len() as u32
    }

    /// Whether ordinal `n` was realized by this measurement.
    pub fn covers(&self, ordinal: u32) -> bool {
        (ordinal as usize) < self.game_of_ordinal.len()
    }

    /// Do the ordinals in `left` and `right` come from DISJOINT generated games,
    /// as measured?
    ///
    /// The MEASURED companion to [`ordinals_are_stride_separated`]'s guarantee,
    /// and the instrument spec §6's independence claim needs where the lemma
    /// does not reach — the development-EXTENSION boundary, where ordinals 7 and
    /// 8 are one apart, so a game contributing a TWO-ordinal run can land exactly
    /// `{7, 8}` and hold both. `None` when the measurement does not cover every
    /// ordinal asked about: an unmeasured ordinal is not a disjoint one.
    pub fn bands_are_disjoint(
        &self,
        left: std::ops::Range<u32>,
        right: std::ops::Range<u32>,
    ) -> Option<bool> {
        let games_of = |band: std::ops::Range<u32>| -> Option<BTreeSet<u32>> {
            band.map(|ordinal| self.game_of_ordinal.get(ordinal as usize).copied()).collect()
        };
        let left = games_of(left)?;
        let right = games_of(right)?;
        Some(left.is_disjoint(&right))
    }
}

/// Measure a predicate's realized ordinal supply at `master` — WITHOUT building
/// or reading any root table.
///
/// Spec §6: "Capacity measurement at step 2 is permitted, required, and not a
/// burn (no root table read)." This is that measurement. It walks the same scan
/// [`generate_roots`] walks (the same function, [`walk_scan`]), counts the
/// strided hits and records which game each came from, and returns
/// [`ScanCapacity`] — a type with nowhere to put a position. Nothing anywhere on
/// this path calls `root_table` or `adjudicate_root`, and NO CONTESTANT IS BUILT
/// TO EVALUATE ANY ROOT — the only builds are the GENERATING seats, the same
/// ones [`generate_roots`] seats, which is what makes this the same walk rather
/// than a cheaper imitation of it. No `GameState` is cloned out of the walk at
/// all.
///
/// `stop_after` bounds the walk at that many ordinals, which is what makes a
/// measurement affordable on the v2 lookahead ecology; the report says
/// [`truncated`](ScanCapacity::truncated) so a bounded count is never read as an
/// exact supply. `None` walks the ecology's whole game budget — the exact
/// figure, at the ecology's whole price. `Some(0)` measures nothing and says so
/// rather than walking.
///
/// **COST is the ECOLOGY's, and it is the same cost a scan pays**: the two V4
/// predicates scan `driven-generated-v2.3-lookahead`, where both generating
/// seats search — minutes to hours for the full walk. That is the price of the
/// number, and knowing it before the masters are drawn is exactly why spec §6
/// puts the measurement at step 2.
pub fn scan_capacity(
    predicate: RootPredicate,
    master: u64,
    stop_after: Option<u32>,
    rules: &Rules,
) -> Result<ScanCapacity, RootError> {
    let ecology = predicate.ecology();
    if stop_after == Some(0) {
        return Ok(ScanCapacity {
            predicate,
            master,
            ecology,
            game_of_ordinal: Vec::new(),
            games_walked: 0,
            truncated: true,
        });
    }
    let mut game_of_ordinal: Vec<u32> = Vec::new();
    let mut truncated = false;
    let walk = walk_scan(predicate, master, rules, &mut |hit| {
        game_of_ordinal.push(hit.game_index);
        match stop_after {
            Some(bound) if game_of_ordinal.len() as u32 >= bound => {
                truncated = true;
                ScanStep::Stop
            }
            _ => ScanStep::Continue,
        }
    })?;
    Ok(ScanCapacity {
        predicate,
        master,
        ecology,
        game_of_ordinal,
        games_walked: walk.games_walked,
        truncated,
    })
}

/// A TEST-ONLY meter for the one thing a signature cannot state: how many times
/// a call WALKED the scan (Phase C, T6 fix wave, review SHOULD-1).
///
/// **Why it exists.** `gen:` roots are resolved by [`generate_roots`], which
/// plays whole games; the runner's rule is that one command resolves its root
/// ONCE and hands the resolved pair on. Nothing about that is visible in a
/// return type — a second resolve is correct, reproducible, and merely costs a
/// scan again — so the only executable form of the rule is a count.
///
/// **Thread-local, on `alloc_probe`'s precedent** (`game::search_actions`): a
/// scan is synchronous on its caller's thread and libtest gives every cell its
/// own thread, so a cell reads exactly its own walks and parallel cells cannot
/// see each other's. The counter is MONOTONE and [`count_scans`] reads a DELTA,
/// so nesting is harmless and no cell can zero another's region.
///
/// `#[cfg(test)]`, so nothing here ships: the integration tiers link the crate
/// without it, and the shipped `generate_roots` is unchanged.
#[cfg(test)]
pub(crate) mod scan_probe {
    use std::cell::Cell;

    thread_local! {
        static WALKS: Cell<u32> = const { Cell::new(0) };
    }

    /// Record one walk. Called from [`super::generate_roots`] and nowhere else.
    pub(super) fn record_walk() {
        let _ = WALKS.try_with(|walks| walks.set(walks.get().saturating_add(1)));
    }

    /// Run `f`, and report how many scan walks it took ON THIS THREAD.
    pub(crate) fn count_scans<R>(f: impl FnOnce() -> R) -> (R, u32) {
        let before = WALKS.with(Cell::get);
        let value = f();
        (value, WALKS.with(Cell::get) - before)
    }
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
    ///
    /// `rules` is here for the STALENESS verdict
    /// ([`RegistryError::Stale`]) — the one check that is about the running
    /// engine rather than about the entry.
    pub fn load(&self, rules: &Rules) -> Result<RootSource, RegistryError> {
        self.load_from(&workspace_root(), rules)
    }

    /// [`load`](Self::load) against an EXPLICIT anchor.
    ///
    /// **The injection point that makes the loader's own failure set reachable**
    /// (review probe-8). Every shipped entry points at a gitignored session save,
    /// so the only arm a corpus-free cell can otherwise reach is `Missing` — and
    /// a check no cell exercises is a check that can be DELETED with the suite
    /// still green, which is exactly what probe 8 demonstrated about the
    /// recorded-move arm. A test supplies a temp-dir anchor and owns the files
    /// under it; nothing it writes goes near the workspace.
    ///
    /// `pub(crate)` deliberately: the anchor is a test affordance and never a
    /// runner knob. A runner resolving registry paths against anything but
    /// [`workspace_root`] would name a different file for the same entry name
    /// depending on where it was launched, which is the CWD hazard that function
    /// exists to close.
    pub(crate) fn load_from(
        &self,
        root: &Path,
        rules: &Rules,
    ) -> Result<RootSource, RegistryError> {
        // The file half is SHARED with the runner's `<path>:<event>` root
        // ([`load_record`]) so a missing / unreadable / non-record / STALE file
        // refuses identically however the root was named. What stays HERE is
        // exactly what is about THIS ENTRY: the transcription checks, which a
        // path root has nothing to transcribe against.
        let record = load_record(self.name, &root.join(self.path), rules)?;

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

/// Read ONE record file and rule on its comparability — the file half of
/// [`RegistryEntry::load_from`], factored so the runner's `<path>:<event>` root
/// takes the SAME path.
///
/// A root named by an explicit path is the same kind of thing as a root named by
/// an entry: the file is there or it is not, it parses or it does not, and it
/// was produced under these semantics or it was not. Re-implementing those four
/// verdicts on the other side of the argv boundary would let them drift — the
/// staleness check especially, which is the one that decides whether a quoted
/// figure is comparable at all. `name` is what the refusals are reported under:
/// an entry's name, or the ARGUMENT FORM for a root that has no entry.
///
/// What is deliberately NOT here is the entry's own transcription checks (the
/// recorded event index, the recorded move). Those compare an entry against a
/// file; a path root has no entry to compare.
pub(crate) fn load_record(
    name: &'static str,
    path: &Path,
    rules: &Rules,
) -> Result<GameRecord, RegistryError> {
    // `read_to_string` would report the absence too, but as one io error among
    // many — and THE absence is the expected outcome here, so it gets its own
    // arm rather than a message a caller has to match on. The test is `exists`,
    // not `is_file`: something that IS at the path and cannot be read (a
    // directory, a permission-denied file) is a real failure, and reporting it
    // as the expected miss would let a runner walk past it.
    if !path.exists() {
        return Err(RegistryError::Missing { name, path: path.to_path_buf() });
    }
    let text = std::fs::read_to_string(path).map_err(|e| RegistryError::Read {
        name,
        path: path.to_path_buf(),
        detail: e.to_string(),
    })?;
    let record: GameRecord = serde_json::from_str(&text).map_err(|e| RegistryError::Parse {
        name,
        path: path.to_path_buf(),
        detail: e.to_string(),
    })?;

    // STALENESS FIRST, before anything reads the log. An event index or a
    // recorded move checked against a record produced under other semantics
    // compares two different games, so a transcription refusal there would
    // blame the ENTRY for the engine having moved — and a transcription
    // refusal that does NOT fire would be worse, since the entry would go on
    // to hand out a root whose comparability nothing checked.
    //
    // The check is NEUTRAL about absent provenance, matching `state_at`
    // (record.rs: neither lenient nor strict — it prints no warning and
    // refuses on neither key): it fires only where the record CARRIES a rev
    // or a hash that DISAGREES. A record that says nothing — `semantics_rev`
    // `None` (pre-Phase-2.9), or an empty `rules_hash` — passes through here
    // and meets whatever verdict the caller's own replay renders, which is
    // `replay_strict`'s business and not the registry's.
    let running_hash = rules.content_hash();
    let rev_disagrees = matches!(record.semantics_rev, Some(rev) if rev != SEMANTICS_REV);
    let hash_disagrees = !record.rules_hash.is_empty() && record.rules_hash != running_hash;
    if rev_disagrees || hash_disagrees {
        return Err(RegistryError::Stale {
            name,
            record_rev: record.semantics_rev,
            running_rev: SEMANTICS_REV,
            record_hash: record.rules_hash,
            running_hash,
        });
    }
    Ok(record)
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
    /// The record was produced under semantics or rules DATA this engine no
    /// longer runs, so the root it names is not a root this engine can be asked
    /// about.
    ///
    /// The comparability key, not a corruption report: the file may be perfectly
    /// well formed. `record_rev` is `None` on a record that carries no
    /// `semantics_rev` at all — which never fires this arm (see
    /// [`RegistryEntry::load_from`]'s neutrality rule), and appears here only
    /// when the HASH is what disagreed.
    Stale {
        name: &'static str,
        record_rev: Option<u32>,
        running_rev: u32,
        record_hash: String,
        running_hash: String,
    },
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
            Self::Stale { name, record_rev, running_rev, record_hash, running_hash } => write!(
                f,
                "{name}: the record was produced at semantics rev {} / rules {record_hash} and \
                 this engine runs rev {running_rev} / rules {running_hash} — a root from another \
                 rev is not this root",
                record_rev.map_or_else(|| "unstamped".to_string(), |rev| rev.to_string()),
            ),
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

// ---------------------------------------------------------------------------
// The adjudicator: every root candidate, forced and played out (T4)
// ---------------------------------------------------------------------------

/// The sentence every report of an [`Adjudication`] prints beside its numbers
/// (design §3, review N-7 — the label rides the OUTPUT, not only the docs).
///
/// A const rather than prose in the runner so the claim is written once and the
/// runner cannot quietly print a weaker one.
pub const ROBUSTNESS_READ: &str =
    "robustness read: each candidate was FORCED and the remainder PLAYED OUT under the named \
     seats, so a margin here is what those policies reach from this position — not exact ground \
     truth about the candidate";

/// The world one replication plays in — design §4's formula, and the ONE place
/// it is written.
///
/// `mix(SALT_DRIVEN_REPLICATION, [master, replication_index])`, then
/// [`agent_stream_seed`] off THAT world per seat. Two derivations already exist
/// in this repo (the arena's long-lived per-seat stream and the explicit
/// single-decision seed) and the harness invents no third: a replication is a
/// derived WORLD which the arena discipline is then re-entered at, which is why
/// this is a `mix` under a named salt rather than, say, `master + r`.
///
/// `pub` because a consumer explaining a run has to be able to name the world a
/// row was measured in without re-deriving it — and a re-derivation is exactly
/// the second copy this function exists to prevent.
pub fn replication_world(master: WorldSeed, replication: u32) -> WorldSeed {
    WorldSeed(mix(SALT_DRIVEN_REPLICATION, &[master.0, u64::from(replication)]))
}

/// ONE playout of ONE candidate: what happened after the forced move, in one
/// replication.
///
/// Deliberately a SUMMARY rather than a retained [`DrivenOutcome`] — a table is
/// `candidates × K` of these and a kept outcome carries a whole `GameState`
/// each. What survives is everything a margin table is read for plus the two
/// identities that make a row checkable: the world it was played in and the
/// terminal state's digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePlayout {
    /// Which replication this is — its index in design §4's formula.
    pub replication: u32,
    /// The world it played in: [`replication_world`] at that index.
    pub world: WorldSeed,
    /// Each seat's play stream, in seat order — `agent_stream_seed(world, seat,
    /// Slot(seat))`, read off the run rather than re-derived here.
    pub agent_seeds: Vec<u64>,
    /// **THE MARGIN, defined exactly**: the ROOT MOVER's final total minus the
    /// BEST RIVAL's, in victory points, where "best rival" is the highest-RANKED
    /// seat other than the mover in `final_result`'s standings — i.e. chosen by
    /// the official order (total, then the clockwise tie-break from the start
    /// player), never by a bare `max` over totals, which would pick an arbitrary
    /// one of several tied rivals.
    ///
    /// **A margin of 0 is NOT a win**, and conflating them is the mistake this
    /// type is shaped to prevent: two seats level on VP are separated by the
    /// clockwise tie-break, which a signed difference cannot express. The
    /// verdict rides [`won`](Self::won) and [`rank`](Self::rank) beside it, both
    /// straight off `final_result`.
    pub margin: i64,
    /// The mover's own final total.
    pub mover_total: u32,
    /// Which seat the margin was taken against.
    pub best_rival: PlayerId,
    /// That seat's final total.
    pub best_rival_total: u32,
    /// The mover's official rank, 1-based.
    pub rank: u8,
    /// Whether the mover WON — `final_result`'s own verdict, tie-break included.
    pub won: bool,
    /// Decisions the remainder took. **Excludes the forced candidate**, which
    /// this module applied itself: it is the REMAINDER that was played.
    pub decisions: u64,
    /// The terminal state's `turn_number` (absolute — subtract
    /// [`Adjudication::root_turn`] for the turns this playout added).
    pub turns: u32,
    /// [`state_digest`] of the terminal state.
    pub final_digest: String,
    /// **THE MOVER'S FIRST TURN, judged** (Phase C, design §3's turn-grain
    /// admission): whether the TURN PREDICATE a caller supplied to
    /// [`adjudicate_root_with`] held at the hand-off this playout's forced
    /// candidate started.
    ///
    /// `None` means NO HOOK WAS SUPPLIED — the counter-absence distinction this
    /// module keeps everywhere ([`SeatCounters`]'s own rule), never "the
    /// predicate was false". An admission arithmetic that read `None` as `false`
    /// would silently put every hook-free playout in the losing group.
    ///
    /// **The turn is ROOT + forced candidate + the mover's remaining moves**,
    /// and the candidate is decision ZERO of it rather than something that
    /// happened before it — see [`adjudicate_root_with`] for the derivation and
    /// for why the predicate's first argument is the PRE-candidate root.
    pub turn_verdict: Option<bool>,
}

/// ONE root candidate, with every replication's playout of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRow {
    /// The move that was forced.
    pub candidate: Move,
    /// [`state_digest`] of ROOT + this candidate — where every playout in this
    /// row began, and the value that joins the row to the playouts' own stamped
    /// provenance.
    pub continuation_digest: String,
    /// One entry per replication, in index order.
    pub playouts: Vec<CandidatePlayout>,
}

impl CandidateRow {
    /// This candidate's margins, in replication order.
    pub fn margins(&self) -> Vec<i64> {
        self.playouts.iter().map(|p| p.margin).collect()
    }

    /// The mean margin. **RECORDED, never asserted** — the spread over K
    /// replications is a measurement about the reply policies, and a harness
    /// that gated on it would be gating a policy choice.
    ///
    /// `0.0` on an empty row, which no row this module builds can be (K >= 1 is
    /// enforced at the entry point).
    pub fn mean_margin(&self) -> f64 {
        if self.playouts.is_empty() {
            return 0.0;
        }
        let sum: i64 = self.playouts.iter().map(|p| p.margin).sum();
        sum as f64 / self.playouts.len() as f64
    }

    /// The smallest margin any replication reached.
    pub fn min_margin(&self) -> Option<i64> {
        self.playouts.iter().map(|p| p.margin).min()
    }

    /// The largest margin any replication reached.
    pub fn max_margin(&self) -> Option<i64> {
        self.playouts.iter().map(|p| p.margin).max()
    }

    /// Replications the mover WON — the tie-break-aware count, which
    /// `margin >= 0` is not.
    pub fn wins(&self) -> usize {
        self.playouts.iter().filter(|p| p.won).count()
    }
}

/// One seat of an adjudication, as the CALLER named it and as it RESOLVES.
///
/// The resolved echo is the load-bearing half (design §3, review S-1): a reply
/// menu name is a symbol whose meaning moves at D1's identity freeze, so a table
/// carrying only `reply:lookahead-at-final` would describe two different
/// opponents identically. The WITNESSED half — what each agent said it was and
/// what the bench let it see — lives on [`Adjudication::witness`], because it
/// exists only once something has actually been played.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdjudicationSeat {
    pub seat: u8,
    /// What the caller ASKED FOR — see [`SeatSpec::requested_label`].
    pub requested: String,
    /// What actually played — `Contestant::echo`.
    pub echo: String,
}

/// What an adjudication WAS, stamped on every table.
///
/// **Never a `GameRecord`**, and never one per playout either: the root is named
/// ONCE, here, with its full origin, and each row carries its own
/// [`CandidateRow::continuation_digest`] so a reader joins the two by value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdjudicationProvenance {
    /// Which position was adjudicated, and where it came from.
    pub root: RootDescriptor,
    /// The RUNNING engine's `SEMANTICS_REV` — the comparability key.
    pub semantics_rev: u32,
    /// The RUNNING `Rules::content_hash()`.
    pub rules_hash: String,
    /// One entry per seat, in seat order.
    pub seats: Vec<AdjudicationSeat>,
    /// The master every replication's world descends from.
    pub master: u64,
    /// K.
    pub replications: u32,
}

/// Every root candidate, forced and played out — family 1's adjudicator.
///
/// **THIS IS A ROBUSTNESS READ, NOT GROUND TRUTH.** Each candidate was forced
/// and the REMAINDER was played out under the named seats, so a margin here says
/// what those policies reach from this position. It is not a claim about the
/// candidate's value under best play, and a table whose candidates all read
/// alike says the position is dead AGAINST THIS REPLY — which is the honest
/// version of family 1's question and the only version v1 can answer (the exact
/// bounded-tree upgrade is layer-2 work, and only in-final-round roots qualify
/// for it). [`ROBUSTNESS_READ`] is the sentence a report prints.
///
/// **The verdict stays the caller's.** [`all_candidates_equal`](Self::all_candidates_equal)
/// reports a fact about the margins; whether that fact means "dead" is a reading
/// this type does not make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adjudication {
    /// The seat on the move at the root — whose margins these are.
    pub mover: PlayerId,
    /// The root's own `turn_number`, so a playout's absolute
    /// [`CandidatePlayout::turns`] can be read as a delta.
    pub root_turn: u32,
    /// The master every replication's world descends from.
    pub master: WorldSeed,
    /// K.
    pub replications: u32,
    /// One row per root candidate, in the engine's own legal-list order. EMPTY
    /// for a terminal root, which is an `Ok` (see [`adjudicate_root`]).
    pub rows: Vec<CandidateRow>,
    /// The FIRST playout's own stamped provenance, kept verbatim so the
    /// WITNESSED half of a run is not lost — the controller descriptors each
    /// agent reported and the observation labels the bench witnessed.
    ///
    /// `None` when nothing was played (a terminal root). Its `master` field is
    /// replication 0's WORLD, not this table's master: the two are different
    /// numbers by design §4, and reading one for the other would mis-identify
    /// every stream.
    ///
    /// **Its ROOT DESCRIPTOR names the FIRST CANDIDATE'S CONTINUATION, not the
    /// adjudicated root** — a playout starts at root + one forced move, so its
    /// digest equals `rows[0].continuation_digest` and never
    /// [`provenance`](Self::provenance)`.root.root_digest`. The two fields are
    /// one name apart and a reader who took this one for the root would be
    /// reading the wrong position.
    pub witness: Option<RootedProvenance>,
    /// What this adjudication WAS.
    pub provenance: AdjudicationProvenance,
}

impl Adjudication {
    /// How many root candidates were adjudicated.
    pub fn candidates(&self) -> usize {
        self.rows.len()
    }

    /// Whether every candidate produced the SAME margins — the "genuinely dead"
    /// reading, as a fact rather than a verdict.
    ///
    /// Compared ELEMENTWISE over the K replications, not by mean: two candidates
    /// whose margins average alike while differing per replication are not
    /// equal, and calling them equal would hide exactly the variation K exists
    /// to expose.
    ///
    /// **Vacuously `true` below two candidates**, and a caller must not read
    /// that as "dead": an empty table is a FINISHED GAME and a one-row table is
    /// a FORCED position. Both are different facts from "nothing the mover could
    /// choose mattered", and [`candidates`](Self::candidates) is what tells them
    /// apart.
    pub fn all_candidates_equal(&self) -> bool {
        let Some(first) = self.rows.first() else { return true };
        let reference = first.margins();
        self.rows.iter().all(|row| row.margins() == reference)
    }

    /// A stable one-line identity for this table.
    ///
    /// Deterministic in `(root, seats, master, K, opts)` and nothing else — the
    /// reproduction check, not a summary, and not a serialization format
    /// (nothing parses it back). Every per-playout margin and world is folded
    /// in, so a table that moved a single replication's world moves this line.
    pub fn digest(&self) -> String {
        let rows: Vec<String> = self
            .rows
            .iter()
            .map(|row| {
                let cells: Vec<String> = row
                    .playouts
                    .iter()
                    .map(|p| {
                        format!(
                            "r{}@{:#018x}={}{}/{}d/{}",
                            p.replication,
                            p.world.0,
                            p.margin,
                            if p.won { "W" } else { "L" },
                            p.decisions,
                            p.final_digest,
                        )
                    })
                    .collect();
                format!("{:?}@{} [{}]", row.candidate, row.continuation_digest, cells.join(" "))
            })
            .collect();
        let seats: Vec<String> =
            self.provenance.seats.iter().map(|s| format!("p{}={}", s.seat, s.echo)).collect();
        format!(
            "adjudication-v1 rev={} root={}@{} mover=p{} master={:#018x} k={} seats=[{}] rows=[{}]",
            self.provenance.semantics_rev,
            self.provenance.root.origin.kind(),
            self.provenance.root.root_digest,
            self.mover.0,
            self.master.0,
            self.replications,
            seats.join(" | "),
            rows.join(" | "),
        )
    }
}

/// Why an adjudication produced no table.
///
/// [`DrivenError`]'s posture one level up: every variant is a hard stop that
/// produces nothing. A table missing a candidate — because its playout failed
/// and was skipped — would read exactly like a table of a position with fewer
/// options, and the reading riding on it would be wrong with no way to notice.
#[derive(Debug)]
pub enum AdjudicationError {
    /// The [`RootSource`] produced no root at all.
    Root(RootError),
    /// `replications` was 0. A table with no playouts is not a measurement, and
    /// silently promoting it to 1 would answer a question nobody asked.
    NoReplications,
    /// The root, the seat list or the options are not something this instrument
    /// can adjudicate — [`DrivenError`]'s own answers, raised BEFORE any
    /// candidate is forced (which is what makes them reachable on a terminal
    /// root, where no playout ever runs).
    Unadjudicable(DrivenError),
    /// The root is not terminal and offered no candidate — the livelock shape.
    /// Distinguished from the terminal empty table, which is an `Ok`.
    DeadEnd { seat: PlayerId },
    /// The engine refused a move IT had just listed as legal, or failed to apply
    /// it. An engine bug rather than an answer, and never a skipped row.
    ForcedMove { candidate: Box<Move>, detail: String },
    /// One candidate's playout did not complete.
    Playout { candidate: Box<Move>, replication: u32, error: Box<DrivenError> },
    /// `final_result` returned standings this module could not read the mover's
    /// margin out of. Unreachable through a supported seat count (every seat has
    /// a standing and every game has a rival), typed rather than panicking
    /// because an instrument that aborts is worse than one that refuses.
    Unscorable { seat: PlayerId },
    /// A TURN HOOK was supplied and the mover's own applied moves would not
    /// re-apply, so the hand-off the predicate is evaluated at could not be
    /// rebuilt (Phase C).
    ///
    /// An engine bug rather than an answer — these are moves the driver ALREADY
    /// applied in this same playout — and typed rather than `expect`ed on
    /// [`Unscorable`]'s grounds: an instrument that aborts is worse than one
    /// that refuses. Never a `None` verdict: a playout silently missing its
    /// verdict would land in no admission group and shrink a denominator
    /// invisibly.
    HandoffReplay { candidate: Box<Move>, replication: u32, detail: String },
}

impl std::fmt::Display for AdjudicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Root(e) => write!(f, "no root: {e}"),
            Self::NoReplications => {
                write!(f, "0 replications: a table with no playout is not a measurement")
            }
            Self::Unadjudicable(e) => write!(f, "the root cannot be adjudicated: {e}"),
            Self::DeadEnd { seat } => write!(
                f,
                "seat {} faces no candidate at a root that is not over (a livelock, not a \
                 finished game)",
                seat.0
            ),
            Self::ForcedMove { candidate, detail } => write!(
                f,
                "the engine listed {candidate:?} as legal at this root and then refused it: \
                 {detail}"
            ),
            Self::Playout { candidate, replication, error } => write!(
                f,
                "replication {replication} of {candidate:?} did not complete: {error}"
            ),
            Self::Unscorable { seat } => {
                write!(f, "the final result carries no margin for seat {}", seat.0)
            }
            Self::HandoffReplay { candidate, replication, detail } => write!(
                f,
                "replication {replication} of {candidate:?} played moves that do not re-apply, so \
                 the turn hand-off the turn predicate reads could not be rebuilt: {detail}"
            ),
        }
    }
}

impl std::error::Error for AdjudicationError {}

/// The mover's standing, as [`CandidatePlayout`] records it.
struct Verdict {
    margin: i64,
    mover_total: u32,
    best_rival: PlayerId,
    best_rival_total: u32,
    rank: u8,
    won: bool,
}

/// Read the mover's margin out of a finished game's result.
///
/// The standings are ALREADY in the official order (total descending, then
/// clockwise closeness to the start player), so the first non-mover row IS the
/// best rival by that same order — which is what makes the tie-break part of the
/// margin's definition rather than an afterthought applied to a `max`.
fn verdict(result: &GameResult, mover: PlayerId) -> Option<Verdict> {
    let mine = result.standings.iter().find(|s| s.player == mover)?;
    let rival = result.standings.iter().find(|s| s.player != mover)?;
    Some(Verdict {
        margin: i64::from(mine.breakdown.total) - i64::from(rival.breakdown.total),
        mover_total: mine.breakdown.total,
        best_rival: rival.player,
        best_rival_total: rival.breakdown.total,
        rank: mine.rank,
        won: result.winner == mover,
    })
}

/// Force every root candidate in turn and play the REMAINDER out, K times.
///
/// **Candidates are the engine's own [`get_legal_moves`] at the root, which is
/// QUEUE-FIRST** — so at a root with a pending choice the candidates are that
/// choice's resolutions rather than the phase's moves. That is correct and worth
/// saying out loud: the decision the mover actually faces there IS the queued
/// choice, and enumerating the phase's moves instead would adjudicate a decision
/// nobody is being asked to make.
///
/// Each candidate is checked with `is_move_legal(.., Canonical)` and applied
/// with `apply_move_unchecked` — the DRIVER's own discipline, not the checked
/// `apply_move` (which would re-enumerate the whole legal list per candidate for
/// an answer this function already holds). A move the engine listed and then
/// refuses is [`AdjudicationError::ForcedMove`], never a skipped row.
///
/// **Seeding is design §4's formula and nothing else**: replication `r` plays in
/// [`replication_world`]`(master, r)`, and each seat's stream descends from that
/// world through the arena's `agent_stream_seed`. **Every candidate of one
/// replication shares that replication's world and streams** — fresh agents per
/// candidate, identical seeds — so within a replication the candidates differ
/// only by the forced move, which is the whole point of pairing them.
///
/// **The seats list governs the WHOLE run, the forced mover included**: its own
/// spec decides its post-root moves exactly as it would have decided the root's,
/// and nothing here substitutes a different seat behind it.
///
/// **A TERMINAL root adjudicates to an EMPTY table, `Ok`** — zero candidates,
/// provenance still stamped, [`Adjudication::witness`] `None`. It is the same
/// contract [`drive_to_terminal`]'s zero-move outcome makes, one level up: a
/// finished game has nothing to force, and refusing would leave the instrument
/// unable to report the position an adjudication walks into whenever a candidate
/// ends the game.
///
/// **The root arrives as a [`RootSource`], not a bare state**, for
/// [`drive_rooted`]'s reason: the table names its root ONCE in provenance, and a
/// descriptor the adjudicator rebuilt from a state it was handed could never say
/// more than "someone gave me this position". A caller holding only a state
/// passes `RootSource::Constructed`, which is exactly what that means.
///
/// **COST, stated because nothing here bounds it.** This is
/// `candidates × K` whole games. The candidate count is the ENGINE's at that
/// root — one at a forced decision, ~5 at a Building point, and up to the
/// hundreds at a money-saturated Movement point (route_enum's ledger L9: max
/// OBSERVED 694 adversarial, 66 organic under the shipped rung, no ceiling
/// known) — so the sizing choices are the ROOT and the reply seats, not a knob
/// on this function. Rungs reply in milliseconds per game; an uncapped
/// `lookahead-v1` reply does not, and a caller putting one on both seats of a
/// wide root is asking for hours. `DrivenOpts::max_decisions` bounds each
/// playout's length, never the table's width.
///
/// **What each playout's own provenance says, and why it is not the root's.** A
/// playout starts at ROOT + one forced candidate — a state THIS MODULE built —
/// so its descriptor is `Constructed`, stamped on the state actually driven.
/// Threading the root's descriptor down instead would have every playout claim
/// it started at a position it did not start at, under an `event_index` that
/// indexes a different move. The link is kept where it is true: the root is
/// named ONCE on [`Adjudication::provenance`], with its full origin, and every
/// row carries [`CandidateRow::continuation_digest`] — the same digest the
/// playout's own provenance records — so a reader joins the two by value rather
/// than by trust.
pub fn adjudicate_root(
    source: &RootSource,
    seats: &[SeatSpec],
    rules: &Rules,
    master: WorldSeed,
    replications: u32,
    opts: DrivenOpts,
) -> Result<Adjudication, AdjudicationError> {
    adjudicate_root_with(source, seats, rules, master, replications, opts, None)
}

/// The MOVER'S FIRST TURN inside one playout, rebuilt by replay from the moves
/// the playout applied (Phase C, design §2's adjudicator hook).
///
/// **Why a replay and not an observer**: a playout runs through
/// `play_from_state`, which exposes no per-move hook, so the only honest way to
/// name the hand-off is the repo's canonical reconstruction pattern —
/// `record::state_at` and the CLI's `why` both rebuild a position by re-applying
/// a recorded move list, and this is that pattern scoped to one turn.
///
/// **`after` is ROOT + THE FORCED CANDIDATE**, so the candidate is decision ZERO
/// of the turn and is already applied when this starts. The walk then takes the
/// mover's applied moves up to the FIRST non-mover move, stopping early the
/// moment the turn ends (`current_player` moves off the mover, or the game ends)
/// — the same dual stop [`drive_turn`] documents, for the same reason: the
/// game-end arm of `advance_phase` never writes `current_player`, so neither
/// condition implies the other.
///
/// Cost: at most one turn of re-applies per playout, and only when a hook is
/// supplied.
fn turn_handoff_from_playout(
    after: &GameState,
    mover: PlayerId,
    applied: &[AppliedMove],
    rules: &Rules,
) -> Result<GameState, String> {
    let mut state = after.clone();
    // The forced candidate may BE the whole turn: a candidate that hands off (or
    // ends the game) leaves a mover-prefix of exactly nothing, and walking the
    // remainder's moves would then attribute the OPPONENT's turn to this mover.
    if is_game_over(&state) || state.current_player != mover {
        return Ok(state);
    }
    for step in applied {
        if step.seat != mover {
            break;
        }
        state = apply_move_unchecked(&state, &step.mv, rules).map_err(|e| e.to_string())?;
        if is_game_over(&state) || state.current_player != mover {
            break;
        }
    }
    Ok(state)
}

/// A TURN PREDICATE, as [`adjudicate_root_with`] takes it: `(root, hand-off) ->
/// bool`.
///
/// Named rather than spelled inline at the one call site, because the two
/// arguments are NOT interchangeable and a name is where that can be said: the
/// first is the ADJUDICATED ROOT, pre-candidate, and the second is where the
/// mover's first turn ENDED. `game::driven_families::TurnPredicate` is the
/// frozen vocabulary of rules that fit here; this alias keeps `driven` itself
/// from depending on that module.
pub type TurnHook<'a> = &'a dyn Fn(&GameState, &GameState) -> bool;

/// [`adjudicate_root`] with design §2's optional TURN-PREDICATE HOOK — the form
/// turn-grain admission (design §4) is computed from.
///
/// Every playout is stamped with [`CandidatePlayout::turn_verdict`]: whether the
/// mover's FIRST TURN out of this root satisfied `turn_hook`. `None` supplies no
/// hook and every verdict reads `None`, which is byte-for-byte what
/// [`adjudicate_root`] produced before this seam existed.
///
/// **THE HOOK'S FIRST ARGUMENT IS THE ADJUDICATED ROOT, PRE-CANDIDATE**, and the
/// rule is load-bearing rather than a convention. The mover's turn INCLUDES the
/// forced candidate — the candidate is its decision zero — so a predicate that
/// asks "did this turn claim an achievement?" must compare against the state
/// BEFORE the candidate. Handed the post-candidate state instead, a playout
/// whose forced candidate IS the claim would compare a grown claimed-set against
/// an already-grown one, read FALSE, and land in the losing group — inverting
/// exactly the dominance partition design §4's admission is computed from.
/// `the_claim_first_playout_is_a_true_verdict_not_a_false_one` pins it.
///
/// The second argument is the HAND-OFF: root + candidate + the mover's remaining
/// moves, rebuilt by [`turn_handoff_from_playout`].
///
/// A hook is a [`TurnHook`] — `&dyn Fn`, not a generic parameter — so that the
/// two entry points share ONE body: a generic would make `adjudicate_root`'s
/// delegation pick a turbofished stand-in type for "no hook", which is a second
/// thing to keep correct for no gain at one call per playout.
pub fn adjudicate_root_with(
    source: &RootSource,
    seats: &[SeatSpec],
    rules: &Rules,
    master: WorldSeed,
    replications: u32,
    opts: DrivenOpts,
    turn_hook: Option<TurnHook<'_>>,
) -> Result<Adjudication, AdjudicationError> {
    if replications == 0 {
        return Err(AdjudicationError::NoReplications);
    }
    let (root, descriptor) = source.resolve(rules).map_err(AdjudicationError::Root)?;
    // The ROOT's guards, run HERE rather than left to a playout: a terminal root
    // has no playout to carry them, and a corrupt root must be refused as the
    // root it is instead of surfacing as a fault attributed to a forced move.
    preflight(&root, seats, opts).map_err(AdjudicationError::Unadjudicable)?;
    check_invariants(&root, rules)
        .map_err(|e| AdjudicationError::Unadjudicable(DrivenError::CorruptRoot(e)))?;

    let mover = root.current_player;
    let candidates = get_legal_moves(&root, rules);
    if candidates.is_empty() && !is_game_over(&root) {
        return Err(AdjudicationError::DeadEnd { seat: mover });
    }

    let mut witness: Option<RootedProvenance> = None;
    let mut rows: Vec<CandidateRow> = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !is_move_legal(&root, &candidate, rules, PathMode::Canonical) {
            return Err(AdjudicationError::ForcedMove {
                candidate: Box::new(candidate),
                detail: "is_move_legal(Canonical) rejected a move from get_legal_moves".to_string(),
            });
        }
        let after = match apply_move_unchecked(&root, &candidate, rules) {
            Ok(state) => state,
            Err(e) => {
                return Err(AdjudicationError::ForcedMove {
                    candidate: Box::new(candidate),
                    detail: e.to_string(),
                })
            }
        };
        // ONE descriptor per candidate, built once and reused across the K
        // playouts — which is also why this goes through the private seam rather
        // than `drive_to_terminal`: the latter would re-digest `after` K times
        // for a value this row already holds.
        let continuation = RootDescriptor::constructed(&after);
        let continuation_digest = continuation.root_digest.clone();

        let mut playouts = Vec::with_capacity(replications as usize);
        for replication in 0..replications {
            let world = replication_world(master, replication);
            let outcome = drive_from_descriptor(
                &after,
                continuation.clone(),
                seats,
                rules,
                world,
                opts,
            )
            .map_err(|e| AdjudicationError::Playout {
                candidate: Box::new(candidate.clone()),
                replication,
                error: Box::new(e),
            })?;
            let v = verdict(&outcome.result, mover)
                .ok_or(AdjudicationError::Unscorable { seat: mover })?;
            // The TURN VERDICT, derived from the moves this playout applied —
            // and evaluated against the PRE-CANDIDATE root, per this function's
            // doc. Skipped entirely without a hook, so the hook-free path pays
            // not one re-apply.
            let turn_verdict = match turn_hook {
                Some(hook) => {
                    let handoff =
                        turn_handoff_from_playout(&after, mover, &outcome.applied, rules).map_err(
                            |detail| AdjudicationError::HandoffReplay {
                                candidate: Box::new(candidate.clone()),
                                replication,
                                detail,
                            },
                        )?;
                    Some(hook(&root, &handoff))
                }
                None => None,
            };
            if witness.is_none() {
                witness = Some(outcome.provenance.clone());
            }
            playouts.push(CandidatePlayout {
                replication,
                world,
                agent_seeds: outcome.seats.iter().map(|s| s.agent_seed).collect(),
                margin: v.margin,
                mover_total: v.mover_total,
                best_rival: v.best_rival,
                best_rival_total: v.best_rival_total,
                rank: v.rank,
                won: v.won,
                decisions: outcome.decisions,
                turns: outcome.final_state.turn_number,
                final_digest: state_digest(&outcome.final_state),
                turn_verdict,
            });
        }
        rows.push(CandidateRow { candidate, continuation_digest, playouts });
    }

    let seat_descriptors = seats
        .iter()
        .enumerate()
        .map(|(index, spec)| AdjudicationSeat {
            seat: index as u8,
            requested: spec.requested_label(),
            echo: spec.echo(),
        })
        .collect();
    Ok(Adjudication {
        mover,
        root_turn: root.turn_number,
        master,
        replications,
        rows,
        witness,
        provenance: AdjudicationProvenance {
            root: descriptor,
            semantics_rev: SEMANTICS_REV,
            rules_hash: rules.content_hash(),
            seats: seat_descriptors,
            master: master.0,
            replications,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    // The CHECKED boundary, imported only here: the module itself drives through
    // `apply_move_unchecked` (moves already drawn from the legal list), so a
    // test recomputing a continuation independently uses the checked entry point
    // rather than a second call to the one under test.
    use crate::game::apply_move::apply_move;
    use crate::game::arena::{
        run_arena_game, ArenaGameStats, ControllerProvenance, DecisionReport, NamedIdentity,
    };
    use crate::game::end_game::is_game_over;
    use crate::game::invariants::check_invariants;
    use crate::game::record::{state_at, RecordedEvent};
    use crate::game::scoring::{ScoreBreakdown, Standing};
    use crate::game::seeds::SeedSchedule;
    use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
    use crate::types::{
        AreaAction, BuildingSite, EndGameTrigger, GameConfig, MercenaryState, PlayerView,
        PresidentLocation, TradingHouseState, TurnPhase,
    };
    use std::collections::BTreeMap;
    use std::cell::RefCell;
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
            Ok(Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT)),
        );
        assert_eq!(ReplyPolicy::GreedyAtFinal.contestant(), Ok(Contestant::GREEDY_V1_DEFAULT));
        assert_eq!(ReplyPolicy::GreedyAtFinal.contestant(), Ok(Contestant::GreedyV1 { m: 8 }));
        assert_eq!(
            ReplyPolicy::LookaheadAtFinal.contestant(),
            Ok(Contestant::LOOKAHEAD_V1_DEFAULT),
        );
        assert!(
            matches!(
                ReplyPolicy::LookaheadAtFinal.contestant(),
                Ok(Contestant::LookaheadV1 { cap: None, .. })
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

        // The root PREDICATE's `!is_game_over` conjunct, pinned at the one place
        // a terminal state is already in hand (review N-4): the generated arm
        // only ever sees positions that satisfy the whole predicate, so without
        // this the conjunct is asserted nowhere and dropping it would be silent.
        // The first line is the anti-vacuity — a state that never triggered the
        // end game would be refused by the OTHER conjuncts and say nothing.
        assert!(
            terminal.end_game_triggered && terminal.final_round,
            "anti-vacuity: this terminal state never reached the final round",
        );
        assert!(
            !RootPredicate::FinalRoundAlreadyTriggered.holds(&terminal, &rules),
            "a state that is OVER satisfied the root predicate — nobody decides there",
        );

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

        // T2-review NOTE-3, pinned on the terminal state already in hand: the
        // OTHER instrument answers the same position differently, and
        // `RootTableError::Agent`'s own doc is where that claim lives ("which is
        // what an ALREADY-TERMINAL root arrives as: no candidates, hence a dead
        // end, hence no table"). A doc-only claim about an error arm is a claim
        // nothing checks — and the contrast is the point: `drive_to_terminal`
        // answers a terminal root with an empty run because a remainder of
        // nothing is a real answer, while `root_table` refuses because a
        // decision that does not exist has no ranking.
        let err = root_table(&terminal, terminal.current_player, Contestant::LOOKAHEAD_V1_DEFAULT, &rules, DECISION_SEED)
            .expect_err("a terminal root has no decision to tabulate");
        assert!(
            matches!(err, RootTableError::Agent { seat, .. } if seat == terminal.current_player),
            "expected Agent at the terminal root's own seat, got {err:?}",
        );
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
            // The V4 step-8 named identities. They are the arms that WOULD
            // work — a named arm is a `LookaheadAgent` carrying the same
            // ranked-root surface `lookahead-v1` carries — so nothing but this
            // row stops a re-route or an absent-minded admission from putting a
            // weight identity into the driven runner's grammar, which V3 §5.1
            // reserves to the scan ("never the runner grammar").
            (
                Contestant::NamedTable(NamedIdentity::V2Padded),
                UnsupportedReason::AlternateTable,
            ),
            (
                Contestant::NamedTable(NamedIdentity::V2RefitPadded),
                UnsupportedReason::AlternateTable,
            ),
            (
                Contestant::NamedTable(NamedIdentity::V4Step9Draft1),
                UnsupportedReason::AlternateTable,
            ),
        ];
        // All three arms are enumerated above, and a FOURTH arm must be dispositioned
        // here rather than inheriting a refusal nothing checked.
        assert_eq!(
            expected
                .iter()
                .filter(|(c, _)| matches!(c, Contestant::NamedTable(_)))
                .count(),
            NamedIdentity::ALL.len(),
            "every named identity must have a row here",
        );
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
            assert_eq!(d.echo, spec.echo(), "seat {index} echoes another contestant");
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
            // The two CONTESTANT-only claims. A harness-rule seat (T3's
            // race-rusher) has no contestant to reconstruct and no
            // `Contestant::descriptor` to agree with — which is the whole point
            // of the second materialization arm — so it is skipped HERE rather
            // than weakened for every seat; its own descriptor is pinned by
            // `a_race_rusher_seat_is_described_by_the_driver_that_seated_it`.
            let Ok(contestant) = spec.contestant() else { continue };
            // The RECONSTRUCTION property: the two record halves this descriptor
            // carries are enough to rebuild the contestant, knobs and all — which
            // is what makes the descriptor an identity rather than a label.
            assert_eq!(
                Contestant::from_parts(&d.controller, &d.params),
                Ok(contestant),
                "seat {index}'s controller+params do not reconstruct what played",
            );
            // And the AGENT's own descriptor is the CONTESTANT's: these come from
            // two different places (the built agent reported the first, the seat
            // spec claims the second), so a contestant whose `build` ever routed
            // to a differently-versioned or differently-knobbed agent would show
            // up here rather than in a report nobody cross-checks.
            let (name, version, params) = contestant.descriptor();
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

    /// A PRE-RESOLVED root hands back exactly what it was given (Phase C's T5).
    ///
    /// The variant's whole job is to let a block resolve its draws ONCE and then
    /// adjudicate them under a whole reply menu without re-scanning, so the
    /// property that matters is that `resolve` is the IDENTITY on it — state and
    /// descriptor both, byte for byte, including a descriptor whose origin is a
    /// seeded scan the caller did earlier. The second half is the discriminator:
    /// the same state wrapped as `Constructed` resolves to a DIFFERENT
    /// descriptor, which is exactly the provenance downgrade this variant exists
    /// to avoid.
    #[test]
    fn a_pre_resolved_root_resolves_to_itself_descriptor_included() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(46).0);
        let descriptor = RootDescriptor {
            origin: RootOrigin::Generated {
                predicate: RootPredicate::FinalRoundAlreadyTriggered,
                master: 0x0C04_2026_0818_9991,
                ordinal: 3,
                game_index: 1,
                ecology: GeneratingEcology::v1(),
            },
            event_index: Some(211),
            root_digest: state_digest(&root),
        };

        let (state, resolved) = RootSource::Resolved {
            state: root.clone(),
            descriptor: descriptor.clone(),
        }
        .resolve(&rules)
        .expect("a pre-resolved source resolves to itself");
        assert_eq!(state, root, "resolving a pre-resolved source moved the state");
        assert_eq!(resolved, descriptor, "the descriptor did not survive the round trip");

        // THE DISCRIMINATOR: the `Constructed` alternative loses the origin.
        let (_, downgraded) =
            RootSource::Constructed(root).resolve(&rules).expect("constructed resolves");
        assert_eq!(downgraded.origin, RootOrigin::Constructed);
        assert_ne!(downgraded, descriptor, "the two arms describe the root identically");
    }

    /// **A MIS-PAIRED pre-resolved root REFUSES BY NAME** (Phase C's T5 fix
    /// wave, review SHOULD-2).
    ///
    /// [`RootSource::Resolved`] is the one arm whose descriptor the caller
    /// supplies, so it is the one arm whose two halves can describe different
    /// positions — and nothing downstream could ever tell: every table, digest
    /// and sidecar would name the wrong root perfectly consistently. The check
    /// is the Phase-B loader's `RegistryError::Stale` posture at the identity
    /// level, and this is its red path.
    ///
    /// The GREEN half rides beside it deliberately: the same descriptor, paired
    /// with the state it actually describes, resolves unchanged — so the cell
    /// separates "the pair disagrees" from "the arm refuses".
    #[test]
    fn a_mis_paired_pre_resolved_root_refuses_naming_both_digests() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(46).0);
        let other = root_2p(&rules, world(47).0);
        assert_ne!(state_digest(&root), state_digest(&other), "the two fixtures are one position");

        let descriptor = RootDescriptor {
            origin: RootOrigin::Generated {
                predicate: RootPredicate::FinalRoundAlreadyTriggered,
                master: 0x0C04_2026_0818_9992,
                ordinal: 1,
                game_index: 0,
                ecology: GeneratingEcology::v1(),
            },
            event_index: Some(97),
            root_digest: state_digest(&root),
        };

        // GREEN: the pair that came from one resolution is untouched.
        let (state, back) =
            RootSource::Resolved { state: root.clone(), descriptor: descriptor.clone() }
                .resolve(&rules)
                .expect("a well-paired root resolves");
        assert_eq!(state, root);
        assert_eq!(back, descriptor);

        // RED: the SAME descriptor over another state.
        let err = RootSource::Resolved { state: other.clone(), descriptor: descriptor.clone() }
            .resolve(&rules)
            .expect_err("a descriptor describing another position must refuse");
        match err {
            RootError::ResolvedMismatch {
                origin,
                ref descriptor_digest,
                state_digest: ref actual,
            } => {
                assert_eq!(origin, "generated", "the refusal misreports the origin kind");
                assert_eq!(descriptor_digest, &descriptor.root_digest);
                assert_eq!(actual, &state_digest(&other));
            }
            ref other => panic!("expected ResolvedMismatch, got {other:?}"),
        }
        // The sentence names BOTH positions, which is what makes it actionable.
        let text = err.to_string();
        assert!(text.contains(&descriptor.root_digest), "the refusal hides the descriptor: {text}");
        assert!(text.contains(&state_digest(&other)), "the refusal hides the state: {text}");
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

        // A KNOBLESS seat's descriptor reads back without a `params` key at all
        // (review N-7) — `SeatProvenance::params`' own rule, carried here. The
        // empty map is the commonest value this field takes, so a sidecar
        // written without it must default rather than fail at parse.
        let params_less = r#"{
            "seat": 0,
            "requested": "reply:family-weighted",
            "echo": "family-weighted",
            "controller": "family-weighted",
            "version": 1,
            "observation": "view",
            "agent_seed": 7
        }"#;
        let knobless: SeatDescriptor =
            serde_json::from_str(params_less).expect("a params-less seat descriptor parses");
        assert!(knobless.params.is_empty(), "the absent knob map did not default to empty");
        assert_eq!(knobless.seat, 0);
        assert_eq!(knobless.agent_seed, 7);

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
    ///
    /// **Plus the ANCHOR** (review probe-3): those three halves are all
    /// RELATIVE, so an ordinal counter that started at the wrong hit satisfies
    /// every one of them — it reproduces, it moves with the ordinal, it moves
    /// with the master, and every position it returns satisfies the predicate.
    /// What pins the ordinal ABSOLUTELY is an independent construction of where
    /// ordinal 0 must be: the first predicate-satisfying position of the first
    /// game of the schedule, rebuilt here without `generate_root`.
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
        assert!(
            predicate.holds(&first, &rules),
            "the scan returned a position the predicate rejects",
        );
        assert!(first.end_game_triggered && first.final_round, "not a final-round position");
        assert!(!is_game_over(&first), "a terminal state is not a root anyone can decide at");
        assert_eq!(descriptor.root_digest, state_digest(&first));
        assert!(descriptor.event_index.is_some(), "a scanned root sits at a position");
        assert!(check_invariants(&first, &rules).is_ok(), "the scan produced a corrupt root");

        match &descriptor.origin {
            RootOrigin::Generated { predicate: p, master, ordinal, ecology, game_index } => {
                assert_eq!(*p, predicate);
                assert_eq!(*master, GEN_MASTER);
                assert_eq!(*ordinal, 0);
                assert_eq!(ecology, &GeneratingEcology::v1(), "the ecology is not v1's");
                assert_eq!(*game_index, 0, "ordinal 0 came from a later game than the first");
            }
            other => panic!("expected a Generated origin, got {other:?}"),
        }

        // THE ANCHOR. The first game of `GEN_MASTER`'s schedule, rebuilt from
        // the ecology's own parts rather than through `generate_root`, folded
        // one event at a time to its FIRST predicate-satisfying position: that
        // position IS ordinal 0, by definition of the ordinal. An off-by-one
        // counter — the scan serving hit n+1 for ordinal n — passes every other
        // assertion in this cell and fails here.
        let mut schedule = SeedSchedule::diagnostic(GEN_MASTER);
        let first_world = schedule.next_world();
        let mut make = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
            GENERATED_SEAT.build(&rules, agent_stream_seed(first_world, seat, Slot(seat.0))).0
        };
        let first_game = run_arena_game(first_world, GENERATED_PLAYERS, &rules, &mut make, true)
            .expect("the ecology's first game must complete");
        let mut walk =
            state_at(&first_game.record, 0, &rules).expect("position 0 of any record replays");
        let mut anchor = None;
        for position in 0..=first_game.record.events.len() {
            if predicate.holds(&walk, &rules) {
                anchor = Some((position, state_digest(&walk)));
                break;
            }
            if let Some(event) = first_game.record.events.get(position) {
                walk = replay_from(&walk, std::slice::from_ref(event), &rules)
                    .expect("the generated game replays event by event");
            }
        }
        let (anchor_position, anchor_digest) =
            anchor.expect("anti-vacuity: the first generated game holds no final-round position");
        assert_eq!(
            descriptor.root_digest, anchor_digest,
            "ordinal 0 is not the FIRST predicate-satisfying position of the FIRST game — the \
             scan's counting starts somewhere else",
        );
        assert_eq!(
            descriptor.event_index,
            Some(anchor_position),
            "ordinal 0's recorded position is not the anchor's",
        );

        // The ordinal is READ.
        let (next, _) = RootSource::Generated { predicate, master: GEN_MASTER, ordinal: 1 }
            .resolve(&rules)
            .expect("the scan supplies more than one root");
        assert_ne!(
            state_digest(&next),
            state_digest(&first),
            "ordinal 1 returned ordinal 0's root, so the ordinal selects nothing",
        );
        assert!(predicate.holds(&next, &rules));

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
        let rules = Rules::standard();
        let mut entry = registry().into_iter().next().expect("the registry is not empty");
        entry.path = "docs/temp/owner-games/__no-such-record__.json";
        assert!(!entry.resolved_path().exists(), "the fixture path must not exist");

        let err = entry.load(&rules).expect_err("an absent record must be a miss, not a root");
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

    /// EVERY OTHER ARM of the loader, on files this cell writes itself.
    ///
    /// **Why it exists** (review probe-8): every shipped entry points at a
    /// gitignored save, so `Missing` was the only arm any cell could reach —
    /// and a check nothing exercises is a check that can be DELETED with the
    /// suite still green. Probe 8 removed the recorded-move comparison and
    /// nothing went red. So the loader takes its anchor as an argument
    /// ([`RegistryEntry::load_from`]) and this cell supplies a temp-dir one,
    /// which keeps the suite CORPUS-FREE in the strong sense: it depends on no
    /// file it did not create, and it removes what it created.
    ///
    /// Six readings off one fixture record: the matching entry LOADS, a wrong
    /// recorded move is `MoveMismatch`, an index past the log is
    /// `EventOutOfRange`, a file that is not a record is `Parse`, a DIRECTORY at
    /// the path is `Read` (there, and unreadable — never the expected miss), and
    /// a record stamped at another `semantics_rev` is `Stale` BEFORE either
    /// transcription check gets to read the log.
    #[test]
    fn the_registry_loader_names_every_way_a_file_can_be_wrong() {
        /// Removes the whole temp anchor when the cell leaves, pass or panic.
        struct TempRoot(PathBuf);
        impl Drop for TempRoot {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        let rules = Rules::standard();
        let record = rung_game(&rules, world(47)).record;
        assert!(record.events.len() > 3, "anti-vacuity: the fixture game recorded nothing");
        assert_eq!(record.semantics_rev, Some(SEMANTICS_REV), "the fixture is not current");

        // A name nothing else can collide with, under the OS temp dir and never
        // under the workspace.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let anchor = TempRoot(
            std::env::temp_dir()
                .join(format!("yokohama-driven-registry-{}-{nanos}", std::process::id())),
        );
        std::fs::create_dir_all(&anchor.0).expect("the temp anchor is creatable");

        const GOOD: &str = "record.json";
        const NOT_A_RECORD: &str = "not-a-record.json";
        const STALE: &str = "stale.json";
        const A_DIRECTORY: &str = "a-directory.json";

        let write = |name: &str, text: &str| {
            std::fs::write(anchor.0.join(name), text).expect("the fixture file is writable");
        };
        write(GOOD, &serde_json::to_string(&record).expect("the fixture record serializes"));
        write(NOT_A_RECORD, r#"{"this":"is not a GameRecord"}"#);
        let mut stale = record.clone();
        stale.semantics_rev = Some(SEMANTICS_REV + 7);
        write(STALE, &serde_json::to_string(&stale).expect("the stale record serializes"));
        std::fs::create_dir_all(anchor.0.join(A_DIRECTORY)).expect("the directory is creatable");

        let k = record
            .events
            .iter()
            .position(|e| matches!(e, RecordedEvent::Move(_)))
            .expect("the fixture game recorded a move");
        let recorded = match &record.events[k] {
            RecordedEvent::Move(mv) => mv.clone(),
            other => panic!("event {k} is {other:?}, not a move"),
        };
        let entry = |path: &'static str, event_index: usize, recorded_move: Option<Move>| {
            RegistryEntry {
                name: "temp-fixture",
                path,
                event_index,
                bot_seat: SESSION_BOT_SEAT,
                decision_seed: DECISION_SEED,
                recorded_move,
                note: "a cell-owned fixture",
            }
        };

        // (0) The MATCHING entry loads — the anti-vacuity every refusal below
        // rests on, since a loader that refused everything would satisfy them
        // all.
        match entry(GOOD, k, Some(recorded.clone())).load_from(&anchor.0, &rules) {
            Ok(RootSource::RecordPrefix { record: loaded, event_index }) => {
                assert_eq!(event_index, k, "the source names another position");
                assert_eq!(loaded, record, "the loader returned another document");
            }
            other => panic!("a matching entry must load a prefix source, got {other:?}"),
        }

        // (1) A recorded move the file does not carry there.
        let wrong = Move::BuildShophouse { slot: 0 };
        assert_ne!(wrong, recorded, "anti-vacuity: the mismatch fixture IS the recorded move");
        let err = entry(GOOD, k, Some(wrong.clone()))
            .load_from(&anchor.0, &rules)
            .expect_err("a recorded move the record does not carry is a refusal");
        match err {
            RegistryError::MoveMismatch { name, event_index, ref expected, ref found } => {
                assert_eq!(name, "temp-fixture");
                assert_eq!(event_index, k);
                assert_eq!(**expected, wrong, "the refusal misnames what was asked for");
                assert!(
                    found.contains(&format!("{recorded:?}")),
                    "the refusal does not show what the file holds: {found}",
                );
            }
            ref other => panic!("expected MoveMismatch, got {other:?}"),
        }
        let rendered = err.to_string();
        assert!(rendered.contains("temp-fixture"), "the refusal hides the entry: {rendered}");

        // (2) An index past the log.
        let past = record.events.len() + 5;
        let err = entry(GOOD, past, None)
            .load_from(&anchor.0, &rules)
            .expect_err("an index past the log is a refusal");
        match err {
            RegistryError::EventOutOfRange { event_index, events, .. } => {
                assert_eq!((event_index, events), (past, record.events.len()));
            }
            ref other => panic!("expected EventOutOfRange, got {other:?}"),
        }

        // (3) A file that is not a record.
        let err = entry(NOT_A_RECORD, 0, None)
            .load_from(&anchor.0, &rules)
            .expect_err("a non-record file is a refusal");
        match err {
            RegistryError::Parse { ref path, ref detail, .. } => {
                assert_eq!(path, &anchor.0.join(NOT_A_RECORD));
                assert!(!detail.is_empty(), "the parse refusal says nothing about why");
            }
            ref other => panic!("expected Parse, got {other:?}"),
        }

        // (4) A DIRECTORY at the path: THERE, and unreadable. The distinction
        // this arm defends is against `Missing` — a runner told "no record at
        // this path" walks past something that is actually broken.
        let err = entry(A_DIRECTORY, 0, None)
            .load_from(&anchor.0, &rules)
            .expect_err("a directory at the path is a refusal");
        match err {
            RegistryError::Read { ref path, ref detail, .. } => {
                assert_eq!(path, &anchor.0.join(A_DIRECTORY));
                assert!(!detail.is_empty(), "the read refusal says nothing about why");
            }
            ref other => panic!("expected Read, got {other:?}"),
        }

        // (5) A record from another rev — and the ORDER: this entry's index is
        // past the log too, so a staleness check running AFTER transcription
        // would report `EventOutOfRange` and blame the entry for the engine
        // having moved.
        let err = entry(STALE, past, None)
            .load_from(&anchor.0, &rules)
            .expect_err("a record from another rev is a refusal");
        match err {
            RegistryError::Stale {
                name,
                record_rev,
                running_rev,
                ref record_hash,
                ref running_hash,
            } => {
                assert_eq!(name, "temp-fixture");
                assert_eq!(record_rev, Some(SEMANTICS_REV + 7));
                assert_eq!(running_rev, SEMANTICS_REV);
                assert_eq!(record_hash, &rules.content_hash(), "the rules DATA agrees");
                assert_eq!(running_hash, &rules.content_hash());
            }
            ref other => panic!("expected Stale, got {other:?}"),
        }
        let rendered = err.to_string();
        assert!(rendered.contains(&(SEMANTICS_REV + 7).to_string()), "no record rev: {rendered}");
        assert!(rendered.contains(&SEMANTICS_REV.to_string()), "no running rev: {rendered}");
        assert!(rendered.contains(&rules.content_hash()), "no rules hash: {rendered}");

        // The cell owns every file it read; the anchor goes when it drops.
        drop(anchor);
    }

    // =======================================================================
    // T4: the adjudicator
    // =======================================================================

    /// A CONSTRUCTED near-terminal 2P root whose final margins are computable
    /// BY HAND — the `scenario_suite` grain (a doctored `GameState` put through
    /// `check_invariants` before anything reads it), built for exactly one
    /// question: what does forcing each root candidate do to the score?
    ///
    /// **The position.** 2P, the final round already under way, and the seat on
    /// the move is the LAST seat of that round — `(start + n - 1) % n`, the seat
    /// `GameState::is_game_over` keys terminality on. It sits in `Building` at
    /// action power 4, its President on the one area this fixture gives a
    /// doctored building site, two shophouses in hand (a fresh setup's
    /// `ShophouseState::new`) and none built. So `generate_building_moves`
    /// offers exactly FIVE candidates: `SkipBuilding`, then one
    /// `BuildShophouse` per empty slot — all four, since nothing is built.
    /// `BuildTradingHouse` is NOT among them: the seat holds 0 trading houses in
    /// hand (they reach the hand only through the Employment Agency).
    ///
    /// **Why the remainder is FORCED, which is what makes the margins
    /// hand-computable at all.** Every candidate leaves `turn_phase` at
    /// `Recovery`, which the same `apply_move_unchecked` resolves before it
    /// returns (`resolve_recovery`; no Telephone owned, so no `RecoveryLeave`
    /// choice defers it) and advances to `AdditionalActionSecond`. There
    /// `generate_additional_action_moves` offers `EndAdditionalPhase` plus
    /// mercenary uses plus University/Spinning-Mill actions — and this seat has
    /// no mercenaries, no technologies and no assistants on board, so the list
    /// is exactly `[EndAdditionalPhase]`. Applying it runs `advance_phase`,
    /// whose `final_round && next_player == start_player` arm assigns `TurnEnd`
    /// without moving `current_player`, and `is_game_over` is then true.
    /// **ONE forced decision after every candidate** — so these margins do not
    /// depend on which policy replies, on the master, or on the replication.
    ///
    /// **Why each seat's total IS its printed VP.** `final_result` adds five
    /// legs to `victory_points`, and this fixture zeroes every one of them for
    /// both seats:
    /// * CHURCH and CUSTOMS majorities — neither real seat has a track pawn, so
    ///   the only ranked competitor is the 2P setup dummy, which TAKES first
    ///   place and is awarded nothing (`award_top_two`, finding D1); second
    ///   place is an absent row. 0 and 0.
    /// * TECHNOLOGY majority — neither seat owns a technology, so both rank at
    ///   doubled industry value 0 behind the dummy's 15 (D5), and the
    ///   participation gate `doubled_iv > 0` denies the seat that inherits
    ///   second place. 0.
    /// * COUNTRY sets — no completed orders and no technologies is no country
    ///   icons at all, and `calculate_country_bonus` needs 2 distinct. 0.
    /// * REMAINDER — `calculate_remainder_bonus` is exactly unused mercenaries
    ///   + imported goods + money/2 + trade goods/3, and all four are zeroed.
    ///
    /// So a seat's final total is its `victory_points`, and building a shophouse
    /// adds exactly the doctored space's printed `vp`: the site's spaces carry
    /// no yen, no goods and no icons, so `apply_space_reward` queues nothing,
    /// and the Mining and Gaslight bonuses need technologies this seat does not
    /// own.
    ///
    /// Returns the root, the seat on the move, and the site's area.
    fn near_terminal_building_root(
        rules: &Rules,
        seed: u64,
        space_vp: [u8; 4],
        mover_vp: u32,
        rival_vp: u32,
    ) -> (GameState, PlayerId, AreaId) {
        let mut state = root_2p(rules, seed);
        let n = state.config.player_count;
        let start = state.config.start_player.0;
        // The seat `is_game_over` keys on — derived, never the literal 1, so the
        // fixture states the RULE rather than one board's answer to it.
        let mover = PlayerId((start + n - 1) % n);
        let area = *state
            .board_layout
            .building_sites
            .keys()
            .next()
            .expect("the 2P board carries at least one building site");
        // A site whose four spaces are PURE VP and whose trading-house space
        // pays nothing: `BuildingSite::new` is the engine's own VP-only
        // constructor, so no reward icon can sneak in through a hand-written
        // literal.
        state.board_layout.building_sites.insert(area, BuildingSite::new(space_vp, 0));

        for p in state.players.iter_mut() {
            p.money = 0;
            p.copper = 0;
            p.silk = 0;
            p.tea = 0;
            p.seafood = 0;
            p.imported_goods = 0;
            p.mercenaries = MercenaryState::new();
            p.victory_points = if p.id == mover { mover_vp } else { rival_vp };
        }
        {
            let m = state.player_mut(mover);
            m.president_location = PresidentLocation::OnBoard(area);
            // Explicit rather than assumed: 0 in hand is what keeps
            // `BuildTradingHouse` off the candidate list, and the cell asserts
            // the list, so a data change that put a token in hand at setup would
            // fail there rather than silently re-shape the table.
            m.trading_houses = TradingHouseState::new();
        }
        state.current_player = mover;
        state.turn_phase = TurnPhase::Building;
        state.current_action_power = Some(4);
        state.is_mercenary_action = false;
        state.mercenary_target_area = None;
        state.president_start_area = None;
        state.pending_choices.clear();
        state.end_game_triggered = true;
        state.final_round = true;
        state.trigger_reason = Some(EndGameTrigger::ChurchTrackFull);
        state.trigger_player = Some(state.config.start_player);

        check_invariants(&state, rules)
            .expect("the doctored root must be a state the engine accepts");
        assert!(!is_game_over(&state), "anti-vacuity: the root is already terminal");
        (state, mover, area)
    }

    /// The same fixture, one field further on: `TurnEnd` at the same seat, which
    /// is `is_game_over`'s whole definition. Cheaper than playing a game out and
    /// exact about WHY it is terminal.
    fn terminal_building_root(rules: &Rules, seed: u64) -> (GameState, PlayerId) {
        let (mut state, mover, _) = near_terminal_building_root(rules, seed, [0; 4], 6, 10);
        state.turn_phase = TurnPhase::TurnEnd;
        check_invariants(&state, rules).expect("a terminal state is still a valid state");
        assert!(is_game_over(&state), "the fixture must actually be a finished game");
        (state, mover)
    }

    /// The same fixture one phase later: `AdditionalActionSecond`, where the
    /// mover's ONLY candidate is `EndAdditionalPhase` and applying it ends the
    /// game outright — so the remainder after the forced move is EMPTY.
    fn last_move_root(rules: &Rules, seed: u64, mover_vp: u32, rival_vp: u32) -> GameState {
        let (mut state, _, _) =
            near_terminal_building_root(rules, seed, [0; 4], mover_vp, rival_vp);
        state.turn_phase = TurnPhase::AdditionalActionSecond;
        check_invariants(&state, rules).expect("still a state the engine accepts");
        assert!(!is_game_over(&state));
        state
    }

    /// The five candidates the fixture's Building root offers, in the
    /// generator's own order.
    fn expected_building_candidates() -> Vec<Move> {
        vec![
            Move::SkipBuilding,
            Move::BuildShophouse { slot: 0 },
            Move::BuildShophouse { slot: 1 },
            Move::BuildShophouse { slot: 2 },
            Move::BuildShophouse { slot: 3 },
        ]
    }

    /// Two rung seats — the cheapest honest reply there is, and the right choice
    /// wherever the property under test is STRUCTURAL (design §6's wall
    /// discipline).
    fn rung_seats() -> [SeatSpec; 2] {
        [SeatSpec::Reply(ReplyPolicy::FamilyWeighted), SeatSpec::Reply(ReplyPolicy::FamilyWeighted)]
    }

    // -----------------------------------------------------------------------
    // The replication derivation
    // -----------------------------------------------------------------------

    /// Design §4's formula, asserted rather than commented: a replication's
    /// world is `mix(SALT_DRIVEN_REPLICATION, [master, replication_index])`.
    ///
    /// Recomputed from the arena's OWN mixing authority and the salt const —
    /// not from a second copy of the formula — so this fails if the derivation
    /// is ever re-routed through a different salt, a different component order,
    /// or a hand-rolled hash. The distinctness half is what keeps replications
    /// from being K copies of one run.
    #[test]
    fn a_replications_world_is_the_designs_mix_of_master_and_index() {
        let master = world(40);
        let derived: Vec<WorldSeed> = (0..6).map(|r| replication_world(master, r)).collect();
        for (r, got) in derived.iter().enumerate() {
            assert_eq!(
                *got,
                WorldSeed(mix(SALT_DRIVEN_REPLICATION, &[master.0, r as u64])),
                "replication {r} did not derive through the design's formula",
            );
        }
        let mut seen = derived.clone();
        seen.sort_unstable_by_key(|w| w.0);
        seen.dedup();
        assert_eq!(seen.len(), derived.len(), "two replications drew the same world");

        // Injective in the MASTER too, at a fixed index — without this the
        // whole table would be master-independent and a recorded master would
        // identify nothing.
        let elsewhere = WorldSeed(master.0 ^ 0x5555_5555_5555_5555);
        assert_ne!(replication_world(master, 3), replication_world(elsewhere, 3));
    }

    // -----------------------------------------------------------------------
    // The pure readers: the margin's definition and the equality helper
    // -----------------------------------------------------------------------

    /// **THE BEST-RIVAL DISCRIMINATOR.** The margin is taken against the
    /// highest-RANKED rival, which on a level board is NOT what a `max` over
    /// totals returns.
    ///
    /// Hand-built and 3P, because that is the only shape where the two readings
    /// come apart: three seats on the SAME total, `final_result`'s official
    /// order putting p1 first, the mover p0 second and p2 third. The
    /// standings-order read answers p1 — the seat the tie-break actually ranks
    /// ahead — while `max_by_key(total)` over the non-movers answers p2, since
    /// `Iterator::max_by_key` returns the LAST maximum. Both readings are taken
    /// here, so the fixture proves it discriminates rather than merely claiming
    /// it does.
    ///
    /// A game costs nothing here on purpose: this is a property of
    /// [`verdict`]'s reading of a result, and driving a real game to a
    /// three-way tie is neither cheap nor reliable.
    #[test]
    fn the_best_rival_is_the_highest_ranked_rival_not_a_max_over_totals() {
        // `total` is the only leg this reading touches; the rest are zeroed so
        // nothing else can be the reason an assertion passes.
        let scored = |total: u32| ScoreBreakdown {
            base_vp: total,
            church: 0,
            customs: 0,
            technology: 0,
            country: 0,
            remainder: 0,
            total,
        };
        let mover = PlayerId(0);
        // Standings arrive in the OFFICIAL order — `final_result` sorts by
        // (total desc, clockwise distance) — so this literal is what a genuine
        // three-way tie broken in p1's favour looks like.
        let result = GameResult {
            standings: vec![
                Standing { player: PlayerId(1), breakdown: scored(30), rank: 1 },
                Standing { player: mover, breakdown: scored(30), rank: 2 },
                Standing { player: PlayerId(2), breakdown: scored(30), rank: 3 },
            ],
            winner: PlayerId(1),
        };

        let v = verdict(&result, mover).expect("the mover has a standing");
        assert_eq!(
            v.best_rival,
            PlayerId(1),
            "the margin must be taken against the highest-RANKED rival",
        );
        assert_eq!((v.mover_total, v.best_rival_total), (30, 30));
        assert_eq!(v.margin, 0, "level totals are a zero margin");
        assert_eq!(v.rank, 2);
        assert!(!v.won, "a zero margin is NOT a win: the tie-break ranks p1 ahead");

        // ANTI-VACUITY: the naive read really does answer differently here, so
        // this fixture is a discriminator rather than a coincidence.
        let naive = result
            .standings
            .iter()
            .filter(|s| s.player != mover)
            .max_by_key(|s| s.breakdown.total)
            .map(|s| s.player);
        assert_eq!(naive, Some(PlayerId(2)), "the fixture does not separate the two readings");
        assert_ne!(v.best_rival, naive.unwrap());
    }

    /// One hand-built playout carrying nothing but its margin — every other
    /// field a stated constant, because [`Adjudication::all_candidates_equal`]
    /// reads margins and nothing else.
    fn hand_playout(replication: u32, margin: i64) -> CandidatePlayout {
        CandidatePlayout {
            replication,
            world: WorldSeed(0),
            agent_seeds: vec![0, 0],
            margin,
            mover_total: 0,
            best_rival: PlayerId(1),
            best_rival_total: 0,
            rank: 2,
            won: false,
            decisions: 0,
            turns: 0,
            final_digest: String::new(),
            // No hook ran over a hand-built playout — the counter-absence
            // reading, never a false verdict.
            turn_verdict: None,
        }
    }

    /// A table assembled from margin vectors alone: no root, no engine, no
    /// game. Each row gets a DISTINCT candidate and continuation digest, so an
    /// equality helper that compared rows by anything but their margins would
    /// read these tables differently.
    fn hand_table(margin_sets: &[&[i64]]) -> Adjudication {
        let k = margin_sets.first().map_or(0, |m| m.len() as u32);
        let rows = margin_sets
            .iter()
            .enumerate()
            .map(|(index, margins)| CandidateRow {
                candidate: Move::BuildShophouse { slot: index as u8 },
                continuation_digest: format!("hand-built-continuation-{index}"),
                playouts: margins
                    .iter()
                    .enumerate()
                    .map(|(r, m)| hand_playout(r as u32, *m))
                    .collect(),
            })
            .collect();
        Adjudication {
            mover: PlayerId(0),
            root_turn: 0,
            master: WorldSeed(0),
            replications: k,
            rows,
            witness: None,
            provenance: AdjudicationProvenance {
                root: RootDescriptor {
                    origin: RootOrigin::Constructed,
                    event_index: None,
                    root_digest: "hand-built-root".to_string(),
                },
                semantics_rev: SEMANTICS_REV,
                rules_hash: "hand-built-rules".to_string(),
                seats: Vec::new(),
                master: 0,
                replications: k,
            },
        }
    }

    /// `all_candidates_equal` compares margins ELEMENTWISE, not by mean — at
    /// the CONSTRUCTOR, where the distinction is statable in microseconds.
    ///
    /// The cells above reach this helper through real adjudications, and both
    /// of the tables they build have candidates whose margins are constant
    /// across K (their remainders are forced), so neither can tell an
    /// elementwise comparison from a mean comparison. `[0, 2]` against `[1, 1]`
    /// can: same mean, different vectors, and calling those equal would hide
    /// exactly the per-replication variation K exists to expose. The `[1, 1]`
    /// against `[1, 1]` half is the anti-vacuity — a helper hard-wired to
    /// `false` fails there.
    #[test]
    fn all_candidates_equal_compares_margin_vectors_and_never_their_means() {
        let equal_means = hand_table(&[&[0, 2], &[1, 1]]);
        assert_eq!(equal_means.candidates(), 2, "anti-vacuity: one row is trivially 'equal'");
        assert!(
            (equal_means.rows[0].mean_margin() - equal_means.rows[1].mean_margin()).abs() < 1e-12,
            "the fixture does not separate the two readings: the means differ",
        );
        assert!(
            !equal_means.all_candidates_equal(),
            "two candidates with equal MEANS and different margin VECTORS are not equal",
        );

        let identical = hand_table(&[&[1, 1], &[1, 1]]);
        assert!(identical.all_candidates_equal(), "identical margin vectors ARE equal");
    }

    // -----------------------------------------------------------------------
    // The hand-computed table
    // -----------------------------------------------------------------------

    /// **THE ARITHMETIC CELL.** Every candidate of a constructed near-terminal
    /// root, forced and played out, against margins derived from the rules in
    /// [`near_terminal_building_root`]'s doc.
    ///
    /// The fixture prints VP 3, 4, 5, 0 on the four shophouse spaces, banks 6 VP
    /// for the mover and 10 for its rival, and zeroes every other scoring leg,
    /// so the five margins are:
    ///
    /// | candidate | mover total | rival | margin | verdict |
    /// |---|---|---|---|---|
    /// | `SkipBuilding`         | 6      | 10 | −4 | rank 2 |
    /// | `BuildShophouse{0}` +3 | 9      | 10 | −1 | rank 2 |
    /// | `BuildShophouse{1}` +4 | **10** | 10 | **0** | rank 2 — LEVEL on VP and beaten by the official clockwise tie-break |
    /// | `BuildShophouse{2}` +5 | 11     | 10 | +1 | rank 1, WINS |
    /// | `BuildShophouse{3}` +0 | 6      | 10 | −4 | rank 2 |
    ///
    /// The `+4` row is the one that earns its place: a margin of 0 is NOT a win,
    /// because the rival is the start player and `final_result` breaks a level
    /// total by clockwise closeness to it. A margin type that resolved ties by
    /// sign, or a `won` flag read off `margin >= 0`, passes every other row here
    /// and fails that one.
    ///
    /// The MOVER seat is `greedy-v1` and the rival a rung, which is also this
    /// file's statement that the forced mover's own spec governs its post-root
    /// play: the one remaining decision is the mover's, the driver has no
    /// fallback arm that could take it instead, and the witnessed provenance
    /// names greedy at that seat.
    #[test]
    fn every_candidate_of_a_near_terminal_root_reports_its_hand_computed_margin() {
        let rules = Rules::standard();
        let (root, mover, _) = near_terminal_building_root(&rules, world(41).0, [3, 4, 5, 0], 6, 10);
        let rival = PlayerId(1 - mover.0);
        let master = world(42);

        let mut seats = rung_seats();
        seats[mover.0 as usize] = SeatSpec::Contestant(Contestant::GREEDY_V1_DEFAULT);

        let table = adjudicate_root(
            &RootSource::Constructed(root.clone()),
            &seats,
            &rules,
            master,
            2,
            DrivenOpts::default(),
        )
        .expect("a near-terminal root adjudicates");

        assert_eq!(table.mover, mover, "the margins are the ROOT MOVER's");
        assert_eq!(table.replications, 2);
        assert_eq!(table.master, master);
        let moves: Vec<Move> = table.rows.iter().map(|r| r.candidate.clone()).collect();
        assert_eq!(
            moves,
            expected_building_candidates(),
            "the candidate set is not the engine's own legal list at this root",
        );

        // The hand-computed table, row by row. `mover_total` and
        // `best_rival_total` are asserted BESIDE the margin so a row that got
        // the right difference from two wrong totals cannot pass.
        let expected: [(i64, u32, u8, bool); 5] =
            [(-4, 6, 2, false), (-1, 9, 2, false), (0, 10, 2, false), (1, 11, 1, true), (-4, 6, 2, false)];
        for (row, (margin, total, rank, won)) in table.rows.iter().zip(expected) {
            assert_eq!(row.playouts.len(), 2, "{:?}: K playouts", row.candidate);
            for p in &row.playouts {
                assert_eq!(p.margin, margin, "{:?}: margin", row.candidate);
                assert_eq!(p.mover_total, total, "{:?}: the mover's own total", row.candidate);
                assert_eq!(p.best_rival, rival, "{:?}: the best rival's seat", row.candidate);
                assert_eq!(p.best_rival_total, 10, "{:?}: the rival's total", row.candidate);
                assert_eq!(p.rank, rank, "{:?}: the mover's rank", row.candidate);
                assert_eq!(p.won, won, "{:?}: the mover's verdict", row.candidate);
                // The forced remainder, asserted rather than assumed — it is
                // what makes every number above policy-independent.
                assert_eq!(p.decisions, 1, "{:?}: the remainder is one forced move", row.candidate);
            }
            // mean/min/max are RECORDED, and on a forced remainder they collapse
            // — which is a statement about this fixture, not about the fields.
            assert_eq!(row.min_margin(), Some(margin));
            assert_eq!(row.max_margin(), Some(margin));
            assert!((row.mean_margin() - margin as f64).abs() < 1e-12);
        }

        // Every row's continuation digest IS the state ROOT + that candidate
        // reaches — recomputed here through the ENGINE's own CHECKED
        // `apply_move` rather than read back from the value the adjudicator
        // computed for itself. Without this the digests could all name the ROOT
        // (or any other single state) and every assertion above would still
        // pass, leaving the row→playout join pointing at a position no playout
        // started from.
        for row in &table.rows {
            let after = apply_move(&root, &row.candidate, &rules).unwrap_or_else(|e| {
                panic!("{:?}: the root's own candidate must apply: {e}", row.candidate)
            });
            assert_eq!(
                row.continuation_digest,
                state_digest(&after),
                "{:?}: the row names a continuation it did not begin at",
                row.candidate,
            );
        }
        // ...and the five continuations are five DIFFERENT states, so no row
        // can be joined to another row's playouts. Note two of these rows score
        // ALIKE (Skip and the 0-VP space), which is exactly why distinctness is
        // asserted on the digest and not on the margin.
        let mut digests: Vec<&str> =
            table.rows.iter().map(|r| r.continuation_digest.as_str()).collect();
        digests.sort_unstable();
        digests.dedup();
        assert_eq!(digests.len(), table.rows.len(), "two candidates share a continuation digest");

        // The zero-margin row is a LOSS, and the +1 row is the only win.
        assert_eq!(table.rows.iter().filter(|r| r.wins() > 0).count(), 1);
        assert!(!table.all_candidates_equal(), "five different margins are not all equal");
        // ...and two rows DO coincide (Skip and the 0-VP space), so "not all
        // equal" is not "all distinct" — the helper compares vectors, not sets.
        assert_eq!(table.rows[0].margins(), table.rows[4].margins());

        // Provenance: the root named once, the seats resolved, the master and K.
        assert_eq!(table.provenance.root, RootDescriptor::constructed(&root));
        assert_eq!(table.provenance.semantics_rev, SEMANTICS_REV);
        assert_eq!(table.provenance.rules_hash, rules.content_hash());
        assert_eq!((table.provenance.master, table.provenance.replications), (master.0, 2));
        assert_eq!(
            table.provenance.seats[mover.0 as usize].echo,
            Contestant::GREEDY_V1_DEFAULT.echo(),
        );
        assert_eq!(table.provenance.seats[mover.0 as usize].requested, "contestant");
        assert_eq!(
            table.provenance.seats[rival.0 as usize].requested,
            "reply:family-weighted",
        );
        // The WITNESSED half, kept verbatim from the first playout: the mover's
        // own agent said what it was, and the bench said what it could see.
        let witness = table.witness.as_ref().expect("a table with playouts has a witness");
        assert_eq!(witness.seats[mover.0 as usize].echo, Contestant::GREEDY_V1_DEFAULT.echo());
        assert_eq!(witness.master, replication_world(master, 0).0, "the witness is replication 0's");
        // THE JOIN, closed by VALUE. The witness is the FIRST playout of the
        // FIRST candidate, and a playout's descriptor is `Constructed` on the
        // state it actually started from — ROOT + that candidate — so the
        // witness's root digest is row 0's CONTINUATION digest and NOT the
        // table's root digest. The two are different positions (asserted
        // below), so reading one for the other would silently mis-identify
        // every playout's starting state.
        assert_eq!(
            witness.root.root_digest, table.rows[0].continuation_digest,
            "the witness does not name the continuation its playout began at",
        );
        assert_ne!(
            witness.root.root_digest, table.provenance.root.root_digest,
            "anti-vacuity: the witness's root and the TABLE's root are different positions",
        );
        assert_eq!(witness.root.origin, RootOrigin::Constructed);
        assert_eq!(witness.root.event_index, None, "a constructed continuation indexes no log");
    }

    /// [`AdjudicationProvenance`] and its [`AdjudicationSeat`]s survive the trip
    /// to a sidecar and back.
    ///
    /// The same fixture and the same MIXED seat vocabulary as the arithmetic
    /// cell above, at K = 1 — a real adjudication's provenance rather than a
    /// literal, so the round trip is taken over the values the module actually
    /// stamps (both `requested` spellings included). `RootedProvenance` has this
    /// pin already; these two types are a separate pair with their own derives,
    /// and a missing one is a compile error only once something asks.
    #[test]
    fn an_adjudications_provenance_round_trips_through_serde() {
        let rules = Rules::standard();
        let (root, mover, _) = near_terminal_building_root(&rules, world(57).0, [3, 4, 5, 0], 6, 10);
        let mut seats = rung_seats();
        seats[mover.0 as usize] = SeatSpec::Contestant(Contestant::GREEDY_V1_DEFAULT);
        let table = adjudicate_root(
            &RootSource::Constructed(root),
            &seats,
            &rules,
            world(58),
            1,
            DrivenOpts::default(),
        )
        .expect("the fixture adjudicates");

        let p = &table.provenance;
        let json = serde_json::to_string(p).expect("the provenance serializes");
        let back: AdjudicationProvenance =
            serde_json::from_str(&json).expect("the provenance parses back");
        assert_eq!(&back, p, "a real adjudication's provenance did not survive serde");

        // The fields a sidecar READER needs are actually in the document —
        // without these the assertion above would still pass on a document that
        // wrote nothing but the fields serde defaults.
        assert_eq!(back.seats.len(), 2);
        assert!(json.contains("\"contestant\""), "the asked-for seat is not written: {json}");
        assert!(json.contains("reply:family-weighted"), "the reply seat is not written: {json}");
        assert!(
            json.contains(&Contestant::GREEDY_V1_DEFAULT.echo()),
            "the resolved echo is not written: {json}",
        );
        assert!(json.contains(&p.root.root_digest), "the root digest is not written: {json}");
    }

    /// A GENUINELY DEAD root: five candidates, one margin.
    ///
    /// The same fixture with every printed space VP set to 0 — so the mover's
    /// only choice is which token to spend, and no choice moves the score. This
    /// is the reading family 1 asks for, and the CONTROL for it is the cell
    /// above: one field of one fixture separates a table that discriminates from
    /// a table that does not, so `all_candidates_equal` cannot be passing here
    /// for a reason unrelated to the position.
    #[test]
    fn a_genuinely_dead_root_reads_all_candidates_equal() {
        let rules = Rules::standard();
        let (root, mover, _) = near_terminal_building_root(&rules, world(43).0, [0; 4], 6, 10);
        let table = adjudicate_root(
            &RootSource::Constructed(root),
            &rung_seats(),
            &rules,
            world(44),
            3,
            DrivenOpts::default(),
        )
        .expect("the dead root adjudicates");

        assert_eq!(table.candidates(), 5, "anti-vacuity: one candidate is trivially 'equal'");
        assert!(
            table.rows.iter().map(|r| r.candidate.clone()).collect::<Vec<_>>()
                == expected_building_candidates(),
            "the candidates are five DIFFERENT moves",
        );
        assert!(table.all_candidates_equal(), "no candidate moved the score, but the table says so");
        for row in &table.rows {
            assert_eq!(row.margins(), vec![-4; 3], "{:?}: 6 banked against 10", row.candidate);
        }
        assert_eq!(table.mover, mover);
    }

    /// A TERMINAL root adjudicates to an EMPTY table — `Ok`, zero candidates,
    /// and the provenance still stamped.
    ///
    /// The counterpart of `drive_to_terminal`'s zero-move contract, one level
    /// up: there is nothing to force at a finished game, and a refusal would
    /// make the instrument unable to report the position an adjudication walks
    /// INTO whenever a candidate ends the game.
    #[test]
    fn a_terminal_root_adjudicates_to_an_empty_table() {
        let rules = Rules::standard();
        let (terminal, mover) = terminal_building_root(&rules, world(45).0);
        let master = world(46);
        let table = adjudicate_root(
            &RootSource::Constructed(terminal.clone()),
            &rung_seats(),
            &rules,
            master,
            2,
            DrivenOpts::default(),
        )
        .expect("a terminal root is an Ok, not a refusal");

        assert_eq!(table.candidates(), 0, "a finished game offers no candidate");
        assert!(table.rows.is_empty());
        assert_eq!(table.mover, mover);
        assert!(table.witness.is_none(), "nothing was played, so nothing was witnessed");
        assert_eq!(table.provenance.root.root_digest, state_digest(&terminal));
        assert_eq!(table.provenance.replications, 2);
        // Vacuously true, and the doc says so — a caller reading "dead" off an
        // empty table is reading a FINISHED GAME, which is a different fact.
        assert!(table.all_candidates_equal());
    }

    /// A candidate that ENDS THE GAME leaves a remainder of exactly nothing —
    /// and the adjudicator reports it, rather than refusing.
    ///
    /// This is the other half of the terminal contract, and the half that joins
    /// the two tasks: `drive_to_terminal`'s zero-move `Ok` was written FOR this
    /// call site ("a candidate that ends the game leaves a remainder of exactly
    /// nothing — refusing it would make 'this move wins immediately' the one
    /// answer the instrument could not report"), and until now nothing exercised
    /// it THROUGH the adjudicator. The cell above pins a terminal ROOT; this one
    /// pins a terminal CONTINUATION, which is the case a real final-turn root
    /// walks into.
    ///
    /// The margin is the banked difference with nothing added: 6 against 10.
    #[test]
    fn a_candidate_that_ends_the_game_plays_out_to_a_zero_move_remainder() {
        let rules = Rules::standard();
        let root = last_move_root(&rules, world(55).0, 6, 10);
        let mover = root.current_player;
        let table = adjudicate_root(
            &RootSource::Constructed(root),
            &rung_seats(),
            &rules,
            world(56),
            2,
            DrivenOpts::default(),
        )
        .expect("a final-move root adjudicates");

        assert_eq!(
            table.rows.iter().map(|r| r.candidate.clone()).collect::<Vec<_>>(),
            vec![Move::EndAdditionalPhase],
            "the fixture's whole point is that this decision is FORCED",
        );
        let row = &table.rows[0];
        assert_eq!(row.playouts.len(), 2);
        for p in &row.playouts {
            assert_eq!(p.decisions, 0, "the forced move ended the game: no remainder to play");
            assert_eq!(p.margin, -4, "6 banked against 10, with nothing added");
            assert_eq!((p.mover_total, p.best_rival_total), (6, 10));
            assert!(!p.won);
            assert_eq!(p.rank, 2);
        }
        assert_eq!(table.mover, mover);
        // ...and a one-row table is vacuously "all equal", which is a FORCED
        // position rather than a dead one — the distinction the helper's doc
        // makes, exercised here rather than only stated.
        assert!(table.all_candidates_equal());
        assert_eq!(table.candidates(), 1);
    }

    // -----------------------------------------------------------------------
    // Determinism, and the per-replication worlds
    // -----------------------------------------------------------------------

    /// Same root, same seats, same master, same K → the same table, twice.
    ///
    /// The anti-vacuity is a DIFFERENT master: on this fixture the play is
    /// forced, so the margins are master-independent BY CONSTRUCTION — what must
    /// still move is the recorded worlds, and a digest that did not would be
    /// witnessing nothing. Both halves are asserted, in that order.
    #[test]
    fn the_same_master_adjudicates_the_same_table_twice() {
        let rules = Rules::standard();
        let (root, _, _) = near_terminal_building_root(&rules, world(47).0, [3, 4, 5, 0], 6, 10);
        let source = RootSource::Constructed(root);
        let master = world(48);
        let run = |m: WorldSeed| {
            adjudicate_root(&source, &rung_seats(), &rules, m, 2, DrivenOpts::default())
                .expect("the adjudication must complete")
        };

        let first = run(master);
        let second = run(master);
        assert_eq!(first.digest(), second.digest(), "the digest moved between identical runs");
        assert_eq!(first, second, "the tables differ beyond what the digest covers");

        let elsewhere = run(WorldSeed(master.0 ^ 0x5555_5555_5555_5555));
        assert_eq!(
            elsewhere.rows.iter().map(|r| r.margins()).collect::<Vec<_>>(),
            first.rows.iter().map(|r| r.margins()).collect::<Vec<_>>(),
            "this fixture's remainder is FORCED, so the master cannot move a margin",
        );
        assert_ne!(
            first.digest(),
            elsewhere.digest(),
            "the digest does not depend on the master, so it cannot witness determinism",
        );
    }

    /// Design §4's second sentence, made executable: the candidates of ONE
    /// replication share that replication's world and streams, so they differ
    /// only by the forced move.
    ///
    /// Three assertions, and the middle one is the one a plausible-looking
    /// implementation gets wrong: (i) every row's r-th playout carries the SAME
    /// world, (ii) that world is `replication_world(master, r)` and not a fresh
    /// draw per candidate, (iii) the seats' streams descend from it through the
    /// arena's own `agent_stream_seed(world, seat, Slot(seat))`. A driver that
    /// re-derived a world per candidate would still produce a deterministic,
    /// reproducible table — and every candidate would have been measured in a
    /// different game.
    #[test]
    fn every_candidate_of_one_replication_plays_in_that_replications_world() {
        let rules = Rules::standard();
        let (root, _, _) = near_terminal_building_root(&rules, world(49).0, [3, 4, 5, 0], 6, 10);
        let master = world(50);
        let k = 3u32;
        let table = adjudicate_root(
            &RootSource::Constructed(root),
            &rung_seats(),
            &rules,
            master,
            k,
            DrivenOpts::default(),
        )
        .expect("the adjudication must complete");
        assert!(table.candidates() >= 2, "one candidate cannot witness a SHARED world");

        for r in 0..k {
            let expected = replication_world(master, r);
            for row in &table.rows {
                let p = &row.playouts[r as usize];
                assert_eq!(p.replication, r);
                assert_eq!(
                    p.world, expected,
                    "{:?} played replication {r} in a world of its own",
                    row.candidate,
                );
                for (seat, agent_seed) in p.agent_seeds.iter().enumerate() {
                    assert_eq!(
                        *agent_seed,
                        agent_stream_seed(expected, PlayerId(seat as u8), Slot(seat as u8)),
                        "{:?}: seat {seat} did not play on the replication's arena-keyed stream",
                        row.candidate,
                    );
                }
            }
        }

        // The replications are DIFFERENT worlds — without this the K axis is K
        // copies of one number and the recorded spread means nothing.
        let worlds: Vec<u64> = (0..k).map(|r| table.rows[0].playouts[r as usize].world.0).collect();
        let mut unique = worlds.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), worlds.len(), "two replications shared a world: {worlds:?}");
    }

    /// K > 1 on an ORGANIC root, where the reply seats actually get to choose:
    /// the spread fields are RECORDED and their arithmetic is checked, and NO
    /// value of any of them is asserted.
    ///
    /// This is the cell that can see a margin move between replications at all
    /// — the constructed fixtures above are forced by design — so it is where
    /// "variance recorded, never asserted" has content. What IS asserted is
    /// internal consistency (`min <= mean <= max`, the mean is the mean, the
    /// vector is K long) and reproduction at the same master.
    ///
    /// **Measured at authoring** (2026-08-18, WSL debug, event 686 of this
    /// record, six `ExecuteAreaAction` candidates, K = 4): the spread is real
    /// and large — per-candidate margins like `[2, −68, −6, −11]` and
    /// `[−12, −37, −25, −3]`, means between −26.50 and −18.25, one won playout
    /// in twenty-four. Read that as what it is: four rung rollouts of a ~60-move
    /// tail, whose noise dwarfs any candidate difference at this N. It is
    /// PRINTED (`--nocapture`) rather than asserted for exactly that reason.
    #[test]
    fn replication_spread_is_recorded_and_never_asserted() {
        let rules = Rules::standard();
        let stats = rung_game(&rules, world(51));
        let events = stats.record.events.len();
        // A LATE root with a small candidate set: deterministic in the record
        // and the window, and cheap — every candidate is played out K times.
        let from = events.saturating_sub(60);
        let (k_root, root) = (from..events)
            .find_map(|k| {
                let state = state_at(&stats.record, k, &rules).ok()?;
                let n = crate::game::legal_moves::get_legal_moves(&state, &rules).len();
                (2..=6).contains(&n).then_some((k, state))
            })
            .expect("a late root with 2..=6 candidates exists in the last 60 events");
        assert!(!is_game_over(&root), "event {k_root}: the chosen root is already terminal");

        let master = world(52);
        let source = RootSource::Constructed(root);
        let table =
            adjudicate_root(&source, &rung_seats(), &rules, master, 4, DrivenOpts::default())
                .expect("the adjudication must complete");

        assert!((2..=6).contains(&table.candidates()));
        for row in &table.rows {
            let margins = row.margins();
            assert_eq!(margins.len(), 4, "{:?}: K margins", row.candidate);
            let (min, max) = (row.min_margin().unwrap(), row.max_margin().unwrap());
            let mean = row.mean_margin();
            assert_eq!(min, *margins.iter().min().unwrap());
            assert_eq!(max, *margins.iter().max().unwrap());
            assert!(min as f64 <= mean && mean <= max as f64, "{:?}: mean outside [min,max]", row.candidate);
            let sum: i64 = margins.iter().sum();
            assert!((mean - sum as f64 / 4.0).abs() < 1e-9, "{:?}: the mean is not the mean", row.candidate);
            assert!(row.wins() <= 4);
            // Every playout ran to a real terminal state.
            for p in &row.playouts {
                assert!(!p.final_digest.is_empty());
                assert!(p.turns >= table.root_turn, "{:?}: a run went backwards", row.candidate);
            }
            // RECORDED, in the one place the fields have content: printed, never
            // gated. `--nocapture` shows it.
            println!(
                "spread event {k_root} {:?}: margins {margins:?} mean {mean:+.2} \
                 min {min:+} max {max:+} wins {}/4",
                row.candidate,
                row.wins(),
            );
        }
        let again = adjudicate_root(&source, &rung_seats(), &rules, master, 4, DrivenOpts::default())
            .expect("the adjudication must complete");
        assert_eq!(table.digest(), again.digest(), "an organic root did not reproduce");
    }

    // -----------------------------------------------------------------------
    // The refusals
    // -----------------------------------------------------------------------

    /// The adjudicator's own guards fire BEFORE any candidate is forced — and
    /// the terminal case is why they have to: a terminal root produces no
    /// playout at all, so a guard left to the first playout would never run and
    /// an ill-formed call would come back `Ok` with an empty table.
    #[test]
    fn the_adjudicators_guards_fire_before_a_candidate_is_forced() {
        let rules = Rules::standard();
        let (root, _, _) = near_terminal_building_root(&rules, world(53).0, [0; 4], 6, 10);
        let (terminal, _) = terminal_building_root(&rules, world(53).0);
        let master = world(54);

        // K = 0 is not a run.
        let err = adjudicate_root(
            &RootSource::Constructed(root.clone()),
            &rung_seats(),
            &rules,
            master,
            0,
            DrivenOpts::default(),
        )
        .expect_err("zero replications must refuse");
        assert!(matches!(err, AdjudicationError::NoReplications), "got {err:?}");

        // A seat list that does not describe the root — on the TERMINAL root, so
        // the guard cannot be riding on a playout.
        let err = adjudicate_root(
            &RootSource::Constructed(terminal),
            &[SeatSpec::Reply(ReplyPolicy::FamilyWeighted)],
            &rules,
            master,
            1,
            DrivenOpts::default(),
        )
        .expect_err("a one-seat list for a two-seat root must refuse");
        match err {
            AdjudicationError::Unadjudicable(DrivenError::SeatMismatch { seats, players }) => {
                assert_eq!((seats, players), (1, 2));
            }
            other => panic!("expected SeatMismatch, got {other:?}"),
        }

        // A corrupt root is refused as a ROOT, never as a candidate's fault.
        let mut corrupt = root;
        corrupt.players[0].victory_points = 1;
        corrupt.players[0].assistants.in_hand += 1;
        let err = adjudicate_root(
            &RootSource::Constructed(corrupt),
            &rung_seats(),
            &rules,
            master,
            1,
            DrivenOpts::default(),
        )
        .expect_err("a corrupt root must refuse");
        match err {
            AdjudicationError::Unadjudicable(DrivenError::CorruptRoot(ref m)) => {
                assert!(m.contains("assistants total"), "the refusal must name the breakage: {m}");
            }
            ref other => panic!("expected CorruptRoot, got {other:?}"),
        }
        assert!(err.to_string().contains("assistants total"), "{err}");
    }

    /// **THE NEVER-A-SKIPPED-ROW CONTRACT, and this is its only pin.** A playout
    /// that does not complete refuses the WHOLE adjudication — no table comes
    /// back at all.
    ///
    /// The load-bearing assertion is the `Err` itself. [`AdjudicationError`]'s
    /// own doc states the reason: a table missing a candidate — because its
    /// playout failed and the row was skipped — reads exactly like a table of a
    /// position with fewer options, and the reading riding on it would be wrong
    /// with no way to notice. That property lives in a single `?` inside
    /// [`adjudicate_root`]'s replication loop, and nothing else in this file
    /// exercises a FAILING playout: every other refusal cell fires in the
    /// guards, BEFORE a candidate is ever forced.
    ///
    /// Driven from an ORGANIC root with a tiny decision bound, for the same
    /// reason `a_root_that_does_not_terminate_within_the_bound_is_a_typed_refusal`
    /// is: a fixture that genuinely fails mid-playout would have to hang if the
    /// guard were missing. The refusal must NAME the candidate and the
    /// replication, since "some row failed" identifies nothing.
    #[test]
    fn a_playout_that_cannot_finish_refuses_the_whole_table() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(59).0);
        let first = get_legal_moves(&root, &rules)
            .first()
            .cloned()
            .expect("a fresh 2P root offers candidates");
        // Three decisions is far under a fresh game's ~1,200, so the FIRST
        // candidate's FIRST replication is the one that reaches the bound.
        let opts = DrivenOpts { max_decisions: 3, ..DrivenOpts::default() };

        let err = adjudicate_root(
            &RootSource::Constructed(root),
            &rung_seats(),
            &rules,
            world(60),
            1,
            opts,
        )
        .expect_err("a playout that cannot finish must produce NO TABLE, not a shorter one");

        match err {
            AdjudicationError::Playout { ref candidate, replication, ref error } => {
                assert_eq!(
                    **candidate, first,
                    "the refusal must NAME the candidate whose playout failed",
                );
                assert_eq!(replication, 0, "the refusal must name WHICH replication failed");
                match **error {
                    DrivenError::DecisionCapReached { decisions } => {
                        assert_eq!(decisions, 3, "the inner refusal must name the bound it reached");
                    }
                    ref other => panic!("expected DecisionCapReached, got {other:?}"),
                }
            }
            ref other => panic!("expected Playout, got {other:?}"),
        }
        // The rendered sentence carries both halves, so a caller reading a log
        // can tell which row died and why.
        let rendered = err.to_string();
        assert!(rendered.contains("replication 0 of"), "the message hides the row: {rendered}");
        assert!(
            rendered.contains("did not terminate within 3 decisions"),
            "the message hides the cause: {rendered}",
        );
    }


    // =======================================================================
    // Phase C T1: the layer-2 generator vocabulary
    // =======================================================================

    /// Phase C T1's committed diagnostic master for the HARVESTS below —
    /// diagnostic BY NAME per `game::seeds`, the same waiver [`MASTER`] and
    /// [`GEN_MASTER`] carry.
    const C_MASTER: u64 = 0x0C01_2026_0818_9990;

    /// The FIRST position of the organic 2P rung games at `master` satisfying
    /// `want`.
    ///
    /// **Harvest-then-doctor is the fixture discipline for every layer-2 cell
    /// below**, and it is deliberate: the layer-2 predicates read the LEGAL
    /// LIST, so a state assembled field by field would have to be legal enough
    /// for `get_legal_moves` to answer sensibly, which is most of a game's
    /// worth of coherence to hand-write. Starting from an engine-produced
    /// position and moving ONE thing (a victory-point total, a seat's
    /// assistants) keeps the fixture inside the engine's own net —
    /// `check_invariants` is asserted on every doctored state — while making
    /// the conjunct under test the only thing that differs.
    ///
    /// Panics with a naming message when nothing matches, so a cell can never
    /// pass by harvesting nothing.
    fn harvest(
        rules: &Rules,
        master: u64,
        games: u32,
        what: &str,
        want: &mut dyn FnMut(&GameState) -> bool,
    ) -> GameState {
        let mut found = None;
        sweep(rules, master, games, &mut |state| {
            if want(state) {
                found = Some(state.clone());
                Visit::Stop
            } else {
                Visit::Continue
            }
        });
        found.unwrap_or_else(|| {
            panic!("anti-vacuity: no position in {games} rung games at {master:#018x} is {what}")
        })
    }

    /// Whether a [`sweep`] visitor has seen enough.
    enum Visit {
        Continue,
        Stop,
    }

    /// [`harvest`]'s walk, without the early return baked in: EVERY position of
    /// the first `games` organic 2P rung games at `master`, in play order, until
    /// the visitor stops it.
    ///
    /// Split out so the two shapes of cell share ONE walk — `harvest` wants the
    /// first match and the V4 equivalence sweeps want all of them, and two
    /// hand-written walks over the same population would eventually disagree
    /// about which population that is.
    fn sweep(
        rules: &Rules,
        master: u64,
        games: u32,
        visit: &mut dyn FnMut(&GameState) -> Visit,
    ) {
        let mut schedule = SeedSchedule::diagnostic(master);
        for _ in 0..games {
            let world = schedule.next_world();
            let mut make = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
                GENERATED_SEAT.build(rules, agent_stream_seed(world, seat, Slot(seat.0))).0
            };
            let stats = run_arena_game(world, 2, rules, &mut make, true)
                .expect("a harvest game must complete");
            let events = &stats.record.events;
            let mut state =
                state_at(&stats.record, 0, rules).expect("position 0 of any record replays");
            for position in 0..=events.len() {
                if matches!(visit(&state), Visit::Stop) {
                    return;
                }
                if let Some(event) = events.get(position) {
                    state = replay_from(&state, std::slice::from_ref(event), rules)
                        .expect("a generated game replays event by event");
                }
            }
        }
    }

    /// Doctor base VP until the mover's true margin is exactly `target`, and
    /// assert it landed.
    ///
    /// Only `victory_points` moves, and only UPWARD on one seat, so nothing
    /// clamps and no other score component is disturbed — which is what makes
    /// the band cells below statements about the band rather than about the
    /// doctoring.
    fn set_true_margin(state: &mut GameState, rules: &Rules, target: i64) {
        let mover = state.current_player;
        let rival = state
            .players
            .iter()
            .map(|p| p.id)
            .find(|id| *id != mover)
            .expect("a 2P fixture has a rival");
        let delta = target - true_margin_now(state, rules);
        if delta >= 0 {
            state.player_mut(mover).victory_points += delta as u32;
        } else {
            state.player_mut(rival).victory_points += (-delta) as u32;
        }
        assert_eq!(
            true_margin_now(state, rules),
            target,
            "the VP doctor did not land the margin it was asked for",
        );
    }

    /// Move one of `p`'s assistants onto `area` from wherever it can be spared,
    /// keeping the 20-assistant conservation the invariant net checks.
    fn stage_assistant(p: &mut crate::types::PlayerState, area: AreaId) {
        if p.assistants.in_hand > 0 {
            p.assistants.in_hand -= 1;
        } else if p.assistants.in_warehouse > 0 {
            p.assistants.in_warehouse -= 1;
        } else {
            panic!("the fixture seat has no assistant left to stage");
        }
        *p.assistants.on_board.entry(area).or_insert(0) += 1;
    }

    /// Return every assistant `p` has on `area` to its hand.
    fn clear_area(p: &mut crate::types::PlayerState, area: AreaId) {
        if let Some(n) = p.assistants.on_board.remove(&area) {
            p.assistants.in_hand += n;
        }
    }

    /// Arm the OPPONENT with a full race threat on both tracks: four assistants
    /// on each track area (power 4, plus the President that travels there = the
    /// capped 5), every payment type in stock, and its President anywhere but
    /// the area it would have to travel to.
    ///
    /// At that power and stock its reachable set is EVERY unoccupied space on
    /// both tracks, which is what makes the intersection with the mover's own
    /// reachable step guaranteed rather than lucky.
    fn arm_race_threat(state: &mut GameState, opponent: PlayerId, areas: [AreaId; 2]) {
        for area in areas {
            let p = state.player_mut(opponent);
            while p.assistants.in_area(area) < 4 {
                stage_assistant(p, area);
            }
            if p.president_location == PresidentLocation::OnBoard(area) {
                p.president_location = PresidentLocation::InHand;
            }
        }
        let p = state.player_mut(opponent);
        p.money = 10;
        p.copper = 1;
        p.silk = 1;
        p.tea = 1;
        p.seafood = 1;
        p.imported_goods = 4;
    }

    /// The two track areas of a 2P board.
    fn track_areas(state: &GameState) -> [AreaId; 2] {
        [
            area_of_type(state, AreaType::Church).expect("every board has a Church"),
            area_of_type(state, AreaType::Customs).expect("every board has a Customs"),
        ]
    }

    /// The seat that is not the mover.
    fn rival_of(state: &GameState) -> PlayerId {
        state
            .players
            .iter()
            .map(|p| p.id)
            .find(|id| *id != state.current_player)
            .expect("a 2P fixture has a rival")
    }

    // -----------------------------------------------------------------------
    // Item 1: `holds` gained `&Rules`, and the rules-free arms ignore it
    // -----------------------------------------------------------------------

    /// The two flag-reading predicates answer IDENTICALLY under two different
    /// `Rules`, and answer their own conjunct.
    ///
    /// The signature change is the interesting part of T1's item 1: every
    /// caller now passes rules, so an arm that started READING them would be
    /// invisible at the call sites. This is the executable form of "the shipped
    /// variant ignores it" — same states, two rules values (the standard
    /// registries and the empty test ones, which share not one card), same
    /// answers, and each answer re-derived from the state's own flags.
    #[test]
    fn the_rules_free_predicates_ignore_the_rules_the_signature_now_carries() {
        let rules = Rules::standard();
        let other = Rules::empty_for_tests();

        let fresh = root_2p(&rules, world(30).0);
        let (final_round, _) =
            RootSource::Generated { predicate: RootPredicate::FinalRoundAlreadyTriggered, master: GEN_MASTER, ordinal: 0 }
                .resolve(&rules)
                .expect("the scan supplies a final-round root");
        let mid_round = harvest(&rules, C_MASTER, 4, "triggered but not yet the final round", &mut |s| {
            s.end_game_triggered && !s.final_round && !is_game_over(s)
        });

        // Anti-vacuity: the three states are three DIFFERENT answers, so the
        // agreement below is not the agreement of a constant.
        assert!(!fresh.end_game_triggered, "the fresh root is already triggered");
        assert!(final_round.end_game_triggered && final_round.final_round);
        assert!(mid_round.end_game_triggered && !mid_round.final_round);

        for state in [&fresh, &final_round, &mid_round] {
            for predicate in
                [RootPredicate::FinalRoundAlreadyTriggered, RootPredicate::TriggerRoundNotFinal]
            {
                let with_standard = predicate.holds(state, &rules);
                assert_eq!(
                    with_standard,
                    predicate.holds(state, &other),
                    "{} read the rules it is supposed to ignore",
                    predicate.as_str(),
                );
                // And the answer is the conjunct, re-derived here.
                let expected = match predicate {
                    RootPredicate::FinalRoundAlreadyTriggered => {
                        state.end_game_triggered && state.final_round && !is_game_over(state)
                    }
                    _ => state.end_game_triggered && !state.final_round && !is_game_over(state),
                };
                assert_eq!(with_standard, expected, "{} is not its own conjunct", predicate.as_str());
            }
        }
    }

    // -----------------------------------------------------------------------
    // Item 2: the five new predicate arms
    // -----------------------------------------------------------------------

    /// Family 1's CONTROL partitions the triggered non-terminal positions with
    /// the shipped predicate, and neither claims an untriggered or a terminal
    /// one.
    #[test]
    fn the_trigger_round_control_partitions_the_triggered_positions() {
        let rules = Rules::standard();
        let shipped = RootPredicate::FinalRoundAlreadyTriggered;
        let control = RootPredicate::TriggerRoundNotFinal;

        // POSITIVE for the control: triggered, not yet the final round.
        let mid_round = harvest(&rules, C_MASTER, 4, "triggered but not yet the final round", &mut |s| {
            s.end_game_triggered && !s.final_round && !is_game_over(s)
        });
        assert!(control.holds(&mid_round, &rules));
        assert!(!shipped.holds(&mid_round, &rules), "the two predicates overlap");

        // NEGATIVE: the final round itself — the shipped predicate's own ground.
        let (final_round, _) =
            RootSource::Generated { predicate: shipped, master: GEN_MASTER, ordinal: 0 }
                .resolve(&rules)
                .expect("the scan supplies a final-round root");
        assert!(!control.holds(&final_round, &rules));
        assert!(shipped.holds(&final_round, &rules));

        // NEGATIVE: never triggered at all, and NEGATIVE: over.
        let fresh = root_2p(&rules, world(31).0);
        assert!(!control.holds(&fresh, &rules) && !shipped.holds(&fresh, &rules));
        let terminal = harvest(&rules, C_MASTER, 4, "a terminal position", &mut |s| is_game_over(s));
        assert!(
            terminal.end_game_triggered,
            "anti-vacuity: this terminal state never triggered, so `!is_game_over` is untested",
        );
        assert!(
            !control.holds(&terminal, &rules),
            "the control claimed a state nobody decides at",
        );
    }

    /// Family 2's POSITIVE needs all four conjuncts, and each one is shown
    /// refusing on its own.
    ///
    /// The band cell is the sharp one: at a margin of exactly
    /// [`TRUE_MARGIN_BAND`] the predicate holds and at one VP less it does not,
    /// on the SAME position with the same firing move — so the constant is
    /// read rather than merely present.
    #[test]
    fn the_ahead_predicate_needs_a_firing_move_a_full_band_and_a_narrow_root() {
        let rules = Rules::standard();
        let ahead = RootPredicate::EndFirableAheadInTruth;
        let behind = RootPredicate::EndFirableBehindInTruth;

        let firable = harvest(&rules, C_MASTER, 4, "a narrow position with a firing move", &mut |s| {
            !s.end_game_triggered
                && width_ok(s, &rules)
                && !firing_moves(s, &rules).is_empty()
        });
        assert!(check_invariants(&firable, &rules).is_ok());

        let mut at_band = firable.clone();
        set_true_margin(&mut at_band, &rules, TRUE_MARGIN_BAND);
        assert!(check_invariants(&at_band, &rules).is_ok(), "the VP doctor left an invalid state");
        assert!(ahead.holds(&at_band, &rules), "the positive fixture does not satisfy the predicate");
        assert!(!behind.holds(&at_band, &rules), "the control claimed an AHEAD position");

        // The BAND is read: one VP short and the same position is out.
        let mut under_band = firable.clone();
        set_true_margin(&mut under_band, &rules, TRUE_MARGIN_BAND - 1);
        assert!(
            !ahead.holds(&under_band, &rules),
            "a margin one VP under the band still satisfied the predicate",
        );
        assert!(
            !firing_moves(&under_band, &rules).is_empty(),
            "anti-vacuity: the under-band fixture lost its firing move, so the band is untested",
        );

        // The FIRING conjunct is read: a fresh root is +10 ahead and still out.
        let mut no_fire = root_2p(&rules, world(32).0);
        set_true_margin(&mut no_fire, &rules, TRUE_MARGIN_BAND);
        assert!(
            firing_moves(&no_fire, &rules).is_empty(),
            "anti-vacuity: the opening position can already end the game",
        );
        assert!(!ahead.holds(&no_fire, &rules));

        // The `!end_game_triggered` conjunct is read: the same board once the
        // trigger has fired has no FIRING move by definition.
        let (final_round, _) = RootSource::Generated {
            predicate: RootPredicate::FinalRoundAlreadyTriggered,
            master: GEN_MASTER,
            ordinal: 0,
        }
        .resolve(&rules)
        .expect("the scan supplies a final-round root");
        let mut triggered = final_round;
        set_true_margin(&mut triggered, &rules, TRUE_MARGIN_BAND);
        assert!(!ahead.holds(&triggered, &rules));
    }

    /// Family 2's CONTROL is the AHEAD predicate mirrored at `-BAND`, on the
    /// same harvested position.
    #[test]
    fn the_behind_predicate_mirrors_the_ahead_one_at_the_negative_band() {
        let rules = Rules::standard();
        let ahead = RootPredicate::EndFirableAheadInTruth;
        let behind = RootPredicate::EndFirableBehindInTruth;

        let firable = harvest(&rules, C_MASTER, 4, "a narrow position with a firing move", &mut |s| {
            !s.end_game_triggered
                && width_ok(s, &rules)
                && !firing_moves(s, &rules).is_empty()
        });

        let mut at_band = firable.clone();
        set_true_margin(&mut at_band, &rules, -TRUE_MARGIN_BAND);
        assert!(check_invariants(&at_band, &rules).is_ok());
        assert!(behind.holds(&at_band, &rules), "the control fixture does not satisfy the control");
        assert!(!ahead.holds(&at_band, &rules), "the positive claimed a BEHIND position");

        let mut under_band = firable.clone();
        set_true_margin(&mut under_band, &rules, -(TRUE_MARGIN_BAND - 1));
        assert!(
            !behind.holds(&under_band, &rules),
            "a margin one VP short of the negative band still satisfied the control",
        );

        // A margin of ZERO satisfies NEITHER — the band is a two-sided
        // exclusion zone, not a sign test.
        let mut level = firable.clone();
        set_true_margin(&mut level, &rules, 0);
        assert!(!ahead.holds(&level, &rules) && !behind.holds(&level, &rules));
    }

    /// Family 3's probe holds where an opponent is one action from a contested
    /// track step the mover can take this move, and each half of that
    /// conjunction is shown refusing alone.
    #[test]
    fn the_race_probe_needs_both_a_threatened_step_and_a_mover_who_can_take_it() {
        let rules = Rules::standard();
        let race = RootPredicate::ExclusiveRaceOneStep;

        // The MOVER half, harvested: a narrow position where one of this move's
        // candidates puts (or queues) this seat's own pawn on a track space.
        let base = harvest(&rules, C_MASTER, 4, "a narrow position where the mover can take a track step", &mut |s| {
            if is_game_over(s) {
                return false;
            }
            let legal = get_legal_moves(s, &rules);
            width_ok_len(legal.len()) && !mover_reachable_steps(s, &legal, &rules).is_empty()
        });
        let areas = track_areas(&base);
        let opponent = rival_of(&base);

        // The OPPONENT half, doctored on: four assistants on each track area and
        // a full purse, so its reachable set is every unoccupied space.
        let mut armed = base.clone();
        arm_race_threat(&mut armed, opponent, areas);
        assert!(check_invariants(&armed, &rules).is_ok(), "the armed fixture is not engine-valid");
        let legal = get_legal_moves(&armed, &rules);
        assert!(
            width_ok_len(legal.len()),
            "the doctoring widened the position past the bound, so this cell tests the bound",
        );
        let reachable = mover_reachable_steps(&armed, &legal, &rules);
        assert!(!reachable.is_empty(), "the doctoring cost the mover its own step");
        let contested = opponent_race_steps(&armed);
        assert!(!contested.is_empty(), "the armed opponent threatens nothing");
        assert!(race.holds(&armed, &rules), "the positive fixture does not satisfy the probe");

        // NEGATIVE, the opponent half alone: take the threat's assistants away
        // and the same position — same mover, same reachable step — is out.
        let mut disarmed = armed.clone();
        for area in areas {
            clear_area(disarmed.player_mut(opponent), area);
        }
        assert!(check_invariants(&disarmed, &rules).is_ok());
        assert!(
            opponent_race_steps(&disarmed).is_empty(),
            "anti-vacuity: the disarmed opponent still threatens a step",
        );
        assert_eq!(
            mover_reachable_steps(&disarmed, &get_legal_moves(&disarmed, &rules), &rules),
            reachable,
            "the disarming changed the MOVER's half, so this negative is not isolated",
        );
        assert!(!race.holds(&disarmed, &rules));

        // NEGATIVE, the President clause: an opponent standing on the area it
        // would have to travel to cannot act there next turn (rulebook p.11).
        let mut parked = armed.clone();
        parked.player_mut(opponent).president_location = PresidentLocation::OnBoard(areas[0]);
        let after_church = opponent_race_steps(&parked);
        assert!(
            !after_church.iter().any(|s| matches!(s, RaceStep::Church(_))),
            "a President parked on the Church still threatened a Church space",
        );
        parked.player_mut(opponent).president_location = PresidentLocation::OnBoard(areas[1]);
        assert!(
            !opponent_race_steps(&parked).iter().any(|s| matches!(s, RaceStep::Customs(_))),
            "a President parked on Customs still threatened a Customs space",
        );

        // NEGATIVE, a mover with no step: the opening position, armed the same
        // way, has a threat but no candidate that takes it.
        let mut opening = root_2p(&rules, world(33).0);
        let opening_areas = track_areas(&opening);
        let opening_rival = rival_of(&opening);
        arm_race_threat(&mut opening, opening_rival, opening_areas);
        assert!(
            !opponent_race_steps(&opening).is_empty(),
            "anti-vacuity: the armed opening threatens nothing, so the mover half is untested",
        );
        assert!(
            mover_reachable_steps(&opening, &get_legal_moves(&opening, &rules), &rules).is_empty(),
            "anti-vacuity: the opening mover can already take a track step",
        );
        assert!(!race.holds(&opening, &rules));
    }

    /// The race probe's mover half is QUEUE-AWARE: a track action with two or
    /// more eligible spaces places nothing at apply time, and the probe still
    /// credits the mover with every space the queued choice can still reach.
    ///
    /// Without this the classifier would see only the already-narrowed
    /// singleton case — i.e. exactly not the race-INITIATING moves the family
    /// is about.
    #[test]
    fn the_race_probes_mover_half_reads_a_queued_track_choice() {
        let rules = Rules::standard();
        let queued = harvest(
            &rules,
            C_MASTER,
            4,
            "a position whose applied candidate QUEUES a track-space choice",
            &mut |s| {
                if is_game_over(s) {
                    return false;
                }
                let legal = get_legal_moves(s, &rules);
                if !width_ok_len(legal.len()) {
                    return false;
                }
                legal.iter().any(|mv| {
                    apply_move_unchecked(s, mv, &rules).is_ok_and(|next| {
                        next.current_player == s.current_player
                            && next.pending_choices.iter().any(|c| {
                                matches!(
                                    c,
                                    PendingChoice::ChurchSpace { .. }
                                        | PendingChoice::CustomsSpace { .. }
                                )
                            })
                    })
                })
            },
        );

        let legal = get_legal_moves(&queued, &rules);
        // The queueing candidate itself places NOTHING, which is the whole
        // point: an occupancy-only classifier reads zero steps from it.
        let queueing = legal
            .iter()
            .find(|mv| {
                apply_move_unchecked(&queued, mv, &rules).is_ok_and(|next| {
                    next.current_player == queued.current_player
                        && next.pending_choices.iter().any(|c| {
                            matches!(
                                c,
                                PendingChoice::ChurchSpace { .. }
                                    | PendingChoice::CustomsSpace { .. }
                            )
                        })
                })
            })
            .expect("the harvest promised one");
        let applied = apply_move_unchecked(&queued, queueing, &rules).expect("it applied once");
        assert_eq!(
            applied.church_track.total_assistants(),
            queued.church_track.total_assistants(),
            "the queueing candidate placed a church pawn, so nothing here is about a QUEUE",
        );
        assert_eq!(
            applied.customs_track.total_assistants(),
            queued.customs_track.total_assistants(),
            "the queueing candidate placed a customs pawn, so nothing here is about a QUEUE",
        );

        assert!(
            !mover_reachable_steps(&queued, std::slice::from_ref(queueing), &rules).is_empty(),
            "the mover half read no step from a candidate that queues a track-space choice",
        );
    }

    /// Family 4's predicate is exactly "a claim is in the legal list", and the
    /// harvested claim position proves it non-vacuously.
    #[test]
    fn the_achievement_predicate_holds_exactly_when_a_claim_is_legal() {
        let rules = Rules::standard();
        let claim = RootPredicate::AchievementClaimAvailable;

        let claimable = harvest(&rules, C_MASTER, 4, "a narrow position with a legal claim", &mut |s| {
            let legal = get_legal_moves(s, &rules);
            width_ok_len(legal.len())
                && legal.iter().any(|mv| matches!(mv, Move::FulfillAchievement { .. }))
        });
        assert!(claim.holds(&claimable, &rules));

        // NEGATIVE: the same predicate on a position with no claim in the list.
        let fresh = root_2p(&rules, world(34).0);
        assert!(
            !get_legal_moves(&fresh, &rules)
                .iter()
                .any(|mv| matches!(mv, Move::FulfillAchievement { .. })),
            "anti-vacuity: the opening position already offers a claim",
        );
        assert!(!claim.holds(&fresh, &rules));
    }

    /// The candidate-width bound is a CONJUNCT of the layer-2 predicates, not a
    /// filter applied after the fact.
    ///
    /// The sharp cell is the last one: a harvested position that satisfies
    /// EVERY other conjunct of `EndFirableAheadInTruth` — untriggered, a real
    /// firing move, a doctored +10 margin — and is refused solely because its
    /// legal list is wider than [`MAX_ROOT_CANDIDATES`]. A bound that lived
    /// outside the predicate would let that position into the ordinal space.
    #[test]
    fn the_candidate_width_bound_is_a_conjunct_of_the_layer_two_predicates() {
        let rules = Rules::standard();
        assert_eq!(MAX_ROOT_CANDIDATES, 60, "the recorded generator constraint moved");
        assert!(width_ok_len(MAX_ROOT_CANDIDATES), "the bound must be inclusive at 60");
        assert!(!width_ok_len(MAX_ROOT_CANDIDATES + 1), "61 candidates must be out");

        // The state-level helper over a REAL wide position — the 61-candidate
        // pin, met by an engine-produced state rather than a synthetic count.
        let wide = harvest(&rules, C_MASTER, 4, "a position wider than the bound", &mut |s| {
            get_legal_moves(s, &rules).len() > MAX_ROOT_CANDIDATES
        });
        assert!(get_legal_moves(&wide, &rules).len() > MAX_ROOT_CANDIDATES);
        assert!(!width_ok(&wide, &rules));

        let narrow = root_2p(&rules, world(35).0);
        assert!(get_legal_moves(&narrow, &rules).len() <= MAX_ROOT_CANDIDATES);
        assert!(width_ok(&narrow, &rules));

        // The conjunct, non-vacuously: every OTHER conjunct of the family-2
        // positive holds here and the width alone refuses it.
        let mut wide_firable = harvest(
            &rules,
            C_MASTER,
            4,
            "a WIDE position that can still fire the end trigger",
            &mut |s| {
                !s.end_game_triggered
                    && get_legal_moves(s, &rules).len() > MAX_ROOT_CANDIDATES
                    && !firing_moves(s, &rules).is_empty()
            },
        );
        set_true_margin(&mut wide_firable, &rules, TRUE_MARGIN_BAND);
        assert!(!wide_firable.end_game_triggered);
        assert!(!firing_moves(&wide_firable, &rules).is_empty());
        assert_eq!(true_margin_now(&wide_firable, &rules), TRUE_MARGIN_BAND);
        assert!(
            !RootPredicate::EndFirableAheadInTruth.holds(&wide_firable, &rules),
            "a position past the width bound entered the family-2 positive's ordinal space",
        );
    }

    // =======================================================================
    // V4 step 2: the two frozen generator predicates (spec §6)
    // =======================================================================

    /// The V4 step-2 DEV-SMOKE master.
    ///
    /// **The 9999-convention throwaway and nothing else.** Spec §6 is
    /// FREEZE-THEN-DRAW: the predicate implementations freeze first and only then
    /// are M-PF / M-OSE drawn from OS entropy and ledger-recorded. A master
    /// invented in a test file before that draw, and used at any ordinal, would
    /// be a read taken before the thing it is a read of exists — so every cell
    /// below runs on this one declared throwaway, whose trailing `9999` says out
    /// loud that it is not anybody's recorded master.
    const V4_MASTER: u64 = 0x0401_2026_0821_9999;

    /// Move one order card from the top of the deck into `holder`'s hand — a
    /// PORT TAKE's net effect on the conservation identity, and nothing else.
    ///
    /// Conservation is why this is a helper rather than three lines inline:
    /// `check_invariants` counts deck + ports + hands + completed + removed = 36,
    /// so a card CONJURED into a hand fails the net and a card moved deck → hand
    /// does not. Every caller asserts the invariants afterwards.
    fn deal_from_deck(state: &mut GameState, holder: PlayerId) -> OrderCardId {
        let card = state.order_deck.pop().expect("the setup deck is not empty");
        let idx = holder.0 as usize;
        state.players[idx].order_cards_in_hand.push(card);
        card
    }

    /// Raise `holder`'s trade goods until `order`'s requirements are met.
    ///
    /// Trade goods have NO conservation invariant (there is no pool — the engine
    /// stores four per-player counters), so raising them is inside the net, and
    /// the callers assert `check_invariants` to say so rather than assume it.
    fn stock_for(state: &mut GameState, holder: PlayerId, order: OrderCardId, rules: &Rules) {
        let req = rules.orders.get(order).expect("the registry knows the card").requirements;
        let p = &mut state.players[holder.0 as usize];
        p.copper = p.copper.max(req.copper as u32);
        p.silk = p.silk.max(req.silk as u32);
        p.tea = p.tea.max(req.tea as u32);
        p.seafood = p.seafood.max(req.seafood as u32);
    }

    /// The seat that is NOT on the move.
    fn non_mover(state: &GameState) -> PlayerId {
        state
            .players
            .iter()
            .map(|p| p.id)
            .find(|id| *id != state.current_player)
            .expect("a 2P fixture has a non-mover")
    }

    /// Both V4 arms are REGISTERED: spec-§6 names, in `ALL`, distinct, and
    /// scanning the ecology §6 declares at the stride cap §6's independence
    /// claim rests on.
    #[test]
    fn the_two_v4_predicates_are_registered_with_their_spec_names_and_ecology() {
        assert_eq!(RootPredicate::PlacementForkSameCount.as_str(), "placement-fork-same-count");
        assert_eq!(RootPredicate::OpponentSittingExecutable.as_str(), "opponent-sitting-executable");

        let mut names: Vec<&str> = RootPredicate::ALL.into_iter().map(RootPredicate::as_str).collect();
        assert_eq!(names.len(), 8, "the predicate vocabulary changed size");
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 8, "two predicates answer to one name");

        for predicate in V4_BANDED_PREDICATES {
            assert!(
                RootPredicate::ALL.contains(&predicate),
                "{} is banded but not in ALL, so `gen:` cannot name it",
                predicate.as_str(),
            );
            let ecology = predicate.ecology();
            assert_eq!(
                ecology.id, GENERATED_ECOLOGY_V2_LOOKAHEAD,
                "{} does not scan the ecology spec §6 registers",
                predicate.as_str(),
            );
            assert_eq!(
                ecology.per_game_cap, GENERATED_STRIDE_CAP,
                "{} does not carry the stride §6's independence claim rests on",
                predicate.as_str(),
            );
            assert_eq!(ecology.games, GENERATED_V2_SCAN_GAMES);
        }
    }

    /// The placement-fork predicate holds at a TURN START offering two
    /// same-total placements into different area sets, and each conjunct is
    /// shown refusing on its own.
    ///
    /// The positive is a FRESH 2P setup and the choice is deliberate: it is a
    /// turn start by construction (`finalize_setup` hands the start player phase
    /// A), every seat holds eight assistants, no President is on the board so
    /// every placement is free, and nothing about it was selected after looking
    /// at what the predicate says.
    #[test]
    fn the_placement_fork_predicate_needs_a_turn_start_and_a_same_count_fork() {
        let rules = Rules::standard();
        let fork = RootPredicate::PlacementForkSameCount;

        let start = root_2p(&rules, world(40).0);
        assert_eq!(start.turn_phase, TurnPhase::AdditionalActionFirst);
        assert!(start.pending_choices.is_empty());
        assert!(fork.holds(&start, &rules), "a fresh 2P turn start offers no same-count fork");

        // THE FORK, re-derived here rather than trusted: the placements the
        // mover reaches by declining every additional action, grouped by total.
        let placing =
            apply_move(&start, &Move::EndAdditionalPhase, &rules).expect("phase A always ends");
        assert_eq!(placing.turn_phase, TurnPhase::Placement);
        let mut by_total: BTreeMap<usize, BTreeSet<BTreeSet<AreaId>>> = BTreeMap::new();
        for mv in get_legal_moves(&placing, &rules) {
            let (total, areas) = match mv {
                Move::PlaceThreeAreas { placements } => {
                    (placements.len(), placements.into_iter().collect())
                }
                Move::PlaceTwoSameArea { area, adjacent_bonus } => {
                    let mut areas: BTreeSet<AreaId> = BTreeSet::new();
                    areas.insert(area);
                    match adjacent_bonus {
                        Some(adj) => {
                            areas.insert(adj);
                            (3, areas)
                        }
                        None => (2, areas),
                    }
                }
                other => panic!("the Placement phase offered a non-placement move: {other:?}"),
            };
            by_total.entry(total).or_default().insert(areas);
        }
        assert!(
            by_total.values().any(|sets| sets.len() >= 2),
            "the independently derived fork is empty, so the positive proves nothing",
        );

        // NEGATIVE — NOT A TURN START: the SAME position one move on. Every
        // placement the predicate is about is now in the legal list itself,
        // which is exactly the position the registration does not name.
        assert!(
            !fork.holds(&placing, &rules),
            "the Placement phase itself was read as a turn START",
        );

        // NEGATIVE — NO ASSISTANT TO PLACE: the only placement is the empty one,
        // so no total has two sets. The eight assistants move hand → warehouse,
        // which keeps each seat's total at the 20 `check_invariants` counts.
        let mut empty_handed = start.clone();
        {
            let mover = empty_handed.current_player.0 as usize;
            let p = &mut empty_handed.players[mover];
            p.assistants.in_warehouse += p.assistants.in_hand;
            p.assistants.in_hand = 0;
        }
        assert!(check_invariants(&empty_handed, &rules).is_ok(), "the doctor broke conservation");
        assert!(
            !fork.holds(&empty_handed, &rules),
            "a mover with nothing in hand was read as facing a fork",
        );
    }

    /// The sitting-executable predicate needs all three of its parts, and the
    /// VISIBILITY part is the one that must be shown separately: a hidden
    /// starting order the opponent could pay for RIGHT NOW does not count.
    ///
    /// The fixture is a fresh 2P setup, where each seat's whole hand is its KEPT
    /// starting order — the masked slot itself. One deck card dealt into the
    /// opponent's hand is the publicly-witnessed Port take that makes the
    /// contrast a one-card difference on one state.
    #[test]
    fn the_sitting_executable_predicate_needs_a_visible_executable_opponent_order() {
        let rules = Rules::standard();
        let sitting = RootPredicate::OpponentSittingExecutable;

        let base = root_2p(&rules, world(41).0);
        let opponent = non_mover(&base);
        let hidden = base.players[opponent.0 as usize].starting_order.expect("a kept deal");
        assert_eq!(
            base.players[opponent.0 as usize].order_cards_in_hand,
            vec![hidden],
            "the fixture's premise is that the opponent's whole hand is the masked slot",
        );

        // NEGATIVE — VISIBILITY: the hidden starting order, fully payable.
        let mut hidden_only = base.clone();
        stock_for(&mut hidden_only, opponent, hidden, &rules);
        assert!(check_invariants(&hidden_only, &rules).is_ok());
        assert!(
            can_fulfill_order(&hidden_only, opponent, hidden, &rules.orders),
            "anti-vacuity: the hidden card is not payable, so visibility is untested",
        );
        assert!(
            !sitting.holds(&hidden_only, &rules),
            "an unfulfilled HIDDEN starting order was counted as visible to the mover",
        );

        // POSITIVE: one publicly-witnessed card in the same hand, payable.
        let mut visible = hidden_only.clone();
        let taken = deal_from_deck(&mut visible, opponent);
        stock_for(&mut visible, opponent, taken, &rules);
        assert!(check_invariants(&visible, &rules).is_ok());
        assert!(sitting.holds(&visible, &rules));

        // NEGATIVE — STOCKS: the same visible card with the goods stripped.
        let mut broke = visible.clone();
        {
            let p = &mut broke.players[opponent.0 as usize];
            p.copper = 0;
            p.silk = 0;
            p.tea = 0;
            p.seafood = 0;
        }
        assert!(check_invariants(&broke, &rules).is_ok());
        assert!(
            !can_fulfill_order(&broke, opponent, taken, &rules.orders),
            "anti-vacuity: the stripped hand can still pay, so the stocks half is untested",
        );
        assert!(!sitting.holds(&broke, &rules));

        // NEGATIVE — THE MOVER'S OWN: an executable visible card in the MOVER's
        // hand is not the non-mover's, and the predicate says non-mover.
        let mut mine = base.clone();
        let mover = mine.current_player;
        let own = deal_from_deck(&mut mine, mover);
        stock_for(&mut mine, mover, own, &rules);
        assert!(check_invariants(&mine, &rules).is_ok());
        assert!(
            can_fulfill_order(&mine, mover, own, &rules.orders),
            "anti-vacuity: the mover cannot pay either, so the seat half is untested",
        );
        assert!(
            !sitting.holds(&mine, &rules),
            "the MOVER's own executable order satisfied a predicate about the NON-mover",
        );

        // NEGATIVE — TERMINAL: nothing is decided at a finished game, so there
        // is no do-nothing successor for the §6 read to contrast against.
        let terminal = harvest(&rules, C_MASTER, 4, "a terminal position", &mut |s| is_game_over(s));
        let mut over = terminal.clone();
        let over_opponent = non_mover(&over);
        let over_card = deal_from_deck(&mut over, over_opponent);
        stock_for(&mut over, over_opponent, over_card, &rules);
        assert!(
            can_fulfill_order(&over, over_opponent, over_card, &rules.orders),
            "anti-vacuity: the terminal fixture has no executable visible order anyway",
        );
        assert!(!sitting.holds(&over, &rules));
    }

    /// **THE VISIBILITY DRIFT PIN.** `order_visible_to_others` is a MIRROR of
    /// `observe::visible_player`'s hand-masking branch, and a mirror that is not
    /// held equal to what it mirrors goes on producing plausible answers about a
    /// masking rule nobody runs.
    ///
    /// The comparison is exact and total over the population: for every seat
    /// pair at every swept position, the mirror's verdict on each held card
    /// equals whether `observe` gave the mover a `HandCard::Known` for it. The
    /// constructed half supplies the shape a sweep of on-policy games may not
    /// reach quickly — ONE hand holding both a masked starting order and a
    /// witnessed Port take — so the pin has a disagreement to find.
    ///
    /// **The `starting_order_hidden` arm is constructed too, and it has to be**:
    /// the mirror has TWO branches and the generator's domain contains only one.
    /// A hidden deal is the LIVE-TRACKING fiction — a state no `new_game` ever
    /// produces and no rooted run may start from (the viewer's own flag is a
    /// typed `observe` refusal) — so a sweep can sweep forever without reaching
    /// it, and this cell's name would over-promise. The fiction is three field
    /// moves and they keep the conservation net: the dealt card goes hand →
    /// `orders_removed` (the identity's total is unchanged), the recorded
    /// identity is cleared (the invariant net forbids flag AND identity
    /// together), and the flag goes up.
    #[test]
    fn the_visibility_mirror_agrees_with_observe_on_every_held_order() {
        let rules = Rules::standard();

        // The CONSTRUCTED half: one hand, one masked card, one witnessed card.
        let mut mixed = root_2p(&rules, world(42).0);
        let opponent = non_mover(&mixed);
        let hidden =
            mixed.players[opponent.0 as usize].starting_order.expect("a kept deal is recorded");
        let taken = deal_from_deck(&mut mixed, opponent);
        assert_ne!(hidden, taken, "the deal and the take must be different cards");
        assert!(check_invariants(&mixed, &rules).is_ok());
        assert!(
            !order_visible_to_others(&mixed.players[opponent.0 as usize], hidden),
            "the mirror let an unfulfilled starting order through",
        );
        assert!(
            order_visible_to_others(&mixed.players[opponent.0 as usize], taken),
            "the mirror masked a witnessed Port take",
        );

        // THE OTHER BRANCH: the same hand under the LIVE-TRACKING fiction, where
        // the deal is recorded nowhere and the view SYNTHESIZES the masked slot.
        // Every card actually in the hand is then a witnessed take, so the arm
        // answers `true` for all of them — including the one the identity arm
        // above masked.
        let mut unrecorded = mixed.clone();
        {
            let holder = &mut unrecorded.players[opponent.0 as usize];
            holder.order_cards_in_hand.retain(|card| *card != hidden);
            holder.starting_order = None;
            holder.starting_order_hidden = true;
        }
        unrecorded.orders_removed.push(hidden);
        assert!(
            check_invariants(&unrecorded, &rules).is_ok(),
            "the live-tracking fiction left an engine-invalid state",
        );
        assert!(
            order_visible_to_others(&unrecorded.players[opponent.0 as usize], taken),
            "the flag arm masked a card the view publishes as Known",
        );
        assert!(
            order_visible_to_others(&unrecorded.players[opponent.0 as usize], hidden),
            "the flag arm is not `true` for every recorded hand card — and the card it \
             refused is not even in that hand, which is the fiction's whole point",
        );

        let mut compared = 0_usize;
        let mut masked_seen = 0_usize;
        let mut check = |state: &GameState| {
            let viewer = state.current_player;
            let view = observe(state, viewer).expect("a generated seat's view is derivable");
            for holder in &state.players {
                if holder.id == viewer {
                    continue;
                }
                // A `Vec` rather than a set: `OrderCardId` carries no `Ord`, and
                // a hand holds at most three cards, so membership over the
                // vector IS set membership at no cost worth a new bound.
                let known: Vec<OrderCardId> = view.players[holder.id.0 as usize]
                    .hand
                    .iter()
                    .filter_map(|card| match card {
                        crate::types::HandCard::Known(id) => Some(*id),
                        crate::types::HandCard::HiddenStartingOrder => None,
                    })
                    .collect();
                for &order in &holder.order_cards_in_hand {
                    let mirrored = order_visible_to_others(holder, order);
                    assert_eq!(
                        mirrored,
                        known.contains(&order),
                        "the mirror and `observe` disagree about p{}'s card {} at turn {}",
                        holder.id.0,
                        order.0,
                        state.turn_number,
                    );
                    compared += 1;
                    if !mirrored {
                        masked_seen += 1;
                    }
                }
            }
        };

        check(&mixed);
        // The `observe` half of the flag arm: the view synthesizes ONE masked
        // slot for the unrecorded deal and publishes the take as Known, and the
        // mirror is held to that, card by card, exactly as everywhere else.
        check(&unrecorded);
        assert_eq!(
            observe(&unrecorded, unrecorded.current_player)
                .expect("the MOVER's own deal is recorded, so its view derives")
                .hidden_slot_count(),
            1,
            "anti-vacuity: the fiction produced no synthesized slot, so the arm is untested",
        );
        sweep(&rules, V4_MASTER, 2, &mut |state| {
            if !is_game_over(state) {
                check(state);
            }
            Visit::Continue
        });
        assert!(compared > 100, "the sweep compared only {compared} cards — too thin to mean much");
        assert!(masked_seen > 0, "anti-vacuity: nothing in the population was ever masked");
    }

    /// **The SET reading and the MULTISET reading never disagree about a
    /// POSITION**, though they do disagree about a move LIST — which is the
    /// whole of `same_count_fork`'s "sets, because sets is what §6 says" note,
    /// made executable.
    ///
    /// The first half is arithmetic on a synthetic pair: the Telegram swap is
    /// two distinct multisets over ONE set, so the readings differ there. The
    /// second half is the reason that never matters: over organic turn starts,
    /// no position offers the swap without also offering the single-area
    /// placements whose sets differ trivially, so the two readings agree
    /// everywhere the scan can see.
    #[test]
    fn the_set_reading_and_the_multiset_reading_never_disagree_about_a_position() {
        let rules = Rules::standard();

        /// The MULTISET reading, computed only here: ≥ 2 same-total placements
        /// whose per-area COUNTS differ.
        fn multiset_fork(placements: &[Move]) -> bool {
            let mut by_total: BTreeMap<usize, BTreeSet<BTreeMap<AreaId, u8>>> = BTreeMap::new();
            for mv in placements {
                let counts: BTreeMap<AreaId, u8> = match mv {
                    Move::PlaceThreeAreas { placements } => {
                        placements.iter().map(|&area| (area, 1)).collect()
                    }
                    Move::PlaceTwoSameArea { area, adjacent_bonus } => {
                        let mut counts = BTreeMap::new();
                        counts.insert(*area, 2);
                        if let Some(adjacent) = adjacent_bonus {
                            counts.insert(*adjacent, 1);
                        }
                        counts
                    }
                    _ => continue,
                };
                let total: usize = counts.values().map(|&n| n as usize).sum();
                by_total.entry(total).or_default().insert(counts);
            }
            by_total.values().any(|shapes| shapes.len() >= 2)
        }

        // The readings DO differ on a LIST — the Telegram swap, alone.
        let a = AreaId(0);
        let b = AreaId(1);
        let swap = [
            Move::PlaceTwoSameArea { area: a, adjacent_bonus: Some(b) },
            Move::PlaceTwoSameArea { area: b, adjacent_bonus: Some(a) },
        ];
        assert!(!same_count_fork(&swap), "the set reading found two sets in {{A, B}} twice");
        assert!(multiset_fork(&swap), "the multiset reading missed a 2+1 / 1+2 difference");

        // And they never differ on a POSITION, because the singles ride along.
        let mut turn_starts = 0_usize;
        let mut forks = 0_usize;
        sweep(&rules, V4_MASTER, 2, &mut |state| {
            if let Some(placements) = turn_start_placements(state, &rules) {
                turn_starts += 1;
                let set_reading = same_count_fork(&placements);
                assert_eq!(
                    set_reading,
                    multiset_fork(&placements),
                    "the two readings disagree at turn {} of a swept game",
                    state.turn_number,
                );
                if set_reading {
                    forks += 1;
                }
            }
            Visit::Continue
        });
        assert!(turn_starts > 20, "only {turn_starts} turn starts swept — too thin to mean much");
        assert!(forks > 0, "anti-vacuity: no swept turn start offered a fork at all");
    }

    /// Spec §6's ordinal bands are half-open, contiguous and disjoint, and the
    /// escrow band's floor is the number the burn rule fences.
    #[test]
    fn the_v4_ordinal_bands_are_half_open_contiguous_and_disjoint() {
        assert_eq!(V4_DEVELOPMENT_ORDINALS, 0..4);
        assert_eq!(V4_DEVELOPMENT_EXTENSION_ORDINALS, 4..8);
        assert_eq!(V4_ESCROW_ORDINALS, 8..16);
        assert_eq!(V4_DEVELOPMENT_ORDINALS.end, V4_DEVELOPMENT_EXTENSION_ORDINALS.start);
        assert_eq!(V4_DEVELOPMENT_EXTENSION_ORDINALS.end, V4_ESCROW_ORDINALS.start);
        for ordinal in V4_DEVELOPMENT_ORDINALS.chain(V4_DEVELOPMENT_EXTENSION_ORDINALS) {
            assert!(!V4_ESCROW_ORDINALS.contains(&ordinal), "ordinal {ordinal} is in two bands");
        }
        assert_eq!(V4_ESCROW_ORDINALS.len(), 8, "the escrow block is eight ordinals");
    }

    /// The CAPACITY read counts the same sequence the resolving scan resolves,
    /// and its report carries the ordinal → game map spec §6's independence
    /// claim is checked with.
    ///
    /// Measured on the CHEAP rung ecology and a family-1 predicate on purpose,
    /// and the choice costs the cell nothing it is about: the stride mechanics,
    /// the ordinal sequence and the map are properties of [`walk_scan`], which
    /// is ONE function for every predicate. Running this on a V4 predicate would
    /// buy the same assertions for minutes of lookahead scan — and would put a
    /// scan of a banded predicate in the per-edit suite, which is not a thing
    /// this task should make routine.
    #[test]
    fn the_capacity_read_counts_the_sequence_the_scan_resolves() {
        let rules = Rules::standard();
        let predicate = RootPredicate::FinalRoundAlreadyTriggered;
        assert_eq!(predicate.ecology().id, GENERATED_ECOLOGY_V1, "this cell is priced as a rung scan");

        let capacity = scan_capacity(predicate, V4_MASTER, Some(6), &rules)
            .expect("the rung scan supplies six strided ordinals");
        assert_eq!(capacity.predicate, predicate);
        assert_eq!(capacity.master, V4_MASTER);
        assert_eq!(capacity.ecology, predicate.ecology(), "the report drops the ecology");
        assert_eq!(capacity.found(), 6);
        assert!(capacity.truncated, "a bounded walk must say it stopped at its bound");
        assert!(capacity.covers(5) && !capacity.covers(6));
        assert!(capacity.games_walked >= 1);

        // THE STRIDE, measured: each game contributes a CONTIGUOUS run of at
        // most `per_game_cap` ordinals, which is the whole of the lemma
        // `ordinals_are_stride_separated` states as arithmetic.
        let cap = capacity.ecology.per_game_cap;
        let mut run_start = 0_usize;
        for n in 1..=capacity.game_of_ordinal.len() {
            let ends = n == capacity.game_of_ordinal.len()
                || capacity.game_of_ordinal[n] != capacity.game_of_ordinal[run_start];
            if ends {
                assert!(
                    (n - run_start) as u32 <= cap,
                    "game {} contributed {} ordinals past a cap of {cap}",
                    capacity.game_of_ordinal[run_start],
                    n - run_start,
                );
                if n < capacity.game_of_ordinal.len() {
                    assert!(
                        capacity.game_of_ordinal[n] > capacity.game_of_ordinal[run_start],
                        "the ordinal sequence revisited an earlier game",
                    );
                }
                run_start = n;
            }
        }

        // The map AGREES with the resolving scan: the descriptors
        // `generate_roots` returns name the same games.
        let roots = generate_roots(predicate, V4_MASTER, &[0, 1, 2, 3, 4, 5], &rules)
            .expect("the same six ordinals resolve");
        for (ordinal, (_, descriptor)) in roots.iter().enumerate() {
            let RootOrigin::Generated { game_index, ordinal: recorded, .. } = descriptor.origin
            else {
                panic!("a generated root carries a non-generated origin");
            };
            assert_eq!(recorded, ordinal as u32);
            assert_eq!(
                capacity.game_of_ordinal[ordinal], game_index,
                "the capacity map and the resolved descriptor disagree about ordinal {ordinal}",
            );
        }

        // A bound of ZERO measures nothing and walks nothing.
        let none = scan_capacity(predicate, V4_MASTER, Some(0), &rules).expect("a zero bound");
        assert_eq!(none.found(), 0);
        assert_eq!(none.games_walked, 0, "a zero-bound measurement played a game");
        assert!(none.truncated);
        assert_eq!(none.bands_are_disjoint(0..4, 8..16), None, "an unmeasured band read as disjoint");

        // The measured disjointness instrument, over what WAS measured.
        assert_eq!(capacity.bands_are_disjoint(0..2, 2..4), Some(true));
        assert_eq!(capacity.bands_are_disjoint(0..2, 0..2), Some(false), "a band is not disjoint from itself");
    }

    /// **THE STRIDE LEMMA, and the place it does not reach.**
    ///
    /// Spec §6 claims `GENERATED_STRIDE_CAP = 2` puts development and escrow
    /// ordinals in different generated games. The arithmetic half of that is
    /// [`ordinals_are_stride_separated`], and it is TRUE for the development
    /// band {0..4} against escrow (min gap 5) and FALSE at the
    /// development-EXTENSION boundary: ordinals 7 and 8 are one apart, so a game
    /// contributing a TWO-ordinal run can land exactly {7, 8} and hold both.
    /// Both halves are pinned, because the second is a residual an orchestrator
    /// has to know about before it draws the extension rather than after.
    #[test]
    fn the_stride_separates_the_development_band_from_escrow_but_not_the_extension() {
        assert_eq!(GENERATED_STRIDE_CAP, 2, "the stride moved, so both claims below move with it");

        for dev in V4_DEVELOPMENT_ORDINALS {
            for escrow in V4_ESCROW_ORDINALS {
                assert!(
                    ordinals_are_stride_separated(dev, escrow, GENERATED_STRIDE_CAP),
                    "development ordinal {dev} and escrow ordinal {escrow} can share a game",
                );
            }
        }
        // The lemma is not vacuous: adjacent ordinals are NOT separated.
        assert!(!ordinals_are_stride_separated(0, 1, GENERATED_STRIDE_CAP));
        assert!(ordinals_are_stride_separated(0, 2, GENERATED_STRIDE_CAP));

        // THE RESIDUAL, stated as an assertion so it cannot be forgotten: the
        // top of the development EXTENSION and the bottom of escrow are one
        // apart, so the stride alone does not separate them.
        let last_extension = V4_DEVELOPMENT_EXTENSION_ORDINALS.end - 1;
        assert_eq!(last_extension + 1, V4_ESCROW_ORDINALS.start);
        assert!(
            !ordinals_are_stride_separated(
                last_extension,
                V4_ESCROW_ORDINALS.start,
                GENERATED_STRIDE_CAP,
            ),
            "the extension/escrow boundary is separated after all — re-read the residual note",
        );
    }

    // -----------------------------------------------------------------------
    // Item 3: `firing_moves` and its short circuit
    // -----------------------------------------------------------------------

    /// The trigger-precondition enumeration visits all FIVE `EndGameTrigger`
    /// kinds, exactly once each.
    ///
    /// The compile-error guard lives in `trigger_kind_after`'s exhaustive
    /// match; this is its runtime witness, and it is what catches a chain that
    /// was made exhaustive by pointing two arms at the same successor.
    #[test]
    fn the_trigger_precondition_chain_visits_every_trigger_kind_once() {
        let mut names: Vec<&'static str> = Vec::new();
        let mut kind = Some(FIRST_TRIGGER_KIND);
        while let Some(k) = kind {
            names.push(crate::game::metrics::trigger_name(&k));
            assert!(names.len() <= 8, "the trigger-kind chain does not terminate");
            kind = trigger_kind_after(&k);
        }
        assert_eq!(
            names,
            [
                "TradingHousesComplete",
                "ShophousesComplete",
                "OrderDeckEmpty",
                "ChurchTrackFull",
                "CustomsTrackFull",
            ],
            "the precondition set is not `EndGameTrigger`'s five kinds in the engine's own order",
        );
    }

    /// A CLOCK position one pawn from a full track finds its firing move
    /// THROUGH the short circuit.
    #[test]
    fn the_short_circuit_finds_a_clock_positions_firing_move() {
        let rules = Rules::standard();
        let clock = harvest(&rules, C_MASTER, 4, "one track pawn from the end-game clock", &mut |s| {
            !s.end_game_triggered
                && (s.church_track.total_assistants() + 1 >= s.config.track_trigger()
                    || s.customs_track.total_assistants() + 1 >= s.config.track_trigger())
                && !firing_moves(s, &rules).is_empty()
        });

        assert!(any_trigger_within_one_move(&clock), "the short circuit skipped a clock position");
        let firing = firing_moves(&clock, &rules);
        assert!(!firing.is_empty());
        // Each move it named really does fire, applied independently through
        // the CHECKED boundary rather than through the helper's own path.
        for mv in &firing {
            let next = apply_move(&clock, mv, &rules).expect("a firing move is a legal move");
            assert!(next.end_game_triggered, "firing_moves named a move that fires nothing: {mv:?}");
        }
    }

    /// A BUILDING position one token from a complete set finds its firing move
    /// through the short circuit too.
    ///
    /// This shape does not occur in the rung corpus — 2P rung games end on the
    /// church clock — so the fixture is CONSTRUCTED: a harvested position with
    /// the mover's trading houses moved warehouse → built (conservation
    /// preserved, `check_invariants` asserted) so that the fourth one is one
    /// build away. Without this cell the two building triggers would be
    /// precondition arms nothing ever exercises, which is the exact bias the
    /// exhaustive enumeration exists to prevent.
    #[test]
    fn the_short_circuit_finds_a_building_positions_firing_move() {
        let rules = Rules::standard();
        let mut built = harvest(
            &rules,
            C_MASTER,
            4,
            "a Building-phase position where the mover may build a trading house",
            &mut |s| {
                !s.end_game_triggered
                    && get_legal_moves(s, &rules).contains(&Move::BuildTradingHouse)
            },
        );

        let mover = built.current_player;
        // Three trading houses already standing, on areas nobody else claims —
        // warehouse → built, so the four-token conservation is untouched.
        let taken: Vec<AreaId> = built
            .players
            .iter()
            .flat_map(|p| p.trading_houses.built.iter().copied())
            .collect();
        let president_area = match built.player(mover).president_location {
            PresidentLocation::OnBoard(a) => a,
            PresidentLocation::InHand => panic!("a Building-phase mover has its President placed"),
        };
        let spare: Vec<AreaId> = built
            .board_layout
            .areas
            .keys()
            .copied()
            .filter(|a| *a != president_area && !taken.contains(a))
            .take(3)
            .collect();
        assert_eq!(spare.len(), 3, "the board has too few free areas for this fixture");
        {
            let p = built.player_mut(mover);
            for area in &spare {
                if p.trading_houses.built.contains(area) {
                    continue;
                }
                if p.trading_houses.in_warehouse > 0 {
                    p.trading_houses.in_warehouse -= 1;
                } else {
                    p.trading_houses.in_hand -= 1;
                }
                p.trading_houses.built.push(*area);
            }
            // The fourth token has to be IN HAND for the build to be legal.
            if p.trading_houses.in_hand == 0 && p.trading_houses.in_warehouse > 0 {
                p.trading_houses.in_warehouse -= 1;
                p.trading_houses.in_hand += 1;
            }
        }
        assert_eq!(
            built.player(mover).trading_houses.total_built(),
            TRADING_HOUSES_COMPLETE - 1,
            "the fixture is not one trading house from complete",
        );
        assert!(check_invariants(&built, &rules).is_ok(), "the constructed fixture is not engine-valid");

        assert!(
            any_trigger_within_one_move(&built),
            "the short circuit skipped a one-building-from-complete position",
        );
        let firing = firing_moves(&built, &rules);
        assert!(
            firing.contains(&Move::BuildTradingHouse),
            "the last trading house is not among the firing moves: {firing:?}",
        );
        let next =
            apply_move(&built, &Move::BuildTradingHouse, &rules).expect("the build is legal");
        assert!(next.end_game_triggered);
        assert!(matches!(
            next.trigger_reason,
            Some(EndGameTrigger::TradingHousesComplete(p)) if p == mover
        ));
    }

    /// The short circuit NEVER LIES: where it declines to probe, an exhaustive
    /// probe finds nothing either.
    ///
    /// Two halves, and the second is the one that matters. The first says a
    /// mid-game position short-circuits to an empty answer; the second re-runs
    /// the SAME question WITHOUT the short circuit — every legal move applied
    /// and checked by hand — and gets the same empty answer, which is the only
    /// way to tell "nothing to find" from "did not look".
    #[test]
    fn the_short_circuit_never_skips_a_position_that_had_a_firing_move() {
        let rules = Rules::standard();
        let mut checked = 0usize;
        let mut skipped = 0usize;
        let mut schedule = SeedSchedule::diagnostic(C_MASTER);
        let world = schedule.next_world();
        let mut make = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
            GENERATED_SEAT.build(&rules, agent_stream_seed(world, seat, Slot(seat.0))).0
        };
        let stats = run_arena_game(world, 2, &rules, &mut make, true).expect("the game completes");
        let events = &stats.record.events;
        let mut state = state_at(&stats.record, 0, &rules).expect("position 0 replays");
        for position in 0..=events.len() {
            if !state.end_game_triggered && !any_trigger_within_one_move(&state) {
                skipped += 1;
                // The exhaustive probe, written out here rather than called, so
                // this is a SECOND opinion and not the same code twice.
                let exhaustive: Vec<Move> = get_legal_moves(&state, &rules)
                    .into_iter()
                    .filter(|mv| {
                        apply_move_unchecked(&state, mv, &rules)
                            .map(|next| next.end_game_triggered)
                            .unwrap_or(false)
                    })
                    .collect();
                assert!(
                    exhaustive.is_empty(),
                    "the short circuit skipped position {position}, where {} moves fire the \
                     trigger: {exhaustive:?}",
                    exhaustive.len(),
                );
                assert!(firing_moves(&state, &rules).is_empty());
                checked += 1;
            }
            if let Some(event) = events.get(position) {
                state = replay_from(&state, std::slice::from_ref(event), &rules)
                    .expect("the game replays event by event");
            }
        }
        assert!(
            skipped > 20,
            "anti-vacuity: the short circuit declined only {skipped} positions of this game, so \
             it is not being exercised",
        );
        assert_eq!(checked, skipped);
    }

    /// `firing_moves` means SETS, not "the result has the flag": on an
    /// already-triggered position it is EMPTY rather than the whole legal list.
    #[test]
    fn firing_moves_is_empty_once_the_trigger_has_already_fired() {
        let rules = Rules::standard();
        let (final_round, _) = RootSource::Generated {
            predicate: RootPredicate::FinalRoundAlreadyTriggered,
            master: GEN_MASTER,
            ordinal: 0,
        }
        .resolve(&rules)
        .expect("the scan supplies a final-round root");
        assert!(final_round.end_game_triggered);
        assert!(
            !get_legal_moves(&final_round, &rules).is_empty(),
            "anti-vacuity: this root has no legal moves, so an empty answer says nothing",
        );
        assert!(
            firing_moves(&final_round, &rules).is_empty(),
            "every legal move at an already-triggered position was called a FIRING move",
        );
    }

    // -----------------------------------------------------------------------
    // Item 4: `true_margin_now`
    // -----------------------------------------------------------------------

    /// The margin is the mover's total minus the best-RANKED rival's, and on a
    /// fixture whose only asymmetry is base VP it is exactly that difference.
    #[test]
    fn true_margin_now_is_the_movers_lead_over_the_best_ranked_rival() {
        let rules = Rules::standard();
        let mut state = root_2p(&rules, world(36).0);
        // Level every component the two seats could differ on, so the only
        // asymmetry left is the victory points set below. The tracks hold
        // nothing but the 2P dummy, and neither seat owns a technology, so
        // every majority is uncontested by construction.
        for p in &mut state.players {
            p.money = 4;
            p.copper = 0;
            p.silk = 0;
            p.tea = 0;
            p.seafood = 0;
            p.imported_goods = 0;
        }
        state.player_mut(PlayerId::new(0)).victory_points = 30;
        state.player_mut(PlayerId::new(1)).victory_points = 10;
        assert!(check_invariants(&state, &rules).is_ok());
        assert_eq!(state.current_player, PlayerId::new(0), "the fixture assumes seat 0 moves");

        let result = final_result(&state, &rules);
        // The premise, verified rather than asserted in prose: everything but
        // base VP is equal between the seats.
        let mut components: Vec<(u32, u32, u32, u32, u32)> = result
            .standings
            .iter()
            .map(|s| {
                (
                    s.breakdown.church,
                    s.breakdown.customs,
                    s.breakdown.technology,
                    s.breakdown.country,
                    s.breakdown.remainder,
                )
            })
            .collect();
        components.dedup();
        assert_eq!(components.len(), 1, "the fixture is not symmetric outside base VP: {result:?}");

        assert_eq!(true_margin_now(&state, &rules), 20, "30 − 10 is the whole margin here");

        // MOVER-RELATIVE: the same board read from the other seat is the
        // negation, which is what "the mover's margin" means at two seats.
        let mut other = state.clone();
        other.current_player = PlayerId::new(1);
        assert_eq!(true_margin_now(&other, &rules), -20);

        // And it is a total, not a rank: one more VP is one more margin.
        let mut richer = state.clone();
        richer.player_mut(PlayerId::new(0)).victory_points += 7;
        assert_eq!(true_margin_now(&richer, &rules), 27);
    }

    /// The 2P dummy PARTICIPATES: the same single church pawn is worth 6 VP of
    /// margin above the dummy's faith-7 space and 3 below it.
    ///
    /// Finding D1's majority rule, hand-computed. The church majority pays 6 to
    /// the most pawns and 3 to the second, count ties broken by the RIGHTMOST
    /// occupied space; the dummy holds faith 7 and one pawn, so a rival pawn at
    /// faith 8 wins the count tie and takes the 6, while a pawn at faith 5
    /// loses it and takes the 3 the dummy leaves behind. Margin against a
    /// pawnless mover: −6 and −3.
    #[test]
    fn true_margin_now_counts_the_two_player_dummy_in_the_majorities() {
        let rules = Rules::standard();
        let mut base = root_2p(&rules, world(37).0);
        for p in &mut base.players {
            p.money = 4;
            p.copper = 0;
            p.silk = 0;
            p.tea = 0;
            p.seafood = 0;
            p.imported_goods = 0;
        }
        let dummy_index = ChurchTrack::faith_to_index(7).expect("faith 7 is on the track");
        assert_eq!(
            base.church_track.slots[dummy_index].map(|s| s.player),
            Some(PlayerId::DUMMY),
            "anti-vacuity: the 2P dummy is not on faith 7, so nothing here is about the dummy",
        );
        assert_eq!(base.church_track.total_assistants(), 0, "the fixture starts with no real pawns");

        let rival = rival_of(&base);
        let with_pawn_at = |faith: u8| {
            let mut state = base.clone();
            let index = ChurchTrack::faith_to_index(faith).expect("a track space");
            state.church_track.slots[index] =
                Some(crate::types::ChurchSlot { player: rival, arrival_order: 1 });
            let p = state.player_mut(rival);
            p.assistants.in_hand -= 1;
            p.assistants.on_church += 1;
            p.church_positions.push(faith);
            assert!(check_invariants(&state, &rules).is_ok(), "the church fixture is not valid");
            state
        };

        let above = with_pawn_at(8);
        assert_eq!(
            true_margin_now(&above, &rules),
            -6,
            "a rival pawn RIGHT of the dummy takes the 6-VP church majority outright",
        );
        let below = with_pawn_at(5);
        assert_eq!(
            true_margin_now(&below, &rules),
            -3,
            "a rival pawn LEFT of the dummy loses the count tie and takes the second place",
        );
    }

    /// Level totals read margin 0 while `final_result` still RANKS the seats —
    /// margin 0 is not a win, and the rival is the best-RANKED seat rather than
    /// the highest raw total.
    #[test]
    fn a_level_pair_reads_margin_zero_while_the_standings_still_rank() {
        let rules = Rules::standard();
        let mut state = root_2p(&rules, world(38).0);
        for p in &mut state.players {
            p.money = 4;
            p.copper = 0;
            p.silk = 0;
            p.tea = 0;
            p.seafood = 0;
            p.imported_goods = 0;
            p.victory_points = 17;
        }
        assert!(check_invariants(&state, &rules).is_ok());

        let result = final_result(&state, &rules);
        assert_eq!(
            result.standings[0].breakdown.total, result.standings[1].breakdown.total,
            "anti-vacuity: the fixture is not level",
        );
        assert_eq!(result.standings[0].rank, 1);
        assert_eq!(result.standings[1].rank, 2);
        assert_ne!(result.standings[0].player, result.standings[1].player);

        // Both seats read ZERO — and the seat the standings put SECOND reads
        // zero too, which is the whole point: a margin of 0 does not say the
        // mover is winning, it says the ranking decides.
        for seat in [PlayerId::new(0), PlayerId::new(1)] {
            let mut from_seat = state.clone();
            from_seat.current_player = seat;
            assert_eq!(true_margin_now(&from_seat, &rules), 0);
        }
        let loser = result.standings[1].player;
        assert_ne!(result.winner, loser, "the level pair still has exactly one winner");
    }

    // -----------------------------------------------------------------------
    // Item 5: the ecologies
    // -----------------------------------------------------------------------

    /// v1's AMENDED identity: the stride cap, the raised bound, and the version
    /// string that rides both.
    #[test]
    fn the_v1_ecology_carries_its_amended_identity() {
        let ecology = GeneratingEcology::v1();
        assert_eq!(ecology.id, "driven-generated-v1.1", "the amendment did not bump the version");
        assert_eq!(ecology.id, GENERATED_ECOLOGY_V1);
        assert_eq!(ecology.per_game_cap, 2);
        assert_eq!(ecology.per_game_cap, GENERATED_STRIDE_CAP);
        assert_eq!(ecology.games, 16, "the bound must cover pilot 4 + confirmatory 6 + escrow 6");
        assert_eq!(ecology.games, GENERATED_SCAN_GAMES);
        assert_eq!(ecology.players, 2);
        assert_eq!(ecology.seats, vec![GENERATED_SEAT.echo(); 2]);
        assert!(ecology.seats[0].contains("family-weighted"), "v1 is the rung ecology: {:?}", ecology.seats);
        assert!(!ecology.driver.is_empty() && !ecology.seed_derivation.is_empty());
        // The supply arithmetic the raise was sized by, stated as arithmetic.
        assert!(
            ecology.games * ecology.per_game_cap >= 4 + 6 + 6,
            "the strided supply cannot cover one predicate's blocks",
        );
    }

    /// v2's identity — pinned WITHOUT running a scan, because a lookahead scan
    /// is up to 240 games of both-seats search and this is a unit test.
    ///
    /// **Since the step-8 re-pin (V3 spec §5.1)** the seats are the PINNED
    /// explicit-V2 construction, and the recorded echo is that construction's
    /// own resolved provenance. That it ALSO equals the live shipped default's
    /// echo today is the padding identity (`pinned-V2 == padded-live-V2`),
    /// asserted separately below as a TRANSCRIPTION FACT — when a later freeze
    /// re-points the live default, DELETE that assertion in the same commit
    /// and keep the pinned echo, exactly as
    /// `the_pre_freeze_table_is_the_corpus_generator...`'s doc prescribes.
    #[test]
    fn the_v2_lookahead_ecology_names_the_pinned_explicit_v2_seat_at_both_seats() {
        let config = v2_lookahead_config();
        assert_eq!(
            config.seat,
            GeneratingSeat::PinnedLookaheadV2,
            "the live configuration must build the pinned seat, not the re-pointable default",
        );
        let ecology = config.ecology;
        assert_eq!(ecology.id, "driven-generated-v2.3-lookahead");
        assert_eq!(ecology.id, GENERATED_ECOLOGY_V2_LOOKAHEAD);
        assert_eq!(ecology.players, 2, "the canon priors are 2P priors");
        assert_eq!(ecology.games, 240);
        assert_eq!(ecology.games, GENERATED_V2_SCAN_GAMES);
        assert_eq!(ecology.per_game_cap, GENERATED_STRIDE_CAP, "both ecologies stride the same");
        assert_eq!(
            ecology.seats,
            vec![GeneratingSeat::PinnedLookaheadV2.echo(); 2],
            "the layer-2 ecology must seat the PINNED explicit-V2 construction, resolved",
        );
        // The pinned echo carries WeightTable::V2's own table identity — the
        // always-on half of V3 §5.1's behavior-identity pin: a construction
        // that drifted off the explicit table would echo a different identity.
        assert!(
            GeneratingSeat::PinnedLookaheadV2.echo().contains(&format!(
                "weights={}",
                crate::game::greedy::weight_table_identity(&WeightTable::V2)
            )),
            "the pinned seat echo must resolve WeightTable::V2's identity: {}",
            GeneratingSeat::PinnedLookaheadV2.echo(),
        );
        // TRANSCRIPTION FACT (deletable at a live-default re-point, per the
        // doc above): today the pinned construction and the live shipped
        // default are the same identity, echo-for-echo.
        assert_eq!(
            GeneratingSeat::PinnedLookaheadV2.echo(),
            Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
            "pinned-V2 == padded-live-V2 at the adoption commit",
        );
        // The two ecologies share their driver and seed derivation verbatim —
        // one spelling each, so a reworded sentence cannot make them look like
        // different instruments.
        let v1 = GeneratingEcology::v1();
        assert_eq!(ecology.driver, v1.driver);
        assert_eq!(ecology.seed_derivation, v1.seed_derivation);
        assert_ne!(ecology.id, v1.id, "two ecologies must not share an identity string");
        assert_ne!(ecology.seats, v1.seats, "two ecologies must not seat the same contestant");
    }

    /// v2's EXTENDED identity: the raised bound, the version string that rides
    /// it, and the cap that did NOT move — which is what makes the raise an
    /// extension rather than a re-draw.
    ///
    /// The v1 pin's TRANSCRIPTION pattern, for the v1 pin's reason: the literal
    /// is written out here as well as asserted against the const, so editing the
    /// const alone cannot move a recorded identity silently — a raise has to be
    /// transcribed twice, in the same commit, on purpose. BOTH pre-extension id
    /// strings survive HERE and nowhere else in the crate, as history.
    #[test]
    fn the_v2_ecology_carries_its_extended_identity() {
        let ecology = GeneratingEcology::v2_lookahead();
        assert_eq!(
            ecology.id, "driven-generated-v2.3-lookahead",
            "the games raise did not bump the version; the step-8 seat re-pin did",
        );
        assert_eq!(ecology.id, GENERATED_ECOLOGY_V2_LOOKAHEAD);
        for retired in [
            "driven-generated-v2-lookahead",
            "driven-generated-v2.1-lookahead",
            "driven-generated-v2.2-lookahead",
        ] {
            assert_ne!(
                ecology.id, retired,
                "a pre-extension id must survive as test history and nowhere else",
            );
        }
        assert_eq!(
            ecology.games, 240,
            "the bound family 2's SECOND exhaustion bought: its control arm realized 26 hits \
             in 120 games against the 28 its block demands, a measured 0.217/game",
        );
        assert_eq!(ecology.games, GENERATED_V2_SCAN_GAMES);
        // The STRIDE did not move, and that is the prefix-stability argument in
        // one assertion: same master, same cap, same walk order, so the strided
        // sequence at 240 games is the sequence at 120 with more on the end —
        // itself the sequence at 24 with more on the end — and every ordinal
        // already read still resolves where it was read.
        assert_eq!(ecology.per_game_cap, 2);
        assert_eq!(ecology.per_game_cap, GENERATED_STRIDE_CAP);
        // The capacity arithmetic, as arithmetic: every ordinal the freeze
        // pre-declared for ONE layer-2 predicate — pilot 4 + confirmatory 24 +
        // the VOID-disposition extension 8 + escrow 6 (post-fix re-read) + 6
        // (final-controller) = 48 — with the raises leaving 10× that. Capacity is
        // not supply: this says the ordinal SPACE is wide enough, never that the
        // predicate hits that often — the very distinction both exhaustions cost.
        assert!(
            ecology.games * ecology.per_game_cap >= 4 + 24 + 8 + 6 + 6,
            "the strided space cannot cover one layer-2 predicate's pre-declared ordinals",
        );
    }

    /// Each predicate names the ecology its family scans under, and the two
    /// layers are disjoint.
    #[test]
    fn each_predicate_names_its_own_generating_ecology() {
        for predicate in RootPredicate::ALL {
            let expected = match predicate {
                RootPredicate::FinalRoundAlreadyTriggered | RootPredicate::TriggerRoundNotFinal => {
                    GeneratingEcology::v1()
                }
                RootPredicate::EndFirableAheadInTruth
                | RootPredicate::EndFirableBehindInTruth
                | RootPredicate::ExclusiveRaceOneStep
                | RootPredicate::AchievementClaimAvailable
                | RootPredicate::PlacementForkSameCount
                | RootPredicate::OpponentSittingExecutable => GeneratingEcology::v2_lookahead(),
            };
            assert_eq!(
                predicate.ecology(),
                expected,
                "{} scans under the wrong ecology",
                predicate.as_str(),
            );
        }
        assert_eq!(
            RootPredicate::ALL.iter().filter(|p| p.ecology() == GeneratingEcology::v1()).count(),
            2,
            "family 1's pair is the whole of the rung-scanned set",
        );
    }

    /// The predicate names are stable, kebab-case and distinct — a descriptor
    /// and a runner spec both address a predicate by them.
    #[test]
    fn the_predicate_names_are_kebab_case_and_distinct() {
        let names: Vec<&str> = RootPredicate::ALL.iter().map(|p| p.as_str()).collect();
        assert_eq!(
            names,
            [
                "final-round-already-triggered",
                "trigger-round-not-final",
                "end-firable-ahead-in-truth",
                "end-firable-behind-in-truth",
                "exclusive-race-one-step",
                "achievement-claim-available",
                "placement-fork-same-count",
                "opponent-sitting-executable",
            ],
        );
        let unique: std::collections::BTreeSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), names.len(), "two predicates share a name");
        for name in &names {
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "predicate name {name} is not kebab-case",
            );
        }
    }

    // -----------------------------------------------------------------------
    // Item 6: the scan's stride
    // -----------------------------------------------------------------------

    /// The scan STRIDES: at most `per_game_cap` hits per generated game enter
    /// the ordinal sequence, and the hits it skips consume no ordinal.
    ///
    /// The sharp assertion is the last one. Ordinals 0 and 1 are game 0's first
    /// two hits and ordinal 2 is game 1's FIRST — not game 0's third, which is
    /// what an unstrided scan would serve and which this cell reconstructs
    /// independently so the difference is visible rather than assumed.
    #[test]
    fn the_scan_strides_at_most_two_hits_per_generated_game() {
        let rules = Rules::standard();
        let predicate = RootPredicate::FinalRoundAlreadyTriggered;
        let cap = GeneratingEcology::v1().per_game_cap;
        assert_eq!(cap, 2, "this cell is written for a cap of 2");

        // The two games' own hit sequences, rebuilt from the ecology's parts
        // rather than through `generate_root` — the same anchor construction the
        // ordinal cell uses, extended to a second game.
        let mut schedule = SeedSchedule::diagnostic(GEN_MASTER);
        let mut per_game: Vec<Vec<String>> = Vec::new();
        for _ in 0..2 {
            let world = schedule.next_world();
            let mut make = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
                GENERATED_SEAT.build(&rules, agent_stream_seed(world, seat, Slot(seat.0))).0
            };
            let stats = run_arena_game(world, 2, &rules, &mut make, true)
                .expect("the ecology's games must complete");
            let events = &stats.record.events;
            let mut state = state_at(&stats.record, 0, &rules).expect("position 0 replays");
            let mut hits = Vec::new();
            for position in 0..=events.len() {
                if predicate.holds(&state, &rules) {
                    hits.push(state_digest(&state));
                }
                if let Some(event) = events.get(position) {
                    state = replay_from(&state, std::slice::from_ref(event), &rules)
                        .expect("the generated game replays event by event");
                }
            }
            per_game.push(hits);
        }
        assert!(
            per_game[0].len() > cap as usize,
            "anti-vacuity: game 0 holds only {} hits, so no hit is ever SKIPPED and this cell \
             tests nothing",
            per_game[0].len(),
        );
        assert!(!per_game[1].is_empty(), "anti-vacuity: game 1 holds no hit to stride into");

        let resolve = |ordinal: u32| {
            RootSource::Generated { predicate, master: GEN_MASTER, ordinal }
                .resolve(&rules)
                .unwrap_or_else(|e| panic!("ordinal {ordinal} must resolve: {e}"))
        };

        for ordinal in 0..cap {
            let (state, descriptor) = resolve(ordinal);
            assert_eq!(
                state_digest(&state),
                per_game[0][ordinal as usize],
                "ordinal {ordinal} is not game 0's hit {ordinal}",
            );
            match descriptor.origin {
                RootOrigin::Generated { game_index, ref ecology, .. } => {
                    assert_eq!(game_index, 0, "ordinal {ordinal} came from a later game");
                    assert_eq!(ecology.per_game_cap, cap, "the descriptor misreports the stride");
                }
                ref other => panic!("expected a Generated origin, got {other:?}"),
            }
        }

        let (strided, descriptor) = resolve(cap);
        match descriptor.origin {
            RootOrigin::Generated { game_index, .. } => assert!(
                game_index >= 1,
                "ordinal {cap} came from game {game_index} — the cap did not stride",
            ),
            ref other => panic!("expected a Generated origin, got {other:?}"),
        }
        assert_eq!(
            state_digest(&strided),
            per_game[1][0],
            "ordinal {cap} is not game 1's FIRST hit",
        );
        assert_ne!(
            state_digest(&strided),
            per_game[0][cap as usize],
            "ordinal {cap} served game 0's hit {cap}, so skipped hits still consume ordinals",
        );
    }

    /// The exhaustion refusal still names its bound after the stride, and the
    /// supply it reports is the STRIDED supply.
    #[test]
    fn the_exhaustion_refusal_reports_the_strided_supply_and_its_bound() {
        let rules = Rules::standard();
        let predicate = RootPredicate::FinalRoundAlreadyTriggered;
        let ecology = GeneratingEcology::v1();
        let err = RootSource::Generated { predicate, master: GEN_MASTER, ordinal: 1_000_000 }
            .resolve(&rules)
            .expect_err("no scan holds a millionth final-round root");
        match err {
            RootError::Exhausted { predicate: p, ordinal, found, games } => {
                assert_eq!(p, predicate);
                assert_eq!(ordinal, 1_000_000);
                assert_eq!(games, ecology.games, "the refusal misnames the bound");
                assert!(found > 0, "anti-vacuity: the scan found nothing at all");
                assert!(
                    found <= ecology.games * ecology.per_game_cap,
                    "the scan admitted {found} hits, more than {} games × cap {} allows — the \
                     stride is not being enforced",
                    ecology.games,
                    ecology.per_game_cap,
                );
            }
            other => panic!("expected Exhausted, got {other:?}"),
        }
    }

    // =======================================================================
    // PHASE C T2: the turn-grain driver
    // =======================================================================

    /// T2's committed diagnostic DECISION seed — the same discipline
    /// [`DECISION_SEED`] carries, one grain up: a turn is keyed by an explicit
    /// single seed the caller supplies, and a fixture cell fixes it so the turn
    /// is one reproducible thing.
    const TURN_SEED: u64 = 0x0C02_2026_0818_9991;

    /// A root at the START of a mover's turn, out of a cheap rung game.
    ///
    /// `TurnPhase::AdditionalActionFirst` is the guarantee these cells need, and
    /// it is a guarantee about the PHASE MACHINE rather than about what the
    /// mover chose: a turn entered at its first phase still has to pass through
    /// Placement, Movement, the area action and the second additional-action
    /// window, so it is ≥2 decisions under ANY policy. That is what makes "the
    /// counters are not one decision's" and "the bound refuses mid-turn"
    /// statable without a cell depending on a particular line of play.
    ///
    /// Both seats of the generating game are rungs, so the fixture pays for no
    /// search at all (design §6's wall discipline) — the searching, where a cell
    /// wants any, is what the cell itself does at the root.
    fn turn_start_root(rules: &Rules, seed: WorldSeed, from: usize) -> GameState {
        let stats = rung_game(rules, seed);
        let events = stats.record.events.len();
        assert!(from < events, "the generating game is only {events} events long");
        let end = (from + 40).min(events);
        (from..end)
            .find_map(|k| {
                let state = state_at(&stats.record, k, rules).ok()?;
                (state.turn_phase == TurnPhase::AdditionalActionFirst
                    && !is_game_over(&state)
                    && !state.end_game_triggered)
                    .then_some(state)
            })
            .unwrap_or_else(|| panic!("no turn start in events {from}..{end}"))
    }

    /// A mid-game root where the mover has a REAL CHOICE — more than one legal
    /// move.
    ///
    /// A mid-game root offering the mover at least `width` legal moves.
    ///
    /// [`turn_start_root`]'s guarantee is about turn LENGTH, and the two are
    /// independent: an `AdditionalActionFirst` position usually offers exactly
    /// `EndAdditionalPhase` and nothing else, which is a fine root for a stop
    /// rule and a useless one for a join — a FORCED decision agrees with
    /// whatever the seeding does, so it would pass a join cell whose seeding was
    /// wrong. This scans for the branching half instead, and `width` is what
    /// makes an accidental agreement unlikely rather than merely possible.
    fn branching_root(rules: &Rules, seed: WorldSeed, from: usize, width: usize) -> GameState {
        let stats = rung_game(rules, seed);
        let events = stats.record.events.len();
        assert!(from < events, "the generating game is only {events} events long");
        let end = (from + 40).min(events);
        (from..end)
            .find_map(|k| {
                let state = state_at(&stats.record, k, rules).ok()?;
                (!is_game_over(&state)
                    && !state.end_game_triggered
                    && get_legal_moves(&state, rules).len() >= width)
                    .then_some(state)
            })
            .unwrap_or_else(|| panic!("no {width}-wide decision in events {from}..{end}"))
    }

    /// The cheapest honest mover there is, for the cells whose subject is the
    /// LOOP rather than the contestant.
    fn rung_mover() -> SeatSpec {
        rung_seats()[0]
    }

    /// The HAND-OFF stop is exact: the turn is the mover's own moves, every one
    /// of them decided in a state the mover was on the move in, and the hand-off
    /// is the first state that is not.
    ///
    /// The exactness half is replayed through the CHECKED `apply_move` — a
    /// second, independent application of the reported moves — so a driver that
    /// reported a move list and a hand-off state that do not belong together
    /// fails here rather than passing on the strength of its own arithmetic.
    #[test]
    fn a_turn_stops_at_the_hand_off_and_names_exactly_the_movers_moves() {
        let rules = Rules::standard();
        let root = turn_start_root(&rules, world(70), 300);
        let mover = root.current_player;

        let outcome = drive_turn(&root, rung_mover(), &rules, TURN_SEED, &DrivenOpts::default())
            .expect("a mid-game turn drives");

        assert!(
            outcome.moves.len() >= 2,
            "anti-vacuity: a turn entered at its first phase is more than one decision, this one \
             ran {}",
            outcome.moves.len(),
        );
        assert_eq!(outcome.decisions, outcome.moves.len() as u64);
        assert!(!outcome.game_over, "a mid-game turn hands off; it does not end the game");
        assert!(!is_game_over(&outcome.handoff));
        assert_ne!(
            outcome.handoff.current_player, mover,
            "the run did not stop at a hand-off",
        );

        let mut state = root.clone();
        for (i, applied) in outcome.moves.iter().enumerate() {
            assert_eq!(applied.seat, mover, "move {i} is attributed to another seat");
            assert_eq!(
                state.current_player, mover,
                "move {i} was decided in a state the mover was not on the move in",
            );
            state = apply_move(&state, &applied.mv, &rules)
                .unwrap_or_else(|e| panic!("move {i} does not re-apply: {e}"));
        }
        assert_eq!(
            state_digest(&state),
            state_digest(&outcome.handoff),
            "the reported hand-off is not where the reported moves lead",
        );
        assert_ne!(state.current_player, mover);
        assert_eq!(
            outcome.counters,
            SeatCounters::NONE,
            "a rung explains nothing — every counter is absent, not zero",
        );
    }

    /// The FIRST-DECISION JOIN: at one seed, a turn's first move is the move
    /// [`root_table`] reports as the winner.
    ///
    /// This is what makes D7's counter check and the family read the same
    /// question — the check runs a table at replication 0's seed and the read
    /// runs a turn at that seed, and without this they would be two decisions
    /// that merely look alike. It holds because the turn constructs ONE agent at
    /// `decision_seed` and the table constructs a fresh one at the same seed:
    /// at stream position 0 those are the same agent.
    ///
    /// **The ONE cell in this section with a SEARCHING subject** (design §6's
    /// wall discipline): a whole turn under the uncapped shipped S1 arm plus one
    /// table at the same root — every other cell here runs a rung or greedy.
    /// The budget is ~3s debug and the MEASURED wall is 0.05s, because the
    /// root the scan lands on is an ordinary mid-game decision (13 candidates,
    /// an 8-decision turn, 174 scored successors at HEAD) rather than a
    /// money-saturated Movement point. The budget is the ceiling this cell must
    /// not grow past, not a prediction of it.
    #[test]
    fn the_first_move_of_a_turn_is_root_tables_winner_at_the_same_seed() {
        let rules = Rules::standard();
        let root = branching_root(&rules, world(71), 300, 8);
        let seat = root.current_player;
        let contestant = Contestant::LOOKAHEAD_V1_DEFAULT;

        let table = root_table(&root, seat, contestant, &rules, TURN_SEED)
            .expect("the shipped S1 arm tabulates an organic root");
        let outcome = drive_turn(
            &root,
            SeatSpec::Contestant(contestant),
            &rules,
            TURN_SEED,
            &DrivenOpts::default(),
        )
        .expect("the same contestant drives the turn");

        assert!(
            table.candidates() >= 8,
            "anti-vacuity: a FORCED (or near-forced) decision joins whatever the seeding does — \
             this root offered {}",
            table.candidates(),
        );
        assert_eq!(
            outcome.moves.first().map(|a| a.mv.clone()),
            Some(table.winner.clone()),
            "the turn's first decision is not the table's winner at the same seed",
        );
        assert_eq!(outcome.moves[0].seat, seat);
        assert!(
            !outcome.game_over && outcome.handoff.current_player != seat,
            "the searching mover drove a whole turn and handed off, so the join is not a \
             one-decision special case",
        );
    }

    /// PREFIX EQUIVALENCE — the pin that keeps the turn loop honest against the
    /// play half BEYOND decision 0.
    ///
    /// `drive_turn` is a THIRD per-decision loop (`play_from_state` runs to
    /// terminal and exposes no stop hook), and the first-decision join above
    /// covers exactly one decision. At `decision_seed = agent_stream_seed(world,
    /// mover, Slot(mover))` the mover's stream is the one
    /// [`drive_to_terminal`] gives it at that world, so the turn's move list
    /// must be the mover's OWN PREFIX of the full run — up to the first
    /// opponent move. A loop that advanced the stream differently, re-built its
    /// agent per decision, or applied the moves in another order diverges here.
    #[test]
    fn a_turns_moves_are_the_terminal_drivers_own_prefix_at_the_arena_seed() {
        let rules = Rules::standard();
        let root = turn_start_root(&rules, world(72), 300);
        let mover = root.current_player;
        let master = world(73);
        let seed = agent_stream_seed(master, mover, Slot(mover.0));

        let turn = drive_turn(&root, rung_mover(), &rules, seed, &DrivenOpts::default())
            .expect("the turn drives");
        let full = drive_to_terminal(&root, &rung_seats(), &rules, master, DrivenOpts::default())
            .expect("the whole game drives");

        let prefix: Vec<Move> = full
            .applied
            .iter()
            .take_while(|a| a.seat == mover)
            .map(|a| a.mv.clone())
            .collect();
        let played: Vec<Move> = turn.moves.iter().map(|a| a.mv.clone()).collect();
        assert_same_moves(&played, &prefix, "the turn loop left the play half's line");

        assert!(prefix.len() >= 2, "anti-vacuity: the prefix is {} moves", prefix.len());
        assert!(
            full.applied.len() > prefix.len(),
            "anti-vacuity: the mover's prefix is the WHOLE game, so this compares a game to itself",
        );
        assert_ne!(
            full.applied[prefix.len()].seat,
            mover,
            "the prefix does not end at an opponent move",
        );
    }

    /// Same root, same seed → the same turn, twice.
    ///
    /// The third cell is the anti-vacuity half: a turn that ignored its seed
    /// would satisfy the first two however it played, so a DIFFERENT seed must
    /// move the run.
    #[test]
    fn driving_the_same_turn_twice_is_byte_identical() {
        let rules = Rules::standard();
        let root = turn_start_root(&rules, world(74), 300);
        let opts = DrivenOpts::default();

        let first = drive_turn(&root, rung_mover(), &rules, TURN_SEED, &opts).expect("first run");
        let second = drive_turn(&root, rung_mover(), &rules, TURN_SEED, &opts).expect("second run");

        assert_eq!(
            state_digest(&first.handoff),
            state_digest(&second.handoff),
            "the hand-off moved between two identical turns",
        );
        assert_eq!(first.moves, second.moves, "the move lists differ");
        assert_eq!(first, second, "the outcomes differ beyond the digest and the move list");
        assert!(first.moves.len() >= 2, "anti-vacuity: {} moves", first.moves.len());

        let elsewhere =
            drive_turn(&root, rung_mover(), &rules, TURN_SEED ^ 0x5555_5555_5555_5555, &opts)
                .expect("another seed still drives");
        assert!(
            elsewhere.moves != first.moves
                || state_digest(&elsewhere.handoff) != state_digest(&first.handoff),
            "the turn does not depend on its decision seed, so nothing above witnesses determinism",
        );
    }

    /// An ALREADY-TERMINAL root is an `Ok` with nothing in it — the zero-move
    /// outcome, `game_over` true, and the root itself as the hand-off.
    ///
    /// [`drive_to_terminal`]'s contract at this grain, and the seat is an
    /// EVALUATOR for the same reason its cell uses one: `Some(0)` is a seat that
    /// HAS an explanation path and was asked for no decisions, which is a
    /// different statement from `None`.
    #[test]
    fn a_terminal_root_drives_a_zero_move_turn_that_reads_game_over() {
        let rules = Rules::standard();
        let (terminal, mover) = terminal_building_root(&rules, world(75).0);

        let outcome = drive_turn(
            &terminal,
            SeatSpec::Contestant(Contestant::GREEDY_V1_DEFAULT),
            &rules,
            TURN_SEED,
            &DrivenOpts::default(),
        )
        .expect("a terminal root is an Ok, not a refusal");

        assert!(outcome.moves.is_empty(), "a finished game has nothing to decide");
        assert_eq!(outcome.decisions, 0);
        assert!(outcome.game_over, "the stop was the game being over, and it must say so");
        assert_eq!(
            state_digest(&outcome.handoff),
            state_digest(&terminal),
            "the hand-off of a zero-move turn is the root itself",
        );
        assert_eq!(outcome.handoff.current_player, mover, "nothing moved, so nobody handed off");
        assert_eq!(
            outcome.counters.explained,
            Some(0),
            "an explaining seat asked for no decisions reads Some(0); None would say it cannot \
             explain at all",
        );
    }

    /// The GAME-OVER corner, and the executable form of why the stop is DUAL.
    ///
    /// `advance_phase`'s game-end arm (`apply_move.rs:259-264`) sets
    /// `TurnPhase::TurnEnd` and never writes `current_player`, so the turn that
    /// ends the game hands off to NOBODY: a `current_player`-only stop would
    /// spin here until the decision bound refused it. The fixture's decision is
    /// FORCED (`EndAdditionalPhase` is the only candidate), so the corner is a
    /// property of the loop rather than of what a policy chose.
    #[test]
    fn a_turn_that_ends_the_game_stops_on_game_over_with_the_mover_unchanged() {
        let rules = Rules::standard();
        let root = last_move_root(&rules, world(76).0, 6, 10);
        let mover = root.current_player;

        let outcome = drive_turn(&root, rung_mover(), &rules, TURN_SEED, &DrivenOpts::default())
            .expect("a final-move root drives one turn");

        assert_eq!(outcome.moves.len(), 1, "anti-vacuity: the loop applied nothing at all");
        assert_eq!(outcome.moves[0].mv, Move::EndAdditionalPhase);
        assert_eq!(outcome.moves[0].seat, mover);
        assert!(outcome.game_over, "the game ended and the outcome must say so");
        assert!(is_game_over(&outcome.handoff));
        assert_eq!(outcome.handoff.turn_phase, TurnPhase::TurnEnd);
        assert_eq!(
            outcome.handoff.current_player, mover,
            "the game-end arm never writes current_player, so the mover must be unchanged — a \
             hand-off here would mean the stop read the wrong field",
        );
    }

    /// The WHOLE-TURN counters, discriminated against a single decision's.
    ///
    /// The comparator is the SAME contestant at the SAME seed asked for the
    /// FIRST decision alone — so it reproduces the turn's decision 0 exactly
    /// (asserted, which is also the first-decision join at the greedy arm) and
    /// its counters are precisely what a per-decision snapshot would have
    /// reported. `explained` is the sharp half: whole-turn reads the decision
    /// count, and both a first-decision and a last-decision snapshot read 1.
    #[test]
    fn a_turns_counters_accumulate_across_every_decision_not_just_one() {
        let rules = Rules::standard();
        let root = turn_start_root(&rules, world(77), 300);
        let seat = root.current_player;
        let greedy = Contestant::GREEDY_V1_DEFAULT;

        let outcome = drive_turn(
            &root,
            SeatSpec::Contestant(greedy),
            &rules,
            TURN_SEED,
            &DrivenOpts::default(),
        )
        .expect("a greedy mover drives its turn");
        assert!(
            outcome.moves.len() >= 2,
            "anti-vacuity: a one-decision turn cannot tell whole-turn counters from one \
             decision's, and this turn ran {}",
            outcome.moves.len(),
        );
        assert_eq!(
            outcome.counters.explained,
            Some(outcome.decisions),
            "the mover applied moves it did not explain",
        );

        let (mut solo, handle) = greedy.build(&rules, TURN_SEED);
        let view = observe(&root, seat).expect("the root is observable");
        let picked = solo.choose(&view, &rules).expect("the comparator decides once").mv;
        let single = SeatCounters::from_handle(handle.as_ref().expect("greedy explains"));

        assert_eq!(
            picked, outcome.moves[0].mv,
            "the comparator did not reproduce the turn's own first decision, so it is not one",
        );
        assert_eq!(single.explained, Some(1), "the comparator was asked for exactly one decision");
        assert!(
            outcome.counters.candidates_scored.unwrap_or(0)
                > single.candidates_scored.unwrap_or(u64::MAX),
            "the turn scored {:?} candidates against decision 0's {:?} — the counters are one \
             decision's, not the turn's",
            outcome.counters.candidates_scored,
            single.candidates_scored,
        );
        assert_eq!(
            outcome.counters.ply_cap_hits, None,
            "a one-ply argmax has no continuation to truncate — None is not zero",
        );
    }

    /// The guards a turn is refused by are the TERMINAL DRIVER'S, shared rather
    /// than re-typed: same variants, same messages, same order.
    ///
    /// Five arms, each with the control that makes it a discriminator: the
    /// decision bound (against the same root driving cleanly), the hidden deal
    /// on the NON-MOVER (against the undoctored root reaching the bound — which
    /// is what makes "up front" executable, since a refusal that waited for the
    /// hidden seat's turn would never fire inside ONE turn at all), the seat
    /// count, the zero bound, and the corrupt root with its `invariants: false`
    /// arm (the root guard does not answer to the per-move flag).
    #[test]
    fn a_turns_guards_refuse_the_same_way_the_terminal_drivers_do() {
        let rules = Rules::standard();
        let root = turn_start_root(&rules, world(78), 300);
        let default = DrivenOpts::default();

        // (a) THE DECISION BOUND, on a turn known to be longer than it.
        let bounded = DrivenOpts { max_decisions: 1, ..default };
        let err = drive_turn(&root, rung_mover(), &rules, TURN_SEED, &bounded)
            .expect_err("a two-decision turn cannot finish inside a one-decision bound");
        match err {
            DrivenError::DecisionCapReached { decisions } => assert_eq!(
                decisions, 1,
                "the refusal must name the bound the caller set, not one past it",
            ),
            other => panic!("expected DecisionCapReached, got {other:?}"),
        }
        assert!(
            drive_turn(&root, rung_mover(), &rules, TURN_SEED, &default).is_ok(),
            "control: the same turn drives cleanly at the default bound",
        );

        // (b) THE HIDDEN DEAL, on the seat that never moves inside this turn —
        // which is what makes the UP-FRONT guard load-bearing rather than
        // redundant here: `observe` is only ever called for the MOVER inside a
        // turn, so a driver without this check would drive the live-tracked root
        // happily and report a turn nobody could have played. Doctored on a
        // fresh post-setup root, where the dealt card is still in hand (a
        // mid-game seat may have completed it, and conservation is not something
        // a fixture may break to make a point).
        let fresh = root_2p(&rules, world(79).0);
        let other_seat = PlayerId(1 - fresh.current_player.0);
        let mut hidden = fresh.clone();
        let index = other_seat.0 as usize;
        let kept = hidden.players[index].starting_order.take().expect("a dealt starting order");
        hidden.players[index].order_cards_in_hand.retain(|c| *c != kept);
        hidden.orders_removed.push(kept);
        hidden.players[index].starting_order_hidden = true;
        check_invariants(&hidden, &rules).expect("the live-tracking root is invariant-clean");
        assert!(
            drive_turn(&fresh, rung_mover(), &rules, TURN_SEED, &default).is_ok(),
            "control: undoctored, the same root drives a turn",
        );
        let err = drive_turn(&hidden, rung_mover(), &rules, TURN_SEED, &default)
            .expect_err("a live-tracked root must refuse");
        match err {
            DrivenError::Unobservable { seat, ref detail } => {
                assert_eq!(seat, other_seat, "the refusal named the wrong seat");
                assert!(
                    detail.contains("starting_order_hidden"),
                    "the refusal does not name the field it read: {detail}",
                );
            }
            ref other => panic!("expected Unobservable, got {other:?}"),
        }

        // (c) THE SEAT COUNT, on the wrapping shape.
        let mut many = root.clone();
        let filler = many.players[0].clone();
        many.players.resize(258, filler);
        match drive_turn(&many, rung_mover(), &rules, TURN_SEED, &default)
            .expect_err("a 258-seat root must refuse rather than narrow to 2")
        {
            DrivenError::UnsupportedRoot(m) => assert!(
                m.contains("258 seats"),
                "the refusal must name the count it SAW, not the one it wrapped to: {m}",
            ),
            other => panic!("expected UnsupportedRoot, got {other:?}"),
        }

        // (d) THE ZERO BOUND: a run that may apply no decision is not a run.
        let zero = DrivenOpts { max_decisions: 0, ..default };
        match drive_turn(&root, rung_mover(), &rules, TURN_SEED, &zero)
            .expect_err("a zero bound must refuse")
        {
            DrivenError::UnsupportedRoot(m) => {
                assert!(m.contains("max_decisions"), "the refusal does not name the knob: {m}");
            }
            other => panic!("expected UnsupportedRoot, got {other:?}"),
        }

        // (e) THE CORRUPT ROOT, named as the ROOT and unconditional in the
        // per-move flag.
        let mut corrupt = root;
        corrupt.players[0].assistants.in_hand += 1;
        assert!(check_invariants(&corrupt, &rules).is_err(), "the fixture must be corrupt");
        for opts in [default, DrivenOpts { invariants: false, ..default }] {
            match drive_turn(&corrupt, rung_mover(), &rules, TURN_SEED, &opts)
                .expect_err("a corrupt root must refuse")
            {
                DrivenError::CorruptRoot(reason) => assert!(
                    reason.contains("assistants total"),
                    "the reason is lost: {reason}",
                ),
                other => panic!("expected CorruptRoot, got {other:?}"),
            }
        }
    }

    // =======================================================================
    // PHASE C T3: the RACE-RUSHER reply
    // =======================================================================

    /// T3's committed diagnostic seed for the seats these cells build by hand —
    /// the same discipline [`TURN_SEED`] carries. A fallback-identity pin needs a
    /// FIXED stream on both sides, and a fixture cell is exactly the case
    /// `game::seeds`' rule carves out.
    const RUSH_SEED: u64 = 0x0C03_2026_0818_9992;

    /// One decision through the honest chain the drivers use — observe, choose,
    /// and the legality boundary — so a cell that asks an agent for a move gets
    /// the same treatment a driven run would give it.
    fn ask(agent: &mut dyn ArenaAgent, state: &GameState, rules: &Rules) -> Move {
        let seat = state.current_player;
        let view = observe(state, seat).expect("the fixture seat observes its own position");
        let mv = agent.choose(&view, rules).expect("the agent decides").mv;
        assert!(
            is_move_legal(state, &mv, rules, PathMode::Canonical),
            "the agent played a move the engine does not allow: {mv:?}",
        );
        mv
    }

    /// A bare greedy@final seat, built the way [`Contestant::build`] builds one.
    fn bare_greedy(rules: &Rules, agent_seed: u64) -> Box<dyn ArenaAgent> {
        Contestant::GREEDY_V1_DEFAULT.build(rules, agent_seed).0
    }

    /// Drive ONE mover's turn with a race-rusher, comparing every NO-RACE
    /// decision against `other` at the same stream position, and return how many
    /// decisions were compared.
    ///
    /// **The comparison is DECISION-SCOPED, and the break is why** (plan review
    /// SHOULD-2): a whole-turn identity claim would need "no candidate ever
    /// races", which no fixture can promise. What IS claimable is that every
    /// decision the race-rusher DELEGATED is the decision a bare greedy seat
    /// makes at that stream position — and the second such decision is the one
    /// that carries the real content, since it can only agree if the first
    /// decision's probes left the inner stream alone.
    fn no_race_prefix_agrees(
        rules: &Rules,
        root: &GameState,
        agent_seed: u64,
        other: &mut dyn ArenaAgent,
    ) -> usize {
        let mover = root.current_player;
        let mut rusher = RaceRusherAgent::new(rules, agent_seed);
        let mut state = root.clone();
        let mut compared = 0usize;
        while state.current_player == mover && !is_game_over(&state) {
            let before = rusher.race_picks;
            let mine = ask(&mut rusher, &state, rules);
            if rusher.race_picks != before {
                // A raced decision: the two agents are answering different
                // questions from here on, so the comparison stops rather than
                // reporting a divergence it invited.
                break;
            }
            let theirs = ask(other, &state, rules);
            assert_eq!(
                mine, theirs,
                "decision {compared} delegated, but did not agree with the bare greedy seat",
            );
            compared += 1;
            state = apply_move_unchecked(&state, &mine, rules).expect("the agreed move applies");
        }
        compared
    }

    /// The FIRST position of the organic 2P rung games at [`C_MASTER`] whose
    /// mover faces a decision in `phase`, with no choice queued.
    fn phase_root(rules: &Rules, phase: TurnPhase) -> GameState {
        harvest(rules, C_MASTER, 4, "a decision in the requested phase", &mut |s| {
            !is_game_over(s)
                && s.pending_choices.is_empty()
                && s.turn_phase == phase
                && get_legal_moves(s, rules).len() >= 2
        })
    }

    /// ONE world at which a CLAIM, a CHURCH step and a CUSTOMS step all classify
    /// — the constructed multi-class decision, plus the three moves.
    ///
    /// **It has to be CONSTRUCTED, and the reason is the phase machine itself.**
    /// `search_actions::families` emits exactly one PROBED family per decision
    /// point (`AdditionalAction`, `AreaAction`, or `ChoiceResolution` — the queue
    /// short-circuits everything else), a claim lives only in the first, and a
    /// track action resolves the ACTING AREA, which is one area. So no legal list
    /// the engine ever emits holds two classes at once, and a priority rule
    /// pinned only on organic lists would be pinned on nothing. What this builds
    /// instead is one STATE at which all three moves genuinely apply — the
    /// classifier's own input — by standing the mover's President on the Church
    /// with the goods for a church action and queueing a customs-space choice it
    /// can resolve.
    ///
    /// Harvest-then-doctor, this module's fixture discipline: the base position
    /// is engine-produced and offers the claim for real, `check_invariants` is
    /// asserted on the result, and each of the three moves is asserted to
    /// classify as the class it is named for before any priority claim rides it.
    fn multi_class_world(rules: &Rules) -> (GameState, Move, Move, Move) {
        let mut world = harvest(rules, C_MASTER, 4, "a position with a legal claim", &mut |s| {
            !is_game_over(s)
                && get_legal_moves(s, rules)
                    .iter()
                    .any(|mv| matches!(mv, Move::FulfillAchievement { .. }))
        });
        let claim = get_legal_moves(&world, rules)
            .into_iter()
            .find(|mv| matches!(mv, Move::FulfillAchievement { .. }))
            .expect("the harvest promised a claim");

        let mover = world.current_player;
        let [church_area, _customs_area] = track_areas(&world);

        // The CHURCH half: the mover acts at the Church, at a power and a stock
        // that reach an eligible space.
        world.player_mut(mover).president_location = PresidentLocation::OnBoard(church_area);
        while world.player(mover).assistants.in_area(church_area) < 1 {
            stage_assistant(world.player_mut(mover), church_area);
        }
        world.current_action_power = Some(4);
        {
            let p = world.player_mut(mover);
            p.copper = 1;
            p.silk = 1;
            p.tea = 1;
            p.seafood = 1;
            p.money = 10;
            p.imported_goods = 4;
        }
        let church = Move::ExecuteAreaAction {
            action: AreaAction::Church {
                copper: true,
                silk: false,
                tea: false,
                seafood: false,
                yen: false,
                imported_good: false,
            },
        };

        // The CUSTOMS half: a queued space choice the mover can resolve. The
        // cheapest printed cost with an unoccupied space is used, so the fixture
        // does not depend on which spaces the base game happened to fill.
        let cost = (1..=7u8)
            .find(|c| !eligible_customs_spaces_for_cost(&world.customs_track, *c).is_empty())
            .expect("a mid-game customs track has an unoccupied space");
        let space = eligible_customs_spaces_for_cost(&world.customs_track, cost)[0];
        world.pending_choices.push(PendingChoice::CustomsSpace { cost });
        let customs = Move::ChooseCustomsSpace { space };

        assert!(
            check_invariants(&world, rules).is_ok(),
            "the doctored multi-class world left the engine's own net",
        );
        (world, claim, church, customs)
    }

    // -----------------------------------------------------------------------
    // The vocabulary, the echo, and the probe world's derivation
    // -----------------------------------------------------------------------

    /// The fourth menu slot names NO contestant, says so in a TYPE, and echoes
    /// its rule together with the RESOLVED identity it falls back to.
    ///
    /// The echo is pinned three ways on purpose: verbatim against the composed
    /// sentence (so a re-pointed fallback moves the string), literally against
    /// its two fixed halves (so a composition change that still interpolates
    /// something is caught), and against `requested_label`, which must NOT move
    /// at D1 — the pair is what makes a re-pointing visible in an artifact.
    #[test]
    fn the_race_rusher_echo_names_the_rule_and_its_resolved_fallback() {
        assert_eq!(ReplyPolicy::RaceRusher.as_str(), "race-rusher");
        assert_eq!(
            ReplyPolicy::RaceRusher.contestant(),
            Err(NotAContestant::RaceRusher),
            "the adversarial arm must never resolve to a contestant",
        );
        assert_eq!(
            SeatSpec::Reply(ReplyPolicy::RaceRusher).contestant(),
            Err(NotAContestant::RaceRusher),
        );
        assert!(!NotAContestant::RaceRusher.as_str().is_empty());

        let echo = SeatSpec::Reply(ReplyPolicy::RaceRusher).echo();
        assert_eq!(
            echo,
            format!("reply:race-rusher-v1{{fallback={}}}", Contestant::GREEDY_V1_DEFAULT.echo()),
        );
        assert!(
            echo.starts_with("reply:race-rusher-v1{fallback=greedy-v1@v"),
            "the echo lost its rule name or its fallback: {echo}",
        );
        assert!(echo.ends_with("}}"), "the echo did not close the fallback: {echo}");
        assert_eq!(
            SeatSpec::Reply(ReplyPolicy::RaceRusher).requested_label(),
            "reply:race-rusher",
            "the REQUESTED label is the menu name and must not carry the resolution",
        );
    }

    /// The fallback is [`ReplyPolicy::GreedyAtFinal`]'s own binding, read from
    /// one place — so D1's identity freeze re-points both together or fails
    /// here.
    #[test]
    fn the_race_rusher_fallback_is_the_greedy_at_final_binding() {
        assert_eq!(RACE_RUSHER_FALLBACK, Contestant::GREEDY_V1_DEFAULT);
        assert_eq!(ReplyPolicy::GreedyAtFinal.contestant(), Ok(RACE_RUSHER_FALLBACK));
        assert_eq!(RACE_RUSHER_FALLBACK, Contestant::GreedyV1 { m: DEFAULT_RETREAT_SAMPLES });
        assert_eq!(RACE_RUSHER_CONTROLLER, "race-rusher-v1");
        assert_eq!(RACE_RUSHER_IMPL_VERSION, 1);
    }

    /// The probe world's seed is the DECLARED derivation, on its own serial, and
    /// it separates both seats and successive decisions.
    ///
    /// The salt-distinctness half is the cheap guard the cross-pointer asks for:
    /// this constant lives in `driven.rs` while its family's home is
    /// `arena.rs`, so nothing but a test stops a future serial from colliding.
    #[test]
    fn the_race_probe_seed_is_the_declared_derivation_on_its_own_serial() {
        assert_ne!(
            SALT_DRIVEN_RACE_PROBE, SALT_DRIVEN_REPLICATION,
            "two salts of the A3E1_D21E family collided",
        );
        let a = 0x1234_5678_9ABC_DEF0u64;
        let b = 0x0FED_CBA9_8765_4321u64;
        assert_eq!(race_probe_seed(a, 0), mix(SALT_DRIVEN_RACE_PROBE, &[a, 0]));
        assert_eq!(race_probe_seed(a, 7), mix(SALT_DRIVEN_RACE_PROBE, &[a, 7]));
        assert_ne!(race_probe_seed(a, 0), race_probe_seed(a, 1), "one seat's decisions share a world");
        assert_ne!(race_probe_seed(a, 0), race_probe_seed(b, 0), "two seats share a probe world");
        // And it is NOT the play stream itself: a probe world derived from the
        // bare agent seed would be a second consumer of the same value.
        assert_ne!(race_probe_seed(a, 0), a);
    }

    /// The probe SCOPE is the three small families, stated over every tag the
    /// factored surface has.
    ///
    /// Exhaustive by hand here rather than by `match`, because the claim is
    /// about the SET and a `match` in a test would only restate the one in the
    /// implementation.
    #[test]
    fn the_probe_scope_is_the_three_small_families() {
        let on_board = BTreeMap::new();
        for family in [
            MoveFamily::AdditionalAction,
            MoveFamily::AreaAction,
            MoveFamily::ChoiceResolution,
        ] {
            assert!(is_probed_family(&family), "{family:?} must be probed");
        }
        for family in [
            MoveFamily::Placement,
            MoveFamily::MovementRoutes,
            MoveFamily::MovementRetreat { on_board },
            MoveFamily::FivePower,
            MoveFamily::Building,
        ] {
            assert!(!is_probed_family(&family), "{family:?} must NOT be probed");
        }
    }

    // -----------------------------------------------------------------------
    // The decision rule
    // -----------------------------------------------------------------------

    /// Claim beats church beats customs beats the fallback, at a world where all
    /// three genuinely apply.
    ///
    /// The order the candidates are OFFERED in is deliberately the reverse of
    /// the priority, so a rule that returned the first classifiable candidate
    /// (rather than the best class) fails every one of these.
    #[test]
    fn the_race_rule_takes_claim_then_church_then_customs() {
        let rules = Rules::standard();
        let (world, claim, church, customs) = multi_class_world(&rules);

        // The classifier first: every priority claim below is only worth as much
        // as these three.
        assert_eq!(race_class(&world, &claim, &rules), Some(RaceClass::Claim));
        assert_eq!(race_class(&world, &church, &rules), Some(RaceClass::Church));
        assert_eq!(race_class(&world, &customs, &rules), Some(RaceClass::Customs));

        // The TIE-BREAK is exercised here rather than merely stated: because the
        // fixture already carries a queued customs choice, the church candidate
        // reaches steps on BOTH tracks, and the declared priority is what makes
        // its class Church.
        let both = mover_reachable_steps(&world, std::slice::from_ref(&church), &rules);
        assert!(
            both.iter().any(|s| matches!(s, RaceStep::Church(_)))
                && both.iter().any(|s| matches!(s, RaceStep::Customs(_))),
            "the tie-break is untested: the church candidate reaches only {both:?}",
        );
        assert_eq!(
            RaceClass::Claim.cmp(&RaceClass::Church),
            std::cmp::Ordering::Less,
            "the class type's own order is the priority order",
        );
        assert!(RaceClass::Church < RaceClass::Customs);

        // All three, worst first.
        assert_eq!(
            race_pick(&world, &rules, &[customs.clone(), church.clone(), claim.clone()]),
            Some((RaceClass::Claim, claim.clone())),
        );
        // Drop the claim: the church step wins.
        assert_eq!(
            race_pick(&world, &rules, &[customs.clone(), church.clone()]),
            Some((RaceClass::Church, church.clone())),
        );
        // Drop it too: the customs step wins.
        assert_eq!(
            race_pick(&world, &rules, std::slice::from_ref(&customs)),
            Some((RaceClass::Customs, customs.clone())),
        );

        // FIRST of the best class, in the order given — two claims, and the
        // earlier one is the answer.
        assert_eq!(
            race_pick(&world, &rules, &[claim.clone(), claim.clone()]),
            Some((RaceClass::Claim, claim.clone())),
        );

        // And nothing to race is a fallback, not a pick.
        let inert = Move::EndAdditionalPhase;
        assert_eq!(race_class(&world, &inert, &rules), None);
        assert_eq!(race_pick(&world, &rules, &[inert]), None);
    }

    /// **MUST-5, the load-bearing pin**: a track action with TWO OR MORE
    /// eligible spaces places nothing at apply time — it queues the choice — and
    /// the race-rusher must still classify it as race progress and TAKE it.
    ///
    /// An occupancy-only classifier passes every other cell in this file and
    /// fails exactly here, which is the point: the queueing candidates are the
    /// race-INITIATING moves this reply exists to model, and a classifier blind
    /// to them would model only the already-narrowed singleton case.
    #[test]
    fn a_queued_track_choice_classifies_as_race_progress_and_is_taken() {
        let rules = Rules::standard();
        // A decision whose own legal list holds a candidate that QUEUES a
        // track-space choice — the T1 harvest shape, re-taken here because this
        // cell needs the DECISION and not merely the state.
        let root = harvest(
            &rules,
            C_MASTER,
            4,
            "a decision offering a candidate that QUEUES a track-space choice",
            &mut |s| {
                if is_game_over(s) || !s.pending_choices.is_empty() {
                    return false;
                }
                get_legal_moves(s, &rules).iter().any(|mv| {
                    apply_move_unchecked(s, mv, &rules).is_ok_and(|next| {
                        next.current_player == s.current_player
                            && next.pending_choices.iter().any(|c| {
                                matches!(
                                    c,
                                    PendingChoice::ChurchSpace { .. }
                                        | PendingChoice::CustomsSpace { .. }
                                )
                            })
                    })
                })
            },
        );

        let legal = get_legal_moves(&root, &rules);
        let queueing = legal
            .iter()
            .find(|mv| {
                apply_move_unchecked(&root, mv, &rules).is_ok_and(|next| {
                    next.current_player == root.current_player
                        && next.pending_choices.iter().any(|c| {
                            matches!(
                                c,
                                PendingChoice::ChurchSpace { .. }
                                    | PendingChoice::CustomsSpace { .. }
                            )
                        })
                })
            })
            .expect("the harvest promised one")
            .clone();

        // The MUST-5 premise, asserted rather than assumed: this candidate places
        // NOTHING, so occupancy cannot be what classifies it.
        let after = apply_move_unchecked(&root, &queueing, &rules).expect("it applies");
        assert_eq!(
            (after.church_track.total_assistants(), after.customs_track.total_assistants()),
            (root.church_track.total_assistants(), root.customs_track.total_assistants()),
            "the fixture candidate placed a pawn, so nothing here is about a QUEUE",
        );
        assert!(
            matches!(race_class(&root, &queueing, &rules), Some(RaceClass::Church | RaceClass::Customs)),
            "a queueing track action classified as no race at all",
        );

        // And the AGENT takes it, through its own honest chain.
        let mut rusher = RaceRusherAgent::new(&rules, RUSH_SEED);
        let played = ask(&mut rusher, &root, &rules);
        assert_eq!(rusher.race_picks, 1, "the decision was delegated, not raced");
        assert!(
            matches!(race_class(&root, &played, &rules), Some(RaceClass::Church | RaceClass::Customs)),
            "the race-rusher raced with a move that races nothing: {played:?}",
        );
        assert!(rusher.probed_candidates > 0, "anti-vacuity: the decision probed nothing");
    }

    /// A movement decision probes NOTHING and delegates — with the
    /// anti-vacuity half that a probed-family decision does probe.
    ///
    /// Both halves are needed: a counter that never moves would satisfy the
    /// first assertion however broken the scope rule was.
    #[test]
    fn a_movement_decision_probes_nothing_and_delegates() {
        let rules = Rules::standard();
        let movement = phase_root(&rules, TurnPhase::Movement);
        assert!(
            crate::game::search_actions::families(&movement, &rules)
                .iter()
                .all(|f| !is_probed_family(f)),
            "the movement fixture offers a probed family, so this cell tests nothing",
        );

        let mut rusher = RaceRusherAgent::new(&rules, RUSH_SEED);
        let mut greedy = bare_greedy(&rules, RUSH_SEED);
        let raced = ask(&mut rusher, &movement, &rules);
        let plain = ask(greedy.as_mut(), &movement, &rules);
        assert_eq!(rusher.probed_candidates, 0, "a movement decision probed a candidate");
        assert_eq!(rusher.race_picks, 0);
        assert_eq!(raced, plain, "a probe-free decision did not delegate");

        // ANTI-VACUITY: the counter does move where the scope says it should.
        let area = phase_root(&rules, TurnPhase::AreaAction);
        let mut probing = RaceRusherAgent::new(&rules, RUSH_SEED);
        let _ = ask(&mut probing, &area, &rules);
        assert!(
            probing.probed_candidates > 0,
            "an area-action decision probed nothing, so the zero above says nothing",
        );
    }

    /// The no-race fallback is byte-identical to a bare greedy seat at the same
    /// seed — for at least TWO decisions, which is what proves the probes left
    /// the inner stream alone.
    ///
    /// One decision would only show that the first delegation agrees. The second
    /// is the content: the race-rusher's probes ran between them, and a probe
    /// that had drawn from the greedy's own `ChaCha8Rng` would move it past the
    /// value the bare seat is about to use.
    #[test]
    fn the_no_race_fallback_is_byte_identical_to_a_bare_greedy_seat() {
        let rules = Rules::standard();
        let root = turn_start_root(&rules, world(80), 200);
        // The MIRROR itself on the other side — the construction `new` performs,
        // written out, so this cell is about the STREAM rather than about the
        // constructor.
        let mut mirrored =
            GreedyAgent::with_weights(&rules, WeightTable::V2, DEFAULT_RETREAT_SAMPLES, RUSH_SEED);
        let compared = no_race_prefix_agrees(&rules, &root, RUSH_SEED, &mut mirrored);
        assert!(
            compared >= 2,
            "anti-vacuity: only {compared} delegated decision(s) were compared, and the stream \
             claim needs two",
        );
    }

    /// The inner greedy IS [`Contestant::build`]'s greedy arm — the drift pin
    /// the mirror owes.
    ///
    /// Compared by MOVE IDENTITY over a whole turn at the same seed rather than
    /// by inspecting fields: a mirror that agreed on every field and differed in
    /// construction ORDER would still diverge here, and a mirror that differed
    /// in a field nobody thought to compare would not be caught by a field
    /// check.
    #[test]
    fn the_inner_greedy_is_contestant_builds_greedy_arm() {
        let rules = Rules::standard();
        let root = turn_start_root(&rules, world(81), 240);
        let mut built = bare_greedy(&rules, RUSH_SEED);
        let compared = no_race_prefix_agrees(&rules, &root, RUSH_SEED, built.as_mut());
        assert!(
            compared >= 1,
            "anti-vacuity: no delegated decision was compared against `Contestant::build`",
        );
    }

    /// The LIVE ordinal reaches the derivation: one seat's successive decisions
    /// probe at ordinals 0 and 1.
    ///
    /// [`the_race_probe_seed_is_the_declared_derivation_on_its_own_serial`] pins
    /// the FUNCTION, and nothing in it says the agent hands that function the
    /// ordinal it is holding. Freeze the argument at 0 — one seat, ONE probe
    /// world for its whole life, every decision classified against the same
    /// determinization — and every other cell in this module stays green: the
    /// scope counters, the fallback identity and the determinism cell are all
    /// blind to WHICH world was probed. So the seed is recorded where the
    /// sampler is built and read back here.
    #[test]
    fn successive_decisions_probe_at_successive_ordinals() {
        let rules = Rules::standard();
        let root = turn_start_root(&rules, world(83), 200);
        let mut rusher = RaceRusherAgent::new(&rules, RUSH_SEED);

        let first = ask(&mut rusher, &root, &rules);
        assert_eq!(
            rusher.last_probe_seed,
            Some(race_probe_seed(RUSH_SEED, 0)),
            "the first decision did not probe at ordinal 0",
        );

        // The two expectations are DIFFERENT values, stated about the pair this
        // cell actually compares — otherwise a frozen argument would satisfy
        // both and the second assertion would carry nothing.
        assert_ne!(race_probe_seed(RUSH_SEED, 0), race_probe_seed(RUSH_SEED, 1));

        let next = apply_move_unchecked(&root, &first, &rules).expect("the first move applies");
        assert!(
            !is_game_over(&next),
            "the fixture ended in one move, so there is no second decision to probe",
        );
        let _ = ask(&mut rusher, &next, &rules);
        assert_eq!(
            rusher.last_probe_seed,
            Some(race_probe_seed(RUSH_SEED, 1)),
            "the second decision did not probe at ordinal 1 — the live ordinal is not reaching \
             the derivation",
        );
    }

    /// Same seed, same root → the same decisions twice.
    ///
    /// **The anti-vacuity half is the LENGTH assertion**, not a second seed:
    /// this cell never runs a different seed, and a turn of one decision (or of
    /// none) would satisfy the equality however broken the seeding was.
    /// Asserting the fixture turn is at least two decisions long is what makes
    /// the equality say something, and [`turn_start_root`]'s phase guarantee is
    /// what supplies it.
    #[test]
    fn a_race_rusher_seat_is_deterministic_at_a_fixed_seed() {
        let rules = Rules::standard();
        let root = turn_start_root(&rules, world(82), 260);
        let turn = |seed: u64| -> Vec<Move> {
            let mut agent = RaceRusherAgent::new(&rules, seed);
            let mover = root.current_player;
            let mut state = root.clone();
            let mut moves = Vec::new();
            while state.current_player == mover && !is_game_over(&state) {
                let mv = ask(&mut agent, &state, &rules);
                state = apply_move_unchecked(&state, &mv, &rules).expect("the move applies");
                moves.push(mv);
            }
            moves
        };
        let first = turn(RUSH_SEED);
        assert!(first.len() >= 2, "anti-vacuity: the fixture turn is {} decision(s)", first.len());
        assert_eq!(first, turn(RUSH_SEED), "the same seed produced two different turns");
    }

    // -----------------------------------------------------------------------
    // Through the drivers
    // -----------------------------------------------------------------------

    /// The seat materializes through BOTH drivers, and the whole-game one
    /// records what it seated: the menu name asked for, the composed echo, the
    /// agent's own controller descriptor, and the view label the bench
    /// witnessed.
    ///
    /// Driven from a LATE root with a rung opponent, so the greedy fallback pays
    /// for a tail rather than a whole game (design §6's wall discipline).
    #[test]
    fn a_race_rusher_seat_is_described_by_the_driver_that_seated_it() {
        let rules = Rules::standard();
        let seed = world(83);
        let stats = rung_game(&rules, seed);
        let events = stats.record.events.len();
        let root = state_at(&stats.record, events.saturating_sub(24), &rules)
            .expect("a late root replays");
        assert!(!is_game_over(&root), "anti-vacuity: the chosen root is already terminal");

        // The TURN driver accepts the seat, which is the cheaper half.
        let turn = drive_turn(
            &root,
            SeatSpec::Reply(ReplyPolicy::RaceRusher),
            &rules,
            RUSH_SEED,
            &DrivenOpts::default(),
        )
        .expect("a race-rusher turn drives");
        assert!(!turn.moves.is_empty(), "anti-vacuity: the turn played nothing");
        assert_eq!(
            turn.counters,
            SeatCounters::NONE,
            "a race-rusher explains only what it delegates, so it must carry no account",
        );

        // And the WHOLE-GAME driver records the seat.
        let seats =
            [SeatSpec::Reply(ReplyPolicy::RaceRusher), SeatSpec::Reply(ReplyPolicy::FamilyWeighted)];
        let master = world(84);
        let outcome = drive_to_terminal(&root, &seats, &rules, master, DrivenOpts::default())
            .expect("a rung-vs-race-rusher run completes");
        assert_seats_describe(&seats, &outcome.provenance, master);

        let d = &outcome.provenance.seats[0];
        assert_eq!(d.requested, "reply:race-rusher");
        assert_eq!(d.echo, SeatSpec::Reply(ReplyPolicy::RaceRusher).echo());
        assert_eq!(d.controller, RACE_RUSHER_CONTROLLER);
        assert_eq!(d.version, RACE_RUSHER_IMPL_VERSION);
        assert_eq!(
            d.params["fallback_version"],
            i64::from(crate::game::greedy::GREEDY_IMPL_VERSION),
            "the descriptor does not name the fallback's version",
        );
        assert_eq!(d.params["m"], i64::from(DEFAULT_RETREAT_SAMPLES));
        assert!(d.params.contains_key("weights"), "the fallback's table identity is not recorded");
        assert_eq!(d.observation, crate::game::record::OBSERVATION_VIEW);
        let reconstructed = Contestant::from_parts(&d.controller, &d.params);
        assert!(
            reconstructed.is_err(),
            "a harness rule reconstructed as the contestant {reconstructed:?}",
        );
        assert_eq!(
            outcome.seats[0].counters,
            SeatCounters::NONE,
            "the race-rusher seat reported an explanation account it does not have",
        );
        assert!(outcome.seats[0].applied_moves > 0, "anti-vacuity: the seat never moved");
    }

    // =======================================================================
    // PHASE C T4: the adjudicator's TURN HOOK
    // =======================================================================

    /// Walk the mover's own turn out of a playout BY HAND, through the CHECKED
    /// `apply_move` — a second, independent application of the moves the driver
    /// reported, so a derivation that agreed with itself and with nothing else
    /// fails here.
    ///
    /// The walk is the design's rule read literally: start at ROOT + the forced
    /// candidate; if that already ended the turn, stop; otherwise apply the
    /// playout's moves while they belong to the mover, stopping the moment the
    /// game ends or the seat changes.
    fn hand_walk_turn(
        after: &GameState,
        mover: PlayerId,
        applied: &[AppliedMove],
        rules: &Rules,
    ) -> GameState {
        let mut state = after.clone();
        if is_game_over(&state) || state.current_player != mover {
            return state;
        }
        for (i, step) in applied.iter().enumerate() {
            if step.seat != mover {
                break;
            }
            state = apply_move(&state, &step.mv, rules)
                .unwrap_or_else(|e| panic!("hand-walk move {i} does not re-apply: {e}"));
            if is_game_over(&state) || state.current_player != mover {
                break;
            }
        }
        state
    }

    /// A LATE root offering the mover between `min` and `max` legal moves.
    ///
    /// Both bounds matter here and only here: the LOWER one makes the cell's
    /// per-row assertions non-trivial, and the UPPER one is the WALL — this cell
    /// pays `candidates × 2` whole rung games (the adjudication's playouts plus
    /// the independent re-drive the hand walk needs), so an unbounded scan that
    /// happened to land on a money-saturated Movement point would cost a minute
    /// for no more evidence. Late in the game for the same reason: a playout
    /// from event ~800 is a tail rather than most of a game.
    fn narrow_late_root(
        rules: &Rules,
        seed: WorldSeed,
        from: usize,
        min: usize,
        max: usize,
    ) -> GameState {
        let stats = rung_game(rules, seed);
        let events = stats.record.events.len();
        assert!(from < events, "the generating game is only {events} events long");
        (from..events)
            .find_map(|k| {
                let state = state_at(&stats.record, k, rules).ok()?;
                if is_game_over(&state) {
                    return None;
                }
                let width = get_legal_moves(&state, rules).len();
                (min..=max).contains(&width).then_some(state)
            })
            .unwrap_or_else(|| panic!("no {min}..={max}-wide decision from event {from}"))
    }

    /// Run an adjudication with a RECORDING hook and return what the hook saw,
    /// in call order: `(first-argument digest, second-argument digest)` per
    /// playout.
    fn recorded_hook_args(
        root: &GameState,
        seats: &[SeatSpec],
        rules: &Rules,
        master: WorldSeed,
        replications: u32,
    ) -> (Adjudication, Vec<(String, String)>) {
        let seen: RefCell<Vec<(String, String)>> = RefCell::new(Vec::new());
        let adj = adjudicate_root_with(
            &RootSource::Constructed(root.clone()),
            seats,
            rules,
            master,
            replications,
            DrivenOpts::default(),
            Some(&|r: &GameState, h: &GameState| {
                seen.borrow_mut().push((state_digest(r), state_digest(h)));
                true
            }),
        )
        .expect("the fixture adjudicates");
        (adj, seen.into_inner())
    }

    /// **THE HOOK'S TWO ARGUMENTS**: the ADJUDICATED ROOT (pre-candidate) and
    /// the HAND-WALKED hand-off.
    ///
    /// The first half is plan MUST-3b's rule, asserted rather than commented: a
    /// hook handed the post-candidate state would invert family 4's dominance
    /// partition, and the anti-vacuity is that the two digests genuinely differ
    /// at every row here (`continuation_digest` is what the wrong argument would
    /// have been).
    ///
    /// The second half is the derivation itself, checked against an independent
    /// walk through the CHECKED `apply_move` over the moves a re-driven playout
    /// applied. The root is a BRANCHING mid-game position, so the hand-offs are
    /// real hand-offs — a near-terminal fixture would exercise only the
    /// game-over branch, which the next cell covers on purpose.
    #[test]
    fn the_turn_hook_reads_the_pre_candidate_root_and_the_hand_walked_hand_off() {
        let rules = Rules::standard();
        let root = narrow_late_root(&rules, world(90), 700, 4, 6);
        let mover = root.current_player;
        let master = world(91);
        let seats = rung_seats();

        let (adj, seen) = recorded_hook_args(&root, &seats, &rules, master, 1);
        assert!(adj.candidates() >= 4, "anti-vacuity: the root offers {} candidates", adj.candidates());
        assert_eq!(seen.len(), adj.candidates(), "one hook call per playout, in row order");

        let root_digest = state_digest(&root);
        for (index, (first, handoff)) in seen.iter().enumerate() {
            let row = &adj.rows[index];
            assert_eq!(
                *first, root_digest,
                "row {index}'s hook saw a first argument that is not the adjudicated root",
            );
            assert_ne!(
                *first, row.continuation_digest,
                "the fixture does not separate the two readings at row {index}",
            );

            // The independent walk: re-drive the SAME continuation in the SAME
            // replication world and follow the mover's own moves by hand.
            let after = apply_move(&root, &row.candidate, &rules)
                .expect("a listed candidate applies through the checked boundary");
            let outcome = drive_to_terminal(
                &after,
                &seats,
                &rules,
                replication_world(master, 0),
                DrivenOpts::default(),
            )
            .expect("the remainder drives");
            let walked = hand_walk_turn(&after, mover, &outcome.applied, &rules);
            assert_eq!(
                *handoff,
                state_digest(&walked),
                "row {index}: the hook's hand-off is not where the mover's own moves lead",
            );
        }

        // The hand-offs are HAND-OFFS, not a whole game and not the candidate
        // alone: at least one row's turn ran past its forced candidate and
        // stopped before the terminal state the playout reached.
        let moved_on = adj.rows.iter().enumerate().any(|(index, row)| {
            seen[index].1 != row.continuation_digest && seen[index].1 != row.playouts[0].final_digest
        });
        assert!(moved_on, "no row's hand-off sits strictly between the candidate and the end");
    }

    /// The corner where the forced candidate IS the whole turn, in both of its
    /// shapes — and the one where the walk runs on past it.
    ///
    /// `last_move_root`'s only candidate ENDS THE GAME, so the post-candidate
    /// state is already terminal and the mover has no remainder at all: the
    /// hand-off must be that state exactly (the continuation digest), never the
    /// playout's own final digest reached by somebody else's moves.
    /// `near_terminal_building_root` is the other half — the mover keeps playing
    /// after its forced build and the turn ends the game, so the hand-off is the
    /// TERMINAL state and the walk covered at least one remainder move.
    #[test]
    fn a_forced_candidate_that_ends_the_turn_is_the_whole_turn_the_hook_reads() {
        let rules = Rules::standard();
        let master = world(92);
        let seats = rung_seats();

        // (a) the candidate itself ends the game.
        let root = last_move_root(&rules, 7, 6, 10);
        let (adj, seen) = recorded_hook_args(&root, &seats, &rules, master, 1);
        assert_eq!(adj.candidates(), 1, "the fixture's premise: one forced candidate");
        assert_eq!(adj.rows[0].candidate, Move::EndAdditionalPhase);
        assert_eq!(seen[0].0, state_digest(&root), "the first argument is still the root");
        assert_eq!(
            seen[0].1, adj.rows[0].continuation_digest,
            "a candidate that ends the game leaves a remainder of exactly nothing",
        );
        assert_eq!(adj.rows[0].playouts[0].decisions, 0, "the fixture's premise: no remainder");

        // (b) the candidate is followed by more of the mover's own turn, and the
        // turn ends the game.
        let (root, mover, _) = near_terminal_building_root(&rules, 8, [3, 4, 5, 0], 6, 10);
        let (adj, seen) = recorded_hook_args(&root, &seats, &rules, master, 1);
        assert_eq!(adj.candidates(), expected_building_candidates().len());
        for (index, row) in adj.rows.iter().enumerate() {
            let after = apply_move(&root, &row.candidate, &rules).expect("a listed candidate applies");
            let outcome = drive_to_terminal(
                &after,
                &seats,
                &rules,
                replication_world(master, 0),
                DrivenOpts::default(),
            )
            .expect("the remainder drives");
            assert!(
                outcome.applied.iter().any(|m| m.seat == mover),
                "row {index}'s premise: the mover plays on after its forced build",
            );
            let walked = hand_walk_turn(&after, mover, &outcome.applied, &rules);
            assert!(is_game_over(&walked), "row {index}: this fixture's turn ends the game");
            assert_eq!(seen[index].1, state_digest(&walked), "row {index}'s hand-off");
            assert_ne!(
                seen[index].1, row.continuation_digest,
                "row {index}: the walk did not run past the forced candidate",
            );
        }
    }

    // -----------------------------------------------------------------------
    // V4 step 2: the §6 ESCROW RECIPE
    // -----------------------------------------------------------------------

    /// A hand-built classified candidate, for the SELECTION rule's own cells.
    ///
    /// The selection rule is a function of classified candidates and of nothing
    /// else, so its arms are reachable from data rather than only from a
    /// position that happens to produce them — which matters because two of
    /// them (no productive side, no shared total) are hard to reach organically
    /// on a printed board, and an arm no cell exercises is an arm that can be
    /// deleted with the suite still green.
    fn candidate(total: usize, areas: &[u8], side: RecipeSide) -> RecipeCandidate {
        let areas: BTreeSet<AreaId> = areas.iter().map(|&a| AreaId(a)).collect();
        RecipeCandidate {
            mv: Move::PlaceThreeAreas { placements: areas.iter().copied().collect() },
            total,
            areas,
            side,
        }
    }

    /// The area of `area_type` on a 2P board, which carries exactly one of each.
    fn area_of(state: &GameState, area_type: AreaType) -> AreaId {
        state
            .board_layout
            .areas
            .iter()
            .find(|(_, config)| config.area_type == area_type)
            .map(|(&id, _)| id)
            .unwrap_or_else(|| panic!("the board carries no {area_type:?}"))
    }

    /// Spec §6's recipe, at the cheapest turn start there is: among the fork's
    /// same-count candidates, one placing where the exported action reward at
    /// realizable power is NONZERO against one where it is ZERO.
    ///
    /// The arithmetic is the dossier's own family-1(a) row: one assistant at a
    /// Copper Mine arrives at power 2 and exports nothing, while one assistant
    /// at any other production area at that same power exports at least one
    /// good. So the pair is rules-derived before any column is read, which is
    /// what §6 requires of it.
    #[test]
    fn the_escrow_recipe_pairs_a_dead_production_placement_against_a_productive_one() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(93).0);
        let pair =
            escrow_recipe_pair(&root, &rules).expect("a fresh 2P turn start carries the recipe");

        assert_eq!(pair.mover, root.current_player);
        assert_eq!(pair.total, 1, "one assistant is the smallest total carrying both sides");
        let area_type = |a: &AreaId| root.board_layout.areas[a].area_type;

        assert_eq!(pair.dead_areas.len(), 1, "the dead side places into one area");
        let dead = *pair.dead_areas.iter().next().expect("one area");
        assert_eq!(area_type(&dead), AreaType::CopperMine);
        assert_eq!(exported_reward(AreaType::CopperMine, 2), Some(0));

        assert_eq!(pair.productive_areas.len(), 1, "the productive side places into one area");
        let live = *pair.productive_areas.iter().next().expect("one area");
        assert!(area_type(&live).is_production(), "{:?} is not a production area", area_type(&live));
        assert!(pair.productive_reward > 0);
        assert_eq!(
            exported_reward(area_type(&live), 2),
            Some(u8::try_from(pair.productive_reward).expect("a printed reward fits a u8")),
            "the recorded quantity is the engine's own export at the arriving power"
        );
    }

    /// The reward is read AFTER the placement lands, which is the only reading
    /// under which the recipe means anything: the placement is what creates the
    /// access being priced.
    ///
    /// The cell is decisive because ONE area sits on both sides of it. At a
    /// fresh 2P start the Copper Mine is DEAD as a one-assistant placement
    /// (arriving power 2, exports 0) and PRODUCTIVE as a two-assistant one
    /// (arriving power 3, exports 1). A classifier reading the pre-placement
    /// position would call both of them dead.
    #[test]
    fn the_recipe_reads_the_exported_reward_after_the_placement_lands() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(94).0);
        let copper = area_of(&root, AreaType::CopperMine);
        let candidates = classify_recipe_candidates(&root, &rules).expect("a turn start classifies");

        let side_of = |mv: &Move| {
            candidates
                .iter()
                .find(|c| &c.mv == mv)
                .unwrap_or_else(|| panic!("the fork does not offer {mv:?}"))
                .side
        };
        assert_eq!(
            side_of(&Move::PlaceThreeAreas { placements: vec![copper] }),
            RecipeSide::Dead,
            "one assistant at the Copper Mine arrives at power 2 and exports nothing"
        );
        assert_eq!(
            side_of(&Move::PlaceTwoSameArea { area: copper, adjacent_bonus: None }),
            RecipeSide::Productive(1),
            "two assistants at the Copper Mine arrive at power 3 and export one copper"
        );
    }

    /// **The empty placement is INELIGIBLE, and it has to be said out loud.**
    /// `PlaceThreeAreas { placements: vec![] }` is always in the list — it is
    /// how a mover declines to place — and its area set is EMPTY, so "every
    /// placed area exports nonzero" and "every placed area exports zero" are
    /// BOTH vacuously true of it. A classifier that let the vacuous truth
    /// decide would pair a placement against a non-placement and call the
    /// difference a rules-derived value distinction.
    #[test]
    fn the_empty_placement_is_never_a_side_of_the_recipe() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(95).0);
        let candidates = classify_recipe_candidates(&root, &rules).expect("a turn start classifies");
        let empty: Vec<&RecipeCandidate> = candidates.iter().filter(|c| c.total == 0).collect();
        assert_eq!(empty.len(), 1, "the decline-to-place move is offered exactly once");
        assert_eq!(empty[0].side, RecipeSide::Ineligible);
        assert!(empty[0].areas.is_empty());
    }

    /// Off a turn start there is no fork to key on, and the refusal is TYPED
    /// rather than an empty result that a caller could read as "no pair here".
    #[test]
    fn the_recipe_refuses_a_position_that_is_not_a_turn_start() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(96).0);
        let placing = apply_move_unchecked(&root, &Move::EndAdditionalPhase, &rules)
            .expect("ending the additional phase is always legal at a turn start");
        assert_eq!(escrow_recipe_pair(&placing, &rules), Err(RecipeMiss::NotATurnStart));
        assert_eq!(
            classify_recipe_candidates(&placing, &rules),
            Err(RecipeMiss::NotATurnStart)
        );
    }

    /// Over ORGANIC turn starts: a returned pair always WITNESSES the frozen
    /// §6 predicate (two distinct area sets at one total is what both objects
    /// mean), both of its sides place only into production areas, and the
    /// answer is a function of the root alone.
    ///
    /// The predicate is read here and never touched — this cell is what says
    /// the recipe and the generator select compatible things.
    #[test]
    fn every_organic_recipe_pair_witnesses_the_frozen_fork_predicate() {
        let rules = Rules::standard();
        let mut pairs = 0usize;
        let mut misses: BTreeMap<String, usize> = BTreeMap::new();
        for index in [11u64, 12] {
            let stats = rung_game(&rules, world(index));
            let events = stats.record.events.len();
            for k in (0..events).step_by(5) {
                let Ok(state) = state_at(&stats.record, k, &rules) else { continue };
                match escrow_recipe_pair(&state, &rules) {
                    Ok(pair) => {
                        pairs += 1;
                        assert!(
                            RootPredicate::PlacementForkSameCount.holds(&state, &rules),
                            "event {k}: a recipe pair at a position the fork predicate rejects"
                        );
                        assert_ne!(pair.productive_areas, pair.dead_areas);
                        for area in pair.productive_areas.iter().chain(pair.dead_areas.iter()) {
                            assert!(
                                state.board_layout.areas[area].area_type.is_production(),
                                "event {k}: the recipe admitted a non-production area"
                            );
                        }
                        assert_eq!(
                            Ok(pair),
                            escrow_recipe_pair(&state, &rules),
                            "event {k}: the recipe is not a function of the root"
                        );
                    }
                    Err(miss) => *misses.entry(miss.to_string()).or_default() += 1,
                }
            }
        }
        assert!(pairs > 0, "the sweep found no recipe pair at all ({misses:?})");
        // RECORDED, never asserted: the yield at a turn start is what says
        // whether §6's "≥ 6 of 8 ordinals" block rule is comfortable or tight,
        // and it is a property of the ecology rather than of this helper.
        let off_turn_start = misses.get(&RecipeMiss::NotATurnStart.to_string()).copied().unwrap_or(0);
        println!(
            "escrow recipe over organic play: {pairs} pairs, {off_turn_start} positions off a \
             turn start, other misses {:?}",
            misses
                .iter()
                .filter(|(name, _)| *name != &RecipeMiss::NotATurnStart.to_string())
                .collect::<Vec<_>>()
        );
    }

    /// The selection rule's five arms, each from data.
    ///
    /// The scan is ASCENDING in placement total and stops at the first total
    /// carrying BOTH sides — so a lower total with only one side is skipped
    /// rather than refused, which the third case is there to show.
    #[test]
    fn the_selection_rule_refuses_by_name_and_scans_totals_upward() {
        let mover = PlayerId(0);
        assert_eq!(select_recipe_pair(mover, &[]), Err(RecipeMiss::NoPlacementCandidates));
        assert_eq!(
            select_recipe_pair(mover, &[candidate(1, &[3], RecipeSide::Ineligible)]),
            Err(RecipeMiss::NoEligibleCandidate)
        );
        assert_eq!(
            select_recipe_pair(
                mover,
                &[candidate(1, &[3], RecipeSide::Dead), candidate(1, &[4], RecipeSide::Ineligible)]
            ),
            Err(RecipeMiss::NoProductiveSide)
        );
        assert_eq!(
            select_recipe_pair(mover, &[candidate(1, &[3], RecipeSide::Productive(2))]),
            Err(RecipeMiss::NoDeadSide)
        );
        assert_eq!(
            select_recipe_pair(
                mover,
                &[candidate(1, &[3], RecipeSide::Productive(2)), candidate(2, &[4], RecipeSide::Dead)]
            ),
            Err(RecipeMiss::NoSharedTotal)
        );
        // A lower total carrying only one side is SKIPPED, not refused.
        let pair = select_recipe_pair(
            mover,
            &[
                candidate(1, &[3], RecipeSide::Productive(2)),
                candidate(2, &[4], RecipeSide::Dead),
                candidate(2, &[5], RecipeSide::Productive(7)),
            ],
        )
        .expect("total 2 carries both sides");
        assert_eq!(pair.total, 2);
        assert_eq!(pair.productive_reward, 7);
        assert_eq!(pair.dead_areas, [AreaId(4)].into_iter().collect::<BTreeSet<_>>());
    }

    /// Within the chosen total the sides are picked by the LOWEST area set, a
    /// tie-break with no value content in it: the alternative — the largest
    /// reward, say — would be the recipe quietly optimizing what it is supposed
    /// to be merely selecting.
    #[test]
    fn the_selection_tie_break_is_the_lowest_area_set() {
        let pair = select_recipe_pair(
            PlayerId(1),
            &[
                candidate(1, &[9], RecipeSide::Productive(4)),
                candidate(1, &[2], RecipeSide::Productive(1)),
                candidate(1, &[8], RecipeSide::Dead),
                candidate(1, &[5], RecipeSide::Dead),
            ],
        )
        .expect("both sides sit at total 1");
        assert_eq!(pair.mover, PlayerId(1));
        assert_eq!(pair.productive_areas, [AreaId(2)].into_iter().collect::<BTreeSet<_>>());
        assert_eq!(pair.productive_reward, 1);
        assert_eq!(pair.dead_areas, [AreaId(5)].into_iter().collect::<BTreeSet<_>>());
    }

    // -----------------------------------------------------------------------
    // V4 step 10: the SHAPE-F FEE-FLOW RECIPE
    // -----------------------------------------------------------------------

    /// A hand-built priced candidate, for the SELECTION rule's own cells —
    /// [`candidate`]'s counterpart, and for its reason: two of the lattice's
    /// arms (no free side, no shared total) are hard to reach on a printed
    /// board, and an arm no cell exercises is an arm that can be deleted with
    /// the suite still green.
    fn fee_candidate(total: usize, areas: &[u8], fee: u32) -> FeeFlowCandidate {
        let areas: BTreeSet<AreaId> = areas.iter().map(|&a| AreaId(a)).collect();
        FeeFlowCandidate {
            mv: Move::PlaceThreeAreas { placements: areas.iter().copied().collect() },
            total,
            areas,
            fee,
        }
    }

    /// A fresh 2P turn start with the OPPONENT's President standing on a
    /// placeable production area — the Shape-F instantiation condition,
    /// CONSTRUCTED.
    ///
    /// It has to be constructed: `new_game` starts both Presidents in hand and
    /// `place_setup_dummies` puts none on the board (it touches the Church
    /// faith-7 and Customs space-4 slots and nothing else), so the fee flow at
    /// a fresh root is identically zero. The organic walk that reaches the
    /// condition — a Movement Option A onto a Possible Action Area, which then
    /// PERSISTS, because Recovery never recalls a President — is the sweep
    /// below's subject. `check_invariants` is what says this shortcut produces
    /// a legal state rather than a convenient one.
    fn fee_flow_root(rules: &Rules, seed: u64) -> (GameState, PlayerId, PlayerId, AreaId) {
        let mut root = root_2p(rules, seed);
        let mover = root.current_player;
        let opponent =
            root.players.iter().map(|p| p.id).find(|id| *id != mover).expect("2P seats two");
        let occupied = area_of(&root, AreaType::CopperMine);
        root.player_mut(opponent).president_location = PresidentLocation::OnBoard(occupied);
        check_invariants(&root, rules).expect("the constructed root is a legal state");
        (root, mover, opponent, occupied)
    }

    /// Move one Telegram card from the tech deck (or, failing that, a lab slot)
    /// into `player`'s technologies — CONSERVING the 30-card total the
    /// invariant checks, which is why the card is taken from somewhere rather
    /// than minted.
    fn grant_telegram(state: &mut GameState, rules: &Rules, player: PlayerId) {
        let is_telegram = |id: crate::types::TechCardId| {
            rules.techs.get(id).is_some_and(|t| t.effect == crate::types::TechEffect::Telegram)
        };
        let card = if let Some(pos) = state.tech_deck.iter().position(|&id| is_telegram(id)) {
            state.tech_deck.remove(pos)
        } else {
            let mut found = None;
            for slot in state
                .lab_state
                .lab_a
                .slots
                .iter_mut()
                .chain(state.lab_state.lab_b.iter_mut().flat_map(|b| b.slots.iter_mut()))
            {
                if slot.is_some_and(is_telegram) {
                    found = slot.take();
                    break;
                }
            }
            found.expect("the standard 30-card deck carries two Telegram cards")
        };
        state.player_mut(player).technologies.push(card);
    }

    /// Two adjacent placeable areas whose edge is NOT a Canal crossing — the
    /// generator's own three conditions for offering a Telegram bonus leg
    /// (`generate_placement_moves_with_tech`), restated as a fixture search so
    /// the fixture cannot ask for a move the engine would not offer.
    fn adjacent_placeable_pair(state: &GameState) -> (AreaId, AreaId) {
        let placeable = |a: &AreaId| {
            state.board_layout.areas.get(a).is_some_and(|c| c.area_type.allows_placement())
        };
        for (&area, config) in &state.board_layout.areas {
            if !config.area_type.allows_placement() {
                continue;
            }
            for &adj in state.board_layout.adjacent_areas(area) {
                if !state.board_layout.is_canal_crossing(area, adj) && placeable(&adj) {
                    return (area, adj);
                }
            }
        }
        panic!("the 2P board carries an adjacent placeable pair");
    }

    /// Off a turn start there is no fork to price, and the refusal is TYPED —
    /// [`the_recipe_refuses_a_position_that_is_not_a_turn_start`]'s cell at the
    /// Shape-F recipe, because both recipes cross the ply through the same
    /// [`turn_start_placements`] and inherit its `None`.
    #[test]
    fn the_fee_flow_recipe_refuses_a_position_that_is_not_a_turn_start() {
        let rules = Rules::standard();
        let (root, ..) = fee_flow_root(&rules, world(101).0);
        let placing = apply_move_unchecked(&root, &Move::EndAdditionalPhase, &rules)
            .expect("ending the additional phase is always legal at a turn start");
        assert_eq!(fee_flow_recipe_pair(&placing, &rules), Err(FeeFlowMiss::NotATurnStart));
        assert_eq!(
            classify_fee_flow_candidates(&placing, &rules),
            Err(FeeFlowMiss::NotATurnStart)
        );
    }

    /// **The instantiation condition, named rather than inferred.** At a fresh
    /// 2P start both Presidents are in hand, so no placement can pay anything;
    /// the recipe says THAT, instead of reporting a fee-less position as a
    /// mover who cannot afford one. A void the step-10 row records has to be a
    /// fact about the position, and "the opponent's President is not on the
    /// board" is the fact.
    #[test]
    fn the_fee_flow_recipe_names_the_missing_opponent_president() {
        let rules = Rules::standard();
        let root = root_2p(&rules, world(102).0);
        for p in &root.players {
            assert_eq!(
                p.president_location,
                PresidentLocation::InHand,
                "a fresh setup starts every President in hand"
            );
        }
        assert_eq!(
            fee_flow_recipe_pair(&root, &rules),
            Err(FeeFlowMiss::NoOpponentPresidentOnBoard)
        );
        assert_eq!(
            classify_fee_flow_candidates(&root, &rules),
            Err(FeeFlowMiss::NoOpponentPresidentOnBoard)
        );
    }

    /// The recorded band is 2P and the registered raw arm names ONE seat, so a
    /// three-seat root is REFUSED rather than answered about an arbitrarily
    /// chosen rival — at 3P two opponents can hold Presidents in the same area
    /// and "the opponent's fee flow" stops naming one number.
    ///
    /// The cell also pins the lattice's ORDER: this root IS a turn start, so
    /// the seat-count arm is what it reports.
    #[test]
    fn the_fee_flow_recipe_refuses_a_root_that_is_not_two_player() {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(3), &rules, world(103).0)
            .expect("new_game builds a 3P setup");
        let root = finalize_setup(
            pending,
            &[
                StartingOrderChoice::Keep(0),
                StartingOrderChoice::Keep(0),
                StartingOrderChoice::Keep(0),
            ],
        )
        .expect("the setup finalizes");
        assert!(is_mover_turn_start(&root), "a finalized setup is a turn start");
        assert_eq!(
            fee_flow_recipe_pair(&root, &rules),
            Err(FeeFlowMiss::NotTwoPlayer { players: 3 })
        );
    }

    /// **The recipe's quantity, at the cheapest instantiation there is**: one
    /// assistant into the area the opponent's President stands in costs 1 Yen,
    /// paid to that President's owner (KB 01-rules-reference.md:97, "pay 1 Yen
    /// to each President's owner per assistant placed"), and the same-total
    /// placement anywhere else costs nothing.
    ///
    /// The cross-check runs through the CHECKED boundary, on purpose: the
    /// helper prices with `calculate_placement_cost` and verifies against its
    /// own `apply_move_unchecked` successor, so a cell re-verifying with the
    /// same call would be asking the same question twice. `EndAdditionalPhase`
    /// then `apply_move` puts both sides through `get_legal_moves`'s membership
    /// gate at the phase they are legal in, and reads the two purses there.
    #[test]
    fn the_fee_flow_pair_prices_one_yen_per_assistant_into_the_opponents_president_area() {
        let rules = Rules::standard();
        let (root, mover, opponent, occupied) = fee_flow_root(&rules, world(104).0);
        assert!(root.player(mover).assistants.in_hand >= 2, "the fixture holds assistants");
        assert!(root.player(mover).money >= 2, "the fixture can afford a fee");

        let pair = fee_flow_recipe_pair(&root, &rules).expect("the constructed root carries F");
        assert_eq!(pair.mover, mover);
        assert_eq!(pair.opponent, opponent);
        assert_eq!(pair.total, 1, "one assistant is the smallest total carrying both sides");
        assert_eq!(pair.fee, 1, "one assistant, one President, one Yen");
        assert_eq!(pair.fee_side, Move::PlaceThreeAreas { placements: vec![occupied] });
        assert_eq!(pair.fee_side_areas, [occupied].into_iter().collect::<BTreeSet<_>>());
        assert_eq!(pair.free_side_areas.len(), 1, "the free side shares the total");
        assert!(
            !pair.free_side_areas.contains(&occupied),
            "the free side must avoid the President's area"
        );

        let placing = apply_move(&root, &Move::EndAdditionalPhase, &rules)
            .expect("ending the additional phase is legal at a turn start");
        assert_eq!(placing.player(mover).money, root.player(mover).money, "the ply is free");

        let paid = apply_move(&placing, &pair.fee_side, &rules)
            .expect("F+ is in the placement phase's own legal list");
        assert_eq!(paid.player(mover).money + pair.fee, root.player(mover).money);
        assert_eq!(paid.player(opponent).money, root.player(opponent).money + pair.fee);

        let free = apply_move(&placing, &pair.free_side, &rules)
            .expect("F0 is in the placement phase's own legal list");
        assert_eq!(free.player(mover).money, root.player(mover).money);
        assert_eq!(free.player(opponent).money, root.player(opponent).money);
    }

    /// The selection rule's arms, each from data, and the two rules that make
    /// it a SELECTION rather than an optimization: ascending totals, and the
    /// MINIMAL nonzero fee at the first total carrying both sides.
    ///
    /// Picking the largest fee instead would be the recipe maximizing the very
    /// quantity the step-10 read exists to test — the registration's own
    /// objection, in its own words.
    #[test]
    fn the_selection_takes_the_minimal_nonzero_fee_then_the_lowest_area_set() {
        let mover = PlayerId(0);
        let opponent = PlayerId(1);
        let select = |candidates: &[FeeFlowCandidate]| {
            select_fee_flow_pair(mover, opponent, candidates)
        };

        assert_eq!(select(&[]), Err(FeeFlowMiss::NoPlacementCandidates));
        assert_eq!(
            select(&[fee_candidate(0, &[], 0)]),
            Err(FeeFlowMiss::NoPlacementCandidates),
            "the decline-to-place move is not a placement candidate"
        );
        assert_eq!(
            select(&[fee_candidate(1, &[3], 0), fee_candidate(2, &[4, 5], 0)]),
            Err(FeeFlowMiss::NoFeeSide)
        );
        assert_eq!(
            select(&[fee_candidate(1, &[3], 1), fee_candidate(2, &[4, 5], 2)]),
            Err(FeeFlowMiss::NoFreeSide)
        );
        assert_eq!(
            select(&[fee_candidate(1, &[3], 1), fee_candidate(2, &[4, 5], 0)]),
            Err(FeeFlowMiss::NoSharedTotal)
        );

        // Total 1 carries only a paying side, so it is SKIPPED, not refused;
        // at total 2 the fees are {2, 1, 1} and the two fee-1 candidates tie,
        // so the lowest area set wins on both sides.
        let pair = select(&[
            fee_candidate(1, &[3], 1),
            fee_candidate(2, &[7, 9], 2),
            fee_candidate(2, &[5, 6], 1),
            fee_candidate(2, &[4, 6], 1),
            fee_candidate(2, &[8, 9], 0),
            fee_candidate(2, &[2, 9], 0),
        ])
        .expect("total 2 carries both sides");
        assert_eq!(pair.mover, mover);
        assert_eq!(pair.opponent, opponent);
        assert_eq!(pair.total, 2, "a lower total with one side is skipped");
        assert_eq!(pair.fee, 1, "the MINIMAL nonzero fee, never the maximal");
        assert_eq!(pair.fee_side_areas, [AreaId(4), AreaId(6)].into_iter().collect::<BTreeSet<_>>());
        assert_eq!(pair.free_side_areas, [AreaId(2), AreaId(9)].into_iter().collect::<BTreeSet<_>>());
    }

    /// A mover with no Yen has no fee side at all — the generator drops every
    /// paying placement at its affordability test, so the paying candidates are
    /// not filtered out downstream, they were never enumerated.
    ///
    /// The void is [`FeeFlowMiss::NoFeeSide`] rather than a wallet-specific
    /// arm, and that is deliberate: a candidate list cannot distinguish "no
    /// placement reaches the President" from "the ones that do are
    /// unaffordable", because an unaffordable placement is not in it. Neither
    /// position carries the read, and the recipe says exactly that much.
    #[test]
    fn the_fee_flow_recipe_is_unaffordable_without_money() {
        let rules = Rules::standard();
        let (mut root, mover, _, occupied) = fee_flow_root(&rules, world(105).0);
        root.player_mut(mover).money = 0;
        check_invariants(&root, &rules).expect("a broke mover is a legal state");

        let candidates = classify_fee_flow_candidates(&root, &rules).expect("a turn start classifies");
        assert!(
            candidates.iter().all(|c| c.fee == 0),
            "every enumerated placement is free — the paying ones were never offered"
        );
        assert!(
            !candidates.iter().any(|c| c.areas.contains(&occupied)),
            "no placement into the President's area survives the generator's price check"
        );
        assert_eq!(fee_flow_recipe_pair(&root, &rules), Err(FeeFlowMiss::NoFeeSide));
    }

    /// **`PlaceTwoSameArea` is priced per ENCODING, exactly as `apply_move`
    /// prices it**: two assistants into `area` (the `[area, area]` slice) plus
    /// the Telegram leg's own `apply_placement(&[adj])` call. So the President
    /// standing in the ADJACENT area is charged once and the President standing
    /// in the MAIN area is charged twice — the "per assistant placed" rule,
    /// which a naive per-AREA price would get wrong in exactly one direction.
    ///
    /// Both fees are cross-checked twice over: `classify_fee_flow_candidates`
    /// refuses with [`FeeFlowMiss::PricingMismatch`] if the applied successor
    /// disagrees with the price, so the classification succeeding IS the
    /// unchecked-path cross-check, and the checked boundary is re-read here for
    /// the fee side of each case.
    #[test]
    fn the_telegram_adjacent_leg_is_priced_like_apply_prices_it() {
        let rules = Rules::standard();
        let (probe, ..) = fee_flow_root(&rules, world(106).0);
        let (area, adj) = adjacent_placeable_pair(&probe);

        // `president` names the area the opponent's President stands in; the
        // expected fee is the number of ASSISTANTS the move puts there.
        for (president, expected) in [(adj, 1u32), (area, 2u32)] {
            let mut root = root_2p(&rules, world(106).0);
            let mover = root.current_player;
            let opponent =
                root.players.iter().map(|p| p.id).find(|id| *id != mover).expect("2P seats two");
            grant_telegram(&mut root, &rules, mover);
            root.player_mut(opponent).president_location = PresidentLocation::OnBoard(president);
            check_invariants(&root, &rules).expect("the Telegram fixture is a legal state");
            assert!(root.player(mover).assistants.in_hand >= 3, "the bonus leg needs three");

            let telegram = Move::PlaceTwoSameArea { area, adjacent_bonus: Some(adj) };
            let candidates =
                classify_fee_flow_candidates(&root, &rules).expect("a turn start classifies");
            let priced = candidates
                .iter()
                .find(|c| c.mv == telegram)
                .unwrap_or_else(|| panic!("the fork does not offer {telegram:?}"));
            assert_eq!(priced.fee, expected, "President in {president:?}");
            assert_eq!(priced.total, 3, "two assistants plus the bonus one");

            let placing = apply_move(&root, &Move::EndAdditionalPhase, &rules)
                .expect("ending the additional phase is legal at a turn start");
            let after = apply_move(&placing, &telegram, &rules)
                .expect("the Telegram placement is in the legal list");
            assert_eq!(after.player(mover).money + expected, root.player(mover).money);
            assert_eq!(after.player(opponent).money, root.player(opponent).money + expected);
        }
    }

    /// Over ORGANIC turn starts: every pair the recipe returns is priced by the
    /// engine's own bytes at the CHECKED boundary, both sides share the total,
    /// the sides differ, and the answer is a function of the root alone.
    ///
    /// The miss histogram is RECORDED, never asserted — the yield is a property
    /// of the ecology (whether the opponent's President is on the board at a
    /// mover's turn start), and the registration says so out loud: "organic
    /// instantiation is realistic but UNCERTIFIABLE; no census fact covers
    /// president positions; the floor does real work." So the only asserted
    /// facts are that the sweep exercised the instrument at all: a position off
    /// a turn start, and a position whose candidate list classified.
    #[test]
    fn every_organic_fee_flow_pair_is_priced_by_the_bytes_and_is_a_function_of_the_root() {
        let rules = Rules::standard();
        let mut pairs = 0usize;
        let mut classified = 0usize;
        let mut misses: BTreeMap<String, usize> = BTreeMap::new();
        for index in [11u64, 12] {
            let stats = rung_game(&rules, world(index));
            let events = stats.record.events.len();
            for k in (0..events).step_by(5) {
                let Ok(state) = state_at(&stats.record, k, &rules) else { continue };
                if classify_fee_flow_candidates(&state, &rules).is_ok() {
                    classified += 1;
                }
                match fee_flow_recipe_pair(&state, &rules) {
                    Ok(pair) => {
                        pairs += 1;
                        let mover = pair.mover;
                        let opponent = pair.opponent;
                        assert!(pair.fee > 0, "event {k}: F+ must export a NONZERO fee");
                        assert_ne!(
                            pair.fee_side_areas, pair.free_side_areas,
                            "event {k}: the two sides are the same placement"
                        );
                        for side in [&pair.fee_side, &pair.free_side] {
                            assert_eq!(
                                placement_shape(side).map(|(total, _)| total),
                                Some(pair.total),
                                "event {k}: a side does not carry the pair's total"
                            );
                        }

                        let placing = apply_move(&state, &Move::EndAdditionalPhase, &rules)
                            .expect("a turn start ends its additional phase");
                        let paid = apply_move(&placing, &pair.fee_side, &rules)
                            .expect("F+ is in the placement phase's legal list");
                        assert_eq!(
                            paid.player(mover).money + pair.fee,
                            state.player(mover).money,
                            "event {k}: the payer's purse disagrees with the price"
                        );
                        assert_eq!(
                            paid.player(opponent).money,
                            state.player(opponent).money + pair.fee,
                            "event {k}: the opponent's purse disagrees with the price"
                        );
                        let free = apply_move(&placing, &pair.free_side, &rules)
                            .expect("F0 is in the placement phase's legal list");
                        assert_eq!(free.player(mover).money, state.player(mover).money);
                        assert_eq!(free.player(opponent).money, state.player(opponent).money);

                        assert_eq!(
                            Ok(pair),
                            fee_flow_recipe_pair(&state, &rules),
                            "event {k}: the recipe is not a function of the root"
                        );
                    }
                    Err(miss) => *misses.entry(miss.to_string()).or_default() += 1,
                }
            }
        }
        // RECORDED, never asserted (see the doc comment).
        let off_turn_start =
            misses.get(&FeeFlowMiss::NotATurnStart.to_string()).copied().unwrap_or(0);
        println!(
            "fee-flow recipe over organic play: {pairs} pairs, {classified} positions \
             classified, {off_turn_start} positions off a turn start, other misses {:?}",
            misses
                .iter()
                .filter(|(name, _)| *name != &FeeFlowMiss::NotATurnStart.to_string())
                .collect::<Vec<_>>()
        );
        assert!(off_turn_start > 0, "the sweep never saw a position off a turn start");
        assert!(classified > 0, "the sweep never classified a candidate list ({misses:?})");
    }

    /// **The empty placement is never a side.** `PlaceThreeAreas { placements:
    /// vec![] }` is always in the list — it is how a mover declines to place —
    /// and it prices at zero because it places NOTHING, not because it avoids
    /// the opponent's President. Admitting it as `F0` would pair a placement
    /// against a non-placement and call the difference a fee-flow distinction,
    /// which is [`the_empty_placement_is_never_a_side_of_the_recipe`]'s
    /// objection at the §6 recipe and the same objection here.
    ///
    /// The second half is the decisive one: from data, a list whose only
    /// zero-priced move is the empty placement refuses as
    /// [`FeeFlowMiss::NoFreeSide`] rather than pairing against it.
    #[test]
    fn the_empty_placement_is_never_a_fee_flow_side() {
        let rules = Rules::standard();
        let (root, ..) = fee_flow_root(&rules, world(107).0);
        let candidates = classify_fee_flow_candidates(&root, &rules).expect("a turn start classifies");
        let empty: Vec<&FeeFlowCandidate> = candidates.iter().filter(|c| c.total == 0).collect();
        assert_eq!(empty.len(), 1, "the decline-to-place move is offered exactly once");
        assert_eq!(empty[0].fee, 0);
        assert!(empty[0].areas.is_empty());

        let pair = fee_flow_recipe_pair(&root, &rules).expect("the constructed root carries F");
        assert!(pair.total > 0, "neither side is the decline");
        assert!(!pair.fee_side_areas.is_empty());
        assert!(!pair.free_side_areas.is_empty());

        assert_eq!(
            select_fee_flow_pair(
                PlayerId(0),
                PlayerId(1),
                &[fee_candidate(0, &[], 0), fee_candidate(1, &[3], 1)]
            ),
            Err(FeeFlowMiss::NoFreeSide),
            "the empty placement is not the zero-fee side of anything"
        );
    }

    // -----------------------------------------------------------------------
    // V4 §7 STEP 8 — the generating-ecology pinning (spec §10; V3 spec §5.1's
    // design adopted verbatim). RED at the pre-move tree by design.
    // -----------------------------------------------------------------------

    /// **THE ECOLOGY RE-PIN (V3 spec §5.1, landing at the step-8 adoption
    /// commit).** The v2 generating ecology's seats stop being the LIVE shipped
    /// contestant and become an EXPLICIT in-crate weighted construction keyed
    /// to `WeightTable::V2` — so every recorded `(predicate, master, ordinal)`
    /// triple keeps resolving to the same state through every later controller
    /// re-point, the identity-freeze era included. The seat field is ecology
    /// identity, so the id bumps `v2.2` → `v2.3` per the in-source
    /// any-field-change rule.
    ///
    /// RED at the pre-move tree (the id still reads `v2.2` there): that
    /// failure is checkpoint §7 step 8's red-then-green, on the ecology half.
    #[test]
    fn the_v2_ecology_repins_its_seats_and_bumps_to_v2_3_at_the_widening() {
        let ecology = GeneratingEcology::v2_lookahead();
        assert_eq!(
            ecology.id, "driven-generated-v2.3-lookahead",
            "the seat re-pin is an ecology field change and must bump the version string",
        );
    }

    /// The pinned seat's knob literals equal the shipped default's TODAY — a
    /// TRANSCRIPTION FACT, not a coupling: the pinned literals are the
    /// historical record and can never move; if the shipped default
    /// legitimately moves later, delete THIS assertion in the moving commit
    /// and the literals stay (the `PRE_FREEZE_2026_07_27` discipline).
    #[test]
    fn the_pinned_v2_seat_transcribes_todays_shipped_knobs() {
        assert_eq!(
            Contestant::LOOKAHEAD_V1_DEFAULT,
            Contestant::LookaheadV1 {
                b: PINNED_V2_SEAT_BEAM,
                k: PINNED_V2_SEAT_WORLDS,
                m: PINNED_V2_SEAT_RETREAT_SAMPLES,
                cap: None,
            },
            "the pinned knob literals transcribe the shipped default of 2026-08-24",
        );
    }

    /// **THE PRE-PADDING DECISION WALK, RECORDED (V3 spec §5.1's agent-level
    /// behavior-identity pin, the fixture-walk half).** Six lookahead decisions
    /// at the v2 ecology's seat construction — shipped knobs `b=4, k=1, m=8`,
    /// `WeightTable::V2` EXPLICIT — over three deterministic positions (a fresh
    /// 2P setup and two family-weighted walks forward), each agent on a fixed
    /// literal stream seed. The chosen moves are transcribed below from the
    /// PRE-MOVE tree (HEAD `08fd5ae`), so the walk enforces, forever and
    /// through the identity freeze, that the PINNED construction decides
    /// exactly as the pre-padding shipped seat decided.
    ///
    /// GREEN at the pre-move tree (it records the standing walk); REQUIRED
    /// GREEN after the move (padding must not move a decision); REQUIRED GREEN
    /// through every later controller re-point (the construction is pinned,
    /// not live). Mutation-proven at the wave: a table swap (V1 or ZERO) or a
    /// knob swap must flip at least one recorded move.
    #[test]
    fn the_pinned_v2_seat_walk_reproduces_the_recorded_pre_padding_decisions() {
        use crate::game::policy::RolloutPolicy;
        use crate::game::search_actions::DecisionHandle;
        use rand::SeedableRng;

        let rules = Rules::standard();
        // (setup seed, family-weighted steps forward, agent stream seed)
        const POSITIONS: [(u64, usize, u64); 3] =
            [(0xD1, 0, 0xA11CE0), (0xD2, 60, 0xA11CE1), (0xD3, 140, 0xA11CE2)];
        // The recorded walk: two consecutive decisions per position, the move's
        // `Debug` form verbatim.
        const RECORDED: [[&str; 2]; 3] = [
            ["EndAdditionalPhase", "PlaceThreeAreas { placements: [AreaId(0), AreaId(1), AreaId(8)] }"],
            [
                "PlaceThreeAreas { placements: [] }",
                "MovePresident { path: [AreaId(7), AreaId(4)], destination: AreaId(4) }",
            ],
            ["SkipBuilding", "EndAdditionalPhase"],
        ];

        for (i, (seed, steps, agent_seed)) in POSITIONS.into_iter().enumerate() {
            let pending =
                new_game(GameConfig::for_player_count(2), &rules, seed).expect("setup");
            let mut state =
                finalize_setup(pending, &[StartingOrderChoice::Keep(0); 2]).expect("finalize");
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed ^ 0x3F3F);
            let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
            for step in 0..steps {
                let mv = {
                    let handle = DecisionHandle::new(&state, &rules);
                    policy
                        .pick(&handle, &mut rng)
                        .unwrap_or_else(|| panic!("dead end at seed {seed:#x} step {step}"))
                };
                state = apply_move_unchecked(&state, &mv, &rules).expect("policy move applies");
            }
            // Build through the SAME configuration path the scan uses. A
            // mutation from the pinned variant to the live default must move
            // this recorded walk rather than escaping through a direct helper.
            let mut agent = v2_lookahead_config().seat.build(&rules, agent_seed);
            for (j, recorded) in RECORDED[i].iter().enumerate() {
                let mover = state.current_player;
                let view = observe(&state, mover).expect("the mover's view");
                let report = agent.choose(&view, &rules).expect("the pinned seat decides");
                assert_eq!(
                    format!("{:?}", report.mv),
                    *recorded,
                    "position {i} decision {j}: the pinned v2 seat construction diverged from \
                     the recorded pre-padding walk",
                );
                state = apply_move(&state, &report.mv, &rules)
                    .expect("the pinned seat's move is legal at the checked boundary");
            }
        }
    }
}
