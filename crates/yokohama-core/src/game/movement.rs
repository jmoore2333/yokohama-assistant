//! President movement and pathfinding.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};
use crate::types::{AreaId, GameState, Move, PlayerId, PresidentLocation, TechEffect};
use super::cards::TechRegistry;

/// Check if an area is a valid destination (Possible Action Area).
/// Without TechRegistry, assumes no Postal System.
pub fn is_possible_action_area(state: &GameState, player: PlayerId, area: AreaId) -> bool {
    is_possible_action_area_full(state, player, area, None, false)
}

/// Check if an area is a valid destination with technology awareness.
pub fn is_possible_action_area_with_tech(
    state: &GameState,
    player: PlayerId,
    area: AreaId,
    tech_registry: Option<&TechRegistry>,
) -> bool {
    is_possible_action_area_full(state, player, area, tech_registry, false)
}

/// Full version with all options.
/// - is_mercenary: If true, opponent presidents don't block.
pub fn is_possible_action_area_full(
    state: &GameState,
    player: PlayerId,
    area: AreaId,
    tech_registry: Option<&TechRegistry>,
    is_mercenary: bool,
) -> bool {
    let p = state.player(player);

    // Must have at least 1 assistant
    if p.assistants.in_area(area) < 1 {
        return false;
    }

    // Mercenaries ignore opponent president blocking
    if is_mercenary {
        return true;
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
/// Returns list of (destination, path_to_reach). Without TechRegistry,
/// assumes no Postal System.
pub fn find_reachable_areas(
    state: &GameState,
    player: PlayerId,
    from: AreaId,
) -> Vec<(AreaId, Vec<AreaId>)> {
    find_reachable_areas_with_tech(state, player, from, None)
}

/// Find all areas reachable from current position via assistant chain, with
/// technology awareness (Postal System lets the destination check tolerate
/// an opponent President already there). Returns one (destination, path)
/// pair per reachable destination: the MINIMUM-TOTAL-COST path into it
/// (canal tolls + opponent-president pass-through/destination fees --
/// `calculate_movement_cost_full`'s exact weighting), not merely the fewest
/// hops (I3 final-review fix: a canal crossing is always hop-count-shortest,
/// so a plain BFS would toll a destination even when a longer, toll-free
/// route through the mover's own assistants also reaches it).
///
/// A Dijkstra over the assistant-presence graph: nodes are `from` plus
/// every area holding >=1 of the mover's own assistants (a path may only
/// travel through -- and may stop at -- areas holding the mover's own
/// assistant, same rule the old BFS enforced, so `from` itself never
/// re-enters as a waypoint). The weight of stepping onto area `b` from `a`
/// is the canal toll on the (a,b) crossing plus 1 Yen per opponent
/// President occupying `b` (waived by Stagecoach) -- summing these along a
/// path reproduces `calculate_movement_cost_full`'s total exactly, since
/// that function's "intermediate" and "destination" loops together price
/// every area in `path` once, and its canal-toll loop scans the same
/// `[from] + path` window sequence this Dijkstra's edges do.
///
/// Ties are broken by lexicographically smallest path (`Vec<AreaId>`'s
/// derived `Ord`), so enumeration stays fully deterministic. This is sound
/// with plain lazy-deletion Dijkstra (no separate tie-break pass needed):
/// extending a path by one hop with a non-negative-weight edge always
/// produces a STRICTLY greater `(cost, path)` key than the path it
/// extends -- if the edge costs > 0, the cost component alone decides it;
/// if it costs 0, the cost ties but a proper prefix always sorts before
/// the longer vector it's a prefix of (`vec![a] < vec![a, b]`). So keys
/// only ever increase along any walk, meaning the first successful pop for
/// any area is already that area's global-optimum (cost, path) pair.
pub fn find_reachable_areas_with_tech(
    state: &GameState,
    player: PlayerId,
    from: AreaId,
    tech_registry: Option<&TechRegistry>,
) -> Vec<(AreaId, Vec<AreaId>)> {
    let p = state.player(player);

    let has_stagecoach = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::Stagecoach).unwrap_or(false)
        })
    });

    // Mirrors calculate_movement_cost_full's per-area pricing for stepping
    // onto `to` from `from_area`: canal tolls are never waived; the
    // opponent-president fee is waived entirely by Stagecoach.
    let edge_cost = |from_area: AreaId, to: AreaId| -> u32 {
        let mut cost = 0;
        if state.board_layout.is_canal_crossing(from_area, to) {
            cost += 1;
        }
        if !has_stagecoach {
            cost += state.players.iter()
                .filter(|o| o.id != player && o.president_location == PresidentLocation::OnBoard(to))
                .count() as u32;
        }
        cost
    };

    let mut best: BTreeMap<AreaId, (u32, Vec<AreaId>)> = BTreeMap::new();
    let mut heap: BinaryHeap<Reverse<(u32, Vec<AreaId>, AreaId)>> = BinaryHeap::new();

    for &adj in state.board_layout.adjacent_areas(from) {
        // Can only move through areas with our assistants
        if p.assistants.in_area(adj) > 0 {
            heap.push(Reverse((edge_cost(from, adj), vec![adj], adj)));
        }
    }

    while let Some(Reverse((cost, path, area))) = heap.pop() {
        if best.contains_key(&area) {
            continue; // a cheaper-or-equal entry already finalized this area (see doc comment)
        }
        best.insert(area, (cost, path.clone()));

        // Continue exploring if we have assistants here
        if p.assistants.in_area(area) > 0 {
            for &adj in state.board_layout.adjacent_areas(area) {
                if adj == from || p.assistants.in_area(adj) == 0 {
                    continue;
                }
                let mut new_path = path.clone();
                new_path.push(adj);
                heap.push(Reverse((cost + edge_cost(area, adj), new_path, adj)));
            }
        }
    }

    best.into_iter()
        .filter(|&(area, _)| is_possible_action_area_with_tech(state, player, area, tech_registry))
        .map(|(area, (_, path))| (area, path))
        .collect()
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
    calculate_movement_cost_full(state, player, path, None, false)
}

