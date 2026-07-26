//! §3.1 Additional-action phases — lane E, implemented per spec §3.1.
//!
//! Both windows (`AdditionalActionFirst` / `AdditionalActionSecond`) share this
//! enumeration verbatim: every listed action is "may … as many times as you
//! like and in any order" [RB p.8], so per STATE the legal set is one instance
//! of each currently-satisfiable action plus the always-legal
//! `EndAdditionalPhase`. (System of Stock is a passive turn-start refill, no
//! move [RB p.16; DL 2026-07-17 Task-20c row].)

use std::collections::BTreeSet;

use yokohama_core::{
    AchievementType, AreaId, Country, GameState, Move, OrderCardId, PlayerId, PresidentLocation,
    Rules, TechEffect,
};

use crate::common::{allows_pawn, owns_effect};

/// §3.1: `EndAdditionalPhase` (always) plus any number of `UseMercenary` /
/// `FulfillOrder` / `FulfillAchievement` / `UseUniversity` / `UseSpinningMill`,
/// each gated per §3.1. Shared by both additional-action windows.
pub(crate) fn additional_action_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    let player = state.current_player;
    let p = state.current_player();

    // §3.1-1: proceeding is always the player's right — every listed action is
    // "may" [RB p.8].
    let mut moves = vec![Move::EndAdditionalPhase];

    // §3.1-2 UseMercenary: face-up tokens only (used mercenaries are flipped
    // face-down [RB p.8]) × own-assistant areas without the own President;
    // once per TURN across both windows [RB p.8 "You may only use Foreign
    // Mercenaries once on your turn"]. Opponent presidents at the target are
    // allowed and NEVER paid [RB p.8], so no affordability gate exists here.
    if !state.additional_action_state.mercenary_used {
        let targets = mercenary_targets(state, player);
        for country in Country::ALL {
            if p.mercenaries.available.get(&country).copied().unwrap_or(0) >= 1 {
                for &target_area in &targets {
                    moves.push(Move::UseMercenary { country, target_area });
                }
            }
        }
    }

    // §3.1-3 FulfillOrder: each hand order whose trade-good requirements are
    // all covered by holdings [RB p.8 "Pay the corresponding Trade Goods"].
    for &order_id in &p.order_cards_in_hand {
        if order_fulfillable(state, rules, player, order_id) {
            moves.push(Move::FulfillOrder { order_id });
        }
    }

    // §3.1-4 FulfillAchievement: an assistant is placed from hand onto the
    // card [RB p.8]; each player may fulfill each Achievement only once
    // [RB p.8 Note] — an OPPONENT's claim does not block; nothing is paid
    // [RB p.14 "You do not need to pay or throw away these items"].
    if p.assistants.in_hand >= 1 {
        for card in &state.achievements {
            if !card.claimed_by.contains(&player)
                && achievement_met(state, player, card.achievement_type)
            {
                moves.push(Move::FulfillAchievement { achievement: card.achievement_type });
            }
        }
    }

    // §3.1-5 UseUniversity: "Once per turn, you may gain an Assistant pawn
    // from the Warehouse tile by paying 1 Yen to the supply" [RB p.16];
    // window-only per DL 2026-07-19 §5.3-4 (dispatch enforces the window).
    if owns_effect(state, rules, player, TechEffect::University)
        && !state.additional_action_state.university_used
        && p.money >= 1
        && p.assistants.in_warehouse >= 1
    {
        moves.push(Move::UseUniversity);
    }

    // §3.1-6 UseSpinningMill (card name "Textile Mill"): "Once per turn, you
    // may discard 1 Raw Silk to gain 2 Yen" [RB p.15]; window-only per DL
    // 2026-07-19 §5.3-4.
    if owns_effect(state, rules, player, TechEffect::SpinningMill)
        && !state.additional_action_state.spinning_mill_used
        && p.silk >= 1
    {
        moves.push(Move::UseSpinningMill);
    }

    moves
}

