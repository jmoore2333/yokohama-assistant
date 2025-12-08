//! Game setup configuration and initialization.

use crate::types::*;
use std::collections::HashMap;

/// Create initial player states for a game configuration.
pub fn create_players(config: &GameConfig) -> Vec<PlayerState> {
    (0..config.player_count)
        .map(|i| {
            let player_id = PlayerId::new(i);
            let is_start = player_id == config.start_player;
            let mut player = PlayerState::new(player_id, config.player_colors[i as usize], is_start);
            // Set starting money based on turn order
            player.money = config.starting_money(player_id);
            player
        })
        .collect()
}

/// Create port state based on game configuration.
pub fn create_port_state(config: &GameConfig) -> PortState {
    PortState::new(config.has_port_b)
}

/// Create lab state based on game configuration.
pub fn create_lab_state(config: &GameConfig) -> LabState {
    LabState::new(config.has_lab_b)
}

/// Create a minimal test game state for the given player count.
/// This is primarily for testing purposes.
pub fn create_test_game_state(player_count: u8) -> GameState {
    let config = GameConfig::for_player_count(player_count);
    let players = create_players(&config);

    GameState {
        config: config.clone(),
        board_layout: BoardLayout {
            areas: HashMap::new(),
            adjacency: HashMap::new(),
            canal_crossings: vec![],
            building_sites: HashMap::new(),
            five_power_tokens: HashMap::new(),
        },
        players,
        current_player: config.start_player,
        turn_phase: TurnPhase::AdditionalActionFirst,
        turn_number: 1,
        additional_action_state: AdditionalActionState::new(),
        current_action_power: None,
        is_mercenary_action: false,
        mercenary_target_area: None,
        mercenary_return_phase: None,
        port_state: create_port_state(&config),
        lab_state: create_lab_state(&config),
        order_deck_remaining: 24 - config.order_cards_to_remove(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_2_player_config() {
        let config = GameConfig::for_player_count(2);

        assert_eq!(config.player_count, 2);
        assert!(!config.has_port_b);
        assert!(!config.has_lab_b);
        assert_eq!(config.track_trigger(), 4);
        assert_eq!(config.order_cards_to_remove(), 10);
        assert_eq!(config.inactive_area_count(), 5);

        // Starting money
        assert_eq!(config.starting_money(PlayerId::new(0)), 7); // Start player
        assert_eq!(config.starting_money(PlayerId::new(1)), 8); // Other player
    }

    #[test]
    fn test_3_player_config() {
        let config = GameConfig::for_player_count(3);

        assert_eq!(config.player_count, 3);
        assert!(config.has_port_b);
        assert!(!config.has_lab_b);
        assert_eq!(config.track_trigger(), 5);
        assert_eq!(config.order_cards_to_remove(), 5);
        assert_eq!(config.inactive_area_count(), 3);
    }

    #[test]
    fn test_4_player_config() {
        let config = GameConfig::for_player_count(4);

        assert_eq!(config.player_count, 4);
        assert!(config.has_port_b);
        assert!(config.has_lab_b);
        assert_eq!(config.track_trigger(), 6);
        assert_eq!(config.order_cards_to_remove(), 0);
        assert_eq!(config.inactive_area_count(), 0);
    }

    #[test]
    fn test_create_players() {
        let config = GameConfig::for_player_count(3);
        let players = create_players(&config);

        assert_eq!(players.len(), 3);

        // Start player (player 0) gets 7 Yen
        assert_eq!(players[0].money, 7);
        assert_eq!(players[0].color, PlayerColor::Red);

        // Other players get 8 Yen
        assert_eq!(players[1].money, 8);
        assert_eq!(players[1].color, PlayerColor::Blue);
        assert_eq!(players[2].money, 8);
        assert_eq!(players[2].color, PlayerColor::Yellow);
    }

    #[test]
    fn test_port_state_by_player_count() {
        // 2 players: no Port B
        let config_2p = GameConfig::for_player_count(2);
        let port_2p = create_port_state(&config_2p);
        assert!(port_2p.port_b.is_none());

        // 3 players: has Port B
        let config_3p = GameConfig::for_player_count(3);
        let port_3p = create_port_state(&config_3p);
        assert!(port_3p.port_b.is_some());

        // 4 players: has Port B
        let config_4p = GameConfig::for_player_count(4);
        let port_4p = create_port_state(&config_4p);
        assert!(port_4p.port_b.is_some());
    }

    #[test]
    fn test_lab_state_by_player_count() {
        // 2 players: no Lab B
        let config_2p = GameConfig::for_player_count(2);
        let lab_2p = create_lab_state(&config_2p);
        assert!(lab_2p.lab_b.is_none());

        // 3 players: no Lab B
        let config_3p = GameConfig::for_player_count(3);
        let lab_3p = create_lab_state(&config_3p);
        assert!(lab_3p.lab_b.is_none());

        // 4 players: has Lab B
        let config_4p = GameConfig::for_player_count(4);
        let lab_4p = create_lab_state(&config_4p);
        assert!(lab_4p.lab_b.is_some());
    }

    #[test]
    fn test_create_test_game_state() {
        // 2-player game
        let state_2p = create_test_game_state(2);
        assert_eq!(state_2p.players.len(), 2);
        assert!(state_2p.port_state.port_b.is_none());
        assert!(state_2p.lab_state.lab_b.is_none());
        assert_eq!(state_2p.order_deck_remaining, 14); // 24 - 10

        // 4-player game
        let state_4p = create_test_game_state(4);
        assert_eq!(state_4p.players.len(), 4);
        assert!(state_4p.port_state.port_b.is_some());
        assert!(state_4p.lab_state.lab_b.is_some());
        assert_eq!(state_4p.order_deck_remaining, 24); // 24 - 0
    }

    #[test]
    fn test_start_player_rotation() {
        // Test that non-zero start player gets correct money
        let config = GameConfig::new(
            3,
            vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow],
            PlayerId::new(1), // Player 1 starts
        );

        let players = create_players(&config);

        assert_eq!(players[0].money, 8); // Not start player
        assert_eq!(players[1].money, 7); // Start player
        assert_eq!(players[2].money, 8); // Not start player
    }
}
