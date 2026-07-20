//! §3.2 placement enumeration — lane B implements per spec §3.2.
//!
//! Skeleton authored by lane A so sibling lanes compile against these EXACT
//! signatures; bodies are `todo!()` until lane B lands. The integration wave
//! removes the `#[allow(dead_code)]` markers as each helper gains a caller.

use yokohama_core::{AreaId, GameState, Move, PlayerId, Rules};

/// §3.2: Option A subsets (0..=N distinct placeable areas) + Option B
/// (2-into-one-area, optional Telegram adjacent bonus), affordability-gated.
pub(crate) fn placement_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    let _ = (state, rules);
    todo!("lane B: §3.2 placement enumeration")
}

/// 1 Yen × assistants placed × opponent presidents, summed per target area
/// (§3.2 fee rule; Option B's 2-into-one costs 2, decision log 2026-07-17 C1).
#[allow(dead_code)] // consumed by lane B's placement impl + fixtures.
pub(crate) fn placement_fee(
    state: &GameState,
    player: PlayerId,
    per_area_counts: &[(AreaId, u8)],
) -> u32 {
    let _ = (state, player, per_area_counts);
    todo!("lane B: §3.2 placement fee")
}

/// Physical-contiguity neighbors minus canal-crossing pairs minus the Canal
/// itself (Telegram adjacent-bonus scope, §3.2; decision log 2026-07-18).
#[allow(dead_code)] // consumed by lane B's Option-B/Telegram impl + fixtures.
pub(crate) fn telegram_adjacent(state: &GameState, area: AreaId) -> Vec<AreaId> {
    let _ = (state, area);
    todo!("lane B: §3.2 Telegram adjacency")
}
