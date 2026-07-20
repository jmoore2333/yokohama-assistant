//! §3.5 5-Power bonus + §3.6 Building — lane E, implemented per spec §3.5/§3.6.
//!
//! Both steps are optional [RB p.7], so their Skip move is always legal —
//! including when the engine presents the phase at ineligible power (an
//! unpinned reachability question, spec §7.2-5: the skip-only answer is
//! correct under the rules either way). The acting area is the mercenary
//! target during a sub-turn (§3.9), else the President's area (§1.4).

use yokohama_core::{GameState, Move, Rules};

use crate::common::acting_area;

/// §3.5: `Take5PowerToken` iff the acting power is exactly 5 and the acting
/// area's token is still present, plus the always-legal `Skip5PowerBonus`.
pub(crate) fn five_power_moves(state: &GameState) -> Vec<Move> {
    let mut moves = Vec::new();
    if take_five_power_available(state) {
        moves.push(Move::Take5PowerToken);
    }
    // The step is optional [RB p.7] ⇒ Skip is always legal in this phase.
    moves.push(Move::Skip5PowerBonus);
    moves
}

/// The bonus requires a 5-Power action in step 3 [RB p.7] AND "the 5-Power
/// token in the corresponding Area is still present" [RB p.7] — it may have
/// been claimed in an earlier turn (`Some(None)` = claimed; no map entry =
/// the Canal, which never carries a token).
fn take_five_power_available(state: &GameState) -> bool {
    if state.current_action_power != Some(5) {
        return false;
    }
    let Some(area) = acting_area(state) else {
        return false;
    };
    matches!(state.board_layout.five_power_tokens.get(&area), Some(Some(_)))
}

/// §3.6: at 4+ acting power, one `BuildShophouse { slot }` per empty
/// site slot (no own shophouse already there) and `BuildTradingHouse` when the
/// site's trading-house space is empty and a token is in hand; plus the
/// always-legal `SkipBuilding`.
pub(crate) fn building_moves(state: &GameState, rules: &Rules) -> Vec<Move> {
    // Site occupancy lives in `state.board_layout.building_sites` (per-game
    // mutable state); `rules` is kept per the spec §5 signature but static
    // site data is not needed for legality.
    let _ = rules;
    let mut moves = build_moves_at_power(state);
    // The step is optional [RB p.7] ⇒ Skip is always legal in this phase.
    moves.push(Move::SkipBuilding);
    moves
}

