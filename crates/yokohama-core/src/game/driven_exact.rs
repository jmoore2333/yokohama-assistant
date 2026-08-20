//! The EXACT bounded-tree adjudicator — an exhaustive perfect-information solve
//! of the remaining game at an IN-FINAL-ROUND root.
//!
//! Spec: `docs/plans/2026-08-20-exact-adjudicator-spec.md` (APPROVED 2026-08-20;
//! its §11 rulings bind this module). Authority trail: the Phase-C family freeze
//! §9 named this instrument as a TRIGGER — "a LIVE family-1 root whose per-policy
//! top sets disagree AT THE FROZEN PARAMETERS reads UNRESOLVED-pending-exact, and
//! building the exact instrument is the named next step" — and the Phase-B
//! harness design scoped it in the same sentence that authorised it: "only
//! family 1's in-final-round roots qualify for it."
//!
//! # What this is NOT
//!
//! It does not replace [`crate::game::driven::adjudicate_root`]. That verb is a
//! ROBUSTNESS READ — candidates forced, remainders PLAYED OUT under named reply
//! policies — and it answers "what do these policies reach from here". This
//! module answers "what is this position worth under optimal opposition". **The
//! two are different questions and a root's full record wants both**; where the
//! policies disagreed (the UNRESOLVED class) this one supplies the pole they
//! were proxying.
//!
//! # The epistemic claim, stated once
//!
//! [`EXACT_CLAIRVOYANT_READ`] is the sentence every report prints FIRST. The
//! claim it makes and the claim it refuses are both load-bearing — see that
//! constant, and read it before quoting any number this module produces.
//!
//! # Why the problem is small here, and only here
//!
//! Once the end is triggered the remainder is bounded, and in the FINAL round it
//! is two turns at 2P (measured: spec §6.2). A turn is a contiguous run of ONE
//! player's decisions, so the tree is max-over-the-mover's-turn then
//! min-over-the-opponent's — very few alternations, which is what makes an exact
//! solve affordable at all. A trigger-round root has a whole further round and
//! is REFUSED: the smallest one measured exceeded 600,000,000 nodes inside its
//! first root candidate (spec §6.7). A midgame root is unbounded and is refused
//! earlier still.
//!
//! # Determinism
//!
//! **This module draws no randomness, takes no master and derives no seed.** Its
//! answer is a pure function of `(root, rules, bounds)`. It declares no salt, so
//! the `A3E1_D21E_*` family's next free serial is unchanged by this lane.

use std::collections::HashMap;

use crate::game::apply_move::apply_move_unchecked;
use crate::game::driven::{require_eligible_root, DrivenError, RootDescriptor, RootError, RootSource};
use crate::game::end_game::is_game_over;
use crate::game::invariants::check_invariants;
use crate::game::legal_moves::get_legal_moves;
use crate::game::move_legality::{is_move_legal, PathMode};
use crate::game::rules::Rules;
use crate::game::scoring::final_result;
use crate::game::semantics::{state_digest, SEMANTICS_REV};
use crate::types::{GameState, Move, PlayerId};

/// The sentence every exact report prints FIRST, verbatim.
///
/// A `pub const` rather than prose in the runner, on [`crate::game::driven::ROBUSTNESS_READ`]'s
/// grounds: the claim is written once so a report cannot quietly print a weaker
/// or stronger one. All four clauses are deliberate — the third is the one a
/// reader is most likely to drop, and dropping it turns this instrument into a
/// misplay accusation it cannot support.
pub const EXACT_CLAIRVOYANT_READ: &str =
    "exact read: a PERFECT-INFORMATION solve of the remaining game over the TRUE state, hidden \
     cards included, with BOTH seats played optimally — so a value here is what the position is \
     worth under best play against best defence. It is therefore an UPPER BOUND on achievable \
     quality: a subject pick outside the optimal set is NOT by itself misplay, because the subject \
     could not see what this solve can. PRIVILEGED, analysis-side only — never an input to any \
     evaluator, weight fit, or contestant.";

/// The exact worth of a position to the ROOT MOVER.
///
/// **The ordering is `(margin, won)` lexicographic and the second field is
/// deliberate belt-and-braces.** At 2P `won` is a function of `margin` plus a
/// constant — the official tie-break is fixed once the start player is — so it
/// only ever separates `margin == 0`. Carrying it explicitly is what keeps
/// "**margin 0 is NOT a win**" from being lost inside an arithmetic shortcut,
/// which is exactly the mistake `CandidatePlayout::margin` is shaped to prevent
/// one layer down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExactValue {
    /// The mover's final total minus the BEST RIVAL'S BY RANK, in victory
    /// points. Never a bare `max` over totals.
    pub margin: i64,
    /// Straight off `final_result`'s `winner`. A margin of 0 is not a win.
    pub won: bool,
}

impl ExactValue {
    /// Below every reachable value, for alpha-beta's opening window.
    const NEG_INF: Self = Self { margin: i64::MIN / 4, won: false };
    /// Above every reachable value.
    const POS_INF: Self = Self { margin: i64::MAX / 4, won: true };
}

/// The bounds that CONTAIN the solve. Never quality dials: exceeding one is a
/// typed refusal (spec §2.2), because there is nothing to approximate down to.
#[derive(Debug, Clone, Copy)]
pub struct ExactBounds {
    /// Expanded decision nodes.
    pub max_nodes: u64,
    /// Decisions from the root.
    pub max_depth: u32,
    /// Transposition-table entries. **0 DISABLES memoization** (the soundness
    /// cell's switch); any positive value is a cap whose breach refuses.
    pub max_memo_entries: usize,
}

impl Default for ExactBounds {
    fn default() -> Self {
        Self { max_nodes: 2_000_000_000, max_depth: 512, max_memo_entries: 4_000_000 }
    }
}

