//! Apply moves to game state.

use crate::types::*;
use crate::game::*;
use crate::game::actions::*;

/// Apply a move to the game state, returning the new state.
pub fn apply_move(state: &GameState, mv: &Move) -> Result<GameState, &'static str> {
    let mut new_state = state.clone();

    match mv {
        Move::EndAdditionalPhase => {
            advance_phase(&mut new_state);
        }

        Move::PlaceThreeAreas { placements } => {
            apply_placement(&mut new_state, placements)?;
            new_state.turn_phase = TurnPhase::Movement;
        }

        Move::PlaceTwoSameArea { area } => {
            apply_placement_same_area(&mut new_state, *area)?;
            new_state.turn_phase = TurnPhase::Movement;
        }

        Move::MovePresident { path, destination } => {
            apply_movement(&mut new_state, path, *destination)?;
            new_state.turn_phase = TurnPhase::AreaAction;
        }

        Move::PlacePresident { destination } => {
            apply_place_president(&mut new_state, *destination)?;
            new_state.turn_phase = TurnPhase::AreaAction;
        }

        Move::ReturnPresidentToHand { recover_assistants_from } => {
            apply_return_to_hand(&mut new_state, recover_assistants_from)?;
            // Skip to Additional Action Second
            new_state.turn_phase = TurnPhase::AdditionalActionSecond;
        }

        Move::ExecuteAreaAction { action } => {
            apply_area_action(&mut new_state, action)?;
            new_state.turn_phase = TurnPhase::FivePowerBonus;
        }

        Move::Take5PowerToken => {
            apply_take_5_power(&mut new_state)?;
            new_state.turn_phase = TurnPhase::Building;
        }

        Move::Skip5PowerBonus => {
            new_state.turn_phase = TurnPhase::Building;
        }

        Move::BuildShophouse { slot } => {
            apply_build_shophouse(&mut new_state, *slot)?;
            new_state.turn_phase = TurnPhase::Recovery;
        }

        Move::BuildTradingHouse => {
            apply_build_trading_house(&mut new_state)?;
            new_state.turn_phase = TurnPhase::Recovery;
        }

        Move::SkipBuilding => {
            new_state.turn_phase = TurnPhase::Recovery;
        }

        Move::UseMercenary { country, target_area } => {
            apply_mercenary(&mut new_state, *country, *target_area)?;
        }

        Move::FulfillOrder { order_id } => {
            apply_fulfill_order(&mut new_state, *order_id)?;
        }

        Move::FulfillAchievement { achievement } => {
            apply_fulfill_achievement(&mut new_state, *achievement)?;
        }
    }

    // Handle Recovery phase automatically
    if new_state.turn_phase == TurnPhase::Recovery {
        apply_recovery(&mut new_state);

        // Check if this was a mercenary sub-turn
        if new_state.is_mercenary_action {
            // Return to the Additional Action phase we came from
            if let Some(return_phase) = new_state.mercenary_return_phase {
                new_state.turn_phase = return_phase;
            } else {
                // Fallback: if return phase not set, go to AdditionalActionSecond
                new_state.turn_phase = TurnPhase::AdditionalActionSecond;
            }
            // Reset mercenary sub-turn state
            new_state.is_mercenary_action = false;
            new_state.mercenary_target_area = None;
            new_state.mercenary_return_phase = None;
            new_state.current_action_power = None;
        } else {
            // Normal turn flow: Recovery -> AdditionalActionSecond
            new_state.turn_phase = TurnPhase::AdditionalActionSecond;
        }
    }

    // Check end game triggers
    check_end_game_triggers(&mut new_state);

    Ok(new_state)
}

fn advance_phase(state: &mut GameState) {
    state.turn_phase = match state.turn_phase {
        TurnPhase::AdditionalActionFirst => TurnPhase::Placement,
        TurnPhase::AdditionalActionSecond => {
            // End of turn - handle end game logic
            let next_player = state.next_player();

            // Check if game should end after this turn
            if state.final_round && next_player == state.config.start_player {
                // Final round completed - game over
                return; // Stay in AdditionalActionSecond, game is over
            }

            // Check if we should start the final round
            if state.end_game_triggered && !state.final_round {
                // Current round completes when we return to start player
                if next_player == state.config.start_player {
                    state.final_round = true;
                }
            }

            // Advance to next player
            state.current_player = next_player;
            state.turn_number += 1;
            state.additional_action_state = AdditionalActionState::new();
            state.current_action_power = None;
            TurnPhase::AdditionalActionFirst
        }
        other => other, // Should not happen
    };
}