/// True iff the current claim-time requirement for achievement `a` is met by
/// `player` (§3.1 categories A/B/C; Tokens5 counts all owned tokens, §7.1-A2).
pub(crate) fn achievement_met(state: &GameState, player: PlayerId, a: AchievementType) -> bool {
    let p = state.player(player);
    match a {
        // Category A [RB p.14]: demonstrate ownership of trade goods.
        AchievementType::Copper5 => p.copper >= 5,
        AchievementType::Silk6 => p.silk >= 6,
        AchievementType::Tea7 => p.tea >= 7,
        AchievementType::Seafood7 => p.seafood >= 7,

        // Category B [RB p.14–15].
        AchievementType::Money10 => p.money >= 10,
        // "Own a total of 5 5-Power and/or Foreign Mercenary tokens" [RB
        // p.14]: counts ALL owned mercenary tokens, face-up AND face-down —
        // the documented §7.1-A2 count-all reading (end-game scoring says
        // "UNUSED … tokens" [RB p.10] where it means face-up only, so the
        // unqualified "own" here reads as all owned).
        AchievementType::Tokens5 => p.five_power_tokens + p.mercenaries.total() >= 5,
        AchievementType::Orders5 => p.completed_orders.len() >= 5,
        AchievementType::Technologies4 => p.technologies.len() >= 4,

        // Category C [RB p.15]: Shophouses and Trading Houses BOTH count;
        // multiple buildings in one area count once ("If you have built more
        // than 1 Shophouse and/or Trading House in the same Area, it counts
        // for only 1 icon"; DL 2026-07-18 owner-confirmed both-types row).
        AchievementType::Production4 => building_icon_counts(state, player).0 >= 4,
        AchievementType::Commercial4 => building_icon_counts(state, player).1 >= 4,
        AchievementType::Prod3Comm2 => {
            let (prod, comm) = building_icon_counts(state, player);
            prod >= 3 && comm >= 2
        }
        AchievementType::Prod2Comm3 => {
            let (prod, comm) = building_icon_counts(state, player);
            prod >= 2 && comm >= 3
        }
    }
}

/// (production, commercial) area counts carrying ≥1 of `player`'s buildings —
/// one icon per area regardless of how many buildings stand there (§3.1-4c).
/// The Canal is neither production nor commercial (`AreaType` predicates) and
/// can hold no building anyway.
fn building_icon_counts(state: &GameState, player: PlayerId) -> (usize, usize) {
    let p = state.player(player);
    let mut areas: BTreeSet<AreaId> = p.shophouses.built.keys().copied().collect();
    areas.extend(p.trading_houses.built.iter().copied());
    let mut prod = 0;
    let mut comm = 0;
    for a in areas {
        if let Some(cfg) = state.board_layout.areas.get(&a) {
            if cfg.area_type.is_production() {
                prod += 1;
            } else if cfg.area_type.is_commercial() {
                comm += 1;
            }
        }
    }
    (prod, comm)
}

/// True iff order `id` in `player`'s hand has all trade-good requirements met
/// (§3.1; requirements via `rules.orders` — data access only, spec §9-C).
pub(crate) fn order_fulfillable(
    state: &GameState,
    rules: &Rules,
    player: PlayerId,
    id: OrderCardId,
) -> bool {
    let p = state.player(player);
    // An id the registry does not know cannot be validated ⇒ not fulfillable
    // (defensive total-function answer; unreachable for engine-dealt hands).
    rules.orders.get(id).is_some_and(|card| {
        let r = card.requirements;
        u32::from(r.copper) <= p.copper
            && u32::from(r.silk) <= p.silk
            && u32::from(r.tea) <= p.tea
            && u32::from(r.seafood) <= p.seafood
    })
}

