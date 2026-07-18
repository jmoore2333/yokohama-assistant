//! Task 19 (spec F3): `GameRecord` save/replay format + replay determinism.

mod common;
use common::run_recorded_game;

use yokohama_core::{replay, GameRecord, Move, OrderCardId, RecordedEvent, Rules};

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
