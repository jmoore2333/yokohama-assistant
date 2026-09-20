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

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::games::Games;
use crate::identity::User;
use crate::ApiError;

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

/// The two deployment facts `/api/health` and `/api/me` report that the layer
/// cannot know: which identity door this host has, and which change transport.
#[derive(Debug, Clone, Copy)]
pub struct Host {
    pub identity_name: &'static str,
    pub signal_kind: &'static str,
}

/// The caller, or the host's own sentence for why there is none.
pub type Identity<'a> = Result<&'a User, &'a str>;

pub(crate) fn ok_json<T: Serialize>(status: u16, v: &T) -> ApiResponse {
    ApiResponse { status, body: serde_json::to_string(v).expect("every payload serializes") }
}

/// The body `IntoResponse for ApiError` writes, byte for byte: `{"error": …}`
/// with `"status"` beside it on the one refusal a client branches on.
pub(crate) fn error_response(e: ApiError) -> ApiResponse {
    let body = match e.table_status {
        Some(status) => serde_json::json!({ "error": e.message, "status": status }),
        None => serde_json::json!({ "error": e.message }),
    };
    ApiResponse { status: e.status.as_u16(), body: body.to_string() }
}

fn unauthorized(msg: &str) -> ApiError {
    ApiError { status: http::StatusCode::UNAUTHORIZED, message: msg.to_string(), table_status: None }
}

/// axum 0.8.9's `Json<T>` extractor, call for call (`json.rs`): the content-type
/// rule through `mime`, `serde_path_to_error` over `serde_json`'s deserializer,
/// the error classified into the DATA sentence or the SYNTAX sentence, and
/// `end()` so trailing input is the syntax sentence too. The three sentences
/// are `JsonDataError`'s, `JsonSyntaxError`'s and `MissingJsonContentType`'s.
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
pub fn routes() -> Vec<Route> {
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
        ("POST", "/api/games/{id}/input"),
        ("GET", "/api/games/{id}/ports"),
        ("GET", "/api/games/{id}/labs"),
        ("GET", "/api/games/{id}/areas"),
        ("GET", "/api/games/{id}/log"),
        ("GET", "/api/chat"),
        ("POST", "/api/chat"),
        ("GET", "/api/games/{id}/at/{k}"),
        ("GET", "/api/games/{id}/suggest"),
        ("GET", "/api/games/{id}/why"),
        ("GET", "/api/games/{id}/index/{kind}"),
        ("GET", "/api/games/{id}/result"),
    ]
    .into_iter()
    .map(|(method, pattern)| Route { method, pattern })
    .collect()
}

/// Segment-wise match; `{name}` binds one segment. Returns the bound values in
/// pattern order. The ids this API accepts are `store::is_safe_id`'s ASCII
/// set, so a bound segment needs no percent-decoding.
fn matches<'a>(pattern: &str, path: &'a str) -> Option<Vec<&'a str>> {
    let mut bound = Vec::new();
    let mut ps = pattern.split('/');
    let mut xs = path.split('/');
    loop {
        match (ps.next(), xs.next()) {
            (None, None) => return Some(bound),
            (Some(p), Some(x)) if p.starts_with('{') => bound.push(x),
            (Some(p), Some(x)) if p == x => {}
            _ => return None,
        }
    }
}

enum Matched<'a> {
    Route(&'static str, Vec<&'a str>),
    WrongMethod,
    NoRoute,
}

/// HEAD reaches the GET arm (R-6.5a.3 — axum's `get` serves HEAD that way);
/// [`dispatch`] strips the body afterwards.
fn find<'a>(method: &str, path: &'a str) -> Matched<'a> {
    let method = if method == "HEAD" { "GET" } else { method };
    let mut path_known = false;
    for r in routes() {
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
    /// the client's polling hook has to know whether to open `…/stream` or to
    /// hold a request against `…/events?wait=25`, and it cannot tell from a
    /// successful call to anything else.
    pub signal: &'static str,
    /// Whether the house chat routes answer (M6.4, R-6.4.6). Off, the client
    /// draws no lobby panel and asks nothing.
    pub lobby_chat: bool,
    /// Messages retained per chat document; `null` for `all` (R-6.4.8).
    pub chat_keep: Option<u64>,
    /// Characters per message, so the client sizes its input from the
    /// server's rule and never from a copy (R-6.4.8).
    pub chat_text: u64,
}

