//! **The per-object host** (M6.5c, spec §2.2): the table layer compiled to
//! WebAssembly for the Durable Objects — ONE `Games` over a `MemStore`, the
//! [`signal::RecordingSignal`], `NoArchive` and a settable clock, behind
//! `wasm-bindgen` exports built for the objects' shell (spec §2.2).
//!
//! **`crates/yokohama-wasm`'s shape, for that crate's reason**: it is a HOST and
//! so is this one. What the two share is the layer; each carries its own clock
//! and its own platform scan, and each copy here names its source.
//!
//! This is HOST code. Its platform calls are the clock and the bindgen glue and
//! nothing else — `this_host_makes_no_platform_call_but_the_clock` holds that
//! over every `src/*.rs` of this crate, with the SAME EIGHT PATTERNS the layer's
//! `platform_calls_stay_behind_the_server_feature` names, reading each file
//! down to its first top-level `#[cfg(test)]`.
//!
//! **Every export is a thin wrapper over a pure-Rust `_inner` function** (spec
//! §2.2). The cells call the inner ones, because `wasm-bindgen`'s non-wasm
//! stubs cannot make a `JsValue` — so the wrappers carry no logic beyond the
//! conversion a JS caller needs, and everything that could be wrong is
//! reachable under `cargo test`.

mod config;
pub mod signal;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use wasm_bindgen::prelude::*;
use yokohama_web::api::{self, ApiRequest, Host};
use yokohama_web::archive::NoArchive;
use yokohama_web::games::Games;
use yokohama_web::identity::{resolve, IdentityMode, User, WORKER_HEADER};
use yokohama_web::store::{Doc, MemStore};

use crate::signal::{Bump, RecordingSignal};

/// The one `Games`.
///
/// [`init`] REPLACES it — a rebuild, never a once, as the browser host's is —
/// so every `init` starts from an EMPTY store, which is what [`load`] (a
/// create) fills: a fresh instance is `init`ed and then `load`ed with every
/// document its object holds (spec §2.4's wake path).
pub(crate) static GAMES: Mutex<Option<Games>> = Mutex::new(None);

/// The store the one `Games` holds — the SAME `Arc`, kept so this host can
/// replace and remove documents through the public `Store` trait where
/// [`load`] would refuse a present one (spec §2.2). [`init`] replaces it with
/// the `Games` it belongs to.
pub(crate) static STORE: Mutex<Option<Arc<MemStore>>> = Mutex::new(None);

/// The signal the one `Games` holds — the SAME `Arc`, so what the layer
/// signals during a call is drained into that call's reply.
pub(crate) static SIGNAL: Mutex<Option<Arc<RecordingSignal>>> = Mutex::new(None);

/// The clock, copied from `yokohama-wasm` (`crates/yokohama-wasm/src/lib.rs`)
/// and never shared, because each host is its own host — with one difference:
/// [`set_clock`] hands back the setting it replaced. Frozen to `FROZEN` while
/// `FROZEN_ON`, else live.
///
/// Live is `Date.now()` on the target and a ticking counter everywhere else —
/// the crate's own cells and the workspace lint gate compile for the host, and
/// `js_sys::Date::now` there is a stub that panics. The ticking shape is
/// `Games::ticking_clock`'s and for its reason: a constant would make every
/// table's `created_at` a tie and turn each "newest first" claim into a claim
/// about the id tie-break.
///
/// **[`set_clock`] is its only writer.** [`init`] rebuilds everything else
/// this crate holds and leaves the clock as it was: the plan's ruling P-2 has
/// the shell pin a reading before every call that can reach the layer's
/// clock.
static FROZEN_ON: AtomicBool = AtomicBool::new(false);
static FROZEN: AtomicU64 = AtomicU64::new(0);
/// Gated to the hosts that read it: on `wasm32` the live arm is `Date.now()`
/// and this counter would be dead code — a warning the target build prints
/// and no gate fails on, which is a warning nobody removes.
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

