//! Technology card effect helpers.

use crate::types::*;
use super::cards::TechRegistry;

/// Check if player has a specific technology effect.
pub fn has_tech_effect(
    state: &GameState,
    player: PlayerId,
    effect: TechEffect,
    tech_registry: &TechRegistry,
) -> bool {
    let p = state.player(player);
    p.technologies.iter().any(|&tech_id| {
        tech_registry.get(tech_id).map(|t| t.effect == effect).unwrap_or(false)
    })
}

/// Apply turn-start effects (System of Stock).
pub fn apply_turn_start_effects(
    state: &mut GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) {
    // System of Stock: if Yen <= 1, refill to 2
    if has_tech_effect(state, player, TechEffect::SystemOfStock, tech_registry) {
        let p = state.player_mut(player);
        if p.money <= 1 {
            p.money = 2;
        }
    }
}

/// Calculate building VP bonus from Gaslight/Electrical Light.
pub fn building_vp_bonus(
    state: &GameState,
    player: PlayerId,
    is_shophouse: bool,
    tech_registry: &TechRegistry,
) -> u32 {
    if is_shophouse {
        if has_tech_effect(state, player, TechEffect::Gaslight, tech_registry) {
            2
        } else {
            0
        }
    } else {
        // Trading House
        if has_tech_effect(state, player, TechEffect::ElectricalLight, tech_registry) {
            3
        } else {
            0
        }
    }
}

/// Calculate copper bonus from Mining Technology.
pub fn copper_bonus(
    state: &GameState,
    player: PlayerId,
    base_copper: u8,
    tech_registry: &TechRegistry,
) -> u8 {
    if base_copper > 0 && has_tech_effect(state, player, TechEffect::MiningTechnology, tech_registry) {
        1
    } else {
        0
    }
}

/// Calculate placement limit (3 or 4 with Newspaper).
pub fn placement_limit(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> u8 {
    if has_tech_effect(state, player, TechEffect::Newspaper, tech_registry) {
        4
    } else {
        3
    }
}

/// Check if player can stop in area with opponent President (Postal System).
pub fn can_stop_with_opponent(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> bool {
    has_tech_effect(state, player, TechEffect::PostalSystem, tech_registry)
}

/// Check if player can pass through opponents without paying (Stagecoach).
pub fn free_passage(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> bool {
    has_tech_effect(state, player, TechEffect::Stagecoach, tech_registry)
}

/// Check if player can leave 1 assistant during recovery (Telephone).
pub fn can_leave_assistant(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> bool {
    has_tech_effect(state, player, TechEffect::Telephone, tech_registry)
}

/// Technology acquisition bonuses.
pub fn tech_acquisition_bonuses(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> (u32, u32) {
    let vp_bonus = if has_tech_effect(state, player, TechEffect::PatentSystem, tech_registry) {
        2
    } else {
        0
    };

    let yen_bonus = if has_tech_effect(state, player, TechEffect::LetterPressPrinting, tech_registry) {
        1
    } else {
        0
    };

    (vp_bonus, yen_bonus)
}

/// Church placement bonus (Winery).
pub fn church_vp_bonus(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> u32 {
    if has_tech_effect(state, player, TechEffect::Winery, tech_registry) {
        3
    } else {
        0
    }
}

/// Customs placement bonus (Brickyard).
pub fn customs_vp_bonus(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> u32 {
    if has_tech_effect(state, player, TechEffect::Brickyard, tech_registry) {
        4
    } else {
        0
    }
}
