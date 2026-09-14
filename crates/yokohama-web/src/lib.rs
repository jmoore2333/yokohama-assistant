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

pub mod art;
pub mod assets;
pub mod client;
pub mod explain;
pub mod games;
pub mod grammar;
pub mod identity;
pub mod outcome;
pub mod store;
pub mod table;

use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{FromRequest, FromRequestParts, Path, Query, Request, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use yokohama_core::{Rules, SEMANTICS_REV};

use crate::client::ClientState;
use crate::games::{Games, NewGame};
use crate::store::{FsStore, MemStore, Store};

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
}

/// Every refusal `Games` raises, in the status a client reads it as.
///
/// **One mapping, in one place.** Fifteen handlers reach `Games::handle`, and
/// before this each mapped its own error with `ApiError::not_found` — so a
/// dealing table answered 404 on fourteen routes and the client had nothing to
/// branch on. A `From` impl means a handler writes `?` and cannot get it wrong.
impl From<games::GameError> for ApiError {
    fn from(e: games::GameError) -> Self {
        use games::GameError as E;
        match e {
            E::NotFound(m) => Self::not_found(m),
            E::Dealing => Self::dealing(),
            E::Forbidden(m) => {
                Self { status: StatusCode::FORBIDDEN, message: m, table_status: None }
            }
            E::Conflict(m) => Self { status: StatusCode::CONFLICT, message: m, table_status: None },
            E::BadRequest(m) => Self::bad_request(m),
            E::Server(m) => Self::server_error(m),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = match self.table_status {
            Some(status) => serde_json::json!({ "error": self.message, "status": status }),
            None => serde_json::json!({ "error": self.message }),
        };
        (self.status, Json(body)).into_response()
    }
}

type ApiResult<T> = Result<Json<T>, ApiError>;

/// This API's request-BODY extractor, and the reason it is not `axum::Json`.
///
/// Two defects, both measured, both closed here:
///
/// 1. **A rejection did not carry a sentence.** `axum::Json`'s own rejection is
///    a PLAIN-TEXT body, and `web/src/api.ts` builds its `ApiError` by reading
///    `.error` off the body — so `{not json` reached the player as a bare status
///    code with nothing to show. Every refusal on this API is one shape, and
///    that has to include the ones raised before a handler runs.
/// 2. **A shape error was a 422.** Honest for a generic JSON extractor, but
///    this API has one meaning for "your request was wrong" and it is 400.
///
/// The wrapped types carry `#[serde(deny_unknown_fields)]`, which is the half
/// that matters: `#[serde(default)]` is right for an ABSENT optional field and
/// wrong for a field the caller believed it was setting. Before the fence, a
/// client that renamed `start_seat` to `startSeat` got a 201 and a DIFFERENT
/// GAME. Because the fence sits at the extractor rather than in a handler, a
/// malformed body is refused before any lookup — so a bad body against a
/// missing game answers 400, not 404, which is the right precedence.
pub struct JsonBody<T>(pub T);

impl<T, S> FromRequest<S> for JsonBody<T>
where
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(req, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|rejection| ApiError::bad_request(rejection.body_text()))
    }
}

/// The same fence over a QUERY STRING, for the same reasons.
///
/// `?tale=5` for `?tail=5` is otherwise a silent full log: the key is dropped,
/// the `Option` defaults to `None`, and the caller is answered confidently with
/// something it did not ask for.
pub struct QueryArgs<T>(pub T);

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

/// Everything a handler may reach.
///
/// `Rules::standard()` is not cheap and is immutable once built, so it is
/// constructed once per process and shared by reference — the same discipline
/// the CLI's `main` uses.
pub struct AppState {
    pub rules: Arc<Rules>,
    pub games: Games,
    /// Which header names the caller (R-1.2). Read once at start; no default.
    pub identity: identity::IdentityMode,
}

impl AppState {
    /// Backed by a directory on disk.
    pub fn new(identity: identity::IdentityMode) -> Self {
        Self::with_store(Arc::new(FsStore::new(store::default_dir())), identity)
    }

    /// Backed by memory, in `claimed` mode — the mode every test drives with a
    /// header. Tests, and any run that must leave no trace.
    pub fn in_memory() -> Self {
        Self::in_memory_with(identity::IdentityMode::Claimed)
    }

    pub fn in_memory_with(identity: identity::IdentityMode) -> Self {
        Self::with_store(Arc::new(MemStore::default()), identity)
    }

    pub fn with_store(store: Arc<dyn Store>, identity: identity::IdentityMode) -> Self {
        let rules = Arc::new(Rules::standard());
        Self { rules: Arc::clone(&rules), games: Games::new(rules, store), identity }
    }
}

/// What a client must agree with this server about before anything else is
/// worth asking.
///
/// `semantics_rev` and `rules_hash` are the two compatibility keys the engine
/// itself uses — a save stops resuming across a `semantics_rev` bump BY DESIGN,
/// and `rules_hash` moves whenever card, site, token or board data does. A
/// client that caches card art or option shapes checks these, not the server
/// version, which is provenance only.
#[derive(Debug, Clone, Serialize)]
pub struct Health {
    pub server_version: &'static str,
    pub semantics_rev: u32,
    pub rules_hash: String,
    /// Which identity source this deployment is configured with, by name — the
    /// one deployment fact a client cannot infer from a successful call.
    pub identity: &'static str,
}

async fn health(State(state): State<Arc<AppState>>) -> Json<Health> {
    Json(Health {
        server_version: env!("CARGO_PKG_VERSION"),
        semantics_rev: SEMANTICS_REV,
        rules_hash: state.rules.content_hash(),
        identity: state.identity.name(),
    })
}

/// What `/api/me` answers. Never a 401: null is the answer that tells the client
/// which screen to show (spec §2.3).
#[derive(Debug, Clone, Serialize)]
pub struct Me {
    pub mode: &'static str,
    pub user: Option<String>,
}

