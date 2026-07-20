//! Phase-2.9 Stage-6 audit: END-GAME TRIGGER TIMING pin tests.
//!
//! Audit evidence, not new behavior. Each test constructs a state ONE
//! increment below an end-game threshold (conservation-respecting surgery,
//! same style as the T18 seeded scenarios in fuzz_full_games.rs), applies
//! the crossing move through the public apply API, and pins that the
//! trigger fires exactly at the documented boundary (rulebook p.8-9 "End
//! of Game Conditions") -- not one increment before, and for OrderDeckEmpty
//! not on deck-exhaustion alone but on the first FAILED replenish
//! ("After replenishing Order cards, the Order deck has run out and there
//! is an empty space...").
//!
//! Also pinned: the final-round turn accounting ("Finish playing out the
//! round ending with the player to the right of the Start player, then
//! there is one final round for all players. Thus, all players will play
//! an equal number of turns.") -- is_game_over must flip exactly when the
//! player before the start player ends their final-round turn, and a
//! second threshold crossed DURING the final round must not extend it.

mod common;
use common::run_random_game_from;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use yokohama_core::{
    apply_move_unchecked, check_end_game_triggers, finalize_setup, get_legal_moves, is_game_over,
    new_game, AreaAction, AreaType, ChurchSlot, EndGameTrigger, GameConfig, GameState, Move,
    PlayerId, PresidentLocation, Rules, StartingOrderChoice, TurnPhase,
};

/// Fresh seeded standard game at `players`, ready for surgical adjustment.
fn fresh_game(seed: u64, players: u8) -> (GameState, ChaCha8Rng, Rules) {
    let rules = Rules::standard();
    let rng = ChaCha8Rng::seed_from_u64(seed);
    let config = GameConfig::for_player_count(players);
    let pending = new_game(config, &rules, seed).expect("new_game");
    let choices: Vec<StartingOrderChoice> =
        (0..players).map(|_| StartingOrderChoice::Keep(0)).collect();
    let state = finalize_setup(pending, &choices).expect("finalize_setup");
    (state, rng, rules)
}

/// Put `player`'s president on `area` in the AreaAction phase at `power`.
fn stage_area_action(state: &mut GameState, player: PlayerId, area: yokohama_core::AreaId, power: u8) {
    state.current_player = player;
    state.turn_phase = TurnPhase::AreaAction;
    state.current_action_power = Some(power);
    state.players[player.0 as usize].president_location = PresidentLocation::OnBoard(area);
}

/// Find an area of the given type in the generated layout.
fn area_of_type(state: &GameState, ty: AreaType) -> yokohama_core::AreaId {
    *state
        .board_layout
        .areas
        .iter()
        .find(|(_, cfg)| cfg.area_type == ty)
        .map(|(id, _)| id)
        .expect("area type present in layout")
}

/// Resolve any queued choices (church-space movement riders etc.) by
/// declining where a Decline is offered, so the next staged apply starts
/// from an empty queue. Every resolution goes through the public apply.
fn drain_pending_choices(state: &mut GameState, rules: &Rules) {
    while !state.pending_choices.is_empty() {
        let moves = get_legal_moves(state, rules);
        let mv = moves
            .iter()
            .find(|m| matches!(m, Move::DeclineAssistantMove | Move::DeclineWarehouseTake))
            .or_else(|| moves.first())
            .expect("queued choice must offer at least one move")
            .clone();
        *state = apply_move_unchecked(state, &mv, rules).expect("choice resolution applies");
    }
}

/// Surgically park `count` pawns on the HIGH end of the church track
/// (faith 11 downward), alternating owners, keeping player assistant
/// accounting consistent (in_hand -> on_church) so invariants hold.
fn stage_church_pawns(state: &mut GameState, count: u8) {
    for i in 0..count {
        let faith = 11 - i; // 11, 10, 9, 8 ... leaves the low spaces free
        let owner = (i % state.config.player_count) as usize;
        let idx = (faith - 4) as usize;
        assert!(state.church_track.slots[idx].is_none());
        let p = &mut state.players[owner];
        p.assistants.in_hand -= 1;
        p.assistants.on_church += 1;
        state.church_track.slots[idx] = Some(ChurchSlot {
            player: PlayerId::new(owner as u8),
            arrival_order: i,
        });
        state.church_track.next_arrival_order = i + 1;
    }
}

