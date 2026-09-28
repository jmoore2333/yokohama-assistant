//! **The per-object host** (M6.5c, spec §2.2): the table layer compiled to
//! WebAssembly for the Durable Objects — ONE `Games` over a `MemStore`, the
//! [`signal::RecordingSignal`], the archive the switch names (`NoArchive`, or
//! under `r2` the receipt archive, `receipt.rs`) and a settable clock, behind
//! `wasm-bindgen` exports built for the objects' shell (spec §2.2).
//!
//! **`crates/yokohama-wasm`'s shape, for that crate's reason**: it is a HOST and
//! so is this one. What the two share is the layer — since M6.5c2 the items
//! each host carried a copy of among it (spec §2.6): both hosts' clock
//! (`games::HostClock`) and platform scan (`platform::calls_in`), and this
//! host's error body, wait check and house frame. The live clock reading and
//! each scan's allowed calls stay each host's own.
//!
//! This is HOST code. Its platform calls are the clock and the bindgen glue and
//! nothing else — `this_host_makes_no_platform_call_but_the_clock` holds that
//! over every `src/*.rs` of this crate, through the layer's own
//! `platform::calls_in`, over its eight patterns (`platform::CALLS`), reading
//! each file down to its first top-level `#[cfg(test)]`.
//!
//! **Every export but two is a thin wrapper over a pure-Rust `_inner`
//! function** (spec §2.2). The cells call the inner ones, because
//! `wasm-bindgen`'s non-wasm stubs cannot make a `JsValue` — so those wrappers
//! carry no logic beyond the conversion a JS caller needs, and everything that
//! could be wrong is reachable under `cargo test`. The two: [`set_clock`],
//! whose logic is the export itself — it takes and answers plain numbers, so
//! the cells call it as it is — and `debug_trap`, the battery's trap hook,
//! which is its own body (`--features debug` only).
//!
//! `build.rs` runs on the build host, never in the module: it digests the
//! engine's, the CLI's and the layer's sources into the picker row's code
//! identity (`picker.rs`), and the scan above reads `src/` alone.

mod config;
mod create;
mod events;
mod house;
pub mod log;
mod picker;
mod receipt;
pub mod signal;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;
use yokohama_web::api::{self, ApiRequest, Host};
use yokohama_web::archive::{Archive, NoArchive};
use yokohama_web::games::{Games, HostClock};
use yokohama_web::identity::{admit, resolve, User};
use yokohama_web::store::{Doc, MemStore};
use yokohama_web::ApiError;

pub use crate::create::create_inner;
pub use crate::events::events_inner;
pub use crate::house::{house_copy_inner, house_frame_inner, put_copy_inner, remove_copy_inner};
pub use crate::picker::restore_picker_inner;
pub use crate::receipt::{expire_inner, receipt_inner, stamp_archive_inner};
use crate::log::{LogLine, ShellLog};
use crate::receipt::ReceiptArchive;
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

/// The door [`init`] was given (M6.5c2, spec §2.1): replaced with the rest of
/// the layer at every `init`, so an instance answers every call under ONE door
/// — the door its object's own configuration names.
pub(crate) static DOOR: Mutex<Option<config::Door>> = Mutex::new(None);

/// The door `init` set, which every call that reads a caller or reports the
/// host's facts follows.
fn door() -> config::Door {
    DOOR.lock().expect("the host's door").expect("init() before a caller is resolved")
}

/// The log the one `Games` holds — the SAME `Arc`, so what the layer logs
/// during a call is drained into that call's reply (M6.5c2).
pub(crate) static LOG: Mutex<Option<Arc<ShellLog>>> = Mutex::new(None);

/// The clock: the layer's `HostClock` (M6.5c2, spec §2.6; R1's cleanup debt
/// 4) — frozen while pinned, else live — where this host carried its own copy
/// of the browser host's three statics.
///
/// Live is `Date.now()` on the target and the clock's own tick everywhere else
/// — the crate's own cells and the workspace lint gate compile for the host,
/// and `js_sys::Date::now` there is a stub that panics. The tick counts 1, 2,
/// 3, … for `Games::ticking_clock`'s reason: a constant would make every
/// table's `created_at` a tie.
///
/// **[`set_clock`] is its only writer.** [`init`] rebuilds everything else
/// this crate holds and leaves the clock as it was: the plan's ruling P-2 has
/// the shell pin a reading before every call that can reach the layer's
/// clock.
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

/// Pin the clock to `seconds`, or pass `undefined` for the live clock — and get
/// back what it replaced (`undefined` = live), so a caller that pins a reading
/// for one call can restore a test configuration's frozen clock after it (spec
/// §2.6). Whole seconds: a fraction is dropped, as the live arm drops it — the
/// `f64` is this export's, and the layer's clock keeps whole seconds.
#[wasm_bindgen]
pub fn set_clock(seconds: Option<f64>) -> Option<f64> {
    CLOCK.set(seconds.map(|s| s as u64)).map(|s| s as f64)
}

/// The deployment facts this host reports (spec §2.2; §4's second class), for
/// the door `init` was given (M6.5c2, spec §2.1): its name on the wire —
/// `cf-access` under `worker`, although the module reads the `worker` mode's
/// header, because the door a client is behind is Cloudflare Access and
/// R-6.5c.3 has the Worker forward its verdict to the objects as
/// `X-Yokohama-User`; `open` under `open` ([`config::Door::wire_name`]).
/// `"ws"`: a table's changes reach a browser over a WebSocket (spec §2.7).
/// Multiplayer is both served and offered.
fn host() -> Host {
    Host { identity_name: door().wire_name(), signal_kind: "ws", serves_games: true, multiplayer_offered: true }
}

