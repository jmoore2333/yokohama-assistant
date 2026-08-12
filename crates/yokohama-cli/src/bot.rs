//! Bot seats for sim-mode `play vs bot` (Phase 3.2 Task 12, spec §9): who
//! plays which seat, how a bot's decision seed is derived, and the honest
//! decision chain itself.
//!
//! **SIM MODE ONLY, any mix with at least one human.** The restriction is
//! structural rather than a policy: `observe` refuses a live-tracked seat whose
//! starting order is `Unknown`, and a live game's reveal choreography would
//! deadlock waiting for a physical card a bot cannot read off the table.
//!
//! **WHO can sit in a seat is a [`Contestant`]** (Phase 3.5c W5, design §1.1):
//! the three preserved rungs as `Contestant::Policy`, plus the two EXPLAINABLE
//! evaluator arms `greedy-v1` and `lookahead-v1`. The two labelled DIAGNOSTIC
//! arms (`mcts-v1`, the S2 atomic arm) are refused by name in [`eligible`] --
//! the CLI is a player's driver, not a second diagnostic surface, and every
//! evaluator seat it runs has to be `why`-explainable. Rungs are grandfathered
//! so that every save ever written by this driver still resumes.
//!
//! THE HONEST CHAIN, per decision (spec §9, the same shape `arena::PolicyAgent`
//! runs): `observe(true_state, seat)` -> `BeliefSampler::from_view` -> ONE
//! sampled world -> `DecisionHandle` over that world -> `RolloutPolicy::pick`.
//! A bot is never handed `Session::state()` or `Session::record()`. The one
//! place the true state appears in this module is [`decide`]'s argument (and
//! [`BotChooser::choose`]'s, which forwards it), and the FIRST line of the
//! honest path is the `observe` call that masks it -- nothing below that line
//! reads `state` again. A sampler refusal is a hard error and never a fallback
//! to privileged play: a bot that quietly starts reading the deck order would
//! still play a legal game, which is exactly why it has to be loud.
//!
//! An EVALUATOR seat runs the agent's OWN honest chain instead
//! (`GreedyAgent`/`LookaheadAgent` sample their world from their own stream
//! inside `choose`), constructed here exactly as `Contestant::build` constructs
//! it -- same table, same constructor, same knobs -- and pinned equal to it by
//! `decide_matches_contestant_build_for_every_eligible_evaluator_arm`. It takes
//! the seat's `PlayerView` and nothing else, so the honesty property is the
//! same one, reached through the agent rather than through this module.
//!
//! ONE decision path serves BOTH callers ([`decide`]): the REPL's bot pump and
//! -- from W5 -- `why`, which re-runs it at a replayed position. Reconstruction
//! identity is then STRUCTURAL: `why` calls literally the function the pump
//! called, so "the pump built the agent one way and `why` built it another"
//! is not a divergence class that exists.
//!
//! Not claimed, stated so nobody chases it: a CLI bot seat is NOT
//! bit-stream-identical to an arena seat. The arena holds ONE agent per game
//! whose RNG stream advances across decisions; this module builds a FRESH agent
//! per decision keyed by `(bot_seed, events.len())`, for the serialization
//! reason below. Both are honest; they are different determinism schemes.
//!
//! WHY THE CLI DERIVES ITS OWN DECISION SEEDS instead of reusing
//! `arena::PolicyAgent`: that agent carries a long-lived `ChaCha8Rng` whose
//! state is not serializable, so a save/reload would silently restart the
//! stream. Here each decision draws from `(bot_seed, record().events.len())` --
//! the recorded seed plus a PUBLIC counter -- so a resumed game continues the
//! bot's own line instead of a fresh one, and `undo` re-derives the same draws
//! it did the first time. The event count is a valid per-decision key only
//! because sim mode never interleaves reveal events with moves (spec §9 pins
//! this as the assumption it is); a live game would break the correspondence,
//! which is the second reason bots are sim-only.
//!
//! Nothing here derives a seed from hidden state. Both sub-seeds come from the
//! agent's own recorded seed and the public event count, so the sampler cannot
//! be correlated with the truth it exists to be blind to (`game::agent`'s SEED
//! HYGIENE rule, restated because this is another implementation of it).

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use yokohama_core::eval::{Explanation, WeightTable};
use yokohama_core::lookahead::{RootScore, SearchScheme, SearchStats};
use yokohama_core::*;

/// [`SeatProvenance::version`] for a human seat. A person has no
/// implementation to version; 0 reads as "not applicable" and cannot collide
/// with a policy's `POLICY_IMPL_VERSION`, which starts at 1.
const HUMAN_CONTROLLER_VERSION: u32 = 0;

/// SplitMix64's increment (the 64-bit golden-gamma constant).
const SPLITMIX_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// One SplitMix64 step. A LOCAL copy of the primitive `game::seeds` streams
/// with: core's `splitmix64` is `pub(crate)` to that crate, so the CLI cannot
/// call it. The constants are the published SplitMix64 ones and this function
/// stands or falls with `seeds.rs`'s -- if that module ever publishes its
/// mixer, this copy should go.
pub(crate) fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(SPLITMIX_GAMMA);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Why a bot produced no move.
///
/// The two shapes get DIFFERENT treatments and conflating them would hide a
/// real fault behind a benign message: a refused view or belief sample means
/// the driver handed the agent something it could not honestly work from (a
/// lockdown), while a dead end means the policy legitimately found no move at a
/// decision point (a halt, on a session that is still sound and playable).
#[derive(Debug)]
pub enum BotError {
    /// The seat's own view could not be taken -- a live `Unknown` seat, which
    /// bots are sim-only to avoid. Treated as a fault.
    View(String),
    /// `BeliefSampler::from_view` refused the view. A fault: the alternative
    /// (play from the true state) is precisely the cheating this chain exists
    /// to prevent.
    Belief(String),
    /// The policy found no move at this decision point.
    DeadEnd(String),
}

impl std::fmt::Display for BotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::View(m) => write!(f, "the seat's view could not be taken: {m}"),
            Self::Belief(m) => write!(f, "the belief sampler refused the seat's view: {m}"),
            Self::DeadEnd(m) => write!(f, "no move at this decision point: {m}"),
        }
    }
}

/// The repo's standing treatment for a public error type -- see
/// `suggest::SuggestError`, which this rides along with.
impl std::error::Error for BotError {}

/// Who plays each seat, and the bots' shared stream seed.
///
/// One entry per seat in `PlayerId` order, so a seat can never be looked up in
/// a table that was built for a different game. An all-`None` table (the
/// default for every game the wizard is not asked about) makes every bot path
/// in the REPL a no-op.
#[derive(Clone, Debug, Default)]
pub struct BotSeats {
    seats: Vec<Option<Contestant>>,
    /// Full-`u64`, drawn once at wizard time and recorded per bot seat in the
    /// record's run provenance -- the only way back to a bot's line.
    seed: u64,
}

