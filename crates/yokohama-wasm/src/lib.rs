//! The browser host (M6.5b, R-6.5b.9): ONE `Games` over `MemStore`, `NoSignal`,
//! `NoArchive` and a settable clock, behind six `wasm-bindgen` exports.
//!
//! This is HOST code. Its platform calls are the clock and the bindgen glue and
//! nothing else — [`this_host_makes_no_platform_call_but_the_clock`] holds that
//! over this crate's own non-test source, with the SAME EIGHT PATTERNS the
//! layer's `platform_calls_stay_behind_the_server_feature` names. The two
//! scans are not the same scan and this doc used to claim they were
//! (whole-branch review, vacuity #5): the layer's ENFORCES per-item `#[cfg]`
//! gating below a file's test module, while this one stops at the first
//! top-level `#[cfg(test)]` and reads what is above it.
//! It is the only caller of the layer in the browser, and its own only caller
//! is the dedicated Web Worker the client half starts
//! (`web/src/local/worker.ts`, which lands with that half) — nothing on the
//! page reaches this module.
//!
//! **Every export is a thin wrapper over a pure-Rust `_inner` function** (spec
//! §2.1). The cells below call the inner ones, because `wasm-bindgen`'s
//! non-wasm stubs cannot make a `JsValue` — so the wrappers carry no logic
//! beyond the conversion a JS caller needs, and everything that could be wrong
//! is reachable natively.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use wasm_bindgen::prelude::*;
use yokohama_web::api::{self, ApiRequest, Host};
use yokohama_web::games::Games;
use yokohama_web::identity::User;
use yokohama_web::store::{Doc, MemStore};

/// The one `Games`.
///
/// [`init`] REPLACES it — a rebuild, never a once: the Worker calls `init` at
/// boot, on every lock acquisition and after a trap, and follows each with
/// [`load`]s of every pair it holds (spec §2.2). A `OnceCell` here would make
/// the acquisition path impossible to write.
static GAMES: Mutex<Option<Games>> = Mutex::new(None);

/// The clock: frozen to `FROZEN` while `FROZEN_ON`, else live.
///
/// Live is `Date.now()` on the target and a ticking counter everywhere else —
/// the crate's own cells and the workspace lint gate compile for the host, and
/// `js_sys::Date::now` there is a stub that panics. The ticking shape is
/// `Games::ticking_clock`'s and for its reason: a constant would make every
/// table's `created_at` a tie and turn each "newest first" claim into a claim
/// about the id tie-break.
static FROZEN_ON: AtomicBool = AtomicBool::new(false);
static FROZEN: AtomicU64 = AtomicU64::new(0);
/// Gated to the hosts that read it: on `wasm32` the live arm is `Date.now()`
/// and this counter is dead code the target build warns about, and a warning
/// no per-edit gate can see is a warning nobody removes.
#[cfg(not(target_arch = "wasm32"))]
static TICKS: AtomicU64 = AtomicU64::new(0);

fn now() -> u64 {
    if FROZEN_ON.load(Ordering::Relaxed) {
        return FROZEN.load(Ordering::Relaxed);
    }
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        TICKS.fetch_add(1, Ordering::Relaxed) + 1
    }
}

/// The deployment facts this host reports. `"none"` rather than `"sse"`:
/// nothing in a browser store can be signalled to, and the page's local arm
/// never opens a stream.
///
/// **`serves_games` and `multiplayer_offered` disagree here, and that is the
/// point of them being two fields** (M6.5b2, spec §3.1): a local game is played
/// by calling `/api/games/…` through this very layer, so the routes must be
/// served — while there is nobody else at the table, so `/api/health` reports
/// no multiplayer. A single field would have made single player stop working
/// the moment this host told the truth about itself.
const HOST: Host =
    Host { identity_name: "local", signal_kind: "none", serves_games: true, multiplayer_offered: false };

