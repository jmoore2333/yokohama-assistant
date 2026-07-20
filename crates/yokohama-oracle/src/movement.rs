//! §3.3 + §4 President movement — lane B implements per spec §3.3/§4.
//!
//! Skeleton authored by lane A so sibling lanes compile against these EXACT
//! signatures; bodies are `todo!()` until lane B lands.

use yokohama_core::{GameState, Move, Rules};

use crate::RouteLabel;

/// §1.4 Movement dispatch: the exact part (`PlacePresident` destinations when
/// the President is in hand, plus the retreat family always) and the on-board
/// route-label part (empty when the President is in hand).
pub(crate) fn movement_moves(state: &GameState, rules: &Rules) -> (Vec<Move>, Vec<RouteLabel>) {
    let _ = (state, rules);
    todo!("lane B: §3.3/§4 movement enumeration")
}

/// §3.3-2 retreat family: `ReturnPresidentToHand` for every per-area recovery
/// count map (each key `1..=in_area`), the empty map included.
#[allow(dead_code)] // consumed by lane B's movement impl + fixtures.
pub(crate) fn retreat_family(state: &GameState) -> Vec<Move> {
    let _ = state;
    todo!("lane B: §3.3-2 retreat family")
}

/// §3.3-1 `PlacePresident` destinations (Possible Action Areas; Postal
/// System/Stagecoach fee-aware), affordability-gated.
#[allow(dead_code)] // consumed by lane B's movement impl + fixtures.
pub(crate) fn place_president_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    let _ = (state, rules);
    todo!("lane B: §3.3-1 PlacePresident destinations")
}

/// §4.1–4.2: simple-path DFS with a monotone cost cutoff, plus Tram/Station
/// teleport labels, deduplicated by the L2 `(destination, fees)` identity.
#[allow(dead_code)] // consumed by lane B's movement impl + fixtures.
pub(crate) fn route_labels(state: &GameState, rules: &Rules) -> Vec<RouteLabel> {
    let _ = (state, rules);
    todo!("lane B: §4 route labels")
}
