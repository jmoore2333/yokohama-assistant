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
        ("POST", "/api/games/{id}/input"),
        ("GET", "/api/games/{id}/ports"),
        ("GET", "/api/games/{id}/labs"),
        ("GET", "/api/games/{id}/areas"),
        ("GET", "/api/games/{id}/log"),
        ("GET", "/api/chat"),
        ("POST", "/api/chat"),
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

/// The rows the multiplayer switch takes away: every game route and both house
/// chat routes. A prefix test and not a list, so a route added under
/// `/api/games` is governed by the switch the day it lands rather than the day
/// someone remembers to add it here.
fn is_game_route(pattern: &str) -> bool {
    pattern.starts_with("/api/games") || pattern.starts_with("/api/chat")
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

/// The methods the table takes at `path`, in table order, with `HEAD` beside
/// `GET` as axum's `MethodRouter` lists them — what the adapter writes into a
/// 405's `Allow` header. Empty for a path no row matches.
///
/// It takes the host's `serves_games` for the same reason [`find`] does: a 405
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
    /// the client's polling hook has to know whether to open `…/stream` or to
    /// hold a request against `…/events?wait=25`, and it cannot tell from a
    /// successful call to anything else.
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
    pub multiplayer: bool,
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
/// session asks [`games::Games::handle`] afterwards and gets the same 409 it
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
pub fn admitted(
    games: &Games,
    id: &str,
    user: &identity::User,
) -> Result<(crate::table::TableDoc, client::Viewer), ApiError> {
    use crate::table::TableStatus as S;
    let table = games.table_for_anyone(id, user)?;
    if !table.seats_of(user).is_empty() {
        let seat = match table.status {
            S::Playing => {
                let handle = games.handle(id)?;
                let repl = handle.lock().expect("a game");
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
            table_status: None,
        }),
        S::Playing if table.can_watch(user) => Ok((table, client::Viewer::Spectator)),
        // **The lock gates watching and says so**, rather than reusing the
        // not-seated sentence: this caller CAN do something about it, and the
        // client draws the unlock face from exactly this answer.
        S::Playing => Err(ApiError {
            status: StatusCode::FORBIDDEN,
            message: "this table is locked — unlock it first".to_string(),
            table_status: None,
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
            table_status: None,
        }),
    }
}

/// Who holds `seat` at this table, for a refusal that names them.
///
/// A bot has no email and is named as what it is. The `None` arm is the same
/// unreachable-by-construction shape as [`admitted`]'s viewer seat:
/// `Games::read_table` refuses a document whose `seats` disagrees with its
/// `players`.
fn holder_of(table: &crate::table::TableDoc, seat: yokohama_core::PlayerId) -> &str {
    match table.seats.get(usize::from(seat.0)) {
        Some(crate::table::SeatDoc::Human { user, .. }) => user,
        _ => "the bot",
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
/// **The mover of event `len - 1` is `state_at(record, len - 1).current_player`
/// — replay-derived**, which is the CLI's own attribution for exactly this
/// question (`repl.rs`'s undo unwinds by replay, not by a boundary stack, so it
/// survives a save and reload). A `GameEvent` carries no seat of its own, and a
/// second attribution here would be a second thing to keep true.
///
/// What pops is `Session::undo`'s unit — one `Move` with the reveals bound to
/// it, or a trailing standalone reveal — and never more: admission requires
/// that event to be the caller's own, so a bot run can never be sitting in
/// front of it, even at a mixed table where one seat is a bot.
pub(crate) fn undo_allowed(
    repl: &yokohama_cli::repl::Repl,
    table: &crate::table::TableDoc,
    viewer: yokohama_core::PlayerId,
) -> Result<(), String> {
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
        return Err(status);
    }
    let record = repl.session.record();
    let len = record.events.len();
    // Not R-P6 speaking: the guard that keeps `len - 1` from underflowing, on
    // a game whose first prompt has not been answered yet. `Session::undo`'s
    // own words for the same condition, so the answer reads the same whichever
    // of the two paths reaches it.
    if len == 0 {
        return Err("nothing to undo".to_string());
    }
    let mover = yokohama_core::state_at(record, len - 1, repl.session.rules())
        .map_err(|e| e.message)?
        .current_player;
    if mover == viewer && repl.session.state().current_player == viewer {
        Ok(())
    } else {
        Err("undo is not offered once your turn has passed".to_string())
    }
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
    Ok(doc.view(user))
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
    /// only when it has no name yet — see [`Games::keep`](games::Games::keep).
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
/// The whole of the ladder is [`games::Games::unlock`]'s, held under the
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
    pub messages: Vec<chat::Message>,
    pub last: u64,
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
    admitted(games, id, user)?;
    let (messages, last) = games.chat_page(id, q.after.unwrap_or(0))?;
    Ok(ChatPage { messages, last })
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
pub fn chat_post(
    games: &Games,
    user: &User,
    id: &str,
    req: &ChatPost,
) -> Result<chat::Message, ApiError> {
    let (table, viewer) = admitted(games, id, user)?;
    let text = chat::check_text(&req.text, &games.chat.limits).map_err(ApiError::bad_request)?;
    let author = author_of(&table, viewer, user, req.name.as_deref())?;
    let message = games.chat_append(id, author, &text)?;
    Ok(message)
}

/// Who a poster IS, from the viewer the admission produced (R-6.4.2, R-6.4.15).
///
/// **A seat's name is read off the TABLE and never off the body**, which is the
/// whole of the rule: a name in a message is a claim about who is speaking, and
/// at a seat the table has already settled that claim. A seated caller who
/// sends one is refused rather than quietly overridden, because a client that
/// believed it was setting a name and was answered 201 would go on sending it.
fn author_of(
    table: &crate::table::TableDoc,
    viewer: client::Viewer,
    user: &identity::User,
    name: Option<&str>,
) -> Result<chat::Author, ApiError> {
    match viewer {
        client::Viewer::Seat(seat) => {
            if name.is_some() {
                return Err(ApiError::bad_request("a seated caller is named by their seat"));
            }
            let stored = match table.seats.get(usize::from(seat.0)) {
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
pub fn house_chat_read(games: &Games, _user: &User, q: &ChatQuery) -> Result<ChatPage, ApiError> {
    lobby_chat_on(games)?;
    let (messages, last) = games.house_page(q.after.unwrap_or(0))?;
    Ok(ChatPage { messages, last })
}

/// `POST /api/chat` — the house. Every author is a `Watcher`: the house has
/// no seats.
///
/// Which is not a simplification but the shape rule's own answer: the house
/// document is checked with `players: None`, and a `Seat` author there is
/// refused on write. There is nothing to be seated at.
pub fn house_chat_post(
    games: &Games,
    user: &User,
    req: &ChatPost,
) -> Result<chat::Message, ApiError> {
    lobby_chat_on(games)?;
    let text = chat::check_text(&req.text, &games.chat.limits).map_err(ApiError::bad_request)?;
    let name = req.name.as_deref().map(tables::check_name).transpose().map_err(ApiError::bad_request)?;
    let author = chat::Author::Watcher { user: user.as_str().to_string(), name };
    let message = games.house_append(author, &text)?;
    Ok(message)
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
    let repl = handle.lock().expect("a game");
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
    if !grammar::is_gameplay_line(&req.line) {
        return Err(ApiError::bad_request(grammar::refusal(&req.line)));
    }
    let (table, viewer) = admitted(games, id, user)?;
    // **A watcher writes nothing, and hears it before the handle is touched**
    // (M6.3, R-3.6). Above the undo rule and the acting-seat fence because both
    // of those are about a SEAT, and this caller has none: R-P6 would read the
    // trailing event's mover against a seat that does not exist.
    spectators_write_nothing(viewer)?;
    // The seat the rules below compare against. Total by the refusal one line
    // up, and written as a `match` rather than an `unwrap` so a third `Viewer`
    // variant is a compile error here instead of a panic in production.
    let seat = match viewer {
        client::Viewer::Seat(s) => s,
        client::Viewer::Spectator => return Err(spectators_write_nothing(viewer).unwrap_err()),
    };
    let handle = games.handle(id)?;
    let mut repl = handle.lock().expect("a game");

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
        undo_allowed(&repl, &table, seat).map_err(|message| ApiError {
            status: StatusCode::FORBIDDEN,
            message,
            table_status: None,
        })?;
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
        return Err(ApiError { status: StatusCode::FORBIDDEN, message, table_status: None });
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
    let messages: Vec<String> =
        body.lines().map(str::trim_end).filter(|l| !l.is_empty()).map(str::to_string).collect();
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
        let repl = handle.lock().expect("a game");
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
                    table_status: None,
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
/// because a branch nobody has ever seen is where defects live.
///
/// Same lock discipline as `suggest`, for the same reason: the reconstruction
/// runs the same `decide` path the pump ran.
pub fn why(games: &Games, user: &User, id: &str, q: &WhyQuery) -> Result<explain::Why, ApiError> {
    // Seated, and no viewer of its own: `why` already answers at the VIEWER's
    // visibility by masking the roads not taken for every caller
    // (`explain::HandVisibility::Masked`), so an unplayed `FulfillOrder` words
    // as "from hand" and names no card, at every seat. What this fence adds is
    // the seat one — an account of a search at a table the caller has no place
    // at is not theirs to read.
    let _admitted = admitted(games, id, user)?;
    let handle = games.handle(id)?;
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
            return Ok(explain::Why::locked(reason));
        }
        (repl.session.mode, repl.session.record().clone(), repl.bots().clone())
    };
    Ok(
        match yokohama_cli::why::account_parts(mode, &record, games.rules(), &bots, q.event) {
            Ok(account) => explain::Why::granted(&account, games.rules()),
            Err(refusal) => explain::Why::refused(&refusal),
        },
    )
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
                "no card index named {other:?} — the four are orders, techs, goals and sites"
            )))
        }
    };
    let filter = match q.filter.as_deref() {
        None => None,
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
    let repl = handle.lock().expect("a game");
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
    let repl = handle.lock().expect("a game");
    if !repl.session.is_over() {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            message: "the game is not over — there is no final score yet".to_string(),
            table_status: None,
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
    let repl = handle.lock().expect("a game");
    let total = repl.session.record().events.len();
    let first = q.tail.map_or(0, |n| total.saturating_sub(n));
    let described: Vec<(usize, String)> =
        (first..total).map(|i| (i, repl.session.event_description(i).to_string())).collect();
    Ok(yokohama_cli::screen_model::build_log(total, described))
}

/// How much of the game a poll gets: the SIGNAL, in five fields (room plan
/// R-P9, spec §6; the fifth is M6.4's).
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
}

/// The resume cursor, VALIDATED AND NOT ACTED ON in this cycle (spec §6).
///
/// **It is here now so the transport can change without the client changing.**
/// M6.3 chooses among a held request, server-sent events and a Durable Object
/// notifier, and every one of those answers "what has happened since N" — so
/// the polled form takes the same parameter and answers immediately whether or
/// not `events` has moved past it. A client written against this cycle keeps
/// working when the transport underneath it is replaced.
///
/// Typed `usize`, so a negative or non-numeric cursor is a 400 from serde
/// rather than a value this handler would have to range-check; and
/// `deny_unknown_fields` through the layer's `parse_query` makes a MISSPELLED
/// cursor a 400 too, rather than a silent `None` that polls from the beginning forever.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventsQuery {
    #[serde(default)]
    pub after: Option<usize>,
    /// Hold this request open for up to N seconds, or until the table bumps
    /// (R-plan-5, spec §3.3).
    ///
    /// **The held-request transport, shipped whether or not it is the one the
    /// client reads.** R-3.1 decides between server-sent events and this on a
    /// measurement through the real tunnel; carrying both costs one cell here
    /// and makes the fallback a one-line change in the client's reader rather
    /// than a second plan. It subscribes to the same channel `…/stream` does
    /// and answers the same object, so a client that swaps transports swaps
    /// nothing else.
    ///
    /// Absent, the route answers at once, exactly as it always has.
    #[serde(default)]
    pub wait: Option<u64>,
}

/// The `Events` object for a live session, from the caller's seat.
///
/// **One function, two transports.** `events` and `…/stream` must answer the
/// same object or the client's compare is against two different things; the
/// stream's own cell asserts the first frame equals an `events` read BY VALUE,
/// which is the property this function makes true by construction rather than
/// by agreement between two bodies.
pub fn events_of(repl: &yokohama_cli::repl::Repl, viewer: client::Viewer, chat: u64) -> Events {
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
        chat,
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
    let chat = games.chat_last(id);
    if table.status == crate::table::TableStatus::Open {
        return Ok(Events { events: 0, to_move: None, status: "open", your_turn: false, chat });
    }
    let (table, viewer) = admitted(games, id, user)?;
    match table.status {
        // Unreachable behind the return above, and written rather than
        // collapsed into the fall-through: a status this function answers by
        // name is a status a reader can find.
        crate::table::TableStatus::Open => {
            Ok(Events { events: 0, to_move: None, status: "open", your_turn: false, chat })
        }
        crate::table::TableStatus::Dealing => {
            Ok(Events { events: 0, to_move: None, status: "dealing", your_turn: false, chat })
        }
        crate::table::TableStatus::Playing => {
            let handle = games.handle(id)?;
            let repl = handle.lock().expect("a game");
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
/// **The cap is checked FIRST**: a run longer than the cap is a malformed ask
/// whatever `total` is, so a fresh game asked for `to=65` reads the cap's
/// sentence and not the range's.
///
/// Admitted as `at_k` is, serves a locked session as `at_k` does, and holds the
/// lock across the body for the same reason `at_k` holds it.
pub fn run(games: &Games, user: &User, id: &str, q: &RunQuery) -> Result<RunFrames, ApiError> {
    let (_table, viewer) = admitted(games, id, user)?;
    let handle = games.handle(id)?;
    let repl = handle.lock().expect("a game");
    let total = repl.session.record().events.len();
    if q.to > q.from && q.to - q.from > RUN_CAP {
        return Err(ApiError::bad_request(format!(
            "a run is at most {RUN_CAP} positions — ask for from={}&to={} and page",
            q.from,
            q.from + RUN_CAP
        )));
    }
    if q.from > q.to || q.to > total + 1 {
        return Err(ApiError::not_found(format!(
            "this game has {total} events, so its last position is {total} — there is no run from {} to {}",
            q.from, q.to
        )));
    }
    let frames = client::HistoricalFrame::run(&repl, q.from, q.to, viewer).map_err(|e| {
        ApiError::server_error(format!(
            "replaying this game failed at event {}: {}",
            e.event_index, e.message
        ))
    })?;
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
pub fn at_k(
    games: &Games,
    user: &User,
    id: &str, k: usize,
) -> Result<client::HistoricalFrame, ApiError> {
    let (_table, viewer) = admitted(games, id, user)?;
    let handle = games.handle(id)?;
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
    //
    // **And a SPECTATOR's sweep harvests nothing at all** (M6.3, R-3.7): every
    // hand is hidden at every `k` and `seat` is null.
    client::HistoricalFrame::at(&repl, k, viewer).map_err(|e| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("replaying this game to {k} failed at event {}: {}", e.event_index, e.message),
        table_status: None,
    })
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
    let repl = handle.lock().expect("a game");
    Ok(build(repl.session.state(), repl.session.rules()))
}


/// `GET /api/games/{id}/record` (M6.5a, R-6.5a.12; umbrella R-6.5.10): a SEAT
/// HOLDER's finished game as the EXACT bytes the store holds for `Doc::Record`
/// — never a re-serialization, so the terminal digest the store stamped is the
/// digest the file carries. A spectator is refused 403 (the record is the
/// players'); a running game is a 409, because the record carries the setup
/// seed — hidden-set member #1 — until the last move. Lockdown does not fence
/// it: this is projection, not computation (the M3 lockdown rule).
pub fn record(games: &Games, user: &User, id: &str) -> Result<String, ApiError> {
    let (_table, viewer) = admitted(games, id, user)?;
    if viewer == client::Viewer::Spectator {
        return Err(ApiError {
            status: StatusCode::FORBIDDEN,
            message: "the record is the players' — watch, download nothing".to_string(),
            table_status: None,
        });
    }
    let handle = games.handle(id)?;
    let over = handle.lock().expect("a game").session.is_over();
    if !over {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            message: "the record is available once the game is over".to_string(),
            table_status: None,
        });
    }
    let (json, _) = games
        .store()
        .read(crate::store::Doc::Record, id)
        .map_err(ApiError::server_error)?
        .ok_or_else(|| ApiError::not_found(format!("no game named {id:?}")))?;
    Ok(json)
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
/// **The four checks, in spec §2.4's order** (five REFUSALS, because a rev that
/// disagrees and a rev that is absent are different sentences): a
/// `semantics_rev` present and equal to this build's; a terminal digest
/// present; `replay_strict` clean; and the table document parsing, passing
/// [`tables::check_shape`] and AGREEING with the record. A fifth guard follows
/// them — the document's id is `is_safe_id`'s, because it becomes a path
/// component. Then **the leaf IS the `Why`** — `Over` iff the
/// replayed state is over, else `Abandoned` — and one put under
/// `browser/<`[`caller_segment`](crate::archive::caller_segment)`>/` with the
/// table RE-STRIPPED server-side from the parsed document, as `delete` does, so
/// a crafted body carrying a lock or a watcher roster never lands verbatim. The
/// record string is put exactly as sent, because its bytes are the digest's.
///
/// 404 where this host keeps no archive — which under `NoArchive`, the
/// browser's own host, is the answer by construction.
pub fn archive_post(games: &Games, user: &User, req: &ArchivePost) -> Result<ArchiveReply, ApiError> {
    use crate::archive::{caller_segment, is_on, paths, strip_for_archive, PutError, PutOutcome, Why};
    if !is_on(&games.archive) {
        return Err(ApiError::not_found("this server keeps no archive"));
    }
    let record: yokohama_core::GameRecord = serde_json::from_str(&req.record)
        .map_err(|e| ApiError::bad_request(format!("the record does not parse: {e}")))?;
    match record.semantics_rev {
        Some(rev) if rev == yokohama_core::SEMANTICS_REV => {}
        Some(rev) => {
            return Err(ApiError::bad_request(format!(
                "this record is at semantics rev {rev}; this server archives rev {}",
                yokohama_core::SEMANTICS_REV
            )))
        }
        // `replay_strict`'s own sentence for unknown provenance, carried
        // verbatim rather than reworded: the engine is the authority on what a
        // record with no rev is, and it says so in one place.
        None => {
            let e = yokohama_core::replay_strict(&record, games.rules())
                .expect_err("a record with no semantics_rev is refused by strict replay");
            return Err(ApiError::bad_request(e.message));
        }
    }
    if record.terminal_digest.is_none() {
        return Err(ApiError::bad_request("the record carries no terminal digest"));
    }
    let state = yokohama_core::replay_strict(&record, games.rules()).map_err(|e| {
        ApiError::bad_request(format!(
            "the record does not replay: at event {}, {}",
            e.event_index, e.message
        ))
    })?;
    let doc: crate::table::TableDoc = serde_json::from_str(&req.table)
        .map_err(|e| ApiError::bad_request(format!("the table document does not parse: {e}")))?;
    tables::check_shape(&doc)
        .map_err(|why| ApiError::bad_request(format!("the table document is malformed: {why}")))?;
    doc.agrees_with(&record).map_err(|why| {
        ApiError::bad_request(format!("the table document does not agree with the record: {why}"))
    })?;
    if !crate::store::is_safe_id(&doc.id) {
        return Err(ApiError::bad_request("the table's id is not a safe name"));
    }
    let why = if yokohama_core::is_game_over(&state) { Why::Over } else { Why::Abandoned };
    let prefix = format!("browser/{}/", caller_segment(user));
    let (path, _) = paths(yokohama_core::SEMANTICS_REV, &prefix, why, &doc.id);
    match games.archive.put(&prefix, why, &doc.id, &req.record, &strip_for_archive(&doc)) {
        Ok(outcome) => Ok(ArchiveReply {
            leaf: why.dir(),
            path,
            identical: outcome == PutOutcome::Identical,
        }),
        // The path is in the archive's own error for the operator's log; the
        // client is told what it can do about it and not where the file is.
        Err(PutError::Conflict(_)) => Err(ApiError {
            status: StatusCode::CONFLICT,
            message: "a different record is archived under this id".to_string(),
            table_status: None,
        }),
        Err(PutError::Failed(e)) => {
            Err(ApiError::server_error(format!("the archive could not be written: {e}")))
        }
    }
}

/// The layer's front door: match, answer, and shape the response. A known path
/// under another method is 405 with an empty body (axum's own shape); a path
/// that matches nothing is the JSON 404 the server's `fallback` writes.
pub fn dispatch(games: &Games, host: &Host, identity: Identity<'_>, req: &ApiRequest<'_>) -> ApiResponse {
    let (pattern, bound) = match find(req.method, req.path, host.serves_games) {
        Matched::Route(p, b) => (p, b),
        Matched::WrongMethod => return ApiResponse { status: 405, body: String::new() },
        Matched::NoRoute => {
            return error_response(ApiError::not_found(format!("no such endpoint: {}", req.path)))
        }
    };
    let out = match answer(games, host, identity, req, pattern, &bound) {
        Ok(r) => r,
        Err(e) => error_response(e),
    };
    // HEAD leaves with an empty body on BOTH arms — a refused HEAD too — so a
    // non-HTTP host never emits a body a HEAD did not ask for.
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
                "/api/games" if req.method == "GET" => {
                    list_games(games, u, &parse_query(req.query)?).map(|v| ok_json(200, &v))
                }
                "/api/games" => create_game(games, u, &parse_json(req.content_type, req.body)?)
                    .map(|v| ok_json(201, &v)),
                "/api/games/{id}" if req.method == "GET" => {
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
                "/api/games/{id}/chat" if req.method == "GET" => {
                    chat_read(games, u, b[0], &parse_query(req.query)?).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}/chat" => {
                    chat_post(games, u, b[0], &parse_json(req.content_type, req.body)?).map(|v| ok_json(201, &v))
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
                "/api/chat" if req.method == "GET" => {
                    house_chat_read(games, u, &parse_query(req.query)?).map(|v| ok_json(200, &v))
                }
                "/api/chat" => house_chat_post(games, u, &parse_json(req.content_type, req.body)?)
                    .map(|v| ok_json(201, &v)),
                // 201 for a pair that landed, 200 for a retry that found its own
                // bytes already there — the client acts on either (spec §2.4).
                "/api/archive" => archive_post(games, u, &parse_json(req.content_type, req.body)?)
                    .map(|v| ok_json(if v.identical { 200 } else { 201 }, &v)),
                "/api/games/{id}/at" => {
                    run(games, u, b[0], &parse_query(req.query)?).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}/at/{k}" => {
                    let k = parse_path_usize(1, b[1])?;
                    at_k(games, u, b[0], k).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}/suggest" => suggest(games, u, b[0]).map(|v| ok_json(200, &v)),
                "/api/games/{id}/why" => {
                    why(games, u, b[0], &parse_query(req.query)?).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}/index/{kind}" => {
                    card_index(games, u, b[0], b[1], &parse_query(req.query)?).map(|v| ok_json(200, &v))
                }
                "/api/games/{id}/result" => result(games, u, b[0]).map(|v| ok_json(200, &v)),
                // The stored bytes, not a re-serialization: the body is the string.
                "/api/games/{id}/record" => {
                    record(games, u, b[0]).map(|json| ApiResponse { status: 200, body: json })
                }
                _ => Err(ApiError::not_found(format!("no such endpoint: {}", req.path))),
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
            "Invalid URL: Cannot parse value at index {index} with value `{raw}` to a `u64`"
        ))
    })
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
            ("POST", "/api/games/{id}/input"),
            ("GET", "/api/games/{id}/ports"),
            ("GET", "/api/games/{id}/labs"),
            ("GET", "/api/games/{id}/areas"),
            ("GET", "/api/games/{id}/log"),
            ("GET", "/api/chat"),
            ("POST", "/api/chat"),
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
    }
}