/// Why an exact table could not be produced.
///
/// Every variant is a HARD stop that produces NOTHING — `AdjudicationError`'s
/// posture, for its reason: a table missing a candidate reads exactly like a
/// table of a position with fewer options, and the reading riding on it would be
/// wrong with no way to notice. What a refusal DOES carry is the COUNTED bound
/// reached, because that is a real measurement about the root.
#[derive(Debug)]
pub enum ExactError {
    /// The end has not been triggered — a MIDGAME root, whose remainder is
    /// unbounded. Refused BY NAME before anything is expanded, never by
    /// exhausting a budget.
    NotTriggered { root_digest: String },
    /// Triggered, but the final round has not begun — the TRIGGER-ROUND class,
    /// which carries a whole further round. Bounded, but one round too large:
    /// spec §6.7 measured the smallest such root past 600,000,000 nodes inside
    /// its first candidate. A DIFFERENT fact from [`Self::NotTriggered`] and
    /// reported apart from it.
    NotFinalRound { root_digest: String },
    /// Not a 2-player game. At 3–4P "best rival by rank" is not antisymmetric,
    /// so the margin is not zero-sum and minimax does not apply.
    NotTwoPlayer { players: u8 },
    /// The root would not resolve.
    Root(RootError),
    /// The root's own guards refused it (hidden deals, corruption).
    Unadjudicable(DrivenError),
    /// A non-terminal state with an empty legal list.
    DeadEnd { seat: PlayerId },
    /// `final_result` yielded no standing for a seat.
    Unscorable { seat: PlayerId },
    /// A move `get_legal_moves` listed was then refused. An engine bug rather
    /// than an answer, and never a skipped row.
    ForcedMove { candidate: Box<Move>, detail: String },
    /// The node budget was exceeded. Carries the COUNTED total.
    NodeBudget { bound: u64, counted: u64, candidate_index: usize },
    /// The depth bound was exceeded.
    DepthBound { bound: u32, reached: u32 },
    /// The memo cap was exceeded.
    MemoBudget { bound: usize, entries: usize },
}

impl std::fmt::Display for ExactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotTriggered { root_digest } => write!(
                f,
                "{root_digest}: midgame root — the end game is not triggered, so the remainder is \
                 unbounded and there is nothing final-round-small to solve"
            ),
            Self::NotFinalRound { root_digest } => write!(
                f,
                "{root_digest}: trigger-round root — the end is triggered but the final round has \
                 not begun, so a whole further round remains. Out of scope for this instrument \
                 (spec §2.1); its frozen verdict stands untouched"
            ),
            Self::NotTwoPlayer { players } => write!(
                f,
                "{players}-player root: the exact solve is 2P-only — at 3+ seats the \
                 best-rival-by-rank margin is not antisymmetric and minimax is unsound"
            ),
            Self::Root(e) => write!(f, "root: {e}"),
            Self::Unadjudicable(e) => write!(f, "unadjudicable root: {e}"),
            Self::DeadEnd { seat } => {
                write!(f, "p{}: a non-terminal state with no legal move", seat.0)
            }
            Self::Unscorable { seat } => {
                write!(f, "p{}: final_result produced no standing for this seat", seat.0)
            }
            Self::ForcedMove { candidate, detail } => {
                write!(f, "the engine listed {candidate:?} and then refused it: {detail}")
            }
            Self::NodeBudget { bound, counted, candidate_index } => write!(
                f,
                "node budget {bound} exceeded at candidate {candidate_index} (counted {counted} \
                 expanded decision nodes — a lower bound on this root's tree, never a ceiling)"
            ),
            Self::DepthBound { bound, reached } => {
                write!(f, "depth bound {bound} exceeded (reached {reached} decisions from the root)")
            }
            Self::MemoBudget { bound, entries } => {
                write!(f, "memo cap {bound} exceeded ({entries} entries)")
            }
        }
    }
}

impl std::error::Error for ExactError {}

/// One root candidate and its exact worth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactRow {
    pub candidate: Move,
    pub value: ExactValue,
    /// Decision nodes expanded while solving THIS candidate. Recorded, never
    /// asserted: it is a fact about the tree's shape, not about the answer.
    pub nodes: u64,
}

/// What the solve cost. Every field RECORDED, none gated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactStats {
    pub nodes: u64,
    pub memo_entries: usize,
    pub memo_hits: u64,
    pub max_depth: u32,
}

/// Which position was solved, under which semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactProvenance {
    pub root: RootDescriptor,
    /// The RUNNING engine's `SEMANTICS_REV` — the comparability key. A table
    /// from another rev is not a table about this root.
    pub semantics_rev: u32,
    pub rules_hash: String,
}

/// Every root candidate with its exact value — the instrument's whole output.
///
/// **The verdict stays the caller's.** This type reports what each candidate is
/// worth; whether a subject's pick being outside [`Self::argmax_set`] means
/// misplay is a reading it does not make, and [`EXACT_CLAIRVOYANT_READ`] says
/// why that distinction matters here more than usual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactTable {
    pub provenance: ExactProvenance,
    /// The seat on the move at the root — whose perspective every value is in.
    pub mover: PlayerId,
    /// One row per candidate, in `get_legal_moves` order (so the rows join the
    /// frozen adjudicator's rows positionally).
    pub rows: Vec<ExactRow>,
    pub stats: ExactStats,
}

impl ExactTable {
    /// The best exact value in the table, or `None` at a terminal root.
    pub fn best(&self) -> Option<ExactValue> {
        self.rows.iter().map(|r| r.value).max()
    }

    /// The TRUE argmax — every row whose value equals the best, ε = 0.
    ///
    /// A SET, never a representative: the family rule asks whether a pick is IN
    /// the top set, and picking one of several value-equal optima would
    /// manufacture a distinction the position does not contain.
    pub fn argmax_set(&self) -> Vec<&ExactRow> {
        match self.best() {
            None => Vec::new(),
            Some(best) => self.rows.iter().filter(|r| r.value == best).collect(),
        }
    }

    /// Every row within `eps_vp` VICTORY POINTS of the best.
    ///
    /// **On an exact read ε is a declared INDIFFERENCE BAND, not noise cover.**
    /// The frozen family parameter (ε = 2 VP) was derived as playout-noise cover
    /// and this instrument has no sampling noise; it is retained as the primary
    /// verdict rule purely for comparability, so the six recorded FAILs are
    /// re-read on the tolerance they failed under (spec §7.3).
    pub fn top_set(&self, eps_vp: i64) -> Vec<&ExactRow> {
        match self.best() {
            None => Vec::new(),
            Some(best) => {
                self.rows.iter().filter(|r| best.margin - r.value.margin <= eps_vp).collect()
            }
        }
    }
}

