//! Port and Laboratory actions.

use crate::types::{GameState, PlayerId, PortSlot, PortPayment, OrderCardId, TechCardId};
use crate::game::cards::{refill_port, refill_lab, TechRegistry};

/// Which Port slots are accessible at a given power level.
pub fn port_accessible_slots(power: u8) -> Vec<PortSlot> {
    match power {
        0..=1 => vec![],
        2 => vec![PortSlot::Low],
        3 => vec![PortSlot::Low, PortSlot::Mid],
        _ => vec![PortSlot::Low, PortSlot::Mid, PortSlot::High], // 4-5
    }
}

/// Does power 5 at Port grant bonus VP?
pub fn port_power_5_bonus_vp(power: u8) -> u8 {
    if power >= 5 { 3 } else { 0 }
}

/// Validate Port action.
pub fn validate_port_action(
    state: &GameState,
    player: PlayerId,
    cards_taken: &[PortSlot],
    payment: Option<PortPayment>,
    power: u8,
) -> Result<(), &'static str> {
    let p = state.player(player);
    let accessible = port_accessible_slots(power);

    // Check all slots are accessible
    for slot in cards_taken {
        if !accessible.contains(slot) {
            return Err("Slot not accessible at this power level");
        }
    }

    // Check hand limit
    if p.order_cards_in_hand.len() + cards_taken.len() > 3 {
        return Err("Would exceed Order card hand limit of 3");
    }

    // Check payment for second card
    if cards_taken.len() == 2 {
        match payment {
            None => return Err("Must pay for second card"),
            Some(PortPayment::TwoYen) => {
                if p.money < 2 {
                    return Err("Not enough money for second card");
                }
            }
            Some(PortPayment::OneImportedGood) => {
                if p.imported_goods < 1 {
                    return Err("Not enough imported goods for second card");
                }
            }
        }
    }

    // Can't take more than 2 cards
    if cards_taken.len() > 2 {
        return Err("Cannot take more than 2 cards from Port");
    }

    Ok(())
}

/// Execute Port action: take cards, pay for second, apply Power 5 VP bonus.
pub fn execute_port_action(
    state: &mut GameState,
    player: PlayerId,
    cards_taken: &[PortSlot],
    payment: Option<PortPayment>,
    power: u8,
    port_is_b: bool,
    order_deck: &mut Vec<OrderCardId>,
) -> Result<(), &'static str> {
    // Validate first
    validate_port_action(state, player, cards_taken, payment, power)?;

    // Pay for second card if taking 2
    if cards_taken.len() == 2 {
        let p = state.player_mut(player);
        match payment {
            Some(PortPayment::TwoYen) => p.money -= 2,
            Some(PortPayment::OneImportedGood) => p.imported_goods -= 1,
            None => unreachable!(), // Validated above
        }
    }

    // Get the appropriate port board
    let port = if port_is_b {
        match &mut state.port_state.port_b {
            Some(p) => p,
            None => return Err("Port B not available"),
        }
    } else {
        &mut state.port_state.port_a
    };

    // First, gather all cards to take (without sliding)
    let mut taken_cards = Vec::new();
    for &slot in cards_taken {
        let card = match slot {
            PortSlot::Low => port.low.take(),
            PortSlot::Mid => port.mid.take(),
            PortSlot::High => port.high.take(),
        };
        if let Some(card_id) = card {
            taken_cards.push(card_id);
        }
    }

    // Now slide cards left to fill gaps
    if port.low.is_none() && port.mid.is_some() {
        port.low = port.mid.take();
    }
    if port.mid.is_none() && port.high.is_some() {
        port.mid = port.high.take();
    }

    // Add cards to player's hand
    let p = state.player_mut(player);
    for card_id in taken_cards {
        p.order_cards_in_hand.push(card_id);
    }

    // Apply Power 5 VP bonus
    if power >= 5 {
        let p = state.player_mut(player);
        p.victory_points += 3;
    }

    // Refill port from deck
    let port = if port_is_b {
        state.port_state.port_b.as_mut().unwrap()
    } else {
        &mut state.port_state.port_a
    };
    refill_port(port, order_deck);

    Ok(())
}

/// Calculate industry value available for Laboratory.
pub fn calculate_industry_value(
    power: u8,
    extra_yen: u8,
    extra_imported: u8,
) -> u8 {
    // Power gives 1:1 industry value, max 5
    let from_power = power.min(5);
    // 2 Yen = 1 Industry Value
    let from_yen = extra_yen / 2;
    // 1 Imported Good = 1 Industry Value
    let from_imported = extra_imported;

    from_power + from_yen + from_imported
}

/// Cost for 5th+ technology card (penalty).
pub fn fifth_tech_penalty_required(tech_count: usize) -> bool {
    tech_count >= 4
}

