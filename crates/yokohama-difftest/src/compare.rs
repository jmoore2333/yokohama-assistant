//! Normalize engine `Vec<Move>` and `OracleMoves` into comparable key sets.
//!
//! Keying scheme:
//! - Non-`MovePresident` moves become exact string keys of the CANONICALIZED
//!   move (`move:<Debug>`). Canonicalization sorts the order-insensitive
//!   payloads (see [`canonical_move`]); the engine already emits each of
//!   them in a canonical order (verified against `legal_moves.rs`,
//!   2026-07-20 — details on [`canonical_move`]), so for engine-emitted
//!   moves sorting is a no-op and exists to normalize the oracle side.
//! - `MovePresident` moves are projected to a `RouteLabel` key by pricing
//!   the move's carried path with the ENGINE's own public pricing function
//!   (`route_fee_vector`, the one place that prices a Movement-step path) —
//!   the engine side of the comparison is the engine's own claim; the
//!   oracle side supplies its labels directly.
//! - Duplicate keys on either side are detected and reported (free
//!   soundness check: the engine's L2 canonicalization promises one move
//!   per distinct label, and an oracle emitting a duplicate is double
//!   counting).

use std::collections::BTreeMap;

use yokohama_core::{route_fee_vector, AreaAction, AreaId, GameState, Move, Rules};
use yokohama_oracle::{OracleMoves, RouteLabel};

/// One state's comparison result: keys present on exactly one side, plus
/// duplicate keys per side.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Comparison {
    /// Keys the engine offers that the oracle does not (SOUNDNESS direction:
    /// the engine may be offering an illegal move). Sorted, deduplicated.
    pub engine_only: Vec<String>,
    /// Keys the oracle finds that the engine misses (COMPLETENESS direction:
    /// the engine may be hiding a legal move — the direction that matters
    /// most for Phase 3). Sorted, deduplicated.
    pub oracle_only: Vec<String>,
    /// Keys appearing more than once on a side, formatted
    /// `"<side>: <key> x<count>"`. Sorted.
    pub duplicates: Vec<String>,
}

impl Comparison {
    /// True when anything at all disagrees (including duplicates — a
    /// duplicate key is a soundness defect on its side even when the sets
    /// match).
    pub fn is_divergent(&self) -> bool {
        !self.engine_only.is_empty() || !self.oracle_only.is_empty() || !self.duplicates.is_empty()
    }
}

/// Return `m` with every order-insensitive payload sorted into a canonical
/// order, leaving order-SENSITIVE payloads (e.g. `MovePresident::path`,
/// which is a concrete route) untouched.
///
/// What is canonicalized, and what the engine already emits (checked against
/// `crates/yokohama-core/src/game/legal_moves.rs`, 2026-07-20 — the sort
/// here is defensive normalization for the oracle side):
/// - `PlaceThreeAreas::placements` — a SET of distinct areas. Engine emits
///   ascending `AreaId` order (BTreeMap iteration + ordered combination
///   loops, `legal_moves.rs:136-196`). Sorted ascending.
/// - `Port::cards_taken` — an unordered pair/singleton. Engine emits one
///   ordering per unordered pair, in board-slot order (`legal_moves.rs:485-523`
///   — slot order, NOT card-id order). Sorted by card id.
/// - `Chinatown::exchanges` — a MULTISET: the engine deliberately collapses
///   different valid orderings of the same exchange multiset to one
///   representative (`chinatown_multiset_key`, `legal_moves.rs`), so an
///   oracle emitting a different valid ordering of the same multiset must
///   compare equal. Sorted by variant (stable Debug order).
/// - `ReturnPresidentToHand::recover` is a `BTreeMap` — already canonical.
pub fn canonical_move(m: &Move) -> Move {
    let mut m = m.clone();
    match &mut m {
        Move::PlaceThreeAreas { placements } => placements.sort(),
        Move::ExecuteAreaAction { action } => match action {
            AreaAction::Port { cards_taken, .. } => cards_taken.sort_by_key(|c| c.0),
            AreaAction::Chinatown { exchanges } => {
                // Stable variant order via the Debug rendering (the enum has
                // no Ord); any fixed total order works for a multiset key.
                exchanges.sort_by_key(|e| format!("{e:?}"));
            }
            _ => {}
        },
        _ => {}
    }
    m
}

/// Exact key for a non-`MovePresident` move: `"move:"` + the Debug rendering
/// of [`canonical_move`]. (`MovePresident` moves are keyed via
/// [`engine_route_label`] by [`compare_moves`], never through this.)
pub fn move_key(m: &Move) -> String {
    format!("move:{:?}", canonical_move(m))
}

/// Key for a president-movement label.
pub fn route_label_key(label: &RouteLabel) -> String {
    format!("route:{label:?}")
}

/// Project an engine `MovePresident { path, destination }` to a
/// [`RouteLabel`] by pricing the carried path with the engine's own public
/// `route_fee_vector` (tech-aware, non-mercenary) for the CURRENT player of
/// `state` — the state the move was enumerated in. An empty path (Station /
/// Tram teleport) prices to the all-zero label, matching the card texts
/// ("for free").
pub fn engine_route_label(
    state: &GameState,
    rules: &Rules,
    path: &[AreaId],
    destination: AreaId,
) -> RouteLabel {
    let vector = route_fee_vector(state, state.current_player, path, Some(&rules.techs), false);
    let mover_cost: u32 = vector.values().sum();
    let mut player_credits = BTreeMap::new();
    let mut bank: u32 = 0;
    for (recipient, amount) in vector {
        match recipient {
            Some(pid) => {
                player_credits.insert(pid, amount);
            }
            None => bank += amount,
        }
    }
    RouteLabel { to: destination, mover_cost, player_credits, bank }
}

