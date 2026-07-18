//! Church and Customs track actions.

use crate::types::{
    AreaId, AreaType, ChurchSlot, ChurchTrack, CustomsSlot, CustomsTrack, GameState, PlayerId,
    PlayerState, PresidentLocation,
};
use crate::game::apply_move::apply_warehouse_takes;
use crate::game::rules::Rules;
use crate::game::tech_effects::{church_vp_bonus, customs_vp_bonus};

/// Take 1 assistant from hand, or (if hand is empty) from the acting area
/// itself - rulebook p.13/p.14: "place 1 Assistant pawn from the
/// [Church/Customs] Area or your hand" (I3 final-review fix). Caller's
/// validator must already have confirmed at least one source has >= 1.
/// Preferring hand keeps existing hand >= 1 play unchanged (the two sources
/// are net-identical in v1's same-turn-recovery flow); the acting-area
/// fallback only engages at hand == 0. Decrementing `on_board[area]` here
/// (rather than leaving a stale entry) is what keeps the automatic Recovery
/// step from later double-recovering a pawn that already moved onto the
/// track.
fn take_church_or_customs_assistant(p: &mut PlayerState, area: AreaId) {
    if p.assistants.in_hand >= 1 {
        p.assistants.in_hand -= 1;
        return;
    }
    if let Some(&count) = p.assistants.on_board.get(&area) {
        if count > 1 {
            p.assistants.on_board.insert(area, count - 1);
        } else {
            p.assistants.on_board.remove(&area);
        }
    }
}

/// True if the player has an assistant available for a Church/Customs
/// placement: in hand, or already on the acting area (I3 final-review fix).
fn has_church_or_customs_assistant(p: &PlayerState, area: AreaId) -> bool {
    p.assistants.in_hand >= 1 || p.assistants.in_area(area) >= 1
}

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