/// Lab position surcharge: positions 0-1 are free, position 2 costs +1, position 3 costs +2.
pub fn lab_position_surcharge(slot_index: usize) -> u8 {
    match slot_index {
        0 | 1 => 0,
        2 => 1,
        3 => 2,
        _ => 0, // Out of bounds - shouldn't happen
    }
}

/// Validate Laboratory action.
#[allow(clippy::too_many_arguments)]
pub fn validate_lab_action(
    state: &GameState,
    player: PlayerId,
    slot_index: usize,
    extra_yen: u8,
    extra_imported: u8,
    fifth_tech_payment: Option<bool>,
    power: u8,
    tech_registry: &TechRegistry,
) -> Result<u8, &'static str> {
    let p = state.player(player);

    // Get the tech card from the appropriate slot
    let lab = if slot_index >= 4 {
        // slot_index 4-7 would be Lab B
        return Err("Invalid slot index");
    } else {
        &state.lab_state.lab_a
    };

    let tech_id = lab.slots[slot_index % 4].ok_or("No card in slot")?;
    let tech_card = tech_registry.get(tech_id).ok_or("Tech card not found in registry")?;

    // Calculate required industry value
    let required_iv = tech_card.industry_cost;
    let position_surcharge = lab_position_surcharge(slot_index % 4);
    let total_required = required_iv + position_surcharge;

    // Calculate available industry value
    let available_iv = calculate_industry_value(power, extra_yen, extra_imported);

    if available_iv < total_required {
        return Err("Insufficient industry value");
    }

    // Calculate total Yen cost
    let mut yen_cost = extra_yen as u32;

    // Check fifth tech penalty
    if fifth_tech_penalty_required(p.technologies.len()) {
        match fifth_tech_payment {
            None => return Err("Must pay fifth tech penalty"),
            Some(true) => yen_cost += 2, // Pay 2 Yen
            Some(false) => {
                // Pay 1 Imported Good - validate below
                if p.imported_goods < extra_imported as u32 + 1 {
                    return Err("Not enough imported goods for fifth tech penalty");
                }
            }
        }
    }

    // Validate payment
    if p.money < yen_cost {
        return Err("Not enough Yen");
    }

    let imported_cost = extra_imported as u32 + if fifth_tech_payment == Some(false) { 1 } else { 0 };
    if p.imported_goods < imported_cost {
        return Err("Not enough imported goods");
    }

    Ok(total_required)
}