/// Build a FRESH `Games` over a fresh `MemStore`, replacing any earlier one.
#[wasm_bindgen]
pub fn init() {
    let games = Games::with_parts(
        Arc::new(yokohama_core::Rules::standard()),
        Arc::new(MemStore::default()),
        yokohama_web::chat::ChatConfig::default(),
        Arc::new(yokohama_web::signal::NoSignal),
        Arc::new(now),
        Arc::new(yokohama_web::archive::NoArchive),
        yokohama_web::games::RunHost::Browser,
    );
    *GAMES.lock().expect("the host's games") = Some(games);
}

/// What [`dispatch`] answers: `ApiResponse`'s two fields, as a JS object with
/// getters (`wasm-bindgen` cannot hand a plain struct across by value).
#[wasm_bindgen]
pub struct Reply {
    status: u16,
    body: String,
}

#[wasm_bindgen]
impl Reply {
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> u16 {
        self.status
    }

    #[wasm_bindgen(getter)]
    pub fn body(&self) -> String {
        self.body.clone()
    }
}

/// The layer's front door, one call.
///
/// `user` absent is the host's own sentence carried into `api::dispatch`, so
/// `/api/me` answers `user: null` as the corpus captures it (`/api/me` never
/// refuses) and the 401 sweep has an arm it can reach. A MALFORMED address
/// carries `User::parse`'s own sentence instead, which is what
/// `identity::resolve` does on the axum host — the two hosts refuse in the same
/// words.
///
/// **The identity is a parameter** so a test can play the corpus walk as its
/// own five people (spec §6.3); the Worker passes `local@browser` and nothing
/// but the Worker can reach the module.
pub fn dispatch_inner(
    user: Option<&str>,
    method: &str,
    path: &str,
    query: &str,
    content_type: Option<&str>,
    body: &[u8],
) -> (u16, String) {
    let guard = GAMES.lock().expect("the host's games");
    let games = guard.as_ref().expect("init() before dispatch()");
    let parsed = user.map(User::parse);
    let identity: api::Identity<'_> = match &parsed {
        Some(Ok(u)) => Ok(u),
        Some(Err(e)) => Err(e.as_str()),
        None => Err("no identity given"),
    };
    let req = ApiRequest { method, path, query, content_type, body };
    let res = api::dispatch(games, &HOST, identity, &req);
    (res.status, res.body)
}

#[wasm_bindgen]
pub fn dispatch(
    user: Option<String>,
    method: &str,
    path: &str,
    query: &str,
    content_type: Option<String>,
    body: &[u8],
) -> Reply {
    let (status, body) = dispatch_inner(
        user.as_deref(),
        method,
        path,
        query,
        content_type.as_deref(),
        body,
    );
    Reply { status, body }
}

/// The two document kinds a host may fill. Not the chat and not the house: a
/// local table has no chat panel (spec §2.5), so nothing would ever be there to
/// load, and naming them here would invite one.
fn doc_of(name: &str) -> Result<Doc, String> {
    match name {
        "record" => Ok(Doc::Record),
        "table" => Ok(Doc::Table),
        other => Err(format!("no such document kind: {other:?} (record | table)")),
    }
}

pub fn load_inner(doc: &str, id: &str, text: &str) -> Result<(), String> {
    let guard = GAMES.lock().expect("the host's games");
    let games = guard.as_ref().expect("init() before load()");
    games.load_document(doc_of(doc)?, id, text)
}

pub fn read_inner(doc: &str, id: &str) -> Result<Option<String>, String> {
    let guard = GAMES.lock().expect("the host's games");
    let games = guard.as_ref().expect("init() before read()");
    games.read_document(doc_of(doc)?, id)
}

/// Write a stored document into the store — a CREATE, which refuses a present
/// document. Only ever called on a fresh store (spec §2.2), so it never meets
/// one and the `Store` trait needs no versionless overwrite.
#[wasm_bindgen]
pub fn load(doc: &str, id: &str, text: &str) -> Result<(), JsValue> {
    load_inner(doc, id, text).map_err(|e| JsValue::from_str(&e))
}

