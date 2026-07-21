//! §3.4 Church + Customs area actions — implemented per spec §3.4 (lane D
//! landed; bodies are complete, no `todo!()` remains).
//!
//! Signatures were skeletoned by lane A so sibling lanes could compile against
//! these EXACT shapes. `Skip` is NOT added here — the lib.rs area-action facade
//! appends the single canonical decline. The `eligible_*` helpers are shared
//! with `choices.rs` (§2.4/§2.5), so they already have a caller and carry no
//! `#[allow(dead_code)]`.

use yokohama_core::{AreaAction, AreaId, ChurchTrack, CustomsTrack, GameState, Move};

use crate::common::track_assistant_available;

/// §3.4 Church: every goods-backed payment combo whose `total_faith` reaches an
/// eligible empty space, with an assistant available from hand or the acting
/// area. `acting` is the Church area (own-assistant source, §3.4/§3.9).
pub(crate) fn church_moves(state: &GameState, power: u8, acting: AreaId) -> Vec<Move> {
    let p = state.current_player();
    // "place 1 Assistant pawn from the Church Area or your hand" [RB p.13;
    // DL 2026-07-18 final-review I3] — no assistant, no combo (Skip remains,
    // appended by the lib.rs facade).
    if !track_assistant_available(state, acting) {
        return Vec::new();
    }
    let mut out = Vec::new();
    // All 2^6 payment combos — booleans encode "max 1 of each kind per turn"
    // [RB p.13 Note]. All-false stays substantive: faith = power
    // [DL 2026-07-17 area-action-optionality row].
    for bits in 0u8..64 {
        let copper = bits & 1 != 0;
        let silk = bits & 2 != 0;
        let tea = bits & 4 != 0;
        let seafood = bits & 8 != 0;
        let yen = bits & 16 != 0;
        let imported_good = bits & 32 != 0;
        // Every true flag must be backed by >= 1 of that resource — combos
        // failing validation are ILLEGAL, not no-ops [RB p.13; DL 2026-07-17
        // "Church executor v1 semantic"].
        if (copper && p.copper < 1)
            || (silk && p.silk < 1)
            || (tea && p.tea < 1)
            || (seafood && p.seafood < 1)
            || (yen && p.money < 1)
            || (imported_good && p.imported_goods < 1)
        {
            continue;
        }
        // Faith from power is LINEAR 1..5 — the rulebook's "2 Power: 1 Faith"
        // is a TYPO [DL 2026-07-17 Jeremy adjudication; KB §Church correction]
        // — plus 1 per distinct payment type.
        let total_faith = power.min(5) + bits.count_ones() as u8;
        // >= 1 eligible space must exist, else the combo is illegal (excess
        // faith is lost but a placement must happen) [RB p.13; spec §3.4].
        if eligible_church_spaces(state, total_faith).is_empty() {
            continue;
        }
        out.push(Move::ExecuteAreaAction {
            action: AreaAction::Church { copper, silk, tea, seafood, yen, imported_good },
        });
    }
    out
}

/// §3.4 Customs: every `Customs { n }` whose `n` exactly matches an empty,
/// unblocked printed space cost, `1 ≤ n ≤ min(power-1, 4)`, assistant available
/// from hand or the acting area.
pub(crate) fn customs_moves(state: &GameState, power: u8, acting: AreaId) -> Vec<Move> {
    let p = state.current_player();
    // "from the Customs Area or your hand" [RB p.14; DL 2026-07-18 I3].
    if !track_assistant_available(state, acting) {
        return Vec::new();
    }
    // Power -> max payable imported goods: 0/1/2/3/4 at power 1-5 [RB p.14];
    // n = 0 is validator-ILLEGAL (Skip canonical) [DL 2026-07-17 Customs
    // executor + optionality rows].
    let max_n = power.min(5).saturating_sub(1).min(4);
    (1..=max_n)
        .filter(|&n| u32::from(n) <= p.imported_goods)
        // The payment must exactly match a printed space cost with an empty,
        // unblocked space [RB p.14 + payment-must-match ruling, DL 2026-07-17
        // Customs re-model row].
        .filter(|&n| !eligible_customs_spaces(state, n).is_empty())
        .map(|n| Move::ExecuteAreaAction { action: AreaAction::Customs { imported_goods_paid: n } })
        .collect()
}

