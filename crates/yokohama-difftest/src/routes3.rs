//! Stage 3 (Phase 2.9): three-way pruning-soundness comparison over the
//! president-movement label family.
//!
//! Three independent enumerations of the SAME `(destination, fees)` label
//! surface, compared as sorted key sets at every qualifying decision point
//! (Movement phase, empty pending-choice queue, President on the board):
//!
//! - **ENGINE** — `get_legal_moves` `MovePresident` moves projected through
//!   the engine's own pricing ([`crate::compare::engine_route_label`]); the
//!   walk behind them (`enumerate_president_routes`) prunes on accumulated
//!   cost > money.
//! - **PRUNED ORACLE** — `yokohama_oracle::oracle_moves(..).movement`, the
//!   independently-authored DFS carrying the same monotone cutoff (oracle
//!   spec §4.4).
//! - **PRUNELESS REFERENCE** —
//!   `yokohama_oracle::route_reference::route_labels_pruneless`: EVERY simple
//!   path over the own-assistant step rule with NO affordability cutoff
//!   anywhere in the walk; each completed path is priced and `cost <= money`
//!   is filtered only at the END.
//!
//! Engine and pruned oracle share the monotonicity ASSUMPTION (no route fee
//! is negative and nothing on a route earns money, so an unaffordable prefix
//! can never extend to an affordable route) but no code; the reference does
//! not make the assumption at all. Teleport labels (Tram / Station incl. the
//! RULED §7.1-A1 Reading-2 assistant gate) are deliberately SHARED between the two
//! oracle surfaces, so a pruned-vs-pruneless diff isolates the WALKS —
//! exactly the pruning-soundness claim under test.

use std::path::{Path, PathBuf};

use yokohama_core::{
    get_legal_moves, GameState, Move, PresidentLocation, Rules, TurnPhase,
};
use yokohama_oracle::oracle_moves;
use yokohama_oracle::route_reference::route_labels_pruneless;

use crate::artifact::{write_artifact, DivergenceArtifact, ARTIFACT_FORMAT_VERSION};
use crate::compare::{engine_route_label, route_label_key};
use crate::drive::{drive_game, DriveStats, Policy, StepOutcome};

/// One of the three enumeration surfaces under comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Engine,
    PrunedOracle,
    PrunelessReference,
}

impl Side {
    /// Stable name used in artifact lines and printed reports.
    pub fn as_str(&self) -> &'static str {
        match self {
            Side::Engine => "engine",
            Side::PrunedOracle => "pruned-oracle",
            Side::PrunelessReference => "pruneless-reference",
        }
    }
}

/// Symmetric difference of one ordered pair of sides (only recorded when
/// non-empty — an all-equal comparison carries no `PairDiff` at all).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairDiff {
    pub left: Side,
    pub right: Side,
    /// Keys `left` has that `right` lacks. Sorted.
    pub left_only: Vec<String>,
    /// Keys `right` has that `left` lacks. Sorted.
    pub right_only: Vec<String>,
}

/// One qualifying state's three-way comparison: the three sorted,
/// deduplicated key sets plus every divergent pair (empty = all three equal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteComparison {
    pub engine: Vec<String>,
    pub pruned: Vec<String>,
    pub pruneless: Vec<String>,
    /// The divergent pairs among the three (at most 3 entries); which pair
    /// diverged is the diagnosis — e.g. pruned == pruneless != engine points
    /// at the engine walk, engine == pruned != pruneless points at the shared
    /// pruning assumption.
    pub diverged: Vec<PairDiff>,
}

impl RouteComparison {
    /// True when any pair of the three sets disagrees.
    pub fn is_divergent(&self) -> bool {
        !self.diverged.is_empty()
    }

    /// Number of distinct labels (engine side; equal across sides when
    /// clean) — the worst-case battery's measurement.
    pub fn label_count(&self) -> usize {
        self.engine.len()
    }
}

/// Sort + dedup a key list into set form.
fn sorted_key_set(mut keys: Vec<String>) -> Vec<String> {
    keys.sort();
    keys.dedup();
    keys
}