/// **Who the server thinks you are — and it never refuses to say "nobody".**
///
/// The one route that reads the identity headers WITHOUT the extractor, because
/// the extractor's answer to an absent identity is a 401 and that is precisely
/// the case this route exists to report. A client with no identity yet needs a
/// fact back: the mode, so it knows whether a sign-in screen is even meaningful
/// here, and `user`, which is `null` when nobody is named.
async fn me(State(state): State<Arc<AppState>>, headers: axum::http::HeaderMap) -> Json<Me> {
    Json(Me {
        mode: state.identity.name(),
        user: identity::resolve(state.identity, &headers).ok().map(|u| u.as_str().to_string()),
    })
}

/// Every static enum the wire carries, in words.
///
/// The models are typed and never worded — `screen_model`'s own contract — so a
/// client needs the wording layer's answers once, and these never change during
/// a game. Built from DATA rather than a hand-kept variant list: the tile set,
/// the phase chain, the registries and the achievement registry are what the
/// engine itself enumerates, so a new variant appears here without anyone
/// remembering to add it.
#[derive(Debug, Clone, Serialize)]
pub struct Labels {
    pub area_types: std::collections::BTreeMap<String, String>,
    pub phases: std::collections::BTreeMap<String, String>,
    pub phase_chain: Vec<String>,
    pub tech_effects: std::collections::BTreeMap<String, String>,
    pub achievements: std::collections::BTreeMap<String, String>,
    /// The evaluator's 34 features, in words (M3).
    ///
    /// Keyed by `Feature::name()`, the stable snake_case token the payload
    /// itself carries — so the wire keeps the identity and this map carries the
    /// presentation, exactly as every other entry here does. Authored in
    /// `explain::feature_labels`, never in `yokohama-core`.
    pub features: std::collections::BTreeMap<String, String>,
}

fn key_of<T: Serialize>(v: &T) -> String {
    match serde_json::to_value(v) {
        Ok(serde_json::Value::String(s)) => s,
        other => format!("{other:?}"),
    }
}

async fn labels(State(state): State<Arc<AppState>>) -> Json<Labels> {
    use yokohama_cli::describe;
    use yokohama_cli::screen_model::{achievement_registry, PHASE_CHAIN};

    let mut area_types = std::collections::BTreeMap::new();
    for t in yokohama_core::full_tile_set() {
        area_types.insert(key_of(&t), describe::area_type_name(t).to_string());
    }
    let mut phases = std::collections::BTreeMap::new();
    for p in PHASE_CHAIN {
        phases.insert(key_of(&p), describe::phase_name(p).to_string());
    }
    let mut tech_effects = std::collections::BTreeMap::new();
    for card in state.rules.techs.iter_sorted_by_id() {
        tech_effects
            .insert(key_of(&card.effect), describe::tech_effect_text(card.effect).to_string());
    }
    let mut achievements = std::collections::BTreeMap::new();
    for a in achievement_registry() {
        achievements
            .insert(key_of(&a.achievement_type), describe::achievement_desc(a.achievement_type));
    }

    Json(Labels {
        area_types,
        phases,
        phase_chain: PHASE_CHAIN.iter().map(|p| describe::phase_chain_name(*p).to_string()).collect(),
        tech_effects,
        achievements,
        features: crate::explain::feature_labels(),
    })
}

/// The one handler that answers a 500, and the reason it is not all 400s.
///
/// `games::preview` now says WHOSE fault a failure is
/// ([`games::PreviewError`]): a request this API cannot start a game from is
/// the caller's and stays a 400, while a `new_game`, a `Session::new_sim` or a
/// `HistoricalFrame::at` failure — every one of them downstream of a request
/// `config_of` has already accepted — is this process's. Mapping those to 400
/// told a client its request was wrong and sent it away to fix something that
/// was not broken.
async fn preview_game(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    JsonBody(req): JsonBody<NewGame>,
) -> ApiResult<games::Preview> {
    games::preview(&req, &state.rules, &user).map(Json).map_err(|e| match e {
        games::PreviewError::Request(m) => ApiError::bad_request(m),
        games::PreviewError::Server(m) => ApiError::server_error(m),
    })
}

// **`identity::Identity` is on all twenty `/api/games` handlers**, and since
// M6.1 §5.3 every one of them USES the `User` it carries: a game route resolves
// the caller to a seat before it touches the `Repl`, and builds what it serves
// for that seat. A caller with no seat here is a 403 (R-1.5) rather than a
// payload, and a dealing table is a 409 carrying a status a client can branch
// on rather than a 404 it cannot.
//
// Twenty handlers over eighteen signatures — `ports`, `labs` and `areas` share
// the `surface!` macro below.

/// Resolve the caller to a seated table and a live handle.
///
/// 404 for no table, 403 for a stranger (R-1.5), 409 while dealing. **Every
/// session route starts here**, which is what makes those three answers one
/// sentence each rather than fifteen.
///
/// The TABLE is read first and deliberately: a stranger holding an id learns
/// that it is not theirs, and learns nothing about whether it is dealing.
fn seated(
    state: &AppState,
    id: &str,
    user: &identity::User,
) -> Result<(Arc<std::sync::Mutex<yokohama_cli::repl::Repl>>, crate::table::TableDoc), ApiError> {
    let table = state.games.table(id, user)?;
    let handle = state.games.handle(id)?;
    Ok((handle, table))
}

/// The seat this caller's payload is built for.
///
/// **The `None` arm is unreachable behind [`seated`], and is written anyway.**
/// `viewer_seat` answers `None` exactly when `TableDoc::seats_of` is empty,
/// which is the same condition `Games::table` already refused — so this is the
/// TYPE being total rather than a second fence, and it is what lets every
/// builder downstream take a `PlayerId` instead of an `Option`. The sentence
/// matches the one `Games::table` raises, so a caller cannot tell which of the
/// two spoke.
fn viewer_of(
    repl: &yokohama_cli::repl::Repl,
    table: &crate::table::TableDoc,
    user: &identity::User,
) -> Result<yokohama_core::PlayerId, ApiError> {
    client::viewer_seat(table, user, repl.session.state()).ok_or_else(|| ApiError {
        status: StatusCode::FORBIDDEN,
        message: "you are not seated at this table".to_string(),
        table_status: None,
    })
}

