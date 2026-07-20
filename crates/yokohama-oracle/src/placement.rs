//! §3.2 placement enumeration — lane B, implemented per spec §3.2.
//!
//! Every rule here is authored from the rules sources only (rulebook p.5/p.16,
//! decision-log rows cited inline); no engine logic file was consulted.

use yokohama_core::{AreaId, GameState, Move, PlayerId, Rules, TechEffect};

use crate::common::{opponent_presidents_in, owns_effect, placeable_areas};

/// §3.2: Option A subsets (0..=N distinct placeable areas) + Option B
/// (2-into-one-area, optional Telegram adjacent bonus), affordability-gated.
///
/// Placement is from HAND only and never into the Canal [RB p.5]; insufficient
/// money makes a placement ILLEGAL, not partially performed [RB p.5 "If you
/// have insufficient money, you may not place"].
pub(crate) fn placement_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    let player = state.current_player;
    let p = state.current_player();
    let hand = p.assistants.in_hand;
    let money = p.money;
    let areas = placeable_areas(state); // ascending AreaId (§7.2 normal form)
    let mut moves = Vec::new();

    // Option A — every subset of distinct placeable areas of size 0..=N,
    // N = min(hand, 3, or 4 with Newspaper [RB p.16]). The empty placement is
    // always legal and free [RB p.5 "1, 2 or 3"; zero ruled legal DL
    // 2026-07-18 I5, expanded DL 2026-07-19 §8 veto row].
    let cap: u8 = if owns_effect(state, rules, player, TechEffect::Newspaper) { 4 } else { 3 };
    let max_n = hand.min(cap) as usize;
    let mut current = Vec::new();
    let mut subsets = vec![Vec::new()];
    collect_subsets(&areas, 0, max_n, &mut current, &mut subsets);
    for subset in subsets {
        // Affordability: 1 Yen per assistant placed to EACH opponent-president
        // owner in the target area [RB p.5; per-assistant reading DL 2026-07-17 C1].
        let counts: Vec<(AreaId, u8)> = subset.iter().map(|a| (*a, 1)).collect();
        if placement_fee(state, player, &counts) <= money {
            moves.push(Move::PlaceThreeAreas { placements: subset });
        }
    }

    // Option B — 2 assistants into one area; fee is PER ASSISTANT, so a
    // president-occupied area costs 2, not 1 [DL 2026-07-17 C1]. No 0/1-form:
    // the zero-placement ruling attaches to Option A's empty vector only
    // [DL 2026-07-18 I5]. Telegram adds an OPTIONAL 3rd assistant into a
    // physically-adjacent area [RB p.16 "may additional place"] — the `None`
    // form is enumerated regardless of Telegram ownership.
    if hand >= 2 {
        let telegram = owns_effect(state, rules, player, TechEffect::Telegram);
        for &area in &areas {
            let base_fee = placement_fee(state, player, &[(area, 2)]);
            if base_fee <= money {
                moves.push(Move::PlaceTwoSameArea { area, adjacent_bonus: None });
            }
            if telegram && hand >= 3 {
                for adj in telegram_adjacent(state, area) {
                    // "Pay the usual cost if another President pawn occupies
                    // this adjacent Area" [RB p.16]: +1 per opponent president.
                    let fee = base_fee + placement_fee(state, player, &[(adj, 1)]);
                    if fee <= money {
                        moves.push(Move::PlaceTwoSameArea { area, adjacent_bonus: Some(adj) });
                    }
                }
            }
        }
    }

    moves
}

/// Enumerate every subset of `areas[start..]` extending `current`, up to
/// `max_n` total elements, into `out` (elements stay ascending because `areas`
/// is ascending — the §7.2 sorted normal form). `out` must be seeded with the
/// empty subset by the caller.
fn collect_subsets(
    areas: &[AreaId],
    start: usize,
    max_n: usize,
    current: &mut Vec<AreaId>,
    out: &mut Vec<Vec<AreaId>>,
) {
    if current.len() >= max_n {
        return;
    }
    for i in start..areas.len() {
        current.push(areas[i]);
        out.push(current.clone());
        collect_subsets(areas, i + 1, max_n, current, out);
        current.pop();
    }
}

