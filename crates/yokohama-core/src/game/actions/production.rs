//! Production area actions (Copper Mine, Silk Mill, Tea Plantation, Fishing Ground).

use crate::types::{GameState, PlayerId};

/// Result of a production action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionResult {
    pub copper: u8,
    pub silk: u8,
    pub tea: u8,
    pub seafood: u8,
}

impl ProductionResult {
    pub fn empty() -> Self {
        Self { copper: 0, silk: 0, tea: 0, seafood: 0 }
    }
}

/// Calculate Copper Mine rewards based on power.
pub fn copper_mine_reward(power: u8) -> u8 {
    match power {
        0..=2 => 0,
        3 => 1,
        4 => 2,
        _ => 3, // 5+
    }
}

/// Calculate Silk Mill rewards based on power.
pub fn silk_mill_reward(power: u8) -> u8 {
    match power {
        0..=1 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        _ => 4, // 5+
    }
}

/// Calculate Tea Plantation rewards based on power.
pub fn tea_plantation_reward(power: u8) -> u8 {
    power.min(5) // 1:1, capped at 5
}

/// Calculate Fishing Ground rewards based on power.
pub fn fishing_ground_reward(power: u8) -> u8 {
    power.min(5) // 1:1, capped at 5
}

/// Apply production rewards to player state, accounting for Mining Technology.
pub fn apply_production(
    state: &mut GameState,
    player: PlayerId,
    result: &ProductionResult,
) {
    let player_state = &mut state.players[player.0 as usize];

    // Check for Mining Technology bonus
    // TODO: implement proper tech lookup when TechRegistry is available
    let has_mining_tech = false;

    let copper_bonus = if has_mining_tech && result.copper > 0 { 1 } else { 0 };

    player_state.copper += result.copper as u32 + copper_bonus as u32;
    player_state.silk += result.silk as u32;
    player_state.tea += result.tea as u32;
    player_state.seafood += result.seafood as u32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copper_mine_rewards() {
        assert_eq!(copper_mine_reward(1), 0);
        assert_eq!(copper_mine_reward(2), 0);
        assert_eq!(copper_mine_reward(3), 1);
        assert_eq!(copper_mine_reward(4), 2);
        assert_eq!(copper_mine_reward(5), 3);
    }

    #[test]
    fn test_silk_mill_rewards() {
        assert_eq!(silk_mill_reward(1), 0);
        assert_eq!(silk_mill_reward(2), 1);
        assert_eq!(silk_mill_reward(3), 2);
        assert_eq!(silk_mill_reward(4), 3);
        assert_eq!(silk_mill_reward(5), 4);
    }

    #[test]
    fn test_tea_plantation_rewards() {
        for power in 1..=5 {
            assert_eq!(tea_plantation_reward(power), power);
        }
    }

    #[test]
    fn test_fishing_ground_rewards() {
        for power in 1..=5 {
            assert_eq!(fishing_ground_reward(power), power);
        }
    }

    #[test]
    fn test_power_zero_returns_zero() {
        // All production areas should return 0 at power 0
        assert_eq!(copper_mine_reward(0), 0);
        assert_eq!(silk_mill_reward(0), 0);
        assert_eq!(tea_plantation_reward(0), 0);
        assert_eq!(fishing_ground_reward(0), 0);
    }

    #[test]
    fn test_power_capped_at_five() {
        // Power above 5 should be treated as 5
        assert_eq!(copper_mine_reward(6), 3);
        assert_eq!(silk_mill_reward(6), 4);
        assert_eq!(tea_plantation_reward(6), 5);
        assert_eq!(fishing_ground_reward(6), 5);
    }
}