/// Symmetric difference over two SORTED key sets, `None` when equal.
fn pair_diff(left: Side, l: &[String], right: Side, r: &[String]) -> Option<PairDiff> {
    let left_only: Vec<String> =
        l.iter().filter(|k| r.binary_search(k).is_err()).cloned().collect();
    let right_only: Vec<String> =
        r.iter().filter(|k| l.binary_search(k).is_err()).cloned().collect();
    if left_only.is_empty() && right_only.is_empty() {
        None
    } else {
        Some(PairDiff { left, right, left_only, right_only })
    }
}

/// Does `state` qualify for the three-way check? On-board-president,
/// empty-queue Movement decision points only — the exact family whose walks
/// the three surfaces all enumerate.
fn qualifies(state: &GameState) -> bool {
    !state.is_game_over()
        && state.turn_phase == TurnPhase::Movement
        && state.pending_choices.is_empty()
        && matches!(state.current_player().president_location, PresidentLocation::OnBoard(_))
}

/// Three-way comparison at one state, or `None` for non-qualifying states.
/// Computes the engine's legal moves itself; use
/// [`three_way_route_check_with_moves`] when the caller already has them
/// (e.g. the drive observer).
pub fn three_way_route_check(state: &GameState, rules: &Rules) -> Option<RouteComparison> {
    if !qualifies(state) {
        return None;
    }
    let engine_moves = get_legal_moves(state, rules);
    three_way_route_check_with_moves(state, rules, &engine_moves)
}

/// [`three_way_route_check`] over pre-computed engine legal moves.
///
/// The engine's `MovePresident` moves are projected to label keys via the
/// engine's OWN pricing ([`engine_route_label`] — the engine side of the
/// comparison is the engine's claim); non-`MovePresident` moves (the retreat
/// family) are outside the label family and ignored. The pruned oracle
/// contributes `oracle_moves(..).movement`; the pruneless reference
/// contributes `route_labels_pruneless`. All three are compared as sorted,
/// deduplicated key sets (duplicate-emission defects are Stage 1's
/// `compare_moves` territory, checked by the Stage-1 sweep on the same
/// states).
pub fn three_way_route_check_with_moves(
    state: &GameState,
    rules: &Rules,
    engine_moves: &[Move],
) -> Option<RouteComparison> {
    if !qualifies(state) {
        return None;
    }

    let engine = sorted_key_set(
        engine_moves
            .iter()
            .filter_map(|m| match m {
                Move::MovePresident { path, destination } => {
                    Some(route_label_key(&engine_route_label(state, rules, path, *destination)))
                }
                _ => None,
            })
            .collect(),
    );
    let pruned = sorted_key_set(
        oracle_moves(state, rules).movement.iter().map(route_label_key).collect(),
    );
    let pruneless = sorted_key_set(
        route_labels_pruneless(state, rules).iter().map(route_label_key).collect(),
    );

    let diverged: Vec<PairDiff> = [
        pair_diff(Side::Engine, &engine, Side::PrunedOracle, &pruned),
        pair_diff(Side::Engine, &engine, Side::PrunelessReference, &pruneless),
        pair_diff(Side::PrunedOracle, &pruned, Side::PrunelessReference, &pruneless),
    ]
    .into_iter()
    .flatten()
    .collect();

    Some(RouteComparison { engine, pruned, pruneless, diverged })
}

/// Format the divergent pairs into artifact lines: left-only findings go in
/// the artifact's `engine_only` field, right-only in `oracle_only` (the
/// schema-v1 envelope reused with `phase: "Routes3"`, following the Stage-2
/// FinalScoring precedent — each line names its pair explicitly, so the
/// two-field envelope loses nothing).
pub fn format_pair_diffs(diffs: &[PairDiff]) -> (Vec<String>, Vec<String>) {
    let mut lefts = Vec::new();
    let mut rights = Vec::new();
    for d in diffs {
        let pair = format!("{} vs {}", d.left.as_str(), d.right.as_str());
        for k in &d.left_only {
            lefts.push(format!("[{pair}] {}-only: {k}", d.left.as_str()));
        }
        for k in &d.right_only {
            rights.push(format!("[{pair}] {}-only: {k}", d.right.as_str()));
        }
    }
    (lefts, rights)
}