/// 1 Yen × assistants placed × opponent presidents, summed per target area
/// (§3.2 fee rule; Option B's 2-into-one costs 2, decision log 2026-07-17 C1).
/// A player's own president never generates a fee against themself (§3
/// preamble — the rulebook's fee text targets "the player owning the President
/// pawn" [RB p.5] and its examples price only other players' presidents).
pub(crate) fn placement_fee(
    state: &GameState,
    player: PlayerId,
    per_area_counts: &[(AreaId, u8)],
) -> u32 {
    per_area_counts
        .iter()
        .map(|(area, n)| {
            opponent_presidents_in(state, player, *area).len() as u32 * u32::from(*n)
        })
        .sum()
}

/// Physical-contiguity neighbors minus canal-crossing pairs minus the Canal
/// itself (Telegram adjacent-bonus scope, §3.2): crossings are movement-transit
/// edges only — placement adjacency is physical tile contiguity [DL 2026-07-18
/// "Canal geometry RE-CONFIRMED … Telegram's adjacent-bonus placement excludes
/// crossing pairs"]. Ascending, duplicate-free.
pub(crate) fn telegram_adjacent(state: &GameState, area: AreaId) -> Vec<AreaId> {
    let board = &state.board_layout;
    let mut v: Vec<AreaId> = board
        .adjacent_areas(area)
        .iter()
        .copied()
        // Never the Canal (no pawns may ever be placed there [RB p.14]).
        .filter(|adj| board.areas.get(adj).is_some_and(|c| c.area_type.allows_placement()))
        // Crossing pairs are transit-only adjacency, not tile contiguity.
        .filter(|adj| !board.is_canal_crossing(area, *adj))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::two_player_state;
    use yokohama_core::{
        AreaConfig, AreaType, BoardLayout, BoardLayoutBuilder, BuildingSite, FivePowerEffect,
        GameConfig, GameState, PlayerColor, PlayerState, PresidentLocation, TechEffect, TurnPhase,
    };

    /// Private board helper (lane B owns no shared fixture; spec §5 lets a lane
    /// keep helpers private to its module).
    fn board(areas: &[(u8, AreaType)], adj: &[(u8, u8)], crossings: &[(u8, u8)]) -> BoardLayout {
        let mut b = BoardLayoutBuilder::new();
        for &(id, ty) in areas {
            let aid = AreaId::new(id);
            b.add_area(aid, AreaConfig { area_type: ty, grid_position: (0, id) });
            if ty != AreaType::Canal {
                b.set_building_site(aid, BuildingSite::new([1, 1, 1, 1], 3));
                b.set_five_power_token(aid, FivePowerEffect::Money(1));
            }
        }
        for &(x, y) in adj {
            b.add_adjacency(AreaId::new(x), AreaId::new(y));
        }
        for &(x, y) in crossings {
            b.add_canal_crossing(AreaId::new(x), AreaId::new(y));
        }
        b.build().expect("test board validates")
    }

    /// F1 [RB p.5] shape: CopperMine(0) president-free, EmploymentAgency(1)
    /// holds Blue's president, LaboratoryA(2) holds Yellow+Green presidents,
    /// SilkMill(3) free. Mover is player 0 (Red) in the Placement phase.
    fn f1_state() -> GameState {
        let mut s = two_player_state();
        s.config = GameConfig::for_player_count(4);
        s.players.push(PlayerState::new(PlayerId::new(2), PlayerColor::Yellow, false));
        s.players.push(PlayerState::new(PlayerId::new(3), PlayerColor::Green, false));
        s.board_layout = board(
            &[
                (0, AreaType::CopperMine),
                (1, AreaType::EmploymentAgency),
                (2, AreaType::LaboratoryA),
                (3, AreaType::SilkMill),
            ],
            &[(0, 1), (1, 2), (2, 3)],
            &[],
        );
        s.turn_phase = TurnPhase::Placement;
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));
        s.players[2].president_location = PresidentLocation::OnBoard(AreaId::new(2));
        s.players[3].president_location = PresidentLocation::OnBoard(AreaId::new(2));
        s
    }

    fn tech_id(rules: &Rules, effect: TechEffect) -> yokohama_core::TechCardId {
        rules
            .techs
            .iter()
            .find(|t| t.effect == effect)
            .map(|t| t.id)
            .expect("standard data carries the tech")
    }

    fn option_a(moves: &[Move]) -> Vec<&Vec<AreaId>> {
        moves
            .iter()
            .filter_map(|m| match m {
                Move::PlaceThreeAreas { placements } => Some(placements),
                _ => None,
            })
            .collect()
    }

    // --- F1 [RB p.5] Hisashi placement example ---

    #[test]
    fn f1_placement_fees_gate_option_a_sets() {
        let rules = Rules::empty_for_tests();
        let mut s = f1_state();
        s.players[0].money = 1;
        let moves = placement_moves(&s, &rules);
        // Copper Mine (free) + Employment Agency (1 Yen to Blue) + a free third
        // area is affordable at 1 Yen [RB p.5].
        assert!(moves.contains(&Move::PlaceThreeAreas {
            placements: vec![AreaId::new(0), AreaId::new(1), AreaId::new(3)],
        }));
        // Every Option-A set touching Laboratory A (2 presidents ⇒ 2 Yen for
        // one assistant) is unaffordable at 1 Yen [RB p.5 "If you have
        // insufficient money, you may not place"].
        assert!(option_a(&moves).iter().all(|p| !p.contains(&AreaId::new(2))));
        // With 2 Yen the Laboratory-A set appears.
        s.players[0].money = 2;
        let moves = placement_moves(&s, &rules);
        assert!(moves.contains(&Move::PlaceThreeAreas { placements: vec![AreaId::new(2)] }));
    }

    // --- §6.2 placement rows ---

    #[test]
    fn option_b_two_into_president_area_costs_two_not_one() {
        // DL 2026-07-17 C1: fees are per assistant PLACED.
        let rules = Rules::empty_for_tests();
        let mut s = f1_state();
        s.players[0].money = 1;
        let moves = placement_moves(&s, &rules);
        assert!(!moves
            .contains(&Move::PlaceTwoSameArea { area: AreaId::new(1), adjacent_bonus: None }));
        s.players[0].money = 2;
        let moves = placement_moves(&s, &rules);
        assert!(moves
            .contains(&Move::PlaceTwoSameArea { area: AreaId::new(1), adjacent_bonus: None }));
    }

    #[test]
    fn multi_president_area_prices_per_president_per_assistant() {
        // [RB p.5 "more than one President pawn … pay 1 Yen to each"].
        let mut s = f1_state();
        let p0 = PlayerId::new(0);
        assert_eq!(placement_fee(&s, p0, &[(AreaId::new(2), 2)]), 4);
        assert_eq!(placement_fee(&s, p0, &[(AreaId::new(1), 1), (AreaId::new(2), 1)]), 3);
        assert_eq!(placement_fee(&s, p0, &[(AreaId::new(0), 3)]), 0);
        // Own president never generates a fee against oneself (§3 preamble).
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        assert_eq!(placement_fee(&s, p0, &[(AreaId::new(0), 2)]), 0);
        // Option B into the two-president area needs 4 Yen.
        let rules = Rules::empty_for_tests();
        s.players[0].money = 3;
        let moves = placement_moves(&s, &rules);
        assert!(!moves
            .contains(&Move::PlaceTwoSameArea { area: AreaId::new(2), adjacent_bonus: None }));
        s.players[0].money = 4;
        let moves = placement_moves(&s, &rules);
        assert!(moves
            .contains(&Move::PlaceTwoSameArea { area: AreaId::new(2), adjacent_bonus: None }));
    }

    #[test]
    fn hand_zero_yields_only_the_empty_option_a() {
        // Zero placement is always legal and free [DL 2026-07-18 I5].
        let mut s = f1_state();
        s.players[0].assistants.in_hand = 0;
        s.players[0].money = 10;
        let moves = placement_moves(&s, &Rules::empty_for_tests());
        assert_eq!(moves, vec![Move::PlaceThreeAreas { placements: vec![] }]);
    }

    #[test]
    fn hand_caps_option_a_size_and_gates_option_b() {
        let mut s = f1_state();
        s.players[0].assistants.in_hand = 1;
        s.players[0].money = 10;
        let moves = placement_moves(&s, &Rules::empty_for_tests());
        assert!(moves.iter().all(|m| !matches!(m, Move::PlaceTwoSameArea { .. })));
        assert!(option_a(&moves).iter().all(|p| p.len() <= 1));
        assert!(moves.contains(&Move::PlaceThreeAreas { placements: vec![AreaId::new(0)] }));
        assert!(moves.contains(&Move::PlaceThreeAreas { placements: vec![] }));
    }

    // --- F14 [RB p.16] Newspaper ---

    #[test]
    fn f14_newspaper_lifts_option_a_cap_to_four() {
        let rules = Rules::standard();
        let mut s = f1_state();
        s.players[0].money = 10;
        // Without Newspaper: capped at 3 distinct areas; empty placement legal.
        let moves = placement_moves(&s, &rules);
        assert!(moves.contains(&Move::PlaceThreeAreas { placements: vec![] }));
        assert!(moves.contains(&Move::PlaceThreeAreas {
            placements: vec![AreaId::new(0), AreaId::new(1), AreaId::new(3)],
        }));
        assert!(option_a(&moves).iter().all(|p| p.len() <= 3));
        // With Newspaper [RB p.16]: the 4-distinct-area set appears; the empty
        // placement stays legal [DL 2026-07-18 I5].
        s.players[0].technologies.push(tech_id(&rules, TechEffect::Newspaper));
        let moves = placement_moves(&s, &rules);
        assert!(moves.contains(&Move::PlaceThreeAreas {
            placements: vec![AreaId::new(0), AreaId::new(1), AreaId::new(2), AreaId::new(3)],
        }));
        assert!(moves.contains(&Move::PlaceThreeAreas { placements: vec![] }));
    }

    // --- F13 [RB p.16] Telegram ---

    /// Telegram board: SilkMill(0) with contiguous neighbor CopperMine(1),
    /// canal-crossing neighbor TeaPlantation(2), and the Canal itself (4).
    fn telegram_state() -> GameState {
        let mut s = two_player_state();
        s.board_layout = board(
            &[
                (0, AreaType::SilkMill),
                (1, AreaType::CopperMine),
                (2, AreaType::TeaPlantation),
                (4, AreaType::Canal),
            ],
            &[(0, 1), (0, 2), (0, 4)],
            &[(0, 2)],
        );
        s.turn_phase = TurnPhase::Placement;
        s
    }

    #[test]
    fn f13_telegram_adjacent_bonus_scope_and_fees() {
        let rules = Rules::standard();
        let mut s = telegram_state();
        s.players[0].technologies.push(tech_id(&rules, TechEffect::Telegram));
        s.players[0].money = 10;
        let moves = placement_moves(&s, &rules);
        assert!(moves.contains(&Move::PlaceTwoSameArea {
            area: AreaId::new(0),
            adjacent_bonus: Some(AreaId::new(1)),
        }));
        // Canal-crossing pairs are transit-only adjacency — excluded from the
        // Telegram bonus [DL 2026-07-18 Canal geometry], as is the Canal itself.
        assert!(!moves.contains(&Move::PlaceTwoSameArea {
            area: AreaId::new(0),
            adjacent_bonus: Some(AreaId::new(2)),
        }));
        assert!(!moves.contains(&Move::PlaceTwoSameArea {
            area: AreaId::new(0),
            adjacent_bonus: Some(AreaId::new(4)),
        }));
        assert_eq!(telegram_adjacent(&s, AreaId::new(0)), vec![AreaId::new(1)]);
        // "Pay the usual cost if another President pawn occupies this adjacent
        // Area" [RB p.16]: +1 per opponent president at the bonus area.
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(1));
        s.players[0].money = 0;
        let moves = placement_moves(&s, &rules);
        assert!(!moves.contains(&Move::PlaceTwoSameArea {
            area: AreaId::new(0),
            adjacent_bonus: Some(AreaId::new(1)),
        }));
        assert!(moves
            .contains(&Move::PlaceTwoSameArea { area: AreaId::new(0), adjacent_bonus: None }));
        s.players[0].money = 1;
        assert!(placement_moves(&s, &rules).contains(&Move::PlaceTwoSameArea {
            area: AreaId::new(0),
            adjacent_bonus: Some(AreaId::new(1)),
        }));
        // Without a 3rd hand assistant the bonus forms vanish (§3.2-2).
        s.players[0].assistants.in_hand = 2;
        s.players[0].money = 10;
        let moves = placement_moves(&s, &rules);
        assert!(moves
            .contains(&Move::PlaceTwoSameArea { area: AreaId::new(0), adjacent_bonus: None }));
        assert!(moves
            .iter()
            .all(|m| !matches!(m, Move::PlaceTwoSameArea { adjacent_bonus: Some(_), .. })));
        // Without Telegram no bonus form exists at all.
        s.players[0].technologies.clear();
        s.players[0].assistants.in_hand = 3;
        assert!(placement_moves(&s, &rules)
            .iter()
            .all(|m| !matches!(m, Move::PlaceTwoSameArea { adjacent_bonus: Some(_), .. })));
    }
}