/// Church action at `power` with no goods contributed: faith == power, so
/// power 4 places on the faith-4 space inline (single eligible space) and
/// power 5 places on faith-5 once faith-4 is occupied.
fn bare_church_action() -> Move {
    Move::ExecuteAreaAction {
        action: AreaAction::Church {
            copper: false,
            silk: false,
            tea: false,
            seafood: false,
            yen: false,
            imported_good: false,
        },
    }
}

/// Church-track threshold pin, shared across player counts: with
/// `threshold - 2` pawns parked high on the track, a placement reaching
/// `threshold - 1` must NOT trigger, and the next placement reaching
/// `threshold` must trigger ChurchTrackFull in that same apply.
fn church_threshold_fires_exactly(players: u8, seed: u64) {
    let (mut state, mut rng, rules) = fresh_game(seed, players);
    let threshold = state.config.track_trigger();
    let church = area_of_type(&state, AreaType::Church);

    stage_church_pawns(&mut state, threshold - 2);

    // Placement to threshold-1 (faith 4, single eligible space -> inline).
    stage_area_action(&mut state, PlayerId::new(0), church, 4);
    state = apply_move_unchecked(&state, &bare_church_action(), &rules).expect("church at faith 4");
    assert_eq!(state.church_track.total_assistants(), threshold - 1);
    assert!(
        !state.end_game_triggered,
        "{players}p: trigger must NOT fire at {} pawns (threshold {threshold})",
        threshold - 1
    );
    drain_pending_choices(&mut state, &rules);
    assert!(!state.end_game_triggered);

    // Crossing placement to threshold (faith 5 now the single eligible).
    stage_area_action(&mut state, PlayerId::new(1), church, 5);
    state = apply_move_unchecked(&state, &bare_church_action(), &rules).expect("church at faith 5");
    assert_eq!(state.church_track.total_assistants(), threshold);
    assert!(
        state.end_game_triggered,
        "{players}p: trigger must fire exactly at {threshold} pawns"
    );
    assert!(matches!(state.trigger_reason, Some(EndGameTrigger::ChurchTrackFull)));
    assert!(!is_game_over(&state), "trigger fires mid-turn; game must not be over yet");
    drain_pending_choices(&mut state, &rules);

    // The triggered game random-plays to a completed, invariant-checked end.
    let stats = run_random_game_from(state, &mut rng, seed, players, &rules)
        .expect("random play finishes the triggered game");
    assert!(matches!(stats.trigger, Some(EndGameTrigger::ChurchTrackFull)));
}

#[test]
fn church_track_2p_trigger_fires_exactly_at_4_pawns() {
    // Rulebook p.9: "2 Players: 4". (2p also has the printed faith-7 dummy,
    // which must NOT count toward the 4 -- it is an eligibility block, not
    // a pawn; staging leaves faith 4-6 free so both placements land inline.)
    church_threshold_fires_exactly(2, 201);
}

#[test]
fn church_track_3p_trigger_fires_exactly_at_5_pawns() {
    // Rulebook p.9: "3 Players: 5".
    church_threshold_fires_exactly(3, 202);
}

#[test]
fn church_track_4p_trigger_fires_exactly_at_6_pawns() {
    // Rulebook p.9: "4 Players: 6". Also pins track_trigger(4) == 6, which
    // no unit test covered (end_game.rs only pinned the 2p/3p constants).
    let config = GameConfig::for_player_count(4);
    assert_eq!(config.track_trigger(), 6);
    church_threshold_fires_exactly(4, 203);
}