pub fn health(games: &Games, host: &Host) -> Health {
    Health {
        server_version: env!("CARGO_PKG_VERSION"),
        semantics_rev: yokohama_core::SEMANTICS_REV,
        rules_hash: games.rules().content_hash(),
        identity: host.identity_name,
        signal: host.signal_kind,
        lobby_chat: games.chat.lobby,
        chat_keep: games.chat.limits.keep.map(|n| n as u64),
        chat_text: games.chat.limits.text as u64,
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

/// The layer's front door: match, answer, and shape the response. A known path
/// under another method is 405 with an empty body (axum's own shape); a path
/// that matches nothing is the JSON 404 the server's `fallback` writes.
pub fn dispatch(games: &Games, host: &Host, identity: Identity<'_>, req: &ApiRequest<'_>) -> ApiResponse {
    let (pattern, bound) = match find(req.method, req.path) {
        Matched::Route(p, b) => (p, b),
        Matched::WrongMethod => return ApiResponse { status: 405, body: String::new() },
        Matched::NoRoute => {
            return error_response(ApiError::not_found(format!("no such endpoint: {}", req.path)))
        }
    };
    match answer(games, host, identity, req, pattern, &bound) {
        Ok(r) if req.method == "HEAD" => ApiResponse { status: r.status, body: String::new() },
        Ok(r) => r,
        Err(e) => error_response(e),
    }
}

/// One arm per route; `?` inside an arm is the refusal order axum has today —
/// identity before body or query, because the `Identity` extractor precedes
/// `JsonBody`/`QueryArgs` in every handler's argument list at HEAD.
fn answer(
    games: &Games,
    host: &Host,
    identity: Identity<'_>,
    req: &ApiRequest<'_>,
    pattern: &'static str,
    b: &[&str],
) -> Result<ApiResponse, ApiError> {
    match pattern {
        "/api/health" => Ok(ok_json(200, &health(games, host))),
        "/api/me" => Ok(ok_json(200, &me(host, identity))),
        "/api/labels" => Ok(ok_json(200, &labels(games))),
        "/api/art" => Err(ApiError::not_found("art is served by the host".to_string())),
        _ => {
            // Task 6 adds the game and house arms below this line; the identity
            // check stays FIRST in the fall-through so the public four never
            // touch it.
            let u = identity.map_err(unauthorized)?;
            let _ = (u, b);
            Err(ApiError::not_found(format!("no such endpoint: {}", req.path)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn the_matcher_finds_every_route_in_apps_order() {
        // Every pattern app() registers, in its order; a route missing here is a
        // route the dispatcher cannot serve.
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
            ("POST", "/api/games/{id}/input"),
            ("GET", "/api/games/{id}/ports"),
            ("GET", "/api/games/{id}/labs"),
            ("GET", "/api/games/{id}/areas"),
            ("GET", "/api/games/{id}/log"),
            ("GET", "/api/chat"),
            ("POST", "/api/chat"),
            ("GET", "/api/games/{id}/at/{k}"),
            ("GET", "/api/games/{id}/suggest"),
            ("GET", "/api/games/{id}/why"),
            ("GET", "/api/games/{id}/index/{kind}"),
            ("GET", "/api/games/{id}/result"),
        ];
        let table = routes();
        let names: Vec<(&str, &str)> = table.iter().map(|r| (r.method, r.pattern)).collect();
        assert_eq!(names, expected.to_vec());
    }

    #[test]
    fn the_matcher_binds_segments_and_tells_a_wrong_method_from_no_route() {
        match find("GET", "/api/games/sim-1/at/7") {
            Matched::Route(p, b) => {
                assert_eq!(p, "/api/games/{id}/at/{k}");
                assert_eq!(b, vec!["sim-1", "7"]);
            }
            _ => panic!("a known route"),
        }
        assert!(matches!(find("HEAD", "/api/health"), Matched::Route("/api/health", _)), "HEAD reaches GET");
        assert!(matches!(find("DELETE", "/api/health"), Matched::WrongMethod));
        assert!(matches!(find("GET", "/api/nothing"), Matched::NoRoute));
        assert!(matches!(find("GET", "/api/games/sim-1/at"), Matched::NoRoute), "a short path binds nothing");
    }
}
