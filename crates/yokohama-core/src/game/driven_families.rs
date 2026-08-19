//! Phase C's LAYER-2 READ MACHINERY: turn predicates, the DEAD-root
//! classifier, root admission, and the binary read itself.
//!
//! Design `docs/plans/2026-08-18-phase-c-families-design.md` §§2–4; plan
//! `docs/plans/2026-08-18-phase-c-families-plan.md` §T4. A sibling of
//! [`crate::game::driven`] rather than a part of it, on the same grounds
//! `driven` is a sibling of `arena`: `driven` answers "what happens from this
//! position" and this module answers "does the subject's behaviour at this
//! position satisfy a DECLARED RULE" — two different questions over one
//! instrument, and the rules belong where a reader can read all of them at once.
//!
//! **What a read here IS and IS NOT.** Every rule in this module is a computed
//! statement about STATE — never a move list, never a named line of play. That
//! is the whole reason a pass rule can be frozen before its roots are drawn: a
//! rule shaped like "the mover took move X" is a strategy script that its author
//! can tune after seeing the play, and a rule shaped like "the trigger was set
//! by the hand-off" cannot be.
//!
//! **The four pieces, and how they compose** (design §4's protocol read
//! bottom-up):
//!
//! 1. [`TurnPredicate`] judges ONE (root, hand-off) pair. It is the grain
//!    families 2–4 read at, because canon 15, F8 and the race question are all
//!    statements about what a TURN accomplished rather than about which move
//!    started it.
//! 2. [`classify_dead`] answers family 1's admission question at the DECISION
//!    grain: is this root's candidate field genuinely indifferent, so that a
//!    pick there confirms nothing and fails nothing?
//! 3. [`admission`] answers families 2–4's, at the BEHAVIOUR grain: across the
//!    adjudicator's forced-candidate sweep, do the playouts whose first turn
//!    SATISFIED the predicate reach better margins than the ones that did not?
//!    That comparison is what makes "the pass behaviour is demonstrably
//!    worthwhile HERE" a measurement rather than an assertion.
//! 4. [`family_read`] runs the frozen R replications at one root and reports the
//!    verdicts, the seeds they were taken at, and D7's step-one COUNTER CHECK —
//!    which can REFUSE the whole read (a truncated search's pick is not a
//!    preference).
//!
//! **The admission arithmetic is BEHAVIOUR-vs-BEHAVIOUR, not
//! candidate-vs-candidate**, and the reason is family 4's proximity corner
//! (design §4, review MUST-2): at a proximity root the claim is TWO decisions
//! away, so there is no claim CANDIDATE at the root at all and a candidate-level
//! pass-set would be structurally empty. The comparison that stays computable
//! everywhere is between the PLAYOUTS that ended up doing the thing and the ones
//! that did not — which is also the comparison the canon question actually asks.
//!
//! **What this module deliberately does NOT do**: judge a family. The three-way
//! all-sets/some-sets/no-set rule for `AdjudicatedBest`, the conjunction across
//! the reply menu, the sole-dissenter routing and the VOID-below-4-admitted rule
//! are the FAMILY RUN's (T5) — they are statements about a BLOCK of roots, and
//! this module never sees more than one.

use crate::actions::{
    calculate_church_faith, customs_max_imported_goods, eligible_church_spaces,
    eligible_customs_spaces_for_cost,
};
use crate::game::apply_move::apply_move_unchecked;
use crate::game::arena::{mix, Contestant};
use crate::game::driven::{
    adjudicate_root, adjudicate_root_with, drive_turn, root_table, true_margin_now, Adjudication,
    AdjudicationError, DrivenError, DrivenOpts, NotAContestant, ReplyPolicy, RootDescriptor,
    RootError, RootPredicate, RootSource, RootTable, RootTableError, SeatCounters, SeatSpec,
    TOP_BAND_POINTS,
};
use crate::game::end_game::is_game_over;
use crate::game::invariants::check_invariants;
use crate::game::legal_moves::get_legal_moves;
use crate::game::observe::observe;
use crate::game::rules::Rules;
use crate::game::seeds::{SeedSchedule, WorldSeed};
use crate::game::semantics::state_digest;
use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
use crate::types::{
    AchievementCard, AchievementType, AreaId, AreaType, ChurchTrack, CustomsTrack,
    GameConfig, GameState, Move, PlayerId, PlayerView, PresidentLocation, TechCardId, TechEffect,
    TurnPhase, VisiblePlayer,
};

// ---------------------------------------------------------------------------
// The contested step (design §3, review NOTE-6)
// ---------------------------------------------------------------------------

/// The ONE race step a family-3 root DECLARES, named at generation time and
/// never inferred at read time.
///
/// **Declared, not inferred, and the distinction is the family's honesty.** A
/// read that looked at a hand-off and asked "did the mover take *some* contested
/// step?" would be free to find whichever step the mover happened to take, which
/// is a rule its own subject can satisfy by accident. The builder (or the
/// predicate) names the step when it BUILDS the position, and the read checks
/// that one.
///
/// **FIELD IDENTITIES ARE THE ENGINE'S OWN, and one of them is not what the plan
/// sketch called it.** Both track spaces are named the way every engine surface
/// names them — the way `Move::ChooseChurchSpace`/`Move::ChooseCustomsSpace`
/// carry them, and the way `eligible_church_spaces` /
/// `eligible_customs_spaces_for_cost` return them:
///
/// * a CHURCH space is its FAITH VALUE, `4..=11` (the plan's `faith` — correct);
/// * a CUSTOMS space is its PRINTED SPACE NUMBER, `1..=7` — **not** the `0..=6`
///   slot index the plan sketch named. The slot index exists only inside
///   [`CustomsTrack::space_to_index`], is never carried by a `Move`, and a
///   declaration written in it would be off by one against every other surface
///   in the repo. The field is therefore `space`, not `index`.
///
/// An identity outside its printed range names NO space, so
/// [`ContestedStepTaken`](TurnPredicate::ContestedStepTaken) reads it as NOT
/// HELD. That direction is safe for the positive arm and UNSAFE for the denial
/// arm — [`DenialNotTaken`](TurnPredicate::DenialNotTaken) would pass
/// vacuously — which is why [`well_formed`](Self::well_formed) exists and why a
/// declaration site checks it where the declaration is MADE rather than where it
/// is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContestedStep {
    /// A specific church space, by FAITH VALUE `4..=11` — `ChurchTrack`'s own
    /// identity (`faith_to_index` maps it to the slot).
    ChurchSpace { faith: u8 },
    /// A specific customs space, by PRINTED SPACE NUMBER `1..=7` —
    /// `CustomsTrack`'s own identity (`space_to_index` maps it to the slot).
    CustomsSpace { space: u8 },
    /// The TECHNOLOGY-MAJORITY step (the F6 arithmetic): the mover's DOUBLED
    /// industry value stands strictly past `doubled_threshold`.
    ///
    /// Doubled because that is the unit `calculate_technology_bonus` compares in
    /// — real totals are `2 × Σ industry_cost` and the 2P dummy is exactly 15
    /// (D5's 7.5). STRICTLY past, because the threshold names a level to BEAT:
    /// at the canonical value 15 the step is "outrank the dummy", and real
    /// totals being even makes a tie against it unrepresentable anyway.
    TechMajorityRankPast { doubled_threshold: u32 },
}

impl ContestedStep {
    /// Whether this declaration names a step that EXISTS.
    ///
    /// Checked where a declaration is made (a builder, a predicate), never
    /// relied on at read time: an ill-formed step reads as "not held", which
    /// silently PASSES the denial arm. There is deliberately no clamping and no
    /// typed constructor — the enum stays a plain declaration a freeze document
    /// can quote — so this is the mechanical check that keeps that decision
    /// honest.
    pub fn well_formed(self) -> bool {
        match self {
            Self::ChurchSpace { faith } => ChurchTrack::faith_to_index(faith).is_some(),
            Self::CustomsSpace { space } => CustomsTrack::space_to_index(space).is_some(),
            // Every threshold names a level; 0 is a level everything clears and
            // is a poor declaration, but it is not a malformed one.
            Self::TechMajorityRankPast { .. } => true,
        }
    }

    /// Whether `mover` HOLDS this step in `state`.
    ///
    /// The two track arms read the space's OCCUPANT; the technology arm reads
    /// the mover's own doubled industry value against the declared level. All
    /// three are statements about the mover alone — "held", never "denied to the
    /// opponent" — which is C-A's participation framing: the family's positive
    /// behaviour is TAKING the step, and denial-as-strategy is a claim this
    /// phase does not make.
    pub fn held_by(self, state: &GameState, mover: PlayerId, rules: &Rules) -> bool {
        match self {
            Self::ChurchSpace { faith } => ChurchTrack::faith_to_index(faith)
                .and_then(|i| state.church_track.slots[i])
                .is_some_and(|slot| slot.player == mover),
            Self::CustomsSpace { space } => CustomsTrack::space_to_index(space)
                .and_then(|i| state.customs_track.slots[i])
                .is_some_and(|slot| slot.player == mover),
            Self::TechMajorityRankPast { doubled_threshold } => {
                doubled_industry_value(state, mover, rules) > doubled_threshold
            }
        }
    }
}

/// One player's DOUBLED total industry value — `calculate_technology_bonus`'s
/// own quantity, recomputed here rather than read off a scoring pass.
///
/// Doubling is not decoration: the 2P dummy competes at 7.5 (D5), and the engine
/// represents that half exactly by comparing `2 × Σ industry_cost` against the
/// dummy's 15. A read in undoubled units could not express the dummy's level at
/// all.
///
/// A tech id the registry does not know contributes nothing, which is
/// `calculate_technology_bonus`'s own `filter_map` behaviour — the two must not
/// disagree about what a total is.
fn doubled_industry_value(state: &GameState, player: PlayerId, rules: &Rules) -> u32 {
    let total: u32 = state
        .player(player)
        .technologies
        .iter()
        .filter_map(|&id| rules.techs.get(id))
        .map(|card| u32::from(card.industry_cost))
        .sum();
    2 * total
}

/// The achievement types `player` has claimed in `state`.
///
/// A `Vec` rather than a set because `AchievementType` carries no `Ord` and the
/// board holds exactly THREE distinct achievement cards — so membership over a
/// three-element vector IS set membership, at no cost worth a new trait bound.
fn claimed_types(state: &GameState, player: PlayerId) -> Vec<AchievementType> {
    state
        .achievements
        .iter()
        .filter(|card| card.claimed_by.contains(&player))
        .map(|card| card.achievement_type)
        .collect()
}

// ---------------------------------------------------------------------------
// The turn predicates (design §3)
// ---------------------------------------------------------------------------

/// A frozen statement about what ONE TURN accomplished — the pass rules families
/// 2–4 read at (design §3, stated exactly).
///
/// Every arm is evaluated against the pair `(root, hand-off)` and nothing else:
/// the root is where the mover's turn began and the hand-off is where it ended
/// (`current_player` moved on, or the game ended). Nothing here reads the move
/// list, and that is the point — a mover who banks its area actions first and
/// fires the trigger with its last action played CORRECTLY, and a rule that
/// named a move would score that correct turn as a failure.
///
/// **`rules` is a parameter and the plan sketch did not have it.** The
/// technology arm of [`ContestedStep`] compares DOUBLED INDUSTRY VALUE, which is
/// `Σ industry_cost` over the mover's owned techs — and `industry_cost` lives in
/// the tech REGISTRY, not in `GameState` (a `GameState` carries `TechCardId`s).
/// A two-argument `eval` therefore cannot compute its own third arm. This is the
/// same widening, for the same reason, that T1 applied to `RootPredicate::holds`
/// one task earlier, and it changes no declared SEMANTICS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnPredicate {
    /// The end-game trigger is SET at the hand-off — family 2's positive rule.
    ///
    /// The root guaranteed `!end_game_triggered` (its generator's own conjunct),
    /// so a set flag at the hand-off is the MOVER'S OWN turn having fired it.
    /// Read as a plain state predicate anyway rather than as a delta, because
    /// that guarantee belongs to the generator and this rule must be readable
    /// against any pair a caller hands it.
    EndFired,
    /// The negation — family 2's CONTROL rule (canon 15: a behind mover keeps
    /// building rather than ending the game).
    NotEndFired,
    /// The MOVER's claimed-achievement set at the hand-off STRICTLY CONTAINS its
    /// set at the root — family 4's rule (ruling 14, the agenda-setters).
    ///
    /// Strict containment rather than a bare size compare: an implementation
    /// reading only the count would call a set that lost one claim and gained
    /// another "grown", and while the engine never un-claims, a rule that
    /// depends on that is a rule that breaks silently if it ever does.
    AchievementClaimed,
    /// The DECLARED contested step is HELD by the mover at the hand-off —
    /// family 3's positive rule (C-A's participation framing).
    ContestedStepTaken(ContestedStep),
    /// The DECLARED denial-shaped step is NOT held by the mover at the hand-off
    /// — family 3's negative control (canon 12/13: denial is not a strategy).
    DenialNotTaken(ContestedStep),
}

impl TurnPredicate {
    /// Judge one turn: `root` is where it began, `handoff` where it ended.
    ///
    /// **The MOVER is `root.current_player`** — the seat whose turn this was —
    /// and never `handoff.current_player`, which is by definition somebody else
    /// at every hand-off that is not a game end. Reading the mover off the
    /// hand-off would judge the OPPONENT's achievements and the OPPONENT's track
    /// spaces at exactly the positions the family cares about.
    pub fn eval(&self, root: &GameState, handoff: &GameState, rules: &Rules) -> bool {
        let mover = root.current_player;
        match self {
            Self::EndFired => handoff.end_game_triggered,
            Self::NotEndFired => !handoff.end_game_triggered,
            Self::AchievementClaimed => {
                let before = claimed_types(root, mover);
                let after = claimed_types(handoff, mover);
                after.len() > before.len() && before.iter().all(|t| after.contains(t))
            }
            Self::ContestedStepTaken(step) => step.held_by(handoff, mover, rules),
            Self::DenialNotTaken(step) => !step.held_by(handoff, mover, rules),
        }
    }

    /// The stable name, for reports and freeze documents.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EndFired => "end-fired-by-handoff",
            Self::NotEndFired => "not-end-fired-by-handoff",
            Self::AchievementClaimed => "achievement-claimed-by-handoff",
            Self::ContestedStepTaken(_) => "contested-step-taken-by-handoff",
            Self::DenialNotTaken(_) => "denial-not-taken-by-handoff",
        }
    }
}

/// A family's declared PASS RULE — one per family, frozen with it (design §2).
///
/// The two arms are the two READ GRAINS, and a family picks exactly one:
/// [`Turn`](Self::Turn) drives the mover's whole turn and judges the hand-off;
/// [`AdjudicatedBest`](Self::AdjudicatedBest) tabulates the ROOT DECISION and
/// judges the pick against an adjudicated top set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassRule {
    /// TURN grain (families 2–4).
    Turn(TurnPredicate),
    /// DECISION grain (family 1): the pick must land within `epsilon` of the
    /// best adjudicated mean margin.
    ///
    /// **`epsilon` is carried, not applied, by [`family_read`]** — the top set
    /// is a fact about an ADJUDICATION under one reply policy, the three-way
    /// all-sets/some-sets/no-set judgment is a fact about the whole declared
    /// MENU, and neither is available at the grain of one replication's table.
    /// The read reports the picks; T5's family run judges them. Leniency
    /// direction, stated once: a LARGER epsilon is an EASIER pass.
    AdjudicatedBest { epsilon: i64 },
}

impl PassRule {
    /// Which grain this rule reads at.
    pub fn grain(&self) -> ReadGrain {
        match self {
            Self::Turn(_) => ReadGrain::Turn,
            Self::AdjudicatedBest { .. } => ReadGrain::Decision,
        }
    }

    /// Whether this rule names a [`ContestedStep`] — i.e. whether the ROOT owes
    /// it one.
    pub fn names_a_step(self) -> bool {
        matches!(
            self,
            Self::Turn(TurnPredicate::ContestedStepTaken(_) | TurnPredicate::DenialNotTaken(_))
        )
    }

    /// Re-point a step-carrying rule at the step THIS ROOT declares (design §3:
    /// the step is "declared by the builder or predicate at generation time,
    /// never inferred at read time").
    ///
    /// A declaration writes its rule with a placeholder
    /// ([`DECLARED_BY_THE_ROOT`]) because the step is a fact about a root and a
    /// declaration is written before any root is drawn. A rule that names no
    /// step is returned unchanged, and so is a step-carrying rule handed `None`
    /// — the caller checks [`names_a_step`](Self::names_a_step) first and
    /// refuses, so nothing is ever READ against the placeholder.
    pub fn with_step(self, step: Option<ContestedStep>) -> Self {
        match (self, step) {
            (Self::Turn(TurnPredicate::ContestedStepTaken(_)), Some(s)) => {
                Self::Turn(TurnPredicate::ContestedStepTaken(s))
            }
            (Self::Turn(TurnPredicate::DenialNotTaken(_)), Some(s)) => {
                Self::Turn(TurnPredicate::DenialNotTaken(s))
            }
            (other, _) => other,
        }
    }
}

/// The two read grains (design §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadGrain {
    /// The mover's whole turn, judged at the hand-off.
    Turn,
    /// The root decision alone, judged by the pick.
    Decision,
}

impl ReadGrain {
    /// The stable name, for reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Turn => "turn",
            Self::Decision => "decision",
        }
    }
}

// ---------------------------------------------------------------------------
// The DEAD-root classifier (design §1, review MUST-1)
// ---------------------------------------------------------------------------

/// The declared mean-margin SPREAD, in victory points, below which a winless
/// candidate field counts as indifferent.
///
/// **4 VP, and the number is the D7 triage §3's own measured dead-field spread**
/// rather than a taste. It exists because the obvious criterion —
/// `Adjudication::all_candidates_equal`, elementwise equality of the margin
/// VECTORS — classified all three of the session's measured dead roots as LIVE:
/// they read 15, 11 and 2 DISTINCT means over fields that no candidate ever won.
/// Feeding those into a CONFIRMATORY denominator would have made the family read
/// keyed-draw coin flips as preferences, which is the healthy-subject-fails
/// channel this bound closes.
pub const DEAD_SPREAD_BOUND: i64 = 4;

/// Is this root's candidate field genuinely indifferent (design §1's measured
/// shape)?
///
/// DEAD iff **no candidate won ANY replication** and the whole-field mean-margin
/// spread (`max mean − min mean`) is within `spread_bound`. A dead root is
/// honest indifference: it confirms nothing and fails nothing, so family 1
/// EXCLUDES it from the pass/fail denominator rather than counting it either
/// way.
///
/// **Wins, not margins, carry the first half.** A field where one candidate wins
/// and the rest lose by a point is a field where the choice MATTERS, and its
/// mean spread can be tiny — `won` is `final_result`'s own verdict with the
/// clockwise tie-break folded in, which a signed margin cannot express (margin 0
/// is not a win).
///
/// **An EMPTY table is NOT dead, and the distinction is a trap worth stating.**
/// A table with no candidates is a FINISHED GAME, not an indifferent position:
/// the winless half is vacuously true and the spread is undefined, so a `true`
/// here would quietly launder "the game was over" into "the mover's options did
/// not matter". A caller separates the two with `Adjudication::candidates()`,
/// exactly as `all_candidates_equal`'s own doc requires. A ONE-ROW table (a
/// FORCED decision) does classify dead by the literal rule and should: nothing
/// the mover could choose mattered, because it had nothing to choose.
///
/// [`Adjudication::all_candidates_equal`] stays the STRICTER fact printed beside
/// this classification — it is what "the candidates are literally
/// indistinguishable" means, and it is a different (and rarer) statement.
pub fn classify_dead(adj: &Adjudication, spread_bound: i64) -> bool {
    if adj.rows.is_empty() {
        return false;
    }
    if adj.rows.iter().any(|row| row.wins() > 0) {
        return false;
    }
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for mean in adj.rows.iter().map(|row| row.mean_margin()) {
        lo = lo.min(mean);
        hi = hi.max(mean);
    }
    hi - lo <= spread_bound as f64
}

/// DEAD across the WHOLE declared menu: [`classify_dead`] per policy, AND-ed
/// (design §4's NOTE-2 — a root the reply menu disagrees about is not
/// indifferent).
///
/// **Split out of `run_one_root` for [`family_verdict`]'s reason** (T5 review
/// MUST-1): the AND lived inline in the run, and every block a test can afford
/// declares a SINGLE-POLICY menu — where AND and OR are the same function, so
/// mutating one into the other passed the whole library. As its own function the
/// MIXED vectors are one call each, and they are what this rule is about.
///
/// **An EMPTY vector is NOT dead** — deliberately, and not the vacuous truth
/// `all` would give: no policy adjudicated the root, so nothing found it
/// indifferent, and a `true` here would exclude every root of a menu-less
/// declaration as dead. It is the same trap [`classify_dead`] states for an
/// empty candidate table, one level up.
fn dead_across_menu(dead_by_policy: &[(ReplyPolicy, bool)]) -> bool {
    !dead_by_policy.is_empty() && dead_by_policy.iter().all(|(_, dead)| *dead)
}

/// How many candidates a DECISION-grain root must offer before its pick can be
/// read as a preference at all.
///
/// **Two, and the pilot is why the constant exists** (the freeze wave). A
/// family-1 pilot root adjudicated ONE candidate, was admitted LIVE and read
/// PASS — vacuously, because the subject's pick was the only move there was and
/// was therefore inside every adjudicated top set by construction.
pub const FORCED_CANDIDATE_FLOOR: usize = 2;

/// Is this root's candidate field FORCED — fewer than [`FORCED_CANDIDATE_FLOOR`]
/// candidates?
///
/// **A different fact from [`classify_dead`], and it must be checked FIRST.**
/// Dead means "the mover had a choice and it did not matter"; forced means
/// "there was no choice". A lone candidate that WINS its replications is `live`
/// by `classify_dead`'s wins clause — the exact shape the pilot hit — so the
/// dead classifier cannot be asked to carry this.
///
/// The EMPTY table (a finished game) rides the same arm deliberately: it offers
/// no choice either, and both are positions a read confirms nothing and fails
/// nothing at. What the two are NOT is interchangeable with dead, which is why
/// the exclusion reason names the candidate count.
///
/// **Applied at the DECISION grain only, and the scope is the point.** A
/// decision-grain read judges the ROOT PICK, which at a forced root is not a
/// preference. A turn-grain read judges what the whole TURN accomplished, and a
/// turn continues past a forced first decision — so a forced root is still a
/// real reading there, and the degenerate shapes that DO defeat a turn-grain
/// read are the ones [`admission`]'s splitting rules answer.
pub fn classify_forced(adj: &Adjudication) -> bool {
    adj.candidates() < FORCED_CANDIDATE_FLOOR
}

/// FORCED across the declared MENU: the smallest candidate count, when it is
/// below the floor.
///
/// The count is a fact about the ROOT (the adjudicator forces the engine's own
/// legal list, and the reply seats decide only the remainder), so every policy
/// sees the same number and the minimum is that number. Taking the MINIMUM
/// rather than a unanimity is the conservative reading: one policy seeing a
/// forced field is a forced field.
///
/// **An EMPTY vector is NOT forced**, on [`dead_across_menu`]'s reasoning: no
/// policy adjudicated the root, so nothing found it forced.
fn forced_across_menu(candidates_by_policy: &[(ReplyPolicy, usize)]) -> Option<usize> {
    let smallest = candidates_by_policy.iter().map(|(_, n)| *n).min()?;
    (smallest < FORCED_CANDIDATE_FLOOR).then_some(smallest)
}


// ---------------------------------------------------------------------------
// Root admission (design §4, review MUST-2/MUST-4)
// ---------------------------------------------------------------------------

/// Why one reply policy's adjudication could DEMONSTRATE nothing about a root.
///
/// Typed rather than a bare "no": each arm routes differently. Three are facts
/// about the POSITION (the playouts all did the thing, none of them did, or one
/// side is too thin for its mean to be a measurement), and one is a fact about
/// the CALL (the adjudication carried no verdicts at all, so it was run without
/// design §2's turn hook and this is a caller error rather than a reading).
///
/// **The three POSITION-shaped arms are RECORDED and stop BLOCKING** (the freeze
/// wave): a policy that demonstrated nothing is evidence about that policy at
/// that root, not a veto over the ones that did demonstrate something.
/// [`Unstamped`](Self::Unstamped) is the exception and keeps its veto, because
/// it is not a fact about the position at all — it says the run was mis-wired,
/// and a mis-wired run must not produce an admission. See
/// [`AdmissionRead::admitted`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CannotDemonstrate {
    /// No playout satisfied the predicate — there is no TRUE group to compare.
    NoTrueGroup,
    /// Every playout satisfied it — there is no FALSE group to compare.
    NoFalseGroup,
    /// Both groups exist and one of them holds fewer than [`SPLIT_MIN_GROUP`]
    /// playouts, so its "group mean" is one game's noise (the freeze wave: the
    /// pilot's family-weighted rung split 1-of-8 and that single playout then
    /// decided δ for the whole root).
    ThinSplit,
    /// At least one playout carries no verdict: the adjudication was produced by
    /// `adjudicate_root` (or `adjudicate_root_with(.., None)`) rather than with
    /// the family's turn hook. Never silently treated as `false` — that would
    /// put every unstamped playout in the losing group and manufacture a
    /// dominance out of plumbing.
    Unstamped,
}

impl CannotDemonstrate {
    /// The sentence a report prints.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoTrueGroup => "no playout's first turn satisfied the predicate",
            Self::NoFalseGroup => "every playout's first turn satisfied the predicate",
            Self::ThinSplit => {
                "one group holds fewer than two playouts, so its mean is one game's noise"
            }
            Self::Unstamped => {
                "the adjudication carries no turn verdicts (it was run without the turn hook)"
            }
        }
    }
}

/// The smallest a group may be and still have a MEAN worth comparing (the freeze
/// wave, pilot-informed).
///
/// **Two playouts a side.** The pilot's family-weighted rung split ACCIDENTALLY
/// — 1 of 8 true groups — and the resulting one-game "group mean" was then
/// compared to δ as though it were a measurement. A policy below this floor is
/// [`CannotDemonstrate::ThinSplit`]: recorded, and non-blocking.
pub const SPLIT_MIN_GROUP: usize = 2;

/// How many SPLITTING policies a root needs before its admission is a
/// demonstration (the freeze wave).
///
/// **Two.** One splitting policy is one reply's measurement, and a root admitted
/// on it would be a root admitted on a single opponent model's taste. Below the
/// floor the root is excluded with the count named — a different finding from
/// "the pass behaviour failed here".
pub const MIN_SPLITTING_POLICIES: usize = 2;

/// One reply policy's admission verdict at one root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionVerdict {
    /// The pass behaviour DOMINATES here by the declared margin — the root
    /// supports the family's question under this policy.
    Dominates,
    /// Both groups exist and the declared margin was not reached. A real
    /// measurement, and a different finding from [`CannotDemonstrate`].
    Fails,
    /// Nothing could be demonstrated — see [`CannotDemonstrate`].
    Cannot(CannotDemonstrate),
}

/// One reply policy's admission arithmetic at one root.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolicyAdmission {
    pub policy: ReplyPolicy,
    /// Playouts whose first turn satisfied the predicate.
    pub true_n: usize,
    /// Playouts whose first turn did not.
    pub false_n: usize,
    /// Mean MARGIN of the TRUE group — `0.0` on an empty group, which the
    /// verdict already refuses to read.
    pub true_mean: f64,
    /// Mean MARGIN of the FALSE group.
    pub false_mean: f64,
    /// `true_mean − false_mean`, the quantity the declared δ is compared to.
    pub difference: f64,
    pub verdict: AdmissionVerdict,
}

impl PolicyAdmission {
    /// Whether this policy admitted the root.
    pub fn dominates(&self) -> bool {
        matches!(self.verdict, AdmissionVerdict::Dominates)
    }

    /// Whether this policy SPLIT the root — both groups exist and both hold at
    /// least [`SPLIT_MIN_GROUP`] playouts, so the comparison it reports is a
    /// measurement (the freeze wave).
    ///
    /// Read off the VERDICT rather than off the counts, deliberately: the two
    /// would then be two spellings of one rule that could drift apart, and the
    /// unstamped arm (which can carry two full groups and still be a caller
    /// error) would need restating here. `Dominates` and `Fails` are exactly the
    /// verdicts [`admission`] produces from a real two-sided comparison.
    pub fn splitting(&self) -> bool {
        matches!(self.verdict, AdmissionVerdict::Dominates | AdmissionVerdict::Fails)
    }

    /// Whether this policy's adjudication was run WITHOUT the family's turn hook
    /// — a caller error rather than a reading, and the one
    /// [`CannotDemonstrate`] arm that still refuses the whole root.
    pub fn unstamped(&self) -> bool {
        self.verdict == AdmissionVerdict::Cannot(CannotDemonstrate::Unstamped)
    }
}

/// One root's admission read across the declared reply menu (design §4).
#[derive(Debug, Clone, PartialEq)]
pub struct AdmissionRead {
    /// The declared dominance margin, in victory points.
    pub delta: i64,
    /// A CONTROL root inverts the test: the pass behaviour must demonstrably
    /// LOSE by δ (family 2's behind-mover control — canon 15's "ending the game
    /// when you know you're going to lose is a bad idea").
    pub control: bool,
    /// One entry per policy, in the order the caller supplied them.
    pub policies: Vec<PolicyAdmission>,
}

impl AdmissionRead {
    /// The policies that SPLIT — the ones whose comparison is a measurement at
    /// all (the freeze wave).
    pub fn splitting(&self) -> impl Iterator<Item = &PolicyAdmission> {
        self.policies.iter().filter(|p| p.splitting())
    }

    /// How many policies split. The number an exclusion NAMES when it falls
    /// below [`MIN_SPLITTING_POLICIES`].
    pub fn splitting_count(&self) -> usize {
        self.splitting().count()
    }

    /// The CONJUNCTION, taken over the SPLITTING policies: the root is admitted
    /// only if at least [`MIN_SPLITTING_POLICIES`] of them split and EVERY
    /// splitting one demonstrates dominance.
    ///
    /// **The freeze wave narrowed the conjunction's domain and nothing else.**
    /// It used to run over the whole declared menu, and the pilot showed what
    /// that costs: greedy never claims within a turn at a proximity root
    /// (all-FALSE) and the race-rusher claims near-universally (all-TRUE), so
    /// two arms that had no comparison in them vetoed every root the other two
    /// could have demonstrated — families 3 and 4 admitted 0 of 4 everywhere.
    /// A policy that demonstrated NOTHING now blocks nothing; a policy that
    /// demonstrated a FAILURE still blocks, which is the half that is not
    /// relaxed.
    ///
    /// **[`CannotDemonstrate::Unstamped`] is the one arm that still vetoes**, and
    /// it is deliberately NOT relaxed with the other two: it says the
    /// adjudication was run without the family's turn hook, which is a fact
    /// about the CALL. A mis-wired run must not be able to admit a root off the
    /// policies that happened to be wired correctly. (`run_one_root` always
    /// passes the hook at the turn grain, so this guards a direct caller.)
    ///
    /// Vacuously false on an empty menu, and false below the splitting floor —
    /// an admission "across no policies", or across one, is not a demonstration.
    pub fn admitted(&self) -> bool {
        if self.policies.iter().any(PolicyAdmission::unstamped) {
            return false;
        }
        self.splitting_count() >= MIN_SPLITTING_POLICIES
            && self.splitting().all(PolicyAdmission::dominates)
    }

    /// The SPLITTING policies that did not demonstrate dominance, in menu order.
    ///
    /// **Splitting only** (the freeze wave): a policy that could demonstrate
    /// nothing did not dissent from anything, and listing it as a dissenter
    /// would route a degenerate shape to D7 as a disagreement.
    pub fn dissenters(&self) -> Vec<ReplyPolicy> {
        self.splitting()
            .filter(|p| !p.dominates())
            .map(|p| p.policy)
            .collect()
    }

    /// **The ADMISSION-CIRCULARITY flag** (design §4, review MUST-4): dominance
    /// held under every declared policy EXCEPT `lookahead-at-final`.
    ///
    /// The subject IS (provisionally) `lookahead@final`, so a subject defect that
    /// degrades its POST-candidate play deflates measured dominance under
    /// exactly that arm — and would therefore exclude exactly the roots where
    /// the defect matters, a broken subject passing by shrinking its own
    /// denominator. Such an exclusion is COLLECTED as potential
    /// subject-suppression evidence and routed to D7 as a finding, never
    /// silently dropped.
    ///
    /// Requires at least two SPLITTING policies — with `lookahead-at-final`
    /// alone among them there is no "the other policies held" half to observe.
    /// The freeze wave re-keyed that requirement from the MENU to the splitting
    /// policies, on this accessor's own logic: a policy that demonstrated
    /// nothing is not evidence that anything held.
    pub fn lookahead_sole_dissenter(&self) -> bool {
        self.splitting_count() >= 2
            && self.dissenters().as_slice() == [ReplyPolicy::LookaheadAtFinal]
    }
}

/// Design §4's ROOT ADMISSION: does the pass BEHAVIOUR demonstrably dominate at
/// this root, under each declared reply policy?
///
/// For each adjudication, every playout of every candidate is partitioned by its
/// [`CandidatePlayout::turn_verdict`](crate::game::driven::CandidatePlayout::turn_verdict)
/// — the stamp `adjudicate_root_with` writes when the family's turn hook rides
/// the sweep — and the two groups' MEAN MARGINS are compared:
///
/// * positive roots admit when `true_mean − false_mean >= delta`;
/// * CONTROL roots admit when `true_mean − false_mean <= −delta`.
///
/// **Both groups must hold at least [`SPLIT_MIN_GROUP`] playouts**, else
/// [`CannotDemonstrate`]: a root where every playout fires (or none does) has no
/// comparison in it, and a root where one side is a single game has a comparison
/// whose "group mean" is that game's own margin. Both are recorded — with their
/// typed reason — rather than counted either way, and neither blocks the
/// policies that did demonstrate something ([`AdmissionRead::admitted`]).
///
/// **MARGINS, never wins.** The 10–10 shape is the trap: two playouts level on
/// VP have `won` split by the clockwise tie-break while both carry margin 0, so
/// a group statistic built on `won` measures the tie-break and a group statistic
/// built on margins measures the position. δ is declared in victory points, so
/// margins are the only unit that can be compared to it at all.
///
/// The conjunction across policies, the sole-dissenter routing and the VOID
/// rule are [`AdmissionRead`]'s accessors and T5's assembly — this function
/// measures one root and judges nothing above it.
pub fn admission(
    adjs: &[(ReplyPolicy, Adjudication)],
    delta: i64,
    control: bool,
) -> AdmissionRead {
    let policies = adjs
        .iter()
        .map(|(policy, adj)| {
            let mut trues: Vec<i64> = Vec::new();
            let mut falses: Vec<i64> = Vec::new();
            let mut unstamped = false;
            for playout in adj.rows.iter().flat_map(|row| row.playouts.iter()) {
                match playout.turn_verdict {
                    Some(true) => trues.push(playout.margin),
                    Some(false) => falses.push(playout.margin),
                    None => unstamped = true,
                }
            }
            let true_mean = mean(&trues);
            let false_mean = mean(&falses);
            let difference = true_mean - false_mean;
            let verdict = if unstamped {
                AdmissionVerdict::Cannot(CannotDemonstrate::Unstamped)
            } else if trues.is_empty() {
                AdmissionVerdict::Cannot(CannotDemonstrate::NoTrueGroup)
            } else if falses.is_empty() {
                AdmissionVerdict::Cannot(CannotDemonstrate::NoFalseGroup)
            } else if trues.len() < SPLIT_MIN_GROUP || falses.len() < SPLIT_MIN_GROUP {
                // THE THIN SPLIT (the freeze wave): both groups exist, and one
                // of them is a single game whose margin would otherwise BE the
                // group mean δ is compared to.
                AdmissionVerdict::Cannot(CannotDemonstrate::ThinSplit)
            } else {
                let threshold = delta as f64;
                let met =
                    if control { difference <= -threshold } else { difference >= threshold };
                if met {
                    AdmissionVerdict::Dominates
                } else {
                    AdmissionVerdict::Fails
                }
            };
            PolicyAdmission {
                policy: *policy,
                true_n: trues.len(),
                false_n: falses.len(),
                true_mean,
                false_mean,
                difference,
                verdict,
            }
        })
        .collect();
    AdmissionRead { delta, control, policies }
}