/// Areas holding ≥1 own assistant and NOT the player's own President — the
/// legal `UseMercenary` target areas (§3.1-2) [RB p.8: the specified area may
/// not contain your own President; opponent presidents are irrelevant].
/// Ascending `AreaId` (BTreeMap iteration order).
///
/// The area must also be one a pawn may OCCUPY [RB p.14; KB
/// 01-rules-reference §Canal: "No pawns or tokens may be placed; the Canal is
/// unplaceable and unstoppable (transit-only)"] — a mercenary acts at an area
/// holding your own assistants, and no assistant ever stands in the Canal. That
/// is a rule in its own right, checked here, NOT a corollary of the
/// assistant-count test: the two coincide only while the no-pawns rule is taken
/// on faith. This surface cannot inherit the walk's shared area predicate, and
/// the reason is itself a rule — opponent Presidents block a walk [RB p.6] but
/// never block a mercenary [RB p.8] — so the Canal exclusion is stated here.
pub(crate) fn mercenary_targets(state: &GameState, player: PlayerId) -> Vec<AreaId> {
    let p = state.player(player);
    p.assistants
        .on_board
        .iter()
        .filter(|&(_, &n)| n >= 1)
        .map(|(&a, _)| a)
        .filter(|&a| p.president_location != PresidentLocation::OnBoard(a))
        .filter(|&a| allows_pawn(state, a))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::two_player_state;
    use yokohama_core::{
        AreaConfig, AreaType, BoardLayoutBuilder, BuildingSite, Country, FivePowerEffect,
        PresidentLocation, TechCardId, TechEffect, TurnPhase,
    };

    fn p0() -> PlayerId {
        PlayerId::new(0)
    }

    /// Mercenary-ready shape [RB p.8 example, F5]: own assistant at
    /// CopperMine(0), own President at SilkMill(1), one face-up English
    /// mercenary, mercenary not yet used this turn.
    fn merc_ready_state() -> yokohama_core::GameState {
        let mut s = two_player_state();
        s.players[0].assistants.on_board.insert(AreaId::new(0), 1);
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(1));
        s.players[0].mercenaries.available.insert(Country::English, 1);
        s
    }

    /// A 9-area board with 4 production + 5 commercial areas so category-C
    /// thresholds (§3.1-4c) are reachable; same shape as `small_board`.
    fn big_board_state() -> yokohama_core::GameState {
        let mut s = two_player_state();
        let mut b = BoardLayoutBuilder::new();
        let areas = [
            (0u8, AreaType::CopperMine),
            (1, AreaType::SilkMill),
            (2, AreaType::TeaPlantation),
            (3, AreaType::FishingGround),
            (4, AreaType::Bank),
            (5, AreaType::Church),
            (6, AreaType::Customs),
            (7, AreaType::EmploymentAgency),
            (8, AreaType::Chinatown),
        ];
        for (id, ty) in areas {
            let aid = AreaId::new(id);
            b.add_area(aid, AreaConfig { area_type: ty, grid_position: (0, id) });
            b.set_building_site(aid, BuildingSite::new([1, 1, 1, 1], 3));
            b.set_five_power_token(aid, FivePowerEffect::Money(1));
        }
        s.board_layout = b.build().expect("big board validates");
        s
    }

    // --- §3.1-1 EndAdditionalPhase ---

    #[test]
    fn end_phase_always_present_and_alone_when_nothing_else_legal() {
        // Default state: no mercs, no orders, no techs, no achievement met
        // (copper 1 < 5, money 3 < 10, no buildings) ⇒ exactly the always-legal
        // EndAdditionalPhase [RB p.8; §3.1-1].
        let s = two_player_state();
        let moves = additional_action_moves(&s, &Rules::empty_for_tests());
        assert_eq!(moves, vec![Move::EndAdditionalPhase]);
    }

    // --- §3.1-2 UseMercenary [RB p.8, F5] ---

    #[test]
    fn f5_mercenary_legal_with_assistant_and_president_elsewhere() {
        let s = merc_ready_state();
        let moves = additional_action_moves(&s, &Rules::empty_for_tests());
        assert!(moves.contains(&Move::UseMercenary {
            country: Country::English,
            target_area: AreaId::new(0)
        }));
        // No other country is face-up.
        assert!(!moves.iter().any(|m| matches!(
            m,
            Move::UseMercenary { country, .. } if *country != Country::English
        )));
    }

    #[test]
    fn f5_mercenary_illegal_where_own_president_stands() {
        // Assistants at 0 AND 1; President at 1 ⇒ only 0 is a target [RB p.8].
        let mut s = merc_ready_state();
        s.players[0].assistants.on_board.insert(AreaId::new(1), 2);
        let moves = additional_action_moves(&s, &Rules::empty_for_tests());
        assert!(moves.contains(&Move::UseMercenary {
            country: Country::English,
            target_area: AreaId::new(0)
        }));
        assert!(!moves.contains(&Move::UseMercenary {
            country: Country::English,
            target_area: AreaId::new(1)
        }));
    }

    #[test]
    fn f5_opponent_president_at_target_neither_blocks_nor_prices() {
        // [RB p.8]: "It is possible to choose an Area that contains another
        // player's President pawn … you do not need to pay them any money."
        let mut s = merc_ready_state();
        s.players[1].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        s.players[0].money = 0; // no fee could be paid — move must still be legal
        let moves = additional_action_moves(&s, &Rules::empty_for_tests());
        assert!(moves.contains(&Move::UseMercenary {
            country: Country::English,
            target_area: AreaId::new(0)
        }));
    }

    #[test]
    fn mercenary_once_per_turn_gate_spans_both_windows() {
        // [RB p.8]: "You may only use Foreign Mercenaries once on your turn" —
        // the flag is per-turn, so it blocks the SECOND window too.
        let mut s = merc_ready_state();
        s.turn_phase = TurnPhase::AdditionalActionSecond;
        s.additional_action_state.mercenary_used = true;
        let moves = additional_action_moves(&s, &Rules::empty_for_tests());
        assert!(!moves.iter().any(|m| matches!(m, Move::UseMercenary { .. })));
    }

    #[test]
    fn mercenary_requires_face_up_token() {
        // Used mercenaries are flipped face-down [RB p.8] — a used-only token
        // grants no move.
        let mut s = merc_ready_state();
        s.players[0].mercenaries.available.clear();
        s.players[0].mercenaries.used.insert(Country::English, 1);
        let moves = additional_action_moves(&s, &Rules::empty_for_tests());
        assert!(!moves.iter().any(|m| matches!(m, Move::UseMercenary { .. })));
    }

    #[test]
    fn mercenary_targets_helper_filters_president_area() {
        let mut s = two_player_state();
        s.players[0].assistants.on_board.insert(AreaId::new(0), 1);
        s.players[0].assistants.on_board.insert(AreaId::new(2), 3);
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(2));
        assert_eq!(mercenary_targets(&s, p0()), vec![AreaId::new(0)]);
    }

    #[test]
    fn canal_is_never_a_mercenary_target_even_with_own_assistants_there() {
        // [RB p.14; KB 01-rules-reference §Canal: "No pawns or tokens may be
        // placed; the Canal is unplaceable and unstoppable (transit-only)"; the
        // DL 2026-07-21 any-to-any re-ruling re-states it]. A mercenary target
        // is an area holding your own assistant pawns [RB p.8], and the Canal
        // is never a tile a pawn stands on — so it is never a target. This is
        // the AREA-TYPE rule enforced in its own right, NOT a corollary of the
        // own-assistant count: the two only coincide while the no-pawns rule is
        // taken on faith, which is exactly what a rules oracle must not do.
        let mut s = merc_ready_state();
        // small_board: 0 CopperMine, 3 Customs, 4 CANAL. Own assistants at the
        // Canal(4) and at Customs(3); President is elsewhere, at SilkMill(1).
        s.players[0].assistants.on_board.insert(AreaId::new(4), 2);
        s.players[0].assistants.on_board.insert(AreaId::new(3), 1);
        assert_eq!(
            mercenary_targets(&s, p0()),
            vec![AreaId::new(0), AreaId::new(3)],
            "the Canal is never a mercenary target [RB p.14], while its non-Canal \
             siblings still are [RB p.8] — the anti-vacuity control"
        );
        let moves = additional_action_moves(&s, &Rules::empty_for_tests());
        assert!(!moves.contains(&Move::UseMercenary {
            country: Country::English,
            target_area: AreaId::new(4)
        }));
        assert!(moves.contains(&Move::UseMercenary {
            country: Country::English,
            target_area: AreaId::new(3)
        }));

        // An AreaId absent from the layout is no tile at all — no pawn stands
        // there, so it is no target either, however the count field reads.
        let mut ghost = merc_ready_state();
        ghost.players[0].assistants.on_board.insert(AreaId::new(9), 1);
        assert_eq!(mercenary_targets(&ghost, p0()), vec![AreaId::new(0)]);
    }

    // --- §3.1-3 FulfillOrder [RB p.8] ---

    #[test]
    fn order_19_fulfillable_with_one_of_each_good() {
        // 07-order-cards.yaml: order #19 needs 1 seafood + 1 tea + 1 silk +
        // 1 copper; default holdings are exactly 1 of each.
        let rules = Rules::standard();
        let mut s = two_player_state();
        s.players[0].order_cards_in_hand.push(OrderCardId(19));
        assert!(order_fulfillable(&s, &rules, p0(), OrderCardId(19)));
        let moves = additional_action_moves(&s, &rules);
        assert!(moves.contains(&Move::FulfillOrder { order_id: OrderCardId(19) }));

        // Drop copper to 0 ⇒ requirement unmet ⇒ move vanishes.
        s.players[0].copper = 0;
        assert!(!order_fulfillable(&s, &rules, p0(), OrderCardId(19)));
        let moves = additional_action_moves(&s, &rules);
        assert!(!moves.contains(&Move::FulfillOrder { order_id: OrderCardId(19) }));
    }

    #[test]
    fn only_affordable_hand_orders_are_offered() {
        // Order #2 (07-order-cards.yaml) needs 1 seafood + 2 tea; holdings of
        // 1 each satisfy #19 but not #2.
        let rules = Rules::standard();
        let mut s = two_player_state();
        s.players[0].order_cards_in_hand = vec![OrderCardId(19), OrderCardId(2)];
        let moves = additional_action_moves(&s, &rules);
        assert!(moves.contains(&Move::FulfillOrder { order_id: OrderCardId(19) }));
        assert!(!moves.contains(&Move::FulfillOrder { order_id: OrderCardId(2) }));
    }

    // --- §3.1-4 FulfillAchievement [RB p.8/p.14–15, F6] ---

    #[test]
    fn f6_copper5_offered_until_the_player_claims_it() {
        // two_player_state puts Copper5 in play (achievements[0]).
        let mut s = two_player_state();
        s.players[0].copper = 5;
        let rules = Rules::empty_for_tests();
        let claim = Move::FulfillAchievement { achievement: AchievementType::Copper5 };
        assert!(additional_action_moves(&s, &rules).contains(&claim));

        // An OPPONENT's claim does not block this player [RB p.8 Note: each
        // player may fulfill each Achievement only once].
        s.achievements[0].claimed_by.push(PlayerId::new(1));
        assert!(additional_action_moves(&s, &rules).contains(&claim));

        // The player's own claim removes it.
        s.achievements[0].claimed_by.push(p0());
        assert!(!additional_action_moves(&s, &rules).contains(&claim));
    }

    #[test]
    fn achievement_requires_assistant_in_hand() {
        // An assistant is placed from hand onto the card [RB p.8].
        let mut s = two_player_state();
        s.players[0].copper = 5;
        s.players[0].assistants.in_hand = 0;
        let moves = additional_action_moves(&s, &Rules::empty_for_tests());
        assert!(!moves.iter().any(|m| matches!(m, Move::FulfillAchievement { .. })));
    }

    #[test]
    fn achievement_met_category_a_thresholds() {
        // [RB p.14]: ≥5 Copper / ≥6 Raw Silk / ≥7 Tea / ≥7 Seafood.
        let mut s = two_player_state();
        s.players[0].copper = 4;
        assert!(!achievement_met(&s, p0(), AchievementType::Copper5));
        s.players[0].copper = 5;
        assert!(achievement_met(&s, p0(), AchievementType::Copper5));
        s.players[0].silk = 6;
        assert!(achievement_met(&s, p0(), AchievementType::Silk6));
        s.players[0].tea = 7;
        assert!(achievement_met(&s, p0(), AchievementType::Tea7));
        s.players[0].seafood = 6;
        assert!(!achievement_met(&s, p0(), AchievementType::Seafood7));
        s.players[0].seafood = 7;
        assert!(achievement_met(&s, p0(), AchievementType::Seafood7));
    }

    #[test]
    fn achievement_met_category_b_thresholds() {
        // [RB p.14–15]: ≥10 Yen; ≥5 completed Orders; ≥4 Technologies.
        let mut s = two_player_state();
        s.players[0].money = 10;
        assert!(achievement_met(&s, p0(), AchievementType::Money10));
        s.players[0].money = 9;
        assert!(!achievement_met(&s, p0(), AchievementType::Money10));

        s.players[0].completed_orders = (1..=5).map(OrderCardId).collect();
        assert!(achievement_met(&s, p0(), AchievementType::Orders5));
        s.players[0].completed_orders.pop();
        assert!(!achievement_met(&s, p0(), AchievementType::Orders5));

        s.players[0].technologies = (1..=4).map(TechCardId).collect();
        assert!(achievement_met(&s, p0(), AchievementType::Technologies4));
        s.players[0].technologies.pop();
        assert!(!achievement_met(&s, p0(), AchievementType::Technologies4));
    }

    #[test]
    fn tokens5_counts_face_down_mercenaries() {
        // §7.1-A2 documented reading: ALL owned mercenary tokens count,
        // face-up AND face-down (3 5-Power + 2 used mercenaries ⇒ claimable).
        let mut s = two_player_state();
        s.players[0].five_power_tokens = 3;
        s.players[0].mercenaries.used.insert(Country::English, 2);
        assert!(achievement_met(&s, p0(), AchievementType::Tokens5));
        s.players[0].mercenaries.used.insert(Country::English, 1);
        assert!(!achievement_met(&s, p0(), AchievementType::Tokens5));
    }

    #[test]
    fn category_c_counts_both_building_types_one_icon_per_area() {
        // [RB p.15 + DL 2026-07-18 both-types row]: Shophouses AND Trading
        // Houses count; multiple buildings in one area count once.
        let mut s = big_board_state();
        // Shophouses in 3 production areas + one in the Church (commercial).
        s.players[0].shophouses.built.insert(AreaId::new(0), 0);
        s.players[0].shophouses.built.insert(AreaId::new(1), 0);
        s.players[0].shophouses.built.insert(AreaId::new(2), 0);
        s.players[0].shophouses.built.insert(AreaId::new(5), 1);
        // Trading houses at Church (5, same area — counts once) and Customs (6).
        s.players[0].trading_houses.built = vec![AreaId::new(5), AreaId::new(6)];
        assert!(achievement_met(&s, p0(), AchievementType::Prod3Comm2));
        assert!(!achievement_met(&s, p0(), AchievementType::Production4)); // prod = 3
        assert!(!achievement_met(&s, p0(), AchievementType::Commercial4)); // comm = 2
        assert!(!achievement_met(&s, p0(), AchievementType::Prod2Comm3)); // comm = 2 < 3
    }

    #[test]
    fn category_c_trading_houses_alone_satisfy_production4() {
        // Both building types count [DL 2026-07-18]: 4 THs in 4 production areas.
        let mut s = big_board_state();
        s.players[0].trading_houses.built =
            vec![AreaId::new(0), AreaId::new(1), AreaId::new(2), AreaId::new(3)];
        assert!(achievement_met(&s, p0(), AchievementType::Production4));
        assert!(!achievement_met(&s, p0(), AchievementType::Commercial4));
    }

    // --- §3.1-5/6 University / Spinning Mill [RB p.15–16; DL 2026-07-19 §5.3-4] ---

    #[test]
    fn university_gated_on_ownership_flag_money_and_warehouse() {
        let rules = Rules::standard();
        let uni_id = rules
            .techs
            .iter()
            .find(|t| t.effect == TechEffect::University)
            .map(|t| t.id)
            .expect("standard data carries University");
        let mut s = two_player_state();

        // Not owned ⇒ absent.
        assert!(!additional_action_moves(&s, &rules).contains(&Move::UseUniversity));

        s.players[0].technologies.push(uni_id);
        assert!(additional_action_moves(&s, &rules).contains(&Move::UseUniversity));

        // Once per turn [RB p.16].
        s.additional_action_state.university_used = true;
        assert!(!additional_action_moves(&s, &rules).contains(&Move::UseUniversity));
        s.additional_action_state.university_used = false;

        // Pay 1 Yen to the supply [RB p.16] ⇒ money 0 blocks.
        s.players[0].money = 0;
        assert!(!additional_action_moves(&s, &rules).contains(&Move::UseUniversity));
        s.players[0].money = 3;

        // Gain from the Warehouse tile [RB p.16] ⇒ empty warehouse blocks.
        s.players[0].assistants.in_warehouse = 0;
        assert!(!additional_action_moves(&s, &rules).contains(&Move::UseUniversity));
    }

    #[test]
    fn spinning_mill_gated_on_ownership_flag_and_silk() {
        let rules = Rules::standard();
        let mill_id = rules
            .techs
            .iter()
            .find(|t| t.effect == TechEffect::SpinningMill)
            .map(|t| t.id)
            .expect("standard data carries Spinning Mill");
        let mut s = two_player_state();

        assert!(!additional_action_moves(&s, &rules).contains(&Move::UseSpinningMill));

        s.players[0].technologies.push(mill_id);
        assert!(additional_action_moves(&s, &rules).contains(&Move::UseSpinningMill));

        // Once per turn [RB p.15].
        s.additional_action_state.spinning_mill_used = true;
        assert!(!additional_action_moves(&s, &rules).contains(&Move::UseSpinningMill));
        s.additional_action_state.spinning_mill_used = false;

        // Discard 1 Raw Silk [RB p.15] ⇒ silk 0 blocks.
        s.players[0].silk = 0;
        assert!(!additional_action_moves(&s, &rules).contains(&Move::UseSpinningMill));
    }

    // --- dispatch integration (§1.4 table) ---

    #[test]
    fn oracle_moves_serves_both_additional_windows() {
        let mut s = merc_ready_state();
        s.turn_phase = TurnPhase::AdditionalActionFirst;
        let rules = Rules::empty_for_tests();
        let om = crate::oracle_moves(&s, &rules);
        assert!(om.exact.contains(&Move::EndAdditionalPhase));
        assert!(om.exact.contains(&Move::UseMercenary {
            country: Country::English,
            target_area: AreaId::new(0)
        }));
        assert!(om.movement.is_empty());

        s.turn_phase = TurnPhase::AdditionalActionSecond;
        let om2 = crate::oracle_moves(&s, &rules);
        assert_eq!(om.exact, om2.exact, "both windows enumerate identically (§3.1)");
    }
}