impl BotSeats {
    /// No bots: every seat human.
    pub fn none() -> Self {
        Self::default()
    }

    /// `seats[i]` is the contestant playing seat `i`, or `None` for a human.
    pub fn new(seats: Vec<Option<Contestant>>, seed: u64) -> Self {
        Self { seats, seed }
    }

    /// Is `seat` played by a bot?
    pub fn is_bot(&self, seat: PlayerId) -> bool {
        self.contestant_at(seat).is_some()
    }

    /// The contestant playing `seat`, if any. A seat past the table's end is a
    /// human: the safe direction, since the alternative is a phantom bot
    /// moving on a seat nobody configured.
    ///
    /// Named for what it returns since the W5 retype -- it was `policy_at`
    /// while a seat could only hold a [`RolloutPolicy`].
    pub fn contestant_at(&self, seat: PlayerId) -> Option<Contestant> {
        self.seats.get(usize::from(seat.0)).copied().flatten()
    }

    /// Does any seat have a bot? (`false` short-circuits every bot path.)
    pub fn any(&self) -> bool {
        self.seats.iter().any(Option::is_some)
    }

    /// The recorded stream seed.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// One `bot` flag per seat, for the panels' badge.
    pub fn flags(&self) -> Vec<bool> {
        self.seats.iter().map(Option::is_some).collect()
    }

    /// The decision seed for the decision taken when the record holds
    /// `event_count` events: a SplitMix64 mix of the recorded seed and that
    /// public counter. Pure, so `undo` and resume re-derive the same draw.
    pub fn decision_seed(&self, event_count: usize) -> u64 {
        let mut state = self.seed ^ (event_count as u64);
        splitmix64(&mut state)
    }

    /// This table as record provenance (spec §5), one entry per seat.
    ///
    /// Names are never hand-typed: a bot seat's whole descriptor comes from
    /// `Contestant::provenance` (which routes to the controller's own
    /// descriptor, and for a rung delegates to `ControllerProvenance::from_policy`
    /// -- so every rung seat serializes to the bytes it always did, pinned by
    /// `a_rung_seats_provenance_is_byte_identical_across_the_retype`), a human
    /// seat's from [`CONTROLLER_HUMAN`]. An evaluator seat's KNOBS ride the
    /// same `params` map, which is what lets [`from_provenance`](Self::from_provenance)
    /// resume the search the record describes rather than the default one.
    /// `observation` is [`OBSERVATION_VIEW`] for every
    /// sim seat -- for a bot that is exactly true (the chain above is the only
    /// way it sees anything), and for a human it records the CHANNEL this
    /// driver offers (per-seat masked panels), not a claim about what the
    /// person at the keyboard saw: they can toggle `privacy` off, and in sim
    /// mode they ran the setup wizard.
    pub fn seat_provenance(&self) -> Vec<SeatProvenance> {
        self.seats
            .iter()
            .map(|contestant| match contestant {
                Some(c) => c.provenance(Some(self.seed)).observed_as(OBSERVATION_VIEW),
                None => SeatProvenance {
                    controller: CONTROLLER_HUMAN.to_string(),
                    version: HUMAN_CONTROLLER_VERSION,
                    params: std::collections::BTreeMap::new(),
                    agent_seed: None,
                    observation: OBSERVATION_VIEW.to_string(),
                },
            })
            .collect()
    }

    /// Rebuild the table from a loaded record's provenance -- the ONE path
    /// both a fresh wizard game and a resumed save take, so a resumed game
    /// cannot come back with a different idea of who plays what.
    ///
    /// Returns any warnings alongside: a `controller` name this build does not
    /// know cannot be reconstructed, so that seat comes back HUMAN and says so
    /// loudly. Silently treating it as a bot of some other policy would change
    /// the game the record describes.
    ///
    /// **Reconstruction reads BOTH stored halves** since the W5 retype (design
    /// §1.5): `Contestant::from_parts(controller, params)`, the inverse of the
    /// descriptor that wrote them, rather than a bare-name reparse -- which was
    /// lossless only while the wizard wrote exclusively default knobs, and
    /// would silently resume a recorded `lookahead-v1` at `b=8` as a `b=4`
    /// search. Four arms, in order:
    ///
    /// * [`CONTROLLER_HUMAN`] -- a human seat, unchanged;
    /// * parsed and [`eligible`] -- the seat's contestant;
    /// * parsed and DIAGNOSTIC -- HUMAN, loudly, naming it as a diagnostic;
    /// * unparsed -- HUMAN, loudly, carrying the refusal's own text (which is
    ///   how `bga-human` seats from imported records keep resuming as people).
    ///
    /// Then the IDENTITY COMPARISON, which never refuses:
    /// [`identity_drift_warnings`] holds the record's stored version and params
    /// against a freshly derived descriptor and warns once per differing field.
    /// It cannot fire inside one build; it exists for the resume of a record
    /// written by another one.
    pub fn from_provenance(run: Option<&RunProvenance>) -> (Self, Vec<String>) {
        let Some(run) = run else {
            return (Self::none(), Vec::new());
        };
        let mut warnings = Vec::new();
        let mut seed: Option<u64> = None;
        let mut seats = Vec::with_capacity(run.seats.len());
        for (i, seat) in run.seats.iter().enumerate() {
            if seat.controller == CONTROLLER_HUMAN {
                seats.push(None);
                continue;
            }
            match Contestant::from_parts(&seat.controller, &seat.params).map(eligible) {
                Ok(Ok(contestant)) => {
                    // This driver keys every bot seat with ONE stream seed, so
                    // a record carrying two is not something it wrote. Say so
                    // rather than silently keeping whichever came last: the
                    // seats would then replay off a stream that was never
                    // theirs, and the record would still claim otherwise.
                    match (seed, seat.agent_seed) {
                        (Some(known), Some(s)) if known != s => warnings.push(format!(
                            "warning: seat {i} records bot stream seed {s}, but an earlier bot seat \
                             records {known} — this driver plays every bot seat off one stream, so \
                             it keeps {known} and seat {i} will not reproduce its recorded line",
                        )),
                        (None, Some(s)) => seed = Some(s),
                        _ => {}
                    }
                    warnings.extend(identity_drift_warnings(i, seat, contestant));
                    seats.push(Some(contestant));
                }
                Ok(Err(why)) => {
                    warnings.push(format!(
                        "warning: seat {i} was played by controller \"{}\", but {why} — it resumes \
                         as a HUMAN seat, so you are now playing it",
                        seat.controller,
                    ));
                    seats.push(None);
                }
                Err(why) => {
                    warnings.push(format!(
                        "warning: seat {i} was played by controller \"{}\", which this build does not \
                         know how to run — it resumes as a HUMAN seat, so you are now playing it \
                         ({why})",
                        seat.controller,
                    ));
                    seats.push(None);
                }
            }
        }
        (Self { seats, seed: seed.unwrap_or(0) }, warnings)
    }

