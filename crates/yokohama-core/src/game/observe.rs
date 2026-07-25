//! `observe`: derive one seat's `PlayerView` from a `GameState` (Phase 3.0).
//!
//! # The destructure rule (load-bearing, do not "simplify")
//!
//! Every state struct is destructured EXHAUSTIVELY here -- every field named,
//! never `..`, never functional-update syntax (`GameState { x, ..other }`).
//! That is what makes adding a state field a COMPILE ERROR
//! (`error[E0027]: pattern does not mention field`) instead of a silent
//! information leak. Functional-update syntax passes new fields through
//! unclassified, which is the exact failure this whole sub-phase exists to
//! prevent. A field that is deliberately dropped is bound and commented.
//!
//! The five hidden members (spec §1, verified 2026-07-25):
//!   1. `config.setup_seed`  -- regenerates deck, box, board AND every deal
//!   2. `order_deck` order   -- size only survives
//!   3. `tech_deck` order    -- size only survives
//!   4. `orders_removed` contents -- derived size only
//!   5. each opponent's UNFULFILLED starting order, at both storage sites

use crate::types::{
    GameConfig, GameState, HandCard, PlayerId, PlayerState, PlayerView, ViewConfig, ViewError,
    VisiblePlayer,
};

pub fn observe(state: &GameState, seat: PlayerId) -> Result<PlayerView, ViewError> {
    if seat.is_dummy() {
        return Err(ViewError::DummySeat);
    }
    let seat_idx = seat.0 as usize;
    if seat_idx >= state.players.len() {
        return Err(ViewError::SeatOutOfRange(seat));
    }
    if state.players[seat_idx].starting_order_hidden {
        return Err(ViewError::ViewerDealUnrecorded(seat));
    }

    // EXHAUSTIVE -- see the module docs before touching this pattern.
    let GameState {
        config,
        board_layout,
        players,
        current_player,
        turn_phase,
        turn_number,
        additional_action_state,
        current_action_power,
        is_mercenary_action,
        mercenary_target_area,
        mercenary_return_phase,
        president_start_area,
        port_state,
        lab_state,
        order_deck,
        tech_deck,
        orders_removed,
        church_track,
        customs_track,
        achievements,
        mercenary_supply,
        station_location,
        end_game_triggered,
        trigger_reason,
        final_round,
        trigger_player,
        pending_choices,
    } = state;

    // EXHAUSTIVE. `setup_seed` is hidden member 1: dropped, never copied.
    let GameConfig {
        player_count,
        player_colors,
        start_player,
        has_port_b,
        has_lab_b,
        setup_seed: _,
    } = config;

    // The live `Unknown` fiction parks BOTH dealt cards in `orders_removed`
    // (game/setup.rs:254-258), so the engine's pile is one larger per hidden
    // seat than the physical box. The view publishes the PHYSICAL count.
    let hidden_seats = players.iter().filter(|p| p.starting_order_hidden).count();
    let orders_removed_len = orders_removed
        .len()
        .checked_sub(hidden_seats)
        .ok_or(ViewError::CorruptState(
            "more hidden-deal seats than cards in the removed pile",
        ))?;

    let visible: Vec<VisiblePlayer> = players.iter().map(|p| visible_player(p, seat)).collect();

    Ok(PlayerView {
        viewer: seat,
        config: ViewConfig {
            player_count: *player_count,
            player_colors: player_colors.clone(),
            start_player: *start_player,
            has_port_b: *has_port_b,
            has_lab_b: *has_lab_b,
        },
        board_layout: board_layout.clone(),
        players: visible,
        current_player: *current_player,
        turn_phase: *turn_phase,
        turn_number: *turn_number,
        additional_action_state: additional_action_state.clone(),
        current_action_power: *current_action_power,
        is_mercenary_action: *is_mercenary_action,
        mercenary_target_area: *mercenary_target_area,
        mercenary_return_phase: *mercenary_return_phase,
        president_start_area: *president_start_area,
        port_state: port_state.clone(),
        lab_state: lab_state.clone(),
        order_deck_len: order_deck.len(),
        tech_deck_len: tech_deck.len(),
        orders_removed_len,
        church_track: church_track.clone(),
        customs_track: customs_track.clone(),
        achievements: achievements.clone(),
        mercenary_supply: mercenary_supply.clone(),
        station_location: *station_location,
        end_game_triggered: *end_game_triggered,
        trigger_reason: *trigger_reason,
        final_round: *final_round,
        trigger_player: *trigger_player,
        pending_choices: pending_choices.clone(),
    })
}

