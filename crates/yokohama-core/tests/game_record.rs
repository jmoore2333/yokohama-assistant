//! Task 19 (spec F3): `GameRecord` save/replay format + replay determinism.

mod common;
use common::run_recorded_game;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use yokohama_core::{
    apply_move_unchecked, finalize_setup, get_legal_moves, is_game_over, new_game, replay, replay_strict,
    state_digest, GameConfig, GameRecord, GameState, Move, OrderCardId, PlayerId, RecordedEvent, Rules,
    StartingOrderChoice, SEMANTICS_REV,
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

// --- Phase 2.9 close-out: semantics-revision stamp + terminal-state digest ---
// guidingpath Phase-3 MANDATORY handover item (a): today's `engine_version`
// (never-bumped workspace 0.1.0) and `rules_hash` (static DATA only) don't
// identify executable semantics, so a legality/apply/scoring/setup change
// moves neither stamp and `replay_strict` would silently reinterpret an old
// record. `SEMANTICS_REV` closes that gap; the terminal digest is the
// defense-in-depth backstop on top of the per-event invariant checks.

/// Every fresh record is stamped with the running engine's `SEMANTICS_REV`.
#[test]
fn records_stamp_the_running_semantics_rev() {
    let rules = Rules::standard();
    let (_, record) = common::run_recorded_game(7, 2, &rules);
    assert_eq!(record.semantics_rev, Some(SEMANTICS_REV));
}

/// Strict replay rejects a record whose `semantics_rev` differs from this
/// engine's -- the whole point of the stamp: a behaviour change bumps the
/// rev, and an old record must not be silently reinterpreted. Lenient replay
/// still warns-and-continues, mirroring the engine_version/rules_hash split.
#[test]
fn strict_replay_rejects_semantics_rev_mismatch_but_lenient_continues() {
    let rules = Rules::standard();
    let (_, mut record) = common::run_recorded_game(7, 2, &rules);
    record.semantics_rev = Some(SEMANTICS_REV + 1);
    let err = replay_strict(&record, &rules).unwrap_err();
    assert_eq!(err.event_index, 0);
    assert!(err.message.contains("semantics_rev"), "got: {}", err.message);
    assert!(replay(&record, &rules).is_ok(), "lenient replay warns but continues");
}

/// A stamped record survives a JSON round-trip and strict-replays clean with
/// its terminal-state digest verified against the replayed state.
#[test]
fn stamped_record_round_trips_and_strict_replay_verifies_the_terminal_digest() {
    let rules = Rules::standard();
    let (final_state, mut record) = common::run_recorded_game(7, 2, &rules);
    record.stamp_terminal_digest(&final_state);
    assert!(record.terminal_digest.is_some(), "stamp must set the digest");

    let json = serde_json::to_string(&record).unwrap();
    let back: GameRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(back.terminal_digest, record.terminal_digest, "digest survives round-trip");
    assert_eq!(back.semantics_rev, Some(SEMANTICS_REV), "semantics_rev survives round-trip");

    // Strict replay recomputes the digest from the replayed terminal state
    // and must accept the match.
    assert_eq!(replay_strict(&back, &rules).unwrap(), final_state);
}

/// Strict replay rejects a stored terminal digest that does not match the
/// state the event log folds to. The error is reported at the past-the-end
/// event index (the digest covers the whole run, not one event) and names
/// the terminal-state digest. Lenient replay does NOT check it (strict-only,
/// like the invariant checks).
#[test]
fn strict_replay_rejects_a_wrong_terminal_digest() {
    let rules = Rules::standard();
    let (final_state, mut record) = common::run_recorded_game(7, 2, &rules);
    record.stamp_terminal_digest(&final_state);
    // All-f is a valid 16-hex digest that cannot match the real one (2^-64).
    record.terminal_digest = Some("ffffffffffffffff".into());
    let err = replay_strict(&record, &rules).unwrap_err();
    assert_eq!(err.event_index, record.events.len(), "digest failure is after the last event");
    assert!(err.message.contains("terminal-state digest"), "got: {}", err.message);
    assert!(replay(&record, &rules).is_ok(), "lenient replay does not verify the digest");
}

/// The terminal digest catches an events-vs-digest divergence that the
/// per-event invariant checks do NOT: dropping the last event leaves a record
/// whose events fold to a perfectly valid EARLIER state, yet the stored digest
/// still describes the full run. Only the digest comparison flags it.
#[test]
fn terminal_digest_catches_a_valid_but_wrong_terminal_state() {
    let rules = Rules::standard();
    let (final_state, mut record) = common::run_recorded_game(7, 2, &rules);
    assert!(record.events.len() >= 2, "need >=2 events to drop one meaningfully");
    record.stamp_terminal_digest(&final_state);

    // Drop the last event: the remaining prefix folds to a real, invariant-
    // clean mid-game state -- different from `final_state`, so its digest
    // differs from the one stamped for the full run.
    record.events.pop();
    let truncated = replay(&record, &rules).expect("a prefix of a real game replays cleanly");
    assert_ne!(
        state_digest(&truncated),
        record.terminal_digest.clone().unwrap(),
        "fixture: the dropped event must actually change the terminal state"
    );

    // Every prefix event is invariant-clean, so strict replay reaches the
    // terminal-digest check and fails THERE (not on an invariant).
    let err = replay_strict(&record, &rules).unwrap_err();
    assert!(err.message.contains("terminal-state digest"), "got: {}", err.message);
}

/// Records written before these fields existed (no `semantics_rev`, no
/// `terminal_digest` key) still deserialize -- but with `semantics_rev: None`
/// (UNKNOWN provenance), not a fabricated rev (Codex PR-3 review, FIX-B P1:
/// the old serde-default of literal 1 made a pre-Phase-2.9 record
/// indistinguishable from a genuine current rev-1 record, so `replay_strict`
/// accepted it). `terminal_digest` still defaults to `None`.
#[test]
fn pre_field_records_deserialize_with_no_semantics_rev_and_no_digest() {
    let rules = Rules::standard();
    let (_, record) = common::run_recorded_game(7, 2, &rules);
    let mut v = serde_json::to_value(&record).unwrap();
    let obj = v.as_object_mut().unwrap();
    obj.remove("semantics_rev");
    obj.remove("terminal_digest");
    let restored: GameRecord = serde_json::from_value(v).unwrap();
    assert_eq!(restored.semantics_rev, None, "a missing semantics_rev key is UNKNOWN provenance, not rev 1");
    assert_eq!(restored.terminal_digest, None);
}

/// FIX-B P1 (Codex PR-3 review): a record whose JSON carries NO
/// `semantics_rev` key has UNKNOWN semantics provenance -- strict replay
/// (the training-record path) must fail closed instead of silently treating
/// it as a current record. Lenient `replay` keeps its warn-and-continue
/// contract: it warns about the missing provenance and replays anyway.
#[test]
fn strict_replay_rejects_a_record_missing_semantics_provenance() {
    let rules = Rules::standard();
    let (_, record) = common::run_recorded_game(7, 2, &rules);
    let mut v = serde_json::to_value(&record).unwrap();
    v.as_object_mut().unwrap().remove("semantics_rev");
    let restored: GameRecord = serde_json::from_value(v).unwrap();

    let err = replay_strict(&restored, &rules).unwrap_err();
    assert_eq!(err.event_index, 0);
    assert!(err.message.contains("semantics"), "got: {}", err.message);
    // Lenient contract unchanged: warns to stderr but still replays.
    assert!(replay(&restored, &rules).is_ok(), "lenient replay warns but continues");
}
