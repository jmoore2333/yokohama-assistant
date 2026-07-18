//! Pending-choice queue mechanics (design 2026-07-18 §5.1).
use crate::types::*;
use crate::game::*;

/// Resolution moves for the HEAD pending choice. Non-empty by construction
/// (every choice type offers decline or has >=1 outcome, else it was
/// auto-resolved at push time).
pub fn generate_choice_resolutions(state: &GameState, head: &PendingChoice, rules: &Rules) -> Vec<Move> {
    generate_choice_resolutions_for(state, head, rules)
}

/// Resolution moves for `choice` AS IF it were the head. Split out from
/// `generate_choice_resolutions` so `push_choice` can price a NOT-yet-queued
/// choice (its singleton probe) with the exact same logic the public
/// head-based entry point uses.
fn generate_choice_resolutions_for(state: &GameState, choice: &PendingChoice, _rules: &Rules) -> Vec<Move> {
    match choice {
        PendingChoice::AnyGood => vec![
            Move::ChooseAnyGood { good: TradeGood::Copper },
            Move::ChooseAnyGood { good: TradeGood::Silk },
            Move::ChooseAnyGood { good: TradeGood::Tea },
            Move::ChooseAnyGood { good: TradeGood::Seafood },
        ],
        PendingChoice::WarehouseTake => {
            let p = state.current_player();
            let mut moves: Vec<Move> = warehouse_take_options(p)
                .into_iter()
                .map(|item| Move::ChooseWarehouseTake { item })
                .collect();
            moves.push(Move::DeclineWarehouseTake);
            moves
        }
    }
}

/// Warehouse items the player can currently afford to take: assistants are
/// free, shophouses cost 2 Yen, trading houses cost the printed 4/5/6/7
/// progression (`TradingHouseState::take_cost`) indexed by tokens already
/// out of the warehouse. Each also requires >=1 in the warehouse.
pub fn warehouse_take_options(p: &PlayerState) -> Vec<WarehouseItem> {
    let mut v = Vec::new();
    if p.assistants.in_warehouse >= 1 { v.push(WarehouseItem::Assistant); }
    if p.shophouses.in_warehouse >= 1 && p.money >= 2 { v.push(WarehouseItem::Shophouse); }
    let already = p.trading_houses.in_hand as usize + p.trading_houses.total_built();
    if p.trading_houses.in_warehouse >= 1 && p.money >= TradingHouseState::take_cost(already) { v.push(WarehouseItem::TradingHouse); }
    v
}

/// Push a choice, auto-resolving singletons inline (design §5.1). For
/// AnyGood the option set is always 4, so it always queues; WarehouseTake
/// collapses to Decline alone whenever nothing in the warehouse is worth
/// taking, and resolves immediately instead of round-tripping through the
/// caller.
///
/// NOTE: the singleton path calls `resolve_choice_move`, which can itself
/// push further choices (a cascade) -- bounded by the same small reward
/// bundles that bound the whole queue, so plain recursion is fine.
pub fn push_choice(state: &mut GameState, choice: PendingChoice, rules: &Rules) {
    // Compute the resolution set AS IF this choice were head; if it has
    // exactly one element, resolve inline (design §5.1 singleton rule).
    let opts = generate_choice_resolutions_for(state, &choice, rules);
    if opts.len() == 1 {
        state.pending_choices.insert(0, choice);
        let mv = opts.into_iter().next().unwrap();
        resolve_choice_move(state, &mv, rules).expect("singleton resolution is legal by construction");
        return;
    }
    state.pending_choices.push(choice);
}