/// Pin the clock to `seconds`, or pass `undefined` for the live clock — and get
/// back what it replaced (`undefined` = live), so a caller that pins a reading
/// for one call can restore a test configuration's frozen clock after it (spec
/// §2.6). Whole seconds: a fraction is dropped, as the live arm drops it.
#[wasm_bindgen]
pub fn set_clock(seconds: Option<f64>) -> Option<f64> {
    let prior = FROZEN_ON.load(Ordering::Relaxed).then(|| FROZEN.load(Ordering::Relaxed) as f64);
    match seconds {
        Some(s) => {
            FROZEN.store(s as u64, Ordering::Relaxed);
            FROZEN_ON.store(true, Ordering::Relaxed);
        }
        None => FROZEN_ON.store(false, Ordering::Relaxed),
    }
    prior
}

/// The deployment facts this host reports (spec §2.2; §4's second class).
///
/// **`cf-access`, although the module reads the `worker` mode's header**: the
/// door a client is behind is Cloudflare Access, and R-6.5c.3 has the Worker
/// forward its verdict to the objects as `X-Yokohama-User` — so `/api/me` and
/// `/api/health` name the door, and [`identity_of`] reads the header. `"ws"`:
/// a table's changes reach a browser over a WebSocket (spec §2.7). Multiplayer
/// is both served and offered.
const HOST: Host =
    Host { identity_name: "cf-access", signal_kind: "ws", serves_games: true, multiplayer_offered: true };

/// Build a FRESH layer from the shell's configuration, replacing any earlier
/// one — the `Games`, its store and its signal together. A refused
/// configuration changes nothing, and its refusal names the variable (spec
/// §2.2).
pub fn init_inner(config: &str) -> Result<(), String> {
    let chat = config::parse(config)?;
    let store = Arc::new(MemStore::default());
    let signal = Arc::new(RecordingSignal::default());
    let games = Games::with_parts(
        Arc::new(yokohama_core::Rules::standard()),
        store.clone(),
        chat,
        signal.clone(),
        Arc::new(now),
        Arc::new(NoArchive),
    );
    *STORE.lock().expect("the host's store") = Some(store);
    *SIGNAL.lock().expect("the host's signal") = Some(signal);
    *GAMES.lock().expect("the host's games") = Some(games);
    Ok(())
}

/// [`init_inner`] for the shell: a refusal is thrown as its sentence.
#[wasm_bindgen]
pub fn init(config: &str) -> Result<(), JsValue> {
    init_inner(config).map_err(|e| JsValue::from_str(&e))
}

/// What a call answers the shell with: `ApiResponse`'s two fields, and what the
/// call leaves the shell to do once its batch has committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyData {
    pub status: u16,
    pub body: String,
    /// The layer's signal calls during this call, in call order.
    pub bumps: Vec<Bump>,
    /// The receipt archive's deferred plan (spec §2.6). `None` from every call
    /// in this build, which has no receipt archive.
    pub archive: Option<String>,
    /// The table's picker list to persist (spec §2.4). `None` from every call
    /// in this build, which keeps no picker.
    pub picker: Option<String>,
}

/// [`ReplyData`] as a JS class with getters — `wasm-bindgen` cannot hand a
/// plain struct across by value.
#[wasm_bindgen]
pub struct Reply {
    data: ReplyData,
}

impl From<ReplyData> for Reply {
    fn from(data: ReplyData) -> Self {
        Self { data }
    }
}

#[wasm_bindgen]
impl Reply {
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> u16 {
        self.data.status
    }

    #[wasm_bindgen(getter)]
    pub fn body(&self) -> String {
        self.data.body.clone()
    }

    /// The list as JSON — `[]` when the call signalled nothing.
    #[wasm_bindgen(getter)]
    pub fn bumps(&self) -> String {
        serde_json::to_string(&self.data.bumps).expect("a bump serializes")
    }