/// Execute Laboratory action: acquire tech card, pay costs.
#[allow(clippy::too_many_arguments)]
pub fn execute_lab_action(
    state: &mut GameState,
    player: PlayerId,
    slot_index: usize,
    extra_yen: u8,
    extra_imported: u8,
    fifth_tech_payment: Option<bool>,
    power: u8,
    lab_is_b: bool,
    tech_registry: &TechRegistry,
    tech_deck: &mut Vec<TechCardId>,
) -> Result<(), &'static str> {
    // Validate first
    let _required_iv = validate_lab_action(
        state,
        player,
        slot_index,
        extra_yen,
        extra_imported,
        fifth_tech_payment,
        power,
        tech_registry,
    )?;

    // Check fifth tech penalty and calculate costs
    let yen_cost = extra_yen as u32
        + if fifth_tech_payment == Some(true) { 2 } else { 0 };
    let imported_cost = extra_imported as u32
        + if fifth_tech_payment == Some(false) { 1 } else { 0 };

    // Get the appropriate lab board
    let lab = if lab_is_b {
        match &mut state.lab_state.lab_b {
            Some(l) => l,
            None => return Err("Lab B not available"),
        }
    } else {
        &mut state.lab_state.lab_a
    };

    // Take the tech card
    let tech_id = lab.slots[slot_index].take().ok_or("No card in slot")?;

    // Slide remaining cards
    for i in slot_index..3 {
        if lab.slots[i].is_none() && lab.slots[i + 1].is_some() {
            lab.slots[i] = lab.slots[i + 1].take();
        }
    }

    // Pay costs
    let p = state.player_mut(player);
    p.money -= yen_cost;
    p.imported_goods -= imported_cost;

    // Add tech to player
    p.technologies.push(tech_id);

    // Refill lab
    let lab = if lab_is_b {
        state.lab_state.lab_b.as_mut().unwrap()
    } else {
        &mut state.lab_state.lab_a
    };
    refill_lab(lab, tech_deck);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::collections::HashMap;

    fn create_test_state() -> GameState {
        let config = GameConfig::new(
            2,
            vec![PlayerColor::Red, PlayerColor::Blue],
            PlayerId::new(0),
        );

        let board_layout = BoardLayout {
            areas: HashMap::new(),
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
            turn_phase: TurnPhase::AreaAction,
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
    fn test_port_accessible_slots() {
        assert!(port_accessible_slots(1).is_empty());
        assert_eq!(port_accessible_slots(2), vec![PortSlot::Low]);
        assert_eq!(port_accessible_slots(3), vec![PortSlot::Low, PortSlot::Mid]);
        assert_eq!(
            port_accessible_slots(4),
            vec![PortSlot::Low, PortSlot::Mid, PortSlot::High]
        );
    }

    #[test]
    fn test_port_power_5_bonus() {
        assert_eq!(port_power_5_bonus_vp(4), 0);
        assert_eq!(port_power_5_bonus_vp(5), 3);
    }

    #[test]
    fn test_port_hand_limit_validation() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        // Player already has 2 cards - can take 1 more
        state.players[0].order_cards_in_hand = vec![OrderCardId(1), OrderCardId(2)];
        let result = validate_port_action(&state, player, &[PortSlot::Low], None, 3);
        assert!(result.is_ok());

        // Player already has 2 cards - cannot take 2 more (would exceed 3)
        let result = validate_port_action(
            &state,
            player,
            &[PortSlot::Low, PortSlot::Mid],
            Some(PortPayment::TwoYen),
            3,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("hand limit"));

        // Player has 3 cards - cannot take any
        state.players[0].order_cards_in_hand = vec![OrderCardId(1), OrderCardId(2), OrderCardId(3)];
        let result = validate_port_action(&state, player, &[PortSlot::Low], None, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_port_second_card_payment_required() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        state.players[0].imported_goods = 2;

        // Taking 2 cards requires payment
        let result = validate_port_action(
            &state,
            player,
            &[PortSlot::Low, PortSlot::Mid],
            None, // No payment specified
            4,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Must pay"));

        // With 2 Yen payment - OK
        let result = validate_port_action(
            &state,
            player,
            &[PortSlot::Low, PortSlot::Mid],
            Some(PortPayment::TwoYen),
            4,
        );
        assert!(result.is_ok());

        // With 1 Imported Good payment - OK
        let result = validate_port_action(
            &state,
            player,
            &[PortSlot::Low, PortSlot::Mid],
            Some(PortPayment::OneImportedGood),
            4,
        );
        assert!(result.is_ok());

        // Not enough money for 2 Yen payment
        state.players[0].money = 1;
        let result = validate_port_action(
            &state,
            player,
            &[PortSlot::Low, PortSlot::Mid],
            Some(PortPayment::TwoYen),
            4,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Not enough money"));
    }

    #[test]
    fn test_port_execute_power_5_vp_bonus() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;

        // Set up port with cards
        state.port_state.port_a.low = Some(OrderCardId(10));
        state.port_state.port_a.mid = Some(OrderCardId(11));
        state.port_state.port_a.high = Some(OrderCardId(12));

        let mut deck: Vec<OrderCardId> = vec![OrderCardId(20), OrderCardId(21)];

        let initial_vp = state.players[0].victory_points;

        // Execute at power 5 - should get +3 VP
        let result = execute_port_action(
            &mut state,
            player,
            &[PortSlot::Low],
            None,
            5,
            false,
            &mut deck,
        );
        assert!(result.is_ok());
        assert_eq!(state.players[0].victory_points, initial_vp + 3);
    }

    #[test]
    fn test_port_execute_no_bonus_below_power_5() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;

        state.port_state.port_a.low = Some(OrderCardId(10));
        let mut deck: Vec<OrderCardId> = vec![OrderCardId(20)];

        let initial_vp = state.players[0].victory_points;

        // Execute at power 4 - should NOT get VP bonus
        let result = execute_port_action(
            &mut state,
            player,
            &[PortSlot::Low],
            None,
            4,
            false,
            &mut deck,
        );
        assert!(result.is_ok());
        assert_eq!(state.players[0].victory_points, initial_vp);
    }

    #[test]
    fn test_port_execute_takes_cards_to_hand() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;

        state.port_state.port_a.low = Some(OrderCardId(10));
        state.port_state.port_a.mid = Some(OrderCardId(11));
        let mut deck: Vec<OrderCardId> = vec![OrderCardId(20), OrderCardId(21)];

        let result = execute_port_action(
            &mut state,
            player,
            &[PortSlot::Low, PortSlot::Mid],
            Some(PortPayment::TwoYen),
            4,
            false,
            &mut deck,
        );
        assert!(result.is_ok());

        // Check cards are in hand
        assert!(state.players[0].order_cards_in_hand.contains(&OrderCardId(10)));
        assert!(state.players[0].order_cards_in_hand.contains(&OrderCardId(11)));

        // Check payment was deducted
        assert_eq!(state.players[0].money, 8); // 10 - 2
    }

    #[test]
    fn test_industry_value_calculation() {
        // Power only
        assert_eq!(calculate_industry_value(3, 0, 0), 3);

        // Power + Yen (2 yen = 1 IV)
        assert_eq!(calculate_industry_value(3, 2, 0), 4);
        assert_eq!(calculate_industry_value(3, 4, 0), 5);
        assert_eq!(calculate_industry_value(3, 1, 0), 3); // 1 yen not enough

        // Power + Imported Goods
        assert_eq!(calculate_industry_value(3, 0, 2), 5);

        // Combined
        assert_eq!(calculate_industry_value(3, 2, 1), 5);
    }

    #[test]
    fn test_fifth_tech_penalty() {
        assert!(!fifth_tech_penalty_required(0));
        assert!(!fifth_tech_penalty_required(3));
        assert!(fifth_tech_penalty_required(4)); // 5th card
        assert!(fifth_tech_penalty_required(5)); // 6th card
    }

    #[test]
    fn test_lab_position_surcharge() {
        assert_eq!(lab_position_surcharge(0), 0);
        assert_eq!(lab_position_surcharge(1), 0);
        assert_eq!(lab_position_surcharge(2), 1);
        assert_eq!(lab_position_surcharge(3), 2);
    }

    #[test]
    fn test_lab_validate_industry_value() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        state.players[0].imported_goods = 5;

        // Create a tech registry with a test card
        let mut tech_registry = TechRegistry::new();
        let tech_card = TechCard {
            id: TechCardId(1),
            name: "Test Tech".to_string(),
            country: Country::English,
            industry_cost: 3,
            effect: TechEffect::Tram, // Use any valid effect
        };
        tech_registry.insert(tech_card, "tech_test");

        // Put the tech in slot 0 (no surcharge)
        state.lab_state.lab_a.slots[0] = Some(TechCardId(1));

        // Power 3 = 3 IV, enough for card cost 3
        let result = validate_lab_action(
            &state, player, 0, 0, 0, None, 3, &tech_registry,
        );
        assert!(result.is_ok());

        // Put in slot 2 (surcharge +1) = needs 4 IV
        state.lab_state.lab_a.slots[2] = Some(TechCardId(1));
        let result = validate_lab_action(
            &state, player, 2, 0, 0, None, 3, &tech_registry,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Insufficient industry value"));

        // Pay 2 extra Yen for +1 IV
        let result = validate_lab_action(
            &state, player, 2, 2, 0, None, 3, &tech_registry,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_lab_fifth_tech_penalty() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        state.players[0].imported_goods = 5;

        // Give player 4 existing techs
        state.players[0].technologies = vec![
            TechCardId(10), TechCardId(11), TechCardId(12), TechCardId(13),
        ];

        // Create a tech registry with a test card
        let mut tech_registry = TechRegistry::new();
        let tech_card = TechCard {
            id: TechCardId(1),
            name: "Test Tech".to_string(),
            country: Country::English,
            industry_cost: 2,
            effect: TechEffect::Tram,
        };
        tech_registry.insert(tech_card, "tech_test");

        state.lab_state.lab_a.slots[0] = Some(TechCardId(1));

        // Without fifth tech payment - should fail
        let result = validate_lab_action(
            &state, player, 0, 0, 0, None, 3, &tech_registry,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("fifth tech penalty"));

        // With 2 Yen payment - should succeed
        let result = validate_lab_action(
            &state, player, 0, 0, 0, Some(true), 3, &tech_registry,
        );
        assert!(result.is_ok());

        // With 1 Imported Good payment - should succeed
        let result = validate_lab_action(
            &state, player, 0, 0, 0, Some(false), 3, &tech_registry,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_lab_execute_acquires_tech() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        state.players[0].imported_goods = 5;

        // Create a tech registry with a test card
        let mut tech_registry = TechRegistry::new();
        let tech_card = TechCard {
            id: TechCardId(1),
            name: "Test Tech".to_string(),
            country: Country::English,
            industry_cost: 2,
            effect: TechEffect::Tram,
        };
        tech_registry.insert(tech_card, "tech_test");

        state.lab_state.lab_a.slots[0] = Some(TechCardId(1));
        let mut deck: Vec<TechCardId> = vec![TechCardId(20)];

        let result = execute_lab_action(
            &mut state,
            player,
            0, // slot index
            0, // extra yen
            0, // extra imported
            None, // fifth tech payment
            3, // power
            false, // not lab B
            &tech_registry,
            &mut deck,
        );
        assert!(result.is_ok());

        // Check tech is in player's collection
        assert!(state.players[0].technologies.contains(&TechCardId(1)));
    }
}
