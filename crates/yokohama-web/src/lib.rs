//! The HTTP surface over the engine — the browser client's whole view of the game.
//!
//! **What this crate is not.** It is not a second frontend. Every model it
//! serves is built by `yokohama_cli::screen_model`, whose module doc calls
//! itself "the seam a later API or GUI layer can consume without a terminal";
//! every word it serves is `yokohama_cli::describe`'s; every move it applies
//! goes through `yokohama_cli::session::Session`, which owns the legal-list
//! gate and the invariant guard. This crate routes, scopes and serializes. If
//! it ever computes something about the game STATE, that is a bug: the
//! duplicate would drift from the terminal, and the two would disagree about a
//! rule.
//!
//! **The one exception, named rather than quietly taken (M3):** `/suggest` and
//! `/why` run a real search. They read an OPINION about a state and never
//! advance one — nothing is applied, recorded or autosaved — and both call the
//! CLI's own entry points, so neither is a second implementation of anything.
//! They are also the only two handlers that DROP the per-game lock before doing
//! their work, because holding it across a multi-second search would block
//! every other request for that game.
//!
//! **The three properties it must not lose**, each of which the engine already
//! guarantees and this layer can only throw away:
//!
//! 1. **The engine is the sole legality authority.** Nothing here constructs a
//!    move the engine did not offer; a client sends back a number from a list
//!    the engine produced.
//! 2. **A seat sees its own view.** The decision zone is scoped by
//!    `screen_model::build_decision_for`, and a hand is masked in the MODEL
//!    (`HandPanel::cards` is empty when `hidden`), so a handler that forgets to
//!    read a flag still cannot leak the cards.
//! 3. **Seeds are drawn over the full `u64`.** `yokohama_core`'s `agent.rs`
//!    records that the public board is an injective fingerprint of the setup
//!    seed, so a sequential or timestamp seed would hand over the hidden set.
//!
//! **The `server` feature and its source convention (M6.5a, R-6.5a.5–7).** The
//! table layer — `Games` and everything it reaches — builds for
//! `wasm32-unknown-unknown` with `--no-default-features`; axum, tokio and every
//! platform call live behind the `server` feature. Two fences hold that, and
//! neither is the other's fallback: the CI check line proves the DEPENDENCY
//! fence (no axum, no mio), and `platform_calls_stay_behind_the_server_feature`
//! below proves the PLATFORM-CALL fence by reading this crate's own source,
//! because a `SystemTime::now()` compiles for the target and only panics there.
//! The cell ENFORCES a convention rather than trusting the compiler: a
//! `server`-only item carries its own `#[cfg(feature = "server")]` (an attribute
//! gates ONE item), a run of them is marked as a region that ends with a comment
//! line `// ---- end server ----` at column 0 — the marker is a reading aid; an
//! item inside a region WITHOUT its own attribute is named by the cell — and a
//! whole-`server` file begins with `#![cfg(feature = "server")]`. A platform call
//! on an ungated item is named by file and line, wherever in the file it sits
//! (the top-level test module is the one span the cell does not read).

#[cfg(feature = "server")]
pub mod art;
#[cfg(feature = "server")]
pub mod assets;
// ---- end server ----
pub mod api;
pub mod archive;
/// The backup envelope and the validator the archive route shares with it
/// (M6.5b2, R-6.5b2.10 – .12). NOT behind `server`: the browser host calls it.
pub mod backup;
pub mod chat;
pub mod client;
pub mod explain;
pub mod games;
pub mod grammar;
pub mod identity;
pub mod outcome;
pub mod signal;
pub mod store;
pub mod table;
pub mod tables;

/// The payload and request types live in the layer (M6.5a, R-6.5a.3); their
/// paths are kept here for the cells, the corpus and the client contract that
/// name them.
pub use api::{
    ArchivePost, ArchiveReply, ChatPage, ChatPost, ChatQuery, Events, EventsQuery, GamesList,
    Health, IndexQuery, Input, Join, Keep, Labels, ListQuery, LogQuery, Me, RunFrames, RunQuery,
    Unlock, WhyQuery, RUN_CAP,
};
pub use archive::{PutError, PutOutcome};

use http::StatusCode;

#[cfg(feature = "server")]
use std::sync::Arc;

#[cfg(feature = "server")]
use axum::extract::rejection::QueryRejection;
#[cfg(feature = "server")]
use axum::extract::{FromRequestParts, Path, Query, State};
#[cfg(feature = "server")]
use axum::http::request::Parts;
#[cfg(feature = "server")]
use axum::response::{IntoResponse, Response};
#[cfg(feature = "server")]
use axum::{
    routing::get,
    Json, Router,
};
#[cfg(feature = "server")]
use yokohama_core::Rules;

#[cfg(feature = "server")]
use crate::games::Games;
#[cfg(feature = "server")]
use crate::store::{FsStore, MemStore, Store};
// ---- end server ----

/// A refused request, as the client sees it.
///
/// One shape for every failure — `{"error": "..."}` with a status — because a
/// client that has to guess which of several error shapes it received will get
/// it wrong on the path it exercises least. The messages come from the engine
/// and the session layer, which already refuse in sentences.
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
    /// The ONE refusal a client branches on rather than displays (M6.1): a
    /// table that is still dealing. It rides beside the sentence as `"status"`,
    /// so a client that opened a game by id knows to draw the setup screen
    /// instead of an error banner.
    ///
    /// **`&'static str`, and absent on every other refusal.** A field that a
    /// handler could fill with anything would be a second, untyped error
    /// channel; the only value that reaches it is [`games::GameError::Dealing`]'s.
    pub table_status: Option<&'static str>,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, message: message.into(), table_status: None }
    }
    pub fn not_found(message: impl Into<String>) -> Self {
        Self { status: StatusCode::NOT_FOUND, message: message.into(), table_status: None }
    }
    /// This process failed at something the caller asked for correctly.
    ///
    /// Rare on purpose: nearly every refusal on this API is the caller's, and a
    /// 500 tells a client not to change its request. It carries a sentence like
    /// every other refusal, because the shape is the same one `web/src/api.ts`
    /// reads `.error` off.
    pub fn server_error(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
            table_status: None,
        }
    }
    /// A session route asked of a table that has not finished dealing.
    pub fn dealing() -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: games::DEALING_SENTENCE.to_string(),
            table_status: Some("dealing"),
        }
    }
    /// A session route asked of a table that is still filling its seats (M6.2).
    pub fn open() -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: games::OPEN_SENTENCE.to_string(),
            table_status: Some("open"),
        }
    }
    /// A table-phase route asked of a table whose game has started (M6.2).
    pub fn playing() -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: games::PLAYING_SENTENCE.to_string(),
            table_status: Some("playing"),
        }
    }
}

/// Every refusal `Games` raises, in the status a client reads it as.
///
/// **One mapping, in one place.** Twelve handlers reach `Games::handle`, and
/// before this each mapped its own error with `ApiError::not_found` — so a
/// dealing table answered 404 on every one of them and the client had nothing
/// to branch on. A `From` impl means a handler writes `?` and cannot get it
/// wrong.
impl From<games::GameError> for ApiError {
    fn from(e: games::GameError) -> Self {
        use games::GameError as E;
        match e {
            E::NotFound(m) => Self::not_found(m),
            E::Dealing => Self::dealing(),
            E::Open => Self::open(),
            E::Playing => Self::playing(),
            E::Forbidden(m) => {
                Self { status: StatusCode::FORBIDDEN, message: m, table_status: None }
            }
            E::Conflict(m) => Self { status: StatusCode::CONFLICT, message: m, table_status: None },
            E::BadRequest(m) => Self::bad_request(m),
            E::Server(m) => Self::server_error(m),
        }
    }
}

#[cfg(feature = "server")]
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = match self.table_status {
            Some(status) => serde_json::json!({ "error": self.message, "status": status }),
            None => serde_json::json!({ "error": self.message }),
        };
        (self.status, Json(body)).into_response()
    }
}

#[cfg(feature = "server")]
type ApiResult<T> = Result<Json<T>, ApiError>;

/// This API's QUERY-STRING extractor, and the reason it is not `axum::Query` —
/// since M6.5a the ONLY axum extractor left here, because request bodies reach
/// the layer as bytes through the adapter and `api::parse_json`, the layer's own
/// fence, which reproduces axum's sentences. It stays for the one route a host
/// writes itself, `events?after=&wait=`.
///
/// Two defects, both measured, both closed here:
///
/// 1. **A rejection did not carry a sentence.** axum's own rejection is a
///    PLAIN-TEXT body, and `web/src/api.ts` builds its `ApiError` by reading
///    `.error` off the body — so a malformed request reached the player as a
///    bare status code with nothing to show. Every refusal on this API is one
///    shape, and that has to include the ones raised before a handler runs.
/// 2. **A shape error was a 422.** Honest for a generic extractor, but this API
///    has one meaning for "your request was wrong" and it is 400.
///
/// The wrapped type carries `#[serde(deny_unknown_fields)]`, which is the half
/// that matters: `?tale=5` for `?tail=5` is otherwise a silent full log — the
/// key is dropped, the `Option` defaults to `None`, and the caller is answered
/// confidently with something it did not ask for.
#[cfg(feature = "server")]
pub struct QueryArgs<T>(pub T);

#[cfg(feature = "server")]
impl<T, S> FromRequestParts<S> for QueryArgs<T>
where
    Query<T>: FromRequestParts<S, Rejection = QueryRejection>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Query::<T>::from_request_parts(parts, state)
            .await
            .map(|Query(value)| Self(value))
            .map_err(|rejection| ApiError::bad_request(rejection.body_text()))
    }
}

/// **Which change transport this build serves, by name** (M6.3, R-plan-4).
///
/// A compile-time constant and deliberately NOT an environment variable: the
/// choice between server-sent events and the held request is a code decision
/// recorded in the cycle's close, taken on a measurement through the real
/// tunnel (R-3.1). A deployment that could flip it would be a deployment that
/// could put the client on a transport nobody measured.
///
/// It reaches the client on `/api/health` beside the identity mode, for the
/// same reason that field is there: it is a deployment fact a client cannot
/// infer from a successful call.
#[cfg(feature = "server")]
pub const SIGNAL_KIND: &str = "sse";

/// The SSE comment cadence in production (R-3.12).
///
/// Under the edge's 100 s idle timeout and the tunnel's 90 s keep-alive, which
/// is the whole of the choice: a stream that says nothing for longer than
/// either is a stream the infrastructure closes while both ends believe it is
/// open.
#[cfg(feature = "server")]
pub const HEARTBEAT: std::time::Duration = std::time::Duration::from_secs(25);
// ---- end server ----

/// **Whether this deployment offers multiplayer** (M6.5b2, R-6.5b2.4).
///
/// The third deployment switch, beside the identity door and the archive
/// directory, and the one that decides whether a route EXISTS rather than how
/// it behaves: off, the game, chat and stream routes are not served, so there
/// is nothing to probe and no Access application to forget about.
///
/// **It has a default and the identity mode does not**, which is
/// `ChatConfig`'s rule and for its reason: a missing switch is not a missing
/// door. Absent is `on`, so every deployment that ran before this cycle goes
/// on running unchanged. A BAD value still refuses to start, naming the
/// variable — a server that read `YOKOHAMA_WEB_MULTIPLAYER=false` as `on`
/// would be a server whose operator believes a switch that is not in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Multiplayer(pub bool);

impl Multiplayer {
    /// The variable's name, held here rather than written at each of its three
    /// sites — the read, the refusal sentence and the cell that asserts the
    /// refusal names it. A refusal that named a variable the reader's
    /// environment does not have is worse than no refusal at all, and that is
    /// exactly what a second spelling of this string would produce.
    pub const VAR: &'static str = "YOKOHAMA_WEB_MULTIPLAYER";

    /// The parse, split from the read so the unset case is testable without
    /// touching the process environment — `IdentityMode::from_env_value`'s
    /// shape, and `ChatConfig::from_env_values`' reading of `on`/`off`:
    /// trimmed and case-insensitive, because the three variables are one
    /// operator's `docker run` line and a rule that held for two of them would
    /// be a rule nobody could remember.
    pub fn from_env_value(v: Option<&str>) -> Result<Self, String> {
        match v.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
            None | Some("on") => Ok(Self(true)),
            Some("off") => Ok(Self(false)),
            Some(other) => Err(format!("{} must be on or off — got {other:?}", Self::VAR)),
        }
    }
}

#[cfg(feature = "server")]
impl Multiplayer {
    /// The read, `server`-only for the reason `ChatConfig::from_env` is: a host
    /// with no process environment (the browser, a Worker) states its own
    /// deployment facts and never asks one.
    pub fn from_env() -> Result<Self, String> {
        Self::from_env_value(std::env::var(Self::VAR).ok().as_deref())
    }
}
// ---- end server ----

/// Everything a transport handler may reach: the layer's state, the identity
/// door, the streams' cadence and the streams' channels.
///
/// `Rules::standard()` is not cheap and is immutable once built, so it is
/// constructed once per process and shared by reference — the same discipline
/// the CLI's `main` uses — and since M6.5a it is held by [`Games`] alone
/// (`Games::rules()`), which is where every reader of it now is.
#[cfg(feature = "server")]
pub struct AppState {
    pub games: Games,
    /// Which header names the caller (R-1.2). Read once at start; no default.
    pub identity: identity::IdentityMode,
    /// How often an idle stream writes a comment line (R-3.12).
    ///
    /// **A field, and the reason is testability rather than configuration.**
    /// Nothing on a built `Sse` is inspectable — the cadence disappears into
    /// axum's `KeepAlive` — so the only way to assert that a heartbeat reaches
    /// the wire at all is to build a state whose cadence is 50 ms and read two
    /// of them off a body. Every production constructor sets [`HEARTBEAT`].
    pub heartbeat: std::time::Duration,
    /// The transport's half of the change signal (M6.5a, R-6.5a.8).
    ///
    /// [`Games`] holds the same object as an `Arc<dyn signal::Signal>` and can
    /// only BUMP it; the streams live here and are the only things that read a
    /// bump back, so the concrete [`signal::Signals`] — the one type that can
    /// hand out a receiver — is held here rather than fetched back out of the
    /// layer below.
    pub signals: Arc<signal::Signals>,
    /// Whether this deployment offers multiplayer (M6.5b2, R-6.5b2.4).
    ///
    /// Read by [`app`], which leaves the three transport routes off the router
    /// when it is off, and by the adapter, which hands it to the layer as
    /// [`api::Host::serves_games`] and [`api::Host::multiplayer_offered`] —
    /// which on THIS host are the same answer, because an axum deployment that
    /// offers no multiplayer plays nothing at all.
    pub multiplayer: Multiplayer,
    /// Where the built client lives (M6.5b2), read from `assets::dist_dir()`
    /// ONCE at construction rather than on every request that misses a route.
    ///
    /// A seam for the same reason the archive and the clock are seams: a cell
    /// can arrange a `dist` the ROUTER really answers from, which is what makes
    /// "the fallback refuses `/enter` rather than serving the shell" a claim a
    /// mutation can break. The alternative was writing `YOKOHAMA_WEB_DIST` from
    /// a test, and `identity::from_env_value`'s own doc is this crate's rule
    /// against mutating the process environment under a threaded runner.
    pub dist: std::path::PathBuf,
}

#[cfg(feature = "server")]
impl AppState {
    /// Backed by a directory on disk.
    /// The production state: the file store, and the archive the environment
    /// names (M6.5a, R-6.5a.9) — `YOKOHAMA_WEB_ARCHIVE_DIR` unset is no archive;
    /// set to a directory that cannot be created is an `Err` `main` exits 1
    /// on, naming the variable, the shape `ChatConfig::from_env` already has.
    pub fn new(
        identity: identity::IdentityMode,
        chat: chat::ChatConfig,
        multiplayer: Multiplayer,
    ) -> Result<Self, String> {
        let archive: Arc<dyn archive::Archive> = match archive::DirArchive::from_env()? {
            Some(dir) => Arc::new(dir),
            None => Arc::new(archive::NoArchive),
        };
        Ok(Self::with_store_chat_and_archive(
            Arc::new(FsStore::new(store::default_dir())),
            identity,
            chat,
            archive,
        )
        .with_multiplayer(multiplayer))
    }

    /// Backed by memory, in `claimed` mode — the mode every test drives with a
    /// header. Tests, and any run that must leave no trace.
    pub fn in_memory() -> Self {
        Self::in_memory_with(identity::IdentityMode::Claimed)
    }

    pub fn in_memory_with(identity: identity::IdentityMode) -> Self {
        Self::with_store(Arc::new(MemStore::default()), identity)
    }

    /// In memory, with a heartbeat a cell can outlive — see [`AppState::heartbeat`].
    pub fn in_memory_with_heartbeat(heartbeat: std::time::Duration) -> Self {
        Self { heartbeat, ..Self::in_memory() }
    }

    /// In memory, with the chat configured rather than defaulted — how a cell
    /// asks for the lobby off, or for a keep small enough to watch a trim.
    pub fn in_memory_with_chat(chat: chat::ChatConfig) -> Self {
        Self::with_store_and_chat(
            Arc::new(MemStore::default()),
            identity::IdentityMode::Claimed,
            chat,
        )
    }

    pub fn with_store(store: Arc<dyn Store>, identity: identity::IdentityMode) -> Self {
        Self::with_store_and_chat(store, identity, chat::ChatConfig::default())
    }

    /// Every constructor above is this one with a store, an identity or a chat
    /// chosen for it, and this one is [`Self::with_store_chat_and_archive`] with
    /// no archive — since M6.5a THAT is the one constructor that builds anything.
    ///
    /// The chat facts live on [`Games`] and nowhere here (M6.5a, R-6.5a.1): the
    /// appender enforces them and `/api/health` reports them off the same
    /// object, so a host passes them in once and nothing on this state can drift
    /// from what the layer enforces.
    pub fn with_store_and_chat(
        store: Arc<dyn Store>,
        identity: identity::IdentityMode,
        chat: chat::ChatConfig,
    ) -> Self {
        Self::with_store_chat_and_archive(store, identity, chat, Arc::new(archive::NoArchive))
    }

    /// The one constructor that builds anything — the store, the identity door,
    /// the chat facts and the archive all given (M6.5a). Every constructor above
    /// is this one with something chosen for it; `in_memory*` keep no archive.
    pub fn with_store_chat_and_archive(
        store: Arc<dyn Store>,
        identity: identity::IdentityMode,
        chat: chat::ChatConfig,
        archive: Arc<dyn archive::Archive>,
    ) -> Self {
        let rules = Arc::new(Rules::standard());
        let signals = Arc::new(signal::Signals::default());
        Self {
            games: Games::with_parts(
                rules,
                store,
                chat,
                signals.clone(),
                Arc::new(table::now_unix),
                archive,
            ),
            identity,
            heartbeat: HEARTBEAT,
            signals,
            // On, which is [`Multiplayer::from_env_value`]'s own answer to an
            // unset variable: every constructor above this one builds the
            // deployment that existed before M6.5b2, and only `new` and
            // [`Self::with_multiplayer`] can say otherwise.
            multiplayer: Multiplayer(true),
            dist: assets::dist_dir(),
        }
    }

    /// This state serving the client from `dist` (M6.5b2) — [`Self::with_clock`]'s
    /// shape again, for the cells that need a real shell behind the fallback.
    pub fn with_dist(self, dist: impl Into<std::path::PathBuf>) -> Self {
        Self { dist: dist.into(), ..self }
    }

    /// This state with the multiplayer switch thrown (M6.5b2, R-6.5b2.4).
    ///
    /// A builder step over the field, [`Self::with_clock`]'s shape, for the two
    /// callers that need it: `main`, which read the variable, and the cells that
    /// assert what a single-player deployment serves.
    pub fn with_multiplayer(self, multiplayer: Multiplayer) -> Self {
        Self { multiplayer, ..self }
    }

    /// This state with a different [`games::Clock`] (M6.5b, spec §6.3).
    ///
    /// A builder step over [`Games::with_clock`], for the one caller that needs
    /// the whole state rather than the layer: the wire corpus, whose listings
    /// sort `(created_at desc, id asc)` and whose two creates therefore have to
    /// land in the SAME second or the committed page swaps its rows. Under a
    /// wall clock that is true roughly 999 runs in 1,000, which is the worst
    /// kind of true for a golden — and the browser host replays the same walk
    /// against a frozen reading of its own, so the two hosts now agree by
    /// construction rather than by coincidence.
    pub fn with_clock(self, clock: games::Clock) -> Self {
        Self { games: self.games.with_clock(clock), ..self }
    }
}

// ---------------------------------------------------------------------------
// The read-only surfaces
// ---------------------------------------------------------------------------
//
// Four `GET`s over four `screen_model` builders that were already `pub`, tested
// and reachable from nowhere. They are the home for the words `grammar` fences
// off the mutation endpoint: `ports`, `labs`, `areas` and `log` used to reach a
// browser only as terminal art through `Repl::step`, which also opened a sticky
// overlay that changed what the next number meant. As routes they are what they
// always were — a projection of state, computing nothing, changing nothing.
//
// Each takes the game's lock briefly and drops it, and none of them applies
// anything itself.
//
// **They are not, however, side-effect free, and an earlier version of this
// comment claimed they were.** All four begin with `Games::handle`, which on a
// cache miss rebuilds from the record and then runs `bring_alive` — the pump a
// game with a bot to move is OWED — and persists if that pump played. So a
// `GET` can advance a bot's turn and write the store.
//
// That is kept rather than fixed, and the reasoning is worth stating because
// the obvious fix is wrong: a read-only rebuild that skipped the pump would
// cache an un-pumped handle, and the next `input` would apply a human move to a
// state the bot had not yet moved in. The pump is a correctness requirement of
// the rebuild, not a side effect of the route.
//
// What bounds the harm: the pump is deterministic and owed — it plays the move
// that was going to be played, from a seed already in the record — so nothing
// an attacker supplies changes what happens, and the same GET twice is
// idempotent. It is recorded here so the next reader does not trust a sentence
// over the code.

/// The longest a `?wait` may hold (spec §3.3).
///
/// Above it is a **400 and not a clamp**: a caller that asked for 120 and was
/// silently given 25 would read the early answer as "nothing changed for two
/// minutes", which is the same failure `deny_unknown_fields` exists to prevent
/// one field over.
#[cfg(feature = "server")]
const MAX_WAIT_SECONDS: u64 = 25;

/// `GET /api/games/{id}/events` — the change signal, polled form.
///
/// **NOT seated, since M6.3 — and the seat it resolves is resolved for one
/// reason only: `your_turn`.** The admission is [`events_at`]'s, which is
/// `table_for_anyone` and then [`admitted`]: any signed-in caller is answered
/// at an OPEN table (ruling R-4.3, the one row where the two part company),
/// and at a playing or finished one a caller who holds no seat is answered as
/// a `Spectator` once the table admits them. `Games::table` is not on this
/// path at all, so every refusal here comes from one of those two — and
/// `…/stream` refuses identically because it calls the same function.
///
/// **The OPEN and DEALING arms answer without a session, and must.**
/// `Games::handle` refuses both with a 409 (neither has a record yet), so a
/// handler that reached for the session first would hand the open-table screen
/// and the setup screen an error instead of the reading each is polling for.
/// `Games::table` has already settled a table whose keeps are all in, so the
/// flip to `playing` is seen here on the first poll after the last keep rather
/// than one poll later.
///
/// **The reading itself is [`events_at`]'s, which `…/stream` also calls** — the
/// two transports answer the same object because they are the same function,
/// not because two bodies were kept in step. The lock is taken only to read
/// three fields, and nothing inside it searches or builds.
///
/// **`?wait` holds the request open** (R-plan-5): it is admitted first, then
/// subscribes, then answers on the first bump or at the deadline. Both arms
/// answer the same way — a held request that times out is not an error, it is
/// "nothing has changed".
#[cfg(feature = "server")]
async fn events(
    State(state): State<Arc<AppState>>,
    identity::Identity(user): identity::Identity,
    Path(id): Path<String>,
    QueryArgs(q): QueryArgs<EventsQuery>,
) -> ApiResult<Events> {
    if let Some(wait) = q.wait {
        if wait > MAX_WAIT_SECONDS {
            return Err(ApiError::bad_request(format!(
                "a request can be held for at most {MAX_WAIT_SECONDS} seconds, not {wait}"
            )));
        }
        // **Admitted BEFORE it subscribes**, for the reason `stream` states:
        // `Signals::subscribe` makes a sender for any id at all, so holding a
        // request for a table the caller may not read — or that does not
        // exist — would be a refusal delayed by 25 seconds and an entry in a
        // map that nothing prunes.
        let _admitted = api::events_at(&state.games, &id, &user)?;
        let mut rx = state.signals.subscribe(&id);
        // Either arm is an answer: the bump, the deadline, or a closed channel
        // (the table was deleted, which the read below reports as a 404).
        let _ = tokio::time::timeout(std::time::Duration::from_secs(wait), rx.changed()).await;
    }
    Ok(Json(api::events_at(&state.games, &id, &user)?))
}