    /// Drop every bot seat, keeping the seat count -- the sim-only rule made
    /// structural for a record that claims otherwise. Returns the seats it
    /// dropped so the caller can say so.
    pub fn without_bots(&self) -> (Self, Vec<usize>) {
        let dropped = self.seats.iter().enumerate().filter(|(_, p)| p.is_some()).map(|(i, _)| i).collect();
        (Self { seats: vec![None; self.seats.len()], seed: 0 }, dropped)
    }
}

/// **What a wizard-configured bot seat plays** -- `lookahead-v1` at the shipped
/// knobs (b=4/k=1/m=8, uncapped, `WeightTable::V2`), named as a const so the
/// one behavioral choice has one home (design §4.1).
///
/// This is the production-intent controller: the S1 turn-boundary search the
/// v2 freeze shipped, the same one `suggest` already runs on your own seat. A
/// bot seat is therefore no longer a rollout RUNG, which sampled its move from
/// a distribution and could account for nothing -- it VALUES its move, which is
/// what makes `why` possible at all.
///
/// **PROVISIONAL, and the word is load-bearing.** At D1's identity freeze this
/// re-points at the FROZEN CHAMPION, constructed from the explicit knobs
/// recorded in that pre-registration -- never from a `*_DEFAULT` constant,
/// which is D1's own rule (a shared default that moves would move every cell
/// quoting it). `suggest`'s own construction (`suggest.rs`'s
/// `LookaheadAgent::new` at constructor defaults) unifies with this const in
/// that same commit, so the seat you play against and the seat that advises you
/// cannot drift apart -- recorded at design §4.3 so the freeze checklist
/// inherits it. Until then `LOOKAHEAD_V1_DEFAULT` is itself frozen identity
/// (D1: the shared default constants may not move without an owner ruling), so
/// the provisional default cannot drift silently either.
///
/// There is NO picker. The wizard's question stays "which seats are bots";
/// difficulty levels and per-seat contestant choice are out of scope (OD-4(i)).
/// A RESUME still restores whatever eligible contestant the record names -- a
/// consequence of the provenance preserve-list, not a configuration surface.
///
/// The wizard uses this const WITHOUT re-checking [`eligible`] (the narrowing
/// guards the resume path, where the name comes from a file). What keeps that
/// safe as the const moves is
/// `the_provisional_default_bot_is_a_contestant_this_driver_runs`, and
/// [`decide`]'s diagnostic arm names that pin for the same reason.
pub(crate) const PROVISIONAL_DEFAULT_BOT: Contestant = Contestant::LOOKAHEAD_V1_DEFAULT;

/// Which contestants this driver will run as a bot seat, and why the rest are
/// refused BY NAME (design §1.2).
///
/// Eligible: the three preserved RUNGS (grandfathered -- every save this driver
/// ever wrote names one, and dropping them would make old games unresumable),
/// plus `greedy-v1` and `lookahead-v1`, the two arms that ACCOUNT for their
/// move. `mcts-v1` and the S2 atomic arm are labelled diagnostics in core
/// (mcts-v1 measured and NOT adopted, per the 3.4 architecture decision), and
/// the principle is worth stating once: every EVALUATOR seat this driver runs
/// must be explainable by `why`, so a diagnostic arm is refused rather than
/// quietly played. The refusal is a downgrade, never a load failure -- the game
/// always opens, the seat degrades loudly (see
/// [`BotSeats::from_provenance`]).
///
/// The `Err` is a complete sentence, because it is quoted verbatim into a
/// resume warning and read on its own by `why`'s address resolution.
/// **The match is EXHAUSTIVE, with no `_` arm, deliberately** (the
/// `move_legality` discipline): a contestant added to core lands here as a
/// COMPILE ERROR asking whether the CLI should run it, rather than being
/// silently admitted or silently refused.
pub(crate) fn eligible(contestant: Contestant) -> Result<Contestant, String> {
    match contestant {
        Contestant::Policy(_) | Contestant::GreedyV1 { .. } | Contestant::LookaheadV1 { .. } => {
            Ok(contestant)
        }
        Contestant::MctsV1 { .. } | Contestant::AtomicDiag { .. } => {
            Err(diagnostic_refusal(contestant))
        }
    }
}

/// The one sentence a diagnostic arm is refused with -- shared by [`eligible`]
/// and [`decide`] so the two cannot describe the same refusal differently.
fn diagnostic_refusal(contestant: Contestant) -> String {
    format!("{} is a diagnostic contestant, not available as a CLI bot", contestant.descriptor().0)
}

/// One warning per field on which the RECORD and THIS BUILD disagree about what
/// the seat's contestant is -- design §1.5's cross-build drift check.
///
/// **Warns, never refuses.** The seat plays on, under the CURRENT build; the
/// alternative (refusing to load) is the failure the provenance preserve-list
/// forbids, and a silent resume is the one that lets a moved weight table pass
/// for the table the record names.
///
/// It compares the stored `version` against the controller's own version const
/// and every param against a freshly derived `descriptor()`. That one generic
/// comparison is what makes the descriptor's IDENTITY keys load-bearing rather
/// than decorative: `Contestant::from_parts` accepts `eval_impl`, `weights` and
/// greedy's `placement_cap` without consuming them (a cross-build record must
/// stay loadable), so THIS is the only place their drift becomes visible.
///
/// The key set is the UNION of both maps, so a key one side does not emit at
/// all is drift too, reported as `absent`. `agent_seed` and `observation` are
/// `SeatProvenance` FIELDS rather than params and are deliberately out of scope
/// here -- the stream seed has its own reconciliation above.
fn identity_drift_warnings(i: usize, seat: &SeatProvenance, contestant: Contestant) -> Vec<String> {
    let (_, version, params) = contestant.descriptor();
    let mut warnings = Vec::new();
    if seat.version != version {
        warnings.push(format!(
            "warning: seat {i} records controller \"{}\" at version {}, but this build runs \
             version {version} — the seat still plays, under the CURRENT build",
            seat.controller, seat.version,
        ));
    }
    let keys: std::collections::BTreeSet<&str> =
        seat.params.keys().chain(params.keys()).map(String::as_str).collect();
    let shown = |v: Option<&i64>| v.map_or_else(|| "absent".to_string(), i64::to_string);
    for key in keys {
        let (recorded, current) = (seat.params.get(key), params.get(key));
        if recorded != current {
            warnings.push(format!(
                "warning: seat {i} records {key}={}, but this build derives {key}={} — the seat \
                 still plays, under the CURRENT build",
                shown(recorded),
                shown(current),
            ));
        }
    }
    warnings
}

