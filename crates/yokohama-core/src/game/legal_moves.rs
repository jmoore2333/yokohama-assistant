//! Legal move generation for each turn phase.

use crate::types::*;
use crate::game::*;
use crate::game::actions::*;

/// Generate all legal moves for the current game state.
pub fn get_legal_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    match state.turn_phase {
        TurnPhase::AdditionalActionFirst | TurnPhase::AdditionalActionSecond => {
            generate_additional_action_moves(state)
        }
        TurnPhase::Placement => generate_placement_moves_with_tech(state, Some(&rules.techs)),
        TurnPhase::Movement => generate_movement_moves_with_tech(state, Some(&rules.techs)),
        TurnPhase::AreaAction => generate_area_action_moves(state, rules),
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

/// Calculate placement cost for a set of areas.
/// `placements` lists one entry per assistant placed (distinct areas for
/// Option A; a single-element slice per area-toll call for Option B/Telegram
/// bonus). KB (01-rules-reference.md:97): "If any President pawn(s) occupy
/// the target area, pay 1 Yen to each President's owner per assistant
/// placed" — so cost sums 1 Yen per (area, opponent-president) pair. Two
/// different opponents can both have a President in the same area (nothing
/// forces either to leave when the other arrives), in which case both are
/// owed a fee. Callers that credit these fees to owners (`apply_move.rs`'s
/// `credit_placement_fees`) must mirror this exact pairing so the payer's
/// deduction and the sum of owner credits always match.
pub(crate) fn calculate_placement_cost(
    state: &GameState,
    player: PlayerId,
    placements: &[AreaId],
) -> u32 {
    let mut cost = 0;
    for &area in placements {
        for other in &state.players {
            if other.id != player && other.president_location == PresidentLocation::OnBoard(area) {
                cost += 1;
            }
        }
    }
    cost
}

fn generate_placement_moves_with_tech(state: &GameState, tech_registry: Option<&TechRegistry>) -> Vec<Move> {
    let mut moves = Vec::new();
    let player = state.current_player;
    let p = state.player(player);
    let player_money = p.money;

    if p.assistants.in_hand == 0 {
        // Must place 0 (degenerate case)
        moves.push(Move::PlaceThreeAreas { placements: vec![] });
        return moves;
    }

    // Check for Newspaper technology (allows 4 areas instead of 3)
    let has_newspaper = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::Newspaper).unwrap_or(false)
        })
    });
    let max_areas = if has_newspaper { 4 } else { 3 };

    // Get all placeable areas (not Canal)
    let placeable_areas: Vec<AreaId> = state
        .board_layout
        .areas
        .iter()
        .filter(|(_, config)| config.area_type.allows_placement())
        .map(|(&id, _)| id)
        .collect();

    // Option A: 1 assistant in each of 1-N different areas (N = 3 or 4 with Newspaper)
    // Note: Placement costs 1 Yen per area with opponent President
    for i in 0..=placeable_areas.len().min(max_areas).min(p.assistants.in_hand as usize) {
        if i == 0 {
            // Empty placement is always free
            moves.push(Move::PlaceThreeAreas { placements: vec![] });
        } else if i == 1 {
            for &area in &placeable_areas {
                let placements = vec![area];
                let cost = calculate_placement_cost(state, player, &placements);
                if player_money >= cost {
                    moves.push(Move::PlaceThreeAreas { placements });
                }
            }
        } else if i == 2 && p.assistants.in_hand >= 2 {
            for (j, &area1) in placeable_areas.iter().enumerate() {
                for &area2 in &placeable_areas[j + 1..] {
                    let placements = vec![area1, area2];
                    let cost = calculate_placement_cost(state, player, &placements);
                    if player_money >= cost {
                        moves.push(Move::PlaceThreeAreas { placements });
                    }
                }
            }
        } else if i == 3 && p.assistants.in_hand >= 3 {
            for (j, &area1) in placeable_areas.iter().enumerate() {
                for (k, &area2) in placeable_areas[j + 1..].iter().enumerate() {
                    for &area3 in &placeable_areas[j + k + 2..] {
                        let placements = vec![area1, area2, area3];
                        let cost = calculate_placement_cost(state, player, &placements);
                        if player_money >= cost {
                            moves.push(Move::PlaceThreeAreas { placements });
                        }
                    }
                }
            }
        } else if i == 4 && p.assistants.in_hand >= 4 && has_newspaper {
            // 4-area placement with Newspaper
            for (j, &area1) in placeable_areas.iter().enumerate() {
                for (k, &area2) in placeable_areas[j + 1..].iter().enumerate() {
                    for (l, &area3) in placeable_areas[j + k + 2..].iter().enumerate() {
                        for &area4 in &placeable_areas[j + k + l + 3..] {
                            let placements = vec![area1, area2, area3, area4];
                            let cost = calculate_placement_cost(state, player, &placements);
                            if player_money >= cost {
                                moves.push(Move::PlaceThreeAreas { placements });
                            }
                        }
                    }
                }
            }
        }
    }

    // Option B: 2 assistants in 1 area (if have at least 2)
    // Cost is 1 Yen if opponent President is in the area
    // With Telegram tech and 3+ assistants, may also place 1 in adjacent area
    let has_telegram = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::Telegram).unwrap_or(false)
        })
    });

    if p.assistants.in_hand >= 2 {
        for &area in &placeable_areas {
            let cost = calculate_placement_cost(state, player, &[area]);
            if player_money >= cost {
                // Basic move without Telegram bonus
                moves.push(Move::PlaceTwoSameArea { area, adjacent_bonus: None });

                // With Telegram and 3+ assistants, offer adjacent placements
                if has_telegram && p.assistants.in_hand >= 3 {
                    for &adj_area in state.board_layout.adjacent_areas(area) {
                        // Adjacent area must be placeable
                        if let Some(adj_config) = state.board_layout.areas.get(&adj_area) {
                            if adj_config.area_type.allows_placement() {
                                // Check cost for adjacent area too
                                let adj_cost = calculate_placement_cost(state, player, &[adj_area]);
                                if player_money >= cost + adj_cost {
                                    moves.push(Move::PlaceTwoSameArea {
                                        area,
                                        adjacent_bonus: Some(adj_area),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    moves
}

fn generate_movement_moves_with_tech(state: &GameState, tech_registry: Option<&TechRegistry>) -> Vec<Move> {
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

            // Station teleport (free move to station area if player has Station tech)
            if let Some(station_area) = can_teleport_to_station_with_tech(state, player, tech_registry) {
                // Only add if not already at station and not already in moves
                if station_area != current && !moves.iter().any(|m| matches!(m, Move::MovePresident { destination, .. } if *destination == station_area)) {
                    moves.push(Move::MovePresident {
                        path: vec![], // Free teleport, no path needed
                        destination: station_area,
                    });
                }
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

fn generate_area_action_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
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

    let power = state.current_action_power.unwrap_or(0);

    // Port and Laboratory need the actual board contents (which slots are
    // filled, and by which A/B board) to enumerate real moves, so they're
    // resolved here rather than by the type-only placeholder dispatch below.
    match area_config.area_type {
        AreaType::PortA => generate_port_actions(state, player, power, &state.port_state.port_a),
        AreaType::PortB => match &state.port_state.port_b {
            Some(port) => generate_port_actions(state, player, power, port),
            None => vec![], // Should never happen if state is consistent
        },
        AreaType::LaboratoryA => {
            generate_lab_actions(state, player, power, &state.lab_state.lab_a, rules)
        }
        AreaType::LaboratoryB => match &state.lab_state.lab_b {
            Some(lab) => generate_lab_actions(state, player, power, lab, rules),
            None => vec![], // Should never happen if state is consistent
        },
        other => vec![generate_area_action_for_type(other)],
    }
}

/// Enumerate legal Port moves at `power` against the resolved acting board
/// (Port A or B -- the caller has already resolved which one from the
/// acting area, mirroring `execute_port_action`'s own resolution). Accessible
/// cards are the `Some` slots within power-accessible zones. Singles need
/// hand length + 1 <= 3; pairs need hand length + 2 <= 3 and offer one move
/// per affordable payment (TwoYen iff money >= 2, OneImportedGood iff
/// imported goods >= 1). Skip is always legal and always offered (Task 16 /
/// decision-log optionality ruling).
fn generate_port_actions(state: &GameState, player: PlayerId, power: u8, port: &PortBoard) -> Vec<Move> {
    let p = state.player(player);
    let hand_len = p.order_cards_in_hand.len();
    let accessible_zones = port_accessible_zones(power);

    // Accessible cards, in board slot order (canonical order for pairs below).
    let accessible: Vec<OrderCardId> = port.slots.iter().enumerate()
        .filter_map(|(i, slot)| {
            if accessible_zones.contains(&PortBoard::zone_of(i)) { *slot } else { None }
        })
        .collect();

    let mut moves = Vec::new();

    if hand_len < 3 {
        for &card in &accessible {
            moves.push(Move::ExecuteAreaAction {
                action: AreaAction::Port { cards_taken: vec![card], second_card_payment: None },
            });
        }
    }

    if hand_len + 2 <= 3 {
        for i in 0..accessible.len() {
            let first = accessible[i];
            for &second in &accessible[i + 1..] {
                if p.money >= 2 {
                    moves.push(Move::ExecuteAreaAction {
                        action: AreaAction::Port {
                            cards_taken: vec![first, second],
                            second_card_payment: Some(PortPayment::TwoYen),
                        },
                    });
                }
                if p.imported_goods >= 1 {
                    moves.push(Move::ExecuteAreaAction {
                        action: AreaAction::Port {
                            cards_taken: vec![first, second],
                            second_card_payment: Some(PortPayment::OneImportedGood),
                        },
                    });
                }
            }
        }
    }

    moves.push(Move::ExecuteAreaAction { action: AreaAction::Skip });
    moves
}

/// Enumerate legal Laboratory moves at `power` against the resolved acting
/// board (Lab A or B -- mirrors `execute_lab_action`'s own resolution). For
/// each filled slot, emits every exact-cover payment mix for the Industry
/// Value deficit (no overpays: `extra_imported = k`, `extra_yen =
/// 2*(deficit-k)` for every affordable `k` in `0..=deficit`), forking into
/// affordable `fifth_tech_payment` variants once the player owns 4+
/// technologies (a mix with neither payment affordable emits nothing). Skip
/// is always legal and always offered (Task 16 / decision-log optionality
/// ruling).
fn generate_lab_actions(state: &GameState, player: PlayerId, power: u8, lab: &LabBoard, rules: &Rules) -> Vec<Move> {
    let p = state.player(player);
    let needs_fifth_payment = p.technologies.len() >= 4;

    let mut moves = Vec::new();

    for (slot_index, slot) in lab.slots.iter().enumerate() {
        let Some(card_id) = *slot else { continue };
        let Some(tech_card) = rules.techs.get(card_id) else { continue };

        let required = tech_card.industry_cost + lab_position_surcharge(slot_index);
        let deficit = required.saturating_sub(power.min(5));

        for k in 0..=deficit {
            if (k as u32) > p.imported_goods {
                continue;
            }
            let extra_yen = 2 * (deficit - k);
            if (extra_yen as u32) > p.money {
                continue;
            }

            if needs_fifth_payment {
                if p.money >= extra_yen as u32 + 2 {
                    moves.push(Move::ExecuteAreaAction {
                        action: AreaAction::Laboratory {
                            card_id, extra_yen, extra_imported: k, fifth_tech_payment: Some(true),
                        },
                    });
                }
                if p.imported_goods > k as u32 {
                    moves.push(Move::ExecuteAreaAction {
                        action: AreaAction::Laboratory {
                            card_id, extra_yen, extra_imported: k, fifth_tech_payment: Some(false),
                        },
                    });
                }
            } else {
                moves.push(Move::ExecuteAreaAction {
                    action: AreaAction::Laboratory {
                        card_id, extra_yen, extra_imported: k, fifth_tech_payment: None,
                    },
                });
            }
        }
    }

    moves.push(Move::ExecuteAreaAction { action: AreaAction::Skip });
    moves
}

fn generate_area_action_for_type(area_type: AreaType) -> Move {
    // Simplified: return default action for area type. Port/Laboratory are
    // handled before this dispatch is reached (see generate_area_action_moves)
    // since they need real board contents, not just the type.
    let action = match area_type {
        AreaType::CopperMine => AreaAction::ProduceCopper,
        AreaType::SilkMill => AreaAction::ProduceSilk,
        AreaType::TeaPlantation => AreaAction::ProduceTea,
        AreaType::FishingGround => AreaAction::ProduceSeafood,
        AreaType::Bank => AreaAction::Bank,
        AreaType::PortA | AreaType::PortB | AreaType::LaboratoryA | AreaType::LaboratoryB => {
            unreachable!("Port/Laboratory are enumerated by generate_port_actions/generate_lab_actions")
        }
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

    let power = state.current_action_power.unwrap_or(0);

    if let Some(site) = state.board_layout.building_sites.get(&area) {
        // Shophouse: requires Power 4+
        if power >= 4 && p.shophouses.in_hand > 0 {
            if let Some(slot) = site.next_shophouse_slot(player) {
                moves.push(Move::BuildShophouse { slot: slot as u8 });
            }
        }

        // Trading House: requires Power 5
        if power >= 5 && p.trading_houses.in_warehouse > 0 && site.can_build_trading_house() {
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
    fn test_placement_moves_with_assistants() {
        let state = create_test_state();
        let moves = generate_placement_moves_with_tech(&state, None);

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
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);

        // Should always include EndAdditionalPhase
        assert!(moves.iter().any(|m| matches!(m, Move::EndAdditionalPhase)));
    }

    #[test]
    fn test_building_requires_power_4() {
        let mut state = create_test_state();
        state.turn_phase = TurnPhase::Building;
        state.current_action_power = Some(3); // Not enough
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);

        // Should only have Skip
        assert_eq!(moves.len(), 1);
        assert!(matches!(moves[0], Move::SkipBuilding));
    }

    #[test]
    fn test_shophouse_requires_power_4() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        state.turn_phase = TurnPhase::Building;
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].shophouses.in_hand = 1;

        // Add a building site (shophouse rewards: [2, 3, 4, 5], trading house reward: 10)
        state.board_layout.building_sites.insert(area, BuildingSite::new([2, 3, 4, 5], 10));
        let rules = Rules::empty_for_tests();

        // Power 3 - no shophouse option
        state.current_action_power = Some(3);
        let moves = get_legal_moves(&state, &rules);
        assert!(!moves.iter().any(|m| matches!(m, Move::BuildShophouse { .. })));

        // Power 4 - shophouse available
        state.current_action_power = Some(4);
        let moves = get_legal_moves(&state, &rules);
        assert!(moves.iter().any(|m| matches!(m, Move::BuildShophouse { .. })));
    }

    #[test]
    fn test_trading_house_requires_power_5() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        state.turn_phase = TurnPhase::Building;
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].trading_houses.in_warehouse = 1;

        // Add a building site with no trading house yet
        state.board_layout.building_sites.insert(area, BuildingSite::new([2, 3, 4, 5], 10));
        let rules = Rules::empty_for_tests();

        // Power 4 - trading house NOT available (needs 5)
        state.current_action_power = Some(4);
        let moves = get_legal_moves(&state, &rules);
        assert!(!moves.iter().any(|m| matches!(m, Move::BuildTradingHouse)));

        // Power 5 - trading house available
        state.current_action_power = Some(5);
        let moves = get_legal_moves(&state, &rules);
        assert!(moves.iter().any(|m| matches!(m, Move::BuildTradingHouse)));
    }

    #[test]
    fn test_both_buildings_at_power_5() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        state.turn_phase = TurnPhase::Building;
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].shophouses.in_hand = 1;
        state.players[0].trading_houses.in_warehouse = 1;

        // Add a building site
        state.board_layout.building_sites.insert(area, BuildingSite::new([2, 3, 4, 5], 10));
        let rules = Rules::empty_for_tests();

        // Power 5 - both available
        state.current_action_power = Some(5);
        let moves = get_legal_moves(&state, &rules);
        assert!(moves.iter().any(|m| matches!(m, Move::BuildShophouse { .. })));
        assert!(moves.iter().any(|m| matches!(m, Move::BuildTradingHouse)));
        assert!(moves.iter().any(|m| matches!(m, Move::SkipBuilding)));
    }

    #[test]
    fn test_placement_cost_opponent_president() {
        let mut state = create_test_state();
        let bank_area = AreaId::new(0);

        // Setup: Opponent President in Bank, player has 0 Yen
        state.players[1].president_location = PresidentLocation::OnBoard(bank_area);
        state.players[0].money = 0;
        state.turn_phase = TurnPhase::Placement;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);

        // Verify: Cannot place in Bank (filtered from legal moves)
        // Empty placement should still be available
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements } if placements.is_empty())));

        // Cannot place in Bank (area 0) since it costs 1 Yen and player has 0
        assert!(!moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 1 && placements[0] == bank_area)));

        // Cannot use PlaceTwoSameArea in Bank either
        assert!(!moves.iter().any(|m| matches!(m, Move::PlaceTwoSameArea { area, .. } if *area == bank_area)));

        // But CAN place in CopperMine (area 1) - no opponent President there
        let mine_area = AreaId::new(1);
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 1 && placements[0] == mine_area)));
    }

    #[test]
    fn test_placement_cost_multiple_areas() {
        let mut state = create_test_state();
        let bank_area = AreaId::new(0);
        let mine_area = AreaId::new(1);

        // Setup: Opponent Presidents in Bank AND CopperMine (need 2nd player for this)
        // Actually, one opponent can only have one president
        // So let's use: Player 1 president in Bank (costs 1 Yen)
        state.players[1].president_location = PresidentLocation::OnBoard(bank_area);
        state.players[0].money = 1;
        state.turn_phase = TurnPhase::Placement;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);

        // With 1 Yen, can place in Bank alone (costs 1)
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 1 && placements[0] == bank_area)));

        // Can also place in CopperMine alone (costs 0)
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 1 && placements[0] == mine_area)));

        // Can place in both Bank AND CopperMine together (costs 1 total - only Bank has opponent)
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 2 && placements.contains(&bank_area) && placements.contains(&mine_area))));

        // Now with 0 Yen, cannot place in Bank
        state.players[0].money = 0;
        let moves = get_legal_moves(&state, &rules);

        // Cannot place in Bank alone
        assert!(!moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 1 && placements[0] == bank_area)));

        // Cannot place in both either (costs 1)
        assert!(!moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 2 && placements.contains(&bank_area))));

        // But can still place in CopperMine alone (costs 0)
        assert!(moves.iter().any(|m| matches!(m, Move::PlaceThreeAreas { placements }
            if placements.len() == 1 && placements[0] == mine_area)));
    }

    #[test]
    fn test_calculate_placement_cost() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        let bank_area = AreaId::new(0);
        let mine_area = AreaId::new(1);

        // No opponent presidents - cost is 0
        assert_eq!(calculate_placement_cost(&state, player, &[bank_area]), 0);
        assert_eq!(calculate_placement_cost(&state, player, &[mine_area]), 0);
        assert_eq!(calculate_placement_cost(&state, player, &[bank_area, mine_area]), 0);

        // Opponent president in Bank - cost is 1 for Bank, 0 for Mine
        state.players[1].president_location = PresidentLocation::OnBoard(bank_area);
        assert_eq!(calculate_placement_cost(&state, player, &[bank_area]), 1);
        assert_eq!(calculate_placement_cost(&state, player, &[mine_area]), 0);
        assert_eq!(calculate_placement_cost(&state, player, &[bank_area, mine_area]), 1);

        // Empty placement always costs 0
        assert_eq!(calculate_placement_cost(&state, player, &[]), 0);
    }

    #[test]
    fn test_calculate_placement_cost_sums_per_distinct_opponent_sharing_area() {
        // KB (01-rules-reference.md:97): "If any President pawn(s) occupy the
        // target area, pay 1 Yen to each President's owner per assistant
        // placed." Two DIFFERENT opponents' Presidents can both sit in the
        // same area (each owns only one President, but nothing forces either
        // to leave when the other arrives), so the cost must sum per distinct
        // opponent owner in that area, not cap at 1 per area.
        let mut state = create_test_state();
        state.players.push(PlayerState::new(PlayerId::new(2), PlayerColor::Yellow, false));
        let player = PlayerId::new(0);
        let bank_area = AreaId::new(0);

        // Both opponents' Presidents in Bank.
        state.players[1].president_location = PresidentLocation::OnBoard(bank_area);
        state.players[2].president_location = PresidentLocation::OnBoard(bank_area);

        assert_eq!(calculate_placement_cost(&state, player, &[bank_area]), 2);
    }

    #[test]
    fn test_station_teleport_included_in_movement() {
        let mut state = create_test_state();
        let current_area = AreaId::new(0);
        let station_area = AreaId::new(2); // Different area

        // Setup: President on board, Station technology, station placed
        state.turn_phase = TurnPhase::Movement;
        state.players[0].president_location = PresidentLocation::OnBoard(current_area);
        state.players[0].assistants.on_board.insert(station_area, 1);
        state.station_location = Some(station_area);

        // Give player Station tech
        let station_tech = TechCardId(100); // Arbitrary ID for test
        state.players[0].technologies.push(station_tech);

        // Create tech registry with Station tech
        let mut tech_registry = TechRegistry::new();
        let station_card = TechCard {
            id: station_tech,
            name: "Station".to_string(),
            industry_cost: 4,
            country: Country::German,
            effect: TechEffect::Station,
        };
        tech_registry.insert(station_card, "tech_station");

        // Get legal moves with tech awareness
        let mut rules = Rules::empty_for_tests();
        rules.techs = tech_registry;
        let moves = get_legal_moves(&state, &rules);

        // Should include station teleport move (free move to station area)
        let has_teleport = moves.iter().any(|m| matches!(m,
            Move::MovePresident { destination, .. } if *destination == station_area
        ));

        assert!(has_teleport, "Should include station teleport move to station area");
    }

    #[test]
    fn test_telegram_adjacent_bonus() {
        let mut state = create_test_state();
        let main_area = AreaId::new(0); // Bank
        let adjacent_area = AreaId::new(1); // CopperMine

        // Add adjacency: Bank <-> CopperMine
        state.board_layout.adjacency.insert(main_area, vec![adjacent_area]);
        state.board_layout.adjacency.insert(adjacent_area, vec![main_area]);

        state.turn_phase = TurnPhase::Placement;
        state.players[0].assistants.in_hand = 3; // Need 3 for bonus (2 + 1)

        // Without Telegram tech - only basic PlaceTwoSameArea moves
        let rules_no_tech = Rules::empty_for_tests();
        let moves_no_tech = get_legal_moves(&state, &rules_no_tech);
        let has_adjacent_bonus_without_tech = moves_no_tech.iter().any(|m| matches!(m,
            Move::PlaceTwoSameArea { area, adjacent_bonus: Some(_) } if *area == main_area
        ));
        assert!(!has_adjacent_bonus_without_tech, "Without Telegram, should not have adjacent bonus");

        // With Telegram tech - should have adjacent bonus moves
        let telegram_tech = TechCardId(101);
        state.players[0].technologies.push(telegram_tech);

        let mut tech_registry = TechRegistry::new();
        let telegram_card = TechCard {
            id: telegram_tech,
            name: "Telegram".to_string(),
            industry_cost: 3,
            country: Country::American,
            effect: TechEffect::Telegram,
        };
        tech_registry.insert(telegram_card, "tech_telegram");

        let mut rules_with_tech = Rules::empty_for_tests();
        rules_with_tech.techs = tech_registry;
        let moves_with_tech = get_legal_moves(&state, &rules_with_tech);

        // Should have basic move (adjacent_bonus: None)
        let has_basic = moves_with_tech.iter().any(|m| matches!(m,
            Move::PlaceTwoSameArea { area, adjacent_bonus: None } if *area == main_area
        ));
        assert!(has_basic, "Should have basic PlaceTwoSameArea move");

        // Should have at least one adjacent bonus move
        let has_adjacent_bonus = moves_with_tech.iter().any(|m| matches!(m,
            Move::PlaceTwoSameArea { adjacent_bonus: Some(_), .. }
        ));
        assert!(has_adjacent_bonus, "With Telegram, should have adjacent bonus options");
    }

    // --- Task 16: Port & Laboratory enumeration -----------------------

    /// Builds a 6-slot PortBoard with the given card ids seeded left to
    /// right (index 0..5) -- mirrors `actions::cards::tests::seeded_port`,
    /// duplicated locally since that helper is private to its own module.
    fn seeded_port(ids: [u8; 6]) -> PortBoard {
        PortBoard { slots: ids.map(|id| Some(OrderCardId(id))) }
    }

    /// Places a PortA area on the board, seeds it, puts the president
    /// there, and enters the AreaAction phase at the given power.
    fn create_port_area_state(power: u8, port: PortBoard) -> GameState {
        let mut state = create_test_state();
        let port_area = AreaId::new(5);
        state.board_layout.areas.insert(port_area, AreaConfig {
            area_type: AreaType::PortA,
            grid_position: (0, 5),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(port_area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(power);
        state.port_state.port_a = port;
        state
    }

    /// Places a LaboratoryA area on the board, puts the president there,
    /// and enters the AreaAction phase at the given power (slots left empty
    /// for the caller to fill).
    fn create_lab_area_state(power: u8) -> GameState {
        let mut state = create_test_state();
        let lab_area = AreaId::new(5);
        state.board_layout.areas.insert(lab_area, AreaConfig {
            area_type: AreaType::LaboratoryA,
            grid_position: (0, 5),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(lab_area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(power);
        state
    }

    fn make_test_tech_card(id: u8, cost: u8) -> TechCard {
        TechCard {
            id: TechCardId(id),
            name: "Test Tech".to_string(),
            country: Country::English,
            industry_cost: cost,
            effect: TechEffect::Tram,
        }
    }

    /// Task 16 "every-move-applies" property: every move a generator emits
    /// must independently pass `apply_move`. The fuzz suite (Task 18)
    /// enforces this globally; this is the focused per-test version.
    fn assert_all_moves_apply(state: &GameState, moves: &[Move], rules: &Rules) {
        for m in moves {
            let result = apply_move(state, m, rules);
            assert!(result.is_ok(), "generated move failed to apply: {m:?} ({result:?})");
        }
    }

    #[test]
    fn port_enumeration_power_4_all_slots_singles_pairs_and_skip() {
        // Power 4, all 6 slots filled, hand empty, money >= 2, imports >= 1:
        // 6 singles + C(6,2)=15 pairs x 2 payments + 1 Skip = 37.
        let mut state = create_port_area_state(4, seeded_port([1, 2, 3, 4, 5, 6]));
        state.players[0].money = 2;
        state.players[0].imported_goods = 1;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 37);
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn port_enumeration_power_2_low_zone_only() {
        // Power 2 -> only Low zone (slots 0-1) accessible: 2 singles + 1
        // pair x 2 payments + 1 Skip = 5.
        let mut state = create_port_area_state(2, seeded_port([1, 2, 3, 4, 5, 6]));
        state.players[0].money = 2;
        state.players[0].imported_goods = 1;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 5);
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn port_enumeration_power_2_hand_2_only_singles() {
        // Hand already holds 2 orders: singles (2+1=3, OK) but pairs
        // (2+2=4 > 3) would exceed the hand limit, so only singles + Skip
        // appear: 2 + 0 + 1 = 3.
        let mut state = create_port_area_state(2, seeded_port([1, 2, 3, 4, 5, 6]));
        state.players[0].money = 2;
        state.players[0].imported_goods = 1;
        state.players[0].order_cards_in_hand = vec![OrderCardId(200), OrderCardId(201)];
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 3);
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn port_enumeration_power_1_is_skip_only() {
        // Power 1 -> no zone accessible: Skip is the only legal move.
        let state = create_port_area_state(1, seeded_port([1, 2, 3, 4, 5, 6]));
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 1);
        assert!(matches!(&moves[0], Move::ExecuteAreaAction { action: AreaAction::Skip }));
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn port_enumeration_includes_same_zone_pair() {
        // Power 2: the only possible pair is the two Low-zone cards (slots
        // 0,1) -- pins the same-zone double-take case.
        let mut state = create_port_area_state(2, seeded_port([1, 2, 3, 4, 5, 6]));
        state.players[0].money = 2;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        let has_same_zone_pair = moves.iter().any(|m| matches!(m,
            Move::ExecuteAreaAction {
                action: AreaAction::Port {
                    cards_taken,
                    second_card_payment: Some(PortPayment::TwoYen),
                }
            } if cards_taken == &vec![OrderCardId(1), OrderCardId(2)]
        ));
        assert!(has_same_zone_pair, "expected a pair move taking both Low-zone cards {{1,2}}");
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn port_enumeration_during_mercenary_sub_turn_resolves_mercenary_target_area() {
        // President is in hand (never placed) -- Port enumeration must
        // resolve the acting area from mercenary_target_area, mirroring how
        // execute_port_action resolves the acting board during a mercenary
        // sub-turn. Same resources as the power-2 pin: 2 + 1x2 + 1 = 5.
        let mut state = create_test_state();
        let port_area = AreaId::new(5);
        state.board_layout.areas.insert(port_area, AreaConfig {
            area_type: AreaType::PortA,
            grid_position: (0, 5),
        });
        state.port_state.port_a = seeded_port([1, 2, 3, 4, 5, 6]);
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(port_area);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(2);
        state.players[0].money = 2;
        state.players[0].imported_goods = 1;
        let rules = Rules::empty_for_tests();

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 5);
        assert_all_moves_apply(&state, &moves, &rules);
    }

    #[test]
    fn lab_enumeration_power_3_two_deficits_no_overpay() {
        // Power 3: slot0 cost 3 (deficit 0) -> 1 move; slot2 cost 3 +
        // surcharge 1 = required 4 (deficit 1) -> 2 exact-cover moves
        // ({extra_yen:2},{extra_imported:1}); + 1 Skip = 4. <4 techs owned,
        // so fifth_tech_payment stays None throughout.
        let mut state = create_lab_area_state(3);
        state.lab_state.lab_a.slots[0] = Some(TechCardId(1));
        state.lab_state.lab_a.slots[2] = Some(TechCardId(2));
        state.players[0].money = 2;
        state.players[0].imported_goods = 1;

        let mut rules = Rules::empty_for_tests();
        rules.techs.insert(make_test_tech_card(1, 3), "tech_1");
        rules.techs.insert(make_test_tech_card(2, 3), "tech_2");

        let moves = get_legal_moves(&state, &rules);
        assert_eq!(moves.len(), 4);
        assert_all_moves_apply(&state, &moves, &rules);
    }
}