/// `GET /api/games/{id}/stream` — the change signal, PUSHED (spec §3.1).
///
/// **The same admission and the same object as `…/events`**, which is the whole
/// design: [`events_at`] answers both, so a client that moves from one to the
/// other moves nothing else. The first thing on the wire is the current
/// `Events` — `WatchStream::new` yields the channel's present value at once —
/// so a reconnecting client needs no separate "what did I miss" request, and
/// R-3.2's "zero requests to `/events`" is reachable at all.
///
/// **It subscribes only after the admission has answered**, and the order is
/// load-bearing rather than tidy: `Signals::subscribe` creates a sender for ANY
/// id, so subscribing first would hold a connection open forever on a deleted
/// or fabricated table and leave one map entry per attempt behind.
///
/// **No `id:` line and no `retry:` line** (R-3.14). The resume cursor is the
/// `events` count every frame already carries; a browser's own `Last-Event-ID`
/// replay would be a second, silent cursor this server does not honour, and
/// writing one would invite a client to rely on it.
///
/// **No await holds a game lock.** The per-frame closure is synchronous: it
/// locks, builds the object, and drops the guard before the frame is yielded.
///
/// On the table's `end` the stream writes one `end` frame and CLOSES, and the
/// close is the CHANNEL's rather than this closure's: `Signals::end` drops the
/// sender after sending, and a `WatchStream` terminates once its sender is
/// gone.
///
/// **The `Err` arm is not that, and it SPLITS** (final review M-5). An
/// admission that changed underneath a live connection — a 403 or a 404 — is
/// an answer about this table that will read the same way next time, so it
/// writes the `gone` frame and the client leaves; the reachable case is the
/// stranger holding an open table's stream while a join fills the last seat.
/// Anything else is a server that failed, not a table that went: a store that
/// stopped answering writes a COMMENT — indistinguishable to the client's
/// parser from a heartbeat — and the connection stays open, because telling
/// every watcher a game is "no longer available" over a transient disk error
/// is a worse answer than saying nothing. Neither arm closes anything by
/// itself: what closes a connection is the client acting on `end`, or the
/// delete that is the only ordinary cause.
#[cfg(feature = "server")]
async fn stream(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use tokio_stream::StreamExt;

    api::events_at(&state.games, &id, &user)?;
    let rx = state.signals.subscribe(&id);
    let heartbeat = state.heartbeat;
    let frames = tokio_stream::wrappers::WatchStream::new(rx).map(move |bump| {
        if bump.end {
            return Ok::<_, std::convert::Infallible>(
                Event::default().event("end").data(r#"{"reason":"deleted"}"#),
            );
        }
        match api::events_at(&state.games, &id, &user) {
            Ok(e) => Ok(Event::default()
                .event("signal")
                .json_data(e)
                .expect("`Events` is five scalars and serializes")),
            // **A REFUSAL ends the stream; a failure does not** (final review
            // M-5). A 403 or a 404 is an answer about this table — the join
            // that filled the open seats without this caller, the delete that
            // raced the on-connect frame — and it is not going to be different
            // next time, so the client is told and navigates away. A store
            // that stopped answering is neither: writing `gone` for it would
            // send every watcher and every non-acting player to the lobby over
            // a transient disk error. So that arm writes a COMMENT — which the
            // client's parser drops exactly as it drops a heartbeat — and the
            // connection stays open for the next bump to retry.
            Err(e) if e.status == StatusCode::FORBIDDEN || e.status == StatusCode::NOT_FOUND => {
                Ok(Event::default().event("end").data(r#"{"reason":"gone"}"#))
            }
            Err(_) => Ok(Event::default().comment("the reading failed; the next bump retries")),
        }
    });
    Ok(Sse::new(frames).keep_alive(KeepAlive::new().interval(heartbeat)))
}

/// `GET /api/stream` — the house channel (spec §3.2).
///
/// **Two counters and never a per-table payload** (R-3.4, amended by M6.4
/// R-6.4.7). A `watch` channel coalesces, so two bumps naming two different
/// tables inside one wake would lose one; a payload naming a table would
/// therefore be a payload that is sometimes wrong. The lobby refetches all its
/// lists on any bump instead, which is table documents and no session.
///
/// A `chat` bump moves the second counter, and the lobby — a `wake` consumer —
/// refetches all three of its listings on EITHER number moving. **That cost is
/// real and is recorded rather than avoided** (spec §6.2, §11): a house message
/// is one more house wake per visible lobby. The two numbers ride one channel
/// so that a consumer reading the frame's VALUE could tell them apart; none
/// does yet.
///
/// **Nothing on connect**, which is what `from_changes` buys and `new` would
/// lose: the lobby has just fetched its lists to render at all, and a frame at
/// connect would make it fetch them again on every navigation.
///
/// Identity is required and is the whole of the admission — the house is not a
/// table and has nothing to scope.
#[cfg(feature = "server")]
async fn house_stream(
    identity::Identity(_user): identity::Identity,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use tokio_stream::StreamExt;

    let rx = state.signals.subscribe_house();
    let frames = tokio_stream::wrappers::WatchStream::from_changes(rx).map(|h| {
        Ok::<_, std::convert::Infallible>(
            Event::default()
                .event("house")
                .data(format!("{{\"seq\":{},\"chat\":{}}}", h.seq, h.chat)),
        )
    });
    Sse::new(frames).keep_alive(KeepAlive::new().interval(state.heartbeat))
}

/// The art manifest, or an honest 404.
///
/// The ONE piece of axum between a request and the layer (R-6.5a.3). Identity
/// is resolved here with today's extractor logic and handed to the layer as the
/// host's sentence; the body and query reach the layer as bytes and text. The
/// body limit is axum's own default (2 MiB), so nothing this API accepted
/// before is refused now.
#[cfg(feature = "server")]
async fn adapter(State(state): State<Arc<AppState>>, req: axum::extract::Request) -> Response {
    let (parts, body) = req.into_parts();
    let bytes = match axum::body::to_bytes(body, 2 << 20).await {
        Ok(b) => b,
        Err(e) => {
            return ApiError::bad_request(format!("the body could not be read: {e}")).into_response()
        }
    };
    let resolved = identity::resolve(state.identity, &parts.headers);
    let identity: api::Identity<'_> = match &resolved {
        Ok(u) => Ok(u),
        Err(m) => Err(m.as_str()),
    };
    let content_type = parts.headers.get(http::header::CONTENT_TYPE).and_then(|v| v.to_str().ok());
    let request = api::ApiRequest {
        method: parts.method.as_str(),
        path: parts.uri.path(),
        query: parts.uri.query().unwrap_or(""),
        content_type,
        body: &bytes,
    };
    // Art is a server route (the manifest reads the gated `art` module), listed
    // in the layer's table for the 405/404 arithmetic and answered here.
    if request.path == "/api/art" && (parts.method == http::Method::GET || parts.method == http::Method::HEAD) {
        let mut response = match art_manifest().await {
            Ok(j) => j.into_response(),
            Err(e) => e.into_response(),
        };
        // HEAD leaves with an empty body here too, as it does through `dispatch`.
        if parts.method == http::Method::HEAD {
            *response.body_mut() = axum::body::Body::empty();
        }
        return response;
    }
    // On THIS host the two multiplayer facts are one answer: an axum deployment
    // that offers no multiplayer serves no game route either, because nothing
    // plays here locally (spec §3.1 — the browser host is where they differ).
    let host = api::Host {
        identity_name: state.identity.name(),
        signal_kind: SIGNAL_KIND,
        serves_games: state.multiplayer.0,
        multiplayer_offered: state.multiplayer.0,
    };
    let out = api::dispatch(&state.games, &host, identity, &request);
    let mut response = Response::new(axum::body::Body::from(out.body.clone()));
    *response.status_mut() = StatusCode::from_u16(out.status).expect("a known status");
    if !out.body.is_empty() {
        response
            .headers_mut()
            .insert(http::header::CONTENT_TYPE, http::HeaderValue::from_static("application/json"));
    }
    // axum's 405 carried an `Allow` header naming the methods the path takes;
    // `any()` skips it, so the adapter puts it back from the layer's own table
    // (R-6.5a.3: "axum's 405 shape, reproduced").
    if out.status == 405 {
        let allow = api::allowed_methods(request.path, host.serves_games).join(",");
        response
            .headers_mut()
            .insert(http::header::ALLOW, http::HeaderValue::from_str(&allow).expect("ASCII method names"));
    }
    response
}

/// **A checkout with no cropped art is a supported state**, not a broken one:
/// the client falls back to drawing what it drew before there was any art. So
/// the absence answers as a refusal with the sentence that explains it — which
/// names the regeneration command, because the one thing a reader needs here is
/// how to make it stop being absent.
#[cfg(feature = "server")]
async fn art_manifest() -> Result<Json<art::ArtManifest>, ApiError> {
    art::load(&art::art_dir()).map(Json).map_err(|e| {
        ApiError::not_found(format!("{e} — run `python3 docs/tools/mkcardart.py` from the repo root"))
    })
}

/// Everything no route claimed.
///
/// **`/api/*` never falls through to the client.** A mistyped endpoint has to
/// answer as the API does — a JSON `{"error": ...}` with a 404 — because
/// serving `index.html` there produces the failure every SPA developer has
/// debugged at least once: the browser reports a syntax error at `<!doctype`,
/// the console blames the client, and the real fault (a wrong path) appears
/// nowhere in the message. `/art/*` is fenced for the same reason and a second
/// one: a missing picture answering with HTML is a broken image whose message
/// blames the decoder. Everything else is the client, with the SPA fallback
/// `assets::serve` describes.
#[cfg(feature = "server")]
async fn fallback(State(state): State<Arc<AppState>>, uri: axum::http::Uri) -> Response {
    if uri.path().starts_with("/api/") {
        return ApiError::not_found(format!("no such endpoint: {}", uri.path())).into_response();
    }
    // **`/enter` never falls through to the shell** (M6.5b2, spec §2.1). It is
    // extensionless, so without this arm a deployment that does not serve it
    // would answer `index.html` with a 200 — and the door's "Play with others"
    // link would load the app again instead of saying why it cannot. When the
    // turnstile IS served this arm is unreachable: a real route wins over the
    // fallback.
    if uri.path() == ENTER_PATH {
        return (StatusCode::NOT_FOUND, "this deployment plays single player only").into_response();
    }
    if uri.path().starts_with("/art/") {
        return art::serve(uri).await;
    }
    assets::serve(&state.dist, uri).await
}

/// The path the turnstile answers on, named once because TWO places read it and
/// they must not drift: [`app`], which registers it only where something is
/// protected, and [`fallback`], which must refuse it everywhere else rather
/// than let an extensionless path reach the SPA shell.
#[cfg(feature = "server")]
const ENTER_PATH: &str = "/enter";

/// **The turnstile** (M6.5b2, R-6.5b2.2): a 302 straight back to the lobby.
///
/// It computes nothing and reads nothing, and that is the whole design. It is
/// one destination of the deployment's Access application, so NAVIGATING to it
/// is what mints the token every later `fetch` to a protected path carries —
/// the app itself stays unaware that Access exists. "Play with others" is
/// therefore a plain link and never a `fetch`, so the login is an ordinary
/// navigation the browser already knows how to do.
///
/// 302 rather than 303 or 307 because a browser must repeat it as a GET and
/// must not cache it; axum's `Redirect` offers 303, 307 and 308 and none of
/// them is that, so the response is built here.
#[cfg(feature = "server")]
async fn enter() -> Response {
    (StatusCode::FOUND, [(http::header::LOCATION, "/#/lobby")]).into_response()
}

/// The router, built over a shared state so tests can drive it through
/// `tower::ServiceExt::oneshot` without binding a port.
///
/// **Two things are conditional on the deployment** (M6.5b2). The three
/// transport routes are registered only when multiplayer is on, because they
/// are the ones the layer's own table cannot remove — they never reach
/// [`api::dispatch`] — and R-6.5b2.4 is that OFF means not served. The
/// turnstile is registered on the OR of the two protected things, multiplayer
/// or an archive: R-6.5b2.4 is about the multiplayer routes not existing, and
/// the turnstile is about acquiring the cookie for whatever protected route
/// does. Multiplayer off with an archive on is a coherent deployment — play
/// single player free and anonymous, pass the door once to contribute a save —
/// and tying the door to multiplayer alone would point the outbox's own remedy
/// at a 404.
#[cfg(feature = "server")]
pub fn app(state: Arc<AppState>) -> Router {
    let mut router = Router::new();
    if state.multiplayer.0 {
        router = router
            .route("/api/games/{id}/events", get(events))
            .route("/api/games/{id}/stream", get(stream))
            .route("/api/stream", get(house_stream));
    }
    if state.multiplayer.0 || archive::is_on(&state.games.archive) {
        router = router.route(ENTER_PATH, get(enter));
    }
    router
        // Every other `/api/…` route is the layer's (R-6.5a.3): one catch-all
        // to the adapter, plus `/api/games` itself, which has no `{*rest}` tail.
        .route("/api/{*rest}", axum::routing::any(adapter))
        .route("/api/games", axum::routing::any(adapter))
        // BEFORE `with_state`, which `fallback` now needs: it reads the client
        // root off the state rather than the environment (M6.5b2).
        .fallback(fallback)
        .with_state(state)
}
// ---- end server ----

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::NOT_SEATED;
    use yokohama_core::SEMANTICS_REV;
    // The four bumps are the trait's since M6.5a (R-6.5a.8); the cells that
    // drive one by hand need it in scope.
    use crate::signal::Signal;
    use crate::store::Doc;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    /// The identity every helper below sends, so a route cell is about the
    /// route rather than about the 401 in front of it. The identity cells
    /// choose their own headers with [`get_with`].
    const WHO: &str = "red@wire.test";

    async fn get_json(uri: &str) -> (StatusCode, serde_json::Value) {
        let res = app(Arc::new(AppState::in_memory()))
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header(identity::CLAIMED_HEADER, WHO)
                    .body(Body::empty())
                    .expect("a GET"),
            )
            .await
            .expect("the router answers");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        let value = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).expect("JSON")
        };
        (status, value)
    }

    async fn post_json(uri: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
        raw_post(uri, serde_json::to_vec(&body).expect("a body")).await
    }

    /// A request under any METHOD, answered as raw bytes — for the cells about
    /// the adapter's shape (HEAD, 405, the JSON 404) and, from Task 9, the record
    /// route's exact bytes. `who` is the claimed identity, or none.
    async fn raw(app: Router, method: &str, uri: &str, who: Option<&str>) -> (StatusCode, Vec<u8>) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(who) = who {
            builder = builder.header(identity::CLAIMED_HEADER, who);
        }
        let res = app
            .oneshot(builder.body(Body::empty()).expect("a request"))
            .await
            .expect("the router answers");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        (status, bytes.to_vec())
    }

    /// [`raw`] with the `Location` header and the body as text (M6.5b2, spec
    /// §2.1). One helper and not two, because the two questions a turnstile
    /// raises are asked of the same call: where it sends a browser when it is
    /// served, and what it answered INSTEAD when it is not. No identity:
    /// `/enter` is a navigation a browser makes before it has one.
    async fn raw_location(
        app: Router,
        method: &str,
        uri: &str,
    ) -> (StatusCode, Option<String>, String) {
        let res = app
            .oneshot(Request::builder().method(method).uri(uri).body(Body::empty()).expect("a request"))
            .await
            .expect("the router answers");
        let status = res.status();
        let location =
            res.headers().get(http::header::LOCATION).and_then(|v| v.to_str().ok()).map(str::to_string);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        (status, location, String::from_utf8_lossy(&bytes).to_string())
    }

    /// The router with the multiplayer switch thrown (M6.5b2, R-6.5b2.4), over
    /// an in-memory state with no archive — which is the shape that makes the
    /// switch the ONLY reason a route is missing.
    async fn app_with_multiplayer(multiplayer: bool) -> Router {
        app(Arc::new(AppState::in_memory().with_multiplayer(Multiplayer(multiplayer))))
    }

    /// The router with the two protected things chosen INDEPENDENTLY (M6.5b2,
    /// spec §2.1) — the pair `/enter`'s OR condition is made of — serving its
    /// client from `dist`, so the SPA fallback behind `/enter` is really live.
    ///
    /// The archive is an [`archive::DirArchive`] over a path that is never
    /// created: `archive::is_on` reads its `kind()` and nothing in these cells
    /// finishes a game, so the switch is thrown without touching a disk.
    ///
    /// Nothing here writes `YOKOHAMA_WEB_DIST`: the root is PASSED, through
    /// `AppState::dist`, because a process-global written from one cell would
    /// race `dist_dir`'s and `art_dir`'s reads in the cells scheduled beside it
    /// — which is the rule `identity::from_env_value`'s own doc states.
    async fn app_with(multiplayer: bool, archive: bool, dist: &std::path::Path) -> Router {
        let archive: Arc<dyn archive::Archive> = if archive {
            Arc::new(archive::DirArchive::new(dist.join("archive")))
        } else {
            Arc::new(archive::NoArchive)
        };
        app(Arc::new(
            AppState::with_store_chat_and_archive(
                Arc::new(MemStore::default()),
                identity::IdentityMode::Claimed,
                chat::ChatConfig::default(),
                archive,
            )
            .with_multiplayer(Multiplayer(multiplayer))
            .with_dist(dist),
        ))
    }

    /// A `dist` directory holding a real `index.html` and NO `sw.js` — the two
    /// facts the turnstile cell's probes are about. The caller removes it.
    fn dist_with_index_html() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("yokohama-enter-dist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a dist directory");
        std::fs::write(root.join("index.html"), b"<!doctype html><title>Yokohama</title>")
            .expect("a shell");
        root
    }

    /// R-6.5a.4's pin: the layer's JSON rejection is axum's own `body_text()`,
    /// word for word, for a canonical bad body (an unknown key under
    /// `deny_unknown_fields`). An axum upgrade that rewords fails here, not in
    /// a client.
    #[tokio::test]
    async fn the_layers_json_rejection_is_axums_word_for_word() {
        use axum::extract::FromRequest;
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/x")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(r#"{"line":"1","nope":1}"#))
            .unwrap();
        let axum_text = match axum::Json::<Input>::from_request(req, &()).await {
            Err(rej) => rej.body_text(),
            Ok(_) => panic!("axum accepted an unknown key"),
        };
        let ours = api::parse_json::<Input>(Some("application/json"), br#"{"line":"1","nope":1}"#).unwrap_err();
        assert_eq!(ours.message, axum_text);
        // And the SYNTAX arm, which is a different rejection type with a
        // different sentence.
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/x")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(r#"{"line":"1"} trailing"#))
            .unwrap();
        let axum_text = match axum::Json::<Input>::from_request(req, &()).await {
            Err(rej) => rej.body_text(),
            Ok(_) => panic!("axum accepted trailing input"),
        };
        let ours = api::parse_json::<Input>(Some("application/json"), br#"{"line":"1"} trailing"#).unwrap_err();
        assert_eq!(ours.message, axum_text);
        assert!(ours.message.starts_with("Failed to parse the request body as JSON: "), "{}", ours.message);
        // And the MISSING content type, the third rejection type — pinned against
        // axum's own rather than a literal, so an upgrade that rewords it is
        // caught here (R-6.5a.4 asks that each of the four be pinned).
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/x")
            .body(axum::body::Body::from(r#"{"line":"1"}"#))
            .unwrap();
        let axum_text = match axum::Json::<Input>::from_request(req, &()).await {
            Err(rej) => rej.body_text(),
            Ok(_) => panic!("axum accepted a body with no content type"),
        };
        let ours = api::parse_json::<Input>(None, br#"{"line":"1"}"#).unwrap_err();
        assert_eq!(ours.message, axum_text);
    }

    /// R-6.5a.3's 405 is axum's SHAPE: an empty body AND the `Allow` header
    /// naming the path's methods — HEAD beside GET, as axum lists it — read off
    /// the layer's table by the adapter, since `any()` drops the one axum's
    /// `MethodRouter` wrote.
    #[tokio::test]
    async fn a_405_carries_axums_allow_header() {
        let app = app(Arc::new(AppState::in_memory()));
        let res = app
            .clone()
            .oneshot(Request::builder().method("DELETE").uri("/api/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(res.headers().get("allow").and_then(|v| v.to_str().ok()), Some("GET,HEAD"));
        let res = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/games/sim-1")
                    .header(identity::CLAIMED_HEADER, WHO)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(res.headers().get("allow").and_then(|v| v.to_str().ok()), Some("GET,HEAD,DELETE"));
        assert_eq!(api::allowed_methods("/api/nothing", true), Vec::<&str>::new(), "no path, no methods");
    }

    #[tokio::test]
    async fn the_layers_query_rejection_is_axums_word_for_word() {
        use axum::extract::FromRequestParts;
        let (mut parts, _) = axum::http::Request::builder().uri("/x?limit=abc").body(()).unwrap().into_parts();
        let axum_text = match axum::extract::Query::<ListQuery>::from_request_parts(&mut parts, &()).await {
            Err(rej) => rej.body_text(),
            Ok(_) => panic!("axum accepted limit=abc"),
        };
        assert_eq!(api::parse_query::<ListQuery>("limit=abc").unwrap_err().message, axum_text);
        // The sentence names the FIELD, which is the part a plain
        // `serde_urlencoded::from_str` would have dropped.
        assert!(axum_text.contains("limit"), "{axum_text}");
    }

    /// R-6.5a.3's 401 pin: a game route handed the host's `Err` sentence answers
    /// that sentence, in this API's shape, in every identity mode — the layer
    /// carries the host's words and never writes its own.
    #[tokio::test]
    async fn an_err_identity_through_dispatch_is_the_extractors_401_sentence_in_every_mode() {
        for mode in [
            identity::IdentityMode::Claimed,
            identity::IdentityMode::CfAccess,
            identity::IdentityMode::Worker,
        ] {
            let sentence = identity::resolve(mode, &http::HeaderMap::new()).unwrap_err();
            let state = AppState::in_memory_with(mode);
            let host = api::Host {
                identity_name: mode.name(),
                signal_kind: SIGNAL_KIND,
                serves_games: true,
                multiplayer_offered: true,
            };
            let out = api::dispatch(
                &state.games,
                &host,
                Err(&sentence),
                &api::ApiRequest { method: "GET", path: "/api/games", query: "", content_type: None, body: b"" },
            );
            assert_eq!(out.status, 401, "{}", mode.name());
            assert_eq!(out.body, serde_json::json!({ "error": sentence }).to_string(), "{}", mode.name());
        }
    }

    /// R-6.5a.4's third pin: the `{k}` segment's rejection is axum's
    /// `Path<(String, usize)>` text word for word — read off a throwaway axum
    /// router, because a `Path` cannot be extracted outside one — and the real
    /// app answers a signed-in caller the same 400 BEFORE any table lookup, the
    /// precedence axum's extractor order gave it.
    #[tokio::test]
    async fn the_layers_path_rejection_is_axums_word_for_word() {
        let probe = Router::new().route(
            "/api/games/{id}/at/{k}",
            get(|Path((id, k)): Path<(String, usize)>| async move { format!("{id}:{k}") }),
        );
        let res = probe
            .oneshot(Request::builder().uri("/api/games/x/at/abc").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let axum_text =
            String::from_utf8(axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap().to_vec()).unwrap();
        assert_eq!(api::parse_path_usize(1, "abc").unwrap_err().message, axum_text);
        let (status, body) =
            raw(app(Arc::new(AppState::in_memory())), "GET", "/api/games/nowhere/at/abc", Some(WHO)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "the segment is refused before the table is looked up");
        assert_eq!(body, serde_json::json!({ "error": axum_text }).to_string().as_bytes());
    }

    /// R-6.5a.12: the record route's three answers — 409 while the game runs,
    /// 403 for a watcher of a finished game, and for a seat holder the store's
    /// EXACT bytes, digest included (the anti-vacuity: the stored record really
    /// carries a `terminal_digest`, so byte-equality is a claim about something).
    #[tokio::test]
    async fn the_record_route_is_409_while_running_403_for_a_watcher_and_the_stored_bytes_when_over() {
        let (state, name, _) = a_finished_two_person_game("rec").await;
        start_as(&state, "running", ENDED_SEED, &["red@wire.test", "blue@wire.test"]).await;
        let (status, body) = get_as(&state, "red@wire.test", "/api/games/running/record").await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], "the record is available once the game is over");
        assert!(body.get("status").is_none(), "no table status rides the 409: {body}");
        // A watcher of the FINISHED game: signed in, no seat, no lock to knock on.
        let (status, body) = get_as(&state, "green@wire.test", &format!("/api/games/{name}/record")).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "the record is the players' — watch, download nothing");
        // A seat holder gets the store's exact bytes.
        let (status, raw_bytes) =
            raw(app(Arc::clone(&state)), "GET", &format!("/api/games/{name}/record"), Some("red@wire.test")).await;
        assert_eq!(status, StatusCode::OK);
        let (stored, _) = state.games.store().read(store::Doc::Record, name).unwrap().unwrap();
        assert_eq!(raw_bytes, stored.as_bytes(), "the body is the stored record, not a re-serialization");
        assert!(stored.contains("\"terminal_digest\""), "anti-vacuity: the stored record carries its digest");
        // And it is a 405 under POST, like every other GET-only route.
        let (status, _) = raw(app(Arc::clone(&state)), "POST", &format!("/api/games/{name}/record"), Some("red@wire.test")).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    // -----------------------------------------------------------------------
    // M6.5b: `POST /api/archive` (R-6.5b.6, R-6.5b.15)
    // -----------------------------------------------------------------------

    /// A directory of this run's own, emptied first: a re-run must meet a fresh
    /// archive rather than the last run's landed pairs, or the once-rule fires
    /// before the cell's first assertion.
    fn tempdir(name: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "yokohama_web_{name}_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp directory");
        dir
    }

    /// A state whose archive is a real directory — the one shape in which the
    /// archive route is anything but a 404.
    fn archive_app(dir: &std::path::Path) -> Arc<AppState> {
        Arc::new(AppState::with_store_chat_and_archive(
            Arc::new(MemStore::default()),
            identity::IdentityMode::Claimed,
            chat::ChatConfig::default(),
            Arc::new(archive::DirArchive::new(dir)),
        ))
    }

    /// The password on the local pair's table document, so the server-side
    /// re-strip has something to remove and the cell has a value to search for.
    const LOCAL_PAIR_PASSWORD: &str = "hunter2";

    /// The two strings a finished browser game sends: a REAL record, played to
    /// the end, and the table document that describes it — with a lock added,
    /// because the claim is that the server re-strips what the body carried.
    ///
    /// Returns the lock's salt too: the strip is asserted against the VALUE the
    /// body really carried rather than against the word "lock", which a copy
    /// under another key would walk straight past (the M6.1 lesson).
    async fn finished_local_pair() -> (String, String, String) {
        let (state, name, _) = a_finished_two_person_game("local-pair").await;
        let (record, _) = state.games.store().read(Doc::Record, name).unwrap().unwrap();
        let (table, _) = state.games.store().read(Doc::Table, name).unwrap().unwrap();
        let mut doc: table::TableDoc = serde_json::from_str(&table).unwrap();
        let lock = table::Lock::new(LOCAL_PAIR_PASSWORD);
        let salt = lock.salt.clone();
        doc.lock = Some(lock);
        (record, serde_json::to_string(&doc).unwrap(), salt)
    }

    /// One key of a JSON object changed, everything else left exactly as it was
    /// — the shape all six body edits below share.
    fn edited(text: &str, f: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>)) -> String {
        let mut v: serde_json::Value = serde_json::from_str(text).expect("the body is JSON");
        f(v.as_object_mut().expect("an object"));
        v.to_string()
    }

    fn with_rev(record: &str, rev: u64) -> String {
        edited(record, |o| {
            o.insert("semantics_rev".into(), rev.into());
        })
    }

    fn without_rev(record: &str) -> String {
        edited(record, |o| {
            o.remove("semantics_rev").expect("the record carried one");
        })
    }

    fn without_digest(record: &str) -> String {
        edited(record, |o| {
            o.remove("terminal_digest").expect("the record carried one");
        })
    }

    /// The first event replaced by a LATER one, which the position it now sits
    /// in cannot legally take — and if it could, the terminal digest no longer
    /// matches. Either way the answer is the replay's refusal, which is the
    /// point: the route does not take the client's word for the history.
    fn with_first_event_replaced(record: &str) -> String {
        edited(record, |o| {
            let events = o.get_mut("events").expect("events").as_array_mut().expect("a list");
            assert!(events.len() > 1, "anti-vacuity: a one-event record cannot be re-ordered");
            events[0] = events[1].clone();
        })
    }

    /// The seed altered by one bit — the shape a `JSON.parse` round trip gives a
    /// seed past 2^53 (spec §6.1). It deals a DIFFERENT game, so the events
    /// replayed against it are not the events that produced them.
    fn with_seed_altered(record: &str) -> String {
        edited(record, |o| {
            let seed = o.get("seed").expect("seed").as_u64().expect("a number");
            o.insert("seed".into(), (seed ^ 1).into());
        })
    }

    /// A record that REPLAYS, is still over, and differs in bytes — the conflict
    /// case. `engine_version` is provenance only (`record.rs`) and
    /// `replay_strict` never reads it, so this is the one edit that keeps the
    /// leaf and the id and therefore the PATH: two builds archiving one game is
    /// exactly the collision the once-rule is about.
    fn with_another_engine_version(record: &str) -> String {
        edited(record, |o| {
            o.insert("engine_version".into(), "0.0.0-elsewhere".into());
        })
    }

    /// A table document that passes `check_shape` and still describes a
    /// different game. `players` cannot do it — `check_shape` refuses a count
    /// that disagrees with the seats — so the disagreement is the start seat,
    /// which is in range at a 2-seat table and wrong against the record.
    fn with_start_seat(table: &str, seat: u64) -> String {
        edited(table, |o| {
            o.insert("start_seat".into(), seat.into());
        })
    }

    /// The first `events` moves of a record, re-stamped so it carries the digest
    /// of the state it really folds to — a game abandoned part way, which is
    /// what the browser's Abandon and save sends.
    fn partial_of(record_text: &str, events: usize) -> String {
        use yokohama_core::{state_at, GameRecord};
        let mut rec: GameRecord = serde_json::from_str(record_text).expect("a record");
        assert!(rec.events.len() > events, "anti-vacuity: the partial must be shorter");
        rec.events.truncate(events);
        let rules = Rules::standard();
        let state = state_at(&rec, events, &rules).expect("the partial replays");
        rec.stamp_terminal_digest(&state);
        serde_json::to_string(&rec).expect("a record")
    }

    async fn post_anonymous(
        state: &Arc<AppState>,
        uri: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let req = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).expect("a body")))
            .expect("a request");
        let res = app(Arc::clone(state)).oneshot(req).await.expect("the router answers");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
    }

    /// R-6.5b.6: the five refusals by five DIFFERENT sentences, the leaf from
    /// the REPLAYED STATE, a partial record ACCEPTED, once per path, the caller
    /// prefix, and the table re-stripped server-side.
    #[tokio::test]
    async fn the_archive_route_checks_and_files_a_browser_game() {
        let dir = tempdir("archive_route");
        let app = archive_app(&dir);
        let (record_text, table_text, salt) = finished_local_pair().await;
        let body = |r: &str, t: &str| serde_json::json!({ "record": r, "table": t });

        // **The premise that makes the leaf a claim**: the table document this
        // body carries does NOT say the game is over — its mirror was written at
        // finalize — so an implementation that read the leaf off the body would
        // file a finished game under `abandoned/`.
        let doc: table::TableDoc = serde_json::from_str(&table_text).unwrap();
        assert!(
            doc.turn.as_ref().is_none_or(|t| !t.over),
            "premise: the body's table does not say the game is over"
        );

        let (status, v) = post_as(&app, WHO, "/api/archive", body(&record_text, &table_text)).await;
        assert_eq!(status, StatusCode::CREATED, "{v}");
        assert_eq!(v["leaf"], "over", "the leaf is the replayed state's, not the body's");
        assert_eq!(v["identical"], false);
        assert_eq!(v["path"], "browser/red_at_wire.test/rev7/over/local-pair.json");
        let landed =
            std::fs::read_to_string(dir.join("browser/red_at_wire.test/rev7/over/local-pair.json"))
                .unwrap();
        assert_eq!(landed, record_text, "the record lands as sent — its bytes are the digest's");
        let table_landed = std::fs::read_to_string(
            dir.join("browser/red_at_wire.test/rev7/over/local-pair.table.json"),
        )
        .unwrap();
        assert!(
            !table_landed.contains(&salt) && !table_landed.contains(LOCAL_PAIR_PASSWORD),
            "re-stripped server-side: the lock the body carried is gone"
        );
        assert!(table_landed.contains("local-pair"), "anti-vacuity: it is the table's document");

        // The same bytes again: 200 `identical`, never a 409 — a lost reply's
        // retry has to be safe to act on, because the client deletes on it.
        let (status, v) = post_as(&app, WHO, "/api/archive", body(&record_text, &table_text)).await;
        assert_eq!(status, StatusCode::OK, "{v}");
        assert_eq!(v["identical"], true);
        assert_eq!(v["path"], "browser/red_at_wire.test/rev7/over/local-pair.json");

        // A DIFFERENT record under the same id: the 409 the client KEEPS the
        // game on. And the landed copy is untouched — a conflict never deletes.
        let other = with_another_engine_version(&record_text);
        assert_ne!(other, record_text, "anti-vacuity: the conflicting record really differs");
        let (status, v) = post_as(&app, WHO, "/api/archive", body(&other, &table_text)).await;
        assert_eq!(status, StatusCode::CONFLICT, "{v}");
        assert_eq!(v["error"], "a different record is archived under this id");
        assert_eq!(
            std::fs::read_to_string(
                dir.join("browser/red_at_wire.test/rev7/over/local-pair.json")
            )
            .unwrap(),
            record_text,
            "a 409 leaves the archived copy exactly as it was"
        );

        // A PARTIAL record is ACCEPTED — under `abandoned/`, the leaf the state
        // shows — which is the amendment R-6.5b.6 makes to umbrella R-6.5.6.
        let partial = partial_of(&record_text, 5);
        let (status, v) = post_as(&app, WHO, "/api/archive", body(&partial, &table_text)).await;
        assert_eq!(status, StatusCode::CREATED, "{v}");
        assert_eq!(v["leaf"], "abandoned");
        assert_eq!(v["path"], "browser/red_at_wire.test/rev7/abandoned/local-pair.json");

        // The five refusals, five sentences.
        let refusals = [
            (with_rev(&record_text, 6), table_text.clone(), "this record is at semantics rev 6; this server archives rev 7"),
            (without_rev(&record_text), table_text.clone(), "missing semantics provenance"),
            (without_digest(&record_text), table_text.clone(), "the record carries no terminal digest"),
            (with_first_event_replaced(&record_text), table_text.clone(), "the record does not replay"),
            (record_text.clone(), with_start_seat(&table_text, 1), "the table document does not agree with the record"),
        ];
        let mut sentences = std::collections::BTreeSet::new();
        for (r, t, expect) in refusals {
            let (status, v) = post_as(&app, WHO, "/api/archive", body(&r, &t)).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{v}");
            let s = v["error"].as_str().unwrap().to_string();
            assert!(s.starts_with(expect), "{s:?} does not start with {expect:?}");
            sentences.insert(s);
        }
        assert_eq!(sentences.len(), 5, "five different sentences");

        // Spec §6.1's own sixth: a seed altered the way a `JSON.parse` round
        // trip alters one deals a different game, and the replay says so.
        let (status, v) =
            post_as(&app, WHO, "/api/archive", body(&with_seed_altered(&record_text), &table_text))
                .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{v}");
        assert!(
            v["error"].as_str().unwrap().starts_with("the record does not replay"),
            "{v}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// The two answers that are not about the record at all: a host that keeps
    /// no archive HAS no such route (404, and under `NoArchive` — the browser's
    /// own host — that is the answer by construction), and a caller with no
    /// identity is refused before the body is looked at.
    ///
    /// **The 404 is sent a REAL pair on purpose**: a host with no archive must
    /// answer before it spends a strict replay, and a body that could not parse
    /// would reach the same 404 by a different road.
    #[tokio::test]
    async fn the_archive_route_is_a_404_where_no_archive_is_kept_and_a_401_with_no_identity() {
        let app = Arc::new(AppState::in_memory());
        let (record_text, table_text, _) = finished_local_pair().await;
        let (status, v) = post_as(
            &app,
            WHO,
            "/api/archive",
            serde_json::json!({ "record": record_text, "table": table_text }),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{v}");
        assert_eq!(v["error"], "this server keeps no archive");
        let (status, _) = post_anonymous(
            &app,
            "/api/archive",
            serde_json::json!({ "record": "{}", "table": "{}" }),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    /// R-6.5a.3: HEAD reaches the GET arm and leaves with an empty body — axum's
    /// `get` served HEAD that way, and a proxy or health checker may send one.
    #[tokio::test]
    async fn head_on_a_get_route_is_200_with_an_empty_body() {
        let (status, body) = raw(app(Arc::new(AppState::in_memory())), "HEAD", "/api/health", None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn a_known_path_under_the_wrong_method_is_405_and_an_unknown_path_is_the_json_404() {
        let app = app(Arc::new(AppState::in_memory()));
        let (status, body) = raw(app.clone(), "DELETE", "/api/health", None).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert!(body.is_empty(), "axum's 405 has no body: {}", String::from_utf8_lossy(&body));
        let (status, body) = raw(app, "GET", "/api/nothing", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body, br#"{"error":"no such endpoint: /api/nothing"}"#);
    }

    async fn raw_post(uri: &str, body: Vec<u8>) -> (StatusCode, serde_json::Value) {
        let res = app(Arc::new(AppState::in_memory()))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("content-type", "application/json")
                    .header(identity::CLAIMED_HEADER, WHO)
                    .body(Body::from(body))
                    .expect("a POST"),
            )
            .await
            .expect("the router answers");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        let value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, value)
    }

    /// A request under headers the CELL chooses — for the identity cells only.
    async fn get_with(
        state: Arc<AppState>,
        uri: &str,
        headers: &[(&str, &str)],
    ) -> (StatusCode, serde_json::Value) {
        let mut req = Request::builder().uri(uri);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let res = app(state)
            .oneshot(req.body(Body::empty()).expect("a GET"))
            .await
            .expect("the router answers");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
    }

    /// **Every game route needs an identity, and the public three do not.**
    ///
    /// Asserted over a route from each family — the collection, a game, and a
    /// sub-surface — because the extractor is added per handler and a handler
    /// that forgot it answers 200 with nothing anywhere saying so. The public
    /// half is what keeps the fence from being "refuse everything": `/api/me`
    /// in particular must answer a caller who has no identity yet, since that
    /// answer is how the client learns which screen to show.
    ///
    /// The last arm is R-1.2 at the ROUTE: under `cf-access` the claimed header
    /// is a header this deployment does not read, so forging it changes nothing.
    #[tokio::test]
    async fn every_game_route_is_401_without_an_identity_and_the_public_ones_are_not() {
        use identity::{IdentityMode as M, CLAIMED_HEADER};
        for uri in ["/api/games", "/api/games/nope", "/api/games/nope/log"] {
            let (status, body) = get_with(Arc::new(AppState::in_memory()), uri, &[]).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}: {body}");
            assert!(
                body["error"].as_str().unwrap_or("").contains("X-Yokohama-Claimed-User"),
                "{body}"
            );
        }
        for uri in ["/api/health", "/api/labels", "/api/me"] {
            let (status, _) = get_with(Arc::new(AppState::in_memory()), uri, &[]).await;
            assert_ne!(status, StatusCode::UNAUTHORIZED, "{uri} is public");
        }
        // The mode reads its own header: under cf-access the claimed header is inert.
        let (status, _) = get_with(
            Arc::new(AppState::in_memory_with(M::CfAccess)),
            "/api/games",
            &[(CLAIMED_HEADER, "forged@t.test")],
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    /// **`/api/me` is the one route that answers "nobody" rather than refusing.**
    ///
    /// A 401 here would leave the client with a status code where it needs a
    /// fact: which mode this deployment runs, and whether the caller is signed
    /// in. An unparseable header is the same answer as no header at all — it
    /// names nobody — and the normalization is the extractor's, so a browser
    /// never has to know that ` Red@Wire.TEST ` and `red@wire.test` are one
    /// person.
    #[tokio::test]
    async fn me_answers_without_identity_and_names_the_mode() {
        use identity::{IdentityMode as M, CF_ACCESS_HEADER, CLAIMED_HEADER};
        let (status, body) = get_with(Arc::new(AppState::in_memory()), "/api/me", &[]).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, serde_json::json!({ "mode": "claimed", "user": null }));
        let (_, body) = get_with(
            Arc::new(AppState::in_memory()),
            "/api/me",
            &[(CLAIMED_HEADER, " Red@Wire.TEST ")],
        )
        .await;
        assert_eq!(body["user"], "red@wire.test", "normalized once, in the extractor");
        let (_, body) = get_with(
            Arc::new(AppState::in_memory()),
            "/api/me",
            &[(CLAIMED_HEADER, "..")],
        )
        .await;
        assert_eq!(
            body["user"],
            serde_json::Value::Null,
            "an invalid header resolves to null here, never 401"
        );
        // M6.5c2: a plain name is a key, normalized once in the extractor.
        let (_, body) = get_with(
            Arc::new(AppState::in_memory()),
            "/api/me",
            &[(CLAIMED_HEADER, "  Ben   Castle ")],
        )
        .await;
        assert_eq!(body["user"], "ben castle", "a name is a key, normalized once");
        let (_, body) = get_with(
            Arc::new(AppState::in_memory_with(M::CfAccess)),
            "/api/me",
            &[(CF_ACCESS_HEADER, "a@b.c")],
        )
        .await;
        assert_eq!(body, serde_json::json!({ "mode": "cf-access", "user": "a@b.c" }));
    }

    /// A key this API does not know is a REFUSAL, not a default.
    ///
    /// The defect, measured before the fence: `NewGame`'s `start_seat`, `seed`,
    /// `bot_seats` and `name` are `#[serde(default)]` and nothing used
    /// `deny_unknown_fields`, so a client that renamed `start_seat` to
    /// `startSeat` got a **201 and a different game** — seat 0 instead of the
    /// seat it asked for — with nothing anywhere saying so. A default is the
    /// right answer for an ABSENT optional field and the wrong one for a field
    /// the caller believed it was setting.
    ///
    /// Asserted in BOTH directions: the renamed key is refused AND names
    /// itself, and the correctly-spelled request still creates the game. A
    /// fence that refused everything would pass the first half.
    #[tokio::test]
    async fn an_unknown_key_in_a_new_game_request_is_refused_rather_than_defaulted() {
        let good = serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 1,
            "seed": "20260831", "name": "fenced", "id": "fenced",
            "seats": [{ "user": WHO }, { "user": WHO }],
        });
        let mut renamed = good.clone();
        renamed.as_object_mut().expect("an object").remove("start_seat");
        renamed["startSeat"] = serde_json::json!(1);

        let (status, body) = post_json("/api/games", renamed).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        let msg = body["error"].as_str().unwrap_or_default();
        assert!(msg.contains("startSeat"), "the refusal names the key it did not know: {msg}");

        let (status, _) = post_json("/api/games", good).await;
        assert_eq!(status, StatusCode::CREATED, "the correctly-spelled request still works");
    }

    /// **A `deal` this server cannot BUILD is a 500; a request it will not
    /// ACCEPT is still a 400 — and neither takes the server down.**
    ///
    /// Inherited from `preview`, whose handler was the one place this API
    /// answered a 500 and which is retired (M6.1 §8). The claim moved rather
    /// than going away, because it is about the API and not about
    /// that route: a failure DOWNSTREAM of a request this server already
    /// accepted is this process's, and answering "your request was wrong" sends
    /// a client away to fix something that is not broken.
    ///
    /// The broken table is `players: 9` with nine seats, nine keeps and nine
    /// colours — every length agreeing with every other, so only `check_shape`'s
    /// 2..=4 bound catches it — written STRAIGHT INTO THE STORE, because no
    /// request can produce one. `games.rs`'s
    /// `a_malformed_table_document_refuses_typed_and_leaves_the_server_alive`
    /// makes the same claim about the typed error; this one is about the STATUS
    /// a browser reads, which that cell cannot see.
    ///
    /// **Every one of these answers comes off ONE router, because the
    /// discrimination is the claim**: a handler that answered 500 to everything
    /// would satisfy the first line alone, and one that answered 400 to
    /// everything would satisfy none of the others while still looking careful.
    ///
    /// **And the closing pair is load-bearing.** `deal` reaches `settle`, which
    /// takes a lock when it has repairing to do, so a panic in there is not a
    /// dropped connection but a POISONED mutex. A create of a different name
    /// answering 201, and its own deal answering 200, is what a `catch_unwind`
    /// could not fake.
    ///
    /// **Read it at its M6.2 size.** The lock `settle` takes is now that
    /// table's own (`Games::lock_for`), so a poisoning here is confined to the
    /// broken game and a create of another name would survive it in any case;
    /// what the closing pair still says is that the PROCESS is alive and
    /// serving, which is what a panic in a handler does not leave behind.
    #[tokio::test]
    async fn a_deal_the_server_cannot_build_is_a_500_a_bad_request_is_400_and_the_server_lives() {
        let state = Arc::new(AppState::in_memory());
        let router = app(Arc::clone(&state));
        let broken = serde_json::json!({
            "schema": 1,
            "id": "broken",
            "name": "broken",
            "created_by": WHO,
            "created_at": 1_789_000_000u64,
            "players": 9,
            "colors": ["Red", "Blue", "Yellow", "Green", "Red", "Blue", "Yellow", "Green", "Red"],
            "start_seat": 0,
            "seats": (0..9)
                .map(|_| serde_json::json!({ "kind": "human", "user": WHO }))
                .collect::<Vec<_>>(),
            "keeps": [null, null, null, null, null, null, null, null, null],
            "seed": 20260830u64,
            "status": "dealing",
            "turn": null,
        });
        state
            .games
            .store()
            .write_if_version(Doc::Table, "broken", &broken.to_string(), None)
            .expect("the store takes what it is given");

        let (status, body) = call(&router, "GET", "/api/games/broken/deal", None).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
        assert!(body["error"].is_string(), "and it carries a sentence: {body}");

        // Not a blanket 500: a table that is not there is a 404...
        let (status, body) = call(&router, "GET", "/api/games/no-such-table/deal", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");

        // ...and a request this API will not accept is the caller's own 400,
        // named by the key it did not know.
        let mut bad = serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
            "seats": seats_for(2, &[1]), "name": "typo", "id": "typo",
        });
        bad["startSeat"] = serde_json::json!(0);
        let (status, body) = call(&router, "POST", "/api/games", Some(bad)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert!(
            body["error"].as_str().unwrap_or_default().contains("startSeat"),
            "the refusal names the key it did not know: {body}",
        );

        // The server is alive: a table of a different name still opens, and its
        // own deal still answers.
        let (status, body) = call(
            &router,
            "POST",
            "/api/games",
            Some(serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
                "seats": seats_for(2, &[1]), "name": "after", "id": "after",
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let (status, body) = call(&router, "GET", "/api/games/after/deal", None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["cards"].as_array().expect("two cards").len(), 2, "{body}");
    }

    /// The same fence on the one MUTATION body. `expect` is what stops a number
    /// meaning a different move, so a misspelling of it must not be silently
    /// absent — that is the stale-picker defect wearing a typo.
    #[tokio::test]
    async fn an_unknown_key_in_an_input_request_is_refused() {
        let (status, body) =
            post_json("/api/games/nope/input", serde_json::json!({ "line": "1", "expects": "options" }))
                .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert!(
            body["error"].as_str().unwrap_or_default().contains("expects"),
            "names the key: {body}"
        );
    }

    /// And on the one QUERY. `?tale=5` for `?tail=5` is a silent full log.
    #[tokio::test]
    async fn an_unknown_query_key_is_refused() {
        let (status, body) = get_json("/api/games/nope/log?tale=5").await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert!(
            body["error"].as_str().unwrap_or_default().contains("tale"),
            "names the key: {body}"
        );
    }

    /// **Every refusal on this API is the same JSON shape**, including the ones
    /// the extractor raises before a handler runs.
    ///
    /// `axum::Json`'s own rejection is a PLAIN-TEXT body and a 422, and
    /// `web/src/api.ts` builds its `ApiError` by reading `.error` off the body —
    /// so an unparseable request reached the player as a bare status code with
    /// no sentence at all. The two cells below are the two rejections a real
    /// client produces: broken syntax, and a missing required field.
    #[tokio::test]
    async fn a_rejected_body_answers_in_this_apis_own_error_shape() {
        let (status, body) = raw_post("/api/games", b"{not json".to_vec()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].is_string(), "syntax error carries a sentence: {body}");

        let (status, body) = post_json("/api/games", serde_json::json!({ "players": 2 })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            body["error"].as_str().unwrap_or_default().contains("colors"),
            "a missing required field names itself: {body}"
        );
    }

    /// A router that OUTLIVES one call, so a test can create a game and then
    /// ask it something. `get_json`/`post_json` each build their own
    /// `AppState::in_memory`, which is right for a one-shot and useless here.
    fn live_router() -> Router {
        app(Arc::new(AppState::in_memory()))
    }

    async fn call(
        router: &Router,
        method: &str,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let req =
            Request::builder().method(method).uri(uri).header(identity::CLAIMED_HEADER, WHO);
        let req = match body {
            Some(b) => req
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&b).expect("a body"))),
            None => req.body(Body::empty()),
        }
        .expect("a request");
        let res = router.clone().oneshot(req).await.expect("the router answers");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
    }

    /// The `seats` list a create takes: [`WHO`] in every human seat, `bot` in
    /// the ones named.
    ///
    /// **Every human seat is the SAME person**, because [`call`] sends one
    /// identity and these cells are about the routes rather than about who holds
    /// what. A hot seat is a real table — the CLI's "one operator driving the
    /// table" — so nothing here is a fiction the product does not support; the
    /// cells whose subject IS two people drive two identities instead
    /// ([`post_as`] and friends).
    fn seats_for(players: u8, bots: &[u8]) -> serde_json::Value {
        serde_json::Value::Array(
            (0..players)
                .map(|seat| {
                    if bots.contains(&seat) {
                        serde_json::json!({ "bot": true })
                    } else {
                        serde_json::json!({ "user": WHO })
                    }
                })
                .collect(),
        )
    }

    /// Open a table, keep at every human seat, and return the game's first
    /// `ClientState` — the payload `create` used to answer with.
    ///
    /// A game is no longer one call (M6.1 §4.3): a create opens a DEALING table
    /// and the last keep is what builds the session. This is that sequence, so
    /// every cell below still reads as "start a game, then ask it something".
    async fn start(
        router: &Router,
        name: &str,
        players: u8,
        bots: &[u8],
    ) -> serde_json::Value {
        let colors = ["Red", "Blue", "Yellow", "Green"];
        let (status, table) = call(
            router,
            "POST",
            "/api/games",
            Some(serde_json::json!({
                "players": players,
                "colors": colors[..usize::from(players)],
                "start_seat": 0,
                "seed": "20260831",
                "seats": seats_for(players, bots),
                "name": name,
                "id": name,
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{table}");
        for seat in (0..players).filter(|s| !bots.contains(s)) {
            let (status, kept) = call(
                router,
                "POST",
                &format!("/api/games/{name}/keep"),
                Some(serde_json::json!({ "seat": seat, "card": 1 })),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "seat {seat}: {kept}");
        }
        let (status, state) = call(router, "GET", &format!("/api/games/{name}"), None).await;
        assert_eq!(status, StatusCode::OK, "{state}");
        state
    }

    /// Start a 2P game and return its first payload.
    async fn a_game(router: &Router, name: &str, bots: &[u8]) -> serde_json::Value {
        start(router, name, 2, bots).await
    }

    /// A request under a NAMED identity, against a state several of them share.
    ///
    /// The cells about seats need two people at one table, which `call`'s single
    /// [`WHO`] cannot express. `app(state)` is rebuilt per call — a `Router` is
    /// cheap and `oneshot` consumes a clone anyway — so what is shared is the
    /// `AppState`, which is the game.
    async fn request_as(
        state: &Arc<AppState>,
        who: &str,
        method: &str,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let req = Request::builder().method(method).uri(uri).header(identity::CLAIMED_HEADER, who);
        let req = match body {
            Some(b) => req
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&b).expect("a body"))),
            None => req.body(Body::empty()),
        }
        .expect("a request");
        let res = app(Arc::clone(state)).oneshot(req).await.expect("the router answers");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
    }

    async fn post_as(
        state: &Arc<AppState>,
        who: &str,
        uri: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        request_as(state, who, "POST", uri, Some(body)).await
    }

    async fn get_as(
        state: &Arc<AppState>,
        who: &str,
        uri: &str,
    ) -> (StatusCode, serde_json::Value) {
        request_as(state, who, "GET", uri, None).await
    }

    async fn delete_as(
        state: &Arc<AppState>,
        who: &str,
        uri: &str,
    ) -> (StatusCode, serde_json::Value) {
        request_as(state, who, "DELETE", uri, None).await
    }

    /// **A started game whose seats belong to the NAMED people, each keeping as
    /// itself** — the multi-identity sibling of [`start`], which drives one.
    ///
    /// Not an overload of that name, because it is not the same helper: `start`
    /// takes a `Router` and a bot list and plays every seat as [`WHO`], and the
    /// cells below need a `state` several identities share. The `_as` suffix is
    /// the one [`get_as`] and [`post_as`] already use for "under this identity",
    /// and here it is per SEAT: every keep is posted by the person who holds the
    /// seat it is for, which is what the keep route requires.
    async fn start_as(state: &Arc<AppState>, name: &str, seed: u64, seats: &[&str]) {
        let colors = ["Red", "Blue", "Yellow", "Green"];
        let players = seats.len();
        let (status, table) = post_as(
            state,
            seats[0],
            "/api/games",
            serde_json::json!({
                "players": players,
                "colors": colors[..players],
                "start_seat": 0,
                "seed": seed.to_string(),
                "seats": seats.iter().map(|u| serde_json::json!({ "user": u })).collect::<Vec<_>>(),
                "name": name,
                "id": name,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{table}");
        for (seat, who) in seats.iter().enumerate() {
            let (status, kept) = post_as(
                state,
                who,
                &format!("/api/games/{name}/keep"),
                serde_json::json!({ "seat": seat, "card": 1 }),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "seat {seat} as {who}: {kept}");
        }
    }

    /// [`start_as`] with a PASSWORD on the table (M6.3): the shape that refuses
    /// a watcher until they have knocked.
    async fn start_locked_as(
        state: &Arc<AppState>,
        name: &str,
        seed: u64,
        seats: &[&str],
        password: &str,
    ) {
        let colors = ["Red", "Blue", "Yellow", "Green"];
        let players = seats.len();
        let (status, table) = post_as(
            state,
            seats[0],
            "/api/games",
            serde_json::json!({
                "players": players,
                "colors": colors[..players],
                "start_seat": 0,
                "seed": seed.to_string(),
                "seats": seats.iter().map(|u| serde_json::json!({ "user": u })).collect::<Vec<_>>(),
                "name": name,
                "id": name,
                "password": password,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{table}");
        for (seat, who) in seats.iter().enumerate() {
            let (status, kept) = post_as(
                state,
                who,
                &format!("/api/games/{name}/keep"),
                serde_json::json!({ "seat": seat, "card": 1 }),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "seat {seat} as {who}: {kept}");
        }
    }

    /// Play `user`'s whole turn, so a cell that needs a turn BOUNDARY has one.
    ///
    /// The last option row each time — the one that ends a phase — which is the
    /// same walk `games::tests::the_turn_mirror_follows_every_persisted_move`
    /// takes at the `Repl` level, here over the route so the fence is exercised
    /// too. Bounded, and the bound FAILS rather than passing quietly: a turn
    /// that never ends means the cell below proves nothing about a boundary.
    async fn play_turn(state: &Arc<AppState>, id: &str, user: &str) {
        let (_, before) = get_as(state, user, &format!("/api/games/{id}")).await;
        let mine = before["to_move"].as_u64().expect("a seat to move");
        for _ in 0..40 {
            let (_, now) = get_as(state, user, &format!("/api/games/{id}")).await;
            if now["to_move"].as_u64() != Some(mine) || now["over"] == serde_json::json!(true) {
                return;
            }
            let rows = now["prompt"]["rows"].as_array().cloned().unwrap_or_default();
            let line = match rows.last().and_then(|r| r["number"].as_u64()) {
                Some(n) => n.to_string(),
                None => "confirm".to_string(),
            };
            let (status, body) = post_as(
                state,
                user,
                &format!("/api/games/{id}/input"),
                serde_json::json!({ "line": line }),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{line}: {body}");
        }
        panic!("{user}'s turn did not end in 40 inputs");
    }

    /// **One person and a bot** — the table R-P6 deliberately leaves alone.
    async fn start_vs_bot(state: &Arc<AppState>, name: &str, seed: u64, who: &str) {
        let (status, table) = post_as(
            state,
            who,
            "/api/games",
            serde_json::json!({
                "players": 2,
                "colors": ["Red", "Blue"],
                "start_seat": 0,
                "seed": seed.to_string(),
                "seats": [{ "user": who }, { "bot": true }],
                "name": name,
                "id": name,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{table}");
        let (status, kept) = post_as(
            state,
            who,
            &format!("/api/games/{name}/keep"),
            serde_json::json!({ "seat": 0, "card": 1 }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{kept}");
    }

    /// The FIRST row of an options prompt — deliberately not [`play_turn`]'s
    /// last one, which is the row that ends a phase. A cell about undoing
    /// INSIDE a turn needs a move that leaves the turn running.
    fn first_option(game: &serde_json::Value) -> String {
        let rows = game["prompt"]["rows"].as_array().expect("an options prompt");
        rows.first()
            .and_then(|r| r["number"].as_u64())
            .unwrap_or_else(|| panic!("a numbered first row: {game}"))
            .to_string()
    }

    /// Play `user`'s whole turn at a table with a BOT, and hand back the reply
    /// that carried the bot's run.
    ///
    /// **[`play_turn`] cannot serve here, and the reason is the behaviour being
    /// tested.** `Repl::step` pumps every bot seat owed a move before it
    /// returns, so between two of this caller's inputs `to_move` is never the
    /// bot's: the turn boundary happens INSIDE one reply and a `to_move` watch
    /// never sees it. So the stop is the bot's own log line, which is also the
    /// evidence the cell needs — an undo with no trailing bot run proves
    /// nothing about unwinding one. Bounded, and the bound FAILS rather than
    /// passing quietly.
    async fn play_turn_vs_bot(
        state: &Arc<AppState>,
        id: &str,
        user: &str,
        bot_color: &str,
    ) -> serde_json::Value {
        for _ in 0..40 {
            let (_, now) = get_as(state, user, &format!("/api/games/{id}")).await;
            let line = match now["prompt"]["rows"].as_array().and_then(|r| r.last()) {
                Some(row) => row["number"].as_u64().expect("a numbered row").to_string(),
                None => "confirm".to_string(),
            };
            let (status, reply) = post_as(
                state,
                user,
                &format!("/api/games/{id}/input"),
                serde_json::json!({ "line": line }),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{line}: {reply}");
            let (_, log) = get_as(state, user, &format!("/api/games/{id}/log?tail=1")).await;
            let last = log["entries"][0]["text"].as_str().unwrap_or_default().to_string();
            if last.starts_with(bot_color) {
                return reply;
            }
        }
        panic!("the bot never moved in 40 of {user}'s inputs");
    }

    /// Every session route, as one list, with the body a POST needs.
    ///
    /// The list rather than a literal per cell, because the property below is
    /// about COVERAGE: a fence applied to sixteen routes and forgotten on the
    /// seventeenth is exactly the shape that ships, and a route added to the
    /// router and not to this list is the same defect one layer up.
    fn session_routes() -> Vec<(&'static str, &'static str, Option<serde_json::Value>)> {
        vec![
            ("GET", "", None),
            ("GET", "/table", None),
            ("GET", "/deal", None),
            ("POST", "/keep", Some(serde_json::json!({ "seat": 1, "card": 1 }))),
            ("POST", "/join", Some(serde_json::json!({ "seat": 1 }))),
            ("POST", "/input", Some(serde_json::json!({ "line": "1" }))),
            ("GET", "/ports", None),
            ("GET", "/labs", None),
            ("GET", "/areas", None),
            ("GET", "/log", None),
            ("GET", "/events", None),
            ("GET", "/at/0", None),
            ("GET", "/suggest", None),
            ("GET", "/why", None),
            ("GET", "/index/orders", None),
            ("GET", "/result", None),
            ("GET", "/chat", None),
            ("POST", "/chat", Some(serde_json::json!({ "text": "hello" }))),
            ("DELETE", "", None),
        ]
    }

    /// **Every route that needs a session refuses a signed-in stranger at an
    /// open, a dealing and a LOCKED playing table, with exactly two exceptions
    /// — `join` at open, and `table` everywhere it is asked here** (R-1.5 with
    /// R-2.7 and M6.3's R-3.13).
    ///
    /// **M6.3 changed the playing column, and the change is the cycle**
    /// (R-3.5): a stranger at an UNLOCKED playing table is now admitted as a
    /// watcher, so this cell's third table is a LOCKED one — the status where a
    /// caller with no seat is still refused everything. The unlocked rows, and
    /// the 200s a spectator gets, belong to
    /// `the_whole_admission_table_is_answered_route_by_route`, which runs
    /// §3.5's grid in full. Split that way because the two cells prove
    /// different things: this one is about COVERAGE of `session_routes()`, and
    /// that one is about the grid.
    ///
    /// **Four routes do not read through `admitted` and keep their own
    /// answers**, which is why the expected sentence is a table and not a
    /// constant: `deal`, `keep` and `DELETE` resolve the seat inside `Games`
    /// (so a lock is not their answer — the seat is), and `join` is
    /// deliberately left on M6.2's refusal order.
    ///
    /// One cell over the whole table rather than nineteen, because a fence
    /// applied to sixteen routes and forgotten on the seventeenth is exactly the
    /// shape that ships — and since M6.2 the claim is two-dimensional: the same
    /// nineteen routes against an OPEN, a DEALING and a PLAYING table, since
    /// R-2.7 widened two of them at one status only and a widening that leaked
    /// into the others is the defect this exists to catch.
    ///
    /// **THREE statuses and not four.** `TableStatus` has exactly three
    /// variants; "over" is not one of them — a finished game is a `Playing`
    /// table whose session says `is_over`, read off the turn mirror — and
    /// reaching it in a lib cell means playing a 2P game to a winner, which the
    /// whole-game gate does in tens of seconds of OS-entropy wall. For a
    /// STRANGER the two are the same request in any case: the table read comes
    /// first and refuses before anything looks at the session.
    ///
    /// **The order of the steps is the cell.** A successful `join` SEATS the
    /// caller and, at a 2P table with one open seat, FLIPS the table to dealing
    /// — so it is done LAST at the open table, after every other route has been
    /// asked of both callers. An earlier version of this walk joined fifth and
    /// measured eleven routes against a DEALING table while reporting them as
    /// open.
    ///
    /// **`POST …/keep` was the one route that answered a stranger differently,
    /// and this cell is where that was found** (ruling #8). `Games::keep` must
    /// settle under the table's lock, and its `Open`/`Playing` arms used to sit
    /// ABOVE the holder check — so a caller with no seat here was told WHICH
    /// status the table was in (409 "this table is still open" / "this table is
    /// playing") where every other route answered one sentence about seats and
    /// nothing else. The check moved above them; all nineteen routes now
    /// refuse a stranger alike, and the only thing `keep` still says
    /// differently is its own sentence, "seat N is not yours", which is
    /// deliberate and older than this cycle.
    #[tokio::test]
    async fn every_session_route_refuses_a_stranger_at_every_status_except_table_and_join_at_open()
    {
        const STRANGER: &str = "green@wire.test";
        const NOT_SEATED: &str = "you are not seated at this table";
        const LOCKED: &str = "this table is locked — unlock it first";
        let state = Arc::new(AppState::in_memory());
        let open_table = serde_json::json!({
            "players": 2,
            "colors": ["Red", "Blue"],
            "start_seat": 0,
            "seed": "20260831",
            "seats": [{ "user": "red@wire.test" }, { "open": true }],
            "id": "opent",
            "name": "opent",
        });
        let (status, body) = post_as(&state, "red@wire.test", "/api/games", open_table).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        assert_eq!(body["status"], "open", "{body}");
        // DEALING is a create with both seats named and no keep posted — the
        // one shape `start_as` cannot leave behind, since it keeps for every
        // seat and finishes at playing.
        let (status, body) = post_as(
            &state,
            "red@wire.test",
            "/api/games",
            serde_json::json!({
                "players": 2,
                "colors": ["Red", "Blue"],
                "start_seat": 0,
                "seed": "20260831",
                "seats": [{ "user": "red@wire.test" }, { "user": "blue@wire.test" }],
                "id": "dealt",
                "name": "dealt",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        assert_eq!(body["status"], "dealing", "{body}");
        start_locked_as(&state, "lockt", 20260831, &["red@wire.test", "blue@wire.test"], "sesame")
            .await;

        // 1. THE STRANGER, at all three tables, on every route but `table` and
        //    `join` — which are asked afterwards, `join` last of all. 403 on
        //    every one of them, at every status, and NO route says which status
        //    it was (ruling #8 closed the one that did).
        for table in ["opent", "dealt", "lockt"] {
            for (method, suffix, body) in session_routes() {
                if suffix == "/table" || suffix == "/join" {
                    continue;
                }
                // **The third exception, and it is at OPEN only** (§3.5 row 2,
                // R-4.3): the signal answers any signed-in caller there, so the
                // joiner's open-table screen learns when the seats fill. It is
                // asserted just below rather than quietly skipped, and the
                // `dealt` and `lockt` rows of this very loop are what keep the
                // widening from having spread.
                if suffix == "/events" && table == "opent" {
                    continue;
                }
                let uri = format!("/api/games/{table}{suffix}");
                let (status, got) = request_as(&state, STRANGER, method, &uri, body).await;
                assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri}: {got}");
                let expected = match (table, method, suffix) {
                    // `keep`'s own sentence is older than this cycle and stays:
                    // a seat nobody holds and a seat somebody else holds are one
                    // refusal. What matters here is that it names a SEAT and
                    // never a status, so it tells a stranger nothing the others
                    // do not.
                    (_, _, "/keep") => "seat 1 is not yours",
                    // The three that resolve the seat inside `Games` rather
                    // than through `admitted`: the lock is not their answer.
                    ("lockt", "DELETE", "") | ("lockt", _, "/deal") => NOT_SEATED,
                    // Everything that reads through `admitted` says what this
                    // caller can DO about it (M6.3) — which is the difference
                    // between a watcher and a stranger. **M6.4's two chat rows
                    // are answered HERE and by the arm below**, and are named
                    // rather than given arms of their own: both halves of the
                    // chat go through `admitted`, so the lock sentence at
                    // `lockt` and the seat sentence at the other two are the
                    // general answers and not a second statement of them. An
                    // arm per route would be a copy that stops moving the day
                    // the general one does.
                    ("lockt", _, _) => LOCKED,
                    _ => NOT_SEATED,
                };
                assert_eq!(got["error"], expected, "{method} {uri}: {got}");
                assert!(got.get("status").is_none(), "no status leaks: {method} {uri}: {got}");
            }
        }

        // The widened signal, asserted where the loop above skipped it.
        let (status, at_open) = get_as(&state, STRANGER, "/api/games/opent/events").await;
        assert_eq!(status, StatusCode::OK, "{at_open}");
        assert_eq!(at_open["status"], "open", "{at_open}");
        assert_eq!(at_open["your_turn"], false, "{at_open}");

        // 2. `table` is the widening: at open since R-2.7, and at playing and
        //    over since R-3.13 — a would-be watcher has to read `locked` and
        //    `can_watch` to know whether to ask for a password. DEALING is the
        //    anti-vacuity half and the whole of the remaining fence: a widening
        //    written as "anybody may read a table" passes the first two lines
        //    and fails the third.
        let (status, view) = get_as(&state, STRANGER, "/api/games/opent/table").await;
        assert_eq!(status, StatusCode::OK, "{view}");
        assert_eq!(view["my_seats"], serde_json::json!([]), "a caller with no seat here: {view}");
        assert_eq!(view["status"], "open", "{view}");
        let (status, view) = get_as(&state, STRANGER, "/api/games/lockt/table").await;
        assert_eq!(status, StatusCode::OK, "{view}");
        assert_eq!(view["my_seats"], serde_json::json!([]), "{view}");
        assert_eq!(view["locked"], true, "{view}");
        assert_eq!(view["can_watch"], false, "seen, and not watchable yet: {view}");
        let (status, got) = get_as(&state, STRANGER, "/api/games/dealt/table").await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{got}");
        assert_eq!(got["error"], NOT_SEATED, "{got}");

        // 3. THE SEAT HOLDER at the open table hears the status instead (§3.4),
        //    on every route but `table`, `join`, `events` and DELETE. Before
        //    the stranger's join below, which would move this table out from
        //    under it.
        //
        //    **`events` left this loop in M6.3** (§3.3, R-3.3) and is asserted
        //    just below instead: a seat holder watching an open table for its
        //    seats to fill is exactly who polls it, and a 409 there would make
        //    the open-table screen branch on an error. It is the second
        //    widening at this status, after `table`'s, and it is written as an
        //    exception at the site rather than as a quietly shorter loop.
        let (status, at_open) = get_as(&state, "red@wire.test", "/api/games/opent/events").await;
        assert_eq!(status, StatusCode::OK, "{at_open}");
        assert_eq!(at_open["status"], "open", "{at_open}");
        // **And `/chat` left it in M6.4**, for the same reason one layer along:
        // a message needs no session, and the people waiting for a table to
        // fill are exactly the people talking at it. The two halves are
        // asserted here rather than skipped quietly, and the `dealt` and
        // `lockt` rows of the stranger loop above are what keep this widening
        // from having spread to callers with no seat.
        let (status, said) = post_as(
            &state,
            "red@wire.test",
            "/api/games/opent/chat",
            serde_json::json!({ "text": "waiting on a second" }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{said}");
        let (status, page) = get_as(&state, "red@wire.test", "/api/games/opent/chat").await;
        assert_eq!(status, StatusCode::OK, "{page}");
        assert_eq!(page["last"], 1, "{page}");
        for (method, suffix, body) in session_routes() {
            if suffix == "/table"
                || suffix == "/join"
                || suffix == "/events"
                || suffix == "/chat"
                || method == "DELETE"
            {
                continue;
            }
            // **Red's keep is for red's OWN seat**, and it has to be: since
            // ruling #8 the holder check comes first, so a keep for the still-
            // OPEN seat 1 is "seat 1 is not yours" whoever sends it. Seat 0 is
            // what makes this line a reading of the STATUS arm rather than of
            // the seat fence — which is the whole of what this loop claims.
            let body = if suffix == "/keep" {
                Some(serde_json::json!({ "seat": 0, "card": 1 }))
            } else {
                body
            };
            let uri = format!("/api/games/opent{suffix}");
            let (status, got) = request_as(&state, "red@wire.test", method, &uri, body).await;
            assert_eq!(status, StatusCode::CONFLICT, "{method} {uri}: {got}");
            assert_eq!(got["status"], "open", "{method} {uri}: {got}");
            assert_eq!(got["error"], games::OPEN_SENTENCE, "{method} {uri}: {got}");
        }

        // 4. `join` is admitted at OPEN and refused by STATUS elsewhere (§3.2:
        //    "you are not seated at this table" is explicitly not an arm there —
        //    a stranger is exactly who joins).
        for (table, status_word) in [("dealt", "dealing"), ("lockt", "playing")] {
            let (status, got) = post_as(
                &state,
                STRANGER,
                &format!("/api/games/{table}/join"),
                serde_json::json!({ "seat": 1 }),
            )
            .await;
            assert_eq!(status, StatusCode::CONFLICT, "{table}: {got}");
            assert_eq!(got["status"], status_word, "{table}: {got}");
        }
        // LAST, because it seats the caller and flips the table to dealing.
        let (status, joined) =
            post_as(&state, STRANGER, "/api/games/opent/join", serde_json::json!({ "seat": 1 }))
                .await;
        assert_eq!(status, StatusCode::OK, "{joined}");
        assert_eq!(joined["my_seats"], serde_json::json!([1]), "{joined}");
        assert_eq!(joined["status"], "dealing", "the filling join flips it: {joined}");

        // 5. R-2.6's SUCCESS, which nothing else witnesses: the creator's delete
        //    answers at a table that was open, and the table is then gone.
        let (status, got) = delete_as(&state, "red@wire.test", "/api/games/opent").await;
        assert_eq!(status, StatusCode::NO_CONTENT, "{got}");
        let (status, got) = get_as(&state, "red@wire.test", "/api/games/opent/table").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{got}");

        // Anti-vacuity: a seated caller gets the routes this cell refused.
        // Without it a server that refused everything to everybody would pass.
        for uri in [
            "/api/games/lockt",
            "/api/games/lockt/table",
            "/api/games/lockt/log",
            "/api/games/lockt/at/0",
            "/api/games/lockt/index/orders",
            "/api/games/lockt/events",
            "/api/games/lockt/ports",
        ] {
            let (status, body) = get_as(&state, "blue@wire.test", uri).await;
            assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        }
    }

    /// **A second page is reached by sending the cursor back** (§3.3), over the
    /// route rather than over `Games::list`.
    ///
    /// The route is where the claim lives and where nothing saw it: the lib
    /// cell `tables::tests::the_cursor_pages_without_gap_or_overlap` walks the
    /// pages inside the type, and `GamesList.next` was never once SERIALIZED
    /// non-null — not in a cell and not in the committed corpus — so the
    /// handler's `before` parse had its four refusals covered and its success
    /// path covered nowhere.
    ///
    /// **The two tables are created in the REVERSE of the order they are
    /// expected in, and that is what keeps this cell off the clock.**
    /// `created_at` is unix SECONDS: two creates normally land in one second,
    /// where the id tie-break puts `oa` first, but a pair straddling a second
    /// boundary would be ordered by recency instead. Making `oa` the LATER
    /// create means both rules give the same answer, so the expected order is
    /// fixed however the clock falls. The tie-break itself is pinned, at five
    /// rows in one second, by the cursor cell in `tables.rs`.
    #[tokio::test]
    async fn the_listing_pages_over_the_route_with_a_cursor_the_client_sends_back() {
        let state = Arc::new(AppState::in_memory());
        for id in ["ob", "oa"] {
            let (status, body) = post_as(
                &state,
                "red@wire.test",
                "/api/games",
                serde_json::json!({
                    "players": 2,
                    "colors": ["Red", "Blue"],
                    "start_seat": 0,
                    "seed": "20260831",
                    "seats": [{ "user": "red@wire.test" }, { "open": true }],
                    "id": id,
                    "name": id,
                }),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED, "{id}: {body}");
        }
        // The whole listing first, so the paged walk below is compared against
        // this server's own order rather than against a re-implementation of
        // the sort. Two rows, and the last page of them says so with a null.
        let (status, all) = get_as(&state, "red@wire.test", "/api/games?scope=open").await;
        assert_eq!(status, StatusCode::OK, "{all}");
        assert_eq!(
            all["games"].as_array().expect("an array").iter().map(|g| g["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["oa", "ob"],
            "{all}",
        );
        assert!(all["next"].is_null(), "one page holds both: {all}");

        // PAGE ONE. `next` is non-null here and nowhere else in this codebase.
        let (status, first) =
            get_as(&state, "red@wire.test", "/api/games?scope=open&limit=1").await;
        assert_eq!(status, StatusCode::OK, "{first}");
        assert_eq!(first["games"].as_array().expect("an array").len(), 1, "{first}");
        assert_eq!(first["games"][0]["id"], "oa", "{first}");
        let cursor = first["next"].as_str().expect("a cursor, not null").to_string();
        // Its exact form, because the client sends this string back verbatim
        // and a shape change here is a shape change on the wire.
        let (at, id) = cursor.split_once(',').expect("<created_at>,<id>");
        assert_eq!(
            at.parse::<u64>().expect("a numeric created_at"),
            first["games"][0]["created_at"].as_u64().expect("a created_at"),
            "the cursor names the row this page ended at: {cursor}",
        );
        assert_eq!(id, "oa", "…by its id too: {cursor}");

        // PAGE TWO, from the cursor the server just handed out. The second row
        // arrives, the first does not arrive again, and the walk ends.
        let (status, second) = get_as(
            &state,
            "red@wire.test",
            &format!("/api/games?scope=open&limit=1&before={cursor}"),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{second}");
        assert_eq!(second["games"].as_array().expect("an array").len(), 1, "{second}");
        assert_eq!(second["games"][0]["id"], "ob", "no gap and no overlap: {second}");
        assert!(second["next"].is_null(), "the last page: {second}");
    }

    /// **The listing fences its query and pages it** (§3.3).
    ///
    /// Four refusals and then the one that answers, because the refusals are
    /// what a client meets when it gets a parameter wrong and a silent default
    /// is the failure mode the layer's `parse_query` exists to stop (as `QueryArgs`
    /// did): `?scoped=open` would
    /// otherwise return the caller's OWN tables with a 200 on it.
    #[tokio::test]
    async fn list_games_validates_its_query() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "one", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        for (query, expected) in [
            ("?scope=nope", "no scope named"),
            ("?limit=0", "limit 0"),
            ("?limit=101", "limit 101"),
            ("?before=junk", "before is"),
            ("?before=notanumber,x", "before is"),
            ("?nope=1", "nope"),
        ] {
            let (status, got) = get_as(&state, "red@wire.test", &format!("/api/games{query}")).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {got}");
            assert!(
                got["error"].as_str().unwrap_or_default().contains(expected),
                "{query}: {got}",
            );
        }
        // The default scope is `mine`, and the reply always carries `next` —
        // `null` when there is no second page, which is the shape the client
        // reads rather than a field that appears only sometimes.
        let (status, got) = get_as(&state, "red@wire.test", "/api/games").await;
        assert_eq!(status, StatusCode::OK, "{got}");
        assert_eq!(got["games"].as_array().expect("an array").len(), 1, "{got}");
        assert!(got.get("next").is_some() && got["next"].is_null(), "{got}");
        let (status, got) = get_as(&state, "red@wire.test", "/api/games?scope=open&limit=1").await;
        assert_eq!(status, StatusCode::OK, "{got}");
        assert!(got["games"].as_array().expect("an array").is_empty(), "no open table: {got}");
        assert!(got["next"].is_null(), "{got}");
    }

    /// **The signal answers in four fields at every status, and fences its
    /// query** (room plan R-P9, spec §6).
    ///
    /// One cell over the whole endpoint, because the four fields are ONE
    /// answer and a poll acts on the combination: a reply with `status` right
    /// and `your_turn` wrong leaves a player watching a board it was their turn
    /// to move on.
    ///
    /// **Both statuses on one table, in the order a real client meets them.**
    /// DEALING first, which is the arm that cannot be served from a session —
    /// none exists yet, and a handler reaching for one answers 409 here rather
    /// than the dealing screen's own reading. Then PLAYING, read by BOTH seats,
    /// because `your_turn` is the one field whose value depends on who asked
    /// and one reader cannot tell a correct answer from a constant.
    ///
    /// **Then a move is played, because every reading before it sees `events`
    /// at 0** — and a count asserted only against zero is a count a stub
    /// satisfies. Measured: replacing the length with a literal `0` left this
    /// cell GREEN, and was caught only by the committed corpus, whose
    /// `events.json` happens to sit at 1. That is a different instrument and it
    /// should not be the only one holding the field the client branches on. The
    /// same move proves `your_turn` FOLLOWS the turn rather than naming a fixed
    /// seat, and that the count is the game's and not the caller's.
    ///
    /// **`over` is not asserted here, and is not asserted anywhere yet.**
    /// Reaching it means playing a game to a winner, which this cell would have
    /// to do for one field; Task 10's two-identity arm of the whole-game gate
    /// is where the cycle puts that reading. Until it lands the third `status`
    /// is written and unproven — said plainly, because "the statuses are
    /// covered" is the sentence a reader would otherwise take from the name.
    ///
    /// **`after` is sent, accepted, and not acted on** — this cycle validates
    /// the resume cursor and nothing more (spec §6), so the reply carrying it
    /// is the same reading as the one without. The MISSPELLING is the other
    /// half: `afer` is a 400 rather than a silent default, which is
    /// `deny_unknown_fields` through `QueryArgs` doing its job on this new
    /// struct rather than an assumption that it was wired up.
    #[tokio::test]
    async fn events_answers_in_five_fields_at_every_status_and_fences_its_query() {
        let state = Arc::new(AppState::in_memory());
        let body = serde_json::json!({ "players": 2, "colors": ["Red", "Blue"], "seed": "20260831", "name": "sig", "id": "sig",
            "seats": [{ "user": "red@wire.test" }, { "user": "blue@wire.test" }] });
        post_as(&state, "red@wire.test", "/api/games", body).await;
        let (status, e) = get_as(&state, "blue@wire.test", "/api/games/sig/events").await;
        assert_eq!(status, StatusCode::OK, "{e}");
        assert_eq!(e, serde_json::json!({ "events": 0, "to_move": null, "status": "dealing", "your_turn": false, "chat": 0 }));
        post_as(&state, "red@wire.test", "/api/games/sig/keep", serde_json::json!({ "seat": 0, "card": 1 })).await;
        post_as(&state, "blue@wire.test", "/api/games/sig/keep", serde_json::json!({ "seat": 1, "card": 1 })).await;
        let (_, e) = get_as(&state, "red@wire.test", "/api/games/sig/events?after=0").await;
        assert_eq!(e, serde_json::json!({ "events": 0, "to_move": 0, "status": "playing", "your_turn": true, "chat": 0 }));
        let (_, e) = get_as(&state, "blue@wire.test", "/api/games/sig/events").await;
        assert_eq!(e["your_turn"], false);
        let (status, err) = get_as(&state, "blue@wire.test", "/api/games/sig/events?afer=0").await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{err}");
        // And the cursor's own VALUE, which spec §6 requires to be a
        // non-negative integer. Both of these are refused by the TYPE rather
        // than by a range check in the handler, which is the reason there is no
        // range check in the handler to read.
        for bad in ["after=-1", "after=abc"] {
            let uri = format!("/api/games/sig/events?{bad}");
            let (status, err) = get_as(&state, "blue@wire.test", &uri).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}: {err}");
        }
        // **M6.3 turned this line over** (R-3.5): a signed-in caller with no
        // seat, at an UNLOCKED playing table, is admitted as a watcher and
        // reads the same four fields — never as the one to move. The 403 this
        // line used to assert is now the LOCKED table's, and lives in the
        // whole-table admission cell with the rest of §3.5.
        let (status, watched) =
            get_as(&state, "green@wire.test", "/api/games/sig/events").await;
        assert_eq!(status, StatusCode::OK, "{watched}");
        assert_eq!(watched["your_turn"], false, "a watcher is never to move: {watched}");
        assert_eq!(watched["status"], "playing", "{watched}");

        // A real move, and the reading the poll exists to deliver.
        play_turn(&state, "sig", "red@wire.test").await;
        let (_, e) = get_as(&state, "red@wire.test", "/api/games/sig/events").await;
        let moved = e["events"].as_u64().expect("an event count");
        assert!(moved > 0, "the count never moved off zero: {e}");
        assert_eq!(e["to_move"], serde_json::json!(1), "{e}");
        assert_eq!(e["your_turn"], false, "the turn passed and this seat still owns it: {e}");
        let (_, e) = get_as(&state, "blue@wire.test", "/api/games/sig/events").await;
        assert_eq!(e["your_turn"], true, "the seat to move is not told so: {e}");
        assert_eq!(
            e["events"].as_u64(),
            Some(moved),
            "the count is the GAME's and must read the same to both seats: {e}",
        );
    }

    /// **A move is the acting seat's holder's, and the refusal names whose turn
    /// it is** (R-P11 on the wire: a player who cannot act needs to know why).
    ///
    /// Both directions in one cell: the seat that is NOT to move is refused,
    /// and the seat that IS goes through — so a fence that refused every input
    /// would fail the second half and make the game unplayable.
    #[tokio::test]
    async fn input_is_the_acting_seats_holders_and_names_whose_turn_it_is() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "acting", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        let (status, body) = post_as(
            &state,
            "blue@wire.test",
            "/api/games/acting/input",
            serde_json::json!({ "line": "1" }),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "it is seat 0's turn (red@wire.test)");
        let (status, body) = post_as(
            &state,
            "red@wire.test",
            "/api/games/acting/input",
            serde_json::json!({ "line": "1" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    /// **R-P6: at a table with more than one person, undo reaches back into
    /// your own turn and no further.**
    ///
    /// The walk, in order: nothing to undo yet, which is the underflow guard
    /// answering and not R-P6; mid-turn, where the caller's own move comes
    /// back; once the turn has passed, where R-P6 answers the button that was
    /// pressed rather than letting the acting-seat fence answer a question
    /// nobody asked (R-5.1); the acting seat whose own turn has not started
    /// yet; and the turn come back around.
    ///
    /// **The last read is what the rule exists for.** Without it, red's `undo`
    /// after blue's turn pops blue's move — another person's move, erased by
    /// someone entitled to act but not to rewind.
    ///
    /// **And one ORDINARY line from the same non-acting caller**, so the
    /// reordering is shown not to have loosened the fence: `"1"` from red while
    /// blue is to move still hears whose turn it is.
    #[tokio::test]
    async fn undo_is_admitted_mid_turn_and_refused_once_the_turn_has_passed() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "undo", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        // One event of red's turn, then undo it: admitted, and the log shrinks by one.
        let (_, before) = get_as(&state, "red@wire.test", "/api/games/undo").await;
        let n0 = before["events"].as_u64().unwrap();
        assert_eq!(n0, 0, "a fresh game, so the next post hits the empty-record guard");
        let (status, body) = post_as(
            &state,
            "red@wire.test",
            "/api/games/undo/input",
            serde_json::json!({ "line": "undo" }),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "nothing to undo", "the guard, not R-P6");
        let line = first_option(&before);
        let (_, after) = post_as(
            &state,
            "red@wire.test",
            "/api/games/undo/input",
            serde_json::json!({ "line": line }),
        )
        .await;
        assert!(after["events"].as_u64().unwrap() > n0, "the option applied: {after}");
        assert_eq!(
            after["to_move"], 0,
            "and the turn is still red's, so the undo below is mid-turn",
        );
        let (status, undone) = post_as(
            &state,
            "red@wire.test",
            "/api/games/undo/input",
            serde_json::json!({ "line": "undo" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{undone}");
        assert_eq!(undone["events"].as_u64().unwrap(), n0);

        // Play the turn out; blue is to move. Red's undo is refused, and what
        // answers is R-P6 — the trailing move IS red's, so the first half
        // passes and the current-player half is what speaks.
        play_turn(&state, "undo", "red@wire.test").await;
        let (status, body) = post_as(
            &state,
            "red@wire.test",
            "/api/games/undo/input",
            serde_json::json!({ "line": "undo" }),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "undo is not offered once your turn has passed");

        // The same caller, at the same moment, sending an ordinary line: the
        // acting-seat fence is exactly where it was, and putting R-P6 in front
        // of it moved nothing else.
        let (status, body) = post_as(
            &state,
            "red@wire.test",
            "/api/games/undo/input",
            serde_json::json!({ "line": "1" }),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "it is seat 1's turn (blue@wire.test)");

        // Blue IS the acting seat and has not moved yet, so blue's undo would
        // cross red's turn boundary. This is R-P6's own refusal.
        let (status, body) = post_as(
            &state,
            "blue@wire.test",
            "/api/games/undo/input",
            serde_json::json!({ "line": "undo" }),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "undo is not offered once your turn has passed");

        // And when the turn comes back around, red is acting again — and is
        // still refused, because the trailing move is blue's.
        play_turn(&state, "undo", "blue@wire.test").await;
        let (_, now) = get_as(&state, "red@wire.test", "/api/games/undo").await;
        assert_eq!(now["to_move"], 0, "red is acting again: {now}");
        let (status, body) = post_as(
            &state,
            "red@wire.test",
            "/api/games/undo/input",
            serde_json::json!({ "line": "undo" }),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "undo is not offered once your turn has passed");
    }

    /// **The other half of R-P6: one person at the table keeps today's undo**,
    /// bot run and all.
    ///
    /// The rule is about not erasing another PERSON's move, so where there is
    /// no other person there is nothing to fence — and fencing anyway would
    /// break the only mode this server has shipped so far, since the bot's run
    /// is the trailing event and its mover is never the caller.
    #[tokio::test]
    async fn undo_at_a_table_with_one_person_unwinds_the_bot_run_as_before() {
        let state = Arc::new(AppState::in_memory());
        start_vs_bot(&state, "undo-bot", 20260830, "red@wire.test").await;
        // Play red's whole turn; the bot then moves inside the same reply.
        let after = play_turn_vs_bot(&state, "undo-bot", "red@wire.test", "Blue").await;
        assert_eq!(after["to_move"], 0, "the bot has played and it is red again: {after}");
        let n = after["events"].as_u64().unwrap();
        let (status, undone) = post_as(
            &state,
            "red@wire.test",
            "/api/games/undo-bot/input",
            serde_json::json!({ "line": "undo" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{undone}");
        assert!(
            undone["events"].as_u64().unwrap() < n,
            "the trailing bot run and red's move are gone: {undone}",
        );
        let (_, log) = get_as(&state, "red@wire.test", "/api/games/undo-bot/log?tail=1").await;
        assert!(
            log["entries"][0]["text"].as_str().is_none_or(|t| !t.starts_with("Blue")),
            "the bot's run is what was unwound: {log}",
        );
    }

    /// **`privacy` is refused at every table** (M6.1 §5.4) — asked from the
    /// ACTING seat both times, so nothing but the grammar can be what refused
    /// it, and at both table shapes, because the vs-bot table is the one where
    /// a reader might expect the old hot-seat toggle to have survived.
    #[tokio::test]
    async fn privacy_is_refused_at_every_table() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "priv", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        start_vs_bot(&state, "priv-bot", 20260830, "red@wire.test").await;
        for id in ["priv", "priv-bot"] {
            let (status, body) = post_as(
                &state,
                "red@wire.test",
                &format!("/api/games/{id}/input"),
                serde_json::json!({ "line": "privacy" }),
            )
            .await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{id}: {body}");
            assert!(
                body["error"].as_str().unwrap().contains("no privacy toggle"),
                "{id}: {body}",
            );
        }
    }

    /// **R-1.7 on the wire: a payload for one seat never carries another's
    /// cards**, asserted from BOTH seats of the same game so a server that
    /// unmasked seat 0 for everyone could not pass.
    #[tokio::test]
    async fn a_payload_for_one_seat_never_carries_another_seats_cards() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "mask", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        for (user, mine) in [("red@wire.test", 0u64), ("blue@wire.test", 1)] {
            let (status, game) = get_as(&state, user, "/api/games/mask").await;
            assert_eq!(status, StatusCode::OK, "{game}");
            assert_eq!(game["seat"], mine);
            let panels = game["frame"]["dashboard"].as_array().expect("panels");
            let players =
                panels.iter().find_map(|p| p.get("Players")).expect("a Players panel");
            let rows = players["players"].as_array().expect("a row per seat");
            assert_eq!(rows.len(), 2);
            for p in rows {
                assert_eq!(p["hand_hidden"], p["seat"] != mine, "{user}: {p}");
            }
            let hands: Vec<&serde_json::Value> =
                panels.iter().filter_map(|p| p.get("Hand")).collect();
            assert_eq!(hands.len(), 1, "{user}: one hand panel, and it is the viewer's");
            assert_eq!(hands[0]["seat"], mine);
            assert_eq!(hands[0]["hidden"], false);
            assert!(
                hands[0]["cards"].as_array().is_some_and(|c| !c.is_empty()),
                "anti-vacuity: the viewer's own hand really is open: {}",
                hands[0],
            );
        }
        let (_, as_blue) = get_as(&state, "blue@wire.test", "/api/games/mask").await;
        assert_eq!(as_blue["your_turn"], false);
        assert_eq!(as_blue["prompt"]["kind"], "watching");
    }

    /// **The replay pin is PER CALLER, and it holds across a turn boundary.**
    ///
    /// M4's sweep cell asserted that one sweep of `0..=total` harvests one
    /// seat's hands; its sibling asserted the pin did not move when the turn
    /// did. Both are this one cell now, and it is strictly stronger than the
    /// pair: the sweep is taken by EACH of two callers over a record that spans
    /// a turn change, and each must see only its own. A follow-the-mover
    /// implementation fails it in both directions at once.
    #[tokio::test]
    async fn a_replay_sweep_as_each_caller_shows_only_that_callers_hands() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "sweep", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        // Red's whole turn, so the sweep below spans a seat boundary.
        play_turn(&state, "sweep", "red@wire.test").await;
        let (_, game) = get_as(&state, "blue@wire.test", "/api/games/sweep").await;
        let total = game["events"].as_u64().expect("an event count") as usize;
        assert!(total > 0, "the walk applied nothing, so the sweep has one stop");
        assert_eq!(game["to_move"], 1, "the turn really did pass");

        for (user, mine) in [("red@wire.test", 0u64), ("blue@wire.test", 1)] {
            let mut seen = std::collections::BTreeSet::new();
            let mut open = 0usize;
            for k in 0..=total {
                let (status, frame) =
                    get_as(&state, user, &format!("/api/games/sweep/at/{k}")).await;
                assert_eq!(status, StatusCode::OK, "{user} at {k}: {frame}");
                assert_eq!(frame["seat"], mine, "{user} at {k}");
                for h in frame["frame"]["dashboard"]
                    .as_array()
                    .expect("panels")
                    .iter()
                    .filter_map(|p| p.get("Hand"))
                {
                    seen.insert(h["seat"].as_u64().expect("a seat"));
                    if h["cards"].as_array().is_some_and(|c| !c.is_empty()) {
                        open += 1;
                    }
                }
            }
            assert_eq!(
                seen,
                std::collections::BTreeSet::from([mine]),
                "{user}: one seat's hands across the sweep",
            );
            assert!(open > 0, "{user}: the sweep unmasked nothing, so it shows nothing");
        }
    }

    /// The two-person seed every finished-game cell below deals from.
    const ENDED_SEED: u64 = 20260831;

    /// A two-person table whose record has been played out to a WINNER, with
    /// the id and the email of the seat that is NOT to move at the end.
    ///
    /// **Built by pumping both seats as bots in process and installing the
    /// record** — the shape the lockdown cells use — because driving a 2P human
    /// table to a winner over the route costs a whole game. The table document
    /// still describes two PEOPLE, which is what the cells are about; only the
    /// record's own run provenance says bot, and no route reads it for these
    /// claims.
    async fn a_finished_two_person_game(
        name: &'static str,
    ) -> (Arc<AppState>, &'static str, &'static str) {
        use yokohama_cli::bot::{BotSeats, PROVISIONAL_DEFAULT_BOT};
        use yokohama_cli::session::Session;
        use yokohama_core::{GameConfig, PlayerColor, PlayerId, StartingOrderChoice};

        let store = Arc::new(MemStore::default());
        let state = Arc::new(AppState::with_store(store.clone(), identity::IdentityMode::Claimed));
        start_as(&state, name, ENDED_SEED, &["red@wire.test", "blue@wire.test"]).await;

        // The very game the table describes: same config, same seed, same keeps
        // (`start_as` keeps card 1 at every seat, which is `Keep(0)`).
        let mut config = GameConfig::for_player_count(2);
        config.player_colors = vec![PlayerColor::Red, PlayerColor::Blue];
        config.start_player = PlayerId::new(0);
        let mut session = Session::new_sim(
            config,
            ENDED_SEED,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            Rules::standard(),
        )
        .expect("a game");
        let bots = BotSeats::new(
            vec![Some(PROVISIONAL_DEFAULT_BOT), Some(PROVISIONAL_DEFAULT_BOT)],
            ENDED_SEED,
        );
        session.set_run_seats(bots.seat_provenance());
        let mut repl = yokohama_cli::repl::Repl::new(session);
        for _ in 0..400 {
            if repl.session.is_over() {
                break;
            }
            repl.pump_bots();
        }
        assert!(repl.session.is_over(), "the both-bot pump must reach a finished game");
        let finished = repl.session.to_record_json().expect("a record");
        let (_, version) = store.read(Doc::Record, name).expect("a store").expect("a record");
        store
            .write_if_version(Doc::Record, name, &finished, Some(version))
            .expect("the finished record");

        // A fresh state over the same store, so the game is rebuilt from the
        // record just written rather than served from the resident handle.
        let state = Arc::new(AppState::with_store(store, identity::IdentityMode::Claimed));
        let (_, game) = get_as(&state, "red@wire.test", &format!("/api/games/{name}")).await;
        let watcher = if game["to_move"].as_u64() == Some(0) {
            "blue@wire.test"
        } else {
            "red@wire.test"
        };
        (state, name, watcher)
    }

    /// **A finished or faulted game answers EVERY caller with what is true of
    /// the GAME, not with whose turn it is** (review I1).
    ///
    /// The defect: at game over a non-acting caller was told "it is seat 0's
    /// turn (red@…)" on a game where it is nobody's turn, while the holder of
    /// that same seat was told the truth. The fence's ordering was right and its
    /// WORDING was a lie — and this crate's standard is that a refusal names
    /// what to do next.
    ///
    /// **Both arms are exercised, because they are two different orderings.**
    /// `input` folds "over" and "locked" into one condition ahead of the seat
    /// fence, so either arm proves it; `suggest` has a four-rung ladder where
    /// `Locked` and `GameOver` are separate rungs, so a locked-only cell would
    /// leave the game-over rung unexecuted. The finished game is built by
    /// pumping both seats as bots in process and installing the record — the
    /// same shape the lockdown cells use, because a 2P game driven to a winner
    /// over the route costs a whole game.
    ///
    /// The claim is an EQUALITY between callers rather than a fixed sentence:
    /// whatever the acting seat's holder is told, the watcher is told too.
    #[tokio::test]
    async fn a_finished_or_locked_game_refuses_every_caller_the_same_way() {
        let store = Arc::new(MemStore::default());
        let state = Arc::new(AppState::with_store(store.clone(), identity::IdentityMode::Claimed));
        start_as(&state, "faultedseat", ENDED_SEED, &["red@wire.test", "blue@wire.test"]).await;

        // A LOCKED record, stamped through the durable record exactly as a
        // driver fault reaches a later process.
        let (stored, version) =
            store.read(Doc::Record, "faultedseat").expect("a store").expect("a record");
        let mut record: serde_json::Value = serde_json::from_str(&stored).expect("a record");
        record["lockdown"] = serde_json::json!("invariant violation applying that move");
        store
            .write_if_version(
                Doc::Record,
                "faultedseat",
                &serde_json::to_string(&record).expect("a record"),
                Some(version),
            )
            .expect("a stored lockdown");

        // A fresh state over the same store, so the locked game is rebuilt from
        // the record just written rather than served from the resident handle.
        let locked = Arc::new(AppState::with_store(store, identity::IdentityMode::Claimed));
        let (ended, ended_id, _) = a_finished_two_person_game("ended").await;

        for (state, id, kind) in
            [(&ended, ended_id, "game_over"), (&locked, "faultedseat", "locked")]
        {
            let (_, game) = get_as(state, "red@wire.test", &format!("/api/games/{id}")).await;
            let acting = game["to_move"].as_u64().expect("a seat to move");
            let (holder, watcher) = if acting == 0 {
                ("red@wire.test", "blue@wire.test")
            } else {
                ("blue@wire.test", "red@wire.test")
            };

            let (hs, hb) = get_as(state, holder, &format!("/api/games/{id}/suggest")).await;
            let (ws, wb) = get_as(state, watcher, &format!("/api/games/{id}/suggest")).await;
            assert_eq!(hb["refusal"]["kind"], kind, "{id}: the holder is told the truth: {hb}");
            assert_eq!((ws, &wb), (hs, &hb), "{id}: and the watcher is told the same thing");

            // **`input` is compared on the REFUSAL, not on the whole body**:
            // the reply is a per-seat payload, so two callers legitimately
            // receive different frames — their own hands. What must match is
            // the status and what the session said, which is the sentence the
            // seat fence was drowning out.
            let line = serde_json::json!({ "line": "1" });
            let (hs, hb) =
                post_as(state, holder, &format!("/api/games/{id}/input"), line.clone()).await;
            let (ws, wb) = post_as(state, watcher, &format!("/api/games/{id}/input"), line).await;
            assert_ne!(
                wb["error"].as_str(),
                Some("it is seat 0's turn (red@wire.test)"),
                "{id}: input must not name a turn on a game that has none: {wb}",
            );
            assert!(
                !hb["messages"].as_array().expect("messages").is_empty(),
                "{id}: anti-vacuity — the session really did refuse the holder, in words: {hb}",
            );
            assert_eq!(hs, StatusCode::OK, "{id}: the acting holder's own path is unchanged");
            assert_eq!(ws, StatusCode::FORBIDDEN, "{id}: and the watcher still cannot act: {wb}");
            assert_ne!(
                wb["error"].as_str(),
                Some("it is seat 0's turn (red@wire.test)"),
                "{id}: input must not name a turn on a game that has none: {wb}",
            );
        }
    }

    /// **A watcher cannot REWIND a finished game, and the fence that stops them
    /// is the same fence at every game status** (controller ruling R-4.3).
    ///
    /// The hole this closes was opened by the first fix for I1, which is why it
    /// gets its own cell rather than a line in the one above. Dropping the seat
    /// fence at game over so the watcher could hear the game's own sentence let
    /// their POST reach `Repl::step` — and `undo` is an allowlisted gameplay
    /// line that `Session::undo` applies with no game-over guard of its own. Any
    /// seated player could rewind a completed shared game, and the server would
    /// persist it, because `events.len()` had changed.
    ///
    /// So the fence never drops; only its WORDING changes. What a non-acting
    /// caller hears at game over is the CLI's own `ApplyError::GameOver`
    /// sentence, and at lockdown the lockdown's own reason.
    ///
    /// **The anti-mutation assertion is the point of the cell**, not the status
    /// code: `events` is read before and after, through a GET as the acting
    /// holder, and must not have moved. A 403 that still rewound the game would
    /// pass a status check.
    #[tokio::test]
    async fn a_watcher_cannot_undo_a_finished_game() {
        let (state, id, watcher) = a_finished_two_person_game("rewind").await;
        let (_, before) = get_as(&state, "red@wire.test", &format!("/api/games/{id}")).await;
        let events = before["events"].as_u64().expect("an event count");
        assert!(events > 0 && before["over"] == serde_json::json!(true), "{before}");

        let (status, body) = post_as(
            &state,
            watcher,
            &format!("/api/games/{id}/input"),
            serde_json::json!({ "line": "undo" }),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(
            body["error"], "the game is already over",
            "the CLI's own ApplyError::GameOver sentence, not one authored here",
        );

        let (_, after) = get_as(&state, "red@wire.test", &format!("/api/games/{id}")).await;
        assert_eq!(
            after["events"].as_u64(),
            Some(events),
            "the watcher's undo rewound a finished shared game",
        );
    }

    /// **A faulted game says so to an `undo`, whoever sends it** — R-4.3's
    /// precedence applied to R-P6's refusal, and the reason the sentence is
    /// single-sourced.
    ///
    /// Both seated callers, because they are refused for different reasons and
    /// must hear the same thing. The WATCHER is refused on the merits — the
    /// trailing move is not theirs — and the HOLDER is not: they are the acting
    /// seat and the trailing move IS theirs, so before the status arm their
    /// `undo` was admitted, reached `Repl::step`, and came back 200 with a
    /// refusal in prose. Two callers, one faulted game, two different answers,
    /// neither of them saying plainly that the game cannot be played at all.
    ///
    /// The lockdown is also the one status where the two fences could still
    /// drift: at a FINISHED game `undo_allowed` refuses on its own (R-5.2), so
    /// a `status_refusal` that read only `is_over` would pass every other cell
    /// in this file.
    ///
    /// **The event count is read back**, as the finished-game siblings do: a
    /// 403 that still rewound a faulted record would pass a status check, and a
    /// faulted record is the one thing in this server that exists to be read
    /// afterwards.
    #[tokio::test]
    async fn a_locked_game_answers_every_callers_undo_with_the_lockdown_reason() {
        let store = Arc::new(MemStore::default());
        let state = Arc::new(AppState::with_store(store.clone(), identity::IdentityMode::Claimed));
        start_as(&state, "faultedundo", ENDED_SEED, &["red@wire.test", "blue@wire.test"]).await;

        // One move first, so the refusal being overridden is R-P6's own and not
        // the empty-record guard's, and so the count read back is not zero.
        let (_, game) = get_as(&state, "red@wire.test", "/api/games/faultedundo").await;
        let (status, _) = post_as(
            &state,
            "red@wire.test",
            "/api/games/faultedundo/input",
            serde_json::json!({ "line": first_option(&game) }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (stored, version) =
            store.read(Doc::Record, "faultedundo").expect("a store").expect("a record");
        let mut record: serde_json::Value = serde_json::from_str(&stored).expect("a record");
        record["lockdown"] = serde_json::json!("invariant violation applying that move");
        store
            .write_if_version(
                Doc::Record,
                "faultedundo",
                &serde_json::to_string(&record).expect("a record"),
                Some(version),
            )
            .expect("a stored lockdown");

        // A fresh state over the same store, so the game is rebuilt FROM the
        // record just written rather than served from the resident handle.
        let locked = Arc::new(AppState::with_store(store, identity::IdentityMode::Claimed));
        let (_, game) = get_as(&locked, "red@wire.test", "/api/games/faultedundo").await;
        assert_eq!(game["prompt"]["kind"], "locked", "anti-vacuity — really faulted: {game}");
        assert_eq!(game["over"], serde_json::json!(false), "locked, not finished: {game}");
        let acting = game["to_move"].as_u64().expect("a seat to move");
        let events = game["events"].as_u64().expect("an event count");
        assert!(events > 0, "a move to undo: {game}");
        let (holder, watcher) = if acting == 0 {
            ("red@wire.test", "blue@wire.test")
        } else {
            ("blue@wire.test", "red@wire.test")
        };

        for who in [holder, watcher] {
            let (status, body) = post_as(
                &locked,
                who,
                "/api/games/faultedundo/input",
                serde_json::json!({ "line": "undo" }),
            )
            .await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{who}: {body}");
            assert_eq!(
                body["error"], "invariant violation applying that move",
                "{who}: the fault's own reason, not a sentence about turn order",
            );
        }

        let (_, after) = get_as(&locked, "red@wire.test", "/api/games/faultedundo").await;
        assert_eq!(
            after["events"].as_u64(),
            Some(events),
            "an undo rewound a faulted game, which is the record a lockdown exists to preserve",
        );
    }

    /// **A finished shared game cannot be rewound by ANYBODY, the seat that is
    /// to move included** (controller ruling R-5.2 on R-P6's reading).
    ///
    /// The sibling above closed this for a WATCHER, and the fence's own comment
    /// left the acting holder to R-P6 — which does not settle it in words: "only
    /// while the caller's own turn has not fully completed" can be read as
    /// leaving the turn a game ended during. Ruled: a finished game has no turn
    /// in progress for ANYONE, so nobody's turn is incomplete, and one player
    /// reopening a game the other has watched end is what R-P6 exists to stop.
    ///
    /// **Measured before the rule, on this fixture**: the acting holder's
    /// `undo` returned 200 and the record went from 357 events to 200 — there,
    /// because the fixture's seats are bot-provenanced, by unwinding a "bot run"
    /// and re-pumping into a `BOT HALT`; at a human-seated table by popping the
    /// caller's own trailing move. Both are a finished shared game changing
    /// under a player who had already seen it end.
    ///
    /// The sentence is the game's own status, not R-P6's: `status_refusal` is
    /// single-sourced with the acting-seat fence, so the two cannot drift. And
    /// the event count is read back through a second GET, because a 403 that
    /// still rewound the game would pass a status check.
    ///
    /// **One person at the table keeps today's CLI behaviour** — R-P6's first
    /// sentence — and that arm is covered by
    /// `undo_at_a_table_with_one_person_unwinds_the_bot_run_as_before`.
    #[tokio::test]
    async fn the_acting_holder_cannot_undo_a_finished_shared_game() {
        let (state, id, _watcher) = a_finished_two_person_game("endedundo").await;
        let (_, game) = get_as(&state, "red@wire.test", &format!("/api/games/{id}")).await;
        let acting = game["to_move"].as_u64().expect("a seat to move");
        let holder = if acting == 0 { "red@wire.test" } else { "blue@wire.test" };
        let events = game["events"].as_u64().expect("an event count");
        assert!(events > 0 && game["over"] == serde_json::json!(true), "{game}");

        let (status, body) = post_as(
            &state,
            holder,
            &format!("/api/games/{id}/input"),
            serde_json::json!({ "line": "undo" }),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(
            body["error"], "the game is already over",
            "the game's own status, not R-P6's sentence",
        );

        let (_, after) = get_as(&state, "red@wire.test", &format!("/api/games/{id}")).await;
        assert_eq!(
            after["events"].as_u64(),
            Some(events),
            "the acting holder's undo rewound a finished shared game",
        );
    }

    /// **The card index is a VIEW OF THE CALLER's cards, and a watcher's index
    /// names none of the mover's** (spec §5.3's viewer column, R-4.1).
    ///
    /// The defect this closes, measured: `Repl::index_placements` marks
    /// `CardLocation::OwnHand` for the seat the TERMINAL is showing, which there
    /// is always the mover. Passed through unscoped, a seated opponent issued
    /// one GET and read which order cards were in the acting seat's hand — and
    /// since every other order card is taken face-up from a port display,
    /// subtracting the public takes names that seat's hidden starting order, the
    /// single thing hidden player-to-player in the engine's representation.
    ///
    /// **Three assertions, and none of the three is enough alone.** The
    /// watcher's index must not mark the mover's cards (the leak); it MUST mark
    /// the watcher's own (or the fix is "drop `OwnHand` entirely", which breaks
    /// the index for the person it is for); and the mover's own index must still
    /// mark the mover's (the same, from the other side). The card ids come from
    /// each seat's own dashboard rather than from a list written here.
    #[tokio::test]
    async fn a_watchers_card_index_never_names_the_acting_seats_hand() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "indexed", 20260831, &["red@wire.test", "blue@wire.test"]).await;

        /// The order ids a payload shows in its own Hand panel.
        fn hand_of(game: &serde_json::Value) -> Vec<u64> {
            game["frame"]["dashboard"]
                .as_array()
                .expect("panels")
                .iter()
                .find_map(|p| p.get("Hand"))
                .expect("a hand panel")["cards"]
                .as_array()
                .expect("cards")
                .iter()
                .map(|c| c["id"].as_u64().expect("a card id"))
                .collect()
        }
        /// The order ids an index marks as the reader's own hand.
        ///
        /// `CardLocation` is externally tagged and `OwnHand` is a unit variant,
        /// so it serializes as the bare string; the id lives under the card's
        /// `face`, which is `CardRef`'s own tagging.
        fn own_hand(index: &serde_json::Value) -> std::collections::BTreeSet<u64> {
            index["groups"]
                .as_array()
                .expect("groups")
                .iter()
                .flat_map(|g| g["cards"].as_array().expect("cards"))
                .filter(|c| c["location"] == serde_json::json!("OwnHand"))
                .map(|c| c["face"]["Order"]["id"].as_u64().expect("an order card id"))
                .collect()
        }

        let (_, as_red) = get_as(&state, "red@wire.test", "/api/games/indexed").await;
        let (_, as_blue) = get_as(&state, "blue@wire.test", "/api/games/indexed").await;
        assert_eq!(as_red["to_move"], 0, "red is the acting seat");
        let (red_hand, blue_hand) = (hand_of(&as_red), hand_of(&as_blue));
        assert!(!red_hand.is_empty() && !blue_hand.is_empty(), "the premise: both hold a card");
        assert_ne!(red_hand, blue_hand, "and they are not the same card");

        let (status, watcher) =
            get_as(&state, "blue@wire.test", "/api/games/indexed/index/orders").await;
        assert_eq!(status, StatusCode::OK, "{watcher}");
        let marked = own_hand(&watcher);
        for id in &red_hand {
            assert!(!marked.contains(id), "the watcher's index names O{id}, which is red's");
        }
        assert_eq!(
            marked,
            blue_hand.iter().copied().collect(),
            "and it names blue's own, which is what the index is for",
        );

        let (_, mover) = get_as(&state, "red@wire.test", "/api/games/indexed/index/orders").await;
        assert_eq!(
            own_hand(&mover),
            red_hand.iter().copied().collect(),
            "the acting seat's own index still marks its own hand",
        );
    }

    // ---------------------------------------------------------------------
    // M6.3 — the spectator (spec §3.5, §4, R-3.5–R-3.7)
    // ---------------------------------------------------------------------

    /// What kind of answer a route owes, in §3.5's terms.
    ///
    /// The grid below is a function of this and the caller's row, and a route
    /// added to [`session_routes`] without a kind here fails the coverage
    /// assertion at the top of the cell — the same shape that list already
    /// enforces one layer up.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Kind {
        /// A projection: 200 to anyone admitted, 409 to a seat holder before
        /// the game exists.
        Read,
        /// `GET …/table` — R-2.7's widening plus R-3.13's.
        Table,
        /// `GET …/events`. `…/stream` is asserted to answer the same status in
        /// every cell, which is §3.5's last sentence.
        Signal,
        /// [`Kind::Read`] plus its own 409 until the game is over.
        Result,
        /// Refused to a caller with no seat, spectator or not, before the
        /// handle is touched.
        Write,
        /// `join` and `unlock`, which keep their own status ladders (R-4.2 and
        /// §3.4) and are asserted by hand below.
        OwnLadder,
        /// `GET …/chat` (M6.4): 200 to a seat holder at EVERY status and to a
        /// watcher; 403 otherwise, with the lock sentence where the lock is
        /// what shuts them out.
        Chat,
        /// `POST …/chat` (M6.4): the same admission and a 201 — the one write a
        /// watcher may make (R-6.4.11).
        ChatPost,
    }

    /// Every session route with the answer it owes. The bodies are
    /// [`session_routes`]', and the coverage of that list is asserted rather
    /// than assumed.
    fn admission_routes() -> Vec<(&'static str, &'static str, Option<serde_json::Value>, Kind)> {
        vec![
            ("GET", "", None, Kind::Read),
            ("GET", "/table", None, Kind::Table),
            ("GET", "/deal", None, Kind::Write),
            ("POST", "/keep", Some(serde_json::json!({ "seat": 1, "card": 1 })), Kind::Write),
            ("POST", "/join", Some(serde_json::json!({ "seat": 1 })), Kind::OwnLadder),
            ("POST", "/unlock", Some(serde_json::json!({ "password": "x" })), Kind::OwnLadder),
            ("POST", "/input", Some(serde_json::json!({ "line": "1" })), Kind::Write),
            ("GET", "/ports", None, Kind::Read),
            ("GET", "/labs", None, Kind::Read),
            ("GET", "/areas", None, Kind::Read),
            ("GET", "/log", None, Kind::Read),
            ("GET", "/events", None, Kind::Signal),
            ("GET", "/at/0", None, Kind::Read),
            // R-6.5b.10: the run route is admitted exactly as `at/{k}` is.
            ("GET", "/at?from=0&to=1", None, Kind::Read),
            ("GET", "/suggest", None, Kind::Write),
            ("GET", "/why", None, Kind::Read),
            ("GET", "/index/orders", None, Kind::Read),
            ("GET", "/result", None, Kind::Result),
            ("GET", "/chat", None, Kind::Chat),
            ("POST", "/chat", Some(serde_json::json!({ "text": "grid" })), Kind::ChatPost),
            ("DELETE", "", None, Kind::Write),
        ]
    }

    /// The status of `GET …/stream` WITHOUT reading its body.
    ///
    /// The router answers before the body streams, so the status is available
    /// at once — and reading an admitted stream's body here would hang, since
    /// it stays open until the table ends. That is why this is a separate
    /// helper from [`stream_frames`] and not a zero-duration call to it.
    async fn stream_status(state: &Arc<AppState>, uri: &str, who: &str) -> StatusCode {
        let res = app(Arc::clone(state))
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header(identity::CLAIMED_HEADER, who)
                    .header("accept", "text/event-stream")
                    .body(Body::empty())
                    .expect("a GET"),
            )
            .await
            .expect("the router answers");
        res.status()
    }

    /// The corpus-seed 2P game PLAYED OUT by both seats as bots, as a record.
    ///
    /// [`a_finished_two_person_game`]'s mechanism, lifted so several tables in
    /// ONE cell can be finished: a 2P game driven to a winner over the route
    /// costs a whole game, and this cell needs two finished tables.
    fn a_finished_record() -> String {
        use yokohama_cli::bot::{BotSeats, PROVISIONAL_DEFAULT_BOT};
        use yokohama_cli::session::Session;
        use yokohama_core::{GameConfig, PlayerColor, PlayerId, StartingOrderChoice};

        let mut config = GameConfig::for_player_count(2);
        config.player_colors = vec![PlayerColor::Red, PlayerColor::Blue];
        config.start_player = PlayerId::new(0);
        let mut session = Session::new_sim(
            config,
            ENDED_SEED,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            Rules::standard(),
        )
        .expect("a game");
        let bots = BotSeats::new(
            vec![Some(PROVISIONAL_DEFAULT_BOT), Some(PROVISIONAL_DEFAULT_BOT)],
            ENDED_SEED,
        );
        session.set_run_seats(bots.seat_provenance());
        let mut repl = yokohama_cli::repl::Repl::new(session);
        for _ in 0..400 {
            if repl.session.is_over() {
                break;
            }
            repl.pump_bots();
        }
        assert!(repl.session.is_over(), "the both-bot pump must reach a finished game");
        repl.session.to_record_json().expect("a record")
    }

    /// **Spec §3.5's whole grid, route by route** — five caller rows against
    /// four statuses, with `stream` asserted to answer exactly what `events`
    /// answers in every one of them.
    ///
    /// One cell and not twenty, for the reason the M6.2 sweep beside it gives:
    /// an admission applied to sixteen routes and forgotten on the
    /// seventeenth is the shape that ships. The five rows are
    ///
    /// | caller | status | what they are |
    /// |---|---|---|
    /// | red | any | holds a seat |
    /// | green | open | signed in, no seat |
    /// | green | dealing | signed in, no seat |
    /// | grey | playing / over, unlocked or presented | a WATCHER |
    /// | green | playing / over, locked | shut out until they knock |
    ///
    /// and they are covered by three identities against six tables, because
    /// "playing" and "over" are two readings of one status and a locked table
    /// is a different document from an unlocked one.
    ///
    /// **The seat holder's WRITE column is not asserted here** and says so:
    /// those answers are M6.2's, unchanged by this cycle and pinned by its own
    /// cells — and one of them is a DELETE that would take a table out from
    /// under the rest of the grid. What this cell claims about the writes is
    /// the row that IS new: a caller with no seat is refused every one of them,
    /// watcher or not.
    ///
    /// **Row 2's signal column is the one place `events` and `admitted` part
    /// company, and it is deliberate** (§3.5 row 2, controller ruling R-4.3):
    /// `events` and `stream` answer ANY signed-in caller at an OPEN table,
    /// while every other read there is `admitted`'s 403. The joiner who
    /// arrived by `#/t/<id>` is on the open-table screen watching for the seats
    /// to fill, and a refusal would end that stream and bounce them to the
    /// lobby — so [`events_at`] reads the table itself at `open` and falls
    /// through to `admitted` at the other two statuses.
    #[tokio::test]
    async fn the_whole_admission_table_is_answered_route_by_route() {
        const NOT_SEATED: &str = "you are not seated at this table";
        const LOCKED: &str = "this table is locked — unlock it first";
        let (red, green, grey) = ("red@wire.test", "green@wire.test", "grey@wire.test");

        // Coverage: every route the stranger sweep knows has a kind here.
        let kinds = admission_routes();
        for (method, suffix, _) in session_routes() {
            assert!(
                kinds.iter().any(|(m, s, _, _)| *m == method && *s == suffix),
                "{method} {suffix} has no admission kind",
            );
        }

        let store = Arc::new(MemStore::default());
        let warm = Arc::new(AppState::with_store(Arc::clone(&store) as Arc<dyn Store>, identity::IdentityMode::Claimed));
        let open_body = serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "20260831",
            "seats": [{ "user": red }, { "open": true }], "id": "opent", "name": "opent",
        });
        let (status, body) = post_as(&warm, red, "/api/games", open_body).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let (status, body) = post_as(
            &warm,
            red,
            "/api/games",
            serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "20260831",
                "seats": [{ "user": red }, { "user": "blue@wire.test" }],
                "id": "dealt", "name": "dealt",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        start_as(&warm, "playt", ENDED_SEED, &[red, "blue@wire.test"]).await;
        start_locked_as(&warm, "lockt", ENDED_SEED, &[red, "blue@wire.test"], "sesame").await;
        start_as(&warm, "overt", ENDED_SEED, &[red, "blue@wire.test"]).await;
        start_locked_as(&warm, "overlk", ENDED_SEED, &[red, "blue@wire.test"], "sesame").await;

        // The two finished tables, by installing the played-out record under
        // the id `start_as` just created — the same game, at the same seed.
        let finished = a_finished_record();
        for id in ["overt", "overlk"] {
            let (_, version) = store.read(Doc::Record, id).expect("a store").expect("a record");
            store
                .write_if_version(Doc::Record, id, &finished, Some(version))
                .expect("the finished record");
        }
        // A fresh state over the same store, so the two finished records are
        // rebuilt from disk rather than served from `warm`'s resident handles.
        let state = Arc::new(AppState::with_store(store, identity::IdentityMode::Claimed));
        for id in ["overt", "overlk"] {
            let (status, game) = get_as(&state, red, &format!("/api/games/{id}")).await;
            assert_eq!(status, StatusCode::OK, "{game}");
            assert_eq!(game["over"], true, "the premise: {id} is finished");
        }

        // Grey knocks at both locked tables, which is the fourth row.
        for id in ["lockt", "overlk"] {
            let (status, view) = post_as(
                &state,
                grey,
                &format!("/api/games/{id}/unlock"),
                serde_json::json!({ "password": "sesame" }),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{id}: {view}");
            assert_eq!(view["can_watch"], true, "{id}: {view}");
        }

        // (table, status word, whether a lock is on it)
        let tables = [
            ("opent", "open", false),
            ("dealt", "dealing", false),
            ("playt", "playing", false),
            ("lockt", "playing", true),
            ("overt", "over", false),
            ("overlk", "over", true),
        ];
        // (who, holds a seat, has presented the password)
        let callers = [(red, true, false), (green, false, false), (grey, false, true)];

        for (table, word, locked) in tables {
            let playing = word == "playing" || word == "over";
            for (who, seated, knocked) in callers {
                // The fourth row's condition, which is `TableDoc::can_watch`'s
                // in the caller's own terms: no seat, playing, and either the
                // table has no lock or this caller has knocked.
                let watches = !seated && playing && (!locked || knocked);
                let shut_out = !seated && playing && locked && !knocked;

                for (method, suffix, body, kind) in admission_routes() {
                    if kind == Kind::OwnLadder {
                        continue;
                    }
                    if seated && kind == Kind::Write {
                        continue; // M6.2's, and a DELETE would end the grid.
                    }
                    let uri = format!("/api/games/{table}{suffix}");
                    let (got, answer) = request_as(&state, who, method, &uri, body).await;
                    let want = match kind {
                        Kind::Table => {
                            if seated || playing || word == "open" {
                                StatusCode::OK
                            } else {
                                StatusCode::FORBIDDEN
                            }
                        }
                        Kind::Write if !seated => StatusCode::FORBIDDEN,
                        Kind::Write => unreachable!("the seated write column is skipped"),
                        // **The signal answers at EVERY status** (R-3.3): the
                        // open-table screen and the setup screen poll the same
                        // endpoint the game does, and a 409 there would make
                        // each of them branch on an error.
                        //
                        // **And at OPEN it answers ANY signed-in caller**
                        // (§3.5 row 2, controller ruling R-4.3) — the same
                        // widening `table` has there since R-2.7, and for the
                        // same reason: the person who followed a join link is
                        // exactly who needs to learn when the seats fill, and a
                        // 403 would end their stream and bounce them to the
                        // lobby. It stops at `open`: a stranger at `dealing` is
                        // still refused, because nobody watches a deal.
                        Kind::Signal if seated || watches || word == "open" => StatusCode::OK,
                        Kind::Signal => StatusCode::FORBIDDEN,
                        // **The chat is admitted exactly as a READ is, at every
                        // status** (M6.4, spec §3.2): a seat holder always, a
                        // watcher once the table admits them, and nobody else.
                        // It never reaches the 409 arms below, because a
                        // message needs no session — a table that is still
                        // filling its seats is exactly where the people at it
                        // are talking.
                        Kind::Chat if seated || watches => StatusCode::OK,
                        Kind::ChatPost if seated || watches => StatusCode::CREATED,
                        Kind::Chat | Kind::ChatPost => StatusCode::FORBIDDEN,
                        _ if !seated && !watches => StatusCode::FORBIDDEN,
                        // A projection needs a session, and there is none
                        // before the last keep. Only a seat holder reaches
                        // here at those two statuses.
                        _ if !playing => StatusCode::CONFLICT,
                        Kind::Result if word != "over" => StatusCode::CONFLICT,
                        _ => StatusCode::OK,
                    };
                    assert_eq!(
                        got, want,
                        "{who} at {table} ({word}): {method} {uri} answered {answer}",
                    );
                    if got == StatusCode::FORBIDDEN && kind != Kind::Write {
                        // The SENTENCE discriminates the two refused rows: a
                        // caller who can do something about it is told what.
                        let expected = if shut_out { LOCKED } else { NOT_SEATED };
                        assert_eq!(answer["error"], expected, "{who} at {table}: {method} {uri}");
                    }
                    if kind == Kind::Signal {
                        let streamed = stream_status(
                            &state,
                            &format!("/api/games/{table}/stream"),
                            who,
                        )
                        .await;
                        assert_eq!(
                            streamed, got,
                            "{who} at {table}: stream and events must be admitted alike",
                        );
                    }
                    // **The chat's two halves are admitted alike** (spec §3.2).
                    // A post is a WRITE and this is the one write a caller with
                    // no seat may make, so the tempting mistake is to fence it
                    // like the others — which would leave a watcher reading a
                    // room they cannot answer in.
                    if kind == Kind::ChatPost {
                        let (read, _) = request_as(&state, who, "GET", &uri, None).await;
                        let expect =
                            if got == StatusCode::CREATED { StatusCode::OK } else { got };
                        assert_eq!(
                            read, expect,
                            "{who} at {table}: the chat GET and POST are admitted alike",
                        );
                    }
                }
            }
        }

        // Anti-vacuity: the watcher's 200s really carry a watcher's payload,
        // and the seat holder's really carry a seat's — a grid of matching
        // status codes could otherwise be a server that served one thing.
        let (_, watched) = get_as(&state, grey, "/api/games/playt").await;
        assert_eq!(watched["seat"], serde_json::Value::Null, "{watched}");
        assert_eq!(watched["your_turn"], false, "{watched}");
        assert_eq!(watched["prompt"]["kind"], "watching", "{watched}");
        let (_, mine) = get_as(&state, red, "/api/games/playt").await;
        assert_eq!(mine["seat"], 0, "{mine}");
        assert_eq!(mine["your_turn"], true, "{mine}");

        // And `join` and `unlock`'s own ladders at the two rows this cell is
        // about, which is all §3.5 says of them (R-4.2, §3.4).
        for (table, word) in [("dealt", "dealing"), ("playt", "playing")] {
            let (status, got) = post_as(
                &state,
                green,
                &format!("/api/games/{table}/join"),
                serde_json::json!({ "seat": 1 }),
            )
            .await;
            assert_eq!(status, StatusCode::CONFLICT, "{table}: {got}");
            assert_eq!(got["status"], word, "{table}: {got}");
        }
        let (status, got) = post_as(
            &state,
            green,
            "/api/games/lockt/unlock",
            serde_json::json!({ "password": "open" }),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{got}");
        assert_eq!(got["error"], "wrong password for this table", "{got}");
        let (status, got) = post_as(
            &state,
            green,
            "/api/games/opent/unlock",
            serde_json::json!({ "password": "sesame" }),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{got}");
        assert_eq!(got["status"], "open", "nothing to watch yet: {got}");
    }

    /// **A watcher writes nothing, and is refused BEFORE the handle is
    /// touched** (R-3.6).
    ///
    /// The instrument is a store that PANICS on any record read: the state is
    /// built fresh over it, so no handle is resident, and any write route that
    /// reached `Games::handle` would take the test down with a named message
    /// rather than quietly refusing one line later. A 403 that arrives after
    /// the record has been read and the bot pump has run is not the refusal
    /// R-3.6 asks for.
    ///
    /// At a PLAYING and at a FINISHED table, because `input`'s fence chooses
    /// its sentence by status and a spectator must never reach the choosing.
    #[tokio::test]
    async fn a_spectator_is_refused_every_write_before_the_record_is_ever_read() {
        /// A [`MemStore`] that refuses to be asked for a record.
        struct NoRecords(MemStore);
        impl Store for NoRecords {
            fn list(&self, doc: Doc) -> Result<Vec<crate::store::GameId>, String> {
                self.0.list(doc)
            }
            fn read(
                &self,
                doc: Doc,
                id: &str,
            ) -> Result<Option<(String, crate::store::Version)>, String> {
                assert_ne!(
                    doc,
                    Doc::Record,
                    "a spectator's write reached the record of {id:?} before refusing",
                );
                self.0.read(doc, id)
            }
            fn write_if_version(
                &self,
                doc: Doc,
                id: &str,
                json: &str,
                expected: Option<crate::store::Version>,
            ) -> Result<crate::store::Version, crate::store::WriteError> {
                self.0.write_if_version(doc, id, json, expected)
            }
            fn delete(&self, id: &str) -> Result<(), String> {
                self.0.delete(id)
            }
        }

        let plain = Arc::new(MemStore::default());
        let warm = Arc::new(AppState::with_store(
            Arc::clone(&plain) as Arc<dyn Store>,
            identity::IdentityMode::Claimed,
        ));
        start_as(&warm, "playt", ENDED_SEED, &["red@wire.test", "blue@wire.test"]).await;
        start_as(&warm, "overt", ENDED_SEED, &["red@wire.test", "blue@wire.test"]).await;
        let finished = a_finished_record();
        let (_, version) = plain.read(Doc::Record, "overt").expect("a store").expect("a record");
        plain
            .write_if_version(Doc::Record, "overt", &finished, Some(version))
            .expect("the finished record");

        // The SAME bytes, behind a store that will not hand out a record — and
        // a fresh state, so nothing is resident to answer from memory.
        let guarded = NoRecords(MemStore::default());
        for doc in [Doc::Table, Doc::Record] {
            for id in plain.list(doc).expect("a store") {
                let (json, _) = plain.read(doc, &id).expect("a store").expect("a document");
                guarded.0.write_if_version(doc, &id, &json, None).expect("a copy");
            }
        }
        let state = Arc::new(AppState::with_store(
            Arc::new(guarded) as Arc<dyn Store>,
            identity::IdentityMode::Claimed,
        ));

        // Grey watches both — an unlocked table admits without a knock — and
        // is still refused every write.
        for table in ["playt", "overt"] {
            let (status, view) = get_as(&state, "grey@wire.test", &format!("/api/games/{table}/table")).await;
            assert_eq!(status, StatusCode::OK, "{view}");
            assert_eq!(view["can_watch"], true, "the premise: grey may watch {table}");

            for (method, suffix, body, kind) in admission_routes() {
                if kind != Kind::Write {
                    continue;
                }
                let uri = format!("/api/games/{table}{suffix}");
                let (status, got) =
                    request_as(&state, "grey@wire.test", method, &uri, body).await;
                assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri}: {got}");
            }
            // **The ONE write a watcher may make, and it reads no record
            // either** (R-6.4.11): behind the same panicking store, grey's post
            // lands. The instrument is what makes that a claim rather than a
            // hope — a chat append that reached for the session to resolve the
            // poster would take this cell down by name.
            let (status, m) = post_as(
                &state,
                "grey@wire.test",
                &format!("/api/games/{table}/chat"),
                serde_json::json!({ "text": "from the rail" }),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED, "{table}: {m}");
            assert_eq!(m["author"]["kind"], "watcher");
            // And the table is untouched: still there, still playing.
            let (status, view) =
                get_as(&state, "red@wire.test", &format!("/api/games/{table}/table")).await;
            assert_eq!(status, StatusCode::OK, "the refused DELETE deleted nothing: {view}");
        }
    }

    /// **A spectator's card index marks no hand at all** (R-3.8), by value over
    /// a game where both seats hold cards.
    ///
    /// The anti-vacuity half is the seated read beside it: an index that had
    /// simply stopped marking anything would pass the first assertion.
    #[tokio::test]
    async fn a_spectators_card_index_marks_nobodys_hand() {
        /// The order ids an index marks as the reader's own hand.
        fn own_hand(index: &serde_json::Value) -> std::collections::BTreeSet<u64> {
            index["groups"]
                .as_array()
                .expect("groups")
                .iter()
                .flat_map(|g| g["cards"].as_array().expect("cards"))
                .filter(|c| c["location"] == serde_json::json!("OwnHand"))
                .map(|c| c["face"]["Order"]["id"].as_u64().expect("an order card id"))
                .collect()
        }

        let state = Arc::new(AppState::in_memory());
        start_as(&state, "indexed", 20260831, &["red@wire.test", "blue@wire.test"]).await;

        let (status, seated) =
            get_as(&state, "red@wire.test", "/api/games/indexed/index/orders").await;
        assert_eq!(status, StatusCode::OK, "{seated}");
        assert!(!own_hand(&seated).is_empty(), "the premise: the acting seat holds cards");

        let (status, watched) =
            get_as(&state, "grey@wire.test", "/api/games/indexed/index/orders").await;
        assert_eq!(status, StatusCode::OK, "{watched}");
        assert!(own_hand(&watched).is_empty(), "a watcher's index names no hand: {watched}");

        // The rest of the index is the SAME public reading — the port slots,
        // the laboratory slots, the sites. Dropping the hand must not drop
        // those, which a `None` arm returning early would.
        let public = |index: &serde_json::Value| -> usize {
            index["groups"]
                .as_array()
                .expect("groups")
                .iter()
                .flat_map(|g| g["cards"].as_array().expect("cards"))
                .filter(|c| c["location"] != serde_json::json!("OwnHand"))
                .filter(|c| c["location"] != serde_json::Value::Null)
                .count()
        };
        assert_eq!(public(&watched), public(&seated), "the public rows are unchanged");
        assert!(public(&watched) > 0, "and there are some");
    }

    /// **`why` reads the same to a watcher as to a seated caller.**
    ///
    /// The route already masks the roads not taken for EVERY caller
    /// (`HandVisibility::Masked`), so admitting a spectator to it adds no
    /// visibility — and the claim is exactly that: equal JSON, not merely two
    /// 200s. A vs-bot table, because `why` accounts for a BOT move and a table
    /// of two people has none.
    #[tokio::test]
    async fn why_reads_the_same_to_a_watcher_as_to_a_seated_caller() {
        let state = Arc::new(AppState::in_memory());
        start_vs_bot(&state, "whywatch", 20260831, "red@wire.test").await;
        play_turn_vs_bot(&state, "whywatch", "red@wire.test", "Blue").await;

        let (status, seated) = get_as(&state, "red@wire.test", "/api/games/whywatch/why").await;
        assert_eq!(status, StatusCode::OK, "{seated}");
        assert!(seated["refusal"].is_null(), "the premise: a bot move to account for: {seated}");
        let (status, watched) = get_as(&state, "grey@wire.test", "/api/games/whywatch/why").await;
        assert_eq!(status, StatusCode::OK, "{watched}");
        assert_eq!(watched, seated, "the account is the same one");
    }

    /// **`suggest` answers the acting seat's holder only** (R-1.8), so the
    /// advisor never ranks a move for a seat the caller cannot play.
    #[tokio::test]
    async fn suggest_answers_the_acting_seats_holder_only() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "advise", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        let (status, body) = get_as(&state, "blue@wire.test", "/api/games/advise/suggest").await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "it is not your turn");
        let (status, body) = get_as(&state, "red@wire.test", "/api/games/advise/suggest").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["ok"]["seat"], 0);
    }

    /// **The whole create flow over the router, as two people drive it.**
    ///
    /// The cell this cycle exists for: a create opens a DEALING table, each seat
    /// is dealt its own two cards, a keep is refused on a seat the caller does
    /// not hold, every session route answers 409 with a status the client can
    /// branch on until the last keep, and the list is scoped to the caller.
    ///
    /// One cell rather than eight, because the SEQUENCE is the claim — each
    /// refusal is stated against a table that a moment later answers the same
    /// call correctly, so a fence that refused everything could not pass.
    #[tokio::test]
    async fn the_create_flow_deals_per_seat_and_the_state_is_409_until_the_last_keep() {
        let state = Arc::new(AppState::in_memory());
        let body = serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "20260831", "name": "flow", "id": "flow",
            "seats": [{ "user": "red@wire.test" }, { "user": "blue@wire.test" }],
        });
        let (status, table) = post_as(&state, "red@wire.test", "/api/games", body).await;
        assert_eq!(status, StatusCode::CREATED, "{table}");
        assert_eq!(table["status"], "dealing");
        assert!(!table.to_string().contains("20260831"), "no seed on the wire: {table}");
        let (status, body) = get_as(&state, "red@wire.test", "/api/games/flow").await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body, serde_json::json!({ "error": "this table is still dealing", "status": "dealing" }));
        let (status, _) = get_as(&state, "red@wire.test", "/api/games/flow/ports").await;
        assert_eq!(status, StatusCode::CONFLICT, "every session route");
        let (status, deal) = get_as(&state, "blue@wire.test", "/api/games/flow/deal").await;
        assert_eq!(status, StatusCode::OK, "{deal}");
        assert_eq!(deal["seat"], 1);
        assert_eq!(deal["cards"].as_array().unwrap().len(), 2);
        let (status, err) = post_as(
            &state,
            "blue@wire.test",
            "/api/games/flow/keep",
            serde_json::json!({ "seat": 0, "card": 1 }),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{err}");
        let (status, _) = post_as(
            &state,
            "blue@wire.test",
            "/api/games/flow/keep",
            serde_json::json!({ "seat": 1, "card": 1 }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, err) = get_as(&state, "green@wire.test", "/api/games/flow/deal").await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{err}");
        let (status, view) = post_as(
            &state,
            "red@wire.test",
            "/api/games/flow/keep",
            serde_json::json!({ "seat": 0, "card": 2 }),
        )
        .await;
        assert_eq!((status, view["status"].as_str()), (StatusCode::OK, Some("playing")), "{view}");
        let (status, game) = get_as(&state, "red@wire.test", "/api/games/flow").await;
        assert_eq!(status, StatusCode::OK, "{game}");
        assert_eq!(game["events"], 0);
        let (status, list) = get_as(&state, "blue@wire.test", "/api/games").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(list["games"][0]["my_seats"], serde_json::json!([1]));
        let (status, _) = get_as(&state, "green@wire.test", "/api/games").await;
        assert_eq!(status, StatusCode::OK);
    }

    /// The OLD create body is refused by name, and delete is the creator's.
    ///
    /// `keeps` and `bot_seats` are gone (M6.1 §4.2), and the existing
    /// `deny_unknown_fields` fence is what turns a client that never updated
    /// into a 400 naming the key rather than a game dealt for somebody else.
    #[tokio::test]
    async fn the_old_body_keys_are_refused_by_name_and_delete_is_the_creators() {
        let state = Arc::new(AppState::in_memory());
        // One at a time, because each must NAME ITSELF: a body carrying both is
        // refused for whichever serde meets first, and the other key's cell
        // would be riding on that one.
        for key in ["keeps", "bot_seats"] {
            let mut old = serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
                "seats": [{ "user": "red@wire.test" }, { "bot": true }],
            });
            old[key] = serde_json::json!([1]);
            let (status, err) = post_as(&state, "red@wire.test", "/api/games", old).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{key}: {err}");
            assert!(err["error"].as_str().unwrap().contains(key), "{key}: {err}");
        }
        let body = serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "name": "own", "id": "own",
            "seats": [{ "user": "red@wire.test" }, { "user": "blue@wire.test" }],
        });
        post_as(&state, "red@wire.test", "/api/games", body).await;
        let (status, _) = delete_as(&state, "blue@wire.test", "/api/games/own").await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, _) = delete_as(&state, "red@wire.test", "/api/games/own").await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    /// **The only destructive route in the product, which nothing exercised.**
    ///
    /// `DELETE /api/games/{id}` permanently removes a game, and no test at any
    /// level issued one — not a route test, not a component cell, not the
    /// browser walk. `Games::delete`'s store-level behaviour is covered
    /// (`games.rs`), but the ROUTE was not: repointing `api.remove` at another
    /// path, or returning 204 without calling `state.games.delete`, left every
    /// suite green. On a lane whose CI runs only `cargo test -p yokohama-core`,
    /// that is a delete button nothing anywhere checks.
    #[tokio::test]
    async fn a_deleted_game_is_gone_from_the_list_and_from_the_store() {
        let router = app(Arc::new(AppState::in_memory()));
        a_game(&router, "doomed", &[]).await;
        a_game(&router, "kept", &[]).await;

        let ids = |list: &serde_json::Value| {
            list["games"]
                .as_array()
                .expect("a games list")
                .iter()
                .map(|g| g["id"].as_str().expect("an id").to_string())
                .collect::<Vec<_>>()
        };
        let (_, before) = call(&router, "GET", "/api/games", None).await;
        let mut names = ids(&before);
        names.sort();
        assert_eq!(names, ["doomed", "kept"], "the premise: both are stored");

        let (status, _) = call(&router, "DELETE", "/api/games/doomed", None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);

        // Gone from the list, gone from the store, and the OTHER game survives —
        // without that last clause a handler that deleted everything would pass.
        let (_, after) = call(&router, "GET", "/api/games", None).await;
        assert_eq!(ids(&after), ["kept"], "only the named game went");
        let (status, _) = call(&router, "GET", "/api/games/doomed", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "the record is really gone");
        let (status, _) = call(&router, "GET", "/api/games/kept", None).await;
        assert_eq!(status, StatusCode::OK, "and its neighbour is untouched");

        // Deleting what is not there is a 404, not a silent success.
        let (status, _) = call(&router, "DELETE", "/api/games/doomed", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "a second delete says so");
    }

    /// **The explanation FOOTS, and that is the whole claim of the `why` screen.**
    ///
    /// `Explanation`'s own documented identity is
    /// `total == sum(terms.product) + terminal.value`. A chart the reader cannot
    /// foot is a picture rather than an explanation, so the sum is asserted here
    /// and re-asserted on every committed fixture.
    ///
    /// **What this doc used to say and should not have:** that the identity
    /// "holds only over ALL 34 terms — which is why the payload carries the
    /// zero-product ones". It does not. A zero product contributes zero, so the
    /// carrying terms alone foot — measured on `web/src/wire/suggest.json`: 34
    /// terms, 8 nonzero, both sums 78 == `total`. The real reason all 34 ship is
    /// `Explanation`'s, and it is about the READER rather than the arithmetic:
    /// a term reported at zero can be SEEN to have contributed nothing, where an
    /// absent one leaves you wondering whether it was computed. That is what
    /// makes the client's "8 of 34" a count of something it is showing.
    #[tokio::test]
    async fn suggest_carries_the_bots_own_arithmetic_and_the_sum_foots() {
        let router = live_router();
        a_game(&router, "advises", &[]).await;
        let (status, body) = call(&router, "GET", "/api/games/advises/suggest", None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["refusal"].is_null(), "a human seat is to move: {body}");
        let ok = &body["ok"];

        let terms = ok["explanation"]["terms"].as_array().expect("terms");
        assert_eq!(terms.len(), 34, "every feature is reported, including the zero ones");
        let sum: i64 = terms.iter().map(|t| t["product"].as_i64().expect("i64")).sum();
        let terminal = ok["explanation"]["terminal"]["value"].as_i64().unwrap_or(0);
        assert_eq!(
            sum + terminal,
            ok["explanation"]["total"].as_i64().expect("a total"),
            "the exact-sum identity holds on the wire"
        );

        assert!(!ok["text"].as_str().unwrap_or_default().is_empty(), "the move is worded");
        let ranking = ok["ranking"].as_array().expect("a ranking");
        assert!(!ranking.is_empty(), "a search reports its candidates");
        assert_eq!(ranking[0]["margin"], 0, "the best candidate's margin against itself is 0");
        assert!(
            ranking.windows(2).all(|w| w[0]["value"].as_i64() >= w[1]["value"].as_i64()),
            "the ranking is best first"
        );
        assert!(ok["stream_seed"].is_string(), "a u64 is a STRING on this wire");

        // **Exactly one row is the chosen move, and it is not necessarily row
        // 0.** Ties at the top are broken by a keyed draw — `tied_at_top`
        // exists to say so — so a client that took row 0 as the answer would
        // list the played move among the ones it beat. Measured on the
        // committed fixture: two candidates tie and the chosen one is row 1.
        let chosen: Vec<&serde_json::Value> =
            ranking.iter().filter(|r| r["chosen"] == serde_json::Value::Bool(true)).collect();
        assert_eq!(chosen.len(), 1, "exactly one candidate is the one that was played");
        assert_eq!(chosen[0]["text"], ok["text"], "and it is the move the payload names");
        assert_eq!(chosen[0]["margin"], 0, "the chosen move is always a maximiser");
        // `nonzero` is the COUNT of terms carrying a product, re-derived here
        // from the terms themselves. An earlier version asserted `<= 34`, which
        // is true of every value a `usize` over a 34-element vector can hold —
        // a reviewer set the server's count to a constant 0 and every lib test
        // stayed green.
        assert_eq!(
            ok["explanation"]["nonzero"].as_u64().expect("a count"),
            terms.iter().filter(|t| t["product"].as_i64() != Some(0)).count() as u64,
            "the nonzero count is a count of the terms that carry one"
        );
    }

    /// A `suggest` press changes NOTHING — it applies, records and autosaves
    /// nothing, which is the terminal's own contract and what makes GET honest
    /// about the effect. Both halves are asserted: the state is unmoved, and two
    /// presses agree, which is the idempotence the CLI guarantees by keying the
    /// stream to the game identity, the seat and the move count.
    #[tokio::test]
    async fn a_suggest_press_moves_nothing_and_repeats_itself() {
        let router = live_router();
        a_game(&router, "inert", &[]).await;
        let (_, before) = call(&router, "GET", "/api/games/inert", None).await;
        let (_, first) = call(&router, "GET", "/api/games/inert/suggest", None).await;
        let (_, second) = call(&router, "GET", "/api/games/inert/suggest", None).await;
        let (_, after) = call(&router, "GET", "/api/games/inert", None).await;
        assert_eq!(first, second, "the same press answers the same way");
        assert_eq!(before, after, "nothing was applied, recorded or autosaved");
    }

    /// **A refusal is the ANSWER, so it is a 200 carrying data** — not a 4xx and
    /// not an empty body. A bot-free game has no bot move to account for, and
    /// the browser renders that sentence rather than a status code.
    #[tokio::test]
    async fn why_refuses_a_game_with_no_bot_move_and_says_so_as_data() {
        let router = live_router();
        a_game(&router, "nobots", &[]).await;
        let (status, body) = call(&router, "GET", "/api/games/nobots/why", None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["ok"].is_null());
        assert_eq!(body["refusal"]["kind"], "no_bot_move_yet");
        assert!(
            body["refusal"]["message"].as_str().unwrap_or_default().contains("bot"),
            "the CLI's own sentence: {body}"
        );
    }

    /// The refusals `why` raises are the GATE's, and three different ones are
    /// reachable from one real game — which is what makes them worth serving as
    /// typed data instead of one catch-all.
    #[tokio::test]
    async fn why_raises_the_gates_own_refusals_by_their_own_names() {
        let router = live_router();
        a_game(&router, "gated", &[1]).await;

        let (_, out) = call(&router, "GET", "/api/games/gated/why?event=99999", None).await;
        assert_eq!(out["refusal"]["kind"], "out_of_range", "{out}");

        // Play the human's opening move, so there IS an event 0 and it is a
        // human's. `why` accounts for BOT moves; being asked about a human's is
        // a refusal with its own name rather than a silent empty answer.
        let (status, _) = call(
            &router,
            "POST",
            "/api/games/gated/input",
            Some(serde_json::json!({ "line": "1", "expect": "options" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (_, human) = call(&router, "GET", "/api/games/gated/why?event=0", None).await;
        assert_eq!(
            human["refusal"]["kind"], "human_seat",
            "event 0 is now the human's own move: {human}"
        );

        // And the query itself is fenced, like every other request on this API.
        let (status, bad) = call(&router, "GET", "/api/games/gated/why?evnt=3", None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
        assert!(bad["error"].as_str().unwrap_or_default().contains("evnt"));
    }

    /// **`why` accounts for a move a bot actually played, and says under what.**
    ///
    /// The provenance is not decoration: an account with no claim attached is
    /// arithmetic about nothing. The reconstruction is `account_parts`' — this
    /// crate re-derives no part of the honesty gate, so a move that did not
    /// reproduce would have refused before reaching here.
    #[tokio::test]
    async fn why_accounts_for_a_bot_move_under_a_named_claim() {
        let router = live_router();
        // Seat 0 is the bot AND the start seat, so `create`'s `pump_bots` plays
        // a whole bot turn before anything else happens.
        a_game(&router, "explains", &[0]).await;
        let (status, body) = call(&router, "GET", "/api/games/explains/why", None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["refusal"].is_null(), "a bot moved: {body}");
        let ok = &body["ok"];

        let p = &ok["provenance"];
        assert!(!p["contestant"].as_str().unwrap_or_default().is_empty(), "names the contestant");
        assert_eq!(p["seat"], 0);
        assert!(
            p["decision_seed"].as_str().unwrap_or_default().starts_with("0x"),
            "the pump's own key, as a string: {p}"
        );
        assert!(p["evaluator_version"].as_u64().is_some());

        let terms = ok["explanation"]["terms"].as_array().expect("terms");
        let sum: i64 = terms.iter().map(|t| t["product"].as_i64().expect("i64")).sum();
        let terminal = ok["explanation"]["terminal"]["value"].as_i64().unwrap_or(0);
        assert_eq!(sum + terminal, ok["explanation"]["total"].as_i64().expect("a total"));

        let counters = ok["counters"].as_array().expect("a searching contestant reports counters");
        assert_eq!(counters.len(), 15, "the whole counter surface, not a chosen two of it");
        assert!(counters.iter().all(|c| c["group"] != "ungrouped"), "{counters:?}");
    }

    /// **A locked session refuses a SEARCH and still serves a PROJECTION.**
    ///
    /// The rule the three route families follow, pinned in both directions
    /// because a gate applied to one of three siblings is the shape a reviewer
    /// found: `suggest` and `why` re-run the bot against a record the driver has
    /// declared faulty, so both refuse; the card indexes and the final score
    /// read the state rather than an opinion about it, so they answer — which is
    /// what M2 already settled for `ports`/`labs`/`areas`/`log`.
    ///
    /// The lockdown is stamped through the DURABLE record and then served from a
    /// fresh `AppState`, because that is how a real driver fault reaches a later
    /// process: `Repl::new` restores it on load.
    ///
    /// Of `suggest`'s four guards this pins `Locked`. `GameOver` is reachable
    /// but costs a whole played game to reach, `BotSeat` never fires at rest
    /// (both mutating paths pump bot seats before returning) and `LiveMode` is
    /// unreachable through this API entirely — all three stated in
    /// `SuggestRefusal`'s doc rather than claimed as tested.
    #[tokio::test]
    async fn a_locked_session_refuses_the_searches_and_still_serves_the_views() {
        let store = Arc::new(MemStore::default());
        let router = app(Arc::new(AppState::with_store(
            store.clone(),
            identity::IdentityMode::Claimed,
        )));
        a_game(&router, "faulted", &[]).await;

        let (stored, version) =
            store.read(Doc::Record, "faulted").expect("a stored game").expect("a stored game");
        let mut record: serde_json::Value = serde_json::from_str(&stored).expect("a record");
        record["lockdown"] = serde_json::json!("invariant violation applying that move");
        store
            .write_if_version(
                Doc::Record,
                "faulted",
                &serde_json::to_string(&record).expect("a record"),
                Some(version),
            )
            .expect("a stored lockdown");
        let router =
            app(Arc::new(AppState::with_store(store, identity::IdentityMode::Claimed)));

        let (status, state) = call(&router, "GET", "/api/games/faulted", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(state["prompt"]["kind"], "locked", "the projection still answers: {state}");

        let (status, s) = call(&router, "GET", "/api/games/faulted/suggest", None).await;
        assert_eq!(status, StatusCode::OK, "a refusal is the answer, not an error");
        assert_eq!(s["refusal"]["kind"], "locked", "{s}");

        let (status, w) = call(&router, "GET", "/api/games/faulted/why", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(w["refusal"]["kind"], "locked", "{w}");
        assert!(w["ok"].is_null(), "and no account is served beside it");

        let (status, ix) = call(&router, "GET", "/api/games/faulted/index/orders", None).await;
        assert_eq!(status, StatusCode::OK, "an index is a projection: {ix}");
        assert_eq!(ix["total"], 36);
    }

    /// **The four card indexes are the CLI's own, served as VIEWS.**
    ///
    /// `target: None` is the load-bearing difference: in the terminal an index
    /// can be open inside a card capture and its cards then carry selection
    /// numbers. This API has no capture — the input grammar refuses `?`, `index`
    /// and the registry words precisely so a digit cannot silently change
    /// meaning — so an index here is read-only and numbers nothing.
    #[tokio::test]
    async fn the_four_card_indexes_are_served_as_views_and_a_fifth_is_refused() {
        let router = live_router();
        a_game(&router, "indexed", &[]).await;
        for (kind, least) in [("orders", 36), ("techs", 20), ("goals", 3), ("sites", 24)] {
            let (status, body) =
                call(&router, "GET", &format!("/api/games/indexed/index/{kind}"), None).await;
            assert_eq!(status, StatusCode::OK, "{kind}: {body}");
            assert_eq!(body["total"], least, "{kind} is over its whole population");
            assert_eq!(body["shown"], least, "and unfiltered it shows all of it");
            assert!(body["target"].is_null(), "{kind} is a view, so it numbers nothing");
            assert!(!body["groups"].as_array().expect("groups").is_empty(), "{kind} has groups");
        }
        let (status, body) = call(&router, "GET", "/api/games/indexed/index/nope", None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            body["error"].as_str().unwrap_or_default().contains("orders, techs, goals and sites"),
            "the refusal names what it would have taken: {body}"
        );
    }

    /// The filter is the CLI's own grammar, refusals included.
    ///
    /// A second parser in this crate would answer an ambiguous prefix
    /// differently from the terminal on the first `f s` a player typed — which
    /// is the exact defect `card_prompt`'s "prefixes refuse ambiguity rather
    /// than resolving it" rule exists to close. So the refusal here IS the
    /// CLI's, and a narrowed index really is narrower.
    #[tokio::test]
    async fn an_index_filter_narrows_it_and_an_ambiguous_one_is_refused_not_guessed() {
        let router = live_router();
        a_game(&router, "filtered", &[]).await;
        let (_, all) = call(&router, "GET", "/api/games/filtered/index/orders", None).await;

        let (status, narrowed) =
            call(&router, "GET", "/api/games/filtered/index/orders?filter=copper", None).await;
        assert_eq!(status, StatusCode::OK, "{narrowed}");
        assert_eq!(narrowed["total"], all["total"], "the population is unchanged");
        assert!(
            narrowed["shown"].as_u64().unwrap() < all["shown"].as_u64().unwrap(),
            "a filter shows fewer: {} vs {}",
            narrowed["shown"],
            all["shown"]
        );
        assert!(!narrowed["filter"].is_null(), "and the payload says what narrowed it");

        let (status, bad) =
            call(&router, "GET", "/api/games/filtered/index/orders?filter=zzzz", None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
        assert!(bad["error"].is_string(), "carrying the CLI's own sentence: {bad}");
    }

    /// The final score is a 409 until there IS one — not an empty payload and
    /// not a standing nobody has earned.
    #[tokio::test]
    async fn the_result_is_refused_while_the_game_is_still_running() {
        let router = live_router();
        a_game(&router, "unfinished", &[]).await;
        let (status, body) = call(&router, "GET", "/api/games/unfinished/result", None).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert!(body["error"].as_str().unwrap_or_default().contains("not over"));
    }

    /// Health names the two keys that actually decide compatibility, and names
    /// them from the ENGINE rather than from a constant kept here — a copy
    /// would go stale exactly when it mattered.
    #[tokio::test]
    async fn health_reports_the_engines_own_compatibility_keys() {
        let (status, body) = get_json("/api/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["semantics_rev"], SEMANTICS_REV);
        assert_eq!(body["rules_hash"], Rules::standard().content_hash());
        assert_eq!(body["server_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(body["identity"], "claimed");
    }

    /// An answer to a prompt the game is no longer asking is REFUSED, not
    /// applied against whatever it is asking instead.
    ///
    /// The failure this closes: `SubPrompt` lives only in the resident `Repl`,
    /// so a restart or an eviction rebuilds with `SubPrompt::None` while the
    /// client — which never re-fetches — keeps sending area ids from an open
    /// placement picker. Measured across two processes over one store: a client
    /// holding `place_a` with areas 3 and 7 selected clicked area 3, and the
    /// rebuilt `Repl` applied option 3, "Place no assistants", spending the
    /// turn's placement on nothing the player chose.
    ///
    /// Both directions are asserted: the stale answer is a 409 that names both
    /// prompts, and the CORRECT tag still goes through — a guard that refused
    /// everything would pass the first half of this test and make the game
    /// unplayable.
    #[tokio::test]
    async fn an_answer_to_a_prompt_the_game_is_no_longer_asking_is_refused() {
        let router = app(Arc::new(AppState::in_memory()));
        let call = |uri: String, body: serde_json::Value| {
            let router = router.clone();
            async move {
                let res = router
                    .oneshot(
                        Request::builder()
                            .method("POST")
                            .uri(uri)
                            .header("content-type", "application/json")
                            .header(identity::CLAIMED_HEADER, WHO)
                            .body(Body::from(serde_json::to_vec(&body).expect("a body")))
                            .expect("a request"),
                    )
                    .await
                    .expect("the router answers");
                let status = res.status();
                let bytes =
                    axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
                (status, serde_json::from_slice::<serde_json::Value>(&bytes).expect("JSON"))
            }
        };

        let state = start(&router, "expects", 2, &[1]).await;
        assert_eq!(state["prompt"]["kind"], "options", "the opening asks for an option");

        // The shape a dropped picker produces: the client believes a placement
        // picker is open and sends an area id; the server is back on the main
        // list, where that id would be an option NUMBER.
        let (status, body) = call(
            "/api/games/expects/input".to_string(),
            serde_json::json!({ "line": "3", "expect": "place_a" }),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        let msg = body["error"].as_str().unwrap_or_default();
        assert!(msg.contains("place_a") && msg.contains("options"), "names both prompts: {msg}");

        // Nothing was applied by the refusal.
        let (_, unchanged) = call(
            "/api/games/expects/input".to_string(),
            serde_json::json!({ "line": "1", "expect": "options" }),
        )
        .await;
        assert_eq!(
            unchanged["frame"]["decision"]["turn_number"], serde_json::json!(1),
            "the correct tag goes through and the game advances normally",
        );
    }

    /// The payload is built for the seat that is ABOUT TO MOVE, so the hand it
    /// shows and the options it offers are the same seat's.
    ///
    /// The regression this pins: `viewing_seat` used to return the first seat
    /// no bot held, which is seat 0 forever. At 3P with one bot, seats 0 and 2
    /// are both human, and the payload then described three different seats at
    /// once — RED's hand panel, "seat 2 is deciding", and seat 2's moves as
    /// live options — with the acting seat's hand on no panel at all. A 3P
    /// table is one click away in the client's own setup screen, so this is
    /// reachable, not theoretical.
    ///
    /// Asserted as an IDENTITY across a whole turn rather than at one moment:
    /// the failure only appears once play has passed seat 0, which is exactly
    /// the state a single snapshot would miss.
    ///
    /// **Re-pointed for M6.1: the state is READ at the top of every step rather
    /// than taken from the previous POST's reply.** A reply is the CALLER's
    /// view now, so the move that ends a turn answers its own mover with a
    /// `watching` prompt — true, and not the payload this cell is about. One
    /// identity holds all three seats, so every request here IS the acting
    /// seat's holder's, and the walk asserts the same identity it always did.
    #[tokio::test]
    async fn every_payload_shows_the_hand_of_the_seat_it_offers_moves_for() {
        let router = app(Arc::new(AppState::in_memory()));
        let call = |method: &'static str, uri: String, body: Option<serde_json::Value>| {
            let router = router.clone();
            async move {
                let req = Request::builder()
                    .method(method)
                    .uri(uri)
                    .header(identity::CLAIMED_HEADER, WHO);
                let req = match body {
                    Some(v) => req
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&v).expect("a body")))
                        .expect("a request"),
                    None => req.body(Body::empty()).expect("a request"),
                };
                let res = router.oneshot(req).await.expect("the router answers");
                let bytes =
                    axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
                serde_json::from_slice::<serde_json::Value>(&bytes).expect("JSON")
            }
        };

        // 3P, so play passes through more than one seat — the shape that broke.
        //
        // **All three seats are human now, and that is R-P10 rather than a
        // preference**: the bot is refused at three and four players with the
        // evaluator's own sentence, so the 3P-with-one-bot table this cell used
        // to build can no longer be created. A hot seat at 3P reaches the same
        // state — the payload follows the mover across a seat boundary — which
        // is what the walk below asserts.
        start(&router, "hotseat", 3, &[]).await;

        let mut saw_a_seat_other_than_zero = false;
        for step in 0..60 {
            // READ, rather than reuse the last reply: a reply is the caller's
            // view and this cell is about the ACTING seat's.
            let state = call("GET", "/api/games/hotseat".to_string(), None).await;
            if state["over"].as_bool() == Some(true) {
                break;
            }
            let seat = state["seat"].as_u64().expect("a seat");
            let to_move = state["to_move"].as_u64().expect("a seat");
            assert_eq!(seat, to_move, "step {step}: the payload is built for the seat to move: {state}");
            assert_eq!(
                state["your_turn"], serde_json::json!(true),
                "step {step}: a payload built for the acting seat is that seat's turn",
            );
            assert_eq!(
                state["frame"]["decision"]["viewer_is_deciding"], serde_json::json!(true),
                "step {step}: ...so the decision zone is the viewer's own",
            );
            let hand = state["frame"]["dashboard"]
                .as_array()
                .expect("panels")
                .iter()
                .find_map(|p| p.get("Hand"))
                .expect("a Hand panel");
            assert_eq!(
                hand["seat"].as_u64(), Some(seat),
                "step {step}: the hand shown belongs to the seat being offered moves: {hand}",
            );
            if seat != 0 {
                saw_a_seat_other_than_zero = true;
            }

            // Answer whatever is asked, first row or confirm — enough to walk
            // the turn order past seat 0.
            let line = match state["prompt"]["kind"].as_str() {
                Some("options") | Some("pick") => state["prompt"]["rows"][0]["number"]
                    .as_u64()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "confirm".to_string()),
                Some("retreat") => "0".to_string(),
                Some("game_over") => break,
                _ => "confirm".to_string(),
            };
            call("POST", "/api/games/hotseat/input".to_string(), Some(serde_json::json!({ "line": line })))
                .await;
        }
        assert!(
            saw_a_seat_other_than_zero,
            "the walk never left seat 0, so it never exercised the case that broke",
        );
    }

    /// The four read-only surfaces answer, and answer with the offer — not with
    /// an empty shell that a client would render as an empty board.
    ///
    /// Asserted on CONTENT, not on a 200: every one of these builders returns a
    /// well-formed empty structure when handed nothing, so a status check alone
    /// would pass a route that had lost its state. The Port board is six slots
    /// with a real zone gate, the Lab board six with its surcharges, the areas
    /// reference one card per tile, and the log the opening's own events.
    #[tokio::test]
    async fn the_read_only_surfaces_carry_the_offer_the_terminal_shows() {
        let router = app(Arc::new(AppState::in_memory()));
        let call = |method: &'static str, uri: String, body: Option<serde_json::Value>| {
            let router = router.clone();
            async move {
                let req = Request::builder()
                    .method(method)
                    .uri(uri)
                    .header(identity::CLAIMED_HEADER, WHO);
                let req = match body {
                    Some(v) => req
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&v).expect("a body")))
                        .expect("a request"),
                    None => req.body(Body::empty()).expect("a request"),
                };
                let res = router.oneshot(req).await.expect("the router answers");
                let status = res.status();
                let bytes =
                    axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
                (status, serde_json::from_slice::<serde_json::Value>(&bytes).expect("JSON"))
            }
        };

        start(&router, "surfaces", 2, &[1]).await;

        let (status, ports) = call("GET", "/api/games/surfaces/ports".into(), None).await;
        assert_eq!(status, StatusCode::OK, "{ports}");
        let board = &ports["boards"][0];
        assert_eq!(board["slots"].as_array().map(Vec::len), Some(6), "a Port board is six slots");
        assert!(
            board["slots"].as_array().unwrap().iter().any(|s| s["card"].is_number()),
            "the opening Port offer is dealt, not empty: {ports}",
        );
        // The zone gate is the engine's, and it is what makes a HIGH card not
        // an offer to a player who cannot reach it.
        let powers: Vec<u64> = board["slots"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|s| s["min_power"].as_u64())
            .collect();
        assert!(powers.contains(&2) && powers.contains(&3) && powers.contains(&4), "{powers:?}");
        assert!(ports["deck_remaining"].as_u64().is_some_and(|n| n > 0), "{ports}");

        let (status, labs) = call("GET", "/api/games/surfaces/labs".into(), None).await;
        assert_eq!(status, StatusCode::OK, "{labs}");
        let slots = labs["boards"][0]["slots"].as_array().expect("six lab slots").clone();
        assert_eq!(slots.len(), 6);
        assert_eq!(
            slots.iter().filter_map(|s| s["surcharge"].as_u64()).collect::<Vec<_>>(),
            vec![0, 0, 1, 1, 2, 2],
            "the printed position surcharges ride with the slots: {labs}",
        );

        let (status, areas) = call("GET", "/api/games/surfaces/areas".into(), None).await;
        assert_eq!(status, StatusCode::OK, "{areas}");
        let tiles: usize =
            areas["rows"].as_array().expect("rows").iter().map(|r| r.as_array().unwrap().len()).sum();
        assert_eq!(tiles, 10, "a 2P board is ten tiles and the reference covers all of them");

        let (status, log) = call("GET", "/api/games/surfaces/log".into(), None).await;
        assert_eq!(status, StatusCode::OK, "{log}");
        let total = log["total"].as_u64().expect("a total");
        assert_eq!(
            log["entries"].as_array().map(Vec::len).unwrap_or(0) as u64,
            total,
            "no tail asked for means no tail taken: {log}",
        );

        // A game with a bot to move has already played; `?tail=` trims and
        // `total` stays truthful, which is what lets a client say how many it
        // is not showing.
        let (_, after) = call(
            "POST",
            "/api/games/surfaces/input".into(),
            Some(serde_json::json!({ "line": "1" })),
        )
        .await;
        assert!(after["end_triggers"].as_str().is_some_and(|s| s.starts_with("end triggers:")));
        let (_, trimmed) = call("GET", "/api/games/surfaces/log?tail=1".into(), None).await;
        assert_eq!(trimmed["entries"].as_array().map(Vec::len), Some(1), "{trimmed}");
        assert!(
            trimmed["total"].as_u64().unwrap() >= 1,
            "the total counts every event, not the tail: {trimmed}",
        );
    }

    /// A seat's-worth of the hidden set rides on this: the seed a client sends
    /// must be the seed the game is dealt from.
    ///
    /// The value here is `0xFEDC_BA98_7654_3210` — past 2^53, so a JSON number
    /// would round it, and past `Number.MAX_SAFE_INTEGER` by enough that the
    /// rounding lands on a DIFFERENT seed and therefore a different deal.
    ///
    /// **Re-pointed from `preview` to `create` (M6.1).** The round trip it used
    /// to walk was preview-then-create; the setup flow is create-then-deal, and
    /// `preview` is retired (§8) — so a seed now crosses this wire in ONE
    /// direction only, inbound on a create. What is asserted
    /// is strictly stronger than before: rather than comparing two previews to
    /// each other, the dealt ids are compared to `new_game`'s OWN deal at that
    /// seed, so a server that rounded consistently could not pass by rounding
    /// both sides the same way.
    #[tokio::test]
    async fn a_seed_survives_the_create_round_trip_past_the_double_limit() {
        const SEED: u64 = 0xFEDC_BA98_7654_3210;
        // The premise, stated rather than assumed: through a double — which is
        // every JSON number a browser parses — this seed is NOT this seed.
        assert_ne!(
            SEED,
            SEED as f64 as u64,
            "a seed that survived an f64 would make this test vacuous",
        );

        let router = app(Arc::new(AppState::in_memory()));
        let body = |name: &str, seed: u64| {
            serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
                "seats": [{ "user": WHO }, { "bot": true }],
                "seed": seed.to_string(), "name": name, "id": name,
            })
        };

        let (status, table) = call(&router, "POST", "/api/games", Some(body("bigseed", SEED))).await;
        assert_eq!(status, StatusCode::CREATED, "{table}");
        // The table does NOT echo the seed back. It used to on the created
        // game's payload, and that was the leak: the setup seed regenerates both
        // decks, the boxed pile, the board and every deal, so a payload carrying
        // it publishes the whole hidden set.
        assert!(
            !table.to_string().contains(&SEED.to_string()),
            "a table payload carries no seed: {table}",
        );

        // The game really was dealt from it: the seat's own two cards are the
        // engine's own deal at this exact seed.
        let dealt = |deal: &serde_json::Value| {
            deal["cards"]
                .as_array()
                .expect("two dealt cards")
                .iter()
                .map(|c| c["card"]["id"].as_u64().expect("a card id"))
                .collect::<Vec<_>>()
        };
        let (status, deal) = call(&router, "GET", "/api/games/bigseed/deal", None).await;
        assert_eq!(status, StatusCode::OK, "{deal}");
        let pending = yokohama_core::new_game(
            yokohama_core::GameConfig::for_player_count(2),
            &Rules::standard(),
            SEED,
        )
        .expect("the setup deals");
        assert_eq!(
            dealt(&deal),
            pending.deals[0].iter().map(|id| u64::from(id.0)).collect::<Vec<_>>(),
            "the dealt cards are the ones this exact seed produces",
        );

        // **The discrimination the assertion above cannot make on its own.**
        // `SEED` and `SEED ^ 1` differ in the LAST BIT and collide as `f64` — so
        // if any step of this path put the value through a double, these two
        // tables deal identically. They must not.
        let neighbour = SEED ^ 1;
        assert_eq!(
            neighbour as f64 as u64, SEED as f64 as u64,
            "the premise: these two seeds are indistinguishable once rounded",
        );
        let (status, table) =
            call(&router, "POST", "/api/games", Some(body("neighbour", neighbour))).await;
        assert_eq!(status, StatusCode::CREATED, "{table}");
        let (_, adjacent) = call(&router, "GET", "/api/games/neighbour/deal", None).await;
        assert_ne!(
            dealt(&adjacent),
            dealt(&deal),
            "a one-bit change in the seed must change the deal — equal here means the \
             full u64 did not survive the round trip",
        );
    }

    /// The fence is on the ROUTE, not merely in the predicate.
    ///
    /// `grammar`'s own tests prove the allowlist decides correctly; this proves
    /// the handler asks it, before touching the game, and that the file the
    /// line named does not appear. A predicate nothing calls is not a fence.
    #[tokio::test]
    async fn the_mutation_endpoint_refuses_a_line_that_would_write_a_file() {
        let target = std::env::temp_dir().join("yokohama-web-must-never-exist.json");
        let _ = std::fs::remove_file(&target);

        let router = app(Arc::new(AppState::in_memory()));
        let post = |uri: String, body: serde_json::Value| {
            let router = router.clone();
            async move {
                let res = router
                    .oneshot(
                        Request::builder()
                            .method("POST")
                            .uri(uri)
                            .header("content-type", "application/json")
                            .header(identity::CLAIMED_HEADER, WHO)
                            .body(Body::from(serde_json::to_vec(&body).expect("a body")))
                            .expect("a request"),
                    )
                    .await
                    .expect("the router answers");
                let status = res.status();
                let bytes =
                    axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
                (status, serde_json::from_slice::<serde_json::Value>(&bytes).expect("JSON"))
            }
        };

        start(&router, "fence", 2, &[1]).await;

        let (status, body) = post(
            "/api/games/fence/input".to_string(),
            serde_json::json!({ "line": format!("save {}", target.display()) }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert!(
            body["error"].as_str().is_some_and(|e| e.contains("writes a file on the server")),
            "the refusal explains itself: {body}",
        );
        assert!(!target.exists(), "the refused line must not have written {}", target.display());
    }

    /// An unknown API path answers as the API, not as the client.
    ///
    /// The client is served from this same origin on a fallback, so without the
    /// `/api/` guard a mistyped endpoint would return `index.html` with a 200
    /// once a build exists — and the caller would see a JSON parse error
    /// pointing at `<!doctype` instead of the 404 it needed.
    #[tokio::test]
    async fn an_unknown_api_path_is_a_json_404_not_the_client() {
        let (status, body) = get_json("/api/nope").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(
            body["error"].as_str().is_some_and(|e| e.contains("/api/nope")),
            "the refusal names the path it refused: {body}",
        );
    }

    // ---- M6.5b2: the deployment switches and the turnstile --------------------

    /// The switch's own parse (M6.5b2, R-6.5b2.4), which every cell below
    /// reaches past by constructing a [`Multiplayer`] directly — so without
    /// this one, an unset variable reading `off`, or a misspelling reading
    /// `on`, would take a whole deployment down with the suite green.
    ///
    /// The three claims are spec §3.1's: absent is `on`, `off` is off, and a
    /// value that is neither refuses NAMING the variable, which is what an
    /// operator has to see to fix it.
    #[test]
    fn the_multiplayer_switch_defaults_on_and_a_bad_value_names_the_variable() {
        assert_eq!(Multiplayer::from_env_value(None), Ok(Multiplayer(true)), "absent is on");
        assert_eq!(Multiplayer::from_env_value(Some("on")), Ok(Multiplayer(true)));
        assert_eq!(Multiplayer::from_env_value(Some("off")), Ok(Multiplayer(false)));
        assert_eq!(
            Multiplayer::from_env_value(Some(" OFF ")),
            Ok(Multiplayer(false)),
            "trimmed and case-insensitive, as the chat variables are read",
        );
        for bad in ["false", "0", "", "yes"] {
            match Multiplayer::from_env_value(Some(bad)) {
                Ok(m) => panic!("{bad:?} is neither on nor off and must refuse, not read as {m:?}"),
                Err(e) => assert!(e.contains(Multiplayer::VAR) && e.contains(bad), "{bad:?}: {e}"),
            }
        }
    }

    /// R-6.5b2.4: OFF means NOT SERVED. Not gated, not hidden — the route is not
    /// in the table, so an unknown path is the table's own 404 and there is no
    /// second code path that "hides" anything.
    ///
    /// **It sweeps every pattern and every method the router can answer, not a
    /// sample.** Five hand-picked GETs would miss `POST /api/games`, and it would
    /// miss the three routes the AXUM ROUTER owns outside the layer's table
    /// (`app()`: `events`, `…/stream`, `/api/stream`) — which is the whole class
    /// this cell exists to catch, because changing [`api::routes`] alone cannot
    /// remove them.
    #[tokio::test]
    async fn multiplayer_off_serves_no_game_route_and_still_serves_the_public_ones() {
        let app = app_with_multiplayer(false).await;
        // Every pattern the layer would serve, with a concrete id, in every method
        // its row declares — plus the router's own three.
        let mut forbidden: Vec<(&str, String)> = api::routes(true)
            .iter()
            .filter(|r| r.pattern.starts_with("/api/games") || r.pattern.starts_with("/api/chat"))
            .map(|r| {
                (r.method, r.pattern.replace("{id}", "x").replace("{k}", "0").replace("{kind}", "orders"))
            })
            .collect();
        for uri in ["/api/games/x/events", "/api/games/x/stream", "/api/stream"] {
            forbidden.push(("GET", uri.to_string()));
        }
        assert!(forbidden.len() >= 20, "anti-vacuity: the sweep is the whole table, not a sample");
        for (method, uri) in &forbidden {
            let (status, body) = raw(app.clone(), method, uri, Some(WHO)).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri} must not be served with multiplayer off");
            // And it is the TABLE's own 404 — the sentence a misspelling gets —
            // rather than a second code path that "hides" a route and says so.
            let body = String::from_utf8_lossy(&body);
            assert!(body.contains("no such endpoint"), "{method} {uri}: a second sentence: {body}");
        }
        // **The anti-vacuity half**: a 404 everywhere would pass the loop above.
        for uri in ["/api/health", "/api/labels", "/api/me"] {
            let (status, _) = raw(app.clone(), "GET", uri, Some(WHO)).await;
            assert_eq!(status, StatusCode::OK, "{uri} is public and still answers");
        }
        // **And `/api/health` SAYS so** (spec §3.1): the two facts a client
        // reads to decide what to draw follow what is served. `lobby_chat` is
        // the one that used to lie — `/api/chat` is a row the switch removes,
        // so a panel drawn on its word would have fetched a 404 for ever.
        let (_, body) = raw(app.clone(), "GET", "/api/health", Some(WHO)).await;
        let off: serde_json::Value = serde_json::from_slice(&body).expect("health is JSON");
        assert_eq!(off["multiplayer"], false, "the switch is reported: {off}");
        assert_eq!(off["lobby_chat"], false, "and so is the chat that went with it: {off}");
        // The contrast, without which both could be constants: the SAME reads
        // under the switch on.
        let (_, body) = raw(app_with_multiplayer(true).await, "GET", "/api/health", Some(WHO)).await;
        let on: serde_json::Value = serde_json::from_slice(&body).expect("health is JSON");
        assert_eq!(on["multiplayer"], true, "{on}");
        assert_eq!(on["lobby_chat"], true, "{on}");
        // `/api/art` is public too, and it is asserted by WHOSE answer it is
        // rather than by a status: the adapter answers it before the layer, and
        // under `cargo test` the working directory is this crate's, so the
        // manifest is not where `art_dir()` looks and the honest reading is
        // that the route is still routed at all.
        let (_, body) = raw(app.clone(), "GET", "/api/art", Some(WHO)).await;
        let body = String::from_utf8_lossy(&body);
        assert!(!body.contains("no such endpoint"), "/api/art is public and still routed: {body}");
        // **And the archive is NOT one of them** (spec §3.1): it has its own
        // variable, so a single-player deployment can still collect saves.
        let (status, _) = raw(app.clone(), "POST", "/api/archive", Some(WHO)).await;
        assert_ne!(status, StatusCode::NOT_FOUND, "the archive is governed by its own variable");
    }

    /// The turnstile (R-6.5b2.2). It exists so a browser can ACQUIRE the Access
    /// cookie by navigating; the origin's only job is to send it back.
    #[tokio::test]
    async fn enter_is_a_turnstile_back_to_the_lobby() {
        let app = app_with_multiplayer(true).await;
        let (status, location, _) = raw_location(app, "GET", "/enter").await;
        assert_eq!(status, StatusCode::FOUND);
        assert_eq!(location.as_deref(), Some("/#/lobby"));
    }

    /// Its condition is the OR and not the multiplayer switch alone (spec §2.1,
    /// §3.1): with multiplayer off and an archive on it is STILL served, because
    /// the archive is behind the door and the outbox's remedy links here.
    ///
    /// **"Absent" has to be ARRANGED, not merely unregistered.** `assets::serve`
    /// answers any extensionless path with `index.html`, so a `/enter` that is
    /// simply not on the router falls through to the SPA fallback and answers
    /// 200 HTML. This cell therefore runs against a `dist` that HAS an
    /// `index.html` AND is the one the ROUTER serves from (`AppState::dist`):
    /// delete `fallback`'s `/enter` arm and the second assertion below reads
    /// the shell, which is what makes it a claim a mutation can break. Against
    /// a dist the router never saw, every arm would pass unchanged.
    ///
    /// The root is passed and never configured — nothing here writes
    /// `YOKOHAMA_WEB_DIST`.
    ///
    /// The same arrangement carries spec §6.1's other pin: an ABSENT `/sw.js`
    /// is a 404 and never the shell, because it has an extension and the SPA
    /// rule does not apply to it. Its anti-vacuity partner is the positive
    /// control below, where an ordinary route on the SAME router does answer
    /// the shell — so the 404 is the extension rule and not an empty build.
    #[tokio::test]
    async fn enter_follows_any_protected_route_and_not_multiplayer_alone() {
        let dist = dist_with_index_html();
        let (status, _, _) =
            raw_location(app_with(false, /*archive*/ true, &dist).await, "GET", "/enter").await;
        assert_eq!(status, StatusCode::FOUND, "multiplayer off + archive on still needs the door");

        let nothing_protected = app_with(false, /*archive*/ false, &dist).await;
        let (status, _, body) = raw_location(nothing_protected.clone(), "GET", "/enter").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "nothing protected, no turnstile");
        assert!(!body.contains("<!doctype"), "and NOT the SPA fallback's index.html");
        // Spec §6.1: `/sw.js` absent is a 404 and never HTML — a worker script
        // answered as `index.html` registers and then fails to parse.
        let (status, _, body) = raw_location(nothing_protected.clone(), "GET", "/sw.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "an absent worker script is a 404");
        assert!(!body.contains("<!doctype"), "and never the shell: {body}");
        // The positive control, which is what the two `<!doctype>` refusals
        // above are measured against: the fallback on this very router IS live.
        let (status, _, body) = raw_location(nothing_protected, "GET", "/lobby").await;
        assert_eq!(status, StatusCode::OK, "the SPA fallback answers an ordinary route");
        assert!(body.contains("<!doctype"), "with the shell: {body}");

        std::fs::remove_dir_all(&dist).expect("the dist is removable");
    }

    // ---- M4: the replay projection -------------------------------------------
    //
    // `GET /api/games/{id}/at/{k}` over `record::state_at`. The cells below are
    // the spec's guards (`docs/plans/2026-09-01-web-frontend-m4-spec.md` §4).

    /// Walk a game forward, and **actually put assistants on the board**.
    ///
    /// **The naive walk is a documented trap and it silently guts these cells.**
    /// Answering every placement picker with a bare `confirm` places NOTHING —
    /// `wire_fixtures`' own walk records the same hazard, measured at turn 3,644
    /// with nothing having happened. An empty board then means the President has
    /// no legal destination, so at EVERY Movement decision the only legal move
    /// is `ReturnPresidentToHand` and the board's `reachable` set is empty
    /// forever. Measured here before this helper was written: 200 events, 50
    /// Movement decision points, ZERO reachable tiles.
    ///
    /// So the placement pickers get a real area, tried against the engine's own
    /// legality rather than predicted: toggle a board tile, and if the selection
    /// did not take, try the next one. The engine stays the only authority on
    /// what is placeable.
    async fn play(router: &Router, name: &str, moves: usize) {
        let mut tile_cursor = 0usize;
        for _ in 0..moves {
            let (_, state) = call(router, "GET", &format!("/api/games/{name}"), None).await;
            let kind = state["prompt"]["kind"].as_str().unwrap_or_default().to_string();
            let line = match kind.as_str() {
                "game_over" | "locked" | "reveal" | "watching" => break,
                // Once every area is answered the sub-prompt is CONFIRMING, and
                // `step_retreat` accepts nothing but `confirm` there: a count
                // gets "! finish or cancel this prompt first" and applies
                // nothing. Answering "0" forever stalled this walk on the very
                // first retreat — measured at 17 events, with the order deck
                // still at its opening ten however many moves were asked for.
                "retreat" => {
                    if state["prompt"]["remaining"].as_u64() == Some(0) {
                        "confirm".to_string()
                    } else {
                        "0".to_string()
                    }
                }
                "place_b" => {
                    // Option B names its own candidates, so no probing is needed.
                    if state["prompt"]["selected"].is_null() {
                        match state["prompt"]["areas"][0].as_u64() {
                            Some(a) => a.to_string(),
                            None => "cancel".to_string(),
                        }
                    } else {
                        "confirm".to_string()
                    }
                }
                "place_a" => {
                    // Option A names none, so walk the board and let the engine
                    // refuse what it will.
                    let chosen = state["prompt"]["selected"].as_array().is_some_and(|a| !a.is_empty());
                    if chosen {
                        "confirm".to_string()
                    } else {
                        let tiles = state["frame"]["board"]["tiles"].as_array().cloned().unwrap_or_default();
                        if tiles.is_empty() {
                            "cancel".to_string()
                        } else {
                            let t = &tiles[tile_cursor % tiles.len()];
                            tile_cursor += 1;
                            match t["area"].as_u64() {
                                Some(a) => a.to_string(),
                                None => "cancel".to_string(),
                            }
                        }
                    }
                }
                _ => state["prompt"]["rows"][0]["number"]
                    .as_u64()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "confirm".to_string()),
            };
            let (status, _) = call(
                router,
                "POST",
                &format!("/api/games/{name}/input"),
                Some(serde_json::json!({ "line": line })),
            )
            .await;
            if status != StatusCode::OK {
                break;
            }
        }
    }

    async fn total_events(router: &Router, name: &str) -> usize {
        let (_, log) = call(router, "GET", &format!("/api/games/{name}/log"), None).await;
        log["total"].as_u64().expect("a log total") as usize
    }

    /// R-6.5b.10: a run is the positions one by one, BY VALUE, over a run
    /// whose three frames all differ from one another (a run of identical
    /// frames would pass a builder that returned the first frame `to - from`
    /// times, and `index` sits OUTSIDE `frame`, so the comparison is on the
    /// projection and not on the counter).
    #[tokio::test]
    async fn a_run_is_the_positions_one_by_one() {
        let router = app(Arc::new(AppState::in_memory()));
        a_game(&router, "run", &[]).await;
        play(&router, "run", 6).await;
        let total = total_events(&router, "run").await;
        assert!(total >= 3, "the walk applied {total} events; a run of three needs three");
        let (status, run) = call(
            &router,
            "GET",
            &format!("/api/games/run/at?from={}&to={}", total - 3, total),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{run}");
        let frames = run["frames"].as_array().expect("frames");
        assert_eq!(frames.len(), 3);
        for (i, k) in (total - 3..total).enumerate() {
            let (_, one) = call(&router, "GET", &format!("/api/games/run/at/{k}"), None).await;
            assert_eq!(frames[i], one, "frame {k} of the run is at/{k}, by value");
        }
        assert_ne!(frames[0]["frame"], frames[1]["frame"], "anti-vacuity: the middle frame differs from the first");
        assert_ne!(frames[1]["frame"], frames[2]["frame"], "… and from the last");
    }

    /// The boundary in numbers (spec §2.4): `to = total + 1` is ADMITTED and
    /// its last frame is the live frame `at/{total}`; `to = total + 2` answers
    /// what `at/{total + 1}` answers; `from > to` is the same 404 and never a
    /// third answer.
    #[tokio::test]
    async fn a_run_admits_the_live_frame_and_refuses_the_one_past_it() {
        let router = app(Arc::new(AppState::in_memory()));
        a_game(&router, "edge", &[]).await;
        play(&router, "edge", 2).await;
        let total = total_events(&router, "edge").await;
        let (status, run) = call(
            &router,
            "GET",
            &format!("/api/games/edge/at?from={total}&to={}", total + 1),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{run}");
        let (_, live) = call(&router, "GET", &format!("/api/games/edge/at/{total}"), None).await;
        assert_eq!(run["frames"].as_array().map(Vec::len), Some(1));
        assert_eq!(run["frames"][0], live, "the run's last frame is the live frame");
        let (status, past) = call(
            &router,
            "GET",
            &format!("/api/games/edge/at?from={total}&to={}", total + 2),
            None,
        )
        .await;
        let (one_status, one) =
            call(&router, "GET", &format!("/api/games/edge/at/{}", total + 1), None).await;
        assert_eq!(status, one_status, "{past} vs {one}");
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(
            past["error"]
                .as_str()
                .unwrap()
                .starts_with(&format!("this game has {total} events, so its last position is {total}")),
            "{past}"
        );
        let (status, bad) = call(&router, "GET", "/api/games/edge/at?from=2&to=1", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "from > to is the same 404: {bad}");
    }

    /// A run over the cap is a 400 naming it — checked BEFORE the range, so a
    /// fresh game asked for `to=65` reads the cap's sentence and not the
    /// range's — and a non-numeric bound is a 400 in axum's words.
    #[tokio::test]
    async fn a_run_over_the_cap_is_a_400_naming_it() {
        let router = app(Arc::new(AppState::in_memory()));
        a_game(&router, "capped", &[]).await;
        let (status, v) = call(&router, "GET", "/api/games/capped/at?from=0&to=65", None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{v}");
        assert_eq!(v["error"], "a run is at most 64 positions — ask for from=0&to=64 and page");
        let (status, v) = call(&router, "GET", "/api/games/capped/at?from=x&to=1", None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            v["error"].as_str().unwrap().starts_with("Failed to deserialize query string"),
            "axum's words: {v}"
        );
        let (status, v) = call(&router, "GET", "/api/games/capped/at?from=0&to=1&k=2", None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "an unknown field is a 400, not a run: {v}");
    }

    /// **The historical path and the live path agree at the top of the record.**
    ///
    /// This is the property that proves `/at/{k}` is a projection of the same
    /// game rather than a second, drifting construction of one. `state_at` at
    /// `k == events.len()` is documented to be identical to a full replay, so
    /// the board and the dashboard must match the live payload's byte for byte.
    /// If they ever diverge, one of the two is building a frame the engine did
    /// not describe — and the cheapest place to catch that is here.
    #[tokio::test]
    async fn the_frame_at_the_last_index_is_the_live_frame() {
        let router = app(Arc::new(AppState::in_memory()));
        a_game(&router, "replayed", &[]).await;
        play(&router, "replayed", 6).await;

        let total = total_events(&router, "replayed").await;
        assert!(total > 0, "the walk applied nothing, so this cell would compare two empty games");

        let (_, live) = call(&router, "GET", "/api/games/replayed", None).await;
        let (status, hist) =
            call(&router, "GET", &format!("/api/games/replayed/at/{total}"), None).await;
        assert_eq!(status, StatusCode::OK, "{hist}");

        assert_eq!(hist["frame"]["board"], live["frame"]["board"], "the board at k == len");
        assert_eq!(
            hist["frame"]["dashboard"], live["frame"]["dashboard"],
            "the dashboard at k == len",
        );
        assert_eq!(hist["index"], total as u64);
        assert_eq!(hist["total"], total as u64);
        assert_eq!(hist["to_move"], live["to_move"]);
        assert_eq!(hist["over"], live["over"]);
        assert_eq!(hist["deck_remaining"], live["deck_remaining"]);
        assert_eq!(hist["end_triggers"], live["end_triggers"]);
        assert_eq!(hist["end_game_triggered"], live["end_game_triggered"]);

        // The narrow DTO's whole point: the fields that would be present-tense
        // lies are absent by construction, not answered with something.
        assert!(hist.get("your_turn").is_none(), "your_turn is meaningless at a past position");
        assert!(hist.get("prompt").is_none(), "a sub-prompt is not in the record");
        assert!(hist.get("autosaved").is_none(), "both values are false about a past position");
    }

    /// **`k` is range-checked by the handler, and the refusal names the bound.**
    ///
    /// Deliberately NOT derived from `ReplayError::event_index`: that struct
    /// carries a `usize` and a `String` and its own doc says a caller branching
    /// on the index cannot tell past-the-end from a last-event failure without
    /// parsing the message. Checking the range first makes the ambiguity
    /// unreachable instead of navigating it.
    ///
    /// Asserted in BOTH directions, because a route that refused every `k`
    /// would pass the first half.
    #[tokio::test]
    async fn a_k_past_the_end_is_a_404_that_names_the_bound() {
        let router = app(Arc::new(AppState::in_memory()));
        a_game(&router, "bounded", &[]).await;
        play(&router, "bounded", 4).await;
        let total = total_events(&router, "bounded").await;
        assert!(total > 0, "an empty record would make the in-range half vacuous");

        let (status, ok) = call(&router, "GET", "/api/games/bounded/at/0", None).await;
        assert_eq!(status, StatusCode::OK, "k = 0 is in the domain: {ok}");

        let (status, body) =
            call(&router, "GET", &format!("/api/games/bounded/at/{}", total + 1), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        let msg = body["error"].as_str().unwrap_or_default();
        assert!(
            msg.contains(&total.to_string()),
            "the refusal names the bound it enforced: {msg}",
        );
    }

    /// **`k == len` is INSIDE the domain, and it is the index that panicked.**
    ///
    /// `Session::event_description` is `&self.event_descriptions[i]` — a bare
    /// `Vec` index — while `state_at` accepts `k` across `0..=len`. So the top
    /// of the record is a position with no event to name, and any handler that
    /// reaches for a description there panics. This crate installs no
    /// `CatchPanicLayer`, and a panic under the per-game mutex poisons it —
    /// after which every later request for that game hits
    /// `handle.lock().expect("a game")` and panics AGAIN. In production that is
    /// a dropped connection, not a JSON error; an earlier draft of this comment
    /// said "a 500", which no code path here produces.
    ///
    /// Under `oneshot` the first panic unwinds into the test body, so this cell
    /// goes red on the panic itself rather than on its follow-up assertions.
    /// Those still earn their place: they are what would catch a handler that
    /// caught the panic and left the game unusable behind a tidy error.
    #[tokio::test]
    async fn the_last_index_answers_and_leaves_the_game_usable() {
        let router = app(Arc::new(AppState::in_memory()));
        a_game(&router, "unpoisoned", &[]).await;
        play(&router, "unpoisoned", 5).await;
        let total = total_events(&router, "unpoisoned").await;
        assert!(total > 0, "k == len == 0 would not exercise the description path at all");

        let (status, body) =
            call(&router, "GET", &format!("/api/games/unpoisoned/at/{total}"), None).await;
        assert_eq!(status, StatusCode::OK, "the top of the record is a position: {body}");

        let (status, live) = call(&router, "GET", "/api/games/unpoisoned", None).await;
        assert_eq!(status, StatusCode::OK, "the game still answers afterwards: {live}");

        let (status, again) =
            call(&router, "GET", &format!("/api/games/unpoisoned/at/{total}"), None).await;
        assert_eq!(status, StatusCode::OK, "and the same request answers twice: {again}");
    }

    // **THE VIEWER PIN's two M4 cells are retired here, and the property is
    // kept rather than dropped.**
    //
    // `a_replay_sweep_never_unmasks_more_than_one_seat` asserted that one sweep
    // of `0..=total` harvests one seat's hands, and
    // `the_replay_pin_does_not_move_when_the_turn_does` asserted the pin
    // survived a turn change — the second existing because the first takes its
    // whole sweep at one instant and passes under a pin that merely tracks the
    // mover. Both were written against `replay_seat`, the lowest non-bot seat,
    // which was the best pin an UNAUTHENTICATED route could have.
    //
    // Their successor is `a_replay_sweep_as_each_caller_shows_only_that
    // _callers_hands`, and it is strictly stronger than the pair: TWO callers
    // sweep the SAME record across a turn boundary and each sees only its own
    // seat. The second cell's claim in particular does not survive the change
    // and must not be restated — a hot seat's viewer follows the turn on
    // purpose (R-1.7 shows a hand to its own seat, and one person holding both
    // seats is entitled to both), so "the pin does not move when the turn does"
    // is now FALSE of the very table it was written over, and true only of the
    // thing that matters: no caller reaches a seat that is not theirs.

    /// The live payload counts its events, so a client that answered from
    /// `events: n` and receives `events: m > n` knows the reply applied
    /// `[n, m)` — what the turn playback walks (M5.5 spec §3.2.2).
    #[tokio::test]
    async fn the_live_payload_counts_its_events() {
        let router = app(Arc::new(AppState::in_memory()));
        a_game(&router, "count", &[]).await;
        let (_, fresh) = call(&router, "GET", "/api/games/count", None).await;
        assert_eq!(fresh["events"], serde_json::json!(0), "{fresh}");
        play(&router, "count", 5).await;
        let total = total_events(&router, "count").await;
        assert!(total > 0, "`play` applied nothing, so this cell proves nothing");
        let (_, later) = call(&router, "GET", "/api/games/count", None).await;
        assert_eq!(later["events"].as_u64(), Some(total as u64));
    }

    /// The replay position carries the port and lab offers AT k — built from
    /// `state_at(k)`, never copied from the live state (M5.5 spec §3.2.1).
    ///
    /// **The walk has to reach a position whose port DIFFERS from the live one,
    /// or this cell cannot see the defect it names.** `PortsPanel::deck_remaining`
    /// and `HistoricalFrame::deck_remaining` are the SAME expression
    /// (`state.order_deck.len()`), and a 2P order deck sits at its opening ten
    /// until a port card is taken — so while nothing has been taken, a `ports`
    /// built from `session.state()` agrees with the frame beside it at every
    /// `k`, the port is still full at `k == 0`, and the whole cell passes
    /// green. Measured: it did, and only the frozen `at_k.json` golden caught
    /// the swap.
    ///
    /// So the walk continues until a card has actually LEFT the port, that
    /// premise is ASSERTED rather than assumed, and the comparison that carries
    /// the claim is between the SLOT CARDS at `k == 0` and the live ones.
    ///
    /// Mutation-proof by building `ports`/`labs` from `session.state()` in
    /// `HistoricalFrame::at`.
    #[tokio::test]
    async fn the_replay_position_carries_the_offers_at_k() {
        let router = app(Arc::new(AppState::in_memory()));
        a_game(&router, "offers", &[]).await;
        let (_, opening) = call(&router, "GET", "/api/games/offers/ports", None).await;
        let opening_deck = opening["deck_remaining"].as_u64().expect("an order-deck count");

        // Walk until the port has actually been drawn from. The cap FAILS the
        // cell rather than letting it pass on an untaken port.
        let mut took = false;
        for _ in 0..12 {
            play(&router, "offers", 20).await;
            let (_, now) = call(&router, "GET", "/api/games/offers/ports", None).await;
            if now["deck_remaining"].as_u64().is_some_and(|d| d < opening_deck) {
                took = true;
                break;
            }
        }
        assert!(
            took,
            "no port card was ever taken, so every offer in this game is the opening one and \
             this cell cannot tell a historical panel from a copy of the live one",
        );

        let total = total_events(&router, "offers").await;
        assert!(total > 0, "`play` applied nothing, so this cell proves nothing");

        let (_, live_ports) = call(&router, "GET", "/api/games/offers/ports", None).await;
        let (_, live_labs) = call(&router, "GET", "/api/games/offers/labs", None).await;
        let (_, at_end) = call(&router, "GET", &format!("/api/games/offers/at/{total}"), None).await;
        assert_eq!(at_end["ports"], live_ports, "at k == total the offers are the live ones");
        assert_eq!(at_end["labs"], live_labs);

        let slot_cards = |panel: &serde_json::Value| -> Vec<serde_json::Value> {
            panel["boards"][0]["slots"]
                .as_array()
                .expect("a first port board")
                .iter()
                .map(|s| s["card"].clone())
                .collect()
        };

        // **The discriminating comparison.** At `k == 0` the port holds the six
        // cards it was dealt, and the live port no longer does.
        let (_, at_start) = call(&router, "GET", "/api/games/offers/at/0", None).await;
        let dealt = slot_cards(&at_start["ports"]);
        assert_eq!(dealt.len(), 6, "the port has six slots: {}", at_start["ports"]);
        assert!(
            dealt.iter().all(|c| !c.is_null()),
            "the port is full after setup: {}",
            at_start["ports"],
        );
        assert_ne!(
            dealt,
            slot_cards(&live_ports),
            "the frame at k == 0 is carrying the LIVE port rather than the position's",
        );
        // And it is the RIGHT past panel, not merely a different one: byte-equal
        // to what this game's own opening served.
        assert_eq!(at_start["ports"], opening, "k == 0 is the opening position");

        // A cheap consistency invariant across the whole domain, and NOT what
        // proves the sourcing above: both counts are `state.order_deck.len()`,
        // so this says the panel and the frame around it were built from one
        // state — whichever state that is.
        for k in 0..=total {
            let (status, h) = call(&router, "GET", &format!("/api/games/offers/at/{k}"), None).await;
            assert_eq!(status, StatusCode::OK, "k={k}");
            assert_eq!(h["ports"]["deck_remaining"], h["deck_remaining"], "k={k}: {}", h["ports"]);
        }
    }

    /// **A projection serves a locked session; the account still refuses.**
    ///
    /// The rule is stated in code beside `why`: the lockdown gate fences
    /// COMPUTATION, not projection. A locked game is exactly the one whose
    /// history you want to read — the lockdown means the driver found a fault,
    /// and the scrubber is the instrument for finding where.
    ///
    /// One test, both directions, so a route that refused everything cannot pass.
    #[tokio::test]
    async fn a_locked_session_serves_the_replay_and_still_refuses_the_account() {
        let store = Arc::new(MemStore::default());
        let router = app(Arc::new(AppState::with_store(
            store.clone(),
            identity::IdentityMode::Claimed,
        )));
        a_game(&router, "faultedreplay", &[]).await;
        play(&router, "faultedreplay", 4).await;

        let (stored, version) =
            store.read(Doc::Record, "faultedreplay").expect("a stored game").expect("a stored game");
        let mut record: serde_json::Value = serde_json::from_str(&stored).expect("a record");
        record["lockdown"] = serde_json::json!("invariant violation applying that move");
        store
            .write_if_version(
                Doc::Record,
                "faultedreplay",
                &serde_json::to_string(&record).expect("a record"),
                Some(version),
            )
            .expect("a stored lockdown");
        let router =
            app(Arc::new(AppState::with_store(store, identity::IdentityMode::Claimed)));

        let (status, hist) = call(&router, "GET", "/api/games/faultedreplay/at/1", None).await;
        assert_eq!(status, StatusCode::OK, "a projection serves a locked session: {hist}");
        // On CONTENT, not on a 200: an empty shell would render as an empty
        // board and still pass a status check.
        assert_eq!(
            hist["frame"]["board"]["tiles"].as_array().map(Vec::len),
            Some(10),
            "and it draws the 2P board rather than an empty shell: {hist}",
        );

        let (status, w) = call(&router, "GET", "/api/games/faultedreplay/why?event=0", None).await;
        assert_eq!(status, StatusCode::OK, "a refusal is the answer, not an error");
        assert_eq!(w["refusal"]["kind"], "locked", "the account still gates: {w}");
    }

    /// **A historical board carries REAL reach marks, and this cell is not
    /// allowed to be vacuous about it.**
    ///
    /// `build_board` derives `tile.reachable` from the legal list it is handed,
    /// so a route that passed `&[]` would mark every tile unreachable at every
    /// position, the `◈` would silently vanish, and NOTHING would error. The
    /// failure mode is silence, so the assertion has to be that something is
    /// marked — not that the marks are consistent, which an all-false board
    /// satisfies trivially.
    ///
    /// **This is why the cell sweeps instead of picking a position.** Reach
    /// marks only exist where the phase permits a President move, and the
    /// committed `at_k.json` fixture — a real mid-game position — has ZERO of
    /// them. A cell pinned to one hand-chosen index is exactly the vacuous
    /// shape that already exists at HEAD (`screen_model`'s own reachable pin
    /// sits at `AdditionalActionFirst`, where no President move can be legal).
    /// So this walks the record and requires that at least one stop marks
    /// something.
    ///
    /// Mutation-proof by passing an empty legal list to `build_frame` in
    /// `HistoricalFrame::at`: every stop then marks nothing and this fails.
    #[tokio::test]
    async fn a_historical_board_still_marks_where_the_president_can_go() {
        let router = app(Arc::new(AppState::in_memory()));
        a_game(&router, "reachable", &[]).await;
        play(&router, "reachable", 14).await;
        let total = total_events(&router, "reachable").await;
        assert!(total > 0, "an empty record has no position to check");

        let mut marked_somewhere = 0usize;
        let mut positions = 0usize;
        for k in 0..=total {
            let (status, hist) =
                call(&router, "GET", &format!("/api/games/reachable/at/{k}"), None).await;
            assert_eq!(status, StatusCode::OK, "k={k}: {hist}");
            let tiles = hist["frame"]["board"]["tiles"].as_array().expect("tiles");
            assert!(!tiles.is_empty(), "k={k} drew no board at all");
            positions += 1;
            if tiles.iter().any(|t| t["reachable"] == serde_json::json!(true)) {
                marked_somewhere += 1;
            }
        }

        assert_eq!(positions, total + 1, "the domain is inclusive at both ends");
        assert!(
            marked_somewhere > 0,
            "no position in {positions} marked a single reachable tile — the historical board is \
             being built with an empty legal list, and the reach marker is silently gone",
        );
    }
    /// **`POST …/{id}/join` answers 200 with the table**, and its body is
    /// fenced like every other.
    ///
    /// The unknown key is posted at a DEALING table on purpose: the extractor
    /// refuses it before the handler runs, so the answer is 400 naming the key
    /// and not the 409 the table's own status would give — which is what says
    /// the body fence sits in front of the status check rather than behind it.
    #[tokio::test]
    async fn join_is_a_route_that_answers_200_with_the_table_and_400_on_an_unknown_key() {
        let state = Arc::new(AppState::in_memory());
        let (status, table) = post_as(
            &state,
            "red@wire.test",
            "/api/games",
            serde_json::json!({
                "players": 2,
                "colors": ["Red", "Blue"],
                "start_seat": 0,
                "seed": "20260831",
                "seats": [{ "user": "red@wire.test" }, { "open": true }],
                "id": "joinable",
                "password": "sesame",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{table}");
        assert_eq!(table["status"], serde_json::json!("open"), "{table}");

        let (status, joined) = post_as(
            &state,
            "blue@wire.test",
            "/api/games/joinable/join",
            serde_json::json!({ "seat": 1, "name": "Alex", "password": "sesame" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{joined}");
        assert_eq!(joined["my_seats"], serde_json::json!([1]), "{joined}");
        assert_eq!(joined["seats"][1]["name"], serde_json::json!("Alex"), "{joined}");
        assert_eq!(joined["status"], serde_json::json!("dealing"), "the filling join flips");

        let (status, body) = post_as(
            &state,
            "blue@wire.test",
            "/api/games/joinable/join",
            serde_json::json!({ "seat": 1, "nope": 1 }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert!(body["error"].as_str().unwrap_or_default().contains("nope"), "names the key: {body}");
    }

    /// **A keep may carry the display name**, which is how a seat somebody
    /// else's create bound gets a name at all.
    #[tokio::test]
    async fn keep_carries_a_name() {
        let state = Arc::new(AppState::in_memory());
        let (status, table) = post_as(
            &state,
            "red@wire.test",
            "/api/games",
            serde_json::json!({
                "players": 2,
                "colors": ["Red", "Blue"],
                "start_seat": 0,
                "seed": "20260831",
                "seats": [{ "user": "red@wire.test" }, { "bot": true }],
                "id": "named",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{table}");
        let (status, kept) = post_as(
            &state,
            "red@wire.test",
            "/api/games/named/keep",
            serde_json::json!({ "seat": 0, "card": 1, "name": "Jeremy" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{kept}");
        assert_eq!(kept["seats"][0]["name"], serde_json::json!("Jeremy"), "{kept}");
    }

    // ---------------------------------------------------------------------
    // M6.3 Task 2 — the streams. `lib.rs` is already large and these sit at
    // the end of the module, beside the `events` cells they extend.
    // ---------------------------------------------------------------------

    /// Open an SSE route and read its body for `read_for`, then hand back what
    /// arrived.
    ///
    /// **A deadline rather than a frame count**, because the claims below are
    /// about what the wire carries over a WINDOW: a heartbeat that fires on a
    /// timer, a stream that ends itself, and frames that a `watch` channel is
    /// free to coalesce. Counting to N would stop the read at the first N
    /// interesting bytes and could never see the comment lines between them.
    /// **The third element is why the read stopped**, and it is the whole of
    /// the `end` frame's "and closes" half: `true` means the BODY ended, and
    /// `false` means the deadline cut a stream that was still open. Inferring
    /// it from the text would not do — a body cut mid-stream ends on a frame
    /// terminator exactly as a closed one does.
    async fn stream_frames(
        state: Arc<AppState>,
        uri: &str,
        who: &str,
        read_for: std::time::Duration,
    ) -> (StatusCode, String, bool) {
        let res = app(state)
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header(identity::CLAIMED_HEADER, who)
                    .header("accept", "text/event-stream")
                    .body(Body::empty())
                    .expect("a GET"),
            )
            .await
            .expect("the router answers");
        let status = res.status();
        let content_type = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        if status == StatusCode::OK {
            assert!(
                content_type.starts_with("text/event-stream"),
                "{uri} answered {status} as {content_type:?}",
            );
        }
        let mut body = res.into_body().into_data_stream();
        let mut text = String::new();
        let deadline = tokio::time::Instant::now() + read_for;
        let mut ended = false;
        use tokio_stream::StreamExt;
        loop {
            match tokio::time::timeout_at(deadline, body.next()).await {
                Ok(Some(Ok(chunk))) => text.push_str(&String::from_utf8_lossy(&chunk)),
                // The body is over — the sender was dropped behind it.
                Ok(None) => {
                    ended = true;
                    break;
                }
                // A transport error, or the deadline. Neither is an end.
                Ok(Some(Err(_))) | Err(_) => break,
            }
        }
        (status, text, ended)
    }

    /// The `event: …` frames of an SSE body, in order. Comment lines are not
    /// frames and are asserted separately.
    fn sse_frames(text: &str) -> Vec<&str> {
        text.split("\n\n").filter(|f| f.starts_with("event:")).collect()
    }

    /// How many heartbeat comments the body carries.
    ///
    /// Counted as LINES rather than as occurrences of `"\n:"`, because the very
    /// first thing a quiet stream writes is a comment with nothing in front of
    /// it — a substring search anchored on a newline misses it, and on the
    /// house stream that is the common case rather than an edge one.
    fn sse_comments(text: &str) -> usize {
        text.lines().filter(|l| l.starts_with(':')).count()
    }

    /// **The house stream's frames while `act` runs**, as parsed `data:` bodies.
    ///
    /// The house channel is `from_changes` and therefore SILENT on connect, so
    /// the first frame a reader here sees is the first bump `act` caused —
    /// which is what makes "one action, one frame" readable at all. The reader
    /// is spawned first and given a moment to reach its subscribe, exactly as
    /// the table-stream cells do it: a bump sent before the subscribe lands is
    /// a bump into a channel nobody is holding.
    async fn house_frames_after<F, Fut>(state: &Arc<AppState>, act: F) -> Vec<serde_json::Value>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let s = Arc::clone(state);
        let reader = tokio::spawn(async move {
            stream_frames(s, "/api/stream", "grey@wire.test", std::time::Duration::from_secs(2))
                .await
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        act().await;
        let (status, text, ended) = reader.await.expect("the reader task");
        assert_eq!(status, StatusCode::OK, "{text}");
        assert!(!ended, "the house stream has no end and must stay open: {text:?}");
        text.lines()
            .filter_map(|l| l.strip_prefix("data: "))
            .map(|d| serde_json::from_str(d).expect("a house frame's data is one line of JSON"))
            .collect()
    }

    /// **The table stream: the current `Events` on connect, a frame per bump,
    /// an `end` on delete — and the connection closes behind it** (spec §3.1).
    ///
    /// Four claims in one cell because they are one behaviour over one
    /// connection, and three of them are only observable in sequence: a frame
    /// that arrives "per bump" means nothing without knowing what the FIRST
    /// frame was, and "and closes" is only visible to a reader that outlives
    /// the end frame.
    ///
    /// **The on-connect frame is compared BY VALUE with a live `events` read**
    /// taken right after the reader connected and before the move — not with a
    /// read taken at the end, which would be of a deleted table. That compare
    /// is the whole of "it is the `events` route's object, built by the same
    /// function": a hand-built frame with the same keys and a stale count would
    /// pass a key check and fail this.
    ///
    /// **The pauses are not padding, and the three of them buy different
    /// things.** The first lets the spawned reader reach its subscribe before
    /// anything is written. The 60 ms one after the move lets the reader
    /// OBSERVE that frame before the delete, because `watch` coalesces and an
    /// `end` sent into an unread slot would replace the move's bump rather than
    /// follow it — without it this cell would be asserting that a coalescing
    /// channel does not coalesce. The third is a QUIET window in which the
    /// server is asked for nothing at all, so the heartbeat has to fire on its
    /// own more than once.
    ///
    /// **The read deadline is not a budget and must not become one.** The loop
    /// in [`stream_frames`] stops the instant the body closes, which on the
    /// passing path happens well inside a second — so the deadline is reached
    /// ONLY when this cell is already failing, and it is set at ten seconds so
    /// that a loaded machine reports the real failure rather than a timeout
    /// dressed as one. An earlier draft budgeted 600 ms for a window that had
    /// to contain two frame builds, a move, a delete and two sleeps; that is a
    /// cell racing the clock, and this suite runs in CI.
    ///
    /// The heartbeat is read off the WIRE at a 50 ms cadence, which is the only
    /// reason [`AppState::in_memory_with_heartbeat`] exists: nothing on a built
    /// `Sse` is inspectable (R-3.12). **TWO of them are required, not one** —
    /// R-3.12 is a CADENCE, and a keep-alive that fired once and stopped would
    /// satisfy a count of one while leaving every real connection to be closed
    /// by the edge at 100 s.
    #[tokio::test]
    async fn the_stream_writes_the_current_events_on_connect_then_one_frame_per_bump_and_an_end_on_delete(
    ) {
        let state = Arc::new(AppState::in_memory_with_heartbeat(
            std::time::Duration::from_millis(50),
        ));
        start_as(&state, "wire", 20260831, &["red@wire.test", "blue@wire.test"]).await;

        let s = Arc::clone(&state);
        let reader = tokio::spawn(async move {
            stream_frames(
                s,
                "/api/games/wire/stream",
                "red@wire.test",
                std::time::Duration::from_secs(10),
            )
            .await
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // The comparator, taken while the reader is connected and the table is
        // untouched — see the doc above on why it cannot be taken at the end.
        let (status, on_connect) =
            get_as(&state, "red@wire.test", "/api/games/wire/events").await;
        assert_eq!(status, StatusCode::OK, "{on_connect}");

        // One move by the acting seat, then a pause long enough for the frame
        // it bumps to be read before the delete coalesces over it.
        let (_, game) = get_as(&state, "red@wire.test", "/api/games/wire").await;
        let line = first_option(&game);
        let (status, moved) = post_as(
            &state,
            "red@wire.test",
            "/api/games/wire/input",
            serde_json::json!({ "line": line }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{moved}");
        let (_, after_move) = get_as(&state, "red@wire.test", "/api/games/wire/events").await;
        let played = after_move["events"].as_u64().expect("an event count");
        assert!(played > 0, "the move applied nothing, so there is no bump to watch: {after_move}");
        tokio::time::sleep(std::time::Duration::from_millis(60)).await;
        // The quiet window: nothing is asked of the server here, so every
        // comment line after this point is the keep-alive's own doing.
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;

        state
            .games
            .delete("wire", &identity::User::parse("red@wire.test").expect("an email"))
            .expect("the creator deletes it");

        let (status, text, ended) = reader.await.expect("the reader task");
        assert_eq!(status, StatusCode::OK);
        let frames = sse_frames(&text);
        assert!(frames.len() >= 3, "connect, the move, the end: {text}");

        let data: Vec<&str> =
            text.lines().filter_map(|l| l.strip_prefix("data: ")).collect();
        let first: serde_json::Value =
            serde_json::from_str(data[0]).expect("the first frame's data is one line of JSON");
        assert_eq!(first, on_connect, "the on-connect frame IS the events route's reply: {text}");

        assert!(frames[0].starts_with("event: signal"), "{text}");
        assert!(
            frames.iter().any(|f| {
                f.starts_with("event: signal") && f.contains(&format!("\"events\":{played}"))
            }),
            "no frame carried the move's count {played}: {text}",
        );
        let last = frames.last().expect("a last frame");
        assert!(last.starts_with("event: end"), "{text}");
        assert!(last.contains("deleted"), "{text}");
        assert!(ended, "the end frame arrived but the body stayed open: {text:?}");

        assert!(
            sse_comments(&text) >= 2,
            "a cadence, not one comment: {} heartbeats at 50 ms over a run with a 250 ms quiet \
             window: {text:?}",
            sse_comments(&text),
        );
        assert!(!text.contains("\nid:") && !text.contains("\nretry:"), "R-3.14: {text}");
    }

    /// **A stream on a table that is not there is the route's 404, and it
    /// leaves no sender behind** (Task 1 review, minors #4/#5).
    ///
    /// `Signals::subscribe` creates a sender for ANY id, so a handler that
    /// subscribed before it admitted would hold a connection open forever on a
    /// fabricated id and grow the map one entry per attempt. The order is the
    /// fix and the sender count is how it is seen: admission first, subscribe
    /// after.
    #[tokio::test]
    async fn the_stream_refuses_an_id_that_is_not_there_without_making_a_sender() {
        let state = Arc::new(AppState::in_memory());
        assert_eq!(state.signals.senders(), 0);
        let (status, text, _) = stream_frames(
            Arc::clone(&state),
            "/api/games/nope/stream",
            "red@wire.test",
            std::time::Duration::from_millis(100),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{text}");
        assert_eq!(
            state.signals.senders(),
            0,
            "a refused stream subscribed to something",
        );
    }

    /// **A store that stops answering does NOT tell every watcher the game is
    /// gone** (final review M-5).
    ///
    /// The `Err` arm used to write `{"reason":"gone"}` for every failure alike,
    /// and the client ACTS on that frame by navigating to the lobby — so one
    /// transient disk error would have swept every watcher and every non-acting
    /// player off a game that was still there. A refusal (403 or 404) still
    /// ends the stream, because it is an answer about this table; anything else
    /// writes a COMMENT, which the client's parser drops exactly as it drops a
    /// heartbeat, and the next bump answers.
    ///
    /// **The failure is armed for ONE table read and fired by a BUMP rather
    /// than by a write**, so the window contains the stream closure's own read
    /// and nothing else; the load-bearing anti-vacuity read is that the arming
    /// was spent, since a cell whose failure never fired would assert the happy
    /// path twice. The two bumps are 250 ms apart because `watch` coalesces:
    /// sent together, the recovery would replace the failure and there would be
    /// nothing to see.
    ///
    /// The heartbeat is the production 25 s, so NO comment in this body is the
    /// keep-alive's — which is what makes a comment count a reading about the
    /// failed frame and not about the clock.
    #[tokio::test]
    async fn a_failed_reading_writes_a_comment_and_the_stream_stays_open_for_the_next_bump() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        /// A [`MemStore`] whose next `n` TABLE reads fail, and only those: a
        /// store that failed everything would refuse the `start_as` that makes
        /// the table in the first place.
        struct Flaky {
            inner: MemStore,
            fail_table_reads: AtomicUsize,
        }
        impl Store for Flaky {
            fn list(&self, doc: Doc) -> Result<Vec<crate::store::GameId>, String> {
                self.inner.list(doc)
            }
            fn read(
                &self,
                doc: Doc,
                id: &str,
            ) -> Result<Option<(String, crate::store::Version)>, String> {
                if doc == Doc::Table
                    && self
                        .fail_table_reads
                        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                        .is_ok()
                {
                    return Err("the disk went away".to_string());
                }
                self.inner.read(doc, id)
            }
            fn write_if_version(
                &self,
                doc: Doc,
                id: &str,
                json: &str,
                expected: Option<crate::store::Version>,
            ) -> Result<crate::store::Version, crate::store::WriteError> {
                self.inner.write_if_version(doc, id, json, expected)
            }
            fn delete(&self, id: &str) -> Result<(), String> {
                self.inner.delete(id)
            }
        }

        let store =
            Arc::new(Flaky { inner: MemStore::default(), fail_table_reads: AtomicUsize::new(0) });
        let state = Arc::new(AppState::with_store(
            Arc::clone(&store) as Arc<dyn Store>,
            identity::IdentityMode::Claimed,
        ));
        start_as(&state, "wire", 20260831, &["red@wire.test", "blue@wire.test"]).await;

        let s = Arc::clone(&state);
        let reader = tokio::spawn(async move {
            stream_frames(
                s,
                "/api/games/wire/stream",
                "red@wire.test",
                std::time::Duration::from_millis(900),
            )
            .await
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        store.fail_table_reads.store(1, Ordering::SeqCst);
        state.signals.bump_table("wire");
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        assert_eq!(
            store.fail_table_reads.load(Ordering::SeqCst),
            0,
            "the armed failure never fired, so this cell watched the happy path twice",
        );

        // And the connection is still there to answer it.
        state.signals.bump_table("wire");

        let (status, text, ended) = reader.await.expect("the reader task");
        assert_eq!(status, StatusCode::OK, "{text}");
        assert!(!ended, "a store error closed a connection the table outlived: {text:?}");
        assert!(
            !text.contains("event: end"),
            "a store error told the client the game was no longer available: {text}",
        );
        assert!(
            sse_comments(&text) >= 1,
            "the failed reading wrote nothing at all, so the client learned less than it could: \
             {text:?}",
        );
        let frames = sse_frames(&text);
        assert_eq!(
            frames.len(),
            2,
            "the connect frame and the bump AFTER the failure, and no frame for the failure \
             itself: {text}",
        );
        assert!(frames.iter().all(|f| f.starts_with("event: signal")), "{text}");
    }

    /// **A stream is refused a caller the LOCK shuts out, and refused BEFORE it
    /// subscribes.**
    ///
    /// The stream is a session route and the sweep above cannot carry it: a
    /// route that answers 200 with an open body would hang a helper that reads
    /// the whole reply. So the refusal is asserted here instead.
    ///
    /// **Re-pointed by M6.3 at the two statuses where a caller with no seat is
    /// still refused, and it had to be**: a stranger at an unlocked playing
    /// table is now admitted as a `Spectator` (R-3.5), and at an OPEN table the
    /// signal answers anybody signed in (§3.5 row 2, R-4.3). So the refusals
    /// are read at a LOCKED playing table and at a DEALING one — nobody
    /// watches a deal — and the two widenings are read beside them rather than
    /// traded for them. Without the positive halves a server that refused every
    /// stream would still pass this cell.
    ///
    /// The sender count is the second half of the refusal claim: it must land
    /// before the subscribe, or a caller could grow the map one entry per
    /// attempt.
    ///
    /// **The open arm goes further than a status code**, because that is the
    /// behaviour R-4.3 exists for: the joiner is held on the stream while
    /// somebody takes the last open seat, and the bump has to reach them. A
    /// 200 that never wrote a second frame would leave that screen exactly as
    /// stuck as the 403 did.
    #[tokio::test]
    async fn the_stream_refuses_a_stranger_before_it_subscribes() {
        let state = Arc::new(AppState::in_memory_with_heartbeat(
            std::time::Duration::from_millis(50),
        ));
        start_as(&state, "open", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        start_locked_as(
            &state,
            "shut",
            20260831,
            &["red@wire.test", "blue@wire.test"],
            "sesame",
        )
        .await;
        let (status, body) = post_as(
            &state,
            "red@wire.test",
            "/api/games",
            serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "20260831",
                "seats": [{ "user": "red@wire.test" }, { "open": true }],
                "id": "lobbyt", "name": "lobbyt",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let (status, body) = post_as(
            &state,
            "red@wire.test",
            "/api/games",
            serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "20260831",
                "seats": [{ "user": "red@wire.test" }, { "user": "blue@wire.test" }],
                "id": "dealt", "name": "dealt",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        assert_eq!(body["status"], "dealing", "{body}");

        // The two refusals, and the count that says each landed before the
        // subscribe.
        for (table, sentence) in
            [("shut", "this table is locked — unlock it first"), ("dealt", NOT_SEATED)]
        {
            let before = state.signals.senders();
            let (status, text, _) = stream_frames(
                Arc::clone(&state),
                &format!("/api/games/{table}/stream"),
                "green@wire.test",
                std::time::Duration::from_millis(100),
            )
            .await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{table}: {text}");
            assert!(text.contains(sentence), "{table}: {text}");
            assert_eq!(
                state.signals.senders(),
                before,
                "{table}: a refused stream subscribed",
            );
        }

        // Widening one: an UNLOCKED playing table streams to a watcher, whose
        // frame says it is not their turn.
        let (status, text, _) = stream_frames(
            Arc::clone(&state),
            "/api/games/open/stream",
            "green@wire.test",
            std::time::Duration::from_millis(100),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{text}");
        assert!(text.contains(r#""your_turn":false"#), "a watcher is never to move: {text}");

        // Widening two (R-4.3): the joiner holds the OPEN table's stream and
        // the join that fills the last seat reaches them — the on-connect frame
        // says `open`, and a later frame says `dealing`.
        let s = Arc::clone(&state);
        let reader = tokio::spawn(async move {
            stream_frames(
                s,
                "/api/games/lobbyt/stream",
                "green@wire.test",
                std::time::Duration::from_secs(10),
            )
            .await
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let (status, joined) = post_as(
            &state,
            "green@wire.test",
            "/api/games/lobbyt/join",
            serde_json::json!({ "seat": 1 }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{joined}");
        assert_eq!(joined["status"], "dealing", "the filling join flips it: {joined}");
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        state
            .games
            .delete("lobbyt", &identity::User::parse("red@wire.test").expect("an email"))
            .expect("the creator ends the stream");
        let (status, text, _) = reader.await.expect("the reader task");
        assert_eq!(status, StatusCode::OK, "{text}");
        let data: Vec<serde_json::Value> = text
            .lines()
            .filter_map(|l| l.strip_prefix("data: "))
            .filter_map(|d| serde_json::from_str(d).ok())
            .collect();
        assert_eq!(
            data.first().and_then(|f| f["status"].as_str()),
            Some("open"),
            "the on-connect frame is the open table's own reading: {text}",
        );
        assert!(
            data.iter().any(|f| f["status"] == "dealing"),
            "the join never reached the stranger's stream: {text}",
        );
    }

    /// **The house stream says nothing on connect and one frame per house
    /// bump** (spec §3.2).
    ///
    /// The silence on connect is the claim `from_changes` buys and `new` would
    /// lose, and it is asserted by COUNT: exactly one frame for exactly one
    /// create. A stream built the other way would have two and still look
    /// healthy.
    ///
    /// **This cell genuinely needs a wall-clock window**, because the house
    /// stream never ends and "exactly one frame" is a claim about a PERIOD: the
    /// reader has to sit past the create long enough that a second frame would
    /// have arrived. So the window is two seconds and the cost is paid every
    /// run — there is no early exit to have.
    ///
    /// **The expensive create is therefore moved OUT of it.** A create builds a
    /// table, draws a seed, deals and writes, and an earlier draft had to fit
    /// all of that inside 300 ms or read zero frames and fail. A warm-up create
    /// runs BEFORE the reader is spawned — its bump is invisible to the reader,
    /// which is `from_changes`'s whole point and is itself part of what this
    /// cell says — so the counted create inside the window runs warm.
    #[tokio::test]
    async fn the_house_stream_writes_nothing_on_connect_and_a_seq_per_table_write() {
        let state = Arc::new(AppState::in_memory_with_heartbeat(
            std::time::Duration::from_millis(50),
        ));
        let table_body = |id: &str| {
            serde_json::json!({
                "players": 2,
                "colors": ["Red", "Blue"],
                "start_seat": 0,
                "seed": "20260831",
                "seats": [{ "user": WHO }, { "bot": true }],
                "id": id,
            })
        };
        // Before the reader exists: it warms every path the counted create
        // below takes, and its own bump is one this reader must never see.
        let (status, warm) = post_as(&state, WHO, "/api/games", table_body("warm")).await;
        assert_eq!(status, StatusCode::CREATED, "{warm}");

        let s = Arc::clone(&state);
        let reader = tokio::spawn(async move {
            stream_frames(s, "/api/stream", WHO, std::time::Duration::from_secs(2)).await
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let (status, table) = post_as(&state, WHO, "/api/games", table_body("housed")).await;
        assert_eq!(status, StatusCode::CREATED, "{table}");

        let (status, text, ended) = reader.await.expect("the reader task");
        assert_eq!(status, StatusCode::OK);
        assert!(!ended, "the house stream has no end and must stay open: {text:?}");
        let frames = sse_frames(&text);
        assert_eq!(
            frames.len(),
            1,
            "exactly the counted create's bump — not the warm-up's, and none on connect: {text:?}",
        );
        assert!(frames[0].starts_with("event: house"), "{text}");
        assert!(frames[0].contains("\"seq\":"), "{text}");
        assert!(
            sse_comments(&text) >= 2,
            "a cadence, not one comment: {} heartbeats at 50 ms over two seconds: {text:?}",
            sse_comments(&text),
        );
        assert!(!text.contains("\nid:") && !text.contains("\nretry:"), "R-3.14: {text}");
    }

    /// **`events` answers an OPEN table, and answers it without a session**
    /// (spec §3.3, R-3.3).
    ///
    /// The arm sits where the `dealing` arm sits and for the same reason: an
    /// open table has no record, so a handler that reached `Games::handle`
    /// would hand the open-table screen a 409 instead of the reading it is
    /// watching for its own seats to fill.
    #[tokio::test]
    async fn events_answers_an_open_table_with_zero_events_and_status_open() {
        let state = Arc::new(AppState::in_memory());
        let (status, table) = post_as(
            &state,
            WHO,
            "/api/games",
            serde_json::json!({
                "players": 2,
                "colors": ["Red", "Blue"],
                "start_seat": 0,
                "seed": "20260831",
                "seats": [{ "user": WHO }, { "open": true }],
                "id": "openish",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{table}");
        assert_eq!(table["status"], serde_json::json!("open"), "{table}");

        let (status, e) = get_as(&state, WHO, "/api/games/openish/events").await;
        assert_eq!(status, StatusCode::OK, "{e}");
        assert_eq!(
            e,
            serde_json::json!({ "events": 0, "to_move": null, "status": "open", "your_turn": false, "chat": 0 }),
        );
    }

    /// **`/api/health` names the transport the client should read** (R-plan-4).
    ///
    /// One deployment fact a client cannot infer from a successful call, beside
    /// the identity mode for the same reason. A compile-time constant rather
    /// than configuration: the fallback from SSE to a held request is a code
    /// decision recorded in the close, not something a deployment may flip.
    #[tokio::test]
    async fn health_names_the_signal() {
        let (status, v) = get_json("/api/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(v["signal"], SIGNAL_KIND);
        assert_eq!(v["signal"], "sse");
    }

    /// **`/api/health` carries the lobby switch and the two caps** (M6.4,
    /// R-6.4.6, R-6.4.8).
    ///
    /// The third and fourth deployment facts a client cannot infer from a
    /// successful call, and the reason is the same as the signal's: with the
    /// lobby chat off the client must draw no panel rather than ask and read a
    /// 404, and it must size its input box from the SERVER's character rule
    /// rather than from a copy that drifts the day the variable changes.
    /// `chat_keep` is `null` for `all`, because a number would be a lie about a
    /// document that is never trimmed.
    #[tokio::test]
    async fn health_carries_the_lobby_switch_and_the_two_caps() {
        let (status, v) = get_json("/api/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(v["lobby_chat"], true);
        assert_eq!(v["chat_keep"], 500);
        assert_eq!(v["chat_text"], 500);

        let configured = chat::ChatConfig { lobby: false, limits: chat::ChatLimits { keep: None, text: 140 } };
        let off = Arc::new(AppState::in_memory_with_chat(configured));
        assert_eq!(
            off.games.limits(),
            configured.limits,
            "the numbers health reports are the ones the APPENDER holds — reported \
             from this state and enforced from a defaulted `Games` would be a server \
             truthfully naming a cap nothing applies",
        );
        let (status, v) = get_as(&off, WHO, "/api/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(v["lobby_chat"], false);
        assert_eq!(v["chat_keep"], serde_json::Value::Null, "`all` is null on the wire");
        assert_eq!(v["chat_text"], 140);
    }

    /// **`?wait` is accepted, bounded, and answers on a bump** (R-plan-5).
    ///
    /// The held-request fallback ships whether or not R-3.1's measurement needs
    /// it, because it costs this cell and makes the fallback a one-line change
    /// in the client's reader rather than a second plan. Three readings: a wait
    /// with nothing happening returns at its own deadline rather than hanging,
    /// a wait that a bump lands inside returns EARLY, and a wait above the cap
    /// is a 400 rather than a clamp nobody was told about.
    #[tokio::test]
    async fn events_can_be_held_until_a_bump_and_the_wait_is_bounded() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "held", 20260831, &["red@wire.test", "blue@wire.test"]).await;

        // Nothing is happening: the deadline answers.
        let began = tokio::time::Instant::now();
        let (status, e) = get_as(&state, "red@wire.test", "/api/games/held/events?wait=1").await;
        assert_eq!(status, StatusCode::OK, "{e}");
        assert!(began.elapsed() >= std::time::Duration::from_millis(900), "it did not hold");
        assert_eq!(e["status"], "playing", "{e}");

        // A bump inside the window answers early.
        let s = Arc::clone(&state);
        let mover = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            let (_, game) = get_as(&s, "red@wire.test", "/api/games/held").await;
            let line = first_option(&game);
            post_as(
                &s,
                "red@wire.test",
                "/api/games/held/input",
                serde_json::json!({ "line": line }),
            )
            .await
        });
        let began = tokio::time::Instant::now();
        let (status, e) = get_as(&state, "blue@wire.test", "/api/games/held/events?wait=25").await;
        assert_eq!(status, StatusCode::OK, "{e}");
        let (status, moved) = mover.await.expect("the mover");
        assert_eq!(status, StatusCode::OK, "{moved}");
        assert!(began.elapsed() < std::time::Duration::from_secs(25), "the bump did not wake it");

        // And the cap is a refusal, not a clamp.
        let (status, err) =
            get_as(&state, "red@wire.test", "/api/games/held/events?wait=26").await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{err}");
        assert!(err["error"].as_str().unwrap_or_default().contains("25"), "{err}");
    }

    // ---------------------------------------------------------------------
    // M6.4 Task 4 — the four chat routes (spec §3, §4, R-6.4.1, R-6.4.2,
    // R-6.4.6, R-6.4.11). They sit at the end of the module beside the stream
    // cells they extend, as M6.3's did.
    // ---------------------------------------------------------------------

    /// **Seats and watchers post, a seat is named by its seat, a watcher by its
    /// body** (R-6.4.1, R-6.4.2), and the reply is the message.
    ///
    /// The read is asked by the OTHER seat, so the page is not this poster's
    /// own memory of what it sent, and the cursor is walked three ways: from
    /// zero, from inside the run, and from PAST the newest number — where an
    /// empty page still carries `last`, which is the whole reason `last` is the
    /// document's and not the page's.
    #[tokio::test]
    async fn a_seat_and_a_watcher_post_and_each_is_labelled_its_own_way() {
        let state = Arc::new(AppState::in_memory());
        start_locked_as(&state, "t", ENDED_SEED, &["red@wire.test", "blue@wire.test"], "sesame")
            .await;
        let (status, view) = post_as(
            &state,
            "grey@wire.test",
            "/api/games/t/unlock",
            serde_json::json!({ "password": "sesame" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{view}");

        let (status, m) = post_as(
            &state,
            "red@wire.test",
            "/api/games/t/chat",
            serde_json::json!({ "text": " hi " }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        assert_eq!(m["seq"], 1);
        assert_eq!(m["text"], "hi", "trimmed");
        assert_eq!(m["author"]["kind"], "seat");
        assert_eq!(m["author"]["seat"], 0);
        assert_eq!(m["author"]["user"], "red@wire.test");

        let (status, m) = post_as(
            &state,
            "grey@wire.test",
            "/api/games/t/chat",
            serde_json::json!({ "text": "hello from the rail", "name": "Grey" }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        assert_eq!(m["author"]["kind"], "watcher");
        assert_eq!(m["author"]["name"], "Grey");
        assert!(m["author"].get("seat").is_none(), "a watcher has no seat: {m}");

        let (status, page) = get_as(&state, "blue@wire.test", "/api/games/t/chat").await;
        assert_eq!(status, StatusCode::OK, "{page}");
        assert_eq!(page["messages"].as_array().map(Vec::len), Some(2));
        assert_eq!(page["last"], 2);
        let (_, delta) = get_as(&state, "blue@wire.test", "/api/games/t/chat?after=1").await;
        assert_eq!(delta["messages"][0]["seq"], 2);
        let (_, none) = get_as(&state, "blue@wire.test", "/api/games/t/chat?after=9").await;
        assert_eq!(none["messages"].as_array().map(Vec::len), Some(0));
        assert_eq!(none["last"], 2);
    }

    /// **A seat's name is the SEAT's stored name and a body name is refused**
    /// (R-6.4.2, mutation 4) — by VALUE, over a seat whose stored name differs
    /// from the one in the body.
    ///
    /// The value is what makes it a reading: a handler that took the body's
    /// name for a seated caller would pass a cell that only checked the refusal,
    /// and a handler that answered `null` would pass one that only checked that
    /// the body's name did not come back.
    #[tokio::test]
    async fn a_seated_caller_is_named_by_the_seat_and_may_not_send_a_name() {
        let state = Arc::new(AppState::in_memory());
        let (status, t) = post_as(
            &state,
            "red@wire.test",
            "/api/games",
            serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "20260831",
                "seats": [{ "user": "red@wire.test" }, { "user": "blue@wire.test" }],
                "id": "named", "name": "named", "display_name": "Captain Red",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{t}");
        let (status, refused) = post_as(
            &state,
            "red@wire.test",
            "/api/games/named/chat",
            serde_json::json!({ "text": "hi", "name": "Somebody Else" }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
        assert_eq!(refused["error"], "a seated caller is named by their seat");
        let (status, m) = post_as(
            &state,
            "red@wire.test",
            "/api/games/named/chat",
            serde_json::json!({ "text": "hi" }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        assert_eq!(m["author"]["name"], "Captain Red", "the stored name, at a DEALING table (R-6.4.5)");
    }

    /// **The refusal order, by adjacent-pair probes** (spec §3.2): admission,
    /// then the text, then the seat/name rule, then the watcher's name.
    ///
    /// **Adjacent PAIRS and not single defects**, which is the lesson three
    /// M6.2 tasks relearned: a request carrying one fault pins nothing about
    /// order, because every ordering refuses it the same way. Each probe here
    /// carries two, and the sentence says which rule ran first.
    #[tokio::test]
    async fn the_post_refuses_in_the_spec_s_order() {
        let state = Arc::new(AppState::in_memory());
        start_locked_as(&state, "t", ENDED_SEED, &["red@wire.test", "blue@wire.test"], "sesame")
            .await;
        // admission before text: a shut-out stranger with an empty text is a
        // 403, not a 400.
        let (status, got) = post_as(
            &state,
            "green@wire.test",
            "/api/games/t/chat",
            serde_json::json!({ "text": "" }),
        )
        .await;
        assert_eq!(
            (status, got["error"].as_str()),
            (StatusCode::FORBIDDEN, Some("this table is locked — unlock it first")),
        );
        // text before the seat/name rule: an empty text AND a name → the text
        // sentence.
        let (status, got) = post_as(
            &state,
            "red@wire.test",
            "/api/games/t/chat",
            serde_json::json!({ "text": "  ", "name": "X" }),
        )
        .await;
        assert_eq!(
            (status, got["error"].as_str()),
            (StatusCode::BAD_REQUEST, Some("a message needs some words")),
        );
        // text before the watcher's name: over the cap AND a bad name → the
        // text sentence.
        let (status, view) = post_as(
            &state,
            "grey@wire.test",
            "/api/games/t/unlock",
            serde_json::json!({ "password": "sesame" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{view}");
        let long = "x".repeat(501);
        let (status, got) = post_as(
            &state,
            "grey@wire.test",
            "/api/games/t/chat",
            serde_json::json!({ "text": long, "name": "y".repeat(65) }),
        )
        .await;
        assert_eq!(
            (status, got["error"].as_str()),
            (StatusCode::BAD_REQUEST, Some("a message is at most 500 characters")),
        );
        // and the watcher's name alone is refused by `check_name`'s sentence.
        let (status, got) = post_as(
            &state,
            "grey@wire.test",
            "/api/games/t/chat",
            serde_json::json!({ "text": "ok", "name": "y".repeat(65) }),
        )
        .await;
        assert_eq!(
            (status, got["error"].as_str()),
            (StatusCode::BAD_REQUEST, Some("a name is 1 to 64 characters")),
        );
        // a control character
        let (status, got) = post_as(
            &state,
            "red@wire.test",
            "/api/games/t/chat",
            serde_json::json!({ "text": "a\nb" }),
        )
        .await;
        assert_eq!(
            (status, got["error"].as_str()),
            (StatusCode::BAD_REQUEST, Some("a message is one line, with no control characters")),
        );
        // an unknown key is the fenced 400
        let (status, _) = post_as(
            &state,
            "red@wire.test",
            "/api/games/t/chat",
            serde_json::json!({ "text": "hi", "seat": 1 }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    /// **The house pair: any signed-in caller, 404 when off** (R-6.4.6).
    ///
    /// **The counter is subscribed BEFORE the two refused requests**, which is
    /// what makes the last assertion a reading at all: a receiver taken
    /// afterwards marks the channel's current value as seen and answers
    /// `has_changed()` false whatever happened. Held from before, it is the
    /// witness that a switched-off route appended nothing and announced
    /// nothing.
    #[tokio::test]
    async fn the_house_chat_answers_anyone_signed_in_and_is_404_when_off() {
        let on = Arc::new(AppState::in_memory());
        let (status, m) = post_as(
            &on,
            "grey@wire.test",
            "/api/chat",
            serde_json::json!({ "text": "evening", "name": "Grey" }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        assert_eq!(m["author"]["kind"], "watcher");
        let (status, page) = get_as(&on, "green@wire.test", "/api/chat?after=0").await;
        assert_eq!((status, page["last"].as_u64()), (StatusCode::OK, Some(1)));
        let (status, _) = request_as(&on, "", "GET", "/api/chat", None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "no identity, no house");

        let off = Arc::new(AppState::in_memory_with_chat(chat::ChatConfig {
            lobby: false,
            ..chat::ChatConfig::default()
        }));
        // Not `mut`: this cell only asks `has_changed`, which borrows shared.
        let house = off.signals.subscribe_house();
        for (method, body) in [("GET", None), ("POST", Some(serde_json::json!({ "text": "x" })))] {
            let (status, got) = request_as(&off, "grey@wire.test", method, "/api/chat", body).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{method}: {got}");
            assert_eq!(got["error"], "the lobby chat is off");
        }
        assert!(!house.has_changed().unwrap(), "nothing moved the house counter");
    }

    /// **A caller holding TWO seats posts as their LOWEST before the deal and
    /// as the ACTING seat after it** (spec §3.2, §9.1).
    ///
    /// The hot seat is the shape this rule was written for and nothing pinned
    /// it: one person at both seats, which `start_as` cannot express and the
    /// corrected `admitted` doc describes in two clauses nothing tried. The
    /// table starts at seat ONE deliberately — at seat zero the lowest seat and
    /// the acting seat are the same number and the cell would read the same
    /// under either rule.
    ///
    /// The acting seat is READ off `events` rather than assumed, so the second
    /// half is a comparison between two things the server said and not between
    /// the server and this cell's arithmetic.
    #[tokio::test]
    async fn a_two_seat_caller_posts_as_the_lowest_seat_before_the_deal_and_the_acting_one_after() {
        let state = Arc::new(AppState::in_memory());
        let both = "red@wire.test";
        let (status, t) = post_as(
            &state,
            both,
            "/api/games",
            serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 1,
                "seed": ENDED_SEED.to_string(),
                "seats": [{ "user": both }, { "user": both }],
                "id": "hotseat", "name": "hotseat",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{t}");
        assert_eq!(t["status"], "dealing");

        let (status, m) = post_as(
            &state,
            both,
            "/api/games/hotseat/chat",
            serde_json::json!({ "text": "before" }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        assert_eq!(m["author"]["seat"], 0, "at a deal there is no acting seat: the lowest stands in");

        for seat in [0, 1] {
            let (status, kept) = post_as(
                &state,
                both,
                "/api/games/hotseat/keep",
                serde_json::json!({ "seat": seat, "card": 1 }),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "seat {seat}: {kept}");
        }
        let (status, e) = get_as(&state, both, "/api/games/hotseat/events").await;
        assert_eq!((status, e["status"].as_str()), (StatusCode::OK, Some("playing")), "{e}");
        let acting = e["to_move"].as_u64().expect("a playing table names its mover");
        assert_eq!(acting, 1, "the table was started at seat one, which is what makes this a reading");

        let (status, m) = post_as(
            &state,
            both,
            "/api/games/hotseat/chat",
            serde_json::json!({ "text": "after" }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        assert_eq!(m["author"]["seat"].as_u64(), Some(acting), "and once it is playing, the acting seat");
    }

    /// **A store conflict on the chat write is a 409, in this API's own error
    /// shape** (spec §9.1).
    ///
    /// The one refusal on this route that is not the caller's fault: another
    /// writer landed a message between this append's read and its swap. It has
    /// to be a 409 and not a 500, because a 409 is a request worth sending
    /// again and a 500 is not.
    ///
    /// The failure is armed for ONE chat write, and the spent arming is the
    /// anti-vacuity: without it a cell whose conflict never fired would assert
    /// the happy path and call it a refusal.
    #[tokio::test]
    async fn a_conflicting_chat_write_is_a_409() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        /// A [`MemStore`] whose next `n` `Doc::Chat` writes come back
        /// `Conflict`, and only those: a store that conflicted on everything
        /// would refuse the `start_as` that makes the table.
        struct Contended {
            inner: MemStore,
            conflict_chat_writes: AtomicUsize,
        }
        impl Store for Contended {
            fn list(&self, doc: Doc) -> Result<Vec<crate::store::GameId>, String> {
                self.inner.list(doc)
            }
            fn read(
                &self,
                doc: Doc,
                id: &str,
            ) -> Result<Option<(String, crate::store::Version)>, String> {
                self.inner.read(doc, id)
            }
            fn write_if_version(
                &self,
                doc: Doc,
                id: &str,
                json: &str,
                expected: Option<crate::store::Version>,
            ) -> Result<crate::store::Version, crate::store::WriteError> {
                if doc == Doc::Chat
                    && self
                        .conflict_chat_writes
                        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                        .is_ok()
                {
                    return Err(crate::store::WriteError::Conflict { current: None });
                }
                self.inner.write_if_version(doc, id, json, expected)
            }
            fn delete(&self, id: &str) -> Result<(), String> {
                self.inner.delete(id)
            }
        }

        let store = Arc::new(Contended {
            inner: MemStore::default(),
            conflict_chat_writes: AtomicUsize::new(0),
        });
        let state = Arc::new(AppState::with_store(
            Arc::clone(&store) as Arc<dyn Store>,
            identity::IdentityMode::Claimed,
        ));
        start_as(&state, "wire", ENDED_SEED, &["red@wire.test", "blue@wire.test"]).await;

        store.conflict_chat_writes.store(1, Ordering::SeqCst);
        let (status, got) = post_as(
            &state,
            "red@wire.test",
            "/api/games/wire/chat",
            serde_json::json!({ "text": "raced" }),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{got}");
        assert!(
            got["error"].as_str().is_some_and(|e| e.contains("in flight")),
            "and says what happened: {got}",
        );
        assert_eq!(
            store.conflict_chat_writes.load(Ordering::SeqCst),
            0,
            "the armed conflict never fired, so this cell watched the happy path",
        );
        let (status, m) = post_as(
            &state,
            "red@wire.test",
            "/api/games/wire/chat",
            serde_json::json!({ "text": "again" }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "a 409 is worth sending again: {m}");
    }

    /// **The chat's query struct is fenced like every other one** (spec §7):
    /// `after` is the only key, and a misspelling is a 400 in this API's shape
    /// rather than a silent read from zero.
    ///
    /// Which is the whole of why it matters here: an unfenced `?afer=0` would
    /// answer 200 with the WHOLE retained page, and a client that had renamed
    /// its cursor would redraw every message it already held on every tick,
    /// with nothing in the reply saying so.
    ///
    /// Both routes, because they parse the same struct through the same
    /// extractor and a fence shown on one of two is a fence shown on one.
    #[tokio::test]
    async fn an_unknown_key_on_a_chat_query_is_a_400_on_both_routes() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "wire", ENDED_SEED, &["red@wire.test", "blue@wire.test"]).await;
        for uri in ["/api/games/wire/chat?afer=0", "/api/chat?afer=0"] {
            let (status, got) = get_as(&state, "red@wire.test", uri).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {got}");
            assert!(got["error"].is_string(), "{uri}: in this API's own shape — {got}");
        }
        // The anti-vacuity: the spelled key is answered, so the 400 above is
        // about the NAME and not about the route.
        for uri in ["/api/games/wire/chat?after=0", "/api/chat?after=0"] {
            let (status, got) = get_as(&state, "red@wire.test", uri).await;
            assert_eq!(status, StatusCode::OK, "{uri}: {got}");
        }
    }

    /// **The house applies the name rule at the route, and the SWITCH runs
    /// before the text** (spec §3.3, §9.1).
    ///
    /// Two things nothing pinned. The name: `POST /api/chat` is the one route
    /// where a name is always the caller's own claim, so `check_name` has to
    /// run on it — the house cell above posts a VALID name and would pass
    /// against a route that never checked.
    ///
    /// And the ORDER, by a two-defect probe: a request that is BOTH sent to a
    /// switched-off house AND carries an empty text is answered 404, not 400.
    /// A single-defect probe pins nothing about order, which is the lesson
    /// three M6.2 tasks relearned — and the order matters here because the 400
    /// would tell a caller their message was the problem at a deployment that
    /// has no house chat at all.
    #[tokio::test]
    async fn the_house_checks_the_name_and_reads_its_switch_before_the_text() {
        let on = Arc::new(AppState::in_memory());
        let (status, got) = post_as(
            &on,
            "grey@wire.test",
            "/api/chat",
            serde_json::json!({ "text": "hi", "name": "y".repeat(65) }),
        )
        .await;
        assert_eq!(
            (status, got["error"].as_str()),
            (StatusCode::BAD_REQUEST, Some("a name is 1 to 64 characters")),
            "{got}",
        );
        let (status, got) = post_as(
            &on,
            "grey@wire.test",
            "/api/chat",
            serde_json::json!({ "text": "hi", "name": "ok\u{7}" }),
        )
        .await;
        assert_eq!(
            (status, got["error"].as_str()),
            (StatusCode::BAD_REQUEST, Some("a name has no control characters")),
            "{got}",
        );
        assert_eq!(on.games.house_page(0).expect("a page").1, 0, "and nothing was appended");

        let off = Arc::new(AppState::in_memory_with_chat(chat::ChatConfig {
            lobby: false,
            ..chat::ChatConfig::default()
        }));
        let (status, got) = post_as(
            &off,
            "grey@wire.test",
            "/api/chat",
            serde_json::json!({ "text": "  " }),
        )
        .await;
        assert_eq!(
            (status, got["error"].as_str()),
            (StatusCode::NOT_FOUND, Some("the lobby chat is off")),
            "the switch is read before the text: {got}",
        );
    }

    /// **`Events.chat` on all three arms, and the stream after a chat bump
    /// carries the new count and an unmoved `events`** (spec §4, mutation 5).
    ///
    /// The fifth field is what makes a message a COUNT on the signal and a
    /// fetch for the text, so it has to be answered at every status a table can
    /// be read at — an open table where the seats are still filling, a dealing
    /// one where nobody has a board yet, and a playing one.
    ///
    /// **The last third is the one that can fail for the interesting reason.**
    /// A message must move `chat` and leave `events` alone: a bump wired to the
    /// wrong counter, or a frame that rebuilt `events` from the chat document,
    /// would send every browser at the table to re-fetch a board that had not
    /// changed. The frames are read off the WIRE rather than from a second
    /// `events` call, because the claim is about what the stream carries.
    #[tokio::test]
    async fn events_carries_the_chat_count_at_open_dealing_and_playing() {
        let state = Arc::new(AppState::in_memory());
        let (status, opened) = post_as(
            &state,
            "red@wire.test",
            "/api/games",
            serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "20260831",
                "seats": [{ "user": "red@wire.test" }, { "open": true }], "id": "opent",
                "name": "opent",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{opened}");
        let (status, m) = post_as(
            &state,
            "red@wire.test",
            "/api/games/opent/chat",
            serde_json::json!({ "text": "anyone?" }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "a seat holder talks at an open table: {m}");
        let (_, e) = get_as(&state, "red@wire.test", "/api/games/opent/events").await;
        assert_eq!((e["status"].as_str(), e["chat"].as_u64()), (Some("open"), Some(1)));
        let (_, e) = get_as(&state, "green@wire.test", "/api/games/opent/events").await;
        assert_eq!(e["chat"], 1, "a stranger at open reads the count (M6.3's R-4.3 arm)");
        let (status, refused) = get_as(&state, "green@wire.test", "/api/games/opent/chat").await;
        assert_eq!(status, StatusCode::FORBIDDEN, "and not the messages: {refused}");

        let (status, dealt) = post_as(
            &state,
            "red@wire.test",
            "/api/games",
            serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "20260831",
                "seats": [{ "user": "red@wire.test" }, { "user": "blue@wire.test" }],
                "id": "dealt", "name": "dealt",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{dealt}");
        let (status, m) = post_as(
            &state,
            "blue@wire.test",
            "/api/games/dealt/chat",
            serde_json::json!({ "text": "choosing…" }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        let (_, e) = get_as(&state, "red@wire.test", "/api/games/dealt/events").await;
        assert_eq!((e["status"].as_str(), e["chat"].as_u64()), (Some("dealing"), Some(1)));

        start_as(&state, "playt", ENDED_SEED, &["red@wire.test", "blue@wire.test"]).await;
        let (_, before) = get_as(&state, "red@wire.test", "/api/games/playt/events").await;
        assert_eq!(before["chat"], 0);

        // The window is a fixed one because a chat bump — unlike a delete —
        // closes nothing, so there is no early exit to have; 300 ms to reach
        // the subscribe and the rest of the two seconds for the frame to
        // arrive. `watch` coalesces, so the claim is about the LAST frame
        // rather than about the second one.
        let s = Arc::clone(&state);
        let reader = tokio::spawn(async move {
            stream_frames(
                s,
                "/api/games/playt/stream",
                "blue@wire.test",
                std::time::Duration::from_secs(2),
            )
            .await
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let (status, m) = post_as(
            &state,
            "red@wire.test",
            "/api/games/playt/chat",
            serde_json::json!({ "text": "gl hf" }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        let (status, text, _) = reader.await.expect("the reader task");
        assert_eq!(status, StatusCode::OK, "{text}");
        let data: Vec<serde_json::Value> = text
            .lines()
            .filter_map(|l| l.strip_prefix("data: "))
            .map(|d| serde_json::from_str(d).expect("a frame's data is one line of JSON"))
            .collect();
        assert!(data.len() >= 2, "the on-connect frame and the chat's: {text}");
        assert_eq!(data[0]["chat"], 0, "the on-connect frame predates the message: {text}");
        let last = data.last().expect("a frame");
        assert_eq!(last["chat"], 1, "{text}");
        assert_eq!(last["events"], before["events"], "a message moves no event");
    }

    /// **The house frame is two counters** (R-plan-2, R-6.4.7).
    ///
    /// One channel because a lobby holds ONE connection; two numbers because a
    /// message must not send every lobby to refetch its lists. The second
    /// assertion is the half that can fail: a `chat` bump that reached for
    /// `seq`, or a frame that printed one number under both names, would make
    /// the pair one counter wearing two hats.
    #[tokio::test]
    async fn the_house_frame_carries_seq_and_chat_independently() {
        let state = Arc::new(AppState::in_memory());
        let frames_after_chat = house_frames_after(&state, || async {
            let (status, m) = post_as(
                &state,
                "grey@wire.test",
                "/api/chat",
                serde_json::json!({ "text": "hi" }),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED, "{m}");
        })
        .await;
        // **By COUNT as well as by value**: one message is one frame, and a
        // missing bump would otherwise read as an index panic rather than as
        // the claim this cell makes.
        assert_eq!(frames_after_chat.len(), 1, "one message, one frame: {frames_after_chat:?}");
        assert_eq!(frames_after_chat[0]["chat"], 1);
        assert_eq!(frames_after_chat[0]["seq"], 0);
    }

    /// R-6.5a.6's fence. The wasm check line cannot see a `SystemTime` (it compiles
    /// for the target and panics there), so the crate's own source is read at run
    /// time with each file's top-level test module cut OUT (the code below it is
    /// scanned too), and any platform call outside the `server`-gated files or a
    /// `#[cfg(feature = "server")]`-gated item is named. **The convention is
    /// ENFORCED, not followed** (the whole-branch review's IMP-2): inside a region
    /// every column-0 item must carry its own attribute, or the cell names it as
    /// ungated — so a region's end marker is documentation for the reader and an
    /// item slipped in between two gated ones cannot hide behind them. Two
    /// anti-vacuity partners: the gated files must exist, and at least one gated
    /// site must match, so a pattern that matches nothing is caught as loudly as
    /// a call that escaped.
    #[test]
    fn platform_calls_stay_behind_the_server_feature() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let gated_files = ["art.rs", "assets.rs", "main.rs"];
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
        let mut gated_hits = 0usize;
        for name in gated_files {
            assert!(src.join(name).exists(), "gated file {name} is missing — update the list");
        }
        for entry in std::fs::read_dir(&src).expect("src is readable") {
            let path = entry.expect("an entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let name = path.file_name().unwrap().to_str().unwrap().to_string();
            let text = std::fs::read_to_string(&path).expect("readable");
            // The TOP-LEVEL test module is cut OUT and everything else is scanned —
            // the code above it and the code below it. Top-level means column 0,
            // so an inner `#[cfg(test)]` on a test-only item (`signal.rs`'s
            // `senders`, `games.rs`'s `evict_for_test`) does not end the scan
            // early (the mid-point review measured `games.rs:675-706` and
            // `signal.rs:128-169` unscanned under the old rule), and a platform
            // call written below `mod tests` is read (the battery's residual).
            let lines: Vec<&str> = text.lines().collect();
            let test_start = lines.iter().position(|l| *l == "#[cfg(test)]");
            let test_end = test_start.map(|start| {
                lines[start..].iter().position(|l| *l == "}").map_or(lines.len(), |k| start + k + 1)
            });
            let in_test_module = |i: usize| test_start.is_some_and(|s| i >= s && i < test_end.unwrap());
            let first_code = lines.iter().position(|l| !l.trim().is_empty()).unwrap_or(0);
            let whole_file_gated = gated_files.contains(&name.as_str())
                || lines[first_code].starts_with("#![cfg(feature = \"server\")]");
            // A region starts at a line reading `#[cfg(feature = "server")]` and
            // ends at a line starting at column 0 with `// ---- end server ----`.
            // Inside a region, EVERY item that starts at column 0 must carry its
            // own attribute (an attribute gates one item — the compiler's rule),
            // else it is named as an ungated item inside a region: the region is a
            // reading aid, not a gate. An item runs from its first column-0 line to
            // the next column-0 `}` unless that first line ends in `;`.
            let starts_item = |l: &str| {
                let item = [
                    "pub ", "fn ", "async fn ", "struct ", "enum ", "impl", "const ", "static ", "type ", "use ",
                    "mod ", "trait ", "macro_rules!",
                ];
                item.iter().any(|k| l.starts_with(k))
                    || (l.chars().next().is_some_and(|c| c.is_ascii_lowercase()) && l.contains("!("))
            };
            let mut in_region = false;
            let mut armed = false; // a `#[cfg(feature = "server")]` seen, its item not yet reached
            let mut item_gated = false; // the current item carries the attribute
            let mut item_open = false; // inside an item that runs to a column-0 `}`
            for (n, line) in lines.iter().enumerate() {
                if in_test_module(n) {
                    continue;
                }
                if line.trim_start().starts_with("#[cfg(feature = \"server\")]") {
                    in_region = true;
                    armed = true;
                    continue;
                }
                if line.starts_with("// ---- end server ----") {
                    in_region = false;
                    armed = false;
                    continue;
                }
                if !item_open && starts_item(line) {
                    if in_region && !armed {
                        escaped.push(format!(
                            "{name}:{}: ungated item inside a server region: {}",
                            n + 1,
                            line.trim()
                        ));
                    }
                    item_gated = armed;
                    armed = false;
                    // Closed on its own line when it ends in `;` or `}` (a one-line
                    // `impl X {}` or `struct X;`); otherwise open until a column-0
                    // closer — `}`, or `]`/`)` for a const array or tuple.
                    let end = line.trim_end();
                    item_open = !(end.ends_with(';') || end.ends_with('}'));
                } else if item_open && (line.starts_with('}') || line.starts_with(']') || line.starts_with(')')) {
                    item_open = false;
                }
                let in_gate = whole_file_gated || item_gated;
                for p in patterns {
                    if line.contains(p) && !line.trim_start().starts_with("//") {
                        if in_gate {
                            gated_hits += 1;
                        } else {
                            escaped.push(format!("{name}:{}: {}", n + 1, line.trim()));
                        }
                    }
                }
            }
        }
        assert!(gated_hits > 0, "no gated site matched any pattern — the patterns are stale");
        assert!(
            escaped.is_empty(),
            "platform calls outside the server feature:\n{}",
            escaped.join("\n")
        );
    }
}
