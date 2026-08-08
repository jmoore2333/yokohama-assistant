//! BGA importer integration pins against the REAL corpus (Phase 3.5b H1).
//!
//! These tests read the gitignored capture `bridge-captures/895724349.ndjson`
//! by path (the always-run tier lives in the `bga` module's unit tests against
//! the committed redacted excerpt — the split the plan §5 states). What only
//! the full corpus can pin: the mapper covering every card the game surfaced,
//! the sheet synthesizing from the true 194-move feed, the m153 acceptance
//! replay, and the follower matching the batch byte-for-byte.
//!
//! Absence of the corpus is a LOUD failure by default (F6 / six-lens lens-4+6):
//! `corpus_text` PANICS with a named, actionable message, because a
//! `libtest`-swallowed `eprintln!` on a passing test made a corpus-less run
//! report these pins as `ok` while they asserted nothing — indistinguishable
//! from a real pass, so the reported gate figures carried no information about
//! whether the tier ran. Set `YOKOHAMA_BGA_CORPUS_OPTIONAL=1` to turn absence
//! back into a quiet skip (the documented opt-out for a fresh clone / CI
//! without the owner's capture; named in the 3.5b ledger's gates row).

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
    // Redaction rule (owner ruling, 2026-08-08): tracked files never carry the
    // real BGA display names — the corpus is the source of truth for them, and
    // what a tracked pin may assert is SHAPE: two distinct, non-empty names.
    // (The committed excerpt fixture uses the Player_A/Player_B placeholders.)
    assert!(!a.seats[0].name.is_empty() && !a.seats[1].name.is_empty());
    assert_ne!(a.seats[0].name, a.seats[1].name);
    // Start player: the first assistantPlacement (move 4) is seat 0's.
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

// ---------------------------------------------------------------------------
// H2: the acceptance replay (census + T8/T9/T10)
// ---------------------------------------------------------------------------

/// The acceptance replay of the full corpus, at rev 6 run to the TRUE END:
/// all 194 BGA moves replay engine-legal from move 1 — the m154 one-leg
/// to-hand assistant move that bounded rev 5 now translates to the engine's
/// `MoveTarget::Hand` (owner ruling D3) — the hypothesis loop resolves the two
/// claimed achievements to Seafood7/Tokens5 (the other three candidates REFUSE
/// at their claim moves), the engine reaches game end, and the finalScoring
/// cross-check MATCHES per seat (BGA: 154 / 41; the Dummy's 9, its D5
/// tech-majority 5 included, is BGA-only context — the engine reports seat
/// scores). Any NEW divergence this replay surfaces is a STOP-AND-DOCUMENT
/// owner item, never a translator fudge — which is why every assertion here is
/// exact.
#[test]
fn the_acceptance_replay_completes_all_194_moves_and_the_final_scores_match() {
    let Some(_) = corpus_text() else { return };
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../bridge-captures/895724349.ndjson"
    );
    let args = yokohama_cli::bga::import::parse_args(&[path.to_string()]).unwrap();
    let outcome = yokohama_cli::bga::import::run(&args, Rules::standard()).unwrap();
    assert!(outcome.session_built, "{}", outcome.text);
    assert!(
        outcome.replay_complete,
        "the rev-6 replay must run the whole corpus to the engine's game end: {}",
        outcome.text
    );
    assert!(
        outcome.text.contains("progressed through BGA move 194"),
        "{}",
        outcome.text
    );
    // The final-score cross-check, per seat, exact (the honesty dividend).
    assert!(
        outcome.text.contains(": 154 == BGA 154 ✓"),
        "seat 0 must match BGA's 154: {}",
        outcome.text
    );
    assert!(
        outcome.text.contains(": 41 == BGA 41 ✓"),
        "seat 1 must match BGA's 41: {}",
        outcome.text
    );
    assert!(
        !outcome.text.contains("MISMATCH"),
        "no seat may mismatch: {}",
        outcome.text
    );
    // The hypothesis loop's survivors-by-elimination: the wrong candidates
    // refuse at their claim moves (m43 / m90).
    assert!(outcome.text.contains("[A=Seafood7, B=Tokens5]"), "{}", outcome.text);
    assert!(
        outcome.text.contains("could not be fulfilled as Tea7"),
        "{}",
        outcome.text
    );
    assert!(
        outcome.text.contains("could not be fulfilled as Orders5"),
        "{}",
        outcome.text
    );
    // The rev-5 D1/D2 offset notes are GONE at rev 6 — the zero-offsets pin
    // (`the_rev6_corpus_replays_with_zero_declared_offsets`) owns that claim.
}

