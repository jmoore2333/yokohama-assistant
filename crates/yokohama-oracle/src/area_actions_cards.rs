//! §3.4 Port + Laboratory area actions (lane C, spec §3.4).
//!
//! Authored from the rules sources only — rulebook [RB p.11-12], the decision
//! log rows cited inline, and the shared `types/` vocabulary. No engine logic
//! file was consulted. `Skip` is NOT added here — the lib.rs area-action
//! facade appends the single canonical decline (`allows_area_action_skip`).

use yokohama_core::{
    AreaAction, AreaType, GameState, LabBoard, Move, PortBoard, PortPayment, Rules,
};

use crate::common::owns_effect;

/// Order-card hand limit: "you may never have more than 3 Order cards in your
/// hand" and orders can never be discarded, so a take that would exceed the
/// limit is illegal [RB p.5 General Rules; RB p.11 Note].
const ORDER_HAND_LIMIT: usize = 3;

/// §3.4 Port A/B: free 1-card takes and paid 2-card takes over eligible-by-power
/// slots, hand-limit gated. `area_type` selects Port A vs Port B state.
pub(crate) fn port_moves(
    state: &GameState,
    rules: &Rules,
    area_type: AreaType,
    power: u8,
) -> Vec<Move> {
    // Slot contents are card ids; the Port take needs no registry data.
    let _ = rules;
    let board: Option<&PortBoard> = match area_type {
        AreaType::PortA => Some(&state.port_state.port_a),
        AreaType::PortB => state.port_state.port_b.as_ref(),
        // Non-Port area type: nothing to enumerate (defensive; the facade
        // only dispatches Port types here).
        _ => None,
    };
    let Some(board) = board else {
        // Port B absent (2-player setup) — defensively empty.
        return Vec::new();
    };
    let p = state.current_player();

    // Slot eligibility by power [RB p.11]: power 1 may not take; power 2
    // reaches the Low zone (slots 0-1); power 3 Low+Mid (0-3); power 4-5 all
    // six (zones are positional Low,Low,Mid,Mid,High,High — six-slot layout
    // [DL 2026-07-17 six-slot row; `PortBoard::zone_of`]).
    let eligible_slots: usize = match power {
        0 | 1 => 0,
        2 => 2,
        3 => 4,
        _ => 6,
    };
    // Eligible occupied slots, ascending slot index (§7.2-2 normal form).
    let eligible: Vec<_> = board.slots[..eligible_slots]
        .iter()
        .enumerate()
        .filter_map(|(slot, cell)| cell.map(|card| (slot, card)))
        .collect();

    let hand = p.order_cards_in_hand.len();
    let mut moves = Vec::new();

    // Take 1 — free, one move per eligible occupied slot [RB p.11], iff the
    // hand limit admits one more card (hand + 1 <= limit).
    if hand < ORDER_HAND_LIMIT {
        for &(_, card) in &eligible {
            moves.push(Move::ExecuteAreaAction {
                action: AreaAction::Port {
                    cards_taken: vec![card],
                    second_card_payment: None,
                },
            });
        }
    }

    // Take 2 — the second card costs 2 Yen OR 1 Imported Good [RB p.11]; each
    // payment form is a separate move, enumerated iff separately affordable.
    // Unordered pairs of distinct eligible cards, `cards_taken` in ascending
    // slot order (§7.2-2 normal form); iff the hand limit admits two more.
    if hand + 2 <= ORDER_HAND_LIMIT {
        for a in 0..eligible.len() {
            for b in (a + 1)..eligible.len() {
                let cards_taken = vec![eligible[a].1, eligible[b].1];
                if p.money >= 2 {
                    moves.push(Move::ExecuteAreaAction {
                        action: AreaAction::Port {
                            cards_taken: cards_taken.clone(),
                            second_card_payment: Some(PortPayment::TwoYen),
                        },
                    });
                }
                if p.imported_goods >= 1 {
                    moves.push(Move::ExecuteAreaAction {
                        action: AreaAction::Port {
                            cards_taken,
                            second_card_payment: Some(PortPayment::OneImportedGood),
                        },
                    });
                }
            }
        }
    }

    // `Port { cards_taken: [] }` is never emitted — Skip is the sole canonical
    // decline [DL 2026-07-17 area-action optionality row]. The power-5 +3 VP
    // is take-gated and apply-side [RB p.11; DL 2026-07-18 BOX CHECK (b)].
    moves
}

