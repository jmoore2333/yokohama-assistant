//! Church and Customs track actions.

use crate::types::{
    AreaId, AreaType, ChurchSlot, ChurchTrack, CustomsSlot, CustomsTrack, GameState, PendingChoice,
    PlayerId, PlayerState, PresidentLocation,
};
use crate::game::choices::push_choice;
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

/// Resolve the Area the current Church/Customs action is acting at: the
/// mercenary target area during a mercenary sub-turn (mercenary sub-turns
/// bypass the President entirely), else the President's on-board location.
/// Shared by both executors AND both placement tails (Task 5 review item
/// 7 -- previously duplicated inline at all four sites) so the
/// mercenary-aware idiom exists in exactly one place. Executors propagate
/// the `Err` (recoverable -- the action is simply illegal); placement
/// tails call this in a LATER `apply_move` (after a deferred choice), where
/// the queue-first gate in `get_legal_moves` guarantees neither
/// `is_mercenary_action`/`mercenary_target_area` nor `president_location`
/// can have drifted since the executor already validated them, so they
/// `.expect(...)` instead. `pub(crate)` since `apply_move::resolve_recovery`
/// (Task 6) also needs the acting area to decide the Telephone Recovery
/// queue-vs-auto split.
pub(crate) fn resolve_acting_area(state: &GameState, player: PlayerId) -> Result<AreaId, &'static str> {
    if state.is_mercenary_action {
        state.mercenary_target_area.ok_or("No mercenary target area")
    } else {
        match state.player(player).president_location {
            PresidentLocation::OnBoard(a) => Ok(a),
            _ => Err("President not on board"),
        }
    }
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

/// Empty Church spaces with value <= `total_faith`, ascending. In
/// 2-player games the faith-7 space is pre-blocked by a printed dummy pawn
/// — an eligibility rule, not a `slots` entry, so arrival-order and
/// end-trigger pawn counts (which count real players only) stay untouched.
/// Faith below 4 yields no eligible space; faith above 11 is capped at the
/// top of the track.
///
/// `pub` (Task 17, extended Task 4): reused directly by the Church
/// legal-move generator (feasibility only) and by `execute_church_action` /
/// the `ChurchSpace` choice generator (the actual candidate set), so
/// enumeration can never drift from the executor's own acceptance criteria.
/// Supersedes `highest_eligible_church_space`: choosing a lower space is no
/// longer auto-decided by the executor — Task 4 defers that pick to the
/// player via `PendingChoice::ChurchSpace` whenever >=2 spaces qualify.
pub fn eligible_church_spaces(track: &ChurchTrack, total_faith: u8, player_count: u8) -> Vec<u8> {
    let ceiling = total_faith.min(11);
    (4..=ceiling).filter(|&faith| {
        if player_count == 2 && faith == 7 {
            return false;
        }
        let index = ChurchTrack::faith_to_index(faith).expect("4..=11 always maps to a valid index");
        track.slots[index].is_none()
    }).collect()
}

/// Execute Church action: resolve the acting area (mirrors
/// `execute_port_action`/`execute_lab_action` — a mercenary sub-turn acts
/// at the mercenary target area, not the president's; Church has only one
/// area, so this resolution exists solely to confirm the actor is
/// actually there), validate contributions/assistant, compute total
/// Faith, pay contributed goods, and either place inline (exactly one
/// eligible space) or defer the pick to the player via
/// `PendingChoice::ChurchSpace` (design 2026-07-18 §5.2, Task 4) — the
/// assistant is NOT consumed yet in the deferred case; see
/// `place_church_assistant`.
///
/// If total Faith < 4, no eligible empty space exists, or the player has
/// no assistant available (hand or acting area), the action is ILLEGAL
/// (v1 semantic: goods are never paid for a placement that doesn't happen,
/// keeping the legal-move projection conservative).
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
    let area = resolve_acting_area(state, player)?;
    match state.board_layout.areas.get(&area).map(|c| c.area_type) {
        Some(AreaType::Church) => {}
        _ => return Err("Church action requires the Church area"),
    }

    validate_church_action(state, player, area, copper, silk, tea, seafood, yen, imported_good)?;

    let total_faith = calculate_church_faith(power, copper, silk, tea, seafood, yen, imported_good);
    let spaces = eligible_church_spaces(&state.church_track, total_faith, state.config.player_count);
    if spaces.is_empty() {
        return Err("No eligible Church space for this Faith total");
    }

    // Pay contributed goods now, regardless of whether placement resolves
    // inline below or is deferred behind a ChurchSpace choice -- the
    // player already committed these when taking the action.
    let p = state.player_mut(player);
    if copper { p.copper -= 1; }
    if silk { p.silk -= 1; }
    if tea { p.tea -= 1; }
    if seafood { p.seafood -= 1; }
    if yen { p.money -= 1; }
    if imported_good { p.imported_goods -= 1; }

    match spaces.len() {
        0 => unreachable!("validated non-empty above"),
        // `?`, not `.expect(...)`: the acting area was already resolved
        // moments ago in this same call, so this can never actually Err --
        // propagating costs nothing and needs no separate soundness
        // argument (I2 final-review fix).
        1 => place_church_assistant(state, player, spaces[0], rules)?,
        _ => state.pending_choices.push(PendingChoice::ChurchSpace { total_faith }),
    }

    Ok(())
}