/// Who holds `seat` at this table, for a refusal that names them.
///
/// A bot has no email and is named as what it is. The `None` arm is the same
/// unreachable-by-construction shape as [`viewer_of`]'s: `Games::read_table`
/// refuses a document whose `seats` disagrees with its `players`.
fn holder_of(table: &crate::table::TableDoc, seat: yokohama_core::PlayerId) -> &str {
    match table.seats.get(usize::from(seat.0)) {
        Some(crate::table::SeatDoc::Human { user }) => user,
        _ => "the bot",
    }
}

/// The lobby: `{ "games": [...] }` rather than a bare array.
///
/// An object because a list is a place fields get added to — a cursor, a count —
/// and a client that has to change shape to read one is a client that breaks.
#[derive(Debug, Clone, Serialize)]
pub struct GamesList {
    pub games: Vec<crate::table::TableView>,
}

async fn list_games(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
) -> ApiResult<GamesList> {
    state
        .games
        .list_for(&user)
        .map(|games| Json(GamesList { games }))
        .map_err(ApiError::server_error)
}

/// **201 with the TABLE, and no game yet.** Nothing is dealt into anyone's hand
/// by this call: the seats are bound and every human seat is owed a deal it
/// fetches for itself.
async fn create_game(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    JsonBody(req): JsonBody<NewGame>,
) -> Result<(StatusCode, Json<crate::table::TableView>), ApiError> {
    let view = state.games.create(&req, &user)?;
    Ok((StatusCode::CREATED, Json(view)))
}

async fn get_table(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<crate::table::TableView> {
    let doc = state.games.table(&id, &user)?;
    Ok(Json(doc.view(&user)))
}

/// The caller's own two cards, over the set table. A seated caller's, or a 403.
async fn deal(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<crate::table::DealView> {
    Ok(Json(state.games.deal(&id, &user)?))
}

/// Which of the two dealt cards a seat keeps.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keep {
    pub seat: u8,
    pub card: u8,
}

async fn keep(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    JsonBody(req): JsonBody<Keep>,
) -> ApiResult<crate::table::TableView> {
    // **`PlayerId::new` PANICS at four and above**, and this number came off the
    // wire — so it is parsed through the fallible constructor and refused as a
    // 400. A panic here would poison nothing (the seat is read before any lock)
    // but would still drop the connection with no sentence in it.
    let seat = yokohama_core::PlayerId::try_new(req.seat)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(state.games.keep(&id, &user, seat, req.card)?))
}

async fn get_game(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<ClientState> {
    // Seated first, so a stranger holding an id is a 403 rather than a frame;
    // then the frame is built for the CALLER's seat, so the hand on it is the
    // caller's own and every other seat's is a count (R-1.7).
    let (handle, table) = seated(&state, &id, &user)?;
    let repl = handle.lock().expect("a game");
    let viewer = viewer_of(&repl, &table, &user)?;
    Ok(Json(ClientState::of(&id, &repl, viewer, Vec::new())))
}

async fn delete_game(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state.games.delete(&id, &user)?;
    Ok(StatusCode::NO_CONTENT)
}

/// What the client sends to act.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// An option's `number`, or a sub-prompt token (`confirm`, `cancel`, an
    /// area id, a count). The grammar is the CLI's — but only the GAMEPLAY part
    /// of it; see [`crate::grammar`] for the fence and why it exists.
    pub line: String,
    /// The `prompt.kind` the client was looking at when it chose this line.
    ///
    /// **This is what stops a number meaning a different move.** In-flight
    /// sub-prompt state lives only in the resident `Repl` — `SubPrompt` is not
    /// in the `GameRecord` and does not belong there — so a game evicted or a
    /// process restarted rebuilds with `SubPrompt::None`. The client, which
    /// never re-fetches, keeps sending area ids from an open placement picker,
    /// and `pick_option` reads them against the MAIN option list. Measured
    /// across two processes over one store: a client holding
    /// `{"kind":"place_a","selected":[3,7]}` clicked area 3, and the rebuilt
    /// `Repl` applied option 3 — "Place no assistants" — spending the turn's
    /// placement on nothing the player chose.
    ///
    /// Optional, so `curl` and the integration test stay terse; when present it
    /// is checked, and a mismatch is a refusal that names both prompts instead
    /// of a move nobody asked for.
    #[serde(default)]
    pub expect: Option<String>,
}

