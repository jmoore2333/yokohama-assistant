//! Feed reader/collapse pins against REAL captured data (Phase 3.5b T1+T2).
//!
//! Two tiers, per the plan's §5 fixture split:
//!
//! - **The committed excerpt** (`tests/data/excerpt-895724349.ndjson`): 16 real
//!   capture lines from table #895724349, redacted (player ids and display
//!   names replaced by stable placeholders), covering every envelope arm —
//!   an xhr history slice, the full undo bracket (packets 121–126), a
//!   duplicate delivery, a moveless `wakeupPlayers` packet, one of each chrome
//!   frame, and a trimmed `gamedatas` snapshot. These tests ALWAYS run, so a
//!   fresh clone without the gitignored corpus still exercises the parser on
//!   real wire shapes.
//! - **The full corpus** (`bridge-captures/895724349.ndjson`, gitignored):
//!   the complete 194-move game. Tests against it are the distributional pins
//!   (194-move spine, all five supersessions, dedupe rates) that only the full
//!   file can provide. Absence is a LOUD failure by default — `corpus_text`
//!   PANICS with a named, actionable message, because a `libtest`-swallowed
//!   `eprintln!` on a passing test made a corpus-less run report these pins as
//!   `ok` while they asserted nothing, indistinguishable from a real pass. Set
//!   `YOKOHAMA_BGA_CORPUS_OPTIONAL=1` to turn absence back into a quiet skip
//!   (the documented opt-out for a fresh clone / CI without the owner's
//!   capture; named in the 3.5b ledger's gates row).

use yokohama_bridge::feed::{read_and_collapse, read_captures, Transport};

const EXCERPT: &str = include_str!("data/excerpt-895724349.ndjson");
const TABLE: u64 = 895_724_349;

/// The gitignored full-corpus capture. Absence PANICS with a named, actionable
/// message by default (F6 / lens-4+6), so a corpus-less run reports these pins
/// as FAILED rather than a swallowed-`eprintln!` `ok`. `YOKOHAMA_BGA_CORPUS_OPTIONAL=1`
/// turns absence back into a quiet skip for a fresh clone / CI.
fn corpus_text() -> Option<String> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../bridge-captures/895724349.ndjson"
    );
    match std::fs::read_to_string(path) {
        Ok(t) => Some(t),
        Err(_) if std::env::var_os("YOKOHAMA_BGA_CORPUS_OPTIONAL").is_some() => {
            eprintln!("SKIPPED (YOKOHAMA_BGA_CORPUS_OPTIONAL set): corpus not present at {path}");
            None
        }
        Err(e) => panic!(
            "the gitignored corpus is not present at {path} ({e}).\n\
             These pins verify the phase's headline acceptance figures against the real 194-move \
             capture; a silent skip would make 'verified' indistinguishable from 'file missing'. \
             Provide the capture, or set YOKOHAMA_BGA_CORPUS_OPTIONAL=1 to skip this tier deliberately."
        ),
    }
}

// ---- the committed excerpt (always runs) -----------------------------------

#[test]
fn the_excerpt_parses_with_every_envelope_arm_represented() {
    let feed = read_captures(EXCERPT).unwrap();
    assert_eq!(feed.snapshots.len(), 1, "one trimmed snapshot");
    assert!(feed.packets.iter().any(|p| p.transport == Transport::History));
    assert!(feed.packets.iter().any(|p| p.transport == Transport::Socket));
    assert_eq!(feed.chrome.heartbeat_frames, 1);
    assert_eq!(feed.chrome.handshake_frames, 1);
    assert_eq!(feed.chrome.single_packets, 1);
    // The own-seat evidence channel survived redaction with the placeholder id.
    assert!(feed.chrome.player_channels.contains("/player/p11111111"));
}

#[test]
fn the_excerpt_collapses_the_real_undo_bracket() {
    let collapsed = read_and_collapse(EXCERPT, Some(TABLE)).unwrap();
    // The bracket: undo packet 124 (prevpacket [121]) supersedes 122 and 123.
    assert_eq!(collapsed.report.superseded.len(), 1);
    let ev = &collapsed.report.superseded[0];
    assert_eq!(ev.undo_packet_id, 124);
    assert_eq!(ev.undo_move_id, Some(117));
    assert_eq!(ev.superseded_packet_ids, vec![122, 123]);
    // The duplicated delivery of packet 122 was dropped as a duplicate
    // (anti-vacuity: a dedupe that never deduped is not evidence).
    assert_eq!(collapsed.report.socket_duplicates_dropped, 1);
    // Moveless wakeupPlayers packets dropped on both transports (packet 177).
    assert_eq!(collapsed.report.moveless_dropped_history, 1);
    assert_eq!(collapsed.report.moveless_dropped_socket, 1);
    // History moves 1..8 + socket moves 116..119 (the surviving bracket).
    let mids: Vec<u64> = collapsed.moves.iter().map(|m| m.move_id).collect();
    assert_eq!(mids, vec![1, 2, 3, 4, 5, 6, 7, 8, 116, 117, 118, 119]);
}

