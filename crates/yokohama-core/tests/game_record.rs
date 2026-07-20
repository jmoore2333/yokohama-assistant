//! Task 19 (spec F3): `GameRecord` save/replay format + replay determinism.

mod common;
use common::run_recorded_game;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use yokohama_core::{
    apply_move_unchecked, finalize_setup, get_legal_moves, is_game_over, new_game, replay, replay_strict,
    GameConfig, GameRecord, GameState, Move, OrderCardId, PlayerId, RecordedEvent, Rules,
    StartingOrderChoice,
};

/// Build a record whose hidden-starting-order player (P1) is holding a FULL
/// order hand (3 -- the limit enforced by `check_invariants`). Plays real
/// legal moves only, so every event in the returned record applies cleanly.
///
/// Searches seeds because whether a given random game fills P1's hand is
/// incidental; returns `None` for a seed that doesn't, so the caller can
/// fail loudly rather than the test going quietly vacuous.
fn record_with_full_order_hand(seed: u64, rules: &Rules) -> Option<(GameState, GameRecord, PlayerId)> {
    let hidden = PlayerId::new(1);
    let config = GameConfig::for_player_count(2);
    let pending = new_game(config, rules, seed).ok()?;
    let record_config = pending.state.config.clone();
    // P1 keeps its starting order PRIVATE, so `starting_order_hidden` stays
    // true and a later RevealStartingOrder is a legal event for it.
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

#[test]
fn record_replays_to_identical_state() {
    let rules = Rules::standard();
    let (final_state, record) = run_recorded_game(7, 2, &rules);

    let replayed = replay(&record, &rules).unwrap();
    assert_eq!(replayed, final_state);

    // JSON round-trip preserves equality too.
    let json = serde_json::to_string(&record).unwrap();
    let back: GameRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(replay(&back, &rules).unwrap(), final_state);
}

#[test]
fn replay_of_mid_list_illegal_move_returns_err_not_panic() {
    let rules = Rules::standard();
    let (_, mut record) = run_recorded_game(7, 2, &rules);
    assert!(!record.events.is_empty());

    // OrderCardId(255) was never dealt (real ids are 1-36), so it can never
    // be in any player's hand: apply_fulfill_order's very first check ("is
    // this order in the current player's hand?") fails cleanly, regardless
    // of phase or list position -- no risk of panicking on a bad index.
    let mid = record.events.len() / 2;
    record.events.insert(mid, RecordedEvent::Move(Move::FulfillOrder { order_id: OrderCardId(255) }));

    let result = replay(&record, &rules);
    assert!(result.is_err());
}

#[test]
fn strict_replay_matches_normal_replay_on_a_clean_record() {
    let rules = Rules::standard();
    let (final_state, record) = common::run_recorded_game(7, 2, &rules);
    assert_eq!(replay_strict(&record, &rules).unwrap(), final_state);
}

#[test]
fn strict_replay_rejects_engine_version_mismatch_instead_of_warning() {
    let rules = Rules::standard();
    let (_, mut record) = common::run_recorded_game(7, 2, &rules);
    record.engine_version = "0.0.0-tampered".into();
    let err = replay_strict(&record, &rules).unwrap_err();
    assert_eq!(err.event_index, 0);
    assert!(err.message.contains("engine_version"));
    // and the lenient path still replays (warn-and-continue preserved):
    assert!(replay(&record, &rules).is_ok());
}

#[test]
fn strict_replay_rejects_an_out_of_phase_tampered_move_with_its_event_index() {
    let rules = Rules::standard();
    let (_, mut record) = common::run_recorded_game(7, 2, &rules);
    // Overwrite event k with a move that is never legal at that point.
    let k = 1;
    record.events[k] = RecordedEvent::Move(Move::Take5PowerToken);
    let err = replay_strict(&record, &rules).unwrap_err();
    assert_eq!(err.event_index, k);
}

#[test]
fn records_stamp_the_rules_hash_and_strict_replay_rejects_a_mismatch() {
    let rules = Rules::standard();
    let (_, mut record) = common::run_recorded_game(7, 2, &rules);
    assert_eq!(record.rules_hash, rules.content_hash());
    record.rules_hash = "deadbeefdeadbeef".into();
    let err = replay_strict(&record, &rules).unwrap_err();
    assert_eq!(err.event_index, 0);
    assert!(err.message.contains("rules_hash"));
    assert!(replay(&record, &rules).is_ok(), "lenient replay warns but continues");
}

/// I2 regression (external six-agent review, 2026-07-20): `replay_strict`'s
/// per-event `check_invariants` is its ONLY behavioural differentiator from
/// lenient `replay` -- both fold through the same `fold_one_event` -- and it
/// had no test. Deleting the call left all 497 tests green.
///
/// The four pre-existing strict tests all bail BEFORE the fold: two on
/// version/hash mismatch (return at the top), one on a tampered move (fails
/// inside `fold_one_event`), one on a clean record. None reaches a state that
/// is structurally invalid yet was produced only by events that applied.
///
/// Vehicle: `RecordedEvent::RevealStartingOrder`. `reveal_starting_order`
/// checks the hidden flag and extracts the card, but NOT the order hand
/// limit -- so revealing into a player already holding 3 orders succeeds as
/// an event while leaving a state that violates `check_invariants`. Lenient
/// replay must fold it happily; strict must reject it, naming the index.
#[test]
fn strict_replay_rejects_a_broken_state_though_every_event_applied() {
    let rules = Rules::standard();

    let (state, mut record, hidden) = (0..64u64)
        .find_map(|seed| record_with_full_order_hand(seed, &rules))
        .expect("some seed must fill P1's order hand, else this test proves nothing");

    // Any extractable card works -- `extract_order_from_deck` searches the
    // deck AND the removed pile. Prefer the removed pile: the `Unknown`
    // choice parks BOTH of P1's dealt cards there, so it is non-empty even
    // deep into a game where the deck has run dry.
    let card = *state
        .orders_removed
        .last()
        .or_else(|| state.order_deck.last())
        .expect("removed pile holds P1's two Unknown-parked cards at minimum");
    record.push_event(RecordedEvent::RevealStartingOrder { player: hidden, card });
    let bad_index = record.events.len() - 1;

    // Lenient replay folds it without complaint: every event APPLIED.
    let lenient = replay(&record, &rules).expect("every event applies, so lenient replay succeeds");
    assert!(
        lenient.player(hidden).order_cards_in_hand.len() > 3,
        "fixture: the reveal must actually overfill the hand, else there is no invariant to break"
    );

    // Strict must catch what lenient waved through, and say which event.
    // Matched rather than `expect_err`-ed: the Ok variant is a whole
    // GameState, and printing it buries the actual failure in a screenful
    // of board dump.
    let err = match replay_strict(&record, &rules) {
        Err(e) => e,
        Ok(_) => panic!(
            "strict replay accepted a state violating the order hand limit -- \
             its per-event check_invariants is the only thing separating it \
             from lenient replay"
        ),
    };
    assert_eq!(err.event_index, bad_index, "strict must name the offending event: {err}");
    assert!(err.message.contains("invariant violated"), "got: {}", err.message);
    assert!(err.message.contains("order hand size"), "got: {}", err.message);
}
