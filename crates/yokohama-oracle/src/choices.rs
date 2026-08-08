//! §2 queue-head choice resolution (spec §2, Author A).
//!
//! When `state.pending_choices` is non-empty the legal set is EXACTLY the head
//! choice's resolution moves (spec §2). One arm per `PendingChoice` variant,
//! wildcard-free so a future 8th variant breaks the build here. Input states
//! are normalized (spec §1.2): a queued head is a real multi-option choice, so
//! this function derives each head's option set from the rules without assuming
//! the count.

use yokohama_core::{
    GameState, Move, MoveSource, MoveTarget, PendingChoice, Rules, TradeGood, TradingHouseState,
    WarehouseItem,
};

use crate::area_actions_tracks::{eligible_church_spaces, eligible_customs_spaces};
use crate::common::placeable_areas;

/// Resolution moves for the head of `state.pending_choices`. Empty when the
/// queue is empty (the lib.rs dispatch only calls this on a non-empty queue,
/// but the guard keeps the function total). Choices belong to the current
/// player (the FIFO queue is emptied at every hand-off, `types/state.rs` doc).
pub(crate) fn head_choice_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    let _ = rules; // §2 option spaces are state-derived; rules kept for parity.
    let Some(head) = state.pending_choices.first() else {
        return Vec::new();
    };
    let p = state.current_player();
    match head {
        // §2.1 AnyGood: exactly the four goods; no decline (gain is unconditional).
        PendingChoice::AnyGood => vec![
            Move::ChooseAnyGood { good: TradeGood::Copper },
            Move::ChooseAnyGood { good: TradeGood::Silk },
            Move::ChooseAnyGood { good: TradeGood::Tea },
            Move::ChooseAnyGood { good: TradeGood::Seafood },
        ],
        // §2.2 WarehouseTake: one move per available AND affordable item, plus
        // the always-offered Decline (a payer cannot be compelled to spend).
        PendingChoice::WarehouseTake => {
            let mut moves = Vec::new();
            // Assistant: free (RB p.13).
            if p.assistants.in_warehouse >= 1 {
                moves.push(Move::ChooseWarehouseTake { item: WarehouseItem::Assistant });
            }
            // Shophouse: 2 Yen (RB p.13).
            if p.shophouses.in_warehouse >= 1 && p.money >= 2 {
                moves.push(Move::ChooseWarehouseTake { item: WarehouseItem::Shophouse });
            }
            // Trading House: printed 4/5/6/7 progression at tokens already out
            // of the warehouse (in hand + built) — `TradingHouseState` is the
            // single source (RB p.13; decision log 2026-07-17 staging).
            if p.trading_houses.in_warehouse >= 1 {
                let cost = TradingHouseState::take_cost(p.trading_houses.already_taken());
                if p.money >= cost {
                    moves.push(Move::ChooseWarehouseTake { item: WarehouseItem::TradingHouse });
                }
            }
            moves.push(Move::DeclineWarehouseTake);
            moves
        }
        // §2.3 AssistantMove: from hand or any own-assistant area, to any
        // non-Canal area (to ≠ from for an area source) OR — for an on-board
        // source only — back to the owner's hand; Decline always offered.
        PendingChoice::AssistantMove => {
            let mut froms = Vec::new();
            if p.assistants.in_hand >= 1 {
                froms.push(MoveSource::Hand);
            }
            for (area, count) in &p.assistants.on_board {
                if *count >= 1 {
                    froms.push(MoveSource::Area(*area));
                }
            }
            let targets = placeable_areas(state);
            let mut moves = Vec::new();
            for from in froms {
                // D3 [DL 2026-08-08, owner-ruled for BGA PARITY]: every
                // ON-BOARD source may return the assistant to the owner's
                // hand, where it stays. The p.10 text is ambiguous and BGA
                // allows it; the owner adopted the return for parity while
                // holding the original area-to-area reading probably
                // accurate. Hand-to-hand is never emitted — a structural
                // no-op indistinguishable from Decline.
                if matches!(from, MoveSource::Area(_)) {
                    moves.push(Move::ChooseAssistantMove { from, to: MoveTarget::Hand });
                }
                for &to in &targets {
                    // "Move … to another Area" (RB p.10): a from-Area source
                    // may not target its own area.
                    if matches!(from, MoveSource::Area(a) if a == to) {
                        continue;
                    }
                    moves.push(Move::ChooseAssistantMove { from, to: MoveTarget::Area(to) });
                }
            }
            moves.push(Move::DeclineAssistantMove);
            moves
        }
        // §2.4 ChurchSpace: every eligible empty space ≤ total_faith; no decline
        // (the action already validated ≥2 eligible spaces). Shared helper.
        PendingChoice::ChurchSpace { total_faith } => eligible_church_spaces(state, *total_faith)
            .into_iter()
            .map(|space| Move::ChooseChurchSpace { space })
            .collect(),
        // §2.5 CustomsSpace: every empty space for the exact cost; no decline.
        PendingChoice::CustomsSpace { cost } => eligible_customs_spaces(state, *cost)
            .into_iter()
            .map(|space| Move::ChooseCustomsSpace { space })
            .collect(),
        // §2.6 RecoveryLeave: exactly two fixed options (Telephone; a lone kept
        // assistant recovers zero, decision log 2026-07-19 §5.3-5).
        PendingChoice::RecoveryLeave => vec![
            Move::ChooseRecoveryLeave { leave_one: false },
            Move::ChooseRecoveryLeave { leave_one: true },
        ],
        // §2.7 StationPlace: every non-Canal area; NO decline (the token must
        // land). Occupancy by pawns/buildings/presidents does not restrict it.
        PendingChoice::StationPlace => placeable_areas(state)
            .into_iter()
            .map(|area| Move::ChooseStationPlace { area })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::two_player_state;
    use yokohama_core::{AreaId, PendingChoice, Rules};
    use yokohama_core::MoveTarget;

    fn rules() -> Rules {
        Rules::empty_for_tests()
    }

    #[test]
    fn any_good_offers_exactly_four() {
        let mut s = two_player_state();
        s.pending_choices = vec![PendingChoice::AnyGood];
        let moves = head_choice_moves(&s, &rules());
        assert_eq!(moves.len(), 4);
        for g in [TradeGood::Copper, TradeGood::Silk, TradeGood::Tea, TradeGood::Seafood] {
            assert!(moves.contains(&Move::ChooseAnyGood { good: g }));
        }
    }

    #[test]
    fn warehouse_take_assistant_free_and_decline_always() {
        let mut s = two_player_state();
        s.pending_choices = vec![PendingChoice::WarehouseTake];
        let moves = head_choice_moves(&s, &rules());
        assert!(moves.contains(&Move::ChooseWarehouseTake { item: WarehouseItem::Assistant }));
        assert!(moves.contains(&Move::DeclineWarehouseTake));
    }

    #[test]
    fn warehouse_take_shophouse_gated_on_money() {
        let mut s = two_player_state();
        s.pending_choices = vec![PendingChoice::WarehouseTake];
        // money 3 ≥ 2 ⇒ shophouse offered.
        assert!(head_choice_moves(&s, &rules())
            .contains(&Move::ChooseWarehouseTake { item: WarehouseItem::Shophouse }));
        s.players[0].money = 1;
        assert!(!head_choice_moves(&s, &rules())
            .contains(&Move::ChooseWarehouseTake { item: WarehouseItem::Shophouse }));
    }

    #[test]
    fn warehouse_take_trading_house_progression_at_first_take() {
        let mut s = two_player_state();
        s.pending_choices = vec![PendingChoice::WarehouseTake];
        // already_taken 0 ⇒ cost 4.
        s.players[0].money = 4;
        assert!(head_choice_moves(&s, &rules())
            .contains(&Move::ChooseWarehouseTake { item: WarehouseItem::TradingHouse }));
        s.players[0].money = 3;
        assert!(!head_choice_moves(&s, &rules())
            .contains(&Move::ChooseWarehouseTake { item: WarehouseItem::TradingHouse }));
    }

    #[test]
    fn warehouse_take_only_decline_when_warehouse_empty() {
        let mut s = two_player_state();
        s.pending_choices = vec![PendingChoice::WarehouseTake];
        s.players[0].assistants.in_warehouse = 0;
        s.players[0].shophouses.in_warehouse = 0;
        s.players[0].trading_houses.in_warehouse = 0;
        s.players[0].money = 100;
        assert_eq!(head_choice_moves(&s, &rules()), vec![Move::DeclineWarehouseTake]);
    }

    #[test]
    fn assistant_move_from_hand_to_noncanal_targets_plus_decline() {
        let mut s = two_player_state();
        s.pending_choices = vec![PendingChoice::AssistantMove];
        let moves = head_choice_moves(&s, &rules());
        for id in [0, 1, 2, 3] {
            assert!(moves.contains(&Move::ChooseAssistantMove {
                from: MoveSource::Hand,
                to: MoveTarget::Area(AreaId::new(id)),
            }));
        }
        assert!(!moves.contains(&Move::ChooseAssistantMove {
            from: MoveSource::Hand,
            to: MoveTarget::Area(AreaId::new(4)), // Canal
        }));
        assert!(moves.contains(&Move::DeclineAssistantMove));
    }

    #[test]
    fn assistant_move_from_area_excludes_self_and_canal() {
        let mut s = two_player_state();
        s.pending_choices = vec![PendingChoice::AssistantMove];
        s.players[0].assistants.in_hand = 0;
        s.players[0].assistants.on_board.insert(AreaId::new(1), 2);
        let moves = head_choice_moves(&s, &rules());
        assert!(moves.contains(&Move::ChooseAssistantMove {
            from: MoveSource::Area(AreaId::new(1)),
            to: MoveTarget::Area(AreaId::new(0)),
        }));
        assert!(!moves.contains(&Move::ChooseAssistantMove {
            from: MoveSource::Area(AreaId::new(1)),
            to: MoveTarget::Area(AreaId::new(1)), // to ≠ from
        }));
        assert!(!moves.contains(&Move::ChooseAssistantMove {
            from: MoveSource::Area(AreaId::new(1)),
            to: MoveTarget::Area(AreaId::new(4)), // Canal
        }));
        // Hand is empty ⇒ no from-Hand options.
        assert!(!moves
            .iter()
            .any(|m| matches!(m, Move::ChooseAssistantMove { from: MoveSource::Hand, .. })));
    }

    #[test]
    fn assistant_move_area_source_offers_the_hand_return_hand_source_does_not() {
        // D3 [DL 2026-08-08, owner-ruled for BGA PARITY]: a movement icon may
        // return one of the mover's ON-BOARD assistants to the owner's hand,
        // where it simply stays — BGA's engine allows it, the printed p.10
        // text is ambiguous, and the owner adopted the return for parity
        // while recording that he holds the original area-to-area reading is
        // probably accurate. Hand-to-hand is NEVER emitted: it would change
        // nothing (a structural no-op indistinguishable from Decline).
        let mut s = two_player_state();
        s.pending_choices = vec![PendingChoice::AssistantMove];
        s.players[0].assistants.on_board.insert(AreaId::new(1), 1);
        let moves = head_choice_moves(&s, &rules());
        assert!(moves.contains(&Move::ChooseAssistantMove {
            from: MoveSource::Area(AreaId::new(1)),
            to: MoveTarget::Hand,
        }));
        assert!(!moves.contains(&Move::ChooseAssistantMove {
            from: MoveSource::Hand,
            to: MoveTarget::Hand, // hand-to-hand is not a move
        }));
    }

    #[test]
    fn recovery_leave_offers_both_options() {
        let mut s = two_player_state();
        s.pending_choices = vec![PendingChoice::RecoveryLeave];
        let moves = head_choice_moves(&s, &rules());
        assert_eq!(moves.len(), 2);
        assert!(moves.contains(&Move::ChooseRecoveryLeave { leave_one: false }));
        assert!(moves.contains(&Move::ChooseRecoveryLeave { leave_one: true }));
    }

    #[test]
    fn station_place_offers_all_noncanal_areas_no_decline() {
        let mut s = two_player_state();
        s.pending_choices = vec![PendingChoice::StationPlace];
        let moves = head_choice_moves(&s, &rules());
        assert_eq!(moves.len(), 4);
        for id in [0, 1, 2, 3] {
            assert!(moves.contains(&Move::ChooseStationPlace { area: AreaId::new(id) }));
        }
        assert!(!moves.contains(&Move::ChooseStationPlace { area: AreaId::new(4) }));
    }

    // --- Stub-dependent (lane D `eligible_*`): written now, ignored until integration. ---

    #[test]
    fn church_space_maps_eligible_spaces_no_decline() {
        let mut s = two_player_state();
        s.pending_choices = vec![PendingChoice::ChurchSpace { total_faith: 6 }];
        let moves = head_choice_moves(&s, &rules());
        assert!(!moves.is_empty());
        assert!(moves.iter().all(|m| matches!(m, Move::ChooseChurchSpace { .. })));
    }

    #[test]
    fn customs_space_maps_eligible_spaces_no_decline() {
        let mut s = two_player_state();
        s.pending_choices = vec![PendingChoice::CustomsSpace { cost: 1 }];
        let moves = head_choice_moves(&s, &rules());
        assert!(!moves.is_empty());
        assert!(moves.iter().all(|m| matches!(m, Move::ChooseCustomsSpace { .. })));
    }
}
