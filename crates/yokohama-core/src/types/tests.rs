//! Unit tests for core types.

#[cfg(test)]
mod tests {
    use crate::types::*;

    #[test]
    fn test_player_id_creation() {
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);
        assert_ne!(p0, p1);
        assert_eq!(p0.0, 0);
    }

    #[test]
    fn test_player_id_try_new() {
        assert!(PlayerId::try_new(0).is_ok());
        assert!(PlayerId::try_new(3).is_ok());
        assert!(PlayerId::try_new(4).is_err());
        assert!(PlayerId::try_new(255).is_err());
    }

    #[test]
    fn test_area_id_try_new() {
        assert!(AreaId::try_new(0).is_ok());
        assert!(AreaId::try_new(17).is_ok());
        assert!(AreaId::try_new(18).is_err());
    }

    #[test]
    fn test_country_mercenary_supply() {
        assert_eq!(Country::English.mercenary_supply(), 3);
        assert_eq!(Country::Dutch.mercenary_supply(), 1);
    }

    #[test]
    fn test_area_type_categories() {
        assert!(AreaType::CopperMine.is_production());
        assert!(!AreaType::CopperMine.is_commercial());
        assert!(AreaType::Bank.is_commercial());
        assert!(!AreaType::Canal.allows_placement());
    }

    #[test]
    fn test_building_site_slot_allocation() {
        let mut site = BuildingSite::new([3, 1, 8, 4], 9);
        let player = PlayerId::new(0);

        // First slot available
        assert_eq!(site.next_shophouse_slot(player), Some(0));

        // Build in slot 0
        site.shophouses_built[0] = Some(player);

        // Player can't build again (one per area)
        assert_eq!(site.next_shophouse_slot(player), None);

        // Different player can build
        let player2 = PlayerId::new(1);
        assert_eq!(site.next_shophouse_slot(player2), Some(1));
    }

    #[test]
    fn test_assistant_state_totals() {
        let mut state = AssistantState::new();
        assert_eq!(state.total(), 20); // 8 hand + 12 warehouse

        state.on_board.insert(AreaId::new(0), 3);
        state.in_hand -= 3;
        assert_eq!(state.total(), 20); // Still 20 total
        assert_eq!(state.in_area(AreaId::new(0)), 3);
        assert_eq!(state.in_area(AreaId::new(1)), 0);
    }

    #[test]
    fn test_trading_house_costs() {
        let mut th = TradingHouseState::new();
        assert_eq!(th.next_cost(), 4);

        th.built.push(AreaId::new(0));
        assert_eq!(th.next_cost(), 5);

        th.built.push(AreaId::new(1));
        assert_eq!(th.next_cost(), 6);

        th.built.push(AreaId::new(2));
        assert_eq!(th.next_cost(), 7);
    }

    #[test]
    fn test_church_track_rewards() {
        assert_eq!(ChurchTrack::vp_reward(4), 3);
        assert_eq!(ChurchTrack::vp_reward(11), 14);
        assert_eq!(ChurchTrack::faith_to_index(4), Some(0));
        assert_eq!(ChurchTrack::faith_to_index(11), Some(7));
        assert_eq!(ChurchTrack::faith_to_index(3), None);
    }

    #[test]
    fn test_customs_track_rewards() {
        assert_eq!(CustomsTrack::vp_reward(1), 5);
        assert_eq!(CustomsTrack::vp_reward(7), 22);
        assert_eq!(CustomsTrack::vp_reward(10), 22); // 7+ all same
    }

    #[test]
    fn test_achievement_vp() {
        assert_eq!(AchievementType::Copper5.first_place_vp(), 8);
        assert_eq!(AchievementType::Copper5.subsequent_vp(), 6);
        assert_eq!(AchievementType::Money10.first_place_vp(), 9);
        assert_eq!(AchievementType::Production4.first_place_vp(), 10);
    }

    #[test]
    fn test_lab_position_modifier() {
        // Board art (asset-sweep-report.md Q4): 6 slots, surcharges +1,+1,+2,+2
        // on slots 3-6 (1-indexed) i.e. indices 2-5.
        assert_eq!(LabBoard::position_modifier(0), 0);
        assert_eq!(LabBoard::position_modifier(1), 0);
        assert_eq!(LabBoard::position_modifier(2), 1);
        assert_eq!(LabBoard::position_modifier(3), 1);
        assert_eq!(LabBoard::position_modifier(4), 2);
        assert_eq!(LabBoard::position_modifier(5), 2);
    }

    #[test]
    fn test_game_config_track_trigger() {
        let config_2p = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));
        assert_eq!(config_2p.track_trigger(), 4);

        let config_4p = GameConfig::new(
            4,
            vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green],
            PlayerId::new(0),
        );
        assert_eq!(config_4p.track_trigger(), 6);
    }

    #[test]
    fn test_player_state_initial_resources() {
        let start_player = PlayerState::new(PlayerId::new(0), PlayerColor::Red, true);
        assert_eq!(start_player.money, 3);

        let other_player = PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false);
        assert_eq!(other_player.money, 4);

        // Both have same goods
        assert_eq!(start_player.copper, 1);
        assert_eq!(other_player.copper, 1);
    }

    #[test]
    fn test_achievement_card_vp() {
        let mut card = AchievementCard::new(AchievementType::Copper5);
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);

        // No one claimed yet
        assert_eq!(card.vp_for_player(p0), 0);

        // First player claims
        card.claimed_by.push(p0);
        assert_eq!(card.vp_for_player(p0), 8); // First place

        // Second player claims
        card.claimed_by.push(p1);
        assert_eq!(card.vp_for_player(p1), 6); // Subsequent
    }

    #[test]
    fn test_mercenary_state() {
        let mut state = MercenaryState::new();
        assert_eq!(state.total(), 0);
        assert_eq!(state.unused_count(), 0);

        state.available.insert(Country::English, 2);
        state.used.insert(Country::French, 1);

        assert_eq!(state.total(), 3);
        assert_eq!(state.unused_count(), 2);
    }

    #[test]
    fn test_trade_good_requirement() {
        let req = TradeGoodRequirement::new(2, 1, 3, 0);
        assert_eq!(req.total(), 6);
    }

    #[test]
    fn test_game_config_removed_area_types() {
        let config_2p = GameConfig::for_player_count(2);
        let removed_2p = config_2p.removed_area_types();
        assert!(removed_2p.contains(&AreaType::LaboratoryB));
        assert!(removed_2p.contains(&AreaType::PortB));
        assert!(removed_2p.contains(&AreaType::Chinatown));
        assert!(removed_2p.contains(&AreaType::Canal));

        let config_3p = GameConfig::for_player_count(3);
        let removed_3p = config_3p.removed_area_types();
        assert!(removed_3p.contains(&AreaType::LaboratoryB));
        assert!(!removed_3p.contains(&AreaType::PortB)); // Port B is in 3p

        let config_4p = GameConfig::for_player_count(4);
        let removed_4p = config_4p.removed_area_types();
        assert!(removed_4p.is_empty()); // Nothing removed for 4p
    }

    #[test]
    fn test_board_layout_builder_rejects_invalid_areas_for_player_count() {
        let config_2p = GameConfig::for_player_count(2);
        let config_4p = GameConfig::for_player_count(4);

        // Create a builder with a LaboratoryB area (invalid for 2p)
        let mut builder = BoardLayoutBuilder::new();
        builder.add_area(
            AreaId::new(0),
            AreaConfig {
                area_type: AreaType::LaboratoryB,
                grid_position: (0, 0),
            },
        );
        builder.set_building_site(AreaId::new(0), BuildingSite::new([2, 3, 4, 5], 8));
        builder.set_five_power_token(AreaId::new(0), FivePowerEffect::VictoryPoints(3));

        // build_for_config() should fail for 2p (LaboratoryB is removed)
        let mut builder_copy = BoardLayoutBuilder::new();
        builder_copy.add_area(
            AreaId::new(0),
            AreaConfig {
                area_type: AreaType::LaboratoryB,
                grid_position: (0, 0),
            },
        );
        builder_copy.set_building_site(AreaId::new(0), BuildingSite::new([2, 3, 4, 5], 8));
        builder_copy.set_five_power_token(AreaId::new(0), FivePowerEffect::VictoryPoints(3));

        let result = builder_copy.build_for_config(&config_2p);
        assert!(matches!(
            result,
            Err(BoardLayoutError::InvalidAreaForPlayerCount(_, AreaType::LaboratoryB, 2))
        ));

        // build_for_config() should succeed for 4p (all areas allowed)
        let result = builder.build_for_config(&config_4p);
        assert!(result.is_ok());
    }
}
