//! §3.5 5-Power bonus + §3.6 Building — lane E implements per spec §3.5/§3.6.
//!
//! Skeleton authored by lane A so sibling lanes compile against these EXACT
//! signatures; bodies are `todo!()` until lane E lands.

use yokohama_core::{GameState, Move, Rules};

/// §3.5: `Take5PowerToken` iff the acting power is exactly 5 and the acting
/// area's token is still present, plus the always-legal `Skip5PowerBonus`.
pub(crate) fn five_power_moves(state: &GameState) -> Vec<Move> {
    let _ = state;
    todo!("lane E: §3.5 5-power bonus")
}

/// §3.6: at 4+ acting power, one `BuildShophouse { slot }` per empty
/// site slot (no own shophouse already there) and `BuildTradingHouse` when the
/// site's trading-house space is empty and a token is in hand; plus the
/// always-legal `SkipBuilding`.
pub(crate) fn building_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    let _ = (state, rules);
    todo!("lane E: §3.6 building")
}
