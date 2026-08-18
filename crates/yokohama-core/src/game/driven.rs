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
//! **What this module still deliberately does NOT ship**: the runner (T5). It is
//! not stubbed here — a stub is a surface a reader has to check the emptiness
//! of.

// T1 RED (Phase C): the layer-2 helpers below are DECLARED with placeholder
// bodies so the failing pins can name them, so their inputs, imports and some
// of the helpers themselves are unused until the GREEN commit implements them.
// This attribute is removed by that commit.
#![allow(dead_code, unused_imports)]

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::actions::{
    calculate_church_faith, customs_max_imported_goods, eligible_church_spaces,
    eligible_customs_spaces_for_cost, has_church_or_customs_assistant,
};
use crate::game::agent::HONEST_MAX_TURNS;
use crate::game::apply_move::apply_move_unchecked;
use crate::game::arena::{
    agent_stream_seed, mix, play_from_state, require_supported_player_count, run_arena_game,
    AgentBench, ArenaAgent, Contestant, ExplainHandle, PlayError, PlayLimits, SeatBench, Slot,
    SALT_DRIVEN_REPLICATION,
};
use crate::game::belief::BeliefSampler;
use crate::game::end_game::is_game_over;
use crate::game::eval::WeightTable;
use crate::game::invariants::check_invariants;
use crate::game::legal_moves::get_legal_moves;
use crate::game::lookahead::{LookaheadAgent, RootScore, SearchScheme, SearchStats};
use crate::game::move_legality::{is_move_legal, PathMode};
use crate::game::observe::observe;
use crate::game::policy::RolloutPolicy;
use crate::game::power::calculate_power;
use crate::game::record::{replay_from, state_at, GameRecord, RecordedEvent, SeatProvenance};
use crate::game::rules::Rules;
use crate::game::scoring::{final_result, GameResult};
use crate::game::seeds::{SeedSchedule, WorldSeed};
use crate::game::semantics::{record_digest, state_digest, SEMANTICS_REV};
use crate::types::{
    AreaId, AreaType, ChurchTrack, CustomsTrack, EndGameTrigger, GameState, Move, PendingChoice,
    PlayerId, PresidentLocation,
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
}