/// THE REV-6 PROOF (2026-08-08 follow-up): the engine now agrees with BGA on
/// D1 (church yen price 2) and D2 (extra-tech surcharge from the 4th card), so
/// the corpus checkpoints MUST pass RAW — zero declared offsets, zero OPEN
/// RULES QUESTION notes, zero T9 counter divergences. This passing is the
/// executable proof the rev-6 rulings landed: before the offset machinery was
/// deleted, the same run either absorbed the divergences into declared offsets
/// (rev 5) or STOPPED at the first checkpoint the now-wrong offset broke
/// (rev 6 with the machinery still present — measured: m107).
#[test]
fn the_rev6_corpus_replays_with_zero_declared_offsets() {
    let Some(_) = corpus_text() else { return };
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../bridge-captures/895724349.ndjson"
    );
    let args = yokohama_cli::bga::import::parse_args(&[path.to_string()]).unwrap();
    let outcome = yokohama_cli::bga::import::run(&args, Rules::standard()).unwrap();
    assert!(outcome.session_built, "{}", outcome.text);
    assert!(
        !outcome.text.contains("OPEN RULES QUESTION"),
        "rev 6 retires the D1/D2 rules questions — no note may declare one: {}",
        outcome.text
    );
    assert!(
        !outcome.text.contains("offset tracked") && !outcome.text.contains("engine-side offset"),
        "no offset may be declared on the rev-6 run: {}",
        outcome.text
    );
    assert!(
        !outcome.text.contains("T9 counter divergence"),
        "every checkpoint must pass RAW (no offset applied, none needed): {}",
        outcome.text
    );
    let progressed: u64 = outcome
        .text
        .lines()
        .find_map(|l| {
            l.trim()
                .strip_prefix("progressed through BGA move ")
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|n| n.parse().ok())
        })
        .expect("the report states its progress");
    assert!(
        progressed >= 153,
        "the raw-checkpoint replay must get at least as far as rev 5's offset-assisted one: {}",
        outcome.text
    );
}

