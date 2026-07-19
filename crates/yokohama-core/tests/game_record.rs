//! Task 19 (spec F3): `GameRecord` save/replay format + replay determinism.

mod common;
use common::run_recorded_game;

use yokohama_core::{replay, replay_strict, GameRecord, Move, OrderCardId, RecordedEvent, Rules};

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