impl RootPredicate {
    /// Every variant, for callers that enumerate the vocabulary (the runner's
    /// `gen:` grammar names this set in its refusal, and the tests sweep it).
    ///
    /// HAND-MAINTAINED: adding a variant is a compile error in [`Self::as_str`],
    /// [`Self::holds`] and [`Self::config`] — all three are exhaustive matches —
    /// but NOT here, so a new variant must be added to this array by hand. The
    /// name-distinctness pin is what catches a copy-paste that forgot to.
    pub const ALL: [Self; 6] = [
        Self::FinalRoundAlreadyTriggered,
        Self::TriggerRoundNotFinal,
        Self::EndFirableAheadInTruth,
        Self::EndFirableBehindInTruth,
        Self::ExclusiveRaceOneStep,
        Self::AchievementClaimAvailable,
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
        // T1 RED: the layer-2 arms are not implemented yet. The two shipped
        // arms keep their behaviour so the existing suite stays green.
        let _ = rules;
        match self {
            Self::FinalRoundAlreadyTriggered => {
                state.end_game_triggered && state.final_round && !is_game_over(state)
            }
            Self::TriggerRoundNotFinal
            | Self::EndFirableAheadInTruth
            | Self::EndFirableBehindInTruth
            | Self::ExclusiveRaceOneStep
            | Self::AchievementClaimAvailable => false,
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
    pub fn ecology(self) -> GeneratingEcology {
        self.config().ecology
    }

    /// [`Self::ecology`] plus the live parts a scan needs — see
    /// [`GeneratingConfig`], which exists so the recorded seat ECHOES cannot
    /// drift from the contestants the scan actually seats.
    fn config(self) -> GeneratingConfig {
        // T1 RED: the ecology mapping is not implemented yet.
        let _ = self;
        v1_config()
    }
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
    // T1 RED: not implemented.
    let _ = (state, rules);
    true
}

/// [`width_ok`]'s arithmetic, over a legal-list length the caller already holds.
///
/// The bound is spelled ONCE, here: two of the layer-2 arms need the legal list
/// anyway (they probe it), so they call this rather than paying a second
/// enumeration, and one spelling is what stops the two paths drifting to
/// different widths.
const fn width_ok_len(len: usize) -> bool {
    // T1 RED: not implemented.
    let _ = len;
    true
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
    // T1 RED: not implemented.
    let _ = (state, rules);
    0
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
    // T1 RED: the enumeration is not implemented yet.
    let _ = trigger;
    None
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
    // T1 RED: not implemented.
    let _ = (trigger, state);
    false
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
    // T1 RED: not implemented.
    let _ = (state, rules);
    Vec::new()
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
    // T1 RED: not implemented.
    let _ = state;
    Vec::new()
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
    // T1 RED: not implemented.
    let _ = (state, legal, rules);
    Vec::new()
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
pub const GENERATED_ECOLOGY_V1: &str = "driven-generated-v1"; // T1 RED: not amended yet

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
pub const GENERATED_SCAN_GAMES: u32 = 4; // T1 RED: not raised yet

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
pub const GENERATED_STRIDE_CAP: u32 = 0; // T1 RED: the stride is not implemented yet

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

/// The v2 generating configuration's NAME — Phase C's layer-2 ecology.
pub const GENERATED_ECOLOGY_V2_LOOKAHEAD: &str = "unimplemented"; // T1 RED

/// The v2 ecology's scan bound.
///
/// Sized by the SAME arithmetic as v1's (pilot 4 + confirmatory 6 + escrow 6 per
/// predicate, at cap 2) with slack, and with NO supply guarantee: the layer-2
/// predicates are selective by design, and an `Exhausted` refusal at this bound
/// is RECORDED EVIDENCE about the family rather than a failure of the scan
/// (design §1 family 3 says so in as many words). A games raise at a fixed
/// master and cap is prefix-stable, which is the recovery if a frozen block runs
/// short.
pub const GENERATED_V2_SCAN_GAMES: u32 = 0; // T1 RED

/// The seat count v2's generating ecology plays at — 2P throughout (the canon
/// priors are 2P priors).
const GENERATED_V2_PLAYERS: u8 = 2;

/// The contestant every seat of v2's generating ecology plays: the shipped
/// provisional production contestant, uncapped.
///
/// This ecology generates the SUBJECT'S OWN reachable positions, which is the
/// right population for an on-policy checkpoint and is said out loud here
/// because it is also the ecology's whole cost: both seats search. PROVISIONAL
/// until D1's identity freeze, exactly as [`ReplyPolicy`]'s `*AtFinal` arms are
/// — when that freeze re-points the champion, this const re-points with it and
/// the version string bumps, because the ecology is identity.
const GENERATED_V2_SEAT: Contestant = Contestant::LOOKAHEAD_V1_DEFAULT;

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
    /// the shipped provisional contestant.
    pub fn v2_lookahead() -> Self {
        v2_lookahead_config().ecology
    }
}

/// A generating configuration: the recorded [`GeneratingEcology`] plus the live
/// parts a scan needs to play the games.
///
/// The two travel together, and the ecology is BUILT FROM the contestant rather
/// than written beside it, so the recorded seat echoes cannot drift from the
/// seats the scan actually benches — the failure mode a descriptor exists to
/// prevent, appearing inside the thing that writes descriptors.
struct GeneratingConfig {
    /// The contestant every seat plays.
    seat: Contestant,
    /// The seat count.
    players: u8,
    /// What gets recorded on the descriptor.
    ecology: GeneratingEcology,
}

/// The v1 (rung) generating configuration — family 1's.
fn v1_config() -> GeneratingConfig {
    GeneratingConfig {
        seat: GENERATED_SEAT,
        players: GENERATED_PLAYERS,
        ecology: GeneratingEcology {
            id: GENERATED_ECOLOGY_V1.to_string(),
            driver: GENERATED_DRIVER.to_string(),
            players: GENERATED_PLAYERS,
            seats: vec![GENERATED_SEAT.echo(); GENERATED_PLAYERS as usize],
            seed_derivation: GENERATED_SEED_DERIVATION.to_string(),
            games: GENERATED_SCAN_GAMES,
            per_game_cap: GENERATED_STRIDE_CAP,
        },
    }
}

/// The v2 (lookahead) generating configuration — the layer-2 families'.
fn v2_lookahead_config() -> GeneratingConfig {
    GeneratingConfig {
        seat: GENERATED_V2_SEAT,
        players: GENERATED_V2_PLAYERS,
        ecology: GeneratingEcology {
            id: GENERATED_ECOLOGY_V2_LOOKAHEAD.to_string(),
            driver: GENERATED_DRIVER.to_string(),
            players: GENERATED_V2_PLAYERS,
            seats: vec![GENERATED_V2_SEAT.echo(); GENERATED_V2_PLAYERS as usize],
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
fn preflight(root: &GameState, seats: &[SeatSpec], opts: DrivenOpts) -> Result<(), DrivenError> {
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
    let config = predicate.config();
    let ecology = config.ecology;
    let games = ecology.games;
    let _per_game_cap = ecology.per_game_cap; // T1 RED: unused until the stride lands
    let mut schedule = SeedSchedule::diagnostic(master);
    let mut found: u32 = 0;

    for game_index in 0..games {
        let world = schedule.next_world();
        // The ecology's own seating, on the arena's own keying — the same
        // derivation `drive_to_terminal` uses, so a generated root is reachable
        // by a run anyone can reproduce from the master alone.
        let mut make = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
            config.seat.build(rules, agent_stream_seed(world, seat, Slot(seat.0))).0
        };
        let stats = run_arena_game(world, config.players, rules, &mut make, true)
            .map_err(|e| RootError::Generate { game_index, detail: e.to_string() })?;

        let events = &stats.record.events;
        let mut state = state_at(&stats.record, 0, rules)
            .map_err(|e| RootError::Generate { game_index, detail: e.to_string() })?;
        // T1 RED: the stride is not implemented yet.
        // INCLUSIVE of the past-the-last position, for totality: `state_at(record,
        // len)` is a real position (the terminal state) and a future predicate may
        // want it. The shipped one excludes it — its `!is_game_over` conjunct —
        // so that position is unreachable today rather than dead, and stopping the
        // range one short would make the scan quietly predicate-specific.
        for position in 0..=events.len() {
            if predicate.holds(&state, rules) {
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

    Err(RootError::Exhausted { predicate, ordinal, found, games })
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
            echo: spec.contestant().echo(),
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
        run_arena_game, ArenaGameStats, ControllerProvenance, DecisionReport,
    };
    use crate::game::end_game::is_game_over;
    use crate::game::invariants::check_invariants;
    use crate::game::record::{state_at, RecordedEvent};
    use crate::game::scoring::{ScoreBreakdown, Standing};
    use crate::game::seeds::SeedSchedule;
    use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
    use crate::types::{
        BuildingSite, EndGameTrigger, GameConfig, MercenaryState, PlayerView, PresidentLocation,
        TradingHouseState, TurnPhase,
    };
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
                if want(&state) {
                    return state;
                }
                if let Some(event) = events.get(position) {
                    state = replay_from(&state, std::slice::from_ref(event), rules)
                        .expect("a generated game replays event by event");
                }
            }
        }
        panic!("anti-vacuity: no position in {games} rung games at {master:#018x} is {what}");
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
    /// is 24 games of both-seats search and this is a unit test.
    #[test]
    fn the_v2_lookahead_ecology_names_the_shipped_contestant_at_both_seats() {
        let ecology = GeneratingEcology::v2_lookahead();
        assert_eq!(ecology.id, "driven-generated-v2-lookahead");
        assert_eq!(ecology.id, GENERATED_ECOLOGY_V2_LOOKAHEAD);
        assert_eq!(ecology.players, 2, "the canon priors are 2P priors");
        assert_eq!(ecology.games, 24);
        assert_eq!(ecology.games, GENERATED_V2_SCAN_GAMES);
        assert_eq!(ecology.per_game_cap, GENERATED_STRIDE_CAP, "both ecologies stride the same");
        assert_eq!(
            ecology.seats,
            vec![Contestant::LOOKAHEAD_V1_DEFAULT.echo(); 2],
            "the layer-2 ecology must seat the SHIPPED provisional contestant, resolved",
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
                | RootPredicate::AchievementClaimAvailable => GeneratingEcology::v2_lookahead(),
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
}