/// Validate Church action. `area` is the acting Church area (I3
/// final-review fix): the assistant may come from hand OR from the acting
/// area itself, per rulebook p.13 ("place 1 Assistant pawn from the Church
/// Area or your hand").
#[allow(clippy::too_many_arguments)]
pub fn validate_church_action(
    state: &GameState,
    player: PlayerId,
    area: AreaId,
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

    // Check assistant available: hand, or the acting area itself.
    if !has_church_or_customs_assistant(p, area) {
        return Err("No assistant available for Church (hand or acting area)");
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
///
/// `pub` (Task 17): reused directly by the Church legal-move generator so
/// enumeration can never drift from `execute_church_action`'s own
/// acceptance criteria.
pub fn highest_eligible_church_space(track: &ChurchTrack, total_faith: u8, player_count: u8) -> Option<u8> {
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

    validate_church_action(state, player, area, copper, silk, tea, seafood, yen, imported_good)?;

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
    take_church_or_customs_assistant(p, area);
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

/// Validate Customs action. `area` is the acting Customs area (I3
/// final-review fix): the assistant may come from hand OR from the acting
/// area itself, per rulebook p.14 ("place 1 Assistant pawn from the Customs
/// Area or your hand").
pub fn validate_customs_action(
    state: &GameState,
    player: PlayerId,
    area: AreaId,
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

    // Check assistant available: hand, or the acting area itself.
    if !has_church_or_customs_assistant(p, area) {
        return Err("No assistant available for Customs (hand or acting area)");
    }

    Ok(())
}

/// Empty Customs space for an exact printed cost, preferring the higher-VP
/// (leftmost) space of the matching pair and falling back to the other
/// space of the pair if it's occupied (cost 4 has no pair -- space 7
/// alone). In 2-player games space 4 (VP 9) is pre-blocked by a printed
/// dummy pawn -- an eligibility rule, not a `slots` entry, mirroring the
/// Church faith-7 2P block so arrival order and the end-trigger assistant
/// count stay untouched. Returns `None` if `cost` doesn't match a printed
/// cost, or every candidate space is occupied/blocked.
///
/// `pub` (Task 17): reused directly by the Customs legal-move generator so
/// enumeration can never drift from `execute_customs_action`'s own
/// acceptance criteria.
pub fn eligible_customs_space_for_cost(track: &CustomsTrack, cost: u8, player_count: u8) -> Option<u8> {
    let candidates = CustomsTrack::spaces_for_cost(cost)?;
    candidates.iter().copied().find(|&space| {
        if player_count == 2 && space == 4 {
            return false;
        }
        let index = CustomsTrack::space_to_index(space).expect("space always in 1..=7");
        track.slots[index].is_none()
    })
}

/// Execute Customs action: resolve the acting area (mirrors
/// `execute_church_action` -- a mercenary sub-turn acts at the mercenary
/// target area, not the president's), validate the payment and assistant,
/// find the eligible space for the exact printed cost (auto-placing on the
/// higher-VP/leftmost space of the matching pair), pay imported goods,
/// lock the assistant, award VP (plus Brickyard bonus per owned copy), and
/// apply the space's rider -- warehouse-takes and/or Yen, via the same
/// Task 7b `apply_warehouse_takes` helper Church-adjacent reward code uses
/// (no new reward machinery).
///
/// If the payment doesn't exactly match a printed cost, every space at
/// that cost is occupied or blocked, or the player has no assistant in
/// hand, the action is ILLEGAL (v1 semantic, same as Church: imported
/// goods are never paid for a placement that doesn't happen).
pub fn execute_customs_action(
    state: &mut GameState,
    player: PlayerId,
    power: u8,
    imported_goods_paid: u8,
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
        Some(AreaType::Customs) => {}
        _ => return Err("Customs action requires the Customs area"),
    }

    validate_customs_action(state, player, area, imported_goods_paid, power)?;

    let target_space = eligible_customs_space_for_cost(
        &state.customs_track,
        imported_goods_paid,
        state.config.player_count,
    )
    .ok_or("No eligible Customs space for this payment")?;

    // Computed via immutable borrows before any mutation below.
    let brickyard_bonus = customs_vp_bonus(state, player, &rules.techs);
    let base_vp = CustomsTrack::vp_reward(target_space) as u32;
    let (warehouse_takes, yen_rider) = CustomsTrack::rider(target_space);
    let index = CustomsTrack::space_to_index(target_space).expect("target_space always in 1..=7");
    let arrival_order = state.customs_track.next_arrival_order;

    state.customs_track.slots[index] = Some(CustomsSlot { player, arrival_order });
    state.customs_track.next_arrival_order += 1;

    let p = state.player_mut(player);
    p.imported_goods -= imported_goods_paid as u32;
    take_church_or_customs_assistant(p, area);
    p.assistants.on_customs += 1;
    p.customs_positions.push(target_space);
    p.victory_points += base_vp + brickyard_bonus;
    p.money += yen_rider as u32;
    apply_warehouse_takes(p, warehouse_takes);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use crate::game::cards::TechRegistry;
    use std::collections::BTreeMap;

    fn create_test_state() -> GameState {
        let config = GameConfig::new(
            2,
            vec![PlayerColor::Red, PlayerColor::Blue],
            PlayerId::new(0),
        );

        let board_layout = BoardLayout {
            areas: BTreeMap::new(),
            adjacency: BTreeMap::new(),
            canal_crossings: vec![],
            building_sites: BTreeMap::new(),
            five_power_tokens: BTreeMap::new(),
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
            pending_choices: Vec::new(),
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

    /// Places a Customs area on the board and puts player 0's president there.
    fn place_president_at_customs(state: &mut GameState) -> AreaId {
        let area = AreaId::new(0);
        state.board_layout.areas.insert(area, AreaConfig {
            area_type: AreaType::Customs,
            grid_position: (0, 0),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        area
    }

    /// Like `create_test_state`, but with 3 players -- needed to prove
    /// non-2P fallback behaves normally (2P blocks Customs space 4).
    fn create_test_state_3p() -> GameState {
        let mut state = create_test_state();
        state.config = GameConfig::new(
            3,
            vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow],
            PlayerId::new(0),
        );
        state.players.push(PlayerState::new(PlayerId::new(2), PlayerColor::Yellow, false));
        state
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

        // No assistant on the acting area either, so both sources are empty.
        let result = execute_church_action(
            &mut state, PlayerId::new(0), 5, false, false, false, false, false, false, &rules,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_church_action_allows_assistant_from_acting_area_when_hand_empty() {
        // I3 final-review fix: rulebook p.13 - "place 1 Assistant pawn from
        // the Church Area OR your hand." With hand empty but an assistant
        // already on the acting area, the action must still be legal - it
        // takes that pawn instead of rejecting.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let area = place_president_at_church(&mut state);
        state.players[0].assistants.in_hand = 0;
        state.players[0].assistants.on_board.insert(area, 2);

        let result = execute_church_action(
            &mut state, PlayerId::new(0), 5, false, false, false, false, false, false, &rules,
        );
        assert!(result.is_ok());

        let p = &state.players[0];
        assert_eq!(p.assistants.in_hand, 0, "hand was already empty and stays empty");
        assert_eq!(p.assistants.on_board.get(&area), Some(&1), "1 pawn taken from the acting area");
        assert_eq!(p.assistants.on_church, 1);
        assert_eq!(p.church_positions, vec![5]);
    }

    #[test]
    fn test_execute_church_action_from_area_assistant_removes_map_entry_at_zero() {
        // Same as above but with exactly 1 assistant on the acting area:
        // the on_board entry must be removed (not left at 0), matching the
        // convention used elsewhere (e.g. apply_return_to_hand).
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let area = place_president_at_church(&mut state);
        state.players[0].assistants.in_hand = 0;
        state.players[0].assistants.on_board.insert(area, 1);

        let result = execute_church_action(
            &mut state, PlayerId::new(0), 5, false, false, false, false, false, false, &rules,
        );
        assert!(result.is_ok());
        assert_eq!(state.players[0].assistants.on_board.get(&area), None);
        assert_eq!(state.players[0].assistants.on_church, 1);
    }

    #[test]
    fn test_execute_church_action_prefers_hand_assistant_when_both_sources_available() {
        // With hand >= 1, the acting-area pawn must be left untouched (the
        // two sources are net-identical in v1's same-turn-recovery flow, but
        // this pins WHICH pawn moves so on_board bookkeeping stays exact).
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let area = place_president_at_church(&mut state);
        state.players[0].assistants.in_hand = 3;
        state.players[0].assistants.on_board.insert(area, 2);

        let result = execute_church_action(
            &mut state, PlayerId::new(0), 5, false, false, false, false, false, false, &rules,
        );
        assert!(result.is_ok());

        let p = &state.players[0];
        assert_eq!(p.assistants.in_hand, 2, "hand pawn spent, not the acting-area one");
        assert_eq!(p.assistants.on_board.get(&area), Some(&2), "acting-area pawns untouched");
    }

    #[test]
    fn test_execute_church_action_during_mercenary_sub_turn_allows_assistant_from_target_area_when_hand_empty() {
        // The report's headline case: EVERY mercenary sub-turn's target area
        // has the player's own assistant by definition (apply_mercenary
        // validates this before starting the sub-turn), so hand-only
        // validation would wrongly reject a Church action there whenever
        // hand happens to be empty.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let church_area = AreaId::new(9);
        state.board_layout.areas.insert(church_area, AreaConfig {
            area_type: AreaType::Church,
            grid_position: (0, 9),
        });
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(church_area);
        state.players[0].assistants.in_hand = 0;
        state.players[0].assistants.on_board.insert(church_area, 1);

        let result = execute_church_action(
            &mut state, PlayerId::new(0), 4, false, false, false, false, false, false, &rules,
        );
        assert!(result.is_ok());
        assert_eq!(state.players[0].church_positions, vec![4]);
        assert_eq!(state.players[0].assistants.on_board.get(&church_area), None);
    }

    #[test]
    fn test_execute_church_action_from_area_assistant_then_recovery_does_not_double_recover() {
        // I3's "recovery must not double-recover" requirement: taking the
        // assistant from the acting area must decrement on_board there SO
        // THAT the automatic Recovery step (which recovers whatever remains
        // in on_board at the acting area) doesn't also return a pawn that
        // already moved onto the Church track.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let area = place_president_at_church(&mut state);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].assistants.in_hand = 0;
        state.players[0].assistants.on_board.insert(area, 1);

        execute_church_action(
            &mut state, PlayerId::new(0), 5, false, false, false, false, false, false, &rules,
        ).unwrap();
        assert_eq!(state.players[0].assistants.on_church, 1);
        assert_eq!(state.players[0].assistants.on_board.get(&area), None);

        crate::game::apply_move::apply_recovery_with_leave(&mut state, false);

        // Nothing was left on the board to recover; the Church-locked pawn
        // must not be pulled back to hand.
        assert_eq!(state.players[0].assistants.in_hand, 0);
        assert_eq!(state.players[0].assistants.on_church, 1);
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
    fn test_eligible_customs_space_for_cost_basic() {
        let track = CustomsTrack::new();
        assert_eq!(eligible_customs_space_for_cost(&track, 1, 3), Some(1));
        assert_eq!(eligible_customs_space_for_cost(&track, 2, 3), Some(3));
        assert_eq!(eligible_customs_space_for_cost(&track, 3, 3), Some(5));
        assert_eq!(eligible_customs_space_for_cost(&track, 4, 3), Some(7));
    }

    #[test]
    fn test_eligible_customs_space_for_cost_falls_back_when_occupied() {
        let mut track = CustomsTrack::new();
        let idx1 = CustomsTrack::space_to_index(1).unwrap();
        track.slots[idx1] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 });
        assert_eq!(eligible_customs_space_for_cost(&track, 1, 3), Some(2));
    }

    #[test]
    fn test_eligible_customs_space_for_cost_both_occupied_is_none() {
        let mut track = CustomsTrack::new();
        for space in [1, 2] {
            let idx = CustomsTrack::space_to_index(space).unwrap();
            track.slots[idx] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 });
        }
        assert_eq!(eligible_customs_space_for_cost(&track, 1, 3), None);
    }

    #[test]
    fn test_eligible_customs_space_for_cost_invalid_cost_is_none() {
        let track = CustomsTrack::new();
        assert_eq!(eligible_customs_space_for_cost(&track, 0, 3), None);
        assert_eq!(eligible_customs_space_for_cost(&track, 5, 3), None);
    }

    #[test]
    fn test_eligible_customs_space_for_cost_2p_blocks_space4() {
        let mut track = CustomsTrack::new();
        let idx3 = CustomsTrack::space_to_index(3).unwrap();
        track.slots[idx3] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 });
        assert_eq!(eligible_customs_space_for_cost(&track, 2, 2), None);
        // 3P (and 4P): no dummy pawn, space 4 is reachable.
        assert_eq!(eligible_customs_space_for_cost(&track, 2, 3), Some(4));
    }

    #[test]
    fn test_execute_customs_action_places_assistant_and_awards_vp() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_customs(&mut state);
        state.players[0].imported_goods = 1;
        state.players[0].assistants.in_warehouse = 3;

        // Power 3 (max 2), pay 1 -> cost pair {1,2}; space 1 is leftmost/default.
        let result = execute_customs_action(&mut state, PlayerId::new(0), 3, 1, &rules);
        assert!(result.is_ok());

        let p = &state.players[0];
        assert_eq!(p.assistants.in_hand, 9); // 8 - 1 (placement) + 2 (warehouse-take rider)
        assert_eq!(p.assistants.on_customs, 1);
        assert_eq!(p.assistants.in_warehouse, 1); // 3 - 2
        assert_eq!(p.imported_goods, 0);
        assert_eq!(p.customs_positions, vec![1]);
        assert_eq!(p.victory_points, CustomsTrack::vp_reward(1) as u32);
        assert_eq!(
            state.customs_track.slots[CustomsTrack::space_to_index(1).unwrap()].unwrap().player,
            PlayerId::new(0)
        );
    }

    #[test]
    fn test_execute_customs_action_falls_back_to_other_space_and_applies_yen_rider() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state_3p();
        place_president_at_customs(&mut state);
        // Another player already occupies space 3.
        let idx3 = CustomsTrack::space_to_index(3).unwrap();
        state.customs_track.slots[idx3] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 });
        state.customs_track.next_arrival_order = 1;
        state.players[0].imported_goods = 2;

        // Power 4 (max 3), pay 2 -> cost pair {3,4}; space 3 taken -> falls to space 4.
        let result = execute_customs_action(&mut state, PlayerId::new(0), 4, 2, &rules);
        assert!(result.is_ok());

        let p = &state.players[0];
        assert_eq!(p.customs_positions, vec![4]);
        assert_eq!(p.victory_points, CustomsTrack::vp_reward(4) as u32);
        assert_eq!(p.money, 4); // starting 3 (first player) + space 4 rider: +1 Yen
        assert_eq!(p.imported_goods, 0);
    }

    #[test]
    fn test_execute_customs_action_rejects_both_spaces_of_pair_occupied() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_customs(&mut state);
        state.players[0].imported_goods = 1;
        for space in [1, 2] {
            let idx = CustomsTrack::space_to_index(space).unwrap();
            state.customs_track.slots[idx] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 });
        }
        state.customs_track.next_arrival_order = 2;

        let result = execute_customs_action(&mut state, PlayerId::new(0), 3, 1, &rules);
        assert!(result.is_err());
        // Nothing paid or placed on rejection.
        assert_eq!(state.players[0].imported_goods, 1);
        assert_eq!(state.players[0].assistants.in_hand, 8);
    }

    #[test]
    fn test_execute_customs_action_2p_blocks_space4_when_space3_occupied() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state(); // 2-player fixture
        place_president_at_customs(&mut state);
        state.players[0].imported_goods = 2;
        let idx3 = CustomsTrack::space_to_index(3).unwrap();
        state.customs_track.slots[idx3] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 });
        state.customs_track.next_arrival_order = 1;

        // Space 3 taken; space 4 would normally be the fallback but is
        // pre-blocked by the printed dummy pawn in 2P games.
        let result = execute_customs_action(&mut state, PlayerId::new(0), 4, 2, &rules);
        assert!(result.is_err());
        assert_eq!(state.players[0].imported_goods, 2);
        assert_eq!(state.players[0].assistants.in_hand, 8);
    }

    #[test]
    fn test_execute_customs_action_rejects_power_cap_exceeded() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_customs(&mut state);
        state.players[0].imported_goods = 5;

        // Power 2 caps at 1 imported good; paying 2 exceeds it.
        let result = execute_customs_action(&mut state, PlayerId::new(0), 2, 2, &rules);
        assert!(result.is_err());
        assert_eq!(state.players[0].imported_goods, 5);
        assert_eq!(state.players[0].assistants.in_hand, 8);
        assert_eq!(state.customs_track.total_assistants(), 0);
    }

    #[test]
    fn test_execute_customs_action_rejects_insufficient_stock() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_customs(&mut state);
        state.players[0].imported_goods = 0;

        let result = execute_customs_action(&mut state, PlayerId::new(0), 5, 1, &rules);
        assert!(result.is_err());
        assert_eq!(state.players[0].assistants.in_hand, 8);
    }

    #[test]
    fn test_execute_customs_action_rejects_zero_payment() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_customs(&mut state);
        state.players[0].imported_goods = 3;

        let result = execute_customs_action(&mut state, PlayerId::new(0), 5, 0, &rules);
        assert!(result.is_err());
        assert_eq!(state.players[0].imported_goods, 3);
    }

    #[test]
    fn test_execute_customs_action_rejects_no_assistant_in_hand() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_customs(&mut state);
        state.players[0].imported_goods = 1;
        state.players[0].assistants.in_hand = 0;

        // No assistant on the acting area either, so both sources are empty.
        let result = execute_customs_action(&mut state, PlayerId::new(0), 3, 1, &rules);
        assert!(result.is_err());
        assert_eq!(state.players[0].imported_goods, 1);
    }

    #[test]
    fn test_execute_customs_action_allows_assistant_from_acting_area_when_hand_empty() {
        // I3 final-review fix: rulebook p.14 - "place 1 Assistant pawn from
        // the Customs Area OR your hand." Uses power 5 / pay 3 -> space 5
        // (no rider), so the assertions below aren't muddied by space 1's
        // +2-warehouse-takes rider (apply_warehouse_takes moves warehouse
        // stock to HAND, which would otherwise mask the pawn-source bug
        // this test targets).
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let area = place_president_at_customs(&mut state);
        state.players[0].imported_goods = 3;
        state.players[0].assistants.in_hand = 0;
        state.players[0].assistants.on_board.insert(area, 1);

        let result = execute_customs_action(&mut state, PlayerId::new(0), 5, 3, &rules);
        assert!(result.is_ok());

        let p = &state.players[0];
        assert_eq!(p.assistants.in_hand, 0);
        assert_eq!(p.assistants.on_board.get(&area), None, "the sole acting-area pawn is removed, not left at 0");
        assert_eq!(p.assistants.on_customs, 1);
        assert_eq!(p.customs_positions, vec![5]);
    }

    #[test]
    fn test_execute_customs_action_prefers_hand_assistant_when_both_sources_available() {
        // Power 5 / pay 3 -> space 5 (no rider); see the acting-area test's
        // comment for why a rider-free space matters here.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let area = place_president_at_customs(&mut state);
        state.players[0].imported_goods = 3;
        state.players[0].assistants.in_hand = 3;
        state.players[0].assistants.on_board.insert(area, 2);

        let result = execute_customs_action(&mut state, PlayerId::new(0), 5, 3, &rules);
        assert!(result.is_ok());

        let p = &state.players[0];
        assert_eq!(p.assistants.in_hand, 2, "hand pawn spent, not the acting-area one");
        assert_eq!(p.assistants.on_board.get(&area), Some(&2), "acting-area pawns untouched");
    }

    #[test]
    fn test_execute_customs_action_during_mercenary_sub_turn_allows_assistant_from_target_area_when_hand_empty() {
        // Same headline case as Church: every mercenary sub-turn's target
        // area has the player's own assistant by definition. Power 5 / pay
        // 3 -> space 5 (no rider).
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let customs_area = AreaId::new(9);
        state.board_layout.areas.insert(customs_area, AreaConfig {
            area_type: AreaType::Customs,
            grid_position: (0, 9),
        });
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(customs_area);
        state.players[0].imported_goods = 3;
        state.players[0].assistants.in_hand = 0;
        state.players[0].assistants.on_board.insert(customs_area, 1);

        let result = execute_customs_action(&mut state, PlayerId::new(0), 5, 3, &rules);
        assert!(result.is_ok());
        assert_eq!(state.players[0].customs_positions, vec![5]);
        assert_eq!(state.players[0].assistants.on_board.get(&customs_area), None);
    }

    #[test]
    fn test_execute_customs_action_from_area_assistant_then_recovery_does_not_double_recover() {
        // Power 5 / pay 3 -> space 5 (no rider).
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let area = place_president_at_customs(&mut state);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].imported_goods = 3;
        state.players[0].assistants.in_hand = 0;
        state.players[0].assistants.on_board.insert(area, 1);

        execute_customs_action(&mut state, PlayerId::new(0), 5, 3, &rules).unwrap();
        assert_eq!(state.players[0].assistants.on_customs, 1);
        assert_eq!(state.players[0].assistants.on_board.get(&area), None);

        crate::game::apply_move::apply_recovery_with_leave(&mut state, false);

        assert_eq!(state.players[0].assistants.in_hand, 0);
        assert_eq!(state.players[0].assistants.on_customs, 1);
    }

    #[test]
    fn test_execute_customs_action_during_mercenary_sub_turn_resolves_target_area() {
        // President stays in hand (default) and is never placed; a Customs
        // action must still work by resolving the acting area from
        // mercenary_target_area, not president_location.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let customs_area = AreaId::new(9);
        state.board_layout.areas.insert(customs_area, AreaConfig {
            area_type: AreaType::Customs,
            grid_position: (0, 9),
        });
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(customs_area);
        state.players[0].imported_goods = 1;

        let result = execute_customs_action(&mut state, PlayerId::new(0), 3, 1, &rules);
        assert!(result.is_ok());
        assert_eq!(state.players[0].customs_positions, vec![1]);
    }

    #[test]
    fn test_execute_customs_action_rejects_when_not_on_customs_area() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let bank_area = AreaId::new(0);
        state.board_layout.areas.insert(bank_area, AreaConfig {
            area_type: AreaType::Bank,
            grid_position: (0, 0),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(bank_area);
        state.players[0].imported_goods = 1;

        let result = execute_customs_action(&mut state, PlayerId::new(0), 3, 1, &rules);
        assert!(result.is_err());
        assert_eq!(state.players[0].imported_goods, 1);
        assert_eq!(state.players[0].assistants.in_hand, 8);
    }

    #[test]
    fn test_execute_customs_action_arrival_order() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let area = place_president_at_customs(&mut state);
        state.players[1].president_location = PresidentLocation::OnBoard(area);
        state.players[0].imported_goods = 1;
        state.players[1].imported_goods = 3;

        execute_customs_action(&mut state, PlayerId::new(0), 3, 1, &rules).unwrap(); // -> space 1
        execute_customs_action(&mut state, PlayerId::new(1), 5, 3, &rules).unwrap(); // -> space 5

        let idx1 = CustomsTrack::space_to_index(1).unwrap();
        let idx5 = CustomsTrack::space_to_index(5).unwrap();
        assert_eq!(state.customs_track.slots[idx1].unwrap().arrival_order, 0);
        assert_eq!(state.customs_track.slots[idx5].unwrap().arrival_order, 1);
        assert_eq!(state.customs_track.next_arrival_order, 2);
    }

    #[test]
    fn test_execute_customs_action_warehouse_rider_zero_stock_tolerance() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_customs(&mut state);
        state.players[0].imported_goods = 1;
        state.players[0].assistants.in_warehouse = 0;

        // Space 1's rider is +2 warehouse-takes; warehouse is empty.
        let result = execute_customs_action(&mut state, PlayerId::new(0), 3, 1, &rules);
        assert!(result.is_ok());
        let p = &state.players[0];
        assert_eq!(p.assistants.in_warehouse, 0);
        assert_eq!(p.assistants.in_hand, 7); // only the -1 placement; rider tolerated 0 stock
    }

    #[test]
    fn test_execute_customs_action_no_rider_on_high_spaces() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_customs(&mut state);
        state.players[0].imported_goods = 3;
        state.players[0].assistants.in_warehouse = 5;

        // Cost 3 -> space 5 (no rider).
        let result = execute_customs_action(&mut state, PlayerId::new(0), 5, 3, &rules);
        assert!(result.is_ok());
        let p = &state.players[0];
        assert_eq!(p.customs_positions, vec![5]);
        assert_eq!(p.money, 3); // starting 3 (first player), no rider
        assert_eq!(p.assistants.in_warehouse, 5); // untouched
        assert_eq!(p.victory_points, CustomsTrack::vp_reward(5) as u32);
    }

    #[test]
    fn test_execute_customs_action_brickyard_bonus_stacks_per_owned_copy() {
        // Standard data has only 1 physical Brickyard, so stacking is proven
        // with a hand-rolled two-copy registry fixture (mirrors Task 13's
        // Winery stacking precedent).
        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(make_test_tech_card(1, TechEffect::Brickyard), "brickyard_1");
        tech_registry.insert(make_test_tech_card(2, TechEffect::Brickyard), "brickyard_2");
        let mut rules = Rules::empty_for_tests();
        rules.techs = tech_registry;

        let mut state = create_test_state();
        place_president_at_customs(&mut state);
        state.players[0].imported_goods = 3;
        state.players[0].technologies = vec![TechCardId(1), TechCardId(2)]; // owns BOTH copies

        let result = execute_customs_action(&mut state, PlayerId::new(0), 5, 3, &rules);
        assert!(result.is_ok());

        let base_vp = CustomsTrack::vp_reward(5) as u32;
        assert_eq!(state.players[0].victory_points, base_vp + 8); // 2 copies x 4 VP
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
