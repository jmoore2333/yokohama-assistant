//! Legal move generation for each turn phase.

use crate::types::*;
use crate::game::*;

/// Generate all legal moves for the current game state.
pub fn get_legal_moves(state: &GameState) -> Vec<Move> {
    match state.turn_phase {
        TurnPhase::AdditionalActionFirst | TurnPhase::AdditionalActionSecond => {
            generate_additional_action_moves(state)
        }
        TurnPhase::Placement => generate_placement_moves(state),
        TurnPhase::Movement => generate_movement_moves(state),
        TurnPhase::AreaAction => generate_area_action_moves(state),
        TurnPhase::FivePowerBonus => generate_five_power_moves(state),
        TurnPhase::Building => generate_building_moves(state),
        TurnPhase::Recovery => vec![], // Automatic, no moves
        TurnPhase::TurnEnd => vec![],
    }
}

fn generate_additional_action_moves(state: &GameState) -> Vec<Move> {
    let mut moves = Vec::new();
    let player = state.current_player;
    let p = state.player(player);

    // Always can end the phase
    moves.push(Move::EndAdditionalPhase);

    // Foreign Mercenaries (once per turn)
    if !state.additional_action_state.mercenary_used {
        for (&country, &count) in &p.mercenaries.available {
            if count > 0 {
                // Find valid target areas
                for (&area, &assistant_count) in &p.assistants.on_board {
                    if assistant_count > 0 {
                        // Must not have our president
                        let has_our_president =
                            p.president_location == PresidentLocation::OnBoard(area);
                        if !has_our_president {
                            moves.push(Move::UseMercenary {
                                country,
                                target_area: area,
                            });
                        }
                    }
                }
            }
        }
    }

    // Fulfill Orders
    for &order_id in &p.order_cards_in_hand {
        // Would need to check if requirements met
        // Simplified: always offer the option, validation happens on execution
        moves.push(Move::FulfillOrder { order_id });
    }

    // Fulfill Achievements
    for achievement in &state.achievements {
        if !achievement.claimed_by.contains(&player) {
            moves.push(Move::FulfillAchievement {
                achievement: achievement.achievement_type,
            });
        }
    }

    moves
}

fn generate_placement_moves(state: &GameState) -> Vec<Move> {
    let mut moves = Vec::new();
    let player = state.current_player;
    let p = state.player(player);

    if p.assistants.in_hand == 0 {
        // Must place 0 (degenerate case)
        moves.push(Move::PlaceThreeAreas { placements: vec![] });
        return moves;
    }

    // Get all placeable areas (not Canal)
    let placeable_areas: Vec<AreaId> = state
        .board_layout
        .areas
        .iter()
        .filter(|(_, config)| config.area_type.allows_placement())
        .map(|(&id, _)| id)
        .collect();

    // Option A: 1 assistant in each of 1-3 different areas
    // We generate combinations
    for i in 0..=placeable_areas.len().min(3).min(p.assistants.in_hand as usize) {
        if i == 0 {
            moves.push(Move::PlaceThreeAreas { placements: vec![] });
        } else if i == 1 {
            for &area in &placeable_areas {
                moves.push(Move::PlaceThreeAreas {
                    placements: vec![area],
                });
            }
        } else if i == 2 && p.assistants.in_hand >= 2 {
            for (j, &area1) in placeable_areas.iter().enumerate() {
                for &area2 in &placeable_areas[j + 1..] {
                    moves.push(Move::PlaceThreeAreas {
                        placements: vec![area1, area2],
                    });
                }
            }
        } else if i == 3 && p.assistants.in_hand >= 3 {
            for (j, &area1) in placeable_areas.iter().enumerate() {
                for (k, &area2) in placeable_areas[j + 1..].iter().enumerate() {
                    for &area3 in &placeable_areas[j + k + 2..] {
                        moves.push(Move::PlaceThreeAreas {
                            placements: vec![area1, area2, area3],
                        });
                    }
                }
            }
        }
    }

    // Option B: 2 assistants in 1 area (if have at least 2)
    if p.assistants.in_hand >= 2 {
        for &area in &placeable_areas {
            moves.push(Move::PlaceTwoSameArea { area });
        }
    }

    moves
}

fn generate_movement_moves(state: &GameState) -> Vec<Move> {
    let mut moves = Vec::new();
    let player = state.current_player;
    let p = state.player(player);

    match p.president_location {
        PresidentLocation::InHand => {
            // Place president on any Possible Action Area
            for &area in state.board_layout.areas.keys() {
                if is_possible_action_area(state, player, area) {
                    moves.push(Move::PlacePresident { destination: area });
                }
            }
        }
        PresidentLocation::OnBoard(current) => {
            // Move to reachable areas
            for (dest, path) in find_reachable_areas(state, player, current) {
                moves.push(Move::MovePresident {
                    path,
                    destination: dest,
                });
            }

            // Return to hand (emergency exit, always available)
            let areas_with_assistants: Vec<AreaId> = p
                .assistants
                .on_board
                .iter()
                .filter(|(_, &count)| count > 0)
                .map(|(&area, _)| area)
                .collect();

            moves.push(Move::ReturnPresidentToHand {
                recover_assistants_from: areas_with_assistants,
            });
        }
    }

    moves
}