/// One bot decision: the move, and -- for a contestant that VALUES rather than
/// samples -- everything needed to account for it.
///
/// **`allow(dead_code)` on the accounting half, and it is temporary by
/// construction.** The bot PUMP wants only [`mv`](Self::mv); the reader of
/// everything below it is `why`, task 5 of this same W5 cut. Producing the
/// parts here rather than at that seam is the design's own point (§1.3): one
/// `decide` serves both callers, so `why` cannot reconstruct a decision by a
/// path the pump did not take. The fields ARE read today -- by
/// `each_evaluator_arm_accounts_for_its_move_in_its_own_shape`, which pins the
/// per-arm shape `why` renders off -- but a `cfg(test)` reader does not satisfy
/// the lint in the plain `lib` build, and `expect` is unusable here for the
/// mirror-image reason (it would go unfulfilled in the test build).
#[allow(dead_code)]
pub(crate) struct Decision {
    /// The move the contestant chose.
    pub mv: Move,
    /// `None` for a [`RolloutPolicy`] rung, which draws its move from a
    /// distribution and has no valuation to show. Not an empty explanation: a
    /// rung has no such quantity, which is a different fact from a zero one.
    pub explain: Option<DecisionExplanation>,
}

/// What an evaluator contestant can say about the move it just chose --
/// `suggest`'s [`Suggestion`](crate::suggest::Suggestion) parts, minus the
/// press-specific ones. See [`Decision`] for why the lint is allowed here.
#[allow(dead_code)]
pub(crate) struct DecisionExplanation {
    /// `eval::explain` at the state the value was READ at: the hand-off for a
    /// search, the one-ply successor for greedy.
    pub explanation: Explanation,
    /// The principal continuation, root move first. EMPTY for greedy, which
    /// searches no line at all.
    pub continuation: Vec<Move>,
    /// Every root candidate with its folded value, sorted best first (stable,
    /// so ties keep the candidate scheme's own order). EMPTY for greedy, whose
    /// one-ply argmax reports no per-candidate vector.
    pub ranking: Vec<RootScore>,
    /// The decision's cost counters. `None` for greedy -- no search ran.
    pub stats: Option<SearchStats>,
    /// `MAX_TURN_PLIES` stopped SOME continuation inside this search -- not
    /// necessarily the one in [`continuation`](Self::continuation). See
    /// `Suggestion::search_hit_ply_cap`, whose caveat this shares verbatim.
    pub hit_ply_cap: bool,
}

/// **THE one decision path**, shared by the REPL's bot pump and `why` (design
/// §1.3). `state` is the TRUE state and is read exactly once, on the first
/// line, to take `seat`'s view; nothing below that line reads it again.
///
/// The `Policy` arm is the pre-W5 `HonestChooser` chain moved VERBATIM, so
/// every recorded rung game replays down the same line
/// (`a_policy_seat_decides_byte_identically_to_the_old_chain` holds it to
/// literals captured before the retype).
///
/// The evaluator arms construct the CONCRETE agent exactly as
/// `Contestant::build` does -- same weight table, same constructor, same knob
/// order -- and are pinned equal to it by
/// `decide_matches_contestant_build_for_every_eligible_evaluator_arm`. Why not
/// `Contestant::build`'s own `Box<dyn ArenaAgent>`: a `DecisionReport` is
/// `{ mv, volume }`, so the trait object exposes no explanation accessors at
/// all and `why` could not be written over it.
///
/// **Error classification.** `observe` failing is this driver's own fault
/// ([`BotError::View`], which the pump treats as a lockdown). A REFUSED VIEW
/// classifies as [`BotError::Belief`] (a lockdown) on EVERY arm: the rung arm
/// builds the sampler itself, and the evaluator arms run [`preflight_belief`]
/// before the agent is built -- so the fault class that would mean this driver
/// handed the agent something inconsistent never masquerades as a benign halt
/// (T4 review SHOULD-2; the pre-flight is sound because every `from_view`
/// refusal is structural, checked before the sampler's RNG is touched, so a
/// pre-flight that passes cannot be followed by an agent-side refusal of the
/// same view). The residual, stated rather than hidden and NARROWER than it
/// was: an evaluator agent's remaining errors -- its internal seat/volume/
/// apply contract violations, arriving through an untyped `String` channel
/// this driver will not sniff -- come back as [`BotError::DeadEnd`], a HALT
/// that leaves the session sound and playable, carrying the agent's words
/// verbatim. Neither shape can fall back to privileged play.
/// Pre-flight the belief census an evaluator agent will run inside `choose`,
/// so a refused view classifies as [`BotError::Belief`] here -- the lockdown
/// the rung arm already gives that fault -- instead of arriving through the
/// agent's untyped error channel as a benign dead-end halt (T4 review
/// SHOULD-2, remedy verified there). Sound because every `from_view` refusal
/// is structural, checked before the sampler's RNG is touched, so a
/// pre-flight that passes cannot be followed by an agent-side refusal of the
/// same view. The probe sampler is DISCARDED unsampled: the agent draws its
/// own worlds from its own stream, and this probe moves no stream anywhere.
fn preflight_belief(view: &PlayerView, rules: &Rules, decision_seed: u64) -> Result<(), BotError> {
    BeliefSampler::from_view(view, rules, decision_seed)
        .map(|_| ())
        .map_err(|e| BotError::Belief(e.to_string()))
}

pub(crate) fn decide(
    contestant: Contestant,
    state: &GameState,
    rules: &Rules,
    seat: PlayerId,
    decision_seed: u64,
) -> Result<Decision, BotError> {
    let view = observe(state, seat).map_err(|e| BotError::View(e.to_string()))?;
    match contestant {
        Contestant::Policy(policy) => {
            // Two independent sub-streams off the one decision seed, so the
            // world the bot imagines and the move it draws inside that world
            // are not the same numbers used twice.
            let mut stream = decision_seed;
            let sampler_seed = splitmix64(&mut stream);
            let pick_seed = splitmix64(&mut stream);

            let mut sampler = BeliefSampler::from_view(&view, rules, sampler_seed)
                .map_err(|e| BotError::Belief(e.to_string()))?;
            let world = sampler.sample();
            let handle = DecisionHandle::new(&world, rules);
            let mut rng = ChaCha8Rng::seed_from_u64(pick_seed);
            let mv = policy.pick(&handle, &mut rng).ok_or_else(|| {
                BotError::DeadEnd(format!(
                    "policy {} found nothing at {:?}",
                    policy.as_str(),
                    handle.turn_phase()
                ))
            })?;
            Ok(Decision { mv, explain: None })
        }
        Contestant::GreedyV1 { m } => {
            preflight_belief(&view, rules, decision_seed)?;
            let mut agent = greedy::GreedyAgent::with_weights(rules, WeightTable::V2, m, decision_seed);
            let report = agent.choose(&view, rules).map_err(BotError::DeadEnd)?;
            let explanation = cloned_explanation(agent.last_explanation(), &contestant)?;
            Ok(Decision {
                mv: report.mv,
                explain: Some(DecisionExplanation {
                    explanation,
                    continuation: Vec::new(),
                    ranking: Vec::new(),
                    stats: None,
                    hit_ply_cap: false,
                }),
            })
        }
        Contestant::LookaheadV1 { b, k, m, cap } => {
            preflight_belief(&view, rules, decision_seed)?;
            let mut agent = lookahead::LookaheadAgent::with_knobs(
                rules,
                SearchScheme::TurnBoundary { beam: b },
                k,
                m,
                WeightTable::V2,
                decision_seed,
            )
            .with_cap(cap);
            let report = agent.choose(&view, rules).map_err(BotError::DeadEnd)?;
            let explanation = cloned_explanation(agent.last_explanation(), &contestant)?;
            let mut ranking = agent.last_roots().to_vec();
            // Stable, so tied candidates keep the candidate scheme's own order
            // and two reconstructions of one decision cannot disagree about
            // which tie is printed first (`suggest`'s rule).
            ranking.sort_by_key(|e| std::cmp::Reverse(e.value));
            let stats = agent.last_stats().copied();
            Ok(Decision {
                mv: report.mv,
                explain: Some(DecisionExplanation {
                    explanation,
                    continuation: agent.last_continuation().to_vec(),
                    ranking,
                    hit_ply_cap: stats.is_some_and(|s| s.ply_cap_hits > 0),
                    stats,
                }),
            })
        }
        // Unreachable today, but by CONSTANT rather than by guard: resume is
        // the one narrowed path (`from_provenance` routes through `eligible`),
        // while the wizard hardcodes an eligible const it never re-checks --
        // `the_provisional_default_bot_is_a_contestant_this_driver_runs` is
        // the pin that keeps that true as the const moves (T3 review
        // SHOULD-1). Answered rather than
        // panicked, and as a HALT rather than a lockdown -- a session that
        // somehow acquired a diagnostic seat is misconfigured, not corrupt,
        // and stays playable by hand. Spelled out rather than caught by a
        // `_`, for [`eligible`]'s reason.
        Contestant::MctsV1 { .. } | Contestant::AtomicDiag { .. } => {
            Err(BotError::DeadEnd(diagnostic_refusal(contestant)))
        }
    }
}