/// Calculate movement cost with technology awareness.
pub fn calculate_movement_cost_with_tech(
    state: &GameState,
    player: PlayerId,
    path: &[AreaId],
    tech_registry: Option<&TechRegistry>,
) -> u32 {
    calculate_movement_cost_full(state, player, path, tech_registry, false)
}

/// Calculate movement cost for mercenary action.
/// Mercenaries don't pay for passing through opponent presidents.
pub fn calculate_movement_cost_mercenary(
    state: &GameState,
    player: PlayerId,
    path: &[AreaId],
) -> u32 {
    calculate_movement_cost_full(state, player, path, None, true)
}

/// Full movement cost calculation with all options.
/// - is_mercenary: If true, no cost for opponent presidents.
pub fn calculate_movement_cost_full(
    state: &GameState,
    player: PlayerId,
    path: &[AreaId],
    tech_registry: Option<&TechRegistry>,
    is_mercenary: bool,
) -> u32 {
    let mut cost: u32 = 0;
    let p = state.player(player);

    // Mercenaries don't pay for opponent presidents
    let skip_president_cost = is_mercenary;

    // Check for Stagecoach technology
    let has_stagecoach = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::Stagecoach).unwrap_or(false)
        })
    });

    // Check each area in path (except destination)
    let intermediate = if path.len() > 1 { &path[..path.len() - 1] } else { &[] };

    for &area in intermediate {
        // Pay for opponent Presidents (unless has Stagecoach or is mercenary)
        if !has_stagecoach && !skip_president_cost {
            for other_player in &state.players {
                if other_player.id != player
                    && other_player.president_location == PresidentLocation::OnBoard(area)
                {
                    cost += 1;
                }
            }
        }
    }

    // Destination stop fee (Postal System, rulebook p.15: "You may end your
    // movement in an Area occupied by another President pawn... You must
    // still pay these players (unless you also own the Stagecoach)").
    // Without Postal System such a destination is never reachable in the
    // first place, so this is vacuous for everyone else. A non-empty path's
    // last area is where the President ends this move; an EMPTY path is a
    // free teleport (Station/Tram) and is exempt - Station's card text is
    // unconditionally "for free... regardless of other President pawns",
    // and Tram's destination is guaranteed president-free by construction.
    if !has_stagecoach && !skip_president_cost {
        if let Some(&destination) = path.last() {
            for other_player in &state.players {
                if other_player.id != player
                    && other_player.president_location == PresidentLocation::OnBoard(destination)
                {
                    cost += 1;
                }
            }
        }
    }

    // Check for canal crossings (mercenaries still pay canal tolls). The
    // toll applies to EVERY crossing in the move, including the very first
    // step out of the President's current area, so the scan must start
    // from wherever the President currently is, not just within `path`
    // (regression fix, I4 final-review: `path.windows(2)` alone missed the
    // start -> path[0] edge). From hand (Movement Option B places, never
    // traverses), there is no "current area" to prepend.
    let full_route: Vec<AreaId> = match p.president_location {
        PresidentLocation::OnBoard(start) => std::iter::once(start).chain(path.iter().copied()).collect(),
        PresidentLocation::InHand => path.to_vec(),
    };
    for window in full_route.windows(2) {
        if state.board_layout.is_canal_crossing(window[0], window[1]) {
            cost += 1;
        }
    }

    cost
}