/// Solve every root candidate exactly.
///
/// **Candidates are the engine's own [`get_legal_moves`], which is QUEUE-FIRST**
/// — at a root with a pending choice the candidates are that choice's
/// resolutions rather than the phase's moves. That is `adjudicate_root`'s
/// convention and it is correct for the same reason: the decision the mover
/// actually faces there IS the queued choice. It also makes the two tables join
/// row-for-row by construction rather than by trust.
///
/// **Each root candidate is solved under a FULL WINDOW — no cut is taken across
/// root siblings.** This costs real time and buys the thing the mandate needs:
/// alpha-beta across siblings returns BOUNDS for the non-best candidates, and
/// the table needs every candidate's VALUE (the top set, the subject's pick's
/// worth, and the gap between them). A best-move-only search would be far
/// cheaper and would not answer the question asked.
pub fn exact_table(
    source: &RootSource,
    rules: &Rules,
    bounds: ExactBounds,
) -> Result<ExactTable, ExactError> {
    let (root, descriptor) = source.resolve(rules).map_err(ExactError::Root)?;
    // The scope fence, checked BEFORE anything is expanded, so a midgame or
    // trigger-round root is refused by NAME rather than by burning a budget.
    if root.config.player_count != 2 {
        return Err(ExactError::NotTwoPlayer { players: root.config.player_count });
    }
    if !root.end_game_triggered {
        return Err(ExactError::NotTriggered { root_digest: descriptor.root_digest.clone() });
    }
    if !root.final_round {
        return Err(ExactError::NotFinalRound { root_digest: descriptor.root_digest.clone() });
    }
    require_eligible_root(&root).map_err(ExactError::Unadjudicable)?;
    check_invariants(&root, rules)
        .map_err(|e| ExactError::Unadjudicable(DrivenError::CorruptRoot(e)))?;

    let mover = root.current_player;
    let candidates = get_legal_moves(&root, rules);
    if candidates.is_empty() && !is_game_over(&root) {
        return Err(ExactError::DeadEnd { seat: mover });
    }

    let mut solver = Solver::new(rules, mover, bounds);
    let mut rows = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.into_iter().enumerate() {
        if !is_move_legal(&root, &candidate, rules, PathMode::Canonical) {
            return Err(ExactError::ForcedMove {
                candidate: Box::new(candidate),
                detail: "is_move_legal(Canonical) rejected a move from get_legal_moves".to_string(),
            });
        }
        let after = apply_move_unchecked(&root, &candidate, rules).map_err(|e| {
            ExactError::ForcedMove { candidate: Box::new(candidate.clone()), detail: e.to_string() }
        })?;
        let before = solver.nodes;
        // A root candidate's successor is a TURN ENTRY exactly when the mover's
        // turn ended on that move — the `drive_turn` dual stop, reused so this
        // module cannot disagree with the harness about where a turn ends.
        let entry = is_game_over(&after) || after.current_player != mover;
        let value =
            solver.solve(&after, ExactValue::NEG_INF, ExactValue::POS_INF, 1, entry, index)?;
        rows.push(ExactRow { candidate, value, nodes: solver.nodes - before });
    }

    Ok(ExactTable {
        provenance: ExactProvenance {
            root: descriptor,
            semantics_rev: SEMANTICS_REV,
            rules_hash: rules.content_hash(),
        },
        mover,
        rows,
        stats: solver.stats(),
    })
}

/// Whether a transposition entry is an exact value or only a bound.
///
/// Needed because a value found under a NARROWED window is not the position's
/// value — it is only known to lie on one side of it. Storing all three kinds
/// and honouring the distinction on lookup is what keeps memoization sound; the
/// soundness cell proves it by running the same roots with the table disabled.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Flag {
    Exact,
    Lower,
    Upper,
}

struct Solver<'a> {
    rules: &'a Rules,
    /// The ROOT mover — the seat every value is in the perspective of, and the
    /// maximizing side.
    mover: PlayerId,
    bounds: ExactBounds,
    nodes: u64,
    memo_hits: u64,
    max_depth: u32,
    memo: HashMap<String, (ExactValue, Flag)>,
}

impl<'a> Solver<'a> {
    fn new(rules: &'a Rules, mover: PlayerId, bounds: ExactBounds) -> Self {
        Self {
            rules,
            mover,
            bounds,
            nodes: 0,
            memo_hits: 0,
            max_depth: 0,
            memo: HashMap::new(),
        }
    }

    fn stats(&self) -> ExactStats {
        ExactStats {
            nodes: self.nodes,
            memo_entries: self.memo.len(),
            memo_hits: self.memo_hits,
            max_depth: self.max_depth,
        }
    }

    /// The leaf quantity: `final_result`'s official standings, best rival by
    /// RANK.
    ///
    /// The standings are ALREADY in the official order (total descending, then
    /// the clockwise tie-break), so the first non-mover row IS the best rival by
    /// that same order — which is what makes the tie-break part of the margin's
    /// definition rather than an afterthought applied to a `max`.
    fn terminal_value(&self, state: &GameState) -> Result<ExactValue, ExactError> {
        let result = final_result(state, self.rules);
        let mine = result
            .standings
            .iter()
            .find(|s| s.player == self.mover)
            .ok_or(ExactError::Unscorable { seat: self.mover })?;
        let rival = result
            .standings
            .iter()
            .find(|s| s.player != self.mover)
            .ok_or(ExactError::Unscorable { seat: self.mover })?;
        Ok(ExactValue {
            margin: i64::from(mine.breakdown.total) - i64::from(rival.breakdown.total),
            won: result.winner == self.mover,
        })
    }

