//! The 2026-10-08 ruling R-SEC2's record-level proof: `replay_strict_with(..,
//! MoveCheck::Predicate)` accepts EXACTLY the records `replay_strict` accepts,
//! folds them to the same states, and refuses the same tampered records at the
//! same index with the same sentence — so the strict replay can stop
//! materializing a legal list per event with `SEMANTICS_REV` unmoved.
//! Companion to `move_legality.rs`'s `is_listed_move_is_exactly_membership_in_the_legal_list`
//! (the move-level half). Per-edit, never `#[ignore]`d: a proof you can defer
//! is a proof that rots.
mod common;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use yokohama_core::{
    apply_move_unchecked, finalize_setup, get_legal_moves, is_game_over, is_move_legal, new_game,
    replay_strict, replay_strict_with, state_at, state_digest, AreaId, GameConfig, GameRecord, GameState,
    Move, MoveCheck, PathMode, PlayerId, PresidentLocation, RecordedEvent, ReplayError, Rules,
    StartingOrderChoice, SEMANTICS_REV,
};

type Shape = Result<String, (usize, String)>;

/// `apply_move`'s own word for a non-member (`types/errors.rs`) — the one
/// sentence both checks refuse a tampered move with.
const NOT_LISTED: &str = "move is not in the legal-move list for this state";

/// A 2P record whose hidden-starting-order player (P1) holds a FULL order hand
/// (3, the limit `check_invariants` enforces), built from legal moves only —
/// `game_record.rs`'s helper, repeated here because an integration binary
/// sees only `common`. `None` for a seed that never fills the hand.
fn record_with_full_order_hand(seed: u64, rules: &Rules) -> Option<(GameState, GameRecord, PlayerId)> {
    let hidden = PlayerId::new(1);
    let config = GameConfig::for_player_count(2);
    let pending = new_game(config, rules, seed).ok()?;
    let record_config = pending.state.config.clone();
    // P1 keeps its starting order PRIVATE, so a later RevealStartingOrder is a
    // legal event for it.
    let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown];
    let mut record = GameRecord::new(seed, record_config, choices.clone(), rules);
    let mut state = finalize_setup(pending, &choices).ok()?;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    for _ in 0..4000 {
        if state.player(hidden).order_cards_in_hand.len() >= 3 {
            return Some((state, record, hidden));
        }
        if is_game_over(&state) {
            return None;
        }
        let moves = get_legal_moves(&state, rules);
        if moves.is_empty() {
            return None;
        }
        let mv = moves[rng.gen_range(0..moves.len())].clone();
        state = apply_move_unchecked(&state, &mv, rules).ok()?;
        record.push_move(mv);
    }
    None
}

/// A record of uniform-random legal play to `max_moves` applied moves or the
/// game's end, its terminal digest stamped: the long prefixes case (f) needs
/// (`common::run_recorded_game` stops at 40 moves, where a mover has too few
/// assistants on board for a detour to be steppable) and the finished game
/// case (h) needs.
fn recorded_game(seed: u64, players: u8, max_moves: usize, rules: &Rules) -> (GameState, GameRecord) {
    let config = GameConfig::for_player_count(players);
    let pending = new_game(config, rules, seed).expect("new_game");
    let record_config = pending.state.config.clone();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let choices: Vec<StartingOrderChoice> =
        (0..players).map(|_| StartingOrderChoice::Keep(rng.gen_range(0..2u8))).collect();
    let mut record = GameRecord::new(seed, record_config, choices.clone(), rules);
    let mut state = finalize_setup(pending, &choices).expect("finalize_setup");
    for _ in 0..max_moves {
        if is_game_over(&state) {
            break;
        }
        let moves = get_legal_moves(&state, rules);
        assert!(!moves.is_empty(), "a live state offers a move");
        let mv = moves[rng.gen_range(0..moves.len())].clone();
        state = apply_move_unchecked(&state, &mv, rules).expect("a listed move applies");
        record.push_move(mv);
    }
    record.stamp_terminal_digest(&state);
    (state, record)
}