/// The non-Skip building family: requires a 4+ Power action in step 3 — the
/// gate applies to BOTH building types, there is no separate Power-5 tier
/// [RB p.7; DL 2026-07-17 final-review C3; DL 2026-07-18 C3 addendum] — and
/// ONE building of either type is built (choosing WHICH is this enumeration;
/// "one per turn" is apply-side phase advance).
fn build_moves_at_power(state: &GameState) -> Vec<Move> {
    let mut moves = Vec::new();
    if state.current_action_power.is_none_or(|p| p < 4) {
        return moves;
    }
    let Some(area) = acting_area(state) else {
        return moves;
    };
    // The Canal has no site [types::board]; defensive empty for it.
    let Some(site) = state.board_layout.building_sites.get(&area) else {
        return moves;
    };
    let p = state.current_player();

    // BuildShophouse: one move PER EMPTY SLOT — the player chooses the space
    // and gains that space's printed reward, so every empty slot is a distinct
    // legal move [RB p.7 "Place a Shophouse token from your hand onto an empty
    // Shophouse space … Gain the benefits stated on the space"]. Blocked
    // entirely if the player already has a Shophouse on this site [RB p.7
    // "only if they do not have a Shophouse already on the Building Site
    // card"]. Token must be in hand [RB p.7].
    let owns_shophouse_here = site.shophouses_built.contains(&Some(p.id));
    if p.shophouses.in_hand >= 1 && !owns_shophouse_here {
        for (slot, owner) in site.shophouses_built.iter().enumerate() {
            if owner.is_none() {
                moves.push(Move::BuildShophouse { slot: slot as u8 });
            }
        }
    }

    // BuildTradingHouse: token in hand (staged warehouse→hand at the
    // Employment Agency; the build itself is FREE [RB p.7; DL 2026-07-17
    // staging row]) and the site's single Trading House space empty [RB p.7].
    if p.trading_houses.in_hand >= 1 && site.trading_house_owner.is_none() {
        moves.push(Move::BuildTradingHouse);
    }

    moves
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::two_player_state;
    use yokohama_core::{AreaId, PlayerId, PresidentLocation, TurnPhase};

    /// Actor at the Church (area 2 of `small_board`, all 4 site slots empty,
    /// trading-house space empty) with the given recorded action power.
    fn church_actor(power: Option<u8>) -> GameState {
        let mut s = two_player_state();
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(2));
        s.current_action_power = power;
        s
    }

    // --- §3.5 5-Power bonus [RB p.7] ---

    #[test]
    fn skip_five_power_always_legal() {
        let mut s = church_actor(None);
        s.turn_phase = TurnPhase::FivePowerBonus;
        assert_eq!(five_power_moves(&s), vec![Move::Skip5PowerBonus]);
    }

    #[test]
    fn take_token_requires_power_exactly_five() {
        // [RB p.7]: the bonus requires a 5-Power action; 4 is not enough.
        let mut s = church_actor(Some(5));
        s.turn_phase = TurnPhase::FivePowerBonus;
        assert!(five_power_moves(&s).contains(&Move::Take5PowerToken));
        s.current_action_power = Some(4);
        assert!(!five_power_moves(&s).contains(&Move::Take5PowerToken));
        s.current_action_power = None;
        assert!(!five_power_moves(&s).contains(&Move::Take5PowerToken));
    }

    #[test]
    fn take_token_requires_unclaimed_token() {
        // [RB p.7]: "If the 5-Power token in the corresponding Area is still
        // present" — a claimed token (None) yields Skip only.
        let mut s = church_actor(Some(5));
        s.turn_phase = TurnPhase::FivePowerBonus;
        s.board_layout.five_power_tokens.insert(AreaId::new(2), None);
        assert_eq!(five_power_moves(&s), vec![Move::Skip5PowerBonus]);
    }

    #[test]
    fn five_power_uses_mercenary_target_as_acting_area() {
        // §3.9: during a mercenary sub-turn the acting area is the target.
        let mut s = two_player_state();
        s.turn_phase = TurnPhase::FivePowerBonus;
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(1));
        s.is_mercenary_action = true;
        s.mercenary_target_area = Some(AreaId::new(3));
        s.current_action_power = Some(5);
        // Token at target area 3 present ⇒ take offered.
        assert!(five_power_moves(&s).contains(&Move::Take5PowerToken));
        // Claim the TARGET's token (president area 1 keeps its token) ⇒ gone.
        s.board_layout.five_power_tokens.insert(AreaId::new(3), None);
        assert!(!five_power_moves(&s).contains(&Move::Take5PowerToken));
    }

    // --- §3.6 Building [RB p.7] ---

    #[test]
    fn f4_seiji_power_four_but_no_tokens_in_hand_skip_only() {
        // F4 [RB p.7]: power 4 at the Church, zero shophouse and trading-house
        // tokens in hand ⇒ Building = {SkipBuilding} only.
        let mut s = church_actor(Some(4));
        s.turn_phase = TurnPhase::Building;
        s.players[0].shophouses.in_hand = 0;
        s.players[0].trading_houses.in_hand = 0;
        assert_eq!(building_moves(&s, &Rules::empty_for_tests()), vec![Move::SkipBuilding]);
    }

    #[test]
    fn one_shophouse_move_per_empty_slot() {
        // [RB p.7]: the player chooses the space and gains ITS printed reward,
        // so every empty slot is a distinct legal move (§3.6).
        let mut s = church_actor(Some(4));
        s.turn_phase = TurnPhase::Building;
        let moves = building_moves(&s, &Rules::empty_for_tests());
        for slot in 0..4u8 {
            assert!(moves.contains(&Move::BuildShophouse { slot }), "slot {slot} missing");
        }
        // An opponent-occupied slot drops out; the others stay.
        s.board_layout
            .building_sites
            .get_mut(&AreaId::new(2))
            .expect("church site")
            .shophouses_built[1] = Some(PlayerId::new(1));
        let moves = building_moves(&s, &Rules::empty_for_tests());
        assert!(!moves.contains(&Move::BuildShophouse { slot: 1 }));
        for slot in [0u8, 2, 3] {
            assert!(moves.contains(&Move::BuildShophouse { slot }), "slot {slot} missing");
        }
    }

    #[test]
    fn own_shophouse_on_site_blocks_every_slot() {
        // [RB p.7]: "only if they do not have a Shophouse already on the
        // Building Site card".
        let mut s = church_actor(Some(4));
        s.turn_phase = TurnPhase::Building;
        s.board_layout
            .building_sites
            .get_mut(&AreaId::new(2))
            .expect("church site")
            .shophouses_built[3] = Some(PlayerId::new(0));
        let moves = building_moves(&s, &Rules::empty_for_tests());
        assert!(!moves.iter().any(|m| matches!(m, Move::BuildShophouse { .. })));
    }

    #[test]
    fn shophouse_requires_token_in_hand() {
        let mut s = church_actor(Some(4));
        s.turn_phase = TurnPhase::Building;
        s.players[0].shophouses.in_hand = 0;
        let moves = building_moves(&s, &Rules::empty_for_tests());
        assert!(!moves.iter().any(|m| matches!(m, Move::BuildShophouse { .. })));
    }

    #[test]
    fn trading_house_free_from_hand_at_power_four() {
        // [DL 2026-07-18 C3 addendum]: the 4+ gate applies to BOTH building
        // types; the build itself is FREE from hand [RB p.7], so money 0 does
        // not block it.
        let mut s = church_actor(Some(4));
        s.turn_phase = TurnPhase::Building;
        s.players[0].trading_houses.in_hand = 1;
        s.players[0].money = 0;
        let moves = building_moves(&s, &Rules::empty_for_tests());
        assert!(moves.contains(&Move::BuildTradingHouse));
        assert!(moves.contains(&Move::SkipBuilding));
    }

    #[test]
    fn trading_house_needs_token_and_empty_space() {
        let mut s = church_actor(Some(4));
        s.turn_phase = TurnPhase::Building;
        // No token in hand (default 0) ⇒ absent.
        assert!(!building_moves(&s, &Rules::empty_for_tests()).contains(&Move::BuildTradingHouse));
        // Token in hand but TH space occupied ⇒ absent [RB p.7].
        s.players[0].trading_houses.in_hand = 1;
        s.board_layout
            .building_sites
            .get_mut(&AreaId::new(2))
            .expect("church site")
            .trading_house_owner = Some(PlayerId::new(1));
        assert!(!building_moves(&s, &Rules::empty_for_tests()).contains(&Move::BuildTradingHouse));
    }

    #[test]
    fn power_below_four_skip_only() {
        // [RB p.7]: Building requires a 4+ Power action in step 3.
        let mut s = church_actor(Some(3));
        s.turn_phase = TurnPhase::Building;
        s.players[0].trading_houses.in_hand = 1; // tokens available, still gated
        assert_eq!(building_moves(&s, &Rules::empty_for_tests()), vec![Move::SkipBuilding]);
        s.current_action_power = None;
        assert_eq!(building_moves(&s, &Rules::empty_for_tests()), vec![Move::SkipBuilding]);
    }

    #[test]
    fn building_uses_mercenary_target_as_acting_area() {
        // §3.9: sub-turn builds happen at the mercenary target area.
        let mut s = two_player_state();
        s.turn_phase = TurnPhase::Building;
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(1));
        s.is_mercenary_action = true;
        s.mercenary_target_area = Some(AreaId::new(3));
        s.current_action_power = Some(4);
        // Occupy every slot at the PRESIDENT's area — must not matter.
        let pres_site =
            s.board_layout.building_sites.get_mut(&AreaId::new(1)).expect("silk mill site");
        pres_site.shophouses_built = [Some(PlayerId::new(1)); 4];
        let moves = building_moves(&s, &Rules::empty_for_tests());
        for slot in 0..4u8 {
            assert!(moves.contains(&Move::BuildShophouse { slot }), "slot {slot} missing");
        }
    }

    // --- dispatch integration (§1.4 table) ---

    #[test]
    fn oracle_moves_dispatches_five_power_and_building_phases() {
        let rules = Rules::empty_for_tests();
        let mut s = church_actor(Some(5));
        s.turn_phase = TurnPhase::FivePowerBonus;
        let om = crate::oracle_moves(&s, &rules);
        assert!(om.exact.contains(&Move::Take5PowerToken));
        assert!(om.exact.contains(&Move::Skip5PowerBonus));
        assert!(om.movement.is_empty());

        s.turn_phase = TurnPhase::Building;
        let om = crate::oracle_moves(&s, &rules);
        assert!(om.exact.contains(&Move::BuildShophouse { slot: 0 }));
        assert!(om.exact.contains(&Move::SkipBuilding));
    }
}
