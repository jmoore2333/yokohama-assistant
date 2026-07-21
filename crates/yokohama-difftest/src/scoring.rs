//! Terminal-state scoring differential: engine `final_result` vs. oracle
//! `final_standings` (Stage 2).
//!
//! ENGINE DIVISION OF LABOR (load-bearing for interpreting divergences —
//! re-derived against the engine source 2026-07-21; anchored to function/field
//! names, not line numbers, because the prior line citations had rotted):
//!
//! Folded into STATE during play (`apply_move_unchecked` path), so already
//! inside `PlayerState::victory_points` (`types/player.rs`, `u32`) by the time
//! a state is terminal — the five `victory_points +=` sites in
//! `game/apply_move.rs` plus the track/achievement/card sites:
//! - achievement claims (`game/achievements.rs` `claim_achievement`);
//! - church track VP + Winery bonus (`game/actions/tracks.rs`
//!   `place_church_assistant`), customs track VP + Brickyard bonus
//!   (`place_customs_assistant`);
//! - Gaslight bonus (`game/apply_move.rs` `apply_build_shophouse`);
//!   Trading-House site VP + Electrical Light bonus (`apply_build_trading_house`);
//!   building-site space-reward VP (`apply_space_reward`); order-completion
//!   reward VP + Ball bonus (`apply_fulfill_order`);
//! - Port-at-5-power +3 VP (`game/actions/cards.rs` `execute_port_action`),
//!   Patent System +2 / Exposition +10 (`apply_lab_acquisition_bonuses`).
//!   (No five-power-token VP source exists — `FivePowerEffect::VictoryPoints`
//!   was removed at the 2026-07-18 box check; a stale citation to one was
//!   dropped in this pass.)
//!
//! End-game BOOKKEEPING is likewise folded: `check_end_game_triggers` is
//! called from `game/apply_move.rs` `apply_move_unchecked` (impl in
//! `game/end_game.rs`); the `final_round`/`TurnEnd` transitions live in
//! `apply_move.rs` `advance_phase`.
//!
//! Computed at QUERY TIME inside `final_result` (`game/scoring.rs`), never
//! folded into state — per-player components assembled in `breakdowns`:
//! - `base_vp` = `victory_points` verbatim (`breakdowns`);
//! - church majority 6/3 (`calculate_church_bonus`), customs majority 8/4
//!   (`calculate_customs_bonus`);
//! - technology industry-value 10/5, zero-iv excluded (`calculate_technology_bonus`);
//! - country greedy set bonus 12/8/4/2 (`calculate_country_bonus`);
//! - remainder = unused mercenaries + imported goods + money/2 + goods/3
//!   (`calculate_remainder_bonus`);
//! - the ranking itself: `final_result` sorts by `(Reverse(total), clockwise
//!   distance from start_player)`, ranks 1..=n never shared,
//!   `winner = standings[0]`.
//!
//! The comparable projection of the engine side is therefore
//! `final_result(..).standings` in order, as `(player, breakdown.total)` —
//! and the oracle side is `final_standings(..)` as `(player, total_vp)`,
//! both ranked best-first with the official tie-break fully applied (a
//! strict total order on both sides by contract).

use std::collections::BTreeMap;

use yokohama_core::{final_result, is_game_over, GameState, PlayerId, Rules};

/// One side's ranked outcome, best-first: `(player, total VP)`.
pub type RankedTotals = Vec<(PlayerId, u32)>;

/// The two sides' answers for one terminal state, plus what disagrees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoringComparison {
    /// Engine `final_result` standings in rank order.
    pub engine: RankedTotals,
    /// Oracle `final_standings` in its emitted (ranked) order.
    pub oracle: RankedTotals,
    /// Per-player total VP differs (compared as player->total maps, so a
    /// missing/extra player on either side also trips this).
    pub totals_diverge: bool,
    /// The ranked player sequences differ (tie-break or ordering defect
    /// even when every total matches).
    pub order_diverges: bool,
    /// Sanity: the oracle's own emitted order is not non-increasing in its
    /// own totals — an internally inconsistent oracle answer (its ranking
    /// contradicts its arithmetic) is a divergence even if it accidentally
    /// matches the engine's order.
    pub oracle_ranking_inconsistent: bool,
}

