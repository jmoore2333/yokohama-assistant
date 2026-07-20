//! §3.4 Port + Laboratory area actions — lane C implements per spec §3.4.
//!
//! Skeleton authored by lane A so sibling lanes compile against these EXACT
//! signatures; bodies are `todo!()` until lane C lands. `Skip` is NOT added
//! here — the lib.rs area-action facade appends the single canonical decline.

use yokohama_core::{AreaType, GameState, Move, Rules};

/// §3.4 Port A/B: free 1-card takes and paid 2-card takes over eligible-by-power
/// slots, hand-limit gated. `area_type` selects Port A vs Port B state.
pub(crate) fn port_moves(
    state: &GameState,
    rules: &Rules,
    area_type: AreaType,
    power: u8,
) -> Vec<Move> {
    let _ = (state, rules, area_type, power);
    todo!("lane C: §3.4 Port")
}

/// §3.4 Laboratory A/B: tech acquisitions with the one-per-title gate, industry
/// cover enumeration, and the 5th-tech penalty variants. `area_type` selects
/// Lab A vs Lab B state.
pub(crate) fn laboratory_moves(
    state: &GameState,
    rules: &Rules,
    area_type: AreaType,
    power: u8,
) -> Vec<Move> {
    let _ = (state, rules, area_type, power);
    todo!("lane C: §3.4 Laboratory")
}
