//! End game trigger detection and final round handling.

use crate::types::*;

/// Check all end game triggers and update state if triggered.
pub fn check_end_game_triggers(state: &mut GameState) {
    if state.end_game_triggered {
        return; // Already triggered
    }

    // 1. Trading Houses: any player built all 4
    for p in &state.players {
        if p.trading_houses.total_built() >= 4 {
            trigger_end_game(state, EndGameTrigger::TradingHousesComplete(p.id), p.id);
            return;
        }
    }

    // 2. Shophouses: any player built all 8
    for p in &state.players {
        if p.shophouses.total_built() >= 8 {
            trigger_end_game(state, EndGameTrigger::ShophousesComplete(p.id), p.id);
            return;
        }
    }

    // 3. Order deck empty AND a Port slot is empty
    if state.order_deck_remaining == 0 {
        let port_has_empty_slot = check_port_empty_slot(state);
        if port_has_empty_slot {
            trigger_end_game(state, EndGameTrigger::OrderDeckEmpty, state.current_player);
            return;
        }
    }

    // 4. Church track: reached threshold
    let church_threshold = state.config.track_trigger();
    if state.church_track.total_assistants() >= church_threshold {
        trigger_end_game(state, EndGameTrigger::ChurchTrackFull, state.current_player);
        return;
    }

    // 5. Customs track: reached threshold
    let customs_threshold = state.config.track_trigger();
    if state.customs_track.total_assistants() >= customs_threshold {
        trigger_end_game(state, EndGameTrigger::CustomsTrackFull, state.current_player);
    }
}

fn trigger_end_game(state: &mut GameState, reason: EndGameTrigger, trigger_player: PlayerId) {
    state.end_game_triggered = true;
    state.trigger_reason = Some(reason);
    state.trigger_player = Some(trigger_player);
}

fn check_port_empty_slot(state: &GameState) -> bool {
    // Check Port A
    if state.port_state.port_a.low.is_none()
        || state.port_state.port_a.mid.is_none()
        || state.port_state.port_a.high.is_none()
    {
        return true;
    }

    // Check Port B if exists
    if let Some(ref port_b) = state.port_state.port_b {
        if port_b.low.is_none() || port_b.mid.is_none() || port_b.high.is_none() {
            return true;
        }
    }

    false
}

/// Determine if the game is completely over (final round finished).
pub fn is_game_over(state: &GameState) -> bool {
    if !state.end_game_triggered {
        return false;
    }

    if !state.final_round {
        // Need to complete current round first, then play final round
        return false;
    }

    // Final round complete when we've looped back after all players took a turn
    // The player before start player should be the last to play
    let player_before_start = PlayerId::new(
        (state.config.start_player.0 + state.config.player_count - 1) % state.config.player_count,
    );

    // Game is over if current player is at TurnEnd and is the player before start player
    state.current_player == player_before_start && state.turn_phase == TurnPhase::TurnEnd
}

/// Advance to final round after completing current round.
pub fn check_round_completion(state: &mut GameState) {
    if !state.end_game_triggered || state.final_round {
        return;
    }

    // Current round ends when we reach the player before start player at TurnEnd
    let player_before_start = PlayerId::new(
        (state.config.start_player.0 + state.config.player_count - 1) % state.config.player_count,
    );

    if state.current_player == player_before_start && state.turn_phase == TurnPhase::TurnEnd {
        state.final_round = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_state() -> GameState {
        let config = GameConfig::new(
            2,
            vec![PlayerColor::Red, PlayerColor::Blue],
            PlayerId::new(0),
        );

        let board_layout = BoardLayout {
            areas: HashMap::new(),
            adjacency: HashMap::new(),
            canal_crossings: vec![],
            building_sites: HashMap::new(),
            five_power_tokens: HashMap::new(),
        };

        let players = vec![
            PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
            PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
        ];

        GameState {
            config,
            board_layout,
            players,
            current_player: PlayerId::new(0),
            turn_phase: TurnPhase::AreaAction,
            turn_number: 1,
            additional_action_state: AdditionalActionState::new(),
            current_action_power: None,
            is_mercenary_action: false,
            mercenary_target_area: None,
            mercenary_return_phase: None,
            port_state: PortState::new(false),
            lab_state: LabState::new(false),
            order_deck_remaining: 20,
            tech_deck_remaining: 20,
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
        }
    }

    #[test]
    fn test_track_trigger_by_player_count() {
        let config_2p = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));
        assert_eq!(config_2p.track_trigger(), 4);

        let config_3p = GameConfig::new(
            3,
            vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow],
            PlayerId::new(0),
        );
        assert_eq!(config_3p.track_trigger(), 5);
    }

    #[test]
    fn test_order_deck_empty_trigger() {
        let mut state = create_test_state();

        // Deck has cards, all port slots full - should NOT trigger
        state.order_deck_remaining = 1;
        state.port_state.port_a.low = Some(OrderCardId(1));
        state.port_state.port_a.mid = Some(OrderCardId(2));
        state.port_state.port_a.high = Some(OrderCardId(3));

        check_end_game_triggers(&mut state);
        assert!(!state.end_game_triggered);

        // Deck empty but all slots full - should NOT trigger
        state.order_deck_remaining = 0;
        check_end_game_triggers(&mut state);
        assert!(!state.end_game_triggered);

        // Deck empty AND slot empty (post-refill) - should trigger
        state.port_state.port_a.high = None; // Empty slot after failed refill
        check_end_game_triggers(&mut state);
        assert!(state.end_game_triggered);
        assert!(matches!(state.trigger_reason, Some(EndGameTrigger::OrderDeckEmpty)));
    }

    #[test]
    fn test_order_deck_empty_with_port_b() {
        let mut state = create_test_state();
        state.port_state.port_b = Some(PortBoard::new()); // Add Port B

        // Deck empty, Port A full, Port B has empty slot
        state.order_deck_remaining = 0;
        state.port_state.port_a.low = Some(OrderCardId(1));
        state.port_state.port_a.mid = Some(OrderCardId(2));
        state.port_state.port_a.high = Some(OrderCardId(3));
        // Port B starts empty (new())

        check_end_game_triggers(&mut state);
        assert!(state.end_game_triggered);
    }

    #[test]
    fn test_trading_house_complete_trigger() {
        let mut state = create_test_state();

        // Player builds 4 trading houses
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
    fn test_shophouse_complete_trigger() {
        let mut state = create_test_state();

        // Player builds 8 shophouses
        for i in 0..8 {
            state.players[0].shophouses.built.insert(AreaId::new(i), 0);
        }
        state.players[0].shophouses.in_hand = 0;

        check_end_game_triggers(&mut state);
        assert!(state.end_game_triggered);
        assert!(matches!(
            state.trigger_reason,
            Some(EndGameTrigger::ShophousesComplete(p)) if p == PlayerId::new(0)
        ));
    }
}