/// Build a FRESH layer from the shell's configuration, replacing any earlier
/// one — the door (M6.5c2), the `Games`, its store, its signal, its archive and
/// its log together, an empty `picker::PICKERS`, and `receipt::RECEIPTS`: under
/// `r2` a new receipt archive with no receipt and no pending plan, under `off`
/// none. A refused configuration changes nothing — the door included, which is
/// stored only once `config::parse` has accepted the whole. A bad VALUE's
/// refusal names its variable (spec §2.2); a configuration of the wrong shape
/// names none — JSON that is not an object is refused in this crate's own
/// sentence, which names the six keys, and an unknown or missing key or a
/// value of the wrong JSON type in serde's words (`config.rs`) — and the shell
/// never builds one. The clock is not reset: [`set_clock`] is its only writer.
pub fn init_inner(config: &str) -> Result<(), String> {
    let config::Config { door, chat, archive } = config::parse(config)?;
    // The door before the `Games` is built: the layer is built for it.
    *DOOR.lock().expect("the host's door") = Some(door);
    let store = Arc::new(MemStore::default());
    let signal = Arc::new(RecordingSignal::default());
    // The SAME `Arc` the layer holds, kept so a receipt reaches the archive the
    // layer puts through, and so its pending plan can be drained into a reply.
    let receipts = match archive {
        config::Switch::Off => None,
        config::Switch::R2 { caller } => Some(Arc::new(ReceiptArchive::new(caller))),
    };
    let layer_archive: Arc<dyn Archive> = match &receipts {
        Some(receipts) => receipts.clone(),
        None => Arc::new(NoArchive),
    };
    let log = Arc::new(ShellLog::default());
    let games = Games::with_parts(
        Arc::new(yokohama_core::Rules::standard()),
        store.clone(),
        chat,
        signal.clone(),
        Arc::new(now),
        layer_archive,
        yokohama_web::games::RunHost::Worker,
    )
    .with_log(log.clone());
    *STORE.lock().expect("the host's store") = Some(store);
    *SIGNAL.lock().expect("the host's signal") = Some(signal);
    *LOG.lock().expect("the host's log") = Some(log);
    *picker::PICKERS.lock().expect("the host's pickers") = Some(HashMap::new());
    *receipt::RECEIPTS.lock().expect("the host's receipts") = receipts;
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
    /// The receipt archive's deferred plan (spec §2.6), as JSON
    /// (`receipt::Plan`): a put during this call met no receipt, and the shell
    /// owes the upload. `None` when no put was deferred — always, with the
    /// archive `off`.
    pub archive: Option<String>,
    /// The table's `picker` row (spec §2.4): the list's JSON when it grew, and
    /// `"[]"` when a list existed and was cleared, which the shell reads as
    /// "delete the row". `None` when neither happened (`picker.rs`).
    pub picker: Option<String>,
    /// A stored picker row `restore_picker` cleared for its CODE — built under
    /// code other than this build's (the orchestrator's ruling I-85) — as
    /// `{"stored": <its code, or null>, "running": <this build's>}`, which the
    /// shell logs as `picker-dropped`. `None` from every other call.
    pub dropped: Option<String>,
    /// The layer's log lines during this call (M6.5c2, spec §2.6), in call
    /// order, less the one deferral `log.rs` drops. Empty from a call that
    /// never reached the layer.
    pub logs: Vec<LogLine>,
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

    /// The row as JSON when the list grew, `[]` when it was cleared, and
    /// `undefined` when neither happened.
    #[wasm_bindgen(getter)]
    pub fn picker(&self) -> Option<String> {
        self.data.picker.clone()
    }

    /// `{"stored", "running"}` when `restore_picker` cleared a row for its
    /// code, and `undefined` otherwise.
    #[wasm_bindgen(getter)]
    pub fn dropped(&self) -> Option<String> {
        self.data.dropped.clone()
    }

    /// The layer's log lines as JSON: `[]` when it logged nothing, else one
    /// `{"event", "fields"}` per line, which the shell logs as its own.
    #[wasm_bindgen(getter)]
    pub fn logs(&self) -> String {
        serde_json::to_string(&self.data.logs).expect("a log line serializes")
    }
}

/// The layer's error body — `{"error": …}`, with `"status"` and
/// `"foreign_bot"` beside it when set: the layer's OWN `api::error_body` since
/// M6.5c2 (spec §2.6; R1's cleanup debt 4), the bytes `dispatch` writes for
/// every refusal, re-exported so this crate's host-written refusals name one
/// path.
pub(crate) use yokohama_web::api::error_body;