/// The arithmetic mean, `0.0` on an empty slice.
///
/// An empty group's mean is never read for a verdict — [`admission`] refuses
/// before it gets there — so a stated constant beats an `Option` that every
/// caller would unwrap.
fn mean(values: &[i64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<i64>() as f64 / values.len() as f64
}

// ---------------------------------------------------------------------------
// The binary read (design §2's `family_read`, §4's seeding)
// ---------------------------------------------------------------------------

/// Domain salt for a FAMILY READ's per-replication decision seed (design §2).
///
/// **The derivation, stated where the constant is**:
/// `mix(SALT_DRIVEN_FAMILY_READ, [master, root_word, replication])`, where
/// `root_word` is [`root_word`]'s value — the root's own state digest read as
/// the `u64` it encodes. Distinct roots therefore draw DISTINCT streams from one
/// recorded master, which is what lets a whole family block ride a single
/// recorded number without two roots secretly sharing worlds.
///
/// **Serial provenance (the A3E1_D21E family)**: this family's HOME is
/// `arena::SALT_DRIVEN_REPLICATION` (serial 0006, whose own doc records 0004 and
/// 0005 as taken elsewhere); serial **0007** is `driven`'s `SALT_DRIVEN_RACE_PROBE`
/// (Phase C's T3 — first consumer keeps the serial, orchestrator ruling at the
/// T3 adjudication), so this is **0008**. The non-collision is pinned
/// arithmetically against the family's home rather than by eyeballing the
/// literals.
pub const SALT_DRIVEN_FAMILY_READ: u64 = 0xA3E1_D21E_0000_0008;

/// A root's identity as ONE WORD, for the seed derivation.
///
/// `state_digest` is `format!("{:016x}", fnv1a_64(..))` — **exactly sixteen hex
/// characters encoding one `u64`**, so the WHOLE value is the word and there is
/// no prefix to take. (An earlier sketch said "the digest's first 8 bytes",
/// which would have been the first 8 HEX CHARACTERS — the digest's top 32 bits —
/// halving the space for no reason.)
///
/// The `expect` is a statement about `state_digest`'s own format, not about the
/// state: any `GameState` that serializes produces sixteen hex characters, and a
/// state that does not serialize could not have reached this function.
pub fn root_word(root: &GameState) -> u64 {
    let digest = state_digest(root);
    u64::from_str_radix(&digest, 16)
        .expect("state_digest is sixteen hex characters encoding one u64")
}

/// The decision seed replication `r` of a read at `root` is taken at.
///
/// THE one place this derivation lives — T7's separator feeds a failing
/// replication's seed straight to `driven table … --decision-seed`, and a
/// second copy of the formula anywhere is a chance for those two to disagree.
pub fn replication_seed(master: u64, root_word: u64, r: u32) -> u64 {
    mix(SALT_DRIVEN_FAMILY_READ, &[master, root_word, u64::from(r)])
}

/// What ONE replication of a read saw.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplicationRead {
    /// Its index, `0..R`.
    pub replication: u32,
    /// The seed it was taken at — [`replication_seed`]'s value, RECORDED rather
    /// than left to be re-derived (design §2, review SHOULD-4: T7's separator
    /// step feeds this straight to `table --decision-seed`).
    pub decision_seed: u64,
    /// The root's own word, verbatim, so a recorded replication carries both
    /// halves of its own derivation.
    pub root_word: u64,
    /// TURN grain: whether the family's predicate held at the hand-off.
    /// `None` at decision grain, where there is no turn to judge.
    pub verdict: Option<bool>,
    /// DECISION grain: the subject's pick. `None` at turn grain — the read
    /// reports the turn, and its first move is already the turn's first entry.
    pub pick: Option<Move>,
    /// TURN grain: how many decisions the turn took. 0 at decision grain.
    pub turn_moves: u64,
    /// TURN grain: the hand-off's digest. `None` at decision grain.
    pub handoff_digest: Option<String>,
    /// TURN grain: whether the turn ENDED THE GAME rather than handing off.
    /// `None` at decision grain.
    pub game_over: Option<bool>,
    /// TURN grain: the mover's WHOLE-TURN counters (design §2's SHOULD-6 — a
    /// mid-turn cap hit must reach D7's step-one filter). [`SeatCounters::NONE`]
    /// at decision grain, where the table's own `SearchStats` is the account.
    pub counters: SeatCounters,
}

/// A counter observation that ANNOTATES a read without refusing it (design
/// §5.4's split, review NOTE-5).
///
/// These are the counters that say the evaluator SEPARATED LITTLE, not that the
/// search was TRUNCATED. A tie-mass root is still a real reading — family 3's
/// expected tie-mass positives must produce their gap output — it just has to go
/// through the mechanical separator (b=4 reproduce, b=8/b=16) before an argument
/// rides it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterAnnotation {
    /// More than one candidate tied AT the best value.
    TiedAtTop { candidates: usize },
    /// The top BAND is wider than the tie — the e113 shape, where the evaluator
    /// separated almost nothing while few candidates were literally equal.
    TopBand { candidates: usize, points: i64 },
    /// A node's placement family exceeded the candidate cap.
    ///
    /// Deliberately an ANNOTATION and not a refusal, and the reason is stated so
    /// the choice is visible: design §5.4 enumerates the hard-refuse counters as
    /// ply/free-cap hits, budget binds and starved decisions, and
    /// `SeatCounters` — the turn grain's own account — does not carry this one
    /// at all. Refusing on it would make the two counter sources of one read
    /// disagree about what a refusal is.
    PlacementCapBinds { count: u64 },
}

/// D7's STEP-ONE COUNTER CHECK, as a value (design §2, §5.4).
#[derive(Debug, Clone, PartialEq)]
pub struct CounterCheck {
    /// The FULL fifteen-counter table at replication 0's seed — the whole
    /// `SearchStats` block plus the ranking's shape, which is why this is a
    /// `RootTable` rather than a summary.
    pub table: RootTable,
    /// Candidates tied at the best value.
    pub tied_at_top: usize,
    /// Candidates within [`TOP_BAND_POINTS`] of the best.
    pub top_band: usize,
    /// Distinct values in the ranking.
    pub distinct_values: usize,
}

/// Which counter source refused a read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterSite {
    /// The step-one root table at replication 0's seed.
    RootTable,
    /// One replication's whole-turn counters.
    Replication(u32),
}

impl CounterSite {
    /// The sentence a report prints.
    pub fn describe(self) -> String {
        match self {
            Self::RootTable => "the step-one root table at replication 0's seed".to_string(),
            Self::Replication(r) => format!("replication {r}'s whole-turn counters"),
        }
    }
}

/// Why a read produced no outcome.
///
/// [`DrivenError`]'s posture at this grain: every variant is a hard stop that
/// produces nothing. In particular a [`CounterRefusal`](Self::CounterRefusal) is
/// NOT a failing read — it is a read that refuses to be summarized as a
/// preference finding at all, and routes to D7's branch 0.
#[derive(Debug)]
pub enum FamilyReadError {
    /// `r` was 0. A read with no replication measures nothing, and silently
    /// promoting it to 1 would answer a question nobody asked
    /// (`AdjudicationError::NoReplications`'s rule at this grain).
    NoReplications,
    /// The subject names no [`Contestant`] at all — a harness reply rule was
    /// handed in as the SUBJECT. Exhaustive over [`NotAContestant`], so a second
    /// harness rule lands here as a compile error rather than as a silent
    /// materialization.
    SubjectIsNotAContestant(NotAContestant),
    /// The step-one root table could not be produced. Carries
    /// [`RootTableError`] verbatim — including its `Unsupported` arm, which is
    /// how "this subject has no ranked root surface" is said: the counter check
    /// is MANDATORY for every family and every grain, so a `Sampled` /
    /// `Unranked` / `Diagnostic` subject cannot be read here at all, and the
    /// refusal names the constraint in that type's own words rather than in a
    /// second taxonomy.
    Table { replication: u32, error: RootTableError },
    /// One replication's turn did not drive.
    Turn { replication: u32, error: Box<DrivenError> },
    /// **BRANCH-0**: a cap hit, a budget bind, or a starved decision. The read
    /// REFUSES to summarize this root as a preference finding — a truncated
    /// search's pick is not a preference — and the offending counters ride the
    /// refusal so the routing carries its own evidence.
    CounterRefusal { site: CounterSite, counters: Vec<(&'static str, u64)> },
}

impl std::fmt::Display for FamilyReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoReplications => {
                write!(f, "0 replications: a read with no replication is not a measurement")
            }
            Self::SubjectIsNotAContestant(reason) => {
                write!(f, "the subject is not a contestant: {}", reason.as_str())
            }
            Self::Table { replication, error } => {
                write!(f, "replication {replication} produced no root table: {error}")
            }
            Self::Turn { replication, error } => {
                write!(f, "replication {replication}'s turn did not drive: {error}")
            }
            Self::CounterRefusal { site, counters } => {
                let named: Vec<String> =
                    counters.iter().map(|(name, value)| format!("{name}={value}")).collect();
                write!(
                    f,
                    "the counter check refuses this root as a preference reading ({}): {} — a \
                     truncated search's pick is not a preference (D7 branch 0)",
                    site.describe(),
                    named.join(" "),
                )
            }
        }
    }
}

impl std::error::Error for FamilyReadError {}

/// One root, read R times under one frozen rule (design §2's `family_read`).
#[derive(Debug, Clone, PartialEq)]
pub struct FamilyReadOutcome {
    /// The rule this read judged by.
    pub rule: PassRule,
    /// Its grain — [`PassRule::grain`], carried so a report does not re-derive
    /// it.
    pub grain: ReadGrain,
    /// The master every replication seed descends from.
    pub master: u64,
    /// The root's word — the seed derivation's second component.
    pub root_word: u64,
    /// The root's digest, in the form [`root_word`] parsed.
    pub root_digest: String,
    /// What the subject RESOLVED to — never the seat spec's requested name,
    /// which is a symbol whose meaning moves at D1's identity freeze.
    pub subject_echo: String,
    /// One entry per replication, in index order.
    pub replications: Vec<ReplicationRead>,
    /// D7's step-one check.
    pub counter_check: CounterCheck,
    /// Tie/band observations — never refusals (design §5.4).
    pub annotations: Vec<CounterAnnotation>,
}

impl FamilyReadOutcome {
    /// R.
    pub fn replication_count(&self) -> u32 {
        self.replications.len() as u32
    }

    /// TURN grain: replications whose predicate did NOT hold. `None` at decision
    /// grain, where a miss is a fact about a TOP SET that T5's family run
    /// resolves and this outcome cannot.
    pub fn misses(&self) -> Option<u32> {
        match self.grain {
            ReadGrain::Turn => Some(
                self.replications.iter().filter(|r| r.verdict == Some(false)).count() as u32,
            ),
            ReadGrain::Decision => None,
        }
    }

    /// Whether the root should go to the mechanical separator before an argument
    /// rides it — any tie/band annotation (design §5.4).
    pub fn separator_recommended(&self) -> bool {
        self.annotations.iter().any(|a| {
            matches!(a, CounterAnnotation::TiedAtTop { .. } | CounterAnnotation::TopBand { .. })
        })
    }

    /// The EXACT BINOMIAL line a family report prints (design §2).
    ///
    /// At 0 misses of R the miss RATE is 0 and the only informative statement is
    /// an upper bound, so the line carries both the exact one-sided 95%
    /// Clopper–Pearson bound and the RULE OF THREE (`3/R`) it approximates —
    /// side by side, because the rule of three is what a reader recognises and
    /// the exact value is what is true. At ≥1 miss the exact bound is still
    /// reported, computed the same way.
    ///
    /// `None` at decision grain, on [`misses`](Self::misses)'s grounds.
    pub fn binomial_line(&self) -> Option<String> {
        let misses = self.misses()?;
        let r = self.replication_count();
        let rate = if r == 0 { 0.0 } else { f64::from(misses) / f64::from(r) };
        let upper = clopper_pearson_upper(misses, r, 0.05);
        let mut line = format!(
            "misses {misses}/{r} (rate {rate:.4}); exact one-sided 95% upper bound on the \
             per-world miss rate {upper:.4}"
        );
        if misses == 0 && r > 0 {
            line.push_str(&format!(" (rule of three: {:.4})", 3.0 / f64::from(r)));
        }
        Some(line)
    }
}

/// The exact (Clopper–Pearson) one-sided upper confidence bound on a binomial
/// rate: the largest `p` with `P(X <= k | n, p) >= alpha`.
///
/// Bisection on the binomial CDF rather than a beta-quantile library call — the
/// CDF is a polynomial in `p`, monotonically DECREASING in `p` at fixed `k`, so
/// 200 halvings pin the root to well past `f64` printing precision at the n this
/// harness uses (R = 20). It is the EXACT method, not a normal approximation.
///
/// Self-checking corner: at `k = 0` the CDF is `(1−p)^n` and the bound has the
/// closed form `1 − alpha^(1/n)`, which the tests pin this bisection against.
/// `k >= n` is total certainty and returns 1.0.
fn clopper_pearson_upper(k: u32, n: u32, alpha: f64) -> f64 {
    if n == 0 || k >= n {
        return 1.0;
    }
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if binomial_cdf(k, n, mid) >= alpha {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// `P(X <= k)` for `X ~ Binomial(n, p)`, summed from the pmf by the standard
/// forward recurrence.
fn binomial_cdf(k: u32, n: u32, p: f64) -> f64 {
    if p <= 0.0 {
        return 1.0;
    }
    if p >= 1.0 {
        return if k >= n { 1.0 } else { 0.0 };
    }
    let n_f = f64::from(n);
    let mut term = (1.0 - p).powf(n_f);
    let mut acc = term;
    for i in 0..k.min(n) {
        let i_f = f64::from(i);
        term *= (n_f - i_f) / (i_f + 1.0) * (p / (1.0 - p));
        acc += term;
    }
    acc.min(1.0)
}

/// The COUNTERS that refuse a read outright (design §5.4's hard-refuse class),
/// read off one `SearchStats`.
///
/// Empty when the search was clean. The names are the counters' own, so a
/// refusal message and the type's fields cannot drift.
fn refusing_stats(table: &RootTable) -> Vec<(&'static str, u64)> {
    let s = &table.stats;
    [
        ("ply_cap_hits", s.ply_cap_hits),
        ("free_cap_hits", s.free_cap_hits),
        ("budget_binds", s.budget_binds),
        ("budget_starved", s.budget_starved),
    ]
    .into_iter()
    .filter(|(_, value)| *value > 0)
    .collect()
}

/// The same class, read off one turn's [`SeatCounters`].
///
/// **`None` is not zero** — a rung carries no such counter at all — so an absent
/// counter contributes nothing rather than a spurious 0, which is
/// `SeatCounters`' own documented rule.
fn refusing_counters(counters: &SeatCounters) -> Vec<(&'static str, u64)> {
    [
        ("ply_cap_hits", counters.ply_cap_hits),
        ("free_cap_hits", counters.free_cap_hits),
        ("budget_binds", counters.budget_binds),
        ("budget_starved_decisions", counters.budget_starved_decisions),
    ]
    .into_iter()
    .filter_map(|(name, value)| value.filter(|v| *v > 0).map(|v| (name, v)))
    .collect()
}

/// Read ONE root R times under one frozen pass rule (design §2).
///
/// **Seeding is design §2's formula and nothing else**: replication `r` is taken
/// at [`replication_seed`]`(master, `[`root_word`]`(root), r)`. Distinct roots
/// draw distinct streams from one master; the derived seed and the root word
/// both RIDE every replication, so T7's separator can feed a failing
/// replication's seed straight to `driven table … --decision-seed` without
/// re-deriving the mix.
///
/// **What a replication IS depends on the grain**, and both are one decision or
/// one turn of the SUBJECT's own honest chain:
///
/// * [`PassRule::Turn`] — one [`drive_turn`] at that seed, then the predicate on
///   `(root, hand-off)`. The mover's WHOLE-TURN counters ride the replication.
/// * [`PassRule::AdjudicatedBest`] — one [`root_table`] at that seed; the PICK is
///   reported and the top-set judgment is T5's.
///
/// **THE COUNTER CHECK IS MANDATORY AND IT CONSTRAINS THE SUBJECT.** D7's step
/// one is a table at replication 0's seed — the full fifteen counters, the
/// ranking's shape — so a subject with no ranked root surface cannot be read
/// here AT ALL, at either grain. That is a refusal in [`root_table`]'s own words
/// ([`RootTableError::Unsupported`], carrying `Sampled` / `Unranked` /
/// `Diagnostic`), never a second taxonomy and never a read that quietly ships
/// without its step one. In practice the subject is always the shipped
/// `lookahead-v1` echo — design's own subject paragraph — and the constraint
/// only bites a machinery smoke that tried to save a few milliseconds with a
/// rung.
///
/// **The hard-refuse / annotate split is design §5.4's**: any cap hit, budget
/// bind or starved decision — at the root table OR inside any replication's turn
/// — is a [`FamilyReadError::CounterRefusal`] routing to branch 0; tie count and
/// top-band structure ANNOTATE and send the root to the mechanical separator.
///
/// **COST.** R turns (or R tables) of the subject, plus one table. Under the
/// uncapped shipped S1 arm an ordinary mid-game turn is tens of milliseconds and
/// a money-saturated Movement root is not; the sizing choices are the ROOT and
/// R, exactly as they are for `adjudicate_root`.
pub fn family_read(
    root: &GameState,
    subject: SeatSpec,
    rule: &PassRule,
    r: u32,
    master: u64,
    rules: &Rules,
) -> Result<FamilyReadOutcome, FamilyReadError> {
    if r == 0 {
        return Err(FamilyReadError::NoReplications);
    }
    let contestant: Contestant =
        subject.contestant().map_err(FamilyReadError::SubjectIsNotAContestant)?;

    let word = root_word(root);
    let seed_of = |replication: u32| replication_seed(master, word, replication);

    // STEP ONE first, before any replication runs: a subject with no ranked root
    // surface, a root nobody can observe, or a search that was truncated is a
    // fact about the whole read, and paying R turns to learn it would be paying
    // for an answer already refused.
    let table = root_table(root, root.current_player, contestant, rules, seed_of(0))
        .map_err(|error| FamilyReadError::Table { replication: 0, error })?;
    let refusing = refusing_stats(&table);
    if !refusing.is_empty() {
        return Err(FamilyReadError::CounterRefusal {
            site: CounterSite::RootTable,
            counters: refusing,
        });
    }

    let mut annotations: Vec<CounterAnnotation> = Vec::new();
    let tied_at_top = table.tied_at_top();
    let top_band = table.top_band();
    if tied_at_top > 1 {
        annotations.push(CounterAnnotation::TiedAtTop { candidates: tied_at_top });
    }
    if top_band > tied_at_top {
        annotations
            .push(CounterAnnotation::TopBand { candidates: top_band, points: TOP_BAND_POINTS });
    }
    if table.stats.placement_cap_binds > 0 {
        annotations.push(CounterAnnotation::PlacementCapBinds {
            count: table.stats.placement_cap_binds,
        });
    }
    let counter_check = CounterCheck {
        tied_at_top,
        top_band,
        distinct_values: table.distinct_values(),
        table,
    };

    let opts = DrivenOpts::default();
    let mut replications: Vec<ReplicationRead> = Vec::with_capacity(r as usize);
    for replication in 0..r {
        let decision_seed = seed_of(replication);
        let read = match rule {
            PassRule::Turn(predicate) => {
                let outcome = drive_turn(root, subject, rules, decision_seed, &opts)
                    .map_err(|error| FamilyReadError::Turn {
                        replication,
                        error: Box::new(error),
                    })?;
                // SHOULD-6: the WHOLE turn's counters reach the filter, not just
                // the root decision's — a mid-turn cap hit is exactly the thing a
                // replication-0 table cannot see.
                let refusing = refusing_counters(&outcome.counters);
                if !refusing.is_empty() {
                    return Err(FamilyReadError::CounterRefusal {
                        site: CounterSite::Replication(replication),
                        counters: refusing,
                    });
                }
                ReplicationRead {
                    replication,
                    decision_seed,
                    root_word: word,
                    verdict: Some(predicate.eval(root, &outcome.handoff, rules)),
                    pick: None,
                    turn_moves: outcome.decisions,
                    handoff_digest: Some(state_digest(&outcome.handoff)),
                    game_over: Some(outcome.game_over),
                    counters: outcome.counters,
                }
            }
            PassRule::AdjudicatedBest { .. } => {
                // Replication 0's table is the one the counter check already
                // built at this very seed — reused rather than recomputed, which
                // is not merely cheaper: a second identical search would be a
                // second chance for the two to disagree.
                let table = if replication == 0 {
                    counter_check.table.clone()
                } else {
                    root_table(root, root.current_player, contestant, rules, decision_seed)
                        .map_err(|error| FamilyReadError::Table { replication, error })?
                };
                let refusing = refusing_stats(&table);
                if !refusing.is_empty() {
                    return Err(FamilyReadError::CounterRefusal {
                        site: CounterSite::Replication(replication),
                        counters: refusing,
                    });
                }
                ReplicationRead {
                    replication,
                    decision_seed,
                    root_word: word,
                    verdict: None,
                    pick: Some(table.winner),
                    turn_moves: 0,
                    handoff_digest: None,
                    game_over: None,
                    counters: SeatCounters::NONE,
                }
            }
        };
        replications.push(read);
    }

    Ok(FamilyReadOutcome {
        rule: *rule,
        grain: rule.grain(),
        master,
        root_word: word,
        root_digest: state_digest(root),
        subject_echo: subject.echo(),
        replications,
        counter_check,
        annotations,
    })
}

// ---------------------------------------------------------------------------
// The family 3/4 BUILDERS (design §1 families 3–4, §2's builder bullet)
// ---------------------------------------------------------------------------

/// Why a builder produced no root.
///
/// Typed, and every arm is a HARD stop: a builder that returned a
/// nearly-right position would put a root into a frozen block whose shape
/// claim is false, and the read riding it would be wrong with no way to
/// notice. That is the same posture [`AdjudicationError`](crate::game::driven::AdjudicationError)
/// takes one level up, for the same reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuilderError {
    /// `new_game` or `finalize_setup` refused the base deal.
    Setup(&'static str),
    /// The generated board carries no area of the type this construction needs.
    /// Unreachable on a printed 2P board (Church and Customs are never among
    /// the tiles a low player count removes) and typed anyway, because a board
    /// generator change must surface HERE rather than as a wrong fixture.
    MissingArea(&'static str),
    /// The technology registry carries no card with the effect this
    /// construction spends.
    MissingTech(&'static str),
    /// A doctored field would have spent a resource the fixture does not hold
    /// (assistants moved out of a hand that is already empty, a track slot
    /// already occupied, a card that sits in no pile).
    OutOfStock(&'static str),
    /// `check_invariants` refused the built state — the fixture is not one the
    /// engine would ever hand an evaluator.
    NotEngineValid(String),
    /// The built state does not satisfy its OWN shape claim. The clause is
    /// named so a failure says which half of the construction moved.
    ShapeCheck { clause: &'static str, detail: String },
}

impl std::fmt::Display for BuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Setup(detail) => write!(f, "the base setup refused the build: {detail}"),
            Self::MissingArea(area) => {
                write!(f, "the generated board carries no {area} area")
            }
            Self::MissingTech(effect) => {
                write!(f, "the technology registry carries no card with the {effect} effect")
            }
            Self::OutOfStock(what) => write!(f, "the fixture cannot spend {what}"),
            Self::NotEngineValid(detail) => {
                write!(f, "the built state is not one the engine accepts: {detail}")
            }
            Self::ShapeCheck { clause, detail } => {
                write!(f, "the built state fails its own shape claim ({clause}): {detail}")
            }
        }
    }
}

impl std::error::Error for BuilderError {}

/// The church space every race-positive CHURCH draw contests — the LOWEST
/// space on the board, faith 4.
///
/// Lowest deliberately: `eligible_church_spaces` returns every empty space AT
/// OR BELOW a computed faith, so the lowest space is the one BOTH sides reach
/// across the whole declared power range. A higher space would make the
/// contest a function of the drawn power rather than of the construction.
const CONTESTED_FAITH: u8 = 4;

/// The customs space every race-positive CUSTOMS draw contests — printed space
/// 1, whose printed cost is 1 imported good, for [`CONTESTED_FAITH`]'s reason.
const CONTESTED_SPACE: u8 = 1;

/// What the no-denial control's better line is WORTH, in victory points.
///
/// **Seven, and it is the engine's own printed arithmetic rather than a taste**
/// — see [`build_race_no_denial`]'s doc for the derivation. The number is what
/// it is because of a fact about this game that the control had to be built
/// AROUND rather than through: a church space PAYS (`ChurchTrack::vp_reward`
/// runs 3, 4, 5, 6, 7, 8, 11, 14 up the track) and each extra Faith costs at
/// most 1 VP of goods, so "climb the track" is never the wasteful move. What IS
/// wasteful is climbing and then spending the climb on the LOW space the
/// opponent was going to take — which is exactly denial-as-strategy, and
/// exactly what this control prices.
pub const NO_DENIAL_FORFEIT_VP: i64 = 7;

/// The achievement [`build_achievement_proximity`] puts one produce away.
///
/// `Money10` because it is the one category-B card whose requirement an
/// ADDITIONAL-ACTION move can move: the phase never advances inside the
/// additional-action window (only `EndAdditionalPhase` advances it), so the
/// produce and the claim are two decisions of ONE window and the "completable
/// within the turn" claim is checkable by a single apply-probe.
pub const PROXIMITY_ACHIEVEMENT: AchievementType = AchievementType::Money10;

/// The in-turn PRODUCE [`build_achievement_proximity`] leaves available — the
/// Textile Mill's silk → 2 Yen conversion.
///
/// **A deliberate reading of design §1's "produce-to-7 then claim".** The
/// literal goods shape (stand on a Tea Plantation with six tea, produce, claim)
/// is the same TWO-DECISION story, but its two decisions straddle a phase
/// advance — the produce is an AREA action and the claim is an additional
/// action several automatic transitions later — so "the claim becomes legal
/// after the produce" stops being an apply-probe and becomes a driven turn.
/// The mill route keeps both decisions inside one additional-action window,
/// which is what makes the completability claim MECHANICAL here. The shape
/// design §1 cares about — a leg the mover completes within its own turn, with
/// no claim candidate at the root — is identical.
pub const PROXIMITY_PRODUCE: Move = Move::UseSpinningMill;

/// The one area of `wanted` type, if the board has one.
///
/// `driven`'s own `area_of_type`, restated here rather than shared because
/// that one is private to the module that scans; both are two lines over
/// `board_layout.areas` and neither is a rules decision.
fn area_of(state: &GameState, wanted: AreaType) -> Option<AreaId> {
    state.board_layout.areas.iter().find(|(_, c)| c.area_type == wanted).map(|(a, _)| *a)
}

/// The same, over a [`PlayerView`] — the board layout is public information and
/// the view carries it verbatim.
fn view_area_of(view: &PlayerView, wanted: AreaType) -> Option<AreaId> {
    view.board_layout.areas.iter().find(|(_, c)| c.area_type == wanted).map(|(a, _)| *a)
}

/// One player's power at one area, read from a VIEW.
///
/// **A MIRROR of [`crate::game::power::calculate_power`], and it names its
/// source because that is the rule for mirrored logic in this harness** (the
/// Phase-B design's §5 pattern): the shipped function takes a `&GameState`,
/// which is exactly the input a PUBLIC-threat clause may not have. Every term
/// is the shipped one in the shipped order — President, assistants, shophouse,
/// trading house, station token, then the cap applied LAST — and every input is
/// a field the view already carries, which is the whole point: a threat
/// computed here cannot accidentally read a hidden deal, because a hidden deal
/// is not in scope.
fn visible_power(view: &PlayerView, player: &VisiblePlayer, area: AreaId) -> u8 {
    let mut power: u8 = 0;
    if player.president_location == PresidentLocation::OnBoard(area) {
        power += 1;
    }
    power += player.assistants.in_area(area);
    if player.shophouses.built.contains_key(&area) {
        power += 1;
    }
    if player.trading_houses.built.contains(&area) {
        power += 1;
    }
    if view.station_location == Some(area) {
        power += 1;
    }
    power.min(5)
}

/// **THE PUBLIC-THREAT CLAUSE**: is `opponent` ONE COMPLETED ACTION away from
/// taking `step`, as far as a seated player can see?
///
/// **It takes a [`PlayerView`], and that signature IS the publicness claim**
/// (design §1 family 3: "the threat is PUBLIC … the race arithmetic rides board
/// state and port-taken orders, never the hidden deal"). A clause written over
/// `&GameState` would be *asserted* to be public; written over a view it cannot
/// be anything else, because the masked starting order and the deck order are
/// not reachable from here at all. That is the confound the amendment's
/// family-3 sentence flags, shaped out by construction rather than by care.
///
/// The arithmetic MIRRORS `driven`'s own generator-side `opponent_race_steps`
/// (whose doc carries the full derivation and the precision/recall trade it
/// takes): the opponent's President is elsewhere, so it can travel to the track
/// area next turn; it already holds an assistant there, which is the durable
/// claim and also supplies `has_church_or_customs_assistant` (implied by that
/// same conjunct, so it does not appear twice); the power it would ARRIVE with
/// is its current power plus the travelling President, re-capped; and that
/// power, with the payment types it can afford, reaches the declared space.
///
/// **The TECH-MAJORITY arm reads FALSE**, exactly as the generator's does and
/// for the generator's stated reason: an opponent's lab affordability needs the
/// board-position surcharge and `validate_lab_action`'s payment search, and an
/// approximation of it would favour recall where this clause exists to favour
/// precision. No builder here declares a tech step; a caller that did would get
/// a refusal from its own shape check rather than a silent pass.
pub fn opponent_one_action_from_step(
    view: &PlayerView,
    opponent: PlayerId,
    step: ContestedStep,
) -> bool {
    let Some(p) = view.players.iter().find(|p| p.id == opponent) else {
        return false;
    };
    match step {
        ContestedStep::ChurchSpace { faith } => {
            let Some(area) = view_area_of(view, AreaType::Church) else {
                return false;
            };
            if !could_act_next_turn(p, area) {
                return false;
            }
            let arriving = (visible_power(view, p, area) + 1).min(5);
            let max_faith = calculate_church_faith(
                arriving,
                p.copper >= 1,
                p.silk >= 1,
                p.tea >= 1,
                p.seafood >= 1,
                // D1 (2026-08-08): the yen payment type costs 2 Yen.
                p.money >= 2,
                p.imported_goods >= 1,
            );
            eligible_church_spaces(&view.church_track, max_faith).contains(&faith)
        }
        ContestedStep::CustomsSpace { space } => {
            let Some(area) = view_area_of(view, AreaType::Customs) else {
                return false;
            };
            if !could_act_next_turn(p, area) {
                return false;
            }
            let arriving = (visible_power(view, p, area) + 1).min(5);
            let payable = customs_max_imported_goods(arriving)
                .min(u8::try_from(p.imported_goods).unwrap_or(u8::MAX));
            (1..=payable).any(|cost| {
                eligible_customs_spaces_for_cost(&view.customs_track, cost).contains(&space)
            })
        }
        ContestedStep::TechMajorityRankPast { .. } => false,
    }
}

/// The President is elsewhere (so it can travel here next turn) and the
/// assistants that give it power are already here.
fn could_act_next_turn(p: &VisiblePlayer, area: AreaId) -> bool {
    p.president_location != PresidentLocation::OnBoard(area) && p.assistants.in_area(area) >= 1
}

/// Whether ONE legal move puts the MOVER on `step` before its turn ends.
///
/// **TWO probe levels, and the second is not an optimisation.** A Church or
/// Customs action with two or more eligible spaces places NOTHING at apply
/// time: it queues a `PendingChoice::ChurchSpace`/`CustomsSpace`, and the pawn
/// lands when the choice resolves. A one-level probe would therefore see
/// exactly the already-narrowed singleton case and call every race-INITIATING
/// move a non-starter — the same blindness `driven`'s `mover_reachable_steps`
/// documents on its own queue-aware arm.
///
/// The walk stops at the mover's own turn: a follow-up is probed only while the
/// probed clone still has the mover on the move and the game is not over, so
/// nothing here credits the mover with a step the OPPONENT would take.
fn move_reaches_step(
    root: &GameState,
    mv: &Move,
    step: ContestedStep,
    rules: &Rules,
) -> bool {
    let mover = root.current_player;
    let Ok(next) = apply_move_unchecked(root, mv, rules) else {
        return false;
    };
    if step.held_by(&next, mover, rules) {
        return true;
    }
    if is_game_over(&next) || next.current_player != mover {
        return false;
    }
    get_legal_moves(&next, rules).iter().any(|follow| {
        apply_move_unchecked(&next, follow, rules)
            .map(|after| step.held_by(&after, mover, rules))
            .unwrap_or(false)
    })
}

/// Whether the MOVER holds ANY move that takes `step` this turn — the
/// builders' "the mover can alter it" clause, as an apply-probe over the root's
/// own legal list.
pub fn mover_can_take_step(root: &GameState, step: ContestedStep, rules: &Rules) -> bool {
    get_legal_moves(root, rules).iter().any(|mv| move_reaches_step(root, mv, step, rules))
}

/// The root's own LINES, split by whether the line ends with the mover holding
/// a declared step, with the best AS-IF-NOW margin each side reaches.
///
/// **A LINE, not a move, and the difference is the whole correctness of the
/// control.** A track action PAYS at apply time and PLACES at choice-resolution
/// time (the choice-encoding shape): with two or more eligible spaces the
/// action queues a `PendingChoice::ChurchSpace`/`CustomsSpace` and the pawn —
/// with its printed VP reward — lands one decision later. A move-grain split
/// would therefore compare a paid-but-unplaced state against a placed one and
/// read the payment as a loss with no gain, which is a wrong answer that looks
/// like a right one. A line is the root move plus, where the mover still has
/// the move and a choice is queued, its resolution — depth 2, which is exactly
/// as deep as a track action goes.
///
/// The margin is [`true_margin_now`] at the line's end — full-state truth, the
/// same generator-side arithmetic family 2's predicates select on. It is
/// deliberately NOT a search: the no-denial control's claim is that taking the
/// step COSTS something a reader can add up, and an arithmetic a reader can add
/// up is the only kind a control may rest on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DenialSplit {
    /// The best margin reachable by a line that ends on the step.
    pub best_denial: i64,
    /// The best margin reachable by a line that does not.
    pub best_alternative: i64,
    /// How many lines end on it.
    pub denial_lines: usize,
    /// How many do not.
    pub alternative_lines: usize,
}

/// [`DenialSplit`] at one root. Both sides are empty-safe: an empty side keeps
/// [`i64::MIN`], which no comparison can mistake for a reachable margin, and
/// the counts are what a caller checks first.
pub fn denial_split(root: &GameState, step: ContestedStep, rules: &Rules) -> DenialSplit {
    let mover = root.current_player;
    let mut split = DenialSplit {
        best_denial: i64::MIN,
        best_alternative: i64::MIN,
        denial_lines: 0,
        alternative_lines: 0,
    };
    for mv in get_legal_moves(root, rules) {
        let Ok(next) = apply_move_unchecked(root, &mv, rules) else {
            continue;
        };
        // The line ENDS here unless the mover still owes a queued decision —
        // in which case each resolution is its own line.
        let ends: Vec<GameState> = if is_game_over(&next)
            || next.current_player != mover
            || next.pending_choices.is_empty()
        {
            vec![next]
        } else {
            get_legal_moves(&next, rules)
                .iter()
                .filter_map(|follow| apply_move_unchecked(&next, follow, rules).ok())
                .collect()
        };
        for end in ends {
            let margin = true_margin_now(&end, rules);
            if step.held_by(&end, mover, rules) {
                split.denial_lines += 1;
                split.best_denial = split.best_denial.max(margin);
            } else {
                split.alternative_lines += 1;
                split.best_alternative = split.best_alternative.max(margin);
            }
        }
    }
    split
}

/// A finalized 2P game with both seats keeping their first dealt Order — the
/// `scenario_suite` construction grain (`tests/scenario_suite.rs:446`'s
/// `keep_game`), restated here because these builders ship in the LIBRARY and
/// a test helper is not reachable from it.
///
/// Keeping (rather than swapping) is what makes both seats OBSERVABLE:
/// `observe` refuses a seat whose own deal is unrecorded, and family 3's
/// public-threat clause is read from the mover's view.
fn keep_game_2p(seed: u64, rules: &Rules) -> Result<GameState, BuilderError> {
    let pending = new_game(GameConfig::for_player_count(2), rules, seed)
        .map_err(BuilderError::Setup)?;
    finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)])
        .map_err(BuilderError::Setup)
}