    /// Exact alpha-beta to terminal.
    ///
    /// `turn_entry` marks a state whose mover differs from its parent's — the
    /// ONLY states the transposition table stores, so the digest cost stays
    /// proportional to hand-offs rather than to nodes. That choice is measured,
    /// not assumed: storing every node cut expanded nodes 3.2x but cost 4.1x
    /// wall at the spec's anchor root, because `state_digest` per node costs
    /// more than the nodes it saves (spec §6.5).
    ///
    /// Nodes do NOT alternate every ply: a turn is a contiguous run of one
    /// player's decisions, so the same side maximises many plies in a row.
    /// Alpha-beta is correct over that shape unchanged.
    fn solve(
        &mut self,
        state: &GameState,
        mut alpha: ExactValue,
        mut beta: ExactValue,
        depth: u32,
        turn_entry: bool,
        candidate_index: usize,
    ) -> Result<ExactValue, ExactError> {
        self.nodes += 1;
        if self.nodes > self.bounds.max_nodes {
            return Err(ExactError::NodeBudget {
                bound: self.bounds.max_nodes,
                counted: self.nodes,
                candidate_index,
            });
        }
        self.max_depth = self.max_depth.max(depth);
        if depth > self.bounds.max_depth {
            return Err(ExactError::DepthBound { bound: self.bounds.max_depth, reached: depth });
        }
        if is_game_over(state) {
            return self.terminal_value(state);
        }
        let moves = get_legal_moves(state, self.rules);
        if moves.is_empty() {
            return self.terminal_value(state);
        }

        let alpha_orig = alpha;
        let beta_orig = beta;
        let key = self.memo_key(state, turn_entry);
        if let Some(k) = &key {
            if let Some((value, flag)) = self.memo.get(k).copied() {
                match flag {
                    Flag::Exact => {
                        self.memo_hits += 1;
                        return Ok(value);
                    }
                    Flag::Lower if value >= beta => {
                        self.memo_hits += 1;
                        return Ok(value);
                    }
                    Flag::Lower => alpha = alpha.max(value),
                    Flag::Upper if value <= alpha => {
                        self.memo_hits += 1;
                        return Ok(value);
                    }
                    Flag::Upper => beta = beta.min(value),
                }
            }
        }

        let mover_here = state.current_player;
        let maximizing = mover_here == self.mover;
        let mut best = if maximizing { ExactValue::NEG_INF } else { ExactValue::POS_INF };
        for mv in &moves {
            let Ok(next) = apply_move_unchecked(state, mv, self.rules) else { continue };
            let child_entry = is_game_over(&next) || next.current_player != mover_here;
            let value = self.solve(&next, alpha, beta, depth + 1, child_entry, candidate_index)?;
            if maximizing {
                best = best.max(value);
                alpha = alpha.max(best);
            } else {
                best = best.min(value);
                beta = beta.min(best);
            }
            if alpha >= beta {
                break;
            }
        }

        if let Some(k) = key {
            let flag = if best <= alpha_orig {
                Flag::Upper
            } else if best >= beta_orig {
                Flag::Lower
            } else {
                Flag::Exact
            };
            if self.memo.len() >= self.bounds.max_memo_entries && !self.memo.contains_key(&k) {
                return Err(ExactError::MemoBudget {
                    bound: self.bounds.max_memo_entries,
                    entries: self.memo.len(),
                });
            }
            self.memo.insert(k, (best, flag));
        }
        Ok(best)
    }

    /// The transposition key, or `None` where this node is not stored.
    ///
    /// **Only TURN-ENTRY states are stored**, per the spec's §5.4 ruling, and
    /// the reason is measured rather than aesthetic: storing every node cut
    /// expanded nodes 3.2x at the anchor root but cost 4.1x WALL, because
    /// `state_digest` per node costs more than the nodes it saves. It is also
    /// where the value is — a whole opponent-turn subtree is a pure function of
    /// its hand-off state.
    ///
    /// `max_memo_entries == 0` DISABLES memoization outright — the switch the
    /// soundness cell flips.
    fn memo_key(&self, state: &GameState, turn_entry: bool) -> Option<String> {
        if self.bounds.max_memo_entries == 0 || !turn_entry {
            return None;
        }
        Some(state_digest(state))
    }
}

#[cfg(test)]
mod tests {
    use crate::game::driven_exact::*;
    use crate::game::end_game::is_game_over;
    use crate::game::invariants::check_invariants;
    use crate::game::legal_moves::get_legal_moves;
    use crate::game::apply_move::apply_move_unchecked;
    use crate::game::driven::{adjudicate_root, DrivenOpts, ReplyPolicy, RootSource, SeatSpec};
    use crate::game::seeds::WorldSeed;
    use crate::game::rules::Rules;
    use crate::game::scoring::final_result;
    use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
    use crate::types::{
        AreaId, BuildingSite, EndGameTrigger, GameConfig, GameState, MercenaryState, Move,
        PlayerId, PresidentLocation, TradingHouseState, TurnPhase,
    };

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

    /// A LAST-SEAT in-final-round Building root whose every candidate is worth a
    /// hand-computable number of VP and whose remainder is FORCED.
    ///
    /// The mover is the seat `is_game_over` keys on — `(start + n - 1) % n`,
    /// derived rather than the literal 1 — so `advance_phase`'s
    /// `final_round && next_player == start_player` arm ends the game without
    /// the opponent ever moving. That makes this fixture a ONE-PLY tree with
    /// known leaves: exactly what an argmax cell wants and deliberately NOT what
    /// the alpha-beta equivalence cell wants (see [`handoff_building_root`]).
    ///
    /// Every seat's final total is its `victory_points`, because the fixture
    /// zeroes every other leg `final_result` adds: no track pawns (so the 2P
    /// dummy takes church/customs first place and is awarded nothing), no
    /// technologies (so the technology majority's participation gate denies
    /// both), no completed orders or tech icons (so no country set), and no
    /// mercenaries, imports, money or trade goods (so no remainder bonus). A
    /// shophouse therefore adds exactly the doctored space's printed `vp`.
    fn near_terminal_building_root(
        rules: &Rules,
        seed: u64,
        space_vp: [u8; 4],
        mover_vp: u32,
        rival_vp: u32,
    ) -> (GameState, PlayerId, AreaId) {
        let mut state = post_setup_2p(rules, seed);
        let n = state.config.player_count;
        let start = state.config.start_player.0;
        let mover = PlayerId::new((start + n - 1) % n);
        strip_to_vp_only(&mut state, mover, mover_vp, rival_vp);
        let area = arm_building_site(&mut state, mover, space_vp);
        state.current_player = mover;
        state.turn_phase = TurnPhase::Building;
        state.current_action_power = Some(4);
        finalize_final_round(&mut state);
        check_invariants(&state, rules).expect("the doctored root must be a state the engine accepts");
        assert!(!is_game_over(&state), "anti-vacuity: the root is already terminal");
        (state, mover, area)
    }