/// Placement tail for a resolved Church space: called either inline from
/// `execute_church_action` (exactly one eligible space) or from
/// `ChooseChurchSpace` resolution (`game::choices::resolve_choice_move`).
/// Consumes the assistant (hand-preferred, acting-area fallback per I3 —
/// the acting area is resolved via the shared `resolve_acting_area` helper
/// (Task 5 review item 7), since a deferred resolution can land in a
/// later `apply_move` call after the triggering action), records the slot
/// with arrival order, awards the space's VP plus Winery bonus per owned
/// copy, then queues the space's movement riders (`ChurchTrack::movement_rider`)
/// as `AssistantMove` choices (design 2026-07-18 §5.2).
///
/// Returns `Err` iff `resolve_acting_area` fails (I2 final-review fix):
/// under ordinary legal-list play this can't happen (queue-first gating
/// guarantees no drift between the triggering `execute_church_action` and
/// a later `ChooseChurchSpace` resolution), but a tampered record can
/// insert an intervening move (e.g. `ReturnPresidentToHand`, which
/// validates nothing) between the two -- `resolve_choice_move` propagates
/// this as a typed `MoveError::RuleViolation` instead of panicking.
pub fn place_church_assistant(state: &mut GameState, player: PlayerId, space: u8, rules: &Rules) -> Result<(), &'static str> {
    let area = resolve_acting_area(state, player)?;

    // Computed via immutable borrow before any mutation below.
    let winery_bonus = church_vp_bonus(state, player, &rules.techs);
    let base_vp = ChurchTrack::vp_reward(space) as u32;
    let index = ChurchTrack::faith_to_index(space).expect("space always in 4..=11");
    let arrival_order = state.church_track.next_arrival_order;

    state.church_track.slots[index] = Some(ChurchSlot { player, arrival_order });
    state.church_track.next_arrival_order += 1;

    let p = state.player_mut(player);
    take_church_or_customs_assistant(p, area);
    p.assistants.on_church += 1;
    p.church_positions.push(space);
    p.victory_points += base_vp + winery_bonus;

    for _ in 0..ChurchTrack::movement_rider(space) {
        push_choice(state, PendingChoice::AssistantMove, rules);
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

/// Empty, unblocked Customs spaces for an exact printed cost, ordered
/// LOWER SPACE NUMBER FIRST (which is VP-descending for every printed pair
/// -- unlike Church, where higher space numbers carry higher VP). In
/// 2-player games space 4 (VP 9) is pre-blocked by a printed dummy pawn --
/// an eligibility rule, not a `slots` entry, mirroring the Church faith-7
/// 2P block so arrival order and the end-trigger assistant count stay
/// untouched. Returns an empty `Vec` if `cost` doesn't match a printed
/// cost, or every candidate space is occupied/blocked.
///
/// `pub`: reused directly by the Customs legal-move generator (feasibility
/// only) and by `execute_customs_action` / the `CustomsSpace` choice
/// generator (the actual candidate set), so enumeration can never drift
/// from the executor's own acceptance criteria. Supersedes
/// `eligible_customs_space_for_cost` (Task 17): choosing the higher-VP
/// space of a pair is no longer auto-decided by the executor -- Task 5
/// defers that pick to the player via `PendingChoice::CustomsSpace`
/// whenever >=2 spaces qualify.
pub fn eligible_customs_spaces_for_cost(track: &CustomsTrack, cost: u8, player_count: u8) -> Vec<u8> {
    let candidates = match CustomsTrack::spaces_for_cost(cost) {
        Some(c) => c,
        None => return Vec::new(),
    };
    candidates.iter().copied().filter(|&space| {
        if player_count == 2 && space == 4 {
            return false;
        }
        let index = CustomsTrack::space_to_index(space).expect("space always in 1..=7");
        track.slots[index].is_none()
    }).collect()
}

/// Execute Customs action: resolve the acting area (mirrors
/// `execute_church_action` -- a mercenary sub-turn acts at the mercenary
/// target area, not the president's; via the shared `resolve_acting_area`
/// helper, Task 5 review item 7), validate the payment and assistant,
/// compute the eligible spaces for the exact printed cost, pay imported
/// goods, and either place inline (exactly one eligible space) or defer
/// the pick to the player via `PendingChoice::CustomsSpace` (design
/// 2026-07-18 §5.2, Task 5) -- the assistant is NOT consumed yet in the
/// deferred case; see `place_customs_assistant`.
///
/// If the payment doesn't exactly match a printed cost, every space at
/// that cost is occupied or blocked, or the player has no assistant
/// available (hand or acting area), the action is ILLEGAL (v1 semantic,
/// same as Church: imported goods are never paid for a placement that
/// doesn't happen).
pub fn execute_customs_action(
    state: &mut GameState,
    player: PlayerId,
    power: u8,
    imported_goods_paid: u8,
    rules: &Rules,
) -> Result<(), &'static str> {
    let area = resolve_acting_area(state, player)?;
    match state.board_layout.areas.get(&area).map(|c| c.area_type) {
        Some(AreaType::Customs) => {}
        _ => return Err("Customs action requires the Customs area"),
    }

    validate_customs_action(state, player, area, imported_goods_paid, power)?;

    let spaces = eligible_customs_spaces_for_cost(
        &state.customs_track,
        imported_goods_paid,
        state.config.player_count,
    );
    if spaces.is_empty() {
        return Err("No eligible Customs space for this payment");
    }

    // Pay imported goods now, regardless of whether placement resolves
    // inline below or is deferred behind a CustomsSpace choice -- the
    // player already committed these when taking the action.
    let p = state.player_mut(player);
    p.imported_goods -= imported_goods_paid as u32;

    match spaces.len() {
        0 => unreachable!("validated non-empty above"),
        // `?`, not `.expect(...)`: same reasoning as execute_church_action's
        // singleton arm (I2 final-review fix).
        1 => place_customs_assistant(state, player, spaces[0], rules)?,
        _ => state.pending_choices.push(PendingChoice::CustomsSpace { cost: imported_goods_paid }),
    }

    Ok(())
}

