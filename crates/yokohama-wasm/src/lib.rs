//! The browser host (M6.5b, R-6.5b.9): ONE `Games` over `MemStore`, `NoSignal`,
//! `NoArchive`, a settable clock and the console for the layer's log, behind
//! six `wasm-bindgen` exports.
//!
//! This is HOST code. Its platform calls are the clock, the console and the
//! bindgen glue, and nothing else —
//! [`this_host_makes_no_platform_call_but_the_clock_and_the_console`] holds that
//! over this crate's own non-test source, through the layer's own
//! `platform::calls_in`, over its eight patterns (`platform::CALLS`; M6.5c2,
//! spec §2.6). The two scans are not the same scan and this doc used to claim
//! they were (whole-branch review, vacuity #5): the layer's own
//! `platform_calls_stay_behind_the_server_feature` ENFORCES per-item `#[cfg]`
//! gating below a file's test module, while the hosts' stops at the first
//! top-level `#[cfg(test)]` and reads what is above it. The clock is the
//! layer's `HostClock` too; the live reading is this host's own.
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

use std::sync::{Arc, Mutex};

use wasm_bindgen::prelude::*;
use yokohama_web::api::{self, ApiRequest, Host};
use yokohama_web::games::{Games, HostClock};
use yokohama_web::identity::User;
use yokohama_web::store::{Doc, MemStore};

/// The one `Games`.
///
/// [`init`] REPLACES it — a rebuild, never a once: the Worker calls `init` at
/// boot, on every lock acquisition and after a trap, and follows each with
/// [`load`]s of every pair it holds (spec §2.2). A `OnceCell` here would make
/// the acquisition path impossible to write.
static GAMES: Mutex<Option<Games>> = Mutex::new(None);

/// The clock: the layer's `HostClock` (M6.5c2, spec §2.6), frozen while
/// pinned, else live. Live is `Date.now()` on the target and the clock's own
/// tick everywhere else — the crate's own cells and the workspace lint gate
/// compile for the host, and `js_sys::Date::now` there is a stub that panics.
static CLOCK: HostClock = HostClock::new();

fn now() -> u64 {
    CLOCK.now(|| {
        #[cfg(target_arch = "wasm32")]
        {
            (js_sys::Date::now() / 1000.0) as u64
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            CLOCK.tick()
        }
    })
}

/// The layer's log on this host (M6.5c2, spec §2.6): the browser's console.
///
/// **`console.warn`, never `console.error`.** A line here reports a table the
/// local listing skipped — a document another build wrote, or a store that
/// went bad — which the page survives; and the browser walk fails a run on any
/// `console.error` (`web/e2e/harness.ts`'s `watchForBreakage`). One JSON line,
/// the shape the Worker's shell logs, so a line reads the same on every host.
///
/// Natively — this crate's own cells and the lint gate — there is no console
/// and a `wasm-bindgen` import is a stub that panics, so the line goes nowhere;
/// `web/src/local/log.node.test.ts` reads it through the COMPILED module.
struct ConsoleLog;

impl yokohama_web::log::Log for ConsoleLog {
    fn line(&self, event: &'static str, fields: serde_json::Value) {
        #[cfg(target_arch = "wasm32")]
        {
            console_warn(&yokohama_web::log::render(event, &fields));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (event, fields);
        }
    }
}

/// `console.warn` — this host's second platform call, beside the clock
/// (M6.5c2). An import rather than a reach through `js_sys`: `js_sys` binds no
/// console, and the probe `js_sys::global()` would take is, by its own doc,
/// "pretty heavy code-size wise", in a module every visitor precaches.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = warn)]
    fn console_warn(line: &str);
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
/// the moment this host told the truth about itself. No table expires here, so
/// it reports no TTL (R-6.5c2.42).
const HOST: Host = Host {
    identity_name: "local",
    signal_kind: "none",
    serves_games: true,
    multiplayer_offered: false,
    table_ttl_days: None,
    open_table_ttl_days: None,
};

/// Build a FRESH `Games` over a fresh `MemStore`, whose log is the console
/// (M6.5c2), replacing any earlier one.
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
    )
    .with_log(Arc::new(ConsoleLog));
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

/// The store's absence, as the refusal it is (M6.5c2 cycle D, dossier row 56).
///
/// **A refusal and not an `expect`**: a panic here traps the module, and a
/// trapped module is the recovery the page makes for a FAULT — a fresh Worker —
/// when what happened is a caller that asked before [`init`]. The Worker always
/// `init`s first, so this names a mistake rather than a state anything ships.
fn no_store(export: &str) -> String {
    format!("init() before {export}(): this host has no store yet")
}