#[test]
fn order_deck_empty_triggers_only_after_failed_replenish() {
    // Rulebook p.9: "After replenishing Order cards, the Order deck has run
    // out and there is an empty space on the card area of the Port Order
    // Form Board." An exhausted deck with all port slots still full is NOT
    // an ending -- the trigger is the first replenish that cannot fill a
    // slot.
    let (mut state, mut rng, rules) = fresh_game(204, 2);

    // Surgery: drain the deck into orders_removed (order conservation:
    // deck + ports + hands + completed + removed = 36 is preserved).
    let drained: Vec<_> = state.order_deck.drain(..).collect();
    state.orders_removed.extend(drained);

    // Deck empty, all 6 Port A slots full (fresh setup): must NOT trigger.
    check_end_game_triggers(&mut state);
    assert!(
        !state.end_game_triggered,
        "empty deck with a full port must not trigger (no failed replenish yet)"
    );

    // Take 1 LOW-zone card at power 2 through the public apply: the take
    // compacts, the refill finds the deck empty, a slot stays open -- the
    // trigger must fire in this same apply.
    let port_a = area_of_type(&state, AreaType::PortA);
    let low_card = state.port_state.port_a.slots[0].expect("fresh port slot 0 filled");
    stage_area_action(&mut state, PlayerId::new(0), port_a, 2);
    let mv = Move::ExecuteAreaAction {
        action: AreaAction::Port { cards_taken: vec![low_card], second_card_payment: None },
    };
    state = apply_move_unchecked(&state, &mv, &rules).expect("port take");

    assert!(state.end_game_triggered, "failed replenish must trigger in the same apply");
    assert!(matches!(state.trigger_reason, Some(EndGameTrigger::OrderDeckEmpty)));
    assert!(state.port_state.port_a.slots.iter().any(|s| s.is_none()));
    assert!(!is_game_over(&state));

    let stats = run_random_game_from(state, &mut rng, 204, 2, &rules)
        .expect("random play finishes the triggered game");
    assert!(matches!(stats.trigger, Some(EndGameTrigger::OrderDeckEmpty)));
}

/// End the current player's turn through the public apply (turn handoff is
/// only ever the explicit EndAdditionalPhase arm). The phase is staged to
/// AdditionalActionSecond surgically; the transition itself -- final_round
/// bookkeeping, TurnEnd assignment -- is all engine code under test.
fn end_turn(state: &mut GameState, rules: &Rules) {
    assert!(state.pending_choices.is_empty(), "cannot end a turn mid-queue");
    state.turn_phase = TurnPhase::AdditionalActionSecond;
    *state = apply_move_unchecked(state, &Move::EndAdditionalPhase, rules).expect("end turn");
}

#[test]
fn final_round_equal_turns_ends_exactly_at_player_before_start() {
    // Rulebook p.9: "Finish playing out the round ending with the player to
    // the right of the Start player, then there is one final round for all
    // players. Thus, all players will play an equal number of turns."
    //
    // 2p, start player 0, trigger fired DURING player 0's turn. Expected
    // walk: P0 ends (trigger round continues) -> P1 ends (round complete,
    // final round begins) -> P0 ends -> P1 ends => TurnEnd, game over --
    // and is_game_over is false at every earlier step.
    let (mut state, _rng, rules) = fresh_game(205, 2);
    let church = area_of_type(&state, AreaType::Church);
    stage_church_pawns(&mut state, 3);

    // Cross the threshold during P0's turn (4th pawn at faith 4).
    stage_area_action(&mut state, PlayerId::new(0), church, 4);
    state = apply_move_unchecked(&state, &bare_church_action(), &rules).expect("crossing placement");
    assert!(state.end_game_triggered);
    drain_pending_choices(&mut state, &rules);
    assert!(!state.final_round);
    assert!(!is_game_over(&state));
    assert!(!state.is_game_over(), "both is_game_over impls must agree");

    // P0 ends the trigger turn: round not yet complete.
    end_turn(&mut state, &rules);
    assert_eq!(state.current_player, PlayerId::new(1));
    assert!(!state.final_round);
    assert!(!is_game_over(&state) && !state.is_game_over());

    // P1 (player before start) ends: trigger round complete, final round on.
    end_turn(&mut state, &rules);
    assert_eq!(state.current_player, PlayerId::new(0));
    assert!(state.final_round, "final round must begin when the round returns to the start player");
    assert!(!is_game_over(&state) && !state.is_game_over());

    // P0's final-round turn.
    end_turn(&mut state, &rules);
    assert_eq!(state.current_player, PlayerId::new(1));
    assert!(!is_game_over(&state) && !state.is_game_over());

    // P1's final-round turn: the game ends exactly here, with the player
    // before the start player at TurnEnd (equal turns for both players:
    // each played exactly one turn after the trigger round completed).
    end_turn(&mut state, &rules);
    assert_eq!(state.turn_phase, TurnPhase::TurnEnd);
    assert_eq!(state.current_player, PlayerId::new(1), "TurnEnd rests on the player before start");
    assert!(is_game_over(&state) && state.is_game_over());
}