/// Fee for placing the President from hand directly into `destination`
/// (Movement Option A) - 1 Yen per opponent President already there,
/// waived by Stagecoach (I2 final-review fix). Such a destination is only
/// reachable at all with Postal System, whose card text is explicit: "You
/// must still pay these players (unless you also own the Stagecoach)."
/// Shared by the legal-move generator (pricing gate) and
/// `apply_place_president` (the charge), so they can never drift apart.
pub fn calculate_place_president_fee(
    state: &GameState,
    player: PlayerId,
    destination: AreaId,
    tech_registry: Option<&TechRegistry>,
) -> u32 {
    let p = state.player(player);
    let has_stagecoach = tech_registry.is_some_and(|registry| {
        p.technologies.iter().any(|&tech_id| {
            registry.get(tech_id).map(|t| t.effect == TechEffect::Stagecoach).unwrap_or(false)
        })
    });
    if has_stagecoach {
        return 0;
    }
    state.players.iter()
        .filter(|o| o.id != player && o.president_location == PresidentLocation::OnBoard(destination))
        .count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use crate::game::apply_move::apply_move;
    use crate::game::legal_moves::get_legal_moves;
    use crate::game::rules::Rules;
    use std::collections::BTreeMap;

    fn create_test_state_with_board() -> GameState {
        let config = GameConfig::new(
            2,
            vec![PlayerColor::Red, PlayerColor::Blue],
            PlayerId::new(0),
        );

        // Create a simple 3-area board: 0 -- 1 -- 2
        let mut areas = BTreeMap::new();
        areas.insert(AreaId::new(0), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(AreaId::new(1), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 1) });
        areas.insert(AreaId::new(2), AreaConfig { area_type: AreaType::SilkMill, grid_position: (0, 2) });

        let mut adjacency = BTreeMap::new();
        adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        adjacency.insert(AreaId::new(1), vec![AreaId::new(0), AreaId::new(2)]);
        adjacency.insert(AreaId::new(2), vec![AreaId::new(1)]);

        let board_layout = BoardLayout {
            areas,
            adjacency,
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
            turn_phase: TurnPhase::Movement,
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
            pending_choices: Vec::new(),
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

    #[test]
    fn test_mercenary_no_cost_for_opponent_presidents() {
        let mut state = create_test_state_with_board();
        let player = PlayerId::new(0);

        // Place opponent president on intermediate area (area 1)
        state.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));

        // Path through area 1 to area 2
        let path = vec![AreaId::new(1), AreaId::new(2)];

        // Normal movement: costs 1 Yen for opponent president
        let normal_cost = calculate_movement_cost(&state, player, &path);
        assert_eq!(normal_cost, 1);

        // Mercenary movement: no cost for opponent president
        let merc_cost = calculate_movement_cost_mercenary(&state, player, &path);
        assert_eq!(merc_cost, 0);
    }

    #[test]
    fn test_mercenary_still_pays_canal_toll() {
        let mut state = create_test_state_with_board();
        let player = PlayerId::new(0);

        // Add a canal crossing
        state.board_layout.canal_crossings.push((AreaId::new(1), AreaId::new(2)));

        // Path through canal
        let path = vec![AreaId::new(1), AreaId::new(2)];

        // Normal movement: costs 1 Yen for canal
        let normal_cost = calculate_movement_cost(&state, player, &path);
        assert_eq!(normal_cost, 1);

        // Mercenary movement: still costs 1 Yen for canal (mercenaries pay canal tolls)
        let merc_cost = calculate_movement_cost_mercenary(&state, player, &path);
        assert_eq!(merc_cost, 1);
    }

    #[test]
    fn test_mercenary_with_opponent_and_canal() {
        let mut state = create_test_state_with_board();
        let player = PlayerId::new(0);

        // Place opponent president on intermediate area
        state.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));

        // Add a canal crossing
        state.board_layout.canal_crossings.push((AreaId::new(1), AreaId::new(2)));

        // Path through both opponent and canal
        let path = vec![AreaId::new(1), AreaId::new(2)];

        // Normal movement: 1 for opponent + 1 for canal = 2
        let normal_cost = calculate_movement_cost(&state, player, &path);
        assert_eq!(normal_cost, 2);

        // Mercenary movement: 0 for opponent + 1 for canal = 1
        let merc_cost = calculate_movement_cost_mercenary(&state, player, &path);
        assert_eq!(merc_cost, 1);
    }

    // --- Task 9: Canal crossing is a live, tolled move ------------------

    fn mover() -> PlayerId {
        PlayerId::new(0)
    }

    /// Minimal 2-area board: the mover's President sits at `start` (area 0),
    /// with an own assistant on `far` (area 1). `start` and `far` flank a
    /// Canal (no Canal-typed cell needs to exist in the fixture - only
    /// `adjacency`/`canal_crossings`, which the movement code actually
    /// consults, matter), so crossing between them is a single-hop path
    /// that tolls 1 Yen to the bank.
    fn canal_crossing_fixture() -> (GameState, Rules) {
        let config = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));

        let mut areas = BTreeMap::new();
        areas.insert(AreaId::new(0), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(AreaId::new(1), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 2) });

        let mut adjacency = BTreeMap::new();
        adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        adjacency.insert(AreaId::new(1), vec![AreaId::new(0)]);

        let board_layout = BoardLayout {
            areas,
            adjacency,
            canal_crossings: vec![(AreaId::new(0), AreaId::new(1))],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };

        let mut players = vec![
            PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
            PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
        ];
        players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        players[0].assistants.on_board.insert(AreaId::new(1), 1);
        players[0].money = 5;
        players[1].money = 5;

        let state = GameState {
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
            pending_choices: Vec::new(),
        };
        (state, Rules::empty_for_tests())
    }

    fn cross_move() -> Move {
        Move::MovePresident { path: vec![AreaId::new(1)], destination: AreaId::new(1) }
    }

    #[test]
    fn president_can_cross_canal_paying_one_yen_to_bank() {
        // Own assistant on the far side; path = [far]; start flanks the canal opposite it.
        let (state, rules) = canal_crossing_fixture();
        let legal = get_legal_moves(&state, &rules);
        assert!(legal.contains(&cross_move()));
        let s = apply_move(&state, &cross_move(), &rules).unwrap();
        assert_eq!(s.player(mover()).money, state.player(mover()).money - 1); // toll
        // Bank toll: NOT credited to any player.
        let others_total: u32 = s.players.iter().filter(|p| p.id != mover()).map(|p| p.money).sum();
        let others_before: u32 = state.players.iter().filter(|p| p.id != mover()).map(|p| p.money).sum();
        assert_eq!(others_total, others_before);
    }

    /// `start`(0) --canal--> `mid`(1) --ordinary adjacency--> `far`(2), with
    /// own assistants on both `mid` and `far` so the President's move is a
    /// two-step path [mid, far]. The canal crossing is on start -> path[0]
    /// (the FIRST step), a regression for the `path.windows(2)` gap that
    /// misses that edge (only pairs WITHIN `path` were ever scanned).
    fn canal_first_step_fixture() -> (GameState, Rules) {
        let (mut state, rules) = canal_crossing_fixture();
        state.board_layout.areas.insert(AreaId::new(2), AreaConfig { area_type: AreaType::SilkMill, grid_position: (0, 3) });
        state.board_layout.adjacency.entry(AreaId::new(1)).or_default().push(AreaId::new(2));
        state.board_layout.adjacency.insert(AreaId::new(2), vec![AreaId::new(1)]);
        state.players[0].assistants.on_board.insert(AreaId::new(2), 1);
        (state, rules)
    }

    fn first_step_cross_move() -> Move {
        Move::MovePresident { path: vec![AreaId::new(1), AreaId::new(2)], destination: AreaId::new(2) }
    }

    #[test]
    fn start_edge_crossing_is_tolled() {
        // The FIRST step of the path crosses the canal (regression for the
        // path.windows(2) gap that missed start -> path[0]).
        let (state, rules) = canal_first_step_fixture();
        let s = apply_move(&state, &first_step_cross_move(), &rules).unwrap();
        assert_eq!(s.player(mover()).money, state.player(mover()).money - 1);
    }

    /// `canal_crossing_fixture` plus the mover holding Stagecoach - which
    /// waives opponent-President pass-through/destination fees but must NOT
    /// waive the canal toll (a separate, unconditional charge).
    fn canal_crossing_with_stagecoach_fixture() -> (GameState, Rules) {
        let (mut state, mut rules) = canal_crossing_fixture();
        let stagecoach_id = TechCardId(1);
        let stagecoach = TechCard {
            id: stagecoach_id,
            name: "Stagecoach".to_string(),
            country: Country::Dutch,
            industry_cost: 2,
            effect: TechEffect::Stagecoach,
        };
        rules.techs.insert(stagecoach, "tech_stagecoach");
        state.players[0].technologies.push(stagecoach_id);
        (state, rules)
    }

    #[test]
    fn stagecoach_does_not_waive_canal_toll() {
        let (state, rules) = canal_crossing_with_stagecoach_fixture();
        let s = apply_move(&state, &cross_move(), &rules).unwrap();
        assert_eq!(s.player(mover()).money, state.player(mover()).money - 1);
    }

    // --- I3 final-review fix: minimum-TOTAL-cost path per destination ----

    /// `start`(0) --canal--> `direct_dest`(1), PLUS a longer, toll-free
    /// around-route 0-2-3-1 (ordinary adjacency, no canal on 0-2 or 2-3)
    /// reaching the SAME destination. The mover's own assistants sit on 1,
    /// 2, AND 3 -- a path must hold the mover's assistant at every hop,
    /// including the destination (`find_reachable_areas_with_tech`'s doc
    /// comment). BFS-shortest-HOP-COUNT picks the 1-hop canal path and
    /// tolls it even though the 3-hop around-route reaches area 1 for
    /// free -- the bug this fixture reproduces.
    fn canal_vs_around_fixture() -> (GameState, Rules) {
        let config = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));

        let mut areas = BTreeMap::new();
        areas.insert(AreaId::new(0), AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) });
        areas.insert(AreaId::new(1), AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, 3) });
        areas.insert(AreaId::new(2), AreaConfig { area_type: AreaType::SilkMill, grid_position: (0, 1) });
        areas.insert(AreaId::new(3), AreaConfig { area_type: AreaType::TeaPlantation, grid_position: (0, 2) });

        let mut adjacency = BTreeMap::new();
        adjacency.insert(AreaId::new(0), vec![AreaId::new(1), AreaId::new(2)]);
        adjacency.insert(AreaId::new(1), vec![AreaId::new(0), AreaId::new(3)]);
        adjacency.insert(AreaId::new(2), vec![AreaId::new(0), AreaId::new(3)]);
        adjacency.insert(AreaId::new(3), vec![AreaId::new(1), AreaId::new(2)]);

        let board_layout = BoardLayout {
            areas,
            adjacency,
            canal_crossings: vec![(AreaId::new(0), AreaId::new(1))],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
        };

        let mut players = vec![
            PlayerState::new(PlayerId::new(0), PlayerColor::Red, true),
            PlayerState::new(PlayerId::new(1), PlayerColor::Blue, false),
        ];
        players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        players[0].assistants.on_board.insert(AreaId::new(1), 1);
        players[0].assistants.on_board.insert(AreaId::new(2), 1);
        players[0].assistants.on_board.insert(AreaId::new(3), 1);
        players[0].money = 5;
        players[1].money = 5;

        let state = GameState {
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
            pending_choices: Vec::new(),
        };
        (state, Rules::empty_for_tests())
    }

    fn only_move_to(legal: &[Move], destination: AreaId) -> Move {
        let matches_dest: Vec<&Move> = legal.iter()
            .filter(|m| matches!(m, Move::MovePresident { destination: d, .. } if *d == destination))
            .collect();
        assert_eq!(matches_dest.len(), 1, "exactly one path must be offered per destination: {matches_dest:?}");
        matches_dest[0].clone()
    }

    #[test]
    fn movement_prefers_a_free_around_path_over_a_tolled_canal_shortcut() {
        let (state, rules) = canal_vs_around_fixture();
        let legal = get_legal_moves(&state, &rules);
        let mv = only_move_to(&legal, AreaId::new(1));
        assert_eq!(
            mv,
            Move::MovePresident { path: vec![AreaId::new(2), AreaId::new(3), AreaId::new(1)], destination: AreaId::new(1) },
            "must route around the canal for free instead of the tolled 1-hop shortcut"
        );
        let s = apply_move(&state, &mv, &rules).unwrap();
        assert_eq!(s.player(mover()).money, state.player(mover()).money, "no toll paid via the around-path");
    }

    /// Reverse pin: same direct-vs-around topology as `canal_vs_around_fixture`,
    /// but the around-route now ALSO crosses a canal (3 -> 1) and passes an
    /// opponent President at area 2 -- pricing it at 2 Yen, strictly worse
    /// than the direct route's 1 Yen toll. Proves the fix genuinely COMPARES
    /// total cost rather than unconditionally preferring the longer/
    /// non-direct route (which would be an overcorrection): the
    /// canal-crossing path must still be the one offered, and still tolled.
    fn canal_still_cheapest_fixture() -> (GameState, Rules) {
        let (mut state, rules) = canal_vs_around_fixture();
        state.board_layout.canal_crossings.push((AreaId::new(3), AreaId::new(1)));
        state.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(2));
        (state, rules)
    }

    #[test]
    fn movement_still_tolls_the_canal_when_it_is_genuinely_cheapest() {
        let (state, rules) = canal_still_cheapest_fixture();
        let legal = get_legal_moves(&state, &rules);
        let mv = only_move_to(&legal, AreaId::new(1));
        assert_eq!(
            mv,
            Move::MovePresident { path: vec![AreaId::new(1)], destination: AreaId::new(1) },
            "the direct canal crossing is still strictly cheaper than the now-pricier around-route"
        );
        let s = apply_move(&state, &mv, &rules).unwrap();
        assert_eq!(s.player(mover()).money, state.player(mover()).money - 1, "canal toll still charged");
    }
}
