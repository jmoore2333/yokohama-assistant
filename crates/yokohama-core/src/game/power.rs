//! Power calculation for area actions.

use crate::types::{AreaId, GameState, PlayerId, PresidentLocation};

/// Calculate power for a player in an area.
///
/// Power calculation order (per rulebook):
/// 1. President pawn (+1)
/// 2. Assistants (+1 each)
/// 3. Shophouse (+1)
/// 4. Trading House (+1)
/// 5. Station token (+1 for ALL players if present)
/// 6. Tech modifiers (future: if any tech adds power)
/// 7. Cap at 5 (ALWAYS applied last)
///
/// Note: All modifiers are applied BEFORE the cap.
pub fn calculate_power(state: &GameState, player: PlayerId, area: AreaId) -> u8 {
    let player_state = state.player(player);
    let mut power: u8 = 0;

    // 1. President pawn (+1)
    if player_state.president_location == PresidentLocation::OnBoard(area) {
        power += 1;
    }

    // 2. Assistants (+1 each)
    power += player_state.assistants.in_area(area);

    // 3. Shophouse (+1)
    if player_state.shophouses.built.contains_key(&area) {
        power += 1;
    }

    // 4. Trading House (+1)
    if player_state.trading_houses.built.contains(&area) {
        power += 1;
    }

    // 5. Station token (+1 for ALL players if present)
    if state.station_location == Some(area) {
        power += 1;
    }

    // 6. Tech modifiers would go here (none in base game add power)

    // 7. Cap at 5 (ALWAYS applied last, after all modifiers)
    power.min(5)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::collections::BTreeMap;

    fn create_test_state() -> GameState {
        let config = GameConfig::new(
            2,
            vec![PlayerColor::Red, PlayerColor::Blue],
            PlayerId::new(0),
        );

        let mut areas = BTreeMap::new();
        areas.insert(AreaId::new(0), AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });

        let board_layout = BoardLayout {
            areas,
            adjacency: BTreeMap::new(),
            canal_crossings: vec![],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
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
            turn_phase: TurnPhase::Placement,
            turn_number: 1,
            additional_action_state: AdditionalActionState::new(),
            current_action_power: None,
            is_mercenary_action: false,
            mercenary_target_area: None,
            mercenary_return_phase: None,
            president_start_area: None,
            port_state: PortState::new(false),
            lab_state: LabState::new(false),
            order_deck: vec![],
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
        }
    }

    #[test]
    fn test_power_president_only() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        let player = PlayerId::new(0);

        state.players[0].president_location = PresidentLocation::OnBoard(area);
        assert_eq!(calculate_power(&state, player, area), 1);
    }

    #[test]
    fn test_power_with_assistants() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        let player = PlayerId::new(0);

        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].assistants.on_board.insert(area, 3);
        assert_eq!(calculate_power(&state, player, area), 4);
    }

    #[test]
    fn test_power_capped_at_5() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        let player = PlayerId::new(0);

        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].assistants.on_board.insert(area, 5);
        state.players[0].shophouses.built.insert(area, 0);
        // President (1) + 5 assistants + 1 shophouse = 7, but capped at 5
        assert_eq!(calculate_power(&state, player, area), 5);
    }

    #[test]
    fn test_power_with_station() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        let player = PlayerId::new(0);

        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.station_location = Some(area);
        // President (1) + Station (1) = 2
        assert_eq!(calculate_power(&state, player, area), 2);
    }

    #[test]
    fn test_power_zero_no_presence() {
        let state = create_test_state();
        let area = AreaId::new(0);
        let player = PlayerId::new(0);
        // Player has no presence in area
        assert_eq!(calculate_power(&state, player, area), 0);
    }

    #[test]
    fn test_power_trading_house_bonus() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        let player = PlayerId::new(0);

        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].trading_houses.built.push(area);
        // President (1) + Trading House (1) = 2
        assert_eq!(calculate_power(&state, player, area), 2);
    }

    #[test]
    fn test_power_shophouse_bonus() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        let player = PlayerId::new(0);

        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].shophouses.built.insert(area, 0);
        // President (1) + Shophouse (1) = 2
        assert_eq!(calculate_power(&state, player, area), 2);
    }

    #[test]
    fn test_power_station_benefits_all_players() {
        let mut state = create_test_state();
        let area = AreaId::new(0);

        // Player 1 places the station and has president there
        state.players[1].president_location = PresidentLocation::OnBoard(area);
        state.station_location = Some(area);

        // Player 0 has only an assistant in the area
        state.players[0].assistants.on_board.insert(area, 1);

        // Player 0: Assistant (1) + Station (1) = 2
        assert_eq!(calculate_power(&state, PlayerId::new(0), area), 2);
        // Player 1: President (1) + Station (1) = 2
        assert_eq!(calculate_power(&state, PlayerId::new(1), area), 2);
    }

    #[test]
    fn test_power_all_sources_combined() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        let player = PlayerId::new(0);

        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].assistants.on_board.insert(area, 1);
        state.players[0].shophouses.built.insert(area, 0);
        state.players[0].trading_houses.built.push(area);
        state.station_location = Some(area);
        // President (1) + Assistant (1) + Shophouse (1) + Trading House (1) + Station (1) = 5
        assert_eq!(calculate_power(&state, player, area), 5);
    }
}