/// SHA-256 of `text`'s bytes, in lowercase hex: the one digest this host takes
/// — the picker row's `record` (spec §2.4), and the receipt archive's two body
/// digests and its plan key (spec §2.6).
pub(crate) fn sha256_hex(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

/// A refusal this host makes ITSELF, before the `Games` is touched: its body is
/// [`error_body`]'s, and its `bumps` is empty because this call signalled
/// nothing.
pub(crate) fn host_refusal(e: ApiError) -> ReplyData {
    ReplyData {
        status: e.status.as_u16(),
        body: error_body(&e),
        bumps: Vec::new(),
        archive: None,
        picker: None,
        dropped: None,
        logs: Vec::new(),
    }
}

/// Who the caller is, by the door's ONE header (spec §2.3; M6.5c2, spec §2.1):
/// the value the shell hands over goes into a header map under the header the
/// door's mode reads — `X-Yokohama-User` under `worker`,
/// `X-Yokohama-Claimed-User` under `open` — and is read by the layer's own
/// `identity::resolve` in that mode, with its key grammar and its mode rule:
/// under `worker` an identity must have the email shape and a plain name
/// reads as absent; under `open` any key the grammar accepts is a caller.
/// `anonymous`, like an absent or blank value, is no caller, and a refusal is
/// the layer's own sentence.
///
/// A value no header can carry — one holding a control character other than a
/// tab — never reaches `resolve`, so it takes `resolve`'s two steps here, in
/// the door's mode: the key grammar (`User::parse`, trimming as `resolve`
/// would and refusing any control character that remains) and then the mode's
/// rule (`identity::admit`), so `"ben\r"` is nobody at the worker door exactly
/// as a header naming `ben` is (review R2-m1), and `ben` at the open door. It
/// is a 401 wherever the mode refuses, with one difference in wording: a value
/// whose only such characters are at its ends, which the trim removes, and
/// which trims to nothing or to `anonymous` — `"anonymous\n"` — gets
/// `User::parse`'s sentence, where `resolve` would answer its absent-caller
/// sentence had a header carried it. No header can: the shell never forwards
/// such a value (`address` in `worker/src/identity.ts`).
pub fn identity_of(user: Option<&str>) -> Result<User, String> {
    let mode = door().mode();
    let mut headers = http::HeaderMap::new();
    if let Some(raw) = user {
        let Ok(value) = http::HeaderValue::from_str(raw) else {
            return User::parse(raw).and_then(|user| admit(mode, user));
        };
        headers.insert(mode.header(), value);
    }
    resolve(mode, &headers)
}

/// [`identity_of`] as a reply: 200 `{"user": <the normalized address>}`, or 401
/// in the layer's error shape — the very body the layer's 401 writes for the
/// same caller. It reads no document and signals nothing.
///
/// The 401 body is built here by hand, as `{"error": <sentence>}`, because this
/// entry point is no row of the layer's route table and never reaches the
/// layer's own writer; the shape is the layer's, kept equal by the cells below
/// and not by sharing code.
///
/// It is for admitting the lobby's socket, `GET /api/stream` (spec §2.5): that
/// route is no row of the layer's route table, so [`dispatch`] would answer it
/// 404.
pub fn identity_inner(user: Option<&str>) -> ReplyData {
    let (status, body) = match identity_of(user) {
        Ok(u) => (200, serde_json::json!({ "user": u.as_str() }).to_string()),
        Err(sentence) => (401, serde_json::json!({ "error": sentence }).to_string()),
    };
    ReplyData { status, body, bumps: Vec::new(), archive: None, picker: None, dropped: None, logs: Vec::new() }
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
/// a caller refuses in the door's mode's own words. The host's facts are
/// `host()`'s, for the same door.
///
/// It answers through `from_the_layer` under `Scope::Dispatch`, so the
/// signal is drained while the `Games` is still held — and with spec §2.4's
/// picker step around the layer's call (`picker::around`), which reads the
/// record before and after it and answers `Reply.picker`.
pub fn dispatch_inner(
    user: Option<&str>,
    method: &str,
    path: &str,
    query: &str,
    content_type: Option<&str>,
    body: &[u8],
) -> ReplyData {
    let caller = identity_of(user);
    // Read before `from_the_layer` takes `GAMES`, so the door's lock is never
    // taken under it.
    let host = host();
    let identity: api::Identity<'_> = match &caller {
        Ok(u) => Ok(u),
        Err(sentence) => Err(sentence.as_str()),
    };
    let req = ApiRequest { method, path, query, content_type, body };
    from_the_layer(Scope::Dispatch, |games| {
        let (res, picker) =
            picker::around(games, &req, caller.as_ref().ok(), || api::dispatch(games, &host, identity, &req));
        (res.status, res.body, picker)
    })
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

/// [`create_inner`] for the shell.
#[wasm_bindgen]
pub fn create(user: Option<String>, body: &[u8], id: &str, content_type: Option<String>) -> Reply {
    create_inner(user.as_deref(), body, id, content_type.as_deref()).into()
}

/// [`events_inner`] for the shell.
#[wasm_bindgen]
pub fn events(id: &str, user: Option<String>, query: &str) -> Reply {
    events_inner(id, user.as_deref(), query).into()
}

/// [`restore_picker_inner`] for the shell: `row` is the stored `picker` row's
/// JSON, or `"null"` when there is none.
#[wasm_bindgen]
pub fn restore_picker(id: &str, row: &str) -> Reply {
    restore_picker_inner(id, row).into()
}

/// [`house_copy_inner`] for the shell: the copy, `undefined` when no table is
/// stored under `id`, and a refusal thrown as its sentence.
#[wasm_bindgen]
pub fn house_copy(id: &str) -> Result<Option<String>, JsValue> {
    house_copy_inner(id).map_err(|e| JsValue::from_str(&e))
}

/// [`put_copy_inner`] for the shell: a refusal is thrown as its sentence.
#[wasm_bindgen]
pub fn put_copy(id: &str, json: &str) -> Result<(), JsValue> {
    put_copy_inner(id, json).map_err(|e| JsValue::from_str(&e))
}

/// [`remove_copy_inner`] for the shell: a refusal is thrown as its sentence.
#[wasm_bindgen]
pub fn remove_copy(id: &str) -> Result<(), JsValue> {
    remove_copy_inner(id).map_err(|e| JsValue::from_str(&e))
}

/// [`receipt_inner`] for the shell: `outcome` is the outcome's JSON, and a
/// refusal is thrown as its sentence.
#[wasm_bindgen]
pub fn receipt(plan_key: &str, outcome: &str) -> Result<(), JsValue> {
    receipt_inner(plan_key, outcome).map_err(|e| JsValue::from_str(&e))
}

/// [`stamp_archive_inner`] for the shell.
#[wasm_bindgen]
pub fn stamp_archive(id: &str) -> Reply {
    stamp_archive_inner(id).into()
}

/// [`expire_inner`] for the shell.
#[wasm_bindgen]
pub fn expire(id: &str) -> Reply {
    expire_inner(id).into()
}

/// [`house_frame_inner`] for the shell, which passes each counter as a JS
/// `number`: the parameters are `u32` because `wasm-bindgen` types a `u64`
/// parameter `bigint` (the orchestrator's ruling I-22). The JS `number` to
/// `u32` step is the lossy one — `wasm-bindgen` wraps a value above 2^32 − 1
/// and drops a fraction, the cost that ruling accepted — and the `u32` then
/// widens to the layer's `u64` without loss.
#[wasm_bindgen]
pub fn house_frame(seq: u32, chat: u32) -> String {
    house_frame_inner(u64::from(seq), u64::from(chat))
}

/// Which of spec §2.4's picker rules a call into the layer answers to (R1's
/// cleanup debt 1; spec §2.7). [`from_the_layer`] takes one with every call,
/// so no call reaches `Games` without its rule named.
pub(crate) enum Scope<'a> {
    /// A dispatch: `picker::around`, INSIDE the call, applies the rule — the
    /// input rule after a 200 `…/input`, and a changed record's clear after
    /// any other dispatch at or below `/api/games/{id}`.
    Dispatch,
    /// Any other call on the game `id` names — `events`, `stamp_archive`,
    /// `expire`'s arm with no creator, `restore_picker`: `id`'s record is read
    /// before and after the call, and a change clears `id`'s picker
    /// (`picker::watching`). Over a game in play, such a call changes the
    /// record by REBUILDING a cold handle, whose owed bot pump persists a move,
    /// and a rebuilt `Repl` starts with its picker closed — so the whole entry
    /// goes, a `Dropped` one included. (`restore_picker` can also move by a
    /// re-fed line, and then clears the list and closes the picker itself.)
    Table(&'a str),
}

/// One call into the layer under the held `GAMES`, answered with what the layer
/// signalled DURING it, and the plan of any put it deferred.
///
/// `call` answers the status, the body and `Reply.picker` — which only the two
/// calls that can change a picker's CONTENT compute (`picker.rs`); every other
/// call answers `None` there. A [`Scope::Table`] call can still CLEAR a picker,
/// through the watch on its game's record, without computing one.
///
/// **Spec §2.4's picker rule rides every call** (R1's cleanup debt 1):
/// `scope` names it — [`Scope::Dispatch`] where `around` applies it inside the
/// call, [`Scope::Table`] for any other call on a game, whose record is watched
/// here. A call's own `Reply.picker` wins over the watch's: `restore_picker`
/// answers its own.
///
/// **The signal and the pending plan are drained while the `Games` is still
/// held**, so each is this call's alone — including a cold handle's bot pump
/// that persisted before the request was refused, which is recorded because it
/// happened (spec §5.1), and the game-over put that pump deferred.
/// [`dispatch_inner`] (and so `create`), [`events_inner`],
/// [`restore_picker_inner`], [`stamp_archive_inner`] and [`expire_inner`] all
/// answer through here (plan re-review m-c; plan review M4), so no two can
/// drain differently.
pub(crate) fn from_the_layer(
    scope: Scope<'_>,
    call: impl FnOnce(&Games) -> (u16, String, Option<String>),
) -> ReplyData {
    let guard = GAMES.lock().expect("the host's games");
    let games = guard.as_ref().expect("init() before any call into the layer");
    let (status, body, picker) = match scope {
        Scope::Dispatch => call(games),
        Scope::Table(id) => {
            let ((status, body, picker), cleared) = picker::watching(games, id, || call(games));
            (status, body, picker.or(cleared))
        }
    };
    ReplyData {
        status,
        body,
        bumps: drain_bumps(),
        archive: receipt::drain_plan(),
        picker,
        dropped: None,
        logs: drain_logs(),
    }
}

/// Everything the layer signalled since the last drain.
fn drain_bumps() -> Vec<Bump> {
    SIGNAL.lock().expect("the host's signal").as_ref().expect("init() before any call into the layer").drain()
}

/// Everything the layer logged since the last drain.
fn drain_logs() -> Vec<LogLine> {
    LOG.lock().expect("the host's log").as_ref().expect("init() before any call into the layer").drain()
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
    use yokohama_web::identity::IdentityMode;

    /// The cells share the global `GAMES` and the global clock; they run one at
    /// a time, as `yokohama-wasm`'s do.
    static SERIAL: Mutex<()> = Mutex::new(());

    /// A poisoned `SERIAL` would fail every later cell with the wrong message,
    /// so the guard is taken through the poison rather than unwrapped.
    ///
    /// Every cell that touches `GAMES` or the clock takes it — `picker.rs`'s
    /// too, through `crate::`, because the orchestrator's ruling M-5 puts each
    /// later task's cells in that task's own file.
    pub(crate) fn serial() -> std::sync::MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn config(archive: &str) -> String {
        door_config(archive, "worker")
    }

    /// `init`'s configuration at `door` (M6.5c2, spec §2.1): the object
    /// `worker/src/config.ts`'s `moduleConfig` builds, the door first.
    pub(crate) fn door_config(archive: &str, door: &str) -> String {
        format!(
            r#"{{"identity":"{door}","archive":"{archive}","caller":null,"lobby_chat":null,"chat_keep":null,"chat_text":null}}"#
        )
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
        assert!(r.bumps.is_empty() && r.archive.is_none() && r.picker.is_none() && r.logs.is_empty(), "an identity read writes nothing");
    }

    /// **The fallback follows the mode rule** (M6.5c2, spec §2.1 rule 4; review
    /// R2-m1). A value no header can carry never reaches `resolve` as a
    /// header, and the key grammar alone trims `"ben\r"` to the plain name
    /// `ben` — a key the `worker` door admits as nobody. So the fallback takes
    /// `resolve`'s two steps itself, and `"ben\r"` answers exactly what a
    /// header naming `ben` answers: the mode's absent sentence. An ADDRESS the
    /// same trim recovers is still that caller — the anti-vacuity, since a
    /// fallback that refused everything would pass the first half.
    #[test]
    fn a_value_no_header_can_carry_is_held_to_the_worker_modes_rule() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let absent = resolve(IdentityMode::Worker, &http::HeaderMap::new()).unwrap_err();
        let r = identity_inner(Some("ben\r"));
        assert_eq!((r.status, r.body.as_str()), (401, serde_json::json!({ "error": absent }).to_string().as_str()));
        let as_header = identity_inner(Some("ben"));
        assert_eq!((as_header.status, &as_header.body), (r.status, &r.body), "the carrier never decides who it is");
        let r = identity_inner(Some("Red@Wire.Test\r"));
        assert_eq!((r.status, r.body.as_str()), (200, r#"{"user":"red@wire.test"}"#));
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

    /// **The open door** (M6.5c2, spec §2.1; R-6.5c2.5): `init`'s `identity`
    /// fixes the mode a caller is resolved under, and the door's name on the
    /// wire. Under `open` a claimed NAME is a caller, keyed by the layer's own
    /// grammar (trimmed, space runs collapsed, ASCII-lowercased), at every
    /// admission — `dispatch` and the lobby socket's `identity` — and
    /// `/api/me` and `/api/health` say `open`. The anti-vacuity partner is the
    /// `worker` door at the next `init`: `cf-access`, and the same name no one
    /// (the mode rule). Mutants: the door parsed and never read; the header
    /// left at the `worker` mode's (every open caller no one); `Host` left
    /// `cf-access`.
    #[test]
    fn the_open_door_resolves_a_claimed_name_and_names_itself_open() {
        let _s = serial();
        init_inner(&door_config("off", "open")).unwrap();
        let r = dispatch_inner(Some("  Ben   Castle "), "GET", "/api/me", "", None, b"");
        assert_eq!((r.status, r.body.as_str()), (200, r#"{"mode":"open","user":"ben castle"}"#));
        let r = dispatch_inner(None, "GET", "/api/me", "", None, b"");
        assert_eq!(r.body, r#"{"mode":"open","user":null}"#);
        let health = dispatch_inner(Some("ben"), "GET", "/api/health", "", None, b"").body;
        assert!(health.contains(r#""identity":"open""#), "{health}");
        let r = identity_inner(Some("Ben"));
        assert_eq!((r.status, r.body.as_str()), (200, r#"{"user":"ben"}"#), "the lobby socket's admission, at the same door");
        let r = identity_inner(None);
        assert_eq!(r.status, 401);
        assert!(!r.body.contains("from the edge Worker"), "the open mode's own sentence, not the worker mode's: {}", r.body);

        init_inner(&config("off")).unwrap();
        let r = dispatch_inner(Some("ben"), "GET", "/api/me", "", None, b"");
        assert_eq!(r.body, r#"{"mode":"cf-access","user":null}"#, "the worker door: a plain name is no one");
    }

    /// `init` takes the Worker's two doors and no default (M6.5c2, spec §2.1).
    /// `cf-access` is the NAS's door, not this host's — here the shell verifies
    /// Access itself, which is `worker` — and a configuration naming no door is
    /// refused in serde's words. A refused `init` changes nothing: the door the
    /// last accepted one set stands. Mutants: a default door; any other string
    /// read as `worker`; the door stored before the configuration is accepted.
    #[test]
    fn init_takes_the_workers_two_doors_and_no_default() {
        let _s = serial();
        init_inner(&door_config("off", "open")).unwrap();
        let err = init_inner(&door_config("off", "cf-access")).unwrap_err();
        assert_eq!(err, r#"YOKOHAMA_WEB_IDENTITY must be worker or open — got "cf-access""#);
        let err = init_inner(r#"{"archive":"off","caller":null,"lobby_chat":null,"chat_keep":null,"chat_text":null}"#)
            .unwrap_err();
        assert!(err.contains("missing field `identity`"), "{err}");
        let err = init_inner(&door_config("s3", "worker")).unwrap_err();
        assert!(err.contains("YOKOHAMA_WEB_ARCHIVE"), "{err}");
        let r = dispatch_inner(Some("ben"), "GET", "/api/me", "", None, b"");
        assert_eq!(r.body, r#"{"mode":"open","user":"ben"}"#, "every refused init changed nothing");
    }

    /// Spec §2.1 rule 4, at BOTH doors: a value no header can carry never
    /// reaches `resolve` as itself. It takes `resolve`'s two steps here — the
    /// key grammar, then the mode rule in the DOOR's mode — so `"ben\r"` is
    /// the name `ben` at the open door and no one at the worker door
    /// (`a_value_no_header_can_carry_is_held_to_the_worker_modes_rule`), in
    /// the very words an absent caller gets; and a control character the
    /// grammar's trim leaves inside the value is the grammar's refusal at
    /// either door. The shell never forwards such a value (`address` in
    /// `worker/src/identity.ts`). Mutant: the fallback's mode left at `Worker`
    /// (the open door refuses `ben`).
    #[test]
    fn a_value_no_header_can_carry_is_judged_under_the_doors_own_mode() {
        let _s = serial();
        init_inner(&door_config("off", "open")).unwrap();
        let r = identity_inner(Some("ben\r"));
        assert_eq!((r.status, r.body.as_str()), (200, r#"{"user":"ben"}"#));
        init_inner(&config("off")).unwrap();
        let worker = identity_inner(Some("ben\r"));
        let absent = identity_inner(None);
        assert_eq!((worker.status, &worker.body), (401, &absent.body), "a plain name at the worker door reads as absent");
        for door in ["open", "worker"] {
            init_inner(&door_config("off", door)).unwrap();
            let r = identity_inner(Some("be\rn"));
            let want = serde_json::json!({ "error": User::parse("be\rn").unwrap_err() }).to_string();
            assert_eq!((r.status, r.body.as_str()), (401, want.as_str()), "{door}");
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
        let err = init_inner(r#"{"identity":"worker","archive":"off","caller":null,"lobby_chat":"maybe","chat_keep":null,"chat_text":null}"#).unwrap_err();
        assert!(err.contains("YOKOHAMA_WEB_LOBBY_CHAT"), "{err}");
        init_inner(r#"{"identity":"worker","archive":"off","caller":null,"lobby_chat":"off","chat_keep":"all","chat_text":"280"}"#).unwrap();
        let body = dispatch_inner(Some("red@wire.test"), "GET", "/api/health", "", None, b"").body;
        for fact in [r#""lobby_chat":false"#, r#""chat_keep":null"#, r#""chat_text":280"#] {
            assert!(body.contains(fact), "{fact} in {body}");
        }
        // Anti-vacuity: the defaults read otherwise, so each fact above is
        // the variable's.
        init_inner(&config("off")).unwrap();
        let body = dispatch_inner(Some("red@wire.test"), "GET", "/api/health", "", None, b"").body;
        for fact in [r#""lobby_chat":true"#, r#""chat_keep":500"#, r#""chat_text":500"#] {
            assert!(body.contains(fact), "the default {fact} in {body}");
        }
    }

    /// `init` reads its configuration ONLY as the object the shell builds
    /// (R1's L-3). serde's derived `Deserialize` also reads a struct from a
    /// positional ARRAY, a form `deny_unknown_fields` does not govern, so an
    /// array is refused before serde reads it — the rule this crate keeps for
    /// a create body, the picker row and a receipt's outcome. The refusal
    /// echoes nothing: the configuration carries the Archive object's address.
    #[test]
    fn init_refuses_its_configuration_as_a_positional_array() {
        let _s = serial();
        for array in [r#"["off",null,null,null,null]"#, r#"["r2","red@t.test",null,null,null]"#] {
            let err = init_inner(array).unwrap_err();
            assert_eq!(
                err,
                "the module's init configuration is the object of its six keys — identity, archive, caller, lobby_chat, chat_keep and chat_text",
                "{array}"
            );
        }
        init_inner(&config("off")).unwrap();
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

    /// **A plain name on a table's watcher list reads on this host too**
    /// (M6.5c2, spec §3's identity cells; review H2): `check_shape` runs on the
    /// module's every table read, over the key grammar an `open` host writes
    /// watchers under. The table is this host's own — created and dealt by its
    /// routes — and its roster is given a plain name by hand, because this
    /// module's `worker` door admits no plain-name CALLER to unlock with (the
    /// mode rule), which is not the property here.
    #[test]
    fn a_table_whose_watcher_list_holds_a_plain_name_reads() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let body = r#"{"players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,"seats":[{"user":"red@t.test"},{"user":"blue@t.test"}],"password":"sesame"}"#;
        assert_eq!(create_inner(Some("red@t.test"), body.as_bytes(), "named", Some("application/json")).status, 201);
        for (who, seat) in [("red@t.test", 0), ("blue@t.test", 1)] {
            let keep = format!(r#"{{"seat":{seat},"card":1}}"#);
            assert_eq!(call(who, "POST", "/api/games/named/keep", Some(&keep)).status, 200, "{who}");
        }
        let mut doc: serde_json::Value = serde_json::from_str(&read_inner("table", "named").unwrap().unwrap()).unwrap();
        let record = read_inner("record", "named").unwrap().expect("the last keep dealt the game");
        for (watcher, status) in [("ben castle", 200), ("..", 500)] {
            doc["unlocked"] = serde_json::json!([watcher]);
            init_inner(&config("off")).unwrap();
            load_inner("table", "named", &doc.to_string()).unwrap();
            load_inner("record", "named", &record).unwrap();
            let r = call("red@t.test", "GET", "/api/games/named/table", None);
            // `..` is the anti-vacuity: the grammar still refuses it, so 200
            // above is a shape check that passed, not one that never ran.
            assert_eq!(r.status, status, "{watcher:?}: {}", r.body);
        }
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

    pub(crate) fn call(user: &str, method: &str, path: &str, body: Option<&str>) -> ReplyData {
        let bytes = body.map(|b| b.as_bytes().to_vec()).unwrap_or_default();
        dispatch_inner(Some(user), method, path, "", body.map(|_| "application/json"), &bytes)
    }
    /// The reply's bumps as the shell reads them: through `Reply`'s own
    /// `bumps` getter (Task 2's M5), not a second serialization of the list.
    pub(crate) fn tagged(r: &ReplyData) -> String {
        Reply::from(r.clone()).bumps()
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

    /// **A GET can write** (the layer's `lib.rs:523-539`): a cold handle rebuilt with a bot to
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

    pub(crate) const CREATE: &str = r#"{"players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,"seats":[{"user":"red@t.test"},{"open":true}]}"#;

    fn created_id(r: &ReplyData) -> String {
        serde_json::from_str::<serde_json::Value>(&r.body).unwrap()["id"].as_str().unwrap().to_string()
    }

    /// Every id the layer's store holds a `doc` under — read through the
    /// `Games`, so it is the store every create writes. A refused create must
    /// leave it empty under ANY id: a body that names no id would have its
    /// table stored under an id the layer drew.
    fn stored(doc: Doc) -> Vec<String> {
        GAMES.lock().unwrap().as_ref().unwrap().store().list(doc).unwrap()
    }

    /// The Worker names the object BEFORE the layer runs, so a body with no id
    /// gets the object's name inserted, and every 201 names the object that holds
    /// the table (spec §2.2).
    #[test]
    fn an_absent_id_is_inserted_and_every_201_names_the_object() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let r = create_inner(Some("red@t.test"), CREATE.as_bytes(), "sim-00000000000000aa", Some("application/json"));
        assert_eq!(r.status, 201, "{}", r.body);
        assert_eq!(created_id(&r), "sim-00000000000000aa");
        assert!(read_inner("table", "sim-00000000000000aa").unwrap().is_some(), "stored under the object's name");
    }

    /// A string `id` that IS the object's name — the arm every explicit-id
    /// create takes — goes on UNCHANGED (spec §2.2): a 201 naming the object,
    /// and exactly that one table stored. Inserting the name again would be the
    /// layer's duplicate-`id` 400; reading every string id as misrouted would be
    /// the host's 500.
    #[test]
    fn a_string_id_that_is_the_objects_name_goes_on_unchanged() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let body = CREATE.replacen('{', r#"{"id":"sim-00000000000000ab","#, 1);
        let r = create_inner(Some("red@t.test"), body.as_bytes(), "sim-00000000000000ab", Some("application/json"));
        assert_eq!(r.status, 201, "{}", r.body);
        assert_eq!(created_id(&r), "sim-00000000000000ab");
        assert_eq!(stored(Doc::Table), ["sim-00000000000000ab"]);
    }

    /// **Review Focus 1**: what `curl --data-binary @file` sends of a file an
    /// editor saved with a final newline — the closing brace is NOT the last byte.
    /// (`curl --data @file` strips carriage returns and newlines but keeps spaces
    /// and tabs — measured with curl 8.7.1.)
    #[test]
    fn trailing_whitespace_after_the_closing_brace_still_inserts_before_it() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        for (n, tail) in ["\n", "  \r\n", "\t \n\n"].iter().enumerate() {
            let id = format!("sim-000000000000000{n}");
            let body = format!("{CREATE}{tail}");
            let r = create_inner(Some("red@t.test"), body.as_bytes(), &id, Some("application/json"));
            assert_eq!(r.status, 201, "tail {tail:?}: {}", r.body);
            assert_eq!(created_id(&r), id, "tail {tail:?}");
        }
    }

    /// `"id": null` is the one body the layer reads as ABSENT after the Worker has
    /// named the object — measured by the re-review to answer 201 for a table no
    /// object held. Refused, and nothing stored under ANY id (`stored`): a table
    /// made from this body would sit under an id the layer drew.
    #[test]
    fn a_null_id_is_refused_and_nothing_is_stored() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let body = CREATE.replacen('{', r#"{"id":null,"#, 1);
        let r = create_inner(Some("red@t.test"), body.as_bytes(), "sim-00000000000000bb", Some("application/json"));
        assert_eq!(r.status, 400);
        assert_eq!(r.body, r#"{"error":"a game id, when given, must be a string"}"#);
        assert!(stored(Doc::Table).is_empty() && stored(Doc::Record).is_empty(), "nothing stored");
        // Anti-vacuity: the same listing names a table the moment one IS stored.
        let r = create_inner(Some("red@t.test"), CREATE.as_bytes(), "sim-00000000000000bb", Some("application/json"));
        assert_eq!(r.status, 201, "{}", r.body);
        assert_eq!(stored(Doc::Table), ["sim-00000000000000bb"]);
    }

    /// A body the layer would refuse reaches it UNCHANGED, so its 400 is the NAS's
    /// byte for byte — including the missing field whose column the first text's
    /// insertion moved from 68 to 96 (spec §0.4).
    ///
    /// The last two cases carry TWO defects each — a shape this host refuses
    /// itself (`"id": null`, a foreign string id) AND a key the layer refuses —
    /// which pins the layer's parse FIRST (spec §2.2: "only that decides"). A
    /// host that read the id before the parse would answer them with its own
    /// 400 and 500.
    #[test]
    fn a_refused_body_is_forwarded_unchanged_and_answered_in_the_layers_words() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let cases: [(&str, &str); 8] = [
            ("a duplicate key", r#"{"players":2,"players":2,"seats":[{"user":"red@t.test"}]}"#),
            ("an unknown field", r#"{"players":2,"seats":[{"user":"red@t.test"}],"bogus":1}"#),
            ("a missing field", r#"{"seats":[{"user":"red@t.test"}],"colors":["Red","Blue"],"start_seat":0,"seed":1}"#),
            ("a non-string id", r#"{"id":5,"players":2,"seats":[{"user":"red@t.test"}]}"#),
            ("a non-object", "[1,2]"),
            ("an unparseable body", "{\"players\":"),
            (
                "a null id AND an unknown field",
                r#"{"id":null,"players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,"seats":[{"user":"red@t.test"},{"open":true}],"bogus":1}"#,
            ),
            (
                "a foreign id AND an unknown field",
                r#"{"id":"elsewhere","players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,"seats":[{"user":"red@t.test"},{"open":true}],"bogus":1}"#,
            ),
        ];
        for (what, body) in cases {
            let via_create = create_inner(Some("red@t.test"), body.as_bytes(), "sim-00000000000000cc", Some("application/json"));
            let direct = dispatch_inner(Some("red@t.test"), "POST", "/api/games", "", Some("application/json"), body.as_bytes());
            assert_eq!(via_create.status, 400, "{what}: {}", via_create.body);
            assert_eq!((via_create.status, via_create.body), (direct.status, direct.body), "{what}: forwarded unchanged");
        }
        let missing = create_inner(Some("red@t.test"), cases[2].1.as_bytes(), "sim-00000000000000cc", Some("application/json"));
        let column = cases[2].1.len();
        assert!(missing.body.contains(&format!("column {column}")), "the ORIGINAL body's column: {}", missing.body);
        let no_type = create_inner(Some("red@t.test"), CREATE.as_bytes(), "sim-00000000000000cc", None);
        let direct = dispatch_inner(Some("red@t.test"), "POST", "/api/games", "", None, CREATE.as_bytes());
        assert_eq!((no_type.status, no_type.body), (direct.status, direct.body), "a missing content type is the layer's own answer");
    }

    /// A `u64`-max seed reaches the stored table exactly, through the insertion
    /// arm.
    #[test]
    fn a_u64_seed_is_stored_exactly() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let body = CREATE.replace("\"seed\":20260831", "\"seed\":\"18446744073709551615\"");
        let r = create_inner(Some("red@t.test"), body.as_bytes(), "sim-00000000000000dd", Some("application/json"));
        assert_eq!(r.status, 201, "{}", r.body);
        assert!(read_inner("table", "sim-00000000000000dd").unwrap().unwrap().contains("18446744073709551615"));
    }

    /// A positional ARRAY body is the LAYER's own 400 since M6.5c2 (spec §2.6):
    /// `api::parse_json` reads an object only, so this module forwards the body
    /// as it forwards every body the layer refuses, and the answer is the
    /// layer's, byte for byte — one sentence on every host.
    #[test]
    fn an_array_body_is_the_layers_own_400() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        // Positional, in `NewGame`'s FIELD ORDER — the body the M6.5c plan's
        // review measured at 201, before the layer refused it.
        let body = r#"[2,["Red","Blue"],0,null,[{"user":"red@t.test"},{"open":true}]]"#;
        let direct = dispatch_inner(Some("red@t.test"), "POST", "/api/games", "", Some("application/json"), body.as_bytes());
        init_inner(&config("off")).unwrap();
        let r = create_inner(Some("red@t.test"), body.as_bytes(), "sim-00000000000000ff", Some("application/json"));
        assert_eq!((r.status, &r.body), (direct.status, &direct.body), "the layer's own answer, byte for byte");
        assert_eq!((r.status, r.body.as_str()), (400, serde_json::json!({ "error": api::NOT_AN_OBJECT }).to_string().as_str()));
        assert!(stored(Doc::Table).is_empty() && stored(Doc::Record).is_empty(), "nothing stored");
        // Anti-vacuity: the same listing names a table the moment one IS stored.
        let r = create_inner(Some("red@t.test"), CREATE.as_bytes(), "sim-00000000000000ff", Some("application/json"));
        assert_eq!(r.status, 201, "{}", r.body);
        assert_eq!(stored(Doc::Table), ["sim-00000000000000ff"]);
    }

    /// Unreachable through the Worker, and loud if it ever is not (spec §2.2, §4).
    #[test]
    fn a_string_id_that_is_not_the_objects_name_is_a_routing_defect() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let body = CREATE.replacen('{', r#"{"id":"elsewhere","#, 1);
        let r = create_inner(Some("red@t.test"), body.as_bytes(), "sim-00000000000000ee", Some("application/json"));
        assert_eq!(r.status, 500);
        assert_eq!(r.body, r#"{"error":"the create's id does not name the table it was routed to"}"#);
        assert!(read_inner("table", "elsewhere").unwrap().is_none() && read_inner("table", "sim-00000000000000ee").unwrap().is_none());
    }

    /// **The caller first** (the orchestrator's ruling at Task 3): the layer's
    /// `dispatch` reads the caller before the body, so the NAS answers every
    /// caller-less create 401, and spec §4 makes a host-written sentence a NAMED
    /// exception, never the default. Two probes, each carrying two defects, each
    /// answered exactly as `dispatch` answers the same body — the layer's own
    /// 401, byte for byte — with nothing stored under ANY id. Neither body names
    /// this object (`"id": null` reads as absent, and the array has no id
    /// element), so a table either made would sit under an id the layer drew,
    /// which a read of the object's name cannot see.
    #[test]
    fn a_callerless_create_is_the_layers_401_before_any_host_refusal() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let null_id = CREATE.replacen('{', r#"{"id":null,"#, 1);
        let array = r#"[2,["Red","Blue"],0,null,[{"user":"red@t.test"},{"open":true}]]"#;
        for (what, body) in [("no caller AND a null id", null_id.as_str()), ("no caller AND an array body", array)] {
            let r = create_inner(None, body.as_bytes(), "sim-0000000000000a11", Some("application/json"));
            let direct = dispatch_inner(None, "POST", "/api/games", "", Some("application/json"), body.as_bytes());
            assert_eq!(r.status, 401, "{what}: the caller before the body — {}", r.body);
            assert_eq!((r.status, r.body), (direct.status, direct.body), "{what}: the layer's own 401");
            assert!(stored(Doc::Table).is_empty() && stored(Doc::Record).is_empty(), "{what}: nothing stored");
        }
        // Anti-vacuity: the same listing names a table the moment one IS stored.
        let r = create_inner(Some("red@t.test"), CREATE.as_bytes(), "sim-0000000000000a11", Some("application/json"));
        assert_eq!(r.status, 201, "{}", r.body);
        assert_eq!(stored(Doc::Table), ["sim-0000000000000a11"]);
    }

    /// The handler's ORDER, pinned by probes that each carry TWO defects — a
    /// single-defect probe pins nothing about order (the lesson three M6.2 tasks
    /// relearned).
    #[test]
    fn events_refuses_in_axums_handler_order() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        assert_eq!(create_inner(Some("red@t.test"), CREATE.as_bytes(), "t", Some("application/json")).status, 201);
        let e = events_inner("t", None, "bogus=1");
        assert_eq!(e.status, 401, "no caller AND an unknown key: the caller first — {}", e.body);
        let e = events_inner("t", Some("red@t.test"), "bogus=1&wait=26");
        assert!(e.status == 400 && e.body.contains("bogus"), "an unknown key AND wait=26: the parse first — {}", e.body);
        let e = events_inner("nope", Some("red@t.test"), "wait=26");
        assert_eq!(e.status, 400, "wait=26 AND an unknown id: the cap before the read — {}", e.body);
        assert_eq!(e.body, r#"{"error":"a request can be held for at most 25 seconds, not 26"}"#);
        // R-C8: wait=26 AND a caller with no seat at a table being dealt, which
        // nobody watches — the cap before the admission, whose 403 the same
        // caller meets without the `wait`.
        let dealt = CREATE.replace(r#"{"open":true}"#, r#"{"bot":true}"#);
        assert_eq!(create_inner(Some("red@t.test"), dealt.as_bytes(), "dealt", Some("application/json")).status, 201);
        let e = events_inner("dealt", Some("blue@t.test"), "");
        assert_eq!(e.status, 403, "premise: a caller with no seat is refused at a deal — {}", e.body);
        let e = events_inner("dealt", Some("blue@t.test"), "wait=26");
        assert_eq!(
            (e.status, e.body.as_str()),
            (400, r#"{"error":"a request can be held for at most 25 seconds, not 26"}"#),
            "wait=26 AND no seat: the cap before the admission",
        );
        let e = events_inner("t", Some("red@t.test"), "wait=25");
        assert_eq!(e.status, 200, "25 is allowed, and answered at once: {}", e.body);
        assert_eq!(e.body, r#"{"events":0,"to_move":null,"status":"open","your_turn":false,"chat":0}"#);
    }

    /// The error body this host writes itself (`host_refusal`, `events`) is
    /// `dispatch`'s own for the same refusal — every arm of it: the plain one,
    /// through the events route's unknown-id 404; `status`, through a
    /// constructed `ApiError` compared with the layer's dealing 409; and
    /// `foreign_bot` (M6.5c2), through the events route's own refusal of a
    /// game not over against a bot this build does not run. Since M6.5c2 both
    /// are the layer's `api::error_body`, so this holds the host's ROUTES to
    /// it: the same refusal, reached the same way.
    #[test]
    fn a_refusal_body_is_the_one_dispatch_writes() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let e = events_inner("nope", Some("red@t.test"), "");
        let d = dispatch_inner(Some("red@t.test"), "GET", "/api/games/nope/table", "", None, b"");
        assert_eq!((e.status, e.body.as_str()), (d.status, d.body.as_str()), "the unknown id's 404");
        let body = r#"{"players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,"seats":[{"user":"red@t.test"},{"user":"blue@t.test"}]}"#;
        assert_eq!(create_inner(Some("red@t.test"), body.as_bytes(), "dealt", Some("application/json")).status, 201);
        let d = dispatch_inner(Some("red@t.test"), "GET", "/api/games/dealt", "", None, b"");
        assert_eq!(d.status, 409, "premise: a dealing table's session route is the 409 that carries `status`");
        let message = serde_json::from_str::<serde_json::Value>(&d.body).unwrap()["error"].as_str().unwrap().to_string();
        let built = error_body(&ApiError { status: http::StatusCode::CONFLICT, message, table_status: Some("dealing"), foreign_bot: None });
        assert_eq!(built, d.body);

        // M6.5c2: the foreign-bot 409, whose pair rides beside the sentence —
        // the events route's body is this host's `Err` arm, dispatch's the
        // layer's.
        let (record, table) = foreign_unfinished("foreign");
        load_inner("record", "foreign", &record).unwrap();
        load_inner("table", "foreign", &table).unwrap();
        let d = dispatch_inner(Some("local@browser"), "GET", "/api/games/foreign", "", None, b"");
        assert_eq!(d.status, 409, "premise: a game not over against a foreign bot — {}", d.body);
        assert!(d.body.contains(r#""foreign_bot":{"played":"V2","#), "premise: the pair rides the body — {}", d.body);
        let e = events_inner("foreign", Some("local@browser"), "");
        assert_eq!((e.status, e.body.as_str()), (d.status, d.body.as_str()), "this host's body, whole");
    }

    /// The committed browser-played pair as a game NOT over against a bot this
    /// build does not run: seat 1 renamed `v2-padded`, cut at 300 events with
    /// the bot to move, re-stamped, and the table's mirror moved with it. After
    /// the layer's `games::test_support::foreign_pair`, less the mirror's `bot`
    /// (ruling R-06: a test-only helper of another crate cannot be imported).
    fn foreign_unfinished(id: &str) -> (String, String) {
        use yokohama_core::{state_at, GameRecord};
        let pair = |file: &str| {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../yokohama-web/tests/fixtures").join(file);
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        };
        let mut record: GameRecord = serde_json::from_str(&pair("browser-played-2p.json")).unwrap();
        let mut table: yokohama_web::table::TableDoc = serde_json::from_str(&pair("browser-played-2p.table.json")).unwrap();
        record.run.as_mut().unwrap().seats[1].controller = "v2-padded".to_string();
        record.events.truncate(300);
        let state = state_at(&record, 300, &yokohama_core::Rules::standard()).unwrap();
        record.stamp_terminal_digest(&state);
        table.id = id.to_string();
        table.turn = Some(yokohama_web::table::TurnMirror {
            to_move: state.current_player.0,
            over: false,
            winner: None,
            bot: None,
        });
        (serde_json::to_string(&record).unwrap(), serde_json::to_string(&table).unwrap())
    }

    /// `events`' 401 is `dispatch`'s own for the same caller, byte for byte —
    /// absent, `anonymous`, and a value no header can carry (`User::parse`'s
    /// sentence) — so the transport route refuses a caller in the words every
    /// other route uses. The order cell above asserts only the status.
    #[test]
    fn an_events_401_is_the_one_dispatch_writes_for_the_same_caller() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        for user in [None, Some("anonymous"), Some("red@wire.test\u{7f}")] {
            let e = events_inner("t", user, "");
            let d = dispatch_inner(user, "GET", "/api/games/t/table", "", None, b"");
            assert_eq!(e.status, 401, "{user:?}: {}", e.body);
            assert_eq!((e.status, e.body), (d.status, d.body), "{user:?}");
        }
    }

    /// **`events` drains exactly as `dispatch` does** (plan re-review m-c): it
    /// reaches `Games::handle` too, and a cold handle rebuilt with a bot to move
    /// pumps that move and persists it (the layer's `lib.rs:523-539`). So the
    /// pump's bump is in THIS reply — the one the shell commits — and none is
    /// left in the buffer for the next call's, which would carry it under a
    /// different request.
    #[test]
    fn events_drains_a_cold_handles_owed_pump_into_its_own_reply() {
        let _s = serial();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../worker/test/fixtures");
        let table = std::fs::read_to_string(dir.join("owed-pump.table.json")).expect("the committed fixture");
        let owed = std::fs::read_to_string(dir.join("owed-pump.json")).expect("the committed fixture");
        init_inner(&config("off")).unwrap();
        load_inner("table", "owed", &table).unwrap();
        load_inner("record", "owed", &owed).unwrap();
        let e = events_inner("owed", Some("red@t.test"), "");
        assert_eq!((e.status, tagged(&e).as_str()), (200, r#"[{"bump":"owed"}]"#), "{}", e.body);
        assert_ne!(read_inner("record", "owed").unwrap().unwrap(), owed, "the pump was persisted");
        let reading: serde_json::Value = serde_json::from_str(&e.body).unwrap();
        assert_eq!((&reading["to_move"], &reading["your_turn"]), (&serde_json::json!(0), &serde_json::json!(true)), "read AFTER the pump: {reading}");
        let next = call("red@t.test", "GET", "/api/games/owed/table", None);
        assert_eq!((next.status, tagged(&next).as_str()), (200, "[]"), "nothing left behind for the next call");
    }

    /// **The crate's own scan** (spec §5.1): this is host code, and its ONLY
    /// platform call is the clock. Every `src/*.rs` of this crate is read by
    /// the layer's own `platform::calls_in` — the hosts' scan over the eight
    /// patterns (`platform::CALLS`), which stops at a file's first column-0
    /// `#[cfg(test)]` (M6.5c2, spec §2.6; R1's cleanup debt 4).
    ///
    /// Three anti-vacuity partners, and they are this host's own: the layer's
    /// scan must find a call where there is one; this one must have looked at
    /// a file; and the one allowed call must be in CODE rather than in prose —
    /// a doc comment naming `js_sys::Date::now` would otherwise keep this cell
    /// green over a crate that had stopped making the call at all.
    #[test]
    fn this_host_makes_no_platform_call_but_the_clock() {
        assert!(
            !yokohama_web::platform::calls_in("probe.rs", "use std::fs;\n").is_empty(),
            "anti-vacuity: the layer's scan finds a call where there is one",
        );
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
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
            scanned += 1;
            escaped.extend(yokohama_web::platform::calls_in(&name, &text));
            // The partner reads the lines `calls_in` reads: those above the
            // file's first column-0 `#[cfg(test)]`, less the comments.
            let lines: Vec<&str> = text.lines().collect();
            let end = lines.iter().position(|l| *l == "#[cfg(test)]").unwrap_or(lines.len());
            clock_in_code |= lines[..end]
                .iter()
                .any(|line| !line.trim_start().starts_with("//") && line.contains("js_sys::Date::now"));
        }
        assert!(escaped.is_empty(), "platform calls in the host crate: {escaped:#?}");
        assert!(scanned > 0, "anti-vacuity: the scan read no file");
        assert!(clock_in_code, "anti-vacuity: the one allowed platform call is CODE, not prose");
    }
}