fn apply_placement(state: &mut GameState, placements: &[AreaId]) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player_mut(player);

    for &area in placements {
        if p.assistants.in_hand < 1 {
            return Err("Not enough assistants in hand");
        }
        p.assistants.in_hand -= 1;
        *p.assistants.on_board.entry(area).or_insert(0) += 1;

        // TODO: Check and pay President fees
    }

    Ok(())
}

fn apply_placement_same_area(state: &mut GameState, area: AreaId) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player_mut(player);

    if p.assistants.in_hand < 2 {
        return Err("Not enough assistants in hand");
    }

    p.assistants.in_hand -= 2;
    *p.assistants.on_board.entry(area).or_insert(0) += 2;

    // TODO: Check and pay President fees

    Ok(())
}

fn apply_movement(
    state: &mut GameState,
    path: &[AreaId],
    destination: AreaId,
) -> Result<(), &'static str> {
    let player = state.current_player;

    // Pay movement costs
    let cost = calculate_movement_cost(state, player, path);
    let p = state.player_mut(player);
    if p.money < cost {
        return Err("Not enough money for movement");
    }
    p.money -= cost;
    p.president_location = PresidentLocation::OnBoard(destination);

    // Calculate and store power for this action
    let power = calculate_power(state, player, destination);
    state.current_action_power = Some(power);

    Ok(())
}

fn apply_place_president(state: &mut GameState, destination: AreaId) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player_mut(player);
    p.president_location = PresidentLocation::OnBoard(destination);

    let power = calculate_power(state, player, destination);
    state.current_action_power = Some(power);

    Ok(())
}

fn apply_return_to_hand(
    state: &mut GameState,
    areas: &[AreaId],
) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player_mut(player);

    p.president_location = PresidentLocation::InHand;

    for &area in areas {
        if let Some(&count) = p.assistants.on_board.get(&area) {
            p.assistants.in_hand += count;
            p.assistants.on_board.remove(&area);
        }
    }

    Ok(())
}

fn apply_area_action(state: &mut GameState, action: &AreaAction) -> Result<(), &'static str> {
    let player = state.current_player;
    let power = state.current_action_power.unwrap_or(0);

    match action {
        AreaAction::ProduceCopper => {
            let reward = copper_mine_reward(power);
            state.player_mut(player).copper += reward as u32;
        }
        AreaAction::ProduceSilk => {
            let reward = silk_mill_reward(power);
            state.player_mut(player).silk += reward as u32;
        }
        AreaAction::ProduceTea => {
            let reward = tea_plantation_reward(power);
            state.player_mut(player).tea += reward as u32;
        }
        AreaAction::ProduceSeafood => {
            let reward = fishing_ground_reward(power);
            state.player_mut(player).seafood += reward as u32;
        }
        AreaAction::Bank => {
            let reward = bank_reward(power);
            state.player_mut(player).money += reward as u32;
        }

        // Employment Agency - retrieve items from warehouse
        AreaAction::EmploymentAgency { assistants, shophouses, trading_houses } => {
            apply_employment_agency(state, player, power, *assistants, *shophouses, *trading_houses)?;
        }

        // Other actions - simplified implementations
        AreaAction::Port { .. } => {
            // TODO: implement full Port action
        }
        AreaAction::Laboratory { .. } => {
            // TODO: implement full Laboratory action
        }
        AreaAction::Chinatown { exchanges } => {
            for exchange in exchanges {
                apply_chinatown_exchange(state, player, *exchange)?;
            }
        }
        AreaAction::Church { copper, silk, tea, seafood, yen, imported_good } => {
            // Validate and apply church action
            validate_church_action(state, player, *copper, *silk, *tea, *seafood, *yen, *imported_good)?;
            // TODO: implement full church placement
        }
        AreaAction::Customs { imported_goods_paid } => {
            validate_customs_action(state, player, *imported_goods_paid, power)?;
            // TODO: implement full customs placement
        }
    }

    Ok(())
}