/// §3.4 Laboratory A/B: tech acquisitions with the one-per-title gate, industry
/// cover enumeration, and the 5th-tech penalty variants. `area_type` selects
/// Lab A vs Lab B state.
pub(crate) fn laboratory_moves(
    state: &GameState,
    rules: &Rules,
    area_type: AreaType,
    power: u8,
) -> Vec<Move> {
    let board: Option<&LabBoard> = match area_type {
        AreaType::LaboratoryA => Some(&state.lab_state.lab_a),
        AreaType::LaboratoryB => state.lab_state.lab_b.as_ref(),
        _ => None,
    };
    let Some(board) = board else {
        // Lab B absent (2-3 player setup) — defensively empty.
        return Vec::new();
    };
    let p = state.current_player();

    // 5th-tech penalty [RB p.12 Note]: owning >=4 techs makes each further
    // acquisition cost an ADDITIONAL 2 Yen or 1 Imported Good — enumerate
    // Some(true) (2 Yen) and Some(false) (1 Imported) as separately affordable
    // variants; under 4 owned techs the only form is None (spec §3.4).
    let fifth_variants: &[Option<bool>] = if p.technologies.len() >= 4 {
        &[Some(true), Some(false)]
    } else {
        &[None]
    };

    let mut moves = Vec::new();
    for (slot, cell) in board.slots.iter().enumerate() {
        let Some(card_id) = *cell else { continue };
        // Registry lookup — data access only (spec §9-C). A card the registry
        // cannot price is skipped defensively.
        let Some(card) = rules.techs.get(card_id) else { continue };

        // One-per-title gate: "players may only own 1 card of the same name"
        // [RB p.5 General Rules; DL 2026-07-17 final-review C2]. Title is 1:1
        // with effect in the standard data (spec §3.4), so effect identity is
        // the gate; different players may each own one of a 2-copy title.
        if owns_effect(state, rules, state.current_player, card.effect) {
            continue;
        }

        // Required Industry = printed cost + position modifier 0,0,1,1,2,2
        // [RB p.12; DL 2026-07-17 six-slot row; `LabBoard::position_modifier`].
        let required = card.industry_cost + LabBoard::position_modifier(slot);
        // Deficit after power (max 5 Industry from power; excess is lost
        // [RB p.12]).
        let deficit = required.saturating_sub(power.min(5));

        // Exact deficit covers [RB p.12: 2 Yen = 1 Industry, 1 Imported Good =
        // 1 Industry]: extra_imported = i, extra_yen = 2*(deficit - i).
        // Over-covering payments are not enumerated (§7.2-3).
        let max_imported = u32::from(deficit).min(p.imported_goods) as u8;
        for extra_imported in 0..=max_imported {
            let extra_yen = 2 * (deficit - extra_imported);
            for &fifth in fifth_variants {
                // Full affordability arithmetic (spec §3.4): the surcharge
                // stacks on the cover's outlay.
                let yen_needed =
                    u32::from(extra_yen) + if fifth == Some(true) { 2 } else { 0 };
                let imported_needed =
                    u32::from(extra_imported) + if fifth == Some(false) { 1 } else { 0 };
                if p.money >= yen_needed && p.imported_goods >= imported_needed {
                    moves.push(Move::ExecuteAreaAction {
                        action: AreaAction::Laboratory {
                            card_id,
                            extra_yen,
                            extra_imported,
                            fifth_tech_payment: fifth,
                        },
                    });
                }
            }
        }
        // Station's queued placement, Exposition's instant VP, and the
        // Patent/Letterpress riders are apply-side (spec §3.4).
    }
    moves
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::two_player_state;
    use yokohama_core::{
        AreaAction, GameState, OrderCardId, PortBoard, PortPayment, TechCard, TechCardId,
        TechEffect,
    };

    /// Unwrap every emitted move as an `AreaAction` (the module must emit only
    /// `ExecuteAreaAction` moves, and never `Skip` — the facade adds Skip).
    fn actions(moves: &[Move]) -> Vec<AreaAction> {
        moves
            .iter()
            .map(|m| match m {
                Move::ExecuteAreaAction { action } => {
                    assert_ne!(*action, AreaAction::Skip, "Skip is the facade's job (spec §3.4)");
                    action.clone()
                }
                other => panic!("non-area-action move emitted: {other:?}"),
            })
            .collect()
    }

    // ---------- Port fixtures ----------

    fn take1(card: u8) -> AreaAction {
        AreaAction::Port {
            cards_taken: vec![OrderCardId(card)],
            second_card_payment: None,
        }
    }

    fn take2(a: u8, b: u8, pay: PortPayment) -> AreaAction {
        AreaAction::Port {
            cards_taken: vec![OrderCardId(a), OrderCardId(b)],
            second_card_payment: Some(pay),
        }
    }

    /// Port A fully stocked with cards 10..=15 in slots 0..=5; mover holds one
    /// order (hand 1), 2 Yen, no imported goods.
    fn six_card_port_state() -> GameState {
        let mut s = two_player_state();
        s.port_state.port_a.slots =
            [10u8, 11, 12, 13, 14, 15].map(|n| Some(OrderCardId(n)));
        s.players[0].order_cards_in_hand = vec![OrderCardId(1)];
        s.players[0].money = 2;
        s.players[0].imported_goods = 0;
        s
    }

    fn assert_same_set(got: &[AreaAction], want: &[AreaAction]) {
        for w in want {
            assert!(got.contains(w), "missing expected action: {w:?}\ngot: {got:#?}");
        }
        assert_eq!(got.len(), want.len(), "extra actions emitted\ngot: {got:#?}");
    }

    #[test]
    fn port_power_1_yields_no_takes() {
        // [RB p.11] power 1: "may not take" — no substantive moves (Skip only,
        // added by the facade). Spec §6.2 area_actions_cards row.
        let s = six_card_port_state();
        let r = Rules::empty_for_tests();
        assert!(port_moves(&s, &r, AreaType::PortA, 1).is_empty());
        assert!(port_moves(&s, &r, AreaType::PortA, 0).is_empty());
    }

    #[test]
    fn port_power_2_low_zone_only() {
        // [RB p.11] power 2 reaches the Low zone: slots 0-1 only.
        let s = six_card_port_state();
        let got = actions(&port_moves(&s, &Rules::empty_for_tests(), AreaType::PortA, 2));
        assert_same_set(
            &got,
            &[take1(10), take1(11), take2(10, 11, PortPayment::TwoYen)],
        );
    }

    #[test]
    fn port_power_3_low_mid_pairs_two_yen_f7() {
        // F7 [RB p.12 Hisashi port example]: power 3, hand 1, money 2, no
        // imported goods — all Low/Mid takes; no pair touches High slots.
        let s = six_card_port_state();
        let got = actions(&port_moves(&s, &Rules::empty_for_tests(), AreaType::PortA, 3));
        let mut want = vec![take1(10), take1(11), take1(12), take1(13)];
        let low_mid = [10u8, 11, 12, 13];
        for a in 0..low_mid.len() {
            for b in (a + 1)..low_mid.len() {
                want.push(take2(low_mid[a], low_mid[b], PortPayment::TwoYen));
            }
        }
        assert_same_set(&got, &want);
    }

    #[test]
    fn port_imported_good_pays_second_card_f7() {
        // F7 continued: with an imported good the OneImportedGood twin appears
        // for every pair [RB p.11 "2 Yen or 1 Imported Good"].
        let mut s = six_card_port_state();
        s.players[0].imported_goods = 1;
        let got = actions(&port_moves(&s, &Rules::empty_for_tests(), AreaType::PortA, 3));
        // 4 take-1 + 6 pairs x 2 payment variants.
        assert_eq!(got.len(), 16, "got: {got:#?}");
        assert!(got.contains(&take2(10, 12, PortPayment::OneImportedGood)));
        assert!(got.contains(&take2(10, 12, PortPayment::TwoYen)));
    }

    #[test]
    fn port_payment_variants_gate_separately() {
        // [RB p.11] each payment form is enumerated iff separately affordable.
        let mut s = six_card_port_state();
        s.players[0].money = 1;
        s.players[0].imported_goods = 1;
        let got = actions(&port_moves(&s, &Rules::empty_for_tests(), AreaType::PortA, 2));
        assert_same_set(
            &got,
            &[take1(10), take1(11), take2(10, 11, PortPayment::OneImportedGood)],
        );

        s.players[0].imported_goods = 0;
        let got = actions(&port_moves(&s, &Rules::empty_for_tests(), AreaType::PortA, 2));
        assert_same_set(&got, &[take1(10), take1(11)]);
    }

    #[test]
    fn port_hand_limit_three_f7() {
        // F7: hand limit 3, orders can never be discarded [RB p.5, p.11 Note].
        let mut s = six_card_port_state();
        s.players[0].order_cards_in_hand = vec![OrderCardId(1), OrderCardId(2)];
        let got = actions(&port_moves(&s, &Rules::empty_for_tests(), AreaType::PortA, 3));
        // hand 2: 2-card takes vanish, 1-card takes remain.
        assert!(got.iter().all(
            |a| matches!(a, AreaAction::Port { cards_taken, .. } if cards_taken.len() == 1)
        ));
        assert_eq!(got.len(), 4);

        // hand 3: nothing substantive (facade will offer Skip alone).
        s.players[0].order_cards_in_hand.push(OrderCardId(3));
        assert!(port_moves(&s, &Rules::empty_for_tests(), AreaType::PortA, 3).is_empty());
    }

    #[test]
    fn port_cards_taken_ascending_slot_order() {
        // Spec §7.2-2 normal form: 2-card takes list cards in ascending slot
        // order; the reversed form is never emitted.
        let s = six_card_port_state();
        let got = actions(&port_moves(&s, &Rules::empty_for_tests(), AreaType::PortA, 3));
        assert!(got.contains(&take2(10, 13, PortPayment::TwoYen)));
        assert!(!got.contains(&take2(13, 10, PortPayment::TwoYen)));
    }

    #[test]
    fn port_power_5_reaches_all_six_slots() {
        // [RB p.11] power 4-5: all six slots eligible.
        let s = six_card_port_state();
        let got = actions(&port_moves(&s, &Rules::empty_for_tests(), AreaType::PortA, 5));
        // 6 take-1 + C(6,2)=15 pairs (TwoYen only: no imported goods).
        assert_eq!(got.len(), 21, "got: {got:#?}");
        assert!(got.contains(&take1(15)));
        assert!(got.contains(&take2(14, 15, PortPayment::TwoYen)));
    }

    #[test]
    fn port_skips_empty_slots() {
        // Only occupied slots yield takes (a take needs a card).
        let mut s = six_card_port_state();
        s.port_state.port_a.slots[1] = None;
        s.port_state.port_a.slots[3] = None;
        let got = actions(&port_moves(&s, &Rules::empty_for_tests(), AreaType::PortA, 3));
        assert_same_set(
            &got,
            &[take1(10), take1(12), take2(10, 12, PortPayment::TwoYen)],
        );
    }

    #[test]
    fn port_b_uses_port_b_board() {
        // `area_type` selects the board; a missing Port B (2P) is defensively empty.
        let mut s = six_card_port_state();
        let r = Rules::empty_for_tests();
        assert!(port_moves(&s, &r, AreaType::PortB, 5).is_empty());

        let mut b = PortBoard::new();
        b.slots[0] = Some(OrderCardId(30));
        s.port_state.port_b = Some(b);
        let got = actions(&port_moves(&s, &r, AreaType::PortB, 2));
        assert_same_set(&got, &[take1(30)]);
    }

    #[test]
    fn port_never_emits_empty_take() {
        // [DL 2026-07-17 optionality row]: `Port { [] }` is validator-illegal —
        // Skip (facade-added) is the sole canonical decline.
        let s = six_card_port_state();
        let got = actions(&port_moves(&s, &Rules::empty_for_tests(), AreaType::PortA, 5));
        assert!(got.iter().all(
            |a| !matches!(a, AreaAction::Port { cards_taken, .. } if cards_taken.is_empty())
        ));
    }

    // ---------- Laboratory fixtures ----------

    fn tech_by_effect(rules: &Rules, effect: TechEffect) -> TechCard {
        rules
            .techs
            .iter()
            .find(|t| t.effect == effect)
            .cloned()
            .unwrap_or_else(|| panic!("standard data carries {effect:?}"))
    }

    fn lab_action(card_id: TechCardId, yen: u8, imported: u8, fifth: Option<bool>) -> AreaAction {
        AreaAction::Laboratory {
            card_id,
            extra_yen: yen,
            extra_imported: imported,
            fifth_tech_payment: fifth,
        }
    }

    /// Empty-lab state for `Rules::standard()` fixtures.
    fn lab_state() -> GameState {
        let mut s = two_player_state();
        s.players[0].money = 0;
        s.players[0].imported_goods = 0;
        s
    }

    #[test]
    fn lab_f8_tram_yen_cover() {
        // F8 [RB p.12 Tram laboratory example]: Tram printed cost 5, slot
        // index 2 (+1 modifier) => required 6; power 5 => deficit 1; 2 Yen
        // buys the missing Industry [RB p.12 "2 Yen = 1 Industry"].
        let rules = Rules::standard();
        let tram = tech_by_effect(&rules, TechEffect::Tram);
        assert_eq!(tram.industry_cost, 5, "F8 premise: Tram printed cost 5");
        let mut s = lab_state();
        s.lab_state.lab_a.slots[2] = Some(tram.id);
        s.players[0].money = 2;
        let got = actions(&laboratory_moves(&s, &rules, AreaType::LaboratoryA, 5));
        assert_same_set(&got, &[lab_action(tram.id, 2, 0, None)]);
    }

    #[test]
    fn lab_f8_tram_imported_cover_when_yen_short() {
        // F8 continued: money 1, imported 1 => the imported cover is present
        // and the yen cover absent [RB p.12 "1 Imported Good = 1 Industry"].
        let rules = Rules::standard();
        let tram = tech_by_effect(&rules, TechEffect::Tram);
        let mut s = lab_state();
        s.lab_state.lab_a.slots[2] = Some(tram.id);
        s.players[0].money = 1;
        s.players[0].imported_goods = 1;
        let got = actions(&laboratory_moves(&s, &rules, AreaType::LaboratoryA, 5));
        assert_same_set(&got, &[lab_action(tram.id, 0, 1, None)]);
    }

    #[test]
    fn lab_enumerates_exact_covers_only() {
        // Spec §3.4: exact deficit covers (extra_imported=i, extra_yen=2(d-i));
        // over-covering payments are not enumerated (spec §7.2-3).
        // Telegram cost 6 at slot 2 (+1) => required 7; power 5 => d = 2.
        let rules = Rules::standard();
        let telegram = tech_by_effect(&rules, TechEffect::Telegram);
        assert_eq!(telegram.industry_cost, 6);
        let mut s = lab_state();
        s.lab_state.lab_a.slots[2] = Some(telegram.id);
        s.players[0].money = 10;
        s.players[0].imported_goods = 2;
        let got = actions(&laboratory_moves(&s, &rules, AreaType::LaboratoryA, 5));
        assert_same_set(
            &got,
            &[
                lab_action(telegram.id, 4, 0, None),
                lab_action(telegram.id, 2, 1, None),
                lab_action(telegram.id, 0, 2, None),
            ],
        );
    }

    #[test]
    fn lab_one_per_title_gate_blocks_second_copy() {
        // [RB p.5 General Rules; DL 2026-07-17 C2]: owning one copy of a
        // 2-copy title blocks acquiring the other physical copy (identity is
        // the title, 1:1 with effect in the standard data).
        let rules = Rules::standard();
        let copies: Vec<TechCardId> = rules
            .techs
            .iter()
            .filter(|t| t.effect == TechEffect::Ball)
            .map(|t| t.id)
            .collect();
        assert_eq!(copies.len(), 2, "Ball circulates as 2 physical cards (08-tech-cards.yaml)");
        let mut s = lab_state();
        s.lab_state.lab_a.slots[0] = Some(copies[1]);
        s.players[0].money = 10;
        // Control: not owned yet => acquirable (cost 3, slot 0, power 3 => d=0).
        let got = actions(&laboratory_moves(&s, &rules, AreaType::LaboratoryA, 3));
        assert_same_set(&got, &[lab_action(copies[1], 0, 0, None)]);
        // Gate: own the OTHER copy => the boarded copy is blocked.
        s.players[0].technologies.push(copies[0]);
        assert!(laboratory_moves(&s, &rules, AreaType::LaboratoryA, 3).is_empty());
    }

    /// Four distinct-title techs to trigger the 5th-tech penalty.
    fn four_owned_techs(rules: &Rules) -> Vec<TechCardId> {
        [
            TechEffect::LanguageSchool,
            TechEffect::Telephone,
            TechEffect::Newspaper,
            TechEffect::Exposition,
        ]
        .iter()
        .map(|e| tech_by_effect(rules, *e).id)
        .collect()
    }

    #[test]
    fn lab_fifth_tech_penalty_variants() {
        // [RB p.12 Note]: with >=4 techs owned, each acquisition costs an
        // ADDITIONAL 2 Yen or 1 Imported Good — Some(true)/Some(false) each
        // enumerated iff separately affordable, never None.
        let rules = Rules::standard();
        let ball = tech_by_effect(&rules, TechEffect::Ball);
        assert_eq!(ball.industry_cost, 3);
        let mut s = lab_state();
        s.players[0].technologies = four_owned_techs(&rules);
        s.lab_state.lab_a.slots[0] = Some(ball.id); // required 3; power 3 => d=0
        s.players[0].money = 2;
        s.players[0].imported_goods = 1;
        let got = actions(&laboratory_moves(&s, &rules, AreaType::LaboratoryA, 3));
        assert_same_set(
            &got,
            &[
                lab_action(ball.id, 0, 0, Some(true)),
                lab_action(ball.id, 0, 0, Some(false)),
            ],
        );

        // Only the yen surcharge affordable.
        s.players[0].imported_goods = 0;
        let got = actions(&laboratory_moves(&s, &rules, AreaType::LaboratoryA, 3));
        assert_same_set(&got, &[lab_action(ball.id, 0, 0, Some(true))]);

        // Only the imported surcharge affordable.
        s.players[0].money = 0;
        s.players[0].imported_goods = 1;
        let got = actions(&laboratory_moves(&s, &rules, AreaType::LaboratoryA, 3));
        assert_same_set(&got, &[lab_action(ball.id, 0, 0, Some(false))]);

        // Neither affordable: the acquisition is illegal.
        s.players[0].imported_goods = 0;
        assert!(laboratory_moves(&s, &rules, AreaType::LaboratoryA, 3).is_empty());
    }

    #[test]
    fn lab_under_four_techs_no_penalty() {
        // [RB p.12 Note] penalty binds from the 5th acquisition only: with 3
        // owned techs, `fifth_tech_payment` is None.
        let rules = Rules::standard();
        let ball = tech_by_effect(&rules, TechEffect::Ball);
        let mut s = lab_state();
        s.players[0].technologies = four_owned_techs(&rules)[..3].to_vec();
        s.lab_state.lab_a.slots[0] = Some(ball.id);
        let got = actions(&laboratory_moves(&s, &rules, AreaType::LaboratoryA, 3));
        assert_same_set(&got, &[lab_action(ball.id, 0, 0, None)]);
    }

    #[test]
    fn lab_fifth_penalty_interacts_with_deficit_cover() {
        // Full affordability arithmetic (spec §3.4): Telegram at slot 0
        // (required 6), power 5 => d=1; 4 techs owned; money 2, imported 1.
        // (yen-cover 2, Some(true)) needs 4 Yen: OUT. (yen-cover, Some(false))
        // and (imported-cover, Some(true)) are IN. (imported-cover,
        // Some(false)) needs 2 imported: OUT.
        let rules = Rules::standard();
        let telegram = tech_by_effect(&rules, TechEffect::Telegram);
        let mut s = lab_state();
        s.players[0].technologies = four_owned_techs(&rules);
        s.lab_state.lab_a.slots[0] = Some(telegram.id);
        s.players[0].money = 2;
        s.players[0].imported_goods = 1;
        let got = actions(&laboratory_moves(&s, &rules, AreaType::LaboratoryA, 5));
        assert_same_set(
            &got,
            &[
                lab_action(telegram.id, 2, 0, Some(false)),
                lab_action(telegram.id, 0, 1, Some(true)),
            ],
        );
    }

    #[test]
    fn lab_unaffordable_deficit_yields_nothing() {
        // Tram at slot 2 => required 6, power 5 => d=1; money 1, no imported:
        // no cover is payable.
        let rules = Rules::standard();
        let tram = tech_by_effect(&rules, TechEffect::Tram);
        let mut s = lab_state();
        s.lab_state.lab_a.slots[2] = Some(tram.id);
        s.players[0].money = 1;
        assert!(laboratory_moves(&s, &rules, AreaType::LaboratoryA, 5).is_empty());
    }

    #[test]
    fn lab_excess_power_is_lost() {
        // [RB p.12] excess Industry is lost: cost 3 at slot 0 with power 5
        // yields the single zero-extra cover, nothing else.
        let rules = Rules::standard();
        let ball = tech_by_effect(&rules, TechEffect::Ball);
        let mut s = lab_state();
        s.lab_state.lab_a.slots[0] = Some(ball.id);
        s.players[0].money = 10;
        s.players[0].imported_goods = 3;
        let got = actions(&laboratory_moves(&s, &rules, AreaType::LaboratoryA, 5));
        assert_same_set(&got, &[lab_action(ball.id, 0, 0, None)]);
    }

    #[test]
    fn lab_b_missing_and_empty_slots_defensive() {
        // Missing Lab B (2-3P) is defensively empty; empty slots yield nothing.
        let rules = Rules::standard();
        let s = lab_state();
        assert!(laboratory_moves(&s, &rules, AreaType::LaboratoryB, 5).is_empty());
        assert!(laboratory_moves(&s, &rules, AreaType::LaboratoryA, 5).is_empty());
    }
}