/// Apply a Choose* move against the head pending choice. Validates the move
/// is a resolution OF the head (type + membership in the generated set),
/// pops the head, applies the effect. Zero mutation on error.
pub fn resolve_choice_move(state: &mut GameState, mv: &Move, rules: &Rules) -> Result<(), MoveError> {
    let head = state.pending_choices.first().cloned()
        .ok_or(MoveError::RuleViolation("No pending choice to resolve"))?;
    if !generate_choice_resolutions(state, &head, rules).contains(mv) {
        return Err(MoveError::RuleViolation("Move does not resolve the pending choice"));
    }
    state.pending_choices.remove(0);
    let player = state.current_player;
    match (head, mv) {
        (PendingChoice::AnyGood, Move::ChooseAnyGood { good }) => {
            let mining = match good {
                TradeGood::Copper => copper_bonus(state, player, 1, &rules.techs),
                _ => 0,
            };
            let p = state.player_mut(player);
            match good {
                TradeGood::Copper => p.copper += 1 + mining as u32,
                TradeGood::Silk => p.silk += 1,
                TradeGood::Tea => p.tea += 1,
                TradeGood::Seafood => p.seafood += 1,
            }
        }
        (PendingChoice::WarehouseTake, Move::ChooseWarehouseTake { item }) => {
            let p = state.player_mut(player);
            match item {
                WarehouseItem::Assistant => {
                    p.assistants.in_warehouse -= 1;
                    p.assistants.in_hand += 1;
                }
                WarehouseItem::Shophouse => {
                    p.money -= 2;
                    p.shophouses.in_warehouse -= 1;
                    p.shophouses.in_hand += 1;
                }
                WarehouseItem::TradingHouse => {
                    let already = p.trading_houses.in_hand as usize + p.trading_houses.total_built();
                    p.money -= TradingHouseState::take_cost(already);
                    p.trading_houses.in_warehouse -= 1;
                    p.trading_houses.in_hand += 1;
                }
            }
        }
        (PendingChoice::WarehouseTake, Move::DeclineWarehouseTake) => {}
        _ => unreachable!("membership check above guarantees type match"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::create_test_game_state;

    #[test]
    fn generate_choice_resolutions_any_good_offers_all_four_goods() {
        let state = create_test_game_state(2);
        let rules = Rules::empty_for_tests();
        let moves = generate_choice_resolutions(&state, &PendingChoice::AnyGood, &rules);
        assert_eq!(moves, vec![
            Move::ChooseAnyGood { good: TradeGood::Copper },
            Move::ChooseAnyGood { good: TradeGood::Silk },
            Move::ChooseAnyGood { good: TradeGood::Tea },
            Move::ChooseAnyGood { good: TradeGood::Seafood },
        ]);
    }

    #[test]
    fn push_choice_appends_to_queue_fifo() {
        let mut state = create_test_game_state(2);
        let rules = Rules::empty_for_tests();
        push_choice(&mut state, PendingChoice::AnyGood, &rules);
        push_choice(&mut state, PendingChoice::AnyGood, &rules);
        assert_eq!(state.pending_choices, vec![PendingChoice::AnyGood, PendingChoice::AnyGood]);
    }

    #[test]
    fn resolve_choice_move_errors_when_queue_empty() {
        let mut state = create_test_game_state(2);
        let rules = Rules::empty_for_tests();
        let result = resolve_choice_move(&mut state, &Move::ChooseAnyGood { good: TradeGood::Copper }, &rules);
        assert!(result.is_err());
        assert!(state.pending_choices.is_empty());
    }

    #[test]
    fn resolve_choice_move_pops_head_and_applies_chosen_good() {
        let mut state = create_test_game_state(2);
        let rules = Rules::empty_for_tests();
        push_choice(&mut state, PendingChoice::AnyGood, &rules);
        let tea_before = state.current_player().tea;
        resolve_choice_move(&mut state, &Move::ChooseAnyGood { good: TradeGood::Tea }, &rules).unwrap();
        assert!(state.pending_choices.is_empty());
        assert_eq!(state.current_player().tea, tea_before + 1);
    }

    /// Warehouse: 1 assistant, 1 shophouse, 2 trading houses; nothing taken
    /// yet; state pre-loaded with a WarehouseTake head (not via push_choice,
    /// so the fixture itself never exercises the singleton path).
    fn warehouse_choice_fixture(money: u32) -> (GameState, Rules) {
        let mut state = create_test_game_state(2);
        let rules = Rules::empty_for_tests();
        state.players[0].money = money;
        state.players[0].assistants.in_warehouse = 1;
        state.players[0].shophouses.in_warehouse = 1;
        state.players[0].trading_houses.in_warehouse = 2;
        state.pending_choices.push(PendingChoice::WarehouseTake);
        (state, rules)
    }

    /// Two queued WarehouseTake choices (the shape `execute_customs_action`
    /// produces for space 1's +2 rider), trading houses fresh out of the
    /// warehouse (already_taken = 0).
    fn double_warehouse_choice_fixture(money: u32) -> (GameState, Rules) {
        let mut state = create_test_game_state(2);
        let rules = Rules::empty_for_tests();
        state.players[0].money = money;
        state.players[0].trading_houses.in_warehouse = 2;
        state.pending_choices.push(PendingChoice::WarehouseTake);
        state.pending_choices.push(PendingChoice::WarehouseTake);
        (state, rules)
    }

    /// Warehouse entirely empty: every item type is unavailable, so the only
    /// resolution is Decline.
    fn warehouse_choice_base() -> (GameState, Rules) {
        let mut state = create_test_game_state(2);
        let rules = Rules::empty_for_tests();
        state.players[0].assistants.in_warehouse = 0;
        state.players[0].shophouses.in_warehouse = 0;
        state.players[0].trading_houses.in_warehouse = 0;
        (state, rules)
    }

    #[test]
    fn warehouse_take_offers_affordable_items_plus_decline() {
        // warehouse: 1 assistant, 1 shophouse, 2 TH; money 4; nothing taken yet.
        let (state, rules) = warehouse_choice_fixture(/*money*/ 4);
        let legal = get_legal_moves(&state, &rules); // state pre-loaded with WarehouseTake head
        // assistant (free) + shophouse (2) + TH (first take costs 4) + decline = 4
        assert_eq!(legal.len(), 4);
    }

    #[test]
    fn warehouse_take_th_repriced_across_consecutive_takes() {
        // Two queued takes (customs space-1 shape), money 9, TH already out: 0.
        let (state, rules) = double_warehouse_choice_fixture(9);
        let s1 = apply_move(&state, &Move::ChooseWarehouseTake { item: WarehouseItem::TradingHouse }, &rules).unwrap();
        assert_eq!(s1.current_player().money, 9 - 4);
        let s2 = apply_move(&s1, &Move::ChooseWarehouseTake { item: WarehouseItem::TradingHouse }, &rules).unwrap();
        assert_eq!(s2.current_player().money, 9 - 4 - 5); // progression advanced
        assert_eq!(s2.current_player().trading_houses.in_hand, 2);
    }

    #[test]
    fn warehouse_take_auto_resolves_when_only_decline_possible() {
        // Empty warehouse: pushing the choice must NOT queue (singleton auto, design §5.1).
        let (mut state, rules) = warehouse_choice_base(/*warehouse all zero*/);
        push_choice(&mut state, PendingChoice::WarehouseTake, &rules);
        assert!(state.pending_choices.is_empty());
    }

    #[test]
    fn decline_warehouse_take_changes_nothing_but_pops() {
        let (state, rules) = warehouse_choice_fixture(4);
        let s = apply_move(&state, &Move::DeclineWarehouseTake, &rules).unwrap();
        assert_eq!(s.current_player().money, 4);
        assert!(s.pending_choices.is_empty());
    }

    #[test]
    fn warehouse_take_shophouse_resolution_charges_and_moves_token() {
        // Money exactly 2 -- the shophouse-take boundary: affordable, lands on 0.
        let (state, rules) = warehouse_choice_fixture(2);
        let starting_hand = state.current_player().shophouses.in_hand;
        let s = apply_move(&state, &Move::ChooseWarehouseTake { item: WarehouseItem::Shophouse }, &rules).unwrap();
        assert_eq!(s.current_player().money, 0);
        assert_eq!(s.current_player().shophouses.in_warehouse, 0);
        assert_eq!(s.current_player().shophouses.in_hand, starting_hand + 1);
        assert!(s.pending_choices.is_empty());
    }

    #[test]
    fn push_choice_singleton_auto_resolve_leaves_pre_existing_queue_untouched() {
        // The insert-at-front trick's target scenario: a genuinely different
        // choice already queued ahead of the new singleton push.
        let (mut state, rules) = warehouse_choice_base(/*warehouse all zero*/);
        state.pending_choices.push(PendingChoice::AnyGood);
        let starting_money = state.current_player().money;

        push_choice(&mut state, PendingChoice::WarehouseTake, &rules);

        // Auto-declined silently: queue and money are exactly what they were before this push.
        assert_eq!(state.pending_choices, vec![PendingChoice::AnyGood]);
        assert_eq!(state.current_player().money, starting_money);
    }
}
