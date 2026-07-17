//! State invariant checks for the Task 18 self-play fuzz gate.
//!
//! `check_invariants` is a pure, read-only auditor: it never mutates state
//! and never depends on how the state was reached. The self-play driver
//! (`examples/selfplay.rs` / `tests/common`) calls it after every applied
//! move; any `Err` is a corrupted-state bug report (conservation broken,
//! a structural double-claim, or an out-of-range field), independent of
//! whether the move sequence that produced it was itself "legal" play.
//!
//! Card-count conservation (orders = 36, techs = 30) is gated on the
//! `Rules` actually containing that many cards, so `Rules::empty_for_tests()`
//! fixtures (empty registries) are exempt rather than spuriously failing.

use std::collections::HashSet;

use crate::game::rules::Rules;
use crate::types::GameState;

/// Check every engine-maintained conservation and structural invariant
/// against a `GameState`. Returns `Err` naming the first violation found.
pub fn check_invariants(state: &GameState, rules: &Rules) -> Result<(), String> {
    for p in &state.players {
        let total = p.assistants.total();
        if total != 20 {
            return Err(format!("P{} assistants total {total} != 20", p.id.0));
        }
        if p.order_cards_in_hand.len() > 3 {
            return Err(format!("P{} order hand size {} > 3", p.id.0, p.order_cards_in_hand.len()));
        }
        // Shophouses: 2 in hand + 6 in warehouse + 0 built at game start;
        // in_hand/in_warehouse/built are the only three zones a shophouse
        // token can occupy (Employment Agency moves warehouse->hand,
        // Building moves hand->built).
        let sh_total = p.shophouses.in_hand as usize
            + p.shophouses.in_warehouse as usize
            + p.shophouses.built.len();
        if sh_total != 8 {
            return Err(format!(
                "P{} shophouse total {sh_total} != 8 (in_hand={} in_warehouse={} built={})",
                p.id.0, p.shophouses.in_hand, p.shophouses.in_warehouse, p.shophouses.built.len()
            ));
        }
        // Trading houses: PlayerState's TradingHouseState has no in_hand
        // zone (confirmed by inspection: only `in_warehouse` and `built`
        // exist) -- Building buys directly from the warehouse. So the
        // conservation total has two terms, not three.
        let th_total = p.trading_houses.in_warehouse as usize + p.trading_houses.built.len();
        if th_total != 4 {
            return Err(format!(
                "P{} trading house total {th_total} != 4 (in_warehouse={} built={})",
                p.id.0, p.trading_houses.in_warehouse, p.trading_houses.built.len()
            ));
        }
    }

    // Order conservation: deck + ports (6 slots/board) + hands + completed +
    // removed = 36, with NO +1-per-hidden term -- the Unknown-starting-order
    // fiction parks BOTH dealt cards in orders_removed, so there's nothing
    // extra to count (decision log 2026-07-17). Gated on the registry
    // actually holding 36 cards so Rules::empty_for_tests() is exempt.
    let in_ports = state.port_state.port_a.slots.iter().flatten().count()
        + state.port_state.port_b.as_ref().map_or(0, |b| b.slots.iter().flatten().count());
    let in_hands: usize = state.players.iter().map(|p| p.order_cards_in_hand.len()).sum();
    let completed: usize = state.players.iter().map(|p| p.completed_orders.len()).sum();
    let total_orders =
        state.order_deck.len() + in_ports + in_hands + completed + state.orders_removed.len();
    if rules.orders.iter().count() == 36 && total_orders != 36 {
        return Err(format!(
            "order conservation broken: deck({}) + ports({in_ports}) + hands({in_hands}) + \
             completed({completed}) + removed({}) = {total_orders} != 36",
            state.order_deck.len(),
            state.orders_removed.len()
        ));
    }

    // Tech conservation: deck + labs (6 slots/board) + owned = 30 physical
    // cards (20 titles, 10 with a duplicate copy -- rules.techs.iter()
    // counts physical cards, verified against
    // `tech_registry_models_30_card_deck` in game/cards.rs). Gated the same
    // way as orders.
    let in_labs = state.lab_state.lab_a.slots.iter().flatten().count()
        + state.lab_state.lab_b.as_ref().map_or(0, |b| b.slots.iter().flatten().count());
    let owned: usize = state.players.iter().map(|p| p.technologies.len()).sum();
    let total_techs = state.tech_deck.len() + in_labs + owned;
    if rules.techs.iter().count() == 30 && total_techs != 30 {
        return Err(format!(
            "tech conservation broken: deck({}) + labs({in_labs}) + owned({owned}) = {total_techs} != 30",
            state.tech_deck.len()
        ));
    }

    // No two players may claim the same (area, slot) shophouse pairing.
    // (≤1 shophouse per player per area is already implied by each player's
    // own `HashMap<AreaId, u8>` key uniqueness; this catches cross-player
    // double-claims of one physical slot.)
    let shophouse_slots: Vec<(crate::types::AreaId, u8)> = state
        .players
        .iter()
        .flat_map(|p| p.shophouses.built.iter().map(|(&area, &slot)| (area, slot)))
        .collect();
    let unique_slots: HashSet<_> = shophouse_slots.iter().collect();
    if unique_slots.len() != shophouse_slots.len() {
        return Err(format!("duplicate shophouse (area, slot) claim across players: {shophouse_slots:?}"));
    }

    // No two players may claim a trading house in the same area.
    let th_areas: Vec<crate::types::AreaId> =
        state.players.iter().flat_map(|p| p.trading_houses.built.iter().copied()).collect();
    let unique_areas: HashSet<_> = th_areas.iter().collect();
    if unique_areas.len() != th_areas.len() {
        return Err(format!("duplicate trading house area claim across players: {th_areas:?}"));
    }

    if let Some(power) = state.current_action_power {
        if power > 5 {
            return Err(format!("current_action_power {power} > 5"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{create_test_game_state, finalize_setup, new_game, StartingOrderChoice};
    use crate::types::{AreaId, OrderCardId};

    /// Baseline fixture for the per-player / structural checks: a minimal
    /// state (empty decks/board) is fine here because these tests use
    /// `Rules::empty_for_tests()`, which gates the card-conservation checks
    /// off (see module docs) — those get their own golden fixture below.
    fn baseline() -> (GameState, Rules) {
        (create_test_game_state(2), Rules::empty_for_tests())
    }

    /// Golden fixture for the two card-conservation checks: a real,
    /// freshly-dealt 2-player game under the standard 36-order/30-tech
    /// registries, which `check_invariants` must accept as-is.
    fn golden_standard_game() -> (GameState, Rules) {
        let rules = Rules::standard();
        let pending = new_game(crate::types::GameConfig::for_player_count(2), &rules, 42).unwrap();
        let state = finalize_setup(
            pending,
            &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
        )
        .unwrap();
        (state, rules)
    }

    #[test]
    fn assistants_must_total_20_per_player() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        state.players[0].assistants.in_hand += 1; // now 21
        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("assistants total"), "expected assistants-total error, got: {err}");
    }

    #[test]
    fn order_hand_limit_is_3() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        state.players[0].order_cards_in_hand =
            vec![OrderCardId(1), OrderCardId(2), OrderCardId(3), OrderCardId(4)];
        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("order hand size"), "expected hand-limit error, got: {err}");
    }

    #[test]
    fn shophouse_total_is_8_per_player() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        state.players[0].shophouses.in_hand += 1; // now 9
        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("shophouse total"), "expected shophouse-total error, got: {err}");
    }

    #[test]
    fn trading_house_total_is_4_per_player() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        state.players[0].trading_houses.in_warehouse += 1; // now 5
        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("trading house total"), "expected TH-total error, got: {err}");
    }

    #[test]
    fn order_conservation_deck_ports_hands_completed_removed_equals_36() {
        let (state, rules) = golden_standard_game();
        assert!(check_invariants(&state, &rules).is_ok());

        let mut corrupt = state.clone();
        corrupt.orders_removed.pop(); // a card vanishes
        let err = check_invariants(&corrupt, &rules).unwrap_err();
        assert!(err.contains("order conservation"), "expected order-conservation error, got: {err}");
    }

    #[test]
    fn tech_conservation_deck_labs_owned_equals_30() {
        let (state, rules) = golden_standard_game();
        assert!(check_invariants(&state, &rules).is_ok());

        let mut corrupt = state.clone();
        corrupt.tech_deck.pop(); // a card vanishes
        let err = check_invariants(&corrupt, &rules).unwrap_err();
        assert!(err.contains("tech conservation"), "expected tech-conservation error, got: {err}");
    }

    #[test]
    fn shophouse_area_slot_pairs_are_unique_across_players() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        let area = AreaId::new(0);
        // Move one shophouse from warehouse to built for each player at the
        // SAME (area, slot) so each player's own total stays 8 (isolating
        // the cross-player uniqueness violation from the total-count check).
        state.players[0].shophouses.in_warehouse -= 1;
        state.players[0].shophouses.built.insert(area, 1);
        state.players[1].shophouses.in_warehouse -= 1;
        state.players[1].shophouses.built.insert(area, 1);

        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("duplicate shophouse"), "expected shophouse-uniqueness error, got: {err}");
    }

    #[test]
    fn trading_house_areas_are_unique_across_players() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        let area = AreaId::new(0);
        state.players[0].trading_houses.in_warehouse -= 1;
        state.players[0].trading_houses.built.push(area);
        state.players[1].trading_houses.in_warehouse -= 1;
        state.players[1].trading_houses.built.push(area);

        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("duplicate trading house"), "expected TH-uniqueness error, got: {err}");
    }

    #[test]
    fn current_action_power_must_not_exceed_5() {
        let (mut state, rules) = baseline();
        assert!(check_invariants(&state, &rules).is_ok());

        state.current_action_power = Some(6);
        let err = check_invariants(&state, &rules).unwrap_err();
        assert!(err.contains("current_action_power"), "expected power-bound error, got: {err}");
    }
}