/// Employment Agency: retrieve items from warehouse to hand/board.
fn apply_employment_agency(
    state: &mut GameState,
    player: PlayerId,
    power: u8,
    assistants: u8,
    shophouses: u8,
    trading_houses: u8,
) -> Result<(), &'static str> {
    let max_items = employment_agency_tokens_allowed(power);

    let total_items = assistants + shophouses + trading_houses;
    if total_items > max_items {
        return Err("Exceeds maximum items for power level");
    }

    let p = state.player(player);

    // Validate availability in warehouse
    if assistants > p.assistants.in_warehouse {
        return Err("Not enough assistants in warehouse");
    }
    if shophouses > p.shophouses.in_warehouse {
        return Err("Not enough shophouses in warehouse");
    }
    if trading_houses > p.trading_houses.in_warehouse {
        return Err("Not enough trading houses in warehouse");
    }

    // Calculate total cost
    let shophouse_cost = shophouses as u32 * 2; // 2 Yen each

    // Trading house cost: based on how many already built
    let already_built = p.trading_houses.total_built();
    let mut th_cost = 0u32;
    for i in 0..trading_houses {
        let th_index = already_built + i as usize;
        th_cost += match th_index {
            0 => 4,
            1 => 5,
            2 => 6,
            _ => 7,
        };
    }

    let total_cost = shophouse_cost + th_cost; // Assistants are FREE

    if p.money < total_cost {
        return Err("Not enough Yen to pay for items");
    }

    // Apply the action
    let p = state.player_mut(player);
    p.money -= total_cost;

    // Move assistants: warehouse -> hand
    p.assistants.in_warehouse -= assistants;
    p.assistants.in_hand += assistants;

    // Move shophouses: warehouse -> hand
    p.shophouses.in_warehouse -= shophouses;
    p.shophouses.in_hand += shophouses;

    Ok(())
}

fn apply_take_5_power(state: &mut GameState) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player(player);

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
        if let Some(effect_opt) = state.board_layout.five_power_tokens.get_mut(&area) {
            if let Some(effect) = effect_opt.take() {
                let p = state.player_mut(player);
                p.five_power_tokens += 1;

                // Apply effect
                match effect {
                    FivePowerEffect::ImportedGoods(n) => p.imported_goods += n as u32,
                    FivePowerEffect::Copper(n) => p.copper += n as u32,
                    FivePowerEffect::Silk(n) => p.silk += n as u32,
                    FivePowerEffect::Tea(n) => p.tea += n as u32,
                    FivePowerEffect::Seafood(n) => p.seafood += n as u32,
                    FivePowerEffect::Money(n) => p.money += n as u32,
                    FivePowerEffect::VictoryPoints(n) => p.victory_points += n as u32,
                    FivePowerEffect::Movement(n) => {
                        // Movement token: recover n assistants from board to hand
                        // This allows the player to reposition assistants on future turns
                        let mut remaining = n as u8;
                        let areas_with_assistants: Vec<AreaId> = p.assistants.on_board
                            .iter()
                            .filter(|(_, &count)| count > 0)
                            .map(|(&area, _)| area)
                            .collect();

                        for area in areas_with_assistants {
                            if remaining == 0 {
                                break;
                            }
                            if let Some(count) = p.assistants.on_board.get_mut(&area) {
                                let to_recover = (*count).min(remaining);
                                *count -= to_recover;
                                p.assistants.in_hand += to_recover;
                                remaining -= to_recover;
                            }
                        }
                        // Clean up empty entries
                        p.assistants.on_board.retain(|_, &mut count| count > 0);
                    }
                }
            }
        }
    }

    Ok(())
}

fn apply_build_shophouse(state: &mut GameState, slot: u8) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player(player);

    // For mercenary sub-turns, use the mercenary target area
    let area = if state.is_mercenary_action {
        state.mercenary_target_area.ok_or("No mercenary target area")?
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => a,
            _ => return Err("President not on board"),
        }
    };

    // Get VP reward
    let vp = state
        .board_layout
        .building_sites
        .get(&area)
        .map(|site| site.shophouse_rewards[slot as usize])
        .unwrap_or(0);

    let p = state.player_mut(player);
    if p.shophouses.in_hand < 1 {
        return Err("No shophouse in hand");
    }
    p.shophouses.in_hand -= 1;
    p.shophouses.built.insert(area, slot);
    p.victory_points += vp as u32;

    // Update building site
    if let Some(site) = state.board_layout.building_sites.get_mut(&area) {
        site.shophouses_built[slot as usize] = Some(player);
    }

    Ok(())
}

