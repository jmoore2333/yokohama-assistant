//! President movement and pathfinding.

use std::collections::{HashSet, VecDeque};
use crate::types::{AreaId, GameState, Move, PlayerId, PresidentLocation, TechEffect};
use super::cards::TechRegistry;

/// Check if an area is a valid destination (Possible Action Area).
/// Without TechRegistry, assumes no Postal System.
pub fn is_possible_action_area(state: &GameState, player: PlayerId, area: AreaId) -> bool {
    is_possible_action_area_with_tech(state, player, area, None)
}

/// Check if an area is a valid destination with technology awareness.
pub fn is_possible_action_area_with_tech(
    state: &GameState,
    player: PlayerId,
    area: AreaId,
    tech_registry: Option<&TechRegistry>,
) -> bool {
    let p = state.player(player);

    // Must have at least 1 assistant
    if p.assistants.in_area(area) < 1 {
        return false;
    }

    // Check for Postal System technology
    let has_postal = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::PostalSystem).unwrap_or(false)
        })
    });

    // Check for other Presidents (unless has Postal System)
    if !has_postal {
        for (i, other_player) in state.players.iter().enumerate() {
            if i != player.0 as usize
                && other_player.president_location == PresidentLocation::OnBoard(area)
            {
                return false;
            }
        }
    }

    true
}

/// Find all areas reachable from current position via assistant chain.
/// Returns list of (destination, path_to_reach).
pub fn find_reachable_areas(
    state: &GameState,
    player: PlayerId,
    from: AreaId,
) -> Vec<(AreaId, Vec<AreaId>)> {
    let p = state.player(player);
    let mut reachable = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // Start from adjacent areas
    visited.insert(from);

    for &adj in state.board_layout.adjacent_areas(from) {
        // Can only move through areas with our assistants
        if p.assistants.in_area(adj) > 0 {
            queue.push_back((adj, vec![adj]));
        }
    }

    while let Some((current, path)) = queue.pop_front() {
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current);

        // Is this a valid destination?
        if is_possible_action_area(state, player, current) && current != from {
            reachable.push((current, path.clone()));
        }

        // Continue exploring if we have assistants here
        if p.assistants.in_area(current) > 0 {
            for &adj in state.board_layout.adjacent_areas(current) {
                if !visited.contains(&adj) && p.assistants.in_area(adj) > 0 {
                    let mut new_path = path.clone();
                    new_path.push(adj);
                    queue.push_back((adj, new_path));
                }
            }
        }
    }

    reachable
}

/// Check if player can teleport to Station area (Station technology).
/// Rulebook p.15: "You may move your President pawn to this area during
/// the Movement Step for free... regardless of other President pawns."
pub fn can_teleport_to_station(
    state: &GameState,
    player: PlayerId,
) -> Option<AreaId> {
    can_teleport_to_station_with_tech(state, player, None)
}

/// Check if player can teleport to Station area with tech awareness.
pub fn can_teleport_to_station_with_tech(
    state: &GameState,
    player: PlayerId,
    tech_registry: Option<&TechRegistry>,
) -> Option<AreaId> {
    let p = state.player(player);

    // Must have Station technology
    let has_station = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::Station).unwrap_or(false)
        })
    });

    if !has_station {
        return None;
    }

    // Must have a station placed
    let station_area = state.station_location?;

    // Station area must have our assistant
    if p.assistants.in_area(station_area) > 0 {
        Some(station_area)
    } else {
        None
    }
}

/// Generate station teleport move if available.
pub fn generate_station_teleport_move(
    state: &GameState,
    player: PlayerId,
) -> Option<Move> {
    let station_area = can_teleport_to_station(state, player)?;

    // Can't teleport to where we already are
    let p = state.player(player);
    if p.president_location == PresidentLocation::OnBoard(station_area) {
        return None;
    }

    // Teleport is free (empty path, 0 cost)
    Some(Move::MovePresident {
        path: vec![], // No intermediate areas
        destination: station_area,
    })
}

/// Calculate movement cost (opponent Presidents and canal tolls).
/// Without TechRegistry, assumes no Stagecoach.
pub fn calculate_movement_cost(
    state: &GameState,
    player: PlayerId,
    path: &[AreaId],
) -> u32 {
    calculate_movement_cost_with_tech(state, player, path, None)
}

