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
pub mod outcome;
pub mod store;

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

async fn preview_game(
    State(state): State<Arc<AppState>>,
    JsonBody(req): JsonBody<NewGame>,
) -> ApiResult<games::Preview> {
    games::preview(&req, &state.rules).map(Json).map_err(ApiError::bad_request)
}

async fn list_games(State(state): State<Arc<AppState>>) -> ApiResult<Vec<String>> {
    state.games.list().map(Json).map_err(ApiError::bad_request)
}

async fn create_game(
    State(state): State<Arc<AppState>>,
    JsonBody(req): JsonBody<NewGame>,
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
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    JsonBody(req): JsonBody<Input>,
) -> ApiResult<ClientState> {
    if !grammar::is_gameplay_line(&req.line) {
        return Err(ApiError::bad_request(grammar::refusal(&req.line)));
    }
    let handle = state.games.handle(&id).map_err(ApiError::not_found)?;
    let mut repl = handle.lock().expect("a game");

    if let Some(expected) = &req.expect {
        let actual = crate::client::prompt_kind(&repl);
        if expected != actual {
            return Err(ApiError {
                status: StatusCode::CONFLICT,
                message: format!(
                    "this game is asking {actual:?}, not {expected:?} — the prompt moved on \
                     (a restart drops an open picker). Re-read the game and answer what it asks.",
                ),
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
    let mut client = ClientState::of(&id, &repl, messages);
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
/// the borrow, the not-found and the lock live once. A fourth copy of this
/// wiring is a fourth chance to forget the `map_err`.
macro_rules! surface {
    ($name:ident, $model:ty, $build:expr) => {
        async fn $name(
            State(state): State<Arc<AppState>>,
            Path(id): Path<String>,
        ) -> ApiResult<$model> {
            let handle = state.games.handle(&id).map_err(ApiError::not_found)?;
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
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<explain::SuggestResponse> {
    use explain::SuggestRefusal as R;
    let handle = state.games.handle(&id).map_err(ApiError::not_found)?;

    let read = {
        let repl = handle.lock().expect("a game");
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
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    QueryArgs(q): QueryArgs<WhyQuery>,
) -> ApiResult<explain::Why> {
    let handle = state.games.handle(&id).map_err(ApiError::not_found)?;
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
    let handle = state.games.handle(&id).map_err(ApiError::not_found)?;
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
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<outcome::Outcome> {
    let handle = state.games.handle(&id).map_err(ApiError::not_found)?;
    let repl = handle.lock().expect("a game");
    if !repl.session.is_over() {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            message: "the game is not over — there is no final score yet".to_string(),
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
async fn log(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    QueryArgs(q): QueryArgs<LogQuery>,
) -> ApiResult<yokohama_cli::screen_model::LogPanel> {
    let handle = state.games.handle(&id).map_err(ApiError::not_found)?;
    let repl = handle.lock().expect("a game");
    let total = repl.session.record().events.len();
    let first = q.tail.map_or(0, |n| total.saturating_sub(n));
    let described: Vec<(usize, String)> =
        (first..total).map(|i| (i, repl.session.event_description(i).to_string())).collect();
    Ok(Json(yokohama_cli::screen_model::build_log(total, described)))
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
        .route("/api/labels", get(labels))
        .route("/api/art", get(art_manifest))
        .route("/api/games", get(list_games).post(create_game))
        .route("/api/games/preview", post(preview_game))
        .route("/api/games/{id}", get(get_game).delete(delete_game))
        .route("/api/games/{id}/input", post(input))
        .route("/api/games/{id}/ports", get(ports))
        .route("/api/games/{id}/labs", get(labs))
        .route("/api/games/{id}/areas", get(areas))
        .route("/api/games/{id}/log", get(log))
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
            "seed": "20260831", "keeps": [1, 1], "name": "fenced",
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
        let req = Request::builder().method(method).uri(uri);
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

    /// Start a 2P game and return the router it lives in.
    async fn a_game(router: &Router, name: &str, bots: &[u8]) -> serde_json::Value {
        let (status, state) = call(
            router,
            "POST",
            "/api/games",
            Some(serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
                "seed": "20260831", "keeps": [1, 1], "bot_seats": bots, "name": name,
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{state}");
        state
    }

    /// **The explanation FOOTS, and that is the whole claim of the `why` screen.**
    ///
    /// `Explanation`'s own documented identity is
    /// `total == sum(terms.product) + terminal.value`, and it holds only over
    /// ALL 34 terms — which is why the payload carries the zero-product ones
    /// rather than shrinking itself. A chart the reader cannot foot is a picture
    /// rather than an explanation, so the sum is asserted here and re-asserted
    /// on every committed fixture.
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
        let router = app(Arc::new(AppState::with_store(store.clone())));
        a_game(&router, "faulted", &[]).await;

        let stored = store.read("faulted").expect("a stored game");
        let mut record: serde_json::Value = serde_json::from_str(&stored).expect("a record");
        record["lockdown"] = serde_json::json!("invariant violation applying that move");
        store
            .write("faulted", &serde_json::to_string(&record).expect("a record"))
            .expect("a stored lockdown");
        let router = app(Arc::new(AppState::with_store(store)));

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

        let (status, state) = call(
            "/api/games".to_string(),
            serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
                "seed": "20260831", "keeps": [1, 1], "bot_seats": [1], "name": "expects",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
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
    #[tokio::test]
    async fn every_payload_shows_the_hand_of_the_seat_it_offers_moves_for() {
        let router = app(Arc::new(AppState::in_memory()));
        let call = |method: &'static str, uri: String, body: Option<serde_json::Value>| {
            let router = router.clone();
            async move {
                let req = Request::builder().method(method).uri(uri);
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

        // 3P with ONE bot, so seats 0 and 2 are both human — the shape that
        // broke. One bot is what the client's own setup screen sends.
        let mut state = call(
            "POST",
            "/api/games".to_string(),
            Some(serde_json::json!({
                "players": 3, "colors": ["Red", "Blue", "Yellow"], "start_seat": 0,
                "seed": "20260831", "keeps": [1, 1, 1], "bot_seats": [1], "name": "hotseat",
            })),
        )
        .await;

        let mut saw_a_seat_other_than_zero = false;
        for step in 0..60 {
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
            state =
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
                let req = Request::builder().method(method).uri(uri);
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

        let (status, _) = call(
            "POST",
            "/api/games".to_string(),
            Some(serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
                "seed": "20260831", "keeps": [1, 1], "bot_seats": [1], "name": "surfaces",
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

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

    /// A seat's-worth of the hidden set rides on this: the seed a client is
    /// shown must be the seed it can send back.
    ///
    /// The value here is `0xFEDC_BA98_7654_3210` — past 2^53, so a JSON number
    /// would round it, and past `Number.MAX_SAFE_INTEGER` by enough that the
    /// rounding lands on a DIFFERENT seed and therefore a different deal. The
    /// test walks the real flow: preview, read the seed back off the wire, send
    /// exactly that value to create the game, and require the game's own seed to
    /// be the one asked for. A rounding bug shows up as an inequality, not as a
    /// subtly different board nobody notices.
    #[tokio::test]
    async fn a_seed_survives_the_preview_round_trip_past_the_double_limit() {
        const SEED: u64 = 0xFEDC_BA98_7654_3210;
        // The premise, stated rather than assumed: through a double — which is
        // every JSON number a browser parses — this seed is NOT this seed.
        assert_ne!(
            SEED,
            SEED as f64 as u64,
            "a seed that survived an f64 would make this test vacuous",
        );

        let router = app(Arc::new(AppState::in_memory()));
        let post = |uri: &'static str, body: serde_json::Value| {
            let router = router.clone();
            async move {
                let res = router
                    .oneshot(
                        Request::builder()
                            .method("POST")
                            .uri(uri)
                            .header("content-type", "application/json")
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

        let base = serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
            "keeps": [1, 1], "bot_seats": [1],
        });
        let mut req = base.clone();
        req["seed"] = serde_json::json!(SEED.to_string());

        let (status, preview) = post("/api/games/preview", req.clone()).await;
        assert_eq!(status, StatusCode::OK, "{preview}");
        assert_eq!(
            preview["seed"],
            serde_json::json!(SEED.to_string()),
            "the seed comes back as a string, exactly",
        );
        // The dealt ids a player is looking at when they choose.
        let previewed = preview["deals"].clone();

        // What the client sends back is the string it was given, verbatim.
        let mut create = req;
        create["seed"] = preview["seed"].clone();
        create["name"] = serde_json::json!("bigseed");
        let (status, game) = post("/api/games", create).await;
        assert_eq!(status, StatusCode::CREATED, "{game}");
        assert_eq!(game["seed"], serde_json::json!(SEED.to_string()), "{}", game["seed"]);

        // And the game really was dealt from it: the same seed previews the
        // same deal, which is the property a rounded seed breaks.
        let mut again = base;
        again["seed"] = serde_json::json!(SEED.to_string());
        let (_, preview2) = post("/api/games/preview", again).await;
        assert_eq!(preview2["deals"], previewed, "the deal is a function of the seed");
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

        let (status, _) = post(
            "/api/games".to_string(),
            serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
                "seed": 20260831u64, "keeps": [1, 1], "bot_seats": [1], "name": "fence",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

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
}
