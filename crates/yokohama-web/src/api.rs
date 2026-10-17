//! The table layer's one route table (M6.5a, R-6.5a.3). Every host — the axum
//! adapter today, the browser and the Durable Object later — converts its
//! request into an [`ApiRequest`], resolves identity ITS way, and calls
//! [`dispatch`]; the three transport routes (`events`, `stream`, `/api/stream`)
//! and the static routes are the ones a host writes itself.
//!
//! **Nothing here names axum.** The parsers reproduce axum's rejection sentences
//! (R-6.5a.4) by making the SAME calls axum's extractors make — `mime` for the
//! content-type rule, `serde_path_to_error` over `serde_json` for a body and
//! over `serde_urlencoded` for a query — and are pinned against axum's own
//! `body_text()` in lib.rs's server-only cells, so an axum upgrade that rewords
//! is caught rather than absorbed.
//!
//! **Identity is the host's sentence, carried.** `dispatch` takes
//! `Result<&User, &str>`: a game route given `Err(msg)` answers 401 with `msg` in
//! this API's shape, and the axum adapter passes `identity::resolve`'s own
//! per-mode message, so the 401 sweep stays byte-equal through the adapter.
//! The public four (`health`, `me`, `labels`, `art`) take no identity; `me`
//! reports an `Err` as `user: null`.

use http::StatusCode;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use yokohama_core::Rules;

use crate::client::ClientState;
use crate::games::Games;
use crate::identity::User;
use crate::tables::NewGame;
use crate::{chat, client, explain, grammar, identity, outcome, tables, ApiError};

/// A request as the layer sees it: method, path, the raw query string, the
/// content type and the body bytes. A host builds one from whatever it has.
pub struct ApiRequest<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub query: &'a str,
    pub content_type: Option<&'a str>,
    pub body: &'a [u8],
}

/// The answer: a status and a body, the body a JSON string in this API's shape
/// or empty (204, 405, HEAD).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiResponse {
    pub status: u16,
    pub body: String,
}

/// The deployment facts the layer cannot know and a host must tell it: which
/// identity door it has, which change transport it runs, whether it serves the
/// game routes at all, and whether it OFFERS multiplayer.
#[derive(Debug, Clone, Copy)]
pub struct Host {
    pub identity_name: &'static str,
    pub signal_kind: &'static str,
    /// Whether [`routes`] includes the game and chat rows on this host
    /// (M6.5b2, R-6.5b2.4). Off, they are not in the table, so an unknown path
    /// is the table's own 404 and nothing "hides" a route.
    pub serves_games: bool,
    /// Whether this DEPLOYMENT offers multiplayer, which [`Health`] reports.
    ///
    /// **Two facts and not one** (spec §3.1). A local game plays by calling
    /// `/api/games/…` through this same [`dispatch`], so the browser host
    /// SERVES the game routes and OFFERS no multiplayer; axum with the switch
    /// off does neither. A single field would have made single player stop
    /// working the moment a deployment said it had no multiplayer.
    pub multiplayer_offered: bool,
    /// The host's two table TTLs in whole days (M6.5c2 cycle E, R-6.5c2.42),
    /// which [`Health`] reports and nothing in this layer's routes applies:
    /// on the Worker an idle table expires after `table_ttl_days`, and one
    /// still waiting for players after `open_table_ttl_days`, by the shell's
    /// alarm; on the NAS, by its hourly sweep (cycle F, R-F9;
    /// [`crate::sweep`]). `None` on a host that expires no table — the
    /// browser — and both `None` on a NAS or a Worker configured to keep every
    /// table.
    pub table_ttl_days: Option<u64>,
    pub open_table_ttl_days: Option<u64>,
}

/// The caller, or the host's own sentence for why there is none.
pub type Identity<'a> = Result<&'a User, &'a str>;

pub(crate) fn ok_json<T: Serialize>(status: u16, v: &T) -> ApiResponse {
    ApiResponse { status, body: serde_json::to_string(v).expect("every payload serializes") }
}

/// The body of every refusal on this API, byte for byte — `{"error": …}`, with
/// `"status"` beside it on a refusal a client branches on at a table's status,
/// and `"foreign_bot"` beside it on a game played against a bot this build does
/// not run (M6.5c2). `pub` since M6.5c2 (spec §2.6's restatements): a host that
/// answers a refusal ITSELF — the Worker module's `error_body` — writes the
/// layer's bytes rather than a copy of the rule.
pub fn error_body(e: &ApiError) -> String {
    error_value(e).to_string()
}

/// [`error_body`] as a value, for `IntoResponse for ApiError`, whose `Json`
/// serializes it — the same bytes, one rule, THREE arms. (The first draft of
/// this export carried two; the corpus's foreign-bot 409 is what caught it.)
///
/// **The message is cut to [`crate::MESSAGE_MAX`] HERE, at the boundary**
/// (security S1c, decision D-2's second layer): whatever built the error — a
/// constructor, which caps already, or a handler's own struct literal, which
/// does not — its sentence becomes bytes through this function and no other
/// in the layer, so no refusal leaves the layer longer than the cap.
///
/// **The `foreign_bot` pair is capped here too, each string as the message
/// is** (the S1 stack review's HI-F1): its two labels are echoed where they
/// are made ([`crate::bots::label`]), but the knobbed arm of
/// [`crate::bots::distinguishing_labels`] joins every parameter of a record's
/// map, a map the record's writer chose, so a pair can still be long after
/// the echo. Before this a 409 carried a 1,025-character message beside a
/// 20,012-character pair.
pub(crate) fn error_value(e: &ApiError) -> serde_json::Value {
    let mut body = serde_json::json!({ "error": ApiError::capped(e.message.clone()) });
    if let Some(status) = e.table_status {
        body["status"] = status.into();
    }
    if let Some(pair) = &e.foreign_bot {
        let capped = crate::bots::ForeignBot {
            played: ApiError::capped(pair.played.clone()),
            runs: ApiError::capped(pair.runs.clone()),
        };
        body["foreign_bot"] = serde_json::to_value(&capped).expect("the pair serializes");
    }
    body
}

/// The `why` answer as it leaves the layer — the ONE site it is serialized,
/// [`dispatch`]'s arm, which every host answers through: a refusal's
/// `message` cut to [`crate::MESSAGE_MAX`] characters and the ellipsis, as
/// [`error_value`] cuts an error's (the S1 stack review's re-review, Finding
/// 2). A `why` refusal is a 200, not an `ApiError`, so no cap saw it, and the
/// echo bounds each value a label repeats but not how many: a record whose
/// bot ties production's label renders every key of its map, and 30,000 keys
/// read a 270,350-character message. Every refusal the CLI or the layer words
/// is far under the cap, and is written as it always was.
fn why_response(mut w: crate::explain::Why) -> ApiResponse {
    if let Some(refusal) = w.refusal.as_mut() {
        refusal.message = ApiError::capped(std::mem::take(&mut refusal.message));
    }
    ok_json(200, &w)
}

/// The body `IntoResponse for ApiError` writes, byte for byte.
pub(crate) fn error_response(e: ApiError) -> ApiResponse {
    ApiResponse { status: e.status.as_u16(), body: error_body(&e) }
}

fn unauthorized(msg: &str) -> ApiError {
    ApiError { status: http::StatusCode::UNAUTHORIZED, message: msg.to_string(), table_status: None, foreign_bot: None }
}

/// The one sentence a body that is not a JSON OBJECT is refused in (M6.5c2,
/// spec §2.6). Every body this API reads is an object — each `T` is a struct
/// under `deny_unknown_fields` — but serde's derived `Deserialize` ALSO reads a
/// struct from a positional ARRAY, a form `deny_unknown_fields` does not
/// govern: `[2,["Red","Blue"],0,null,[…]]` opened a table. `pub` so a host that
/// forwards a create — the Worker module — refuses in these words, not its own.
pub const NOT_AN_OBJECT: &str = "the request body must be a JSON object";

/// axum 0.8.9's `Json<T>` extractor, call for call (`json.rs`): the content-type
/// rule through `mime`, `serde_path_to_error` over `serde_json`'s deserializer,
/// the error classified into the DATA sentence or the SYNTAX sentence, and
/// `end()` so trailing input is the syntax sentence too. The three sentences
/// are `JsonDataError`'s, `JsonSyntaxError`'s and `MissingJsonContentType`'s.
/// And one rule of the layer's own, last: an object only (M6.5c2) — the one
/// place its answer departs from axum's, and only where axum's is a table
/// opened from a list.
pub fn parse_json<T: DeserializeOwned>(content_type: Option<&str>, body: &[u8]) -> Result<T, ApiError> {
    let is_json = content_type
        .and_then(|ct| ct.parse::<mime::Mime>().ok())
        .map(|m| {
            m.type_() == "application" && (m.subtype() == "json" || m.suffix().is_some_and(|s| s == "json"))
        })
        .unwrap_or(false);
    if !is_json {
        return Err(ApiError::bad_request("Expected request with `Content-Type: application/json`"));
    }
    let mut de = serde_json::Deserializer::from_slice(body);
    let value = serde_path_to_error::deserialize::<_, T>(&mut de).map_err(|e| {
        ApiError::bad_request(match e.inner().classify() {
            serde_json::error::Category::Data => {
                format!("Failed to deserialize the JSON body into the target type: {e}")
            }
            serde_json::error::Category::Syntax
            | serde_json::error::Category::Eof
            | serde_json::error::Category::Io => format!("Failed to parse the request body as JSON: {e}"),
        })
    })?;
    de.end().map_err(|e| ApiError::bad_request(format!("Failed to parse the request body as JSON: {e}")))?;
    // **The object form only** (M6.5c2, spec §2.6), and checked LAST, after
    // serde has read the body, so every body axum refuses keeps axum's sentence
    // word for word (R-6.5a.4): this refuses only what axum ACCEPTS — a
    // positional array serde read a struct from. A body that parsed whole, with
    // nothing after it, is one JSON value, and its first byte past JSON's own
    // whitespace says which kind.
    if body.iter().find(|b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r')) != Some(&b'{') {
        return Err(ApiError::bad_request(NOT_AN_OBJECT));
    }
    Ok(value)
}

/// axum 0.8.9's `Query<T>`, call for call (`extract/query.rs`): `form_urlencoded`
/// parses, `serde_urlencoded`'s deserializer reads, `serde_path_to_error` names
/// the field — which is why the sentence carries a path (`limit: invalid digit
/// …`) and a plain `serde_urlencoded::from_str` would not.
pub fn parse_query<T: DeserializeOwned>(query: &str) -> Result<T, ApiError> {
    let de = serde_urlencoded::Deserializer::new(form_urlencoded::parse(query.as_bytes()));
    serde_path_to_error::deserialize::<_, T>(de)
        .map_err(|e| ApiError::bad_request(format!("Failed to deserialize query string: {e}")))
}

/// One row of the route table.
pub struct Route {
    pub method: &'static str,
    pub pattern: &'static str,
}

/// `app()`'s table, in its order (R-6.5a.3), minus the three transport routes a
/// host writes itself. `/api/art` is listed so the 405/404 arithmetic is one
/// table, but its body lives in the server (art is gated) and the adapter
/// answers it before calling [`dispatch`].
///
/// **`serves_games` removes rows rather than gating them** (M6.5b2, R-6.5b2.4):
/// a host that does not serve games has no game or chat row at all, so every
/// such path is the same `no such endpoint` 404 a misspelling is, and there is
/// no second sentence to leak which of the two it was. `/api/archive` is NOT
/// one of them — it has its own variable (spec §3.1), so a single-player
/// deployment can still collect saves — and neither are the public four.
pub fn routes(serves_games: bool) -> Vec<Route> {
    [
        ("GET", "/api/health"),
        ("GET", "/api/me"),
        ("GET", "/api/labels"),
        ("GET", "/api/art"),
        ("GET", "/api/games"),
        ("POST", "/api/games"),
        ("GET", "/api/games/{id}"),
        ("DELETE", "/api/games/{id}"),
        ("GET", "/api/games/{id}/table"),
        ("GET", "/api/games/{id}/deal"),
        ("POST", "/api/games/{id}/keep"),
        ("POST", "/api/games/{id}/join"),
        ("POST", "/api/games/{id}/unlock"),
        ("GET", "/api/games/{id}/chat"),
        ("POST", "/api/games/{id}/chat"),
        ("DELETE", "/api/games/{id}/chat/{seq}"),
        ("POST", "/api/games/{id}/input"),
        ("GET", "/api/games/{id}/ports"),
        ("GET", "/api/games/{id}/labs"),
        ("GET", "/api/games/{id}/areas"),
        ("GET", "/api/games/{id}/log"),
        ("GET", "/api/chat"),
        ("POST", "/api/chat"),
        ("DELETE", "/api/chat/{seq}"),
        ("POST", "/api/archive"),
        ("GET", "/api/games/{id}/at"),
        ("GET", "/api/games/{id}/at/{k}"),
        ("GET", "/api/games/{id}/suggest"),
        ("GET", "/api/games/{id}/why"),
        ("GET", "/api/games/{id}/index/{kind}"),
        ("GET", "/api/games/{id}/result"),
        ("GET", "/api/games/{id}/record"),
    ]
    .into_iter()
    .filter(|(_, pattern)| serves_games || !is_game_route(pattern))
    .map(|(method, pattern)| Route { method, pattern })
    .collect()
}

/// The routes the NAS answers behind its replay gate (security S1b, findings
/// A2 and D6): the ones that replay a record or run a search — an archive
/// upload's strict replay, a run or a position of a game's history, the two
/// searches. `input` is NOT among them: a move must land while a burst of
/// replays waits, and an undo folds from the session's own ladder.
pub const REPLAY_ROUTES: [(&str, &str); 5] = [
    ("POST", "/api/archive"),
    ("GET", "/api/games/{id}/at"),
    ("GET", "/api/games/{id}/at/{k}"),
    ("GET", "/api/games/{id}/suggest"),
    ("GET", "/api/games/{id}/why"),
];

/// Whether `method` and `path` name one of [`REPLAY_ROUTES`] — by the table's
/// own match, with the PATTERN as the key. The method is `find`'s to match,
/// and it folds HEAD onto GET as [`dispatch`] answers it (the body dropped,
/// the replay paid), so a HEAD is gated with its GET. A wrong method on a
/// gated path is `WrongMethod` and a path the table does not route is
/// `NoRoute`, and neither is gated. The method column of [`REPLAY_ROUTES`]
/// names the row each pattern routes: each routes that one method alone.
pub fn is_replay_route(method: &str, path: &str) -> bool {
    match find(method, path, true) {
        Matched::Route(pattern, _) => REPLAY_ROUTES.iter().any(|(_, p)| *p == pattern),
        Matched::WrongMethod | Matched::NoRoute => false,
    }
}

/// The rows the multiplayer switch takes away: every game route and both house
/// chat routes. A prefix test and not a list, so a route added under
/// `/api/games` is governed by the switch the day it lands rather than the day
/// someone remembers to add it here.
fn is_game_route(pattern: &str) -> bool {
    pattern.starts_with("/api/games") || pattern.starts_with("/api/chat")
}

/// Segment-wise match; `{name}` binds one NON-EMPTY segment. Returns the bound
/// values in pattern order. The ids this API accepts are `store::is_safe_id`'s
/// ASCII set, so a bound segment needs no percent-decoding.
///
/// **An empty segment binds nothing** (M6.5c2 cycle D, straw poll row 10), as
/// axum's matchit reads it: `/api/games//table` is no route, a 404. Bound, it
/// reached `get_table` and came back as the store's "not a usable game name",
/// a 500 — the server blamed for a path the caller mistyped — and since
/// security S1c an id the store would refuse is `dispatch`'s own 404 before
/// any handler (finding B8).
fn matches<'a>(pattern: &str, path: &'a str) -> Option<Vec<&'a str>> {
    let mut bound = Vec::new();
    let mut ps = pattern.split('/');
    let mut xs = path.split('/');
    loop {
        match (ps.next(), xs.next()) {
            (None, None) => return Some(bound),
            (Some(p), Some(x)) if p.starts_with('{') && !x.is_empty() => bound.push(x),
            (Some(p), Some(x)) if p == x => {}
            _ => return None,
        }
    }
}

/// The methods the table takes at `path`, in table order, with `HEAD` beside
/// `GET` as axum's `MethodRouter` lists them — what the adapter writes into a
/// 405's `Allow` header. Empty for a path no row matches.
///
/// It takes the host's `serves_games` for the same reason `find` does: a 405
/// and its `Allow` are answers ABOUT the table, so they have to be read off the
/// table this host actually serves.
pub fn allowed_methods(path: &str, serves_games: bool) -> Vec<&'static str> {
    let mut out = Vec::new();
    for r in routes(serves_games) {
        if matches(r.pattern, path).is_some() {
            out.push(r.method);
            if r.method == "GET" {
                out.push("HEAD");
            }
        }
    }
    out
}

enum Matched<'a> {
    Route(&'static str, Vec<&'a str>),
    WrongMethod,
    NoRoute,
}

/// HEAD reaches the GET arm (R-6.5a.3 — axum's `get` serves HEAD that way);
/// [`dispatch`] strips the body afterwards.
fn find<'a>(method: &str, path: &'a str, serves_games: bool) -> Matched<'a> {
    let method = if method == "HEAD" { "GET" } else { method };
    let mut path_known = false;
    for r in routes(serves_games) {
        if let Some(bound) = matches(r.pattern, path) {
            path_known = true;
            if r.method == method {
                return Matched::Route(r.pattern, bound);
            }
        }
    }
    if path_known { Matched::WrongMethod } else { Matched::NoRoute }
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
    /// Which change transport to read: the server's `SIGNAL_KIND` (M6.3, R-plan-4).
    ///
    /// The second such fact, and it is here for the same reason as the first:
    /// the client's signal hook has to know whether to read `…/stream` as a
    /// stream (`"sse"`, the NAS) or as a socket (`"ws"`, the Worker), and it
    /// cannot tell from a successful call to anything else. Its one plain
    /// `…/events` read is the socket's probe; since M6.5c2 cycle D (R-D5) no
    /// client holds a request, and a `wait` is accepted and ignored up to
    /// [`MAX_WAIT_SECONDS`] — above it, still a 400.
    pub signal: &'static str,
    /// Which archive this deployment writes finished and abandoned games to
    /// (M6.5a, R-6.5a.13): `"off"`, `"dir"`, or M6.5c's `"r2"`. The client's
    /// Abandon control reads it for its label, and nothing else does.
    pub archive: &'static str,
    /// Whether the house chat routes answer (M6.4, R-6.4.6). Off, the client
    /// draws no lobby panel and asks nothing.
    ///
    /// **Since M6.5b2 it is the chat switch AND [`Host::serves_games`]**, which
    /// is the predicate the sentence above always claimed: `/api/chat` is one
    /// of the rows the multiplayer switch removes, so a deployment with the
    /// switch off served no house chat while reporting that it did — and a
    /// client believing it drew a panel whose every fetch was a 404.
    pub lobby_chat: bool,
    /// Messages retained per chat document; `null` for `all` (R-6.4.8).
    pub chat_keep: Option<u64>,
    /// Characters per message, so the client sizes its input from the
    /// server's rule and never from a copy (R-6.4.8).
    pub chat_text: u64,
    /// Whether this deployment offers multiplayer (M6.5b2, R-6.5b2.4).
    ///
    /// [`Host::multiplayer_offered`] and never `serves_games`: the browser host
    /// serves every game route — a local game is played through them — and
    /// reports `false` here, because there is no one else at the table.
    ///
    /// **A deployment reporting `false` still SERVES `/api/me` and this route**
    /// (whole-branch review, Minor 12). Spec §2.1 lists `/api/me` among the
    /// protected paths and §3.1 lists what the switch removes — the game, chat
    /// and stream rows — and the two lists disagree about this one. §3.1 is
    /// what is built, deliberately: the identity read is how a browser learns
    /// there is nobody to be, and answering it costs nothing a single-player
    /// deployment wants to hide.
    pub multiplayer: bool,
    /// How many days an idle table stands before it expires (M6.5c2 cycle E,
    /// R-6.5c2.42): [`Host::table_ttl_days`]. `null` where no table expires —
    /// the browser, and a NAS or a Worker with no TTL set — sent as `null`
    /// rather than left out, so a client reads "no TTL", never "a server too
    /// old to say".
    pub table_ttl_days: Option<u64>,
    /// How many days a table still WAITING for players stands before it is
    /// closed (R-6.5c2.42): [`Host::open_table_ttl_days`]; `null` as above.
    pub open_table_ttl_days: Option<u64>,
}