/// The corpus negative behind the zero-offsets proof: perturb the m120 socket
/// checkpoint's `yens` counter and the replay must STOP on the T9 counter
/// divergence — with the D1/D2 offset machinery deleted there is NOTHING left
/// that could absorb a counter divergence, at the 4th-tech move or anywhere
/// else. (This test's rev-5 ancestor pinned the F3 vocabulary bound of the
/// then-live D2 absorber; the absorber is gone, so the pin is now simply that
/// divergence means STOP.)
#[test]
fn a_perturbed_checkpoint_counter_stops_the_replay_nothing_absorbs_it() {
    use yokohama_cli::bga::facts;
    let Some(text) = corpus_text() else { return };
    let rules = Rules::standard();
    let feed = read_and_collapse(&text, None).unwrap();
    let answers = synthesize(&feed, &rules).unwrap();
    let mapper = OrderMapper::from_rules(&rules).unwrap();
    let seats = answers.seats.clone();
    let mut stream = facts::extract(
        &feed,
        &|pid| seats.iter().find(|s| s.bga_player_id == pid).map(|s| s.seat),
        &mapper,
    )
    .unwrap();
    // Perturb Player_A's (seat 0) yens at the m120 checkpoint by +5. Its yens
    // matched before (rev 6 agrees with BGA raw), so this manufactures a
    // divergence no engine state in the turn window can satisfy.
    let cp = stream
        .checkpoints
        .iter_mut()
        .find(|c| c.move_id == 120)
        .expect("the corpus has a socket checkpoint at m120");
    let pid = answers.seats[0].bga_player_id.clone();
    let counters = cp.per_player.get_mut(&pid).expect("seat 0 counters at m120");
    let obj = counters.as_object_mut().expect("counters object");
    let bga_yens = obj.get("yens").and_then(yokohama_bridge::feed::flex_u64).expect("yens present");
    obj.insert("yens".to_string(), serde_json::json!(bga_yens + 5));

    let mut resolved = yokohama_cli::bga::resolve::resolve(answers, &feed, &stream, &rules).unwrap();
    // Pin the acceptance's SURVIVING hypothesis assignment (A=Seafood7,
    // B=Tokens5) so the replay clears m43/m90 and actually REACHES the m120
    // 4th-tech move — otherwise it STOPs on the wrong candidate long before D2.
    // (`replay_attempts` finds this by elimination; the corpus makes it fixed.)
    use yokohama_cli::bga::setup::Evidence;
    use yokohama_core::AchievementType;
    resolved.answers.achievements[0] = Evidence::Evidenced {
        value: AchievementType::Seafood7,
        provenance: "test: the acceptance's surviving hypothesis A".to_string(),
    };
    resolved.answers.achievements[1] = Evidence::Evidenced {
        value: AchievementType::Tokens5,
        provenance: "test: the acceptance's surviving hypothesis B".to_string(),
    };
    let session = yokohama_cli::bga::setup::build_session(&resolved.answers, rules).unwrap();
    let mut t = yokohama_cli::bga::translate::Translator::new(session, &resolved, stream, &feed).unwrap();
    let detail = t
        .run()
        .expect_err("a perturbed checkpoint counter must STOP the replay, not be absorbed")
        .to_string();
    assert!(
        detail.contains("T9 counter divergence"),
        "the STOP is the T9 checkpoint cross-check: {detail}"
    );
    assert!(detail.contains("yens"), "the STOP names the diverging counter: {detail}");
}

