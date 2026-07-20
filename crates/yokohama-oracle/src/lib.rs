//! Independent rules oracle for the Yokohama engine (Phase 2.9, Stage 1).
//!
//! **Visible-coupling guarantee (phase plan, Structure section):** this crate
//! depends on `yokohama-core` for TYPES ONLY — `GameState`, `Rules`, `Move`,
//! the id types, and the money type (`u32`) — never for logic. No function
//! from `yokohama_core::game` may be called here; every rules computation in
//! this crate must be authored from the rules sources (`analysis-1/yokohama-kb/`,
//! the rulebook PDF, `guidingpath.md`'s decision log) by an agent that has not
//! read the engine's `legal_moves.rs` / `apply_move.rs` / `movement.rs` /
//! `scoring.rs`. Any coupling beyond types is visible in `Cargo.toml` imports
//! and is re-checked at every stage review.
//!
//! **Current status: compiling stub.** [`legal_moves`] returns empty
//! collections and awaits the Stage-1 spec (the independence-protected oracle
//! track fills it in). The public API below is PINNED — the harness in
//! `yokohama-difftest` is built against it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use yokohama_core::{AreaId, GameState, Move, PlayerId, Rules};

/// Label for one distinct president-movement option, mirroring the engine's
/// L2 route canonicalization (one representative per distinct
/// destination/cost/credit-vector — see the engine-authority design,
/// 2026-07-19 §4.3): the engine emits exactly one `Move::MovePresident` per
/// distinct label, with an arbitrary (lexicographically-least) concrete path,
/// so paths themselves are not comparable across implementations — labels
/// are. All money amounts are in Yen, using the engine's money type (`u32`,
/// the type of `PlayerState::money`).
///
/// `Ord` is derived (field order: `to`, `mover_cost`, `player_credits`,
/// `bank`) purely so labels can live in sorted sets for set comparison — no
/// semantic meaning attaches to the ordering.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RouteLabel {
    /// Destination area of the move.
    pub to: AreaId,
    /// Total Yen the mover pays for the route (president pass-through/stop
    /// fees plus canal tolls). Always equals the sum of `player_credits`
    /// values plus `bank`.
    pub mover_cost: u32,
    /// Yen credited to each opponent president-owner along the route
    /// (recipients with a zero credit are omitted).
    pub player_credits: BTreeMap<PlayerId, u32>,
    /// Yen paid to the bank (canal tolls — box-ruled 2026-07-18: tolls go to
    /// the BANK, not a player).
    pub bank: u32,
}

/// The oracle's answer for one state: every legal-move family EXCEPT
/// president movement in the engine's exact `Move` encoding (`exact`), plus
/// the president-movement family as [`RouteLabel`]s (`movement`).
///
/// President movement is split out because the engine canonicalizes routes
/// to one representative path per distinct (destination, cost,
/// credit-vector) label — exact `Move::MovePresident` values are therefore
/// not comparable across independently-authored implementations, but their
/// labels are. `exact` must never contain a `Move::MovePresident`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleMoves {
    /// Every legal move except `Move::MovePresident`, in the engine's exact
    /// `Move` encoding.
    pub exact: Vec<Move>,
    /// The president-movement family, one entry per distinct legal label.
    pub movement: Vec<RouteLabel>,
}

/// Independently-authored legal-move enumeration for `state` under `rules`.
///
/// **STUB — awaiting the Stage-1 spec.** Returns empty collections; the
/// independence-protected oracle track (rules-sources-only authorship)
/// replaces this body. The signature is pinned: the `yokohama-difftest`
/// harness compares this result against the engine's `get_legal_moves`.
pub fn legal_moves(state: &GameState, rules: &Rules) -> OracleMoves {
    // No logic may be written here by implementation-aware agents (harness
    // track); the oracle track authors the enumeration from rules sources.
    let _ = (state, rules);
    OracleMoves { exact: Vec::new(), movement: Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Type-behavior tests only: constructing a real `GameState` requires
    // engine logic (`new_game`/`finalize_setup`), which this crate must not
    // call even from tests — the types-only rule is checked on imports.

    fn label(to: u8, mover_cost: u32, bank: u32) -> RouteLabel {
        RouteLabel {
            to: AreaId::new(to),
            mover_cost,
            player_credits: BTreeMap::new(),
            bank,
        }
    }

    #[test]
    fn route_label_ord_is_total_and_deterministic() {
        let mut labels = [label(3, 1, 0), label(1, 2, 1), label(1, 0, 0)];
        labels.sort();
        assert_eq!(
            labels.iter().map(|l| (l.to, l.mover_cost)).collect::<Vec<_>>(),
            vec![(AreaId::new(1), 0), (AreaId::new(1), 2), (AreaId::new(3), 1)],
        );
    }

    #[test]
    fn route_label_round_trips_through_serde() {
        let mut credits = BTreeMap::new();
        credits.insert(PlayerId::new(1), 2u32);
        let original = RouteLabel {
            to: AreaId::new(5),
            mover_cost: 3,
            player_credits: credits,
            bank: 1,
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let back: RouteLabel = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, back);
    }
}