/// Calculate movement cost with technology awareness.
pub fn calculate_movement_cost_with_tech(
    state: &GameState,
    player: PlayerId,
    path: &[AreaId],
    tech_registry: Option<&TechRegistry>,
) -> u32 {
    let mut cost: u32 = 0;
    let p = state.player(player);

    // Check for Stagecoach technology
    let has_stagecoach = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::Stagecoach).unwrap_or(false)
        })
    });

    // Check each area in path (except destination)
    let intermediate = if path.len() > 1 { &path[..path.len() - 1] } else { &[] };

    for &area in intermediate {
        // Pay for opponent Presidents (unless has Stagecoach)
        if !has_stagecoach {
            for other_player in &state.players {
                if other_player.id != player
                    && other_player.president_location == PresidentLocation::OnBoard(area)
                {
                    cost += 1;
                }
            }
        }
    }

    // Check for canal crossings
    if path.len() >= 2 {
        for window in path.windows(2) {
            if state.board_layout.is_canal_crossing(window[0], window[1]) {
                cost += 1;
            }
        }
    }

    cost
}

/// Validate a movement action.
pub fn validate_movement(
    state: &GameState,
    player: PlayerId,
    path: &[AreaId],
    destination: AreaId,
) -> Result<(), &'static str> {
    let p = state.player(player);

    // Check we can afford the movement
    let cost = calculate_movement_cost(state, player, path);
    if p.money < cost {
        return Err("Not enough money to pay movement costs");
    }

    // Check destination is reachable and valid
    if !is_possible_action_area(state, player, destination) {
        return Err("Destination is not a valid action area");
    }

    // Verify path connectivity
    let current_pos = match p.president_location {
        PresidentLocation::OnBoard(area) => area,
        PresidentLocation::InHand => {
            return Err("President must be on board to move (use place instead)");
        }
    };

    // Check path starts adjacent to current position
    if let Some(&first) = path.first() {
        if !state.board_layout.adjacent_areas(current_pos).contains(&first) {
            return Err("Path must start adjacent to current position");
        }
    }

    // Check destination matches end of path
    if path.last() != Some(&destination) && !path.is_empty() {
        return Err("Path must end at destination");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::collections::HashMap;

    fn create_test_state_with_board() -> GameState {
        let config = GameConfig::new(
            2,
            vec![PlayerColor::Red, PlayerColor::Blue],
            PlayerId::new(0),
        );

        // Create a simple 3-area board: 0 -- 1 -- 2
        let mut areas = HashMap::new();
        areas.insert(AreaId::new(0), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(AreaId::new(1), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });
        areas.insert(AreaId::new(2), AreaConfig { area_type: AreaType::SilkMill, grid_position: (0, 2) });

        let mut adjacency = HashMap::new();
        adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        adjacency.insert(AreaId::new(1), vec![AreaId::new(0), AreaId::new(2)]);
        adjacency.insert(AreaId::new(2), vec![AreaId::new(1)]);

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

        GameState {
            config,
            board_layout,
            players,
            current_player: PlayerId::new(0),
            turn_phase: TurnPhase::Movement,
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
    fn test_is_possible_action_area() {
        let mut state = create_test_state_with_board();
        let player = PlayerId::new(0);
        let area = AreaId::new(1);

        // No assistant -> not valid
        assert!(!is_possible_action_area(&state, player, area));

        // With assistant -> valid
        state.players[0].assistants.on_board.insert(area, 1);
        assert!(is_possible_action_area(&state, player, area));

        // With opponent president -> not valid (no Postal System)
        state.players[1].president_location = PresidentLocation::OnBoard(area);
        assert!(!is_possible_action_area(&state, player, area));
    }

    #[test]
    fn test_find_reachable_areas() {
        let mut state = create_test_state_with_board();
        let player = PlayerId::new(0);

        // Place assistants on areas 1 and 2
        state.players[0].assistants.on_board.insert(AreaId::new(1), 1);
        state.players[0].assistants.on_board.insert(AreaId::new(2), 1);

        let reachable = find_reachable_areas(&state, player, AreaId::new(0));

        // Should be able to reach area 1 and 2
        assert_eq!(reachable.len(), 2);

        let destinations: Vec<AreaId> = reachable.iter().map(|(d, _)| *d).collect();
        assert!(destinations.contains(&AreaId::new(1)));
        assert!(destinations.contains(&AreaId::new(2)));
    }

    #[test]
    fn test_movement_cost_canal() {
        let mut state = create_test_state_with_board();
        state.board_layout.canal_crossings.push((AreaId::new(0), AreaId::new(1)));

        let cost = calculate_movement_cost(&state, PlayerId::new(0), &[AreaId::new(0), AreaId::new(1)]);
        assert_eq!(cost, 1); // 1 Yen for canal
    }
}