/// The seat that is not `mover`, in a 2P state.
fn other_seat(state: &GameState, mover: PlayerId) -> Result<PlayerId, BuilderError> {
    state
        .players
        .iter()
        .map(|p| p.id)
        .find(|id| *id != mover)
        .ok_or(BuilderError::OutOfStock("a rival seat"))
}

/// Zero every seat's goods and money, so a construction's payment arithmetic is
/// exactly what it writes back afterwards rather than what the deal happened to
/// hand out.
fn zero_goods(state: &mut GameState) {
    for p in state.players.iter_mut() {
        p.money = 0;
        p.copper = 0;
        p.silk = 0;
        p.tea = 0;
        p.seafood = 0;
        p.imported_goods = 0;
    }
}

/// Move `n` of `player`'s assistants from its hand onto `area`, keeping the
/// twenty-assistant accounting coherent.
fn place_area_assistants(
    state: &mut GameState,
    player: PlayerId,
    area: AreaId,
    n: u8,
) -> Result<(), BuilderError> {
    let p = state.player_mut(player);
    if p.assistants.in_hand < n {
        return Err(BuilderError::OutOfStock("assistants in hand for an area placement"));
    }
    p.assistants.in_hand -= n;
    *p.assistants.on_board.entry(area).or_insert(0) += n;
    Ok(())
}

/// Take one Technology card out of whichever pile the setup left it in, so a
/// fixture may file it under a seat's `technologies` without minting a 31st
/// card — `scenario_suite`'s `take_tech_out_of_play`, restated for the library
/// for [`keep_game_2p`]'s reason.
fn take_tech_out_of_play(state: &mut GameState, id: TechCardId) -> Result<(), BuilderError> {
    if let Some(i) = state.tech_deck.iter().position(|c| *c == id) {
        state.tech_deck.remove(i);
        return Ok(());
    }
    let labs = std::iter::once(&mut state.lab_state.lab_a).chain(state.lab_state.lab_b.as_mut());
    for slot in labs.flat_map(|lab| lab.slots.iter_mut()) {
        if *slot == Some(id) {
            *slot = None;
            return Ok(());
        }
    }
    Err(BuilderError::OutOfStock("a technology card that sits in no pile"))
}

/// Whether the declared step's own slot is still OPEN — nobody holds it.
///
/// A race whose prize is already taken is not a race, and the two track arms
/// are the only ones with a slot to read: the tech-majority arm names a LEVEL
/// rather than a space, and a level is never occupied.
fn step_is_open(state: &GameState, step: ContestedStep) -> bool {
    match step {
        ContestedStep::ChurchSpace { faith } => ChurchTrack::faith_to_index(faith)
            .is_some_and(|i| state.church_track.slots[i].is_none()),
        ContestedStep::CustomsSpace { space } => CustomsTrack::space_to_index(space)
            .is_some_and(|i| state.customs_track.slots[i].is_none()),
        ContestedStep::TechMajorityRankPast { .. } => true,
    }
}

/// A `ShapeCheck` refusal, spelled once.
fn shape(clause: &'static str, detail: impl Into<String>) -> BuilderError {
    BuilderError::ShapeCheck { clause, detail: detail.into() }
}

/// **FAMILY 3's POSITIVE BUILDER** — a rooted position where the opponent is
/// one completed action from a contested track step, the mover holds a move
/// that takes it this turn, and the threat is PUBLIC (design §1 family 3).
///
/// **DECLARED PARAMETER RANGES** (the freeze document quotes this list; every
/// draw is taken from `SeedSchedule::diagnostic(seed)` — the repo's own
/// SplitMix stream, so a builder block and a scan block at one master are two
/// blocks of the same seeded sequence and no new salt serial is consumed). All
/// six draws are taken UNCONDITIONALLY, before any branch, so the stream
/// position does not depend on which arm a seed selects:
///
/// | # | parameter | range |
/// |---|-----------|-------|
/// | 0 | base setup seed | the whole `u64` space — board layout, port/lab contents and both deals |
/// | 1 | contested track | Church \| Customs |
/// | 2 | mover assistants at a CHURCH area | 3..=4 (⇒ acting power 4..=5) |
/// | 3 | opponent assistants at a CHURCH area | 3..=4 (⇒ arriving power 4..=5) |
/// | 4 | mover assistants at a CUSTOMS area | 1..=3 (⇒ acting power 2..=4) |
/// | 5 | opponent assistants at a CUSTOMS area | 1..=3 (⇒ arriving power 2..=4) |
/// | 6 | opponent imported goods (CUSTOMS arm) | 1..=2 |
///
/// **The construction, per arm.** The root sits at `TurnPhase::AreaAction` with
/// the mover's President on the track area and `current_action_power` set to the
/// power that position gives it, which is the one decision point where a track
/// step is one move away. Every good and every Yen is zeroed for both seats and
/// written back only where the arm's arithmetic needs it, so a payment type is
/// available exactly when the construction says it is.
///
/// * CHURCH: with no payment types affordable, `calculate_church_faith` is the
///   power itself, so the mover's faith is 4..=5 and the OPPONENT's arriving
///   faith is 4..=5 — and [`CONTESTED_FAITH`] (4) is the space both reach at
///   every draw. At faith 4 exactly one space is eligible, so the action places
///   INLINE; at 5, two are, so it defers the pick to a queued choice. Both
///   shapes are real and both are exercised by the sweep.
/// * CUSTOMS: the mover holds exactly ONE imported good, so the only payable
///   cost is 1, whose printed spaces are 1 and 2 — two eligible spaces, so this
///   arm always exercises the QUEUED-CHOICE shape. The opponent holds 1..=2,
///   which is enough for the same cost at its own arriving power.
///
/// **The shape self-check** runs `check_invariants` and then the four clauses
/// this builder's declaration rests on: the step is well formed; it is still
/// OPEN; the opponent's threat holds when read from the MOVER'S OWN VIEW
/// ([`opponent_one_action_from_step`] — the publicness claim, by signature);
/// the mover can take it this turn; and — the predicate/builder coherence pin —
/// the shipped [`RootPredicate::ExclusiveRaceOneStep`] agrees this is one of its
/// own positions.
pub fn build_race_positive(
    seed: u64,
    rules: &Rules,
) -> Result<(GameState, ContestedStep), BuilderError> {
    let mut draws = SeedSchedule::diagnostic(seed);
    let setup_seed = draws.next_world().0;
    let church_arm = draws.next_world().0.is_multiple_of(2);
    let mover_church = 3 + (draws.next_world().0 % 2) as u8;
    let opponent_church = 3 + (draws.next_world().0 % 2) as u8;
    let mover_customs = 1 + (draws.next_world().0 % 3) as u8;
    let opponent_customs = 1 + (draws.next_world().0 % 3) as u8;
    let opponent_imported = 1 + (draws.next_world().0 % 2) as u32;

    let mut state = keep_game_2p(setup_seed, rules)?;
    let mover = state.current_player;
    let opponent = other_seat(&state, mover)?;
    zero_goods(&mut state);

    let (area_type, step, mover_pawns, opponent_pawns) = if church_arm {
        (
            AreaType::Church,
            ContestedStep::ChurchSpace { faith: CONTESTED_FAITH },
            mover_church,
            opponent_church,
        )
    } else {
        (
            AreaType::Customs,
            ContestedStep::CustomsSpace { space: CONTESTED_SPACE },
            mover_customs,
            opponent_customs,
        )
    };
    let area = area_of(&state, area_type)
        .ok_or(BuilderError::MissingArea(if church_arm { "Church" } else { "Customs" }))?;

    place_area_assistants(&mut state, mover, area, mover_pawns)?;
    place_area_assistants(&mut state, opponent, area, opponent_pawns)?;
    state.player_mut(mover).president_location = PresidentLocation::OnBoard(area);
    // The opponent's President is OFF the contested area — clause 1 of the
    // threat, and the reason it can travel there next turn.
    state.player_mut(opponent).president_location = PresidentLocation::InHand;
    if !church_arm {
        state.player_mut(mover).imported_goods = 1;
        state.player_mut(opponent).imported_goods = opponent_imported;
    }
    state.turn_phase = TurnPhase::AreaAction;
    state.current_action_power = Some((mover_pawns + 1).min(5));
    state.is_mercenary_action = false;
    state.mercenary_target_area = None;
    state.pending_choices.clear();

    check_invariants(&state, rules).map_err(BuilderError::NotEngineValid)?;
    self_check_race_root(&state, step, mover, opponent, rules)?;
    Ok((state, step))
}

/// The four clauses [`build_race_positive`] declares, checked on the built
/// state — shared with nothing, because a shape check that ran anywhere but at
/// the end of the construction it describes would be checking a different
/// position.
fn self_check_race_root(
    state: &GameState,
    step: ContestedStep,
    mover: PlayerId,
    opponent: PlayerId,
    rules: &Rules,
) -> Result<(), BuilderError> {
    if !step.well_formed() {
        return Err(shape("well-formed step", format!("{step:?} names no space")));
    }
    if !step_is_open(state, step) {
        return Err(shape("open step", format!("{step:?} is already held")));
    }
    let view = observe(state, mover)
        .map_err(|e| shape("observable mover", format!("{e:?}")))?;
    if !opponent_one_action_from_step(&view, opponent, step) {
        return Err(shape(
            "public opponent threat",
            format!("seat {} is not one action from {step:?} in the mover's view", opponent.0),
        ));
    }
    if !mover_can_take_step(state, step, rules) {
        return Err(shape(
            "mover can take the step",
            format!("no legal move at this root reaches {step:?}"),
        ));
    }
    if !RootPredicate::ExclusiveRaceOneStep.holds(state, rules) {
        return Err(shape(
            "predicate coherence",
            "the shipped ExclusiveRaceOneStep predicate does not recognise this root".to_string(),
        ));
    }
    Ok(())
}

/// **FAMILY 3's NO-DENIAL CONTROL BUILDER** — a rooted position where a
/// denial-shaped step is available and taking it FORFEITS a strictly better
/// line (design §1 family 3; canon 12/13: denial is not a strategy).
///
/// **DECLARED PARAMETER RANGES** (drawn from `SeedSchedule::diagnostic(seed)`,
/// unconditionally and in order, exactly as [`build_race_positive`] draws
/// them):
///
/// | # | parameter | range |
/// |---|-----------|-------|
/// | 0 | base setup seed | the whole `u64` space |
/// | 1 | the mover's Yen | 0..=1 — below the Church's 2-Yen payment (D1), so Yen is never a payment type and `money / 2` is 0 either way |
/// | 2 | the opponent's Yen | 0..=1, for the same two reasons |
/// | 3 | the mover's starting victory points | 0..=3 — a pure shift that cancels in the difference below |
///
/// Three figures are PREMISES of the arithmetic rather than free parameters and
/// are therefore fixed: the mover's four assistants at the Church area (power
/// 5), the opponent's three (arriving power 4), and the mover's one each of the
/// four trade goods.
///
/// **THE CONSTRUCTION ARITHMETIC, written out** — and the first line of it is
/// the fact the control had to be built around:
///
/// 1. A church space PAYS its printed reward on placement
///    (`ChurchTrack::vp_reward`: 3, 4, 5, 6, 7, 8, 11, 14 up the track), and
///    each extra Faith costs the mover at most 1 VP of goods. So climbing the
///    track is never the wasteful move, and a control resting on "taking a
///    space costs you" would be resting on something false.
/// 2. What IS wasteful is climbing and then spending the climb on the LOW space
///    — which is precisely denial, because the low space is the only one the
///    opponent can reach. The opponent arrives at power 4 holding no goods, so
///    its maximum Faith is exactly 4 and [`CONTESTED_FAITH`] is the ONE space it
///    threatens.
/// 3. The mover acts at power 5 holding one each of copper/silk/tea/seafood, so
///    the church masks are the 16 subsets of those four (Yen and imported goods
///    are unaffordable by construction) and the mask paying `k` types reaches
///    Faith `5 + k`. Its goods bill is `1 − (4 − k)/3` VP of `remainder_bonus`
///    — 0 for k ≤ 1 and 1 for k ≥ 2 — because four goods score 1 and three
///    still do.
/// 4. **The church MAJORITY moves with the space too**, and it is the larger
///    half of the price. At the root the track holds only the 2P dummy at Faith
///    7 (slot index 3); the mover's first pawn ties it on count, so the
///    majority's positional tie-break — RIGHTMOST occupied slot — hands FIRST
///    place (6 VP) to a pawn on Faith 8 or above and leaves a lower pawn in
///    SECOND (3 VP).
/// 5. Best line NOT ending on Faith 4: `k = 4`, Faith 9, space 9 — 8 printed
///    plus 6 majority against a 1 VP bill, **+13**. (The five masks read +7,
///    +8, +7, +12, +13.)
/// 6. Best line ENDING on Faith 4: the free mask — 3 printed plus 3 majority
///    against a 0 VP bill, **+6**. (The `k = 1` mask TIES it at +6 rather than
///    trailing it — three goods still score 1, so paying ONE type costs
///    nothing, which is step 3's own bill read at `k = 1`. It is the masks
///    paying `k ≥ 2` that reach the same space for 1 VP less. The best DENIAL
///    line is therefore +6 either way, which is what the forfeit below
///    subtracts.)
///
/// Hence [`NO_DENIAL_FORFEIT_VP`] = 13 − 6 = 7, identical at every draw (the
/// declared parameters shift both sides or neither), and asserted by the
/// builder's own self-check rather than bounded.
///
/// The control still SITS in the race — [`RootPredicate::ExclusiveRaceOneStep`]
/// holds here, and the self-check requires it — which is the point: it is the
/// same kind of position as the positive draws, and the only thing that differs
/// is whether taking the contested step is worth what it costs.
pub fn build_race_no_denial(
    seed: u64,
    rules: &Rules,
) -> Result<(GameState, ContestedStep), BuilderError> {
    /// The mover's assistants at the church area — with its President there,
    /// power 5, so the free mask already reaches Faith 5.
    const MOVER_AREA_ASSISTANTS: u8 = 4;
    /// The opponent's — arriving power 4, so Faith 4 is the ONE space it can
    /// reach and the contested step is unambiguous.
    const OPPONENT_AREA_ASSISTANTS: u8 = 3;

    let mut draws = SeedSchedule::diagnostic(seed);
    let setup_seed = draws.next_world().0;
    let mover_money = (draws.next_world().0 % 2) as u32;
    let opponent_money = (draws.next_world().0 % 2) as u32;
    let mover_start_vp = (draws.next_world().0 % 4) as u32;

    let mut state = keep_game_2p(setup_seed, rules)?;
    let mover = state.current_player;
    let opponent = other_seat(&state, mover)?;
    zero_goods(&mut state);
    let area = area_of(&state, AreaType::Church).ok_or(BuilderError::MissingArea("Church"))?;

    place_area_assistants(&mut state, mover, area, MOVER_AREA_ASSISTANTS)?;
    place_area_assistants(&mut state, opponent, area, OPPONENT_AREA_ASSISTANTS)?;
    state.player_mut(mover).president_location = PresidentLocation::OnBoard(area);
    state.player_mut(opponent).president_location = PresidentLocation::InHand;
    {
        let p = state.player_mut(mover);
        p.money = mover_money;
        p.victory_points = mover_start_vp;
        // ONE of each trade good: the four affordable payment types, and a
        // goods bill of 1 VP that two of them buy back.
        p.copper = 1;
        p.silk = 1;
        p.tea = 1;
        p.seafood = 1;
    }
    state.player_mut(opponent).money = opponent_money;
    state.turn_phase = TurnPhase::AreaAction;
    state.current_action_power = Some(MOVER_AREA_ASSISTANTS + 1);
    state.is_mercenary_action = false;
    state.mercenary_target_area = None;
    state.pending_choices.clear();

    let step = ContestedStep::ChurchSpace { faith: CONTESTED_FAITH };
    check_invariants(&state, rules).map_err(BuilderError::NotEngineValid)?;
    self_check_race_root(&state, step, mover, opponent, rules)?;
    let split = denial_split(&state, step, rules);
    if split.denial_lines == 0 || split.alternative_lines == 0 {
        return Err(shape("both sides of the denial split", format!("{split:?}")));
    }
    if split.best_alternative - split.best_denial != NO_DENIAL_FORFEIT_VP {
        return Err(shape(
            "the declared forfeit",
            format!(
                "taking {step:?} forfeits {} VP, not the declared {NO_DENIAL_FORFEIT_VP}: {split:?}",
                split.best_alternative - split.best_denial,
            ),
        ));
    }
    Ok((state, step))
}

/// **FAMILY 4's PROXIMITY-CORNER BUILDER** — the F8 shape: an achievement leg
/// the mover completes WITHIN one turn, two decisions from the root, with NO
/// claim candidate at the root at all (design §1 family 4).
///
/// This is the corner that makes family 4's read a TURN-grain read rather than
/// a decision-grain one, and it is also the corner design §4's admission
/// arithmetic is shaped by: a candidate-level pass set is structurally EMPTY
/// here, because the claim is not in the root's legal list.
///
/// **DECLARED PARAMETER RANGES** (drawn from `SeedSchedule::diagnostic(seed)`,
/// unconditionally and in order):
///
/// | # | parameter | range |
/// |---|-----------|-------|
/// | 0 | base setup seed | the whole `u64` space |
/// | 1 | the mover's Yen | 8..=9 — one mill conversion short of ten |
/// | 2 | the mover's silk | 1..=3 — the conversion's fuel |
/// | 3 | the opponent's Yen | 0..=7 — public context, kept clear of its own claim |
///
/// **The construction.** The three achievement cards are doctored to types only
/// one of which is reachable at all ([`PROXIMITY_ACHIEVEMENT`], `Money10`; the
/// other two need seven tea and four production-area buildings, and both seats
/// have neither), every good is zeroed and written back only where the
/// arithmetic needs it, and the mover owns the Textile Mill — taken out of the
/// deck or a lab slot so tech conservation holds. The root sits where the deal
/// leaves it, at `TurnPhase::AdditionalActionFirst`.
///
/// **The shape self-check is the completability claim itself**: the claim is
/// NOT legal at the root, [`PROXIMITY_PRODUCE`] IS, applying it leaves the
/// claim legal, and the mover still has the assistant a claim places. The
/// produce never advances the phase (only `EndAdditionalPhase` does), so the
/// two decisions are two decisions of one turn.
pub fn build_achievement_proximity(seed: u64, rules: &Rules) -> Result<GameState, BuilderError> {
    let mut draws = SeedSchedule::diagnostic(seed);
    let setup_seed = draws.next_world().0;
    let mover_money = 8 + (draws.next_world().0 % 2) as u32;
    let mover_silk = 1 + (draws.next_world().0 % 3) as u32;
    let opponent_money = (draws.next_world().0 % 8) as u32;

    let mut state = keep_game_2p(setup_seed, rules)?;
    let mover = state.current_player;
    let opponent = other_seat(&state, mover)?;
    zero_goods(&mut state);
    state.achievements = [
        AchievementCard::new(AchievementType::Tea7),
        AchievementCard::new(PROXIMITY_ACHIEVEMENT),
        AchievementCard::new(AchievementType::Production4),
    ];
    {
        let p = state.player_mut(mover);
        p.money = mover_money;
        p.silk = mover_silk;
    }
    state.player_mut(opponent).money = opponent_money;

    let mill = rules
        .techs
        .iter_sorted_by_id()
        .into_iter()
        .find(|card| card.effect == TechEffect::SpinningMill)
        .map(|card| card.id)
        .ok_or(BuilderError::MissingTech("SpinningMill"))?;
    take_tech_out_of_play(&mut state, mill)?;
    state.player_mut(mover).technologies.push(mill);
    state.pending_choices.clear();

    check_invariants(&state, rules).map_err(BuilderError::NotEngineValid)?;
    let claim = Move::FulfillAchievement { achievement: PROXIMITY_ACHIEVEMENT };
    let legal = get_legal_moves(&state, rules);
    if legal.contains(&claim) {
        return Err(shape(
            "the claim is not yet available",
            "the root already offers the claim — this is F8's LIVE shape".to_string(),
        ));
    }
    if !legal.contains(&PROXIMITY_PRODUCE) {
        return Err(shape(
            "the in-turn produce is available",
            format!("{PROXIMITY_PRODUCE:?} is not legal at this root"),
        ));
    }
    if state.player(mover).assistants.in_hand < 1 {
        return Err(shape(
            "an assistant for the claim",
            "the mover holds no assistant to place on the achievement".to_string(),
        ));
    }
    let after = apply_move_unchecked(&state, &PROXIMITY_PRODUCE, rules)
        .map_err(|e| shape("the produce applies", format!("{e:?}")))?;
    if after.current_player != mover {
        return Err(shape(
            "the produce stays inside the turn",
            "applying the produce handed the turn off".to_string(),
        ));
    }
    if !get_legal_moves(&after, rules).contains(&claim) {
        return Err(shape(
            "two-decision completability",
            "the claim is not legal one produce later".to_string(),
        ));
    }
    Ok(state)
}

// ---------------------------------------------------------------------------
// The FROZEN FAMILY REGISTRY (design §1's four families, §2's `FamilyDecl`)
// ---------------------------------------------------------------------------

/// The declared reply MENU — all four policies, for every family (design §4).
///
/// `RaceRusher` is in it deliberately: the conjunction across the menu is only
/// as informative as its most hostile arm, and a menu of three cooperative
/// replies would admit roots that only look worthwhile because nobody competed
/// for them.
pub const REPLY_MENU: &[ReplyPolicy] = &[
    ReplyPolicy::FamilyWeighted,
    ReplyPolicy::GreedyAtFinal,
    ReplyPolicy::LookaheadAtFinal,
    ReplyPolicy::RaceRusher,
];

/// The PILOT block's ordinals, for every family — design §4's `n = 4` strided
/// draws.
pub const PILOT_DRAWS: std::ops::Range<u32> = 0..4;

/// How many strided draws a CONFIRMATORY block spans (design §4's `m = 6`).
pub const CONFIRMATORY_BLOCK_ROOTS: usize = 6;

/// The VOID rule's floor: a confirmatory result quoting fewer than this many
/// ADMITTED roots confirms nothing (design §4, review MUST-4).
pub const MIN_ADMITTED_ROOTS: usize = 4;

/// The pre-declared disposition when the VOID rule fires — carried on the
/// verdict so a report states the recovery rather than leaving it to be
/// remembered.
pub const VOID_DISPOSITION: &str =
    "extend to the next PRE-DECLARED strided ordinals (never a re-draw), or record and route as \
     under-powered";

/// R — replications per root (design §4).
///
/// **STILL THE DRAFT POSTURE after the freeze wave**, which fixed ε, δ and the
/// per-policy K and deliberately left R and F alone: design §4 states R = 20,
/// and what prices R honestly is the per-root miss distribution the
/// CONFIRMATORY runs themselves report. The freeze document is where that
/// pricing lands — so this number, and [`PROVISIONAL_PER_ROOT_F`] with it, are
/// the two in this block a reader must not read as frozen.
pub const PROVISIONAL_REPLICATIONS: u32 = 20;

/// Per-root allowed misses (design §4's draft posture, F = 1 of 20).
///
/// It is not 0 for a stated reason: admission dominance is measured in TRUTH
/// while the subject decides in a SAMPLED world, so a sampled deal can flip the
/// in-world best line at a truth-dominant root, and a blanket F = 0 would make
/// family failure a Bernoulli event over ~120 trials with unstated false-alarm
/// arithmetic. PROVISIONAL until the freeze prices it from pilot-measured
/// world-sensitivity.
pub const PROVISIONAL_PER_ROOT_F: u32 = 1;

/// The FAMILY-level failure allowance (design §4's draft: 0 roots for families
/// 1, 2 and 4). Family 3 declares no bar and skips it entirely.
pub const FAMILY_FAILURE_ALLOWANCE: usize = 0;

/// ε for family 1's [`PassRule::AdjudicatedBest`], in victory points.
///
/// **FROZEN AT 2 VP by the freeze wave** (a REAL LOOSENING, disclosed — the design
/// drafts no ε; the sanctioned sd-UCB derivation is recorded in the runs report), raised
/// from the 1 VP the pilots ran at. Design §4 derives it at the freeze from
/// ADJUDICATION-MARGIN noise at pilot roots, and the pilot supplied exactly that
/// measurement: at ε = 1 with K = 4 the trigger-round control read UNRESOLVED at
/// EVERY root — the declared policies' top sets differed by less than a point of
/// adjudication noise, which is a statement about the instrument rather than
/// about policy disagreement.
///
/// The leniency direction, stated once more where the number is: a LARGER
/// epsilon is an EASIER pass. Raising it is therefore a real loosening of
/// family 1's rule, taken deliberately and on measured grounds — the
/// alternative was a family whose every root routed to the exact adjudicator.
pub const FROZEN_EPSILON_VP: i64 = 2;

/// δ for root admission, in victory points (design §4's draft 5 VP).
///
/// **UNMOVED by the freeze wave, and that is a decision rather than an
/// omission**: the one pilot family whose admission discriminated cleanly
/// (family 2, 1 of 4 admitted on each arm, the admitted control root showing the
/// subject firing 20/20 where not-firing dominates) read at this δ, and the
/// families that admitted nothing failed on DEGENERATE GROUP SHAPES rather than
/// on the margin — which is what the splitting rules answer and what a δ move
/// would have papered over.
pub const DELTA_VP: i64 = 5;

/// The per-policy admission K the PILOT posture runs (design §4's draft: the
/// full menu at K = 4).
///
/// Kept, and kept at 4, so the pilot runs that already happened reproduce
/// exactly. [`FROZEN_ADMISSION_K`] is what a confirmatory block runs.
pub const PILOT_ADMISSION_K: u32 = 4;

/// The per-policy admission K a CONFIRMATORY block runs (the freeze wave).
///
/// **8, doubled from the pilot's 4, and the reason is group-mean noise.** An
/// admission compares two group MEANS, and at K = 4 a four-candidate root gives
/// groups of a handful of playouts whose means move a full δ on one game — the
/// pilot's family-weighted rung split 1-of-8 at more than one root. Doubling K
/// is the cheapest lever on that variance (the walls are trivial: the whole
/// ten-command pilot ran in about three minutes at K = 4), and it composes with
/// [`SPLIT_MIN_GROUP`], which refuses the groups that stay thin anyway.
pub const FROZEN_ADMISSION_K: u32 = 8;

/// The PLACEHOLDER step a step-carrying declaration is written with.
///
/// Design §3 is explicit that a contested step is "declared by the builder or
/// predicate at generation time, never inferred at read time" — so family 3's
/// declaration names the SHAPE of its rule and the ROOT names the step. A block
/// run re-points the rule per root through [`PassRule::with_step`]; a root that
/// declares no step under a step-carrying rule is a typed refusal, never a run
/// against this placeholder.
pub const DECLARED_BY_THE_ROOT: ContestedStep =
    ContestedStep::ChurchSpace { faith: CONTESTED_FAITH };

/// What a family CLAIMS when it passes (design §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyLabel {
    /// The family confirms a behaviour the canon (or the rules) already says is
    /// right. A pass is evidence FOR the subject; a failure routes through D7.
    Confirmatory,
    /// The family demonstrates a GAP and declares no pass bar. Reads at the
    /// current subject are EXPECTED to fail its positive roots — the output is
    /// the demonstrated per-root gap, which supports a pivot or a routing and
    /// never a GO.
    ExistenceDemonstration,
}

/// The PARTITION multiplicity is judged over (design §4, review SHOULD-9).
///
/// The amendment's rule is "two or more failures in the same strategic class →
/// the pivot conversation", which is undefined without the partition — so it is
/// declared here rather than left to a reader. Families 2 and 4 are ONE class
/// (D7 branch 2's own cluster); family 3 is its own (branch 3), which is why its
/// expected failures cannot combine with the other two to fire the pivot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategicClass {
    /// Family 1 — rules-derivable final-round behaviour.
    RulesFinalRound,
    /// Families 2 and 4 — end timing and claim proximity.
    EndTimingProximity,
    /// Family 3 — opponent reply and the cross-hand-off question.
    OpponentReply,
}

impl StrategicClass {
    /// The stable name, for reports and the freeze document.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RulesFinalRound => "rules/final-round",
            Self::EndTimingProximity => "end-timing/proximity",
            Self::OpponentReply => "opponent-reply",
        }
    }
}

/// A block of draws BAKED INTO the declaration at the freeze commit (design §2,
/// review SHOULD-4 — the committed-diagnostic-master pattern the gates already
/// use).
///
/// Before the freeze every declaration carried `None` and every run supplied its
/// own master; since the freeze wave each declaration carries ONE PER ARM
/// ([`FamilyDecl::confirmatory`] and [`FamilyDecl::confirmatory_control`]),
/// `--block confirmatory` runs the committed identity, and a caller-typed master
/// cannot silently substitute a non-frozen block. **Escrow ordinals are
/// deliberately NOT representable here**: they live in the freeze document, so
/// no command can burn one by accident — which is why there are TWO block fields
/// and not three.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenBlock {
    /// The recorded master this block's draws descend from.
    pub master: u64,
    /// Its ordinals, in the STRIDED space (design §4).
    pub ordinals: std::ops::Range<u32>,
}

/// What a BUILDER hands back: the state, plus the step it declares if its family
/// reads a step-carrying rule.
#[derive(Debug, Clone, PartialEq)]
pub struct BuiltRoot {
    pub state: GameState,
    /// The step this root DECLARES — `Some` for family 3's builders, `None` for
    /// a family whose rule names no step.
    pub step: Option<ContestedStep>,
}

/// The one signature a declared builder has.
///
/// **Why the three public builders do not have this signature themselves**: they
/// return what each family actually needs (`(GameState, ContestedStep)` for the
/// race pair, a bare `GameState` for the proximity corner), which is the honest
/// type at each call site and is what their own pins read. A `'static`
/// declaration needs ONE type, so three thin adapters normalize them into
/// [`BuiltRoot`] — the adaptation is visible, one line each, and neither half
/// pretends the other's shape.
pub type BuilderFn = fn(u64, &Rules) -> Result<BuiltRoot, BuilderError>;

/// How a family's roots are DRAWN (design §1: "generators are rules-plus-draws,
/// never positions picked after seeing play").
///
/// **THE SHAPE DECISION, stated because the plan left it open**: an enum with a
/// PREDICATE arm and a BUILDER arm carrying a plain `fn` pointer plus its NAME.
/// A trait object would have needed a `dyn` behind a `&'static` in a `const`
/// context for no gain — there is exactly one operation ("draw the root at this
/// seed") and no state — and a bare `fn` pointer without the name would leave a
/// report unable to say WHICH builder produced a block, which is half of a
/// builder draw's identity.
#[derive(Clone, Copy)]
pub enum FamilyGenerator {
    /// A seeded SCAN for positions satisfying a predicate, under the ecology
    /// `RootPredicate::ecology` names.
    Predicate(RootPredicate),
    /// A parameterized CONSTRUCTOR, drawn at seeds blocked the same way a scan's
    /// ordinals are.
    Builder { build: BuilderFn, name: &'static str },
}

impl std::fmt::Debug for FamilyGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Predicate(p) => write!(f, "Predicate({})", p.as_str()),
            Self::Builder { name, .. } => write!(f, "Builder({name})"),
        }
    }
}

impl PartialEq for FamilyGenerator {
    /// Compared by IDENTITY — the predicate, or the builder's declared name.
    /// Function-pointer equality is deliberately not the comparison: two
    /// monomorphizations of one function are not required to share an address,
    /// and the name is what a freeze document quotes anyway.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Predicate(a), Self::Predicate(b)) => a == b,
            (Self::Builder { name: a, .. }, Self::Builder { name: b, .. }) => a == b,
            _ => false,
        }
    }
}

impl FamilyGenerator {
    /// The stable name a report and a freeze document quote.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Predicate(p) => p.as_str(),
            Self::Builder { name, .. } => name,
        }
    }
}

/// ONE FROZEN FAMILY, as code (design §2).
///
/// A frozen family is a declared root GENERATOR + declared reply policies +
/// pre-declared R and failure counts + a declared PASS RULE + a CONFIRMATORY or
/// EXISTENCE-DEMONSTRATION label + disjoint pilot/confirmatory blocks. Every one
/// of those is a field here, and the point of putting them in code rather than
/// in prose is that a block run cannot quietly use different ones.
#[derive(Debug, Clone, PartialEq)]
pub struct FamilyDecl {
    /// The name a runner asks for.
    pub id: &'static str,
    pub label: FamilyLabel,
    pub class: StrategicClass,
    pub generator: FamilyGenerator,
    /// The declared pass rule. A step-carrying rule is written with
    /// [`DECLARED_BY_THE_ROOT`] and re-pointed per root.
    pub rule: PassRule,
    /// R.
    pub replications: u32,
    /// Per-root allowed misses.
    pub per_root_f: u32,
    /// The declared reply menu, in menu order.
    pub menu: &'static [ReplyPolicy],
    /// The PILOT block's ordinals. Pilots quote nothing confirmatory, so they
    /// carry no master: the caller records one before use.
    pub pilot: std::ops::Range<u32>,
    /// The POSITIVE arm's CONFIRMATORY block, baked at the freeze commit. `None`
    /// before it, and `None` is still the shape a newly declared arm carries.
    pub confirmatory: Option<FrozenBlock>,
    /// The CONTROL arm's CONFIRMATORY block (the freeze wave).
    ///
    /// **Its own block, because the arms draw their own roots.** A control arm
    /// is a different generator — a different predicate, or a builder — so it
    /// has its own ordinal space and a control run that inherited the positive
    /// arm's block would quote the positive arm's identity for a different set
    /// of positions. `driven_runner::control_arm` re-points `confirmatory` from
    /// this field, which is what keeps `resolve_block` a single rule over one
    /// field rather than a second rule that has to know about arms.
    ///
    /// **THERE IS NO THIRD BLOCK FIELD.** Escrow ordinals live in the freeze
    /// DOCUMENT (design §2's escrow hygiene): no command can burn an escrowed
    /// block by accident because no command can name one.
    pub confirmatory_control: Option<FrozenBlock>,
    /// Which canon (or which rules clause) this family rests on.
    pub canon: &'static str,
}

/// [`build_race_positive`] as a [`BuilderFn`].
fn built_race_positive(seed: u64, rules: &Rules) -> Result<BuiltRoot, BuilderError> {
    let (state, step) = build_race_positive(seed, rules)?;
    Ok(BuiltRoot { state, step: Some(step) })
}

