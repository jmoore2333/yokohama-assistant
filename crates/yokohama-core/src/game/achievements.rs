//! Achievement card validation and fulfillment.

use crate::types::*;

/// Check if player can claim an Achievement.
pub fn can_claim_achievement(
    state: &GameState,
    player: PlayerId,
    achievement: AchievementType,
) -> bool {
    let p = state.player(player);

    // Check if already claimed by this player
    for card in &state.achievements {
        if card.achievement_type == achievement && card.claimed_by.contains(&player) {
            return false;
        }
    }

    // Check if player has assistant in hand (required to place)
    if p.assistants.in_hand < 1 {
        return false;
    }

    // Check specific achievement requirements
    match achievement {
        // Category A: Trade good holdings (no payment, just demonstrate ownership)
        AchievementType::Copper5 => p.copper >= 5,
        AchievementType::Silk6 => p.silk >= 6,
        AchievementType::Tea7 => p.tea >= 7,
        AchievementType::Seafood7 => p.seafood >= 7,

        // Category B: Collection goals
        AchievementType::Money10 => p.money >= 10,
        AchievementType::Tokens5 => {
            p.five_power_tokens + p.mercenaries.total() >= 5
        }
        AchievementType::Orders5 => p.completed_orders.len() >= 5,
        AchievementType::Technologies4 => p.technologies.len() >= 4,

        // Category C: Building presence
        AchievementType::Production4 => {
            count_production_buildings(state, player) >= 4
        }
        AchievementType::Commercial4 => {
            count_commercial_buildings(state, player) >= 4
        }
        AchievementType::Prod3Comm2 => {
            count_production_buildings(state, player) >= 3
                && count_commercial_buildings(state, player) >= 2
        }
        AchievementType::Prod2Comm3 => {
            count_production_buildings(state, player) >= 2
                && count_commercial_buildings(state, player) >= 3
        }
    }
}

/// Count unique production areas where player has a building.
fn count_production_buildings(state: &GameState, player: PlayerId) -> usize {
    let p = state.player(player);
    let mut count = 0;

    for &area_id in p.shophouses.built.keys() {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_production() {
                count += 1;
            }
        }
    }

    for &area_id in &p.trading_houses.built {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_production()
                && !p.shophouses.built.contains_key(&area_id)
            {
                // Don't double-count if also have shophouse
                count += 1;
            }
        }
    }

    count
}

/// Count unique commercial areas where player has a building.
fn count_commercial_buildings(state: &GameState, player: PlayerId) -> usize {
    let p = state.player(player);
    let mut count = 0;

    for &area_id in p.shophouses.built.keys() {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_commercial() {
                count += 1;
            }
        }
    }

    for &area_id in &p.trading_houses.built {
        if let Some(config) = state.board_layout.areas.get(&area_id) {
            if config.area_type.is_commercial()
                && !p.shophouses.built.contains_key(&area_id)
            {
                count += 1;
            }
        }
    }

    count
}

/// Claim an Achievement.
pub fn claim_achievement(
    state: &mut GameState,
    player: PlayerId,
    achievement: AchievementType,
) -> Result<u32, &'static str> {
    if !can_claim_achievement(state, player, achievement) {
        return Err("Cannot claim achievement: requirements not met");
    }

    // Find the achievement card and add player to claimed list
    let mut vp_awarded = 0u32;
    for card in &mut state.achievements {
        if card.achievement_type == achievement {
            let is_first = card.claimed_by.is_empty();
            card.claimed_by.push(player);
            vp_awarded = if is_first {
                achievement.first_place_vp() as u32
            } else {
                achievement.subsequent_vp() as u32
            };
            break;
        }
    }

    // Lock an assistant on the achievement
    let p = state.player_mut(player);
    p.assistants.in_hand -= 1;
    p.assistants.on_achievements += 1;
    p.victory_points += vp_awarded;

    Ok(vp_awarded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_achievement_first_vs_subsequent() {
        assert_eq!(AchievementType::Copper5.first_place_vp(), 8);
        assert_eq!(AchievementType::Copper5.subsequent_vp(), 6);
    }
}