/// The store's exact bytes, or `undefined`.
#[wasm_bindgen]
pub fn read(doc: &str, id: &str) -> Result<Option<String>, JsValue> {
    read_inner(doc, id).map_err(|e| JsValue::from_str(&e))
}

/// A frozen reading, or `undefined` for the live clock.
#[wasm_bindgen]
pub fn set_clock(seconds: Option<f64>) {
    match seconds {
        Some(s) => {
            FROZEN.store(s as u64, Ordering::Relaxed);
            FROZEN_ON.store(true, Ordering::Relaxed);
        }
        None => FROZEN_ON.store(false, Ordering::Relaxed),
    }
}

/// Read a backup file and say what each game in it is (M6.5b2, R-6.5b2.10 –
/// .12) — the report as JSON, a list of `{"id":…,"verdict":{"kind":…,…}}`.
///
/// **The ONE export this cycle adds**, and it carries no logic: the validator
/// is `yokohama_web::backup`, which the axum host's archive route calls too, so
/// a backup and an upload cannot disagree about what a valid game is.
///
/// Writing the envelope is deliberately NOT here (spec §2.5): the page builds
/// it from IndexedDB directly, because a module that will not load is when you
/// most need your games out.
pub fn restore_read_inner(envelope: &str) -> Result<String, String> {
    yokohama_web::backup::restore_report_json(envelope)
}

#[wasm_bindgen]
pub fn restore_read(envelope: &str) -> Result<String, JsValue> {
    restore_read_inner(envelope).map_err(|e| JsValue::from_str(&e))
}