#[test]
fn the_excerpt_carries_the_two_order_discards_in_full() {
    // The two orderDiscarded notifications (moves 2 and 3) name the DISCARDED
    // card in full — the setup importer's `orders_removed` evidence.
    let collapsed = read_and_collapse(EXCERPT, Some(TABLE)).unwrap();
    let discards: Vec<(u64, String)> = collapsed
        .moves
        .iter()
        .filter_map(|m| m.history.as_ref())
        .flat_map(|p| p.notifications.iter().map(|n| (p.move_id.unwrap(), n)))
        .filter(|(_, n)| n.kind == "orderDiscarded")
        .map(|(mid, n)| {
            (
                mid,
                n.args["order"]["order_id"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(
        discards,
        vec![(2, "24".to_string()), (3, "25".to_string())],
        "BGA order 24 (engine O03) and 25 (engine O27), one per seat"
    );
}

#[test]
fn the_excerpt_snapshot_is_the_mid_game_gamedatas() {
    let collapsed = read_and_collapse(EXCERPT, Some(TABLE)).unwrap();
    let snap = collapsed.earliest_snapshot().expect("snapshot present");
    let gd = &snap.gamedatas;
    assert_eq!(gd["tile_count"].as_u64(), Some(10), "2P board");
    assert_eq!(gd["gamestate"]["name"].as_str(), Some("placeAssistantsStep"));
    assert_eq!(gd["tiles"].as_object().unwrap().len(), 10);
    assert_eq!(gd["achievements"].as_object().unwrap().len(), 3);
}

// ---- the full corpus (skips loudly when absent) ----------------------------

#[test]
fn the_corpus_collapses_to_the_full_194_move_spine() {
    let Some(text) = corpus_text() else { return };
    let collapsed = read_and_collapse(&text, None).unwrap();
    assert_eq!(collapsed.table_id, TABLE);
    assert_eq!(collapsed.report.moves, 194);
    assert_eq!(collapsed.moves.first().unwrap().move_id, 1);
    assert_eq!(collapsed.moves.last().unwrap().move_id, 194);
    assert!(
        collapsed.report.missing_move_ids.is_empty(),
        "the four overlapping history fetches cover 1..194 with no gaps"
    );
    assert_eq!(collapsed.report.snapshots, 3);
}

#[test]
fn the_corpus_supersessions_are_exactly_the_five_measured_undos() {
    let Some(text) = corpus_text() else { return };
    let collapsed = read_and_collapse(&text, None).unwrap();
    let events: Vec<(u64, Vec<u64>)> = collapsed
        .report
        .superseded
        .iter()
        .map(|e| (e.undo_packet_id, e.superseded_packet_ids.clone()))
        .collect();
    // Inventory §4.1's table, verbatim. Note undos 149 and 170 supersede ONE
    // packet each — the shape a move_id-regression detector misses.
    assert_eq!(
        events,
        vec![
            (124, vec![122, 123]),
            (131, vec![129, 130]),
            (149, vec![148]),
            (159, vec![157, 158]),
            (170, vec![169]),
        ]
    );
}

#[test]
fn the_corpus_dedupe_rates_match_the_measured_delivery_multiplicities() {
    let Some(text) = corpus_text() else { return };
    let collapsed = read_and_collapse(&text, None).unwrap();
    let r = &collapsed.report;
    // History: 310 raw over four overlapping fetches, 199 distinct.
    assert_eq!(r.history_packets_raw, 310);
    assert_eq!(r.history_duplicates_dropped, 111);
    // Socket: every packet delivered exactly twice (two sockets).
    assert_eq!(r.socket_packets_raw, 202);
    assert_eq!(r.socket_duplicates_dropped, 101);
    // Anti-vacuity: the dedupe demonstrably deduped.
    assert!(r.history_duplicates_dropped > 0 && r.socket_duplicates_dropped > 0);
    // Moveless chrome packets (177/188/199/200/209) on both transports.
    assert_eq!(r.moveless_dropped_history, 5);
    assert_eq!(r.moveless_dropped_socket, 5);
}

#[test]
fn the_corpus_socket_window_enriches_the_88_covered_moves() {
    let Some(text) = corpus_text() else { return };
    let collapsed = read_and_collapse(&text, None).unwrap();
    // Surviving socket packets with a move id cover moves 107..194 — and the
    // merge check (history == socket filtered to shared kinds, on (kind, uid))
    // held on every one, or collapse would have errored.
    assert_eq!(collapsed.report.enriched, 88);
    let enriched: Vec<u64> = collapsed
        .moves
        .iter()
        .filter(|m| m.socket.is_some() && m.history.is_some())
        .map(|m| m.move_id)
        .collect();
    assert_eq!(enriched.first(), Some(&107));
    assert_eq!(enriched.last(), Some(&194));
}

#[test]
fn the_corpus_chrome_tally_matches_the_inventory_census() {
    let Some(text) = corpus_text() else { return };
    let feed = read_captures(&text).unwrap();
    assert_eq!(feed.chrome.heartbeat_frames, 128);
    // 104 `single` packets on the TABLE channel; the inventory's 124 raw
    // singles include the 12 `/player/…` and 8 `/chat/…` pubs, which this
    // tally counts as other_channel_pubs instead (they never reach the
    // packet-type switch — channel routing comes first).
    assert_eq!(feed.chrome.single_packets, 104);
    assert_eq!(feed.chrome.other_channel_pubs, 20);
    // The capturing session's own player channel — the own-seat evidence.
    assert!(feed.chrome.player_channels.contains("/player/p88889596"));
}