/// The one mutation.
///
/// Deliberately a single endpoint over `Repl::step`, not a family of typed
/// verbs. `step` already IS the state machine — it validates against the
/// engine's legal list, opens and closes sub-pickers, and pumps any bot seats
/// that are owed a move afterwards. A second dispatcher here would be a
/// reimplementation of it, and the two would drift; the client's typed
/// affordances come from `PromptState` telling it what to send, not from the
/// server growing a verb per interaction.
///
/// **The line is fenced before it gets there.** `step` is the terminal's front
/// door and answers to more than the game — it writes files, and it opens
/// sticky overlays in which a number stops meaning a move. [`crate::grammar`]
/// carries the allowlist and the reasoning; the fence is here, at the boundary,
/// rather than inside `grammar` calling `step`, so that reading this handler
/// shows both that a line is checked and that a checked line goes through
/// untouched.
async fn input(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    JsonBody(req): JsonBody<Input>,
) -> ApiResult<ClientState> {
    if !grammar::is_gameplay_line(&req.line) {
        return Err(ApiError::bad_request(grammar::refusal(&req.line)));
    }
    let (handle, table) = seated(&state, &id, &user)?;
    let mut repl = handle.lock().expect("a game");
    let viewer = viewer_of(&repl, &table, &user)?;

    // **A move is the ACTING SEAT'S HOLDER'S, and nobody else's.**
    //
    // `viewer_seat` returns the acting seat whenever the caller holds it, so
    // this comparison IS "do you hold the seat that is to move" — one person in
    // several seats passes it at every one of them, which is what keeps a hot
    // seat playable. Everyone else is refused by a sentence that names whose
    // turn it is, because "403" alone tells a player nothing about what to do
    // next: wait, or look at another table.
    //
    // Before `step`, so a refused caller cannot advance the game, and before
    // the `expect` check below, so a watcher is told whose turn it is rather
    // than handed a prompt-moved-on conflict about a prompt that was never
    // theirs.
    let acting = repl.session.state().current_player;
    if acting != viewer {
        return Err(ApiError {
            status: StatusCode::FORBIDDEN,
            message: format!("it is seat {}'s turn ({})", acting.0, holder_of(&table, acting)),
            table_status: None,
        });
    }

    if let Some(expected) = &req.expect {
        let actual = crate::client::prompt_kind(&repl);
        if expected != actual {
            return Err(ApiError {
                status: StatusCode::CONFLICT,
                message: format!(
                    "this game is asking {actual:?}, not {expected:?} — the prompt moved on \
                     (a restart drops an open picker). Re-read the game and answer what it asks.",
                ),
                table_status: None,
            });
        }
    }

    let before = repl.session.record().events.len();
    let locked_before = repl.session.is_locked();
    let out = repl.step(&req.line);

    // A FAILED SAVE IS NOT A FAILED MOVE, and must not be reported as one.
    //
    // `step` has already mutated the resident `Repl`; every later request
    // reuses that handle. Returning the store's error as a 400 told the client
    // its move was refused while the server kept it — measured with the store
    // directory at mode 0555: the client got `400 Permission denied` and the
    // very next GET showed the move applied, with the durable record still one
    // event behind. So the applied state is returned, and the failure is said
    // OUT LOUD as a warning: the move happened, and it is not saved.
    // A LOCKDOWN MUST BE WRITTEN THROUGH even though it adds no event.
    //
    // `Repl::step` engages one AFTER an apply has failed, so `events.len()` is
    // unchanged and an event-count gate skips the write. `Session::lock_down`
    // tries to persist itself via `autosave`, but that is a no-op here —
    // `from_record` and `new_sim` both leave `save_path` as `None`, so this
    // layer is the only writer. Without this the container sleeps, the game is
    // rebuilt from a record with no `lockdown`, `Repl::new` restores
    // `locked = false`, and a desynced game is handed back playable with the
    // fault's reason lost from the record it was deliberately written into.
    let mut warnings = Vec::new();
    if repl.session.record().events.len() != before || repl.session.is_locked() != locked_before {
        if let Err(e) = state.games.persist(&id, &repl) {
            warnings.push(format!("APPLIED BUT NOT SAVED — {e}"));
        }
    }

    // STRIP THE TERMINAL PROMPT BLOCK, keep the deltas.
    //
    // `Repl::out` composes `text` as "what just happened" plus a rendered
    // screen — for the main decision that screen is `prompt()`, which the
    // browser already draws structurally from `PromptState`. Passing it through
    // meant the client printed the numbered options a second time as dead text,
    // a bare `>` caret, and the CLI's standing hint: "'view' any surface · '?'
    // the card index · 'help' for commands" — three words this same server
    // REFUSES, in a client with no text input to type them into. The browser was
    // instructing the player in a grammar it cannot accept.
    //
    // The suffix is removed by IDENTITY, not by pattern-matching prose:
    // `prompt()` is `pub` and pure over the post-step state, so it returns
    // exactly what `out` appended. A sub-prompt's block is deliberately KEPT —
    // it carries the live legality verdict and the placement fee, which have no
    // structured equivalent on the wire yet, so dropping it would lose the only
    // statement of a cost the player is about to pay.
    let body = if matches!(repl.sub, yokohama_cli::repl::SubPrompt::None) {
        let tail = repl.prompt();
        out.text.strip_suffix(&tail).unwrap_or(&out.text).to_string()
    } else {
        out.text.clone()
    };
    let messages: Vec<String> =
        body.lines().map(str::trim_end).filter(|l| !l.is_empty()).map(str::to_string).collect();
    // **The reply is the CALLER's view, not the new mover's.** A move that ends
    // a turn therefore answers its own mover with a `watching` prompt, which is
    // the true thing to say: the game has moved on and this person is not the
    // one it is waiting for.
    let mut client = ClientState::of(&id, &repl, viewer, messages);
    // **The banner must not say "saved" on the payload that says it is not.**
    //
    // `TitleModel::autosaved` is a literal `true` in `client.rs` — correct as a
    // statement about this server, which persists after every applied move — but
    // when THIS write failed the payload carried both "· saved" in the banner and
    // "APPLIED BUT NOT SAVED" three inches below it, on the same screen. The
    // warning is the true one; the banner is corrected to agree with it here,
    // where the write's outcome is actually known.
    if !warnings.is_empty() {
        client.frame.title.autosaved = false;
    }
    client.warnings.extend(warnings);
    Ok(Json(client))
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

/// A handler that reads one game and projects it.
///
/// The three lock-and-project routes differ only in which builder they call, so
/// the borrow, the not-found, the seat fence and the lock live once. A fourth
/// copy of this wiring is a fourth chance to forget one of them.
///
/// **Seated, and no viewer.** All three of these are the public table — the two
/// offer boards and the area reference — and none carries a hand or a seat's
/// own anything, so there is nothing here for a viewer to scope. The fence in
/// front of them is still the seat one: what the three would leak to a stranger
/// is the offer at a table they have no business at.
macro_rules! surface {
    ($name:ident, $model:ty, $build:expr) => {
        async fn $name(
            identity::Identity(user): identity::Identity,
            State(state): State<Arc<AppState>>,
            Path(id): Path<String>,
        ) -> ApiResult<$model> {
            let (handle, _table) = seated(&state, &id, &user)?;
            let repl = handle.lock().expect("a game");
            let f: fn(&yokohama_core::GameState, &Rules) -> $model = $build;
            Ok(Json(f(repl.session.state(), repl.session.rules())))
        }
    };
}

surface!(ports, yokohama_cli::screen_model::PortsPanel, yokohama_cli::screen_model::build_ports);
surface!(labs, yokohama_cli::screen_model::LabsPanel, yokohama_cli::screen_model::build_labs);
surface!(areas, yokohama_cli::screen_model::AreasPanel, yokohama_cli::screen_model::build_areas);

/// How much of the log a request gets when it does not say.
///
/// **The whole thing.** The CLI tails it because a terminal has 24 lines to
/// spend; a browser has a scrollbar, and truncating by default would make the
/// most common request the one that quietly hides the move you are looking for.
/// `?tail=N` trims for a caller that wants a summary, and `LogPanel::total`
/// stays truthful either way, so a client can always say how many it is not
/// showing.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogQuery {
    #[serde(default)]
    pub tail: Option<usize>,
}