/// H3's CORPUS-STREAMED-AS-LIVE proof (plan §6 / the H3 brief, requirement 4),
/// at rev 6 run to COMPLETION: the real corpus appended to a temp file in raw
/// byte chunks — boundaries cutting JSON lines wherever they fall — while the
/// follower polls, with a KILL-AND-RESTART in the middle, must reach a record
/// BYTE-IDENTICAL to the batch import of the same file, and must EXIT on its
/// own at game end (the one spontaneous exit the design allows) with the
/// final-score cross-check in the completion frame. The rev-5 ancestor pinned
/// the m154 hold instead; the D3 ruling landed, so no ENGINE-RULES divergence
/// may print anywhere in the stream. Transient mid-stream HOLDs (an ingested
/// prefix ending inside a turn window — H3 review N2) remain legitimate; what
/// is asserted is completion, not hold-freedom.
#[test]
fn the_follower_streams_the_corpus_to_completion_and_matches_the_batch() {
    use yokohama_cli::bga::follow::{FollowOptions, Follower};
    let Some(_) = corpus_text() else { return };
    let corpus = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../bridge-captures/895724349.ndjson"
    );
    let dir = std::env::temp_dir().join(format!("bga-h3-stream-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    // The batch reference: the full-corpus import, complete, saved.
    let batch_out = dir.join("batch.json");
    let args = yokohama_cli::bga::import::parse_args(&[
        corpus.to_string(),
        "--out".to_string(),
        batch_out.display().to_string(),
    ])
    .unwrap();
    let outcome = yokohama_cli::bga::import::run(&args, Rules::standard()).unwrap();
    assert!(outcome.replay_complete, "{}", outcome.text);
    assert!(outcome.text.contains("progressed through BGA move 194"), "{}", outcome.text);

    // The live stream: raw 80KB chunks, a restart after chunk 10.
    let live = dir.join("live.ndjson");
    let follow_out = dir.join("follow.json");
    let mk = || {
        Follower::new(
            FollowOptions { path: live.clone(), table: None, out: Some(follow_out.clone()) },
            Rules::standard(),
        )
    };
    let bytes = std::fs::read(corpus).unwrap();
    let mut follower = mk();
    let mut frames_after_restart: Vec<String> = Vec::new();
    let mut completed = false;
    for (i, chunk) in bytes.chunks(80_000).enumerate() {
        use std::io::Write;
        if i == 10 {
            // The kill: a fresh follower re-ingests the same file (dedupe
            // makes that idempotent) and continues where the game is.
            follower = mk();
        }
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&live).unwrap();
        f.write_all(chunk).unwrap();
        drop(f);
        let p = follower.poll();
        if let Some(frame) = p.frame {
            if i >= 10 {
                frames_after_restart.push(frame);
            }
        }
        completed |= p.complete;
    }
    assert!(completed, "the completed corpus must read as completion");

    // No frame anywhere may print the retired rules-gap divergence.
    assert!(
        frames_after_restart.iter().all(|f| !f.contains("ENGINE-RULES divergence")),
        "the D3 ruling landed — no rules-gap hold may print"
    );

    // The completion frame: the final-score cross-check rides the exit.
    let last = frames_after_restart.last().expect("a completion frame rendered");
    assert!(last.contains("replayed through BGA move 194"), "{last}");
    assert!(last.contains(": 154 == BGA 154 ✓"), "{last}");
    assert!(last.contains(": 41 == BGA 41 ✓"), "{last}");
    assert!(last.contains("game complete"), "{last}");

    // The byte-identity: streamed-as-live == batch, record and digest alike.
    assert_eq!(
        std::fs::read(&batch_out).unwrap(),
        std::fs::read(&follow_out).unwrap(),
        "the streamed follower's record differs from the batch import's"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The bounded run (`--through 153`) completes cleanly — 32 checkpoints green
/// on exactly one surviving hypothesis — and its emitted record LOADS, which
/// is a full replay + invariants + terminal-digest verification in
/// `Session::load`.
#[test]
fn the_bounded_replay_emits_a_record_that_reloads_clean() {
    let Some(_) = corpus_text() else { return };
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../bridge-captures/895724349.ndjson"
    );
    let dir = std::env::temp_dir().join(format!("bga-h2-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("bga-895724349-through153.json");
    let args = yokohama_cli::bga::import::parse_args(&[
        path.to_string(),
        "--through".to_string(),
        "153".to_string(),
        "--out".to_string(),
        out.display().to_string(),
    ])
    .unwrap();
    let outcome = yokohama_cli::bga::import::run(&args, Rules::standard()).unwrap();
    assert!(outcome.text.contains("ran to the end of the captured stream"), "{}", outcome.text);
    // Left-ANCHORED on ", 32": a bare `contains("32 checkpoints green")` is
    // satisfied by a mutated "132 checkpoints green" (import.rs's
    // `checkpoints_passed + 100`), so the count is anchored to the surrounding
    // line instead (F2 / lens-2's substring-vacuity recurrence class).
    assert!(
        outcome.text.contains("reveal events, 32 checkpoints green"),
        "{}",
        outcome.text
    );
    let session = yokohama_cli::session::Session::load(&out, Rules::standard())
        .expect("the emitted record replays clean on load");
    let record = session.record();
    assert_eq!(record.run.as_ref().unwrap().run_id.as_deref(), Some("bga-import/895724349"));
    assert!(record.events.len() > 240, "241 moves + reveals expected");
    std::fs::remove_dir_all(&dir).ok();
}
