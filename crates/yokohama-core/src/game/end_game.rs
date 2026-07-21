//! End game trigger detection and final round handling.

use crate::types::*;

/// Check all end game triggers and update state if triggered.
pub fn check_end_game_triggers(state: &mut GameState) {
    if state.end_game_triggered {
        return; // Already triggered
    }

    // 1. Trading Houses: any player built all 4
    for p in &state.players {
        if p.trading_houses.total_built() >= 4 {
            trigger_end_game(state, EndGameTrigger::TradingHousesComplete(p.id), p.id);
            return;
        }
    }

    // 2. Shophouses: any player built all 8
    for p in &state.players {
        if p.shophouses.total_built() >= 8 {
            trigger_end_game(state, EndGameTrigger::ShophousesComplete(p.id), p.id);
            return;
        }
    }

    // 3. Order deck empty AND a Port slot is empty
    if state.order_deck.is_empty() {
        let port_has_empty_slot = check_port_empty_slot(state);
        if port_has_empty_slot {
            trigger_end_game(state, EndGameTrigger::OrderDeckEmpty, state.current_player);
            return;
        }
    }

    // 4. Church track: reached threshold
    let church_threshold = state.config.track_trigger();
    if state.church_track.total_assistants() >= church_threshold {
        trigger_end_game(state, EndGameTrigger::ChurchTrackFull, state.current_player);
        return;
    }

    // 5. Customs track: reached threshold
    let customs_threshold = state.config.track_trigger();
    if state.customs_track.total_assistants() >= customs_threshold {
        trigger_end_game(state, EndGameTrigger::CustomsTrackFull, state.current_player);
    }
}

fn trigger_end_game(state: &mut GameState, reason: EndGameTrigger, trigger_player: PlayerId) {
    state.end_game_triggered = true;
    state.trigger_reason = Some(reason);
    state.trigger_player = Some(trigger_player);
}

fn check_port_empty_slot(state: &GameState) -> bool {
    // Check Port A
    if state.port_state.port_a.has_empty_slot() {
        return true;
    }

    // Check Port B if exists
    if let Some(ref port_b) = state.port_state.port_b {
        if port_b.has_empty_slot() {
            return true;
        }
    }

    false
}

/// Determine if the game is completely over (final round finished).
///
/// Delegates to [`GameState::is_game_over`] — the SINGLE definition (Stage-6
/// audit finding: this free fn and the method had drifted; the method
/// additionally requires `trigger_player` to be set, which
/// `check_end_game_triggers` guarantees on every engine-produced state, so
/// legal-play behavior is identical and only tampered states differ — they
/// now also trip the trigger-coherence invariant).
pub fn is_game_over(state: &GameState) -> bool {
    state.is_game_over()
}