/// [`build_race_no_denial`] as a [`BuilderFn`] — family 3's declared CONTROL,
/// reachable by a caller that runs the control arm.
fn built_race_no_denial(seed: u64, rules: &Rules) -> Result<BuiltRoot, BuilderError> {
    let (state, step) = build_race_no_denial(seed, rules)?;
    Ok(BuiltRoot { state, step: Some(step) })
}

/// [`build_achievement_proximity`] as a [`BuilderFn`] — family 4's declared
/// proximity CORNER, reachable by a caller that runs the corner block.
fn built_achievement_proximity(seed: u64, rules: &Rules) -> Result<BuiltRoot, BuilderError> {
    Ok(BuiltRoot { state: build_achievement_proximity(seed, rules)?, step: None })
}

/// Family 3's declared CONTROL generator (design §1: "a no-denial root — a
/// denial-shaped move available but not worthwhile").
///
/// It is a named handle rather than a fifth entry in [`families`] because the
/// design's registry is FOUR families: the control is an ARM of family 3, run
/// under family 3's identity with `DenialNotTaken` in place of
/// `ContestedStepTaken`, and giving it its own row would make the multiplicity
/// rule count it as a family.
pub const RACE_CONTROL_GENERATOR: FamilyGenerator =
    FamilyGenerator::Builder { build: built_race_no_denial, name: "build_race_no_denial" };

/// Family 4's declared proximity CORNER generator (design §1 family 4), an arm
/// of family 4 for [`RACE_CONTROL_GENERATOR`]'s reason.
pub const PROXIMITY_CORNER_GENERATOR: FamilyGenerator = FamilyGenerator::Builder {
    build: built_achievement_proximity,
    name: "build_achievement_proximity",
};

/// Family 2's declared CONTROL generator (design §1 family 2: "a behind mover
/// should hand off WITHOUT firing"), an arm of family 2 for
/// [`RACE_CONTROL_GENERATOR`]'s reason.
///
/// It is the EXACT COMPLEMENT of family 2's positive generator on the truth-margin
/// conjunct — [`RootPredicate::EndFirableBehindInTruth`] against
/// `EndFirableAheadInTruth` — read against `TurnPredicate::NotEndFired`, so a
/// result that holds on one and not the other is a statement about the SIGN of
/// the margin rather than about firing. A PREDICATE arm rather than a builder,
/// unlike the other two declared arms, because the complement is a scan
/// question and nothing here is constructed.
///
/// Declared as a named handle for the same reason the other two are (T5 review
/// NOTE-4): the design's registry is FOUR families, the control is an ARM of
/// family 2, and all three control/corner arms now share ONE convention a runner
/// can expose without a fifth registry row.
pub const FAMILY2_CONTROL_GENERATOR: FamilyGenerator =
    FamilyGenerator::Predicate(RootPredicate::EndFirableBehindInTruth);

/// **THE FOUR FROZEN FAMILIES** (design §1), in the design's own order.
///
/// **BAKED at the freeze wave**: every declaration carries a confirmatory block
/// for BOTH arms, each naming the master its ordinals descend from — recorded
/// before use by the caller who drew them, and transcribed here hex for hex.
/// The pilot ordinals (`0..4`) and every confirmatory block are DISJOINT, so a
/// confirmatory result is a fresh reading rather than a re-quote of the roots
/// the pilot already looked at.
///
/// The block sizes are the supply arithmetic, verified against each generator's
/// declared ecology in `driven_runner`'s
/// `every_frozen_block_fits_the_supply_its_generator_declares`: family 1 spans
/// **16 draws** (4..20) inside the v1.1 rung ecology's 16 games × cap 2 = 32
/// strided ordinals, and the layer-2 families span **24 draws** (4..28) inside
/// the v2 lookahead ecology's 24 × 2 = 48. Neither needed a games-bound change.
///
/// What remains the design's draft posture is R and F, which the freeze document
/// prices from the confirmatory runs themselves.
pub fn families() -> Vec<FamilyDecl> {
    vec![
        // ---- FAMILY 1 -----------------------------------------------------
        // Already-triggered final round. CONFIRMATORY and RULES-DERIVABLE — no
        // canon is needed, which is why it is the family a healthy subject is
        // most expected to pass and therefore the sharpest instrument check.
        //
        // The DECISION grain is right here and nowhere else: the adjudicator
        // forces exactly the decision the read judges, and a final-round playout
        // is at most two turns.
        //
        // Its declared CONTROL is `RootPredicate::TriggerRoundNotFinal` — the
        // exact complement on the `final_round` conjunct, so a result that holds
        // on one and not the other is a statement about the FINAL ROUND rather
        // than about the trigger (the session's g1:228 shape). Admission here is
        // the LIVE/DEAD split rather than a dominance demonstration.
        FamilyDecl {
            id: "final-round",
            label: FamilyLabel::Confirmatory,
            class: StrategicClass::RulesFinalRound,
            generator: FamilyGenerator::Predicate(RootPredicate::FinalRoundAlreadyTriggered),
            rule: PassRule::AdjudicatedBest { epsilon: FROZEN_EPSILON_VP },
            replications: PROVISIONAL_REPLICATIONS,
            per_root_f: PROVISIONAL_PER_ROOT_F,
            menu: REPLY_MENU,
            pilot: PILOT_DRAWS,
            // Both arms at ONE master: the control is a different PREDICATE, so
            // the same master and the same ordinals name a different ordinal
            // space and therefore different roots.
            confirmatory: Some(FrozenBlock {
                master: 0xA3E1_2026_0818_0004,
                ordinals: 4..20,
            }),
            confirmatory_control: Some(FrozenBlock {
                master: 0xA3E1_2026_0818_0004,
                ordinals: 4..20,
            }),
            canon: "rules-derivable (rulebook p.9's final round); no canon ruling needed — \
                    design §1 family 1",
        },
        // ---- FAMILY 2 -----------------------------------------------------
        // End-trigger firing. CONFIRMATORY on canon 15, the INVERTED BRAKE.
        //
        // The positive predicate supplies the TRUE arithmetic canon 15 is about
        // (the F9 lesson made a selection rule) rather than testing whether the
        // subject can reconstruct it. Its declared CONTROL is
        // `RootPredicate::EndFirableBehindInTruth` read against
        // `TurnPredicate::NotEndFired`: a behind mover should hand off WITHOUT
        // firing, and what it built instead is recorded, not gated.
        FamilyDecl {
            id: "end-trigger",
            label: FamilyLabel::Confirmatory,
            class: StrategicClass::EndTimingProximity,
            generator: FamilyGenerator::Predicate(RootPredicate::EndFirableAheadInTruth),
            rule: PassRule::Turn(TurnPredicate::EndFired),
            replications: PROVISIONAL_REPLICATIONS,
            per_root_f: PROVISIONAL_PER_ROOT_F,
            menu: REPLY_MENU,
            pilot: PILOT_DRAWS,
            confirmatory: Some(FrozenBlock {
                master: 0xA3E1_2026_0818_0005,
                ordinals: 4..28,
            }),
            confirmatory_control: Some(FrozenBlock {
                master: 0xA3E1_2026_0818_0006,
                ordinals: 4..28,
            }),
            canon: "canon 15 (the inverted brake): ending the game while behind is a bad idea — \
                    design §1 family 2",
        },
        // ---- FAMILY 3 -----------------------------------------------------
        // Exclusive-race opponent awareness. EXISTENCE DEMONSTRATION (canon
        // 12/13 + C-A, C-B discounted): it supports a pivot or a routing and
        // NEVER a GO, and its verdict arm skips the failure allowance entirely.
        //
        // The BUILDER is the honest source (design §1 family 3): the generated
        // arm `RootPredicate::ExclusiveRaceOneStep` is a pilot PROBE whose supply
        // at the scan bound is measured and recorded, and a recorded `Exhausted`
        // refusal there is itself the evidence that the builder arm is what the
        // family rests on. Its declared CONTROL is [`RACE_CONTROL_GENERATOR`],
        // read against `DenialNotTaken` of the same declared step.
        //
        // The rule is written with [`DECLARED_BY_THE_ROOT`] and re-pointed per
        // root — the step belongs to the ROOT's declaration.
        FamilyDecl {
            id: "exclusive-race",
            label: FamilyLabel::ExistenceDemonstration,
            class: StrategicClass::OpponentReply,
            generator: FamilyGenerator::Builder {
                build: built_race_positive,
                name: "build_race_positive",
            },
            rule: PassRule::Turn(TurnPredicate::ContestedStepTaken(DECLARED_BY_THE_ROOT)),
            replications: PROVISIONAL_REPLICATIONS,
            per_root_f: PROVISIONAL_PER_ROOT_F,
            menu: REPLY_MENU,
            pilot: PILOT_DRAWS,
            confirmatory: Some(FrozenBlock {
                master: 0xA3E1_2026_0818_0008,
                ordinals: 4..28,
            }),
            confirmatory_control: Some(FrozenBlock {
                master: 0xA3E1_2026_0818_0009,
                ordinals: 4..28,
            }),
            canon: "canon 12/13 (race arithmetic and awareness, never denial-as-default) plus \
                    C-A's participation framing — design §1 family 3",
        },
        // ---- FAMILY 4 -----------------------------------------------------
        // Achievement proximity. CONFIRMATORY on ruling 14's agenda-setters plus
        // C-A. No denial claim is made or needed: BOTH seats can claim the same
        // card, so this family is about the mover's own agenda.
        //
        // Its declared CORNER is [`PROXIMITY_CORNER_GENERATOR`] — F8's
        // produce-then-claim shape, where the claim is TWO decisions away and
        // therefore not a root candidate at all, which is precisely what makes
        // the turn grain necessary here.
        FamilyDecl {
            id: "achievement",
            label: FamilyLabel::Confirmatory,
            class: StrategicClass::EndTimingProximity,
            generator: FamilyGenerator::Predicate(RootPredicate::AchievementClaimAvailable),
            rule: PassRule::Turn(TurnPredicate::AchievementClaimed),
            replications: PROVISIONAL_REPLICATIONS,
            per_root_f: PROVISIONAL_PER_ROOT_F,
            menu: REPLY_MENU,
            pilot: PILOT_DRAWS,
            confirmatory: Some(FrozenBlock {
                master: 0xA3E1_2026_0818_0007,
                ordinals: 4..28,
            }),
            confirmatory_control: Some(FrozenBlock {
                master: 0xA3E1_2026_0818_000A,
                ordinals: 4..28,
            }),
            canon: "ruling 14 (achievements are agenda-setters, denial is not canon) plus C-A — \
                    design §1 family 4",
        },
    ]
}

// ---------------------------------------------------------------------------
// The BLOCK RUN (design §4; plan §T5(B))
// ---------------------------------------------------------------------------

/// Which block of a family's declared draws to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// The pilot draws — the caller supplies (and records) the master.
    Pilot,
    /// The frozen confirmatory draws — the declaration supplies the master, and
    /// a caller-typed one is refused.
    Confirmatory,
}

impl BlockKind {
    /// The stable name, for reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pilot => "pilot",
            Self::Confirmatory => "confirmatory",
        }
    }
}

/// The sizing a block run uses — the parameters the freeze wave priced.
///
/// **K, δ and the dead-spread bound are PARAMETERS and not decl fields**, and
/// the freeze wave is why that stayed the right split: the numbers are a
/// property of the RUN (a pilot and a confirmatory of the same family run at
/// different K), so there are two POSTURES rather than two declarations —
/// [`BlockOpts::pilot`] and [`BlockOpts::freeze`]. The two-argument
/// [`run_family_block`] uses the pilot posture; `driven_runner`'s
/// `family_block_opts` is the seam that picks per block.
#[derive(Debug, Clone)]
pub struct BlockOpts {
    /// Replications per candidate in an ADMISSION adjudication (design §4's
    /// per-policy K; uniform across the menu — 4 in the pilot posture, 8 in the
    /// frozen one).
    pub admission_k: u32,
    /// The admission dominance margin, in victory points.
    pub delta: i64,
    /// [`classify_dead`]'s bound.
    pub dead_spread_bound: i64,
    /// A CONTROL block inverts admission: the pass behaviour must demonstrably
    /// LOSE by δ (family 2's behind-mover control).
    pub control: bool,
    /// The driver options every adjudication runs under.
    pub driven: DrivenOpts,
}

impl BlockOpts {
    /// The PILOT posture — design §4's drafts, UNCHANGED by the freeze wave so
    /// that the pilot runs which already happened reproduce exactly.
    pub fn pilot() -> Self {
        Self {
            admission_k: PILOT_ADMISSION_K,
            delta: DELTA_VP,
            dead_spread_bound: DEAD_SPREAD_BOUND,
            control: false,
            driven: DrivenOpts::default(),
        }
    }

    /// The FROZEN posture — what a CONFIRMATORY block runs (the freeze wave).
    ///
    /// One field moves against [`pilot`](Self::pilot): the per-policy admission
    /// K, 4 → 8, priced from pilot-measured group-mean noise (see
    /// [`FROZEN_ADMISSION_K`]). δ and the dead-spread bound stand, and `control`
    /// is `false` here for the reason it is false in both arms of
    /// `driven_runner::family_block_opts`: the admission inversion is a SEAM's
    /// decision about one run, never a property of a posture.
    pub fn freeze() -> Self {
        Self {
            admission_k: FROZEN_ADMISSION_K,
            delta: DELTA_VP,
            dead_spread_bound: DEAD_SPREAD_BOUND,
            control: false,
            driven: DrivenOpts::default(),
        }
    }
}

impl Default for BlockOpts {
    fn default() -> Self {
        Self::pilot()
    }
}

/// Why a block produced no report.
#[derive(Debug)]
pub enum FamilyRunError {
    /// A confirmatory run of an ARM whose block was never baked. Every declared
    /// arm carries one since the freeze wave, so this is the shape a NEWLY
    /// declared arm has on the day it is written and before it is frozen.
    NoFrozenBlock { id: &'static str },
    /// A confirmatory run carrying a caller's master. Refused rather than
    /// honoured (design §2, review SHOULD-4): a typo must not be able to run a
    /// plausible-looking non-frozen block.
    MasterRefused { id: &'static str },
    /// A pilot run with no master. Recorded-before-use stays the caller's
    /// obligation, so the run cannot invent one.
    MasterRequired { id: &'static str },
    /// The block names no ordinals at all.
    EmptyBlock { id: &'static str },
    /// A generated draw could not be resolved.
    Root { ordinal: u32, error: RootError },
    /// A builder draw refused.
    Builder { ordinal: u32, error: BuilderError },
    /// A step-carrying rule met a root that declares no step — a generator/rule
    /// mismatch, never a read against [`DECLARED_BY_THE_ROOT`].
    StepNotDeclared { ordinal: u32 },
    /// A root declared a step that names no space.
    IllFormedStep { ordinal: u32, step: ContestedStep },
    /// An admission adjudication did not complete.
    Adjudication { ordinal: u32, policy: ReplyPolicy, error: Box<AdjudicationError> },
    /// A read did not complete for a reason that is NOT a counter refusal — a
    /// refusal is a per-root FINDING and rides the row.
    Read { ordinal: u32, error: Box<FamilyReadError> },
}

impl std::fmt::Display for FamilyRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoFrozenBlock { id } => write!(
                f,
                "family {id} carries no frozen confirmatory block for this arm: a block is baked \
                 at the FREEZE commit, and an arm without one can only run `--block pilot`"
            ),
            Self::MasterRefused { id } => write!(
                f,
                "family {id}'s confirmatory block runs the master its declaration carries — a \
                 caller-supplied master is refused so a typo cannot run a plausible-looking \
                 non-frozen block"
            ),
            Self::MasterRequired { id } => write!(
                f,
                "family {id}'s pilot block needs a master: recorded-before-use is the caller's \
                 obligation and this run will not invent one"
            ),
            Self::EmptyBlock { id } => {
                write!(f, "family {id}'s block names no ordinals, so there is nothing to run")
            }
            Self::Root { ordinal, error } => write!(f, "draw {ordinal}: {error}"),
            Self::Builder { ordinal, error } => write!(f, "draw {ordinal}: {error}"),
            Self::StepNotDeclared { ordinal } => write!(
                f,
                "draw {ordinal}: the family's rule names a contested step and this root declares \
                 none — the step is the ROOT's declaration, never the rule's placeholder"
            ),
            Self::IllFormedStep { ordinal, step } => {
                write!(f, "draw {ordinal}: the declared step {step:?} names no space")
            }
            Self::Adjudication { ordinal, policy, error } => write!(
                f,
                "draw {ordinal}: the admission adjudication under {} did not complete: {error}",
                policy.as_str()
            ),
            Self::Read { ordinal, error } => {
                write!(f, "draw {ordinal}: the read did not complete: {error}")
            }
        }
    }
}

impl std::error::Error for FamilyRunError {}

/// Why a drawn root was EXCLUDED from the denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExclusionReason {
    /// Family 1's LIVE/DEAD split: the candidate field is genuinely indifferent
    /// under EVERY declared policy, so a pick there confirms nothing and fails
    /// nothing.
    Dead,
    /// **The decision grain's FORCED field** (the freeze wave): fewer than
    /// [`FORCED_CANDIDATE_FLOOR`] candidates, so the mover had nothing to
    /// prefer. Checked BEFORE `Dead` because a lone WINNING candidate reads
    /// `live` — the pilot's own shape.
    Forced { candidates: usize },
    /// Families 2–4: the pass behaviour does not demonstrably dominate at this
    /// root under every SPLITTING policy. A real measurement, and the rate is
    /// carried on the report.
    NotDemonstrated,
    /// **Families 2–4's degenerate field** (the freeze wave): fewer than
    /// [`MIN_SPLITTING_POLICIES`] policies produced a two-sided comparison at
    /// all, so there is no measurement here to fail. A different finding from
    /// [`NotDemonstrated`](Self::NotDemonstrated), and the count is named.
    TooFewSplitting { splitting: usize, minimum: usize },
}

/// One drawn root's verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootVerdict {
    /// Excluded before any read — see [`ExclusionReason`].
    Excluded(ExclusionReason),
    /// **BRANCH 0**: the counter check refused to summarize this root as a
    /// preference finding. Not a failure — a routing.
    CounterRefused(String),
    /// The subject's reads met the rule within the per-root allowance.
    Pass,
    /// They did not.
    Fail { misses: u32, allowance: u32 },
    /// Family 1 only: the declared policies DISAGREE about this position, so the
    /// pick landed in some top sets and not others. Never a silent fail — it is
    /// the exact-adjudicator trigger.
    UnresolvedPendingExact { replications: u32 },
    /// Family 3's arm: the gap is recorded and no bar is declared.
    GapRecorded { misses: u32 },
}

/// The three-way reading of ONE pick against the declared policies' top sets
/// (design §1's family-1 rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickJudgment {
    /// In EVERY declared policy's top set — the pass.
    InEverySet,
    /// In at least one but not all — UNRESOLVED-pending-exact.
    InSomeSet,
    /// In none — the fail. Also what an EMPTY menu reads, deliberately: a
    /// judgment across no policies is not a pass.
    InNoSet,
}

/// Judge one pick against the per-policy top sets.
pub fn judge_pick(pick: &Move, top_sets: &[(ReplyPolicy, Vec<Move>)]) -> PickJudgment {
    if top_sets.is_empty() {
        return PickJudgment::InNoSet;
    }
    let hits = top_sets.iter().filter(|(_, set)| set.contains(pick)).count();
    if hits == 0 {
        PickJudgment::InNoSet
    } else if hits == top_sets.len() {
        PickJudgment::InEverySet
    } else {
        PickJudgment::InSomeSet
    }
}

/// The candidates whose MEAN margin is within `epsilon` of the best — the top
/// set [`PassRule::AdjudicatedBest`] judges against (design §3).
///
/// Empty on an empty table, which is the honest reading of a finished game and
/// is what makes [`judge_pick`] fail rather than vacuously pass there.
pub fn top_set(adj: &Adjudication, epsilon: i64) -> Vec<Move> {
    let mut best = f64::NEG_INFINITY;
    for row in &adj.rows {
        best = best.max(row.mean_margin());
    }
    if !best.is_finite() {
        return Vec::new();
    }
    let floor = best - epsilon as f64;
    adj.rows
        .iter()
        .filter(|row| row.mean_margin() >= floor)
        .map(|row| row.candidate.clone())
        .collect()
}

/// The family-4 RIDER: the shipped achievement readings at one root, beside a
/// RACE-PROXIMITY reading (design §1 family 4 — D2's named pre-bar trigger test,
/// taken exactly where it was promised).
///
/// **RECORDED, never gated.** Nothing in this phase may move the evaluator (the
/// STOP fence), so the proximity column exists here to be read at rooted
/// positions and carried into the V3 candidate discussion, not to change a
/// verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AchievementRider {
    /// `eval::achievement_progress_sum` — the PRE-FREEZE base, with no claim
    /// filter. Recorded because design §1 names it, and because it is the base
    /// the shipped filter is defined as a filter OVER.
    pub progress_sum: i64,
    /// `eval::achievement_progress_postclaim_sum` — **the SHIPPED feature since
    /// the v2 freeze** (T3's post-claim filter). The two are printed side by
    /// side so a reader can see the filter's effect on-policy, which is the
    /// other half of what design §1 asks this rider for.
    pub postclaim_sum: i64,
    /// The race-proximity reading — see
    /// [`achievement_race_proximity_mirror`].
    pub race_proximity: i64,
}

/// Both shipped achievement readings plus the proximity mirror, for one seat.
pub fn achievement_rider(view: &PlayerView, player: &VisiblePlayer) -> AchievementRider {
    AchievementRider {
        progress_sum: crate::game::eval::achievement_progress_sum(view, player),
        postclaim_sum: crate::game::eval::achievement_progress_postclaim_sum(view, player),
        race_proximity: achievement_race_proximity_mirror(view, player),
    }
}

/// How near this seat is to its NEXT achievement claim, negated so that nearer
/// reads HIGHER.
///
/// **A THIN MIRROR of `achievement_race_proximity` in
/// `crates/yokohama-core/examples/eval_probe.rs`**, and it names its source
/// because that is the rule for mirrored logic in this harness (the Phase-B
/// design's §5 pattern): the column is an EXAMPLE-local candidate, examples are
/// not a library dependency, and family 4's rider has to print it beside the
/// shipped sums. The three decisions the probe records ride the mirror
/// unchanged — a card an OPPONENT claimed stays live (the engine's claim gate
/// refuses only a repeat claim by the SAME seat), the per-card cap is inherited
/// from `achievement_progress`, and a seat that has claimed all three reads 0,
/// which collides with the reading for a seat standing on a claimable card
/// because both are "nothing left to chase".
///
/// It is pinned to a HAND-COMPUTED value on a constructed state, which is what
/// keeps a mirror from drifting into a second arithmetic.
fn achievement_race_proximity_mirror(view: &PlayerView, me: &VisiblePlayer) -> i64 {
    let mut nearest: Option<u32> = None;
    for card in &view.achievements {
        if card.claimed_by.contains(&me.id) {
            continue;
        }
        let (current, needed) =
            crate::game::eval::achievement_progress(view, me, card.achievement_type);
        let gap = needed.saturating_sub(current.min(needed));
        nearest = Some(nearest.map_or(gap, |best: u32| best.min(gap)));
    }
    -i64::from(nearest.unwrap_or(0))
}

/// ONE drawn root's row in a block report.
#[derive(Debug, Clone, PartialEq)]
pub struct RootRow {
    /// Its ordinal in the block's (strided, or seed-blocked) space. Together
    /// with the report's master and generator NAME, this is the draw's identity
    /// — which matters most for a BUILDER draw, whose `RootDescriptor` is
    /// honestly `Constructed` because the state was constructed.
    pub ordinal: u32,
    pub descriptor: RootDescriptor,
    /// The seed a BUILDER draw was built from — the `(ordinal + 1)`-th world of
    /// `SeedSchedule::diagnostic(master)`.
    ///
    /// **Derivable and recorded anyway** (T5 review NOTE-3), which is the
    /// registry's own lesson made a field: a number a reader must re-derive from
    /// a schedule is a number that silently re-points the moment the derivation
    /// moves, and this one identifies a CONSTRUCTED position whose descriptor
    /// (honestly `Constructed`) names nothing else. With it, `build(seed, rules)`
    /// reproduces the exact root a row was read at, from the row alone.
    ///
    /// **`None` for a PREDICATE draw, and that is the honest shape rather than a
    /// gap**: a scanned root has no builder and no input seed of its own — its
    /// identity is the descriptor's `Generated` origin (predicate, master,
    /// ordinal, game index, and the generating ecology), which is complete. The
    /// generating GAME's world seed would be a second, differently-derived
    /// identity sitting beside a sufficient one, so it is deliberately not
    /// recorded here.
    pub draw_seed: Option<u64>,
    /// The step this root DECLARED, for a step-carrying rule.
    pub step: Option<ContestedStep>,
    /// The rule as it was RUN — the declaration's rule re-pointed at this root's
    /// step.
    pub rule: PassRule,
    /// Families 2–4: the admission read across the declared menu.
    pub admission: Option<AdmissionRead>,
    /// Family 1: `classify_dead` per declared policy, in menu order.
    pub dead_by_policy: Vec<(ReplyPolicy, bool)>,
    /// Family 1: DEAD under EVERY declared policy (design §4's NOTE-2 — dead
    /// means dead under all of them, spread bound applied per policy).
    pub dead: bool,
    /// Family 1: the per-policy top sets the picks were judged against.
    pub top_sets: Vec<(ReplyPolicy, Vec<Move>)>,
    /// The read, when the root was admitted and the counter check let it stand.
    pub read: Option<FamilyReadOutcome>,
    /// Replications that missed the rule (turn grain) or landed in NO top set
    /// (decision grain).
    pub misses: u32,
    /// Decision grain: replications whose pick landed in SOME but not all top
    /// sets.
    pub unresolved: u32,
    pub verdict: RootVerdict,
    /// The admission-circularity flag (design §4, review MUST-4).
    pub lookahead_sole_dissenter: bool,
    /// Family 4's recorded rider.
    pub rider: Option<AchievementRider>,
}

/// A family's verdict over a whole block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FamilyVerdict {
    /// Every admitted root passed within the allowances.
    Pass,
    /// More admitted roots failed than the family's allowance permits.
    Fail { failed_roots: usize, allowance: usize },
    /// At least one root's declared policies disagreed. Never a silent fail.
    UnresolvedPendingExact { roots: usize },
    /// **THE VOID RULE** (design §4): a confirmatory result quoting fewer than
    /// [`MIN_ADMITTED_ROOTS`] admitted roots confirms NOTHING, whatever those
    /// roots said. The disposition rides the verdict.
    VoidAsConfirmation {
        admitted: usize,
        drawn: usize,
        minimum: usize,
        disposition: &'static str,
    },
    /// **THE EMPTY DENOMINATOR** (the freeze wave, pilot finding 2): NOTHING was
    /// admitted, so the block read nothing at all.
    ///
    /// It is its own arm rather than a `Pass` with zero failures because that is
    /// exactly what the pilot printed. A PILOT block escapes the VOID rule by
    /// design (it confirms nothing, so it cannot be void AS a confirmation), and
    /// the allowance arm then read "0 failures over 0 admitted roots" as clean.
    NothingAdmitted { drawn: usize },
    /// The EXISTENCE arm: no bar was declared, so the output is the per-root
    /// gap block and the failure allowance does not apply.
    GapDemonstrated { roots: usize },
}

/// The family verdict, as arithmetic over the block's counts (design §4).
///
/// Split out from [`run_family_block_with`] so the rules can be pinned without
/// running a block — the same reason `classify_dead` and `admission` are their
/// own functions. The ORDER of the arms is the semantics: the existence
/// exemption comes first (it skips everything), then the VOID rule (a block that
/// cannot confirm has no verdict to give), then the EMPTY DENOMINATOR, then the
/// allowance, then the unresolved routing.
///
/// **Why the empty denominator sits where it does** (the freeze wave): a
/// confirmatory block below the minimum keeps the VOID wording, which is the
/// stronger statement — it names the floor it fell short of — so the void arm
/// runs first and this arm catches everything else that admitted nothing,
/// PILOTS included. It is deliberately above the allowance and unresolved arms:
/// over an empty denominator those counts are structurally zero, and reading a
/// zero-of-zero as a pass is the pilot defect this arm exists for.
pub fn family_verdict(
    label: FamilyLabel,
    block: BlockKind,
    drawn: usize,
    admitted: usize,
    failed: usize,
    unresolved: usize,
) -> FamilyVerdict {
    if label == FamilyLabel::ExistenceDemonstration {
        return FamilyVerdict::GapDemonstrated { roots: admitted };
    }
    if block == BlockKind::Confirmatory && admitted < MIN_ADMITTED_ROOTS {
        return FamilyVerdict::VoidAsConfirmation {
            admitted,
            drawn,
            minimum: MIN_ADMITTED_ROOTS,
            disposition: VOID_DISPOSITION,
        };
    }
    if admitted == 0 {
        return FamilyVerdict::NothingAdmitted { drawn };
    }
    if failed > FAMILY_FAILURE_ALLOWANCE {
        return FamilyVerdict::Fail { failed_roots: failed, allowance: FAMILY_FAILURE_ALLOWANCE };
    }
    if unresolved > 0 {
        return FamilyVerdict::UnresolvedPendingExact { roots: unresolved };
    }
    FamilyVerdict::Pass
}

/// ONE family block, run and judged.
#[derive(Debug, Clone, PartialEq)]
pub struct FamilyBlockReport {
    pub id: &'static str,
    pub label: FamilyLabel,
    pub class: StrategicClass,
    pub block: BlockKind,
    /// The generator's declared name.
    pub generator: &'static str,
    /// The master every draw, adjudication and read descended from.
    pub master: u64,
    /// What the SUBJECT resolved to — never the seat spec's requested name.
    pub subject_echo: String,
    /// The declaration's rule, before per-root re-pointing.
    pub rule: PassRule,
    pub replications: u32,
    pub per_root_f: u32,
    pub menu: Vec<ReplyPolicy>,
    pub admission_k: u32,
    pub delta: i64,
    pub control: bool,
    pub roots: Vec<RootRow>,
    pub drawn: usize,
    /// Roots that were admitted AND produced a read. A counter-refused root is
    /// admitted but UNREAD — it is a branch-0 routing rather than a reading, so
    /// it is counted separately and never inflates a confirmation's denominator.
    pub admitted: usize,
    pub counter_refused: usize,
    /// Ordinals whose exclusion had `lookahead-at-final` as the SOLE DISSENTER
    /// (design §4, review MUST-4): collected as potential subject-suppression
    /// evidence and routed to D7, never silently dropped.
    pub sole_dissenters: Vec<u32>,
    pub verdict: FamilyVerdict,
    /// The sentence every adjudicated table prints about what it is.
    pub robustness_read: &'static str,
}

/// Run ONE declared block of ONE family (design §4's protocol, as a function).
///
/// The runner owns no execution logic: it parses argv, calls this, and renders
/// the report. See [`run_family_block_with`] for the sizing parameters; this
/// entry point uses [`BlockOpts::pilot`]. **The runner does NOT come through
/// here** — it calls `run_family_block_with` via its own `family_block_opts`
/// seam, so that a confirmatory block runs [`BlockOpts::freeze`] instead.
pub fn run_family_block(
    decl: &FamilyDecl,
    block: BlockKind,
    master_override: Option<u64>,
    subject: SeatSpec,
    rules: &Rules,
) -> Result<FamilyBlockReport, FamilyRunError> {
    run_family_block_with(decl, block, master_override, subject, rules, BlockOpts::pilot())
}

/// [`run_family_block`] with the sizing parameters made explicit.
///
/// **The order of operations is design §4's and the order matters**: resolve the
/// block's draws in ONE pass; adjudicate each root under the whole declared menu
/// (with the family's turn predicate as the hook, at the turn grain); apply
/// admission (`classify_dead` at the decision grain, `admission` at the turn
/// grain), AND-ing the verdict across the menu; read the SUBJECT only at
/// admitted roots; then judge.
///
/// **ONE MASTER runs the whole block** — draws, adjudications and reads. That is
/// sound because the instruments mix under DIFFERENT salts
/// (`SALT_DRIVEN_REPLICATION` for adjudication worlds,
/// [`SALT_DRIVEN_FAMILY_READ`] for read seeds), so no read shares a world with
/// the adjudication that admitted its root by accident of arithmetic.
///
/// **COST**, stated because nothing here bounds it: `roots × menu × candidates ×
/// K` whole games for admission, plus `roots × R` turns of the subject for the
/// reads. At the design's own sizing that is tens of minutes per root in
/// release. The pilots priced it, and the freeze then DOUBLED K on that
/// evidence (the whole ten-command pilot ran in about three minutes), so a
/// confirmatory block costs about twice its pilot per root.
pub fn run_family_block_with(
    decl: &FamilyDecl,
    block: BlockKind,
    master_override: Option<u64>,
    subject: SeatSpec,
    rules: &Rules,
    opts: BlockOpts,
) -> Result<FamilyBlockReport, FamilyRunError> {
    let (master, ordinals) = resolve_block(decl, block, master_override)?;
    let draws = resolve_draws(decl, master, &ordinals, rules)?;

    let mut roots: Vec<RootRow> = Vec::with_capacity(draws.len());
    for draw in draws {
        roots.push(run_one_root(decl, master, subject, rules, &opts, draw)?);
    }

    let drawn = roots.len();
    let admitted = roots.iter().filter(|r| r.read.is_some()).count();
    let counter_refused =
        roots.iter().filter(|r| matches!(r.verdict, RootVerdict::CounterRefused(_))).count();
    let failed = roots.iter().filter(|r| matches!(r.verdict, RootVerdict::Fail { .. })).count();
    let unresolved = roots
        .iter()
        .filter(|r| matches!(r.verdict, RootVerdict::UnresolvedPendingExact { .. }))
        .count();
    let sole_dissenters =
        roots.iter().filter(|r| r.lookahead_sole_dissenter).map(|r| r.ordinal).collect();

    Ok(FamilyBlockReport {
        id: decl.id,
        label: decl.label,
        class: decl.class,
        block,
        generator: decl.generator.name(),
        master,
        subject_echo: subject.echo(),
        rule: decl.rule,
        replications: decl.replications,
        per_root_f: decl.per_root_f,
        menu: decl.menu.to_vec(),
        admission_k: opts.admission_k,
        delta: opts.delta,
        control: opts.control,
        verdict: family_verdict(decl.label, block, drawn, admitted, failed, unresolved),
        roots,
        drawn,
        admitted,
        counter_refused,
        sole_dissenters,
        robustness_read: crate::game::driven::ROBUSTNESS_READ,
    })
}

/// The master and ordinals a block runs — and the three refusals that guard
/// them.
///
/// **`pub(crate)` since the freeze wave, for a stated reason**: before it, the
/// refusals could be pinned through the public [`run_family_block`] because a
/// confirmatory run had no Ok path at all. It has one now, and it is hours of
/// real games — so the ARGUMENT contract (which master, which ordinals, and the
/// three refusals) is pinned here, where it costs nothing, and the runner's own
/// arm cells reach it the same way.
///
/// The CONFIRMATORY arm reads `decl.confirmatory` and nothing else:
/// `driven_runner::control_arm` re-points that field from
/// [`FamilyDecl::confirmatory_control`], so this stays one rule over one field
/// rather than a second rule that has to know which arm it is serving.
pub(crate) fn resolve_block(
    decl: &FamilyDecl,
    block: BlockKind,
    master_override: Option<u64>,
) -> Result<(u64, Vec<u32>), FamilyRunError> {
    let (master, range) = match block {
        BlockKind::Pilot => (
            master_override.ok_or(FamilyRunError::MasterRequired { id: decl.id })?,
            decl.pilot.clone(),
        ),
        BlockKind::Confirmatory => {
            if master_override.is_some() {
                return Err(FamilyRunError::MasterRefused { id: decl.id });
            }
            let frozen = decl
                .confirmatory
                .as_ref()
                .ok_or(FamilyRunError::NoFrozenBlock { id: decl.id })?;
            (frozen.master, frozen.ordinals.clone())
        }
    };
    let ordinals: Vec<u32> = range.collect();
    if ordinals.is_empty() {
        return Err(FamilyRunError::EmptyBlock { id: decl.id });
    }
    Ok((master, ordinals))
}