pub fn health(games: &Games, host: &Host) -> Health {
    Health {
        server_version: env!("CARGO_PKG_VERSION"),
        semantics_rev: yokohama_core::SEMANTICS_REV,
        rules_hash: games.rules().content_hash(),
        identity: host.identity_name,
        signal: host.signal_kind,
        archive: games.archive.kind(),
        lobby_chat: games.chat.lobby && host.serves_games,
        chat_keep: games.chat.limits.keep.map(|n| n as u64),
        chat_text: games.chat.limits.text as u64,
        multiplayer: host.multiplayer_offered,
        table_ttl_days: host.table_ttl_days,
        open_table_ttl_days: host.open_table_ttl_days,
    }
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
/// The one route that reads the identity WITHOUT refusing on its absence,
/// because a 401 is precisely the case this route exists to report. A client
/// with no identity yet needs a fact back: the mode, so it knows whether a
/// sign-in screen is even meaningful here, and `user`, which is `null` when
/// nobody is named.
pub fn me(host: &Host, identity: Identity<'_>) -> Me {
    Me { mode: host.identity_name, user: identity.ok().map(|u| u.as_str().to_string()) }
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

pub fn labels(games: &Games) -> Labels {
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
    for card in games.rules().techs.iter_sorted_by_id() {
        tech_effects
            .insert(key_of(&card.effect), describe::tech_effect_text(card.effect).to_string());
    }
    let mut achievements = std::collections::BTreeMap::new();
    for a in achievement_registry() {
        achievements
            .insert(key_of(&a.achievement_type), describe::achievement_desc(a.achievement_type));
    }

    Labels {
        area_types,
        phases,
        phase_chain: PHASE_CHAIN.iter().map(|p| describe::phase_chain_name(*p).to_string()).collect(),
        tech_effects,
        achievements,
        features: crate::explain::feature_labels(),
    }
}

// **`identity::Identity` is on all twenty-three `/api/games` handlers**, and
// since M6.1 §5.3 every one of them USES the `User` it carries: a game route
// resolves the caller to a seat before it touches the `Repl`, and builds what it
// serves for that seat. A caller with no seat here is a 403 (R-1.5) rather than
// a payload, and a dealing table is a 409 carrying a status a client can branch
// on rather than a 404 it cannot.
//
// Twenty-three handlers over twenty-one bodies — `ports`, `labs` and `areas` are
// three handlers from the one `surface!` macro below. **The number was stale
// before M6.4 touched it** (it said eighteen, and M6.2's `join` and `unlock` and
// M6.3's `stream` had already landed), which is what a count in a comment does;
// it is recounted here rather than incremented. The HOUSE pair is not among
// them: `/api/chat` is not a game route and has no table to scope.

/// **Who is asking, and may they read this table at all** (M6.3, spec §3.5).
///
/// Every session route starts here, and it is M6.1's `seated` widened by
/// exactly one case: a signed-in caller with no seat, at a table that is PLAYING or
/// finished, whose lock they have presented (or which has none), is admitted as
/// a [`client::Viewer::Spectator`]. Nothing else moves — a stranger at a
/// `dealing` table is refused exactly as before, because nobody watches a deal,
/// and that is the moment the other seats' cards are in flight.
///
/// **It returns NO handle**, which is the second difference and the reason
/// `events` and `stream` can share it: a table that is open or dealing has no
/// record, and a function that fetched one would have to refuse those two
/// before the routes that must answer them got a look. A route that needs a
/// session asks [`Games::handle`](crate::games::Games::handle) afterwards and gets the same 409 it
/// always did.
///
/// **The table is read FIRST, as `seated` read it**, so a stranger holding an
/// id learns that it is not theirs before learning anything about what is
/// happening at it.
///
/// **The seat holder's arm takes the handle and drops it**, because
/// [`client::viewer_seat`] needs the acting seat to answer "the acting seat
/// when the caller holds it" — which is what `seated` and `viewer_of`, the two
/// functions this replaces, did between them. At `open` and `dealing` there is no session to ask, and every
/// route that reaches a handle from there refuses with the status first, so the
/// caller's lowest seat stands in — and since M6.4 the chat post READS it:
/// before the deal a multi-seat caller posts as their lowest seat, and once the
/// table is playing as the acting seat WHEN THEY HOLD IT, else again as their
/// lowest (spec §3.2, [`client::viewer_seat`]). Pinned by
/// `a_two_seat_caller_posts_as_the_lowest_seat_before_the_deal_and_the_acting_one_after`.
///
/// **The seat it answers was read under ITS OWN lock take** and is stale the
/// moment that take ends: a route that takes the game's lock again to ACT
/// reads the seat again under that take ([`input`], M6.5c2 cycle D, row 129).
///
/// The read and the admission are two steps since M6.5c2 cycle D: this is
/// [`admitted_with`] over one [`Games::table_for_anyone`] read, for a route
/// that has no table in hand.
pub fn admitted(
    games: &Games,
    id: &str,
    user: &identity::User,
) -> Result<(crate::table::TableDoc, client::Viewer), ApiError> {
    let table = games.table_for_anyone(id, user)?;
    admitted_with(games, &table, id, user)
}

/// [`admitted`]'s rule over a table the CALLER has already read (M6.5c2 cycle
/// D, row 129) — so a route that needs the document before it knows whether to
/// admit (`events_at`, which answers an open table itself) reads it ONCE.
///
/// `table` must be what [`Games::table_for_anyone`] answered for this `id` and
/// `user`: that read is the first half of the admission (a stranger at a deal
/// is refused there), and this function is the second. It answers the table
/// back beside the viewer, so a call site reads as `admitted`'s does.
///
/// **It reads no table document, the handle's cold rebuild included**: the
/// seat holder's arm reaches the handle through `Games::live_handle`, which
/// takes the table in hand rather than settling it again — the read
/// [`Games::handle`] would make.
pub fn admitted_with(
    games: &Games,
    table: &crate::table::TableDoc,
    id: &str,
    user: &identity::User,
) -> Result<(crate::table::TableDoc, client::Viewer), ApiError> {
    use crate::table::TableStatus as S;
    let table = table.clone();
    if !table.seats_of(user).is_empty() {
        let seat = match table.status {
            S::Playing => {
                let handle = games.live_handle(id, &table)?;
                let repl = games.lock_game(&handle, id)?;
                client::viewer_seat(&table, user, repl.session.state())
                    .expect("a seat holder has a viewer seat")
            }
            S::Open | S::Dealing => table.seats_of(user)[0],
        };
        return Ok((table, client::Viewer::Seat(seat)));
    }
    match table.status {
        // `table_for_anyone` already refuses a stranger at `dealing`; `open`
        // reaches here because `GET …/{id}/table` is allowed to answer it
        // (R-2.7) and every other route is not.
        S::Open | S::Dealing => Err(ApiError {
            status: StatusCode::FORBIDDEN,
            message: NOT_SEATED.to_string(),
            table_status: None, foreign_bot: None,
        }),
        S::Playing if table.can_watch(user) => Ok((table, client::Viewer::Spectator)),
        // **The lock gates watching and says so**, rather than reusing the
        // not-seated sentence: this caller CAN do something about it, and the
        // client draws the unlock face from exactly this answer.
        S::Playing => Err(ApiError {
            status: StatusCode::FORBIDDEN,
            message: "this table is locked — unlock it first".to_string(),
            table_status: None, foreign_bot: None,
        }),
    }
}

/// The one sentence a route answers a caller with no business at this table
/// with, written once so the routes that share it cannot drift apart.
///
/// **It is not the only refusal a stranger earns, and the sets are more than
/// one BY DESIGN (R-4.4).** At a locked playing table the writes that resolve
/// the caller inside `Games` rather than through `admitted` — `deal` and the
/// table's `DELETE` — answer this, while every READ answers "this table is
/// locked — unlock it first", because a reader can do something about a lock
/// and a writer cannot. **`keep` is in neither set**: it answers its own older
/// sentence, "seat N is not yours", at every status, which names a SEAT and
/// never a status and so tells a stranger nothing the other two do not. The
/// stranger sweep pins all three route by route, so a refusal moving between
/// the sets is a failing cell rather than a silent widening.
pub(crate) const NOT_SEATED: &str = "you are not seated at this table";

/// The refusal a WRITE route gives an admitted spectator: the same sentence a
/// caller with no seat has always heard, because that is what they are.
fn spectators_write_nothing(viewer: client::Viewer) -> Result<(), ApiError> {
    match viewer {
        client::Viewer::Seat(_) => Ok(()),
        client::Viewer::Spectator => Err(ApiError {
            status: StatusCode::FORBIDDEN,
            message: NOT_SEATED.to_string(),
            table_status: None, foreign_bot: None,
        }),
    }
}

/// Who holds `seat` at this table, for a refusal that names them — by the
/// seat's NAME, or by its COLOUR when its holder gave none, and NEVER by
/// address (M6.5c2 cycle D, R-6.5c2.35, R-D3): the sentence goes to every
/// other seated caller who presses a key out of turn, and it used to hand them
/// the acting seat's email.
///
/// A bot is named as what it is, as it always was. The colour's `None` arm is
/// the same unreachable-by-construction shape as [`admitted`]'s viewer seat:
/// `Games::read_table` refuses a document whose `colors` or `seats` disagrees
/// with its `players`.
fn holder_of(table: &crate::table::TableDoc, seat: yokohama_core::PlayerId) -> String {
    let i = usize::from(seat.0);
    match table.seats.get(i) {
        Some(crate::table::SeatDoc::Human { name: Some(name), .. }) => name.clone(),
        Some(crate::table::SeatDoc::Human { name: None, .. }) => table
            .colors
            .get(i)
            .map_or_else(|| format!("seat {i}"), |c| yokohama_cli::render::color_name(*c).to_string()),
        _ => "the bot".to_string(),
    }
}

/// The sentence a refusal carries when the GAME'S OWN STATUS is the reason a
/// caller cannot act — the lockdown's reason, or the CLI's own
/// `ApplyError::GameOver` line, in `client::prompt_of`'s precedence so a game
/// that is both faulted and finished reports the fault. `None` on a live game,
/// where the reason is about the caller instead.
///
/// **Single-sourced because TWO fences in [`input`] need it and they must not
/// disagree** (R-4.3 with R-5.1). R-4.3 ruled that a non-acting caller at a
/// finished game hears the game's own sentence rather than one about seats;
/// R-5.1 then put R-P6's undo rule in FRONT of that fence. Without this, a
/// watcher's `undo` at a finished game started answering "undo is not offered
/// once your turn has passed" — still a 403, still no rewind, but a ruled
/// sentence overturned by an ordering change rather than by a decision.
///
/// The two readers are [`undo_allowed`], which refuses outright at either
/// status (R-5.2), and the acting-seat fence, which uses it in place of naming
/// a seat. `None` means the game is live and the reason is about the caller.
pub(crate) fn status_refusal(repl: &yokohama_cli::repl::Repl) -> Option<String> {
    repl.session.record().lockdown.clone().or_else(|| {
        repl.session.is_over().then(|| yokohama_cli::session::ApplyError::GameOver.to_string())
    })
}

/// **How far back an undo may reach** (R-P6), checked before `Repl::step`.
///
/// ONE distinct person at the table — vs-bot, or one person sitting in every
/// human seat — keeps today's undo untouched: the rule exists to stop somebody
/// erasing ANOTHER PERSON's move, and where there is no other person there is
/// nothing to protect. Fencing there would also break the only mode this server
/// has shipped, since the trailing event at a vs-bot table is the bot's and its
/// mover is never the caller.
///
/// Otherwise the game must still be RUNNING — finished or faulted, it is not
/// rewound by anybody (R-5.2: a finished game has no turn in progress for
/// anyone, so nobody's turn is incomplete, and one player reopening a game the
/// other has watched end is what R-P6 exists to stop; a faulted record is the
/// one thing here that exists to be read afterwards) — and the undo may then
/// reach back into the caller's own turn and no further: the trailing event's
/// mover is the caller's seat AND the current player is still that seat.
///
/// **The two halves refuse different callers.** Without the MOVER half, the
/// seat that is to move pops whatever the PREVIOUS player did last — measured
/// on this server before the rule: blue's `undo`, sent before blue had moved at
/// all, returned 200 and popped red's move. Without the CURRENT-PLAYER half, a
/// caller whose own move is the trailing event undoes it after their turn has
/// already ended, which is the case R-P6 closes by name.
///
/// **This runs BEFORE the acting-seat fence** (R-5.1), which is what makes the
/// second half reachable: the fence would otherwise refuse that caller first,
/// with a sentence about whose turn it is instead of about the button they
/// pressed. Both sentences are true and neither mutates, so the order chooses
/// only which one is heard. The fence still runs afterwards, for this line and
/// every other — where this function waves a line through (a table with one
/// person at it) the fence is what guards, exactly as before.
///
/// **The mover of event `len - 1` is the state before it, `.current_player` —
/// replay-derived** ([`trailing_state`], read from the session's ladder since
/// security S1b; the state `state_at(record, len - 1)` names), which is the
/// CLI's own attribution for exactly this question (`repl.rs`'s undo unwinds
/// by replay, not by a boundary stack, so it survives a save and reload). A
/// `RecordedEvent` carries no seat of its own — nor does the `Move` inside one
/// — and a second attribution here would be a second thing to keep true.
///
/// **A record that will not replay is the SERVER's failure, not a refusal**
/// (M6.5c2 cycle D, straw poll row 5): [`UndoRefusal::Replay`], a 500 carrying
/// the replay's own sentence. It used to be folded into the refusal's `String`
/// and answered 403, which tells a caller the rule turned them away when no
/// rule was consulted. No sound record reaches that arm — a resident `Repl`
/// was built by folding the same chain over the whole record — so it is a
/// server fault by construction, which is exactly what a 500 says.
///
/// What pops is `Session::undo`'s unit — one `Move` with the reveals bound to
/// it, or a trailing standalone reveal — and never more: admission requires
/// that event to be the caller's own, so a bot run can never be sitting in
/// front of it, even at a mixed table where one seat is a bot.
///
/// **And at a table of more than one person, a unit that drew from a hidden
/// deck is not rewound** (R-SEC8(a), 2026-10-08): the trailing state's decks
/// against the current state's, refused with [`UNDO_DREW`]. A solo table —
/// one person, or one person and a bot — keeps today's undo, bot run and all:
/// the rule is about a card another PERSON never saw.
pub(crate) fn undo_allowed(
    repl: &mut yokohama_cli::repl::Repl,
    table: &crate::table::TableDoc,
    viewer: yokohama_core::PlayerId,
) -> Result<(), UndoRefusal> {
    // **Nobody undoes a game played against a bot this build does not run**
    // (M6.5c2, R-6.5c2.10; spec §2.4's third mask), on the one-human path and
    // the shared path alike, and so FIRST. Since cycle F (spec §2.3) which
    // bots those are is the CLI's own resume's answer ([`crate::bots::foreign`]):
    // a seat it does not seat exactly, for the CLI's reason. Such a game
    // reaches here only finished (`Games::live_handle` refuses one that is
    // not), and an undo would give its bot's seat back to something other than
    // the bot that played it: the resume plays a drifted seat on under the
    // CURRENT build and makes an unknown one a human seat, so the undo would
    // either let a different bot move where the recorded one moved, or pop the
    // bot's final move as a human's and leave a game nobody can play. The bot
    // is named by the `played` half of the refusal's pair (R-C5), so a
    // same-name re-point reads "V4.2", never "V4" on a server that runs V4;
    // and the CLI's reason follows it, as on every foreign-bot refusal.
    if let Some((_, played, why)) = crate::bots::foreign(repl.session.record()).first() {
        return Err(UndoRefusal::Refused(format!(
            "undo is not offered on a game played against {}, which this server doesn't run: {why}",
            crate::bots::distinguishing_labels(played, &crate::bots::production()).0,
        )));
    }
    if table.distinct_humans() <= 1 {
        return Ok(());
    }
    // **A game that is FINISHED or FAULTED is not rewound by anybody** (R-5.2),
    // the seat that is to move included, and so BEFORE the conjunction below —
    // which can hold at either status, since the state still names a current
    // player and the trailing event can be that player's. Measured on the code
    // without this arm: at a finished two-person game the acting holder's
    // `undo` returned 200 and took the record from 357 events to 200; at a
    // faulted one it returned 200 with a refusal in prose, while the watcher
    // beside them got a 403 — two callers, one game, two different answers.
    //
    // The sentence is the GAME's own status rather than R-P6's, from the same
    // helper the acting-seat fence reads, so the two cannot drift (R-4.3 with
    // R-5.1). It returns `None` on a live game, which is where R-P6 speaks.
    if let Some(status) = status_refusal(repl) {
        return Err(UndoRefusal::Refused(status));
    }
    let before = trailing_state(&mut repl.session)?;
    let now = repl.session.state();
    if before.current_player != viewer || now.current_player != viewer {
        return Err(UndoRefusal::Refused("undo is not offered once your turn has passed".to_string()));
    }
    // **A unit that DREW from a hidden deck stays** (R-SEC8(a), finding B2):
    // the decks of the state before the trailing event against the decks now
    // — shorter now means the event drew, and the mover has seen the card.
    // The unit `Session::undo` pops is that event with its bound reveals; a
    // record this layer deals carries no reveals (the grammar refuses
    // `reveal`, and the CLI captures bound reveals only in live mode); one
    // that arrives from elsewhere — a backup restored in the browser, or a
    // file placed in a store — can end on a standalone starting-order reveal,
    // which can take its card out of the order deck (`extract_order_from_deck`
    // searches it first) and so reads here as a draw: refused, the safe side.
    // AFTER the turn test, so a passed turn keeps its own sentence (R-P6).
    if before.order_deck.len() > now.order_deck.len() || before.tech_deck.len() > now.tech_deck.len() {
        return Err(UndoRefusal::Refused(UNDO_DREW.to_string()));
    }
    Ok(())
}

/// The sentence an undo whose unit DREW from a hidden deck is refused with at a
/// table of more than one person (the 2026-10-08 ruling R-SEC8(a), finding
/// B2). A port take refills the port from the order deck and a lab acquire the
/// lab from the tech deck, inside the move itself (`game/actions/cards.rs`), so
/// the mover has read the deck's top; undoing would put the card back where the
/// other seats never saw it, which physical play cannot do either.
pub(crate) const UNDO_DREW: &str =
    "undo is not offered on a move that drew from a deck — the card it showed cannot be un-seen at a shared table";

/// Why [`undo_allowed`] did not wave an undo through — two different answers,
/// typed so the caller cannot give them one status.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum UndoRefusal {
    /// The RULE refused this caller: the foreign-bot rule, R-5.2's finished or
    /// faulted game, "nothing to undo", R-P6 itself, or the un-draw rule
    /// (R-SEC8(a), [`UNDO_DREW`]). A 403, in the rule's own words — the four
    /// older ones exactly as they have always answered.
    Refused(String),
    /// The record would not replay far enough to name who made its last move:
    /// a 500 carrying the replay's sentence (`Session::state_before`'s, which
    /// names the event that failed).
    Replay(String),
}

/// **The one place the two arms become statuses**: a refusal is
/// [`GameError::Forbidden`](crate::games::GameError::Forbidden) and a replay
/// failure [`GameError::Server`](crate::games::GameError::Server), which the
/// crate root's `From<GameError>` turns into a 403 and a 500.
impl From<UndoRefusal> for crate::games::GameError {
    fn from(e: UndoRefusal) -> Self {
        match e {
            UndoRefusal::Refused(m) => Self::Forbidden(m),
            UndoRefusal::Replay(m) => Self::Server(m),
        }
    }
}

/// The state BEFORE the session's last event — whose `current_player` is who
/// made that event (see [`undo_allowed`] on why by replay), and whose decks are
/// what the event drew from.
///
/// **Read from the session's ladder** (`Session::state_before`; security S1b,
/// finding A3): the state `state_at(record, len - 1)` names, by the core PR's
/// proof, folded from the nearest kept state through the predicate. It was
/// `state_at` itself — a checked replay from setup through the legal list
/// materialized per event — on every undo asked of a live game at a table of
/// more than one person, which at a record built to a large retreat family is the cost A3 exists to
/// remove. `&mut` because a read past a gap keeps the multiples it folds past.
fn trailing_state(
    session: &mut yokohama_cli::session::Session,
) -> Result<yokohama_core::GameState, UndoRefusal> {
    // Not R-P6 speaking: the guard that keeps `len - 1` from underflowing, on
    // a game whose first prompt has not been answered yet. `Session::undo`'s
    // own words for the same condition, so the answer reads the same whichever
    // of the two paths reaches it.
    let Some(last) = session.record().events.len().checked_sub(1) else {
        return Err(UndoRefusal::Refused("nothing to undo".to_string()));
    };
    session.state_before(last).map_err(UndoRefusal::Replay)
}

/// What `GET /api/games` is being asked (§3.3).
///
/// `deny_unknown_fields` through the layer's `parse_query` (the same fence
/// `QueryArgs` was): `?scoped=open` would otherwise default silently to `mine` and answer
/// a lobby asking for open tables with the caller's own — confidently, and with
/// something it did not ask for.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListQuery {
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub before: Option<String>,
}

/// The lobby: `{ "games": [...], "next": … }` rather than a bare array.
///
/// An object because a list is a place fields get added to — and `next` is the
/// field it was: a client reading a bare array would have had to change shape to
/// learn there was a second page.
#[derive(Debug, Clone, Serialize)]
pub struct GamesList {
    pub games: Vec<crate::table::TableView>,
    /// The cursor for the NEXT page, or `null` when this is the last one.
    /// `<created_at>,<id>` — the row this page ended at, in the listing's own
    /// order; a client sends it straight back as `before`.
    pub next: Option<String>,
}

pub fn list_games(games: &Games, user: &User, q: &ListQuery) -> Result<GamesList, ApiError> {
    use crate::tables::{Scope, LIST_LIMIT_DEFAULT, LIST_LIMIT_MAX};
    let scope = match q.scope.as_deref() {
        None => Scope::Mine,
        Some(s) => s.parse::<Scope>().map_err(ApiError::bad_request)?,
    };
    let limit = q.limit.unwrap_or(LIST_LIMIT_DEFAULT);
    // Both ends refused rather than clamped: a caller that asked for 0 or for
    // 500 believed something about this API, and answering it with 1 or with
    // 100 leaves it believing that. `Games::list` guards `0` again for the same
    // reason it is refused here — see its doc.
    if limit == 0 || limit > LIST_LIMIT_MAX {
        return Err(ApiError::bad_request(format!("limit {limit} — 1 to {LIST_LIMIT_MAX}")));
    }
    let before = match q.before.as_deref() {
        None => None,
        Some(c) => {
            let (at, id) = c
                .split_once(',')
                .ok_or_else(|| ApiError::bad_request("before is <created_at>,<id>"))?;
            let at: u64 =
                at.parse().map_err(|_| ApiError::bad_request("before is <created_at>,<id>"))?;
            Some((at, id.to_string()))
        }
    };
    let (games, next) =
        games.list(user, scope, limit, before).map_err(ApiError::server_error)?;
    Ok(GamesList { games, next })
}

/// **201 with the TABLE, and no game yet.** Nothing is dealt into anyone's hand
/// by this call: the seats are bound and every human seat is owed a deal it
/// fetches for itself.
pub fn create_game(
    games: &Games,
    user: &User,
    req: &NewGame,
) -> Result<crate::table::TableView, ApiError> {
    let view = games.create(req, user)?;
    Ok(view)
}

/// The table as this caller may read it.
///
/// **The one route R-2.7 widens**: at `Open` any signed-in caller is answered,
/// because a join link is an address and the person following one has to see
/// what they are joining. At every other status this is `Games::table`'s fence
/// unchanged, and `my_seats` on the view may therefore be EMPTY.
pub fn get_table(
    games: &Games,
    user: &User,
    id: &str,
) -> Result<crate::table::TableView, ApiError> {
    let doc = games.table_for_anyone(id, user)?;
    Ok(doc.view(user, games.archive_on()))
}

/// The caller's own two cards, over the set table. A seated caller's, or a 403.
pub fn deal(games: &Games, user: &User, id: &str) -> Result<crate::table::DealView, ApiError> {
    Ok(games.deal(id, user)?)
}

/// Which of the two dealt cards a seat keeps.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keep {
    pub seat: u8,
    pub card: u8,
    /// What this seat is called on screen (M6.2, §3.5). Written onto the seat
    /// only when it has no name yet — see [`Games::keep`](crate::games::Games::keep).
    #[serde(default)]
    pub name: Option<String>,
}

/// Taking an open seat at an open table (M6.2, §3.2).
///
/// The password is the TABLE's and not the caller's: a table without a lock
/// ignores one, and a table with a lock refuses without it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Join {
    pub seat: u8,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

pub fn join(
    games: &Games,
    user: &User,
    id: &str,
    req: &Join,
) -> Result<crate::table::TableView, ApiError> {
    // The same fallible parse `keep` makes, and for the same reason: the number
    // came off the wire and `PlayerId::new` panics at four and above.
    let seat = yokohama_core::PlayerId::try_new(req.seat)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(games.join(id, user, seat, req.name.as_deref(), req.password.as_deref())?)
}

/// The table's password, presented once so this caller may WATCH (M6.3, §3.4).
///
/// **The table's and not the caller's** — the same sentence [`Join`] carries,
/// and for the same reason: it gates a shared game behind a door Cloudflare
/// Access has already opened, and nobody's identity rests on it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unlock {
    pub password: String,
}

/// `POST /api/games/{id}/unlock` — knock, and be remembered.
///
/// The whole of the ladder is [`Games::unlock`](crate::games::Games::unlock)'s, held under the
/// table's own lock; this handler carries no rule of its own. It answers the
/// `TableView`, whose `can_watch` now reads true, so a client that was
/// drawing the unlock face has the payload it needs to draw the game instead
/// without a second request.
pub fn unlock(
    games: &Games,
    user: &User,
    id: &str,
    req: &Unlock,
) -> Result<crate::table::TableView, ApiError> {
    Ok(games.unlock(id, user, &req.password)?)
}

/// A message to post (M6.4, spec §3.2). `name` is a WATCHER's (and anyone's
/// at the house); a seated caller sending one is a 400, because a seat's name
/// is the seat's (R-6.4.2).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatPost {
    pub text: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// The cursor: messages numbered ABOVE it (spec §3.1). Typed, so a negative
/// cursor is serde's 400.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatQuery {
    #[serde(default)]
    pub after: Option<u64>,
}

/// What the read answers: the retained messages above the cursor, and the
/// newest number ever assigned — the client's next cursor.
///
/// **`last` is the DOCUMENT's and not the page's**, which is what makes a
/// cursor survive a trim: a client that asks from a number the retention has
/// already dropped is told where the room has got to rather than left to infer
/// it from an empty page.
#[derive(Debug, Clone, Serialize)]
pub struct ChatPage {
    /// Built for the reader ([`chat::ChatDoc::page_for`]): `you` on their own
    /// messages, and no author's address (M6.5c2 cycle D, R-6.5c2.35).
    pub messages: Vec<chat::MessageView>,
    pub last: u64,
    /// The document's REVISION (M6.5c2 cycle D, R-D4), beside `last` for the
    /// same reason `last` is the document's: a reader holding `rev - last`
    /// from its previous page sees it move exactly when a message it may
    /// already hold was deleted. Every chat panel is a count host and compares
    /// that held difference with each signal frame's — `Events.chat` and
    /// `chat_rev` at a table, the house frame's `chat` and `rev` in the lobby —
    /// which are this document's `last` and `rev` on both hosts.
    pub rev: u64,
}

/// `GET /api/games/{id}/chat?after=N` — admitted exactly as a read is.
///
/// **The same [`admitted`] every projection takes, and deliberately not a rule
/// of its own** (spec §3.2): a seat holder at every status, a watcher once the
/// table admits them, and the lock's own sentence for the caller who has not
/// knocked.
///
/// **Which is NARROWER than seeing the table, and the gap is the point.**
/// `GET …/table` and `…/events` answer any signed-in caller at an OPEN table
/// (R-2.7, R-4.3) so that someone who followed a join link can read the seat
/// picker and watch for the seats to fill; the chat takes no such widening. A
/// stranger on that screen reads the table and the message COUNT off the
/// signal, and is 403 on the messages themselves — pinned by
/// `events_carries_the_chat_count_at_open_dealing_and_playing`, which asserts
/// both halves of that one caller in two adjacent lines. A room is readable by
/// the people `admitted` admits, not by everyone who can see the door.
pub fn chat_read(
    games: &Games,
    user: &User,
    id: &str,
    q: &ChatQuery,
) -> Result<ChatPage, ApiError> {
    // The admission's table is the page's too (M6.5c2 cycle D, row 130): the
    // chat is bounded by its seat count, and reading the document again for
    // that one number was a second read per page.
    let (table, _) = admitted(games, id, user)?;
    let (messages, counts) = games.chat_page(id, &table, user, q.after.unwrap_or(0))?;
    Ok(ChatPage { messages, last: counts.last, rev: counts.rev })
}

/// `POST /api/games/{id}/chat` — the one write a watcher may make (R-6.4.11).
///
/// The order is the spec's §3.2 and is pinned by adjacent-pair probes:
/// admission, the text, the seat/name rule, the watcher's name, the append.
/// Every step before the append is a refusal this caller can fix, and the text
/// is checked before the APPEND takes the table's lock, so a message that will
/// be refused never makes anyone wait behind another message. (Not "before any
/// lock": `admitted` runs first, and on a dealing table where every human has
/// kept it goes through `settle`, which takes that table's lock to finish the
/// deal.)
///
/// **The reply is the message as its POSTER reads it** (M6.5c2 cycle D,
/// R-6.5c2.35): `you: true`, and no address — the page's shape, so a client
/// reads one author type.
pub fn chat_post(
    games: &Games,
    user: &User,
    id: &str,
    req: &ChatPost,
) -> Result<chat::MessageView, ApiError> {
    let (table, viewer) = admitted(games, id, user)?;
    let text = chat::check_text(&req.text, &games.chat.limits).map_err(ApiError::bad_request)?;
    let author = author_of(&table.seats, viewer, user, req.name.as_deref())?;
    let (message, salt) = games.chat_append_salted(id, author, &text)?;
    Ok(message.view_for(user, salt.as_deref()))
}