/// Outcome of one three-way-checked game.
#[derive(Debug, Clone)]
pub struct Routes3Outcome {
    pub stats: DriveStats,
    /// Decision points that qualified (Movement, empty queue, on-board
    /// president) and were therefore actually compared.
    pub qualifying_steps: u64,
    /// Maximum label count observed across qualifying states.
    pub max_labels: usize,
    /// Path of the artifact written for the first divergence, if any.
    pub divergence: Option<PathBuf>,
}

/// Drive the `(seed, players, policy)` game, running the three-way check at
/// every qualifying decision point. The first divergence writes a
/// `phase: "Routes3"` artifact (schema-v1 envelope, lines per
/// [`format_pair_diffs`]) and stops the game — the state snapshot is the
/// whole repro. `Err` carries hard driver failures only, never a divergence.
pub fn run_routes3_game(
    seed: u64,
    players: u8,
    policy: Policy,
    rules: &Rules,
    artifact_dir: &Path,
) -> Result<Routes3Outcome, String> {
    let mut divergence: Option<PathBuf> = None;
    let mut qualifying_steps: u64 = 0;
    let mut max_labels: usize = 0;

    let stats = drive_game(seed, players, policy, rules, None, |state, engine_moves, step| {
        let Some(cmp) = three_way_route_check_with_moves(state, rules, engine_moves) else {
            return Ok(StepOutcome::Continue);
        };
        qualifying_steps += 1;
        max_labels = max_labels.max(cmp.label_count());
        if !cmp.is_divergent() {
            return Ok(StepOutcome::Continue);
        }
        let (engine_only, oracle_only) = format_pair_diffs(&cmp.diverged);
        let (rules_hash, semantics_rev) = crate::artifact::current_provenance(rules);
        let artifact = DivergenceArtifact {
            format_version: ARTIFACT_FORMAT_VERSION,
            seed,
            players,
            policy: policy.as_str().to_string(),
            step_index: step,
            turn_number: state.turn_number,
            phase: "Routes3".to_string(),
            state_json: serde_json::to_value(state)
                .map_err(|e| format!("state serialization for routes3 artifact failed: {e}"))?,
            engine_only,
            oracle_only,
            duplicates: Vec::new(),
            rules_hash,
            semantics_rev,
        };
        let path = write_artifact(&artifact, artifact_dir)?;
        divergence = Some(path);
        Ok(StepOutcome::Stop)
    })?;

    Ok(Routes3Outcome { stats, qualifying_steps, max_labels, divergence })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sorted_key_set_sorts_and_dedups() {
        assert_eq!(sorted_key_set(keys(&["b", "a", "b"])), keys(&["a", "b"]));
    }

    #[test]
    fn pair_diff_is_none_on_equal_sets_and_reports_both_directions() {
        let l = keys(&["a", "b", "c"]);
        let r = keys(&["b", "c", "d"]);
        assert_eq!(pair_diff(Side::Engine, &l, Side::PrunedOracle, &l), None);
        let d = pair_diff(Side::Engine, &l, Side::PrunedOracle, &r).expect("diverges");
        assert_eq!(d.left_only, keys(&["a"]));
        assert_eq!(d.right_only, keys(&["d"]));
        assert_eq!(d.left, Side::Engine);
        assert_eq!(d.right, Side::PrunedOracle);
    }

    #[test]
    fn format_pair_diffs_names_the_pair_on_every_line() {
        let diffs = vec![PairDiff {
            left: Side::PrunedOracle,
            right: Side::PrunelessReference,
            left_only: keys(&["route:X"]),
            right_only: keys(&["route:Y"]),
        }];
        let (lefts, rights) = format_pair_diffs(&diffs);
        assert_eq!(
            lefts,
            keys(&["[pruned-oracle vs pruneless-reference] pruned-oracle-only: route:X"])
        );
        assert_eq!(
            rights,
            keys(&["[pruned-oracle vs pruneless-reference] pruneless-reference-only: route:Y"])
        );
    }

    #[test]
    fn side_names_are_stable() {
        assert_eq!(Side::Engine.as_str(), "engine");
        assert_eq!(Side::PrunedOracle.as_str(), "pruned-oracle");
        assert_eq!(Side::PrunelessReference.as_str(), "pruneless-reference");
    }
}