/// One resolved draw.
struct Draw {
    ordinal: u32,
    state: GameState,
    descriptor: RootDescriptor,
    /// The builder's input seed, when the draw came from a builder — see
    /// [`RootRow::draw_seed`], which this becomes verbatim.
    draw_seed: Option<u64>,
    step: Option<ContestedStep>,
}

/// Resolve the whole block's draws in ONE pass (plan review NOTE-8).
///
/// A PREDICATE block goes through [`crate::game::driven::generate_roots`], which
/// walks the generated games once for the whole block rather than once per
/// ordinal — at the v2 lookahead ecology, where both generating seats search,
/// that difference is the block's entire cost. A BUILDER block draws its seeds
/// from `SeedSchedule::diagnostic(master)`, the SAME stream the scan draws its
/// games from, so a builder block and a scan block at one master are two blocks
/// of one seeded sequence rather than two unrelated conventions.
fn resolve_draws(
    decl: &FamilyDecl,
    master: u64,
    ordinals: &[u32],
    rules: &Rules,
) -> Result<Vec<Draw>, FamilyRunError> {
    match decl.generator {
        FamilyGenerator::Predicate(predicate) => {
            let resolved = crate::game::driven::generate_roots(predicate, master, ordinals, rules)
                .map_err(|error| {
                    // The refusal already names the ordinal it could not supply.
                    let ordinal = match &error {
                        RootError::Exhausted { ordinal, .. } => *ordinal,
                        _ => ordinals[0],
                    };
                    FamilyRunError::Root { ordinal, error }
                })?;
            Ok(ordinals
                .iter()
                .copied()
                .zip(resolved)
                .map(|(ordinal, (state, descriptor))| Draw {
                    ordinal,
                    state,
                    descriptor,
                    // A scanned root has no builder seed — see
                    // [`RootRow::draw_seed`] for why that is the honest shape.
                    draw_seed: None,
                    step: None,
                })
                .collect())
        }
        FamilyGenerator::Builder { build, .. } => {
            let highest = ordinals.iter().copied().max().unwrap_or(0);
            let mut schedule = SeedSchedule::diagnostic(master);
            let seeds: Vec<u64> =
                (0..=highest).map(|_| schedule.next_world().0).collect();
            ordinals
                .iter()
                .copied()
                .map(|ordinal| {
                    let draw_seed = seeds[ordinal as usize];
                    let built = build(draw_seed, rules)
                        .map_err(|error| FamilyRunError::Builder { ordinal, error })?;
                    let descriptor = RootDescriptor::constructed(&built.state);
                    Ok(Draw {
                        ordinal,
                        state: built.state,
                        descriptor,
                        draw_seed: Some(draw_seed),
                        step: built.step,
                    })
                })
                .collect()
        }
    }
}

/// The seats an admission adjudication runs — the SAME spec at every seat, which
/// is the harness's own convention (`driven_runner::seats_or_default`).
///
/// Every seat, not just the opponent's, and that is what makes MUST-4's
/// sole-dissenter reading work: the `lookahead-at-final` arm IS the subject's own
/// agent, so a subject defect that degrades post-candidate play shows up in that
/// arm's playouts and in no other.
fn menu_seats(policy: ReplyPolicy, players: usize) -> Vec<SeatSpec> {
    vec![SeatSpec::Reply(policy); players]
}