/// Advance to final round after completing current round.
#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn test_track_trigger_by_player_count() {
        let config_2p = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));
        assert_eq!(config_2p.track_trigger(), 4);

        let config_3p = GameConfig::new(
            3,
            vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow],
            PlayerId::new(0),
        );
        assert_eq!(config_3p.track_trigger(), 5);
    }

    #[test]
    fn test_order_deck_empty_trigger() {
        let mut state = create_test_state();

        // Deck has cards, all 6 port slots full - should NOT trigger
        state.order_deck = vec![OrderCardId(1)];
        state.port_state.port_a.slots =
            [1, 2, 3, 4, 5, 6].map(|id| Some(OrderCardId(id)));

        check_end_game_triggers(&mut state);
        assert!(!state.end_game_triggered);

        // Deck empty but all slots full - should NOT trigger
        state.order_deck = vec![];
        check_end_game_triggers(&mut state);
        assert!(!state.end_game_triggered);

        // Deck empty AND a slot empty (post-refill) - should trigger
        state.port_state.port_a.slots[5] = None; // Empty slot after failed refill
        check_end_game_triggers(&mut state);
        assert!(state.end_game_triggered);
        assert!(matches!(state.trigger_reason, Some(EndGameTrigger::OrderDeckEmpty)));
    }

    #[test]
    fn test_order_deck_empty_with_port_b() {
        let mut state = create_test_state();
        state.port_state.port_b = Some(PortBoard::new()); // Add Port B

        // Deck empty, Port A full, Port B has empty slots
        state.order_deck = vec![];
        state.port_state.port_a.slots =
            [1, 2, 3, 4, 5, 6].map(|id| Some(OrderCardId(id)));
        // Port B starts empty (new())

        check_end_game_triggers(&mut state);
        assert!(state.end_game_triggered);
    }

    #[test]
    fn test_trading_house_complete_trigger() {
        let mut state = create_test_state();

        // Player builds 4 trading houses
        state.players[0].trading_houses.built = vec![
            AreaId::new(0),
            AreaId::new(1),
            AreaId::new(2),
            AreaId::new(3),
        ];
        state.players[0].trading_houses.in_warehouse = 0;

        check_end_game_triggers(&mut state);
        assert!(state.end_game_triggered);
        assert!(matches!(
            state.trigger_reason,
            Some(EndGameTrigger::TradingHousesComplete(p)) if p == PlayerId::new(0)
        ));
    }

    /// Two-sided near-threshold pin (Phase-2.9 review addendum, Tier 1):
    /// the Trading-House end-game trigger fires at EXACTLY 4, not 3. The
    /// existing `test_trading_house_complete_trigger` only checks the
    /// positive side at 4, so mutating `>= 4` to `>= 3` survived. Here 3
    /// built must NOT trigger, and the 4th build -- through the public apply
    /// path (`apply_move_unchecked`, which runs `check_end_game_triggers`) --
    /// must. Mirrors `tests/end_game_timing.rs`'s two-sided church pattern.
    #[test]
    fn trading_house_trigger_fires_at_4_not_3() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        state.board_layout.areas.insert(
            area,
            AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) },
        );
        state
            .board_layout
            .building_sites
            .insert(area, BuildingSite::new([1, 2, 3, 4], 7));
        state.turn_phase = TurnPhase::Building;
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(5);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        // Three already built (distinct areas), one in hand to build the 4th.
        state.players[0].trading_houses.built =
            vec![AreaId::new(10), AreaId::new(11), AreaId::new(12)];
        state.players[0].trading_houses.in_hand = 1;
        state.players[0].trading_houses.in_warehouse = 0;
        state.order_deck = vec![OrderCardId(99)]; // avoid the OrderDeckEmpty trigger

        // 3 built must NOT trigger (kills the `>= 4` -> `>= 3` mutation).
        let mut probe = state.clone();
        check_end_game_triggers(&mut probe);
        assert!(!probe.end_game_triggered, "3 trading houses must not trigger end game");

        let rules = crate::game::Rules::empty_for_tests();
        let next = crate::game::apply_move_unchecked(&state, &Move::BuildTradingHouse, &rules)
            .expect("the 4th trading house builds");
        assert_eq!(next.players[0].trading_houses.total_built(), 4);
        assert!(next.end_game_triggered, "the 4th trading house must trigger end game");
        assert!(matches!(
            next.trigger_reason,
            Some(EndGameTrigger::TradingHousesComplete(p)) if p == PlayerId::new(0)
        ));
    }

    /// Two-sided near-threshold pin (Phase-2.9 review addendum, Tier 1): the
    /// Shophouse end-game trigger fires at EXACTLY 8, not 7 -- mutating
    /// `>= 8` to `>= 7` survived the one-sided existing test. 7 built must
    /// NOT trigger; the 8th build through the public apply path must.
    #[test]
    fn shophouse_trigger_fires_at_8_not_7() {
        let mut state = create_test_state();
        let area = AreaId::new(0);
        state.board_layout.areas.insert(
            area,
            AreaConfig { area_type: AreaType::Bank, grid_position: (0, 0) },
        );
        state
            .board_layout
            .building_sites
            .insert(area, BuildingSite::new([1, 2, 3, 4], 7));
        state.turn_phase = TurnPhase::Building;
        state.current_player = PlayerId::new(0);
        state.current_action_power = Some(5);
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        // Seven already built (distinct areas), one in hand to build the 8th.
        for i in 10..17u8 {
            state.players[0].shophouses.built.insert(AreaId::new(i), 0);
        }
        state.players[0].shophouses.in_hand = 1;
        state.order_deck = vec![OrderCardId(99)]; // avoid the OrderDeckEmpty trigger

        // 7 built must NOT trigger (kills the `>= 8` -> `>= 7` mutation).
        let mut probe = state.clone();
        check_end_game_triggers(&mut probe);
        assert!(!probe.end_game_triggered, "7 shophouses must not trigger end game");

        let rules = crate::game::Rules::empty_for_tests();
        let next =
            crate::game::apply_move_unchecked(&state, &Move::BuildShophouse { slot: 0 }, &rules)
                .expect("the 8th shophouse builds");
        assert_eq!(next.players[0].shophouses.total_built(), 8);
        assert!(next.end_game_triggered, "the 8th shophouse must trigger end game");
        assert!(matches!(
            next.trigger_reason,
            Some(EndGameTrigger::ShophousesComplete(p)) if p == PlayerId::new(0)
        ));
    }

    #[test]
    fn test_shophouse_complete_trigger() {
        let mut state = create_test_state();

        // Player builds 8 shophouses
        for i in 0..8 {
            state.players[0].shophouses.built.insert(AreaId::new(i), 0);
        }
        state.players[0].shophouses.in_hand = 0;

        check_end_game_triggers(&mut state);
        assert!(state.end_game_triggered);
        assert!(matches!(
            state.trigger_reason,
            Some(EndGameTrigger::ShophousesComplete(p)) if p == PlayerId::new(0)
        ));
    }
}
