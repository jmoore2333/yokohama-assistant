//! The determinism pin (M6.5b, spec §6.4; umbrella §2.5, O-3, O-11).
//!
//! **One pair, played by the wasm bot in a REAL browser, reconstructed here
//! natively.** The corpus replay (§6.3) proves the WALK across the compilation
//! boundary and says so: it is bot-free by construction. This is the other
//! half — that the bot the module runs in a browser is the bot this workspace
//! builds, move for move — and it is the only instrument in the cycle that can
//! see the two disagree.
//!
//! The pair is the RECORD and its LOCAL TABLE DOCUMENT, which is exactly what
//! "Send to the archive" sends. It **carries a seed, deliberately** — it is a
//! record, and the seed is what the replay needs — and it lives HERE rather
//! than in `web/src/wire/`, the one directory the corpus's seed value-pin
//! scans (`wire_fixtures.rs`'s `corpus()`), so that fence cuts as intended in
//! both directions. Nobody should move this pair into the corpus.
//!
//! **Regeneration is one command**, run from `web/`:
//!
//! ```text
//! YOKOHAMA_WEB_UPDATE_PIN=1 npm run e2e -- plays_locally
//! ```
//!
//! which plays a whole local game in Chromium against the module the same
//! command builds, and rewrites the three files beside this one — the record,
//! the table and the sidecar naming the release, the date, the browser and the
//! module's hash. A regenerated pair is a NEW game: the seed is drawn in the
//! browser and nothing here is tuned to it.

use std::sync::Arc;
use std::time::Instant;

use yokohama_web::api::{dispatch, ApiRequest, Host};
use yokohama_web::archive::NoArchive;
use yokohama_web::chat::ChatConfig;
use yokohama_web::games::Games;
use yokohama_web::identity::User;
use yokohama_web::signal::NoSignal;
use yokohama_web::store::{Doc, MemStore};

const RECORD: &str = include_str!("fixtures/browser-played-2p.json");
const TABLE: &str = include_str!("fixtures/browser-played-2p.table.json");
const SIDECAR: &str = include_str!("fixtures/browser-played-2p.provenance.json");

/// The one address a local game is ever played by (`web/src/local/ids.ts`'s
/// `LOCAL_USER`): the module answers as it whatever door the browser signed in
/// at, so it is the caller `why` has to be asked as.
const LOCAL_USER: &str = "local@browser";

/// The browser host's own deployment facts (`crates/yokohama-wasm`'s `HOST`),
/// restated here because the pin drives the layer the way that host does and a
/// `why` answered under another host's facts would not be the same call —
/// `serves_games` above all, since with it false there would be no `why` route
/// to drive (M6.5b2).
const HOST: Host =
    Host { identity_name: "local", signal_kind: "none", serves_games: true, multiplayer_offered: false };

/// The layer over a `MemStore` with the browser host's own parts, holding the
/// committed pair.
fn host_with_the_pair() -> (Games, String) {
    let games = Games::with_parts(
        Arc::new(yokohama_core::Rules::standard()),
        Arc::new(MemStore::default()),
        ChatConfig::default(),
        Arc::new(NoSignal),
        Arc::new(|| 1_789_000_000),
        Arc::new(NoArchive),
        yokohama_web::games::RunHost::Browser,
    );
    let doc: yokohama_web::table::TableDoc =
        serde_json::from_str(TABLE).expect("the committed table document parses");
    let id = doc.id.clone();
    games.load_document(Doc::Record, &id, RECORD).expect("the record loads");
    games.load_document(Doc::Table, &id, TABLE).expect("the table loads");
    (games, id)
}

/// The committed pair is what it says it is: one human seat, one bot seat, and
/// a table document that AGREES with the record.
///
/// **The `agrees_with` arm spec §6.1 asks for** (N-4): the five refusal cells
/// beside it in `table.rs` are all synthetic, and a rule can refuse every
/// synthetic defect and still refuse the only real input there is. This is the
/// real input.
#[test]
fn the_committed_pair_is_one_human_one_bot_and_the_table_agrees_with_the_record() {
    let record: yokohama_core::GameRecord =
        serde_json::from_str(RECORD).expect("the committed record parses");
    let doc: yokohama_web::table::TableDoc =
        serde_json::from_str(TABLE).expect("the committed table document parses");

    doc.agrees_with(&record).expect("the local table document agrees with the record it was sent with");

    let run = record.run.as_ref().expect("a browser-played record carries its run provenance");
    let controllers: Vec<&str> = run.seats.iter().map(|s| s.controller.as_str()).collect();
    assert_eq!(controllers.len(), 2, "O-11: the pair is 2P, {controllers:?}");
    assert_eq!(
        controllers[0],
        yokohama_core::CONTROLLER_HUMAN,
        "seat 0 is the person at the browser"
    );
    assert_ne!(
        controllers[1],
        yokohama_core::CONTROLLER_HUMAN,
        "seat 1 is the bot — a pair with no bot in it would reconstruct nothing"
    );
    assert_eq!(
        doc.keeps.iter().filter(|k| k.is_some()).count(),
        1,
        "one human keep and the bot's is the record's: {:?}",
        doc.keeps
    );
    assert!(record.terminal_digest.is_some(), "a finished record is stamped");
    assert_eq!(record.semantics_rev, Some(yokohama_core::SEMANTICS_REV));

    // The sidecar is provenance and is READ, so a pair regenerated without one
    // — or with one that names no module — is a failure here rather than a
    // file nobody opens.
    let side: serde_json::Value = serde_json::from_str(SIDECAR).expect("the sidecar parses");
    for key in ["release", "date", "browser", "module_sha256"] {
        assert!(
            side.get(key).and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty()),
            "the sidecar names {key}: {side}"
        );
    }
}