    #[wasm_bindgen(getter)]
    pub fn archive(&self) -> Option<String> {
        self.data.archive.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn picker(&self) -> Option<String> {
        self.data.picker.clone()
    }
}

/// Who the caller is, by the `worker` mode's ONE header (spec §2.3): the value
/// the shell hands over goes into a header map as `X-Yokohama-User` and is read
/// by the layer's own `identity::resolve` — so `anonymous`, like an absent or
/// blank value, is no caller, and a refusal is the layer's own sentence.
///
/// A value no header can carry — one holding a control character other than a
/// tab — never reaches `resolve`: `User::parse` answers it instead, trimming it
/// as `resolve` would and refusing any control character that remains, in the
/// sentence the layer's 401 carries for a malformed caller.
pub fn identity_of(user: Option<&str>) -> Result<User, String> {
    let mut headers = http::HeaderMap::new();
    if let Some(raw) = user {
        let Ok(value) = http::HeaderValue::from_str(raw) else {
            return User::parse(raw);
        };
        headers.insert(WORKER_HEADER, value);
    }
    resolve(IdentityMode::Worker, &headers)
}

/// [`identity_of`] as a reply: 200 `{"user": <the normalized address>}`, or 401
/// in the layer's error shape — the very body the layer's 401 writes for the
/// same caller. It reads no document and signals nothing.
///
/// It is for admitting the lobby's socket, `GET /api/stream` (spec §2.5): that
/// route is no row of the layer's route table, so [`dispatch`] would answer it
/// 404.
pub fn identity_inner(user: Option<&str>) -> ReplyData {
    let (status, body) = match identity_of(user) {
        Ok(u) => (200, serde_json::json!({ "user": u.as_str() }).to_string()),
        Err(sentence) => (401, serde_json::json!({ "error": sentence }).to_string()),
    };
    ReplyData { status, body, bumps: Vec::new(), archive: None, picker: None }
}

#[wasm_bindgen]
pub fn identity(user: Option<String>) -> Reply {
    identity_inner(user.as_deref()).into()
}

/// The layer's front door, one call — and what it signalled.
///
/// The caller is [`identity_of`]'s answer, carried into `api::dispatch` as the
/// browser host carries its own: `Ok(&user)`, or `Err(sentence)` for the
/// layer's 401 — so `/api/me` answers `user: null` and every route that needs
/// a caller refuses in the `worker` mode's own words.
///
/// **The signal is drained while the `Games` is still held**, so the list is
/// this call's alone — including a cold handle's bot pump that persisted before
/// the request was refused, which is recorded because it happened (spec §5.1).
pub fn dispatch_inner(
    user: Option<&str>,
    method: &str,
    path: &str,
    query: &str,
    content_type: Option<&str>,
    body: &[u8],
) -> ReplyData {
    let guard = GAMES.lock().expect("the host's games");
    let games = guard.as_ref().expect("init() before dispatch()");
    let caller = identity_of(user);
    let identity: api::Identity<'_> = match &caller {
        Ok(u) => Ok(u),
        Err(sentence) => Err(sentence.as_str()),
    };
    let req = ApiRequest { method, path, query, content_type, body };
    let res = api::dispatch(games, &HOST, identity, &req);
    ReplyData { status: res.status, body: res.body, bumps: drain_bumps(), archive: None, picker: None }
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
    dispatch_inner(user.as_deref(), method, path, query, content_type.as_deref(), body).into()
}

/// Everything the layer signalled since the last drain.
fn drain_bumps() -> Vec<Bump> {
    SIGNAL.lock().expect("the host's signal").as_ref().expect("init() before dispatch()").drain()
}

/// The four document kinds the layer keeps, by the names [`load`] and [`read`]
/// take. Not the `picker`: that row is the host's own (spec §2.4), and nothing
/// in the layer reads it.
fn doc_of(name: &str) -> Result<Doc, String> {
    match name {
        "record" => Ok(Doc::Record),
        "table" => Ok(Doc::Table),
        "chat" => Ok(Doc::Chat),
        "house" => Ok(Doc::House),
        other => Err(format!("no such document kind: {other:?} (record | table | chat | house)")),
    }
}