fn visible_player(p: &PlayerState, viewer: PlayerId) -> VisiblePlayer {
    // EXHAUSTIVE -- see the module docs.
    let PlayerState {
        id,
        color,
        victory_points,
        president_location,
        assistants,
        shophouses,
        trading_houses,
        money,
        copper,
        silk,
        tea,
        seafood,
        imported_goods,
        order_cards_in_hand,
        starting_order,
        starting_order_hidden,
        completed_orders,
        technologies,
        five_power_tokens,
        mercenaries,
        church_positions,
        customs_positions,
    } = p;

    let is_viewer = *id == viewer;
    let hand: Vec<HandCard> = if is_viewer {
        order_cards_in_hand.iter().copied().map(HandCard::Known).collect()
    } else if *starting_order_hidden {
        // Live `Unknown`: the card is in NEITHER storage site, so the slot is
        // synthesized at index 0 -- the setup position of a kept starting
        // order -- ahead of the publicly-witnessed Port takes.
        std::iter::once(HandCard::HiddenStartingOrder)
            .chain(order_cards_in_hand.iter().copied().map(HandCard::Known))
            .collect()
    } else {
        match starting_order {
            // Mask ONLY while unfulfilled: once fulfilled the card sits in the
            // face-up `completed_orders` and the residual field leaks nothing.
            Some(c) if order_cards_in_hand.contains(c) => order_cards_in_hand
                .iter()
                .copied()
                .map(|x| if x == *c { HandCard::HiddenStartingOrder } else { HandCard::Known(x) })
                .collect(),
            _ => order_cards_in_hand.iter().copied().map(HandCard::Known).collect(),
        }
    };

    VisiblePlayer {
        id: *id,
        color: *color,
        victory_points: *victory_points,
        president_location: *president_location,
        assistants: assistants.clone(),
        shophouses: shophouses.clone(),
        trading_houses: trading_houses.clone(),
        money: *money,
        copper: *copper,
        silk: *silk,
        tea: *tea,
        seafood: *seafood,
        imported_goods: *imported_goods,
        hand,
        // Hidden member 5, site B. `starting_order_hidden` is deliberately
        // dropped: the masked hand slot is the signal a seat may act on.
        starting_order: if is_viewer { *starting_order } else { None },
        completed_orders: completed_orders.clone(),
        technologies: technologies.clone(),
        five_power_tokens: *five_power_tokens,
        mercenaries: mercenaries.clone(),
        church_positions: church_positions.clone(),
        customs_positions: customs_positions.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::Rules;
    use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
    use crate::types::{GameConfig, HandCard, PlayerId};

    /// 2P game, both seats Keep(0). Seat 0 is the viewer.
    fn keep_game(players: u8, seed: u64) -> (GameState, Rules) {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(players), &rules, seed).unwrap();
        let choices: Vec<_> = (0..players).map(|_| StartingOrderChoice::Keep(0)).collect();
        (finalize_setup(pending, &choices).unwrap(), rules)
    }

    #[test]
    fn view_masks_the_five_hidden_members_and_keeps_public_state() {
        let (state, _rules) = keep_game(2, 7);
        let view = observe(&state, PlayerId(0)).expect("seat 0 is a real seat");

        // 1-3: decks and the boxed pile survive only as sizes.
        assert_eq!(view.order_deck_len, state.order_deck.len());
        assert_eq!(view.tech_deck_len, state.tech_deck.len());
        assert_eq!(view.orders_removed_len, state.orders_removed.len());

        // 4: the opponent's unfulfilled starting order is hidden at BOTH sites.
        let opp = &view.players[1];
        assert_eq!(opp.starting_order, None, "opponent's starting_order is masked");
        assert!(
            opp.hand.contains(&HandCard::HiddenStartingOrder),
            "the opponent's hand slot is masked, not removed"
        );
        assert_eq!(
            opp.hand.len(),
            state.players[1].order_cards_in_hand.len(),
            "hand SIZE stays public"
        );

        // Own seat sees everything about itself.
        let me = &view.players[0];
        assert_eq!(me.starting_order, state.players[0].starting_order);
        assert!(me.hand.iter().all(|c| matches!(c, HandCard::Known(_))));

        // Public state is carried verbatim.
        assert_eq!(view.board_layout, state.board_layout);
        assert_eq!(view.port_state, state.port_state);
        assert_eq!(view.lab_state, state.lab_state);
        assert_eq!(view.church_track, state.church_track);
        assert_eq!(view.customs_track, state.customs_track);
        assert_eq!(view.achievements, state.achievements);
        assert_eq!(view.mercenary_supply, state.mercenary_supply);
        assert_eq!(view.players[1].money, state.players[1].money);
        assert_eq!(view.players[1].technologies, state.players[1].technologies);
        assert_eq!(view.players[1].completed_orders, state.players[1].completed_orders);
    }

    #[test]
    fn view_config_has_no_seed_field_and_carries_the_public_config() {
        let (state, _rules) = keep_game(3, 11);
        let view = observe(&state, PlayerId(1)).unwrap();
        assert_eq!(view.config.player_count, state.config.player_count);
        assert_eq!(view.config.player_colors, state.config.player_colors);
        assert_eq!(view.config.start_player, state.config.start_player);
        assert_eq!(view.config.has_port_b, state.config.has_port_b);
        assert_eq!(view.config.has_lab_b, state.config.has_lab_b);
        // `ViewConfig` carries no seed at all: two games differing ONLY in seed
        // produce equal `ViewConfig`s. (The board still differs -- that residual
        // is spec L7, closed by seed hygiene, not by this type.)
        let (other, _) = keep_game(3, 12);
        let other_view = observe(&other, PlayerId(1)).unwrap();
        assert_eq!(view.config, other_view.config);
    }

    #[test]
    fn dummy_and_out_of_range_seats_are_typed_errors() {
        let (state, _rules) = keep_game(2, 3);
        assert_eq!(observe(&state, PlayerId::DUMMY), Err(ViewError::DummySeat));
        assert_eq!(observe(&state, PlayerId(2)), Err(ViewError::SeatOutOfRange(PlayerId(2))));
    }

    #[test]
    fn a_fulfilled_starting_order_is_no_longer_masked() {
        // Rules: a fulfilled order is placed face-up (01-rules-reference.md:457)
        // and never returns to hand (:464) -- the mask lifts permanently.
        let (mut state, _rules) = keep_game(2, 7);
        let card = state.players[1].starting_order.expect("Keep shape stores it");
        state.players[1].order_cards_in_hand.retain(|&c| c != card);
        state.players[1].completed_orders.push(card);

        let view = observe(&state, PlayerId(0)).unwrap();
        assert!(
            view.players[1].hand.iter().all(|c| matches!(c, HandCard::Known(_))),
            "no hidden slot survives fulfillment"
        );
        assert!(view.players[1].completed_orders.contains(&card));
    }

    /// Viewer keeps; every OTHER seat is a live `Unknown`.
    fn live_game(players: u8, seed: u64) -> (GameState, Rules) {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(players), &rules, seed).unwrap();
        let mut choices = vec![StartingOrderChoice::Keep(0)];
        choices.extend((1..players).map(|_| StartingOrderChoice::Unknown));
        (finalize_setup(pending, &choices).unwrap(), rules)
    }

    #[test]
    fn live_unknown_opponent_gets_a_synthesized_hidden_slot_at_index_zero() {
        let (mut state, _rules) = live_game(2, 7);
        // The engine really does hold NOTHING for that seat.
        assert!(state.players[1].order_cards_in_hand.is_empty());
        assert_eq!(state.players[1].starting_order, None);
        assert!(state.players[1].starting_order_hidden);

        let view = observe(&state, PlayerId(0)).unwrap();
        assert_eq!(view.players[1].hand.first(), Some(&HandCard::HiddenStartingOrder));
        assert_eq!(view.players[1].hand.len(), 1, "physically they hold exactly one card");

        // The INDEX is only observable once the seat holds a second card. A
        // Port take is witnessed by the table (01-rules-reference.md:703), so
        // it must land AFTER the setup card -- exactly where the Keep shape
        // stores it (setup pushes the kept card first, takes append).
        let taken = *state.order_deck.last().expect("deck is not empty");
        state.players[1].order_cards_in_hand.push(taken);
        let view = observe(&state, PlayerId(0)).unwrap();
        assert_eq!(
            view.players[1].hand,
            vec![HandCard::HiddenStartingOrder, HandCard::Known(taken)],
            "the synthesized slot sits at index 0, ahead of witnessed Port takes"
        );
    }

    #[test]
    fn removed_length_is_the_physical_box_not_the_engine_fiction() {
        // 2P: 16 setup removals + 1 discard per seat = 18 physical boxed cards,
        // whichever tracking shape the game was finalized in.
        let (sim, _r1) = keep_game(2, 7);
        let (live, _r2) = live_game(2, 7);
        assert_eq!(sim.orders_removed.len(), 18, "engine SIM pile");
        assert_eq!(live.orders_removed.len(), 19, "engine LIVE pile is one larger");

        assert_eq!(observe(&sim, PlayerId(0)).unwrap().orders_removed_len, 18);
        assert_eq!(observe(&live, PlayerId(0)).unwrap().orders_removed_len, 18);
    }

    #[test]
    fn a_viewer_whose_own_deal_is_unrecorded_is_refused() {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 7).unwrap();
        let state = finalize_setup(
            pending,
            &[StartingOrderChoice::Unknown, StartingOrderChoice::Keep(0)],
        )
        .unwrap();
        assert_eq!(
            observe(&state, PlayerId(0)),
            Err(ViewError::ViewerDealUnrecorded(PlayerId(0))),
            "a seat cannot be unknown to itself"
        );
        // Seat 1 CAN view -- seat 0 is just a hidden opponent from there.
        assert!(observe(&state, PlayerId(1)).is_ok());
    }

    #[test]
    fn sim_and_live_views_agree_when_the_physical_truth_agrees() {
        // Same seed => same board, same deck, same deals. The only difference
        // is whether the opponent's kept card was RECORDED. Seat 0's view must
        // be identical either way -- it never knew that card.
        let (mut sim, _r1) = keep_game(3, 21);
        let (mut live, _r2) = live_game(3, 21);
        let a = observe(&sim, PlayerId(0)).unwrap();
        let b = observe(&live, PlayerId(0)).unwrap();
        assert_eq!(a, b, "tracking shape must not be visible in a seat view");

        // BOTH opponents are masked at both storage sites, not just seat 1 --
        // this is the Keep-shaped 3P view (`a`), so the mask is in-place.
        for opp in [1usize, 2] {
            assert_eq!(a.players[opp].starting_order, None, "seat {opp} starting_order masked");
            assert_eq!(
                a.players[opp]
                    .hand
                    .iter()
                    .filter(|c| matches!(c, HandCard::HiddenStartingOrder))
                    .count(),
                1,
                "seat {opp} holds exactly one masked slot"
            );
            assert_eq!(
                a.players[opp].hand.len(),
                sim.players[opp].order_cards_in_hand.len(),
                "seat {opp} hand SIZE stays public"
            );
        }

        // Agreement must survive past setup, where the two shapes hold their
        // cards at DIFFERENT hand indices: the Keep seat's masked card is at 0
        // with the take at 1, while the Unknown seat physically holds only the
        // take. Same deck under the same seed, so give each opponent a distinct
        // witnessed take.
        assert_eq!(sim.order_deck, live.order_deck, "tracking shape leaves the deck alone");
        let top = sim.order_deck.len();
        let takes = [sim.order_deck[top - 1], sim.order_deck[top - 2]];
        for (opp, take) in [1usize, 2].into_iter().zip(takes) {
            sim.players[opp].order_cards_in_hand.push(take);
            live.players[opp].order_cards_in_hand.push(take);
        }
        assert_eq!(
            observe(&sim, PlayerId(0)).unwrap(),
            observe(&live, PlayerId(0)).unwrap(),
            "a witnessed Port take must not expose the tracking shape either"
        );
    }
}