fn apply_build_trading_house(state: &mut GameState) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player(player);

    // For mercenary sub-turns, use the mercenary target area
    let area = if state.is_mercenary_action {
        state.mercenary_target_area.ok_or("No mercenary target area")?
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => a,
            _ => return Err("President not on board"),
        }
    };

    let vp = state
        .board_layout
        .building_sites
        .get(&area)
        .map(|site| site.trading_house_reward)
        .unwrap_or(0);

    let p = state.player_mut(player);
    if p.trading_houses.in_warehouse < 1 {
        return Err("No trading house in warehouse");
    }

    let cost = p.trading_houses.next_cost();
    if p.money < cost as u32 {
        return Err("Not enough money for trading house");
    }

    p.money -= cost as u32;
    p.trading_houses.in_warehouse -= 1;
    p.trading_houses.built.push(area);
    p.victory_points += vp as u32;

    if let Some(site) = state.board_layout.building_sites.get_mut(&area) {
        site.trading_house_owner = Some(player);
    }

    Ok(())
}

/// Apply recovery phase, optionally leaving 1 assistant (Telephone effect).
fn apply_recovery(state: &mut GameState) {
    apply_recovery_with_leave(state, false);
}

/// Apply recovery with explicit control over leaving an assistant.
/// If `leave_one` is true and player has Telephone tech, leaves 1 assistant.
pub fn apply_recovery_with_leave(state: &mut GameState, leave_one: bool) {
    let player = state.current_player;
    let p = state.player(player);

    // During mercenary sub-turn, recover from mercenary target area
    // During normal turn, recover from president's location
    let recovery_area = if state.is_mercenary_action {
        state.mercenary_target_area
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => Some(a),
            PresidentLocation::InHand => None,
        }
    };

    if let Some(area) = recovery_area {
        let p = state.player_mut(player);
        if let Some(&count) = p.assistants.on_board.get(&area) {
            if leave_one && count > 1 {
                // Telephone effect: leave exactly 1 assistant
                p.assistants.in_hand += count - 1;
                p.assistants.on_board.insert(area, 1);
            } else {
                // Normal recovery: take all assistants
                p.assistants.in_hand += count;
                p.assistants.on_board.remove(&area);
            }
        }
    }
}

fn apply_mercenary(
    state: &mut GameState,
    country: Country,
    target_area: AreaId,
) -> Result<(), &'static str> {
    let player = state.current_player;
    let p = state.player(player);

    // Validate: mercenary not already used this turn
    if state.additional_action_state.mercenary_used {
        return Err("Mercenary already used this turn");
    }

    // Validate: player has this mercenary available
    let available = p.mercenaries.available.get(&country).copied().unwrap_or(0);
    if available == 0 {
        return Err("No available mercenary of this country");
    }

    // Validate: target area has player's assistant but NOT president
    let assistant_count = p.assistants.on_board.get(&target_area).copied().unwrap_or(0);
    if assistant_count == 0 {
        return Err("Target area must have your assistant");
    }
    if let PresidentLocation::OnBoard(loc) = p.president_location {
        if loc == target_area {
            return Err("Target area cannot have your president");
        }
    }

    // Calculate power at target area for the mercenary action
    // Power = Assistants + Shophouse + TradingHouse + Station (NO president)
    let mut power: u8 = assistant_count;
    if p.shophouses.built.contains_key(&target_area) {
        power += 1;
    }
    if p.trading_houses.built.contains(&target_area) {
        power += 1;
    }
    if state.station_location == Some(target_area) {
        power += 1;
    }
    power = power.min(5);

    // Consume the mercenary (flip face-down)
    let p_mut = state.player_mut(player);
    *p_mut.mercenaries.available.entry(country).or_insert(0) -= 1;
    *p_mut.mercenaries.used.entry(country).or_insert(0) += 1;

    // Mark mercenary as used for this turn
    state.additional_action_state.mercenary_used = true;

    // Store return phase (where to go after sub-turn completes)
    state.mercenary_return_phase = Some(state.turn_phase);

    // Set up mercenary sub-turn state
    state.is_mercenary_action = true;
    state.mercenary_target_area = Some(target_area);
    state.current_action_power = Some(power);

    // Transition to AreaAction phase
    state.turn_phase = TurnPhase::AreaAction;

    Ok(())
}

fn apply_fulfill_order(_state: &mut GameState, _order_id: OrderCardId) -> Result<(), &'static str> {
    // TODO: Implement order fulfillment when OrderRegistry is available
    Ok(())
}

fn apply_fulfill_achievement(
    _state: &mut GameState,
    _achievement: AchievementType,
) -> Result<(), &'static str> {
    // TODO: Implement achievement fulfillment
    Ok(())
}

