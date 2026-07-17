//! Apply moves to game state.

use crate::types::*;
use crate::game::*;
use crate::game::actions::*;

/// Apply a move to the game state, returning the new state.
pub fn apply_move(state: &GameState, mv: &Move) -> Result<GameState, MoveError> {
    apply_move_with_registries(state, mv, None)
}

/// Apply a move with access to card registries for order/tech fulfillment.
pub fn apply_move_with_registries(
    state: &GameState,
    mv: &Move,
    order_registry: Option<&OrderRegistry>,
) -> Result<GameState, MoveError> {
    let mut new_state = state.clone();

    match mv {
        Move::EndAdditionalPhase => {
            advance_phase(&mut new_state);
        }

        Move::PlaceThreeAreas { placements } => {
            apply_placement(&mut new_state, placements)?;
            // Record president's starting area for this turn's movement
            set_president_start_area(&mut new_state);
            new_state.turn_phase = TurnPhase::Movement;
        }

        Move::PlaceTwoSameArea { area, adjacent_bonus } => {
            apply_placement_same_area(&mut new_state, *area)?;
            // Telegram bonus: place 1 in adjacent area
            if let Some(adj_area) = adjacent_bonus {
                let p = new_state.player_mut(new_state.current_player);
                if p.assistants.in_hand > 0 {
                    p.assistants.in_hand -= 1;
                    *p.assistants.on_board.entry(*adj_area).or_insert(0) += 1;
                }
            }
            // Record president's starting area for this turn's movement
            set_president_start_area(&mut new_state);
            new_state.turn_phase = TurnPhase::Movement;
        }

        Move::MovePresident { path, destination } => {
            // Validate: cannot return to starting area
            validate_movement_destination(&new_state, *destination)?;
            apply_movement(&mut new_state, path, *destination)?;
            new_state.president_start_area = None; // Clear after movement
            new_state.turn_phase = TurnPhase::AreaAction;
        }

        Move::PlacePresident { destination } => {
            // Note: PlacePresident is for placing from hand - no start area restriction
            apply_place_president(&mut new_state, *destination)?;
            new_state.president_start_area = None; // Clear after placement
            new_state.turn_phase = TurnPhase::AreaAction;
        }

        Move::ReturnPresidentToHand { recover_assistants_from } => {
            apply_return_to_hand(&mut new_state, recover_assistants_from)?;
            new_state.president_start_area = None; // Clear after return
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
            apply_fulfill_order(&mut new_state, *order_id, order_registry)?;
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

/// Set president's starting area for movement restriction.
/// Called when entering Movement phase.
fn set_president_start_area(state: &mut GameState) {
    let player = state.current_player;
    let p = state.player(player);

    state.president_start_area = match p.president_location {
        PresidentLocation::OnBoard(area) => Some(area),
        PresidentLocation::InHand => None,
    };
}

/// Validate that movement destination is not the starting area.
/// "The President pawn may not return to the Area that it started in."
fn validate_movement_destination(state: &GameState, destination: AreaId) -> Result<(), MoveError> {
    if let Some(start) = state.president_start_area {
        if destination == start {
            return Err(MoveError::RuleViolation("Cannot return to starting area in same turn"));
        }
    }
    Ok(())
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

fn apply_placement(state: &mut GameState, placements: &[AreaId]) -> Result<(), MoveError> {
    let player = state.current_player;
    let p = state.player_mut(player);

    for &area in placements {
        if p.assistants.in_hand < 1 {
            return Err(MoveError::RuleViolation("Not enough assistants in hand"));
        }
        p.assistants.in_hand -= 1;
        *p.assistants.on_board.entry(area).or_insert(0) += 1;

        // TODO: Check and pay President fees
    }

    Ok(())
}

fn apply_placement_same_area(state: &mut GameState, area: AreaId) -> Result<(), MoveError> {
    let player = state.current_player;
    let p = state.player_mut(player);

    if p.assistants.in_hand < 2 {
        return Err(MoveError::RuleViolation("Not enough assistants in hand"));
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
) -> Result<(), MoveError> {
    let player = state.current_player;

    // Pay movement costs
    let cost = calculate_movement_cost(state, player, path);
    let p = state.player_mut(player);
    if p.money < cost {
        return Err(MoveError::RuleViolation("Not enough money for movement"));
    }
    p.money -= cost;
    p.president_location = PresidentLocation::OnBoard(destination);

    // Calculate and store power for this action
    let power = calculate_power(state, player, destination);
    state.current_action_power = Some(power);

    Ok(())
}

fn apply_place_president(state: &mut GameState, destination: AreaId) -> Result<(), MoveError> {
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
) -> Result<(), MoveError> {
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

fn apply_area_action(state: &mut GameState, action: &AreaAction) -> Result<(), MoveError> {
    let player = state.current_player;
    let power = state.current_action_power.unwrap_or(0);

    // Pay Trading House passive income to owner (if not the current player)
    apply_trading_house_passive_income(state);

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
                apply_chinatown_exchange(state, player, *exchange).map_err(MoveError::RuleViolation)?;
            }
        }
        AreaAction::Church { copper, silk, tea, seafood, yen, imported_good } => {
            // Validate and apply church action
            validate_church_action(state, player, *copper, *silk, *tea, *seafood, *yen, *imported_good)
                .map_err(MoveError::RuleViolation)?;
            // TODO: implement full church placement
        }
        AreaAction::Customs { imported_goods_paid } => {
            validate_customs_action(state, player, *imported_goods_paid, power).map_err(MoveError::RuleViolation)?;
            // TODO: implement full customs placement
        }
    }

    Ok(())
}

/// Pay passive income to Trading House owner (if not the current player).
/// When a player takes an area action in an area with another player's Trading House,
/// that Trading House owner receives 1 Yen.
fn apply_trading_house_passive_income(state: &mut GameState) {
    let player = state.current_player;
    let p = state.player(player);

    // Determine the action area (president location or mercenary target)
    let area = if state.is_mercenary_action {
        state.mercenary_target_area
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => Some(a),
            PresidentLocation::InHand => None,
        }
    };

    if let Some(area) = area {
        // Check if there's a trading house in this area owned by another player
        if let Some(site) = state.board_layout.building_sites.get(&area) {
            if let Some(th_owner) = site.trading_house_owner {
                // Pay 1 Yen to the owner if it's not the current player
                if th_owner != player {
                    state.player_mut(th_owner).money += 1;
                }
            }
        }
    }
}

/// Employment Agency: retrieve items from warehouse to hand/board.
fn apply_employment_agency(
    state: &mut GameState,
    player: PlayerId,
    power: u8,
    assistants: u8,
    shophouses: u8,
    trading_houses: u8,
) -> Result<(), MoveError> {
    let max_items = employment_agency_tokens_allowed(power);

    let total_items = assistants + shophouses + trading_houses;
    if total_items > max_items {
        return Err(MoveError::RuleViolation("Exceeds maximum items for power level"));
    }

    let p = state.player(player);

    // Validate availability in warehouse
    if assistants > p.assistants.in_warehouse {
        return Err(MoveError::RuleViolation("Not enough assistants in warehouse"));
    }
    if shophouses > p.shophouses.in_warehouse {
        return Err(MoveError::RuleViolation("Not enough shophouses in warehouse"));
    }
    if trading_houses > p.trading_houses.in_warehouse {
        return Err(MoveError::RuleViolation("Not enough trading houses in warehouse"));
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
        return Err(MoveError::RuleViolation("Not enough Yen to pay for items"));
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

fn apply_take_5_power(state: &mut GameState) -> Result<(), MoveError> {
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

fn apply_build_shophouse(state: &mut GameState, slot: u8) -> Result<(), MoveError> {
    let player = state.current_player;
    let p = state.player(player);

    // For mercenary sub-turns, use the mercenary target area
    let area = if state.is_mercenary_action {
        state.mercenary_target_area.ok_or(MoveError::RuleViolation("No mercenary target area"))?
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => a,
            _ => return Err(MoveError::RuleViolation("President not on board")),
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
        return Err(MoveError::RuleViolation("No shophouse in hand"));
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

fn apply_build_trading_house(state: &mut GameState) -> Result<(), MoveError> {
    let player = state.current_player;
    let p = state.player(player);

    // For mercenary sub-turns, use the mercenary target area
    let area = if state.is_mercenary_action {
        state.mercenary_target_area.ok_or(MoveError::RuleViolation("No mercenary target area"))?
    } else {
        match p.president_location {
            PresidentLocation::OnBoard(a) => a,
            _ => return Err(MoveError::RuleViolation("President not on board")),
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
        return Err(MoveError::RuleViolation("No trading house in warehouse"));
    }

    let cost = p.trading_houses.next_cost();
    if p.money < cost as u32 {
        return Err(MoveError::RuleViolation("Not enough money for trading house"));
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
) -> Result<(), MoveError> {
    let player = state.current_player;
    let p = state.player(player);

    // Validate: mercenary not already used this turn
    if state.additional_action_state.mercenary_used {
        return Err(MoveError::RuleViolation("Mercenary already used this turn"));
    }

    // Validate: player has this mercenary available
    let available = p.mercenaries.available.get(&country).copied().unwrap_or(0);
    if available == 0 {
        return Err(MoveError::RuleViolation("No available mercenary of this country"));
    }

    // Validate: target area has player's assistant but NOT president
    let assistant_count = p.assistants.on_board.get(&target_area).copied().unwrap_or(0);
    if assistant_count == 0 {
        return Err(MoveError::RuleViolation("Target area must have your assistant"));
    }
    if let PresidentLocation::OnBoard(loc) = p.president_location {
        if loc == target_area {
            return Err(MoveError::RuleViolation("Target area cannot have your president"));
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

fn apply_fulfill_order(
    state: &mut GameState,
    order_id: OrderCardId,
    order_registry: Option<&OrderRegistry>,
) -> Result<(), MoveError> {
    let player = state.current_player;
    let p = state.player(player);

    // Verify order is in player's hand
    if !p.order_cards_in_hand.contains(&order_id) {
        return Err(MoveError::RuleViolation("Order card not in hand"));
    }

    // Look up order details from registry
    let order = match order_registry {
        Some(registry) => match registry.get(order_id) {
            Some(o) => o,
            None => return Err(MoveError::RuleViolation("Order card not found in registry")),
        },
        None => return Err(MoveError::RuleViolation("OrderRegistry required for order fulfillment")),
    };

    // Clone requirements and rewards before mutable borrow
    let req = order.requirements;
    let rewards = order.rewards;

    // Check if player has required trade goods
    if p.copper < req.copper as u32
        || p.silk < req.silk as u32
        || p.tea < req.tea as u32
        || p.seafood < req.seafood as u32
    {
        return Err(MoveError::RuleViolation("Insufficient trade goods to fulfill order"));
    }

    // Deduct trade goods
    let p = state.player_mut(player);
    p.copper -= req.copper as u32;
    p.silk -= req.silk as u32;
    p.tea -= req.tea as u32;
    p.seafood -= req.seafood as u32;

    // Award rewards
    p.victory_points += rewards.victory_points as u32;
    p.money += rewards.money as u32;
    p.imported_goods += rewards.imported_goods as u32;
    p.copper += rewards.copper as u32;
    p.silk += rewards.silk as u32;
    p.tea += rewards.tea as u32;
    p.seafood += rewards.seafood as u32;

    // Handle any_trade_good rewards (player chooses - for now add to copper)
    // In a full implementation, this would be a separate move type
    p.copper += rewards.any_trade_good as u32;

    // Move order from hand to completed
    p.order_cards_in_hand.retain(|&id| id != order_id);
    p.completed_orders.push(order_id);

    Ok(())
}

fn apply_fulfill_achievement(
    state: &mut GameState,
    achievement: AchievementType,
) -> Result<(), MoveError> {
    let player = state.current_player;
    claim_achievement(state, player, achievement).map_err(MoveError::RuleViolation)?;
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
            president_start_area: None,
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
    fn test_mercenary_once_per_turn() {
        let mut state = create_minimal_state(2);
        let player = state.current_player;
        let target1 = AreaId::new(5);
        let target2 = AreaId::new(6);

        // Give player two mercenaries and assistants at both target areas
        {
            let p = state.player_mut(player);
            p.mercenaries.available.insert(Country::English, 1);
            p.mercenaries.available.insert(Country::French, 1);
            p.assistants.on_board.insert(target1, 2);
            p.assistants.on_board.insert(target2, 2);
            p.president_location = PresidentLocation::OnBoard(AreaId::new(1));
        }

        // First use should succeed
        assert!(!state.additional_action_state.mercenary_used);
        let result = apply_mercenary(&mut state, Country::English, target1);
        assert!(result.is_ok());
        assert!(state.additional_action_state.mercenary_used);

        // Reset mercenary state (simulating completing the sub-turn)
        state.is_mercenary_action = false;
        state.mercenary_target_area = None;
        state.turn_phase = TurnPhase::AdditionalActionSecond;

        // Second use should fail (once per turn)
        // The mercenary_used flag should prevent this
        assert!(state.additional_action_state.mercenary_used);

        // Verify flag persists across both additional action phases
        state.turn_phase = TurnPhase::AdditionalActionFirst;
        assert!(state.additional_action_state.mercenary_used);
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

    #[test]
    fn test_trading_house_passive_income() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);

        // Player 0 is current player, Player 1 owns trading house at action area
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        state.players[0].assistants.on_board.insert(action_area, 2);
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(3);
        state.turn_phase = TurnPhase::AreaAction;

        // Add building site with Player 1's trading house
        let mut site = BuildingSite::new([2, 3, 4, 5], 10);
        site.trading_house_owner = Some(PlayerId::new(1));
        state.board_layout.building_sites.insert(action_area, site);
        state.board_layout.areas.insert(action_area, AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 0),
        });

        // Record initial money
        let player1_initial_money = state.players[1].money;

        // Player 0 takes action
        let result = apply_move(&state, &Move::ExecuteAreaAction {
            action: AreaAction::ProduceCopper,
        });
        assert!(result.is_ok());

        let new_state = result.unwrap();

        // Player 1 should have received 1 Yen passive income
        assert_eq!(new_state.players[1].money, player1_initial_money + 1);
    }

    #[test]
    fn test_trading_house_no_passive_income_own_building() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(0);

        // Player 0 owns their own trading house at the action area
        state.players[0].president_location = PresidentLocation::OnBoard(action_area);
        state.players[0].assistants.on_board.insert(action_area, 2);
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(3);
        state.turn_phase = TurnPhase::AreaAction;

        // Add building site with Player 0's own trading house
        let mut site = BuildingSite::new([2, 3, 4, 5], 10);
        site.trading_house_owner = Some(PlayerId::new(0)); // Own building
        state.board_layout.building_sites.insert(action_area, site);
        state.board_layout.areas.insert(action_area, AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 0),
        });

        // Record initial money for player 1 (we only check that player 1 is unchanged)
        let player1_initial_money = state.players[1].money;

        // Player 0 takes action
        let result = apply_move(&state, &Move::ExecuteAreaAction {
            action: AreaAction::ProduceCopper,
        });
        assert!(result.is_ok());

        let new_state = result.unwrap();

        // Neither player should receive passive income (player 0 owns their own building)
        // Player 0's money may change due to the action, but player 1 should be unchanged
        assert_eq!(new_state.players[1].money, player1_initial_money);
    }

    #[test]
    fn test_trading_house_passive_income_mercenary_action() {
        let mut state = create_minimal_state(2);
        let action_area = AreaId::new(5);

        // Set up mercenary action at area where Player 1 owns trading house
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(action_area);
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(2);
        state.turn_phase = TurnPhase::AreaAction;

        state.players[0].assistants.on_board.insert(action_area, 1);

        // Add building site with Player 1's trading house
        let mut site = BuildingSite::new([2, 3, 4, 5], 10);
        site.trading_house_owner = Some(PlayerId::new(1));
        state.board_layout.building_sites.insert(action_area, site);
        state.board_layout.areas.insert(action_area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 5),
        });

        let player1_initial_money = state.players[1].money;

        // Player 0 takes mercenary action
        let result = apply_move(&state, &Move::ExecuteAreaAction {
            action: AreaAction::Bank,
        });
        assert!(result.is_ok());

        let new_state = result.unwrap();

        // Player 1 should receive passive income even during mercenary action
        assert_eq!(new_state.players[1].money, player1_initial_money + 1);
    }

    #[test]
    fn test_cannot_return_to_start_area() {
        let mut state = create_minimal_state(2);

        // Set up board with connected areas: 0 -- 1 -- 2
        state.board_layout.areas.insert(AreaId::new(0), AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.board_layout.areas.insert(AreaId::new(1), AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 1),
        });
        state.board_layout.areas.insert(AreaId::new(2), AreaConfig {
            area_type: AreaType::SilkMill,
            grid_position: (0, 2),
        });
        state.board_layout.adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        state.board_layout.adjacency.insert(AreaId::new(1), vec![AreaId::new(0), AreaId::new(2)]);
        state.board_layout.adjacency.insert(AreaId::new(2), vec![AreaId::new(1)]);

        // President starts in area 0
        state.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        state.players[0].assistants.on_board.insert(AreaId::new(0), 1);
        state.players[0].assistants.on_board.insert(AreaId::new(1), 1);

        // Simulate entering Movement phase (set start area)
        state.turn_phase = TurnPhase::Placement;
        let result = apply_move(&state, &Move::PlaceThreeAreas { placements: vec![] });
        assert!(result.is_ok());
        let state = result.unwrap();

        // president_start_area should be set to area 0
        assert_eq!(state.president_start_area, Some(AreaId::new(0)));
        assert_eq!(state.turn_phase, TurnPhase::Movement);

        // Try to move back to area 0 (should fail)
        let result = apply_move(&state, &Move::MovePresident {
            path: vec![AreaId::new(1), AreaId::new(0)],
            destination: AreaId::new(0),
        });
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(MoveError::RuleViolation("Cannot return to starting area in same turn"))
        ));
    }

    #[test]
    fn test_can_move_to_different_area() {
        let mut state = create_minimal_state(2);

        // Set up board
        state.board_layout.areas.insert(AreaId::new(0), AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.board_layout.areas.insert(AreaId::new(1), AreaConfig {
            area_type: AreaType::CopperMine,
            grid_position: (0, 1),
        });
        state.board_layout.adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        state.board_layout.adjacency.insert(AreaId::new(1), vec![AreaId::new(0)]);

        // President starts in area 0
        state.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        state.players[0].assistants.on_board.insert(AreaId::new(0), 1);
        state.players[0].assistants.on_board.insert(AreaId::new(1), 1);

        // Enter Movement phase
        state.turn_phase = TurnPhase::Placement;
        let result = apply_move(&state, &Move::PlaceThreeAreas { placements: vec![] });
        assert!(result.is_ok());
        let state = result.unwrap();

        // Move to area 1 (different from start) - should succeed
        let result = apply_move(&state, &Move::MovePresident {
            path: vec![AreaId::new(1)],
            destination: AreaId::new(1),
        });
        assert!(result.is_ok());

        let new_state = result.unwrap();
        // president_start_area should be cleared after movement
        assert_eq!(new_state.president_start_area, None);
        assert_eq!(new_state.turn_phase, TurnPhase::AreaAction);
    }

    #[test]
    fn test_president_from_hand_no_start_restriction() {
        let mut state = create_minimal_state(2);

        // President is in hand
        state.players[0].president_location = PresidentLocation::InHand;
        state.players[0].assistants.on_board.insert(AreaId::new(0), 1);

        state.board_layout.areas.insert(AreaId::new(0), AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });

        // Enter Movement phase (no start area since president is in hand)
        state.turn_phase = TurnPhase::Placement;
        let result = apply_move(&state, &Move::PlaceThreeAreas { placements: vec![] });
        assert!(result.is_ok());
        let state = result.unwrap();

        // No start area should be set
        assert_eq!(state.president_start_area, None);

        // Place president - should succeed (no start area restriction)
        let result = apply_move(&state, &Move::PlacePresident {
            destination: AreaId::new(0),
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_fulfill_order_awards_vp() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;

        // Give player the required resources
        state.players[0].copper = 2;
        state.players[0].silk = 1;

        // Create order card
        let order_id = OrderCardId(42);
        state.players[0].order_cards_in_hand.push(order_id);

        // Create order registry with the card
        let mut order_registry = OrderRegistry::new();
        let order_card = OrderCard {
            id: order_id,
            country: Country::English,
            requirements: TradeGoodRequirement::new(2, 1, 0, 0), // 2 copper, 1 silk
            rewards: OrderRewards {
                victory_points: 5,
                money: 3,
                imported_goods: 0,
                copper: 0,
                silk: 0,
                tea: 0,
                seafood: 0,
                movement: 0,
                any_trade_good: 0,
                warehouse_tokens: 0,
            },
        };
        order_registry.insert(order_card, "test_order");

        // Fulfill order
        let result = apply_move_with_registries(
            &state,
            &Move::FulfillOrder { order_id },
            Some(&order_registry),
        );
        assert!(result.is_ok());

        let new_state = result.unwrap();

        // Verify VP awarded
        assert_eq!(new_state.players[0].victory_points, 5);
        // Verify money awarded
        assert_eq!(new_state.players[0].money, 3 + state.players[0].money); // 3 reward + starting money
        // Verify trade goods deducted
        assert_eq!(new_state.players[0].copper, 0); // 2 - 2 = 0
        assert_eq!(new_state.players[0].silk, 0);   // 1 - 1 = 0
        // Verify order moved to completed
        assert!(!new_state.players[0].order_cards_in_hand.contains(&order_id));
        assert!(new_state.players[0].completed_orders.contains(&order_id));
    }

    #[test]
    fn test_fulfill_order_requires_resources() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;

        // Give player INSUFFICIENT resources
        state.players[0].copper = 1; // Need 2
        state.players[0].silk = 1;

        let order_id = OrderCardId(42);
        state.players[0].order_cards_in_hand.push(order_id);

        let mut order_registry = OrderRegistry::new();
        let order_card = OrderCard {
            id: order_id,
            country: Country::English,
            requirements: TradeGoodRequirement::new(2, 1, 0, 0),
            rewards: OrderRewards::default(),
        };
        order_registry.insert(order_card, "test_order");

        // Fulfill order should fail
        let result = apply_move_with_registries(
            &state,
            &Move::FulfillOrder { order_id },
            Some(&order_registry),
        );
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(MoveError::RuleViolation("Insufficient trade goods to fulfill order"))
        ));
    }

    #[test]
    fn test_fulfill_achievement_awards_vp() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;

        // Give player enough copper for Copper5 achievement
        state.players[0].copper = 5;
        state.players[0].assistants.in_hand = 2;

        // Fulfill achievement
        let result = apply_move(&state, &Move::FulfillAchievement {
            achievement: AchievementType::Copper5,
        });
        assert!(result.is_ok());

        let new_state = result.unwrap();

        // Verify VP awarded (first place = 8 VP for Category A)
        assert_eq!(new_state.players[0].victory_points, 8);
        // Verify assistant locked on achievement
        assert_eq!(new_state.players[0].assistants.in_hand, 1);
        assert_eq!(new_state.players[0].assistants.on_achievements, 1);
        // Verify player added to claimed list
        assert!(new_state.achievements.iter().any(|a|
            a.achievement_type == AchievementType::Copper5 &&
            a.claimed_by.contains(&PlayerId::new(0))
        ));
    }

    #[test]
    fn test_fulfill_achievement_subsequent_player() {
        let mut state = create_minimal_state(2);
        state.turn_phase = TurnPhase::AdditionalActionFirst;

        // First player claims achievement
        state.players[0].copper = 5;
        state.players[0].assistants.in_hand = 2;
        let state = apply_move(&state, &Move::FulfillAchievement {
            achievement: AchievementType::Copper5,
        }).unwrap();

        // Switch to player 2
        let mut state = state;
        state.current_player = PlayerId::new(1);
        state.players[1].copper = 5;
        state.players[1].assistants.in_hand = 2;

        // Second player claims same achievement
        let result = apply_move(&state, &Move::FulfillAchievement {
            achievement: AchievementType::Copper5,
        });
        assert!(result.is_ok());

        let new_state = result.unwrap();

        // Verify subsequent VP (6 VP for Category A)
        assert_eq!(new_state.players[1].victory_points, 6);
    }
}