/// `GET /api/games/{id}/suggest` — what the production contestant would play on
/// the viewing seat's own view.
///
/// **The first route in this crate that COMPUTES something**, which is why the
/// module doc's "if it ever computes something about the game, that is a bug"
/// carries its one exception: this reads an OPINION about a state and never
/// advances one. It applies nothing, records nothing and autosaves nothing —
/// `suggest`'s contract in the terminal, kept exactly — so GET is honest about
/// the effect, and it is idempotent in the strong sense the CLI already
/// guarantees: byte-identical on repeat, keyed by game identity, seat and move
/// count.
///
/// **A refusal is a 200, not a 4xx.** Being asked to advise on a bot's seat is
/// an ANSWER; only a missing game (404) or a malformed request (400) is an
/// error. The four conditions are `repl::suggest_cmd`'s own, restated because
/// they live in the terminal's command wrapper rather than in `suggest.rs` — a
/// handler that called `suggest::suggest` directly would inherit none of them
/// and would happily search a live game, which the module doc forbids.
///
/// **The lock is dropped before the search runs.** Every other handler holds the
/// per-game `Mutex<Repl>` across its whole body, which is right for a projection
/// and wrong for a multi-second search: it would block every request for this
/// game, including the poll that draws the board. `GameState` is history-free
/// and cheap to clone for exactly this reason.
async fn suggest(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<explain::SuggestResponse> {
    use explain::SuggestRefusal as R;
    let (handle, table) = seated(&state, &id, &user)?;

    // **R-1.8: the acting seat's holder only, and this is a 403 rather than a
    // typed refusal.** The four refusals below are ANSWERS about the game — it
    // is over, it is locked, a bot is thinking — and a client renders each as a
    // sentence in the advisor panel. "This is not your seat" is not that: it is
    // the same access answer every other route on this table gives, and serving
    // it as a 200 would put the advisor's own panel in front of a person who
    // must not be ranking that seat's moves.
    //
    // It also subsumes `SuggestRefusal::BotSeat` at a table where no human
    // holds the bot's seat, which is every table: a bot seat is nobody's, so
    // the caller is refused here first. That refusal stays declared because it
    // is the CLI's own and this crate carries `repl::suggest_cmd`'s ladder
    // verbatim rather than a subset of it.
    let read = {
        let repl = handle.lock().expect("a game");
        let acting = repl.session.state().current_player;
        if !table.seats_of(&user).contains(&acting) {
            return Err(ApiError {
                status: StatusCode::FORBIDDEN,
                message: "it is not your turn".to_string(),
                table_status: None,
            });
        }
        let refuse = if repl.session.record().lockdown.is_some() {
            Some(R::Locked)
        } else if matches!(repl.session.mode, yokohama_cli::session::TrackingMode::Live) {
            Some(R::LiveMode)
        } else if repl.session.is_over() {
            Some(R::GameOver)
        } else if repl.bots().is_bot(repl.session.state().current_player) {
            Some(R::BotSeat)
        } else {
            None
        };
        match refuse {
            Some(r) => Err(r),
            None => {
                let seat = repl.session.state().current_player;
                Ok((
                    seat,
                    repl.session.state().clone(),
                    yokohama_cli::suggest::stream_seed(repl.session.record(), seat),
                ))
            }
        }
    };

    let (seat, game, seed) = match read {
        Err(r) => return Ok(Json(explain::SuggestResponse::refused(r.kind(), r.message()))),
        Ok(parts) => parts,
    };

    Ok(Json(match yokohama_cli::suggest::suggest(&game, &state.rules, seat, seed) {
        Ok(s) => explain::SuggestResponse::granted(explain::Suggest::of(&s, &game, &state.rules)),
        // The agent's own errors are surfaced VERBATIM rather than swallowed —
        // `SuggestError::Inconsistent` describes itself as a bug worth
        // reporting, and a generic "no suggestion" would throw that away.
        Err(e) => explain::SuggestResponse::refused("search", e.to_string()),
    }))
}

/// `GET /api/games/{id}/why[?event=n]` — a bot move, reconstructed by replay.
///
/// `event` is the log's printed 0-based index; omitted means the most recent bot
/// move, which is what bare `why` does in the terminal.
///
/// **The refusals are `why::account_parts`', and not one of them is re-derived
/// here.** That function's own doc is the instruction: *"it must not
/// reimplement the first half: the refusals ARE the honesty gate."* A
/// reconstruction that does not equal the recorded move refuses, naming both
/// moves and both versions — and that refusal reaches the browser as data,
/// because a branch nobody has ever seen is where defects live.
///
/// Same lock discipline as `suggest`, for the same reason: the reconstruction
/// runs the same `decide` path the pump ran.
async fn why(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    QueryArgs(q): QueryArgs<WhyQuery>,
) -> ApiResult<explain::Why> {
    // Seated, and no viewer of its own: `why` already answers at the VIEWER's
    // visibility by masking the roads not taken for every caller
    // (`explain::HandVisibility::Masked`), so an unplayed `FulfillOrder` words
    // as "from hand" and names no card, at every seat. What this fence adds is
    // the seat one — an account of a search at a table the caller has no place
    // at is not theirs to read.
    let (handle, _table) = seated(&state, &id, &user)?;
    let (mode, record, bots) = {
        let repl = handle.lock().expect("a game");
        // **The lockdown gate fences COMPUTATION, not projection**, and this is
        // the rule the three route families follow. M2 already settled that a
        // read-only GET serves a locked session — `ports`/`labs`/`areas`/`log`
        // do, and `ClientState` reports the lockdown as its prompt rather than
        // refusing. But `why` is not a projection: it re-runs the same `decide`
        // path against a record the driver has declared faulty, and answering
        // with an authoritative account of a search over a suspect record is
        // exactly what a lockdown exists to stop. `suggest` refuses for the same
        // reason; the card indexes and the final score do not, because they read
        // the state rather than an opinion about it.
        if let Some(reason) = repl.session.record().lockdown.clone() {
            return Ok(Json(explain::Why::locked(reason)));
        }
        (repl.session.mode, repl.session.record().clone(), repl.bots().clone())
    };
    Ok(Json(
        match yokohama_cli::why::account_parts(mode, &record, &state.rules, &bots, q.event) {
            Ok(account) => explain::Why::granted(&account, &state.rules),
            Err(refusal) => explain::Why::refused(&refusal),
        },
    ))
}

/// `GET /api/games/{id}/index/{kind}` — a card index, as a VIEW.
///
/// The four indexes are the CLI's canonical identification mechanism, and this
/// serves the same model the terminal draws: `screen_model::build_index` over
/// `Repl::index_placements`, which is why that function was widened rather than
/// copied — its doc defends a leak fence (no other seat's hand, because in a
/// live game an opponent's starting order is hidden and an index that dimmed it
/// with "in Seat 1's hand" would leak the one card the observation boundary
/// exists to protect).
///
/// **`target` is always `None` here, and that is the whole difference between
/// an index the terminal opens and an index this API serves.** In the CLI an
/// index can be open INSIDE a card capture, and then its cards carry selection
/// numbers that answer that capture. This API has no capture — the web's input
/// grammar refuses `?`/`index`/`orders`/`techs` precisely so that a digit never
/// silently changes meaning — so the index is a read-only view and its cards
/// carry no numbers.
///
/// The filter is parsed by the CLI's own `card_prompt::parse_filter`. There is
/// exactly ONE card-entry grammar in this project and the web does not get a
/// second one: a second parser would answer an ambiguous prefix differently from
/// the terminal on the first `f s` a player typed.
async fn card_index(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path((id, kind)): Path<(String, String)>,
    QueryArgs(q): QueryArgs<IndexQuery>,
) -> ApiResult<yokohama_cli::screen_model::IndexModel> {
    use yokohama_cli::screen_model::IndexKind;
    let kind = match kind.as_str() {
        "orders" => IndexKind::Orders,
        "techs" => IndexKind::Techs,
        "goals" => IndexKind::Achievements,
        "sites" => IndexKind::Sites,
        other => {
            return Err(ApiError::bad_request(format!(
                "no card index named {other:?} — the four are orders, techs, goals and sites"
            )))
        }
    };
    let filter = match q.filter.as_deref() {
        None => None,
        Some(term) => yokohama_cli::card_prompt::parse_filter(term, kind)
            .map_err(ApiError::bad_request)?,
    };
    // Seated, and no viewer: `Repl::index_placements`' own doc defends the leak
    // fence this index needs — it never says "in seat N's hand" — so the model
    // is the same for every seat at the table and there is nothing to scope.
    let (handle, _table) = seated(&state, &id, &user)?;
    let repl = handle.lock().expect("a game");
    Ok(Json(yokohama_cli::screen_model::build_index(
        kind,
        repl.session.rules(),
        &repl.index_placements(),
        &yokohama_cli::screen_model::build_achievements(repl.session.state()).cards,
        None,
        filter,
    )))
}

/// Narrows an index. The grammar is the CLI's; an unparseable term is a 400
/// carrying the CLI's own refusal, which names what it would have accepted.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexQuery {
    #[serde(default)]
    pub filter: Option<String>,
}