/// Non-canonical WITNESSES for a walked `MovePresident` at `state`: the same
/// route with a detour prepended through a neighbour adjacent to both the
/// start and the first hop, the direct hop where a longer walk's destination
/// is adjacent to the start, and the walk reversed. A variant is a TAMPER exactly when
/// `PathMode::Canonical` accepts it and the list lacks it — the witness-wider
/// case R-SEC2 names, where only the family conjunct sides with the list.
fn witness_variants(state: &GameState, path: &[AreaId], destination: AreaId) -> Vec<Move> {
    let PresidentLocation::OnBoard(current) = state.player(state.current_player).president_location
    else {
        return vec![];
    };
    let Some(&first) = path.first() else { return vec![] };
    let mut out = Vec::new();
    for &via in state.board_layout.adjacent_areas(current) {
        if via == first || path.contains(&via) || !state.board_layout.adjacent_areas(via).contains(&first) {
            continue;
        }
        let mut detour = Vec::with_capacity(path.len() + 1);
        detour.push(via);
        detour.extend_from_slice(path);
        out.push(Move::MovePresident { path: detour, destination });
    }
    if path.len() >= 2 {
        if state.board_layout.adjacent_areas(current).contains(&destination) {
            out.push(Move::MovePresident { path: vec![destination], destination });
        }
        let mut reversed = path.to_vec();
        reversed.reverse();
        let dest = reversed[reversed.len() - 1];
        out.push(Move::MovePresident { path: reversed, destination: dest });
    }
    out
}

fn shape(r: Result<GameState, ReplayError>) -> Shape {
    match r {
        Ok(state) => Ok(state_digest(&state)),
        Err(e) => Err((e.event_index, e.message)),
    }
}

/// Both replays of one record: the strict one, then the predicate one.
fn both(record: &GameRecord, rules: &Rules) -> (Shape, Shape) {
    (shape(replay_strict(record, rules)), shape(replay_strict_with(record, rules, MoveCheck::Predicate)))
}

#[test]
fn the_predicate_replay_folds_every_recorded_game_to_the_strict_replays_state() {
    let rules = Rules::standard();
    let mut games = 0usize;
    for players in [2u8, 3, 4] {
        for seed in 0..8u64 {
            let (final_state, mut record) = common::run_recorded_game(seed, players, &rules);
            record.stamp_terminal_digest(&final_state);
            let (strict, predicate) = both(&record, &rules);
            assert!(strict.is_ok(), "premise: {players}P seed {seed} replays strictly: {strict:?}");
            assert_eq!(predicate, strict, "{players}P seed {seed}: the predicate replay diverged");
            games += 1;
        }
    }
    assert_eq!(games, 24);
}

#[test]
fn the_listed_check_is_replay_strict_move_for_move() {
    let rules = Rules::standard();
    let (final_state, mut record) = common::run_recorded_game(3, 3, &rules);
    record.stamp_terminal_digest(&final_state);
    assert_eq!(
        shape(replay_strict_with(&record, &rules, MoveCheck::Listed)),
        shape(replay_strict(&record, &rules)),
    );
}

#[test]
fn an_empty_record_folds_to_the_same_initial_state_under_both_checks() {
    let rules = Rules::standard();
    let (_, mut record) = common::run_recorded_game(11, 2, &rules);
    record.events.clear();
    record.terminal_digest = None;
    let (strict, predicate) = both(&record, &rules);
    assert!(strict.is_ok(), "an empty record replays to the initial state: {strict:?}");
    assert_eq!(predicate, strict);
}

