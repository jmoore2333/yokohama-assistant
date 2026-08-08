//! BGA importer integration pins against the REAL corpus (Phase 3.5b H1).
//!
//! These tests read the gitignored capture `bridge-captures/895724349.ndjson`
//! by path and SKIP with a printed notice when it is absent (the always-run
//! tier lives in the `bga` module's unit tests against the committed redacted
//! excerpt — the split the plan §5 states). What only the full corpus can
//! pin: the mapper covering every card the game surfaced, and the sheet
//! synthesizing from the true 194-move feed.

use yokohama_bridge::feed::read_and_collapse;
use yokohama_cli::bga::orders::OrderMapper;
use yokohama_cli::bga::setup::synthesize;
use yokohama_core::Rules;

fn corpus_text() -> Option<String> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../bridge-captures/895724349.ndjson"
    );
    match std::fs::read_to_string(path) {
        Ok(t) => Some(t),
        Err(_) => {
            eprintln!("SKIPPED: gitignored corpus not present at {path}");
            None
        }
    }
}

#[test]
fn every_visible_order_the_corpus_surfaced_maps_cleanly() {
    let Some(text) = corpus_text() else { return };
    let feed = read_and_collapse(&text, None).unwrap();
    let mapper = OrderMapper::from_rules(&Rules::standard()).unwrap();

    // Every order object in notification args: the card-bearing arg keys the
    // inventory names. Deck rows (`location "1"`) are EXCLUDED — the §8.3
    // fence: identity is read only at reveal-shaped locations.
    let mut seen: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut mapped: std::collections::BTreeSet<u8> = std::collections::BTreeSet::new();
    for mv in &feed.moves {
        for pk in [mv.history.as_ref(), mv.socket.as_ref()].into_iter().flatten() {
            for n in &pk.notifications {
                let mut cards: Vec<&serde_json::Value> = Vec::new();
                for key in ["order", "order_card"] {
                    if let Some(c) = n.args.get(key) {
                        if c.is_object() {
                            cards.push(c);
                        }
                    }
                }
                if let Some(orders) = n.args.get("orders").and_then(|o| o.as_object()) {
                    cards.extend(orders.values().filter(|c| {
                        c.get("location").and_then(|l| l.as_str()) != Some("1")
                    }));
                }
                for card in cards {
                    let m = mapper
                        .map_row(card, &format!("{} at move {}", n.kind, mv.move_id))
                        .unwrap_or_else(|e| panic!("move {}: {e}", mv.move_id));
                    seen.insert(m.bga_order_id);
                    mapped.insert(m.card.0);
                }
            }
        }
    }
    // 19 of the game's 20 in-play cards surface outside the deck across the
    // whole corpus (the 20th, BGA order 2 / our O06, never left the deck —
    // deck_count was 1 at game end). Zero unmatched, zero ambiguous, or the
    // loop would have panicked.
    assert_eq!(seen.len(), 19, "distinct BGA order ids seen outside the deck: {seen:?}");
    assert_eq!(mapped.len(), 19);
}

#[test]
fn the_corpus_sheet_synthesizes_with_the_measured_facts() {
    let Some(text) = corpus_text() else { return };
    let feed = read_and_collapse(&text, None).unwrap();
    let a = synthesize(&feed, &Rules::standard()).unwrap();
    assert_eq!(a.table_id, 895_724_349);
    assert_eq!(a.player_count, 2);
    assert_eq!(a.seats[0].name, "XeroxMan");
    assert_eq!(a.seats[1].name, "acmuriel");
    // Start player: first assistantPlacement (move 4) is XeroxMan's.
    assert_eq!(a.start_player.value(), Some(&0));
    // Own seat: the socket /player/p88889596 channel.
    assert_eq!(a.own_seat.value(), Some(&0));
    // The board evidenced and topology-checked; ten areas at 2P.
    assert_eq!(a.board_tiles.value().map(|t| t.len()), Some(10));
    // The published discards: BGA 24 → O03 (seat 0), BGA 25 → O27 (seat 1).
    let discards: Vec<(u8, u8)> = a.orders_removed_known.iter().map(|d| (d.seat, d.card.0)).collect();
    assert_eq!(discards, vec![(0, 3), (1, 27)]);
    // The earliest snapshot is move 16 — mid-game, so the initial Port/Lab
    // stay Absent (plan §0.5) and no Session can be built from this corpus.
    assert_eq!(a.snapshot_move_nbr, 16);
    assert!(a.port_a.value().is_none());
    assert!(a.lab_a.value().is_none());
    // The fence in numbers: the snapshot's 8 undrawn orders are a count.
    assert_eq!(a.deck_order_count, 8);
}