/// `GET /api/games/{id}/result` — the finished game, ranked, with every seat's
/// score broken into the categories the engine scored it in.
///
/// **A 409 while the game is still running**, not an empty payload: asking for
/// the result of an unfinished game is a client mistake, and `final_result`
/// would answer with a standing nobody has earned.
async fn result(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<outcome::Outcome> {
    let (handle, _table) = seated(&state, &id, &user)?;
    let repl = handle.lock().expect("a game");
    if !repl.session.is_over() {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            message: "the game is not over — there is no final score yet".to_string(),
            table_status: None,
        });
    }
    Ok(Json(outcome::Outcome::of(&repl.session.final_result())))
}

/// Which event `why` should account for. Absent means the most recent bot move.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WhyQuery {
    #[serde(default)]
    pub event: Option<usize>,
}

/// `GET /api/games/{id}/log` — the move trail, worded as it read when it applied.
///
/// The text comes from `Session::event_description`, which describes an event
/// against the state it applied IN and caches the result. Re-describing now
/// would re-price a trading-house take at today's progression and re-address a
/// movement fee to today's president owners — a log that rewrites its own
/// history as the game moves. This route therefore reads the cache and never
/// builds a description.
/// **Seated, and no viewer — stated because it is the one projection where a
/// reader would expect masking.** The CLI's event wording names an order card
/// only at the PUBLIC moments it is named at a table, and words the one secret
/// as "hidden starting order" (M6.1 §5.3). So there is nothing here to scope to
/// a seat, and a per-viewer log would be a second, divergent wording of events
/// that were already described once, against the state they applied in.
async fn log(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    QueryArgs(q): QueryArgs<LogQuery>,
) -> ApiResult<yokohama_cli::screen_model::LogPanel> {
    let (handle, _table) = seated(&state, &id, &user)?;
    let repl = handle.lock().expect("a game");
    let total = repl.session.record().events.len();
    let first = q.tail.map_or(0, |n| total.saturating_sub(n));
    let described: Vec<(usize, String)> =
        (first..total).map(|i| (i, repl.session.event_description(i).to_string())).collect();
    Ok(Json(yokohama_cli::screen_model::build_log(total, described)))
}