    /// A FIRST-SEAT in-final-round root at an arbitrary turn PHASE, so the
    /// mover's turn hands off and the opponent gets a real final turn.
    ///
    /// The phase is the knob the equivalence fixture needs. A Building-phase
    /// root leaves the rest of the mover's turn FORCED, and a forced maximizing
    /// layer cannot expose a broken cut — mutation testing proved exactly that
    /// against the first version of this module's equivalence cell.
    fn handoff_root_at(
        rules: &Rules,
        seed: u64,
        space_vp: [u8; 4],
        mover_vp: u32,
        rival_vp: u32,
        phase: TurnPhase,
    ) -> (GameState, PlayerId) {
        let mut state = post_setup_2p(rules, seed);
        let mover = state.config.start_player;
        strip_to_vp_only(&mut state, mover, mover_vp, rival_vp);
        arm_building_site(&mut state, mover, space_vp);
        state.current_player = mover;
        state.turn_phase = phase;
        state.current_action_power = Some(4);
        finalize_final_round(&mut state);
        check_invariants(&state, rules).expect("the doctored root must be a state the engine accepts");
        assert!(!is_game_over(&state), "anti-vacuity: the root is already terminal");
        (state, mover)
    }

    /// Zero every scoring leg except `victory_points`, both seats.
    fn strip_to_vp_only(state: &mut GameState, mover: PlayerId, mover_vp: u32, rival_vp: u32) {
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
    }

    /// Put the mover's president on a building site whose four spaces are PURE
    /// VP — `BuildingSite::new` is the engine's own VP-only constructor, so no
    /// reward icon can sneak in through a hand-written literal.
    fn arm_building_site(state: &mut GameState, mover: PlayerId, space_vp: [u8; 4]) -> AreaId {
        let area = *state
            .board_layout
            .building_sites
            .keys()
            .next()
            .expect("the 2P board carries at least one building site");
        state.board_layout.building_sites.insert(area, BuildingSite::new(space_vp, 0));
        let m = state.player_mut(mover);
        m.president_location = PresidentLocation::OnBoard(area);
        m.trading_houses = TradingHouseState::new();
        area
    }

    fn finalize_final_round(state: &mut GameState) {
        state.is_mercenary_action = false;
        state.mercenary_target_area = None;
        state.president_start_area = None;
        state.pending_choices.clear();
        state.end_game_triggered = true;
        state.final_round = true;
        state.trigger_reason = Some(EndGameTrigger::ChurchTrackFull);
        state.trigger_player = Some(state.config.start_player);
    }

    /// A NAIVE exhaustive minimax — no pruning, no memoization, no bounds.
    ///
    /// The independent oracle the equivalence cell compares against. It is
    /// deliberately the dumbest correct thing that can compute this quantity: if
    /// it and the shipped solver ever disagree, the shipped solver is wrong.
    fn reference_minimax(state: &GameState, rules: &Rules, mover: PlayerId) -> ExactValue {
        if is_game_over(state) {
            return terminal_value_for(state, rules, mover);
        }
        let moves = get_legal_moves(state, rules);
        if moves.is_empty() {
            return terminal_value_for(state, rules, mover);
        }
        let maximizing = state.current_player == mover;
        let mut best: Option<ExactValue> = None;
        for mv in &moves {
            let Ok(next) = apply_move_unchecked(state, mv, rules) else { continue };
            let v = reference_minimax(&next, rules, mover);
            best = Some(match best {
                None => v,
                Some(b) if maximizing => b.max(v),
                Some(b) => b.min(v),
            });
        }
        best.expect("a non-empty move list yields at least one value")
    }

    /// The leaf quantity, recomputed independently of the module under test.
    fn terminal_value_for(state: &GameState, rules: &Rules, mover: PlayerId) -> ExactValue {
        let result = final_result(state, rules);
        let mine = result.standings.iter().find(|s| s.player == mover).expect("mover standing");
        let rival = result.standings.iter().find(|s| s.player != mover).expect("rival standing");
        ExactValue {
            margin: i64::from(mine.breakdown.total) - i64::from(rival.breakdown.total),
            won: result.winner == mover,
        }
    }

    // =======================================================================
    // Pin 1 — the hand-computable optimal set
    // =======================================================================

    /// On a one-ply fixture whose four building spaces are worth 1/5/2/3 VP, the
    /// exact optimum is the 5-VP space and its value is arithmetic a reader can
    /// check without running anything.
    #[test]
    fn the_exact_solver_returns_the_hand_computed_optimal_set() {
        let rules = rules();
        let (root, mover, area) = near_terminal_building_root(&rules, 0x51, [1, 5, 2, 3], 20, 18);
        let candidates = get_legal_moves(&root, &rules);
        // The fixture's own shape, pinned: the four builds plus the skip, and
        // nothing else, so the arithmetic below is over the WHOLE table rather
        // than a chosen subset. A data change that adds a candidate fails here
        // rather than silently re-shaping the margins.
        assert!(
            candidates
                .iter()
                .all(|mv| matches!(mv, Move::BuildShophouse { .. } | Move::SkipBuilding)),
            "fixture drift: {candidates:?}"
        );
        assert_eq!(
            candidates.iter().filter(|mv| matches!(mv, Move::BuildShophouse { .. })).count(),
            4,
            "fixture drift: the site should offer four slots: {candidates:?}"
        );
        let _ = area;

        let table = exact_table(&RootSource::Constructed(root), &rules, ExactBounds::default())
            .expect("the fixture is in the final round and small");

        assert_eq!(table.mover, mover);
        assert_eq!(table.rows.len(), candidates.len());
        // mover 20 + the space's printed vp, rival 18 -> 3, 7, 4, 5 for the four
        // builds, and 2 for the skip. Hand arithmetic, no run required.
        let mut margins: Vec<i64> = table.rows.iter().map(|r| r.value.margin).collect();
        margins.sort_unstable();
        assert_eq!(margins, vec![2, 3, 4, 5, 7], "leaf arithmetic moved");

        let best = table.best().expect("a non-empty table has a best");
        assert_eq!(best, ExactValue { margin: 7, won: true });
        let argmax = table.argmax_set();
        assert_eq!(argmax.len(), 1, "the 5-VP space is the unique optimum");
        assert_eq!(argmax[0].value.margin, 7);
    }