/// Every tampered record `game_record.rs` refuses, both replays refuse at the
/// same index with the same sentence — the preamble and digest check
/// `replay_strict_with` repeats are pinned to `replay_strict`'s words here —
/// and so are the four shapes only this binary builds: a non-canonical
/// witness (f), a family member out of its phase (i), an invariant break (g)
/// and a move after the game's end (h).
#[test]
fn the_two_replays_refuse_the_same_tampered_records_with_the_same_words() {
    let rules = Rules::standard();
    let (final_state, clean) = common::run_recorded_game(7, 2, &rules);
    let stamped = {
        let mut r = clean.clone();
        r.stamp_terminal_digest(&final_state);
        r
    };
    let mut refusals = 0usize;
    let refuse = |record: &GameRecord, what: &str, refusals: &mut usize| {
        let (strict, predicate) = both(record, &rules);
        assert!(strict.is_err(), "premise: {what} is refused by replay_strict");
        assert_eq!(predicate, strict, "{what}: the two replays disagree");
        *refusals += 1;
    };

    // (a) an out-of-phase move at event 1 (game_record.rs:118-128)
    let mut r = stamped.clone();
    r.events[1] = RecordedEvent::Move(Move::Take5PowerToken);
    refuse(&r, "an out-of-phase move", &mut refusals);

    // (b) a wrong terminal digest (game_record.rs:258-268)
    let mut r = stamped.clone();
    r.terminal_digest = Some("ffffffffffffffff".into());
    refuse(&r, "a wrong terminal digest", &mut refusals);

    // (c) a rules-hash mismatch (game_record.rs:129-138)
    let mut r = stamped.clone();
    r.rules_hash = "0000000000000000".into();
    refuse(&r, "a rules-hash mismatch", &mut refusals);

    // (d) no semantics provenance (game_record.rs:323)
    let mut r = stamped.clone();
    r.semantics_rev = None;
    refuse(&r, "a record with no provenance", &mut refusals);

    // (e) another semantics rev (game_record.rs:223-231)
    let mut r = stamped.clone();
    r.semantics_rev = Some(SEMANTICS_REV + 1);
    refuse(&r, "a semantics-rev mismatch", &mut refusals);

    // (f) a walked president move replaced by a NON-CANONICAL WITNESS the list
    //     refuses and `PathMode::Canonical` accepts — the witness-wider case
    //     (R-SEC2) where only the family conjunct sides with the list. A
    //     conjunct-less predicate would APPLY the variant (same label, same
    //     successor state) and fold on to the very digest the record carries,
    //     answering Ok where `replay_strict` refuses at k — the divergence
    //     this case exists to see. Beside it, the repeated-hop witness the
    //     predicate refuses by itself. Long prefixes, walked ONCE with the
    //     decision state carried along: a 40-move record holds walks a mover
    //     cannot yet detour around.
    let mut walked = 0usize;
    let mut witness_tampers = 0usize;
    let mut repeated_hop_tampers = 0usize;
    let mut out_of_phase_tampers = 0usize;
    'games: for players in [2u8, 3, 4] {
        for seed in 0..3u64 {
            let (_, record) = recorded_game(seed, players, 400, &rules);
            let mut before = state_at(&record, 0, &rules).expect("the initial state rebuilds");
            for k in 0..record.events.len() {
                let RecordedEvent::Move(mv) = &record.events[k] else {
                    unreachable!("a recorded game holds moves only");
                };
                if let Move::MovePresident { path, destination } = mv {
                    if !path.is_empty() {
                        walked += 1;
                        let listed = get_legal_moves(&before, &rules);
                        let mut tampers = Vec::new();
                        if repeated_hop_tampers < 3 {
                            let mut repeated = path.clone();
                            repeated.insert(0, path[0]);
                            tampers.push((Move::MovePresident { path: repeated, destination: *destination }, false));
                        }
                        for variant in witness_variants(&before, path, *destination) {
                            if is_move_legal(&before, &variant, &rules, PathMode::Canonical)
                                && !listed.contains(&variant)
                            {
                                tampers.push((variant, true));
                            }
                        }
                        for (tamper, is_witness) in tampers {
                            let mut r = record.clone();
                            r.events[k] = RecordedEvent::Move(tamper.clone());
                            let (strict, predicate) = both(&r, &rules);
                            assert_eq!(
                                strict,
                                Err((k, NOT_LISTED.to_string())),
                                "premise: {players}P seed {seed} event {k}: replay_strict refuses {tamper:?}",
                            );
                            assert_eq!(
                                predicate, strict,
                                "{players}P seed {seed} event {k}: the two replays disagree on {tamper:?}",
                            );
                            if is_witness {
                                witness_tampers += 1;
                            } else {
                                repeated_hop_tampers += 1;
                            }
                        }
                        // (i) a MEMBER of the phase-blind Movement family planted at
                        //     the NEXT event, where the phase is AreaAction: the list
                        //     there cannot hold it, and only `is_move_legal`'s phase
                        //     gate refuses it — a family-alone predicate would apply
                        //     it raw and fold on to a different state. The
                        //     member is read off `get_legal_moves` on a copy of that
                        //     state with its phase set to Movement (the family consults
                        //     nothing about the phase by contract, so the copy's
                        //     Movement subsequence IS the family at the real state).
                        if out_of_phase_tampers < 4 && k + 1 < record.events.len() {
                            let after = apply_move_unchecked(&before, mv, &rules).expect("a recorded move applies");
                            let mut as_movement = after.clone();
                            as_movement.turn_phase = yokohama_core::TurnPhase::Movement;
                            let member = get_legal_moves(&as_movement, &rules)
                                .into_iter()
                                .find(|m| matches!(m, Move::MovePresident { .. } | Move::PlacePresident { .. }));
                            if let Some(member) = member {
                                assert_eq!(after.turn_phase, yokohama_core::TurnPhase::AreaAction, "premise: a walk hands off to AreaAction");
                                let mut r = record.clone();
                                r.events[k + 1] = RecordedEvent::Move(member.clone());
                                let (strict, predicate) = both(&r, &rules);
                                assert_eq!(
                                    strict,
                                    Err((k + 1, NOT_LISTED.to_string())),
                                    "premise: {players}P seed {seed} event {}: replay_strict refuses the out-of-phase member {member:?}",
                                    k + 1,
                                );
                                assert_eq!(
                                    predicate, strict,
                                    "{players}P seed {seed} event {}: the two replays disagree on an out-of-phase member {member:?}",
                                    k + 1,
                                );
                                out_of_phase_tampers += 1;
                            }
                        }
                        if witness_tampers >= 8 && out_of_phase_tampers >= 4 {
                            break 'games;
                        }
                    }
                }
                before = apply_move_unchecked(&before, mv, &rules).expect("a recorded move applies");
            }
        }
    }
    assert!(walked > 0, "no recorded game held a walked president move");
    assert!(repeated_hop_tampers > 0, "no repeated-hop tamper ran");
    assert!(
        witness_tampers > 0,
        "no Canonical-accepted non-member witness was built — the family conjunct is untested at the record level",
    );
    assert!(
        out_of_phase_tampers > 0,
        "no family member was planted out of its phase — the phase gate is untested at the record level",
    );
    refusals += 1;

    // (g) a broken state though every event applied (the shape of
    //     game_record.rs:156-172): a RevealStartingOrder into a hand already
    //     holding 3 orders APPLIES as an event and leaves a state
    //     `check_invariants` refuses — the invariant check pinned under both.
    let (state, mut r, hidden) = (0..64u64)
        .find_map(|seed| record_with_full_order_hand(seed, &rules))
        .expect("some seed must fill P1's order hand, else this case proves nothing");
    let card = *state
        .orders_removed
        .last()
        .or_else(|| state.order_deck.last())
        .expect("removed pile holds P1's two Unknown-parked cards at minimum");
    r.push_event(RecordedEvent::RevealStartingOrder { player: hidden, card });
    refuse(&r, "a broken state though every event applied", &mut refusals);

    // (h) a move appended after the game's END. The phase machine parks a
    //     finished game at `TurnEnd` (`apply_move.rs`'s `advance_phase`), where
    //     the list is empty and no predicate arm answers, so both replays
    //     refuse the appended event at the past-the-end index with `NotLegal`'s
    //     words — neither check continues a finished game.
    let (end, mut done) = recorded_game(0, 2, 20_000, &rules);
    assert!(is_game_over(&end), "premise: the game ran to its end within 20,000 moves");
    let extra = done.events.len();
    done.push_move(Move::EndAdditionalPhase);
    let (strict, predicate) = both(&done, &rules);
    assert_eq!(strict, Err((extra, NOT_LISTED.to_string())), "premise: a move after the end is refused");
    assert_eq!(predicate, strict, "a move after the end: the two replays disagree");
    refusals += 1;

    println!(
        "tampers: walked {walked}, witness {witness_tampers}, repeated-hop {repeated_hop_tampers}, \
         out-of-phase members {out_of_phase_tampers}; the finished game held {extra} events"
    );
    assert_eq!(refusals, 8, "every refusal shape above ran");
}

/// Recorded, never asserted: the wall-clock of both replays over the longest
/// recorded game, printed under `--nocapture` for the task report.
#[test]
fn the_two_replays_walls_are_recorded() {
    let rules = Rules::standard();
    let (final_state, mut record) = common::run_recorded_game(0, 4, &rules);
    record.stamp_terminal_digest(&final_state);
    let t = std::time::Instant::now();
    let strict = replay_strict(&record, &rules).map(|s| state_digest(&s));
    let strict_wall = t.elapsed();
    let t = std::time::Instant::now();
    let predicate = replay_strict_with(&record, &rules, MoveCheck::Predicate).map(|s| state_digest(&s));
    let predicate_wall = t.elapsed();
    assert_eq!(predicate.ok(), strict.ok());
    println!(
        "replay walls over {} events: strict {:?}, predicate {:?}",
        record.events.len(),
        strict_wall,
        predicate_wall
    );
}