impl ScoringComparison {
    /// True when anything at all disagrees.
    pub fn is_divergent(&self) -> bool {
        self.totals_diverge || self.order_diverges || self.oracle_ranking_inconsistent
    }
}

/// True when `side` is ranked consistently with its own totals: totals are
/// non-increasing best-first. (Equal adjacent totals are consistent — the
/// tie-break is positional, not numeric; ORDER agreement between sides is
/// `order_diverges`' job.)
pub fn ranking_consistent(side: &RankedTotals) -> bool {
    side.windows(2).all(|w| w[0].1 >= w[1].1)
}

/// Pure comparison core over two pre-projected ranked lists (testable
/// without a terminal state or the oracle crate's internals).
pub fn compare_rankings(engine: RankedTotals, oracle: RankedTotals) -> ScoringComparison {
    fn totals_map(side: &RankedTotals) -> BTreeMap<PlayerId, u32> {
        side.iter().copied().collect()
    }
    let totals_diverge = totals_map(&engine) != totals_map(&oracle)
        // A duplicated player collapses in the map; compare lengths too so a
        // side listing a player twice can't masquerade as agreement.
        || engine.len() != oracle.len();
    let order_diverges =
        !engine.iter().map(|(p, _)| p).eq(oracle.iter().map(|(p, _)| p));
    let oracle_ranking_inconsistent = !ranking_consistent(&oracle);
    ScoringComparison { engine, oracle, totals_diverge, order_diverges, oracle_ranking_inconsistent }
}

/// The ENGINE's final-result API projected into comparable form:
/// `final_result(..).standings` is already ranked best-first
/// (`standings[0]` is rank 1 — `game/scoring.rs:31,63-66`), so the
/// projection keeps its order and takes `(player, breakdown.total)`.
pub fn engine_ranked_totals(state: &GameState, rules: &Rules) -> RankedTotals {
    final_result(state, rules)
        .standings
        .iter()
        .map(|s| (s.player, s.breakdown.total))
        .collect()
}

/// The ORACLE's answer projected the same way: `final_standings` is ranked
/// best-first with the official tie-break by contract (pinned Stage-2 API).
pub fn oracle_ranked_totals(state: &GameState, rules: &Rules) -> RankedTotals {
    yokohama_oracle::scoring::final_standings(state, rules)
        .iter()
        .map(|s| (s.player, s.total_vp))
        .collect()
}

/// Differentially compare engine vs. oracle FINAL scoring on `state`.
/// Returns `None` on a non-terminal state (final scoring is only defined at
/// game over); `Some` carries both sides' ranked totals and the divergence
/// flags — a `Some` with `is_divergent() == false` is the clean outcome.
pub fn compare_final(state: &GameState, rules: &Rules) -> Option<ScoringComparison> {
    if !is_game_over(state) {
        return None;
    }
    Some(compare_rankings(engine_ranked_totals(state, rules), oracle_ranked_totals(state, rules)))
}

/// Format one side's ranked totals as artifact lines, best-first:
/// `"P<id>: <total>"` (the schema-v1 `engine_only`/`oracle_only` convention
/// for scoring artifacts — full sides, not set differences, so the artifact
/// shows both complete answers).
pub fn format_side(side: &RankedTotals) -> Vec<String> {
    side.iter().map(|(p, total)| format!("P{}: {total}", p.0)).collect()
}

/// The `duplicates` line a scoring artifact carries when the oracle's
/// emitted order contradicts its own totals.
pub const ORACLE_RANKING_INCONSISTENT_LINE: &str =
    "oracle: emitted ranking is not descending in its own totals";

