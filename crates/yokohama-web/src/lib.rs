//! The HTTP surface over the engine — the browser client's whole view of the game.
//!
//! **What this crate is not.** It is not a second frontend. Every model it
//! serves is built by `yokohama_cli::screen_model`, whose module doc calls
//! itself "the seam a later API or GUI layer can consume without a terminal";
//! every word it serves is `yokohama_cli::describe`'s; every move it applies
//! goes through `yokohama_cli::session::Session`, which owns the legal-list
//! gate and the invariant guard. This crate routes, scopes and serializes. If
//! it ever computes something about the game, that is a bug: the duplicate
//! would drift from the terminal, and the two would disagree about a rule.
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

pub mod assets;
pub mod client;
pub mod games;
pub mod store;

use std::sync::Arc;

use axum::extract::{Path, State};
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
use crate::store::{FsStore, MemStore, SessionStore};

/// A refused request, as the client sees it.
///
/// One shape for every failure — `{"error": "..."}` with a status — because a
/// client that has to guess which of several error shapes it received will get
/// it wrong on the path it exercises least. The messages come from the engine
/// and the session layer, which already refuse in sentences.
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, message: message.into() }
    }
    pub fn not_found(message: impl Into<String>) -> Self {
        Self { status: StatusCode::NOT_FOUND, message: message.into() }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(serde_json::json!({ "error": self.message }))).into_response()
    }
}

type ApiResult<T> = Result<Json<T>, ApiError>;

/// Everything a handler may reach.
///
/// `Rules::standard()` is not cheap and is immutable once built, so it is
/// constructed once per process and shared by reference — the same discipline
/// the CLI's `main` uses.
pub struct AppState {
    pub rules: Arc<Rules>,
    pub games: Games,
}

impl AppState {
    /// Backed by a directory on disk.
    pub fn new() -> Self {
        Self::with_store(Arc::new(FsStore::new(store::default_dir())))
    }

    /// Backed by memory — tests, and any run that must leave no trace.
    pub fn in_memory() -> Self {
        Self::with_store(Arc::new(MemStore::default()))
    }

    pub fn with_store(store: Arc<dyn SessionStore>) -> Self {
        let rules = Arc::new(Rules::standard());
        Self { rules: Arc::clone(&rules), games: Games::new(rules, store) }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
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
}

async fn health(State(state): State<Arc<AppState>>) -> Json<Health> {
    Json(Health {
        server_version: env!("CARGO_PKG_VERSION"),
        semantics_rev: SEMANTICS_REV,
        rules_hash: state.rules.content_hash(),
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
    })
}

async fn preview_game(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NewGame>,
) -> ApiResult<games::Preview> {
    games::preview(&req, &state.rules).map(Json).map_err(ApiError::bad_request)
}

async fn list_games(State(state): State<Arc<AppState>>) -> ApiResult<Vec<String>> {
    state.games.list().map(Json).map_err(ApiError::bad_request)
}

async fn create_game(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NewGame>,
) -> Result<(StatusCode, Json<ClientState>), ApiError> {
    let (id, handle) = state.games.create(&req).map_err(ApiError::bad_request)?;
    let repl = handle.lock().expect("a fresh game");
    Ok((StatusCode::CREATED, Json(ClientState::of(&id, &repl, Vec::new()))))
}

async fn get_game(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<ClientState> {
    let handle = state.games.handle(&id).map_err(ApiError::not_found)?;
    let repl = handle.lock().expect("a game");
    Ok(Json(ClientState::of(&id, &repl, Vec::new())))
}

async fn delete_game(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state.games.delete(&id).map_err(ApiError::not_found)?;
    Ok(StatusCode::NO_CONTENT)
}

/// What the client sends to act.
#[derive(Debug, Clone, Deserialize)]
pub struct Input {
    /// An option's `number`, or a sub-prompt token (`confirm`, `cancel`, an
    /// area id, a count). The grammar is the CLI's, which is what makes this
    /// endpoint complete: anything the terminal can express, a client can send.
    pub line: String,
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
async fn input(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<Input>,
) -> ApiResult<ClientState> {
    let handle = state.games.handle(&id).map_err(ApiError::not_found)?;
    let mut repl = handle.lock().expect("a game");

    let before = repl.session.record().events.len();
    let out = repl.step(&req.line);
    if repl.session.record().events.len() != before {
        state.games.persist(&id, &repl).map_err(ApiError::bad_request)?;
    }

    let messages: Vec<String> =
        out.text.lines().map(str::trim_end).filter(|l| !l.is_empty()).map(str::to_string).collect();
    Ok(Json(ClientState::of(&id, &repl, messages)))
}

/// Everything no route claimed.
///
/// **`/api/*` never falls through to the client.** A mistyped endpoint has to
/// answer as the API does — a JSON `{"error": ...}` with a 404 — because
/// serving `index.html` there produces the failure every SPA developer has
/// debugged at least once: the browser reports a syntax error at `<!doctype`,
/// the console blames the client, and the real fault (a wrong path) appears
/// nowhere in the message. Everything else is the client, with the SPA
/// fallback `assets::serve` describes.
async fn fallback(uri: axum::http::Uri) -> Response {
    if uri.path().starts_with("/api/") {
        return ApiError::not_found(format!("no such endpoint: {}", uri.path())).into_response();
    }
    assets::serve(uri).await
}

/// The router, built over a shared state so tests can drive it through
/// `tower::ServiceExt::oneshot` without binding a port.
pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/labels", get(labels))
        .route("/api/games", get(list_games).post(create_game))
        .route("/api/games/preview", post(preview_game))
        .route("/api/games/{id}", get(get_game).delete(delete_game))
        .route("/api/games/{id}/input", post(input))
        .with_state(state)
        .fallback(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn get_json(uri: &str) -> (StatusCode, serde_json::Value) {
        let res = app(Arc::new(AppState::in_memory()))
            .oneshot(Request::builder().uri(uri).body(Body::empty()).expect("a GET"))
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
}
