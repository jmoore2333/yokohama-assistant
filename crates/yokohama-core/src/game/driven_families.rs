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

use crate::game::arena::{mix, Contestant};
use crate::game::driven::{
    drive_turn, root_table, Adjudication, DrivenError, DrivenOpts, NotAContestant, ReplyPolicy,
    RootTable, RootTableError, SeatCounters, SeatSpec, TOP_BAND_POINTS,
};
use crate::game::rules::Rules;
use crate::game::semantics::state_digest;
use crate::types::{AchievementType, ChurchTrack, CustomsTrack, GameState, Move, PlayerId};

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


// ---------------------------------------------------------------------------
// Root admission (design §4, review MUST-2/MUST-4)
// ---------------------------------------------------------------------------

/// Why one reply policy's adjudication could DEMONSTRATE nothing about a root.
///
/// Typed rather than a bare "no": each arm routes differently. Two are facts
/// about the POSITION (the playouts all did the thing, or none of them did — the
/// comparison has no other side), and one is a fact about the CALL (the
/// adjudication carried no verdicts at all, so it was run without design §2's
/// turn hook and this is a caller error rather than a reading).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CannotDemonstrate {
    /// No playout satisfied the predicate — there is no TRUE group to compare.
    NoTrueGroup,
    /// Every playout satisfied it — there is no FALSE group to compare.
    NoFalseGroup,
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
            Self::Unstamped => {
                "the adjudication carries no turn verdicts (it was run without the turn hook)"
            }
        }
    }
}

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
    /// The CONJUNCTION: the root is admitted only if every declared policy
    /// demonstrates dominance.
    ///
    /// Vacuously false on an empty menu — an admission "across no policies" is
    /// not a demonstration, and returning `true` there would admit every root of
    /// a mis-wired block.
    pub fn admitted(&self) -> bool {
        !self.policies.is_empty() && self.policies.iter().all(PolicyAdmission::dominates)
    }

    /// The policies that did NOT demonstrate dominance, in menu order.
    pub fn dissenters(&self) -> Vec<ReplyPolicy> {
        self.policies
            .iter()
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
    /// Requires a menu of at least two policies — with `lookahead-at-final`
    /// alone there is no "other policies held" half to observe.
    pub fn lookahead_sole_dissenter(&self) -> bool {
        self.policies.len() >= 2
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
/// **Both groups must be non-empty**, else [`CannotDemonstrate`]: a root where
/// every playout fires (or none does) has no comparison in it, and is
/// recorded-excluded rather than counted either way.
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
        let read = admission(&[(ReplyPolicy::GreedyAtFinal, adj)], 5, false);
        let p = &read.policies[0];
        assert_eq!((p.true_n, p.false_n), (2, 2));
        assert!((p.true_mean - 0.0).abs() < 1e-12, "the TRUE group is level, not won: {p:?}");
        assert!((p.false_mean + 10.0).abs() < 1e-12);
        assert!((p.difference - 10.0).abs() < 1e-12);
        assert_eq!(p.verdict, AdmissionVerdict::Dominates);
        assert!(read.admitted());

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
        let good = || verdict_table(&[(10, false, Some(true)), (-10, false, Some(false))]);
        let bad = || verdict_table(&[(0, false, Some(true)), (0, false, Some(false))]);

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
}