pub fn load_inner(doc: &str, id: &str, text: &str) -> Result<(), String> {
    // The kind FIRST, so a kind that is not one is named whether or not there
    // is a store to put it in.
    let doc = doc_of(doc)?;
    let guard = GAMES.lock().expect("the host's games");
    let games = guard.as_ref().ok_or_else(|| no_store("load"))?;
    games.load_document(doc, id, text)
}

pub fn read_inner(doc: &str, id: &str) -> Result<Option<String>, String> {
    let doc = doc_of(doc)?;
    let guard = GAMES.lock().expect("the host's games");
    let games = guard.as_ref().ok_or_else(|| no_store("read"))?;
    games.read_document(doc, id)
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

/// Pin the clock to `seconds`, or `None` for the live reading — whole seconds,
/// as the layer's clock keeps them; a fraction is dropped, as the live arm
/// drops it.
///
/// **A reading that is not one is REFUSED, by name, and changes nothing**
/// (M6.5c2 cycle D, dossier row 56). `s as u64` saturates: a negative reading
/// and `NaN` became 0 — the epoch — and an infinity `u64::MAX`, so a caller's
/// mistake stamped every time the layer writes at a date nobody chose, in
/// silence.
pub fn set_clock_inner(seconds: Option<f64>) -> Result<(), String> {
    let reading = match seconds {
        None => None,
        Some(s) if s.is_nan() => {
            return Err("set_clock: NaN is not a clock reading \u{2014} whole seconds from 0, or undefined for the live clock".to_string())
        }
        Some(s) if s.is_infinite() => {
            return Err(format!("set_clock: {s} is not a clock reading \u{2014} it is not finite"))
        }
        Some(s) if s < 0.0 => {
            return Err(format!("set_clock: {s} is not a clock reading \u{2014} it is negative, and the clock counts from 0"))
        }
        Some(s) => Some(s as u64),
    };
    CLOCK.set(reading);
    Ok(())
}

/// A frozen reading, or `undefined` for the live clock — whole seconds, as the
/// layer's clock keeps them. A reading that is not one throws its refusal
/// ([`set_clock_inner`]).
#[wasm_bindgen]
pub fn set_clock(seconds: Option<f64>) -> Result<(), JsValue> {
    set_clock_inner(seconds).map_err(|e| JsValue::from_str(&e))
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
        set_clock_inner(Some(1_789_000_000.0)).expect("a reading");
        assert_eq!(now(), 1_789_000_000);
        assert_eq!(now(), 1_789_000_000, "frozen is frozen — two readings agree");
        set_clock_inner(None).expect("the live clock");
        let a = now();
        let b = now();
        assert!(b > a, "the native clock ticks: {a} then {b}");
    }

    /// **A reading that is not one is REFUSED, by name** (M6.5c2 cycle D,
    /// dossier row 56). `s as u64` saturates: a negative reading and `NaN`
    /// both became 0 — the epoch — and an infinity `u64::MAX`, so a caller's
    /// mistake froze every stamp the layer writes at a date nobody chose, in
    /// silence. Each is refused in a sentence naming what it was given, and a
    /// refusal leaves the setting it met exactly as it was.
    #[test]
    fn set_clock_refuses_a_negative_nan_or_infinite_reading_by_name() {
        let _s = serial();
        set_clock_inner(Some(1_789_000_000.0)).expect("a reading");
        for (given, named) in [(-1.0, "-1"), (f64::NAN, "NaN"), (f64::INFINITY, "inf"), (f64::NEG_INFINITY, "-inf")] {
            let err = set_clock_inner(Some(given)).expect_err("not a clock reading");
            assert!(err.contains(named), "the refusal names {named}: {err}");
            assert_eq!(now(), 1_789_000_000, "{named} changed nothing");
        }
        // The edges that ARE readings: zero, and a fraction (dropped, as the
        // live arm drops it).
        set_clock_inner(Some(0.0)).expect("the epoch is a reading");
        assert_eq!(now(), 0);
        set_clock_inner(Some(1_789_000_000.9)).expect("a fraction is a reading");
        assert_eq!(now(), 1_789_000_000);
        set_clock_inner(None).expect("the live clock");
    }

    /// **Before `init()` there is no store, and that is a REFUSAL, not a
    /// panic** (M6.5c2 cycle D, dossier row 56): `load` and `read` answered it
    /// with an `expect` that trapped the module, and a trapped module is the
    /// recovery the page makes for a FAULT. The kind is read first, so a kind
    /// that is not one is named whether or not a store exists.
    #[test]
    fn load_and_read_before_init_are_refused_by_name_and_the_kind_is_read_first() {
        let _s = serial();
        *GAMES.lock().unwrap_or_else(|e| e.into_inner()) = None;
        let err = load_inner("table", "local-1", "{}").expect_err("no store yet");
        assert!(err.contains("init()"), "{err}");
        let err = read_inner("record", "local-1").expect_err("no store yet");
        assert!(err.contains("init()"), "{err}");
        let err = load_inner("chat", "local-1", "{}").expect_err("not a kind");
        assert!(err.contains("no such document kind: \"chat\""), "{err}");
        let err = read_inner("house", "local-1").expect_err("not a kind");
        assert!(err.contains("no such document kind: \"house\""), "{err}");
        init();
        assert_eq!(read_inner("table", "local-1").expect("a store now"), None);
    }

    /// **The crate's own scan** (spec §2.1): this is host code, and its
    /// platform calls are the clock and, since M6.5c2, the console the layer's
    /// log writes to. Every `src/*.rs` of this crate is read by the layer's own
    /// `platform::calls_in` — the hosts' scan over the eight patterns
    /// (`platform::CALLS`), which stops at a file's first column-0
    /// `#[cfg(test)]` (M6.5c2, spec §2.6).
    ///
    /// Four anti-vacuity partners, and they are this host's own: the layer's
    /// scan must find a call where there is one; this one must have looked at
    /// a file; and each of the two allowed calls — the clock and the console —
    /// must be in CODE rather than in prose. A doc comment naming
    /// `js_sys::Date::now` or the console import would otherwise keep this
    /// cell green over a crate that had stopped making the call at all.
    #[test]
    fn this_host_makes_no_platform_call_but_the_clock_and_the_console() {
        assert!(
            !yokohama_web::platform::calls_in("probe.rs", "use std::fs;\n").is_empty(),
            "anti-vacuity: the layer's scan finds a call where there is one",
        );
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut escaped: Vec<String> = Vec::new();
        let mut scanned = 0usize;
        let mut clock_in_code = false;
        let mut console_in_code = false;
        for entry in std::fs::read_dir(&src).expect("src is readable") {
            let path = entry.expect("an entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let name = path.file_name().unwrap().to_str().unwrap().to_string();
            let text = std::fs::read_to_string(&path).expect("readable");
            scanned += 1;
            escaped.extend(yokohama_web::platform::calls_in(&name, &text));
            // The partners read the lines `calls_in` reads: those above the
            // file's first column-0 `#[cfg(test)]`, less the comments.
            let lines: Vec<&str> = text.lines().collect();
            let end = lines.iter().position(|l| *l == "#[cfg(test)]").unwrap_or(lines.len());
            let code: Vec<&str> = lines[..end].iter().copied().filter(|l| !l.trim_start().starts_with("//")).collect();
            clock_in_code |= code.iter().any(|l| l.contains("js_sys::Date::now"));
            console_in_code |= code.iter().any(|l| l.contains("js_namespace = console"));
        }
        assert!(escaped.is_empty(), "platform calls in the host crate: {escaped:#?}");
        assert!(scanned > 0, "anti-vacuity: the scan read no file");
        assert!(clock_in_code, "anti-vacuity: the clock, one of the two allowed platform calls, is CODE, not prose");
        assert!(console_in_code, "anti-vacuity: the console import is CODE, not prose");
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

        // **The version-2 file carries a key version 1 lacks** (M6.5c2 cycle
        // D, the mutation battery's row 41), as `yokohama_web::backup`'s own
        // `an_envelope_of_another_version_is_refused_whole` makes its file. A
        // file that bumped only the number parsed whole as version 1, so it
        // answered "version 2" whichever the order and the version-before-parse
        // order had no witness on this host; with the key, a whole parse first
        // answers serde's "unknown field" and this cell fails.
        let newer = ENVELOPE
            .replacen("\"envelope\": 1", "\"envelope\": 2", 1)
            .replacen("{\n  \"kind\"", "{\n  \"added_in_version_2\": true,\n  \"kind\"", 1);
        assert!(
            newer.contains("\"envelope\": 2") && newer.contains("\"added_in_version_2\": true"),
            "the mutation must change the version and add a key version 1 lacks",
        );
        let err = restore_read_inner(&newer).expect_err("a version this build does not know");
        assert!(err.contains("version 2 backup"), "{err}");
        assert!(!err.contains("added_in_version_2"), "by its version, not by its key: {err}");
    }

}