/// Adjudicate, admit, read and judge ONE drawn root.
fn run_one_root(
    decl: &FamilyDecl,
    master: u64,
    subject: SeatSpec,
    rules: &Rules,
    opts: &BlockOpts,
    draw: Draw,
) -> Result<RootRow, FamilyRunError> {
    let Draw { ordinal, state, descriptor, draw_seed, step } = draw;
    if decl.rule.names_a_step() {
        match step {
            None => return Err(FamilyRunError::StepNotDeclared { ordinal }),
            Some(s) if !s.well_formed() => {
                return Err(FamilyRunError::IllFormedStep { ordinal, step: s })
            }
            Some(_) => {}
        }
    }
    let rule = decl.rule.with_step(step);
    let source =
        RootSource::Resolved { state: state.clone(), descriptor: descriptor.clone() };
    let players = state.players.len();
    let world = WorldSeed(master);

    let mut dead_by_policy: Vec<(ReplyPolicy, bool)> = Vec::new();
    let mut top_sets: Vec<(ReplyPolicy, Vec<Move>)> = Vec::new();
    let mut admission_read: Option<AdmissionRead> = None;
    let mut dead = false;
    // The exclusion this root would carry if it is not admitted — set by
    // whichever branch measured it, so a reason is never inferred after the fact.
    let mut exclusion = ExclusionReason::NotDemonstrated;

    let admitted = match rule {
        PassRule::AdjudicatedBest { epsilon } => {
            let mut candidates_by_policy: Vec<(ReplyPolicy, usize)> = Vec::new();
            for &policy in decl.menu {
                let adj = adjudicate_root(
                    &source,
                    &menu_seats(policy, players),
                    rules,
                    world,
                    opts.admission_k,
                    opts.driven,
                )
                .map_err(|error| FamilyRunError::Adjudication {
                    ordinal,
                    policy,
                    error: Box::new(error),
                })?;
                candidates_by_policy.push((policy, adj.candidates()));
                dead_by_policy.push((policy, classify_dead(&adj, opts.dead_spread_bound)));
                top_sets.push((policy, top_set(&adj, epsilon)));
            }
            // DEAD is judged PER POLICY and AND-ed across the menu (plan review
            // NOTE-2): a root the reply menu disagrees about is not indifferent.
            // The AND is `dead_across_menu`'s, so it is pinnable on mixed
            // vectors that no affordable block can produce.
            dead = dead_across_menu(&dead_by_policy);
            // FORCED FIRST (the freeze wave): a one-row field is not a
            // preference at all, and a lone WINNING candidate reads `live`, so
            // leaving this to the dead classifier admits it — which is what the
            // pilot did.
            match forced_across_menu(&candidates_by_policy) {
                Some(candidates) => {
                    exclusion = ExclusionReason::Forced { candidates };
                    false
                }
                None => {
                    if dead {
                        exclusion = ExclusionReason::Dead;
                    }
                    !dead
                }
            }
        }
        PassRule::Turn(predicate) => {
            let mut adjs: Vec<(ReplyPolicy, Adjudication)> = Vec::new();
            for &policy in decl.menu {
                let hook = |root: &GameState, handoff: &GameState| {
                    predicate.eval(root, handoff, rules)
                };
                let adj = adjudicate_root_with(
                    &source,
                    &menu_seats(policy, players),
                    rules,
                    world,
                    opts.admission_k,
                    opts.driven,
                    Some(&hook),
                )
                .map_err(|error| FamilyRunError::Adjudication {
                    ordinal,
                    policy,
                    error: Box::new(error),
                })?;
                adjs.push((policy, adj));
            }
            let read = admission(&adjs, opts.delta, opts.control);
            let ok = read.admitted();
            if !ok {
                // The two exclusions are DIFFERENT findings (the freeze wave):
                // too few policies produced a comparison at all, or the
                // comparisons they produced did not dominate.
                let splitting = read.splitting_count();
                if splitting < MIN_SPLITTING_POLICIES {
                    exclusion = ExclusionReason::TooFewSplitting {
                        splitting,
                        minimum: MIN_SPLITTING_POLICIES,
                    };
                }
            }
            admission_read = Some(read);
            ok
        }
    };

    let lookahead_sole_dissenter =
        admission_read.as_ref().is_some_and(AdmissionRead::lookahead_sole_dissenter);

    // Family 4's RIDER is recorded at every root of that family, admitted or
    // not — it is evidence about the family's own question rather than an input
    // to the verdict.
    let rider = (decl.rule == PassRule::Turn(TurnPredicate::AchievementClaimed))
        .then(|| {
            let mover = state.current_player;
            let view = observe(&state, mover).ok()?;
            let me = view.players.iter().find(|p| p.id == mover)?;
            Some(achievement_rider(&view, me))
        })
        .flatten();

    let mut row = RootRow {
        ordinal,
        descriptor,
        draw_seed,
        step,
        rule,
        admission: admission_read,
        dead_by_policy,
        dead,
        top_sets,
        read: None,
        misses: 0,
        unresolved: 0,
        verdict: RootVerdict::Excluded(exclusion),
        lookahead_sole_dissenter,
        rider,
    };
    if !admitted {
        return Ok(row);
    }

    let outcome =
        match family_read(&state, subject, &rule, decl.replications, master, rules) {
            Ok(outcome) => outcome,
            Err(err @ FamilyReadError::CounterRefusal { .. }) => {
                row.verdict = RootVerdict::CounterRefused(err.to_string());
                return Ok(row);
            }
            Err(error) => {
                return Err(FamilyRunError::Read { ordinal, error: Box::new(error) })
            }
        };

    match rule.grain() {
        ReadGrain::Turn => {
            row.misses = outcome.misses().unwrap_or(0);
        }
        ReadGrain::Decision => {
            // THE THREE-WAY JUDGMENT (design §1): per replication, against every
            // declared policy's top set.
            for replication in &outcome.replications {
                let Some(pick) = replication.pick.as_ref() else {
                    continue;
                };
                match judge_pick(pick, &row.top_sets) {
                    PickJudgment::InEverySet => {}
                    PickJudgment::InSomeSet => row.unresolved += 1,
                    PickJudgment::InNoSet => row.misses += 1,
                }
            }
        }
    }

    row.verdict = if decl.label == FamilyLabel::ExistenceDemonstration {
        // The EXISTENCE arm declares no bar (design §1 family 3): the output is
        // the per-root gap, and the failure allowance never applies.
        RootVerdict::GapRecorded { misses: row.misses }
    } else if row.misses > decl.per_root_f {
        RootVerdict::Fail { misses: row.misses, allowance: decl.per_root_f }
    } else if row.unresolved > 0 {
        RootVerdict::UnresolvedPendingExact { replications: row.unresolved }
    } else {
        RootVerdict::Pass
    };
    row.read = Some(outcome);
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::arena::SALT_DRIVEN_REPLICATION;
    use crate::game::driven::{
        adjudicate_root, adjudicate_root_with, AdjudicationProvenance, AdjudicationSeat,
        CandidatePlayout, CandidateRow, RootDescriptor, RootOrigin, RootSource, UnsupportedReason,
    };
    use crate::game::lookahead::SearchStats;
    use crate::game::semantics::SEMANTICS_REV;
    use crate::game::seeds::WorldSeed;
    use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
    use crate::types::{
        AchievementCard, ChurchSlot, CustomsSlot, GameConfig, Move, TechCardId, TurnPhase,
    };

    /// This task's committed diagnostic master. The NAME of the constructor is
    /// the waiver, per `game::seeds`: an asserting cell wants reproducibility,
    /// and a rooted fixture is exactly the diagnostic case spec §3 carves out.
    const MASTER: u64 = 0x0C04_2026_0818_9990;

    fn rules() -> Rules {
        Rules::standard()
    }

    /// A plain 2P post-setup state — the cheapest legal root there is.
    fn post_setup_2p(rules: &Rules, seed: u64) -> GameState {
        let pending = new_game(GameConfig::for_player_count(2), rules, seed)
            .expect("new_game builds a 2P setup");
        finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)])
            .expect("the setup finalizes")
    }

    /// Two rung seats — the cheapest honest reply there is, and the right choice
    /// wherever the property under test is STRUCTURAL (design §6's wall
    /// discipline).
    fn rung_seats() -> [SeatSpec; 2] {
        [SeatSpec::Reply(ReplyPolicy::FamilyWeighted), SeatSpec::Reply(ReplyPolicy::FamilyWeighted)]
    }

    /// The lowest-id technology whose printed industry cost is exactly `cost`.
    ///
    /// Read off the registry in ID ORDER rather than hard-coded, so a data
    /// change moves the fixture instead of silently invalidating its arithmetic.
    fn tech_with_industry(rules: &Rules, cost: u8) -> TechCardId {
        rules
            .techs
            .iter_sorted_by_id()
            .into_iter()
            .find(|card| card.industry_cost == cost)
            .unwrap_or_else(|| panic!("the standard tech registry has no industry-{cost} card"))
            .id
    }

    // =======================================================================
    // The turn predicates (design §3)
    // =======================================================================

    /// [`TurnPredicate::EndFired`] and its negation SPLIT the same pair, and
    /// each reads the HAND-OFF's flag.
    ///
    /// The inversion control is the second half: on the flipped pair the two
    /// verdicts swap, so a predicate hard-wired to either constant fails one of
    /// the four assertions.
    #[test]
    fn the_end_fired_predicate_and_its_negation_split_the_same_pair() {
        let rules = rules();
        let root = post_setup_2p(&rules, 0x11);
        assert!(!root.end_game_triggered, "anti-vacuity: the root must start untriggered");
        let mut fired = root.clone();
        fired.end_game_triggered = true;

        assert!(TurnPredicate::EndFired.eval(&root, &fired, &rules));
        assert!(!TurnPredicate::NotEndFired.eval(&root, &fired, &rules));

        // INVERSION: the same pair with the hand-off untriggered.
        assert!(!TurnPredicate::EndFired.eval(&root, &root, &rules));
        assert!(TurnPredicate::NotEndFired.eval(&root, &root, &rules));
    }

    /// [`TurnPredicate::AchievementClaimed`] reads the MOVER's own set, and
    /// reads it by STRICT CONTAINMENT rather than by size.
    ///
    /// Four cells, each a different way the naive reading goes wrong: the
    /// mover's set grew (TRUE), nothing grew (FALSE — the inversion control),
    /// the OPPONENT's set grew (FALSE — the mover-identity trap), and a set that
    /// gained two while LOSING one (FALSE — the size-compare trap, which a
    /// `len() >` implementation alone would call true).
    #[test]
    fn the_achievement_predicate_reads_the_movers_own_set_and_reads_it_strictly() {
        let rules = rules();
        let root = post_setup_2p(&rules, 0x12);
        let mover = root.current_player;
        let rival = root.players.iter().map(|p| p.id).find(|id| *id != mover).expect("a rival");
        let types = [
            AchievementType::Money10,
            AchievementType::Tea7,
            AchievementType::Seafood7,
        ];
        let table = |claims: &[(usize, PlayerId)]| {
            let mut state = root.clone();
            state.achievements = [
                AchievementCard::new(types[0]),
                AchievementCard::new(types[1]),
                AchievementCard::new(types[2]),
            ];
            for (index, player) in claims {
                state.achievements[*index].claimed_by.push(*player);
            }
            state
        };

        let none = table(&[]);
        let mover_one = table(&[(0, mover)]);
        let mover_two = table(&[(0, mover), (1, mover)]);
        let rival_one = table(&[(0, rival)]);
        // Same COUNT growth as `mover_two` over `mover_one`, but the root's own
        // claim is gone — the strictness discriminator.
        let swapped_two = table(&[(1, mover), (2, mover)]);

        assert!(TurnPredicate::AchievementClaimed.eval(&none, &mover_one, &rules));
        assert!(TurnPredicate::AchievementClaimed.eval(&mover_one, &mover_two, &rules));
        // INVERSION: no growth at all.
        assert!(!TurnPredicate::AchievementClaimed.eval(&mover_one, &mover_one, &rules));
        // The MOVER-IDENTITY trap: the rival claimed, the mover did not.
        assert!(!TurnPredicate::AchievementClaimed.eval(&none, &rival_one, &rules));
        // The SIZE-COMPARE trap: two claims at the hand-off, one at the root,
        // and the root's own claim is not among them.
        assert_eq!(swapped_two.achievements.iter().filter(|c| c.claimed_by.contains(&mover)).count(), 2);
        assert!(!TurnPredicate::AchievementClaimed.eval(&mover_one, &swapped_two, &rules));
    }

    /// Put `player`'s assistant on the church space `faith`, keeping the
    /// twenty-assistant accounting coherent (hand → church).
    fn occupy_church(state: &mut GameState, faith: u8, player: PlayerId) {
        let index = ChurchTrack::faith_to_index(faith).expect("a real faith value");
        state.church_track.slots[index] = Some(ChurchSlot { player, arrival_order: 0 });
        let p = state.player_mut(player);
        assert!(p.assistants.in_hand >= 1, "the fixture needs an assistant to place");
        p.assistants.in_hand -= 1;
        p.assistants.on_church += 1;
    }

    /// The same, for a customs space named by its PRINTED NUMBER.
    fn occupy_customs(state: &mut GameState, space: u8, player: PlayerId) {
        let index = CustomsTrack::space_to_index(space).expect("a real space number");
        state.customs_track.slots[index] = Some(CustomsSlot { player, arrival_order: 0 });
        let p = state.player_mut(player);
        assert!(p.assistants.in_hand >= 1, "the fixture needs an assistant to place");
        p.assistants.in_hand -= 1;
        p.assistants.on_customs += 1;
    }

    /// A CHURCH step is held only by ITS OWN occupant, at ITS OWN space.
    ///
    /// Faith 5 rather than 7: at 2P the faith-7 space is the setup dummy's and
    /// an invariant says so, which would make the fixture illegal rather than
    /// merely wrong.
    #[test]
    fn the_contested_church_step_is_held_only_by_its_own_occupant() {
        let rules = rules();
        let root = post_setup_2p(&rules, 0x13);
        let mover = root.current_player;
        let rival = root.players.iter().map(|p| p.id).find(|id| *id != mover).expect("a rival");
        let step = ContestedStep::ChurchSpace { faith: 5 };

        let mut mine = root.clone();
        occupy_church(&mut mine, 5, mover);
        crate::game::invariants::check_invariants(&mine, &rules).expect("a legal fixture");
        let mut theirs = root.clone();
        occupy_church(&mut theirs, 5, rival);
        let mut elsewhere = root.clone();
        occupy_church(&mut elsewhere, 6, mover);

        assert!(TurnPredicate::ContestedStepTaken(step).eval(&root, &mine, &rules));
        // INVERSION and the two ways it can be wrong.
        assert!(!TurnPredicate::ContestedStepTaken(step).eval(&root, &root, &rules));
        assert!(!TurnPredicate::ContestedStepTaken(step).eval(&root, &theirs, &rules));
        assert!(!TurnPredicate::ContestedStepTaken(step).eval(&root, &elsewhere, &rules));
    }

    /// A CUSTOMS step is named by its PRINTED SPACE NUMBER (1..=7), not by its
    /// slot index (0..=6) — the off-by-one the plan sketch's `index` field would
    /// have shipped.
    ///
    /// The discriminator is the pair: declaring space 3 must read the occupant
    /// of slot 2, and must NOT read the occupant of slot 3 (which is space 4 —
    /// and at 2P is the dummy's, so the fixture uses spaces 2 and 3 instead).
    #[test]
    fn the_contested_customs_step_is_named_by_its_printed_space_not_its_slot_index() {
        let rules = rules();
        let root = post_setup_2p(&rules, 0x14);
        let mover = root.current_player;
        let step = ContestedStep::CustomsSpace { space: 3 };

        let mut at_space_3 = root.clone();
        occupy_customs(&mut at_space_3, 3, mover);
        crate::game::invariants::check_invariants(&at_space_3, &rules).expect("a legal fixture");
        assert!(
            at_space_3.customs_track.slots[2].is_some(),
            "space 3 is slot INDEX 2 — the fixture's own premise",
        );
        let mut at_space_2 = root.clone();
        occupy_customs(&mut at_space_2, 2, mover);

        assert!(TurnPredicate::ContestedStepTaken(step).eval(&root, &at_space_3, &rules));
        // The INDEX reading would find the mover here (slot 3) and must not.
        assert!(!TurnPredicate::ContestedStepTaken(step).eval(&root, &at_space_2, &rules));
        assert!(!TurnPredicate::ContestedStepTaken(step).eval(&root, &root, &rules));
    }

    /// The TECH-MAJORITY step compares DOUBLED industry value, STRICTLY past the
    /// declared threshold — the F6 arithmetic, in the unit
    /// `calculate_technology_bonus` itself compares in.
    ///
    /// The canonical threshold is the dummy's 15 (D5's 7.5 doubled), which is
    /// exactly the level a real total can never tie: industry 7 doubles to 14
    /// (below) and industry 8 doubles to 16 (past).
    ///
    /// That unrepresentable tie is why this test also carries an EVEN-threshold
    /// pair. Against 15 the comparison is UNOBSERVABLE — every real doubled
    /// total is even, so `>` and `>=` agree on every fixture the canonical cells
    /// can build, and a weakening would pass unseen. An even threshold is the
    /// only shape that separates them, and STRICTNESS is this arm's named
    /// semantic: at equal doubled value the clockwise tie-break decides rank, so
    /// a `>=` read would over-claim.
    #[test]
    fn the_tech_majority_step_reads_doubled_industry_value_strictly_past_the_threshold() {
        let rules = rules();
        let root = post_setup_2p(&rules, 0x15);
        let mover = root.current_player;
        let step = ContestedStep::TechMajorityRankPast { doubled_threshold: 15 };

        let with = |costs: &[u8]| {
            let mut state = root.clone();
            let p = state.player_mut(mover);
            p.technologies = costs.iter().map(|c| tech_with_industry(&rules, *c)).collect();
            state
        };
        let below = with(&[3, 4]); // 7 -> doubled 14
        let past = with(&[4, 4]); // 8 -> doubled 16

        assert!(TurnPredicate::ContestedStepTaken(step).eval(&root, &past, &rules));
        assert!(!TurnPredicate::ContestedStepTaken(step).eval(&root, &below, &rules));
        // INVERSION: a mover owning no technology at all.
        assert!(!TurnPredicate::ContestedStepTaken(step).eval(&root, &root, &rules));

        // STRICTNESS, on an EVEN threshold — the only shape a real total can
        // MEET rather than clear, and so the only shape that tells `>` from
        // `>=`. The two cells above are blind to that difference by
        // construction, which is what this pair repairs.
        let even = ContestedStep::TechMajorityRankPast { doubled_threshold: 14 };
        assert!(
            !TurnPredicate::ContestedStepTaken(even).eval(&root, &below, &rules),
            "doubled 14 TIES the threshold 14 and does not stand PAST it — a \
             `>=` read here would claim a rank the clockwise tie-break decides",
        );
        assert!(TurnPredicate::ContestedStepTaken(even).eval(&root, &past, &rules));
    }

    /// The DENIAL control is the exact negation of the taken rule, on the same
    /// fixtures — pinned as a relation rather than restated, so the two can
    /// never drift apart.
    #[test]
    fn the_denial_control_is_the_negation_of_the_taken_rule_on_the_same_fixtures() {
        let rules = rules();
        let root = post_setup_2p(&rules, 0x16);
        let mover = root.current_player;
        let step = ContestedStep::ChurchSpace { faith: 6 };
        let mut held = root.clone();
        occupy_church(&mut held, 6, mover);

        for handoff in [&root, &held] {
            assert_eq!(
                TurnPredicate::DenialNotTaken(step).eval(&root, handoff, &rules),
                !TurnPredicate::ContestedStepTaken(step).eval(&root, handoff, &rules),
                "the control is not the negation of the positive rule",
            );
        }
        // Anti-vacuity: the two fixtures really do answer differently.
        assert!(TurnPredicate::DenialNotTaken(step).eval(&root, &root, &rules));
        assert!(!TurnPredicate::DenialNotTaken(step).eval(&root, &held, &rules));
    }

    /// An ILL-FORMED step names no space — so the positive arm reads FALSE and
    /// the DENIAL arm passes VACUOUSLY, which is the asymmetry
    /// [`ContestedStep::well_formed`] exists for.
    ///
    /// The pin states the trap rather than hiding it: a declaration site that
    /// skipped `well_formed` could ship a control root that every subject
    /// passes.
    #[test]
    fn an_ill_formed_step_names_no_space_and_the_denial_arm_passes_vacuously() {
        let rules = rules();
        let root = post_setup_2p(&rules, 0x17);

        for (step, formed) in [
            (ContestedStep::ChurchSpace { faith: 4 }, true),
            (ContestedStep::ChurchSpace { faith: 11 }, true),
            (ContestedStep::ChurchSpace { faith: 3 }, false),
            (ContestedStep::ChurchSpace { faith: 12 }, false),
            (ContestedStep::CustomsSpace { space: 1 }, true),
            (ContestedStep::CustomsSpace { space: 7 }, true),
            (ContestedStep::CustomsSpace { space: 0 }, false),
            (ContestedStep::CustomsSpace { space: 8 }, false),
            (ContestedStep::TechMajorityRankPast { doubled_threshold: 15 }, true),
        ] {
            assert_eq!(step.well_formed(), formed, "{step:?} is misclassified");
        }

        let bad = ContestedStep::CustomsSpace { space: 8 };
        assert!(!TurnPredicate::ContestedStepTaken(bad).eval(&root, &root, &rules));
        assert!(
            TurnPredicate::DenialNotTaken(bad).eval(&root, &root, &rules),
            "the vacuous pass is the documented consequence — and the reason a \
             declaration site checks well_formed()",
        );
    }

    /// Every predicate names itself, distinctly and in kebab case — a report's
    /// vocabulary, pinned so a freeze document and the code cannot disagree.
    #[test]
    fn the_turn_predicates_name_themselves_distinctly() {
        let step = ContestedStep::ChurchSpace { faith: 5 };
        let names: Vec<&str> = [
            TurnPredicate::EndFired,
            TurnPredicate::NotEndFired,
            TurnPredicate::AchievementClaimed,
            TurnPredicate::ContestedStepTaken(step),
            TurnPredicate::DenialNotTaken(step),
        ]
        .iter()
        .map(TurnPredicate::as_str)
        .collect();
        for name in &names {
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "`{name}` is not kebab case",
            );
        }
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "two predicates share a name");

        assert_eq!(PassRule::Turn(TurnPredicate::EndFired).grain(), ReadGrain::Turn);
        assert_eq!(PassRule::AdjudicatedBest { epsilon: 1 }.grain(), ReadGrain::Decision);
        assert_eq!(ReadGrain::Turn.as_str(), "turn");
        assert_eq!(ReadGrain::Decision.as_str(), "decision");
    }

    // =======================================================================
    // The adjudicator hook (design §2, plan MUST-3b)
    // =======================================================================

    /// A 2P root at `AdditionalActionSecond` where the mover can claim exactly
    /// ONE achievement, and where declining ends its turn.
    ///
    /// Shaped so both admission groups are guaranteed rather than hoped for:
    /// `FulfillAchievement` grows the mover's claimed set inside the turn, and
    /// `EndAdditionalPhase` from the SECOND additional-action window ends the
    /// turn outright — so the claim row's verdicts are TRUE and the decline
    /// row's are FALSE, whatever the reply policy does afterwards.
    ///
    /// The three achievement cards are doctored to types only ONE of which the
    /// mover satisfies (10 Yen), with every good zeroed for both seats, so the
    /// candidate list is exactly two moves wide.
    fn claim_first_root(rules: &Rules, seed: u64) -> (GameState, PlayerId) {
        let mut state = post_setup_2p(rules, seed);
        let mover = state.current_player;
        state.achievements = [
            AchievementCard::new(AchievementType::Money10),
            AchievementCard::new(AchievementType::Tea7),
            AchievementCard::new(AchievementType::Seafood7),
        ];
        for p in state.players.iter_mut() {
            p.money = 0;
            p.copper = 0;
            p.silk = 0;
            p.tea = 0;
            p.seafood = 0;
            p.imported_goods = 0;
        }
        state.player_mut(mover).money = 10;
        state.turn_phase = TurnPhase::AdditionalActionSecond;
        state.pending_choices.clear();
        crate::game::invariants::check_invariants(&state, rules)
            .expect("the doctored root must be a state the engine accepts");
        assert!(state.player(mover).assistants.in_hand >= 1, "a claim needs an assistant");
        (state, mover)
    }

    /// **THE CLAIM-FIRST TRAP** (plan MUST-3b): a playout whose FORCED CANDIDATE
    /// IS THE CLAIM must classify predicate-TRUE.
    ///
    /// The candidate is decision ZERO of the mover's turn, so the turn DID claim
    /// — and the only way to see that is to compare the hand-off against the
    /// PRE-CANDIDATE root. Evaluated against the post-candidate state instead,
    /// the claimed set has already grown on both sides, the row reads FALSE, and
    /// family 4's dominance partition inverts: the behaviour the family exists to
    /// confirm would be scored as the losing group.
    ///
    /// The in-test discriminator is the third assertion — the WRONG first
    /// argument really does answer differently on this very fixture, so the pin
    /// separates the two readings rather than merely exercising one.
    #[test]
    fn the_claim_first_playout_is_a_true_verdict_not_a_false_one() {
        let rules = rules();
        let (root, mover) = claim_first_root(&rules, 0x21);
        let claim = Move::FulfillAchievement { achievement: AchievementType::Money10 };
        let legal = crate::game::legal_moves::get_legal_moves(&root, &rules);
        assert!(legal.contains(&claim), "the fixture must offer the claim: {legal:?}");
        assert!(legal.contains(&Move::EndAdditionalPhase), "and the decline: {legal:?}");

        let adj = adjudicate_root_with(
            &RootSource::Constructed(root.clone()),
            &rung_seats(),
            &rules,
            WorldSeed(MASTER),
            2,
            DrivenOpts::default(),
            Some(&|r: &GameState, h: &GameState| {
                TurnPredicate::AchievementClaimed.eval(r, h, &rules)
            }),
        )
        .expect("the fixture adjudicates");

        let claim_row = adj
            .rows
            .iter()
            .find(|row| row.candidate == claim)
            .expect("the claim is a candidate");
        assert!(
            claim_row.playouts.iter().all(|p| p.turn_verdict == Some(true)),
            "a playout whose forced candidate IS the claim must classify TRUE: {:?}",
            claim_row.playouts.iter().map(|p| p.turn_verdict).collect::<Vec<_>>(),
        );
        let decline_row = adj
            .rows
            .iter()
            .find(|row| row.candidate == Move::EndAdditionalPhase)
            .expect("the decline is a candidate");
        assert!(
            decline_row.playouts.iter().all(|p| p.turn_verdict == Some(false)),
            "anti-vacuity: ending the turn without claiming must classify FALSE",
        );

        // THE DISCRIMINATOR: the post-candidate state — what a hook handed the
        // wrong first argument would compare against — answers FALSE here.
        let after = crate::game::apply_move::apply_move(&root, &claim, &rules)
            .expect("the claim applies");
        assert!(
            after.achievements.iter().any(|c| c.claimed_by.contains(&mover)),
            "the fixture's premise: the candidate itself grows the claimed set",
        );
        assert!(
            !TurnPredicate::AchievementClaimed.eval(&after, &after, &rules),
            "the fixture does not separate the two readings",
        );
    }

    // =======================================================================
    // The DEAD-root classifier (design §1, review MUST-1)
    // =======================================================================

    /// One hand-built playout carrying nothing but its margin and its verdict —
    /// [`classify_dead`] reads margins and wins, and nothing else.
    fn hand_playout(replication: u32, margin: i64, won: bool) -> CandidatePlayout {
        CandidatePlayout {
            replication,
            world: WorldSeed(0),
            agent_seeds: vec![0, 0],
            margin,
            mover_total: 0,
            best_rival: PlayerId(1),
            best_rival_total: 0,
            rank: if won { 1 } else { 2 },
            won,
            decisions: 0,
            turns: 0,
            final_digest: String::new(),
            turn_verdict: None,
        }
    }

    /// A table of `deltas.len()` rows, each K = 25 playouts: 24 at margin −100
    /// and one at `−100 + delta`, so row `i`'s mean is `−100 + delta_i / 25` and
    /// the whole field's mean SPREAD is `(max delta − min delta) / 25`.
    ///
    /// Every margin stays negative and every playout is a loss, which is what a
    /// genuinely dead field looks like; `win_row` flips exactly one playout's
    /// verdict when a cell wants the LIVE half.
    fn spread_table(deltas: &[i64], win_row: Option<usize>) -> Adjudication {
        const K: u32 = 25;
        let rows: Vec<CandidateRow> = deltas
            .iter()
            .enumerate()
            .map(|(index, delta)| CandidateRow {
                candidate: Move::BuildShophouse { slot: (index % 4) as u8 },
                continuation_digest: format!("hand-built-continuation-{index}"),
                playouts: (0..K)
                    .map(|r| {
                        let margin = if r == 0 { -100 + delta } else { -100 };
                        let won = win_row == Some(index) && r == 0;
                        hand_playout(r, margin, won)
                    })
                    .collect(),
            })
            .collect();
        Adjudication {
            mover: PlayerId(0),
            root_turn: 0,
            master: WorldSeed(0),
            replications: K,
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
                seats: Vec::<AdjudicationSeat>::new(),
                master: 0,
                replications: K,
            },
        }
    }

    /// The MEASURED SESSION SHAPE, in three cells (review MUST-1).
    ///
    /// The D7 triage §3's dead roots read 15, 11 and 2 DISTINCT candidate means
    /// over fields no candidate ever won — so `all_candidates_equal`, the
    /// elementwise criterion, would have called all three LIVE and fed a
    /// CONFIRMATORY denominator with keyed-draw coin flips. This is the fix
    /// pinned at that shape: fifteen distinct means, spread 3.76, zero wins.
    ///
    /// The three cells are the classifier's two clauses and the bound itself —
    /// one win flips it LIVE at the same spread, and a spread of 5.00 flips it
    /// LIVE at the same (zero) wins — so neither clause can be dropped without
    /// failing here.
    #[test]
    fn the_dead_classifier_reads_the_measured_session_shape() {
        // 15 distinct deltas over a range of 94 -> spread 94/25 = 3.76 exactly.
        let deltas: Vec<i64> = [0, 7, 14, 21, 28, 35, 42, 49, 56, 63, 70, 77, 84, 91, 94].to_vec();
        let dead = spread_table(&deltas, None);
        let means: Vec<f64> = dead.rows.iter().map(|r| r.mean_margin()).collect();
        let spread = means.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - means.iter().copied().fold(f64::INFINITY, f64::min);
        assert_eq!(dead.candidates(), 15, "the fixture's premise: fifteen candidates");
        let mut distinct = means.clone();
        distinct.sort_by(|a, b| a.partial_cmp(b).unwrap());
        distinct.dedup();
        assert_eq!(distinct.len(), 15, "the fixture's premise: fifteen DISTINCT means");
        assert!((spread - 3.76).abs() < 1e-9, "the fixture's premise: spread 3.76, got {spread}");
        assert_eq!(dead.rows.iter().map(|r| r.wins()).sum::<usize>(), 0);
        assert!(
            !dead.all_candidates_equal(),
            "MUST-1's whole point: the STRICTER elementwise criterion calls this field LIVE",
        );
        assert!(classify_dead(&dead, DEAD_SPREAD_BOUND), "fifteen distinct means, zero wins: DEAD");

        // CELL 2 — one win, same spread.
        let one_win = spread_table(&deltas, Some(3));
        assert_eq!(one_win.rows.iter().map(|r| r.wins()).sum::<usize>(), 1);
        assert!(
            !classify_dead(&one_win, DEAD_SPREAD_BOUND),
            "a field one candidate WINS is live however tight its means are",
        );

        // CELL 3 — zero wins, spread 5.00 (a range of 125 over K = 25).
        let wide: Vec<i64> = deltas.iter().map(|d| d * 125 / 94).collect();
        let live = spread_table(&wide, None);
        let wide_means: Vec<f64> = live.rows.iter().map(|r| r.mean_margin()).collect();
        let wide_spread = wide_means.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - wide_means.iter().copied().fold(f64::INFINITY, f64::min);
        assert!((wide_spread - 5.0).abs() < 1e-9, "the cell's premise: spread 5.00, got {wide_spread}");
        assert!(
            !classify_dead(&live, DEAD_SPREAD_BOUND),
            "a five-point spread is past the declared bound",
        );
        // The bound is a PARAMETER: the same field is dead at a bound that admits it.
        assert!(classify_dead(&live, 5));
    }

    /// An EMPTY table is a FINISHED GAME, never a dead root — and a ONE-ROW
    /// table is a FORCED decision, which the literal rule does call dead.
    ///
    /// Both are documented corners of [`classify_dead`] and both are traps: the
    /// winless clause is vacuously true on an empty field, so a classifier
    /// without the explicit arm would launder "the game was over" into "the
    /// mover's options did not matter".
    #[test]
    fn an_empty_table_is_a_finished_game_and_never_a_dead_root() {
        let empty = spread_table(&[], None);
        assert_eq!(empty.candidates(), 0);
        assert!(empty.all_candidates_equal(), "the vacuous-true criterion this arm exists beside");
        assert!(!classify_dead(&empty, DEAD_SPREAD_BOUND));

        let forced = spread_table(&[0], None);
        assert_eq!(forced.candidates(), 1);
        assert!(
            classify_dead(&forced, DEAD_SPREAD_BOUND),
            "a forced decision is dead by the literal rule — nothing the mover could choose \
             mattered, because it had nothing to choose",
        );
    }

    /// **THE FORCED FIELD IS ITS OWN CLASSIFICATION** (the freeze wave, pilot
    /// finding 1): fewer than two candidates is not a preference at all.
    ///
    /// The pilot admitted a family-1 root whose adjudication carried ONE
    /// candidate and read it PASS — vacuously, because the subject's pick was
    /// the only move there was and the top set was therefore whatever the
    /// adjudicator forced. `classify_dead` cannot catch it: a lone candidate that
    /// WINS its replications is `live` by the wins clause, which is exactly the
    /// shape the pilot hit.
    ///
    /// The EMPTY table rides the same arm and for the same reason a caller must
    /// not read it as dead — it is a finished game, it offers no choice either,
    /// and neither confirms nor fails anything.
    #[test]
    fn a_forced_field_is_classified_forced_and_two_candidates_stay_live() {
        assert_eq!(FORCED_CANDIDATE_FLOOR, 2, "the floor is two candidates");
        assert!(classify_forced(&spread_table(&[], None)), "a finished game offers no choice");
        assert!(classify_forced(&spread_table(&[0], None)), "a one-row table is FORCED");
        assert!(
            !classify_forced(&spread_table(&[0, 25], None)),
            "two candidates is a CHOICE — the live path this rule must not touch",
        );

        // ACROSS THE MENU: the smallest count decides, and it is reported so the
        // exclusion can name it.
        let p = ReplyPolicy::FamilyWeighted;
        let q = ReplyPolicy::GreedyAtFinal;
        assert_eq!(forced_across_menu(&[(p, 1), (q, 1)]), Some(1));
        assert_eq!(
            forced_across_menu(&[(p, 2), (q, 1)]),
            Some(1),
            "one policy seeing a forced field is a forced field — the count is the ROOT's",
        );
        assert_eq!(forced_across_menu(&[(p, 2), (q, 3)]), None, "two candidates is live");
        assert_eq!(
            forced_across_menu(&[]),
            None,
            "no adjudication ran, so nothing found the field forced — the empty trap \
             `dead_across_menu` states one level up",
        );
    }

    /// The hook-FREE path is byte-for-byte what it was: [`adjudicate_root`]
    /// equals [`adjudicate_root_with`] at `None`, and every playout carries the
    /// counter-ABSENCE reading rather than a `false` verdict.
    #[test]
    fn adjudicate_root_delegates_with_no_hook_and_stamps_no_verdict() {
        let rules = rules();
        let (root, _) = claim_first_root(&rules, 0x22);
        let source = RootSource::Constructed(root);
        let plain = adjudicate_root(&source, &rung_seats(), &rules, WorldSeed(MASTER), 2,
            DrivenOpts::default())
            .expect("the fixture adjudicates");
        let delegated = adjudicate_root_with(&source, &rung_seats(), &rules, WorldSeed(MASTER), 2,
            DrivenOpts::default(), None)
            .expect("the fixture adjudicates");
        assert_eq!(plain.digest(), delegated.digest());
        assert_eq!(plain, delegated, "the delegation is not the identity it claims to be");
        assert!(plain.candidates() >= 2, "anti-vacuity: the fixture has candidates to stamp");
        assert!(
            plain.rows.iter().flat_map(|r| r.playouts.iter()).all(|p| p.turn_verdict.is_none()),
            "no hook means NO verdict — never a false one",
        );
        let digest = &plain.provenance.root.root_digest;
        assert_eq!(digest.len(), 16, "a root digest is sixteen characters");
        assert!(
            digest.chars().all(|c| c.is_ascii_hexdigit()),
            "and all of them hex — the read's own seeding premise",
        );
    }

    // =======================================================================
    // PHASE C T4 WAVE B: admission
    // =======================================================================

    /// One hand-built playout carrying a MARGIN, a WIN flag and a VERDICT — the
    /// three things [`admission`] and [`classify_dead`] read.
    fn verdict_playout(
        replication: u32,
        margin: i64,
        won: bool,
        verdict: Option<bool>,
    ) -> CandidatePlayout {
        let mut p = hand_playout(replication, margin, won);
        p.turn_verdict = verdict;
        p
    }

    /// A one-row adjudication whose playouts are given as
    /// `(margin, won, verdict)` — the smallest thing that carries an admission
    /// partition, since the partition is over PLAYOUTS and ignores which
    /// candidate produced them.
    fn verdict_table(playouts: &[(i64, bool, Option<bool>)]) -> Adjudication {
        let rows = vec![CandidateRow {
            candidate: Move::EndAdditionalPhase,
            continuation_digest: "hand-built-continuation".to_string(),
            playouts: playouts
                .iter()
                .enumerate()
                .map(|(r, (margin, won, verdict))| {
                    verdict_playout(r as u32, *margin, *won, *verdict)
                })
                .collect(),
        }];
        let k = playouts.len() as u32;
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
                seats: Vec::<AdjudicationSeat>::new(),
                master: 0,
                replications: k,
            },
        }
    }

    /// **THE δ GROUP ARITHMETIC**, on a hand-built adjudication with a known
    /// verdict mix — and with the 10–10 TIE-BREAK TRAP built into the fixture.
    ///
    /// The TRUE group is two playouts LEVEL on victory points: margin 0 each,
    /// one `won` and one lost, which is exactly what the clockwise tie-break
    /// produces and exactly what a signed margin cannot express. The FALSE group
    /// loses by 10. So the honest difference is `0 − (−10) = +10` and the root
    /// admits at δ = 5 — while an implementation built on WINS would read the
    /// groups as 0.5 and 0.0 and could not be compared to a δ in victory points
    /// at all.
    #[test]
    fn the_admission_delta_arithmetic_compares_group_margins_and_never_wins() {
        let adj = verdict_table(&[
            (0, true, Some(true)),
            (0, false, Some(true)),
            (-10, false, Some(false)),
            (-10, false, Some(false)),
        ]);
        let read = admission(&[(ReplyPolicy::GreedyAtFinal, adj.clone())], 5, false);
        let p = &read.policies[0];
        assert_eq!((p.true_n, p.false_n), (2, 2));
        assert!((p.true_mean - 0.0).abs() < 1e-12, "the TRUE group is level, not won: {p:?}");
        assert!((p.false_mean + 10.0).abs() < 1e-12);
        assert!((p.difference - 10.0).abs() < 1e-12);
        assert_eq!(p.verdict, AdmissionVerdict::Dominates);
        assert!(p.splitting(), "two a side is a measurement");
        // The ROOT-level conjunction needs TWO splitting policies since the
        // freeze wave, so the same table under a second policy is what admits.
        assert!(
            admission(
                &[(ReplyPolicy::GreedyAtFinal, adj.clone()), (ReplyPolicy::FamilyWeighted, adj)],
                5,
                false,
            )
            .admitted(),
        );

        // ANTI-VACUITY: the wins reading really does answer differently on this
        // fixture — one of the two LEVEL playouts is a WIN, so a group statistic
        // built on `won` would read 0.5 against 0.0 and could not be compared to
        // a δ in victory points at all.
        let adj = verdict_table(&[
            (0, true, Some(true)),
            (0, false, Some(true)),
            (-10, false, Some(false)),
            (-10, false, Some(false)),
        ]);
        assert_eq!(adj.rows[0].wins(), 1, "the fixture's premise: margin 0 with a win in it");

        // The SAME groups at a δ the difference does not reach.
        let read = admission(&[(ReplyPolicy::GreedyAtFinal, adj)], 11, false);
        assert_eq!(read.policies[0].verdict, AdmissionVerdict::Fails);
        assert!(!read.admitted());
    }

    /// A CONTROL root inverts the test: the pass behaviour must demonstrably
    /// LOSE by δ (canon 15's behind-mover arm).
    #[test]
    fn a_control_root_admits_on_the_inverted_difference() {
        let adj = verdict_table(&[
            (-12, false, Some(true)),
            (-12, false, Some(true)),
            (0, false, Some(false)),
            (0, false, Some(false)),
        ]);
        let control = admission(&[(ReplyPolicy::GreedyAtFinal, adj.clone())], 5, true);
        assert!((control.policies[0].difference + 12.0).abs() < 1e-12);
        assert_eq!(control.policies[0].verdict, AdmissionVerdict::Dominates);

        // The same table read as a POSITIVE root fails — the two modes are not
        // the same test with a sign flip in the report.
        let positive = admission(&[(ReplyPolicy::GreedyAtFinal, adj)], 5, false);
        assert_eq!(positive.policies[0].verdict, AdmissionVerdict::Fails);
    }

    /// BOTH GROUPS must be non-empty, else a typed [`CannotDemonstrate`] — and
    /// an UNSTAMPED adjudication is its own arm rather than a group of falses.
    #[test]
    fn an_admission_with_an_empty_group_or_no_verdicts_demonstrates_nothing() {
        let all_true = verdict_table(&[(5, false, Some(true)), (5, false, Some(true))]);
        let all_false = verdict_table(&[(5, false, Some(false)), (5, false, Some(false))]);
        let unstamped = verdict_table(&[(5, false, Some(true)), (-5, false, None)]);

        for (adj, expected) in [
            (all_true, CannotDemonstrate::NoFalseGroup),
            (all_false, CannotDemonstrate::NoTrueGroup),
            (unstamped, CannotDemonstrate::Unstamped),
        ] {
            let read = admission(&[(ReplyPolicy::GreedyAtFinal, adj)], 5, false);
            assert_eq!(
                read.policies[0].verdict,
                AdmissionVerdict::Cannot(expected),
                "the wrong reason was reported",
            );
            assert!(!read.admitted());
            assert!(!expected.as_str().is_empty());
        }

        // An UNSTAMPED playout is never silently a `false` — the mixed table
        // above has a TRUE group and would otherwise read NoFalseGroup.
        let mixed = verdict_table(&[(5, false, Some(true)), (-5, false, None)]);
        let read = admission(&[(ReplyPolicy::GreedyAtFinal, mixed)], 5, false);
        assert_ne!(
            read.policies[0].verdict,
            AdmissionVerdict::Cannot(CannotDemonstrate::NoFalseGroup),
            "an unstamped playout was treated as a false verdict",
        );
    }

    /// The CONJUNCTION and the SOLE-DISSENTER flag (review MUST-4), over a menu.
    ///
    /// The flag fires only when `lookahead-at-final` is the ONLY policy that did
    /// not demonstrate — a subject defect that deflates dominance under exactly
    /// the arm the subject is. Two dissenters, or a lone dissenter that is
    /// somebody else, are different findings and must not raise it.
    #[test]
    fn the_conjunction_and_the_sole_dissenter_flag_read_the_per_policy_verdicts() {
        // TWO playouts a side, because the freeze wave's SPLIT_MIN_GROUP makes a
        // one-game group non-demonstrating — a policy that cannot demonstrate is
        // outside the conjunction entirely, which is the rule the cells below
        // this one pin and NOT the one this cell is about.
        let good = || {
            verdict_table(&[
                (10, false, Some(true)),
                (10, false, Some(true)),
                (-10, false, Some(false)),
                (-10, false, Some(false)),
            ])
        };
        let bad = || {
            verdict_table(&[
                (0, false, Some(true)),
                (0, false, Some(true)),
                (0, false, Some(false)),
                (0, false, Some(false)),
            ])
        };

        let all_good = admission(
            &[
                (ReplyPolicy::FamilyWeighted, good()),
                (ReplyPolicy::GreedyAtFinal, good()),
                (ReplyPolicy::LookaheadAtFinal, good()),
            ],
            5,
            false,
        );
        assert!(all_good.admitted());
        assert!(all_good.dissenters().is_empty());
        assert!(!all_good.lookahead_sole_dissenter());

        let lookahead_only = admission(
            &[
                (ReplyPolicy::FamilyWeighted, good()),
                (ReplyPolicy::GreedyAtFinal, good()),
                (ReplyPolicy::LookaheadAtFinal, bad()),
            ],
            5,
            false,
        );
        assert!(!lookahead_only.admitted());
        assert_eq!(lookahead_only.dissenters(), vec![ReplyPolicy::LookaheadAtFinal]);
        assert!(lookahead_only.lookahead_sole_dissenter(), "MUST-4's collection trigger");

        let someone_else = admission(
            &[
                (ReplyPolicy::FamilyWeighted, bad()),
                (ReplyPolicy::GreedyAtFinal, good()),
                (ReplyPolicy::LookaheadAtFinal, good()),
            ],
            5,
            false,
        );
        assert!(!someone_else.lookahead_sole_dissenter(), "a different lone dissenter is a different finding");

        let two = admission(
            &[
                (ReplyPolicy::FamilyWeighted, bad()),
                (ReplyPolicy::GreedyAtFinal, good()),
                (ReplyPolicy::LookaheadAtFinal, bad()),
            ],
            5,
            false,
        );
        assert_eq!(two.dissenters().len(), 2);
        assert!(!two.lookahead_sole_dissenter());

        // An EMPTY menu admits nothing — an admission "across no policies" is
        // not a demonstration.
        let empty = admission(&[], 5, false);
        assert!(!empty.admitted());
        assert!(!empty.lookahead_sole_dissenter());
    }

    // =======================================================================
    // THE FREEZE WAVE: the SPLITTING rules (pilot findings on families 3/4)
    // =======================================================================

    /// A one-row adjudication with `trues` playouts at `true_margin` and
    /// `falses` at `false_margin` — the shape every splitting rule is about.
    ///
    /// One ROW, because the admission partition is over PLAYOUTS and ignores
    /// which candidate produced them (`verdict_table`'s own reason).
    fn split_table(trues: usize, true_margin: i64, falses: usize, false_margin: i64) -> Adjudication {
        let mut playouts: Vec<(i64, bool, Option<bool>)> = Vec::new();
        playouts.extend((0..trues).map(|_| (true_margin, false, Some(true))));
        playouts.extend((0..falses).map(|_| (false_margin, false, Some(false))));
        verdict_table(&playouts)
    }

    /// **(a) A ONE-SIDED POLICY IS RECORDED AND NON-BLOCKING** (the freeze wave).
    ///
    /// The pilot's families 3 and 4 admitted 0 of 4 roots everywhere, and the
    /// per-policy tables named the mechanism: greedy never claims within a turn
    /// at a proximity root (all-FALSE) and the race-rusher claims near-universally
    /// (all-TRUE). Those are DEGENERATE SHAPES — a policy with one group has no
    /// comparison in it — and under the landed conjunction each of them vetoed
    /// every root a splitting policy could have demonstrated.
    ///
    /// So the CANNOT line stays (it is a finding about that policy at that root)
    /// and it stops blocking. This is noise reduction, not bar-lowering: what a
    /// non-demonstrating policy loses is its VETO, never its record.
    #[test]
    fn a_one_sided_policy_is_recorded_but_does_not_block_the_conjunction() {
        let split = || split_table(2, 10, 2, -10);
        let read = admission(
            &[
                (ReplyPolicy::FamilyWeighted, split()),
                (ReplyPolicy::GreedyAtFinal, split()),
                (ReplyPolicy::RaceRusher, split_table(4, 10, 0, 0)),
                (ReplyPolicy::LookaheadAtFinal, split_table(0, 0, 4, -10)),
            ],
            5,
            false,
        );

        // RECORDED: both degenerate arms keep their typed CANNOT reading.
        assert_eq!(
            read.policies[2].verdict,
            AdmissionVerdict::Cannot(CannotDemonstrate::NoFalseGroup),
            "the all-TRUE arm lost its record",
        );
        assert_eq!(
            read.policies[3].verdict,
            AdmissionVerdict::Cannot(CannotDemonstrate::NoTrueGroup),
            "the all-FALSE arm lost its record",
        );
        assert!(!read.policies[2].splitting() && !read.policies[3].splitting());

        // NON-BLOCKING: the two splitting policies decide the root.
        assert_eq!(read.splitting_count(), 2);
        assert!(read.admitted(), "a one-sided policy vetoed the conjunction: {read:?}");
        assert!(
            read.dissenters().is_empty(),
            "a policy that could not demonstrate is not a DISSENTER: {:?}",
            read.dissenters(),
        );

        // THE ONE ARM THAT STILL VETOES: an UNSTAMPED adjudication is a fact
        // about the CALL (the turn hook was not passed), not about the position,
        // so it refuses the root rather than standing aside for the policies
        // that were wired correctly.
        let mis_wired = admission(
            &[
                (ReplyPolicy::FamilyWeighted, split()),
                (ReplyPolicy::GreedyAtFinal, split()),
                (ReplyPolicy::RaceRusher, verdict_table(&[(5, false, None), (5, false, None)])),
            ],
            5,
            false,
        );
        assert_eq!(mis_wired.splitting_count(), 2, "the two honest policies still split");
        assert!(mis_wired.policies[2].unstamped());
        assert!(
            !mis_wired.admitted(),
            "a mis-wired adjudication admitted a root off the policies that were wired right",
        );
    }

    /// **(b) A GROUP OF ONE IS ONE GAME'S NOISE** (the freeze wave).
    ///
    /// The pilot's family-weighted rung SPLIT ACCIDENTALLY — 1 of 8 true groups,
    /// whose "group mean" is a single playout's margin — and that one game then
    /// decided δ for the whole root. A policy therefore SPLITS only at two
    /// playouts a side; below that it is non-demonstrating, recorded and
    /// non-blocking exactly like the one-sided arms above.
    #[test]
    fn a_policy_splits_only_when_both_groups_hold_at_least_two_playouts() {
        assert_eq!(SPLIT_MIN_GROUP, 2, "the floor is two playouts a side");

        // THE PILOT'S OWN SHAPE: 1 of 8, at a difference δ would otherwise clear.
        let thin = admission(&[(ReplyPolicy::FamilyWeighted, split_table(1, 40, 7, -40))], 5, false);
        assert_eq!(
            thin.policies[0].verdict,
            AdmissionVerdict::Cannot(CannotDemonstrate::ThinSplit),
            "a one-game group mean was read as a measurement",
        );
        assert!(!thin.policies[0].splitting());
        assert_eq!((thin.policies[0].true_n, thin.policies[0].false_n), (1, 7));

        // The FALSE side is read the same way round.
        let other_way =
            admission(&[(ReplyPolicy::FamilyWeighted, split_table(7, 40, 1, -40))], 5, false);
        assert_eq!(
            other_way.policies[0].verdict,
            AdmissionVerdict::Cannot(CannotDemonstrate::ThinSplit),
        );

        // ANTI-VACUITY: the SAME difference at two-a-side is a real measurement.
        let fat = admission(&[(ReplyPolicy::FamilyWeighted, split_table(2, 40, 6, -40))], 5, false);
        assert_eq!(fat.policies[0].verdict, AdmissionVerdict::Dominates);
        assert!(fat.policies[0].splitting());
    }

    /// **(c) TWO SPLITTING POLICIES, ALL OF THEM DOMINATING** (the freeze wave).
    ///
    /// The conjunction is now taken over the SPLITTING policies rather than over
    /// the whole menu, with a floor of two: one splitting policy is one
    /// measurement, and a root admitted on it would be a root admitted on a
    /// single reply's taste. Fewer than two is an EXCLUSION whose reason names
    /// the count — a different finding from "the behaviour failed here".
    #[test]
    fn admission_needs_two_splitting_policies_and_every_one_of_them_dominating() {
        assert_eq!(MIN_SPLITTING_POLICIES, 2);
        let dominating = || split_table(2, 10, 2, -10);
        let failing = || split_table(2, 0, 2, 0);
        let one_sided = || split_table(4, 10, 0, 0);

        // EXACTLY ONE splitting policy: excluded, whatever that one said.
        let lonely = admission(
            &[
                (ReplyPolicy::FamilyWeighted, dominating()),
                (ReplyPolicy::GreedyAtFinal, one_sided()),
                (ReplyPolicy::RaceRusher, one_sided()),
            ],
            5,
            false,
        );
        assert_eq!(lonely.splitting_count(), 1);
        assert!(!lonely.admitted(), "one splitting policy is one reply's taste");

        // TWO, both dominating: admitted.
        let two = admission(
            &[
                (ReplyPolicy::FamilyWeighted, dominating()),
                (ReplyPolicy::GreedyAtFinal, dominating()),
                (ReplyPolicy::RaceRusher, one_sided()),
            ],
            5,
            false,
        );
        assert_eq!(two.splitting_count(), 2);
        assert!(two.admitted());

        // A SPLITTING DISSENTER still blocks — this is the half that is NOT
        // relaxed, and the reason the change is noise reduction rather than a
        // lower bar.
        let dissent = admission(
            &[
                (ReplyPolicy::FamilyWeighted, dominating()),
                (ReplyPolicy::GreedyAtFinal, failing()),
                (ReplyPolicy::RaceRusher, dominating()),
            ],
            5,
            false,
        );
        assert_eq!(dissent.splitting_count(), 3);
        assert!(!dissent.admitted(), "a splitting policy that FAILED stopped blocking");
        assert_eq!(dissent.dissenters(), vec![ReplyPolicy::GreedyAtFinal]);

        // THE CONTROL INVERSE, at the same floor: two splitting policies must
        // both demonstrate the LOSS.
        let control = admission(
            &[
                (ReplyPolicy::FamilyWeighted, split_table(2, -10, 2, 10)),
                (ReplyPolicy::GreedyAtFinal, split_table(2, -10, 2, 10)),
                (ReplyPolicy::RaceRusher, one_sided()),
            ],
            5,
            true,
        );
        assert_eq!(control.splitting_count(), 2);
        assert!(control.admitted(), "the control's inverted direction lost its two policies");
    }

    /// **THE SOLE-DISSENTER GUARD KEYS ON SPLITTING POLICIES ONLY** (the freeze
    /// wave, on review MUST-4's own logic).
    ///
    /// The flag exists to catch a subject defect that deflates dominance under
    /// exactly the arm the subject IS. A policy that demonstrated nothing at all
    /// is not evidence either way, so it can neither raise the flag nor mask it —
    /// the "every other policy held" half has to be read over the policies that
    /// could hold.
    #[test]
    fn the_sole_dissenter_flag_keys_on_splitting_policies_only() {
        let dominating = || split_table(2, 10, 2, -10);
        let failing = || split_table(2, 0, 2, 0);
        let one_sided = || split_table(4, 10, 0, 0);

        // TWO splitting policies, lookahead the only dissenter among them — the
        // one-sided third arm neither raises nor masks it.
        let flagged = admission(
            &[
                (ReplyPolicy::FamilyWeighted, dominating()),
                (ReplyPolicy::RaceRusher, one_sided()),
                (ReplyPolicy::LookaheadAtFinal, failing()),
            ],
            5,
            false,
        );
        assert!(!flagged.admitted());
        assert_eq!(flagged.dissenters(), vec![ReplyPolicy::LookaheadAtFinal]);
        assert!(flagged.lookahead_sole_dissenter(), "MUST-4's collection trigger");

        // A lookahead arm that DEMONSTRATED NOTHING is not a dissenter at all.
        let quiet = admission(
            &[
                (ReplyPolicy::FamilyWeighted, dominating()),
                (ReplyPolicy::GreedyAtFinal, dominating()),
                (ReplyPolicy::LookaheadAtFinal, one_sided()),
            ],
            5,
            false,
        );
        assert!(quiet.admitted());
        assert!(!quiet.lookahead_sole_dissenter());

        // ONE splitting policy cannot state the flag — there is no "the others
        // held" half to observe, which is the accessor's own requirement read
        // over the splitting policies instead of over the menu.
        let alone = admission(
            &[
                (ReplyPolicy::FamilyWeighted, one_sided()),
                (ReplyPolicy::LookaheadAtFinal, failing()),
            ],
            5,
            false,
        );
        assert_eq!(alone.splitting_count(), 1);
        assert!(!alone.lookahead_sole_dissenter());
    }

    // =======================================================================
    // PHASE C T4 WAVE B: the binary read
    // =======================================================================

    /// The salt is its OWN serial in the A3E1_D21E family — pinned
    /// ARITHMETICALLY against the family's home rather than by eyeballing two
    /// literals, so a future serial cannot silently collide.
    #[test]
    fn the_family_read_salt_is_its_own_serial_in_the_d21e_family() {
        assert_eq!(SALT_DRIVEN_FAMILY_READ, 0xA3E1_D21E_0000_0008);
        assert_ne!(SALT_DRIVEN_FAMILY_READ, SALT_DRIVEN_REPLICATION);
        assert_eq!(
            SALT_DRIVEN_FAMILY_READ,
            SALT_DRIVEN_REPLICATION + 2,
            "0006 is the family's home and 0007 is driven's race probe — this is 0008",
        );
    }

    /// [`root_word`] is the WHOLE digest read as one `u64`, and distinct roots
    /// therefore draw distinct streams from one master.
    ///
    /// The derivation is asserted word for word against `state_digest` and the
    /// crate's own mixing authority — not against a second copy of the formula —
    /// so a re-route through a different salt, a different component order, or a
    /// truncated word fails here.
    #[test]
    fn the_root_word_is_the_whole_digest_and_distinct_roots_draw_distinct_streams() {
        let rules = rules();
        let a = post_setup_2p(&rules, 0x31);
        let b = post_setup_2p(&rules, 0x32);
        let digest = state_digest(&a);
        assert_eq!(digest.len(), 16, "the derivation's premise: sixteen hex characters");
        assert_eq!(
            root_word(&a),
            u64::from_str_radix(&digest, 16).unwrap(),
            "the word is the WHOLE digest, not a prefix of it",
        );
        assert_ne!(root_word(&a), root_word(&b), "two roots share a word");

        let master = MASTER;
        for r in 0..4 {
            assert_eq!(
                replication_seed(master, root_word(&a), r),
                mix(SALT_DRIVEN_FAMILY_READ, &[master, root_word(&a), u64::from(r)]),
                "replication {r} did not derive through the declared mix",
            );
            assert_ne!(
                replication_seed(master, root_word(&a), r),
                replication_seed(master, root_word(&b), r),
                "distinct roots share replication {r}'s stream",
            );
        }
        let seeds: Vec<u64> = (0..8).map(|r| replication_seed(master, root_word(&a), r)).collect();
        let mut sorted = seeds.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), seeds.len(), "two replications drew the same seed");
    }

    /// The EXACT binomial line: at zero misses the bisection reproduces the
    /// CLOSED FORM `1 − α^(1/n)`, and the rule of three `3/n` sits just above it.
    #[test]
    fn the_binomial_bound_is_exact_at_zero_and_the_rule_of_three_approximates_it() {
        for n in [5_u32, 20, 100] {
            let closed = 1.0 - 0.05_f64.powf(1.0 / f64::from(n));
            let got = clopper_pearson_upper(0, n, 0.05);
            assert!(
                (got - closed).abs() < 1e-9,
                "n={n}: the bisection ({got}) does not reproduce the closed form ({closed})",
            );
            let rule_of_three = 3.0 / f64::from(n);
            assert!(
                rule_of_three > closed,
                "n={n}: the rule of three is meant to be the conservative approximation",
            );
        }
        // Monotone in the miss count, and total certainty once every trial missed.
        assert!(clopper_pearson_upper(1, 20, 0.05) > clopper_pearson_upper(0, 20, 0.05));
        assert!((clopper_pearson_upper(20, 20, 0.05) - 1.0).abs() < 1e-12);
        // The exact one-sided bound at 1/20 is the standard textbook value.
        assert!((clopper_pearson_upper(1, 20, 0.05) - 0.216_0).abs() < 5e-4);
    }

    /// The cheapest ROOT the shipped S1 arm can be read at: a mid-game position
    /// out of a rung game, scanned for a real branch.
    ///
    /// Rung-generated, so the fixture pays for NO search — the searching is what
    /// the cell itself does at the root (design §6's wall discipline).
    fn read_root(rules: &Rules, seed: WorldSeed, width: usize) -> GameState {
        let mut make = |seat: PlayerId, _players: u8| -> Box<dyn crate::game::arena::ArenaAgent> {
            Contestant::Policy(crate::game::policy::RolloutPolicy::FAMILY_WEIGHTED_DEFAULT)
                .build(rules, crate::game::arena::agent_stream_seed(seed, seat,
                    crate::game::arena::Slot(seat.0)))
                .0
        };
        let stats = crate::game::arena::run_arena_game(seed, 2, rules, &mut make, true)
            .expect("the generating rung game completes");
        let events = stats.record.events.len();
        (200..(240.min(events)))
            .find_map(|k| {
                let state = crate::game::record::state_at(&stats.record, k, rules).ok()?;
                (!crate::game::end_game::is_game_over(&state)
                    && crate::game::legal_moves::get_legal_moves(&state, rules).len() >= width)
                    .then_some(state)
            })
            .expect("a branching mid-game root")
    }

    /// **THE TURN-GRAIN SMOKE** — one lookahead subject, R = 2, and every part
    /// of the outcome a report quotes.
    ///
    /// The ONE cell here with a SEARCHING subject (design §6's wall discipline):
    /// two whole turns under the uncapped shipped S1 arm plus the step-one
    /// table. Budget ~3s debug; the measured wall is a fraction of that, because
    /// the root is an ordinary mid-game decision rather than a money-saturated
    /// Movement point. The budget is the ceiling this cell must not grow past.
    #[test]
    fn a_turn_grain_read_reports_a_verdict_and_a_seed_per_replication() {
        let rules = rules();
        let root = read_root(&rules, WorldSeed(MASTER), 8);
        let subject = SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT);
        let rule = PassRule::Turn(TurnPredicate::NotEndFired);

        let out = family_read(&root, subject, &rule, 2, MASTER, &rules)
            .expect("the shipped S1 arm reads an organic root");

        assert_eq!(out.grain, ReadGrain::Turn);
        assert_eq!(out.replication_count(), 2);
        assert_eq!(out.master, MASTER);
        assert_eq!(out.root_word, root_word(&root));
        assert_eq!(out.root_digest, state_digest(&root));
        assert_eq!(out.subject_echo, Contestant::LOOKAHEAD_V1_DEFAULT.echo());
        for (r, read) in out.replications.iter().enumerate() {
            let r = r as u32;
            assert_eq!(read.replication, r);
            assert_eq!(
                read.decision_seed,
                replication_seed(MASTER, out.root_word, r),
                "replication {r}'s recorded seed is not the derived one — T7 feeds this \
                 straight to `table --decision-seed`",
            );
            assert_eq!(read.root_word, out.root_word, "the word rides every replication");
            // A mid-game turn under an untriggered root satisfies NotEndFired.
            assert_eq!(read.verdict, Some(true), "replication {r}");
            assert!(read.pick.is_none(), "the turn grain reports no pick");
            assert!(read.turn_moves >= 1, "replication {r} played no move");
            assert!(read.handoff_digest.is_some());
            assert_eq!(read.game_over, Some(false));
            assert_eq!(
                read.counters.explained,
                Some(read.turn_moves),
                "the WHOLE turn's counters ride the replication (SHOULD-6)",
            );
        }
        assert_eq!(out.misses(), Some(0));
        let line = out.binomial_line().expect("the turn grain has a binomial line");
        assert!(line.contains("misses 0/2"), "{line}");
        assert!(line.contains("rule of three"), "{line}");

        // The STEP-ONE table is the root decision at replication 0's seed, and
        // its first move is the turn's first move — the join D7's counter check
        // rides on.
        assert_eq!(out.counter_check.table.decision_seed, out.replications[0].decision_seed);
        assert!(out.counter_check.table.candidates() >= 8);
        assert!(out.counter_check.distinct_values >= 1);
        assert!(out.counter_check.tied_at_top >= 1);
        assert!(out.counter_check.top_band >= out.counter_check.tied_at_top);
        assert_eq!(out.counter_check.table.stats.ply_cap_hits, 0);
        assert_eq!(out.counter_check.table.stats.budget_binds, 0);
    }

    /// **THE DECISION-GRAIN SMOKE**: R picks, no verdicts, no binomial line —
    /// the three-way top-set judgment is T5's and this outcome says so by
    /// carrying picks rather than pass/fail.
    #[test]
    fn a_decision_grain_read_reports_a_pick_and_no_verdict_per_replication() {
        let rules = rules();
        let root = read_root(&rules, WorldSeed(MASTER), 8);
        let subject = SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT);
        let rule = PassRule::AdjudicatedBest { epsilon: 2 };

        let out = family_read(&root, subject, &rule, 2, MASTER, &rules)
            .expect("the shipped S1 arm tabulates an organic root");
        assert_eq!(out.grain, ReadGrain::Decision);
        assert_eq!(out.rule, rule, "epsilon is CARRIED, for T5 to apply");
        assert!(out.misses().is_none(), "a miss at this grain is a fact about a TOP SET");
        assert!(out.binomial_line().is_none());
        for (r, read) in out.replications.iter().enumerate() {
            assert!(read.pick.is_some(), "replication {r} reported no pick");
            assert!(read.verdict.is_none());
            assert_eq!(read.turn_moves, 0);
            assert_eq!(read.counters, SeatCounters::NONE);
            assert_eq!(read.decision_seed, replication_seed(MASTER, out.root_word, r as u32));
        }
        // Replication 0 IS the step-one table, reused rather than re-searched.
        assert_eq!(out.replications[0].pick.as_ref(), Some(&out.counter_check.table.winner));
    }

    /// SEED-SCHEDULE DETERMINISM: the same inputs read the same outcome twice,
    /// field for field.
    #[test]
    fn the_same_inputs_read_the_same_outcome_twice() {
        let rules = rules();
        let root = read_root(&rules, WorldSeed(MASTER), 8);
        let subject = SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT);
        let rule = PassRule::Turn(TurnPredicate::NotEndFired);
        let once = family_read(&root, subject, &rule, 2, MASTER, &rules).expect("first read");
        let twice = family_read(&root, subject, &rule, 2, MASTER, &rules).expect("second read");
        assert_eq!(once, twice, "the same master read two different outcomes");

        // A DIFFERENT master moves the seeds — the anti-vacuity half.
        let elsewhere = family_read(&root, subject, &rule, 2, MASTER ^ 1, &rules).expect("third read");
        assert_ne!(
            elsewhere.replications[0].decision_seed, once.replications[0].decision_seed,
            "the master does not reach the derivation",
        );
    }

    /// A subject with NO RANKED ROOT SURFACE is refused in [`root_table`]'s own
    /// words — because the counter check is MANDATORY at both grains, not
    /// because the read had nothing to do.
    ///
    /// All four kinds are named: a rung SAMPLES, greedy is UNRANKED, mcts and the
    /// atomic-diag arm are DIAGNOSTIC, and the RaceRusher reply is not a
    /// contestant at all.
    #[test]
    fn a_subject_with_no_ranked_root_surface_is_refused_by_root_tables_own_taxonomy() {
        let rules = rules();
        let root = read_root(&rules, WorldSeed(MASTER), 4);
        let rule = PassRule::Turn(TurnPredicate::NotEndFired);

        for (subject, expected) in [
            (
                SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
                UnsupportedReason::Sampled,
            ),
            (SeatSpec::Reply(ReplyPolicy::GreedyAtFinal), UnsupportedReason::Unranked),
            (SeatSpec::Contestant(Contestant::MCTS_V1_DEFAULT), UnsupportedReason::Diagnostic),
            (SeatSpec::Contestant(Contestant::ATOMIC_DIAG_DEFAULT), UnsupportedReason::Diagnostic),
        ] {
            match family_read(&root, subject, &rule, 2, MASTER, &rules) {
                Err(FamilyReadError::Table {
                    replication: 0,
                    error: RootTableError::Unsupported { reason, .. },
                }) => assert_eq!(reason, expected, "{subject:?} was refused for the wrong reason"),
                other => panic!("{subject:?}: expected an Unsupported refusal, got {other:?}"),
            }
        }

        match family_read(&root, SeatSpec::Reply(ReplyPolicy::RaceRusher), &rule, 2, MASTER, &rules) {
            Err(FamilyReadError::SubjectIsNotAContestant(NotAContestant::RaceRusher)) => {}
            other => panic!("a harness reply rule is not a subject: {other:?}"),
        }

        match family_read(&root, SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT), &rule, 0,
            MASTER, &rules) {
            Err(FamilyReadError::NoReplications) => {}
            other => panic!("0 replications is not a measurement: {other:?}"),
        }
    }

    /// **BRANCH 0**: a capped subject whose budget BINDS refuses the whole read
    /// rather than reporting a preference.
    ///
    /// `cap=1` is the cheapest binding search there is — the mandatory root pass
    /// alone spends the whole budget, so no continuation can start and every
    /// candidate is decided on its one-ply value. The refusal names the SITE and
    /// carries the offending counters, which is what makes the D7 routing carry
    /// its own evidence.
    ///
    /// The uncapped arm at the SAME root is the anti-vacuity: it reads clean, so
    /// the refusal is about the cap and not about the position.
    #[test]
    fn the_counter_check_refuses_a_capped_subject_and_names_its_counters() {
        let rules = rules();
        let root = read_root(&rules, WorldSeed(MASTER), 8);
        let rule = PassRule::Turn(TurnPredicate::NotEndFired);
        let capped = Contestant::from_str_name("lookahead-v1:cap=1").expect("a capped S1 arm");

        match family_read(&root, SeatSpec::Contestant(capped), &rule, 2, MASTER, &rules) {
            Err(FamilyReadError::CounterRefusal { site, counters }) => {
                assert_eq!(site, CounterSite::RootTable, "the cheap refusal comes first");
                let named: Vec<&str> = counters.iter().map(|(n, _)| *n).collect();
                assert!(named.contains(&"budget_binds"), "{named:?}");
                assert!(counters.iter().all(|(_, v)| *v > 0), "a zero counter is not a refusal");
                let text = FamilyReadError::CounterRefusal { site, counters }.to_string();
                assert!(text.contains("budget_binds"), "{text}");
                assert!(text.contains("branch 0"), "{text}");
            }
            other => panic!("a bound budget must refuse the preference framing: {other:?}"),
        }

        // ANTI-VACUITY: the same root, uncapped, reads clean.
        family_read(&root, SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT), &rule, 1,
            MASTER, &rules)
            .expect("the uncapped arm reads this root");
    }

    /// The hard-refuse and annotate classes, at the two helpers that decide them
    /// — including the `None`-is-not-zero rule the turn grain's account rides.
    ///
    /// Pinned directly because the two SITES are not equally reachable
    /// end-to-end: the root-table check runs FIRST and a capped subject trips it
    /// there, so `CounterSite::Replication` is only reached when the root
    /// decision is clean and a later one is not — a shape a fixture cannot
    /// manufacture cheaply, and the reason this cell exists.
    #[test]
    fn the_counter_classes_split_hard_refusals_from_annotations() {
        // `None` is NOT zero — a rung carries no such counter at all.
        assert!(refusing_counters(&SeatCounters::NONE).is_empty());
        let mut counters = SeatCounters::NONE;
        counters.budget_binds = Some(0);
        assert!(refusing_counters(&counters).is_empty(), "a zero counter is not a refusal");
        counters.budget_binds = Some(3);
        counters.ply_cap_hits = Some(1);
        let named: Vec<&str> = refusing_counters(&counters).iter().map(|(n, _)| *n).collect();
        assert_eq!(named, vec!["ply_cap_hits", "budget_binds"]);

        assert_eq!(CounterSite::RootTable.describe(),
            "the step-one root table at replication 0's seed");
        assert_eq!(CounterSite::Replication(4).describe(),
            "replication 4's whole-turn counters");

        // The ANNOTATE class carries no refusal, and `placement_cap_binds` is
        // the deliberate member of it: a table whose ONLY nonzero counter is that
        // one reads CLEAN, because design §5.4's hard-refuse enumeration does not
        // name it and `SeatCounters` — the turn grain's own account — does not
        // carry it at all.
        let stats = SearchStats {
            root_candidates: 3,
            scored: 3,
            placement_cap_binds: 7,
            ..SearchStats::default()
        };
        let table = hand_table_with(stats);
        assert!(
            refusing_stats(&table).is_empty(),
            "a placement-cap bind ANNOTATES; it does not refuse",
        );
        // ANTI-VACUITY: the same table with one hard-refuse counter DOES refuse,
        // so the cell separates the two classes rather than testing an empty rule.
        // Each cell sets ONE hard-refuse counter on top of the annotate-class
        // one, so the assertion is that exactly that name comes back.
        let one_of_each = [
            SearchStats { ply_cap_hits: 1, placement_cap_binds: 7, ..SearchStats::default() },
            SearchStats { free_cap_hits: 1, placement_cap_binds: 7, ..SearchStats::default() },
            SearchStats { budget_binds: 1, placement_cap_binds: 7, ..SearchStats::default() },
            SearchStats { budget_starved: 1, placement_cap_binds: 7, ..SearchStats::default() },
        ];
        let names = ["ply_cap_hits", "free_cap_hits", "budget_binds", "budget_starved"];
        for (name, stats) in names.into_iter().zip(one_of_each) {
            let refusing = refusing_stats(&hand_table_with(stats));
            assert_eq!(
                refusing.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
                vec![name],
                "{name} is a hard-refuse counter and placement_cap_binds is not",
            );
        }
    }

    /// A hand-built [`RootTable`] carrying the given counters and nothing else
    /// worth reading — `refusing_stats` reads `stats` and no other field.
    fn hand_table_with(stats: SearchStats) -> RootTable {
        RootTable {
            seat: PlayerId(0),
            contestant: Contestant::LOOKAHEAD_V1_DEFAULT,
            echo: Contestant::LOOKAHEAD_V1_DEFAULT.echo(),
            decision_seed: 0,
            winner: Move::EndAdditionalPhase,
            ranked: vec![crate::game::lookahead::RootScore {
                mv: Move::EndAdditionalPhase,
                value: 0,
            }],
            stats,
            world: crate::game::driven::SampledWorld {
                sampler_seed: 0,
                worlds: 1,
                digest: "hand-built-world".to_string(),
            },
        }
    }

    /// The TIE/BAND annotations fire off the step-one table and RECOMMEND the
    /// separator without refusing the read (design §5.4's split).
    #[test]
    fn tie_and_band_counters_annotate_the_read_and_never_refuse_it() {
        let rules = rules();
        let root = read_root(&rules, WorldSeed(MASTER), 8);
        let subject = SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT);
        let out = family_read(&root, subject, &PassRule::AdjudicatedBest { epsilon: 2 }, 1,
            MASTER, &rules)
            .expect("the read completes");
        let table = &out.counter_check.table;

        let expected_tie = (table.tied_at_top() > 1)
            .then(|| CounterAnnotation::TiedAtTop { candidates: table.tied_at_top() });
        let expected_band = (table.top_band() > table.tied_at_top()).then(|| {
            CounterAnnotation::TopBand { candidates: table.top_band(), points: TOP_BAND_POINTS }
        });
        for expected in [expected_tie, expected_band].into_iter().flatten() {
            assert!(out.annotations.contains(&expected), "missing {expected:?} in {:?}",
                out.annotations);
        }
        assert_eq!(
            out.separator_recommended(),
            expected_tie.is_some() || expected_band.is_some(),
            "the separator recommendation does not follow its own annotations",
        );
        // ANTI-VACUITY: this root really does carry tie mass to annotate.
        assert!(
            table.top_band() > 1,
            "the fixture does not exercise the annotate class (band {})",
            table.top_band(),
        );
    }

    // =======================================================================
    // The family 3/4 BUILDERS (design §1 families 3–4, plan §T5(A))
    // =======================================================================

    /// The seed sweep every builder pin runs. Nine draws is enough that each
    /// declared parameter range is exercised at more than one value while the
    /// whole block stays a fraction of a second (no builder plays a game).
    const BUILDER_SEEDS: std::ops::Range<u64> = 0..9;

    /// Every seed of the sweep builds an ENGINE-VALID race root whose declared
    /// step is coherent with the shipped predicate.
    ///
    /// The builder already refuses on its own shape claims, so an `Ok` here is
    /// most of the pin; what the cell adds is the two statements a builder
    /// cannot make about itself — that `check_invariants` accepts the state
    /// (re-run independently of the builder's own call) and that
    /// [`RootPredicate::ExclusiveRaceOneStep`] AGREES the position is one of
    /// its own (the predicate/builder coherence pin the plan names).
    #[test]
    fn the_race_positive_builder_is_engine_valid_and_predicate_coherent() {
        let rules = rules();
        let mut steps: Vec<ContestedStep> = Vec::new();
        for seed in BUILDER_SEEDS {
            let (state, step) = build_race_positive(seed, &rules)
                .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            crate::game::invariants::check_invariants(&state, &rules)
                .unwrap_or_else(|e| panic!("seed {seed}: the built state is not engine-valid: {e}"));
            assert!(step.well_formed(), "seed {seed}: {step:?} names no space");
            assert!(
                RootPredicate::ExclusiveRaceOneStep.holds(&state, &rules),
                "seed {seed}: the shipped predicate does not recognise the built root",
            );
            assert!(
                !step.held_by(&state, state.current_player, &rules),
                "seed {seed}: the mover already HOLDS the contested step — not a race",
            );
            steps.push(step);
        }
        // ANTI-VACUITY on the declared TRACK parameter: the sweep must exercise
        // both arms, or the customs half of the construction is untested.
        assert!(
            steps.iter().any(|s| matches!(s, ContestedStep::ChurchSpace { .. }))
                && steps.iter().any(|s| matches!(s, ContestedStep::CustomsSpace { .. })),
            "the seed sweep never reached one of the two track arms: {steps:?}",
        );
    }

    /// **THE PREDICATE-COHERENCE CLAUSE'S OWN RED PATH** (T5 review SHOULD-1).
    ///
    /// The sweep above asserts [`RootPredicate::ExclusiveRaceOneStep`] on every
    /// built root, which is the property — but it asserts it INDEPENDENTLY, so
    /// deleting the fifth clause of [`self_check_race_root`] left the whole
    /// library green. This cell is the clause's own falsifier: a position that
    /// satisfies the other FOUR clauses and that the shipped predicate still
    /// rejects.
    ///
    /// **The separating conjunct is the WIDTH BOUND**, which is the one part of
    /// the predicate the four shape clauses do not restate: widening the mover
    /// to all six church payment types takes the root's legal list from 2 moves
    /// to 65, past [`crate::game::driven::MAX_ROOT_CANDIDATES`], while the step
    /// stays open, the threat stays public and the mover can still take it. That
    /// is a REAL disagreement rather than a doctored flag — a root too wide to be
    /// adjudicated is exactly what the predicate exists to exclude.
    #[test]
    fn the_race_self_check_refuses_a_root_the_shipped_predicate_rejects() {
        let rules = rules();
        // A CHURCH draw, found rather than assumed: the customs arm's legal list
        // does not widen with payment types, so a seed-arm re-mapping must fail
        // this cell loudly instead of quietly making it vacuous.
        let (mut state, step) = BUILDER_SEEDS
            .map(|seed| build_race_positive(seed, &rules).expect("builds"))
            .find(|(_, step)| matches!(step, ContestedStep::ChurchSpace { .. }))
            .expect("the sweep reaches the church arm");
        let mover = state.current_player;
        let opponent = other_seat(&state, mover).expect("a 2P root has a rival");

        // The UNWIDENED root passes the whole self-check — the green half.
        self_check_race_root(&state, step, mover, opponent, &rules)
            .expect("the builder's own root satisfies its shape claim");

        // All SIX church payment types at once (the builder's church arm zeroes
        // every good) ⇒ 2⁶ payment masks in the legal list.
        {
            let p = state.player_mut(mover);
            p.copper = 1;
            p.silk = 1;
            p.tea = 1;
            p.seafood = 1;
            p.money = 9;
            p.imported_goods = 3;
        }
        assert!(
            check_invariants(&state, &rules).is_ok(),
            "the widened root must stay engine-valid, or the cell proves nothing",
        );
        assert!(
            get_legal_moves(&state, &rules).len() > crate::game::driven::MAX_ROOT_CANDIDATES,
            "the widened root is not past the width bound — the separator moved",
        );
        assert!(
            !RootPredicate::ExclusiveRaceOneStep.holds(&state, &rules),
            "the shipped predicate still recognises the widened root",
        );

        // ANTI-VACUITY: the other four clauses still hold, so the refusal below
        // can only be the fifth.
        assert!(step.well_formed() && step_is_open(&state, step));
        assert!(mover_can_take_step(&state, step, &rules));
        let view = observe(&state, mover).expect("the mover observes its own root");
        assert!(opponent_one_action_from_step(&view, opponent, step));

        let err = self_check_race_root(&state, step, mover, opponent, &rules)
            .expect_err("a root the predicate rejects is not a race root");
        assert!(
            matches!(err, BuilderError::ShapeCheck { clause: "predicate coherence", .. }),
            "expected the predicate-coherence clause, got {err}",
        );
    }

    /// **THE THREAT IS PUBLIC — and it is public BY CONSTRUCTION, not by
    /// assertion.**
    ///
    /// [`opponent_one_action_from_step`] takes a [`PlayerView`], so the hidden
    /// starting order is not merely unread: it is not in scope. The pin
    /// evaluates the clause on the MOVER's own view — the information a seated
    /// player has — and requires it to hold there.
    ///
    /// The second half is the discriminator: the same clause on a view where
    /// the opponent's assistants have LEFT the contested area reads FALSE, so
    /// the cell separates "the threat is visible" from "the function returns
    /// true".
    #[test]
    fn the_race_positive_builders_threat_is_public_by_construction() {
        let rules = rules();
        for seed in BUILDER_SEEDS {
            let (state, step) = build_race_positive(seed, &rules)
                .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            let mover = state.current_player;
            let opponent = state
                .players
                .iter()
                .map(|p| p.id)
                .find(|id| *id != mover)
                .expect("a 2P root has a rival");
            let view = crate::game::observe::observe(&state, mover)
                .unwrap_or_else(|e| panic!("seed {seed}: the mover cannot observe its own root: {e:?}"));
            assert!(
                opponent_one_action_from_step(&view, opponent, step),
                "seed {seed}: the threat is not visible from the mover's own view",
            );

            // INVERSION: strip the opponent's presence at the contested area and
            // the clause must fall.
            let mut vacated = state.clone();
            {
                let p = vacated.player_mut(opponent);
                let moved: u8 = p.assistants.on_board.values().copied().sum();
                p.assistants.on_board.clear();
                p.assistants.in_hand += moved;
            }
            let vacated_view = crate::game::observe::observe(&vacated, mover).expect("observable");
            assert!(
                !opponent_one_action_from_step(&vacated_view, opponent, step),
                "seed {seed}: the clause holds with the opponent off the board — it is not \
                 reading the board at all",
            );
        }
    }

    /// Distinct seeds build distinct positions — the declared parameter ranges
    /// are live rather than decorative.
    #[test]
    fn the_race_positive_builder_varies_with_its_seed() {
        let rules = rules();
        let digests: Vec<String> = BUILDER_SEEDS
            .map(|seed| {
                let (state, _) = build_race_positive(seed, &rules).expect("builds");
                state_digest(&state)
            })
            .collect();
        let mut unique = digests.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), digests.len(), "two seeds built the same position");
    }

    /// The NO-DENIAL control: the denial step is TAKEABLE, and taking it
    /// forfeits a strictly better line — by exactly the constructed 7 VP.
    ///
    /// The arithmetic is the engine's own final scoring read as-if-now at the
    /// LINE grain, so the pin re-derives it independently of the builder: the
    /// best line that avoids the contested space climbs to Faith 9 and banks 8
    /// printed VP plus first place in the church majority against a 1 VP goods
    /// bill (+13), while the best line that takes the contested space banks
    /// Faith 4's printed 3 VP and second place for nothing (+6).
    ///
    /// The second half is the DISCRIMINATOR the first draft of this control did
    /// not have: the same split against a step the OPPONENT cannot reach — the
    /// track's top space — inverts the sign, so the cell separates "denial is
    /// priced" from "any low space is priced".
    #[test]
    fn the_no_denial_builder_offers_the_step_and_a_strictly_better_line() {
        let rules = rules();
        for seed in BUILDER_SEEDS {
            let (state, step) = build_race_no_denial(seed, &rules)
                .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            crate::game::invariants::check_invariants(&state, &rules)
                .unwrap_or_else(|e| panic!("seed {seed}: not engine-valid: {e}"));
            assert!(step.well_formed(), "seed {seed}: {step:?} names no space");
            assert!(
                mover_can_take_step(&state, step, &rules),
                "seed {seed}: the denial step is not takeable — the control has no denial in it",
            );
            let split = denial_split(&state, step, &rules);
            assert!(
                split.denial_lines > 0 && split.alternative_lines > 0,
                "seed {seed}: the split has an empty side: {split:?}",
            );
            assert_eq!(
                split.best_alternative - split.best_denial,
                NO_DENIAL_FORFEIT_VP,
                "seed {seed}: the constructed forfeit is not the declared arithmetic: {split:?}",
            );

            // The DISCRIMINATOR: price the track's BEST space instead. It is
            // not a denial step (the opponent cannot reach it), and taking it
            // is what the better line does — so the sign flips.
            let best_space = ContestedStep::ChurchSpace { faith: 9 };
            let inverted = denial_split(&state, best_space, &rules);
            assert!(
                inverted.best_alternative - inverted.best_denial < 0,
                "seed {seed}: taking the track's best space reads as a forfeit too — the \
                 control is pricing height, not denial: {inverted:?}",
            );
        }
    }

    /// Distinct seeds build distinct no-denial positions.
    #[test]
    fn the_no_denial_builder_varies_with_its_seed() {
        let rules = rules();
        let digests: Vec<String> = BUILDER_SEEDS
            .map(|seed| {
                let (state, _) = build_race_no_denial(seed, &rules).expect("builds");
                state_digest(&state)
            })
            .collect();
        let mut unique = digests.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), digests.len(), "two seeds built the same position");
    }

    /// **THE F8 PROXIMITY SHAPE, as an apply-probe**: the claim is NOT legal at
    /// the root and IS legal one in-turn produce later — two decisions, one
    /// turn, which is exactly what makes family 4's read a TURN-grain read.
    #[test]
    fn the_proximity_builder_is_two_decisions_from_a_claim_it_cannot_make_yet() {
        let rules = rules();
        for seed in BUILDER_SEEDS {
            let state = build_achievement_proximity(seed, &rules)
                .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            crate::game::invariants::check_invariants(&state, &rules)
                .unwrap_or_else(|e| panic!("seed {seed}: not engine-valid: {e}"));
            let claim = Move::FulfillAchievement { achievement: PROXIMITY_ACHIEVEMENT };
            let legal = crate::game::legal_moves::get_legal_moves(&state, &rules);
            assert!(
                !legal.contains(&claim),
                "seed {seed}: the claim is already legal — this is F8's LIVE shape, not its \
                 proximity corner",
            );
            assert!(
                legal.contains(&PROXIMITY_PRODUCE),
                "seed {seed}: the in-turn produce is not available: {legal:?}",
            );
            let after = crate::game::apply_move::apply_move(&state, &PROXIMITY_PRODUCE, &rules)
                .unwrap_or_else(|e| panic!("seed {seed}: the produce does not apply: {e:?}"));
            assert!(
                crate::game::legal_moves::get_legal_moves(&after, &rules).contains(&claim),
                "seed {seed}: the claim is not reachable one decision after the produce",
            );
            // And the leg really is completable WITHIN the turn: the produce
            // never hands off.
            assert_eq!(
                after.current_player, state.current_player,
                "seed {seed}: the produce ended the mover's turn",
            );
        }
    }

    /// Distinct seeds build distinct proximity positions.
    #[test]
    fn the_proximity_builder_varies_with_its_seed() {
        let rules = rules();
        let digests: Vec<String> = BUILDER_SEEDS
            .map(|seed| state_digest(&build_achievement_proximity(seed, &rules).expect("builds")))
            .collect();
        let mut unique = digests.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), digests.len(), "two seeds built the same position");
    }

    // =======================================================================
    // The REGISTRY and the block run (design §2's `FamilyDecl`, plan §T5(B))
    // =======================================================================

    /// [`families`] is EXACTLY design §1's four, in its order, with the label,
    /// class, generator and pass rule each family is declared with.
    #[test]
    fn the_registry_declares_exactly_the_four_families_the_design_names() {
        let decls = families();
        assert_eq!(decls.len(), 4, "the design declares four families and no more");
        let ids: Vec<&str> = decls.iter().map(|d| d.id).collect();
        assert_eq!(ids, vec!["final-round", "end-trigger", "exclusive-race", "achievement"]);

        // Family 1 — the rules-derivable decision-grain family.
        assert_eq!(decls[0].label, FamilyLabel::Confirmatory);
        assert_eq!(
            decls[0].generator,
            FamilyGenerator::Predicate(RootPredicate::FinalRoundAlreadyTriggered),
        );
        assert!(matches!(decls[0].rule, PassRule::AdjudicatedBest { .. }));
        assert_eq!(decls[0].rule.grain(), ReadGrain::Decision);

        // Family 2 — canon 15's inverted brake, at the turn grain.
        assert_eq!(decls[1].label, FamilyLabel::Confirmatory);
        assert_eq!(
            decls[1].generator,
            FamilyGenerator::Predicate(RootPredicate::EndFirableAheadInTruth),
        );
        assert_eq!(decls[1].rule, PassRule::Turn(TurnPredicate::EndFired));

        // Family 3 — the EXISTENCE demonstration, off a builder.
        assert_eq!(decls[2].label, FamilyLabel::ExistenceDemonstration);
        assert!(
            matches!(decls[2].generator, FamilyGenerator::Builder { .. }),
            "family 3's honest source is the builder arm",
        );
        assert!(decls[2].rule.names_a_step(), "family 3's rule is step-carrying");

        // Family 4 — the agenda-setters.
        assert_eq!(decls[3].label, FamilyLabel::Confirmatory);
        assert_eq!(
            decls[3].generator,
            FamilyGenerator::Predicate(RootPredicate::AchievementClaimAvailable),
        );
        assert_eq!(decls[3].rule, PassRule::Turn(TurnPredicate::AchievementClaimed));

        for decl in &decls {
            assert_eq!(decl.menu, REPLY_MENU, "{}: the declared menu is the full four", decl.id);
            assert_eq!(decl.pilot, PILOT_DRAWS, "{}: pilot ordinals are 0..4", decl.id);
            assert_eq!(decl.replications, PROVISIONAL_REPLICATIONS, "{}", decl.id);
            assert_eq!(decl.per_root_f, PROVISIONAL_PER_ROOT_F, "{}", decl.id);
            assert!(!decl.canon.is_empty(), "{}: every family cites its canon", decl.id);
        }
    }

    /// **THE FROZEN STATE**: every declaration carries BOTH frozen blocks, and
    /// no third block field exists to carry an escrowed one.
    ///
    /// The escrow half is checked on the DECLARATION rather than on the source
    /// text (design §2: "escrow ordinals live in the freeze DOCUMENT,
    /// deliberately not in code — no command can burn an escrowed block by
    /// accident"). A `Debug` rendering of the struct names every field it has,
    /// so a field called anything escrow-shaped would show up here. That half is
    /// UNCHANGED by the freeze wave, which baked two blocks and not three.
    #[test]
    fn every_declaration_carries_both_frozen_blocks_and_no_escrow_field_exists() {
        for decl in families() {
            assert!(
                decl.confirmatory.is_some(),
                "{}: the freeze wave bakes the positive arm's block",
                decl.id,
            );
            assert!(
                decl.confirmatory_control.is_some(),
                "{}: the freeze wave bakes the CONTROL arm's block too — the two arms draw \
                 different roots and cannot share one identity",
                decl.id,
            );
            let rendered = format!("{decl:?}");
            assert!(rendered.contains("pilot"), "{}: the pilot block is not declared", decl.id);
            assert!(
                rendered.contains("confirmatory"),
                "{}: the confirmatory block is not declared",
                decl.id,
            );
            assert!(
                !rendered.to_lowercase().contains("escrow"),
                "{}: an escrow block reached the code: {rendered}",
                decl.id,
            );
        }
    }

    /// **THE BAKED BLOCKS, HEX FOR HEX** (the freeze wave) — a TRANSCRIPTION
    /// pin over the masters the orchestrator recorded before use and the strided
    /// ordinals the freeze sizes.
    ///
    /// Written as literals rather than derived from anything: these numbers ARE
    /// the committed identity of every confirmatory result Phase C will quote,
    /// and a derivation would let one of them move without a test noticing.
    ///
    /// Family 1's two arms share a master DELIBERATELY: its control is a
    /// different PREDICATE (`TriggerRoundNotFinal`), so the same master and the
    /// same ordinals name a different ordinal space and therefore different
    /// roots. The other three families' arms carry their own masters.
    #[test]
    fn the_four_declarations_carry_the_baked_confirmatory_and_control_blocks() {
        let block = |master: u64, ordinals: std::ops::Range<u32>| FrozenBlock { master, ordinals };
        let expected = [
            ("final-round", block(0xA3E1_2026_0818_0004, 4..20), block(0xA3E1_2026_0818_0004, 4..20)),
            ("end-trigger", block(0xA3E1_2026_0818_0005, 4..28), block(0xA3E1_2026_0818_0006, 4..28)),
            (
                "exclusive-race",
                block(0xA3E1_2026_0818_0008, 4..28),
                block(0xA3E1_2026_0818_0009, 4..28),
            ),
            ("achievement", block(0xA3E1_2026_0818_0007, 4..28), block(0xA3E1_2026_0818_000A, 4..28)),
        ];
        let decls = families();
        assert_eq!(decls.len(), expected.len());
        for (decl, (id, positive, control)) in decls.iter().zip(expected) {
            assert_eq!(decl.id, id, "the registry order moved");
            // DISJOINT from the pilot, which is what makes a confirmatory result
            // a fresh reading rather than a re-quote of the pilot's roots.
            for arm in [&positive, &control] {
                assert!(
                    arm.ordinals.start >= decl.pilot.end,
                    "{id}: a confirmatory block overlaps the pilot draws {:?}",
                    decl.pilot,
                );
            }
            assert_eq!(
                decl.confirmatory,
                Some(positive),
                "{id}: the positive arm's frozen block",
            );
            assert_eq!(
                decl.confirmatory_control,
                Some(control),
                "{id}: the control arm's frozen block",
            );
        }

        // The MASTERS: every family's positive master is its own, and only
        // family 1 shares one across its arms (a different predicate at the same
        // master is a different ordinal space).
        let mut positives: Vec<u64> =
            decls.iter().map(|d| d.confirmatory.as_ref().expect("baked").master).collect();
        positives.sort_unstable();
        positives.dedup();
        assert_eq!(positives.len(), 4, "two families share a confirmatory master");
        for decl in &decls {
            let positive = decl.confirmatory.as_ref().expect("baked").master;
            let control = decl.confirmatory_control.as_ref().expect("baked").master;
            if decl.id == "final-round" {
                assert_eq!(positive, control, "family 1's arms share a master by design");
            } else {
                assert_ne!(positive, control, "{}: an arm re-uses its family's master", decl.id);
            }
        }
    }

    /// **THE FROZEN SIZING** (the freeze wave): K rises 4 → 8 for group-mean
    /// noise, δ and the dead-spread bound stand, and the PILOT posture is
    /// untouched so the pilots that already ran still reproduce.
    #[test]
    fn the_frozen_sizing_raises_k_and_leaves_the_pilot_posture_untouched() {
        let pilot = BlockOpts::pilot();
        assert_eq!(pilot.admission_k, PILOT_ADMISSION_K);
        assert_eq!(pilot.admission_k, 4, "the pilots ran at K = 4 and their readings must stand");

        let freeze = BlockOpts::freeze();
        assert_eq!(freeze.admission_k, FROZEN_ADMISSION_K);
        assert_eq!(freeze.admission_k, 8, "the freeze doubles K against group-mean noise");
        assert_eq!(freeze.delta, DELTA_VP, "δ is the same number in both postures");
        assert_eq!(freeze.delta, pilot.delta);
        assert_eq!(freeze.dead_spread_bound, DEAD_SPREAD_BOUND);
        assert!(
            !freeze.control,
            "the admission inversion is the SEAM's decision, never a posture's",
        );
        assert_eq!(
            format!("{:?}", freeze.driven),
            format!("{:?}", DrivenOpts::default()),
            "the frozen posture runs the driver's own defaults",
        );
        // WHOLE-VALUE, so a field added later cannot be silently equal.
        assert_ne!(
            format!("{freeze:?}"),
            format!("{pilot:?}"),
            "the two postures are the same sizing — then one of them is not a posture",
        );
    }

    /// **FAMILY 1'S ε IS FROZEN AT 2 VP** (the freeze wave, pilot-informed).
    ///
    /// The pilot's trigger-round controls read UNRESOLVED at every root under
    /// ε = 1 at K = 4 — the declared policies' top sets differed by less than a
    /// point of adjudication noise, which is a statement about the instrument
    /// rather than about policy disagreement. ε = 2 is the design's draft value
    /// and is the freeze's answer.
    ///
    /// The leniency direction is pinned where the number is: a LARGER ε is an
    /// EASIER pass, so the second half here shows what the move buys — the ε = 2
    /// top set CONTAINS the ε = 1 one, never the other way round.
    #[test]
    fn family_ones_epsilon_is_frozen_at_two_victory_points() {
        assert_eq!(FROZEN_EPSILON_VP, 2);
        let decl = &families()[0];
        assert_eq!(
            decl.rule,
            PassRule::AdjudicatedBest { epsilon: FROZEN_EPSILON_VP },
            "family 1's declared ε is not the frozen one",
        );

        // Means −100.0 / −99.0 / −98.0 (see `spread_table`): ε = 1 admits two
        // rows and ε = 2 admits all three.
        let adj = spread_table(&[0, 25, 50], None);
        let tight = top_set(&adj, 1);
        let frozen = top_set(&adj, FROZEN_EPSILON_VP);
        assert_eq!(tight.len(), 2);
        assert_eq!(frozen.len(), 3);
        assert!(
            tight.iter().all(|m| frozen.contains(m)),
            "a larger ε must be an EASIER pass — the frozen top set dropped a candidate",
        );
    }

    /// The STRATEGIC-CLASS partition (design §4, review SHOULD-9): the pivot
    /// trigger is undefined without it, so it is pinned rather than described.
    #[test]
    fn the_strategic_class_partition_matches_the_design() {
        let decls = families();
        assert_eq!(decls[0].class, StrategicClass::RulesFinalRound);
        assert_eq!(decls[1].class, StrategicClass::EndTimingProximity);
        assert_eq!(decls[2].class, StrategicClass::OpponentReply);
        assert_eq!(decls[3].class, StrategicClass::EndTimingProximity);
        // The consequence the partition exists for: families 2 and 4 CAN
        // combine to fire the pivot, and family 3's expected failures cannot
        // combine with either.
        assert_eq!(
            decls[1].class, decls[3].class,
            "families 2 and 4 are one class — the end-timing/proximity cluster",
        );
        assert_ne!(decls[2].class, decls[1].class);
        assert_ne!(decls[2].class, decls[0].class);
        assert_ne!(decls[0].class, decls[1].class);
    }

    /// **THE FLIPPED PIN** (the freeze wave): a confirmatory block now RESOLVES
    /// its baked identity, and the missing-block refusal fires only on a
    /// declaration that carries none.
    ///
    /// **Why this is `resolve_block` and not `run_family_block`**: before the
    /// freeze the Ok path could be pinned by running the block, because there
    /// was no Ok path — every family refused. There is one now, and it is
    /// `roots × menu × candidates × K` whole games at the shipped subject, i.e.
    /// hours. `resolve_block` is the whole of what the refusals guard (master
    /// and ordinals, before a single draw is resolved), so pinning it is pinning
    /// the argument contract without buying a block run.
    ///
    /// The refusal CODE is kept and re-pointed rather than deleted: a
    /// declaration whose block is `None` is exactly what a future family arm
    /// looks like on the day it is declared and before it is baked.
    #[test]
    fn a_confirmatory_block_resolves_its_baked_identity_and_a_blockless_decl_refuses() {
        let rules = rules();
        let decl = &families()[0];
        let subject = SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT);
        let frozen = decl.confirmatory.clone().expect("baked at the freeze");

        let (master, ordinals) =
            resolve_block(decl, BlockKind::Confirmatory, None).expect("the frozen block resolves");
        assert_eq!(master, frozen.master, "the block ran another master");
        assert_eq!(ordinals, frozen.ordinals.clone().collect::<Vec<u32>>());

        // THE REFUSAL, re-pointed at the shape it now describes.
        let blockless = FamilyDecl { confirmatory: None, ..decl.clone() };
        let no_block =
            resolve_block(&blockless, BlockKind::Confirmatory, None).unwrap_err();
        assert!(
            matches!(no_block, FamilyRunError::NoFrozenBlock { .. }),
            "expected a missing-frozen-block refusal, got {no_block}",
        );

        // The other two refusals are UNMOVED, and both still fire before a draw
        // — so they stay pinned through the public entry point.
        let substituted =
            run_family_block(decl, BlockKind::Confirmatory, Some(MASTER), subject, &rules)
                .unwrap_err();
        assert!(
            matches!(substituted, FamilyRunError::MasterRefused { .. }),
            "a typed master must not be able to run a plausible-looking non-frozen block: \
             {substituted}",
        );

        let unrecorded =
            run_family_block(decl, BlockKind::Pilot, None, subject, &rules).unwrap_err();
        assert!(
            matches!(unrecorded, FamilyRunError::MasterRequired { .. }),
            "recorded-before-use stays the caller's obligation: {unrecorded}",
        );
    }

    /// **THE BATCH SEAM**: resolving a block's ordinals in one walk produces
    /// descriptors IDENTICAL to resolving them one at a time.
    ///
    /// On the rung ecology, and at ordinals the stride cap keeps inside game 0,
    /// so the cell costs one generated game rather than sixteen.
    #[test]
    fn batch_resolution_reproduces_per_ordinal_descriptors() {
        let rules = rules();
        let predicate = RootPredicate::FinalRoundAlreadyTriggered;
        let ordinals = [0u32, 1];
        let batched = crate::game::driven::generate_roots(predicate, MASTER, &ordinals, &rules)
            .expect("the rung scan supplies two strided roots");
        assert_eq!(batched.len(), 2);
        for (i, ordinal) in ordinals.into_iter().enumerate() {
            let (state, descriptor) =
                crate::game::driven::RootSource::Generated { predicate, master: MASTER, ordinal }
                    .resolve(&rules)
                    .expect("the per-ordinal path supplies the same root");
            assert_eq!(batched[i].0, state, "ordinal {ordinal}: the batch resolved another state");
            assert_eq!(
                batched[i].1, descriptor,
                "ordinal {ordinal}: the batch described the root differently",
            );
        }
    }

    /// The THREE-WAY judgment (design §1): a pick in every policy's top set
    /// passes, in some but not all is UNRESOLVED-pending-exact, and in none
    /// fails.
    #[test]
    fn the_three_way_judgment_reads_all_some_and_no_top_sets() {
        let a = Move::EndAdditionalPhase;
        let b = Move::BuildShophouse { slot: 0 };
        let c = Move::BuildShophouse { slot: 1 };
        let sets = |first: Vec<Move>, second: Vec<Move>| {
            vec![(ReplyPolicy::FamilyWeighted, first), (ReplyPolicy::GreedyAtFinal, second)]
        };

        assert_eq!(
            judge_pick(&a, &sets(vec![a.clone(), b.clone()], vec![a.clone()])),
            PickJudgment::InEverySet,
        );
        assert_eq!(
            judge_pick(&b, &sets(vec![a.clone(), b.clone()], vec![a.clone()])),
            PickJudgment::InSomeSet,
        );
        assert_eq!(
            judge_pick(&c, &sets(vec![a.clone(), b.clone()], vec![a.clone()])),
            PickJudgment::InNoSet,
        );
        // An EMPTY menu is not a judgment: nothing is in every set of no sets,
        // and calling that a pass would pass a mis-wired block.
        assert_eq!(judge_pick(&a, &[]), PickJudgment::InNoSet);
    }

    /// The TOP SET is the rows within epsilon of the best MEAN margin, and
    /// epsilon's leniency direction is pinned: larger epsilon, easier pass.
    #[test]
    fn the_top_set_widens_with_epsilon_and_never_shrinks() {
        // Row i's mean is −100 + delta_i / 25 (see `spread_table`): deltas
        // 0/25/50 give means −100.0, −99.0, −98.0.
        let adj = spread_table(&[0, 25, 50], None);
        assert_eq!(top_set(&adj, 0).len(), 1, "epsilon 0 admits only the best row");
        assert_eq!(top_set(&adj, 1).len(), 2);
        assert_eq!(top_set(&adj, 2).len(), 3);
        // The best row is in every top set, whatever epsilon is.
        for epsilon in [0, 1, 2, 50] {
            assert!(top_set(&adj, epsilon).contains(&adj.rows[2].candidate));
        }
        assert!(top_set(&spread_table(&[], None), 1).is_empty(), "an empty table has no top set");
    }

    /// The FAMILY VERDICT arms, as arithmetic over counts — pass, the failure
    /// allowance, UNRESOLVED, the VOID rule, and the existence exemption.
    #[test]
    fn the_family_verdict_applies_the_allowance_the_void_rule_and_the_existence_exemption() {
        use FamilyVerdict as V;
        let conf = FamilyLabel::Confirmatory;
        let exist = FamilyLabel::ExistenceDemonstration;

        // A clean confirmatory block.
        assert_eq!(family_verdict(conf, BlockKind::Confirmatory, 6, 6, 0, 0), V::Pass);
        // One failed root exceeds the allowance of zero.
        assert!(matches!(
            family_verdict(conf, BlockKind::Confirmatory, 6, 6, 1, 0),
            V::Fail { .. }
        ));
        // An unresolved root is never a silent fail.
        assert!(matches!(
            family_verdict(conf, BlockKind::Confirmatory, 6, 6, 0, 1),
            V::UnresolvedPendingExact { .. }
        ));
        // THE VOID RULE: fewer than four admitted of six drawn.
        assert!(matches!(
            family_verdict(conf, BlockKind::Confirmatory, 6, 3, 0, 0),
            V::VoidAsConfirmation { .. }
        ));
        assert_eq!(family_verdict(conf, BlockKind::Confirmatory, 6, 4, 0, 0), V::Pass);
        // A PILOT block confirms nothing, so it cannot be void as confirmation.
        assert!(matches!(family_verdict(conf, BlockKind::Pilot, 4, 1, 0, 0), V::Pass));
        // The EXISTENCE arm skips the allowance entirely, failures included.
        assert!(matches!(
            family_verdict(exist, BlockKind::Confirmatory, 6, 2, 2, 0),
            V::GapDemonstrated { .. }
        ));
    }

    /// **AN EMPTY DENOMINATOR IS NOT A PASS** (the freeze wave, pilot finding 2).
    ///
    /// The pilot printed `verdict PASS` for blocks that admitted NOTHING — a
    /// PILOT block escapes the VOID rule by design (it confirms nothing, so it
    /// cannot be void AS a confirmation), and the allowance arm then read "0
    /// failures of 0 admitted" as a clean pass. That sentence is the one reading
    /// a block with no denominator must not produce.
    ///
    /// The three arms stay separate on purpose: a CONFIRMATORY block below four
    /// keeps the VOID wording (it is a stronger statement — it names the minimum
    /// it fell short of), the EXISTENCE arm keeps its gap verdict (its output is
    /// per-root gaps, and zero of them is still zero gaps, which the render says
    /// out loud), and everything else with zero admitted reads NOTHING ADMITTED.
    #[test]
    fn a_block_that_admitted_nothing_reads_nothing_even_as_a_pilot() {
        use FamilyVerdict as V;
        let conf = FamilyLabel::Confirmatory;
        let exist = FamilyLabel::ExistenceDemonstration;

        // THE PILOT FINDING, as arithmetic: this read `Pass` before the freeze.
        assert_eq!(
            family_verdict(conf, BlockKind::Pilot, 4, 0, 0, 0),
            V::NothingAdmitted { drawn: 4 },
        );
        // A CONFIRMATORY block keeps the VOID wording — the stronger statement.
        assert!(matches!(
            family_verdict(conf, BlockKind::Confirmatory, 6, 0, 0, 0),
            V::VoidAsConfirmation { admitted: 0, minimum: MIN_ADMITTED_ROOTS, .. },
        ));
        // The EXISTENCE arm keeps its own verdict at zero admitted; what it
        // gains is the render's "this demonstrates nothing" clause.
        assert_eq!(
            family_verdict(exist, BlockKind::Pilot, 4, 0, 0, 0),
            V::GapDemonstrated { roots: 0 },
        );
        // ANTI-VACUITY: ONE admitted root is a reading again.
        assert_eq!(family_verdict(conf, BlockKind::Pilot, 4, 1, 0, 0), V::Pass);
    }

    /// **DEAD is AND-ed across the menu, pinned on MIXED vectors** — the shape
    /// no block a test can afford produces (T5 review MUST-1).
    ///
    /// Every block cell in this file declares a SINGLE-POLICY menu, where AND
    /// and OR are the same function: mutating one into the other passed the
    /// whole library. A one-element vector cannot separate them, so the cells
    /// that matter here are the two mixed ones — one policy calling the field
    /// dead and the other calling it live, in BOTH menu orders, so the rule is
    /// pinned as a conjunction rather than as "whatever the first policy said".
    #[test]
    fn dead_is_and_ed_across_the_declared_menu_and_an_empty_menu_is_not_dead() {
        let p = ReplyPolicy::FamilyWeighted;
        let q = ReplyPolicy::LookaheadAtFinal;

        // THE DISCRIMINATOR: disagreement is not indifference, either way round.
        assert!(
            !dead_across_menu(&[(p, true), (q, false)]),
            "one policy finding the field LIVE makes the root live",
        );
        assert!(
            !dead_across_menu(&[(p, false), (q, true)]),
            "the menu's ORDER decided the verdict — this is not the AND",
        );
        // Unanimity is the only way to dead.
        assert!(dead_across_menu(&[(p, true), (q, true)]));
        assert!(!dead_across_menu(&[(p, false), (q, false)]));
        // The EMPTY vector is NOT dead — deliberately not `all`'s vacuous truth.
        assert!(!dead_across_menu(&[]), "no adjudication is not an indifferent field");
        // A one-policy menu is the degenerate case the block cells exercise, and
        // it agrees with `classify_dead` verbatim — which is exactly why it can
        // never discriminate between the two readings above.
        assert!(dead_across_menu(&[(p, true)]));
        assert!(!dead_across_menu(&[(p, false)]));
    }

    /// The family-4 RIDER's proximity mirror reproduces a HAND-COMPUTED value.
    ///
    /// The mirror names `examples/eval_probe.rs`'s
    /// `achievement_race_proximity` as its source in-source (the harness §5
    /// pattern for example-local logic); this is the pin that keeps the copy
    /// honest, on a state whose three gaps are constructed rather than drawn.
    #[test]
    fn the_achievement_rider_mirrors_the_probes_proximity_arithmetic() {
        let rules = rules();
        let mut state = post_setup_2p(&rules, 0x31);
        let mover = state.current_player;
        state.achievements = [
            AchievementCard::new(AchievementType::Copper5),
            AchievementCard::new(AchievementType::Money10),
            AchievementCard::new(AchievementType::Tea7),
        ];
        for p in state.players.iter_mut() {
            p.copper = 0;
            p.silk = 0;
            p.tea = 0;
            p.seafood = 0;
            p.money = 0;
            p.imported_goods = 0;
        }
        {
            let p = state.player_mut(mover);
            p.copper = 3; // gap 2 toward Copper5
            p.money = 8; // gap 2 toward Money10
            p.tea = 4; // gap 3 toward Tea7
        }
        let view = crate::game::observe::observe(&state, mover).expect("observable");
        let me = view.players.iter().find(|p| p.id == mover).expect("the mover is in its own view");
        let rider = achievement_rider(&view, me);

        // HAND-COMPUTED: the capped progress sum is 3 + 8 + 4 = 15, nothing is
        // claimed so the post-claim sum is the same, and the nearest gap is 2.
        assert_eq!(rider.progress_sum, 15);
        assert_eq!(rider.postclaim_sum, 15);
        assert_eq!(rider.race_proximity, -2);

        // A CLAIM by this seat drops its own leg from the post-claim sum and
        // from the proximity scan, leaving Money10's gap 2 and Tea7's 3.
        state.achievements[0].claimed_by.push(mover);
        let view = crate::game::observe::observe(&state, mover).expect("observable");
        let me = view.players.iter().find(|p| p.id == mover).expect("in view");
        let after = achievement_rider(&view, me);
        assert_eq!(after.progress_sum, 15, "the pre-freeze sum has no claim filter");
        assert_eq!(after.postclaim_sum, 12, "the shipped sum drops the claimed leg");
        assert_eq!(after.race_proximity, -2, "Money10's gap is still the nearest");
    }

    /// A SMOKE FAMILY: two constructed final-round roots where the mover can
    /// claim exactly one achievement and declining ends its turn.
    ///
    /// Final-round so a playout is at most two turns — the whole reason the
    /// block smoke is affordable at all.
    fn smoke_root(seed: u64, rules: &Rules) -> Result<BuiltRoot, BuilderError> {
        let mut state = post_setup_2p(rules, 0x40 + seed);
        let mover = state.current_player;
        state.achievements = [
            AchievementCard::new(AchievementType::Money10),
            AchievementCard::new(AchievementType::Tea7),
            AchievementCard::new(AchievementType::Seafood7),
        ];
        for p in state.players.iter_mut() {
            p.money = 0;
            p.copper = 0;
            p.silk = 0;
            p.tea = 0;
            p.seafood = 0;
            p.imported_goods = 0;
        }
        state.player_mut(mover).money = 10;
        state.turn_phase = TurnPhase::AdditionalActionSecond;
        // The trigger's three fields move together — `check_end_game_triggers`
        // always writes all of them, and `check_invariants` refuses a state
        // where they do not agree.
        state.end_game_triggered = true;
        state.trigger_reason = Some(crate::types::EndGameTrigger::ChurchTrackFull);
        state.trigger_player = Some(mover);
        state.final_round = true;
        state.pending_choices.clear();
        crate::game::invariants::check_invariants(&state, rules)
            .map_err(BuilderError::NotEngineValid)?;
        Ok(BuiltRoot { state, step: None })
    }

    /// A declaration over [`smoke_root`], at smoke scale.
    fn smoke_decl(label: FamilyLabel, per_root_f: u32) -> FamilyDecl {
        FamilyDecl {
            id: "smoke",
            label,
            class: StrategicClass::EndTimingProximity,
            generator: FamilyGenerator::Builder { build: smoke_root, name: "smoke_root" },
            rule: PassRule::Turn(TurnPredicate::AchievementClaimed),
            replications: 2,
            per_root_f,
            menu: SMOKE_MENU,
            pilot: 0..2,
            confirmatory: Some(FrozenBlock { master: MASTER, ordinals: 0..2 }),
            // The smoke never runs a control arm; the field is declared so the
            // fixture is a whole `FamilyDecl` rather than a partial one.
            confirmatory_control: None,
            canon: "a machinery smoke, not a family",
        }
    }

    /// The smoke's declared menu — TWO policies, because the freeze wave's
    /// admission needs [`MIN_SPLITTING_POLICIES`] of them before any root is a
    /// demonstration. Both are cheap at a final-round root (a playout is at most
    /// two turns), which is what keeps the block cells affordable.
    const SMOKE_MENU: &[ReplyPolicy] = &[ReplyPolicy::FamilyWeighted, ReplyPolicy::GreedyAtFinal];

    /// The smoke's own sizing: K = 2 and a delta that admits.
    ///
    /// **K = 2 and not 1** since the freeze wave: at K = 1 the smoke's two
    /// candidates give one playout a side, which is [`SPLIT_MIN_GROUP`]'s thin
    /// split — the group mean would be one game's margin, and the rule that
    /// refuses to read it is the rule these cells run under.
    fn smoke_opts(delta: i64) -> BlockOpts {
        BlockOpts { admission_k: 2, delta, ..BlockOpts::pilot() }
    }

    /// **THE BLOCK SMOKE** — every arm of the run, end to end at smoke scale:
    /// an ADMITTED root read to PASS and to FAIL, an EXCLUDED one, the VOID
    /// rule, and the EXISTENCE arm's exemption.
    ///
    /// One lookahead subject, two roots, R = 2, K = 1, a one-policy menu and
    /// final-round playouts. Budget ~3s debug (design §6's wall discipline for
    /// the one searching cell); **MEASURED 0.04s**, because the roots are
    /// two-candidate additional-action decisions and a final-round playout is at
    /// most two turns. The budget is the ceiling this cell must not grow past.
    ///
    /// **A FIXTURE FACT worth recording rather than tuning away**: at these
    /// constructed final-round roots the shipped subject values
    /// `EndAdditionalPhase` at 20 against `FulfillAchievement{Money10}` at 4 and
    /// hands off WITHOUT claiming, so the rule misses at every replication. That
    /// is what gives the smoke its FAIL arm for free, and it is a fact about two
    /// hand-built positions at R = 2 — NOT a family-4 reading, which is T7's
    /// business at drawn roots under the full menu.
    #[test]
    fn the_block_run_covers_pass_fail_excluded_void_and_the_existence_arm() {
        let rules = rules();
        let subject = SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT);

        // (1) ADMITTED and PASSING — the allowance absorbs the misses. Run as a
        // CONFIRMATORY block, so the VOID rule is exercised with real admitted
        // roots behind it rather than only with zero.
        let passing = smoke_decl(FamilyLabel::Confirmatory, 2);
        let report =
            run_family_block_with(&passing, BlockKind::Confirmatory, None, subject, &rules,
                smoke_opts(-1_000))
                .expect("the smoke block runs");
        assert_eq!(report.drawn, 2);
        assert_eq!(report.admitted, 2, "a permissive delta admits both roots: {:?}", report.roots);
        assert!(report.roots.iter().all(|r| r.read.is_some()), "an admitted root is READ");
        assert!(
            report.roots.iter().all(|r| r.verdict == RootVerdict::Pass),
            "misses within the allowance are a PASS: {:?}",
            report.roots.iter().map(|r| &r.verdict).collect::<Vec<_>>(),
        );
        assert!(
            report.roots.iter().all(|r| r.rider.is_some()),
            "an AchievementClaimed family records its rider at every root",
        );
        // Two admitted of two drawn is below the VOID rule's four, so the
        // block's own verdict IS void — the honest reading of a two-root block,
        // asserted rather than papered over.
        assert!(
            matches!(report.verdict, FamilyVerdict::VoidAsConfirmation { admitted: 2, .. }),
            "a two-root confirmatory block cannot confirm: {:?}",
            report.verdict,
        );
        assert_eq!(report.subject_echo, Contestant::LOOKAHEAD_V1_DEFAULT.echo());
        assert_eq!(report.robustness_read, crate::game::driven::ROBUSTNESS_READ);
        assert_eq!(report.generator, "smoke_root");
        assert_eq!(report.counter_refused, 0);

        // (2) ADMITTED and FAILING — the same reads against an allowance of
        // zero, run as a PILOT so the family verdict is the allowance's and not
        // the VOID rule's.
        let failing = smoke_decl(FamilyLabel::Confirmatory, 0);
        let failed = run_family_block_with(&failing, BlockKind::Pilot, Some(MASTER), subject,
            &rules, smoke_opts(-1_000))
            .expect("the smoke block runs");
        assert!(
            failed.roots.iter().all(|r| matches!(r.verdict, RootVerdict::Fail { misses: 2, .. })),
            "{:?}",
            failed.roots.iter().map(|r| &r.verdict).collect::<Vec<_>>(),
        );
        assert!(
            matches!(failed.verdict, FamilyVerdict::Fail { failed_roots: 2, .. }),
            "{:?}",
            failed.verdict,
        );

        // (3) EXCLUDED: an unreachable delta demonstrates nothing, so no root is
        // read at all and the block is void with zero admitted.
        let excluded = run_family_block_with(&passing, BlockKind::Confirmatory, None, subject,
            &rules, smoke_opts(10_000))
            .expect("the smoke block runs");
        assert_eq!(excluded.admitted, 0);
        assert!(
            excluded.roots.iter().all(|r| {
                r.verdict == RootVerdict::Excluded(ExclusionReason::NotDemonstrated)
            }),
            "{:?}",
            excluded.roots.iter().map(|r| &r.verdict).collect::<Vec<_>>(),
        );
        assert!(excluded.roots.iter().all(|r| r.read.is_none()), "an excluded root is not read");
        assert!(matches!(
            excluded.verdict,
            FamilyVerdict::VoidAsConfirmation { admitted: 0, .. }
        ));

        // (4) THE EXISTENCE ARM: the same failing block under family 3's label
        // skips the allowance entirely and emits the gap block instead.
        let gap_decl = smoke_decl(FamilyLabel::ExistenceDemonstration, 0);
        let gap = run_family_block_with(&gap_decl, BlockKind::Pilot, Some(MASTER), subject, &rules,
            smoke_opts(-1_000))
            .expect("the existence block runs");
        assert!(
            gap.roots.iter().all(|r| matches!(r.verdict, RootVerdict::GapRecorded { misses: 2 })),
            "the existence arm records the gap instead of failing: {:?}",
            gap.roots.iter().map(|r| &r.verdict).collect::<Vec<_>>(),
        );
        assert!(
            matches!(gap.verdict, FamilyVerdict::GapDemonstrated { roots: 2 }),
            "the existence arm declares no bar: {:?}",
            gap.verdict,
        );
    }

    /// **THE BUILDER DRAW'S SEED IS RECORDED, not merely derivable** (T5 review
    /// NOTE-3) — the registry's own lesson, one level down.
    ///
    /// The seed IS derivable: it is the `(ordinal + 1)`-th world of
    /// `SeedSchedule::diagnostic(master)`. That is exactly what makes recording
    /// it worth a field — a number a reader must re-derive silently re-points
    /// the moment the derivation moves, and a builder row's descriptor is
    /// honestly `Constructed` and so names nothing else at all.
    ///
    /// The pin is therefore not "the field is `Some`" but the property the field
    /// exists for: **the recorded seed rebuilds the row's own root**, digest for
    /// digest, through the declaration's own builder. The `None` half rides a
    /// PREDICATE draw, where there is no builder and no input seed and the
    /// descriptor's `Generated` origin is the whole identity.
    #[test]
    fn a_builder_block_records_the_seed_each_root_was_built_from() {
        let rules = rules();
        let subject = SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT);
        let decl = smoke_decl(FamilyLabel::Confirmatory, 2);
        let report = run_family_block_with(&decl, BlockKind::Pilot, Some(MASTER), subject, &rules,
            smoke_opts(-1_000))
            .expect("the smoke block runs");
        assert_eq!(report.roots.len(), 2);

        // The DECLARED derivation, restated here rather than read off the rows.
        let mut schedule = SeedSchedule::diagnostic(MASTER);
        let stream: Vec<u64> = (0..report.roots.len()).map(|_| schedule.next_world().0).collect();
        for (row, seed) in report.roots.iter().zip(&stream) {
            assert_eq!(row.draw_seed, Some(*seed), "ordinal {}: the wrong draw", row.ordinal);
            // THE PROPERTY: the recorded seed rebuilds this row's own root.
            let rebuilt = smoke_root(*seed, &rules).expect("the recorded seed rebuilds").state;
            assert_eq!(
                state_digest(&rebuilt),
                row.descriptor.root_digest,
                "ordinal {}: the recorded seed builds another position",
                row.ordinal,
            );
        }
        // ANTI-VACUITY: two ordinals draw two DIFFERENT seeds, so a field
        // populated with one constant could not pass.
        assert_ne!(report.roots[0].draw_seed, report.roots[1].draw_seed);

        // THE `None` HALF, at the cheap (rung) ecology: a scanned draw carries a
        // `Generated` descriptor and no builder seed. Run through `resolve_draws`
        // rather than a whole block — the adjudication is not what is under test.
        let scanned = FamilyDecl {
            generator: FamilyGenerator::Predicate(RootPredicate::FinalRoundAlreadyTriggered),
            ..decl
        };
        let draws = resolve_draws(&scanned, MASTER, &[0], &rules).expect("the scan finds a root");
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].draw_seed, None, "a scanned root has no builder seed to record");
        assert!(
            matches!(draws[0].descriptor.origin, RootOrigin::Generated { .. }),
            "the scanned draw's identity is its origin: {:?}",
            draws[0].descriptor.origin,
        );
    }

    /// **THE CANNOT-DEMONSTRATE ARM**, through the block: a rule every playout
    /// satisfies has no FALSE group, so the root is recorded-excluded rather
    /// than admitted on a comparison that does not exist.
    ///
    /// At these roots the trigger is already set, so `EndFired` holds at every
    /// hand-off whatever the forced candidate was — which is exactly the
    /// one-sided shape design §4 refuses to read.
    #[test]
    fn a_root_whose_playouts_all_agree_is_recorded_excluded_not_admitted() {
        let rules = rules();
        let subject = SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT);
        let decl = FamilyDecl {
            rule: PassRule::Turn(TurnPredicate::EndFired),
            ..smoke_decl(FamilyLabel::Confirmatory, 0)
        };
        let report = run_family_block_with(&decl, BlockKind::Pilot, Some(MASTER), subject, &rules,
            smoke_opts(-1_000))
            .expect("the block runs");
        assert_eq!(report.admitted, 0, "a one-sided root demonstrates nothing");
        for row in &report.roots {
            // THE REASON MOVED AT THE FREEZE WAVE and this is the honest one: no
            // policy split its playouts at all, so the root is excluded for
            // having no measurement rather than for a measurement that failed.
            assert_eq!(
                row.verdict,
                RootVerdict::Excluded(ExclusionReason::TooFewSplitting {
                    splitting: 0,
                    minimum: MIN_SPLITTING_POLICIES,
                }),
            );
            let admission = row.admission.as_ref().expect("a turn-grain root carries an admission");
            assert!(
                admission.policies.iter().all(|p| {
                    p.verdict == AdmissionVerdict::Cannot(CannotDemonstrate::NoFalseGroup)
                }),
                "expected a NoFalseGroup refusal, got {:?}",
                admission.policies,
            );
        }
        // ANTI-VACUITY: the rider is a family-4 declaration and does NOT ride a
        // family whose rule is something else.
        assert!(report.roots.iter().all(|r| r.rider.is_none()));
    }

    /// **THE DECISION-GRAIN ARM**, through the block: family 1's shape — a
    /// per-policy adjudication, the LIVE/DEAD split as admission, and the
    /// three-way judgment wired to the read's picks.
    ///
    /// The three-way rule itself is pinned as arithmetic in
    /// `the_three_way_judgment_reads_all_some_and_no_top_sets`; what this cell
    /// adds is the WIRING — that `run_family_block` reaches
    /// `PassRule::AdjudicatedBest`'s branch at all, fills `top_sets`, and judges
    /// the read's picks against them rather than leaving the arm dead.
    ///
    /// **The same FIXTURE FACT the block smoke records, seen from the decision
    /// grain**: the subject picks `EndAdditionalPhase` while the adjudicated top
    /// set is the CLAIM, so the row FAILS. That disagreement is what makes the
    /// cell non-vacuous — the judgment discriminated rather than admitting
    /// everything — and it is a fact about two hand-built positions at K = 1,
    /// not a family-1 reading.
    #[test]
    fn a_decision_grain_block_admits_on_the_live_dead_split_and_judges_the_picks() {
        let rules = rules();
        let subject = SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT);
        let decl = FamilyDecl {
            rule: PassRule::AdjudicatedBest { epsilon: FROZEN_EPSILON_VP },
            replications: 1,
            ..smoke_decl(FamilyLabel::Confirmatory, 0)
        };
        let report = run_family_block_with(&decl, BlockKind::Pilot, Some(MASTER), subject, &rules,
            smoke_opts(DELTA_VP))
            .expect("the block runs");

        assert_eq!(report.admitted, 2, "a live field is admitted: {:?}", report.roots);
        for row in &report.roots {
            assert!(row.admission.is_none(), "the decision grain admits on LIVE/DEAD, not δ");
            assert_eq!(
                row.dead_by_policy.len(),
                SMOKE_MENU.len(),
                "one entry per declared policy",
            );
            assert!(!row.dead, "the fixture's field is LIVE");
            assert_eq!(row.top_sets.len(), SMOKE_MENU.len());
            assert!(
                row.top_sets.iter().all(|(_, set)| !set.is_empty()),
                "a live field has a top set under every policy",
            );
            let read = row.read.as_ref().expect("an admitted root is read");
            assert!(
                read.replications.iter().all(|r| r.pick.is_some() && r.verdict.is_none()),
                "a decision-grain read reports picks and no turn verdict",
            );
            // ANTI-VACUITY: the judgment DISCRIMINATED. The subject's pick is
            // outside EVERY adjudicated top set at this fixture, so the row
            // fails — a judgment that admitted everything would read Pass here.
            let pick = read.replications[0].pick.clone().expect("a pick");
            assert!(
                row.top_sets.iter().all(|(_, set)| !set.contains(&pick)),
                "the fixture does not separate the two: {:?} vs {:?}",
                pick,
                row.top_sets,
            );
            assert_eq!(
                row.verdict,
                RootVerdict::Fail { misses: 1, allowance: 0 },
                "a pick in NO top set is the fail arm",
            );
            assert_eq!(row.unresolved, 0, "a pick in no set at all cannot be UNRESOLVED");
        }
        // A one-policy menu makes `lookahead_sole_dissenter` unstatable, which
        // is the accessor's own documented requirement — pinned so the flag is
        // never read as "no suppression" at a menu that cannot show it.
        assert!(report.sole_dissenters.is_empty());
    }

    /// [`smoke_root`] with the CLAIM taken away: at `AdditionalActionSecond`
    /// with nothing claimable, no standing additional action and no technology,
    /// `EndAdditionalPhase` is the only legal move — a FORCED decision.
    fn forced_root(seed: u64, rules: &Rules) -> Result<BuiltRoot, BuilderError> {
        let mut built = smoke_root(seed, rules)?;
        let mover = built.state.current_player;
        built.state.player_mut(mover).money = 0;
        check_invariants(&built.state, rules).map_err(BuilderError::NotEngineValid)?;
        Ok(built)
    }

    /// **A FORCED ROOT IS EXCLUDED AND NEVER READ** (the freeze wave, pilot
    /// finding 1) — through the block, at the decision grain.
    ///
    /// The pilot LIVE-admitted a one-candidate family-1 root and read it PASS:
    /// the subject's pick was the only move on offer, so it was trivially in
    /// every adjudicated top set. A one-row table confirms nothing and fails
    /// nothing, and this is the exclusion that says so.
    ///
    /// The cell carries its own ANTI-VACUITY in the second half: the SAME
    /// declaration over the two-candidate smoke root still admits and still
    /// reads, so the rule excludes forced fields rather than decision-grain
    /// roots.
    #[test]
    fn a_forced_decision_grain_root_is_excluded_and_never_read() {
        let rules = rules();
        let subject = SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT);
        let decl = FamilyDecl {
            generator: FamilyGenerator::Builder { build: forced_root, name: "forced_root" },
            rule: PassRule::AdjudicatedBest { epsilon: 1 },
            replications: 1,
            pilot: 0..1,
            ..smoke_decl(FamilyLabel::Confirmatory, 0)
        };

        // THE FIXTURE'S PREMISE, asserted rather than assumed.
        let built = forced_root(0x77, &rules).expect("the forced root builds");
        assert_eq!(
            get_legal_moves(&built.state, &rules),
            vec![Move::EndAdditionalPhase],
            "the fixture is not forced",
        );

        let report = run_family_block_with(&decl, BlockKind::Pilot, Some(MASTER), subject, &rules,
            smoke_opts(-1_000))
            .expect("the block runs");
        assert_eq!(report.drawn, 1);
        assert_eq!(report.admitted, 0, "a FORCED root was admitted: {:?}", report.roots);
        assert_eq!(
            report.roots[0].verdict,
            RootVerdict::Excluded(ExclusionReason::Forced { candidates: 1 }),
            "the forced exclusion did not fire: {:?}",
            report.roots[0].verdict,
        );
        assert!(report.roots[0].read.is_none(), "a forced root was READ");
        assert!(
            !report.roots[0].dead,
            "the pilot's shape: a lone WINNING candidate is `live` by the wins clause, which is \
             why the forced arm cannot be left to `classify_dead`",
        );

        // ANTI-VACUITY: the two-candidate root of the same shape still admits.
        let live = FamilyDecl {
            generator: FamilyGenerator::Builder { build: smoke_root, name: "smoke_root" },
            ..decl
        };
        let report = run_family_block_with(&live, BlockKind::Pilot, Some(MASTER), subject, &rules,
            smoke_opts(-1_000))
            .expect("the block runs");
        assert_eq!(report.admitted, 1, "the LIVE path moved: {:?}", report.roots);
        assert!(report.roots[0].read.is_some());
    }

    /// Family 3's declared CONTROL and family 4's declared CORNER are reachable
    /// as generators and BUILD — a declaration nothing can run is a declaration
    /// that rots.
    #[test]
    fn the_declared_control_and_corner_generators_build_their_roots() {
        let rules = rules();
        for generator in [RACE_CONTROL_GENERATOR, PROXIMITY_CORNER_GENERATOR] {
            let FamilyGenerator::Builder { build, name } = generator else {
                panic!("{generator:?} is not a builder arm");
            };
            let built = build(0, &rules).unwrap_or_else(|e| panic!("{name}: {e}"));
            crate::game::invariants::check_invariants(&built.state, &rules)
                .unwrap_or_else(|e| panic!("{name}: not engine-valid: {e}"));
            assert_eq!(generator.name(), name);
        }
        // The control declares a step (its rule is step-carrying); the corner
        // does not (family 4's rule names none).
        let FamilyGenerator::Builder { build: control, .. } = RACE_CONTROL_GENERATOR else {
            unreachable!()
        };
        assert!(control(0, &rules).expect("builds").step.is_some());
        let FamilyGenerator::Builder { build: corner, .. } = PROXIMITY_CORNER_GENERATOR else {
            unreachable!()
        };
        assert!(corner(0, &rules).expect("builds").step.is_none());
    }

    /// **THE THIRD DECLARED ARM** (T5 review NOTE-4): family 2's control, beside
    /// family 3's and family 4's, so all three share ONE convention a runner can
    /// expose without a fifth registry row.
    ///
    /// This one is a PREDICATE arm rather than a builder — the complement is a
    /// scan question, and nothing about it is constructed — so the pin its two
    /// siblings get by BUILDING is here the two statements a predicate arm can
    /// make: it names the predicate the declaration's comment names, and it is
    /// the EXACT COMPLEMENT of family 2's positive generator on the truth-margin
    /// sign (same firability, same width, opposite band). A scan is deliberately
    /// NOT run — family 2 scans the v2 lookahead ecology, which is minutes.
    #[test]
    fn the_three_declared_control_and_corner_arms_share_one_convention() {
        let arms = [
            FAMILY2_CONTROL_GENERATOR,
            RACE_CONTROL_GENERATOR,
            PROXIMITY_CORNER_GENERATOR,
        ];
        let names: Vec<&str> = arms.iter().map(FamilyGenerator::name).collect();
        assert_eq!(names.len(), 3);
        assert!(names.iter().all(|n| !n.is_empty()), "a declared arm with no name: {names:?}");
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), 3, "two declared arms answer to one name: {names:?}");

        // Family 2's arm, specifically: the predicate its declaration names.
        assert_eq!(
            FAMILY2_CONTROL_GENERATOR,
            FamilyGenerator::Predicate(RootPredicate::EndFirableBehindInTruth),
        );
        // THE COMPLEMENT: family 2's positive generator is the same predicate
        // with the margin band's sign flipped, and the two are never equal.
        let positive = families()[1].generator;
        assert_eq!(
            positive,
            FamilyGenerator::Predicate(RootPredicate::EndFirableAheadInTruth),
            "family 2's positive generator moved out from under its control",
        );
        assert_ne!(FAMILY2_CONTROL_GENERATOR, positive, "a control equal to its positive");
    }

    /// A step-carrying rule is RE-POINTED at the root's own step, and a root
    /// that declares none is a typed refusal — never a read against the
    /// placeholder.
    #[test]
    fn a_step_carrying_rule_is_repointed_by_the_root_and_refuses_without_one() {
        let declared = ContestedStep::CustomsSpace { space: 5 };
        let taken = PassRule::Turn(TurnPredicate::ContestedStepTaken(DECLARED_BY_THE_ROOT));
        let denial = PassRule::Turn(TurnPredicate::DenialNotTaken(DECLARED_BY_THE_ROOT));
        assert!(taken.names_a_step() && denial.names_a_step());
        assert_eq!(
            taken.with_step(Some(declared)),
            PassRule::Turn(TurnPredicate::ContestedStepTaken(declared)),
        );
        assert_eq!(
            denial.with_step(Some(declared)),
            PassRule::Turn(TurnPredicate::DenialNotTaken(declared)),
        );
        // A rule that names no step is untouched by either argument.
        let claimed = PassRule::Turn(TurnPredicate::AchievementClaimed);
        assert!(!claimed.names_a_step());
        assert_eq!(claimed.with_step(Some(declared)), claimed);
        assert_eq!(taken.with_step(None), taken, "with no step there is nothing to re-point");

        // THE REFUSAL: a step-carrying rule over a generator whose roots declare
        // none never reads against `DECLARED_BY_THE_ROOT`.
        let rules = rules();
        let decl = FamilyDecl { rule: taken, ..smoke_decl(FamilyLabel::Confirmatory, 0) };
        let err = run_family_block_with(
            &decl,
            BlockKind::Pilot,
            Some(MASTER),
            SeatSpec::Contestant(Contestant::LOOKAHEAD_V1_DEFAULT),
            &rules,
            smoke_opts(-1_000),
        )
        .unwrap_err();
        assert!(
            matches!(err, FamilyRunError::StepNotDeclared { ordinal: 0 }),
            "expected a step-not-declared refusal, got {err}",
        );
    }
}