    /// Value-EQUAL optima are reported as a SET, never collapsed to a
    /// representative — the family rule asks whether a pick is IN the top set.
    #[test]
    fn value_equal_optima_are_all_reported() {
        let rules = rules();
        let (root, _, _) = near_terminal_building_root(&rules, 0x52, [4, 4, 1, 2], 20, 18);
        let table = exact_table(&RootSource::Constructed(root), &rules, ExactBounds::default())
            .expect("the fixture solves");
        let argmax = table.argmax_set();
        assert_eq!(argmax.len(), 2, "two spaces are worth 4 VP and both are optimal");
        assert!(argmax.iter().all(|r| r.value.margin == 6));
    }

    // =======================================================================
    // Pin 2 — THE correctness cell: alpha-beta == naive minimax
    // =======================================================================

    /// Interior decision points where the ROOT MOVER is on the move, below the
    /// root, with more than one option — the layer a broken cut corrupts.
    ///
    /// Exists because the first version of the equivalence cell was VACUOUS
    /// against exactly that bug: a Building-phase fixture leaves the rest of the
    /// mover's turn forced, so an over-aggressive cut (break as soon as alpha
    /// rises, instead of on alpha >= beta) changed no value and the cell stayed
    /// green. Mutation testing found it; this counter is the fix, asserted
    /// rather than assumed.
    fn branching_interior_max_nodes(state: &GameState, rules: &Rules, mover: PlayerId, depth: u32) -> usize {
        if is_game_over(state) {
            return 0;
        }
        let moves = get_legal_moves(state, rules);
        if moves.is_empty() {
            return 0;
        }
        let here = usize::from(depth > 0 && state.current_player == mover && moves.len() > 1);
        here + moves
            .iter()
            .filter_map(|mv| apply_move_unchecked(state, mv, rules).ok())
            .map(|next| branching_interior_max_nodes(&next, rules, mover, depth + 1))
            .sum::<usize>()
    }

    /// The shipped solver's per-candidate values equal a naive exhaustive
    /// minimax's, on fixtures carrying BOTH a minimising layer and a BRANCHING
    /// interior maximising layer.
    ///
    /// Both conditions are load-bearing and both are asserted, not hoped for. A
    /// one-ply tree has no opponent layer for a bad cut to corrupt; a tree whose
    /// interior maximising nodes are all forced has no alpha to raise wrongly.
    /// Mutation-proven against two independent breakages: an over-aggressive cut
    /// and a minimising layer that maximises.
    #[test]
    fn alpha_beta_agrees_with_naive_minimax_where_both_layers_branch() {
        let rules = rules();
        let fixtures = [
            handoff_root_at(&rules, 0x81, [1, 5, 2, 3], 20, 18, TurnPhase::AreaAction),
            handoff_root_at(&rules, 0x61, [3, 3, 6, 1], 20, 18, TurnPhase::Building),
        ];
        let mut saw_branching_max = false;
        for (root, mover) in fixtures {
            let branching = branching_interior_max_nodes(&root, &rules, mover, 0);
            saw_branching_max |= branching > 0;
            let table =
                exact_table(&RootSource::Constructed(root.clone()), &rules, ExactBounds::default())
                    .expect("the fixture solves");
            assert!(table.rows.len() >= 2, "anti-vacuity: a one-row table proves nothing");

            for row in &table.rows {
                let after = apply_move_unchecked(&root, &row.candidate, &rules)
                    .expect("a listed move applies");
                let reference = reference_minimax(&after, &rules, mover);
                assert_eq!(
                    row.value, reference,
                    "alpha-beta disagreed with naive minimax on {:?}",
                    row.candidate
                );
            }
        }
        assert!(
            saw_branching_max,
            "ANTI-VACUITY: no fixture carried a branching interior maximising node, so this cell              cannot see an over-aggressive cut — the exact bug mutation testing caught here"
        );
    }

    // =======================================================================
    // Pin — margin 0 is NOT a win
    // =======================================================================

    /// A dead-level finish reads `margin 0, won FALSE` for the seat that loses
    /// the official tie-break — the rule a signed difference cannot express.
    ///
    /// The mover here is `(start + n - 1) % n`, i.e. NOT the start player, and
    /// the official order breaks ties by clockwise closeness to the start
    /// player. So the tie goes against this seat, and `won` must say so. A
    /// solver that derived `won` from `margin >= 0` passes every other cell in
    /// this module and fails this one.
    #[test]
    fn a_level_finish_is_not_a_win_for_the_seat_that_loses_the_tie_break() {
        let rules = rules();
        let (root, mover, _) = near_terminal_building_root(&rules, 0x53, [0, 0, 0, 0], 20, 20);
        assert_ne!(mover, root.config.start_player, "fixture drift: the mover must lose the tie");
        let table = exact_table(&RootSource::Constructed(root), &rules, ExactBounds::default())
            .expect("the fixture solves");
        assert!(
            table.rows.iter().any(|r| r.value.margin == 0),
            "anti-vacuity: no candidate finished level"
        );
        for row in table.rows.iter().filter(|r| r.value.margin == 0) {
            assert!(
                !row.value.won,
                "margin 0 was reported as a WIN for the seat that loses the tie-break: {:?}",
                row.candidate
            );
        }
        // And the ordering agrees: a level finish ranks BELOW any positive
        // margin and ABOVE any negative one.
        assert!(ExactValue { margin: 0, won: false } < ExactValue { margin: 1, won: false });
        assert!(ExactValue { margin: 0, won: false } > ExactValue { margin: -1, won: true });
    }

    // =======================================================================
    // Pin — the transposition table's BOUND flags
    // =======================================================================

