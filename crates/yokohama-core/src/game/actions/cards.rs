//! Port and Laboratory actions.

use crate::types::{AreaType, GameState, LabBoard, MoveError, OrderCardId, PlayerId, PortBoard, PortZone, PortPayment, PresidentLocation, TechCardId, TechEffect};
use crate::game::cards::{check_mercenary_eligibility, refill_port, refill_lab, remove_and_slide_lab, OrderRegistry, TechRegistry};

/// Which Port zones are accessible at a given power level.
pub fn port_accessible_zones(power: u8) -> Vec<PortZone> {
    match power {
        0..=1 => vec![],
        2 => vec![PortZone::Low],
        3 => vec![PortZone::Low, PortZone::Mid],
        _ => vec![PortZone::Low, PortZone::Mid, PortZone::High], // 4-5
    }
}

/// Validate Port action. Cards are named by id (not slot, since slots shift
/// under compaction/refill): each must be present on `port`, in a zone
/// accessible at `power`, distinct from each other, within the ≤2-card and
/// hand-limit caps, and paid for if taking a second card.
pub fn validate_port_action(
    state: &GameState,
    player: PlayerId,
    port: &PortBoard,
    cards_taken: &[OrderCardId],
    payment: Option<PortPayment>,
    power: u8,
) -> Result<(), &'static str> {
    let p = state.player(player);
    let accessible = port_accessible_zones(power);

    // Task 16 ruling: Skip is the only legal way to decline a Port take;
    // an explicit empty take is no longer a legal encoding of "take nothing".
    if cards_taken.is_empty() {
        return Err("Must take 1-2 cards from Port (use Skip to decline)");
    }

    // Check cards are distinct (can't take the same physical card twice)
    if cards_taken.len() == 2 && cards_taken[0] == cards_taken[1] {
        return Err("Cannot take the same card twice");
    }

    // Check every named card is present on this board and in an accessible zone
    for &card_id in cards_taken {
        match port.find_card(card_id) {
            None => return Err("Card not found on Port board"),
            Some(index) => {
                if !accessible.contains(&PortBoard::zone_of(index)) {
                    return Err("Card's zone not accessible at this power level");
                }
            }
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
///
/// Resolves the acting Port board (A or B) from the current acting area
/// (the mercenary target area during a mercenary sub-turn, otherwise the
/// president's location) and reads/refills `state.order_deck` directly;
/// callers no longer choose the board or thread the deck themselves.
pub fn execute_port_action(
    state: &mut GameState,
    player: PlayerId,
    cards_taken: &[OrderCardId],
    payment: Option<PortPayment>,
    power: u8,
) -> Result<(), &'static str> {
    // Resolve acting area: mercenary sub-turns act at the mercenary target
    // area, not the president's location (mirrors apply_build_shophouse in
    // apply_move.rs).
    let area = if state.is_mercenary_action {
        state.mercenary_target_area.ok_or("No mercenary target area")?
    } else {
        match state.player(player).president_location {
            PresidentLocation::OnBoard(a) => a,
            _ => return Err("President not on board"),
        }
    };

    // Resolve which Port board (A or B) that area refers to.
    let port_is_b = match state.board_layout.areas.get(&area).map(|c| c.area_type) {
        Some(AreaType::PortA) => false,
        Some(AreaType::PortB) => true,
        _ => return Err("Port action requires a Port area"),
    };
    if port_is_b && state.port_state.port_b.is_none() {
        return Err("Port B not available");
    }

    // Validate against the resolved board.
    let port_ref = if port_is_b {
        state.port_state.port_b.as_ref().unwrap() // presence confirmed above
    } else {
        &state.port_state.port_a
    };
    validate_port_action(state, player, port_ref, cards_taken, payment, power)?;

    // Pay for second card if taking 2
    if cards_taken.len() == 2 {
        let p = state.player_mut(player);
        match payment {
            Some(PortPayment::TwoYen) => p.money -= 2,
            Some(PortPayment::OneImportedGood) => p.imported_goods -= 1,
            None => unreachable!(), // Validated above
        }
    }

    // Take cards, slide the remainder left, and refill from the deck —
    // done through one borrow of `port` so it can be handed to refill_port
    // (which borrows the disjoint state.order_deck field) without
    // re-resolving the board a second time.
    let port = if port_is_b {
        state.port_state.port_b.as_mut().unwrap() // presence confirmed above
    } else {
        &mut state.port_state.port_a
    };

    // Gather all named cards first (without sliding), so a 2-card take
    // reads both cards from their original slots rather than a slot that
    // already slid after the first take. Presence was already confirmed by
    // validate_port_action above.
    let mut taken_cards = Vec::new();
    for &card_id in cards_taken {
        if let Some(index) = port.find_card(card_id) {
            if let Some(card) = port.slots[index].take() {
                taken_cards.push(card);
            }
        }
    }

    // Slide left via total compaction: collect the remaining cards in
    // left-to-right order and reassign them starting at slot 0. A fixed
    // adjacent-slot shift only handles some gap patterns correctly — e.g.
    // shifting slot 5->4 before 4->3 compacts a contiguous take correctly
    // but strands cards when 2 non-contiguous slots are vacated in the same
    // action. Full compaction handles every combination of taken slots
    // uniformly (same principle as commit 5a9a266's 3-slot fix, generalized
    // to 6 slots).
    let remaining: Vec<OrderCardId> = port.slots.iter_mut().filter_map(|s| s.take()).collect();
    for (i, card_id) in remaining.into_iter().enumerate() {
        port.slots[i] = Some(card_id);
    }

    // Refill from the state-owned deck (rightmost empty spaces first).
    refill_port(port, &mut state.order_deck);

    // Add taken cards to player's hand.
    let p = state.player_mut(player);
    for card_id in taken_cards {
        p.order_cards_in_hand.push(card_id);
    }

    // Apply Power 5 VP bonus.
    if power >= 5 {
        state.player_mut(player).victory_points += 3;
    }

    Ok(())
}

/// Calculate industry value available for Laboratory. Sums in u32 (Phase
/// 2.9 Stage 4, census gap 2): the raw arm feeds arbitrary u8 payloads
/// here, and values near u8::MAX overflowed the old u8 sum in debug builds
/// (arithmetic totality — no input may panic or wrap in any profile).
pub fn calculate_industry_value(
    power: u8,
    extra_yen: u8,
    extra_imported: u8,
) -> u32 {
    // Power gives 1:1 industry value, max 5
    let from_power = power.min(5) as u32;
    // 2 Yen = 1 Industry Value
    let from_yen = extra_yen as u32 / 2;
    // 1 Imported Good = 1 Industry Value
    let from_imported = extra_imported as u32;

    from_power + from_yen + from_imported
}

/// Whether the NEXT technology acquisition owes the extra-technology
/// surcharge (+2 Yen or 1 import), given how many the player already owns.
/// Rulebook p.14 verbatim (D2 owner ruling 2026-08-08): "Starting with your
/// 4th card, each technology card requires an additional cost of 2 yen or 1
/// import" — owning 3 or more means the next card is the 4th or later, so
/// EVERY card from the 4th onward pays it. (The engine started at the 5th
/// until SEMANTICS_REV 6; the KB's "after your 4th" was a misreading.)
pub fn extra_tech_surcharge_required(tech_count: usize) -> bool {
    tech_count >= 3
}

/// Lab position surcharge: positions 0-1 are free, 2-3 cost +1, 4-5 cost +2
/// (board art: `.superpowers/sdd/asset-sweep-report.md` Q4).
pub fn lab_position_surcharge(slot_index: usize) -> u8 {
    match slot_index {
        0 | 1 => 0,
        2 | 3 => 1,
        4 | 5 => 2,
        _ => 0, // Out of bounds - shouldn't happen
    }
}

/// Validate Laboratory action against the resolved board. `lab` must already
/// be the board `slot_index` refers to (the acting Lab board — A or B — is
/// resolved by the caller; buying across boards is illegal, so this function
/// never assumes Lab A).
///
/// Returns the priced `(yen_cost, imported_cost)` the purchase will charge —
/// since Phase 2.9 Stage 4 the executor deducts EXACTLY these validated
/// amounts, so no arithmetic on unvalidated payload fields exists in the
/// execute path (census gap 1: the old executor re-derived the extra-tech
/// surcharge itself and could charge a +2 Yen validation never priced,
/// underflowing `money`).
#[allow(clippy::too_many_arguments)]
pub fn validate_lab_action(
    state: &GameState,
    player: PlayerId,
    lab: &LabBoard,
    slot_index: usize,
    extra_yen: u8,
    extra_imported: u8,
    extra_tech_payment: Option<bool>,
    power: u8,
    tech_registry: &TechRegistry,
) -> Result<(u32, u32), &'static str> {
    let p = state.player(player);

    if slot_index >= 6 {
        return Err("Invalid slot index");
    }

    let tech_id = lab.slots[slot_index].ok_or("No card in slot")?;
    let tech_card = tech_registry.get(tech_id).ok_or("Tech card not found in registry")?;

    // Rulebook General Rules: "players may only own 1 card of the same
    // name." Ten titles circulate as 2 physical copies each so different
    // players can each own one, but a single player may never hold both.
    // Effects are 1:1 with titles in the standard data, so effect equality
    // is the ownership-by-title check (C2 final-review finding).
    let already_owns_title = p.technologies.iter().any(|&owned_id| {
        tech_registry.get(owned_id).map(|t| t.effect == tech_card.effect).unwrap_or(false)
    });
    if already_owns_title {
        return Err("Already own a Technology card with this name");
    }

    // Calculate required industry value
    let required_iv = tech_card.industry_cost;
    let position_surcharge = lab_position_surcharge(slot_index);
    let total_required = required_iv + position_surcharge;

    // Calculate available industry value (u32 sum — census gap 2)
    let available_iv = calculate_industry_value(power, extra_yen, extra_imported);

    if available_iv < total_required as u32 {
        return Err("Insufficient industry value");
    }

    // Exact-cover enumeration convention (Phase 2.9 adjudication against
    // stage-1 oracle spec §7.2-3): the legal-move generator emits only
    // `extra_yen = 2*(deficit-k)`, `extra_imported = k` — so extra payment
    // never exceeds the Industry deficit and extra_yen is always a whole
    // number of 2-Yen Industry units. Raw-arm payloads that over-cover (or
    // waste an odd Yen that buys no Industry) are rejected for consistency
    // rather than silently charged for nothing.
    if !extra_yen.is_multiple_of(2) {
        return Err("Odd extra Yen buys no Industry (2 Yen = 1 Industry; exact covers only)");
    }
    let deficit = (total_required as u32).saturating_sub(power.min(5) as u32);
    if extra_yen as u32 / 2 + extra_imported as u32 > deficit {
        return Err("Payment over-covers the Industry deficit (exact covers only)");
    }

    // Calculate total Yen cost
    let mut yen_cost = extra_yen as u32;

    // Check the extra-technology surcharge (4th card onward, rulebook p.14)
    if extra_tech_surcharge_required(p.technologies.len()) {
        match extra_tech_payment {
            None => return Err("Must pay the extra-technology surcharge (4th card onward)"),
            Some(true) => yen_cost += 2, // Pay 2 Yen
            Some(false) => {
                // Pay 1 Imported Good - validate below
                if p.imported_goods < extra_imported as u32 + 1 {
                    return Err("Not enough imported goods for the extra-technology surcharge");
                }
            }
        }
    } else if extra_tech_payment.is_some() {
        // Census gap 1: a supplied payment for a surcharge that is not due
        // must be rejected here — the old validation ignored it while the
        // executor still charged it (money underflow for Some(true)).
        return Err("Extra-technology payment supplied but no surcharge is due");
    }

    // Validate payment
    if p.money < yen_cost {
        return Err("Not enough Yen");
    }

    let imported_cost = extra_imported as u32 + if extra_tech_payment == Some(false) { 1 } else { 0 };
    if p.imported_goods < imported_cost {
        return Err("Not enough imported goods");
    }

    Ok((yen_cost, imported_cost))
}