/// One comparison's stored-field projection — the exact
/// `(engine_only, oracle_only, duplicates)` line triple a `FinalScoring`
/// artifact carries. Shared by the Stage-2 writer
/// (`runner::write_scoring_artifact`) and the repro re-run
/// (`repro::repro_artifact`) so a fresh artifact reproduces byte-identically
/// by construction.
pub fn scoring_artifact_fields(
    cmp: &ScoringComparison,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut duplicates = Vec::new();
    if cmp.oracle_ranking_inconsistent {
        duplicates.push(ORACLE_RANKING_INCONSISTENT_LINE.to_string());
    }
    (format_side(&cmp.engine), format_side(&cmp.oracle), duplicates)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(rows: &[(u8, u32)]) -> RankedTotals {
        rows.iter().map(|&(p, t)| (PlayerId::new(p), t)).collect()
    }

    #[test]
    fn identical_sides_are_clean() {
        let cmp = compare_rankings(side(&[(1, 30), (0, 20)]), side(&[(1, 30), (0, 20)]));
        assert!(!cmp.is_divergent(), "{cmp:?}");
        assert!(!cmp.totals_diverge);
        assert!(!cmp.order_diverges);
        assert!(!cmp.oracle_ranking_inconsistent);
    }

    #[test]
    fn total_mismatch_trips_totals_only() {
        let cmp = compare_rankings(side(&[(1, 30), (0, 20)]), side(&[(1, 31), (0, 20)]));
        assert!(cmp.totals_diverge);
        assert!(!cmp.order_diverges, "same player sequence, only a total differs");
        assert!(cmp.is_divergent());
    }

    #[test]
    fn tie_break_disagreement_trips_order_only() {
        // Equal totals, opposite tie-break: totals agree per player, the
        // ranked order does not — exactly the clockwise-tie-break defect
        // class this flag exists for.
        let cmp = compare_rankings(side(&[(1, 30), (2, 30)]), side(&[(2, 30), (1, 30)]));
        assert!(!cmp.totals_diverge);
        assert!(cmp.order_diverges);
        assert!(cmp.is_divergent());
    }

    #[test]
    fn missing_player_on_one_side_diverges() {
        let cmp = compare_rankings(side(&[(0, 20), (1, 10)]), side(&[(0, 20)]));
        assert!(cmp.totals_diverge);
        assert!(cmp.order_diverges);
    }

    #[test]
    fn duplicated_player_cannot_masquerade_as_agreement() {
        // Oracle lists P0 twice; the map collapses it, but the length guard
        // still reports a totals divergence.
        let cmp = compare_rankings(side(&[(0, 20), (1, 10)]), side(&[(0, 20), (0, 20), (1, 10)]));
        assert!(cmp.totals_diverge);
    }

    #[test]
    fn oracle_order_contradicting_its_own_totals_is_flagged() {
        // Same rows both sides, but the oracle emitted them worst-first:
        // internally inconsistent even though the multisets match.
        let cmp = compare_rankings(side(&[(1, 30), (0, 20)]), side(&[(0, 20), (1, 30)]));
        assert!(cmp.oracle_ranking_inconsistent);
        assert!(cmp.is_divergent());
    }

    #[test]
    fn ranking_consistency_allows_equal_adjacent_totals() {
        assert!(ranking_consistent(&side(&[(0, 30), (1, 30), (2, 10)])));
        assert!(ranking_consistent(&side(&[])));
        assert!(ranking_consistent(&side(&[(0, 5)])));
        assert!(!ranking_consistent(&side(&[(0, 10), (1, 20)])));
    }

    #[test]
    fn format_side_is_ranked_player_total_lines() {
        assert_eq!(
            format_side(&side(&[(2, 41), (0, 41), (1, 7)])),
            vec!["P2: 41", "P0: 41", "P1: 7"],
        );
    }
}