fn check_end_game_triggers(state: &mut GameState) {
    if state.end_game_triggered {
        return;
    }

    let player = state.current_player;

    // Check Trading Houses (4 built)
    for p in &state.players {
        if p.trading_houses.total_built() >= 4 {
            state.end_game_triggered = true;
            state.trigger_reason = Some(EndGameTrigger::TradingHousesComplete(p.id));
            state.trigger_player = Some(p.id);
            return;
        }
    }

    // Check Shophouses (8 built)
    for p in &state.players {
        if p.shophouses.total_built() >= 8 {
            state.end_game_triggered = true;
            state.trigger_reason = Some(EndGameTrigger::ShophousesComplete(p.id));
            state.trigger_player = Some(p.id);
            return;
        }
    }

    // Check Church track
    if state.church_track.total_assistants() >= state.config.track_trigger() {
        state.end_game_triggered = true;
        state.trigger_reason = Some(EndGameTrigger::ChurchTrackFull);
        state.trigger_player = Some(player);
    }

    // Check Customs track
    if state.customs_track.total_assistants() >= state.config.track_trigger() {
        state.end_game_triggered = true;
        state.trigger_reason = Some(EndGameTrigger::CustomsTrackFull);
        state.trigger_player = Some(player);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_minimal_state(player_count: u8) -> GameState {
        let colors = match player_count {
            2 => vec![PlayerColor::Red, PlayerColor::Blue],
            3 => vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow],
            4 => vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green],
            _ => vec![PlayerColor::Red, PlayerColor::Blue],
        };
        let actual_count = colors.len() as u8;

        let config = GameConfig::new(actual_count, colors, PlayerId::new(0));

        let board_layout = BoardLayout {
            areas: HashMap::new(),
            adjacency: HashMap::new(),
            canal_crossings: vec![],
            building_sites: HashMap::new(),
            five_power_tokens: HashMap::new(),
        };

        let players: Vec<PlayerState> = (0..actual_count)
            .map(|i| {
                let color = match i {
                    0 => PlayerColor::Red,
                    1 => PlayerColor::Blue,
                    2 => PlayerColor::Yellow,
                    _ => PlayerColor::Green,
                };
                PlayerState::new(PlayerId::new(i), color, i == 0)
            })
            .collect();

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
            port_state: PortState::new(player_count >= 3),
            lab_state: LabState::new(player_count >= 4),
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
    fn test_phase_advance() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;

        advance_phase(&mut state);
        assert_eq!(state.turn_phase, TurnPhase::Placement);
    }

    #[test]
    fn test_placement_decrements_hand() {
        let mut state = create_minimal_state(2);

        // Add an area
        state.board_layout.areas.insert(AreaId::new(0), AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });

        let initial_hand = state.player(PlayerId::new(0)).assistants.in_hand;

        let result = apply_placement(&mut state, &[AreaId::new(0)]);
        assert!(result.is_ok());

        assert_eq!(
            state.player(PlayerId::new(0)).assistants.in_hand,
            initial_hand - 1
        );
    }

    #[test]
    fn test_mercenary_sub_turn_setup() {
        let mut state = create_minimal_state(2);
        let player = state.current_player;
        let target = AreaId::new(5);

        // Give player a mercenary and assistant at target area
        {
            let p = state.player_mut(player);
            p.mercenaries.available.insert(Country::English, 1);
            p.assistants.on_board.insert(target, 2);
            p.president_location = PresidentLocation::OnBoard(AreaId::new(1));
        }

        let result = apply_mercenary(&mut state, Country::English, target);
        assert!(result.is_ok());

        // Verify sub-turn state
        assert!(state.is_mercenary_action);
        assert_eq!(state.mercenary_target_area, Some(target));
        assert_eq!(state.turn_phase, TurnPhase::AreaAction);
    }

    #[test]
    fn test_recovery_normal() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);

        // Place president and 3 assistants
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].assistants.on_board.insert(area, 3);
        state.players[0].assistants.in_hand = 5;

        apply_recovery_with_leave(&mut state, false);

        // All 3 assistants should be recovered
        assert_eq!(state.players[0].assistants.in_hand, 8);
        assert_eq!(state.players[0].assistants.on_board.get(&area), None);
    }

    #[test]
    fn test_recovery_with_telephone() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);

        // Place president and 3 assistants
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].assistants.on_board.insert(area, 3);
        state.players[0].assistants.in_hand = 5;

        // With Telephone effect (leave_one = true)
        apply_recovery_with_leave(&mut state, true);

        // Only 2 assistants should be recovered, 1 remains
        assert_eq!(state.players[0].assistants.in_hand, 7);
        assert_eq!(state.players[0].assistants.on_board.get(&area), Some(&1));
    }

    #[test]
    fn test_recovery_with_telephone_only_one_assistant() {
        let mut state = create_minimal_state(2);
        let area = AreaId::new(0);

        // Place president and only 1 assistant
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].assistants.on_board.insert(area, 1);
        state.players[0].assistants.in_hand = 5;

        // With Telephone effect but only 1 assistant - recovers it
        apply_recovery_with_leave(&mut state, true);

        // The 1 assistant should be recovered (can't leave 1 if only have 1)
        assert_eq!(state.players[0].assistants.in_hand, 6);
        assert_eq!(state.players[0].assistants.on_board.get(&area), None);
    }

    #[test]
    fn test_five_power_movement_token() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);

        // Place president and set up 5-power token with Movement effect
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        state.players[0].assistants.on_board.insert(AreaId::new(1), 2);
        state.players[0].assistants.on_board.insert(AreaId::new(2), 1);
        state.players[0].assistants.in_hand = 3;
        state.current_action_power = Some(5);
        state.turn_phase = TurnPhase::FivePowerBonus;

        // Add Movement(2) token on action area
        state.board_layout.five_power_tokens.insert(action_area, Some(FivePowerEffect::Movement(2)));

        // Take the token
        let result = apply_move(&state, &Move::Take5PowerToken);
        assert!(result.is_ok());

        let new_state = result.unwrap();

        // Should have recovered 2 assistants from board
        assert_eq!(new_state.players[0].assistants.in_hand, 5); // 3 + 2 recovered
        assert_eq!(new_state.players[0].five_power_tokens, 1);

        // Token should be consumed
        assert_eq!(
            new_state.board_layout.five_power_tokens.get(&action_area),
            Some(&None)
        );
    }

    #[test]
    fn test_five_power_movement_token_partial() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);

        // Place president
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        // Only 1 assistant on board but Movement(3) token
        state.players[0].assistants.on_board.insert(AreaId::new(1), 1);
        state.players[0].assistants.in_hand = 5;
        state.current_action_power = Some(5);
        state.turn_phase = TurnPhase::FivePowerBonus;

        // Add Movement(3) token - but only 1 assistant available
        state.board_layout.five_power_tokens.insert(action_area, Some(FivePowerEffect::Movement(3)));

        let result = apply_move(&state, &Move::Take5PowerToken);
        assert!(result.is_ok());

        let new_state = result.unwrap();

        // Should have recovered only 1 assistant (all that was available)
        assert_eq!(new_state.players[0].assistants.in_hand, 6); // 5 + 1 recovered
        assert!(new_state.players[0].assistants.on_board.get(&AreaId::new(1)).is_none()
            || *new_state.players[0].assistants.on_board.get(&AreaId::new(1)).unwrap() == 0);
    }

    #[test]
    fn test_five_power_other_effects() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);

        // Place president
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        state.current_action_power = Some(5);
        state.turn_phase = TurnPhase::FivePowerBonus;

        // Players start with: 1 copper, 1 silk, 1 tea, 1 seafood, 3 money (start player)
        // Test various token effects - expected values are base + bonus
        let test_cases = vec![
            (FivePowerEffect::Copper(2), 1 + 2, 1, 3, 0u32),       // +2 copper
            (FivePowerEffect::Silk(1), 1, 1 + 1, 3, 0),            // +1 silk
            (FivePowerEffect::Money(3), 1, 1, 3 + 3, 0),           // +3 money
            (FivePowerEffect::VictoryPoints(5), 1, 1, 3, 0 + 5),   // +5 VP
        ];

        for (effect, exp_copper, exp_silk, exp_money, exp_vp) in test_cases {
            let mut test_state = state.clone();
            test_state.board_layout.five_power_tokens.insert(action_area, Some(effect));

            let result = apply_move(&test_state, &Move::Take5PowerToken);
            assert!(result.is_ok());

            let new_state = result.unwrap();
            assert_eq!(new_state.players[0].copper, exp_copper);
            assert_eq!(new_state.players[0].silk, exp_silk);
            assert_eq!(new_state.players[0].money, exp_money);
            assert_eq!(new_state.players[0].victory_points, exp_vp);
        }
    }
}