/// `GET /api/games/{id}/at/{k}` — the position as it stood after `k` events.
///
/// **A projection, and therefore it SERVES A LOCKED SESSION.** The rule is the
/// one stated beside `why` above: the lockdown gate fences COMPUTATION, not
/// projection. A locked game is precisely the one whose history you want to
/// read — the lockdown means the driver found a fault, and this route is the
/// instrument for finding where. So there is no lockdown check here, and adding
/// one would be the bug.
///
/// **`k` counts events APPLIED, not an event index**, so the domain is
/// `0..=events.len()` INCLUSIVE at both ends: `0` is the post-setup position the
/// first event was decided in, and `len` is the game as it now stands. The track
/// a client draws therefore has `len + 1` stops and the last one names no event.
/// Note the asymmetry with `why`, which addresses an EVENT and so requires
/// `n < len` — a client that calls both at the same stop gets one avoidable
/// refusal at the top.
///
/// **The range is checked HERE, against a length this handler already holds,
/// and every error from `state_at` is then a 500.** The alternative — reading
/// `ReplayError::event_index` to tell past-the-end from a failing event — cannot
/// be done correctly: `ReplayError` is a struct, not an enum, and its own doc
/// says the past-the-end convention is shared with `replay_strict`'s
/// terminal-digest mismatch, so *"a caller branching on `event_index` alone
/// cannot tell those from a last-event failure — the message text is the
/// discriminator."* Checking first makes that ambiguity unreachable rather than
/// navigating it, and it cannot rot.
///
/// **Nothing here indexes `event_description`.** That method is a bare `Vec`
/// index and `k == len` is a legal position with no event at it, so reaching for
/// a description here would panic on a client-supplied number — under the
/// per-game mutex, in a crate with no `CatchPanicLayer`, poisoning that game
/// until it is evicted. The log route already serves the words; this one serves
/// the position.
///
/// **The lock is HELD across the body**, unlike `suggest` and `why`. Those two
/// drop it because they run a multi-second search; this is a replay measured in
/// milliseconds, `input` already holds the same mutex across a bot search AND a
/// store write, and the clone that dropping would require copies every `Move` in
/// the record. Following `why`'s pattern here would cost more than it saves.
async fn at_k(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path((id, k)): Path<(String, usize)>,
) -> ApiResult<client::HistoricalFrame> {
    let (handle, table) = seated(&state, &id, &user)?;
    let repl = handle.lock().expect("a game");
    let total = repl.session.record().events.len();
    if k > total {
        return Err(ApiError::not_found(format!(
            "this game has {total} events, so its last position is {total} — there is no position {k}"
        )));
    }
    // **The pin is the CALLER's seat, at every `k`.** M4 pinned the lowest
    // non-bot seat because an unauthenticated route had nothing better; an
    // authenticated one has the caller's own, which is stable for the same
    // reason AND is the seat the caller is entitled to. So a sweep over
    // `0..=total` still harvests exactly one seat's hands — and now it is the
    // sweeper's own.
    let viewer = viewer_of(&repl, &table, &user)?;
    client::HistoricalFrame::at(&repl, k, viewer).map(Json).map_err(|e| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("replaying this game to {k} failed at event {}: {}", e.event_index, e.message),
        table_status: None,
    })
}

/// The art manifest, or an honest 404.
///
/// **A checkout with no cropped art is a supported state**, not a broken one:
/// the client falls back to drawing what it drew before there was any art. So
/// the absence answers as a refusal with the sentence that explains it — which
/// names the regeneration command, because the one thing a reader needs here is
/// how to make it stop being absent.
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
async fn fallback(uri: axum::http::Uri) -> Response {
    if uri.path().starts_with("/api/") {
        return ApiError::not_found(format!("no such endpoint: {}", uri.path())).into_response();
    }
    if uri.path().starts_with("/art/") {
        return art::serve(uri).await;
    }
    assets::serve(uri).await
}

