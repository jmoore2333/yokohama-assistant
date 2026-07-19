//! Pending-choice queue mechanics (design 2026-07-18 §5.1).
use crate::types::*;
use crate::game::*;
use crate::game::actions::*;

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
        PendingChoice::AssistantMove => {
            let mut moves: Vec<Move> = assistant_move_options(state)
                .into_iter()
                .map(|(from, to)| Move::ChooseAssistantMove { from, to })
                .collect();
            moves.push(Move::DeclineAssistantMove);
            moves
        }
        PendingChoice::ChurchSpace { total_faith } => {
            eligible_church_spaces(&state.church_track, *total_faith, state.config.player_count)
                .into_iter()
                .map(|space| Move::ChooseChurchSpace { space })
                .collect()
        }
        PendingChoice::CustomsSpace { cost } => {
            eligible_customs_spaces_for_cost(&state.customs_track, *cost, state.config.player_count)
                .into_iter()
                .map(|space| Move::ChooseCustomsSpace { space })
                .collect()
        }
        PendingChoice::RecoveryLeave => vec![
            Move::ChooseRecoveryLeave { leave_one: false },
            Move::ChooseRecoveryLeave { leave_one: true },
        ],
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

/// Movement-icon reward options (rulebook p.10): "Move an Assistant pawn
/// from your hand or from an Area to another Area... You do not need to pay
/// other President pawns." Sources are the player's hand (if it holds any
/// assistants) plus every Area currently holding >=1 of the player's own
/// on-board assistants -- warehouse/management-board assistants are never a
/// source, since they aren't tracked in `on_board`. Destinations are every
/// non-Canal Area other than the source Area itself (Hand has no exclusion).
/// No fees apply, even into an Area occupied by another player's President.
pub fn assistant_move_options(state: &GameState) -> Vec<(MoveSource, AreaId)> {
    let p = state.current_player();
    let non_canal_areas: Vec<AreaId> = state.board_layout.areas.iter()
        .filter(|(_, cfg)| cfg.area_type != AreaType::Canal)
        .map(|(&id, _)| id)
        .collect();

    let mut options = Vec::new();
    if p.assistants.in_hand >= 1 {
        options.extend(non_canal_areas.iter().map(|&to| (MoveSource::Hand, to)));
    }
    for (&area, &count) in &p.assistants.on_board {
        if count == 0 {
            continue;
        }
        options.extend(
            non_canal_areas.iter().filter(|&&to| to != area).map(|&to| (MoveSource::Area(area), to)),
        );
    }
    options
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
        (PendingChoice::AssistantMove, Move::ChooseAssistantMove { from, to }) => {
            let p = state.player_mut(player);
            match from {
                MoveSource::Hand => p.assistants.in_hand -= 1,
                MoveSource::Area(area) => {
                    if let Some(count) = p.assistants.on_board.get_mut(area) {
                        *count -= 1;
                        if *count == 0 {
                            p.assistants.on_board.remove(area);
                        }
                    }
                }
            }
            *p.assistants.on_board.entry(*to).or_insert(0) += 1;
        }
        (PendingChoice::AssistantMove, Move::DeclineAssistantMove) => {}
        (PendingChoice::ChurchSpace { .. }, Move::ChooseChurchSpace { space }) => {
            place_church_assistant(state, player, *space, rules);
        }
        (PendingChoice::CustomsSpace { .. }, Move::ChooseCustomsSpace { space }) => {
            place_customs_assistant(state, player, *space, rules);
        }
        (PendingChoice::RecoveryLeave, Move::ChooseRecoveryLeave { leave_one }) => {
            apply_recovery_with_leave(state, *leave_one);
            finish_recovery_transition(state);
        }
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

    fn area_a() -> AreaId { AreaId::new(0) }
    fn area_b() -> AreaId { AreaId::new(1) }
    fn canal_area() -> AreaId { AreaId::new(2) }

    /// 3-area board (incl. a Canal) with the current player holding 1
    /// assistant in Area A and 1 in hand, AssistantMove already queued as
    /// head (bypassing push_choice's singleton probe -- that path belongs to
    /// `assistant_move_auto_declines_when_no_pawn_anywhere` below). An
    /// opponent President sits in Area B to prove the p.10 "you do not need
    /// to pay other President pawns" rule applies to this icon.
    fn assistant_move_fixture() -> (GameState, Rules) {
        let mut state = create_test_game_state(3);
        let rules = Rules::empty_for_tests();
        state.board_layout.areas.insert(area_a(), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 0) });
        state.board_layout.areas.insert(area_b(), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 1) });
        state.board_layout.areas.insert(canal_area(), AreaConfig { area_type: AreaType::Canal, grid_position: (0, 2) });
        state.current_player = PlayerId::new(0);
        state.players[0].assistants.on_board.insert(area_a(), 1);
        state.players[0].assistants.in_hand = 1;
        state.players[1].president_location = PresidentLocation::OnBoard(area_b());
        state.pending_choices.push(PendingChoice::AssistantMove);
        (state, rules)
    }

    /// All 20 assistants off the board (hand empty, on_board empty -- i.e.
    /// all sitting in the warehouse/management-board pool, which is never a
    /// valid AssistantMove source). Only Decline remains possible.
    fn no_movable_assistant_fixture() -> (GameState, Rules) {
        let mut state = create_test_game_state(2);
        let rules = Rules::empty_for_tests();
        state.players[0].assistants.in_hand = 0;
        state.players[0].assistants.in_warehouse = 20;
        state.players[0].assistants.on_board.clear();
        (state, rules)
    }

    #[test]
    fn assistant_move_options_cover_hand_and_own_areas_excluding_canal() {
        // 3p board (has Canal). Own assistants: 1 in area A, hand 1. Opponent president in area B.
        let (state, rules) = assistant_move_fixture();
        let legal = get_legal_moves(&state, &rules); // AssistantMove head queued
        // sources: Hand + Area(A); dests: every non-Canal area != source. Decline included.
        assert!(legal.contains(&Move::ChooseAssistantMove { from: MoveSource::Hand, to: area_b() })); // opp president dest OK, free
        assert!(legal.iter().all(|m| match m {
            Move::ChooseAssistantMove { to, .. } => *to != canal_area(),
            _ => true,
        }));
        assert!(legal.contains(&Move::DeclineAssistantMove));
    }

    #[test]
    fn assistant_move_is_free_and_conserves_pawns() {
        let (state, rules) = assistant_move_fixture();
        let money = state.current_player().money;
        let s = apply_move(&state, &Move::ChooseAssistantMove { from: MoveSource::Area(area_a()), to: area_b() }, &rules).unwrap();
        assert_eq!(s.current_player().money, money); // no fees, even into an opponent-president area
        assert_eq!(s.current_player().assistants.on_board.get(&area_a()), None); // cleaned up at 0
        assert_eq!(s.current_player().assistants.on_board[&area_b()], 1);
    }

    #[test]
    fn assistant_move_auto_declines_when_no_pawn_anywhere() {
        // All 20 assistants in warehouse/locked: only Decline possible -> singleton auto.
        let (mut state, rules) = no_movable_assistant_fixture();
        push_choice(&mut state, PendingChoice::AssistantMove, &rules);
        assert!(state.pending_choices.is_empty());
    }
}