    /// A value found under a NARROWED window is a bound, not the position's
    /// value, and re-querying that node under a full window must still return
    /// the truth.
    ///
    /// Targeted rather than incidental: the ordinary cells never reach this
    /// code, because a full-window root search stores only `Exact` entries at
    /// these fixture sizes — mutation testing showed that flattening every flag
    /// to `Exact` left all of them green. This cell FORCES the situation by
    /// searching once under a deliberately narrow window (so bounds land in the
    /// table) and then asking the same solver again with a full one.
    #[test]
    fn a_bound_stored_under_a_narrow_window_is_never_reused_as_a_value() {
        let rules = rules();
        let (root, mover) =
            handoff_root_at(&rules, 0x81, [1, 5, 2, 3], 20, 18, TurnPhase::AreaAction);
        let candidate = get_legal_moves(&root, &rules).remove(0);
        let after = apply_move_unchecked(&root, &candidate, &rules).expect("a listed move applies");
        let truth = reference_minimax(&after, &rules, mover);
        let entry = is_game_over(&after) || after.current_player != mover;

        let mut solver = Solver::new(&rules, mover, ExactBounds::default());
        // A window entirely ABOVE the true value: every node cuts, so what lands
        // in the table are UPPER BOUNDS rather than values.
        let lo = ExactValue { margin: truth.margin + 5, won: true };
        let hi = ExactValue { margin: truth.margin + 6, won: true };
        solver.solve(&after, lo, hi, 1, entry, 0).expect("the narrow search completes");
        assert!(
            solver.memo.values().any(|(_, flag)| *flag != Flag::Exact),
            "ANTI-VACUITY: the narrow window stored no bound, so this cell cannot see the bug"
        );

        let again = solver
            .solve(&after, ExactValue::NEG_INF, ExactValue::POS_INF, 1, entry, 0)
            .expect("the full search completes");
        assert_eq!(
            again, truth,
            "a bound stored under a narrow window was handed back as an exact value"
        );
    }

    // =======================================================================
    // Pins — the scope fence and its typed refusals
    // =======================================================================

    /// A MIDGAME root is refused BY NAME, and the refusal happens BEFORE
    /// anything is expanded.
    ///
    /// **`max_nodes: 1` is what makes the second half observable**: if the
    /// structural check ran after any expansion, this call would come back
    /// `NodeBudget`. It comes back `NotTriggered`, so the fence is genuinely
    /// pre-flight — `route_enum`'s probe-then-delegate posture, with refusal in
    /// place of the delegation there is nothing here to delegate to.
    #[test]
    fn a_midgame_root_is_refused_by_name_before_any_expansion() {
        let rules = rules();
        let root = post_setup_2p(&rules, 0x41);
        assert!(!root.end_game_triggered, "anti-vacuity: the fixture must be midgame");
        let err = exact_table(
            &RootSource::Constructed(root),
            &rules,
            ExactBounds { max_nodes: 1, ..ExactBounds::default() },
        )
        .expect_err("a midgame root must refuse");
        assert!(
            matches!(err, ExactError::NotTriggered { .. }),
            "expected NotTriggered, got {err:?}"
        );
        assert!(err.to_string().contains("midgame root"), "the refusal must name what it saw: {err}");
    }

    /// A TRIGGER-ROUND root is refused as its own fact, apart from the midgame
    /// one — the class the spec's §6.7 measurement put out of reach, and the
    /// nine control-arm roots' disposition.
    #[test]
    fn a_trigger_round_root_is_refused_apart_from_the_midgame_case() {
        let rules = rules();
        let mut root = post_setup_2p(&rules, 0x42);
        root.end_game_triggered = true;
        root.final_round = false;
        root.trigger_reason = Some(EndGameTrigger::ChurchTrackFull);
        root.trigger_player = Some(root.config.start_player);
        check_invariants(&root, &rules).expect("a triggered non-final state is still valid");
        let err = exact_table(
            &RootSource::Constructed(root),
            &rules,
            ExactBounds { max_nodes: 1, ..ExactBounds::default() },
        )
        .expect_err("a trigger-round root must refuse");
        assert!(
            matches!(err, ExactError::NotFinalRound { .. }),
            "expected NotFinalRound, got {err:?}"
        );
        assert!(
            err.to_string().contains("trigger-round root"),
            "the refusal must name the class it saw: {err}"
        );
    }

    /// 3–4P is refused: `best rival by rank` is not antisymmetric there, so the
    /// margin is not zero-sum and minimax is unsound. Refusing beats returning a
    /// number nobody can interpret.
    #[test]
    fn a_three_player_root_is_refused_because_minimax_is_unsound_there() {
        let rules = rules();
        let pending = new_game(GameConfig::for_player_count(3), &rules, 0x43)
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
        let err = exact_table(&RootSource::Constructed(root), &rules, ExactBounds::default())
            .expect_err("3P must refuse");
        assert!(matches!(err, ExactError::NotTwoPlayer { players: 3 }), "got {err:?}");
    }

    /// The node budget refuses the WHOLE table and names both the bound and the
    /// COUNTED total — the lower bound that tells a later reader whether a
    /// bigger budget would help.
    #[test]
    fn the_node_budget_refuses_the_whole_table_and_carries_its_count() {
        let rules = rules();
        let (root, _) = handoff_root_at(&rules, 0x81, [1, 5, 2, 3], 20, 18, TurnPhase::AreaAction);
        let err = exact_table(
            &RootSource::Constructed(root),
            &rules,
            ExactBounds { max_nodes: 50, ..ExactBounds::default() },
        )
        .expect_err("50 nodes cannot solve this root");
        let ExactError::NodeBudget { bound, counted, .. } = err else {
            panic!("expected NodeBudget, got {err:?}")
        };
        assert_eq!(bound, 50);
        assert!(counted > bound, "the counted total must be a real lower bound: {counted}");
        // No partial table escaped: the error IS the whole return value.
    }

    // =======================================================================
    // Pins — determinism, antisymmetry, and joinability
    // =======================================================================

    /// The solve draws no randomness: the same root yields a byte-identical
    /// table, counters included, and the entry point takes no seed at all.
    #[test]
    fn the_exact_table_is_deterministic_and_takes_no_seed() {
        let rules = rules();
        let (root, _) = handoff_root_at(&rules, 0x81, [1, 5, 2, 3], 20, 18, TurnPhase::AreaAction);
        let source = RootSource::Constructed(root);
        let first = exact_table(&source, &rules, ExactBounds::default()).expect("solves");
        let second = exact_table(&source, &rules, ExactBounds::default()).expect("solves");
        assert_eq!(first, second, "the exact table is not reproducible");
        assert_eq!(first.stats, second.stats, "the counters moved between identical runs");
    }

