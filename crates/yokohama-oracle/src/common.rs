//! Shared rules-arithmetic helpers (spec §5, Author A).
//!
//! Every helper is authored from the rulebook + decision log (§1.4 power table,
//! §3.4 assistant-source rule) — no engine logic file was consulted.

use yokohama_core::{AreaId, GameState, PlayerId, PresidentLocation, Rules, TechEffect};

/// §1.4 acting area: the mercenary target during a mercenary sub-turn, else the
/// current player's on-board President area (`None` if the President is in hand
/// and no mercenary sub-turn is active).
pub(crate) fn acting_area(state: &GameState) -> Option<AreaId> {
    if state.is_mercenary_action {
        state.mercenary_target_area
    } else {
        match state.current_player().president_location {
            PresidentLocation::OnBoard(a) => Some(a),
            PresidentLocation::InHand => None,
        }
    }
}

/// §1.4 action power for `player` in `area`, capped at 5: +1 for the player's
/// own President in the area (never counted during a mercenary sub-turn), +1
/// per own assistant there, +1 for an own Shophouse, +1 for an own Trading
/// House, +1 if the Station token is there (Station powers ALL players).
/// Shophouses/Trading Houses never count as assistants for movement (RB p.6),
/// only for power here.
pub(crate) fn action_power(state: &GameState, player: PlayerId, area: AreaId) -> u8 {
    let p = state.player(player);
    let mut power: u8 = 0;
    // +1 for own President in the area — never during a mercenary sub-turn
    // (the target may not contain the actor's own President, RB p.8).
    let own_president_here = !state.is_mercenary_action
        && matches!(p.president_location, PresidentLocation::OnBoard(a) if a == area);
    if own_president_here {
        power += 1;
    }
    power += p.assistants.in_area(area);
    if p.shophouses.built.contains_key(&area) {
        power += 1;
    }
    if p.trading_houses.built.contains(&area) {
        power += 1;
    }
    if state.station_location == Some(area) {
        power += 1;
    }
    power.min(5)
}

/// §1.4/§3 opponent presidents in `area`: the ids of every OTHER player whose
/// President is `OnBoard(area)` (each such president is one entry).
#[allow(dead_code)] // consumed by lanes B (movement/placement fees) once landed.
pub(crate) fn opponent_presidents_in(
    state: &GameState,
    mover: PlayerId,
    area: AreaId,
) -> Vec<PlayerId> {
    state
        .players
        .iter()
        .filter(|p| p.id != mover)
        .filter(|p| p.president_location == PresidentLocation::OnBoard(area))
        .map(|p| p.id)
        .collect()
}

/// True iff `player` owns a Technology whose effect is `effect` (via the
/// `rules.techs` id lookup — data access only, spec §9-C).
#[allow(dead_code)] // consumed by lanes B/C/E (tech-gated moves) once landed.
pub(crate) fn owns_effect(
    state: &GameState,
    rules: &Rules,
    player: PlayerId,
    effect: TechEffect,
) -> bool {
    state
        .player(player)
        .technologies
        .iter()
        .any(|id| rules.techs.get(*id).is_some_and(|t| t.effect == effect))
}

/// §3.4 Church/Customs assistant source: the current player has an assistant in
/// hand OR in the acting `area` (decision log 2026-07-18 I3).
#[allow(dead_code)] // consumed by lane D (church/customs) once landed.
pub(crate) fn track_assistant_available(state: &GameState, area: AreaId) -> bool {
    let p = state.current_player();
    p.assistants.in_hand >= 1 || p.assistants.in_area(area) >= 1
}