#[test]
fn second_threshold_crossed_during_final_round_does_not_extend_the_game() {
    // A second end condition met during the final round must neither change
    // the recorded trigger nor add turns: the game still ends when the
    // player before the start player finishes the final round.
    let (mut state, _rng, rules) = fresh_game(206, 2);
    let church = area_of_type(&state, AreaType::Church);
    let customs = area_of_type(&state, AreaType::Customs);
    stage_church_pawns(&mut state, 3);

    // Trigger ChurchTrackFull during P0's turn, then complete the round.
    stage_area_action(&mut state, PlayerId::new(0), church, 4);
    state = apply_move_unchecked(&state, &bare_church_action(), &rules).expect("church trigger");
    assert!(matches!(state.trigger_reason, Some(EndGameTrigger::ChurchTrackFull)));
    drain_pending_choices(&mut state, &rules);
    end_turn(&mut state, &rules); // P0
    end_turn(&mut state, &rules); // P1 -> final round begins
    assert!(state.final_round);

    // During P0's FINAL-ROUND turn, cross the customs threshold too:
    // 3 pawns staged + 1 placed at space 7 (cost 4) = the 2p threshold of 4.
    for (i, (player, space)) in [(0u8, 1u8), (1, 2), (0, 3)].into_iter().enumerate() {
        let idx = (space - 1) as usize;
        let p = &mut state.players[player as usize];
        p.assistants.in_hand -= 1;
        p.assistants.on_customs += 1;
        state.customs_track.slots[idx] = Some(yokohama_core::CustomsSlot {
            player: PlayerId::new(player),
            arrival_order: i as u8,
        });
        state.customs_track.next_arrival_order = i as u8 + 1;
    }
    state.players[0].imported_goods = 4;
    stage_area_action(&mut state, PlayerId::new(0), customs, 5);
    let mv = Move::ExecuteAreaAction { action: AreaAction::Customs { imported_goods_paid: 4 } };
    state = apply_move_unchecked(&state, &mv, &rules).expect("customs placement in final round");
    assert_eq!(state.customs_track.total_assistants(), 4);
    drain_pending_choices(&mut state, &rules);

    // The original trigger stands; the boundary is unchanged.
    assert!(matches!(state.trigger_reason, Some(EndGameTrigger::ChurchTrackFull)));
    assert!(state.final_round);
    end_turn(&mut state, &rules); // P0 ends final-round turn
    assert!(!is_game_over(&state));
    end_turn(&mut state, &rules); // P1 ends -> game over, same boundary
    assert_eq!(state.turn_phase, TurnPhase::TurnEnd);
    assert!(is_game_over(&state) && state.is_game_over());
}

#[test]
fn simultaneous_trigger_priority_is_the_documented_check_order() {
    // ENCODING PIN, not a rules claim: the rulebook (p.8-9) gives the five
    // conditions no priority -- game-end timing is identical regardless of
    // which is recorded. The engine checks TradingHouses, Shophouses,
    // OrderDeckEmpty, ChurchTrackFull, CustomsTrackFull in that order and
    // records the first. Pin it so the trigger_reason label in records
    // cannot drift silently.
    let (mut state, _rng, _rules) = fresh_game(207, 2);

    // Both the church threshold AND a trading-house completion "hold" at
    // once (unreachable in real play -- the first crossing would already
    // have triggered -- but exactly what a re-ordered checker would
    // misreport). No invariant-checked run from this state: it exists only
    // to observe the checker's own priority.
    stage_church_pawns(&mut state, 4);
    let areas: Vec<_> = state.board_layout.areas.keys().copied().take(4).collect();
    state.players[0].trading_houses.built = areas;
    state.players[0].trading_houses.in_hand = 0;
    state.players[0].trading_houses.in_warehouse = 0;

    check_end_game_triggers(&mut state);
    assert!(state.end_game_triggered);
    assert!(matches!(
        state.trigger_reason,
        Some(EndGameTrigger::TradingHousesComplete(p)) if p == PlayerId::new(0)
    ));
}