/// Eligible empty Church spaces: value in 4..=11, empty slot, `≤ total_faith`,
/// excluding the 2-player faith-7 dummy (spec §2.4; shared with choices.rs).
pub(crate) fn eligible_church_spaces(state: &GameState, total_faith: u8) -> Vec<u8> {
    // "any empty space with value <= total faith, excess faith lost"
    // [RB p.13; DL 2026-07-17 Church placement row]; the board's printed
    // values run 4-11. 2P games pre-block the faith-7 space [DL 2026-07-17
    // 2P-dummy row] — excluded by rule here regardless of how the engine
    // represents the dummy (an engine-side pre-fill would also fail the
    // empty-slot check).
    let two_player = state.config.player_count == 2;
    (4u8..=11)
        .filter(|&s| s <= total_faith)
        .filter(|&s| !(two_player && s == 7))
        .filter(|&s| {
            let idx = ChurchTrack::faith_to_index(s).expect("faith 4-11 always maps");
            state.church_track.slots[idx].is_none()
        })
        .collect()
}

/// Eligible empty Customs spaces for an exact printed `cost`, excluding the
/// 2-player space-4 dummy (spec §2.5; shared with choices.rs).
pub(crate) fn eligible_customs_spaces(state: &GameState, cost: u8) -> Vec<u8> {
    // Printed cost pairs: 1 -> {1,2}; 2 -> {3,4}; 3 -> {5,6}; 4 -> {7}
    // [DL 2026-07-17 Customs re-model row]; 2P games dummy-block space 4
    // [DL 2026-07-17] — excluded by rule, same belt-and-braces note as the
    // Church helper.
    let two_player = state.config.player_count == 2;
    CustomsTrack::spaces_for_cost(cost)
        .map(|spaces| {
            spaces
                .iter()
                .copied()
                .filter(|&s| !(two_player && s == 4))
                .filter(|&s| {
                    let idx = CustomsTrack::space_to_index(s).expect("spaces 1-7 always map");
                    state.customs_track.slots[idx].is_none()
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::two_player_state;
    use yokohama_core::{
        AreaAction, ChurchSlot, ChurchTrack, CustomsSlot, CustomsTrack, GameConfig, Move, PlayerId,
    };

    /// The Church acting area on the small test board.
    fn church_area() -> AreaId {
        AreaId::new(2)
    }
    /// The Customs acting area on the small test board.
    fn customs_area() -> AreaId {
        AreaId::new(3)
    }

    fn church(
        copper: bool,
        silk: bool,
        tea: bool,
        seafood: bool,
        yen: bool,
        imported_good: bool,
    ) -> Move {
        Move::ExecuteAreaAction {
            action: AreaAction::Church { copper, silk, tea, seafood, yen, imported_good },
        }
    }

    fn customs(n: u8) -> Move {
        Move::ExecuteAreaAction { action: AreaAction::Customs { imported_goods_paid: n } }
    }

    fn fill_church(s: &mut yokohama_core::GameState, faith: u8) {
        let idx = ChurchTrack::faith_to_index(faith).unwrap();
        s.church_track.slots[idx] =
            Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 0 });
    }

    fn fill_customs(s: &mut yokohama_core::GameState, space: u8) {
        let idx = CustomsTrack::space_to_index(space).unwrap();
        s.customs_track.slots[idx] =
            Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 });
    }

    // --- eligible_church_spaces (§2.4) ---

    #[test]
    fn eligible_church_spaces_empty_track_up_to_faith() {
        // [RB p.13] any empty space with value <= total faith, values 4-11.
        let s = two_player_state();
        assert_eq!(eligible_church_spaces(&s, 6), vec![4, 5, 6]);
        assert_eq!(eligible_church_spaces(&s, 3), Vec::<u8>::new());
    }

    #[test]
    fn eligible_church_spaces_2p_faith7_dummy_blocked() {
        // [DL 2026-07-17] 2P games: church faith-7 space PRE-BLOCKED.
        let s = two_player_state();
        assert_eq!(eligible_church_spaces(&s, 11), vec![4, 5, 6, 8, 9, 10, 11]);
    }

    #[test]
    fn eligible_church_spaces_faith7_open_at_three_players() {
        let mut s = two_player_state();
        s.config = GameConfig::for_player_count(3);
        assert_eq!(eligible_church_spaces(&s, 8), vec![4, 5, 6, 7, 8]);
    }

    #[test]
    fn eligible_church_spaces_excludes_filled() {
        let mut s = two_player_state();
        fill_church(&mut s, 4);
        fill_church(&mut s, 5);
        assert_eq!(eligible_church_spaces(&s, 6), vec![6]);
    }

    // --- eligible_customs_spaces (§2.5) ---

    #[test]
    fn eligible_customs_spaces_printed_pairs() {
        // [DL 2026-07-17 Customs re-model] cost 1 -> {1,2}; 2 -> {3,4};
        // 3 -> {5,6}; 4 -> {7}; 2P space-4 dummy-blocked.
        let s = two_player_state();
        assert_eq!(eligible_customs_spaces(&s, 1), vec![1, 2]);
        assert_eq!(eligible_customs_spaces(&s, 2), vec![3]); // 4 is the 2P dummy
        assert_eq!(eligible_customs_spaces(&s, 3), vec![5, 6]);
        assert_eq!(eligible_customs_spaces(&s, 4), vec![7]);
        assert_eq!(eligible_customs_spaces(&s, 0), Vec::<u8>::new());
        assert_eq!(eligible_customs_spaces(&s, 5), Vec::<u8>::new());
    }

    #[test]
    fn eligible_customs_spaces_space4_open_at_three_players() {
        let mut s = two_player_state();
        s.config = GameConfig::for_player_count(3);
        assert_eq!(eligible_customs_spaces(&s, 2), vec![3, 4]);
    }

    #[test]
    fn eligible_customs_spaces_excludes_filled() {
        let mut s = two_player_state();
        fill_customs(&mut s, 3);
        // 2P: 3 filled + 4 dummy => cost-2 pair fully unplaceable.
        assert_eq!(eligible_customs_spaces(&s, 2), Vec::<u8>::new());
        fill_customs(&mut s, 1);
        assert_eq!(eligible_customs_spaces(&s, 1), vec![2]);
    }

    // --- church_moves (§3.4; F9 [RB p.13]) ---

    #[test]
    fn f9_church_faith_example() {
        // F9 [RB p.13]: power 3, pay copper+silk+yen => faith 6; the combo is
        // legal iff an empty space <= 6 exists.
        let mut s = two_player_state();
        let combo = church(true, true, false, false, true, false);
        assert!(church_moves(&s, 3, church_area()).contains(&combo));
        // Spaces 4,5 filled, 6 empty (singleton) — combo stays legal; the
        // inline-vs-queue split is apply-side (spec §2.4).
        fill_church(&mut s, 4);
        fill_church(&mut s, 5);
        assert!(church_moves(&s, 3, church_area()).contains(&combo));
        assert_eq!(eligible_church_spaces(&s, 6), vec![6]);
        // Space 6 also filled => nothing <= faith 6 left => combo illegal.
        fill_church(&mut s, 6);
        assert!(!church_moves(&s, 3, church_area()).contains(&combo));
    }

    #[test]
    fn f9_linear_faith_pin() {
        // Faith from power is LINEAR 1..5 — the rulebook's "2 Power: 1 Faith"
        // is a TYPO [DL 2026-07-17 Jeremy adjudication]. Power 2, no goods =>
        // faith 2 < 4 => illegal; one flag => 3 => illegal; two flags => 4 =>
        // legal when backed.
        let s = two_player_state();
        let moves = church_moves(&s, 2, church_area());
        assert!(!moves.contains(&church(false, false, false, false, false, false)));
        assert!(!moves.contains(&church(true, false, false, false, false, false)));
        assert!(moves.contains(&church(true, true, false, false, false, false)));
    }

    #[test]
    fn church_all_false_substantive_at_power_four() {
        // All-false is substantive (faith = power) and legal at power >= 4
        // with an eligible space [DL 2026-07-17 optionality row].
        let s = two_player_state();
        assert!(church_moves(&s, 4, church_area())
            .contains(&church(false, false, false, false, false, false)));
    }

    #[test]
    fn church_flags_must_be_backed_by_holdings() {
        // [RB p.13] each payment type max 1, and only if actually held.
        let mut s = two_player_state();
        s.players[0].copper = 0;
        s.players[0].money = 0;
        s.players[0].imported_goods = 0;
        let moves = church_moves(&s, 5, church_area());
        assert!(!moves.contains(&church(true, false, false, false, false, false)));
        assert!(!moves.contains(&church(false, false, false, false, true, false)));
        assert!(!moves.contains(&church(false, false, false, false, false, true)));
        // Silk is still held (default 1) — a silk-only combo stays legal.
        assert!(moves.contains(&church(false, true, false, false, false, false)));
        s.players[0].imported_goods = 1;
        assert!(church_moves(&s, 5, church_area())
            .contains(&church(false, false, false, false, false, true)));
    }

    #[test]
    fn church_assistant_from_hand_or_acting_area() {
        // [RB p.13; DL 2026-07-18 I3] assistant from hand OR the Church area —
        // the mercenary hand-0 shape stays legal.
        let mut s = two_player_state();
        s.players[0].assistants.in_hand = 0;
        assert!(church_moves(&s, 5, church_area()).is_empty());
        s.players[0].assistants.on_board.insert(church_area(), 1);
        assert!(!church_moves(&s, 5, church_area()).is_empty());
    }

    #[test]
    fn church_requires_an_eligible_space() {
        // Combos failing eligibility are ILLEGAL, not no-ops (DL 2026-07-17
        // Church executor v1 semantic).
        let mut s = two_player_state();
        for faith in 4..=11 {
            if faith != 7 {
                fill_church(&mut s, faith);
            }
        }
        // Track full (7 is the 2P dummy) => no combo at any power.
        assert!(church_moves(&s, 5, church_area()).is_empty());
    }

    // --- customs_moves (§3.4; F10 [RB p.14]) ---

    #[test]
    fn f10_customs_example() {
        // F10 [RB p.14]: power 2, 1 imported good => Customs{1} legal
        // (spaces 1,2 empty); Customs{2} absent (power caps payment at 1).
        let mut s = two_player_state();
        s.players[0].imported_goods = 1;
        let moves = customs_moves(&s, 2, customs_area());
        assert_eq!(moves, vec![customs(1)]);
        // Power 5 + 4 imported => Customs{4} legal iff space 7 empty.
        s.players[0].imported_goods = 4;
        assert!(customs_moves(&s, 5, customs_area()).contains(&customs(4)));
        fill_customs(&mut s, 7);
        assert!(!customs_moves(&s, 5, customs_area()).contains(&customs(4)));
    }

    #[test]
    fn customs_payment_capped_by_imported_goods() {
        let mut s = two_player_state();
        s.players[0].imported_goods = 2;
        let moves = customs_moves(&s, 5, customs_area());
        assert!(moves.contains(&customs(1)));
        assert!(moves.contains(&customs(2)));
        assert!(!moves.contains(&customs(3)));
    }

    #[test]
    fn customs_exact_cost_pair_filled_unplaceable() {
        // [DL 2026-07-17 Customs re-model] payment must match a printed cost
        // with an empty space; 2P space-4 dummy makes {3} the whole cost-2 pair.
        let mut s = two_player_state();
        s.players[0].imported_goods = 4;
        fill_customs(&mut s, 3);
        let moves = customs_moves(&s, 5, customs_area());
        assert!(!moves.contains(&customs(2)));
        assert!(moves.contains(&customs(1)));
        assert!(moves.contains(&customs(3)));
    }

    #[test]
    fn customs_assistant_from_hand_or_acting_area() {
        // [RB p.14; DL 2026-07-18 I3].
        let mut s = two_player_state();
        s.players[0].imported_goods = 2;
        s.players[0].assistants.in_hand = 0;
        assert!(customs_moves(&s, 5, customs_area()).is_empty());
        s.players[0].assistants.on_board.insert(customs_area(), 1);
        assert!(!customs_moves(&s, 5, customs_area()).is_empty());
    }

    #[test]
    fn customs_zero_payment_never_emitted() {
        // n = 0 is validator-ILLEGAL — Skip is the canonical decline
        // (DL 2026-07-17 Customs executor + optionality rows).
        let mut s = two_player_state();
        s.players[0].imported_goods = 4;
        for power in 1..=5u8 {
            assert!(!customs_moves(&s, power, customs_area()).contains(&customs(0)));
        }
    }
}