/// Execute Laboratory action: resolve the acting Lab board (A or B) from the
/// current acting area — mirrors `execute_port_action`'s area resolution, so
/// a mercenary sub-turn acts at the mercenary target area, not the
/// president's — find `card_id` on THAT board only (cross-board buying is
/// illegal, hence the typed `MoveError::CardNotFound` rather than a generic
/// message), then acquire it and refill from `state.tech_deck`.
#[allow(clippy::too_many_arguments)]
pub fn execute_lab_action(
    state: &mut GameState,
    player: PlayerId,
    card_id: TechCardId,
    extra_yen: u8,
    extra_imported: u8,
    extra_tech_payment: Option<bool>,
    power: u8,
    tech_registry: &TechRegistry,
) -> Result<(), MoveError> {
    // Resolve acting area: mercenary sub-turns act at the mercenary target
    // area, not the president's location (mirrors execute_port_action).
    let area = if state.is_mercenary_action {
        state.mercenary_target_area.ok_or(MoveError::RuleViolation("No mercenary target area"))?
    } else {
        match state.player(player).president_location {
            PresidentLocation::OnBoard(a) => a,
            _ => return Err(MoveError::RuleViolation("President not on board")),
        }
    };

    // Resolve which Lab board (A or B) that area refers to.
    let lab_is_b = match state.board_layout.areas.get(&area).map(|c| c.area_type) {
        Some(AreaType::LaboratoryA) => false,
        Some(AreaType::LaboratoryB) => true,
        _ => return Err(MoveError::RuleViolation("Laboratory action requires a Laboratory area")),
    };
    if lab_is_b && state.lab_state.lab_b.is_none() {
        return Err(MoveError::RuleViolation("Lab B not available"));
    }

    // Find the card on the acting board ONLY — cross-board buying is illegal.
    let lab_ref = if lab_is_b {
        state.lab_state.lab_b.as_ref().unwrap() // presence confirmed above
    } else {
        &state.lab_state.lab_a
    };
    let slot_index = lab_ref.slots.iter().position(|s| *s == Some(card_id))
        .ok_or(MoveError::CardNotFound)?;

    // Validate against the resolved board. The returned (yen_cost,
    // imported_cost) is the ONLY amount deducted below (Phase 2.9 Stage 4,
    // census gap 1): the executor no longer re-derives costs from the raw
    // payload, so a validate/execute pricing mismatch is structurally
    // impossible.
    let (yen_cost, imported_cost) = validate_lab_action(
        state, player, lab_ref, slot_index, extra_yen, extra_imported,
        extra_tech_payment, power, tech_registry,
    ).map_err(MoveError::RuleViolation)?;

    // Take the card, slide the remainder left, and refill from the
    // state-owned deck — one borrow of `lab` so refill_lab (which borrows
    // the disjoint state.tech_deck field) doesn't need the board resolved
    // a second time.
    let lab = if lab_is_b {
        state.lab_state.lab_b.as_mut().unwrap()
    } else {
        &mut state.lab_state.lab_a
    };
    let _ = remove_and_slide_lab(lab, slot_index);
    refill_lab(lab, &mut state.tech_deck);

    // Pay costs, grant the tech.
    let p = state.player_mut(player);
    p.money -= yen_cost;
    p.imported_goods -= imported_cost;
    p.technologies.push(card_id);

    Ok(())
}