/// Who a poster IS, from the viewer the admission produced (R-6.4.2, R-6.4.15).
///
/// **A seat's name is read off the TABLE and never off the body**, which is the
/// whole of the rule: a name in a message is a claim about who is speaking, and
/// at a seat the table has already settled that claim. A seated caller who
/// sends one is refused rather than quietly overridden, because a client that
/// believed it was setting a name and was answered 201 would go on sending it.
///
/// **It takes the SEATS, not the table** (M6.5c2 cycle D, row 130), so the
/// house — which has none — asks it too, as a [`client::Viewer::Spectator`]
/// over `&[]`: the watcher's rule (the name checked, the key the caller's) is
/// written once, here, and not restated in [`house_chat_post`].
fn author_of(
    seats: &[crate::table::SeatDoc],
    viewer: client::Viewer,
    user: &identity::User,
    name: Option<&str>,
) -> Result<chat::Author, ApiError> {
    match viewer {
        client::Viewer::Seat(seat) => {
            if name.is_some() {
                return Err(ApiError::bad_request("a seated caller is named by their seat"));
            }
            let stored = match seats.get(usize::from(seat.0)) {
                Some(crate::table::SeatDoc::Human { name, .. }) => name.clone(),
                _ => None,
            };
            Ok(chat::Author::Seat { seat: seat.0, user: user.as_str().to_string(), name: stored })
        }
        client::Viewer::Spectator => {
            let name = name.map(tables::check_name).transpose().map_err(ApiError::bad_request)?;
            Ok(chat::Author::Watcher { user: user.as_str().to_string(), name })
        }
    }
}

/// The house switch (R-6.4.6): a 404 with a sentence, before anything else.
///
/// A 404 rather than a 403, because with the lobby chat off the route is not
/// something this deployment refuses you — it is something this deployment does
/// not have. The client reads the same fact off `/api/health` and draws no
/// panel at all; the sentence is for whoever asks anyway.
pub fn lobby_chat_on(games: &Games) -> Result<(), ApiError> {
    if games.chat.lobby {
        Ok(())
    } else {
        Err(ApiError::not_found("the lobby chat is off"))
    }
}

/// `GET /api/chat?after=N` — the house. Any signed-in caller.
pub fn house_chat_read(games: &Games, user: &User, q: &ChatQuery) -> Result<ChatPage, ApiError> {
    lobby_chat_on(games)?;
    let (messages, counts) = games.house_page(user, q.after.unwrap_or(0))?;
    Ok(ChatPage { messages, last: counts.last, rev: counts.rev })
}

/// `POST /api/chat` — the house. Every author is a `Watcher`: the house has
/// no seats.
///
/// Which is not a simplification but the shape rule's own answer: the house
/// document is checked with `players: None`, and a `Seat` author there is
/// refused on write. There is nothing to be seated at.
///
/// The reply is the message as its poster reads it, as [`chat_post`]'s is.
pub fn house_chat_post(
    games: &Games,
    user: &User,
    req: &ChatPost,
) -> Result<chat::MessageView, ApiError> {
    lobby_chat_on(games)?;
    let text = chat::check_text(&req.text, &games.chat.limits).map_err(ApiError::bad_request)?;
    let author = author_of(&[], client::Viewer::Spectator, user, req.name.as_deref())?;
    let (message, salt) = games.house_append_salted(author, &text)?;
    Ok(message.view_for(user, salt.as_deref()))
}

/// `DELETE /api/games/{id}/chat/{seq}` (M6.5c2 cycle D, R-D4) — a message
/// taken down by its AUTHOR or by the table's CREATOR, and by nobody else.
///
/// **Admitted exactly as [`chat_post`] is**, by the same [`admitted`]: a seat
/// holder at every status, a watcher once the table admits them, the lock's
/// sentence to a caller who has not knocked — so the one write a spectator
/// may make and its undoing are behind the same door. Then the standing,
/// which is [`chat::ChatDoc::delete`]'s: `only the message's author or the
/// table's creator may delete it` (403) for anyone else, and `no message N in
/// this chat` (404) for a number the document does not hold.
///
/// **The answer is an empty 204**, and a second delete of the same message by
/// someone entitled is the same 204 — idempotent, writing and signalling
/// nothing. The message stays as a TOMBSTONE (`text: ""`, `deleted: true`), so
/// its number is never reused and every cursor still means what it meant; the
/// table's `Events.chat_rev` moves and `Events.chat` does not.
pub fn chat_delete(games: &Games, user: &User, id: &str, seq: u64) -> Result<(), ApiError> {
    admitted(games, id, user)?;
    Ok(games.delete_chat(id, seq, user)?)
}

/// `DELETE /api/chat/{seq}` (M6.5c2 cycle D, R-D4) — the house, where nobody
/// is a creator, so only the message's AUTHOR may take it down. The switch
/// first, as the read and the post have it; any signed-in caller is asked.
/// The house frame's `rev` moves and its `chat` does not.
pub fn house_chat_delete(games: &Games, user: &User, seq: u64) -> Result<(), ApiError> {
    lobby_chat_on(games)?;
    Ok(games.delete_house_chat(seq, user)?)
}

pub fn keep(
    games: &Games,
    user: &User,
    id: &str,
    req: &Keep,
) -> Result<crate::table::TableView, ApiError> {
    // **`PlayerId::new` PANICS at four and above**, and this number came off the
    // wire — so it is parsed through the fallible constructor and refused as a
    // 400. A panic here would poison nothing (the seat is read before any lock)
    // but would still drop the connection with no sentence in it.
    let seat = yokohama_core::PlayerId::try_new(req.seat)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(games.keep(id, user, seat, req.card, req.name.as_deref())?)
}

pub fn get_game(games: &Games, user: &User, id: &str) -> Result<ClientState, ApiError> {
    // Admitted first, so a caller with no business here is a 403 rather than a
    // frame; then the frame is built for that caller — the hand on it is their
    // own and every other seat's is a count (R-1.7), or, for a watcher, every
    // seat's is (M6.3, R-3.7).
    let (_table, viewer) = admitted(games, id, user)?;
    let handle = games.handle(id)?;
    let repl = games.lock_game(&handle, id)?;
    Ok(ClientState::of(id, &repl, viewer, Vec::new()))
}