/// **Every event of a browser-played game is ACCOUNTED FOR by the native
/// `why`**: each one is either a bot move this build reconstructs, or the
/// route's own typed refusal saying the mover was a human seat (or that the
/// event was not a move at all).
///
/// A reconstruction that does not equal the recorded move REFUSES — that is
/// `why`'s honesty gate, and it is what makes this a determinism claim rather
/// than a "the route answered" claim. So the assertion is that no bot event
/// refuses, and the two counts below are what keep it from passing vacuously
/// on a record whose every event was classified away.
///
/// The fifteen counters ride every granted answer and `placement_cap_binds` is
/// read at zero as the scope partner (spec §6.4): a search that started binding
/// its placement cap would be a different search under the same name.
#[test]
fn the_browser_played_pair_reconstructs_under_the_native_why() {
    let (games, id) = host_with_the_pair();
    let record: yokohama_core::GameRecord =
        serde_json::from_str(RECORD).expect("the committed record parses");
    let user = User::parse(LOCAL_USER).expect("the local address parses");

    // **O-3, RECORDED and never gated**: what a cold rebuild of this record
    // costs natively, beside the browser's own figure for the same record
    // (the e2e's reload cell prints the Worker's).
    let rules = yokohama_core::Rules::standard();
    let t = Instant::now();
    let session = yokohama_cli::session::Session::from_record(record.clone(), rules)
        .expect("the browser-played record rebuilds");
    let wall = t.elapsed();
    eprintln!(
        "O-3 (native): Session::from_record over {} events took {:?}",
        record.events.len(),
        wall
    );
    assert_eq!(session.record().events.len(), record.events.len());

    let mut reconstructed = 0usize;
    let mut human = 0usize;
    let mut not_moves = 0usize;
    for e in 0..record.events.len() {
        let path = format!("/api/games/{id}/why");
        let query = format!("event={e}");
        let req = ApiRequest {
            method: "GET",
            path: &path,
            query: &query,
            content_type: None,
            body: b"",
        };
        let res = dispatch(&games, &HOST, Ok(&user), &req);
        assert_eq!(res.status, 200, "event {e}: {}", res.body);
        let v: serde_json::Value = serde_json::from_str(&res.body).expect("why answers JSON");
        match v["refusal"].as_object() {
            None => {
                let counters = v["ok"]["counters"]
                    .as_array()
                    .unwrap_or_else(|| panic!("event {e}: a granted why carries its counters"));
                let row = counters
                    .iter()
                    .find(|c| c["name"] == "placement_cap_binds")
                    .unwrap_or_else(|| panic!("event {e}: no placement_cap_binds counter"));
                assert_eq!(row["value"], 0, "event {e}: the scope partner");
                assert_eq!(counters.len(), 15, "event {e}: the fifteen counters");
                reconstructed += 1;
            }
            // The two refusals a healthy record MAY carry, each named: the
            // human's own moves, and the setup reveals that are not moves at
            // all. Every other refusal — a mismatch above all, which is the
            // one that would mean the browser's bot and this one disagree —
            // fails here.
            Some(r) => match r["kind"].as_str() {
                Some("human_seat") => human += 1,
                Some("not_a_move") => not_moves += 1,
                _ => panic!("event {e} was REFUSED: {}", res.body),
            },
        }
    }

    eprintln!(
        "the browser-played pair: {reconstructed} bot events reconstructed, \
         {human} human, {not_moves} not moves, of {} in {:?}",
        record.events.len(),
        t.elapsed()
    );
    // Anti-vacuity, both ways. A record whose every event came back
    // `human_seat` would pass a "no refusal" assertion and prove nothing, and
    // a record with no human in it is not the game this cycle ships.
    assert!(reconstructed > 50, "only {reconstructed} bot events reconstructed");
    assert!(human > 50, "only {human} human events — is this the pair that was played?");
    assert_eq!(
        reconstructed + human + not_moves,
        record.events.len(),
        "every event is accounted for"
    );
}
