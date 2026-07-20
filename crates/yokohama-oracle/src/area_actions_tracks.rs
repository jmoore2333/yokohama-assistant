//! §3.4 Church + Customs area actions — lane D implements per spec §3.4.
//!
//! Skeleton authored by lane A so sibling lanes compile against these EXACT
//! signatures; bodies are `todo!()` until lane D lands. `Skip` is NOT added
//! here — the lib.rs area-action facade appends the single canonical decline.
//! The `eligible_*` helpers are shared with `choices.rs` (§2.4/§2.5), so they
//! already have a caller and carry no `#[allow(dead_code)]`.

use yokohama_core::{AreaId, GameState, Move};

/// §3.4 Church: every goods-backed payment combo whose `total_faith` reaches an
/// eligible empty space, with an assistant available from hand or the acting
/// area. `acting` is the Church area (own-assistant source, §3.4/§3.9).
pub(crate) fn church_moves(state: &GameState, power: u8, acting: AreaId) -> Vec<Move> {
    let _ = (state, power, acting);
    todo!("lane D: §3.4 Church")
}

/// §3.4 Customs: every `Customs { n }` whose `n` exactly matches an empty,
/// unblocked printed space cost, `1 ≤ n ≤ min(power-1, 4)`, assistant available
/// from hand or the acting area.
pub(crate) fn customs_moves(state: &GameState, power: u8, acting: AreaId) -> Vec<Move> {
    let _ = (state, power, acting);
    todo!("lane D: §3.4 Customs")
}

/// Eligible empty Church spaces: value in 4..=11, empty slot, `≤ total_faith`,
/// excluding the 2-player faith-7 dummy (spec §2.4; shared with choices.rs).
pub(crate) fn eligible_church_spaces(state: &GameState, total_faith: u8) -> Vec<u8> {
    let _ = (state, total_faith);
    todo!("lane D: §2.4 eligible church spaces")
}

/// Eligible empty Customs spaces for an exact printed `cost`, excluding the
/// 2-player space-4 dummy (spec §2.5; shared with choices.rs).
pub(crate) fn eligible_customs_spaces(state: &GameState, cost: u8) -> Vec<u8> {
    let _ = (state, cost);
    todo!("lane D: §2.5 eligible customs spaces")
}