fn generate_area_action_moves(state: &GameState) -> Vec<Move> {
    // This is complex - depends on area type and player resources
    // Simplified: return single action based on area type
    let player = state.current_player;
    let p = state.player(player);

    // For mercenary sub-turns, use the mercenary target area instead of president location
    let area = if state.is_mercenary_action {
        match state.mercenary_target_area {
            Some(a) => a,
            None => return vec![], // Should never happen if state is consistent
        }
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => a,
            PresidentLocation::InHand => return vec![],
        }
    };

    let area_config = match state.board_layout.areas.get(&area) {
        Some(c) => c,
        None => return vec![],
    };

    vec![generate_area_action_for_type(area_config.area_type)]
}

fn generate_area_action_for_type(area_type: AreaType) -> Move {
    // Simplified: return default action for area type
    let action = match area_type {
        AreaType::CopperMine => AreaAction::ProduceCopper,
        AreaType::SilkMill => AreaAction::ProduceSilk,
        AreaType::TeaPlantation => AreaAction::ProduceTea,
        AreaType::FishingGround => AreaAction::ProduceSeafood,
        AreaType::Bank => AreaAction::Bank,
        AreaType::PortA | AreaType::PortB => AreaAction::Port {
            cards_taken: vec![PortSlot::Low],
            second_card_payment: None,
        },
        AreaType::LaboratoryA | AreaType::LaboratoryB => AreaAction::Laboratory {
            card_id: TechCardId(0), // Placeholder
            extra_yen: 0,
            extra_imported: 0,
            fifth_tech_payment: None,
        },
        AreaType::EmploymentAgency => AreaAction::EmploymentAgency {
            assistants: 0,
            shophouses: 0,
            trading_houses: 0,
        },
        AreaType::Chinatown => AreaAction::Chinatown { exchanges: vec![] },
        AreaType::Church => AreaAction::Church {
            copper: false,
            silk: false,
            tea: false,
            seafood: false,
            yen: false,
            imported_good: false,
        },
        AreaType::Customs => AreaAction::Customs { imported_goods_paid: 1 },
        AreaType::Canal => AreaAction::Bank, // Should never happen
    };

    Move::ExecuteAreaAction { action }
}

fn generate_five_power_moves(state: &GameState) -> Vec<Move> {
    let player = state.current_player;
    let p = state.player(player);

    // Check if power was 5 and token is still there
    if state.current_action_power == Some(5) {
        // For mercenary sub-turns, use the mercenary target area
        let area = if state.is_mercenary_action {
            state.mercenary_target_area
        } else {
            match p.president_location {
                PresidentLocation::OnBoard(a) => Some(a),
                PresidentLocation::InHand => None,
            }
        };

        if let Some(area) = area {
            if let Some(Some(_)) = state.board_layout.five_power_tokens.get(&area) {
                return vec![Move::Take5PowerToken, Move::Skip5PowerBonus];
            }
        }
    }

    vec![Move::Skip5PowerBonus]
}

fn generate_building_moves(state: &GameState) -> Vec<Move> {
    let mut moves = vec![Move::SkipBuilding];
    let player = state.current_player;
    let p = state.player(player);

    // Need 4+ power action
    if state.current_action_power.unwrap_or(0) < 4 {
        return moves;
    }

    // For mercenary sub-turns, use the mercenary target area instead of president location
    let area = if state.is_mercenary_action {
        match state.mercenary_target_area {
            Some(a) => a,
            None => return moves,
        }
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => a,
            PresidentLocation::InHand => return moves,
        }
    };

    if let Some(site) = state.board_layout.building_sites.get(&area) {
        // Shophouse
        if p.shophouses.in_hand > 0 {
            if let Some(slot) = site.next_shophouse_slot(player) {
                moves.push(Move::BuildShophouse { slot: slot as u8 });
            }
        }

        // Trading House
        if p.trading_houses.in_warehouse > 0 && site.can_build_trading_house() {
            moves.push(Move::BuildTradingHouse);
        }
    }

    moves
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

        let mut areas = HashMap::new();
        areas.insert(AreaId::new(0), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(AreaId::new(1), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });

        let board_layout = BoardLayout {
            areas,
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
            turn_phase: TurnPhase::Placement,
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
    fn test_placement_moves_with_assistants() {
        let state = create_test_state();
        let moves = generate_placement_moves(&state);

        // Should have placement options
        assert!(!moves.is_empty());

        // Should include empty placement (0 assistants)
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements } if placements.is_empty())));

        // Should include single area placements
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements } if placements.len() == 1)));

        // Should include Option B (2 in same area) since we have 8 assistants
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceTwoSameArea { .. })));
    }

    #[test]
    fn test_additional_action_always_has_end() {
        let mut state = create_test_state();
        state.turn_phase = TurnPhase::AdditionalActionFirst;

        let moves = get_legal_moves(&state);

        // Should always include EndAdditionalPhase
        assert!(moves.iter().any(|m| matches!(m, Move::EndAdditionalPhase)));
    }

    #[test]
    fn test_building_requires_power_4() {
        let mut state = create_test_state();
        state.turn_phase = TurnPhase::Building;
        state.current_action_power = Some(3); // Not enough

        let moves = get_legal_moves(&state);

        // Should only have Skip
        assert_eq!(moves.len(), 1);
        assert!(matches!(moves[0], Move::SkipBuilding));
    }
}
