//! Integration tests for complete game scenarios.

use yokohama_core::*;
use std::collections::HashMap;

fn setup_2p_game() -> (GameState, OrderRegistry, TechRegistry) {
    // Create minimal 2-player game state for testing
    let config = GameConfig::new(
        2,
        vec![PlayerColor::Red, PlayerColor::Blue],
        PlayerId::new(0),
    );

    let mut areas = HashMap::new();
    areas.insert(AreaId::new(0), AreaConfig {
        area_type: AreaType::Bank,
        grid_position: (0, 0),
    });
    areas.insert(AreaId::new(1), AreaConfig {
        area_type: AreaType::CopperMine,
        grid_position: (0, 1),
    });

    let mut adjacency = HashMap::new();
    adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
    adjacency.insert(AreaId::new(1), vec![AreaId::new(0)]);

    let board_layout = BoardLayout {
        areas,
        adjacency,
        canal_crossings: vec![],
        building_sites: HashMap::new(),
        five_power_tokens: HashMap::new(),
    };

    let players = vec![
        PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
        PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
    ];

    let state = GameState {
        config,
        board_layout,
        players,
        current_player: PlayerId::new(0),
        turn_phase: TurnPhase::AdditionalActionFirst,
        turn_number: 1,
        additional_action_state: AdditionalActionState::new(),
        current_action_power: None,
        is_mercenary_action: false,
        mercenary_target_area: None,
        mercenary_return_phase: None,
        president_start_area: None,
        port_state: PortState::new(false),
        lab_state: LabState::new(false),
        // Non-empty by default so `check_end_game_triggers` doesn't spuriously fire the
        // order-deck-empty condition (Port A starts with all slots empty too) for tests
        // that aren't exercising that trigger specifically.
        order_deck: vec![OrderCardId(1)],
        tech_deck: vec![],
        orders_removed: vec![],
        church_track: ChurchTrack::new(),
        customs_track: CustomsTrack::new(),
        achievements: [
            AchievementCard::new(AchievementType::Copper5),
            AchievementCard::new(AchievementType::Money10),
            AchievementCard::new(AchievementType::Production4),
        ],
        mercenary_supply: MercenarySupply::new(),
        station_location: None,
        end_game_triggered: false,
        trigger_reason: None,
        final_round: false,
        trigger_player: None,
    };

    (state, OrderRegistry::new(), TechRegistry::new())
}

#[test]
fn test_basic_turn_flow() {
    let (state, _, _) = setup_2p_game();

    // Should start in Additional Action First
    assert_eq!(state.turn_phase, TurnPhase::AdditionalActionFirst);

    // Get legal moves
    let rules = Rules::empty_for_tests();
    let moves = get_legal_moves(&state, &rules);
    assert!(!moves.is_empty());

    // Should include EndAdditionalPhase
    assert!(moves.iter().any(|m| matches!(m, Move::EndAdditionalPhase)));
}

#[test]
fn test_power_calculation_comprehensive() {
    let (mut state, _, _) = setup_2p_game();
    let player = PlayerId::new(0);
    let area = AreaId::new(0);

    // No power initially
    assert_eq!(calculate_power(&state, player, area), 0);

    // Add president
    state.players[0].president_location = PresidentLocation::OnBoard(area);
    assert_eq!(calculate_power(&state, player, area), 1);

    // Add assistants
    state.players[0].assistants.on_board.insert(area, 2);
    assert_eq!(calculate_power(&state, player, area), 3);

    // Add shophouse
    state.players[0].shophouses.built.insert(area, 0);
    assert_eq!(calculate_power(&state, player, area), 4);

    // Add trading house
    state.players[0].trading_houses.built.push(area);
    assert_eq!(calculate_power(&state, player, area), 5); // Capped at 5
}

#[test]
fn test_end_game_trigger_trading_houses() {
    let (mut state, _, _) = setup_2p_game();

    // Build 4 trading houses for player 0
    state.players[0].trading_houses.built = vec![
        AreaId::new(0),
        AreaId::new(1),
        AreaId::new(2),
        AreaId::new(3),
    ];
    state.players[0].trading_houses.in_warehouse = 0;

    check_end_game_triggers(&mut state);

    assert!(state.end_game_triggered);
    assert!(matches!(
        state.trigger_reason,
        Some(EndGameTrigger::TradingHousesComplete(p)) if p == PlayerId::new(0)
    ));
}

#[test]
fn test_remainder_scoring() {
    let mut player = PlayerState::new(PlayerId::new(0), PlayerColor::Red, true);

    // 3 imported goods = 3 VP
    player.imported_goods = 3;
    // 6 yen = 3 VP
    player.money = 6;
    // 9 trade goods = 3 VP
    player.copper = 3;
    player.silk = 3;
    player.tea = 3;

    assert_eq!(calculate_remainder_bonus(&player), 9);
}

#[test]
fn test_phase_transitions() {
    let (mut state, _, _) = setup_2p_game();

    // End Additional Action First -> Placement
    let rules = Rules::empty_for_tests();
    state.turn_phase = TurnPhase::AdditionalActionFirst;
    let new_state = apply_move(&state, &Move::EndAdditionalPhase, &rules).unwrap();
    assert_eq!(new_state.turn_phase, TurnPhase::Placement);

    // Test placement with 0 assistants -> Movement
    let placement_state = new_state;
    let moves = get_legal_moves(&placement_state, &rules);
    assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements } if placements.is_empty())));

    let after_placement = apply_move(&placement_state, &Move::PlaceThreeAreas { placements: vec![] }, &rules).unwrap();
    assert_eq!(after_placement.turn_phase, TurnPhase::Movement);
}

#[test]
fn test_church_track_threshold() {
    let (mut state, _, _) = setup_2p_game();

    // 2-player game: threshold is 4 assistants
    assert_eq!(state.config.track_trigger(), 4);

    // Add 3 assistants to church track - shouldn't trigger
    state.church_track.slots[0] = Some(ChurchSlot { player: PlayerId::new(0), arrival_order: 0 });
    state.church_track.slots[1] = Some(ChurchSlot { player: PlayerId::new(0), arrival_order: 1 });
    state.church_track.slots[2] = Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 2 });

    check_end_game_triggers(&mut state);
    assert!(!state.end_game_triggered);

    // Add 4th assistant - should trigger
    state.church_track.slots[3] = Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 3 });
    check_end_game_triggers(&mut state);
    assert!(state.end_game_triggered);
    assert!(matches!(state.trigger_reason, Some(EndGameTrigger::ChurchTrackFull)));
}
