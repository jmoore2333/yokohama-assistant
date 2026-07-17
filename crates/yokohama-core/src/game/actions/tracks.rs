//! Church and Customs track actions.

use crate::types::{AreaType, ChurchSlot, ChurchTrack, GameState, PlayerId, PresidentLocation};
use crate::game::rules::Rules;
use crate::game::tech_effects::church_vp_bonus;

/// Calculate Faith value from power. Faith scales LINEARLY with power:
/// 1,2,3,4,5 (capped at 5 for power above 5). Jeremy-adjudicated
/// 2026-07-17: the rulebook's "2 Power: 1 Faith" text is a typo.
pub fn church_faith_from_power(power: u8) -> u8 {
    power.min(5)
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

/// Highest empty Church space with value <= `total_faith` (v1 auto-place:
/// choosing a lower space is weakly dominated and unmodeled; any faith
/// beyond the chosen space's value is lost). In 2-player games the
/// faith-7 space is pre-blocked by a printed dummy pawn — an eligibility
/// rule, not a `slots` entry, so arrival-order and end-trigger pawn counts
/// (which count real players only) stay untouched. Faith below 4 has no
/// eligible space; faith above 11 is capped at the top of the track.
fn highest_eligible_church_space(track: &ChurchTrack, total_faith: u8, player_count: u8) -> Option<u8> {
    let ceiling = total_faith.min(11);
    (4..=ceiling).rev().find(|&faith| {
        if player_count == 2 && faith == 7 {
            return false;
        }
        let index = ChurchTrack::faith_to_index(faith).expect("4..=11 always maps to a valid index");
        track.slots[index].is_none()
    })
}

/// Execute Church action: resolve the acting area (mirrors
/// `execute_port_action`/`execute_lab_action` — a mercenary sub-turn acts
/// at the mercenary target area, not the president's; Church has only one
/// area, so this resolution exists solely to confirm the actor is
/// actually there), validate contributions/assistant, compute total
/// Faith, auto-place on the highest empty eligible space, pay contributed
/// goods, lock the assistant, and award VP (plus Winery bonus per owned
/// copy).
///
/// If total Faith < 4, no eligible empty space exists, or the player has
/// no assistant in hand, the action is ILLEGAL (v1 semantic: goods are
/// never paid for a placement that doesn't happen, keeping the legal-move
/// projection conservative).
///
/// Per-space movement riders (faith 4-11: 1,1,2,2,2,2,3,3) are v1 no-ops,
/// consistent with the standing movement-icon policy — unmodeled here.
#[allow(clippy::too_many_arguments)]
pub fn execute_church_action(
    state: &mut GameState,
    player: PlayerId,
    power: u8,
    copper: bool,
    silk: bool,
    tea: bool,
    seafood: bool,
    yen: bool,
    imported_good: bool,
    rules: &Rules,
) -> Result<(), &'static str> {
    let area = if state.is_mercenary_action {
        state.mercenary_target_area.ok_or("No mercenary target area")?
    } else {
        match state.player(player).president_location {
            PresidentLocation::OnBoard(a) => a,
            _ => return Err("President not on board"),
        }
    };
    match state.board_layout.areas.get(&area).map(|c| c.area_type) {
        Some(AreaType::Church) => {}
        _ => return Err("Church action requires the Church area"),
    }

    validate_church_action(state, player, copper, silk, tea, seafood, yen, imported_good)?;

    let total_faith = calculate_church_faith(power, copper, silk, tea, seafood, yen, imported_good);
    let target_faith =
        highest_eligible_church_space(&state.church_track, total_faith, state.config.player_count)
            .ok_or("No eligible Church space for this Faith total")?;

    // Computed via immutable borrows before any mutation below.
    let winery_bonus = church_vp_bonus(state, player, &rules.techs);
    let base_vp = ChurchTrack::vp_reward(target_faith) as u32;
    let index = ChurchTrack::faith_to_index(target_faith).expect("target_faith always in 4..=11");
    let arrival_order = state.church_track.next_arrival_order;

    state.church_track.slots[index] = Some(ChurchSlot { player, arrival_order });
    state.church_track.next_arrival_order += 1;

    let p = state.player_mut(player);
    if copper { p.copper -= 1; }
    if silk { p.silk -= 1; }
    if tea { p.tea -= 1; }
    if seafood { p.seafood -= 1; }
    if yen { p.money -= 1; }
    if imported_good { p.imported_goods -= 1; }
    p.assistants.in_hand -= 1;
    p.assistants.on_church += 1;
    p.church_positions.push(target_faith);
    p.victory_points += base_vp + winery_bonus;

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
    use crate::types::*;
    use crate::game::cards::TechRegistry;
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

    /// Places a Church area on the board and puts the president there.
    fn place_president_at_church(state: &mut GameState) -> AreaId {
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Church,
            grid_position: (0, 0),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        area
    }

    fn make_test_tech_card(id: u8, effect: TechEffect) -> TechCard {
        TechCard {
            id: TechCardId(id),
            name: "Test Tech".to_string(),
            country: Country::French,
            industry_cost: 2,
            effect,
        }
    }

    #[test]
    fn test_highest_eligible_church_space_basic() {
        let track = ChurchTrack::new();
        assert_eq!(highest_eligible_church_space(&track, 5, 3), Some(5));
    }

    #[test]
    fn test_highest_eligible_church_space_falls_back_when_occupied() {
        let mut track = ChurchTrack::new();
        let idx5 = ChurchTrack::faith_to_index(5).unwrap();
        track.slots[idx5] = Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 0 });
        assert_eq!(highest_eligible_church_space(&track, 5, 3), Some(4));
    }

    #[test]
    fn test_highest_eligible_church_space_below_4_is_none() {
        let track = ChurchTrack::new();
        assert_eq!(highest_eligible_church_space(&track, 3, 3), None);
    }

    #[test]
    fn test_highest_eligible_church_space_none_when_all_occupied() {
        let mut track = ChurchTrack::new();
        for faith in 4..=5 {
            let idx = ChurchTrack::faith_to_index(faith).unwrap();
            track.slots[idx] = Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 0 });
        }
        assert_eq!(highest_eligible_church_space(&track, 5, 3), None);
    }

    #[test]
    fn test_highest_eligible_church_space_2p_blocks_faith_7() {
        let track = ChurchTrack::new();
        assert_eq!(highest_eligible_church_space(&track, 7, 2), Some(6));
        // 3P (and 4P): no dummy pawn, faith-7 is reachable.
        assert_eq!(highest_eligible_church_space(&track, 7, 3), Some(7));
    }

    #[test]
    fn test_execute_church_action_places_assistant_and_awards_vp() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_church(&mut state);
        state.players[0].copper = 1;
        state.players[0].silk = 1;

        // Power 3 + copper + silk = faith 5.
        let result = execute_church_action(
            &mut state, PlayerId::new(0), 3, true, true, false, false, false, false, &rules,
        );
        assert!(result.is_ok());

        let p = &state.players[0];
        assert_eq!(p.assistants.in_hand, 7); // started at 8
        assert_eq!(p.assistants.on_church, 1);
        assert_eq!(p.copper, 0);
        assert_eq!(p.silk, 0);
        assert_eq!(p.church_positions, vec![5]);
        assert_eq!(p.victory_points, ChurchTrack::vp_reward(5) as u32);
        assert_eq!(
            state.church_track.slots[ChurchTrack::faith_to_index(5).unwrap()].unwrap().player,
            PlayerId::new(0)
        );
    }

    #[test]
    fn test_execute_church_action_falls_back_to_lower_occupied_space() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_church(&mut state);
        // Another player already occupies faith-5.
        let idx5 = ChurchTrack::faith_to_index(5).unwrap();
        state.church_track.slots[idx5] = Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 0 });
        state.church_track.next_arrival_order = 1;

        // Power 5, no goods = faith 5, but faith-5 is taken -> falls to 4.
        let result = execute_church_action(
            &mut state, PlayerId::new(0), 5, false, false, false, false, false, false, &rules,
        );
        assert!(result.is_ok());

        let p = &state.players[0];
        assert_eq!(p.church_positions, vec![4]);
        assert_eq!(p.victory_points, ChurchTrack::vp_reward(4) as u32);
    }

    #[test]
    fn test_execute_church_action_rejects_faith_below_4() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_church(&mut state);

        // Power 3, no goods = faith 3 -> below the floor of 4.
        let result = execute_church_action(
            &mut state, PlayerId::new(0), 3, false, false, false, false, false, false, &rules,
        );
        assert!(result.is_err());
        // Nothing paid or placed on rejection.
        assert_eq!(state.players[0].assistants.in_hand, 8);
        assert_eq!(state.church_track.total_assistants(), 0);
    }

    #[test]
    fn test_execute_church_action_rejects_no_eligible_empty_space() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_church(&mut state);
        // Occupy both spaces the faith-5 action below could reach (4 and 5).
        for faith in 4..=5 {
            let idx = ChurchTrack::faith_to_index(faith).unwrap();
            state.church_track.slots[idx] = Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 0 });
        }

        let result = execute_church_action(
            &mut state, PlayerId::new(0), 5, false, false, false, false, false, false, &rules,
        );
        assert!(result.is_err());
        assert_eq!(state.players[0].assistants.in_hand, 8);
    }

    #[test]
    fn test_execute_church_action_rejects_no_assistant_in_hand() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_church(&mut state);
        state.players[0].assistants.in_hand = 0;

        let result = execute_church_action(
            &mut state, PlayerId::new(0), 5, false, false, false, false, false, false, &rules,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_church_action_2p_blocks_faith_7_places_at_6() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state(); // 2-player fixture
        place_president_at_church(&mut state);
        state.players[0].copper = 1;
        state.players[0].silk = 1;

        // Power 5 + copper + silk = faith 7, but 2P blocks the faith-7 space.
        let result = execute_church_action(
            &mut state, PlayerId::new(0), 5, true, true, false, false, false, false, &rules,
        );
        assert!(result.is_ok());
        assert_eq!(state.players[0].church_positions, vec![6]);
    }

    #[test]
    fn test_execute_church_action_during_mercenary_sub_turn_resolves_target_area() {
        // President stays in hand (default) and is never placed; a Church
        // action must still work by resolving the acting area from
        // mercenary_target_area, not president_location.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let church_area = AreaId::new(9);
        state.board_layout.areas.insert(church_area, AreaConfig {
            area_type: AreaType::Church,
            grid_position: (0, 9),
        });
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(church_area);

        let result = execute_church_action(
            &mut state, PlayerId::new(0), 4, false, false, false, false, false, false, &rules,
        );
        assert!(result.is_ok());
        assert_eq!(state.players[0].church_positions, vec![4]);
    }

    #[test]
    fn test_execute_church_action_rejects_when_not_on_church_area() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let bank_area = AreaId::new(0);
        state.board_layout.areas.insert(bank_area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(bank_area);

        let result = execute_church_action(
            &mut state, PlayerId::new(0), 5, false, false, false, false, false, false, &rules,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_church_action_winery_bonus_stacks_per_owned_copy() {
        // Standard data has only 1 physical Winery, so stacking is proven
        // with a hand-rolled two-copy registry fixture (mirrors Task 12's
        // Patent System stacking precedent).
        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(make_test_tech_card(1, TechEffect::Winery), "winery_1");
        tech_registry.insert(make_test_tech_card(2, TechEffect::Winery), "winery_2");
        let mut rules = Rules::empty_for_tests();
        rules.techs = tech_registry;

        let mut state = create_test_state();
        place_president_at_church(&mut state);
        state.players[0].technologies = vec![TechCardId(1), TechCardId(2)]; // owns BOTH copies

        // Power 4, no goods = faith 4.
        let result = execute_church_action(
            &mut state, PlayerId::new(0), 4, false, false, false, false, false, false, &rules,
        );
        assert!(result.is_ok());

        let base_vp = ChurchTrack::vp_reward(4) as u32;
        assert_eq!(state.players[0].victory_points, base_vp + 6); // 2 copies x 3 VP
    }

    #[test]
    fn test_church_faith_from_power() {
        assert_eq!(church_faith_from_power(1), 1);
        // Jeremy-adjudicated 2026-07-17: faith scales LINEARLY with power;
        // the rulebook's "2 Power: 1 Faith" text is a typo.
        assert_eq!(church_faith_from_power(2), 2);
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
    fn test_calculate_church_faith_power_2_is_linear_not_1_rulebook_typo() {
        // Dedicated pinning test per the ruling: power 2 with no goods
        // contributes faith 2 (not 1 as the rulebook text's typo implied).
        assert_eq!(calculate_church_faith(2, false, false, false, false, false, false), 2);
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
