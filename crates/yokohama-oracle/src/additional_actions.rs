//! §3.1 Additional-action phases — lane E implements per spec §3.1.
//!
//! Skeleton authored by lane A so sibling lanes compile against these EXACT
//! signatures; bodies are `todo!()` until lane E lands.

use yokohama_core::{AchievementType, AreaId, GameState, Move, OrderCardId, PlayerId, Rules};

/// §3.1: `EndAdditionalPhase` (always) plus any number of `UseMercenary` /
/// `FulfillOrder` / `FulfillAchievement` / `UseUniversity` / `UseSpinningMill`,
/// each gated per §3.1. Shared by both additional-action windows.
pub(crate) fn additional_action_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    let _ = (state, rules);
    todo!("lane E: §3.1 additional actions")
}

/// True iff the current claim-time requirement for achievement `a` is met by
/// `player` (§3.1 categories A/B/C; Tokens5 counts all owned tokens, §7.1-A2).
#[allow(dead_code)] // consumed by lane E's additional-action impl + fixtures.
pub(crate) fn achievement_met(state: &GameState, player: PlayerId, a: AchievementType) -> bool {
    let _ = (state, player, a);
    todo!("lane E: §3.1 achievement requirement")
}

/// True iff order `id` in `player`'s hand has all trade-good requirements met
/// (§3.1; requirements via `rules.orders`).
#[allow(dead_code)] // consumed by lane E's additional-action impl + fixtures.
pub(crate) fn order_fulfillable(
    state: &GameState,
    rules: &Rules,
    player: PlayerId,
    id: OrderCardId,
) -> bool {
    let _ = (state, rules, player, id);
    todo!("lane E: §3.1 order fulfillable")
}

/// Areas holding ≥1 own assistant and NOT the player's own President — the
/// legal `UseMercenary` target areas (§3.1-2).
#[allow(dead_code)] // consumed by lane E's additional-action impl + fixtures.
pub(crate) fn mercenary_targets(state: &GameState, player: PlayerId) -> Vec<AreaId> {
    let _ = (state, player);
    todo!("lane E: §3.1 mercenary targets")
}
