//! Port and Laboratory actions.

use crate::types::{GameState, PlayerId, PortSlot, PortPayment};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
