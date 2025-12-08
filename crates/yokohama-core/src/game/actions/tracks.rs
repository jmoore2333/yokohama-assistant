//! Church and Customs track actions.

use crate::types::{GameState, PlayerId};

/// Calculate Faith value from power.
/// CRITICAL: Power 2 gives only 1 Faith (not 2). This is per rulebook p.14.
/// Mapping: 1→1, 2→1, 3→3, 4→4, 5→5
pub fn church_faith_from_power(power: u8) -> u8 {
    match power {
        0 => 0,
        1 => 1,
        2 => 1, // CRITICAL: power 2 gives 1 Faith, not 2 (rulebook p.14)
        3 => 3,
        4 => 4,
        _ => 5,
    }
}

/// Calculate total Faith for Church action.
pub fn calculate_church_faith(
    power: u8,
    copper: bool,
    silk: bool,
    tea: bool,
    seafood: bool,
    yen: bool,
    imported_good: bool,
) -> u8 {
    let mut faith = church_faith_from_power(power);

    // Each good type contributes max 1 Faith
    if copper { faith += 1; }
    if silk { faith += 1; }
    if tea { faith += 1; }
    if seafood { faith += 1; }
    if yen { faith += 1; }
    if imported_good { faith += 1; }

    faith
}

/// Validate Church action.
#[allow(clippy::too_many_arguments)]
pub fn validate_church_action(
    state: &GameState,
    player: PlayerId,
    copper: bool,
    silk: bool,
    tea: bool,
    seafood: bool,
    yen: bool,
    imported_good: bool,
) -> Result<(), &'static str> {
    let p = state.player(player);

    if copper && p.copper < 1 {
        return Err("Not enough copper");
    }
    if silk && p.silk < 1 {
        return Err("Not enough silk");
    }
    if tea && p.tea < 1 {
        return Err("Not enough tea");
    }
    if seafood && p.seafood < 1 {
        return Err("Not enough seafood");
    }
    if yen && p.money < 1 {
        return Err("Not enough money");
    }
    if imported_good && p.imported_goods < 1 {
        return Err("Not enough imported goods");
    }

    // Check assistant available
    if p.assistants.in_hand < 1 {
        return Err("No assistant in hand for Church");
    }

    Ok(())
}

/// Maximum imported goods payable at Customs based on power.
pub fn customs_max_imported_goods(power: u8) -> u8 {
    match power {
        0..=1 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        _ => 4,
    }
}

/// Validate Customs action.
pub fn validate_customs_action(
    state: &GameState,
    player: PlayerId,
    imported_goods_to_pay: u8,
    power: u8,
) -> Result<(), &'static str> {
    let p = state.player(player);

    let max_allowed = customs_max_imported_goods(power);
    if imported_goods_to_pay > max_allowed {
        return Err("Exceeds maximum imported goods for power level");
    }

    if imported_goods_to_pay > p.imported_goods as u8 {
        return Err("Not enough imported goods");
    }

    if imported_goods_to_pay < 1 {
        return Err("Must pay at least 1 imported good");
    }

    // Check assistant available
    if p.assistants.in_hand < 1 {
        return Err("No assistant in hand for Customs");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_church_faith_from_power() {
        assert_eq!(church_faith_from_power(1), 1);
        assert_eq!(church_faith_from_power(2), 1); // Special case
        assert_eq!(church_faith_from_power(3), 3);
        assert_eq!(church_faith_from_power(4), 4);
        assert_eq!(church_faith_from_power(5), 5);
    }

    #[test]
    fn test_calculate_church_faith() {
        // Power 3 only
        assert_eq!(calculate_church_faith(3, false, false, false, false, false, false), 3);

        // Power 3 + all goods (max 6 extra)
        assert_eq!(calculate_church_faith(3, true, true, true, true, true, true), 9);

        // Power 5 + 2 goods
        assert_eq!(calculate_church_faith(5, true, true, false, false, false, false), 7);
    }

    #[test]
    fn test_customs_max_imported() {
        assert_eq!(customs_max_imported_goods(1), 0);
        assert_eq!(customs_max_imported_goods(2), 1);
        assert_eq!(customs_max_imported_goods(3), 2);
        assert_eq!(customs_max_imported_goods(4), 3);
        assert_eq!(customs_max_imported_goods(5), 4);
    }

    #[test]
    fn test_power_zero_edge_cases() {
        assert_eq!(church_faith_from_power(0), 0);
        assert_eq!(customs_max_imported_goods(0), 0);
    }

    #[test]
    fn test_power_above_five_capped() {
        assert_eq!(church_faith_from_power(6), 5);
        assert_eq!(customs_max_imported_goods(6), 4);
    }
}