/// Pure multiset diff over pre-built key lists (the testable core of
/// [`compare_moves`]): `engine_only`/`oracle_only` are the keys present on
/// one side and absent on the other (deduplicated, sorted); `duplicates`
/// lists every key occurring more than once on a side.
pub fn diff_key_multisets(engine: &[String], oracle: &[String]) -> Comparison {
    fn counts(keys: &[String]) -> BTreeMap<&String, usize> {
        let mut map = BTreeMap::new();
        for k in keys {
            *map.entry(k).or_insert(0) += 1;
        }
        map
    }
    let engine_counts = counts(engine);
    let oracle_counts = counts(oracle);

    let engine_only: Vec<String> = engine_counts
        .keys()
        .filter(|k| !oracle_counts.contains_key(*k))
        .map(|k| (*k).clone())
        .collect();
    let oracle_only: Vec<String> = oracle_counts
        .keys()
        .filter(|k| !engine_counts.contains_key(*k))
        .map(|k| (*k).clone())
        .collect();

    let mut duplicates: Vec<String> = Vec::new();
    for (side, side_counts) in [("engine", &engine_counts), ("oracle", &oracle_counts)] {
        for (key, n) in side_counts {
            if *n > 1 {
                duplicates.push(format!("{side}: {key} x{n}"));
            }
        }
    }
    duplicates.sort();

    // BTreeMap iteration already yields engine_only/oracle_only sorted.
    Comparison { engine_only, oracle_only, duplicates }
}

/// Compare the engine's legal-move list against the oracle's answer for the
/// same `state`. Engine `MovePresident` moves are projected to route-label
/// keys (see [`engine_route_label`]); everything else uses exact
/// canonicalized keys. `oracle.exact` should never contain a
/// `MovePresident` (see `OracleMoves` docs) — if one appears anyway it is
/// priced identically to an engine move so a genuinely-equal move cannot
/// spuriously diverge on encoding.
pub fn compare_moves(
    state: &GameState,
    rules: &Rules,
    engine_moves: &[Move],
    oracle: &OracleMoves,
) -> Comparison {
    let key_for = |m: &Move| -> String {
        match m {
            Move::MovePresident { path, destination } => {
                route_label_key(&engine_route_label(state, rules, path, *destination))
            }
            other => move_key(other),
        }
    };
    let engine_keys: Vec<String> = engine_moves.iter().map(key_for).collect();
    let mut oracle_keys: Vec<String> = oracle.exact.iter().map(key_for).collect();
    oracle_keys.extend(oracle.movement.iter().map(route_label_key));
    diff_key_multisets(&engine_keys, &oracle_keys)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yokohama_core::{ChinatownExchange, OrderCardId, PortPayment};

    fn place(areas: &[u8]) -> Move {
        Move::PlaceThreeAreas { placements: areas.iter().map(|&a| AreaId::new(a)).collect() }
    }

    #[test]
    fn place_three_areas_key_is_order_insensitive() {
        assert_eq!(move_key(&place(&[3, 1, 2])), move_key(&place(&[1, 2, 3])));
    }

    #[test]
    fn port_cards_key_is_order_insensitive() {
        let port = |a: u8, b: u8| Move::ExecuteAreaAction {
            action: AreaAction::Port {
                cards_taken: vec![OrderCardId(a), OrderCardId(b)],
                second_card_payment: Some(PortPayment::TwoYen),
            },
        };
        assert_eq!(move_key(&port(9, 4)), move_key(&port(4, 9)));
    }

    #[test]
    fn chinatown_exchanges_key_is_order_insensitive() {
        let china = |ex: Vec<ChinatownExchange>| Move::ExecuteAreaAction {
            action: AreaAction::Chinatown { exchanges: ex },
        };
        assert_eq!(
            move_key(&china(vec![
                ChinatownExchange::YenToTea,
                ChinatownExchange::CopperToYen,
                ChinatownExchange::YenToTea,
            ])),
            move_key(&china(vec![
                ChinatownExchange::YenToTea,
                ChinatownExchange::YenToTea,
                ChinatownExchange::CopperToYen,
            ])),
        );
    }

    #[test]
    fn canonicalization_never_conflates_distinct_moves() {
        // Different multisets stay different...
        assert_ne!(move_key(&place(&[1, 2])), move_key(&place(&[1, 3])));
        assert_ne!(move_key(&place(&[1])), move_key(&place(&[1, 2])));
        // ...and payment variants stay distinct even with equal card sets.
        let port = |pay: PortPayment| Move::ExecuteAreaAction {
            action: AreaAction::Port {
                cards_taken: vec![OrderCardId(4), OrderCardId(9)],
                second_card_payment: Some(pay),
            },
        };
        assert_ne!(move_key(&port(PortPayment::TwoYen)), move_key(&port(PortPayment::OneImportedGood)));
    }

    #[test]
    fn diff_reports_each_side_and_duplicates() {
        let e = |s: &str| s.to_string();
        let engine = vec![e("a"), e("b"), e("b"), e("c")];
        let oracle = vec![e("b"), e("c"), e("d")];
        let cmp = diff_key_multisets(&engine, &oracle);
        assert_eq!(cmp.engine_only, vec!["a"]);
        assert_eq!(cmp.oracle_only, vec!["d"]);
        assert_eq!(cmp.duplicates, vec!["engine: b x2"]);
        assert!(cmp.is_divergent());
    }

    #[test]
    fn identical_multisets_are_not_divergent() {
        let keys = vec!["x".to_string(), "y".to_string()];
        let cmp = diff_key_multisets(&keys, &keys);
        assert_eq!(cmp, Comparison::default());
        assert!(!cmp.is_divergent());
    }
}