/// Post-purchase acquisition bonuses (called after `execute_lab_action`
/// succeeds — purchase completes, then bonuses). Patent System (+2 VP) and
/// Letter Press Printing (+1 Yen) stack per OTHER copy the buyer already
/// owns — the just-acquired physical card never counts towards its own
/// bonus. Exposition is a one-shot +10 VP self-trigger when the ACQUIRED
/// card itself carries that effect. Titles are identified via `TechEffect`,
/// not name strings.
///
/// Station's token placement on acquisition is handled one level up: the
/// caller's `apply_move.rs` Laboratory arm queues `PendingChoice::
/// StationPlace` (design §4.2) immediately after this function returns,
/// using the same acquired `card_id` -- `station_location` is intentionally
/// left untouched by this function.
///
/// Finishes by re-running the same country-icon mercenary-eligibility check
/// order fulfillment uses, for the acquired card's country.
pub fn apply_lab_acquisition_bonuses(
    state: &mut GameState,
    player: PlayerId,
    acquired: TechCardId,
    tech_registry: &TechRegistry,
    order_registry: &OrderRegistry,
) {
    let Some(acquired_card) = tech_registry.get(acquired) else { return; };
    let country = acquired_card.country;
    let acquired_is_exposition = acquired_card.effect == TechEffect::Exposition;

    let p = state.player(player);
    let other_patent_system = p.technologies.iter()
        .filter(|&&id| id != acquired)
        .filter(|&&id| tech_registry.get(id).map(|c| c.effect == TechEffect::PatentSystem).unwrap_or(false))
        .count() as u32;
    let other_letter_press = p.technologies.iter()
        .filter(|&&id| id != acquired)
        .filter(|&&id| tech_registry.get(id).map(|c| c.effect == TechEffect::LetterPressPrinting).unwrap_or(false))
        .count() as u32;

    let p = state.player_mut(player);
    p.victory_points += other_patent_system * 2;
    p.money += other_letter_press;
    if acquired_is_exposition {
        p.victory_points += 10;
    }

    check_mercenary_eligibility(state, player, country, order_registry, tech_registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use crate::game::{apply_move_unchecked, Rules};
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

    /// Builds a 6-slot PortBoard with the given card ids seeded left to
    /// right (index 0..5), for tests that need known cards at known
    /// positions — and thus known zones (0,1=Low; 2,3=Mid; 4,5=High).
    fn seeded_port(ids: [u8; 6]) -> PortBoard {
        PortBoard { slots: ids.map(|id| Some(OrderCardId(id))) }
    }

    #[test]
    fn test_port_zone_of_maps_six_slots_two_per_zone() {
        // Board art (asset-sweep-report.md Q3): LOW,LOW,MID,MID,HIGH,HIGH.
        assert_eq!(PortBoard::zone_of(0), PortZone::Low);
        assert_eq!(PortBoard::zone_of(1), PortZone::Low);
        assert_eq!(PortBoard::zone_of(2), PortZone::Mid);
        assert_eq!(PortBoard::zone_of(3), PortZone::Mid);
        assert_eq!(PortBoard::zone_of(4), PortZone::High);
        assert_eq!(PortBoard::zone_of(5), PortZone::High);
    }

    #[test]
    fn test_port_accessible_zones() {
        assert!(port_accessible_zones(1).is_empty());
        assert_eq!(port_accessible_zones(2), vec![PortZone::Low]);
        assert_eq!(port_accessible_zones(3), vec![PortZone::Low, PortZone::Mid]);
        assert_eq!(
            port_accessible_zones(4),
            vec![PortZone::Low, PortZone::Mid, PortZone::High]
        );
    }

    /// The ALLOCATION-FREE core, stated as the three printed gates (manual
    /// p.13): LOW opens at power 2, MID at 3, HIGH at 4. V4 spec §13 registers
    /// this refactor, and §7 names it as X2's rules-arithmetic comparator.
    ///
    /// The sweep runs 0..=6 — past the printed range on both ends, since power
    /// 0 and 1 open nothing and the engine caps at 5.
    #[test]
    fn port_zone_accessible_is_the_three_printed_power_gates() {
        for power in 0..=6u8 {
            assert_eq!(
                port_zone_accessible(power, PortZone::Low),
                power >= 2,
                "LOW at power {power}"
            );
            assert_eq!(
                port_zone_accessible(power, PortZone::Mid),
                power >= 3,
                "MID at power {power}"
            );
            assert_eq!(
                port_zone_accessible(power, PortZone::High),
                power >= 4,
                "HIGH at power {power}"
            );
        }
    }

    /// BEHAVIOUR IDENTITY: the allocating list is exactly the core's `true`
    /// zones, in the printed Low/Mid/High order, at every power.
    ///
    /// This is what makes the refactor single-sourcing rather than a second
    /// derivation — `validate_port_action` and `legal_moves` keep calling the
    /// list, and the list now calls the core, so the two can no longer drift.
    #[test]
    fn the_accessible_zone_list_is_exactly_the_cores_true_zones() {
        for power in 0..=6u8 {
            let expected: Vec<PortZone> = [PortZone::Low, PortZone::Mid, PortZone::High]
                .into_iter()
                .filter(|zone| port_zone_accessible(power, *zone))
                .collect();
            assert_eq!(port_accessible_zones(power), expected, "power {power}");
        }
    }

    #[test]
    fn test_port_validate_zone_accessibility_by_power() {
        let state = create_test_state();
        let player = PlayerId::new(0);
        // slots: Low(1), Low(2), Mid(3), Mid(4), High(5), High(6)
        let port = seeded_port([1, 2, 3, 4, 5, 6]);

        // Power 2: only Low cards accessible
        assert!(validate_port_action(&state, player, &port, &[OrderCardId(1)], None, 2).is_ok());
        assert!(validate_port_action(&state, player, &port, &[OrderCardId(3)], None, 2).is_err());
        assert!(validate_port_action(&state, player, &port, &[OrderCardId(5)], None, 2).is_err());

        // Power 3: Low + Mid accessible, High still not
        assert!(validate_port_action(&state, player, &port, &[OrderCardId(3)], None, 3).is_ok());
        assert!(validate_port_action(&state, player, &port, &[OrderCardId(5)], None, 3).is_err());

        // Power 4: every zone accessible
        assert!(validate_port_action(&state, player, &port, &[OrderCardId(5)], None, 4).is_ok());
    }

    #[test]
    fn test_port_validate_rejects_card_not_on_board() {
        let state = create_test_state();
        let player = PlayerId::new(0);
        let port = seeded_port([1, 2, 3, 4, 5, 6]);

        let result = validate_port_action(&state, player, &port, &[OrderCardId(99)], None, 4);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_port_validate_rejects_empty_take() {
        // Task 16 ruling: Skip is now the only legal way to decline a Port
        // take; an explicit empty `cards_taken` is no longer legal.
        let state = create_test_state();
        let player = PlayerId::new(0);
        let port = seeded_port([1, 2, 3, 4, 5, 6]);

        let result = validate_port_action(&state, player, &port, &[], None, 4);
        assert!(result.is_err());
    }

    #[test]
    fn test_port_validate_rejects_duplicate_card() {
        let state = create_test_state();
        let player = PlayerId::new(0);
        let port = seeded_port([1, 2, 3, 4, 5, 6]);

        let result = validate_port_action(
            &state,
            player,
            &port,
            &[OrderCardId(1), OrderCardId(1)],
            Some(PortPayment::TwoYen),
            4,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_port_hand_limit_validation() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        let port = seeded_port([1, 2, 3, 4, 5, 6]);

        // Player already has 2 cards - can take 1 more
        state.players[0].order_cards_in_hand = vec![OrderCardId(100), OrderCardId(101)];
        let result = validate_port_action(&state, player, &port, &[OrderCardId(1)], None, 3);
        assert!(result.is_ok());

        // Player already has 2 cards - cannot take 2 more (would exceed 3)
        let result = validate_port_action(
            &state,
            player,
            &port,
            &[OrderCardId(1), OrderCardId(3)],
            Some(PortPayment::TwoYen),
            3,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("hand limit"));

        // Player has 3 cards - cannot take any
        state.players[0].order_cards_in_hand =
            vec![OrderCardId(100), OrderCardId(101), OrderCardId(102)];
        let result = validate_port_action(&state, player, &port, &[OrderCardId(1)], None, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_port_second_card_payment_required() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        state.players[0].imported_goods = 2;
        let port = seeded_port([1, 2, 3, 4, 5, 6]);

        // Taking 2 cards requires payment
        let result = validate_port_action(
            &state,
            player,
            &port,
            &[OrderCardId(1), OrderCardId(3)],
            None, // No payment specified
            4,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Must pay"));

        // With 2 Yen payment - OK
        let result = validate_port_action(
            &state,
            player,
            &port,
            &[OrderCardId(1), OrderCardId(3)],
            Some(PortPayment::TwoYen),
            4,
        );
        assert!(result.is_ok());

        // With 1 Imported Good payment - OK
        let result = validate_port_action(
            &state,
            player,
            &port,
            &[OrderCardId(1), OrderCardId(3)],
            Some(PortPayment::OneImportedGood),
            4,
        );
        assert!(result.is_ok());

        // Not enough money for 2 Yen payment
        state.players[0].money = 1;
        let result = validate_port_action(
            &state,
            player,
            &port,
            &[OrderCardId(1), OrderCardId(3)],
            Some(PortPayment::TwoYen),
            4,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Not enough money"));
    }

    /// Places a PortA area on the board and puts the president there, as
    /// `execute_port_action` resolves the acting Port board from the
    /// area (rather than taking a `port_is_b` flag directly).
    fn place_president_at_port_a(state: &mut GameState) -> AreaId {
        let port_area = AreaId::new(0);
        state.board_layout.areas.insert(port_area, AreaConfig {
            area_type: AreaType::PortA,
            grid_position: (0, 0),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(port_area);
        port_area
    }

    #[test]
    fn test_port_execute_power_5_vp_bonus() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        place_president_at_port_a(&mut state);

        state.port_state.port_a = seeded_port([10, 11, 12, 13, 14, 15]);
        state.order_deck = vec![OrderCardId(20), OrderCardId(21)];

        let initial_vp = state.players[0].victory_points;

        // Execute at power 5 - should get +3 VP
        let result = execute_port_action(&mut state, player, &[OrderCardId(10)], None, 5);
        assert!(result.is_ok());
        assert_eq!(state.players[0].victory_points, initial_vp + 3);
    }

    #[test]
    fn test_port_execute_no_bonus_below_power_5() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        place_president_at_port_a(&mut state);

        state.port_state.port_a = seeded_port([10, 11, 12, 13, 14, 15]);
        state.order_deck = vec![OrderCardId(20)];

        let initial_vp = state.players[0].victory_points;

        // Execute at power 4 - should NOT get VP bonus
        let result = execute_port_action(&mut state, player, &[OrderCardId(10)], None, 4);
        assert!(result.is_ok());
        assert_eq!(state.players[0].victory_points, initial_vp);
    }

    // Pin task (Task 8, 2026-07-18): owner-adjudicated at the physical box
    // that the Port Power-5 +3 VP bonus is TAKE-GATED — it fires once per
    // executed take (never per card), a full hand bars the action entirely,
    // and Skip forfeits it (design §5.3-1). The four tests below pin that
    // ruling against current engine behavior; all are expected GREEN.

    #[test]
    fn port_take_at_five_grants_plus_3vp_exactly_once() {
        // owner-adjudicated 2026-07-18: the +3 VP is a flat per-action bonus
        // for acting at Power 5, not per-card — taking 2 cards must still
        // grant exactly +3, never +6 (design §5.3-1).
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        place_president_at_port_a(&mut state);

        state.port_state.port_a = seeded_port([10, 11, 12, 13, 14, 15]);
        state.order_deck = vec![OrderCardId(20), OrderCardId(21)];

        let initial_vp = state.players[0].victory_points;

        // Take 2 cards (Low + High, both accessible at power 5), paying 2
        // Yen for the second.
        let result = execute_port_action(
            &mut state,
            player,
            &[OrderCardId(10), OrderCardId(14)],
            Some(PortPayment::TwoYen),
            5,
        );
        assert!(result.is_ok());
        assert_eq!(state.players[0].victory_points, initial_vp + 3);
    }

    #[test]
    fn port_at_four_power_take_grants_no_bonus_vp() {
        // owner-adjudicated 2026-07-18: the bonus requires Power 5 exactly —
        // a Power 4 take grants nothing extra even when it reaches the High
        // zone, which power 4 already unlocks (design §5.3-1).
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        place_president_at_port_a(&mut state);

        state.port_state.port_a = seeded_port([10, 11, 12, 13, 14, 15]);
        state.order_deck = vec![OrderCardId(20)];

        let initial_vp = state.players[0].victory_points;

        // OrderCardId(14) sits in the High zone (index 4), reachable at
        // power 4 -- confirms the gate is on power, not on zone reach.
        let result = execute_port_action(&mut state, player, &[OrderCardId(14)], None, 4);
        assert!(result.is_ok());
        assert_eq!(state.players[0].victory_points, initial_vp);
    }

    #[test]
    fn test_port_execute_takes_named_cards_to_hand() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        place_president_at_port_a(&mut state);

        state.port_state.port_a = seeded_port([10, 11, 12, 13, 14, 15]);
        state.order_deck = vec![OrderCardId(20), OrderCardId(21)];

        let result = execute_port_action(
            &mut state,
            player,
            &[OrderCardId(10), OrderCardId(12)], // Low(10) + Mid(12)
            Some(PortPayment::TwoYen),
            4,
        );
        assert!(result.is_ok());

        // Check cards are in hand
        assert!(state.players[0].order_cards_in_hand.contains(&OrderCardId(10)));
        assert!(state.players[0].order_cards_in_hand.contains(&OrderCardId(12)));

        // Check payment was deducted
        assert_eq!(state.players[0].money, 8); // 10 - 2
    }

    #[test]
    fn test_port_execute_take_one_card_compacts_all_six_and_refills_rightmost() {
        // Regression pin, generalized to 6 slots: taking a single named
        // card must fully compact the remaining 5 down to indices 0-4,
        // leaving only index 5 empty for refill. An earlier buggy 3-slot
        // implementation shifted adjacent slots in a fixed order rather
        // than doing full compaction, stranding cards for some patterns.
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        place_president_at_port_a(&mut state);

        state.port_state.port_a = seeded_port([1, 2, 3, 4, 5, 6]);
        state.order_deck = vec![OrderCardId(20)];

        // Card 1 sits at index 0 (zone Low).
        let result = execute_port_action(&mut state, player, &[OrderCardId(1)], None, 2);
        assert!(result.is_ok());

        assert_eq!(
            state.port_state.port_a.slots,
            [
                Some(OrderCardId(2)),
                Some(OrderCardId(3)),
                Some(OrderCardId(4)),
                Some(OrderCardId(5)),
                Some(OrderCardId(6)),
                Some(OrderCardId(20)),
            ]
        );
        assert!(state.order_deck.is_empty());
    }

    #[test]
    fn test_port_execute_take_middle_card_compacts_cards_right_of_it() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        place_president_at_port_a(&mut state);

        state.port_state.port_a = seeded_port([1, 2, 3, 4, 5, 6]);
        state.order_deck = vec![OrderCardId(20)];

        // Card 3 sits at index 2 (zone Mid) — needs power 3.
        let result = execute_port_action(&mut state, player, &[OrderCardId(3)], None, 3);
        assert!(result.is_ok());

        // Cards left of the taken slot are untouched; everything right of
        // it slides down one, and the vacated rightmost slot refills.
        assert_eq!(
            state.port_state.port_a.slots,
            [
                Some(OrderCardId(1)),
                Some(OrderCardId(2)),
                Some(OrderCardId(4)),
                Some(OrderCardId(5)),
                Some(OrderCardId(6)),
                Some(OrderCardId(20)),
            ]
        );
        assert!(state.order_deck.is_empty());
    }

    #[test]
    fn test_port_execute_take_two_noncontiguous_cards_compacts_and_refills_both_gaps() {
        // Non-contiguous take (index 0 and index 4): the 4 surviving cards
        // compact to indices 0-3, and the 2 vacated slots (indices 4, 5)
        // refill from a 2-card deck, rightmost first.
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        place_president_at_port_a(&mut state);

        state.port_state.port_a = seeded_port([1, 2, 3, 4, 5, 6]);
        state.order_deck = vec![OrderCardId(20), OrderCardId(21)]; // top = 21

        // Card 1 (index 0, Low) + card 5 (index 4, High) — needs power 4.
        let result = execute_port_action(
            &mut state,
            player,
            &[OrderCardId(1), OrderCardId(5)],
            Some(PortPayment::TwoYen),
            4,
        );
        assert!(result.is_ok());

        assert_eq!(
            state.port_state.port_a.slots,
            [
                Some(OrderCardId(2)),
                Some(OrderCardId(3)),
                Some(OrderCardId(4)),
                Some(OrderCardId(6)),
                Some(OrderCardId(20)),
                Some(OrderCardId(21)),
            ]
        );
        assert!(state.order_deck.is_empty());
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
    fn test_extra_tech_surcharge_threshold() {
        assert!(!extra_tech_surcharge_required(0));
        assert!(!extra_tech_surcharge_required(2)); // 3rd card: free
        assert!(extra_tech_surcharge_required(3)); // 4th card pays (p.14, D2)
        assert!(extra_tech_surcharge_required(4)); // 5th card
        assert!(extra_tech_surcharge_required(5)); // 6th card
    }

    #[test]
    fn the_fourth_tech_card_pays_the_extra_tech_surcharge() {
        // D2 owner ruling 2026-08-08, rulebook p.14 verbatim: "Starting with
        // your 4th card, each technology card requires an additional cost of
        // 2 yen or 1 import." Owning 3 means the NEXT acquisition is the 4th
        // card, so the surcharge is due; the engine started it at the 5th
        // (the KB's "after your 4th" was a misreading).
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        state.players[0].imported_goods = 5;
        state.players[0].technologies = vec![TechCardId(10), TechCardId(11), TechCardId(12)];

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(TechCard {
            id: TechCardId(1),
            name: "Test Tech".to_string(),
            country: Country::English,
            industry_cost: 2,
            effect: TechEffect::Tram,
        }, "tech_test");
        state.lab_state.lab_a.slots[0] = Some(TechCardId(1));

        // The 4th card with no surcharge payment supplied must be rejected.
        let result = validate_lab_action(
            &state, player, &state.lab_state.lab_a, 0, 0, 0, None, 3, &tech_registry,
        );
        assert!(result.is_err(), "the 4th card owes the surcharge (p.14)");

        // Paying 2 Yen prices the whole purchase at exactly 2 Yen extra.
        let result = validate_lab_action(
            &state, player, &state.lab_state.lab_a, 0, 0, 0, Some(true), 3, &tech_registry,
        );
        assert_eq!(result, Ok((2, 0)));

        // Paying 1 import prices 1 imported good.
        let result = validate_lab_action(
            &state, player, &state.lab_state.lab_a, 0, 0, 0, Some(false), 3, &tech_registry,
        );
        assert_eq!(result, Ok((0, 1)));
    }

    #[test]
    fn the_third_tech_card_pays_no_surcharge() {
        // D2 companion: owning 2 means the next acquisition is the 3rd card —
        // below the p.14 threshold, so a supplied payment must be rejected
        // (census gap 1 discipline: never charge what is not due).
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        state.players[0].imported_goods = 5;
        state.players[0].technologies = vec![TechCardId(10), TechCardId(11)];

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(TechCard {
            id: TechCardId(1),
            name: "Test Tech".to_string(),
            country: Country::English,
            industry_cost: 2,
            effect: TechEffect::Tram,
        }, "tech_test");
        state.lab_state.lab_a.slots[0] = Some(TechCardId(1));

        assert_eq!(validate_lab_action(
            &state, player, &state.lab_state.lab_a, 0, 0, 0, None, 3, &tech_registry,
        ), Ok((0, 0)));
        assert!(validate_lab_action(
            &state, player, &state.lab_state.lab_a, 0, 0, 0, Some(true), 3, &tech_registry,
        ).is_err(), "no surcharge is due on the 3rd card, so a supplied payment is rejected");
    }

    #[test]
    fn test_lab_position_surcharge() {
        assert_eq!(lab_position_surcharge(0), 0);
        assert_eq!(lab_position_surcharge(1), 0);
        assert_eq!(lab_position_surcharge(2), 1);
        assert_eq!(lab_position_surcharge(3), 1);
        assert_eq!(lab_position_surcharge(4), 2);
        assert_eq!(lab_position_surcharge(5), 2);
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
            &state, player, &state.lab_state.lab_a, 0, 0, 0, None, 3, &tech_registry,
        );
        assert!(result.is_ok());

        // Put in slot 2 (surcharge +1) = needs 4 IV
        state.lab_state.lab_a.slots[2] = Some(TechCardId(1));
        let result = validate_lab_action(
            &state, player, &state.lab_state.lab_a, 2, 0, 0, None, 3, &tech_registry,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Insufficient industry value"));

        // Pay 2 extra Yen for +1 IV
        let result = validate_lab_action(
            &state, player, &state.lab_state.lab_a, 2, 2, 0, None, 3, &tech_registry,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_lab_extra_tech_surcharge() {
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

        // Without the surcharge payment - should fail
        let result = validate_lab_action(
            &state, player, &state.lab_state.lab_a, 0, 0, 0, None, 3, &tech_registry,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("extra-technology surcharge"));

        // With 2 Yen payment - should succeed
        let result = validate_lab_action(
            &state, player, &state.lab_state.lab_a, 0, 0, 0, Some(true), 3, &tech_registry,
        );
        assert!(result.is_ok());

        // With 1 Imported Good payment - should succeed
        let result = validate_lab_action(
            &state, player, &state.lab_state.lab_a, 0, 0, 0, Some(false), 3, &tech_registry,
        );
        assert!(result.is_ok());
    }

    /// Places a LaboratoryA area on the board and puts the president there,
    /// as `execute_lab_action` resolves the acting Lab board from the area
    /// (rather than taking a `lab_is_b` flag directly) — mirrors
    /// `place_president_at_port_a`.
    fn place_president_at_lab_a(state: &mut GameState) -> AreaId {
        let lab_area = AreaId::new(0);
        state.board_layout.areas.insert(lab_area, AreaConfig {
            area_type: AreaType::LaboratoryA,
            grid_position: (0, 0),
        });
        state.players[0].president_location = PresidentLocation::OnBoard(lab_area);
        lab_area
    }

    fn make_test_tech_card(id: u8, cost: u8, effect: TechEffect) -> TechCard {
        TechCard {
            id: TechCardId(id),
            name: "Test Tech".to_string(),
            country: Country::English,
            industry_cost: cost,
            effect,
        }
    }

    #[test]
    fn test_lab_execute_acquires_tech() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        state.players[0].imported_goods = 5;
        place_president_at_lab_a(&mut state);

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(make_test_tech_card(1, 2, TechEffect::Tram), "tech_test");

        state.lab_state.lab_a.slots[0] = Some(TechCardId(1));
        state.tech_deck = vec![TechCardId(20)];

        let result = execute_lab_action(
            &mut state,
            player,
            TechCardId(1),
            0,    // extra yen
            0,    // extra imported
            None, // extra tech payment
            3,    // power
            &tech_registry,
        );
        assert!(result.is_ok());

        // Check tech is in player's collection
        assert!(state.players[0].technologies.contains(&TechCardId(1)));
        // Refilled from state.tech_deck: only slot 0 was occupied, so taking
        // it leaves the whole board empty, and refill picks the rightmost
        // empty slot (index 5).
        assert_eq!(state.lab_state.lab_a.slots[5], Some(TechCardId(20)));
        assert!(state.tech_deck.is_empty());
    }

    /// Same fixture shape as `test_lab_execute_acquires_tech` above (money,
    /// imported goods, president at Lab A, a minimal custom tech registry,
    /// lab slot 0 holding the purchased card) but driven through
    /// `apply_move_unchecked` rather than calling `execute_lab_action`
    /// directly: the Station-queueing hook (design §4.2) lives one level up,
    /// in `apply_move.rs`'s `Laboratory` `AreaAction` arm, AFTER
    /// `execute_lab_action` + `apply_lab_acquisition_bonuses` both run --
    /// `execute_lab_action` alone never touches `pending_choices`, so only a
    /// call that reaches the wrapping arm can observe the hook. A second
    /// placeable area is added (`place_president_at_lab_a` inserts only
    /// one) so the choice genuinely queues instead of `push_choice`'s
    /// singleton path silently placing it on the sole placeable area.
    #[test]
    fn acquiring_station_queues_a_station_place_choice() {
        let mut state = create_test_state();
        state.players[0].money = 10;
        state.players[0].imported_goods = 5;
        place_president_at_lab_a(&mut state);
        state.board_layout.areas.insert(
            AreaId::new(1),
            AreaConfig { area_type: AreaType::Bank, grid_position: (0, 1) },
        );
        state.current_action_power = Some(4); // Station's industry_cost (data.rs:64)

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(make_test_tech_card(15, 4, TechEffect::Station), "tech_station");

        state.lab_state.lab_a.slots[0] = Some(TechCardId(15));
        state.tech_deck = vec![TechCardId(20)];

        let mut rules = Rules::empty_for_tests();
        rules.techs = tech_registry;

        let next = apply_move_unchecked(
            &state,
            &Move::ExecuteAreaAction {
                action: AreaAction::Laboratory {
                    card_id: TechCardId(15),
                    extra_yen: 0,
                    extra_imported: 0,
                    extra_tech_payment: None,
                },
            },
            &rules,
        )
        .unwrap();

        assert!(next.players[0].technologies.contains(&TechCardId(15)));
        assert_eq!(next.station_location, None, "token placement is a queued choice, not automatic");
        assert_eq!(next.pending_choices, vec![PendingChoice::StationPlace]);
    }

    #[test]
    fn test_lab_execute_rejects_card_on_other_board() {
        // Cross-board buying is illegal: the president acts at Lab A, but
        // the named card only exists on Lab B, so resolution must search
        // the ACTING board only and report CardNotFound (not "no card in
        // slot" or a silent lab_a/lab_b fallback).
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        place_president_at_lab_a(&mut state);
        state.lab_state.lab_b = Some(LabBoard::new());
        state.lab_state.lab_b.as_mut().unwrap().slots[0] = Some(TechCardId(1));

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(make_test_tech_card(1, 2, TechEffect::Tram), "tech_test");

        let result = execute_lab_action(
            &mut state, player, TechCardId(1), 0, 0, None, 3, &tech_registry,
        );
        assert!(matches!(result, Err(MoveError::CardNotFound)));
    }

    #[test]
    fn test_lab_execute_tolerates_empty_deck() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        place_president_at_lab_a(&mut state);

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(make_test_tech_card(1, 2, TechEffect::Tram), "tech_test");
        state.lab_state.lab_a.slots[0] = Some(TechCardId(1));
        // state.tech_deck stays empty (create_test_state() default).

        let result = execute_lab_action(
            &mut state, player, TechCardId(1), 0, 0, None, 3, &tech_registry,
        );
        assert!(result.is_ok());
        assert!(state.players[0].technologies.contains(&TechCardId(1)));
        assert!(state.lab_state.lab_a.slots.iter().all(|s| s.is_none()));
    }

    #[test]
    fn test_validate_lab_action_rejects_owning_same_title() {
        // Rulebook General Rules: "players may only own 1 card of the same
        // name." Two physical Language School copies circulate on boards
        // (verified YAML `copies: 2`), but the player may never hold both
        // (C2 final-review finding). Effects are 1:1 with titles in the
        // standard data, so effect equality is the ownership check.
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(make_test_tech_card(1, 3, TechEffect::LanguageSchool), "language_school_1");
        tech_registry.insert(make_test_tech_card(2, 3, TechEffect::LanguageSchool), "language_school_2");

        // Already owns Language School copy #1.
        state.players[0].technologies = vec![TechCardId(1)];
        state.lab_state.lab_a.slots[0] = Some(TechCardId(2));

        let result = validate_lab_action(
            &state, player, &state.lab_state.lab_a, 0, 0, 0, None, 5, &tech_registry,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_lab_action_allows_distinct_titles_even_with_shared_effect_slot() {
        // Owning an UNRELATED tech must never block a purchase.
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(make_test_tech_card(1, 3, TechEffect::LanguageSchool), "language_school_1");
        tech_registry.insert(make_test_tech_card(9, 3, TechEffect::Tram), "tram_1");

        state.players[0].technologies = vec![TechCardId(9)]; // owns Tram, not Language School
        state.lab_state.lab_a.slots[0] = Some(TechCardId(1));

        let result = validate_lab_action(
            &state, player, &state.lab_state.lab_a, 0, 0, 0, None, 5, &tech_registry,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_lab_execute_rejects_owning_same_title() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        place_president_at_lab_a(&mut state);

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(make_test_tech_card(1, 3, TechEffect::LanguageSchool), "language_school_1");
        tech_registry.insert(make_test_tech_card(2, 3, TechEffect::LanguageSchool), "language_school_2");

        state.players[0].technologies = vec![TechCardId(1)];
        state.lab_state.lab_a.slots[0] = Some(TechCardId(2));

        let result = execute_lab_action(
            &mut state, player, TechCardId(2), 0, 0, None, 5, &tech_registry,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))));
        // Nothing acquired or paid on rejection.
        assert_eq!(state.players[0].technologies, vec![TechCardId(1)]);
        assert_eq!(state.players[0].money, 10);
    }

    /// Registry with two Patent System copies (ids 1,2), one Letter Press
    /// Printing copy (id 3), one Exposition (id 4), and an unrelated tech
    /// (id 5) — used to pin acquisition-bonus semantics generically,
    /// independent of the real standard registry's exact copy counts.
    fn make_bonus_tech_registry() -> TechRegistry {
        let mut r = TechRegistry::new();
        r.insert(make_test_tech_card(1, 3, TechEffect::PatentSystem), "patent_1");
        r.insert(make_test_tech_card(2, 3, TechEffect::PatentSystem), "patent_2");
        r.insert(make_test_tech_card(3, 3, TechEffect::LetterPressPrinting), "letterpress_1");
        r.insert(make_test_tech_card(4, 5, TechEffect::Exposition), "exposition_1");
        r.insert(make_test_tech_card(5, 5, TechEffect::Tram), "tram_1");
        r
    }

    // --- Phase 2.9 Stage 4: raw-arm self-defense (census gaps 1-2) ---

    #[test]
    fn lab_extra_tech_yen_payment_without_penalty_due_is_rejected_no_underflow() {
        // Census gap 1 probe: 0 techs owned (penalty NOT required), money 0,
        // cost fully covered by power. Supplying Some(true) slipped past
        // validation (which only inspects extra_tech_payment when the
        // penalty is due) and the executor then charged the unvalidated
        // +2 Yen: debug underflow panic, release wrap to ~4.29e9 Yen.
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 0;
        place_president_at_lab_a(&mut state);

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(make_test_tech_card(1, 2, TechEffect::Tram), "tech_test");
        state.lab_state.lab_a.slots[0] = Some(TechCardId(1));
        let before = state.clone();

        let result = execute_lab_action(
            &mut state, player, TechCardId(1), 0, 0, Some(true), 3, &tech_registry,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
        assert_eq!(state, before, "zero mutation on Err");
    }

    #[test]
    fn lab_extra_tech_imported_payment_without_penalty_due_is_rejected() {
        // Some(false) sibling of the probe above: with imported_goods >= 1
        // the old validation ACCEPTED the spurious penalty payment and the
        // executor charged 1 imported good for a penalty that was not due.
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 5;
        state.players[0].imported_goods = 1;
        place_president_at_lab_a(&mut state);

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(make_test_tech_card(1, 2, TechEffect::Tram), "tech_test");
        state.lab_state.lab_a.slots[0] = Some(TechCardId(1));
        let before = state.clone();

        let result = execute_lab_action(
            &mut state, player, TechCardId(1), 0, 0, Some(false), 3, &tech_registry,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
        assert_eq!(state, before, "zero mutation on Err");
    }

    #[test]
    fn lab_overpay_beyond_deficit_is_rejected_exact_covers_only() {
        // Census static note, adjudicated against the enumeration
        // convention (stage-1 oracle spec §7.2-3: the generator emits exact
        // covers only; over-cover adds nothing at the table): power 5 fully
        // covers cost 2, so ANY extra payment over-covers -- previously
        // accepted, silently charging 2 Yen for zero additional Industry.
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        place_president_at_lab_a(&mut state);

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(make_test_tech_card(1, 2, TechEffect::Tram), "tech_test");
        state.lab_state.lab_a.slots[0] = Some(TechCardId(1));
        let before = state.clone();

        let result = execute_lab_action(
            &mut state, player, TechCardId(1), 2, 0, None, 5, &tech_registry,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
        assert_eq!(state, before, "zero mutation on Err");
    }

    #[test]
    fn lab_odd_extra_yen_is_rejected_exact_covers_only() {
        // extra_yen = 3 covers a deficit of 1 (3/2 = 1 Industry) but wastes
        // 1 Yen -- not of the generator's 2*(deficit-k) form (§7.2-3), so
        // the raw arm rejects it rather than silently charging the odd Yen.
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        state.players[0].money = 10;
        place_president_at_lab_a(&mut state);

        let mut tech_registry = TechRegistry::new();
        tech_registry.insert(make_test_tech_card(1, 4, TechEffect::Tram), "tech_test");
        state.lab_state.lab_a.slots[0] = Some(TechCardId(1));
        let before = state.clone();

        // Power 3 vs required 4 -> deficit 1; extra_yen 3 is odd.
        let result = execute_lab_action(
            &mut state, player, TechCardId(1), 3, 0, None, 3, &tech_registry,
        );
        assert!(matches!(result, Err(MoveError::RuleViolation(_))), "{result:?}");
        assert_eq!(state, before, "zero mutation on Err");
    }

    #[test]
    fn calculate_industry_value_sums_without_u8_overflow() {
        // Census gap 2 probe: extra_yen/extra_imported near u8::MAX
        // overflowed the old u8 sum in debug builds. The sum is u32 now.
        assert_eq!(u64::from(calculate_industry_value(5, 255, 255)), 5 + 127 + 255);
    }

    #[test]
    fn test_lab_acquisition_bonus_patent_system_stacks_per_owned_copy() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        let tech_registry = make_bonus_tech_registry();
        let order_registry = OrderRegistry::new();
        // Owns BOTH Patent System copies already; just acquired an unrelated tech.
        state.players[0].technologies = vec![TechCardId(1), TechCardId(2), TechCardId(5)];

        apply_lab_acquisition_bonuses(&mut state, player, TechCardId(5), &tech_registry, &order_registry);

        assert_eq!(state.players[0].victory_points, 4); // 2 owned copies x 2 VP
    }

    /// RULED (Jeremy, 2026-07-20, closing ruling A4): rulebook p.17 —
    /// "Whenever you obtain a technology (after this one), score 2
    /// additional points." Self-EXCLUSIVE: acquiring Patent System itself
    /// grants nothing; a player's next tech after it scores +2. Confirms
    /// the shipped filter (`id != acquired`).
    #[test]
    fn test_lab_acquisition_bonus_excludes_just_acquired_card() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        let tech_registry = make_bonus_tech_registry();
        let order_registry = OrderRegistry::new();
        // The ONLY Patent System copy owned is the one just acquired: no bonus.
        state.players[0].technologies = vec![TechCardId(1)];

        apply_lab_acquisition_bonuses(&mut state, player, TechCardId(1), &tech_registry, &order_registry);

        assert_eq!(state.players[0].victory_points, 0);
    }

    /// RULED (Jeremy, 2026-07-21, ruling A5): Letter Press Printing is
    /// self-EXCLUSIVE, exactly as Patent System is (ruling A4) -- rulebook
    /// p.16 (the PDF is 16 pages; Patent System / Letter Press are on p.16):
    /// "Whenever you obtain a technology (after this one), gain 1 Yen."
    /// Acquiring Letter Press itself grants no Yen; a SUBSEQUENT acquisition
    /// grants +1 Yen per owned copy. Mirrors
    /// `test_lab_acquisition_bonus_excludes_just_acquired_card` (the Patent
    /// System twin) and confirms the shipped `id != acquired` filter.
    #[test]
    fn test_lab_acquisition_letterpress_excludes_just_acquired_card() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        let tech_registry = make_bonus_tech_registry();
        let order_registry = OrderRegistry::new();

        // The ONLY Letter Press copy owned is the one just acquired: no Yen.
        state.players[0].technologies = vec![TechCardId(3)];
        let money_before = state.players[0].money;
        apply_lab_acquisition_bonuses(&mut state, player, TechCardId(3), &tech_registry, &order_registry);
        assert_eq!(
            state.players[0].money, money_before,
            "acquiring Letter Press itself grants no Yen (self-exclusive, ruling A5)"
        );

        // A subsequent acquisition (an unrelated tech) with Letter Press
        // already owned grants +1 Yen for that owned copy.
        let money_before = state.players[0].money;
        state.players[0].technologies = vec![TechCardId(3), TechCardId(5)];
        apply_lab_acquisition_bonuses(&mut state, player, TechCardId(5), &tech_registry, &order_registry);
        assert_eq!(
            state.players[0].money,
            money_before + 1,
            "a later acquisition scores +1 Yen per owned Letter Press copy"
        );
    }

    #[test]
    fn test_lab_acquisition_letterpress_bonus_grants_yen_per_owned_copy() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        let tech_registry = make_bonus_tech_registry();
        let order_registry = OrderRegistry::new();
        state.players[0].technologies = vec![TechCardId(3), TechCardId(5)];
        let starting_money = state.players[0].money;

        apply_lab_acquisition_bonuses(&mut state, player, TechCardId(5), &tech_registry, &order_registry);

        assert_eq!(state.players[0].money, starting_money + 1);
    }

    #[test]
    fn test_lab_acquisition_exposition_self_trigger() {
        let mut state = create_test_state();
        let player = PlayerId::new(0);
        let tech_registry = make_bonus_tech_registry();
        let order_registry = OrderRegistry::new();
        state.players[0].technologies = vec![TechCardId(4)]; // just-acquired Exposition itself

        apply_lab_acquisition_bonuses(&mut state, player, TechCardId(4), &tech_registry, &order_registry);

        assert_eq!(state.players[0].victory_points, 10);
    }
}