/// Spec §0.3(b): an evaluator agent cannot return a move it did not explain, so
/// a missing explanation is a broken contract rather than a state to render
/// around (`suggest`'s precedent, suggest.rs's own `ok_or_else`).
fn cloned_explanation(
    explanation: Option<&Explanation>,
    contestant: &Contestant,
) -> Result<Explanation, BotError> {
    explanation.cloned().ok_or_else(|| {
        BotError::DeadEnd(format!("{contestant:?} returned a move it did not explain"))
    })
}

/// How the REPL asks for a bot's move.
///
/// A trait rather than a free function so the failure shapes can be driven in a
/// test: the honest implementation is supposed to make an illegal proposal
/// unreachable, and a property nothing can exercise is a property nobody can
/// check.
pub trait BotChooser {
    /// One move for `seat`.
    ///
    /// `state` is the TRUE state and is passed for exactly one purpose: to take
    /// the seat's view from it. See the module doc -- the honest implementation
    /// reads it on its first line and never again.
    fn choose(
        &mut self,
        state: &GameState,
        rules: &Rules,
        seat: PlayerId,
        contestant: Contestant,
        decision_seed: u64,
    ) -> Result<Move, BotError>;
}

/// The shipped chooser: [`decide`], of which the pump wants only the move.
pub struct HonestChooser;

