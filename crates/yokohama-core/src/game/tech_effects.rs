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

/// Check if player can use Spinning Mill this turn.
/// Requires: has tech, has at least 1 silk, hasn't used this turn.
pub fn can_use_spinning_mill(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> bool {
    has_tech_effect(state, player, TechEffect::SpinningMill, tech_registry)
        && state.player(player).silk >= 1
        && !state.additional_action_state.spinning_mill_used
}

/// Apply Spinning Mill effect: discard 1 silk → gain 2 Yen.
pub fn apply_spinning_mill(
    state: &mut GameState,
    player: PlayerId,
) -> Result<(), &'static str> {
    let p = state.player_mut(player);
    if p.silk < 1 {
        return Err("Not enough silk");
    }
    p.silk -= 1;
    p.money += 2;
    state.additional_action_state.spinning_mill_used = true;
    Ok(())
}

/// Check if player can use University this turn.
/// Requires: has tech, has at least 1 Yen, has assistants in warehouse, hasn't used this turn.
pub fn can_use_university(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> bool {
    let p = state.player(player);
    has_tech_effect(state, player, TechEffect::University, tech_registry)
        && p.money >= 1
        && p.assistants.in_warehouse > 0
        && !state.additional_action_state.university_used
}

/// Apply University effect: pay 1 Yen → take assistant from warehouse to hand.
pub fn apply_university(
    state: &mut GameState,
    player: PlayerId,
) -> Result<(), &'static str> {
    let p = state.player_mut(player);
    if p.money < 1 {
        return Err("Not enough money");
    }
    if p.assistants.in_warehouse < 1 {
        return Err("No assistants in warehouse");
    }
    p.money -= 1;
    p.assistants.in_warehouse -= 1;
    p.assistants.in_hand += 1;
    state.additional_action_state.university_used = true;
    Ok(())
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

/// Church placement bonus (Winery): +3 VP per owned Winery copy — stacks
/// per physical copy the player owns (Task 13, consistent with the Lab
/// acquisition bonus precedent for Patent System/Letter Press Printing).
pub fn church_vp_bonus(
    state: &GameState,
    player: PlayerId,
    tech_registry: &TechRegistry,
) -> u32 {
    let p = state.player(player);
    let winery_copies = p.technologies.iter()
        .filter(|&&tech_id| tech_registry.get(tech_id).map(|t| t.effect == TechEffect::Winery).unwrap_or(false))
        .count() as u32;
    winery_copies * 3
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

#[cfg(test)]
mod tests {
    use super::*;
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
            turn_phase: TurnPhase::Building,
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
    fn test_gaslight_shophouse_bonus() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        // Create registry with Gaslight tech
        let mut tech_registry = TechRegistry::new();
        let gaslight = TechCard {
            id: TechCardId(1),
            name: "Gaslight".to_string(),
            country: Country::English,
            industry_cost: 3,
            effect: TechEffect::Gaslight,
        };
        tech_registry.insert(gaslight, "tech_gaslight");

        // Without tech - no bonus
        assert_eq!(building_vp_bonus(&state, player, true, &tech_registry), 0);

        // Give player the tech
        state.players[0].technologies.push(TechCardId(1));

        // With Gaslight - +2 VP for shophouse
        assert_eq!(building_vp_bonus(&state, player, true, &tech_registry), 2);
    }

    #[test]
    fn test_electrical_light_trading_house_bonus() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        // Create registry with Electrical Light tech
        let mut tech_registry = TechRegistry::new();
        let electrical = TechCard {
            id: TechCardId(2),
            name: "Electrical Light".to_string(),
            country: Country::American,
            industry_cost: 4,
            effect: TechEffect::ElectricalLight,
        };
        tech_registry.insert(electrical, "tech_electrical");

        // Without tech - no bonus
        assert_eq!(building_vp_bonus(&state, player, false, &tech_registry), 0);

        // Give player the tech
        state.players[0].technologies.push(TechCardId(2));

        // With Electrical Light - +3 VP for trading house
        assert_eq!(building_vp_bonus(&state, player, false, &tech_registry), 3);
    }

    #[test]
    fn test_winery_church_bonus_stacks_per_owned_copy() {
        // Standard data has only 1 physical Winery, so stacking is proven
        // with a hand-rolled two-copy registry fixture (Task 13, mirrors
        // Task 12's Patent System stacking precedent).
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        let mut tech_registry = TechRegistry::new();
        let winery_1 = TechCard {
            id: TechCardId(3),
            name: "Winery".to_string(),
            country: Country::French,
            industry_cost: 2,
            effect: TechEffect::Winery,
        };
        let winery_2 = TechCard {
            id: TechCardId(30),
            name: "Winery".to_string(),
            country: Country::French,
            industry_cost: 2,
            effect: TechEffect::Winery,
        };
        tech_registry.insert(winery_1, "tech_winery_1");
        tech_registry.insert(winery_2, "tech_winery_2");

        // Without tech
        assert_eq!(church_vp_bonus(&state, player, &tech_registry), 0);

        // With one owned copy
        state.players[0].technologies.push(TechCardId(3));
        assert_eq!(church_vp_bonus(&state, player, &tech_registry), 3);

        // With both owned copies - stacks
        state.players[0].technologies.push(TechCardId(30));
        assert_eq!(church_vp_bonus(&state, player, &tech_registry), 6);
    }

    #[test]
    fn test_brickyard_customs_bonus() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        let mut tech_registry = TechRegistry::new();
        let brickyard = TechCard {
            id: TechCardId(4),
            name: "Brickyard".to_string(),
            country: Country::German,
            industry_cost: 3,
            effect: TechEffect::Brickyard,
        };
        tech_registry.insert(brickyard, "tech_brickyard");

        // Without tech
        assert_eq!(customs_vp_bonus(&state, player, &tech_registry), 0);

        // With tech
        state.players[0].technologies.push(TechCardId(4));
        assert_eq!(customs_vp_bonus(&state, player, &tech_registry), 4);
    }

    #[test]
    fn test_mining_technology_bonus() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        let mut tech_registry = TechRegistry::new();
        let mining = TechCard {
            id: TechCardId(5),
            name: "Mining Technology".to_string(),
            country: Country::Dutch,
            industry_cost: 2,
            effect: TechEffect::MiningTechnology,
        };
        tech_registry.insert(mining, "tech_mining");

        // Without tech - no bonus
        assert_eq!(copper_bonus(&state, player, 3, &tech_registry), 0);

        // With tech - +1 copper when receiving copper
        state.players[0].technologies.push(TechCardId(5));
        assert_eq!(copper_bonus(&state, player, 3, &tech_registry), 1);

        // But not if base copper is 0
        assert_eq!(copper_bonus(&state, player, 0, &tech_registry), 0);
    }

    #[test]
    fn test_tech_acquisition_bonuses() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        let mut tech_registry = TechRegistry::new();
        let patent = TechCard {
            id: TechCardId(6),
            name: "Patent System".to_string(),
            country: Country::English,
            industry_cost: 3,
            effect: TechEffect::PatentSystem,
        };
        let letterpress = TechCard {
            id: TechCardId(7),
            name: "Letter Press Printing".to_string(),
            country: Country::American,
            industry_cost: 2,
            effect: TechEffect::LetterPressPrinting,
        };
        tech_registry.insert(patent, "tech_patent");
        tech_registry.insert(letterpress, "tech_letterpress");

        // Without tech - no bonuses
        let (vp, yen) = tech_acquisition_bonuses(&state, player, &tech_registry);
        assert_eq!(vp, 0);
        assert_eq!(yen, 0);

        // With Patent System - +2 VP
        state.players[0].technologies.push(TechCardId(6));
        let (vp, yen) = tech_acquisition_bonuses(&state, player, &tech_registry);
        assert_eq!(vp, 2);
        assert_eq!(yen, 0);

        // With both - +2 VP and +1 Yen
        state.players[0].technologies.push(TechCardId(7));
        let (vp, yen) = tech_acquisition_bonuses(&state, player, &tech_registry);
        assert_eq!(vp, 2);
        assert_eq!(yen, 1);
    }

    #[test]
    fn test_placement_limit() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        let mut tech_registry = TechRegistry::new();
        let newspaper = TechCard {
            id: TechCardId(8),
            name: "Newspaper".to_string(),
            country: Country::French,
            industry_cost: 2,
            effect: TechEffect::Newspaper,
        };
        tech_registry.insert(newspaper, "tech_newspaper");

        // Without tech - 3 areas
        assert_eq!(placement_limit(&state, player, &tech_registry), 3);

        // With Newspaper - 4 areas
        state.players[0].technologies.push(TechCardId(8));
        assert_eq!(placement_limit(&state, player, &tech_registry), 4);
    }

    #[test]
    fn test_movement_tech_effects() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        let mut tech_registry = TechRegistry::new();
        let postal = TechCard {
            id: TechCardId(9),
            name: "Postal System".to_string(),
            country: Country::German,
            industry_cost: 3,
            effect: TechEffect::PostalSystem,
        };
        let stagecoach = TechCard {
            id: TechCardId(10),
            name: "Stagecoach".to_string(),
            country: Country::Dutch,
            industry_cost: 2,
            effect: TechEffect::Stagecoach,
        };
        tech_registry.insert(postal, "tech_postal");
        tech_registry.insert(stagecoach, "tech_stagecoach");

        // Without tech
        assert!(!can_stop_with_opponent(&state, player, &tech_registry));
        assert!(!free_passage(&state, player, &tech_registry));

        // With Postal System
        state.players[0].technologies.push(TechCardId(9));
        assert!(can_stop_with_opponent(&state, player, &tech_registry));
        assert!(!free_passage(&state, player, &tech_registry));

        // With both
        state.players[0].technologies.push(TechCardId(10));
        assert!(can_stop_with_opponent(&state, player, &tech_registry));
        assert!(free_passage(&state, player, &tech_registry));
    }

    #[test]
    fn test_telephone_effect() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        let mut tech_registry = TechRegistry::new();
        let telephone = TechCard {
            id: TechCardId(11),
            name: "Telephone".to_string(),
            country: Country::English,
            industry_cost: 4,
            effect: TechEffect::Telephone,
        };
        tech_registry.insert(telephone, "tech_telephone");

        // Without tech
        assert!(!can_leave_assistant(&state, player, &tech_registry));

        // With Telephone
        state.players[0].technologies.push(TechCardId(11));
        assert!(can_leave_assistant(&state, player, &tech_registry));
    }

    #[test]
    fn test_system_of_stock_turn_start() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        let mut tech_registry = TechRegistry::new();
        let stock = TechCard {
            id: TechCardId(12),
            name: "System of Stock".to_string(),
            country: Country::American,
            industry_cost: 3,
            effect: TechEffect::SystemOfStock,
        };
        tech_registry.insert(stock, "tech_stock");

        // Player with 0 yen
        state.players[0].money = 0;

        // Without tech - no change
        apply_turn_start_effects(&mut state, player, &tech_registry);
        assert_eq!(state.players[0].money, 0);

        // Give player the tech
        state.players[0].technologies.push(TechCardId(12));

        // With 0 yen - should become 2
        apply_turn_start_effects(&mut state, player, &tech_registry);
        assert_eq!(state.players[0].money, 2);

        // With 1 yen - should become 2
        state.players[0].money = 1;
        apply_turn_start_effects(&mut state, player, &tech_registry);
        assert_eq!(state.players[0].money, 2);

        // With 2 yen - stays at 2
        state.players[0].money = 2;
        apply_turn_start_effects(&mut state, player, &tech_registry);
        assert_eq!(state.players[0].money, 2);

        // With 5 yen - stays at 5
        state.players[0].money = 5;
        apply_turn_start_effects(&mut state, player, &tech_registry);
        assert_eq!(state.players[0].money, 5);
    }

    #[test]
    fn test_spinning_mill_effect() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        let mut tech_registry = TechRegistry::new();
        let spinning_mill = TechCard {
            id: TechCardId(13),
            name: "Spinning Mill".to_string(),
            country: Country::Dutch,
            industry_cost: 3,
            effect: TechEffect::SpinningMill,
        };
        tech_registry.insert(spinning_mill, "tech_spinning_mill");

        // Setup: player has 2 silk and 0 money
        state.players[0].silk = 2;
        state.players[0].money = 0;

        // Without tech - cannot use
        assert!(!can_use_spinning_mill(&state, player, &tech_registry));

        // Give player the tech
        state.players[0].technologies.push(TechCardId(13));

        // With tech - can use
        assert!(can_use_spinning_mill(&state, player, &tech_registry));

        // Apply effect: discard 1 silk, gain 2 Yen
        let result = apply_spinning_mill(&mut state, player);
        assert!(result.is_ok());
        assert_eq!(state.players[0].silk, 1);
        assert_eq!(state.players[0].money, 2);

        // Cannot use again this turn (once per turn)
        assert!(!can_use_spinning_mill(&state, player, &tech_registry));

        // Reset for next turn
        state.additional_action_state.spinning_mill_used = false;

        // Can use again
        assert!(can_use_spinning_mill(&state, player, &tech_registry));
    }

    #[test]
    fn test_spinning_mill_requires_silk() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        let mut tech_registry = TechRegistry::new();
        let spinning_mill = TechCard {
            id: TechCardId(13),
            name: "Spinning Mill".to_string(),
            country: Country::Dutch,
            industry_cost: 3,
            effect: TechEffect::SpinningMill,
        };
        tech_registry.insert(spinning_mill, "tech_spinning_mill");

        // Give tech but no silk
        state.players[0].technologies.push(TechCardId(13));
        state.players[0].silk = 0;

        // Cannot use without silk
        assert!(!can_use_spinning_mill(&state, player, &tech_registry));

        // Apply should fail
        let result = apply_spinning_mill(&mut state, player);
        assert!(result.is_err());
    }

    #[test]
    fn test_university_effect() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        let mut tech_registry = TechRegistry::new();
        let university = TechCard {
            id: TechCardId(14),
            name: "University".to_string(),
            country: Country::French,
            industry_cost: 4,
            effect: TechEffect::University,
        };
        tech_registry.insert(university, "tech_university");

        // Setup: player has 3 Yen and 2 assistants in warehouse
        state.players[0].money = 3;
        state.players[0].assistants.in_warehouse = 2;
        state.players[0].assistants.in_hand = 5;

        // Without tech - cannot use
        assert!(!can_use_university(&state, player, &tech_registry));

        // Give player the tech
        state.players[0].technologies.push(TechCardId(14));

        // With tech - can use
        assert!(can_use_university(&state, player, &tech_registry));

        // Apply effect: pay 1 Yen, take assistant from warehouse
        let result = apply_university(&mut state, player);
        assert!(result.is_ok());
        assert_eq!(state.players[0].money, 2);
        assert_eq!(state.players[0].assistants.in_warehouse, 1);
        assert_eq!(state.players[0].assistants.in_hand, 6);

        // Cannot use again this turn (once per turn)
        assert!(!can_use_university(&state, player, &tech_registry));

        // Reset for next turn
        state.additional_action_state.university_used = false;

        // Can use again
        assert!(can_use_university(&state, player, &tech_registry));
    }

    #[test]
    fn test_university_requires_money_and_warehouse() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);

        let mut tech_registry = TechRegistry::new();
        let university = TechCard {
            id: TechCardId(14),
            name: "University".to_string(),
            country: Country::French,
            industry_cost: 4,
            effect: TechEffect::University,
        };
        tech_registry.insert(university, "tech_university");

        // Give tech
        state.players[0].technologies.push(TechCardId(14));

        // No money
        state.players[0].money = 0;
        state.players[0].assistants.in_warehouse = 2;
        assert!(!can_use_university(&state, player, &tech_registry));

        // Has money but no warehouse assistants
        state.players[0].money = 5;
        state.players[0].assistants.in_warehouse = 0;
        assert!(!can_use_university(&state, player, &tech_registry));

        // Both requirements met
        state.players[0].assistants.in_warehouse = 1;
        assert!(can_use_university(&state, player, &tech_registry));
    }
}