/// Non-Canal areas of the board, ascending `AreaId` (every area that
/// `allows_placement`).
pub(crate) fn placeable_areas(state: &GameState) -> Vec<AreaId> {
    state
        .board_layout
        .areas
        .iter()
        .filter(|(_, cfg)| cfg.area_type.allows_placement())
        .map(|(id, _)| *id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::two_player_state;
    use yokohama_core::{PresidentLocation, Rules, TechEffect};

    #[test]
    fn acting_area_is_president_area_normally() {
        let mut s = two_player_state();
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(1));
        assert_eq!(acting_area(&s), Some(AreaId::new(1)));
    }

    #[test]
    fn acting_area_is_none_when_president_in_hand() {
        let s = two_player_state();
        assert_eq!(acting_area(&s), None);
    }

    #[test]
    fn acting_area_is_mercenary_target_during_sub_turn() {
        let mut s = two_player_state();
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        s.is_mercenary_action = true;
        s.mercenary_target_area = Some(AreaId::new(2));
        assert_eq!(acting_area(&s), Some(AreaId::new(2)));
    }

    #[test]
    fn action_power_tallies_president_assistants_shophouse() {
        // F3 [RB p.7]: own president + 3 assistants + 1 shophouse ⇒ 5.
        let mut s = two_player_state();
        let a = AreaId::new(1);
        s.players[0].president_location = PresidentLocation::OnBoard(a);
        s.players[0].assistants.on_board.insert(a, 3);
        s.players[0].shophouses.built.insert(a, 0);
        assert_eq!(action_power(&s, PlayerId::new(0), a), 5);
    }

    #[test]
    fn action_power_caps_at_five() {
        let mut s = two_player_state();
        let a = AreaId::new(1);
        s.players[0].president_location = PresidentLocation::OnBoard(a);
        s.players[0].assistants.on_board.insert(a, 4);
        s.players[0].shophouses.built.insert(a, 0);
        s.players[0].trading_houses.built.push(a);
        s.station_location = Some(a);
        // 1 + 4 + 1 + 1 + 1 = 8 ⇒ capped 5.
        assert_eq!(action_power(&s, PlayerId::new(0), a), 5);
    }

    #[test]
    fn action_power_excludes_president_during_mercenary() {
        let mut s = two_player_state();
        let a = AreaId::new(1);
        s.players[0].president_location = PresidentLocation::OnBoard(a);
        s.players[0].assistants.on_board.insert(a, 2);
        s.is_mercenary_action = true;
        s.mercenary_target_area = Some(a);
        assert_eq!(action_power(&s, PlayerId::new(0), a), 2);
    }

    #[test]
    fn action_power_station_powers_all_players() {
        let mut s = two_player_state();
        let a = AreaId::new(1);
        s.station_location = Some(a);
        // Player 1 has no presence at the area but the Station gives +1.
        assert_eq!(action_power(&s, PlayerId::new(1), a), 1);
    }

    #[test]
    fn opponent_presidents_excludes_mover_and_finds_others() {
        let mut s = two_player_state();
        let a = AreaId::new(2);
        s.players[0].president_location = PresidentLocation::OnBoard(a);
        s.players[1].president_location = PresidentLocation::OnBoard(a);
        assert_eq!(opponent_presidents_in(&s, PlayerId::new(0), a), vec![PlayerId::new(1)]);
        assert!(opponent_presidents_in(&s, PlayerId::new(0), AreaId::new(0)).is_empty());
    }

    #[test]
    fn owns_effect_matches_held_tech_only() {
        let rules = Rules::standard();
        let tram_id = rules
            .techs
            .iter()
            .find(|t| t.effect == TechEffect::Tram)
            .map(|t| t.id)
            .expect("standard data carries a Tram tech");
        let mut s = two_player_state();
        s.players[0].technologies.push(tram_id);
        assert!(owns_effect(&s, &rules, PlayerId::new(0), TechEffect::Tram));
        assert!(!owns_effect(&s, &rules, PlayerId::new(0), TechEffect::Stagecoach));
        assert!(!owns_effect(&s, &rules, PlayerId::new(1), TechEffect::Tram));
    }

    #[test]
    fn track_assistant_available_from_hand_or_area() {
        let mut s = two_player_state();
        let a = AreaId::new(2);
        assert!(track_assistant_available(&s, a)); // 8 in hand by default
        s.players[0].assistants.in_hand = 0;
        s.players[0].assistants.on_board.insert(a, 1);
        assert!(track_assistant_available(&s, a));
        s.players[0].assistants.on_board.remove(&a);
        assert!(!track_assistant_available(&s, a));
    }

    #[test]
    fn placeable_areas_excludes_canal_ascending() {
        let s = two_player_state();
        assert_eq!(
            placeable_areas(&s),
            vec![AreaId::new(0), AreaId::new(1), AreaId::new(2), AreaId::new(3)],
        );
    }
}
