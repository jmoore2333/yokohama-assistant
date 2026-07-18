//! Achievement card validation and fulfillment.

use crate::types::*;

/// Check if player can claim an Achievement.
pub fn can_claim_achievement(
    state: &GameState,
    player: PlayerId,
    achievement: AchievementType,
) -> bool {
    let p = state.player(player);

    // Check if already claimed by this player
    for card in &state.achievements {
        if card.achievement_type == achievement && card.claimed_by.contains(&player) {
            return false;
        }
    }

    // Check if player has assistant in hand (required to place)
    if p.assistants.in_hand < 1 {
        return false;
    }

    // Check specific achievement requirements
    match achievement {
        // Category A: Trade good holdings (no payment, just demonstrate ownership)
        AchievementType::Copper5 => p.copper >= 5,
        AchievementType::Silk6 => p.silk >= 6,
        AchievementType::Tea7 => p.tea >= 7,
        AchievementType::Seafood7 => p.seafood >= 7,

        // Category B: Collection goals
        AchievementType::Money10 => p.money >= 10,
        AchievementType::Tokens5 => {
            p.five_power_tokens + p.mercenaries.total() >= 5
        }
        AchievementType::Orders5 => p.completed_orders.len() >= 5,
        AchievementType::Technologies4 => p.technologies.len() >= 4,

        // Category C: Building presence
        AchievementType::Production4 => {
            count_production_buildings(state, player) >= 4
        }
        AchievementType::Commercial4 => {
            count_commercial_buildings(state, player) >= 4
        }
        AchievementType::Prod3Comm2 => {
            count_production_buildings(state, player) >= 3
                && count_commercial_buildings(state, player) >= 2
        }
        AchievementType::Prod2Comm3 => {
            count_production_buildings(state, player) >= 2
                && count_commercial_buildings(state, player) >= 3
        }
    }
}

/// Count unique production areas where player has a building.
fn count_production_buildings(state: &GameState, player: PlayerId) -> usize {
    let p = state.player(player);
    let mut count = 0;

    for &area_id in p.shophouses.built.keys() {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_production() {
                count += 1;
            }
        }
    }

    for &area_id in &p.trading_houses.built {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_production()
                && !p.shophouses.built.contains_key(&area_id)
            {
                // Don't double-count if also have shophouse
                count += 1;
            }
        }
    }

    count
}

/// Count unique commercial areas where player has a building.
fn count_commercial_buildings(state: &GameState, player: PlayerId) -> usize {
    let p = state.player(player);
    let mut count = 0;

    for &area_id in p.shophouses.built.keys() {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_commercial() {
                count += 1;
            }
        }
    }

    for &area_id in &p.trading_houses.built {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_commercial()
                && !p.shophouses.built.contains_key(&area_id)
            {
                count += 1;
            }
        }
    }

    count
}

/// Claim an Achievement.
pub fn claim_achievement(
    state: &mut GameState,
    player: PlayerId,
    achievement: AchievementType,
) -> Result<u32, &'static str> {
    if !can_claim_achievement(state, player, achievement) {
        return Err("Cannot claim achievement: requirements not met");
    }

    // Find the achievement card and add player to claimed list
    let mut vp_awarded = 0u32;
    for card in &mut state.achievements {
        if card.achievement_type == achievement {
            let is_first = card.claimed_by.is_empty();
            card.claimed_by.push(player);
            vp_awarded = if is_first {
                achievement.first_place_vp() as u32
            } else {
                achievement.subsequent_vp() as u32
            };
            break;
        }
    }

    // Lock an assistant on the achievement
    let p = state.player_mut(player);
    p.assistants.in_hand -= 1;
    p.assistants.on_achievements += 1;
    p.victory_points += vp_awarded;

    Ok(vp_awarded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_achievement_first_vs_subsequent() {
        assert_eq!(AchievementType::Copper5.first_place_vp(), 8);
        assert_eq!(AchievementType::Copper5.subsequent_vp(), 6);
    }

    fn create_test_state() -> GameState {
        let config = GameConfig::new(
            2,
            vec![PlayerColor::Red, PlayerColor::Blue],
            PlayerId::new(0),
        );

        let board_layout = BoardLayout {
            areas: BTreeMap::new(),
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

    /// Jeremy-adjudicated 2026-07-18: Category-C achievements ("Buildings in
    /// N distinct areas") count Shophouses AND Trading Houses the same way -
    /// pins that `count_production_buildings`/`count_commercial_buildings`
    /// already tally both (they did before this pin was added; this guards
    /// against a future regression, e.g. someone "simplifying" away the
    /// trading_houses.built loop).
    #[test]
    fn test_count_production_buildings_counts_trading_houses_too() {
        let mut state = create_test_state();
        let mine_area = AreaId::new(0);
        let mill_area = AreaId::new(1);
        state.board_layout.areas.insert(mine_area, AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 0),
        });
        state.board_layout.areas.insert(mill_area, AreaConfig {
            area_type: AreaType::SilkMill,
            grid_position: (0, 1),
        });
        // A Shophouse in one production area, a Trading House (no
        // Shophouse) in another distinct production area: both must count.
        state.players[0].shophouses.built.insert(mine_area, 0);
        state.players[0].trading_houses.built.push(mill_area);

        assert_eq!(count_production_buildings(&state, PlayerId::new(0)), 2);
    }

    #[test]
    fn test_count_commercial_buildings_counts_trading_houses_too() {
        let mut state = create_test_state();
        let bank_area = AreaId::new(0);
        let port_area = AreaId::new(1);
        state.board_layout.areas.insert(bank_area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.board_layout.areas.insert(port_area, AreaConfig {
            area_type: AreaType::PortA,
            grid_position: (0, 1),
        });
        state.players[0].shophouses.built.insert(bank_area, 0);
        state.players[0].trading_houses.built.push(port_area);

        assert_eq!(count_commercial_buildings(&state, PlayerId::new(0)), 2);
    }

    #[test]
    fn test_count_production_buildings_does_not_double_count_area_with_both() {
        // A single area holding BOTH a Shophouse and a Trading House is one
        // distinct area, not two - the achievement counts distinct AREAS,
        // not distinct buildings.
        let mut state = create_test_state();
        let mine_area = AreaId::new(0);
        state.board_layout.areas.insert(mine_area, AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 0),
        });
        state.players[0].shophouses.built.insert(mine_area, 0);
        state.players[0].trading_houses.built.push(mine_area);

        assert_eq!(count_production_buildings(&state, PlayerId::new(0)), 1);
    }

    #[test]
    fn test_production4_achievement_claimable_via_trading_houses_alone() {
        // End-to-end through can_claim_achievement: 4 Trading Houses (no
        // Shophouses at all) in 4 distinct production areas must satisfy
        // Production4, exactly as 4 Shophouses would.
        let mut state = create_test_state();
        state.players[0].assistants.in_hand = 1;
        for i in 0..4u8 {
            let area = AreaId::new(i);
            state.board_layout.areas.insert(area, AreaConfig {
                area_type: AreaType::CopperMine,
                grid_position: (0, i),
            });
            state.players[0].trading_houses.built.push(area);
        }

        assert!(can_claim_achievement(&state, PlayerId::new(0), AchievementType::Production4));
    }
}