pub fn delete_game(games: &Games, user: &User, id: &str) -> Result<(), ApiError> {
    games.delete(id, user)?;
    Ok(())
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

/// `input`'s messages when THIS step engaged a lockdown (finding D5): the
/// lines before the lockdown line — the mover's own deltas — and the line,
/// and nothing under it. `Repl::step` appends `lockdown_dump()` there — the
/// setup seed, the turn and the log tail — for a terminal's operator, who owns
/// the game; a seat at a shared table does not. A text with no lockdown line
/// is replaced by the reason alone.
pub(crate) fn lockdown_messages(lines: Vec<String>, reason: &str) -> Vec<String> {
    match lines.iter().position(|l| l.starts_with("LOCKDOWN — ")) {
        Some(i) => lines.into_iter().take(i + 1).collect(),
        None => vec![format!("LOCKDOWN — {reason}")],
    }
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
pub fn input(games: &Games, user: &User, id: &str, req: &Input) -> Result<ClientState, ApiError> {
    grammar::check_line(&req.line).map_err(ApiError::bad_request)?;
    if !grammar::is_gameplay_line(&req.line) {
        return Err(ApiError::bad_request(grammar::refusal(&req.line)));
    }
    let (table, viewer) = admitted(games, id, user)?;
    // **A watcher writes nothing, and hears it before the handle is touched**
    // (M6.3, R-3.6). Above the undo rule and the acting-seat fence because both
    // of those are about a SEAT, and this caller has none: R-P6 would read the
    // trailing event's mover against a seat that does not exist.
    spectators_write_nothing(viewer)?;
    let handle = games.handle(id)?;
    let mut repl = games.lock_game(&handle, id)?;
    // **The seat the rules below compare against is read AGAIN, under THIS
    // take** (M6.5c2 cycle D, row 129). `admitted` read one under its own take
    // and let that go, and a move landing in between — this person's other
    // tab, a double click — moves the turn from one of their seats to the
    // other: the fence then compared the NEW acting seat against the OLD one
    // and told the holder of both "it is seat 1's turn". It never admitted
    // anybody wrongly; it refused the one person it should have let through.
    // Pinned, with the gap held open, by
    // `a_hot_seat_creator_is_never_told_it_is_not_their_turn_between_two_lock_takes`.
    //
    // `None` is unreachable — a caller admitted as a seat holds one, and
    // `viewer_seat` answers for everyone who does — and is refused rather than
    // unwrapped, so nothing under the game's lock can panic here.
    let seat = client::viewer_seat(&table, user, repl.session.state()).ok_or_else(|| ApiError {
        status: StatusCode::FORBIDDEN,
        message: NOT_SEATED.to_string(),
        table_status: None,
        foreign_bot: None,
    })?;
    let viewer = client::Viewer::Seat(seat);

    // **An undo reaches back into your own turn and no further** (R-P6).
    //
    // The check is HERE rather than inside `step` because it needs the TABLE:
    // the rule turns on how many PEOPLE sit at this one, which is a fact of the
    // seat bindings and not of the record. Before `step`, because
    // `Session::undo` pops events and a refusal afterwards would be a refusal
    // of something that already happened.
    //
    // **And before the acting-seat fence, deliberately** (R-5.1). Both refuse
    // the same caller — a player whose turn has passed is neither the acting
    // seat nor entitled to undo — and both are a 403 that mutates nothing, so
    // the only thing the order decides is which true sentence they hear. "Undo
    // is not offered once your turn has passed" answers the button they
    // pressed; "it is seat 1's turn" answers a question they did not ask. The
    // fence is untouched and still runs, for this line and every other.
    if grammar::is_undo(&req.line) {
        undo_allowed(&mut repl, &table, seat).map_err(crate::games::GameError::from)?;
    }

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
    //
    // **The fence never drops. Only its WORDING changes** (R-4.3).
    //
    // A finished game has no turn and a locked one accepts no gameplay, so
    // naming a seat there is a false sentence — the false half of a pair, since
    // the holder of that same seat is told the truth by `step`. But the first
    // fix for that let a non-acting caller's POST through to `step` when the
    // game was over, and `undo` is an allowlisted gameplay line that
    // `Session::undo` applies with no game-over guard of its own. Measured on
    // that code: a seated WATCHER posted `undo` at a finished game, got 200,
    // and the record went from 357 events to 200 with `over` back to false —
    // any seated player could rewind a completed shared game, and this server
    // persisted it because `events.len()` had changed.
    //
    // So the comparison is unconditional and the SENTENCE is chosen: the
    // lockdown's own reason, the CLI's own `ApplyError::GameOver` line, or whose
    // turn it is. The acting seat's holder's path is untouched in all three.
    //
    // **The measured defect stays closed with R-P6's check now in front of
    // this one** (R-5.1): that seated watcher's `undo` is refused by
    // `undo_allowed` before it arrives here, because the watcher is not the
    // current player. Where `undo_allowed` waves a line through — a table with
    // ONE person at it — this comparison is what guards, exactly as it did
    // before. Neither check is the other's fallback; both run.
    let acting = repl.session.state().current_player;
    if acting != seat {
        let message = status_refusal(&repl).unwrap_or_else(|| {
            format!("it is seat {}'s turn ({})", acting.0, holder_of(&table, acting))
        });
        return Err(ApiError { status: StatusCode::FORBIDDEN, message, table_status: None, foreign_bot: None });
    }

    if let Some(expected) = &req.expect {
        let actual = crate::client::prompt_kind(&repl);
        if expected != actual {
            return Err(ApiError {
                status: StatusCode::CONFLICT,
                message: format!(
                    "this game is asking {actual:?}, not \"{expected}\" — the prompt moved on \
                     (a restart drops an open picker). Re-read the game and answer what it asks.",
                    expected = crate::echo::echo(expected),
                ),
                table_status: None, foreign_bot: None,
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
        if let Err(e) = games.persist(id, &repl) {
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
    let lines: Vec<String> =
        body.lines().map(str::trim_end).filter(|l| !l.is_empty()).map(str::to_string).collect();
    // **A lockdown's dump stays in the terminal** (finding D5): the reply
    // carries the lockdown line and nothing under it.
    let messages = match (repl.session.is_locked() && !locked_before, repl.session.record().lockdown.as_deref()) {
        (true, Some(reason)) => lockdown_messages(lines, reason),
        _ => lines,
    };
    // **The reply is the CALLER's view, not the new mover's.** A move that ends
    // a turn therefore answers its own mover with a `watching` prompt, which is
    // the true thing to say: the game has moved on and this person is not the
    // one it is waiting for.
    let mut client = ClientState::of(id, &repl, viewer, messages);
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
    Ok(client)
}

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
pub fn suggest(games: &Games, user: &User, id: &str) -> Result<explain::SuggestResponse, ApiError> {
    use explain::SuggestRefusal as R;
    let (table, viewer) = admitted(games, id, user)?;
    // **A watcher gets the not-seated sentence, not the advisor's panel**
    // (M6.3, R-3.6). The R-1.8 refusal below is for a non-acting HOLDER — "it
    // is not your turn" is a true thing to tell somebody who has a turn coming.
    spectators_write_nothing(viewer)?;
    let handle = games.handle(id)?;

    // **R-1.8: the acting seat's holder only, and this is a 403 rather than a
    // typed refusal.** The four refusals in the ladder are ANSWERS about the
    // game — it is over, it is locked, a bot is thinking — and a client renders
    // each as a sentence in the advisor panel. "This is not your seat" is not
    // that: it is the same access answer every other route on this table gives,
    // and serving it as a 200 would put the advisor's own panel in front of a
    // person who must not be ranking that seat's moves.
    //
    // **It sits AFTER the three refusals that are true of the GAME, and that
    // ordering was a review finding.** A finished game has no turn, so telling
    // a watcher "it is not your turn" there is false — and the holder of the
    // same seat was being told the truth on the same table. Whatever the game
    // itself has to say, every caller hears it; the seat answer is for the case
    // where the game is genuinely waiting for somebody.
    //
    // It still subsumes `SuggestRefusal::BotSeat` at a table where no human
    // holds the bot's seat, which is every table: a bot seat is nobody's, so
    // the caller is refused here first. That refusal stays declared because it
    // is the CLI's own and this crate carries `repl::suggest_cmd`'s ladder
    // verbatim rather than a subset of it.
    let read = {
        let repl = games.lock_game(&handle, id)?;
        let refuse = if repl.session.record().lockdown.is_some() {
            Some(R::Locked)
        } else if matches!(repl.session.mode, yokohama_cli::session::TrackingMode::Live) {
            Some(R::LiveMode)
        } else if repl.session.is_over() {
            Some(R::GameOver)
        } else {
            let acting = repl.session.state().current_player;
            if !table.seats_of(user).contains(&acting) {
                return Err(ApiError {
                    status: StatusCode::FORBIDDEN,
                    message: "it is not your turn".to_string(),
                    table_status: None, foreign_bot: None,
                });
            }
            if repl.bots().is_bot(acting) {
                Some(R::BotSeat)
            } else {
                None
            }
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
        Err(r) => return Ok(explain::SuggestResponse::refused(r.kind(), r.message())),
        Ok(parts) => parts,
    };

    Ok(match yokohama_cli::suggest::suggest(&game, games.rules(), seat, seed) {
        Ok(s) => explain::SuggestResponse::granted(explain::Suggest::of(&s, &game, games.rules())),
        // The agent's own errors are surfaced VERBATIM rather than swallowed —
        // `SuggestError::Inconsistent` describes itself as a bug worth
        // reporting, and a generic "no suggestion" would throw that away.
        Err(e) => explain::SuggestResponse::refused("search", e.to_string()),
    })
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
/// because a branch nobody has ever seen is where defects live. The route
/// calls its two halves, `why::locate` and `why::account_from`, and reads the
/// requester's visibility between them: a refusal the search finds is
/// reachable only at a public visibility, since the search runs only there
/// (security S2c).
///
/// Same lock discipline as `suggest`, for the same reason: the reconstruction
/// runs the same `decide` path the pump ran.
pub fn why(games: &Games, user: &User, id: &str, q: &WhyQuery) -> Result<explain::Why, ApiError> {
    // Seated, with the seats the caller holds in hand: `why` is served at the
    // REQUESTER's visibility (finding B1) — withheld while the mover's starting
    // order is hidden from them, which the engine's own view decides, save
    // that a seat holder reads everything once the game is over, as the record
    // route serves them the record (`explain::visibility_of`); what this fence
    // adds is the seat one — an account of a search at a table the caller has
    // no place at is not theirs to read.
    let (table, _viewer) = admitted(games, id, user)?;
    let held = table.seats_of(user);
    let handle = games.handle(id)?;
    let (mode, record, bots, now) = {
        let repl = games.lock_game(&handle, id)?;
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
            return Ok(explain::Why::locked(reason));
        }
        (
            repl.session.mode,
            repl.session.record().clone(),
            repl.bots().clone(),
            // History-free and cheap to clone (the engine's own rule); the
            // visibility is a fact about NOW, read off this copy outside the lock.
            repl.session.state().clone(),
        )
    };
    // **A move a foreign bot made is the layer's to word** (M6.5c2,
    // R-6.5c2.10): `account_parts` would call that seat a diagnostic that
    // "resumed as a HUMAN seat". Outside the lock, over the clone, as the
    // search below is.
    if let Some(refused) = explain::foreign_why(&record, games.rules(), q.event) {
        return Ok(refused);
    }
    // **The visibility is read BEFORE any search** (security S2c, the explain surface's first rule; D-6, D-7):
    // `locate` is pure replay — the index, the mover, its contestant and every pre-search refusal — and costs
    // nothing a hidden hand could vary; a withheld answer is built from it and never reaches `decide`.
    let located = match yokohama_cli::why::locate(mode, &record, games.rules(), q.event) {
        Ok(l) => l,
        Err(refusal) => return Ok(explain::Why::refused(&refusal)),
    };
    let visibility = explain::visibility_of(&now, located.seat, &held);
    if visibility == explain::Visibility::Withheld {
        return Ok(explain::Why::withheld_from(&located, games.rules()));
    }
    Ok(match yokohama_cli::why::account_from(located, &bots, games.rules()) {
        Ok(account) => explain::Why::account(&account, games.rules(), visibility),
        Err(refusal) => explain::Why::refused(&refusal),
    })
}

/// `GET /api/games/{id}/index/{kind}` — a card index, as a VIEW.
///
/// The four indexes are the CLI's canonical identification mechanism, and this
/// serves the same model the terminal draws: `screen_model::build_index` over
/// `Repl::index_placements_for`, which is why that function was widened rather
/// than copied — its doc defends a leak fence (no other seat's hand, because in
/// a live game an opponent's starting order is hidden and an index that dimmed
/// it with "in Seat 1's hand" would leak the one card the observation boundary
/// exists to protect).
///
/// **The `_for` is M6.1's, and it is the difference between that fence holding
/// and reading as though it did.** The terminal's fence is around the MOVER's
/// hand, because the terminal's one operator is the mover; this API's reader is
/// whoever asked. Asking the REPL for the viewer's placements keeps one fence in
/// one place rather than writing a second one here.
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
pub fn card_index(
    games: &Games,
    user: &User,
    id: &str, kind: &str,
    q: &IndexQuery,
) -> Result<yokohama_cli::screen_model::IndexModel, ApiError> {
    use yokohama_cli::screen_model::IndexKind;
    let kind = match kind {
        "orders" => IndexKind::Orders,
        "techs" => IndexKind::Techs,
        "goals" => IndexKind::Achievements,
        "sites" => IndexKind::Sites,
        other => {
            return Err(ApiError::bad_request(format!(
                "no card index named \"{other}\" — the four are orders, techs, goals and sites",
                other = crate::echo::echo(other)
            )))
        }
    };
    let filter = match q.filter.as_deref() {
        None => None,
        // **What `echo` would change is refused, not echoed**: the CLI's
        // `parse_filter` (a fenced crate) repeats the term raw in single
        // quotes, so a term `{:?}` would escape — a double quote, a backslash,
        // a control, a format character such as U+202E, a combining mark —
        // never reaches it. An apostrophe is plain text to `{:?}`, and passes.
        // (The length half is LOAD-BEARING, not subsumed: a term whose 65th
        // character is `…` echoes as itself — `x`×64+`…` — so without the
        // count a 129-character `\`×128+`…` would reach `parse_filter` raw.
        // The cell's terms carry one such term; mutant ML reads it.)
        Some(term) if term.chars().count() > crate::echo::ECHO_MAX || crate::echo::echo(term) != term => {
            return Err(ApiError::bad_request(format!(
                "a filter is at most {} characters of plain text — nothing a quoted string would escape: no quote, backslash, control, format or combining character",
                crate::echo::ECHO_MAX
            )))
        }
        Some(term) => yokohama_cli::card_prompt::parse_filter(term, kind)
            .map_err(ApiError::bad_request)?,
    };
    // **The index is scoped to the VIEWER, and that is not a formality.**
    //
    // `Repl::index_placements`' leak fence is real but is written from the
    // terminal's standpoint, where the one operator's seat IS the mover's: it
    // marks `CardLocation::OwnHand` for `current_player`. Served unscoped, a
    // seated opponent read the acting seat's whole hand from one GET — and the
    // public port takes then name that seat's hidden starting order. So the
    // REPL is asked for the placements of the seat this payload is FOR, which
    // is what keeps `OwnHand`'s own contract ("the index never says more than
    // the dashboard") true of a dashboard that is now the caller's.
    //
    // **And a SPECTATOR is asked for nobody's** (M6.3, R-3.8): `None` marks no
    // `OwnHand` row at all, which is the same sentence one step further —
    // there is no hand here that is this reader's.
    let (_table, viewer) = admitted(games, id, user)?;
    let handle = games.handle(id)?;
    let repl = games.lock_game(&handle, id)?;
    let me = match viewer {
        client::Viewer::Seat(s) => Some(s),
        client::Viewer::Spectator => None,
    };
    Ok(yokohama_cli::screen_model::build_index(
        kind,
        repl.session.rules(),
        &repl.index_placements_for(me),
        &yokohama_cli::screen_model::build_achievements(repl.session.state()).cards,
        None,
        filter,
    ))
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
pub fn result(games: &Games, user: &User, id: &str) -> Result<outcome::Outcome, ApiError> {
    let _admitted = admitted(games, id, user)?;
    let handle = games.handle(id)?;
    let repl = games.lock_game(&handle, id)?;
    if !repl.session.is_over() {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            message: "the game is not over — there is no final score yet".to_string(),
            table_status: None, foreign_bot: None,
        });
    }
    Ok(outcome::Outcome::of(&repl.session.final_result()))
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
pub fn log(
    games: &Games,
    user: &User,
    id: &str,
    q: &LogQuery,
) -> Result<yokohama_cli::screen_model::LogPanel, ApiError> {
    let _admitted = admitted(games, id, user)?;
    let handle = games.handle(id)?;
    let repl = games.lock_game(&handle, id)?;
    let total = repl.session.record().events.len();
    let first = q.tail.map_or(0, |n| total.saturating_sub(n));
    let described: Vec<(usize, String)> =
        (first..total).map(|i| (i, repl.session.event_description(i).to_string())).collect();
    Ok(yokohama_cli::screen_model::build_log(total, described))
}

/// How much of the game a poll gets: the SIGNAL, in six fields (room plan
/// R-P9, spec §6; the fifth is M6.4's and the sixth M6.5c2 cycle D's).
///
/// **A few bytes, and no frame is built.** This is the endpoint the client asks
/// every three seconds while it is somebody else's turn, and the whole point of
/// R-P9 is that the signal is separate from the fetch: the client learns that
/// this game now has N events and seat S to move, and asks for frames only when
/// that moved. Serving a `ClientState` here instead would make the poll cost a
/// whole board, a dashboard and a decision list, most of them identical to the
/// one already on screen.
///
/// **`status` is the field that lets ONE poll serve two screens.** A dealing
/// table answers `dealing` with `events` 0 and `to_move` null, so the setup
/// screen watches for its own start on the same endpoint the game watches for
/// the opponent's move, and the client has one timer rather than two.
///
/// **`your_turn` is per-caller, and it is why this route resolves a seat.**
/// `to_move` alone would make every client compute it from a seat number it
/// would have to be holding correctly; answering it here means the poll's
/// consumer branches on one boolean. It is false once the game is over, so a
/// finished game never reads as waiting for the reader.
#[derive(Debug, Clone, Serialize)]
pub struct Events {
    pub events: usize,
    pub to_move: Option<yokohama_core::PlayerId>,
    pub status: &'static str,
    pub your_turn: bool,
    /// The table's newest message number, 0 with none (M6.4, R-6.4.7) — the
    /// fifth field, and the one that makes a message a count on the signal and
    /// a fetch for the text.
    ///
    /// It is the DOCUMENT's `last` and never the number retained, so a trim
    /// does not walk it backwards and a client comparing it against its own
    /// cursor cannot be told to fetch a room that has not spoken.
    pub chat: u64,
    /// The table's chat REVISION (M6.5c2 cycle D, R-D4): `chat` is a count for
    /// the text, this a revision for the shape.
    pub chat_rev: u64,
}

/// `…/events`'s query, PARSED AND IGNORED since M6.5c2 cycle D (R-D5).
///
/// **Both keys stay so an older release keeps working.** M6.3 carried a resume
/// cursor (`after`) and a hold (`wait`) for the held-request transport; the
/// held arm is retired on every host, and the route answers at once whatever
/// either key says. An older precached release's close probe still sends
/// `wait=25`, and with the key gone `deny_unknown_fields` would answer it a
/// 400.
///
/// Typed `usize` and `u64`, so a negative or non-numeric value is still a 400
/// from serde; `deny_unknown_fields` through the layer's `parse_query` still
/// makes a MISSPELLED key a 400 rather than a silent default; and a `wait`
/// above [`MAX_WAIT_SECONDS`] is still a 400 ([`check_wait`]).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventsQuery {
    /// A resume cursor: parsed and ignored (R-D5). No host ever acted on it.
    #[serde(default)]
    pub after: Option<usize>,
    /// A hold of up to N seconds: parsed, bounded by [`check_wait`], and
    /// ignored (R-D5). The route answers at once.
    #[serde(default)]
    pub wait: Option<u64>,
}

/// The longest a `?wait` may name (spec §3.3) — kept, with its refusal, since
/// the hold was retired (R-D5): the value is still bounded, and then ignored.
///
/// Above it is a **400 and not a clamp**: a caller that asked for 120 and was
/// silently given 25 would read the early answer as "nothing changed for two
/// minutes", which is the same failure `deny_unknown_fields` exists to prevent
/// one field over. `pub` since M6.5c2 (spec §2.6's restatements): the Worker
/// module's `events` route is its own and refuses with the layer's cap and
/// sentence through [`check_wait`] rather than a copy.
pub const MAX_WAIT_SECONDS: u64 = 25;

/// `?wait` above [`MAX_WAIT_SECONDS`] refused in the handler's own words; no
/// `wait`, or one at or below the cap, answers `Ok`.
pub fn check_wait(wait: Option<u64>) -> Result<(), ApiError> {
    match wait {
        Some(w) if w > MAX_WAIT_SECONDS => Err(ApiError::bad_request(format!(
            "a request can be held for at most {MAX_WAIT_SECONDS} seconds, not {w}"
        ))),
        _ => Ok(()),
    }
}

/// The `Events` object for a live session, from the caller's seat.
///
/// **One function, two transports.** `events` and `…/stream` must answer the
/// same object or the client's compare is against two different things; the
/// stream's own cell asserts the first frame equals an `events` read BY VALUE,
/// which is the property this function makes true by construction rather than
/// by agreement between two bodies.
pub fn events_of(repl: &yokohama_cli::repl::Repl, viewer: client::Viewer, chat: chat::ChatCounts) -> Events {
    let s = repl.session.state();
    let over = repl.session.is_over();
    Events {
        events: repl.session.record().events.len(),
        to_move: Some(s.current_player),
        status: if over { "over" } else { "playing" },
        // **False for a spectator by construction** (M6.3): a watcher is never
        // the one the game is waiting for, and a poll consumer branching on
        // this boolean must never draw them a decision zone.
        your_turn: !over && matches!(viewer, client::Viewer::Seat(x) if s.current_player == x),
        // A PARAMETER rather than a read, because this function holds a `Repl`
        // and the count lives nowhere near one: a chat message is not an event
        // and must never move the number a client re-fetches a board on.
        chat: chat.last,
        chat_rev: chat.rev,
    }
}

/// The reading `events` answers with, at every table status.
///
/// **The admission is this function's, and it is what `…/stream` shares.** The
/// stream is "the events route, pushed" (spec §3.1), so anything the events
/// route would refuse the stream refuses identically and at the same point —
/// including before it subscribes to anything.
///
/// The OPEN and DEALING arms answer without a session and must: neither table
/// has a record, so a handler that reached `Games::handle` first would hand the
/// open-table screen and the setup screen a 409 instead of the reading each is
/// watching for.
///
/// **At OPEN the admission is the TABLE's own and not [`admitted`]'s** (spec
/// §3.5 row 2, controller ruling R-4.3): any signed-in caller is answered,
/// which is the same widening `GET …/{id}/table` has had since R-2.7 and is
/// there for the same person. Someone who followed a join link is sitting on
/// the open-table screen waiting for the seats to fill; a 403 would end their
/// stream and bounce them to the lobby, which is the one thing the screen must
/// not do. It is the ONLY row where the two part company — every other read at
/// an open table is still `admitted`'s 403 — and it stops at `open`: `dealing`
/// and `playing` fall straight through, so a stranger at a deal is refused and
/// a locked game still asks for its password.
pub fn events_at(games: &Games, id: &str, user: &identity::User) -> Result<Events, ApiError> {
    // An unsafe `{id}` is the layer's 404 here too: `app()` serves the two transport routes outside `dispatch` (S1c, Task 2's C6).
    if !crate::store::is_safe_id(id) {
        return Err(ApiError::not_found(format!("no game named \"{id}\"", id = crate::echo::echo(id))));
    }
    // The table is read FIRST, exactly as `admitted` reads it, so the open arm
    // and the fall-through see one document and one `settle`.
    let table = games.table_for_anyone(id, user)?;
    // **Read ONCE, above all three arms**, because every one of them answers
    // it: an open table where the seats are still filling is exactly where the
    // people at it are talking. It sits AFTER the table read, which is as far
    // as the guard goes and no further: an id that is not there, or a stranger
    // at a DEAL, is refused by `table_for_anyone` above and never reaches the
    // chat document. A stranger at a locked PLAYING table passes that read and
    // does reach it — one read and, on a miss, a cache seed — and is then
    // refused by `admitted` below. Nothing of it crosses the wire: the 403
    // carries no count.
    let chat = games.chat_counts(id);
    // The two arms with no session answer the same two numbers `events_of`
    // does for a live one.
    let unplayed = |status| Events {
        events: 0,
        to_move: None,
        status,
        your_turn: false,
        chat: chat.last,
        chat_rev: chat.rev,
    };
    if table.status == crate::table::TableStatus::Open {
        return Ok(unplayed("open"));
    }
    // **The table read above IS the admission's** (M6.5c2 cycle D, row 129):
    // `admitted` would read and settle it a second time, once per poll, per
    // NAS stream frame per connection, and per Worker reading, where a read is
    // a row. Pinned by `events_at_reads_the_table_once`.
    let (table, viewer) = admitted_with(games, &table, id, user)?;
    match table.status {
        // Unreachable behind the return above, and written rather than
        // collapsed into the fall-through: a status this function answers by
        // name is a status a reader can find.
        crate::table::TableStatus::Open => Ok(unplayed("open")),
        crate::table::TableStatus::Dealing => Ok(unplayed("dealing")),
        crate::table::TableStatus::Playing => {
            // `live_handle`, not `handle`: the table is in hand, and a cold
            // handle's rebuild through `handle` would settle it once more.
            let handle = games.live_handle(id, &table)?;
            let repl = games.lock_game(&handle, id)?;
            Ok(events_of(&repl, viewer, chat))
        }
    }
}

/// The batch of positions `[from, to)` (M6.5b, R-6.5b.10). Both required — a
/// run has two ends — and `deny_unknown_fields` through `parse_query`, so a
/// misspelled bound is a 400 in axum's words rather than a run from zero.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunQuery {
    pub from: usize,
    pub to: usize,
}

/// The most positions one run call answers. Above it the client pages; a clamp
/// would read as "the run ended" (the M6.3 `wait` rule: never a silent clamp).
pub const RUN_CAP: usize = 64;

/// The frames of `[from, to)`, in order.
#[derive(Debug, Clone, Serialize)]
pub struct RunFrames {
    pub frames: Vec<client::HistoricalFrame>,
}

/// `GET /api/games/{id}/at?from=&to=` — the frames of `[from, to)` in one reply,
/// computed by ONE forward replay (`HistoricalFrame::run`), each frame
/// byte-equal to `/at/{k}`'s.
///
/// **The boundary in numbers** (spec §2.4): a game with `total` events has
/// `total + 1` positions, `0..=total`, so the largest admitted `to` is
/// `total + 1` — the client's every call sends exactly that, so the run ends on
/// the live frame — and the 404 begins at `to = total + 2`, with `at/{k}`'s own
/// sentence extended to name the range, so the two routes refuse one condition
/// one way. `from > to` is DELIBERATELY the same 404 rather than a third answer.
/// The 404 begins at `from = total + 1` as at `to = total + 2`, and an empty run
/// is answered without a replay (security S1b, finding D27).
/// **The cap is checked FIRST**: a run longer than the cap is a malformed ask
/// whatever `total` is, so a fresh game asked for `to=65` reads the cap's
/// sentence and not the range's.
///
/// Admitted as `at_k` is, serves a locked session as `at_k` does, and holds the
/// lock across the body for the same reason `at_k` holds it.
pub fn run(games: &Games, user: &User, id: &str, q: &RunQuery) -> Result<RunFrames, ApiError> {
    let (_table, viewer) = admitted(games, id, user)?;
    let handle = games.handle(id)?;
    let mut repl = games.lock_game(&handle, id)?;
    let total = repl.session.record().events.len();
    if q.to > q.from && q.to - q.from > RUN_CAP {
        return Err(ApiError::bad_request(format!(
            "a run is at most {RUN_CAP} positions — ask for from={}&to={} and page",
            q.from,
            q.from + RUN_CAP
        )));
    }
    // `from` past the last position is the same 404 as `to` past it (finding
    // D27): the range test let `from = to = total + 1` through to a replay
    // that refused it as a 500.
    if q.from > q.to || q.from > total || q.to > total + 1 {
        return Err(ApiError::not_found(format!(
            "this game has {total} events, so its last position is {total} — there is no run from {} to {}",
            q.from, q.to
        )));
    }
    // An empty run answers without a replay (finding D27).
    if q.from == q.to {
        return Ok(RunFrames { frames: Vec::new() });
    }
    let frames = client::HistoricalFrame::run(&mut repl, q.from, q.to, viewer)
        .map_err(|e| ApiError::server_error(format!("replaying this game failed {e}")))?;
    Ok(RunFrames { frames })
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
/// and every error from `HistoricalFrame::at` is then a 500** — since security
/// S1b, `Session::state_before`'s `String`, which past the end never reaches,
/// so what it carries is a genuine fold failure. The range was first checked
/// here because `state_at`'s error could not tell past-the-end from a failing
/// event: `ReplayError` is a struct, not an enum, and its own doc says the
/// past-the-end convention is shared with `replay_strict`'s
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
pub fn at_k(
    games: &Games,
    user: &User,
    id: &str, k: usize,
) -> Result<client::HistoricalFrame, ApiError> {
    let (_table, viewer) = admitted(games, id, user)?;
    let handle = games.handle(id)?;
    let mut repl = games.lock_game(&handle, id)?;
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
    //
    // **And a SPECTATOR's sweep harvests nothing at all** (M6.3, R-3.7): every
    // hand is hidden at every `k` and `seat` is null.
    client::HistoricalFrame::at(&mut repl, k, viewer).map_err(|e| ApiError { status: StatusCode::INTERNAL_SERVER_ERROR, message: format!("replaying this game to {k} failed: {e}"), table_status: None, foreign_bot: None })
}

/// A route that reads one game and projects it — the three lock-and-project
/// routes (`ports`, `labs`, `areas`) differ only in which builder they call, so
/// the borrow, the not-found, the seat fence and the lock live once. A fourth
/// copy of this wiring is a fourth chance to forget one of them.
///
/// **Seated, and no viewer.** All three are the public table — the two offer
/// boards and the area reference — and none carries a hand or a seat's own
/// anything, so there is nothing here for a viewer to scope. The fence in front
/// of them is still the seat one: what the three would leak to a stranger is
/// the offer at a table they have no business at.
pub fn surface<T>(
    games: &Games,
    user: &User,
    id: &str,
    build: fn(&yokohama_core::GameState, &Rules) -> T,
) -> Result<T, ApiError> {
    let _admitted = admitted(games, id, user)?;
    let handle = games.handle(id)?;
    let repl = games.lock_game(&handle, id)?;
    Ok(build(repl.session.state(), repl.session.rules()))
}


/// The 409 a finished game whose STORED record is not the finished game
/// answers (finding D11) — shorter than the handle's, or without the finished
/// state's terminal digest: its last save failed, so the store holds the game
/// from before the end — a running game's record, seed and all.
pub(crate) const RECORD_UNSAVED: &str =
    "the record is available once the game is over and its last move is saved — that save failed";

/// `GET /api/games/{id}/record` (M6.5a, R-6.5a.12; umbrella R-6.5.10): a SEAT
/// HOLDER's finished game as the EXACT bytes the store holds for `Doc::Record`
/// — never a re-serialization, so the terminal digest the store stamped is the
/// digest the file carries. A spectator is refused 403 (the record is the
/// players'); a running game is a 409, because the record carries the setup
/// seed — hidden-set member #1 — until the last move. Lockdown does not fence
/// it: this is projection, not computation (the M3 lockdown rule).
///
/// **The last move is final only at a SHARED table** (the S1 stack review's
/// HI-F2). At a table of one person whose bot, if any, this build runs,
/// `undo_allowed` answers `Ok` before its finished-game arm (the refinish
/// rule, R-6.5c2.11), so that person may read this record, seed included, and
/// then undo the game back to running. That undo gains them nothing: a solo
/// creator's seed is theirs to choose (security S1a, finding B3), and no other
/// person holds a seat to read it against. `undo_allowed`'s FIRST arm refuses
/// an undo of a game played against a bot this build does not run, at any
/// table, so a lone person who played a foreign bot cannot undo at all (the
/// re-review's Finding 5 corrected "with or without a bot"). Where two people
/// sit, nobody rewinds a finished game (R-5.2), and the fence holds as
/// written.
///
/// Since security S1a the store is read UNDER the handle's lock and its record
/// held to the handle by its event count AND its terminal digest — a stored
/// record of the same length that is not the finished game (undone and
/// replayed to the same count, a failed final save behind it) carries no
/// digest, or another. A shorter one, or one without the finished state's
/// digest, is `RECORD_UNSAVED`. The digest is computed under the lock, so the
/// lock keeps the answer consistent — which true sentence the caller hears —
/// and not the seed: read after the guard dropped, a store that moved in
/// between would carry another digest and be refused, or still hold the
/// finished game.
pub fn record(games: &Games, user: &User, id: &str) -> Result<String, ApiError> {
    let (_table, viewer) = admitted(games, id, user)?;
    if viewer == client::Viewer::Spectator {
        return Err(ApiError {
            status: StatusCode::FORBIDDEN,
            message: "the record is the players' — a spectator downloads nothing".to_string(),
            table_status: None, foreign_bot: None,
        });
    }
    let handle = games.handle(id)?;
    // Under the lock, both facts: the handle's "over" and the stored bytes,
    // held to each other (finding D11) — a failed final persist leaves the
    // store behind a resident handle that says over: one move short, or, with
    // an undo and a replay before it, the same length on another line.
    let repl = games.lock_game(&handle, id)?;
    if !repl.session.is_over() {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            message: "the record is available once the game is over".to_string(),
            table_status: None, foreign_bot: None,
        });
    }
    let (json, _) = games
        .store()
        .read(crate::store::Doc::Record, id)
        .map_err(ApiError::server_error)?
        .ok_or_else(|| ApiError::not_found(format!("no game named \"{id}\"", id = crate::echo::echo(id))))?;
    #[derive(Deserialize)]
    struct StoredRecord {
        events: Vec<serde::de::IgnoredAny>,
        terminal_digest: Option<String>,
    }
    let stored: StoredRecord = serde_json::from_str(&json)
        .map_err(|e| ApiError::server_error(format!("the stored record does not parse: {e}")))?;
    // **The digest of the handle's STATE, never the handle record's own
    // `terminal_digest` field.** `Session::to_record_json` stamps the live
    // state's digest on a clone at every write, so a stored record names the
    // state it was written from; the in-memory field is what the session was
    // loaded with — `None` for a game dealt in this process, the loaded
    // record's for a rebuilt one — and no apply or undo refreshes it.
    let finished = yokohama_core::state_digest(repl.session.state());
    if stored.events.len() < repl.session.record().events.len()
        || stored.terminal_digest.as_deref() != Some(finished.as_str())
    {
        return Err(ApiError { status: StatusCode::CONFLICT, message: RECORD_UNSAVED.to_string(), table_status: None, foreign_bot: None });
    }
    Ok(json)
}

/// The largest body `POST /api/archive` reads (security S1b, finding B12;
/// R-SEC8(f)): 512 KiB, down from M6.5c2 cycle D's 1 MiB. Set from real
/// games: a finished game pretty-printed, which is what a browser sends, is
/// under 60 KB (the longest measured, the browser-played fixture, 54 KB), so
/// this is eight times that — and above 5,000 pretty-printed events, so the
/// event count ([`crate::backup::EVENT_CAP`]) speaks first on every real
/// record. Random play runs to 1.9 MB and the archive is not an import path
/// for it. Over the cap the route answers 413 BEFORE it parses anything
/// (`check_archive_size`), under both hosts' body limits — the NAS adapter's
/// 2 MiB and the Worker's `BODY_LIMIT`.
pub const ARCHIVE_BODY_LIMIT: usize = 512 << 10;

/// The 507 a browser's save meets once its caller's share of the archive is
/// full (security S1b, finding B12): the SERVER's state, not the record's —
/// the outbox keeps the row, shows this sentence and retries it on its next
/// drain, so an operator who makes room sees the game land.
pub fn archive_full_sentence(records: usize, bytes: u64) -> String {
    format!(
        "your archive here is full — {records} records or {} MiB; nothing was written, and the game is kept on this device",
        bytes >> 20
    )
}

/// The archive route's size check: over [`ARCHIVE_BODY_LIMIT`] is a 413 that
/// names the bound, answered after the caller and before the body is parsed —
/// pinned first in
/// `tests::the_archive_route_keeps_its_statuses_sentences_and_refusal_order`.
fn check_archive_size(body: &[u8]) -> Result<(), ApiError> {
    if body.len() > ARCHIVE_BODY_LIMIT {
        return Err(ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            message: "the record is larger than 512 KiB".to_string(),
            table_status: None,
            foreign_bot: None,
        });
    }
    Ok(())
}

/// What a browser sends to the archive (M6.5b, R-6.5b.6): the record and the
/// local table document as TWO STRINGS, parsed HERE and never by the client.
///
/// Strings rather than objects because the record's bytes are what its terminal
/// digest is over — a client that parsed and re-serialized it would hand this
/// route a record that no longer verifies — and because the client has no
/// business knowing either shape.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivePost {
    pub record: String,
    pub table: String,
}

/// Where a sent game landed.
#[derive(Debug, Clone, Serialize)]
pub struct ArchiveReply {
    /// `"over"` or `"abandoned"` — chosen from the REPLAYED state, never from
    /// what the body said.
    pub leaf: &'static str,
    pub path: String,
    /// The record at that path already held these very bytes (a retry after a
    /// lost reply): the reply is a 200 rather than a 201, and the client
    /// proceeds exactly as it would on a 201.
    pub identical: bool,
}

/// `POST /api/archive` (M6.5b, R-6.5b.6) — a game played in a browser, filed by
/// what a strict replay shows it to be.
///
/// **The TWELVE checks `backup::check_pair_filed` makes, in the order it makes
/// them** (the archive-off 404 is asked before any of them, and the rev check
/// speaks two sentences — one for a rev that disagrees, one for a rev that is
/// absent — so the route's refusals outnumber the checks; a restore meets the
/// same checks through `check_pair_with`, the wrapper): the record parsing —
/// through a mirror that refuses a key the engine's record does not have, then
/// as the engine's own type — its event count within
/// [`crate::backup::EVENT_CAP`], a `semantics_rev` present and equal to this
/// build's, a terminal digest present, no lockdown, the table document
/// parsing, passing [`tables::check_shape`] and AGREEING with the record, the
/// document's id being `is_safe_id`'s, the configuration one `validate_config`
/// accepts, the seats agreeing with the record's bots
/// ([`crate::bots::seat_disagreement`]), and LAST
/// `replay_strict_with(.., Predicate)` clean — behind a table that
/// agrees, so an untrusted record reaches setup only beside one (M6.5c2 cycle
/// D, straw poll row 81; this paragraph listed the replay before the table's
/// checks until security S1b, finding D29). **The count is stated here once**
/// — an earlier draft said four here, four again at the call below and a
/// third count in an earlier document (whole-branch review, false sentence
/// 4); the order is the one
/// `tests::the_archive_route_keeps_its_statuses_sentences_and_refusal_order`
/// probes pairwise. **Between the last cheap check and the replay, a retry of
/// a record already filed is answered** (security S1b, finding A2): the two
/// leaves are asked through [`Archive::holds`](crate::archive::Archive::holds)
/// whether they hold these very bytes under this caller — a `true` is the 200
/// `identical` the put's once-rule would give, with no replay paid — and a
/// table that fails its checks reads its own refusal first, as on every host,
/// so the NAS, whose archive reads its files, and the Worker, whose cannot,
/// answer every row alike (the corpus's `archive_400_table` among them). After
/// the validator, the route refuses a record naming a bot this server does not
/// run (`bots::foreign`, finding D25 — the game routes' rule, asked once more
/// here), a held one included, so a retry reads `identical` only past it. Then **the
/// leaf IS the `Why`** — `Over` iff the replayed state is over, else
/// `Abandoned` — and one put under
/// `browser/<`[`caller_segment`](crate::archive::caller_segment)`>/` with the
/// table RE-STRIPPED server-side from the parsed document, as `delete` does, so
/// a crafted body carrying a lock or a watcher roster never lands verbatim. The
/// record string is put exactly as sent, because its bytes are the digest's.
/// Where the caller's share under `browser/<caller>/` is full (security S1b,
/// finding B12), the put refuses and the route answers 507 in
/// [`archive_full_sentence`]'s words, nothing written, and the browser's outbox
/// keeps the row and retries it.
///
/// 404 where this host keeps no archive — which under `NoArchive`, the
/// browser's own host, is the answer by construction.
pub fn archive_post(games: &Games, user: &User, req: &ArchivePost) -> Result<ArchiveReply, ApiError> {
    use crate::archive::{caller_segment, is_on, paths, strip_for_archive, PutError, PutOutcome, Why};
    if !is_on(&games.archive) {
        return Err(ApiError::not_found("this server keeps no archive"));
    }
    // **The checks above are `backup::check_pair_filed`'s** — `check_pair`'s
    // since M6.5b2, the body's name since security S1b
    // (R-6.5b2.10, spec §2.5): one function, two callers, so a backup and an
    // upload cannot disagree about what a VALID GAME is. Whether this server
    // RUNS the game's bot is this route's own question, asked after the
    // validator (`bots::foreign`, finding D25, below) and not by a restore —
    // a restored game is the device's own, and the game routes refuse to play
    // on one whose bot this build does not run. The order and every sentence are
    // this route's own and did not move — pinned through `dispatch` by
    // `tests::the_archive_route_keeps_its_statuses_sentences_and_refusal_order`,
    // which was written and green BEFORE the extraction.
    // **A retry of a filed record is answered before any replay** (security S1b,
    // finding A2; the doc above): once every cheap check has passed, the archive
    // is asked whether either leaf already holds these very bytes under this
    // caller — through the validator's `filed` hook, so the question sits
    // exactly before the replay and after nothing else.
    let prefix = format!("browser/{}/", caller_segment(user));
    let filed = |doc: &crate::table::TableDoc, _rec: &yokohama_core::GameRecord| {
        [Why::Over, Why::Abandoned]
            .into_iter()
            .find(|why| games.archive.holds(&prefix, *why, &doc.id, &req.record))
            .map(|why| why == Why::Over)
    };
    let facts = crate::backup::check_pair_filed(&req.record, &req.table, games.rules(), &filed)
        .map_err(ApiError::bad_request)?;
    // **A bot this server does not run is refused** (finding D25): the record
    // is the module's own production bot's game or it is not a browser's game;
    // `bots::foreign` is the game routes' rule, asked once more here. The
    // record parsed a line ago inside `check_pair_filed`; this second parse is
    // the price of keeping `PairFacts` a few scalars rather than carrying the
    // parsed record.
    let rec: yokohama_core::GameRecord = serde_json::from_str(&req.record)
        .map_err(|e| ApiError::bad_request(format!("the record does not parse: {e}")))?;
    if let Some((seat, played, why)) = crate::bots::foreign(&rec).into_iter().next() {
        let (played, _) = crate::bots::distinguishing_labels(&played, &crate::bots::production());
        return Err(ApiError::bad_request(format!(
            "seat {seat} was played by {played}, which this server does not run: {why}"
        )));
    }
    // The document again, for the strip and the path. **Unreachable as a
    // refusal** — `check_pair_filed` parsed this very string a line ago — and written
    // as one anyway rather than as an `expect`, because a panic inside a route
    // takes the process out and this costs a parse of 500 bytes.
    let doc: crate::table::TableDoc = serde_json::from_str(&req.table)
        .map_err(|e| ApiError::bad_request(format!("the table document does not parse: {e}")))?;
    let why = if facts.over { Why::Over } else { Why::Abandoned };
    // Unreachable as a refusal — the key grammar and `is_safe_id` admit no `.`
    // or `..` — and answered in the put's own failure words if it ever is.
    let (path, _) = paths(yokohama_core::SEMANTICS_REV, &prefix, why, &doc.id)
        .map_err(|e| ApiError::server_error(format!("the archive could not be written: {e}")))?;
    if facts.held {
        return Ok(ArchiveReply { leaf: why.dir(), path, identical: true });
    }
    match games.archive.put(&prefix, why, &doc.id, &req.record, &strip_for_archive(&doc)) {
        Ok(outcome) => Ok(ArchiveReply {
            leaf: why.dir(),
            path,
            identical: outcome == PutOutcome::Identical,
        }),
        // The path is in the archive's own error for the operator's log; the
        // client is told what it can do about it and not where the file is.
        Err(PutError::Conflict { .. }) => Err(ApiError {
            status: StatusCode::CONFLICT,
            message: "a different record is archived under this id".to_string(),
            table_status: None, foreign_bot: None,
        }),
        Err(PutError::Quota { records, bytes }) => Err(ApiError {
            status: StatusCode::INSUFFICIENT_STORAGE,
            message: archive_full_sentence(records, bytes),
            table_status: None,
            foreign_bot: None,
        }),
        Err(PutError::Failed(e)) => {
            Err(ApiError::server_error(format!("the archive could not be written: {e}")))
        }
    }
}

/// The VALIDATOR's verdict on this pair — whether the checks [`archive_post`]
/// makes through `backup::check_pair_filed` accept it — with no archive, no
/// caller and no put, for the cell that compares the two callers of
/// [`backup::check_pair`](crate::backup::check_pair).
///
/// **It is not the route's whole answer**: the route's foreign-bot refusal
/// (`bots::foreign`, finding D25), asked after the validator, is not asked
/// here — so a pair naming a bot this server does not run reads `true` here
/// and 400 at the route.
///
/// **Not evidence that the route is unchanged**: it is a SECOND ENTRY POINT
/// into the same function, so it can only ever agree with itself. What the
/// route does is pinned by
/// [`tests::the_archive_route_keeps_its_statuses_sentences_and_refusal_order`],
/// which drives [`dispatch`].
///
/// It goes through the PUBLIC [`check_pair`](crate::backup::check_pair), which
/// builds a `Rules::standard()`, while the route passes `games.rules()` — the
/// same registries either way, since `Rules::standard()` is a pure function of
/// the generated data.
///
/// **Not `#[cfg(test)]`**, although its only callers are cells: the layer's
/// `platform_calls_stay_behind_the_server_feature` scan cuts each file at its
/// FIRST column-0 `#[cfg(test)]`, so one here would end the cut early and put
/// this file's own test module — which reads the filesystem and the
/// environment, as a test may — back inside the scan.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn archive_would_accept(record: &str, table: &str) -> bool {
    crate::backup::check_pair(record, table).is_ok()
}

/// The layer's front door: match, answer, and shape the response. A known path
/// under another method is 405 with an empty body (axum's own shape); a path
/// that matches nothing is the JSON 404 the server's `fallback` writes; and an
/// `{id}` the store would refuse is the JSON 404 `no game named "<echo>"`
/// (security S1c, finding B8). All three come before `answer`, so before its
/// 401 and its 400. Under HEAD both 404s leave with no body.
pub fn dispatch(games: &Games, host: &Host, identity: Identity<'_>, req: &ApiRequest<'_>) -> ApiResponse {
    let out = match find(req.method, req.path, host.serves_games) {
        Matched::WrongMethod => return ApiResponse { status: 405, body: String::new() },
        // An arm of `out` and not an early return (security S1c's final wave),
        // so the HEAD below leaves this refusal bodiless too.
        Matched::NoRoute => {
            error_response(ApiError::not_found(format!("no such endpoint: {}", crate::echo::echo(req.path))))
        }
        // **An `{id}` the store would refuse is a 404 here, before any handler**
        // (security S1c, finding B8): the id is the raw segment, and a 16 KB one
        // reached `lock_for` as a key. `is_safe_id` is the store's own rule, so
        // nothing this refuses could have been a table. An arm of `out` and not
        // an early return, so the HEAD below leaves this refusal bodiless too.
        Matched::Route(pattern, bound) if pattern.contains("{id}") && !crate::store::is_safe_id(bound[0]) => {
            error_response(ApiError::not_found(format!("no game named \"{id}\"", id = crate::echo::echo(bound[0]))))
        }
        Matched::Route(pattern, bound) => match answer(games, host, identity, req, pattern, &bound) {
            Ok(r) => r,
            Err(e) => error_response(e),
        },
    };
    // HEAD leaves with an empty body on every arm of `out` — a refused HEAD too,
    // an unknown path's included — so a non-HTTP host never emits a body a
    // HEAD did not ask for.
    if req.method == "HEAD" {
        ApiResponse { status: out.status, body: String::new() }
    } else {
        out
    }
}

/// One arm per route; `?` inside an arm is the refusal order axum had before
/// M6.5a — identity before body or query, because the `Identity` extractor
/// preceded `JsonBody`/`QueryArgs` in every handler's argument list then (the
/// mid-point review checked all twenty).
fn answer(
    games: &Games,
    host: &Host,
    identity: Identity<'_>,
    req: &ApiRequest<'_>,
    pattern: &'static str,
    b: &[&str],
) -> Result<ApiResponse, ApiError> {
    // A HEAD once took a write arm because the guards read the literal method (S1c Task 3's finding); every arm guard below reads `method`, never `req.method`.
    let method = if req.method == "HEAD" { "GET" } else { req.method };
    match pattern {
        "/api/health" => Ok(ok_json(200, &health(games, host))),
        "/api/me" => Ok(ok_json(200, &me(host, identity))),
        "/api/labels" => Ok(ok_json(200, &labels(games))),
        "/api/art" => Err(ApiError::not_found("art is served by the host".to_string())),
        _ => {
            // Identity FIRST, then the body or query, then the route: the order
            // axum ran the extractors in, so every refusal keeps its precedence.
            let u = identity.map_err(unauthorized)?;
            match pattern {
                "/api/games" if method == "GET" => {
                    list_games(games, u, &parse_query(req.query)?).map(|v| ok_json(200, &v))
                }
                "/api/games" => create_game(games, u, &parse_json(req.content_type, req.body)?)
                    .map(|v| ok_json(201, &v)),
                "/api/games/{id}" if method == "GET" => {
                    get_game(games, u, b[0]).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}" => {
                    delete_game(games, u, b[0]).map(|()| ApiResponse { status: 204, body: String::new() })
                }
                "/api/games/{id}/table" => get_table(games, u, b[0]).map(|v| ok_json(200, &v)),
                "/api/games/{id}/deal" => deal(games, u, b[0]).map(|v| ok_json(200, &v)),
                "/api/games/{id}/keep" => keep(games, u, b[0], &parse_json(req.content_type, req.body)?)
                    .map(|v| ok_json(200, &v)),
                "/api/games/{id}/join" => join(games, u, b[0], &parse_json(req.content_type, req.body)?)
                    .map(|v| ok_json(200, &v)),
                "/api/games/{id}/unlock" => {
                    unlock(games, u, b[0], &parse_json(req.content_type, req.body)?).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}/chat" if method == "GET" => {
                    chat_read(games, u, b[0], &parse_query(req.query)?).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}/chat" => {
                    chat_post(games, u, b[0], &parse_json(req.content_type, req.body)?).map(|v| ok_json(201, &v))
                }
                "/api/games/{id}/chat/{seq}" => {
                    let seq = parse_path_u64(1, b[1])?;
                    chat_delete(games, u, b[0], seq).map(|()| ApiResponse { status: 204, body: String::new() })
                }
                "/api/games/{id}/input" => {
                    input(games, u, b[0], &parse_json(req.content_type, req.body)?).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}/ports" => {
                    surface(games, u, b[0], yokohama_cli::screen_model::build_ports).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}/labs" => {
                    surface(games, u, b[0], yokohama_cli::screen_model::build_labs).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}/areas" => {
                    surface(games, u, b[0], yokohama_cli::screen_model::build_areas).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}/log" => {
                    log(games, u, b[0], &parse_query(req.query)?).map(|v| ok_json(200, &v))
                }
                "/api/chat" if method == "GET" => {
                    house_chat_read(games, u, &parse_query(req.query)?).map(|v| ok_json(200, &v))
                }
                "/api/chat" => house_chat_post(games, u, &parse_json(req.content_type, req.body)?)
                    .map(|v| ok_json(201, &v)),
                "/api/chat/{seq}" => {
                    let seq = parse_path_u64(0, b[0])?;
                    house_chat_delete(games, u, seq).map(|()| ApiResponse { status: 204, body: String::new() })
                }
                // 201 for a pair that landed, 200 for a retry that found its own
                // bytes already there — the client acts on either (spec §2.4).
                // The body's SIZE first, before the parse (straw poll row 81).
                "/api/archive" => {
                    check_archive_size(req.body)?;
                    archive_post(games, u, &parse_json(req.content_type, req.body)?)
                        .map(|v| ok_json(if v.identical { 200 } else { 201 }, &v))
                }
                "/api/games/{id}/at" => {
                    run(games, u, b[0], &parse_query(req.query)?).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}/at/{k}" => {
                    let k = parse_path_usize(1, b[1])?;
                    at_k(games, u, b[0], k).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}/suggest" => suggest(games, u, b[0]).map(|v| ok_json(200, &v)),
                "/api/games/{id}/why" => why(games, u, b[0], &parse_query(req.query)?).map(why_response),
                "/api/games/{id}/index/{kind}" => {
                    card_index(games, u, b[0], b[1], &parse_query(req.query)?).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}/result" => result(games, u, b[0]).map(|v| ok_json(200, &v)),
                // The stored bytes, not a re-serialization: the body is the string.
                "/api/games/{id}/record" => {
                    record(games, u, b[0]).map(|json| ApiResponse { status: 200, body: json })
                }
                _ => Err(ApiError::not_found(format!("no such endpoint: {}", crate::echo::echo(req.path)))),
            }
        }
    }
}