    /// At 2P the margin is ANTISYMMETRIC — the premise alpha-beta rests on.
    ///
    /// Asserted over real terminal states rather than argued: if this ever fails
    /// the game is not zero-sum in the margin and every value in this module is
    /// meaningless, which is why it is a pin and not a comment.
    #[test]
    fn the_two_player_margin_is_antisymmetric_at_every_terminal() {
        let rules = rules();
        let (root, mover, _) = near_terminal_building_root(&rules, 0x54, [1, 5, 2, 3], 20, 18);
        let rival = root.players.iter().map(|p| p.id).find(|id| *id != mover).expect("a rival");
        let mut terminals = 0;
        for mv in get_legal_moves(&root, &rules) {
            let mut state = apply_move_unchecked(&root, &mv, &rules).expect("applies");
            while !is_game_over(&state) {
                let next = get_legal_moves(&state, &rules);
                let Some(first) = next.first() else { break };
                state = apply_move_unchecked(&state, first, &rules).expect("applies");
            }
            if !is_game_over(&state) {
                continue;
            }
            terminals += 1;
            let mine = terminal_value_for(&state, &rules, mover);
            let theirs = terminal_value_for(&state, &rules, rival);
            assert_eq!(mine.margin, -theirs.margin, "the 2P margin is not antisymmetric");
            assert!(!(mine.won && theirs.won), "both seats cannot win the same game");
        }
        assert!(terminals >= 2, "anti-vacuity: only {terminals} terminal(s) reached");
    }

    /// The exact table's candidates are the frozen adjudicator's candidates,
    /// row for row.
    ///
    /// This is what lets a results doc put the two tables side by side and join
    /// them BY VALUE rather than by trust. Both take `get_legal_moves` at the
    /// root, so the property holds by construction — and this cell is what keeps
    /// it holding if either ever changes its source.
    #[test]
    fn the_exact_tables_candidates_are_the_frozen_adjudicators_candidates() {
        let rules = rules();
        let (root, _, _) = near_terminal_building_root(&rules, 0x55, [1, 5, 2, 3], 20, 18);
        let source = RootSource::Constructed(root);
        let seats = [
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
        ];
        let robustness =
            adjudicate_root(&source, &seats, &rules, WorldSeed(0x0E00_2026_0820_9990), 1, DrivenOpts::default())
                .expect("the robustness read runs on this fixture");
        let exact = exact_table(&source, &rules, ExactBounds::default()).expect("solves");
        let a: Vec<&Move> = robustness.rows.iter().map(|r| &r.candidate).collect();
        let b: Vec<&Move> = exact.rows.iter().map(|r| &r.candidate).collect();
        assert_eq!(a, b, "the two instruments disagree about the root's candidates");
    }

    /// Where the robustness read is UNANIMOUS, the exact optimum agrees with it.
    ///
    /// **A DIRECTIONAL pin, deliberately not an equality claim.** The two
    /// instruments answer different questions — "what do these policies reach"
    /// versus "what is this worth under best play" — and asserting they produce
    /// the same set would assert something neither promises. What must hold is
    /// that they do not point in opposite directions where the position is
    /// clear-cut.
    ///
    /// Run on a CONSTRUCTED fixture rather than family 1's PASS root: solving
    /// that root is a results-phase read of tens of minutes (spec §6.6), which
    /// is not a per-edit test.
    #[test]
    fn the_exact_optimum_agrees_with_a_unanimous_robustness_read() {
        let rules = rules();
        let (root, _, _) = near_terminal_building_root(&rules, 0x56, [1, 9, 2, 3], 20, 18);
        let source = RootSource::Constructed(root);
        let seats = [
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
            SeatSpec::Reply(ReplyPolicy::FamilyWeighted),
        ];
        let robustness =
            adjudicate_root(&source, &seats, &rules, WorldSeed(0x0E00_2026_0820_9991), 3, DrivenOpts::default())
                .expect("the robustness read runs");
        let best_playout = robustness
            .rows
            .iter()
            .max_by_key(|r| r.playouts.iter().map(|p| p.margin).sum::<i64>())
            .expect("a non-empty table");
        let exact = exact_table(&source, &rules, ExactBounds::default()).expect("solves");
        let argmax = exact.argmax_set();
        assert_eq!(argmax.len(), 1, "fixture drift: the 9-VP space should be a unique optimum");
        assert_eq!(
            argmax[0].candidate, best_playout.candidate,
            "the exact optimum and the unanimous robustness read point at different moves"
        );
    }

    /// The memoization is SOUND: the same values come back with the
    /// transposition table disabled, AND both agree with the independent naive
    /// minimax.
    ///
    /// **The `memo_hits > 0` assertion is the point.** After the §5.4 ruling
    /// narrowed storage to TURN-ENTRY states only, the Building-phase fixtures
    /// stopped producing a single table read — this cell was measured at 0 hits
    /// and would have "proved" the soundness of a table it never consulted. The
    /// fixture moved to one that actually transposes and the counter is asserted
    /// so the cell can never go vacuous again silently.
    #[test]
    fn the_transposition_table_is_exercised_and_changes_no_value() {
        let rules = rules();
        let (root, mover) =
            handoff_root_at(&rules, 0x81, [1, 5, 2, 3], 20, 18, TurnPhase::AreaAction);
        let source = RootSource::Constructed(root.clone());
        let with = exact_table(&source, &rules, ExactBounds::default()).expect("solves");
        let without = exact_table(
            &source,
            &rules,
            ExactBounds { max_memo_entries: 0, ..ExactBounds::default() },
        )
        .expect("solves");

        assert!(
            with.stats.memo_hits > 0,
            "ANTI-VACUITY: the transposition table was never READ ({} entries, {} hits), so this              cell proves nothing about it",
            with.stats.memo_entries,
            with.stats.memo_hits
        );
        assert_eq!(without.stats.memo_entries, 0, "max_memo_entries = 0 must disable storage");

        let a: Vec<_> = with.rows.iter().map(|r| (&r.candidate, r.value)).collect();
        let b: Vec<_> = without.rows.iter().map(|r| (&r.candidate, r.value)).collect();
        assert_eq!(a, b, "memoization changed a value");

        for row in &with.rows {
            let after =
                apply_move_unchecked(&root, &row.candidate, &rules).expect("a listed move applies");
            assert_eq!(
                row.value,
                reference_minimax(&after, &rules, mover),
                "the memoized solve disagreed with naive minimax on {:?}",
                row.candidate
            );
        }
    }
}