/// Write a stored document into the store — a CREATE, which refuses a present
/// document (`Games::load_document`). The house chat answers to one id,
/// `chat::HOUSE`, and the store refuses any other.
pub fn load_inner(doc: &str, id: &str, text: &str) -> Result<(), String> {
    let doc = doc_of(doc)?;
    let guard = GAMES.lock().expect("the host's games");
    let games = guard.as_ref().expect("init() before load()");
    games.load_document(doc, id, text)
}

/// The store's exact bytes, or `None` when absent (`Games::read_document`).
pub fn read_inner(doc: &str, id: &str) -> Result<Option<String>, String> {
    let doc = doc_of(doc)?;
    let guard = GAMES.lock().expect("the host's games");
    let games = guard.as_ref().expect("init() before read()");
    games.read_document(doc, id)
}

#[wasm_bindgen]
pub fn load(doc: &str, id: &str, text: &str) -> Result<(), JsValue> {
    load_inner(doc, id, text).map_err(|e| JsValue::from_str(&e))
}

/// The store's exact bytes, or `undefined`.
#[wasm_bindgen]
pub fn read(doc: &str, id: &str) -> Result<Option<String>, JsValue> {
    read_inner(doc, id).map_err(|e| JsValue::from_str(&e))
}

/// The battery's trap hook (`--features debug` only, spec §2.2): a panic under
/// the held `GAMES` lock — `yokohama-wasm`'s hook, for its reason.
///
/// `wasm32-unknown-unknown` aborts on panic rather than unwinding, so the guard
/// is never dropped and the lock is left HELD, not poisoned: the next call that
/// takes it traps too, including `init`. The shell discards a trapped instance
/// (spec §2.4).
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
    /// a time, as `yokohama-wasm`'s do.
    static SERIAL: Mutex<()> = Mutex::new(());

    /// A poisoned `SERIAL` would fail every later cell with the wrong message,
    /// so the guard is taken through the poison rather than unwrapped.
    fn serial() -> std::sync::MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn config(archive: &str) -> String {
        format!(r#"{{"archive":"{archive}","caller":null,"lobby_chat":null,"chat_keep":null,"chat_text":null}}"#)
    }

    /// The objects read the caller from `X-Yokohama-User` (spec §2.3), so a
    /// missing caller is the layer's 401 in the `worker` mode's OWN sentence —
    /// `identity::resolve`'s, never a restatement — and the literal `anonymous`
    /// is no caller at all.
    #[test]
    fn a_missing_caller_is_the_worker_modes_401_and_anonymous_is_missing() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        for user in [None, Some("anonymous"), Some("  ")] {
            let r = dispatch_inner(user, "GET", "/api/games", "", None, b"");
            assert_eq!(r.status, 401, "{user:?}");
            assert!(r.body.contains("X-Yokohama-User from the edge Worker"), "{}", r.body);
        }
        let r = dispatch_inner(None, "GET", "/api/me", "", None, b"");
        assert_eq!((r.status, r.body.as_str()), (200, r#"{"mode":"cf-access","user":null}"#));
        let r = dispatch_inner(Some("Red@Wire.Test"), "GET", "/api/me", "", None, b"");
        assert_eq!(r.body, r#"{"mode":"cf-access","user":"red@wire.test"}"#, "normalized once, as the layer does");
    }

    /// `identity` is for admitting the house's `/api/stream` — the axum
    /// ROUTER's route, not a row of the layer's table, so `dispatch` would
    /// answer it 404. Its 401 is the layer's own refusal for the same caller,
    /// BYTE FOR BYTE: the body is compared with what `dispatch` answers on a
    /// game route, so a restated shape that drifted from the layer's fails
    /// here rather than on the client.
    #[test]
    fn identity_answers_the_normalized_caller_or_the_worker_modes_401() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        for user in [None, Some("anonymous"), Some("  ")] {
            let r = identity_inner(user);
            assert_eq!(r.status, 401, "{user:?}");
            assert!(r.body.contains("X-Yokohama-User from the edge Worker"), "{}", r.body);
            let layer = dispatch_inner(user, "GET", "/api/games", "", None, b"");
            assert_eq!((r.status, &r.body), (layer.status, &layer.body), "the layer's own 401, byte for byte");
        }
        // A value no header can carry (a DEL byte) never reaches `resolve`; it is
        // `User::parse`'s sentence, which is what the layer's 401 carries for a
        // malformed caller.
        let raw = "red@wire.test\u{7f}";
        let r = identity_inner(Some(raw));
        let want = serde_json::json!({ "error": User::parse(raw).unwrap_err() }).to_string();
        assert_eq!((r.status, r.body.as_str()), (401, want.as_str()));
        let r = identity_inner(Some("Red@Wire.Test"));
        assert_eq!((r.status, r.body.as_str()), (200, r#"{"user":"red@wire.test"}"#), "normalized once");
        assert!(r.bumps.is_empty() && r.archive.is_none() && r.picker.is_none(), "an identity read writes nothing");
    }

    /// This host's four deployment facts on `/api/health` (spec §4's second class).
    #[test]
    fn health_reports_cf_access_ws_multiplayer_and_the_archive_switch() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let body = dispatch_inner(Some("red@wire.test"), "GET", "/api/health", "", None, b"").body;
        for fact in [r#""identity":"cf-access""#, r#""signal":"ws""#, r#""archive":"off""#, r#""multiplayer":true"#] {
            assert!(body.contains(fact), "{fact} in {body}");
        }
    }

    /// `init` refuses a bad value NAMING the variable — the Worker's form of the
    /// NAS refusing to start (spec §2.6) — and the chat variables are the NAS's,
    /// through the layer's own parser.
    #[test]
    fn init_refuses_a_bad_value_by_the_variables_name() {
        let _s = serial();
        let err = init_inner(&config("s3")).unwrap_err();
        assert!(err.contains("YOKOHAMA_WEB_ARCHIVE"), "{err}");
        let err = init_inner(r#"{"archive":"off","caller":null,"lobby_chat":"maybe","chat_keep":null,"chat_text":null}"#).unwrap_err();
        assert!(err.contains("YOKOHAMA_WEB_LOBBY_CHAT"), "{err}");
        init_inner(r#"{"archive":"off","caller":null,"lobby_chat":"off","chat_keep":"all","chat_text":"280"}"#).unwrap();
        let body = dispatch_inner(Some("red@wire.test"), "GET", "/api/health", "", None, b"").body;
        assert!(body.contains(r#""lobby_chat":false"#) && body.contains(r#""chat_text":280"#), "{body}");
    }

    /// The side channel IN and OUT, over all four kinds, and `load` is a CREATE.
    #[test]
    fn load_and_read_cover_the_four_kinds_and_load_refuses_a_present_document() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        for (kind, id) in [("record", "t1"), ("table", "t1"), ("chat", "t1"), ("house", "house")] {
            load_inner(kind, id, "{}").unwrap();
            assert_eq!(read_inner(kind, id).unwrap().as_deref(), Some("{}"), "{kind}");
            assert!(load_inner(kind, id, "{}").is_err(), "{kind}: a present document is refused");
        }
        assert!(load_inner("picker", "t1", "[]").is_err(), "the picker is the HOST's row, never the layer's");
    }

    /// **`house` is the LOBBY's chat** (spec §2.5), not a table's chat document
    /// that happens to be named `house` — which the round trip above cannot
    /// tell apart, because `load` and `read` agree with each other whichever
    /// document they both name (a mutation probe mapping `house` to `Doc::Chat`
    /// survived it). So the layer is the witness in both directions: what a
    /// lobby post writes, `read` hands back, and what `load` puts back into a
    /// fresh store, `GET /api/chat` serves.
    #[test]
    fn the_house_kind_is_the_lobbys_chat_in_both_directions() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        assert_eq!(call("red@t.test", "POST", "/api/chat", Some(r#"{"text":"first"}"#)).status, 201);
        let written = read_inner("house", "house").unwrap().expect("the post wrote the lobby's document");
        init_inner(&config("off")).unwrap();
        load_inner("house", "house", &written).unwrap();
        let page: serde_json::Value =
            serde_json::from_str(&call("red@t.test", "GET", "/api/chat", None).body).unwrap();
        assert_eq!(page["messages"][0]["text"], "first", "{page}");
    }

    /// `set_clock` hands back what it replaced, so a caller that pins a reading
    /// can restore a test configuration's frozen clock (spec §2.6).
    #[test]
    fn set_clock_returns_the_prior_setting() {
        let _s = serial();
        assert_eq!(set_clock(None), None);
        assert_eq!(set_clock(Some(1_789_000_000.0)), None);
        assert_eq!(set_clock(Some(1_789_000_500.0)), Some(1_789_000_000.0));
        assert_eq!(now(), 1_789_000_500);
        assert_eq!(set_clock(None), Some(1_789_000_500.0));
    }

    fn call(user: &str, method: &str, path: &str, body: Option<&str>) -> ReplyData {
        let bytes = body.map(|b| b.as_bytes().to_vec()).unwrap_or_default();
        dispatch_inner(Some(user), method, path, "", body.map(|_| "application/json"), &bytes)
    }
    fn tagged(r: &ReplyData) -> String {
        serde_json::to_string(&r.bumps).unwrap()
    }

    /// Every layer bump site reaches the reply, tagged and in call order — driven
    /// through `dispatch` exactly as the Worker drives it (spec §5.1).
    #[test]
    fn every_bump_site_arrives_in_the_reply_tagged_and_in_order() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let create = r#"{"id":"b","players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,
                         "seats":[{"user":"red@t.test"},{"open":true}]}"#;
        let r = call("red@t.test", "POST", "/api/games", Some(create));
        assert_eq!((r.status, tagged(&r).as_str()), (201, r#"[{"bump":"b"}]"#), "create");
        let r = call("blue@t.test", "POST", "/api/games/b/join", Some(r#"{"seat":1}"#));
        assert_eq!((r.status, tagged(&r).as_str()), (200, r#"[{"bump":"b"}]"#), "the join that fills the last seat");
        let r = call("red@t.test", "POST", "/api/games/b/keep", Some(r#"{"seat":0,"card":1}"#));
        assert_eq!((r.status, tagged(&r).as_str()), (200, r#"[{"bump_table":"b"}]"#), "a keep that does not finalize");
        let r = call("blue@t.test", "POST", "/api/games/b/keep", Some(r#"{"seat":1,"card":1}"#));
        // TWO bumps, in this order (tables.rs:1520-1523, then :1271): the keep's own write
        // bumps the table, and the finalize it then runs is a second write — `flip_to_playing`
        // bumps the house. A recorder that merged them would change what the table frames on.
        assert_eq!(
            (r.status, tagged(&r).as_str()),
            (200, r#"[{"bump_table":"b"},{"bump":"b"}]"#),
            "the last keep finalizes",
        );
        let r = call("red@t.test", "POST", "/api/games/b/input", Some(r#"{"line":"1"}"#));
        let state: serde_json::Value = serde_json::from_str(&r.body).unwrap();
        assert!(state["events"].as_u64().unwrap() > 0, "premise: line 1 APPLIED a move at the first prompt");
        assert_eq!((r.status, tagged(&r).as_str()), (200, r#"[{"bump":"b"}]"#), "an applied move persists and bumps");
        let r = call("red@t.test", "POST", "/api/games/b/chat", Some(r#"{"text":"hello"}"#));
        assert_eq!((r.status, tagged(&r).as_str()), (201, r#"[{"bump_table":"b"}]"#), "a table chat post");
        let r = call("red@t.test", "POST", "/api/chat", Some(r#"{"name":"Red","text":"anyone?"}"#));
        assert_eq!((r.status, tagged(&r).as_str()), (201, r#"[{"bump_house_chat":null}]"#), "a house chat post");
        let r = call("red@t.test", "DELETE", "/api/games/b", None);
        assert_eq!((r.status, tagged(&r).as_str()), (204, r#"[{"end":"b"}]"#), "the creator's delete");
    }

    /// **The owed-pump FIXTURE** (plan review M3). Task 9's pool cell needs the SAME
    /// bytes — a record whose last events leave the bot to move — and only Rust can
    /// make them. They cannot be re-derived byte-identically, because the finalize
    /// draws the bot seat's agent seed from OS entropy. So they are GENERATED once,
    /// under `UPDATE_WORKER_FIXTURES=1`, COMMITTED, and loaded by both cells.
    /// Without the variable this cell asserts the committed pair's premise instead.
    #[test]
    fn the_owed_pump_fixture_owes_the_bot_a_move() {
        let _s = serial();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../worker/test/fixtures");
        if std::env::var_os("UPDATE_WORKER_FIXTURES").is_some() {
            init_inner(&config("off")).unwrap();
            let prior = set_clock(Some(1_789_000_000.0));
            let create = r#"{"id":"owed","players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,
                             "seats":[{"user":"red@t.test"},{"bot":true}]}"#;
            assert_eq!(call("red@t.test", "POST", "/api/games", Some(create)).status, 201);
            assert_eq!(call("red@t.test", "POST", "/api/games/owed/keep", Some(r#"{"seat":0,"card":1}"#)).status, 200);
            let table = read_inner("table", "owed").unwrap().unwrap();
            let record: yokohama_core::GameRecord =
                serde_json::from_str(&read_inner("record", "owed").unwrap().unwrap()).unwrap();
            let mut session =
                yokohama_cli::session::Session::from_record(record, yokohama_core::Rules::standard()).unwrap();
            let bot = yokohama_core::PlayerId::new(1);
            while session.state().current_player != bot {
                let mv = session.legal()[0].clone();
                session.apply(mv, vec![]).unwrap();
            }
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("owed-pump.table.json"), &table).unwrap();
            std::fs::write(dir.join("owed-pump.json"), session.to_record_json().unwrap()).unwrap();
            set_clock(prior);
        }
        let record: yokohama_core::GameRecord =
            serde_json::from_str(&std::fs::read_to_string(dir.join("owed-pump.json")).expect("the committed fixture")).unwrap();
        let session = yokohama_cli::session::Session::from_record(record, yokohama_core::Rules::standard()).unwrap();
        assert_eq!(session.state().current_player, yokohama_core::PlayerId::new(1), "premise: the bot is owed a move");
    }

    /// **A GET can write** (the layer's `lib.rs:520-538`): a cold handle rebuilt with a bot to
    /// move pumps that move and persists it — and the bump is in the reply even
    /// though the request it rode in on is then REFUSED, because it happened. The
    /// shell (Task 9) relies on this: it compares documents after EVERY dispatch.
    /// The record route builds the handle before it refuses (`api.rs:1839-1850`,
    /// measured by the plan's review).
    #[test]
    fn a_cold_handles_owed_bot_move_is_recorded_before_a_later_refusal() {
        let _s = serial();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../worker/test/fixtures");
        let table = std::fs::read_to_string(dir.join("owed-pump.table.json")).expect("the committed fixture");
        let owed = std::fs::read_to_string(dir.join("owed-pump.json")).expect("the committed fixture");
        init_inner(&config("off")).unwrap();
        load_inner("table", "owed", &table).unwrap();
        load_inner("record", "owed", &owed).unwrap();
        let r = call("red@t.test", "GET", "/api/games/owed/record", None);
        assert_eq!(r.status, 409, "premise: the record route refuses a running game — AFTER the handle is built");
        assert_eq!(tagged(&r), r#"[{"bump":"owed"}]"#, "the owed pump happened, so its bump is recorded");
        assert_ne!(read_inner("record", "owed").unwrap().unwrap(), owed, "and the pump was persisted");
    }

    /// **The crate's own scan** (spec §5.1), copied from `yokohama-wasm`'s
    /// `this_host_makes_no_platform_call_but_the_clock`
    /// (`crates/yokohama-wasm/src/lib.rs`) — each host is its own host, and
    /// what the two share is the layer, never a test module. This is host code
    /// and its ONLY platform call is the clock. The eight patterns the layer's
    /// `platform_calls_stay_behind_the_server_feature` names are refused here
    /// too, over EVERY `src/*.rs` of this crate with each file's top-level test
    /// module cut out.
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
}