/// axum 0.8.9's `Path<(String, usize)>` rejection for a segment that is not a
/// number (`extract/path/mod.rs`, `ErrorKind::ParseErrorAtIndex` under the
/// `Invalid URL: ` prefix of `FailedToDeserializePathParams::body_text`), pinned
/// against axum's own text in lib.rs. `index` is the tuple position. **The type
/// is spelled `u64`, not `usize`**: axum's path deserializer visits a `usize`
/// as a `u64` and names that — the pin caught the plan's `usize` (R-plan-3).
pub(crate) fn parse_path_usize(index: usize, raw: &str) -> Result<usize, ApiError> {
    raw.parse::<usize>().map_err(|_| {
        ApiError::bad_request(format!(
            "Invalid URL: Cannot parse value at index {index} with value `{raw}` to a `u64`",
            raw = crate::echo::echo(raw)
        ))
    })
}

/// A chat message's number off the path (M6.5c2 cycle D, R-D4), refused in
/// [`parse_path_usize`]'s words — axum's own for a `Path<(String, u64)>`
/// segment that is not a number. A `u64` and not a `usize`, so a number is the
/// same number on a 32-bit host: the browser's module and the Worker's are
/// `wasm32`, where `usize` stops at four billion and a stored `seq` does not.
pub(crate) fn parse_path_u64(index: usize, raw: &str) -> Result<u64, ApiError> {
    raw.parse::<u64>().map_err(|_| {
        ApiError::bad_request(format!(
            "Invalid URL: Cannot parse value at index {index} with value `{raw}` to a `u64`",
            raw = crate::echo::echo(raw)
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A lockdown's dump never reaches the `input` reply** (finding D5):
    /// `Repl::step` appends `seed: <setup seed>`, the turn and the log tail
    /// under the `LOCKDOWN — …` line at every lockdown site, and `input`
    /// forwarded the whole text as `messages`. The filter keeps what came
    /// BEFORE the lockdown line — the mover's own deltas, public — and the
    /// line itself, and drops everything under it; a reason that carries
    /// digits is kept whole (Review Focus 7); a text with no lockdown line —
    /// a lockdown engaged somewhere the line was not printed — is replaced by
    /// the reason alone, never passed through.
    #[test]
    fn a_lockdowns_dump_is_cut_from_the_reply_and_the_reason_is_kept_whole() {
        let reason = "bot seat 1 could not decide honestly: the view was refused at turn 3 (code 4)";
        let lines: Vec<String> = [
            "Placed 1 assistant in Bank (A07)",
            &format!("LOCKDOWN — {reason}"),
            "seed: 20260831",
            "turn: 3",
            "  12  Blue: Move President to Bank (A07)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let kept = lockdown_messages(lines, reason);
        assert_eq!(kept, vec!["Placed 1 assistant in Bank (A07)".to_string(), format!("LOCKDOWN — {reason}")]);
        assert!(!kept.join("\n").contains("20260831"));
        // No lockdown line in the text: the reason alone.
        let kept = lockdown_messages(vec!["seed: 20260831".into(), "turn: 3".into()], reason);
        assert_eq!(kept, vec![format!("LOCKDOWN — {reason}")]);
    }

    #[test]
    fn a_missing_content_type_is_axums_sentence() {
        let e = parse_json::<crate::Input>(None, br#"{"line":"1"}"#).unwrap_err();
        assert_eq!(e.status, http::StatusCode::BAD_REQUEST);
        assert_eq!(e.message, "Expected request with `Content-Type: application/json`");
    }

    #[test]
    fn an_unknown_key_in_a_body_is_axums_sentence() {
        let e = parse_json::<crate::Input>(Some("application/json"), br#"{"line":"1","nope":1}"#)
            .unwrap_err();
        assert!(
            e.message.starts_with("Failed to deserialize the JSON body into the target type: "),
            "{}",
            e.message
        );
        assert!(e.message.contains("unknown field `nope`"), "{}", e.message);
    }

    #[test]
    fn a_bad_query_is_axums_sentence() {
        let e = parse_query::<crate::ListQuery>("limit=abc").unwrap_err();
        assert!(e.message.starts_with("Failed to deserialize query string: "), "{}", e.message);
    }

    /// **A body that is not a JSON object is refused in the layer's own words**
    /// (M6.5c2, spec §2.6) — and ONLY where serde would have read it.
    /// - **The refused.** Every body type the route table reads, each as a
    ///   positional array serde accepts today (asserted, so the refusal is
    ///   never read off a body serde would refuse anyway), answers
    ///   [`NOT_AN_OBJECT`].
    /// - **Axum's words, kept (R-6.5a.4).** A malformed array and a short one
    ///   keep axum's own sentences, because the object rule runs after serde
    ///   has answered.
    #[test]
    fn a_positional_array_body_is_refused_and_every_other_body_keeps_axums_sentence() {
        fn refused<T: DeserializeOwned>(body: &[u8]) -> ApiError {
            let text = String::from_utf8_lossy(body).to_string();
            assert!(serde_json::from_slice::<T>(body).is_ok(), "premise: serde reads {text} as the struct");
            parse_json::<T>(Some("application/json"), body).err().unwrap_or_else(|| panic!("{text} was read"))
        }
        for e in [
            refused::<crate::tables::NewGame>(br#"[2,["Red","Blue"],0,null,[{"user":"red@t.test"},{"open":true}]]"#),
            refused::<Input>(br#"["1"]"#),
            refused::<Join>(b"[1]"),
            refused::<Keep>(b"[0, 1]"),
            refused::<ChatPost>(br#"["hi"]"#),
            refused::<Unlock>(br#"["sesame"]"#),
            refused::<ArchivePost>(br#"["{}", "{}"]"#),
            refused::<Input>(b"\n  [\"1\"]\n"),
        ] {
            assert_eq!((e.status, e.message.as_str()), (StatusCode::BAD_REQUEST, NOT_AN_OBJECT));
        }
        let ct = Some("application/json");
        let e = parse_json::<Input>(ct, br#"["1""#).expect_err("a malformed array");
        assert!(e.message.starts_with("Failed to parse the request body as JSON: "), "{}", e.message);
        let e = parse_json::<Keep>(ct, b"[0]").expect_err("a short array");
        assert!(e.message.starts_with("Failed to deserialize the JSON body into the target type: "), "{}", e.message);
        let read = parse_json::<Input>(ct, b" {\"line\":\"1\"} \n").ok().map(|i| i.line);
        assert_eq!(read.as_deref(), Some("1"), "an object, JSON whitespace around it, reads");
    }

    /// Through the front door: a positional create opens no table.
    #[test]
    fn a_positional_create_opens_no_table() {
        let games = crate::games::test_support::games();
        let red = User::parse("red@t.test").expect("an address");
        let body = br#"[2,["Red","Blue"],0,null,[{"user":"red@t.test"},{"open":true}]]"#;
        let res = dispatch(
            &games,
            &LOCAL_HOST,
            Ok(&red),
            &ApiRequest { method: "POST", path: "/api/games", query: "", content_type: Some("application/json"), body },
        );
        assert_eq!((res.status, res.body), (400, serde_json::json!({ "error": NOT_AN_OBJECT }).to_string()));
        assert!(games.store().list(crate::store::Doc::Table).expect("a listing").is_empty(), "no table opened");
    }

    /// **An unsafe `{id}` is `dispatch`'s own 404, BEFORE any handler**
    /// (security S1c, finding B8). "Before any handler" is read off the
    /// identity: `answer` asks for it ahead of every game route, so a refused
    /// identity at a SAFE id is the 401 and at an UNSAFE one the 404. The
    /// boundary is `is_safe_id`'s — 64 safe bytes reach the handler and a 65th
    /// does not — and the check reads `bound[0]` because every pattern that
    /// carries `{id}` binds it FIRST, which the last loop pins. A HEAD of an
    /// unsafe id leaves with no body, as every refused HEAD does — an unknown
    /// path's 404 included.
    #[test]
    fn an_unsafe_id_is_dispatchs_own_404_before_any_handler() {
        fn ask(games: &Games, method: &str, path: &str, identity: Identity<'_>) -> ApiResponse {
            dispatch(games, &LOCAL_HOST, identity, &ApiRequest { method, path, query: "", content_type: None, body: b"" })
        }
        let games = crate::games::test_support::games();
        let red = User::parse("red@t.test").expect("an address");
        let (safe, unsafe_) = ("a".repeat(64), "a".repeat(65));
        let sentence = |id: &str| error_body(&ApiError::not_found(format!("no game named \"{id}\"", id = crate::echo::echo(id))));

        let res = ask(&games, "GET", &format!("/api/games/{safe}/table"), Err("no identity"));
        assert_eq!(res.status, 401, "64 safe bytes reach `answer`, which asks the identity first: {}", res.body);
        let res = ask(&games, "GET", &format!("/api/games/{unsafe_}/table"), Err("no identity"));
        assert_eq!((res.status, res.body), (404, sentence(&unsafe_)), "a 65th byte is refused before the identity is asked");
        for id in ["..", ".hidden", "a b", "-x", "_house", "caf\u{e9}"] {
            let res = ask(&games, "GET", &format!("/api/games/{id}"), Err("no identity"));
            assert_eq!((res.status, res.body), (404, sentence(id)), "{id:?} is not a name the store keeps");
        }

        let res = ask(&games, "GET", &format!("/api/games/{safe}/table"), Ok(&red));
        assert_eq!((res.status, res.body), (404, sentence(&safe)), "the handler's own absence, the same sentence");
        let res = dispatch(
            &games,
            &LOCAL_HOST,
            Ok(&red),
            &ApiRequest { method: "POST", path: &format!("/api/games/{safe}/join"), query: "", content_type: Some("application/json"), body: br#"{"seat":1}"# },
        );
        assert_eq!((res.status, res.body), (404, sentence(&safe)), "a join at an absent safe id is the handler's 404");
        assert_eq!(games.locks.lock().expect("the map").len(), 0, "and it never touched the lock map");

        let res = ask(&games, "HEAD", &format!("/api/games/{unsafe_}"), Ok(&red));
        assert_eq!((res.status, res.body.as_str()), (404, ""), "a refused HEAD carries no body");
        // The unknown path's 404 too: an arm of `out`, so the HEAD strip reads
        // it (the GET beside it carries the sentence, so the strip is the
        // difference).
        let res = ask(&games, "GET", "/api/nope", Ok(&red));
        assert_eq!((res.status, res.body), (404, error_body(&ApiError::not_found("no such endpoint: /api/nope"))));
        let res = ask(&games, "HEAD", "/api/nope", Ok(&red));
        assert_eq!((res.status, res.body.as_str()), (404, ""), "an unknown path's HEAD carries no body");

        for r in routes(true).iter().filter(|r| r.pattern.contains("{id}")) {
            assert_eq!(r.pattern.find('{'), r.pattern.find("{id}"), "{} binds {{id}} first", r.pattern);
        }
    }

    /// **`error_body` is the body `dispatch` writes for a refusal** (M6.5c2,
    /// spec §2.6's restatements) — all THREE shapes: bare (an unknown
    /// endpoint's 404), with `"status"` (a dealing table's 409), and with
    /// `"foreign_bot"` (a game not over against a bot this build does not run).
    /// The Worker module restates it today; once it imports this, a drift is
    /// impossible — and the third shape is the one a restatement dropped when
    /// this export was first written.
    #[test]
    fn error_body_is_the_body_dispatch_writes() {
        use crate::games::test_support::{games, two_people, BOT_TO_MOVE_AT, CORPUS_SEED};
        let games = games();
        let red = User::parse("red@t.test").expect("an address");
        let get = |games: &Games, path: &str| {
            dispatch(games, &LOCAL_HOST, Ok(&red), &ApiRequest { method: "GET", path, query: "", content_type: None, body: b"" })
        };
        let res = get(&games, "/api/nope");
        assert_eq!((res.status, res.body), (404, error_body(&ApiError::not_found("no such endpoint: /api/nope"))));
        games.create(&two_people("dealing", CORPUS_SEED), &red).expect("a dealing table");
        let res = get(&games, "/api/games/dealing");
        assert_eq!((res.status, res.body.clone()), (409, error_body(&ApiError::dealing())));
        assert!(res.body.contains(r#""status":"dealing""#), "anti-vacuity: the second shape: {}", res.body);
        // The third shape, through the same door Task 4's cells use.
        let foreign = holding_foreign("lookahead-v1", Some(BOT_TO_MOVE_AT));
        let local = User::parse("local@browser").expect("an address");
        let res = dispatch(
            &foreign,
            &LOCAL_HOST,
            Ok(&local),
            &ApiRequest { method: "GET", path: &format!("/api/games/{PLAYED_ID}"), query: "", content_type: None, body: b"" },
        );
        let e: ApiError = crate::games::GameError::ForeignBot {
            played: "V2 lookahead".to_string(),
            runs: crate::bots::label(&crate::bots::production()),
            why: mongrel_drift(),
        }
        .into();
        assert_eq!((res.status, res.body.clone()), (409, error_body(&e)));
        assert!(res.body.contains(r#""foreign_bot":{"played":"V2 lookahead""#), "anti-vacuity: the third shape: {}", res.body);
    }

    /// **The wait cap is the layer's, exported** (M6.5c2): 25 is accepted, 26
    /// is a 400 in the handler's own words — a refusal, never a clamp — and no
    /// `wait` is accepted too. Since R-D5 an accepted `wait` is ignored. The
    /// axum handler calls this; `a_wait_is_accepted_and_answered_at_once`
    /// (`lib.rs`) pins it there end to end.
    #[test]
    fn a_wait_above_the_cap_is_refused_never_clamped() {
        assert_eq!(MAX_WAIT_SECONDS, 25);
        assert!(check_wait(None).is_ok() && check_wait(Some(0)).is_ok() && check_wait(Some(25)).is_ok());
        let e = check_wait(Some(26)).expect_err("26 is over the cap");
        assert_eq!(
            (e.status, e.message.as_str()),
            (StatusCode::BAD_REQUEST, "a request can be held for at most 25 seconds, not 26"),
        );
        assert!(check_wait(Some(u64::MAX)).is_err());
    }

    // -----------------------------------------------------------------------
    // The archive route's own behaviour, through `dispatch` (M6.5b2, Task 8)
    // -----------------------------------------------------------------------

    /// The finished pair a browser really played, committed beside the layer's
    /// own tests. Its record carries a seed, which is why it lives under
    /// `tests/fixtures/` and not in `web/src/wire/` — see
    /// `tests/browser_played_pin.rs`'s own note.
    const PLAYED_RECORD: &str = include_str!("../tests/fixtures/browser-played-2p.json");
    const PLAYED_TABLE: &str = include_str!("../tests/fixtures/browser-played-2p.table.json");
    const PLAYED_ID: &str = "local-242a5429453c593c";

    /// The cap-sized spread record `backup.rs`'s cells read (security S1b):
    /// 4,096 events, exactly [`crate::backup::EVENT_CAP`], so one event more is
    /// the order cell's over-cap record.
    const SPREAD_RECORD: &str = include_str!("../tests/fixtures/spread-4p-4096.json");

    // The two one-key JSON edits, ONE copy shared with `backup.rs`'s cells
    // (security S1b, finding D25), which edit the same fixtures the same way.
    use crate::backup::tests::{keyed, without};

    /// The first event replaced by a later one, which the position it now sits
    /// in cannot legally take — and if it could, the terminal digest no longer
    /// matches. Either way the replay refuses.
    fn replay_broken(record: &str) -> String {
        let mut v: serde_json::Value = serde_json::from_str(record).expect("a record");
        let events =
            v.get_mut("events").expect("events").as_array_mut().expect("a list");
        assert!(events.len() > 1, "anti-vacuity: a one-event record cannot be re-ordered");
        events[0] = events[1].clone();
        v.to_string()
    }

    /// A `Games` whose archive is a real directory, and one that keeps none.
    /// The directory is this run's own and is emptied first, so the once-rule
    /// does not fire before the cell's first assertion.
    fn games_with(archive: std::sync::Arc<dyn crate::archive::Archive>) -> Games {
        Games::with_parts(
            std::sync::Arc::new(yokohama_core::Rules::standard()),
            std::sync::Arc::new(crate::store::MemStore::default()),
            crate::chat::ChatConfig::default(),
            std::sync::Arc::new(crate::signal::NoSignal),
            std::sync::Arc::new(|| 1_789_000_000u64),
            archive,
            crate::games::RunHost::Server,
        )
        .with_seed_policy(crate::games::SeedPolicy::Diagnostic)
    }

    fn scratch(name: &str) -> std::path::PathBuf {
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

    /// The browser host's facts — the host that actually posts to this route.
    const LOCAL_HOST: Host = Host {
        identity_name: "local",
        signal_kind: "none",
        serves_games: true,
        multiplayer_offered: false,
        table_ttl_days: None,
        open_table_ttl_days: None,
    };

    /// Who [`post_archive`] sends as: a cell that reads a path the archive
    /// keeps builds its prefix from this, as the route does.
    const ARCHIVE_CALLER: &str = "local@browser";

    /// [`ARCHIVE_CALLER`]'s segment, as the route computes it — the readable
    /// half and a digest since security S1c (D26), which no cell spells.
    fn archive_segment() -> String {
        crate::archive::caller_segment(&User::parse(ARCHIVE_CALLER).unwrap())
    }

    fn post_archive(games: &Games, record: &str, table: &str) -> (u16, serde_json::Value) {
        let body = serde_json::json!({ "record": record, "table": table }).to_string();
        post_archive_bytes(games, body.as_bytes())
    }

    /// [`post_archive`] with the body as given, for the rows about the body
    /// itself rather than the pair inside it.
    fn post_archive_bytes(games: &Games, body: &[u8]) -> (u16, serde_json::Value) {
        let user = User::parse(ARCHIVE_CALLER).expect("an address");
        let res = dispatch(
            games,
            &LOCAL_HOST,
            Ok(&user),
            &ApiRequest {
                method: "POST",
                path: "/api/archive",
                query: "",
                content_type: Some("application/json"),
                body,
            },
        );
        let v = serde_json::from_str(&res.body).unwrap_or(serde_json::Value::Null);
        (res.status, v)
    }

    // -----------------------------------------------------------------------
    // M6.5c2: the foreign-bot rule, through `dispatch` (R-6.5c2.10)
    // -----------------------------------------------------------------------

    /// One call through `dispatch`, as `who`, on the local host's facts.
    fn ask(games: &Games, who: &str, method: &str, path: &str, query: &str, body: &str) -> (u16, serde_json::Value) {
        let user = User::parse(who).expect("an address");
        let res = dispatch(
            games,
            &LOCAL_HOST,
            Ok(&user),
            &ApiRequest {
                method,
                path,
                query,
                content_type: (!body.is_empty()).then_some("application/json"),
                body: body.as_bytes(),
            },
        );
        (res.status, serde_json::from_str(&res.body).unwrap_or(serde_json::Value::Null))
    }

    /// The CLI's reason for [`crate::games::test_support::foreign_pair`]'s
    /// MONGREL `lookahead-v1` — the name over V4's knobs — which the CLI's
    /// resume seats with a drift warning on `weights` (cycle F, spec §2.3).
    fn mongrel_drift() -> String {
        let v4 = crate::bots::production().params["weights"];
        let lookahead = yokohama_core::Contestant::LOOKAHEAD_V1_DEFAULT.descriptor().2["weights"];
        format!("records weights={v4}, but this build derives weights={lookahead}")
    }

    /// A `Games` holding the committed pair as a game against `controller`,
    /// cut at `cut` events when given.
    fn holding_foreign(controller: &str, cut: Option<usize>) -> Games {
        let games = games_with(std::sync::Arc::new(crate::archive::NoArchive));
        let (record, table) = crate::games::test_support::foreign_pair(controller, cut);
        games.load_document(crate::store::Doc::Record, PLAYED_ID, &record).unwrap();
        games.load_document(crate::store::Doc::Table, PLAYED_ID, &table).unwrap();
        games
    }

    /// **Every game route refuses a game not over against a foreign bot, in
    /// one typed 409** (M6.5c2, R-6.5c2.10; cycle F, R-F4) — the sentence
    /// naming the bot and the CLI's reason, and the pair beside it for the
    /// client to branch on, still exactly `{played, runs}`. The foreign bot is
    /// the MONGREL `lookahead-v1`, refused as drift on `weights`. The table
    /// still reads (the lobby row says `runs_here: false`), and Abandon, which
    /// never rebuilds, still deletes it.
    #[test]
    fn every_game_route_refuses_an_unfinished_foreign_game_in_one_typed_409() {
        let games = holding_foreign("lookahead-v1", Some(crate::games::test_support::BOT_TO_MOVE_AT));
        let runs = crate::bots::label(&crate::bots::production());
        let want = serde_json::json!({
            "error": format!("This game was played against V2 lookahead. This server can't continue it: {}", mongrel_drift()),
            "foreign_bot": { "played": "V2 lookahead", "runs": runs },
        });
        let base = format!("/api/games/{PLAYED_ID}");
        for (method, route, query, body) in [
            ("GET", "", "", ""),
            ("POST", "/input", "", r#"{"line":"1"}"#),
            ("GET", "/ports", "", ""),
            ("GET", "/labs", "", ""),
            ("GET", "/areas", "", ""),
            ("GET", "/log", "", ""),
            ("GET", "/at", "from=0&to=1", ""),
            ("GET", "/at/0", "", ""),
            ("GET", "/suggest", "", ""),
            ("GET", "/why", "", ""),
            ("GET", "/index/orders", "", ""),
            ("GET", "/result", "", ""),
            ("GET", "/record", "", ""),
            ("GET", "/chat", "", ""),
        ] {
            let (status, v) = ask(&games, "local@browser", method, &format!("{base}{route}"), query, body);
            assert_eq!((status, &v), (409, &want), "{method} {base}{route}");
        }
        let (status, v) = ask(&games, "local@browser", "GET", &format!("{base}/table"), "", "");
        assert_eq!(status, 200, "{v}");
        assert_eq!(v["bot"], serde_json::json!({ "label": "V2 lookahead", "runs_here": false }));
        let (status, v) = ask(&games, "local@browser", "DELETE", &base, "", "");
        assert_eq!(status, 204, "Abandon never rebuilds, so it is not refused: {v}");
    }

    /// **A stored record's bot controller is cut and escaped wherever the layer
    /// repeats it** (the S1 stack review's HI-F1; finding D12's echo rule). A
    /// controller of 20,000 characters holding a bidi override and control
    /// characters — a record the browser host's restore admits — is read
    /// back through every game route's 409 at the unfinished game (its
    /// sentence and the `foreign_bot` pair beside it, which the message cap
    /// never covered), the table's and the list's `bot.label`, and the
    /// finished game's `why`, a 200 no cap sees at all. The review read a
    /// 21 KB 409, a 20 KB label and a 60,392-character `why` sentence. Each
    /// body here stays under 2 KB, and no string in it carries a raw control
    /// character, the override or a 65-character run of the value.
    #[test]
    fn a_hostile_bot_controller_is_cut_and_escaped_wherever_the_layer_repeats_it() {
        use crate::games::test_support::BOT_TO_MOVE_AT;
        let hostile = format!("\u{202e}EVIL\n{}\u{7}\u{1b}[31m", "q".repeat(20_000));
        let call = |games: &Games, method: &str, path: &str, query: &str, body: &str| -> (u16, String, serde_json::Value) {
            let user = User::parse("local@browser").expect("an address");
            let res = dispatch(
                games,
                &LOCAL_HOST,
                Ok(&user),
                &ApiRequest {
                    method,
                    path,
                    query,
                    content_type: (!body.is_empty()).then_some("application/json"),
                    body: body.as_bytes(),
                },
            );
            let v = serde_json::from_str(&res.body).unwrap_or(serde_json::Value::Null);
            (res.status, res.body, v)
        };
        // A string the layer wrote: bounded, and the value cut and escaped.
        let clean = |what: &str, s: &str| {
            assert!(s.chars().count() < 2_048, "{what}: {} characters", s.chars().count());
            assert!(!s.chars().any(|c| c.is_control() || c == '\u{202e}'), "{what}: a raw control or override character: {:?}", s.chars().take(80).collect::<String>());
            assert!(!s.contains(&"q".repeat(65)), "{what}: the raw run came back");
        };
        let base = format!("/api/games/{PLAYED_ID}");
        let unfinished = holding_foreign(&hostile, Some(BOT_TO_MOVE_AT));
        for (method, route, query, body) in [
            ("GET", "", "", ""),
            ("POST", "/input", "", r#"{"line":"1"}"#),
            ("GET", "/ports", "", ""),
            ("GET", "/labs", "", ""),
            ("GET", "/areas", "", ""),
            ("GET", "/log", "", ""),
            ("GET", "/at", "from=0&to=1", ""),
            ("GET", "/at/0", "", ""),
            ("GET", "/suggest", "", ""),
            ("GET", "/why", "", ""),
            ("GET", "/index/orders", "", ""),
            ("GET", "/result", "", ""),
            ("GET", "/record", "", ""),
            ("GET", "/chat", "", ""),
        ] {
            let row = format!("{method} {base}{route}");
            let (status, text, v) = call(&unfinished, method, &format!("{base}{route}"), query, body);
            assert_eq!(status, 409, "{row}: {}", text.chars().take(200).collect::<String>());
            assert!(text.len() < 2_048, "{row}: a {} byte body", text.len());
            for (part, s) in [("error", &v["error"]), ("played", &v["foreign_bot"]["played"]), ("runs", &v["foreign_bot"]["runs"])] {
                clean(&format!("{row} {part}"), s.as_str().unwrap_or_else(|| panic!("{row}: no {part}")));
            }
        }
        let (status, text, v) = call(&unfinished, "GET", &format!("{base}/table"), "", "");
        assert_eq!(status, 200, "{}", text.chars().take(200).collect::<String>());
        clean("the table's bot.label", v["bot"]["label"].as_str().expect("a label"));
        let (status, text, v) = call(&unfinished, "GET", "/api/games", "", "");
        assert_eq!(status, 200, "{}", text.chars().take(200).collect::<String>());
        let row = v["games"].as_array().expect("a list").iter().find(|g| g["id"] == PLAYED_ID).expect("the table is listed");
        clean("the list's bot.label", row["bot"]["label"].as_str().expect("a label"));

        let finished = holding_foreign(&hostile, None);
        let (status, text, v) = call(&finished, "GET", &format!("{base}/why"), "", "");
        assert_eq!(status, 200, "{}", text.chars().take(200).collect::<String>());
        assert!(text.len() < 2_048, "the finished game's why: a {} byte body", text.len());
        assert_eq!(v["refusal"]["kind"], "foreign_bot", "{}", text.chars().take(200).collect::<String>());
        clean("the finished game's why", v["refusal"]["message"].as_str().expect("a message"));
    }

    /// A `Games` holding the committed pair as [`holding_foreign`] does, with
    /// seat 1's parameters — in the record's run provenance AND the table
    /// mirror's descriptor, so the two say one bot — replaced by `params`.
    fn holding_foreign_with_params(controller: &str, cut: Option<usize>, params: serde_json::Value) -> Games {
        let (record, table) = crate::games::test_support::foreign_pair(controller, cut);
        let mut record: serde_json::Value = serde_json::from_str(&record).expect("the pair's record");
        let mut table: serde_json::Value = serde_json::from_str(&table).expect("the pair's table");
        record["run"]["seats"][1]["params"] = params.clone();
        table["turn"]["bot"]["params"] = params;
        let games = games_with(std::sync::Arc::new(crate::archive::NoArchive));
        games.load_document(crate::store::Doc::Record, PLAYED_ID, &record.to_string()).unwrap();
        games.load_document(crate::store::Doc::Table, PLAYED_ID, &table.to_string()).unwrap();
        games
    }

    /// One call through `dispatch` as `local@browser`, answering the status,
    /// the body's bytes as written and the body parsed.
    fn dispatch_raw(games: &Games, method: &str, path: &str) -> (u16, String, serde_json::Value) {
        let user = User::parse("local@browser").expect("an address");
        let res = dispatch(
            games,
            &LOCAL_HOST,
            Ok(&user),
            &ApiRequest { method, path, query: "", content_type: None, body: b"" },
        );
        let v = serde_json::from_str(&res.body).unwrap_or(serde_json::Value::Null);
        (res.status, res.body, v)
    }

    /// **A hostile parameter KEY on a controller this build KNOWS is cut and
    /// escaped too** (the S1 stack review's re-review, Finding 1). The CLI
    /// parses `lookahead-v1`'s name, refuses the key in `from_parts`' own words
    /// — `unknown param {key:?} for lookahead-v1 (…)`, escaped by `{:?}` but
    /// whole — and the web repeats that clause in the unfinished game's 409
    /// and the finished game's `why` 200. `echo_record_values` echoes every key
    /// of the record's map; the mutant that echoes the controller alone left
    /// all 492 lib cells green, and this cell's key reached the `why` 200 whole
    /// (the re-reviewer read 20,208 characters against 266). The key is 20,000
    /// characters holding a bidi override and an escape; the controller and
    /// every other knob are the pair's own, so the clause is the key's alone.
    #[test]
    fn a_hostile_parameter_key_on_a_known_controller_is_cut_and_escaped() {
        use crate::games::test_support::BOT_TO_MOVE_AT;
        let key = format!("\u{202e}{}\u{1b}[31m", "q".repeat(20_000));
        let params = || {
            let mut map = serde_json::to_value(&crate::bots::production().params).expect("production's params");
            map[key.as_str()] = serde_json::json!(1);
            map
        };
        let clean = |what: &str, s: &str| {
            assert!(s.len() < 2_048, "{what}: {} bytes", s.len());
            assert!(!s.chars().any(|c| c.is_control() || c == '\u{202e}'), "{what}: a raw control or override character");
            assert!(!s.contains(&"q".repeat(65)), "{what}: the raw run came back ({} bytes)", s.len());
        };
        let base = format!("/api/games/{PLAYED_ID}");
        let unfinished = holding_foreign_with_params("lookahead-v1", Some(BOT_TO_MOVE_AT), params());
        let (status, text, v) = dispatch_raw(&unfinished, "GET", &base);
        assert_eq!(status, 409, "{}", text.chars().take(200).collect::<String>());
        assert!(text.len() < 2_048, "the 409: a {} byte body", text.len());
        let said = v["error"].as_str().expect("a sentence");
        // The probe's own reading: the clause is `from_parts`' refusal of the key.
        assert!(said.contains("unknown param \""), "the clause is the key's: {}", said.chars().take(300).collect::<String>());
        for (part, s) in [("error", &v["error"]), ("played", &v["foreign_bot"]["played"]), ("runs", &v["foreign_bot"]["runs"])] {
            clean(&format!("the 409's {part}"), s.as_str().unwrap_or_else(|| panic!("no {part}")));
        }

        let finished = holding_foreign_with_params("lookahead-v1", None, params());
        let (status, text, v) = dispatch_raw(&finished, "GET", &format!("{base}/why"));
        assert_eq!(status, 200, "{}", text.chars().take(200).collect::<String>());
        assert_eq!(v["refusal"]["kind"], "foreign_bot", "{}", text.chars().take(200).collect::<String>());
        clean("the finished game's why", v["refusal"]["message"].as_str().expect("a message"));
    }

    /// **The `why` 200's refusal message is capped at the boundary, as an
    /// error's is** (the S1 stack review's re-review, Finding 2). The echo
    /// bounds each value a label repeats, not how MANY: a record whose
    /// controller is spelled as production's label at production's version
    /// ties both labels, so the knobbed arm renders every key of the record's
    /// map — 30,000 here, `k00001…k30000`. The 409 was capped already
    /// (`api::error_value`); the finished game's `why` is a 200 no cap saw, and
    /// read 270,427 bytes with a 270,350-character message. Now the message is
    /// cut at the one site the answer is serialized, to `MESSAGE_MAX`
    /// characters and the ellipsis. The mutant that drops the cap fails the
    /// count.
    #[test]
    fn the_why_refusal_message_is_capped_at_the_boundary() {
        let keys: serde_json::Map<String, serde_json::Value> =
            (1..=30_000).map(|i| (format!("k{i:05}"), serde_json::json!(1))).collect();
        let finished = holding_foreign_with_params("V4", None, serde_json::Value::Object(keys));
        let (status, text, v) = dispatch_raw(&finished, "GET", &format!("/api/games/{PLAYED_ID}/why"));
        assert_eq!(status, 200, "{}", text.chars().take(200).collect::<String>());
        assert_eq!(v["refusal"]["kind"], "foreign_bot", "{}", text.chars().take(200).collect::<String>());
        let said = v["refusal"]["message"].as_str().expect("a message");
        assert!(said.starts_with("This move was made by V4 (V4 k00001=1,k00002=1,"), "the knobbed arm: {}", said.chars().take(120).collect::<String>());
        assert_eq!(said.chars().count(), crate::MESSAGE_MAX + 1, "the message is cut to MESSAGE_MAX and the ellipsis");
        assert!(said.ends_with('…'));
        assert!(text.len() < 4_096, "a {} byte body", text.len());
    }

    /// **`why` reconstructs a move by a runnable bot that is not the production
    /// one, by replay, and refuses a foreign bot's move with the CLI's reason**
    /// (cycle F, spec §2.3; the thirteenth refusal shape kept).
    ///
    /// A GENUINE `lookahead-v1` game cut where its bot is to move: the route's
    /// rebuild seats `lookahead-v1` and pumps its move, and `why` at that event
    /// — and bare `why`, its most recent move — reconstructs the decision, its
    /// provenance naming `lookahead-v1` at seat 1: no refusal. A finished game
    /// against `v2-padded`, which the CLI's `eligible` refuses: the authored
    /// refusal, bare and at a bot move, naming the bot and `eligible`'s
    /// sentence.
    #[test]
    fn why_reconstructs_a_runnable_non_production_bot_and_refuses_a_foreign_one_with_its_reason() {
        use crate::games::test_support::{genuine_pair, BOT_TO_MOVE_AT};
        use yokohama_core::Contestant;
        let games = games_with(std::sync::Arc::new(crate::archive::NoArchive));
        let (record, table) = genuine_pair(Contestant::LOOKAHEAD_V1_DEFAULT, Some(BOT_TO_MOVE_AT));
        games.load_document(crate::store::Doc::Record, PLAYED_ID, &record).unwrap();
        games.load_document(crate::store::Doc::Table, PLAYED_ID, &table).unwrap();
        let base = format!("/api/games/{PLAYED_ID}");
        let query = format!("event={BOT_TO_MOVE_AT}");
        let (status, v) = ask(&games, "local@browser", "GET", &format!("{base}/why"), &query, "");
        assert_eq!(status, 200, "{v}");
        assert_eq!(v["refusal"], serde_json::Value::Null, "no refusal: {v}");
        let provenance = &v["ok"]["provenance"];
        assert_eq!(provenance["contestant"], Contestant::LOOKAHEAD_V1_DEFAULT.echo(), "{v}");
        assert_eq!((provenance["index"].as_u64(), provenance["seat"].as_u64()), (Some(BOT_TO_MOVE_AT as u64), Some(1)));
        let (status, bare) = ask(&games, "local@browser", "GET", &format!("{base}/why"), "", "");
        assert_eq!(status, 200, "{bare}");
        assert_eq!(bare["refusal"], serde_json::Value::Null, "bare `why` reconstructs too: {bare}");
        assert_eq!(bare["ok"]["provenance"]["contestant"], Contestant::LOOKAHEAD_V1_DEFAULT.echo());

        let foreign = holding_foreign("v2-padded", None);
        let refused = serde_json::json!({
            "ok": null,
            "withheld": null,
            "refusal": {
                "kind": "foreign_bot",
                "message": "This move was made by V2, which this server doesn't run: v2-padded is a named research \
                            identity for arena weight comparisons, not available as a CLI bot",
            },
        });
        // Event 4 is a move seat 1 made (the same-name re-point cell in
        // `games.rs` reads it so over the same pair).
        for query in ["", "event=4"] {
            assert_eq!(ask(&foreign, "local@browser", "GET", &format!("{base}/why"), query, ""), (200, refused.clone()), "why?{query}");
        }
    }

    /// **A pair whose table and record disagree about which seats are bots is
    /// refused, in one 409 naming the seat** (M6.5c2 cycle D, the review of
    /// Task 4), at `Games::live_handle` — the one place every host rebuilds a
    /// stored game — and so on every route that reads one.
    ///
    /// Row 1 is the shape the review measured: a record naming seat 1
    /// `bga-human` beside the committed table, which seats a BOT there. Before
    /// the guard it opened at seat 1's turn and nothing ever moved it — the CLI
    /// resumes seat 1 as a human, so no bot runs, and the acting-seat fence
    /// refuses the only person at the table. Unfinished and finished alike, and
    /// Abandon, which never rebuilds, still deletes it. Row 2 is the other
    /// direction (a recorded bot at a table's human seat); row 3 a record with
    /// no run provenance beside a bot seat, which no build of this layer writes
    /// (`finalize` has stamped the run seats since the first table document).
    #[test]
    fn a_pair_whose_table_and_record_disagree_about_a_bot_is_refused_in_one_409() {
        use crate::games::test_support::{BOT_TO_MOVE_AT, PLAYED_RECORD, PLAYED_TABLE};
        let holding = |record: &str, table: &str| -> Games {
            let games = games_with(std::sync::Arc::new(crate::archive::NoArchive));
            games.load_document(crate::store::Doc::Record, PLAYED_ID, record).unwrap();
            games.load_document(crate::store::Doc::Table, PLAYED_ID, table).unwrap();
            games
        };
        let refusal = |why: &str| {
            serde_json::json!({ "error": format!("the table document does not agree with the record: {why}") })
        };
        let base = format!("/api/games/{PLAYED_ID}");

        // Row 1: `bga-human` in the record, a bot at the table.
        let row_1 = refusal("seat 1 is a person in the record (bga-human) but a bot at the table");
        let unfinished = holding_foreign(yokohama_cli::bga::CONTROLLER_BGA_HUMAN, Some(BOT_TO_MOVE_AT));
        for (method, route, query, body) in [
            ("GET", "", "", ""),
            ("POST", "/input", "", r#"{"line":"1"}"#),
            ("GET", "/why", "", ""),
            ("GET", "/suggest", "", ""),
        ] {
            let (status, v) = ask(&unfinished, "local@browser", method, &format!("{base}{route}"), query, body);
            assert_eq!((status, &v), (409, &row_1), "unfinished: {method} {base}{route}");
        }
        let finished = holding_foreign(yokohama_cli::bga::CONTROLLER_BGA_HUMAN, None);
        assert_eq!(ask(&finished, "local@browser", "GET", &base, "", ""), (409, row_1.clone()), "finished too");
        let (status, v) = ask(&unfinished, "local@browser", "DELETE", &base, "", "");
        assert_eq!(status, 204, "Abandon never rebuilds, so it is not refused: {v}");

        // Row 2: the record's bot at a table's human seat.
        let mut table: crate::table::TableDoc = serde_json::from_str(PLAYED_TABLE).unwrap();
        table.seats[1] = crate::table::SeatDoc::Human { user: "ada@t.test".to_string(), name: None, bound_at_create: false };
        table.keeps[1] = Some(1);
        let games = holding(PLAYED_RECORD, &serde_json::to_string(&table).unwrap());
        let runs = crate::bots::label(&crate::bots::production());
        assert_eq!(
            ask(&games, "local@browser", "GET", &base, "", ""),
            (409, refusal(&format!("seat 1 is a bot in the record ({runs}) but a person at the table"))),
        );

        // Row 3: no run provenance beside the table's bot seat.
        let mut record: yokohama_core::GameRecord = serde_json::from_str(PLAYED_RECORD).unwrap();
        record.run = None;
        let games = holding(&serde_json::to_string(&record).unwrap(), PLAYED_TABLE);
        assert_eq!(
            ask(&games, "local@browser", "GET", &base, "", ""),
            (409, refusal("seat 1 is a bot at the table but the record names no controller for it")),
        );
    }

    /// **A finished game against a foreign bot opens, scores and downloads —
    /// and the four places the CLI's seat conversion would show are masked**
    /// (M6.5c2, R-6.5c2.10; review R2-M1). `v2-padded` is a named identity
    /// this CLI refuses to run, so `Repl::new` turns seat 1 into a human one
    /// inside the resident `Repl`, exactly as a V4 game will read once V5 ships.
    #[test]
    fn a_finished_foreign_game_opens_scores_downloads_and_masks_the_conversion() {
        let games = holding_foreign("v2-padded", None);
        let base = format!("/api/games/{PLAYED_ID}");
        let (status, state) = ask(&games, "local@browser", "GET", &base, "", "");
        assert_eq!(status, 200, "{state}");
        assert_eq!(state["over"], true);
        // 1. the warning: the WHOLE array, so a reworded CLI line fails here.
        assert_eq!(state["warnings"], serde_json::json!([]), "no conversion line, and nothing else");
        // 4. the Players panel says bot where the record does, and agrees with `bots`.
        assert_eq!(state["bots"], serde_json::json!([{ "seat": 1, "label": "V2" }]));
        let players = state["frame"]["dashboard"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|p| p.get("Players"))
            .expect("a Players panel");
        let is_bot: Vec<bool> =
            players["players"].as_array().unwrap().iter().map(|p| p["is_bot"].as_bool().unwrap()).collect();
        assert_eq!(is_bot, [false, true], "the panel and `bots` agree");
        // It scores and downloads as today.
        let (status, _) = ask(&games, "local@browser", "GET", &format!("{base}/result"), "", "");
        assert_eq!(status, 200);
        let (status, _) = ask(&games, "local@browser", "GET", &format!("{base}/record"), "", "");
        assert_eq!(status, 200);
        // 2. `why`: bare, and on a move the bot made (event 4), names the bot —
        // never "a human's move"; a human's own move (event 0) is still the CLI's.
        let foreign = serde_json::json!({
            "ok": null,
            "withheld": null,
            "refusal": {
                "kind": "foreign_bot",
                "message": "This move was made by V2, which this server doesn't run: v2-padded is a named research \
                            identity for arena weight comparisons, not available as a CLI bot",
            },
        });
        for query in ["", "event=4"] {
            let (status, v) = ask(&games, "local@browser", "GET", &format!("{base}/why"), query, "");
            assert_eq!((status, &v), (200, &foreign), "why?{query}");
        }
        let (_, v) = ask(&games, "local@browser", "GET", &format!("{base}/why"), "event=0", "");
        assert_eq!(v["refusal"]["kind"], "human_seat", "a person's move is the CLI's to word: {v}");
        // 3. undo: refused at this one-human table, and the game stays finished.
        let (status, v) = ask(&games, "local@browser", "POST", &format!("{base}/input"), "", r#"{"line":"undo"}"#);
        assert_eq!(status, 403, "{v}");
        assert_eq!(
            v["error"],
            "undo is not offered on a game played against V2, which this server doesn't run: v2-padded is a named \
             research identity for arena weight comparisons, not available as a CLI bot"
        );
        let (_, after) = ask(&games, "local@browser", "GET", &base, "", "");
        assert_eq!((after["over"].clone(), after["events"].clone()), (serde_json::json!(true), state["events"].clone()));
    }

    /// **(Review Focus 3) A SPECTATOR — seated nowhere — opening a finished game
    /// played against a foreign bot.** The Players panel says bot, `why` names
    /// the bot's version rather than "a human's move", the watcher's undo is the
    /// not-seated refusal, and nothing a finished game shows a watcher today is
    /// refused.
    #[test]
    fn a_spectator_reads_a_finished_foreign_game_as_the_bots() {
        let games = holding_foreign("v2-padded", None);
        let base = format!("/api/games/{PLAYED_ID}");
        let (status, state) = ask(&games, "grey@t.test", "GET", &base, "", "");
        assert_eq!(status, 200, "{state}");
        assert_eq!(state["seat"], serde_json::Value::Null, "premise: a watcher");
        assert_eq!(state["bots"], serde_json::json!([{ "seat": 1, "label": "V2" }]));
        let players = state["frame"]["dashboard"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|p| p.get("Players"))
            .expect("a Players panel");
        assert_eq!(players["players"][1]["is_bot"], true, "the panel says the bot played seat 1");
        let (_, why) = ask(&games, "grey@t.test", "GET", &format!("{base}/why"), "", "");
        assert_eq!(why["refusal"]["kind"], "foreign_bot", "{why}");
        assert_eq!(
            why["refusal"]["message"],
            "This move was made by V2, which this server doesn't run: v2-padded is a named research identity for \
             arena weight comparisons, not available as a CLI bot"
        );
        let (status, v) = ask(&games, "grey@t.test", "POST", &format!("{base}/input"), "", r#"{"line":"undo"}"#);
        assert_eq!((status, v["error"].as_str()), (403, Some(NOT_SEATED)), "a watcher writes nothing: {v}");
        for route in ["/result", "/log", "/at/0", "/ports", "/labs", "/areas", "/index/orders"] {
            let (status, v) = ask(&games, "grey@t.test", "GET", &format!("{base}{route}"), "", "");
            assert_eq!(status, 200, "{route}: {v}");
        }
    }
    /// **The archive route, driven through `dispatch` itself** — written BEFORE
    /// `archive_post`'s checks moved into [`crate::backup::check_pair`] and
    /// re-run afterwards unedited, which is the whole of why it exists
    /// (M6.5b2 Task 8, spec §2.5: "validation lives in Rust and is shared with
    /// the archive route").
    ///
    /// `backup::check_pair` and [`archive_would_accept`] cannot prove the route
    /// did not move: they are the same function asked twice. This drives the
    /// front door and pins three things —
    ///
    /// 1. the STATUS of the accepted pair, of the retry, and of every refusal;
    /// 2. the WORDING of each refusal, verbatim;
    /// 3. the ORDER, by ADJACENT-PAIR probes — each probe carries BOTH of two
    ///    neighbouring defects and asserts the EARLIER sentence. A single-defect
    ///    probe pins no order at all, which is the lesson M6.2 relearned three
    ///    times.
    #[test]
    fn the_archive_route_keeps_its_statuses_sentences_and_refusal_order() {
        let dir = scratch("archive_order");
        let on = games_with(std::sync::Arc::new(crate::archive::DirArchive::new(&dir)));
        let off = games_with(std::sync::Arc::new(crate::archive::NoArchive));

        // The premise that makes the accepted arm a claim about the REPLAY: the
        // body's table says the game is over, but the leaf is not read off it.
        let (status, v) = post_archive(&on, PLAYED_RECORD, PLAYED_TABLE);
        assert_eq!(status, 201, "{v}");
        assert_eq!(v["leaf"], "over");
        assert_eq!(v["identical"], false);
        assert_eq!(v["path"], format!("browser/{}/rev7/over/{PLAYED_ID}.json", archive_segment()));

        // The same bytes again: 200 `identical`, never a 409.
        let (status, v) = post_archive(&on, PLAYED_RECORD, PLAYED_TABLE);
        assert_eq!(status, 200, "{v}");
        assert_eq!(v["identical"], true);

        // **The body's size, FIRST and before any parse** (M6.5c2 cycle D,
        // straw poll row 81) — the catcher for "the archive cap after the
        // parse". One byte over the cap, and not JSON either, to an archive
        // that is OFF: the 413 is answered before the body parse would say 400
        // and before the switch would say 404. At the cap exactly, the same
        // non-JSON body reaches the parse — so the bound is the cap and not one
        // below it.
        assert_eq!(ARCHIVE_BODY_LIMIT, 512 << 10, "the cap R-SEC8(f) set (security S1b)");
        let over = vec![b'x'; (512 << 10) + 1];
        let (status, v) = post_archive_bytes(&off, &over);
        assert_eq!((status, v["error"].as_str()), (413, Some("the record is larger than 512 KiB")), "{v}");
        let (status, v) = post_archive_bytes(&on, &over[..512 << 10]);
        assert_eq!(status, 400, "at the cap the body is read, and refused for what it is: {v}");

        // The checks, in order, each named by the sentence it answers.
        let rev_wrong = keyed(PLAYED_RECORD, "semantics_rev", 6.into());
        let no_replay = replay_broken(PLAYED_RECORD);
        let bad_players = keyed(PLAYED_TABLE, "players", 3.into());
        let bad_seat = keyed(PLAYED_TABLE, "start_seat", 1.into());
        let unsafe_id = keyed(PLAYED_TABLE, "id", "not/a/safe/id".into());
        let over_cap = {
            let mut v: serde_json::Value = serde_json::from_str(SPREAD_RECORD).expect("a record");
            let events = v["events"].as_array_mut().expect("events");
            events.push(serde_json::json!({ "Move": { "PlacePresident": { "destination": 255 } } }));
            v.to_string()
        };
        let locked = keyed(PLAYED_RECORD, "lockdown", "a fault".into());
        let impossible = {
            let mut v: serde_json::Value = serde_json::from_str(PLAYED_RECORD).expect("a record");
            v["config"]["has_port_b"] = true.into();
            v["config"]["has_lab_b"] = true.into();
            v.to_string()
        };

        // **The adjacent-pair probes.** Each row carries the defects of TWO
        // neighbouring checks and expects the earlier one's sentence.
        //
        // **The TABLE's four checks run BEFORE the replay since M6.5c2 cycle D**
        // (straw poll row 81): the replay rebuilds the game's setup from the
        // record's own configuration, and it is the table's checks that bound
        // `players`, `colors` and `start_seat` against it — so an untrusted
        // record now reaches setup only once a table agrees with it. The
        // catcher for "the table checks after the replay" is the row after the
        // lockdown row ("the table parses < the table's shape") and the id
        // row: each carries a record that would not replay, and expects a
        // TABLE sentence.
        let probes: [(&Games, String, String, &str); 12] = [
            // archive-off  <  the record parses
            (&off, "not json at all".to_string(), PLAYED_TABLE.to_string(), "this server keeps no archive"),
            // the record parses  <  the count is within the cap
            (&on, keyed(&over_cap, "seed", "not a number".into()), PLAYED_TABLE.to_string(), "the record does not parse"),
            // the count is within the cap  <  the rev agrees
            (&on, keyed(&over_cap, "semantics_rev", 6.into()), PLAYED_TABLE.to_string(), "the record carries 4097 events"),
            // the rev agrees  <  a terminal digest is present
            (&on, without(&rev_wrong, "terminal_digest"), PLAYED_TABLE.to_string(),
             "this record is at semantics rev 6; this server archives rev 7"),
            // a terminal digest is present  <  the record is not locked down
            (&on, without(&locked, "terminal_digest"), PLAYED_TABLE.to_string(), "the record carries no terminal digest"),
            // the record is not locked down  <  the table parses
            (&on, locked.clone(), "not json at all".to_string(), crate::backup::LOCKED_DOWN),
            // the table parses  <  the table's shape
            (
                &on,
                no_replay.clone(),
                keyed(&bad_players, "archived_at", "not a number".into()),
                "the table document does not parse",
            ),
            // the table's shape  <  the table agrees with the record
            (&on, no_replay.clone(), bad_players.clone(),
             "the table document is malformed"),
            // the table agrees  <  the table's id is a safe name
            (&on, no_replay.clone(), keyed(&bad_seat, "id", "not/a/safe/id".into()),
             "the table document does not agree with the record"),
            // the table's id is a safe name  <  the configuration is possible
            (&on, impossible.clone(), unsafe_id.clone(), "the table's id is not a safe name"),
            // the configuration is possible  <  the seats agree with the record's bots
            (&on, without(&impossible, "run"), PLAYED_TABLE.to_string(), "the record's configuration is impossible"),
            // the seats agree with the record's bots  <  the record replays
            (&on, replay_broken(&without(PLAYED_RECORD, "run")), PLAYED_TABLE.to_string(), "the table's seats disagree"),
        ];
        let mut sentences = std::collections::BTreeSet::new();
        for (games, record, table, expect) in probes {
            let (status, v) = post_archive(games, &record, &table);
            let want = if expect == "this server keeps no archive" { 404 } else { 400 };
            assert_eq!(status, want, "{expect}: {v}");
            let said = v["error"].as_str().expect("a sentence").to_string();
            assert!(said.starts_with(expect), "{said:?} does not start with {expect:?}");
            sentences.insert(said);
        }
        assert_eq!(sentences.len(), 12, "twelve checks, twelve different sentences");

        // The last, on its own — nothing follows it to probe against.
        let (status, v) = post_archive(&on, &no_replay, PLAYED_TABLE);
        assert_eq!(status, 400, "{v}");
        assert!(
            v["error"].as_str().expect("a sentence").starts_with("the record does not replay"),
            "{v}",
        );

        // A record with NO rev is a different sentence from one that disagrees:
        // the engine's own, carried verbatim.
        let (status, v) = post_archive(&on, &without(PLAYED_RECORD, "semantics_rev"), PLAYED_TABLE);
        assert_eq!(status, 400, "{v}");
        assert!(
            v["error"].as_str().expect("a sentence").starts_with("missing semantics provenance"),
            "{v}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// An archive that claims to hold EVERYTHING — the instrument that reads
    /// where the retry's answer sits: a record that could not replay is
    /// answered `identical` through it only if the question was asked before
    /// the replay.
    struct HoldsEverything;

    impl crate::archive::Archive for HoldsEverything {
        fn put(
            &self,
            _prefix: &str,
            _why: crate::archive::Why,
            _id: &str,
            _record_json: &str,
            _table_json: &str,
        ) -> Result<crate::archive::PutOutcome, crate::archive::PutError> {
            unreachable!("a held record is answered before the put")
        }
        fn kind(&self) -> &'static str {
            "dir"
        }
        fn holds(&self, _prefix: &str, _why: crate::archive::Why, _id: &str, _record_json: &str) -> bool {
            true
        }
    }

    /// **A retry of a filed record is answered before any replay** (finding
    /// A2): once every cheap check has passed, the archive is asked whether
    /// either leaf already holds these very bytes under this caller, and a
    /// `true` is the 200 `identical` the put's once-rule gives — with no replay
    /// paid. The proof: a record that does NOT replay reads 200 `identical`
    /// through an archive that claims it, where the same pair on a real, empty
    /// archive reads the replay's 400 — so the answer came before the replay.
    /// And the cheap checks still speak first: the claiming archive beside a
    /// table that disagrees reads the agreement's 400, which is what keeps the
    /// committed corpus's `archive_400_table` row a 400 on every host, the
    /// Worker's included.
    #[test]
    fn a_retry_of_a_filed_record_is_answered_identical_before_any_replay() {
        let claims = games_with(std::sync::Arc::new(HoldsEverything));
        let dir = scratch("archive_retry");
        let real = games_with(std::sync::Arc::new(crate::archive::DirArchive::new(&dir)));
        let broken = replay_broken(PLAYED_RECORD);
        let (status, v) = post_archive(&real, &broken, PLAYED_TABLE);
        assert_eq!(status, 400, "the control, on a real archive holding nothing: {v}");
        assert!(v["error"].as_str().unwrap_or("").starts_with("the record does not replay"), "{v}");
        let (status, v) = post_archive(&claims, &broken, PLAYED_TABLE);
        assert_eq!((status, v["identical"].as_bool()), (200, Some(true)), "held, so never replayed: {v}");
        assert_eq!(v["leaf"], "over");
        assert_eq!(v["path"], format!("browser/{}/rev7/over/{PLAYED_ID}.json", archive_segment()));
        let disagreeing = keyed(PLAYED_TABLE, "start_seat", 1.into());
        let (status, v) = post_archive(&claims, &broken, &disagreeing);
        assert_eq!(status, 400, "the table's checks come first: {v}");
        assert!(v["error"].as_str().unwrap_or("").starts_with("the table document does not agree"), "{v}");
        // A real archive's retry of landed bytes is the same 200, through
        // the same hook (`DirArchive::holds` reads the marker).
        let (status, _) = post_archive(&real, PLAYED_RECORD, PLAYED_TABLE);
        assert_eq!(status, 201);
        let (status, v) = post_archive(&real, PLAYED_RECORD, PLAYED_TABLE);
        assert_eq!((status, v["identical"].as_bool()), (200, Some(true)), "{v}");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// An archive that holds a record under the `Abandoned` leaf and nothing
    /// under `Over`.
    struct HoldsAbandoned;

    impl crate::archive::Archive for HoldsAbandoned {
        fn put(
            &self,
            _prefix: &str,
            _why: crate::archive::Why,
            _id: &str,
            _record_json: &str,
            _table_json: &str,
        ) -> Result<crate::archive::PutOutcome, crate::archive::PutError> {
            unreachable!("a held record is answered before the put")
        }
        fn kind(&self) -> &'static str {
            "dir"
        }
        fn holds(&self, _prefix: &str, why: crate::archive::Why, _id: &str, _record_json: &str) -> bool {
            why == crate::archive::Why::Abandoned
        }
    }

    /// **A record held under `abandoned/` is answered from that leaf**
    /// (security S1b, finding A2): 200 `identical`, leaf `"abandoned"`, the path
    /// under `abandoned/` — the leaf the archive holds, never the one the
    /// replay would choose, since no replay is paid. The catcher for the hook
    /// reading every held record as `over` (`.map(|_| true)` in
    /// `archive_post`'s `filed`), which the retry cell cannot see because both
    /// of its archives hold the record under `Over`, the leaf asked first.
    #[test]
    fn a_record_held_under_the_abandoned_leaf_is_answered_from_that_leaf() {
        let games = games_with(std::sync::Arc::new(HoldsAbandoned));
        let (status, v) = post_archive(&games, PLAYED_RECORD, PLAYED_TABLE);
        assert_eq!((status, v["identical"].as_bool()), (200, Some(true)), "held, so never replayed: {v}");
        assert_eq!(v["leaf"], "abandoned", "{v}");
        assert_eq!(v["path"], format!("browser/{}/rev7/abandoned/{PLAYED_ID}.json", archive_segment()));
    }

    /// **Every cheap check speaks before the archive is asked** (security S1b,
    /// finding A2): beside an archive that claims to hold everything, each row
    /// is a pair that could never be accepted, its FIRST defect one cheap
    /// check's, and reads that check's own 400 — so `filed` is asked after the
    /// LAST cheap check, the `is_safe_id` refusal included. The catcher for the
    /// hook asked above the `is_safe_id` refusal, which the retry cell cannot
    /// see because its one claiming row carries a table that DISAGREES, and the
    /// agreement check sits above `is_safe_id`. The rows of finding D25's
    /// three checks (security S1b, Task 2) — the lockdown, the configuration
    /// and the seats — hold the same for them, the last two the catcher for
    /// the hook asked above the two checks that follow `is_safe_id`.
    #[test]
    fn every_cheap_check_speaks_before_a_claiming_archive_is_asked() {
        let claims = games_with(std::sync::Arc::new(HoldsEverything));
        let broken = replay_broken(PLAYED_RECORD);
        let over_cap = {
            let mut v: serde_json::Value = serde_json::from_str(SPREAD_RECORD).expect("a record");
            let events = v["events"].as_array_mut().expect("events");
            events.push(serde_json::json!({ "Move": { "PlacePresident": { "destination": 255 } } }));
            v.to_string()
        };
        let impossible = {
            let mut v: serde_json::Value = serde_json::from_str(&broken).expect("a record");
            v["config"]["has_port_b"] = true.into();
            v["config"]["has_lab_b"] = true.into();
            v.to_string()
        };
        let rows = [
            ("not json at all".to_string(), PLAYED_TABLE.to_string(), "the record does not parse"),
            (over_cap, PLAYED_TABLE.to_string(), "the record carries 4097 events"),
            (keyed(&broken, "semantics_rev", 6.into()), PLAYED_TABLE.to_string(), "this record is at semantics rev 6"),
            (without(&broken, "semantics_rev"), PLAYED_TABLE.to_string(), "missing semantics provenance"),
            (without(&broken, "terminal_digest"), PLAYED_TABLE.to_string(), "the record carries no terminal digest"),
            (keyed(&broken, "lockdown", "a fault".into()), PLAYED_TABLE.to_string(), crate::backup::LOCKED_DOWN),
            (broken.clone(), "not json at all".to_string(), "the table document does not parse"),
            (broken.clone(), keyed(PLAYED_TABLE, "players", 3.into()), "the table document is malformed"),
            (broken.clone(), keyed(PLAYED_TABLE, "start_seat", 1.into()), "the table document does not agree"),
            (broken.clone(), keyed(PLAYED_TABLE, "id", "not/a/safe/id".into()), "the table's id is not a safe name"),
            (impossible, PLAYED_TABLE.to_string(), "the record's configuration is impossible"),
            (without(&broken, "run"), PLAYED_TABLE.to_string(), "the table's seats disagree"),
        ];
        for (record, table, expect) in rows {
            let (status, v) = post_archive(&claims, &record, &table);
            assert_eq!(status, 400, "{expect}: {v}");
            let said = v["error"].as_str().unwrap_or("");
            assert!(said.starts_with(expect), "{said:?} does not start with {expect:?}");
        }
    }

    /// **A bot this server does not run is refused at the archive** (finding
    /// D25): a browser's game is played by the module's production bot, so a
    /// record naming another controller is forged, or a game this build could
    /// not have played — `bots::foreign`'s own answer, the rule the game
    /// routes apply. The sentence names the bot as the game route's 409 does.
    /// **A held record is refused too** ("a held one included",
    /// `archive_post`'s doc): through [`HoldsEverything`] the same record reads
    /// the foreign 400, never the 200 `identical` — the catcher for the held
    /// early return moved above the foreign check, which answers 200 there.
    /// The foreign check moved above `check_pair_filed`'s call reads the same
    /// 400 here; its catcher is the two-defect probe below, which then reads
    /// the foreign sentence instead of the digest's.
    #[test]
    fn a_record_naming_a_bot_this_server_does_not_run_is_refused() {
        let dir = scratch("archive_foreign_bot");
        let on = games_with(std::sync::Arc::new(crate::archive::DirArchive::new(&dir)));
        let mut v: serde_json::Value = serde_json::from_str(PLAYED_RECORD).expect("a record");
        let seat = v.pointer_mut("/run/seats/1/controller").expect("the bot's controller");
        assert_eq!(seat.as_str(), Some("v4-step9-draft1"), "the premise");
        *seat = "mcts-v1".into();
        let (status, said) = post_archive(&on, &v.to_string(), PLAYED_TABLE);
        assert_eq!(status, 400, "{said}");
        assert!(said["error"].as_str().unwrap_or("").starts_with("seat 1 was played by "), "{said}");
        assert!(said["error"].as_str().unwrap_or("").contains(", which this server does not run: "), "{said}");
        // Held: an archive that claims these bytes still reads the foreign
        // 400 — the refusal sits between the validator and the held answer.
        let claims = games_with(std::sync::Arc::new(HoldsEverything));
        let (status, said) = post_archive(&claims, &v.to_string(), PLAYED_TABLE);
        assert_eq!(status, 400, "a held record is refused too: {said}");
        assert!(said["error"].as_str().unwrap_or("").starts_with("seat 1 was played by "), "{said}");
        // Two defects — the foreign bot and no terminal digest — read the
        // digest's sentence: the validator speaks before the foreign refusal.
        let (status, said) = post_archive(&on, &without(&v.to_string(), "terminal_digest"), PLAYED_TABLE);
        assert_eq!(status, 400, "{said}");
        assert!(said["error"].as_str().unwrap_or("").starts_with("the record carries no terminal digest"), "{said}");
        assert!(std::fs::read_dir(dir.join("browser")).is_err(), "nothing was written");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// **A full share is a 507 in the archive's words, and a retry of a filed
    /// record is still 200** (finding B12; Review Focus 2): the archive's
    /// `Quota` is the server's state and not the record's, so it is neither
    /// the 400 nor the 409 the outbox ENDS a row on — the outbox keeps the row,
    /// shows this sentence and retries on its next drain.
    #[test]
    fn a_full_share_is_a_507_and_the_retry_of_a_filed_record_is_still_200() {
        let dir = scratch("archive_quota");
        let on = games_with(std::sync::Arc::new(
            crate::archive::DirArchive::new(&dir).with_quota(crate::archive::Quota { records: 1, bytes: 1 << 20 }),
        ));
        let (status, v) = post_archive(&on, PLAYED_RECORD, PLAYED_TABLE);
        assert_eq!(status, 201, "{v}");
        let second = keyed(PLAYED_TABLE, "id", "local-second".into());
        let (status, v) = post_archive(&on, PLAYED_RECORD, &second);
        assert_eq!(status, 507, "{v}");
        assert_eq!(v["error"], archive_full_sentence(1, 1 << 20));
        // The prefix as the route builds it, for the caller `post_archive` sends as.
        let caller = User::parse(ARCHIVE_CALLER).expect("an address");
        let (record, table) = crate::archive::paths(
            yokohama_core::SEMANTICS_REV,
            &format!("browser/{}/", crate::archive::caller_segment(&caller)),
            crate::archive::Why::Over,
            "local-second",
        )
        .unwrap();
        assert!(!dir.join(&record).exists() && !dir.join(&table).exists(), "nothing was written");
        let (status, v) = post_archive(&on, PLAYED_RECORD, PLAYED_TABLE);
        assert_eq!((status, v["identical"].as_bool()), (200, Some(true)), "{v}");
        std::fs::remove_dir_all(&dir).ok();
    }

    // -----------------------------------------------------------------------
    // M6.5c2 cycle D: the chat's delete, through `dispatch` (R-D4)
    // -----------------------------------------------------------------------

    /// The only sentence a delete is refused with for want of standing.
    const NOT_YOURS: &str = "only the message's author or the table's creator may delete it";

    /// A playing THREE-person table, created by red: red, blue and green each
    /// keep through the route helper, so the third seat — neither the creator
    /// nor, below, the author — is a person the table admits.
    fn three_people_playing(games: &Games, id: &str) {
        use crate::tables::SeatSpec;
        use yokohama_core::PlayerColor;
        let mut req = crate::games::test_support::req_seeded(id, crate::games::test_support::CORPUS_SEED);
        req.players = 3;
        req.colors = vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Green];
        req.seats = ["red@t.test", "blue@t.test", "green@t.test"]
            .iter()
            .map(|u| SeatSpec { user: Some((*u).to_string()), bot: false, open: false })
            .collect();
        crate::games::test_support::playing(games, &req, &["red@t.test", "blue@t.test", "green@t.test"]);
    }

    /// **A message is deleted by its author or the table's creator, and by
    /// NOBODY else** (M6.5c2 cycle D, R-D4; Review Focus 2) — the catcher for
    /// "a third seat may delete".
    ///
    /// Three callers who must be refused: green, a third SEAT who neither wrote
    /// blue's message nor created the table; grey, a WATCHER, on a seat's
    /// message; and blue, a seat, on the CREATOR's message. Two who may: blue
    /// on her own, and red — the creator — on a watcher's. The watcher's own
    /// second delete is the idempotent 204. Every refusal is the one sentence,
    /// and the page afterwards shows exactly the two tombstones.
    #[test]
    fn a_chat_message_is_deleted_by_its_author_or_the_tables_creator_and_nobody_else() {
        let games = games_with(std::sync::Arc::new(crate::archive::NoArchive));
        three_people_playing(&games, "three");
        let chat = "/api/games/three/chat";
        let post = |who: &str, body: &str| ask(&games, who, "POST", chat, "", body);
        assert_eq!(post("blue@t.test", r#"{"text":"blue's"}"#).1["seq"], 1);
        assert_eq!(post("grey@t.test", r#"{"text":"grey's","name":"Grey"}"#).1["seq"], 2, "a watcher posts");
        assert_eq!(post("red@t.test", r#"{"text":"red's"}"#).1["seq"], 3);
        let delete = |who: &str, seq: &str| ask(&games, who, "DELETE", &format!("{chat}/{seq}"), "", "");

        for (who, seq, what) in [
            ("green@t.test", "1", "a third seat, on another seat's message"),
            ("grey@t.test", "1", "a watcher, on a seat's message"),
            ("blue@t.test", "3", "a seat, on the creator's message"),
        ] {
            let (status, v) = delete(who, seq);
            assert_eq!((status, v["error"].as_str()), (403, Some(NOT_YOURS)), "{what}: {v}");
        }
        assert_eq!(delete("blue@t.test", "1"), (204, serde_json::Value::Null), "the author, an empty 204");
        assert_eq!(delete("red@t.test", "2"), (204, serde_json::Value::Null), "the creator, on a watcher's message");
        assert_eq!(delete("grey@t.test", "2"), (204, serde_json::Value::Null), "the watcher's own, again: idempotent");

        let (status, page) = ask(&games, "green@t.test", "GET", chat, "", "");
        assert_eq!(status, 200, "{page}");
        let shape: Vec<(u64, bool, String)> = page["messages"]
            .as_array()
            .expect("messages")
            .iter()
            .map(|m| (m["seq"].as_u64().expect("a seq"), m["deleted"] == true, m["text"].as_str().expect("text").to_string()))
            .collect();
        assert_eq!(
            shape,
            [(1, true, String::new()), (2, true, String::new()), (3, false, "red's".to_string())],
            "the two deleted, and only those: {page}",
        );
        assert_eq!(page["last"], 3, "no number moved");
        // **The page carries the document's REVISION** (R-D4): three posts and
        // two deletes — the watcher's repeat changed nothing — so a reader
        // holding `rev - last` from an earlier page sees it move by two.
        assert_eq!(page["rev"], 5, "{page}");
    }

    /// **The delete is admitted exactly as the post is, and its own answers
    /// follow** (R-D4): a stranger at an open table, at a dealing one and at a
    /// LOCKED playing one reads `admitted`'s own sentence — the seat one, the
    /// seat one, and the lock's — before anything about the message; then, for
    /// a caller admitted, a number nobody was given is a 404, and a segment
    /// that is no number is axum's own 400 for a `u64` path segment.
    #[test]
    fn a_chat_delete_is_admitted_as_a_post_is() {
        use crate::games::test_support::{playing, req_seeded, two_people, CORPUS_SEED};
        use crate::tables::SeatSpec;
        let games = games_with(std::sync::Arc::new(crate::archive::NoArchive));
        let red = User::parse("red@t.test").expect("an address");
        let mut open = req_seeded("opent", CORPUS_SEED);
        open.seats[1] = SeatSpec { user: None, bot: false, open: true };
        games.create(&open, &red).expect("an open table");
        games.create(&two_people("dealt", CORPUS_SEED), &red).expect("a dealing table");
        let mut locked = two_people("lockt", CORPUS_SEED);
        locked.password = Some("sesame".to_string());
        playing(&games, &locked, &["red@t.test", "blue@t.test"]);
        for id in ["opent", "dealt", "lockt"] {
            let (status, v) = ask(&games, "red@t.test", "POST", &format!("/api/games/{id}/chat"), "", r#"{"text":"hi"}"#);
            assert_eq!(status, 201, "{id}: {v}");
        }

        for (id, sentence) in [
            ("opent", "you are not seated at this table"),
            ("dealt", "you are not seated at this table"),
            ("lockt", "this table is locked — unlock it first"),
        ] {
            let (status, v) = ask(&games, "green@t.test", "DELETE", &format!("/api/games/{id}/chat/1"), "", "");
            assert_eq!((status, v["error"].as_str()), (403, Some(sentence)), "a stranger at {id}: {v}");
        }
        let (status, v) = ask(&games, "red@t.test", "DELETE", "/api/games/dealt/chat/9", "", "");
        assert_eq!((status, v["error"].as_str()), (404, Some("no message 9 in this chat")), "{v}");
        let (status, v) = ask(&games, "red@t.test", "DELETE", "/api/games/dealt/chat/one", "", "");
        assert_eq!(
            (status, v["error"].as_str()),
            (400, Some("Invalid URL: Cannot parse value at index 1 with value `one` to a `u64`")),
            "{v}",
        );
        let (status, v) = ask(&games, "red@t.test", "DELETE", "/api/games/nothing/chat/1", "", "");
        assert_eq!(status, 404, "no table, no message: {v}");
    }

    /// **The house's delete: the author alone, and the switch before
    /// anything** (R-D4). The house has no creator, so blue may not take down
    /// red's post; red may. With the lobby chat off the route is the switch's
    /// 404, as the read and the post are.
    #[test]
    fn a_house_message_is_deleted_by_its_author_alone() {
        let games = games_with(std::sync::Arc::new(crate::archive::NoArchive));
        let (status, m) = ask(&games, "red@t.test", "POST", "/api/chat", "", r#"{"text":"anyone?","name":"Red"}"#);
        assert_eq!(status, 201, "{m}");
        let (status, v) = ask(&games, "blue@t.test", "DELETE", "/api/chat/1", "", "");
        assert_eq!((status, v["error"].as_str()), (403, Some(NOT_YOURS)), "{v}");
        assert_eq!(ask(&games, "red@t.test", "DELETE", "/api/chat/1", "", ""), (204, serde_json::Value::Null));
        let (_, page) = ask(&games, "blue@t.test", "GET", "/api/chat", "", "");
        assert_eq!(
            (page["messages"][0]["deleted"].as_bool(), page["last"].as_u64(), page["rev"].as_u64()),
            (Some(true), Some(1), Some(2)),
            "a tombstone, the number held, the revision moved: {page}",
        );
        let (status, v) = ask(&games, "red@t.test", "DELETE", "/api/chat/x", "", "");
        assert_eq!(
            (status, v["error"].as_str()),
            (400, Some("Invalid URL: Cannot parse value at index 0 with value `x` to a `u64`")),
            "{v}",
        );

        let off = Games::with_parts(
            std::sync::Arc::new(yokohama_core::Rules::standard()),
            std::sync::Arc::new(crate::store::MemStore::default()),
            crate::chat::ChatConfig { lobby: false, ..crate::chat::ChatConfig::default() },
            std::sync::Arc::new(crate::signal::NoSignal),
            std::sync::Arc::new(|| 1_789_000_000u64),
            std::sync::Arc::new(crate::archive::NoArchive),
            crate::games::RunHost::Server,
        )
        .with_seed_policy(crate::games::SeedPolicy::Diagnostic);
        let (status, v) = ask(&off, "red@t.test", "DELETE", "/api/chat/1", "", "");
        assert_eq!((status, v["error"].as_str()), (404, Some("the lobby chat is off")), "{v}");
    }

    /// **`Events.chat_rev` moves on a delete while `chat` does not** (R-D4;
    /// Review Focus 2's "the client learns of the deletion at the next frame")
    /// — at an open table, where there is no session, and at a playing one,
    /// through `events_of`. `chat` is a count for the text and a delete adds
    /// no text; `chat_rev` is the revision for the shape, and the tombstone
    /// changed it.
    #[test]
    fn events_carries_chat_rev_which_a_delete_moves_and_chat_does_not() {
        use crate::games::test_support::{playing, req_seeded, two_people, CORPUS_SEED};
        use crate::tables::SeatSpec;
        let games = games_with(std::sync::Arc::new(crate::archive::NoArchive));
        let red = User::parse("red@t.test").expect("an address");
        let mut open = req_seeded("opent", CORPUS_SEED);
        open.seats[1] = SeatSpec { user: None, bot: false, open: true };
        games.create(&open, &red).expect("an open table");
        playing(&games, &two_people("playt", CORPUS_SEED), &["red@t.test", "blue@t.test"]);
        for id in ["opent", "playt"] {
            let (status, v) = ask(&games, "red@t.test", "POST", &format!("/api/games/{id}/chat"), "", r#"{"text":"hi"}"#);
            assert_eq!(status, 201, "{v}");
            let e = events_at(&games, id, &red).unwrap_or_else(|e| panic!("a reading: {}", e.message));
            assert_eq!((e.chat, e.chat_rev), (1, 1), "{id}: one append moves both");
            assert_eq!(ask(&games, "red@t.test", "DELETE", &format!("/api/games/{id}/chat/1"), "", "").0, 204);
            let e = events_at(&games, id, &red).unwrap_or_else(|e| panic!("a reading: {}", e.message));
            assert_eq!((e.chat, e.chat_rev), (1, 2), "{id}: the delete moves the revision alone");
            let wire = serde_json::to_value(&e).expect("an Events serializes");
            assert_eq!((wire["chat"].as_u64(), wire["chat_rev"].as_u64()), (Some(1), Some(2)), "{wire}");
        }
    }

    #[test]
    fn the_matcher_finds_every_route_in_apps_order() {
        // Every pattern `app()` registered before M6.5a, in its order, plus the
        // record route: a hand-kept copy that is a change detector for the table.
        // A route missing here is a route the dispatcher cannot serve. (The order
        // is not load-bearing for `find`: no two patterns can match one path.)
        let expected = [
            ("GET", "/api/health"),
            ("GET", "/api/me"),
            ("GET", "/api/labels"),
            ("GET", "/api/art"),
            ("GET", "/api/games"),
            ("POST", "/api/games"),
            ("GET", "/api/games/{id}"),
            ("DELETE", "/api/games/{id}"),
            ("GET", "/api/games/{id}/table"),
            ("GET", "/api/games/{id}/deal"),
            ("POST", "/api/games/{id}/keep"),
            ("POST", "/api/games/{id}/join"),
            ("POST", "/api/games/{id}/unlock"),
            ("GET", "/api/games/{id}/chat"),
            ("POST", "/api/games/{id}/chat"),
            ("DELETE", "/api/games/{id}/chat/{seq}"),
            ("POST", "/api/games/{id}/input"),
            ("GET", "/api/games/{id}/ports"),
            ("GET", "/api/games/{id}/labs"),
            ("GET", "/api/games/{id}/areas"),
            ("GET", "/api/games/{id}/log"),
            ("GET", "/api/chat"),
            ("POST", "/api/chat"),
            ("DELETE", "/api/chat/{seq}"),
            ("POST", "/api/archive"),
            ("GET", "/api/games/{id}/at"),
            ("GET", "/api/games/{id}/at/{k}"),
            ("GET", "/api/games/{id}/suggest"),
            ("GET", "/api/games/{id}/why"),
            ("GET", "/api/games/{id}/index/{kind}"),
            ("GET", "/api/games/{id}/result"),
            ("GET", "/api/games/{id}/record"),
        ];
        let table = routes(true);
        let names: Vec<(&str, &str)> = table.iter().map(|r| (r.method, r.pattern)).collect();
        assert_eq!(names, expected.to_vec());
    }

    /// **An undo over a record that will not replay is the SERVER's failure — a
    /// 500 carrying the replay's own sentence — and never a 403** (M6.5c2 cycle
    /// D, straw poll row 5). A 403 tells the caller the RULE refused them, and
    /// here no rule was consulted: the record could not name who made its last
    /// move.
    ///
    /// **No route reaches this arm, and the cell says why rather than
    /// pretending otherwise.** A resident `Repl` exists only through
    /// `Session::from_record` (or a session that applied every event itself),
    /// which folds the SAME `new_game` → `finalize_setup` → fold chain over the
    /// whole record; a prefix of a record that folds, folds. Since security S1b
    /// the mover's derivation reads a SESSION's ladder (`trailing_state`), and
    /// a session cannot hold a HAND-BUILT record that will not replay (a
    /// 5-Power take before any placement — asserted as a premise through
    /// `state_at` itself): `from_record` refuses it, asserted here. So the
    /// cell holds the derivation at the two answers a session can reach — "nothing
    /// to undo" on an empty record, the state `state_at` names on a played one
    /// — and the arm at the one mapping `input` sends [`UndoRefusal`] through,
    /// with the replay's own sentence. The rule's own refusal is read through
    /// that mapping too, still a 403. **`trailing_state`'s `Replay` arm is
    /// unreachable by construction** — a `Session` cannot hold a record that
    /// will not replay, since `from_record` refuses it — so the cell asserts
    /// that refusal and the two answers a session can reach, and the arm's
    /// `map_err(UndoRefusal::Replay)` is read, not run: what runs is the status
    /// a `Replay` value maps to.
    #[test]
    fn undo_on_a_record_that_will_not_replay_is_a_500_not_a_403() {
        use yokohama_cli::session::Session;
        use yokohama_core::{GameConfig, GameRecord, Move, RecordedEvent, StartingOrderChoice};
        let rules = Rules::standard();
        let mut record =
            GameRecord::new(7, GameConfig::for_player_count(2), vec![StartingOrderChoice::Keep(0); 2], &rules);
        record.events = vec![RecordedEvent::Move(Move::Take5PowerToken), RecordedEvent::Move(Move::SkipBuilding)];
        let sentence = yokohama_core::state_at(&record, 1, &rules)
            .expect_err("premise: the record will not replay")
            .to_string();
        assert!(
            Session::from_record(record, rules.clone()).is_err(),
            "no session holds the record this arm would need"
        );

        let mut session = Session::new_sim(
            GameConfig::for_player_count(2),
            7,
            vec![StartingOrderChoice::Keep(0); 2],
            rules.clone(),
        )
        .expect("a game");
        assert_eq!(trailing_state(&mut session), Err(UndoRefusal::Refused("nothing to undo".to_string())));
        let mv = session.legal()[0].clone();
        session.apply(mv, Vec::new()).expect("a listed move applies");
        let before = trailing_state(&mut session).expect("the mover's state");
        assert_eq!(before, yokohama_core::state_at(session.record(), 0, &rules).expect("position 0"));

        let refusal = UndoRefusal::Replay(sentence.clone());
        let answer = ApiError::from(crate::games::GameError::from(refusal));
        assert_eq!((answer.status, answer.message), (StatusCode::INTERNAL_SERVER_ERROR, sentence));

        let rule = "undo is not offered once your turn has passed";
        let answer = ApiError::from(crate::games::GameError::from(UndoRefusal::Refused(rule.to_string())));
        assert_eq!((answer.status, answer.message.as_str()), (StatusCode::FORBIDDEN, rule));
    }

    #[test]
    fn the_matcher_binds_segments_and_tells_a_wrong_method_from_no_route() {
        match find("GET", "/api/games/sim-1/at/7", true) {
            Matched::Route(p, b) => {
                assert_eq!(p, "/api/games/{id}/at/{k}");
                assert_eq!(b, vec!["sim-1", "7"]);
            }
            _ => panic!("a known route"),
        }
        assert!(matches!(find("HEAD", "/api/health", true), Matched::Route("/api/health", _)), "HEAD reaches GET");
        assert!(matches!(find("DELETE", "/api/health", true), Matched::WrongMethod));
        assert!(matches!(find("GET", "/api/nothing", true), Matched::NoRoute));
        // Since M6.5b the five-segment `…/at` is the RUN route (R-6.5b.10), and
        // the six-segment `…/at/{k}` beside it: the matcher is segment-wise.
        match find("GET", "/api/games/sim-1/at", true) {
            Matched::Route(p, b) => {
                assert_eq!(p, "/api/games/{id}/at");
                assert_eq!(b, vec!["sim-1"]);
            }
            _ => panic!("the run route"),
        }
        assert!(matches!(find("GET", "/api/games/sim-1/index", true), Matched::NoRoute), "a short path binds nothing");
        // An EMPTY segment binds nothing (M6.5c2 cycle D, straw poll row 10):
        // bound, `/api/games//table` reached `get_table` and answered the
        // store's "not a usable game name" as a 500, where axum's matchit
        // answers 404 — so no route, in the middle and at the end alike.
        assert!(matches!(find("GET", "/api/games//table", true), Matched::NoRoute), "an empty {{id}}");
        assert!(matches!(find("GET", "/api/games/sim-1/at/", true), Matched::NoRoute), "an empty {{k}}");
    }

    /// **The replay gate's five routes, by the table's own match** (security
    /// S1b, findings A2 and D6): an archive upload, a run and a position of a
    /// game's history, the two searches, and a HEAD of each gated GET, which
    /// `find` folds onto it and `dispatch` pays the replay for — and nothing
    /// else: a move, a frame, the record, the health, a wrong method on a
    /// gated path, a path the table does not route. Each gated pattern routes
    /// ONE method in the table, so the pattern is the whole key: a second
    /// method a future row adds to a gated pattern fails here, not unseen.
    #[test]
    fn the_replay_routes_are_the_five_that_replay_or_search() {
        for (method, path) in [
            ("POST", "/api/archive"),
            ("GET", "/api/games/wire/at"),
            ("GET", "/api/games/wire/at/12"),
            ("GET", "/api/games/wire/suggest"),
            ("GET", "/api/games/wire/why"),
            ("HEAD", "/api/games/wire/at"),
            ("HEAD", "/api/games/wire/at/12"),
            ("HEAD", "/api/games/wire/suggest"),
            ("HEAD", "/api/games/wire/why"),
        ] {
            assert!(is_replay_route(method, path), "{method} {path} is gated");
        }
        for (method, path) in [
            ("POST", "/api/games/wire/input"),
            ("GET", "/api/games/wire"),
            ("GET", "/api/games/wire/record"),
            ("GET", "/api/health"),
            ("GET", "/api/archive"),
            ("HEAD", "/api/archive"),
            ("POST", "/api/games/wire/at"),
            ("GET", "/api/games//at"),
            ("GET", "/api/games/wire/at/12/13"),
        ] {
            assert!(!is_replay_route(method, path), "{method} {path} is never gated");
        }
        assert_eq!(REPLAY_ROUTES.len(), 5);
        let table = routes(true);
        for (method, pattern) in REPLAY_ROUTES {
            let routed: Vec<&str> = table.iter().filter(|r| r.pattern == pattern).map(|r| r.method).collect();
            assert_eq!(routed, vec![method], "{pattern} routes {method} alone");
        }
    }

    /// **`events` reads the table document ONCE per call** (M6.5c2 cycle D, row
    /// 129) — at every status, for a seat holder and for a watcher, with the
    /// handle resident and with it cold.
    ///
    /// It read it twice: once itself, to answer an open table, and again inside
    /// `admitted`. That is every poll, every NAS stream frame per connection
    /// per bump, and every Worker reading, where a read is a row — and a COLD
    /// handle added a third, since `Games::handle` settles the table again
    /// before it rebuilds. The count is the store's, so a route that read the
    /// same document twice and served it once cannot pass.
    #[test]
    fn events_at_reads_the_table_once() {
        use std::sync::Arc;
        use crate::games::test_support::*;
        use crate::store::Store;
        use crate::tables::SeatSpec;
        let store = Arc::new(CountingStore::default());
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let (red, green) = (who("red@t.test"), who("green@t.test"));
        let reads = |id: &str, user: &User| {
            let before = store.table_reads();
            events_at(&games, id, user).unwrap_or_else(|e| panic!("{id}: {}", e.message));
            store.table_reads() - before
        };

        let mut open = two_people("open", CORPUS_SEED);
        open.seats[1] = SeatSpec { user: None, bot: false, open: true };
        games.create(&open, &red).expect("an open table");
        games.create(&two_people("dealing", CORPUS_SEED), &red).expect("a dealing table");
        playing(&games, &two_people("playing", CORPUS_SEED), &["red@t.test", "blue@t.test"]);

        assert_eq!(reads("open", &red), 1, "open");
        assert_eq!(reads("dealing", &red), 1, "dealing");
        assert_eq!(reads("playing", &red), 1, "playing, the handle resident");
        assert_eq!(reads("playing", &green), 1, "playing, a watcher");
        games.evict_for_test("playing");
        assert_eq!(reads("playing", &red), 1, "playing, the handle cold");
        games.evict_for_test("playing");
        assert_eq!(reads("playing", &green), 1, "playing, a watcher, the handle cold");
    }

    /// **The non-acting refusal names the seat, never its holder's address**
    /// (M6.5c2 cycle D, R-6.5c2.35, R-D3): `it is seat {n}'s turn (…)` carries
    /// the seat's NAME, or its COLOUR when its holder gave none. It was the
    /// holder's email, handed to every other seated caller who pressed a key
    /// out of turn.
    ///
    /// Two tables, one per arm: seat 1 unnamed reads its colour, and seat 1
    /// named reads the name — so a sentence that always said the colour would
    /// fail the second half.
    #[test]
    fn a_non_acting_callers_refusal_names_the_seat_by_name_or_colour_never_an_address() {
        use crate::games::test_support::{games, two_people, who, CORPUS_SEED};
        let games = games();
        for (id, blue_name, said) in [("colour", None, "Blue"), ("named", Some("Ada Bell"), "Ada Bell")] {
            let mut req = two_people(id, CORPUS_SEED);
            req.start_seat = 1;
            let (red, blue) = (who("red@t.test"), who("blue@t.test"));
            games.create(&req, &red).expect("a table");
            games.keep(id, &red, yokohama_core::PlayerId::new(0), 1, None).expect("red keeps");
            games.keep(id, &blue, yokohama_core::PlayerId::new(1), 1, blue_name).expect("blue keeps");
            let (status, body) = ask(&games, "red@t.test", "POST", &format!("/api/games/{id}/input"), "", r#"{"line":"1"}"#);
            assert_eq!(status, 403, "{body}");
            assert_eq!(body["error"], format!("it is seat 1's turn ({said})"), "{id}");
            assert!(!body.to_string().contains('@'), "{id}: no address in the refusal: {body}");
        }
    }

    /// **The reply to a move whose write CONFLICTED says so, out loud** (M6.5c2
    /// cycle D, straw poll row 31): the move applied, the store answered
    /// `Conflict` at the swap, and the 200 carries `APPLIED BUT NOT SAVED`
    /// instead of a refusal the server did not make. Its games.rs partner
    /// asserts the eviction; this asserts the sentence on the wire, and is not
    /// a catcher for a conflict that keeps the handle (the battery's row 25):
    /// the sentence is the same either way.
    #[test]
    fn a_move_whose_write_conflicted_is_answered_applied_but_not_saved() {
        use crate::games::test_support::{conflicting_games, req, who};
        use std::sync::atomic::Ordering;
        let (games, store) = conflicting_games();
        let red = who("red@t.test");
        games.create(&req("cnf"), &red).expect("a table");
        games.keep("cnf", &red, yokohama_core::PlayerId::new(0), 1, None).expect("red keeps");
        store.armed.store(true, Ordering::SeqCst);
        let (status, body) = ask(&games, "red@t.test", "POST", "/api/games/cnf/input", "", r#"{"line":"1"}"#);
        assert_eq!(status, 200, "{body}");
        let warnings = body["warnings"].as_array().expect("warnings ride the reply");
        assert!(
            warnings.iter().any(|w| w.as_str().is_some_and(|w| w.starts_with("APPLIED BUT NOT SAVED")
                && w.contains("changed under this handle"))),
            "{body}",
        );
    }

    /// **A hot seat's holder is never told it is not their turn** (M6.5c2 cycle
    /// D, row 129) — the catcher for an acting-seat fence that compares against
    /// the seat `admitted` read under its OWN lock take.
    ///
    /// `input` takes the game's lock twice: once inside the admission, to learn
    /// which of the caller's seats they act as, and again to act. A move landing
    /// between the two — the same person's other tab, or a double click —
    /// moved the turn from seat 0 to seat 1, and the fence then told the person
    /// holding BOTH seats "it is seat 1's turn" about their own seat. It never
    /// admitted anybody wrongly; it refused the one person it should have let
    /// through.
    ///
    /// **The gap is held open on the `live` map's lock, not on the store**:
    /// with the handle resident nothing between the two takes reaches the store,
    /// and the second take's `Games::handle` is the one step that passes through
    /// that map. So the cell holds the game's lock (the request waits at its
    /// first take), takes the map's lock, lets the first take through, waits
    /// for the request to drop its handle (the first take is over), plays seat
    /// 0's whole turn in the gap, and only then lets the second take through.
    ///
    /// **Nothing can panic while the cell holds either lock**: every wait is
    /// bounded and answers a bool, the gap's moves report rather than `expect`,
    /// and every assertion runs after both guards are gone and the request has
    /// been joined — so a failure releases `live` and the game's lock unpoisoned
    /// and never leaves the request thread waiting on either.
    #[test]
    fn a_hot_seat_creator_is_never_told_it_is_not_their_turn_between_two_lock_takes() {
        use std::sync::Arc;
        use crate::games::test_support::*;
        use crate::tables::SeatSpec;
        use yokohama_core::PlayerId;
        /// Spin until `done`, for at most 20 s; whether it came true.
        fn waited(done: impl Fn() -> bool) -> bool {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
            while !done() {
                if std::time::Instant::now() >= deadline {
                    return false;
                }
                std::thread::yield_now();
            }
            true
        }
        let games = games();
        let red = who("red@t.test");
        let mut hot = two_people("hot", CORPUS_SEED);
        hot.seats[1] = SeatSpec { user: Some("red@t.test".into()), bot: false, open: false };
        let handle = playing(&games, &hot, &["red@t.test"]);
        assert_eq!(handle.lock().unwrap().session.state().current_player, PlayerId::new(0), "premise: seat 0 opens");
        let resident = Arc::strong_count(&handle);
        let req = Input { line: "1".to_string(), expect: None };

        let (took_the_handle, first_take_over, gap, answer) = std::thread::scope(|scope| {
            let game = handle.lock().unwrap();
            let request = scope.spawn(|| input(&games, &red, "hot", &req));
            let took_the_handle = waited(|| Arc::strong_count(&handle) > resident);
            let live = games.live.lock().unwrap();
            drop(game);
            let first_take_over = took_the_handle && waited(|| Arc::strong_count(&handle) == resident);
            // Seat 0's whole turn, played in the gap — reported, never unwrapped.
            let gap: Result<PlayerId, String> = if !first_take_over {
                Err("the gap never opened".to_string())
            } else {
                match handle.lock() {
                    Err(_) => Err("the game's lock was poisoned".to_string()),
                    Ok(mut repl) => {
                        let mut played = Ok(());
                        for _ in 0..200 {
                            if repl.session.state().current_player != PlayerId::new(0) {
                                break;
                            }
                            let Some(mv) = repl.session.legal().first().cloned() else {
                                played = Err("seat 0 had no legal move".to_string());
                                break;
                            };
                            if let Err(e) = repl.session.apply(mv, Vec::new()) {
                                played = Err(format!("a legal move did not apply: {e:?}"));
                                break;
                            }
                        }
                        played.map(|()| repl.session.state().current_player)
                    }
                }
            };
            drop(live);
            (took_the_handle, first_take_over, gap, request.join())
        });
        assert!(took_the_handle, "timed out: the request never took the handle");
        assert!(first_take_over, "timed out: the request's first take never ended");
        assert_eq!(gap, Ok(PlayerId::new(1)), "premise: the gap moved the turn to seat 1");
        match answer {
            Err(_) => panic!("the request panicked"),
            Ok(Err(e)) => panic!("the hot seat's holder was refused between two lock takes: {} {}", e.status, e.message),
            Ok(Ok(_)) => {}
        }
    }
}