/// Placement tail for a resolved Customs space: called either inline from
/// `execute_customs_action` (exactly one eligible space) or from
/// `ChooseCustomsSpace` resolution (`game::choices::resolve_choice_move`).
/// Consumes the assistant (hand-preferred, acting-area fallback per I3 --
/// the acting area is resolved via the shared `resolve_acting_area` helper,
/// since a deferred resolution can land in a later `apply_move` call after
/// the triggering action), records the slot with arrival order, awards the
/// space's VP plus Brickyard bonus per owned copy, applies the space's Yen
/// rider immediately, then queues the space's warehouse-take rider
/// (`CustomsTrack::rider`) as `WarehouseTake` choices via `push_choice` --
/// the same mechanism order/space Warehouse-icon rewards use (design
/// 2026-07-18 §5.2, Task 5; no new reward machinery).
///
/// Returns `Err` iff `resolve_acting_area` fails (I2 final-review fix) --
/// see `place_church_assistant`'s doc comment for the exact (tampered-record-
/// only) scenario this guards against; `resolve_choice_move` propagates it
/// as a typed `MoveError::RuleViolation` instead of panicking.
pub fn place_customs_assistant(state: &mut GameState, player: PlayerId, space: u8, rules: &Rules) -> Result<(), &'static str> {
    let area = resolve_acting_area(state, player)?;

    // Computed via immutable borrows before any mutation below.
    let brickyard_bonus = customs_vp_bonus(state, player, &rules.techs);
    let base_vp = CustomsTrack::vp_reward(space) as u32;
    let (warehouse_takes, yen_rider) = CustomsTrack::rider(space);
    let index = CustomsTrack::space_to_index(space).expect("space always in 1..=7");
    let arrival_order = state.customs_track.next_arrival_order;

    state.customs_track.slots[index] = Some(CustomsSlot { player, arrival_order });
    state.customs_track.next_arrival_order += 1;

    let p = state.player_mut(player);
    take_church_or_customs_assistant(p, area);
    p.assistants.on_customs += 1;
    p.customs_positions.push(space);
    p.victory_points += base_vp + brickyard_bonus;
    p.money += yen_rider as u32;

    // Warehouse-take rider: one WarehouseTake choice per printed take
    // (design 2026-07-18 §5.2) -- queues, or auto-resolves via the
    // singleton rule (push_choice) if nothing in the warehouse is worth
    // taking.
    for _ in 0..warehouse_takes {
        push_choice(state, PendingChoice::WarehouseTake, rules);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use crate::game::apply_move::{apply_move, apply_move_unchecked};
    use crate::game::cards::TechRegistry;
    use crate::game::get_legal_moves;
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
    fn test_eligible_church_spaces_basic() {
        let track = ChurchTrack::new();
        assert_eq!(eligible_church_spaces(&track, 5, 3), vec![4, 5]);
    }

    #[test]
    fn test_eligible_church_spaces_excludes_occupied() {
        let mut track = ChurchTrack::new();
        let idx5 = ChurchTrack::faith_to_index(5).unwrap();
        track.slots[idx5] = Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 0 });
        assert_eq!(eligible_church_spaces(&track, 5, 3), vec![4]);
    }

    #[test]
    fn test_eligible_church_spaces_below_4_is_empty() {
        let track = ChurchTrack::new();
        assert_eq!(eligible_church_spaces(&track, 3, 3), Vec::<u8>::new());
    }

    #[test]
    fn test_eligible_church_spaces_empty_when_all_occupied() {
        let mut track = ChurchTrack::new();
        for faith in 4..=5 {
            let idx = ChurchTrack::faith_to_index(faith).unwrap();
            track.slots[idx] = Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 0 });
        }
        assert_eq!(eligible_church_spaces(&track, 5, 3), Vec::<u8>::new());
    }

    #[test]
    fn test_eligible_church_spaces_2p_blocks_faith_7() {
        let track = ChurchTrack::new();
        assert_eq!(eligible_church_spaces(&track, 7, 2), vec![4, 5, 6]);
        // 3P (and 4P): no dummy pawn, faith-7 is reachable.
        assert_eq!(eligible_church_spaces(&track, 7, 3), vec![4, 5, 6, 7]);
    }

    #[test]
    fn test_execute_church_action_places_assistant_and_awards_vp() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_church(&mut state);
        state.players[0].copper = 1;
        state.players[0].silk = 1;

        // Power 3 + copper + silk = faith 5 -> 2 eligible spaces (4,5) on a
        // fresh track -> goods paid, placement deferred behind a
        // ChurchSpace choice (design §5.2); the assistant is not consumed yet.
        let result = execute_church_action(
            &mut state, PlayerId::new(0), 3, true, true, false, false, false, false, &rules,
        );
        assert!(result.is_ok());
        assert_eq!(state.pending_choices, vec![PendingChoice::ChurchSpace { total_faith: 5 }]);
        assert_eq!(state.players[0].copper, 0);
        assert_eq!(state.players[0].silk, 0);
        assert_eq!(state.players[0].assistants.in_hand, 8, "not consumed until the space choice resolves");

        // Resolve as if the player picked space 5 -- proves the full
        // placement tail (assistant consumption, VP, slot, arrival order).
        place_church_assistant(&mut state, PlayerId::new(0), 5, &rules).unwrap();

        let p = &state.players[0];
        assert_eq!(p.assistants.in_hand, 7); // started at 8
        assert_eq!(p.assistants.on_church, 1);
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

        // Power 5, no goods = faith 5 -> 2 eligible spaces -> defers to a
        // ChurchSpace choice; the I3 fallback happens at placement (below).
        let result = execute_church_action(
            &mut state, PlayerId::new(0), 5, false, false, false, false, false, false, &rules,
        );
        assert!(result.is_ok());
        assert_eq!(state.players[0].assistants.on_board.get(&area), Some(&2), "not consumed until placement resolves");

        place_church_assistant(&mut state, PlayerId::new(0), 5, &rules).unwrap();

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

        place_church_assistant(&mut state, PlayerId::new(0), 5, &rules).unwrap();

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

        place_church_assistant(&mut state, PlayerId::new(0), 5, &rules).unwrap();

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
        place_church_assistant(&mut state, PlayerId::new(0), 5, &rules).unwrap();
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

        // Power 5 + copper + silk = faith 7, but 2P blocks the faith-7 space
        // -- 3 spaces (4,5,6) remain eligible, so placement defers to a
        // ChurchSpace choice (design §5.2) rather than auto-placing.
        let result = execute_church_action(
            &mut state, PlayerId::new(0), 5, true, true, false, false, false, false, &rules,
        );
        assert!(result.is_ok());
        assert_eq!(state.pending_choices, vec![PendingChoice::ChurchSpace { total_faith: 7 }]);
        let legal = get_legal_moves(&state, &rules);
        assert!(!legal.contains(&Move::ChooseChurchSpace { space: 7 }), "2P dummy pawn must exclude faith-7");

        // Resolve to space 6 (the old auto-place target) to prove placement still works end to end.
        place_church_assistant(&mut state, PlayerId::new(0), 6, &rules).unwrap();
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

    // --- Task 4: ChurchSpace choice + movement riders (design 2026-07-18 §5.2) ---

    /// Church move shared by the ChurchSpace-choice fixtures below: fixed
    /// contributions of copper+silk+tea (+3 Faith) so `church_fixture` /
    /// `church_2p_fixture` can reach Faith totals above the power cap of 5
    /// (`church_faith_from_power` caps at 5) by varying ONLY
    /// `current_action_power` -- the single lever those fixtures tune.
    fn church_power_only_move() -> Move {
        Move::ExecuteAreaAction {
            action: AreaAction::Church {
                copper: true, silk: true, tea: true,
                seafood: false, yen: false, imported_good: false,
            },
        }
    }

    /// Church area + president there (3P, so the 2P faith-7 dummy never
    /// interferes), player 0 stocked with the 3 goods `church_power_only_move`
    /// spends, `current_action_power` set so Faith = power + 3 lands on
    /// `total_faith` (power = total_faith - 3, always <= 5 for the Faith
    /// values these tests use). Fresh (empty) Church track --
    /// `eligible_church_spaces`'s ceiling at `total_faith` naturally limits
    /// which spaces are ever considered, so nothing needs pre-occupying.
    fn church_fixture(total_faith: u8) -> (GameState, Rules) {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state_3p();
        place_president_at_church(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(total_faith - 3);
        state.players[0].copper = 1;
        state.players[0].silk = 1;
        state.players[0].tea = 1;
        (state, rules)
    }

    /// Faith 4 via the shared move (power 1): a fresh Church track's ceiling
    /// at Faith 4 naturally admits only space 4 -- the singleton-placement
    /// case.
    fn church_single_space_fixture() -> (GameState, Rules) {
        church_fixture(4)
    }

    /// 2-player Church fixture at the given Faith total (`create_test_state`
    /// is the 2P base) -- exercises the printed faith-7 dummy block.
    fn church_2p_fixture(total_faith: u8) -> (GameState, Rules) {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state(); // 2P by construction
        place_president_at_church(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(total_faith - 3);
        state.players[0].copper = 1;
        state.players[0].silk = 1;
        state.players[0].tea = 1;
        (state, rules)
    }

    #[test]
    fn church_movement_riders_match_decision_log() {
        assert_eq!((4..=11).map(ChurchTrack::movement_rider).collect::<Vec<_>>(), vec![1,1,2,2,2,2,3,3]);
    }

    #[test]
    fn church_with_multiple_eligible_spaces_queues_space_choice() {
        // Faith 6, spaces 4,5,6 empty -> 3 eligible -> queue, nothing placed yet.
        let (state, rules) = church_fixture(/*faith yields*/ 6);
        let s = apply_move(&state, &church_power_only_move(), &rules).unwrap();
        assert_eq!(s.pending_choices, vec![PendingChoice::ChurchSpace { total_faith: 6 }]);
        assert_eq!(s.church_track.total_assistants(), 0);
        let legal = get_legal_moves(&s, &rules);
        assert_eq!(legal, vec![
            Move::ChooseChurchSpace { space: 4 },
            Move::ChooseChurchSpace { space: 5 },
            Move::ChooseChurchSpace { space: 6 },
        ]);
    }

    #[test]
    fn choosing_church_space_places_scores_and_queues_riders() {
        let (state, rules) = church_fixture(6);
        let mid = apply_move(&state, &church_power_only_move(), &rules).unwrap();
        let s = apply_move(&mid, &Move::ChooseChurchSpace { space: 6 }, &rules).unwrap();
        assert_eq!(s.church_track.total_assistants(), 1);
        // faith-6 space: 5 VP (existing vp table) + rider move 2 queued
        assert_eq!(s.pending_choices, vec![PendingChoice::AssistantMove, PendingChoice::AssistantMove]);
    }

    #[test]
    fn church_with_single_eligible_space_places_inline_no_choice_event() {
        // Faith 4, only space 4 empty-eligible -> inline placement (singleton, design §5.1).
        let (state, rules) = church_single_space_fixture();
        let s = apply_move(&state, &church_power_only_move(), &rules).unwrap();
        assert_eq!(s.church_track.total_assistants(), 1);
        assert!(s.pending_choices.iter().all(|c| !matches!(c, PendingChoice::ChurchSpace { .. })));
    }

    #[test]
    fn church_2p_faith7_dummy_space_never_offered() {
        let (state, rules) = church_2p_fixture(/*faith*/ 8);
        let s = apply_move(&state, &church_power_only_move(), &rules).unwrap();
        let legal = get_legal_moves(&s, &rules);
        assert!(!legal.contains(&Move::ChooseChurchSpace { space: 7 }));
    }

    #[test]
    fn church_space_choice_deferred_across_mercenary_sub_turn_return() {
        // Task 1 review M3 + I3 coverage: a mercenary Church action with
        // >=2 eligible spaces queues ChurchSpace; the mercenary sub-turn's
        // return to the additional-action window must NOT fire until the
        // WHOLE cascade -- the space choice AND its movement riders -- has
        // drained (the apply_move deferral gate keys the mercenary teardown
        // on pending_choices.is_empty() && turn_phase == Recovery, not on
        // the choice queue alone). Also exercises I3's acting-area fallback
        // under a mercenary sub-turn: hand is empty, so the assistant must
        // come from mercenary_target_area (which holds the player's
        // assistant by definition -- apply_mercenary validates this before
        // starting the sub-turn).
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state_3p();
        let church_area = AreaId::new(0);
        let other_area = AreaId::new(1);
        state.board_layout.areas.insert(church_area, AreaConfig { area_type: AreaType::Church, grid_position: (0, 0) });
        state.board_layout.areas.insert(other_area, AreaConfig { area_type: AreaType::Bank, grid_position: (0, 1) });
        state.is_mercenary_action = true;
        state.mercenary_target_area = Some(church_area);
        state.mercenary_return_phase = Some(TurnPhase::AdditionalActionSecond);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3); // + 3 fixed goods (church_power_only_move) = faith 6
        state.players[0].copper = 1;
        state.players[0].silk = 1;
        state.players[0].tea = 1;
        state.players[0].assistants.in_hand = 0; // I3: forces the acting-area fallback
        state.players[0].assistants.on_board.insert(church_area, 2); // 1 for placement, 1 left for the rider

        // Church action: pays goods, faith 6 -> 3 eligible spaces -> queues ChurchSpace, places nothing yet.
        let mid = apply_move(&state, &church_power_only_move(), &rules).unwrap();
        assert_eq!(mid.pending_choices, vec![PendingChoice::ChurchSpace { total_faith: 6 }]);
        assert!(mid.is_mercenary_action, "mercenary sub-turn must not have torn down mid-cascade");
        assert_eq!(mid.turn_phase, TurnPhase::FivePowerBonus);

        // Resolve the space choice: I3 fallback takes the assistant from
        // the mercenary target area (hand is empty), then queues 2
        // movement riders (faith-6 rider count) -- still not torn down.
        let after_space = apply_move(&mid, &Move::ChooseChurchSpace { space: 6 }, &rules).unwrap();
        assert_eq!(after_space.players[0].assistants.on_board.get(&church_area), Some(&1), "I3 fallback: 1 of 2 area assistants consumed");
        assert_eq!(after_space.pending_choices, vec![PendingChoice::AssistantMove, PendingChoice::AssistantMove]);
        assert!(after_space.is_mercenary_action, "still mid-cascade after the space choice");
        assert_eq!(after_space.turn_phase, TurnPhase::FivePowerBonus);

        // Resolve the first rider: one choice still pending -- sub-turn still up.
        let after_rider_1 = apply_move(&after_space, &Move::DeclineAssistantMove, &rules).unwrap();
        assert_eq!(after_rider_1.pending_choices, vec![PendingChoice::AssistantMove]);
        assert!(after_rider_1.is_mercenary_action, "still mid-cascade after the first rider");

        // Resolve the last rider: the choice queue drains, but turn_phase
        // is still FivePowerBonus (Choose* moves never touch turn_phase),
        // so the mercenary return does NOT fire yet -- it's gated on
        // turn_phase == Recovery, not merely on an empty queue.
        let after_rider_2 = apply_move(&after_rider_1, &Move::DeclineAssistantMove, &rules).unwrap();
        assert!(after_rider_2.pending_choices.is_empty());
        assert!(after_rider_2.is_mercenary_action, "queue drained but still FivePowerBonus, not Recovery yet");
        assert_eq!(after_rider_2.turn_phase, TurnPhase::FivePowerBonus);

        // Walk the remaining deterministic phase steps (no more choices
        // queue from these): Skip5PowerBonus -> Building -> SkipBuilding ->
        // Recovery, whose automatic post-processing gate (pending_choices
        // empty AND turn_phase == Recovery) fires the mercenary return in
        // this same call.
        let after_5power = apply_move(&after_rider_2, &Move::Skip5PowerBonus, &rules).unwrap();
        assert_eq!(after_5power.turn_phase, TurnPhase::Building);
        assert!(after_5power.is_mercenary_action);

        let returned = apply_move(&after_5power, &Move::SkipBuilding, &rules).unwrap();
        assert!(!returned.is_mercenary_action, "mercenary sub-turn torn down once Recovery's gate fires");
        assert_eq!(returned.turn_phase, TurnPhase::AdditionalActionSecond, "returned to the stored mercenary_return_phase");
        assert_eq!(returned.mercenary_target_area, None);
    }

    #[test]
    fn tampered_return_to_hand_mid_church_space_queue_errors_not_panics() {
        // I2 final-review fix: apply_move validates nothing about
        // pending_choices before dispatching a move (I1(a)'s gap), so a
        // tampered record can insert a legal-LOOKING ReturnPresidentToHand
        // while a ChurchSpace choice is queued -- it applies Ok and moves
        // the President to hand. The queued choice's membership check
        // (eligible_church_spaces) depends only on the track/faith, not
        // president location, so ChooseChurchSpace still passes it -- but
        // placing now has no acting area to resolve. Must surface a typed
        // error, never panic (previously an unconditional .expect() in
        // place_church_assistant).
        let (state, rules) = church_fixture(6); // >=2 eligible spaces -> queues ChurchSpace
        let mid = apply_move(&state, &church_power_only_move(), &rules).unwrap();
        assert_eq!(mid.pending_choices, vec![PendingChoice::ChurchSpace { total_faith: 6 }]);

        // apply_move_unchecked: the tamper itself is the point of this test
        // -- ReturnPresidentToHand is not in get_legal_moves while a choice
        // is queued (queue-first gate), so checked apply_move now rejects it
        // with NotLegal before ever reaching the raw arm this test exercises.
        let tampered = apply_move_unchecked(
            &mid, &Move::ReturnPresidentToHand { recover: std::collections::BTreeMap::new() }, &rules,
        ).unwrap();
        assert_eq!(tampered.players[0].president_location, PresidentLocation::InHand);
        assert_eq!(
            tampered.pending_choices, vec![PendingChoice::ChurchSpace { total_faith: 6 }],
            "queue survives the tampering untouched"
        );

        let result = apply_move(&tampered, &Move::ChooseChurchSpace { space: 6 }, &rules);
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
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
    fn test_eligible_customs_spaces_for_cost_basic() {
        let track = CustomsTrack::new();
        assert_eq!(eligible_customs_spaces_for_cost(&track, 1, 3), vec![1, 2]);
        assert_eq!(eligible_customs_spaces_for_cost(&track, 2, 3), vec![3, 4]);
        assert_eq!(eligible_customs_spaces_for_cost(&track, 3, 3), vec![5, 6]);
        assert_eq!(eligible_customs_spaces_for_cost(&track, 4, 3), vec![7]);
    }

    #[test]
    fn test_eligible_customs_spaces_for_cost_excludes_occupied() {
        let mut track = CustomsTrack::new();
        let idx1 = CustomsTrack::space_to_index(1).unwrap();
        track.slots[idx1] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 });
        assert_eq!(eligible_customs_spaces_for_cost(&track, 1, 3), vec![2]);
    }

    #[test]
    fn test_eligible_customs_spaces_for_cost_both_occupied_is_empty() {
        let mut track = CustomsTrack::new();
        for space in [1, 2] {
            let idx = CustomsTrack::space_to_index(space).unwrap();
            track.slots[idx] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 });
        }
        assert_eq!(eligible_customs_spaces_for_cost(&track, 1, 3), Vec::<u8>::new());
    }

    #[test]
    fn test_eligible_customs_spaces_for_cost_invalid_cost_is_empty() {
        let track = CustomsTrack::new();
        assert_eq!(eligible_customs_spaces_for_cost(&track, 0, 3), Vec::<u8>::new());
        assert_eq!(eligible_customs_spaces_for_cost(&track, 5, 3), Vec::<u8>::new());
    }

    #[test]
    fn test_eligible_customs_spaces_for_cost_2p_blocks_space4() {
        let mut track = CustomsTrack::new();
        let idx3 = CustomsTrack::space_to_index(3).unwrap();
        track.slots[idx3] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 });
        assert_eq!(eligible_customs_spaces_for_cost(&track, 2, 2), Vec::<u8>::new());
        // 3P (and 4P): no dummy pawn, space 4 is reachable.
        assert_eq!(eligible_customs_spaces_for_cost(&track, 2, 3), vec![4]);
    }

    #[test]
    fn test_execute_customs_action_places_assistant_and_awards_vp() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_customs(&mut state);
        state.players[0].imported_goods = 1;
        // Warehouse fully empty so space 1's +2-warehouse-takes rider
        // auto-declines (push_choice singleton rule, design §5.1) instead of
        // queuing -- rider queueing itself is covered by
        // customs_space_one_rider_queues_two_warehouse_takes.
        state.players[0].assistants.in_warehouse = 0;
        state.players[0].shophouses.in_warehouse = 0;
        state.players[0].trading_houses.in_warehouse = 0;

        // Power 3 (max 2), pay 1 -> cost pair {1,2}; both empty on a fresh
        // track -> 2 eligible -> placement deferred behind a CustomsSpace
        // choice (design §5.2, Task 5) instead of auto-placing.
        let result = execute_customs_action(&mut state, PlayerId::new(0), 3, 1, &rules);
        assert!(result.is_ok());
        assert_eq!(state.pending_choices, vec![PendingChoice::CustomsSpace { cost: 1 }]);
        assert_eq!(state.players[0].assistants.in_hand, 8, "not consumed until the space choice resolves");
        assert_eq!(state.players[0].imported_goods, 0, "imports paid up front regardless of deferral");

        // Resolve as if the player picked space 1 -- proves the full
        // placement tail (assistant consumption, VP, slot, arrival order,
        // rider). Calling place_customs_assistant directly, as here, does
        // NOT pop the queue -- that is resolve_choice_move's job, covered
        // separately by choosing_customs_space_places_scores_and_applies_riders
        // -- so the CustomsSpace entry above is still expected below.
        place_customs_assistant(&mut state, PlayerId::new(0), 1, &rules).unwrap();

        let p = &state.players[0];
        assert_eq!(p.assistants.in_hand, 7); // 8 - 1 (placement); rider auto-declined
        assert_eq!(p.assistants.on_customs, 1);
        assert_eq!(p.assistants.in_warehouse, 0);
        assert_eq!(p.imported_goods, 0);
        assert_eq!(p.customs_positions, vec![1]);
        assert_eq!(p.victory_points, CustomsTrack::vp_reward(1) as u32);
        assert_eq!(
            state.customs_track.slots[CustomsTrack::space_to_index(1).unwrap()].unwrap().player,
            PlayerId::new(0)
        );
        // Rider auto-declined (empty warehouse) -> no NEW choice appended
        // beyond the still-unpopped CustomsSpace entry from above.
        assert_eq!(state.pending_choices, vec![PendingChoice::CustomsSpace { cost: 1 }]);
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

        // Power 4 (max 3), pay 2 -> cost pair {3,4}; space 3 taken -> only
        // space 4 eligible -> singleton, still places inline.
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
        // the Customs Area OR your hand." Uses power 5 / pay 3 -> cost pair
        // {5,6} (neither carries a rider), so the assertions below aren't
        // muddied by space 1's +2-warehouse-takes rider (a WarehouseTake
        // choice per take, which would otherwise mask the pawn-source bug
        // this test targets). Both spaces are open on a fresh track ->
        // deferred to a CustomsSpace choice; the I3 fallback is exercised
        // at placement (below), same as Church's analogous test.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let area = place_president_at_customs(&mut state);
        state.players[0].imported_goods = 3;
        state.players[0].assistants.in_hand = 0;
        state.players[0].assistants.on_board.insert(area, 1);

        let result = execute_customs_action(&mut state, PlayerId::new(0), 5, 3, &rules);
        assert!(result.is_ok());
        assert_eq!(state.players[0].assistants.on_board.get(&area), Some(&1), "not consumed until placement resolves");

        place_customs_assistant(&mut state, PlayerId::new(0), 5, &rules).unwrap();

        let p = &state.players[0];
        assert_eq!(p.assistants.in_hand, 0);
        assert_eq!(p.assistants.on_board.get(&area), None, "the sole acting-area pawn is removed, not left at 0");
        assert_eq!(p.assistants.on_customs, 1);
        assert_eq!(p.customs_positions, vec![5]);
    }

    #[test]
    fn test_execute_customs_action_prefers_hand_assistant_when_both_sources_available() {
        // Power 5 / pay 3 -> cost pair {5,6} (neither carries a rider); see
        // the acting-area test's comment for why a rider-free pair matters here.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let area = place_president_at_customs(&mut state);
        state.players[0].imported_goods = 3;
        state.players[0].assistants.in_hand = 3;
        state.players[0].assistants.on_board.insert(area, 2);

        let result = execute_customs_action(&mut state, PlayerId::new(0), 5, 3, &rules);
        assert!(result.is_ok());

        place_customs_assistant(&mut state, PlayerId::new(0), 5, &rules).unwrap();

        let p = &state.players[0];
        assert_eq!(p.assistants.in_hand, 2, "hand pawn spent, not the acting-area one");
        assert_eq!(p.assistants.on_board.get(&area), Some(&2), "acting-area pawns untouched");
    }

    #[test]
    fn test_execute_customs_action_during_mercenary_sub_turn_allows_assistant_from_target_area_when_hand_empty() {
        // Same headline case as Church: every mercenary sub-turn's target
        // area has the player's own assistant by definition. Power 5 / pay
        // 3 -> cost pair {5,6} (neither carries a rider), both open -> deferred.
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

        place_customs_assistant(&mut state, PlayerId::new(0), 5, &rules).unwrap();

        assert_eq!(state.players[0].customs_positions, vec![5]);
        assert_eq!(state.players[0].assistants.on_board.get(&customs_area), None);
    }

    #[test]
    fn test_execute_customs_action_from_area_assistant_then_recovery_does_not_double_recover() {
        // Power 5 / pay 3 -> cost pair {5,6}, both open -> deferred; resolve
        // to space 5 (no rider either way in this pair).
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        let area = place_president_at_customs(&mut state);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[0].imported_goods = 3;
        state.players[0].assistants.in_hand = 0;
        state.players[0].assistants.on_board.insert(area, 1);

        execute_customs_action(&mut state, PlayerId::new(0), 5, 3, &rules).unwrap();
        place_customs_assistant(&mut state, PlayerId::new(0), 5, &rules).unwrap();
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

        place_customs_assistant(&mut state, PlayerId::new(0), 1, &rules).unwrap();

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

        // Both cost pairs are fully open on a fresh track -> both actions
        // defer; arrival order is pinned by PLACEMENT order, not execute order.
        execute_customs_action(&mut state, PlayerId::new(0), 3, 1, &rules).unwrap();
        place_customs_assistant(&mut state, PlayerId::new(0), 1, &rules).unwrap(); // -> space 1
        execute_customs_action(&mut state, PlayerId::new(1), 5, 3, &rules).unwrap();
        place_customs_assistant(&mut state, PlayerId::new(1), 5, &rules).unwrap(); // -> space 5

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
        state.players[0].shophouses.in_warehouse = 0;
        state.players[0].trading_houses.in_warehouse = 0;

        // Cost pair {1,2} both open -> deferred to a CustomsSpace choice.
        let result = execute_customs_action(&mut state, PlayerId::new(0), 3, 1, &rules);
        assert!(result.is_ok());
        assert_eq!(state.pending_choices, vec![PendingChoice::CustomsSpace { cost: 1 }]);

        // Space 1's rider is +2 warehouse-takes; the whole warehouse is
        // empty, so both pushes auto-decline (singleton rule, design §5.1)
        // rather than queuing a choice with nothing left to take.
        place_customs_assistant(&mut state, PlayerId::new(0), 1, &rules).unwrap();
        let p = &state.players[0];
        assert_eq!(p.assistants.in_warehouse, 0);
        assert_eq!(p.assistants.in_hand, 7); // only the -1 placement; rider auto-declined
        // No NEW choice appended (both rider pushes auto-declined); the
        // CustomsSpace entry above is still unpopped since
        // place_customs_assistant was called directly rather than via
        // resolve_choice_move.
        assert_eq!(state.pending_choices, vec![PendingChoice::CustomsSpace { cost: 1 }]);
    }

    #[test]
    fn test_execute_customs_action_no_rider_on_high_spaces() {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_customs(&mut state);
        state.players[0].imported_goods = 3;
        state.players[0].assistants.in_warehouse = 5;

        // Cost 3 -> cost pair {5,6}, both open -> deferred; resolve to space 5 (no rider).
        let result = execute_customs_action(&mut state, PlayerId::new(0), 5, 3, &rules);
        assert!(result.is_ok());

        place_customs_assistant(&mut state, PlayerId::new(0), 5, &rules).unwrap();

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
        // Winery stacking precedent). Cost 3's pair {5,6} is both open on a
        // fresh track -> deferred; resolving through place_customs_assistant
        // exercises Brickyard stacking through the choice path (Task 5 item f).
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
        assert_eq!(state.players[0].victory_points, 0, "not awarded until the space choice resolves");

        place_customs_assistant(&mut state, PlayerId::new(0), 5, &rules).unwrap();

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

    /// Customs action paying 1 import at power 3 -> cost pair {1,2}, both
    /// empty on a fresh track. Player 0's default money (3) and warehouse
    /// stock leave Assistant and Shophouse both affordable, so
    /// `warehouse_take_options` returns more than Decline alone and space
    /// 1's rider queues 2 WarehouseTake choices instead of auto-resolving
    /// once chosen.
    fn customs_space1_fixture() -> (GameState, Rules) {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state();
        place_president_at_customs(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(3);
        state.players[0].imported_goods = 1;
        (state, rules)
    }

    #[test]
    fn customs_space_one_rider_queues_two_warehouse_takes() {
        // Customs action paying 1 import: cost pair {1,2} both open ->
        // deferred to a CustomsSpace choice first; choosing space 1 then
        // queues its +2-warehouse-takes rider.
        let (state, rules) = customs_space1_fixture();
        let mid = apply_move(&state, &Move::ExecuteAreaAction { action: AreaAction::Customs { imported_goods_paid: 1 } }, &rules).unwrap();
        assert_eq!(mid.pending_choices, vec![PendingChoice::CustomsSpace { cost: 1 }]);
        let s = apply_move(&mid, &Move::ChooseCustomsSpace { space: 1 }, &rules).unwrap();
        assert_eq!(s.pending_choices, vec![PendingChoice::WarehouseTake, PendingChoice::WarehouseTake]);
    }

    // --- Task 5: CustomsSpace choice (design 2026-07-18 §5.2) ---

    /// Customs move paying a fixed cost, reused across the
    /// CustomsSpace-choice fixtures below.
    fn customs_move(imported_goods_paid: u8) -> Move {
        Move::ExecuteAreaAction { action: AreaAction::Customs { imported_goods_paid } }
    }

    /// Customs area + president there (3P, so the 2P space-4 dummy never
    /// interferes), player 0 with enough imported goods to pay `cost` and
    /// power 5 (covers every printed cost up to 4). Fresh (empty) Customs
    /// track -- every cost's full candidate pair/singleton is open, so
    /// which spaces are ever considered is driven entirely by `cost` itself.
    fn customs_fixture(cost: u8) -> (GameState, Rules) {
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state_3p();
        place_president_at_customs(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(5);
        state.players[0].imported_goods = cost as u32;
        (state, rules)
    }

    #[test]
    fn customs_with_both_pair_spaces_open_queues_choice() {
        // Cost 2, spaces 3+4 both open (3P) -> queues, nothing placed yet;
        // imports are paid up front regardless of the deferral.
        let (state, rules) = customs_fixture(2);
        let s = apply_move(&state, &customs_move(2), &rules).unwrap();
        assert_eq!(s.pending_choices, vec![PendingChoice::CustomsSpace { cost: 2 }]);
        assert_eq!(s.customs_track.total_assistants(), 0);
        assert_eq!(s.players[0].imported_goods, 0);
        let legal = get_legal_moves(&s, &rules);
        assert_eq!(legal, vec![
            Move::ChooseCustomsSpace { space: 3 },
            Move::ChooseCustomsSpace { space: 4 },
        ]);
    }

    #[test]
    fn choosing_customs_space_places_scores_and_applies_riders() {
        let (state, rules) = customs_fixture(2);
        let mid = apply_move(&state, &customs_move(2), &rules).unwrap();
        let s = apply_move(&mid, &Move::ChooseCustomsSpace { space: 3 }, &rules).unwrap();
        assert_eq!(s.customs_track.total_assistants(), 1);
        assert_eq!(s.players[0].customs_positions, vec![3]);
        assert_eq!(s.players[0].victory_points, CustomsTrack::vp_reward(3) as u32);
        // Space 3's rider is 1 warehouse-take; default stock/money offer
        // more than Decline alone, so it queues instead of auto-resolving.
        assert_eq!(s.pending_choices, vec![PendingChoice::WarehouseTake]);
    }

    #[test]
    fn customs_with_one_pair_space_open_places_inline() {
        // Cost 2, space 3 pre-occupied -> only space 4 eligible -> inline
        // placement, no CustomsSpace event; space 4's rider (+1 Yen) applies
        // immediately.
        let (mut state, rules) = customs_fixture(2);
        let idx3 = CustomsTrack::space_to_index(3).unwrap();
        state.customs_track.slots[idx3] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 });
        state.customs_track.next_arrival_order = 1;
        let money_before = state.players[0].money;

        let s = apply_move(&state, &customs_move(2), &rules).unwrap();
        assert_eq!(s.players[0].customs_positions, vec![4]);
        assert_eq!(s.players[0].money, money_before + 1);
        assert!(s.pending_choices.is_empty());
    }

    #[test]
    fn customs_2p_space4_dummy_never_offered() {
        // 2P: space 4 is pre-blocked by the printed dummy pawn, so cost 2's
        // pair {3,4} always collapses to the single real candidate (3) --
        // inline placement, CustomsSpace choice never reachable. The
        // "space 3 also occupied -> illegal" companion case is already
        // pinned by test_execute_customs_action_2p_blocks_space4_when_space3_occupied.
        let rules = Rules::empty_for_tests();
        let mut state = create_test_state(); // 2P by construction
        place_president_at_customs(&mut state);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(5);
        state.players[0].imported_goods = 2;

        let s = apply_move(&state, &customs_move(2), &rules).unwrap();
        assert_eq!(s.players[0].customs_positions, vec![3]);
        assert!(s.pending_choices.iter().all(|c| !matches!(c, PendingChoice::CustomsSpace { .. })));
    }

    #[test]
    fn customs_space1_end_to_end_through_apply_move_resolves_both_warehouse_riders() {
        // Task-2 review m1: restore full end-to-end coverage of the +2
        // warehouse-take rider. test_execute_customs_action_places_assistant_and_awards_vp
        // stopped covering resolution once Task 2 converted the rider to a
        // queued choice (that test now zeroes the warehouse so both takes
        // auto-decline). Drives the WHOLE cascade through apply_move: cost 1
        // -> both spaces of the pair open -> CustomsSpace choice -> choose
        // space 1 -> its rider queues 2 WarehouseTake choices -> resolve one
        // Assistant take, one Decline -- then assert final
        // in_hand/in_warehouse/money.
        let (state, rules) = customs_fixture(1);
        let starting_hand = state.players[0].assistants.in_hand;
        let starting_warehouse = state.players[0].assistants.in_warehouse;
        let starting_money = state.players[0].money;

        let mid = apply_move(&state, &customs_move(1), &rules).unwrap();
        assert_eq!(mid.pending_choices, vec![PendingChoice::CustomsSpace { cost: 1 }]);

        let placed = apply_move(&mid, &Move::ChooseCustomsSpace { space: 1 }, &rules).unwrap();
        assert_eq!(placed.pending_choices, vec![PendingChoice::WarehouseTake, PendingChoice::WarehouseTake]);
        assert_eq!(placed.players[0].customs_positions, vec![1]);
        assert_eq!(placed.players[0].victory_points, CustomsTrack::vp_reward(1) as u32);

        let after_take = apply_move(
            &placed, &Move::ChooseWarehouseTake { item: WarehouseItem::Assistant }, &rules,
        ).unwrap();
        assert_eq!(after_take.pending_choices, vec![PendingChoice::WarehouseTake]);

        let final_state = apply_move(&after_take, &Move::DeclineWarehouseTake, &rules).unwrap();
        assert!(final_state.pending_choices.is_empty());

        let p = &final_state.players[0];
        assert_eq!(p.assistants.in_hand, starting_hand - 1 + 1, "placement spent 1, the Assistant take refunded 1");
        assert_eq!(p.assistants.in_warehouse, starting_warehouse - 1);
        assert_eq!(p.money, starting_money, "assistant take is free; decline spends nothing");
    }

    #[test]
    fn tampered_return_to_hand_mid_customs_space_queue_errors_not_panics() {
        // Customs counterpart of tampered_return_to_hand_mid_church_space_queue_errors_not_panics
        // (I2 final-review fix) -- same tampering, same reasoning:
        // eligible_customs_spaces_for_cost's membership check depends only
        // on the track/cost, not president location.
        let (state, rules) = customs_fixture(2); // spaces 3+4 both open (3P) -> queues CustomsSpace
        let mid = apply_move(&state, &customs_move(2), &rules).unwrap();
        assert_eq!(mid.pending_choices, vec![PendingChoice::CustomsSpace { cost: 2 }]);

        // apply_move_unchecked: same reasoning as the church counterpart --
        // the tamper is the point; checked apply_move would reject it with
        // NotLegal before reaching the raw arm under test.
        let tampered = apply_move_unchecked(
            &mid, &Move::ReturnPresidentToHand { recover: std::collections::BTreeMap::new() }, &rules,
        ).unwrap();
        assert_eq!(tampered.players[0].president_location, PresidentLocation::InHand);
        assert_eq!(
            tampered.pending_choices, vec![PendingChoice::CustomsSpace { cost: 2 }],
            "queue survives the tampering untouched"
        );

        let result = apply_move(&tampered, &Move::ChooseCustomsSpace { space: 3 }, &rules);
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
    }
}
