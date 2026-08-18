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
//! **The pieces, and how they compose** (design §4's protocol read
//! bottom-up):
//!
//! 1. [`TurnPredicate`] judges ONE (root, hand-off) pair. It is the grain
//!    families 2–4 read at, because canon 15, F8 and the race question are all
//!    statements about what a TURN accomplished rather than about which move
//!    started it.
//! 2. [`classify_dead`] answers family 1's admission question at the DECISION
//!    grain: is this root's candidate field genuinely indifferent, so that a
//!    pick there confirms nothing and fails nothing?
//! 3. ROOT ADMISSION and the BINARY READ land in this task's second wave — the
//!    behaviour-vs-behaviour dominance test families 2–4 admit roots by, and
//!    the R-replication read that reports verdicts, seeds and D7's step-one
//!    counter check.
//!
//! **What this module deliberately does NOT do**: judge a family. The three-way
//! all-sets/some-sets/no-set rule for `AdjudicatedBest`, the conjunction across
//! the reply menu, the sole-dissenter routing and the VOID-below-4-admitted rule
//! are the FAMILY RUN's (T5) — they are statements about a BLOCK of roots, and
//! this module never sees more than one.

use crate::game::driven::Adjudication;
use crate::game::rules::Rules;
use crate::types::{AchievementType, ChurchTrack, CustomsTrack, GameState, PlayerId};

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
        // DECLARED PLACEHOLDER (T4 wave A, RED): nothing is ever held, so the
        // positive arms fail on assertions and the denial arms pass vacuously —
        // which is exactly the asymmetry the ill-formed-step pin is about.
        let _ = doubled_industry_value(state, mover, rules);
        false
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
        // DECLARED PLACEHOLDER (T4 wave A, RED): every predicate reads false,
        // so each arm's positive pin fails on its assertion while its inversion
        // control passes — the two halves are separately visible in the RED
        // output.
        let mover = root.current_player;
        let _ = (claimed_types(handoff, mover), rules);
        false
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
    /// **`epsilon` is carried, not applied, by the read** — the top set
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
    // DECLARED PLACEHOLDER (T4 wave A, RED): the MUST-1 criterion is not
    // implemented yet, so the measured-session cell fails on its assertion
    // while the two LIVE cells pass — the RED output separates the halves.
    let _ = (adj, spread_bound);
    false
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::driven::{
        adjudicate_root, adjudicate_root_with, AdjudicationProvenance, AdjudicationSeat,
        CandidatePlayout, CandidateRow, DrivenOpts, ReplyPolicy, RootDescriptor, RootOrigin,
        RootSource, SeatSpec,
    };
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
    /// The threshold is the dummy's 15 (D5's 7.5 doubled), which is exactly the
    /// level a real total can never tie: industry 7 doubles to 14 (below) and
    /// industry 8 doubles to 16 (past).
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
}
