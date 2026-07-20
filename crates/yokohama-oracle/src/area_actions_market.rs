//! §3.4 production / Bank / Employment Agency / Chinatown — lane D implements
//! per spec §3.4.
//!
//! Skeleton authored by lane A so sibling lanes compile against these EXACT
//! signatures; bodies are `todo!()` until lane D lands. `Skip` is NOT added
//! here — the lib.rs area-action facade appends the single canonical decline
//! (production areas and the Bank never get one, `allows_area_action_skip`).

use yokohama_core::{AreaType, GameState, Move};

/// §3.4 production areas / Bank: the single imperative "Gain …" move for the
/// area type (`ProduceCopper`/`ProduceSilk`/`ProduceTea`/`ProduceSeafood`/
/// `Bank`), or `None` for a non-production, non-Bank area type. No `Skip`.
pub(crate) fn production_or_bank_move(area_type: AreaType) -> Option<Move> {
    let _ = area_type;
    todo!("lane D: §3.4 production/Bank")
}

/// §3.4 Employment Agency: every `EmploymentAgency { a, s, t }` with
/// `1 ≤ a+s+t ≤ power-1`, warehouse-available and affordable (TH take-cost
/// progression).
pub(crate) fn employment_moves(state: &GameState, power: u8) -> Vec<Move> {
    let _ = (state, power);
    todo!("lane D: §3.4 Employment Agency")
}

/// §3.4 Chinatown: one representative `Chinatown` move per achievable exchange
/// multiset (spec §9-B), `1 ≤ len ≤ power`, sequential running-balance feasible.
pub(crate) fn chinatown_moves(state: &GameState, power: u8) -> Vec<Move> {
    let _ = (state, power);
    todo!("lane D: §3.4 Chinatown")
}