impl BotChooser for HonestChooser {
    fn choose(
        &mut self,
        state: &GameState,
        rules: &Rules,
        seat: PlayerId,
        contestant: Contestant,
        decision_seed: u64,
    ) -> Result<Move, BotError> {
        decide(contestant, state, rules, seat, decision_seed).map(|d| d.mv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fw() -> RolloutPolicy {
        RolloutPolicy::FAMILY_WEIGHTED_DEFAULT
    }

    /// The same rung as a SEAT: since the W5 retype a seat holds a
    /// `Contestant`, and a rung seat is `Contestant::Policy(..)`.
    fn fw_seat() -> Contestant {
        Contestant::Policy(fw())
    }

    #[test]
    fn seat_lookup_is_by_player_id_and_a_missing_seat_is_human() {
        let bots = BotSeats::new(vec![None, Some(fw_seat())], 9);
        assert!(!bots.is_bot(PlayerId::new(0)));
        assert!(bots.is_bot(PlayerId::new(1)));
        assert!(!bots.is_bot(PlayerId::new(3)), "a seat past the table is a human, never a phantom bot");
        assert!(bots.any());
        assert!(!BotSeats::none().any());
        assert_eq!(bots.flags(), vec![false, true]);
    }

    /// The decision seed is a pure function of `(recorded seed, event count)`:
    /// the same pair always re-derives the same draw (what makes undo and
    /// resume replay the bot's own line), and different counts diverge.
    ///
    /// **The first assertion is a VALUE, not a self-comparison.** It used to
    /// read `decision_seed(7) == decision_seed(7)`, which cannot fail:
    /// `decision_seed` takes `&self` over a plain `u64` with no interior
    /// mutability, so its purity is a type-system fact rather than a testable
    /// one. What is genuinely worth pinning is the DERIVATION -- a changed
    /// SplitMix64 mix would silently replay a resumed or undone game down a
    /// different line, and nothing else in the crate holds it. Recorded from
    /// the shipped function, not chosen.
    #[test]
    fn decision_seeds_are_pure_and_move_with_the_event_count() {
        let bots = BotSeats::new(vec![Some(fw_seat())], 0xDEAD_BEEF_1234_5678);
        assert_eq!(bots.decision_seed(7), 0xAC19_410F_8F80_1469);
        assert_ne!(bots.decision_seed(7), bots.decision_seed(8));
        let other = BotSeats::new(vec![Some(fw_seat())], 1);
        assert_ne!(bots.decision_seed(7), other.decision_seed(7), "the recorded seed keys the stream");
    }

    /// Provenance round-trips through the record's shape: the names come from
    /// the policy and the human const, the observation label is `view` for
    /// every sim seat, and reading it back reproduces the table.
    #[test]
    fn provenance_round_trips_who_plays_each_seat() {
        let bots = BotSeats::new(vec![None, Some(fw_seat()), None], 0x0102_0304_0506_0708);
        let seats = bots.seat_provenance();
        assert_eq!(seats.len(), 3);
        assert_eq!(seats[0].controller, CONTROLLER_HUMAN);
        assert_eq!(seats[0].version, HUMAN_CONTROLLER_VERSION);
        assert_eq!(seats[0].agent_seed, None, "a human seat has no stream to record");
        assert_eq!(seats[1].controller, fw().as_str());
        assert_eq!(seats[1].version, POLICY_IMPL_VERSION);
        assert_eq!(seats[1].agent_seed, Some(bots.seed()));
        assert_eq!(seats[1].params["retreat_ppm"], 166_667, "the knobs ride the record, never a bare name");
        assert!(seats.iter().all(|s| s.observation == OBSERVATION_VIEW));

        let run = RunProvenance { schema: RUN_PROVENANCE_SCHEMA, master_seed: None, run_id: None, seats };
        let (back, warnings) = BotSeats::from_provenance(Some(&run));
        assert!(warnings.is_empty());
        assert_eq!(back.flags(), bots.flags());
        assert_eq!(back.seed(), bots.seed());
    }

    /// A controller this build cannot run comes back HUMAN with a warning --
    /// never silently as some other bot, and since the W5 retype the warning
    /// carries the PARSE ERROR'S OWN TEXT so it names what was actually seen
    /// rather than only that something was.
    ///
    /// Renamed at W5 (was `an_unknown_controller_resumes_as_a_human_seat_with_a_warning`)
    /// because the added clause is the point of the change.
    #[test]
    fn an_unknown_controller_resumes_as_human_with_the_parse_error_text() {
        let run = RunProvenance {
            schema: RUN_PROVENANCE_SCHEMA,
            master_seed: None,
            run_id: None,
            seats: vec![SeatProvenance {
                controller: "mcts-2027".to_string(),
                version: 4,
                params: std::collections::BTreeMap::new(),
                agent_seed: Some(5),
                observation: OBSERVATION_VIEW.to_string(),
            }],
        };
        let (bots, warnings) = BotSeats::from_provenance(Some(&run));
        assert!(!bots.is_bot(PlayerId::new(0)));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("mcts-2027"), "{}", warnings[0]);
        assert!(warnings[0].contains("HUMAN"), "{}", warnings[0]);
        assert!(
            warnings[0].contains("unknown controller"),
            "the refusal `Contestant::from_parts` produced must reach the reader: {}",
            warnings[0],
        );
        // Produced before there is a session to walk, so the REPL's dash sweep
        // cannot reach it (T11 review F5).
        crate::style::test_support::assert_no_ascii_prose_dash(
            "the unknown-controller warning",
            &warnings.join("\n"),
        );
    }

    /// A 2P state at the first decision point offering MORE THAN ONE move --
    /// the opening additional-action point is forced, so a fixture built there
    /// would make the seed-sensitivity pin below vacuous.
    fn branching_state() -> (GameState, Rules) {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 4).unwrap();
        let mut state =
            finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)]).unwrap();
        for _ in 0..20 {
            let legal = get_legal_moves(&state, &rules);
            if legal.len() > 1 {
                return (state, rules);
            }
            state = apply_move_unchecked(&state, &legal[0], &rules).unwrap();
        }
        panic!("no branching decision point within 20 moves of setup");
    }

    /// Two different recorded stream seeds is not a record this driver wrote;
    /// it keeps one and SAYS so rather than silently playing a seat off a
    /// stream that was never its own.
    ///
    /// **The FIXTURE moved at the W5 retype; the assertions did not.** It used
    /// to hand-build the seat with an EMPTY `params` map, a shape no writer in
    /// this repo produces (`RolloutPolicy::descriptor` always emits
    /// `retreat_ppm` and `recovery_min_of` for the family-weighted rung, which
    /// `provenance_round_trips_who_plays_each_seat` pins by value). Resume now
    /// reconstructs from BOTH stored halves, so a knobless params map is a
    /// refusal ("missing construction key") and that seat would have come back
    /// HUMAN before ever reaching the stream reconciliation this test is about.
    /// Building it through `Contestant::provenance` — the function
    /// `seat_provenance` itself calls — makes the fixture the record shape the
    /// driver actually writes.
    #[test]
    fn conflicting_recorded_stream_seeds_are_reported_not_silently_merged() {
        let bot_seat =
            |seed: u64| fw_seat().provenance(Some(seed)).observed_as(OBSERVATION_VIEW);
        let run = RunProvenance {
            schema: RUN_PROVENANCE_SCHEMA,
            master_seed: None,
            run_id: None,
            seats: vec![bot_seat(11), bot_seat(22)],
        };
        let (bots, warnings) = BotSeats::from_provenance(Some(&run));
        assert_eq!(bots.seed(), 11, "the first recorded stream wins");
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("22") && warnings[0].contains("11"), "{}", warnings[0]);
        crate::style::test_support::assert_no_ascii_prose_dash(
            "the conflicting-stream-seed warning",
            &warnings.join("\n"),
        );
    }

    /// `without_bots` keeps the seat COUNT (a table is still a table) and
    /// names what it dropped.
    #[test]
    fn without_bots_keeps_the_seat_count_and_names_the_dropped_seats() {
        let bots = BotSeats::new(vec![None, Some(fw_seat()), Some(fw_seat())], 7);
        let (plain, dropped) = bots.without_bots();
        assert_eq!(dropped, vec![1, 2]);
        assert!(!plain.any());
        assert_eq!(plain.flags().len(), 3);
    }

    /// The honest chooser produces a move the ENGINE'S OWN legal list contains,
    /// at a real decision point, without ever being shown the true state past
    /// its `observe` call.
    #[test]
    fn the_honest_chooser_picks_a_move_the_engine_offers() {
        let (state, rules) = branching_state();
        let seat = state.current_player;
        let mv = HonestChooser.choose(&state, &rules, seat, fw_seat(), 12_345).expect("a move");
        assert!(
            get_legal_moves(&state, &rules).contains(&mv),
            "the sampled world's pick must be legal in the TRUE state: {mv:?}"
        );
    }

    /// The chain is deterministic in its decision seed, and a different seed
    /// can reach a different move (anti-vacuity for the pin above it -- a
    /// chooser that ignored the seed entirely would pass that one).
    #[test]
    fn the_honest_chooser_is_a_pure_function_of_its_decision_seed() {
        let (state, rules) = branching_state();
        let seat = state.current_player;
        assert!(get_legal_moves(&state, &rules).len() > 1, "fixture: a point with a real choice");
        let a = HonestChooser.choose(&state, &rules, seat, fw_seat(), 77).unwrap();
        let b = HonestChooser.choose(&state, &rules, seat, fw_seat(), 77).unwrap();
        assert_eq!(a, b, "same seed, same move");
        let differs = (0..200u64).any(|s| HonestChooser.choose(&state, &rules, seat, fw_seat(), s).unwrap() != a);
        assert!(differs, "the draw must actually depend on the seed");
    }

    // ---- Phase 3.5c W5 task 3: the four-seam `Contestant` retype ----------
    //
    // The two GOLDENS below were captured by RUNNING THE PRE-RETYPE CODE (a
    // scratch integration test over the shipped `BotSeats`/`HonestChooser`,
    // deleted once its output was read), not by writing down what the new code
    // produces. That is the whole of their value: a literal taken off the new
    // implementation would pin the retype to itself.

    /// Drive `state` forward to a MOVEMENT decision point with a real choice —
    /// the second cell of the drift pin, and a structurally different one from
    /// [`branching_state`]'s opening Placement (route enumeration, a handful of
    /// candidates instead of 186).
    ///
    /// The walk cycles the legal index rather than always taking `legal[0]`,
    /// which loops inside one turn without ever reaching Movement.
    fn movement_state(mut state: GameState, rules: &Rules) -> GameState {
        for step in 0..400usize {
            let legal = get_legal_moves(&state, rules);
            if state.turn_phase == TurnPhase::Movement
                && state.pending_choices.is_empty()
                && legal.len() > 1
            {
                return state;
            }
            state = apply_move_unchecked(&state, &legal[step % legal.len()], rules).unwrap();
        }
        panic!("no branching movement decision point within 400 moves");
    }

    /// **The retype must not move one recorded BYTE for a rung seat.** Design
    /// §1.4: `Contestant::provenance` delegates to `ControllerProvenance::from_policy`
    /// on the `Policy` arm, so every save this driver has ever written keeps
    /// meaning exactly what it meant — and the way to know that is a literal
    /// captured before the edit, not an argument about delegation.
    #[test]
    fn a_rung_seats_provenance_is_byte_identical_across_the_retype() {
        let bots = BotSeats::new(vec![None, Some(fw_seat()), None], 0x0102_0304_0506_0708);
        let json = serde_json::to_string(&bots.seat_provenance()).expect("provenance serializes");
        assert_eq!(
            json,
            "[{\"controller\":\"human\",\"version\":0,\"params\":{},\"observation\":\"view\"},\
             {\"controller\":\"family-weighted\",\"version\":1,\
             \"params\":{\"recovery_min_of\":5,\"retreat_ppm\":166667},\
             \"agent_seed\":72623859790382856,\"observation\":\"view\"},\
             {\"controller\":\"human\",\"version\":0,\"params\":{},\"observation\":\"view\"}]",
        );
    }

    /// **A rung seat decides byte-identically to the pre-retype chain.** The
    /// three moves below came off `HonestChooser` BEFORE `decide` existed, at
    /// the same fixture and the same seeds; they differ from each other, so a
    /// `decide` that ignored its seed could not pass by accident.
    #[test]
    fn a_policy_seat_decides_byte_identically_to_the_old_chain() {
        let (state, rules) = branching_state();
        let seat = state.current_player;
        let goldens = [
            (12_345u64, "{\"PlaceThreeAreas\":{\"placements\":[2,3,9]}}"),
            (77u64, "{\"PlaceThreeAreas\":{\"placements\":[1,3,7]}}"),
            (0xDEAD_BEEFu64, "{\"PlaceThreeAreas\":{\"placements\":[0,6,9]}}"),
        ];
        for (seed, golden) in goldens {
            let decision = decide(fw_seat(), &state, &rules, seat, seed).expect("a move");
            assert_eq!(serde_json::to_string(&decision.mv).unwrap(), golden, "at seed {seed}");
            assert!(
                decision.explain.is_none(),
                "a rung SAMPLES its move; there is no valuation to account for",
            );
        }
    }

    /// **The drift pin (design §1.3).** `decide` builds its own concrete agent
    /// rather than `Contestant::build`'s boxed one — because a `DecisionReport`
    /// carries no explanation and `why` needs one — so the two constructions
    /// can silently come apart (a wrong weight table, a dropped `.with_cap`,
    /// the adjacent `k`/`m` arguments transposed). This pins them equal at
    /// every eligible evaluator arm.
    ///
    /// The capped cell's `cap = 200` is deliberately OUTSIDE the measured
    /// 32-128 non-terminating band (`LookaheadAgent::with_cap`'s doc): the
    /// standing constraint binds fixtures too.
    #[test]
    fn decide_matches_contestant_build_for_every_eligible_evaluator_arm() {
        let (placement, rules) = branching_state();
        let movement = movement_state(placement.clone(), &rules);
        assert_eq!(placement.turn_phase, TurnPhase::Placement, "fixture: two different shapes");
        assert_eq!(movement.turn_phase, TurnPhase::Movement, "fixture: two different shapes");
        let arms = [
            Contestant::GREEDY_V1_DEFAULT,
            Contestant::LOOKAHEAD_V1_DEFAULT,
            Contestant::LookaheadV1 { b: 2, k: 1, m: 4, cap: Some(200) },
        ];
        for state in [&placement, &movement] {
            let seat = state.current_player;
            let view = observe(state, seat).expect("a sim seat's view");
            for arm in arms {
                for seed in [1u64, 7, 0x5EED] {
                    let mine = decide(arm, state, &rules, seat, seed).expect("a decision");
                    let (mut theirs, _handle) = arm.build(&rules, seed);
                    let report = theirs.choose(&view, &rules).expect("the arena agent's own decision");
                    assert_eq!(mine.mv, report.mv, "{arm:?} at seed {seed}");
                    assert!(
                        mine.explain.is_some(),
                        "an evaluator arm accounts for the move it chose: {arm:?}",
                    );
                }
            }
        }
    }

    /// **The accounting SHAPE differs by arm, and the difference is the honest
    /// one rather than a gap.** Greedy takes a one-ply argmax: it explains the
    /// successor it picked and searches no line at all, so an EMPTY
    /// continuation and ranking is what it has to report — `why` renders the
    /// leaner block off exactly this. A search reports the line it valued the
    /// move down (root move first) and every root candidate with it, sorted
    /// best first so two reconstructions of one decision cannot disagree about
    /// which tie prints first.
    #[test]
    fn each_evaluator_arm_accounts_for_its_move_in_its_own_shape() {
        let (state, rules) = branching_state();
        let seat = state.current_player;

        let greedy = decide(Contestant::GREEDY_V1_DEFAULT, &state, &rules, seat, 1)
            .expect("a decision")
            .explain
            .expect("greedy accounts for its move");
        assert!(!greedy.explanation.terms.is_empty(), "the term-by-term breakdown is the whole point");
        assert!(greedy.continuation.is_empty(), "a one-ply argmax searches no line");
        assert!(greedy.ranking.is_empty(), "and reports no per-candidate vector");
        assert!(greedy.stats.is_none(), "no search ran, so there are no search counters");
        assert!(!greedy.hit_ply_cap);

        for arm in [
            Contestant::LOOKAHEAD_V1_DEFAULT,
            Contestant::LookaheadV1 { b: 2, k: 1, m: 4, cap: Some(200) },
        ] {
            let decision = decide(arm, &state, &rules, seat, 1).expect("a decision");
            let e = decision.explain.expect("a search accounts for its move");
            assert!(!e.explanation.terms.is_empty(), "{arm:?}");
            let stats = e.stats.expect("a search reports its counters");
            assert!(stats.scored > 0 && stats.root_candidates > 0, "{arm:?}: {stats:?}");
            assert!(!e.hit_ply_cap, "{arm:?}: the ply guard sits far above any real turn");
            assert_eq!(e.continuation.first(), Some(&decision.mv), "{arm:?}: root move first");
            assert!(
                e.ranking.iter().any(|r| r.mv == decision.mv),
                "{arm:?}: the chosen move is one of the candidates it ranked",
            );
            assert!(
                e.ranking.windows(2).all(|w| w[0].value >= w[1].value),
                "{arm:?}: best first, so a reader can take margins off the head",
            );
        }
    }

    /// **The wizard's default contestant must be one this driver will run** --
    /// the pin [`decide`]'s diagnostic arm names as what keeps its own
    /// unreachability true (T3 review SHOULD-1).
    ///
    /// The wizard hardcodes [`PROVISIONAL_DEFAULT_BOT`] and never re-checks it:
    /// [`eligible`] guards the RESUME path only, so a const re-pointed at an
    /// arm this driver refuses would build a table whose every decision is a
    /// halt, with nothing between the edit and the game to say so. That matters
    /// precisely because the const is PROVISIONAL -- D1's identity freeze will
    /// move it -- so the check belongs to the const, not to the arm it happens
    /// to hold today.
    #[test]
    fn the_provisional_default_bot_is_a_contestant_this_driver_runs() {
        assert_eq!(
            eligible(PROVISIONAL_DEFAULT_BOT),
            Ok(PROVISIONAL_DEFAULT_BOT),
            "the wizard's default must survive the eligibility narrowing it never runs",
        );
    }

    /// A DIAGNOSTIC contestant resumes HUMAN and says why (design §1.2). The
    /// CLI must not become a second surface for arms core labels diagnostic.
    #[test]
    fn a_diagnostic_contestant_resumes_as_human_loudly() {
        let (controller, version, params) = Contestant::MCTS_V1_DEFAULT.descriptor();
        let run = RunProvenance {
            schema: RUN_PROVENANCE_SCHEMA,
            master_seed: None,
            run_id: None,
            seats: vec![SeatProvenance {
                controller,
                version,
                params,
                agent_seed: Some(5),
                observation: OBSERVATION_VIEW.to_string(),
            }],
        };
        let (bots, warnings) = BotSeats::from_provenance(Some(&run));
        assert!(!bots.is_bot(PlayerId::new(0)), "a diagnostic seat is never played by this driver");
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("mcts-v1"), "{}", warnings[0]);
        assert!(warnings[0].contains("diagnostic"), "{}", warnings[0]);
        assert!(warnings[0].contains("HUMAN"), "{}", warnings[0]);
        crate::style::test_support::assert_no_ascii_prose_dash(
            "the diagnostic-contestant warning",
            &warnings.join("\n"),
        );
    }

    /// **Identity drift WARNS per field and never refuses** (design §1.5). The
    /// record's stored version and params are compared against a freshly
    /// derived descriptor, so a cross-build resume (a moved weight table, a
    /// bumped `eval_impl`) is visible without making the save unloadable.
    #[test]
    fn a_version_or_identity_param_mismatch_warns_per_field() {
        let (controller, version, mut params) = Contestant::LOOKAHEAD_V1_DEFAULT.descriptor();
        assert!(params.contains_key("weights"), "fixture: the descriptor carries the identity key");
        let stale = params["weights"].wrapping_add(1);
        params.insert("weights".to_string(), stale);
        let run = RunProvenance {
            schema: RUN_PROVENANCE_SCHEMA,
            master_seed: None,
            run_id: None,
            seats: vec![SeatProvenance {
                controller,
                version: version - 1,
                params,
                agent_seed: Some(5),
                observation: OBSERVATION_VIEW.to_string(),
            }],
        };
        let (bots, warnings) = BotSeats::from_provenance(Some(&run));
        assert_eq!(
            bots.contestant_at(PlayerId::new(0)),
            Some(Contestant::LOOKAHEAD_V1_DEFAULT),
            "drift is loud, never a refusal: the seat still plays, under the CURRENT build",
        );
        assert_eq!(warnings.len(), 2, "one warning per differing field: {warnings:?}");
        assert!(
            warnings.iter().any(|w| w.contains("version") && w.contains(&(version - 1).to_string())),
            "{warnings:?}",
        );
        assert!(
            warnings.iter().any(|w| w.contains("weights") && w.contains(&stale.to_string())),
            "{warnings:?}",
        );
        crate::style::test_support::assert_no_ascii_prose_dash(
            "the identity-drift warnings",
            &warnings.join("\n"),
        );

        // The ABSENT side of the union comparison (T3 review SHOULD-3): a key
        // one side does not emit at all is drift too — the cross-build case
        // the function exists for, an older build emitting a smaller identity
        // set. Reported as `absent`, still never a refusal.
        let (controller, version, mut params) = Contestant::LOOKAHEAD_V1_DEFAULT.descriptor();
        params.remove("weights").expect("fixture: the descriptor carries the identity key");
        let run = RunProvenance {
            schema: RUN_PROVENANCE_SCHEMA,
            master_seed: None,
            run_id: None,
            seats: vec![SeatProvenance {
                controller,
                version,
                params,
                agent_seed: Some(5),
                observation: OBSERVATION_VIEW.to_string(),
            }],
        };
        let (bots, warnings) = BotSeats::from_provenance(Some(&run));
        assert_eq!(
            bots.contestant_at(PlayerId::new(0)),
            Some(Contestant::LOOKAHEAD_V1_DEFAULT),
            "an absent identity key is drift, never a refusal",
        );
        assert_eq!(warnings.len(), 1, "exactly the absent-key warning: {warnings:?}");
        assert!(
            warnings[0].contains("weights") && warnings[0].contains("absent"),
            "the warning names the key and its absence: {warnings:?}",
        );
    }

    /// **Knobs ride the record and come back.** The defect design §1.5 names:
    /// a bare-name reparse of `lookahead-v1` resumes at `b=4` whatever the
    /// record says, so a seat recorded at `b=8` would go on playing a search
    /// the record does not describe. `Contestant::from_parts` reads BOTH stored
    /// halves, and the capped/uncapped pair pins the preserve-list's
    /// absent-cap-is-the-uncapped-identity rule at this seam too.
    #[test]
    fn an_evaluator_seat_resumes_with_its_recorded_knobs_not_the_defaults() {
        let battery = [
            Contestant::LookaheadV1 { b: 8, k: 1, m: 8, cap: None },
            Contestant::LookaheadV1 { b: 2, k: 1, m: 4, cap: Some(200) },
            Contestant::GreedyV1 { m: 3 },
        ];
        for contestant in battery {
            let bots = BotSeats::new(vec![Some(contestant), None], 0x0BAD_F00D_1234_5678);
            let run = RunProvenance {
                schema: RUN_PROVENANCE_SCHEMA,
                master_seed: None,
                run_id: None,
                seats: bots.seat_provenance(),
            };
            let (back, warnings) = BotSeats::from_provenance(Some(&run));
            assert!(warnings.is_empty(), "a same-build round trip is silent: {warnings:?}");
            assert_eq!(back.contestant_at(PlayerId::new(0)), Some(contestant));
            assert_eq!(back.contestant_at(PlayerId::new(1)), None);
            assert_eq!(back.seed(), bots.seed());
        }
    }
}