/// The e2e build's trap hook (spec §6.5, `--features debug`): a panic under a
/// held lock is a trap that leaves `GAMES` LOCKED FOR EVER — the shape a fresh
/// Worker recovers from and a re-`init()` cannot.
///
/// **Not "poisoned", which is the wrong word on this target** (whole-branch
/// review, vacuity #6): `wasm32-unknown-unknown` aborts on panic rather than
/// unwinding, so the guard is never dropped and the mutex is never poisoned —
/// it is simply still held. The consequence is the same one the recovery is
/// built for: the next `init()` traps too.
#[cfg(feature = "debug")]
#[wasm_bindgen]
pub fn debug_trap() {
    let _guard = GAMES.lock().expect("the host's games");
    panic!("debug_trap");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cells share the global `GAMES` and the global clock; they run one at
    /// a time. Not a `#[serial]` dependency — one `Mutex` and four cells.
    static SERIAL: Mutex<()> = Mutex::new(());

    /// A poisoned `SERIAL` would fail every later cell with the wrong message,
    /// so the guard is taken through the poison rather than unwrapped.
    fn serial() -> std::sync::MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn a_missing_identity_is_the_layers_own_401_and_me_answers_null() {
        let _s = serial();
        init();
        let (status, _) = dispatch_inner(None, "GET", "/api/games", "", None, b"");
        assert_eq!(status, 401, "every game route refuses a caller with no identity");
        let (status, body) = dispatch_inner(None, "GET", "/api/me", "", None, b"");
        assert_eq!(status, 200, "/api/me never refuses — null is its answer");
        assert_eq!(body, r#"{"mode":"local","user":null}"#);
        let (_, body) = dispatch_inner(Some("local@browser"), "GET", "/api/health", "", None, b"");
        assert!(
            body.contains(r#""identity":"local""#)
                && body.contains(r#""signal":"none""#)
                && body.contains(r#""archive":"off""#),
            "this host's three deployment facts: {body}",
        );
        // A malformed caller is `User::parse`'s own sentence, which is what the
        // axum host's `identity::resolve` carries into ITS 401 — so the two
        // hosts refuse the same input in the same words. `..` is the name the
        // key grammar refuses by its leading-character rule (M6.5c2, review H1).
        let (status, body) = dispatch_inner(Some(".."), "GET", "/api/games", "", None, b"");
        assert_eq!(status, 401);
        assert_eq!(body, serde_json::json!({ "error": User::parse("..").unwrap_err() }).to_string());
    }

    #[test]
    fn init_is_a_rebuild_and_load_is_a_create() {
        let _s = serial();
        init();
        load_inner("table", "local-1", "{\"schema\":3}").unwrap();
        assert!(
            load_inner("table", "local-1", "{\"schema\":3}").is_err(),
            "a present document is refused",
        );
        assert_eq!(read_inner("table", "local-1").unwrap().as_deref(), Some("{\"schema\":3}"));
        assert!(load_inner("chat", "local-1", "{}").is_err(), "only record | table");
        init();
        assert_eq!(read_inner("table", "local-1").unwrap(), None, "init() replaced the store");
    }

    /// **A plain name on a table's watcher list reads on the browser host**
    /// (M6.5c2, spec §3's identity cells): a restored or loaded table runs the
    /// layer's `check_shape` here as everywhere, over the one key grammar.
    #[test]
    fn a_table_whose_watcher_list_holds_a_plain_name_reads() {
        let _s = serial();
        init();
        let body = br#"{"players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,"seats":[{"user":"red@t.test"},{"user":"blue@t.test"}],"id":"named"}"#;
        let (status, created) = dispatch_inner(Some("red@t.test"), "POST", "/api/games", "", Some("application/json"), body);
        assert_eq!(status, 201, "{created}");
        for (who, keep) in [("red@t.test", br#"{"seat":0,"card":1}"#), ("blue@t.test", br#"{"seat":1,"card":1}"#)] {
            let (status, _) = dispatch_inner(Some(who), "POST", "/api/games/named/keep", "", Some("application/json"), keep);
            assert_eq!(status, 200, "{who}");
        }
        let mut doc: serde_json::Value = serde_json::from_str(&read_inner("table", "named").unwrap().unwrap()).unwrap();
        let record = read_inner("record", "named").unwrap().expect("the last keep dealt the game");
        for (watcher, want) in [("ben castle", 200), ("..", 500)] {
            doc["unlocked"] = serde_json::json!([watcher]);
            init();
            load_inner("table", "named", &doc.to_string()).unwrap();
            load_inner("record", "named", &record).unwrap();
            let (status, body) = dispatch_inner(Some("red@t.test"), "GET", "/api/games/named/table", "", None, b"");
            assert_eq!(status, want, "{watcher:?}: {body}");
        }
    }

    /// **A game dealt in the browser names it** (M6.5c2, spec §2.4): the
    /// record `finalize` writes on this host says `run_id: "web/browser"`.
    #[test]
    fn a_game_dealt_here_names_web_browser() {
        let _s = serial();
        init();
        let user = Some("local@browser");
        let create = br#"{"players":2,"colors":["Red","Blue"],"start_seat":0,"seed":"20260831","seats":[{"user":"local@browser"},{"bot":true}],"id":"local-named"}"#;
        let (status, body) = dispatch_inner(user, "POST", "/api/games", "", Some("application/json"), create);
        assert_eq!(status, 201, "{body}");
        let keep = br#"{"seat":0,"card":1}"#;
        let (status, body) =
            dispatch_inner(user, "POST", "/api/games/local-named/keep", "", Some("application/json"), keep);
        assert_eq!(status, 200, "{body}");
        let record = read_inner("record", "local-named").unwrap().expect("the keep dealt the game");
        let v: serde_json::Value = serde_json::from_str(&record).expect("a record");
        assert_eq!(v["run"]["run_id"], "web/browser");
    }

    #[test]
    fn the_clock_freezes_and_ticks() {
        let _s = serial();
        init();
        set_clock(Some(1_789_000_000.0));
        assert_eq!(now(), 1_789_000_000);
        assert_eq!(now(), 1_789_000_000, "frozen is frozen — two readings agree");
        set_clock(None);
        let a = now();
        let b = now();
        assert!(b > a, "the native clock ticks: {a} then {b}");
    }

    /// **The crate's own scan** (spec §2.1): this is host code and its ONLY
    /// platform call is the clock. The eight patterns the layer's
    /// `platform_calls_stay_behind_the_server_feature` names are refused here
    /// too, over this crate's own `src` with each file's top-level test module
    /// cut out.
    ///
    /// Two anti-vacuity partners, and the second is the one worth reading: the
    /// scan must have looked at a file, and the one allowed call must be in
    /// CODE rather than in prose — a doc comment naming `js_sys::Date::now`
    /// would otherwise keep this cell green over a crate that had stopped
    /// making the call at all.
    #[test]
    fn this_host_makes_no_platform_call_but_the_clock() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let patterns = [
            "SystemTime",
            "Instant::now",
            "std::fs",
            "std::env",
            "std::thread",
            "std::process",
            "tokio::",
            "axum::",
        ];
        let mut escaped: Vec<String> = Vec::new();
        let mut scanned = 0usize;
        let mut clock_in_code = false;
        for entry in std::fs::read_dir(&src).expect("src is readable") {
            let path = entry.expect("an entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let name = path.file_name().unwrap().to_str().unwrap().to_string();
            let text = std::fs::read_to_string(&path).expect("readable");
            // Top-level means column 0, so an inner `#[cfg(test)]` on a
            // test-only item does not end the scan early — the layer's own rule.
            let lines: Vec<&str> = text.lines().collect();
            let end = lines.iter().position(|l| *l == "#[cfg(test)]").unwrap_or(lines.len());
            scanned += 1;
            for (n, line) in lines[..end].iter().enumerate() {
                let is_comment = line.trim_start().starts_with("//");
                for pat in patterns {
                    if line.contains(pat) {
                        escaped.push(format!("{name}:{}: {pat}: {}", n + 1, line.trim()));
                    }
                }
                if !is_comment && line.contains("js_sys::Date::now") {
                    clock_in_code = true;
                }
            }
        }
        assert!(escaped.is_empty(), "platform calls in the host crate: {escaped:#?}");
        assert!(scanned > 0, "anti-vacuity: the scan read no file");
        assert!(clock_in_code, "anti-vacuity: the one allowed platform call is CODE, not prose");
    }

    /// The module's one new export, over the COMMITTED envelope fixture
    /// (`web/src/wire-script/backup-envelope.json`, emitted and pinned by
    /// `yokohama_web::backup`'s own test).
    ///
    /// **All four verdicts in one file**, so a report that answered one thing
    /// for everything could not pass — and the whole-file refusal beside them,
    /// because a version this build does not know must be refused as a WHOLE
    /// rather than half-read.
    ///
    /// This is the native half. `web/src/backup.node.test.ts` runs the same
    /// fixture through the module COMPILED to wasm, which is what says the
    /// client's fake answers what this build really gives.
    #[test]
    fn restore_read_reports_every_verdict_and_refuses_a_file_it_cannot_read() {
        let _s = serial();
        const ENVELOPE: &str = include_str!("../../../web/src/wire-script/backup-envelope.json");
        let report = restore_read_inner(ENVELOPE).expect("the committed fixture reads");
        let rows: serde_json::Value = serde_json::from_str(&report).expect("the report is JSON");
        let kinds: Vec<&str> =
            rows.as_array().expect("a list").iter().map(|r| r["verdict"]["kind"].as_str().expect("a kind")).collect();
        assert_eq!(kinds, ["accepted", "dealing", "unplayable", "rejected"]);
        assert_eq!(rows[0]["id"], "local-242a5429453c593c");
        assert_eq!(rows[0]["verdict"]["events"], 595);

        let newer = ENVELOPE.replace("\"envelope\": 1", "\"envelope\": 2");
        assert_ne!(newer, ENVELOPE, "the mutation must actually change the fixture");
        let err = restore_read_inner(&newer).expect_err("a version this build does not know");
        assert!(err.contains("envelope"), "{err}");
    }

}