/// The router, built over a shared state so tests can drive it through
/// `tower::ServiceExt::oneshot` without binding a port.
pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/me", get(me))
        .route("/api/labels", get(labels))
        .route("/api/art", get(art_manifest))
        .route("/api/games", get(list_games).post(create_game))
        .route("/api/games/preview", post(preview_game))
        .route("/api/games/{id}", get(get_game).delete(delete_game))
        .route("/api/games/{id}/table", get(get_table))
        .route("/api/games/{id}/deal", get(deal))
        .route("/api/games/{id}/keep", post(keep))
        .route("/api/games/{id}/input", post(input))
        .route("/api/games/{id}/ports", get(ports))
        .route("/api/games/{id}/labs", get(labs))
        .route("/api/games/{id}/areas", get(areas))
        .route("/api/games/{id}/log", get(log))
        .route("/api/games/{id}/at/{k}", get(at_k))
        .route("/api/games/{id}/suggest", get(suggest))
        .route("/api/games/{id}/why", get(why))
        .route("/api/games/{id}/index/{kind}", get(card_index))
        .route("/api/games/{id}/result", get(result))
        .with_state(state)
        .fallback(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
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
            &[(CLAIMED_HEADER, "not-an-email")],
        )
        .await;
        assert_eq!(
            body["user"],
            serde_json::Value::Null,
            "an invalid header resolves to null here, never 401"
        );
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
            "seed": "20260831", "name": "fenced",
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

    /// The preview route serves the SET TABLE, and withholds the bot's deal.
    ///
    /// **Its second half moved.** This cell used to end by refusing a keep on a
    /// bot seat, which a create body can no longer express — a keep is its own
    /// route now, and its 403 is
    /// [`the_create_flow_deals_per_seat_and_the_state_is_409_until_the_last_keep`]'s.
    /// What is left here is the route's remaining contract: the position after
    /// setup, with one seat's cards absent from the payload rather than filtered
    /// by the client.
    ///
    /// The route itself is retired by Task 10 of this cycle (R-plan-4).
    #[tokio::test]
    async fn the_preview_serves_the_set_table_and_withholds_the_bots_deal() {
        // **Seed 20260830, not the corpus seed**, kept from the era when this
        // cell read the bot's own Keep off the payload: there the argmax is
        // card 2, so a constant could not have passed. The deal left the wire
        // in the 2026-09-06 corrections (R-C1) and what this route now owes is
        // the ABSENCE — asserted on the serialized body, which is the only
        // place a stray field could still appear.
        let body = serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": 20260830,
            "seats": [{ "user": WHO }, { "bot": true }],
        });
        let (status, preview) = post_json("/api/games/preview", body).await;
        assert_eq!(status, StatusCode::OK, "{preview}");
        assert_eq!(preview["table"]["index"], serde_json::json!(0));
        let bot = &preview["deals"][1];
        assert_eq!(bot["is_bot"], serde_json::json!(true), "{bot}");
        assert_eq!(bot["cards"].as_array().expect("an array").len(), 0, "{bot}");
        assert!(!preview.to_string().contains("bot_keep"), "{preview}");
        // Anti-vacuity: the reader's OWN deal is whole, so the route withholds
        // one seat's cards rather than serving an empty set table.
        assert_eq!(preview["deals"][0]["cards"].as_array().expect("two cards").len(), 2);
        assert!(preview["deals"][0]["cards"][0]["keep_score"].is_i64());
    }

    /// A preview this server cannot BUILD is a 500; a request it will not
    /// ACCEPT is still a 400.
    ///
    /// Both halves on one router, because the direction that matters is the
    /// discrimination: a handler that answered 500 to everything would pass the
    /// first assertion. The rules here are `Rules::empty_for_tests()`, which
    /// `config_of` never looks at and `new_game` cannot deal from — a failure
    /// downstream of a request that is perfectly well formed, which is exactly
    /// the case this split exists for.
    #[tokio::test]
    async fn a_preview_the_server_cannot_build_is_a_500_and_a_bad_request_is_still_a_400() {
        let rules = Arc::new(yokohama_core::Rules::empty_for_tests());
        let router = app(Arc::new(AppState {
            rules: Arc::clone(&rules),
            games: Games::new(rules, Arc::new(crate::store::MemStore::default())),
            identity: identity::IdentityMode::Claimed,
        }));
        let good = serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": 20260830,
            "seats": [{ "user": WHO }, { "bot": true }],
        });
        let (status, body) = call(&router, "POST", "/api/games/preview", Some(good.clone())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
        assert!(body["error"].is_string(), "and it carries a sentence: {body}");

        // A seat that is BOTH a person and the bot — the caller's own mistake,
        // refused by name before the setup is ever attempted.
        let mut both = good;
        both["seats"][1] = serde_json::json!({ "user": "other@wire.test", "bot": true });
        let (status, body) = call(&router, "POST", "/api/games/preview", Some(both)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert!(body["error"].as_str().unwrap_or("").contains("seat 1"), "{body}");
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

    /// **Every route that needs a session is a 403 for a signed-in stranger**
    /// (R-1.5), in one cell over the whole table, because a fence applied to
    /// fourteen routes and forgotten on the fifteenth is exactly the shape that
    /// ships.
    ///
    /// `/events` is absent deliberately: that route does not exist yet (M6.1
    /// Task 6 adds it), and a URI with no route answers 404 from the fallback
    /// rather than 403 from a handler.
    #[tokio::test]
    async fn every_session_route_is_403_for_a_signed_in_stranger() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "fenced", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        for uri in [
            "/api/games/fenced",
            "/api/games/fenced/table",
            "/api/games/fenced/ports",
            "/api/games/fenced/labs",
            "/api/games/fenced/areas",
            "/api/games/fenced/log",
            "/api/games/fenced/at/0",
            "/api/games/fenced/suggest",
            "/api/games/fenced/why",
            "/api/games/fenced/index/orders",
            "/api/games/fenced/result",
        ] {
            let (status, body) = get_as(&state, "green@wire.test", uri).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{uri}: {body}");
            assert_eq!(body["error"], "you are not seated at this table", "{uri}");
        }
        let (status, body) = post_as(
            &state,
            "green@wire.test",
            "/api/games/fenced/input",
            serde_json::json!({ "line": "1" }),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

        // Anti-vacuity: a seated caller gets every one of them. Without this a
        // server that refused all fifteen to everybody would pass above.
        for uri in [
            "/api/games/fenced",
            "/api/games/fenced/table",
            "/api/games/fenced/log",
            "/api/games/fenced/at/0",
            "/api/games/fenced/index/orders",
        ] {
            let (status, body) = get_as(&state, "blue@wire.test", uri).await;
            assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        }
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
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "20260831", "name": "flow",
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
            "players": 2, "colors": ["Red", "Blue"], "name": "own",
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
    /// to walk was preview-then-create; the setup flow is now create-then-deal,
    /// and `preview` is retired by the last task of this cycle. What is asserted
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
                "seed": seed.to_string(), "name": name,
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
}
