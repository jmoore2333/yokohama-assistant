//! **The wire contract, frozen as files the browser client type-checks against.**
//!
//! `web/src/types.ts` is hand-written, and until this existed nothing checked
//! it against the wire: `npx tsc -b --noEmit` proves the client agrees with
//! ITSELF, and the whole-game gate is entirely server-side. A field renamed
//! here and not there compiles on both sides and fails in a browser.
//!
//! So this test emits every payload shape the client reads, as committed JSON
//! under `web/src/wire/`, and refuses when what the server emits is not what is
//! committed. The client's `src/wire/contract.ts` then type-checks those same
//! files against `types.ts` under the EXISTING type gate — so a server-side
//! rename fails here, and a client-side one fails there, and neither can be
//! merged believing the two agree.
//!
//! **Two properties make the corpus trustworthy rather than decorative:**
//!
//! 1. **Every gameplay fixture is produced by the REAL router**, through
//!    `tower::ServiceExt::oneshot`, from a deterministic bot-free 2P game. Not
//!    hand-written: a hand-written sample is a third statement of the shape and
//!    drifts from both the other two.
//! 2. **The prompt corpus is EXHAUSTIVE over `PromptState`**, checked by a
//!    second test with a named exemption list. A new prompt variant that no
//!    fixture covers fails that test rather than shipping a browser branch
//!    nobody rendered.
//!
//! **Why the game has no bot.** `Games::create` draws the bot seed from OS
//! entropy (`games.rs`'s `config_of`), deliberately — so a game with a bot in
//! it does not reproduce, and a golden built from one would churn on every run.
//! A 2P table of two human seats is what the CLI calls one operator driving the
//! table, is fully determined by the setup seed, and exercises every prompt the
//! client can be shown. The ONE payload that genuinely needs a bot —
//! `Watching`, which only exists because the acting seat is a bot — is built in
//! process with a fixed bot seed instead.
//!
//! Regenerate with:
//!
//! ```text
//! UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures
//! ```
//!
//! and then LOOK at the diff. These files are a contract; a regeneration that
//! is not read is a contract signed unread.

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use yokohama_core::Rules;
use yokohama_web::{app, AppState};

/// The setup seed every gameplay fixture is dealt from. Fixed, and small on
/// purpose: this is a DIAGNOSTIC seed in the engine's own sense — a
/// reproduction, never an evaluation — so the full-`u64` rule that governs a
/// real game's seed does not apply, and a short literal is readable in a diff.
const SETUP_SEED: u64 = 20_260_831;

/// The bot seed for the one fixture that needs a bot. Fixed for the same
/// reason, and never drawn from entropy here.
const BOT_SEED: u64 = 0x4649_5854_5552_4553; // "FIXTURES"

/// The walk's own stream. Independent of the setup seed so a re-deal does not
/// silently re-order the walk as well.
const WALK_SEED: u64 = 0x5741_4C4B_5041_5448; // "WALKPATH"

/// Who the walk plays as. Every `/api/games` route needs an identity (M6.1),
/// and this one is a constant like the seeds so the corpus stays a function of
/// what is written in this file.
const WALKER: &str = "red@wire.test";

/// A tiny deterministic PRNG, so the corpus is a function of three constants
/// and nothing else. SplitMix64, the shape the engine uses for its own stream
/// derivation — no new dependency for a test that only picks an index.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

struct Api {
    router: axum::Router,
    /// The state BEHIND the router, kept reachable.
    ///
    /// A later task of this cycle reads the store the walk wrote to, which is
    /// only reachable from here — `app()` moves the `Arc` in and nothing gives
    /// it back. It is used in the walk below rather than carried as dead weight,
    /// and the assertion it carries is a real one: the record is written at
    /// FINALIZE, not at create.
    state: Arc<AppState>,
}

impl Api {
    fn new() -> Self {
        let state = Arc::new(AppState::in_memory());
        Self { router: app(Arc::clone(&state)), state }
    }

    /// A call as [`WALKER`], which is every call the walk makes that is not a
    /// MOVE — those go as the acting seat (see [`acting`]).
    async fn call(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        self.call_as(Some(WALKER), method, uri, body).await
    }

    /// The same call with the identity header omitted, for the one fixture that
    /// is the answer to a caller who has none.
    async fn get_anonymous(&self, uri: &str) -> Value {
        let (status, v) = self.call_as(None, "GET", uri, None).await;
        assert_eq!(status, StatusCode::OK, "GET {uri} without an identity: {v}");
        v
    }

    async fn call_as(
        &self,
        who: Option<&str>,
        method: &str,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(uri);
        if let Some(who) = who {
            req = req.header("x-yokohama-claimed-user", who);
        }
        let req = match body {
            Some(v) => req
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&v).expect("a body")))
                .expect("a request"),
            None => req.body(Body::empty()).expect("a request"),
        };
        let res = self.router.clone().oneshot(req).await.expect("the router answers");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                panic!("{uri} must answer JSON: {e}: {:?}", String::from_utf8_lossy(&bytes))
            })
        };
        (status, value)
    }

    async fn get(&self, uri: &str) -> Value {
        self.get_as(WALKER, uri).await
    }

    /// A GET from a NAMED seat's holder. Every payload is built for the caller
    /// (M6.1 §5.1), so a fixture of the acting seat's frame has to be FETCHED by
    /// the acting seat's holder — see [`as_actor`].
    async fn get_as(&self, who: &str, uri: &str) -> Value {
        let (status, v) = self.call_as(Some(who), "GET", uri, None).await;
        assert_eq!(status, StatusCode::OK, "GET {uri} as {who}: {v}");
        v
    }

    /// A move from a NAMED seat's holder.
    ///
    /// Every move in the walk goes as the ACTING seat (M6.1: `input` becomes the
    /// acting seat's later in this cycle, and a corpus recorded from the wrong
    /// caller would have to be re-cut then). `acting` reads the payload's own
    /// `to_move` rather than tracking it here.
    async fn send_as(&self, who: &str, line: &str) -> Value {
        let (status, v) = self
            .call_as(Some(who), "POST", "/api/games/wire/input", Some(json!({ "line": line })))
            .await;
        assert_eq!(status, StatusCode::OK, "input {line:?} as {who} was refused: {v}");
        v
    }
}

/// What `created_at` reads as in the committed corpus.
///
/// **The one value in this file that is not the server's own**, and it is here
/// because a wall clock is not a function of three constants: a table document
/// stamps `now_unix()` (R-plan-1), so every regeneration would rewrite five
/// fixtures with a new number and the golden would fail on the next run without
/// anything having changed. Freezing it keeps the corpus a function of the seeds
/// and the identities, which is the property that makes a diff here mean
/// something.
///
/// **What is therefore NOT pinned**: the VALUE. What still is: the field's
/// presence and its type, on both sides — `KEYS.table` names it, `Widen<TableView>`
/// requires a number, and [`freeze_clock`] REFUSES a value that is not a
/// plausible unix timestamp, so a `created_at` that became a string, a null or a
/// zero fails here rather than being normalized away.
const FROZEN_CREATED_AT: u64 = 1_789_000_000;

/// Replace every `created_at` with [`FROZEN_CREATED_AT`], wherever it is nested.
///
/// A walk rather than a list of paths: the field rides on `TableView`, which
/// appears at the root of two fixtures, inside `deal.table`, and once per row of
/// `games.games` — and a list of those would be a fourth statement of the wire
/// shape to keep true.
fn freeze_clock(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, v) in map.iter_mut() {
                if key == "created_at" {
                    let seen = v.as_u64().unwrap_or_else(|| {
                        panic!("created_at is not a unix timestamp: {v}")
                    });
                    assert!(
                        seen > 1_700_000_000,
                        "created_at reads {seen}, which is not a clock — freezing it would hide \
                         a field that has stopped being stamped",
                    );
                    *v = json!(FROZEN_CREATED_AT);
                } else {
                    freeze_clock(v);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(freeze_clock),
        _ => {}
    }
}

/// Which of the walk's two people holds the seat that is to move.
fn acting(state: &Value) -> &'static str {
    if state["to_move"].as_u64() == Some(0) {
        WALKER
    } else {
        OTHER
    }
}

/// The payload the ACTING seat's holder sees, re-read when the turn has passed.
///
/// A reply is built for its CALLER (M6.1 §5.1), so the move that ends a turn
/// answers its own mover with a `watching` prompt and that seat's frame — true,
/// and not what the walk must answer next, nor the payload the client renders
/// while it is somebody's turn. So whenever the reply's `seat` is no longer the
/// seat to move, the game is re-read as the new mover.
///
/// **The condition is what keeps the corpus honest about `messages`.** A reply
/// that did NOT cross a turn boundary is returned untouched, so the fixtures
/// captured from one still carry the delta lines the client renders; only the
/// turn-crossing ones become a fresh read, which has none.
async fn as_actor(api: &Api, state: Value) -> Value {
    if state["over"] == Value::Bool(true) || state["seat"] == state["to_move"] {
        return state;
    }
    api.get_as(acting(&state), "/api/games/wire").await
}

/// The walk's second person. Seat 1 is a HUMAN in this corpus (see the module
/// doc on why there is no bot), and since M6.1 a seat belongs to somebody.
const OTHER: &str = "blue@wire.test";

/// Someone with no seat at this table, for the one fixture that is a 403.
const STRANGER: &str = "green@wire.test";

fn new_game_body() -> Value {
    json!({
        "players": 2,
        "colors": ["Red", "Blue"],
        "start_seat": 0,
        "seed": SETUP_SEED,
        // NO bot seats. See the module doc: a bot's seed is OS entropy and a
        // golden built over one would churn on every run. Two PEOPLE instead,
        // which is also what makes the per-seat deal fixtures below real.
        "seats": [{ "user": WALKER }, { "user": OTHER }],
        "name": "wire",
    })
}

/// Where the committed corpus lives — inside the client's own `src`, so the
/// existing `tsc -b --noEmit` type gate sees it without a second tsconfig.
fn wire_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/src/wire")
}

/// The art directory, resolved from THIS crate rather than from the process's
/// working directory: `art::art_dir()`'s default is relative to the repo root,
/// and a cargo test runs in the crate root.
fn art_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/art")
}

/// One deterministic 2P game, walked far enough to see every prompt the client
/// can be shown, capturing the FIRST payload of each kind.
///
/// The walk's shape is the whole-game gate's — a seeded choice among whatever
/// the prompt offers — with two differences, and the first of them is not a
/// preference:
///
/// 1. **It PLACES.** The gate answers every placement picker with `confirm` on
///    an empty selection, which is legal and keeps the gate from having to know
///    which tiles accept an assistant. The gate can afford that because seat 1
///    is a bot that drives the game to an end regardless. This walk has no bot,
///    and placing nothing means no assistant ever reaches the board, no area
///    action ever fires, and nothing ever triggers the end of the game:
///    MEASURED at turn 3,644 with ten cards still in the deck, every seat still
///    holding eight assistants and nobody above zero VP. So the placement arms
///    below ASK the server which areas it will take — the engine owns that fact
///    and this test must not restate it — and place in the first one it accepts.
/// 2. **It captures INTERMEDIATE states.** A placement with something selected
///    carries a live fee, and that is where a real defect lived: a player
///    pressed Confirm and paid a yen no surface had stated.
async fn walk() -> Vec<(String, Value)> {
    let api = Api::new();

    let (status, preview) =
        api.call("POST", "/api/games/preview", Some(new_game_body())).await;
    assert_eq!(status, StatusCode::OK, "{preview}");

    // A SECOND preview, with a bot at seat 1, so the corpus carries the shape
    // the set table's bot half has: `is_bot` true, an EMPTY `cards` (R-C1: a
    // seat only ever sees its own deal), and a hand that already holds the card
    // the bot kept. No `create` beside it — a preview pumps nothing and draws
    // no bot seed into anything it returns, so unlike the walk's game it is
    // deterministic with a bot on the table.
    let mut with_bot = new_game_body();
    with_bot["seats"] = json!([{ "user": WALKER }, { "bot": true }]);
    with_bot["name"] = json!("wire-bot");
    let (status, preview_bot) = api.call("POST", "/api/games/preview", Some(with_bot)).await;
    assert_eq!(status, StatusCode::OK, "{preview_bot}");

    // **The opening sequence, and every payload it produces** (M6.1 §4). A
    // create opens a DEALING table; each seat fetches its own two cards and
    // keeps; the last keep starts the game. Six shapes the client draws exist
    // only inside that sequence, so they are captured as they go past rather
    // than reconstructed afterwards.
    let (status, table_dealing) = api.call("POST", "/api/games", Some(new_game_body())).await;
    assert_eq!(status, StatusCode::CREATED, "{table_dealing}");

    let deal = api.get("/api/games/wire/deal").await;
    // Every session route while the table is dealing: a 409 carrying a STATUS
    // the client branches on, not a 404 it cannot act on.
    let (status, dealing_409) = api.call("GET", "/api/games/wire", None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{dealing_409}");
    // **No record exists yet**, read off the store behind the walk rather than
    // inferred from the 409 above. The two are different claims: the 409 says
    // this API declines to serve a session, and this says there is none.
    assert!(
        api.state
            .games
            .store()
            .read(yokohama_web::store::Doc::Record, "wire")
            .expect("a readable store")
            .is_none(),
        "a dealing table has no record",
    );

    let (status, kept) = api
        .call("POST", "/api/games/wire/keep", Some(json!({ "seat": 0, "card": 1 })))
        .await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    // The same route once this caller's seats have all chosen: `seat` and
    // `cards` are null together, which is the "waiting for the others" screen.
    let deal_waiting = api.get("/api/games/wire/deal").await;
    // The lobby while a table is still dealing.
    let games_list = api.get("/api/games").await;
    // A caller with no seat here. A 403, and the client draws it as one.
    let (status, not_seated_403) =
        api.call_as(Some(STRANGER), "GET", "/api/games/wire/deal", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{not_seated_403}");

    let (status, kept) = api
        .call_as(Some(OTHER), "POST", "/api/games/wire/keep", Some(json!({ "seat": 1, "card": 1 })))
        .await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    // The table AFTER finalize, read by the seat that is not the creator — so
    // the corpus carries `my_seats` at something other than `[0]`.
    let (status, table) = api.call_as(Some(OTHER), "GET", "/api/games/wire/table", None).await;
    assert_eq!(status, StatusCode::OK, "{table}");
    // ...and now there is one. The other half of the pair above: the record is
    // written by the LAST keep and by nothing before it.
    assert!(
        api.state
            .games
            .store()
            .read(yokohama_web::store::Doc::Record, "wire")
            .expect("a readable store")
            .is_some(),
        "the last keep wrote the record",
    );

    let created = api.get("/api/games/wire").await;

    let mut out: Vec<(String, Value)> = vec![
        ("preview".to_string(), preview),
        ("preview_bot".to_string(), preview_bot),
        ("table_dealing".to_string(), table_dealing),
        ("table".to_string(), table),
        ("deal".to_string(), deal),
        ("deal_waiting".to_string(), deal_waiting),
        ("dealing_409".to_string(), dealing_409),
        ("games".to_string(), games_list),
        ("not_seated_403".to_string(), not_seated_403),
        ("health".to_string(), api.get("/api/health").await),
        // **Both sides of `/api/me`, because the client branches on both.**
        // Signed in, and signed out — the second taken with the header omitted
        // rather than constructed, since "what a caller with no identity is
        // handed" is exactly the payload a route test cannot fake and the one a
        // sign-in screen renders from.
        ("me".to_string(), api.get("/api/me").await),
        ("me_signed_out".to_string(), api.get_anonymous("/api/me").await),
        ("labels".to_string(), api.get("/api/labels").await),
        ("ports".to_string(), api.get("/api/games/wire/ports").await),
        ("labs".to_string(), api.get("/api/games/wire/labs").await),
        ("areas".to_string(), api.get("/api/games/wire/areas").await),
    ];
    let mut seen: Vec<String> = Vec::new();
    let capture = |out: &mut Vec<(String, Value)>,
                       seen: &mut Vec<String>,
                       name: &str,
                       v: &Value| {
        if !seen.iter().any(|s| s == name) {
            seen.push(name.to_string());
            out.push((name.to_string(), v.clone()));
        }
    };

    let mut state = as_actor(&api, created).await;
    let mut rng = Rng(WALK_SEED);
    let mut steps = 0usize;
    // The area the server last accepted an assistant in, tried first next time.
    // Placement legality is the engine's, so this is a CACHE over the answers
    // it already gave, never a rule: a miss falls back to asking about every
    // area in turn, and a hit costs one request instead of up to twenty-four.
    let mut placed_before: Option<u64> = None;
    // A prompt that does not move is a walk that does not terminate. Three
    // identical prompts in a row means the arm below is answering with
    // something the game will not take, and the answer is to back out of the
    // picker rather than to keep pushing.
    let mut stuck = 0usize;
    let mut last_prompt = Value::Null;

    while state["over"] != Value::Bool(true) {
        steps += 1;
        assert!(steps < 40_000, "the walk must terminate; last state: {state}");
        if state["prompt"] == last_prompt {
            stuck += 1;
        } else {
            stuck = 0;
            last_prompt = state["prompt"].clone();
        }
        let kind = state["prompt"]["kind"].as_str().expect("a prompt kind").to_string();
        // Every move this iteration makes goes as the seat that is to move.
        let mover = acting(&state);
        capture(&mut out, &mut seen, &kind, &state);

        // The SHAPES within a kind that the client branches on separately.
        //
        // A corpus of one payload per prompt KIND is a corpus in which every
        // main prompt happens to be the one-row "end the additional-action
        // phase", every retreat still has areas to answer, and no row carries a
        // price — so three client branches have no payload behind them. These
        // are the same first-sighting rule applied to the distinctions the
        // client actually renders differently.
        let rows = state["prompt"]["rows"].as_array().cloned().unwrap_or_default();
        if kind == "options" && rows.len() >= 3 {
            capture(&mut out, &mut seen, "options_many", &state);
            // `suggest` from the REAL ROUTER, at a position with something to
            // choose between. Captured under the same condition as
            // `options_many` and for the same reason: taken at the opening it
            // answers "End additional-action phase" with a one-row ranking, and
            // the runner-up table, the margin column and the multi-step
            // continuation all go to the browser with no payload behind them.
            //
            // This game has no bot, so the search is keyed by the record
            // identity, the seat and the move count — all three fixed by
            // `SETUP_SEED` and the walk — and the answer is the same answer
            // every run.
            if !seen.iter().any(|s| s == "suggest") {
                // **As the MOVER, because `suggest` answers the acting seat's
                // holder only** (R-1.8): the advisor ranks the moves of the
                // seat that is to move, and nobody else may ask for them.
                let s = api.get_as(mover, "/api/games/wire/suggest").await;
                capture(&mut out, &mut seen, "suggest", &s);
            }
        }
        if kind == "options" && rows.iter().any(|r| r["fee"] != Value::Null) {
            capture(&mut out, &mut seen, "options_priced", &state);
        }
        // A fee that actually COSTS something, wherever it turns up.
        //
        // `options_priced` above catches a fee OBJECT, and every President move
        // carries one whether it is free or not — so on its own the client's
        // `pay ¥N (n to Blue)` branch has no payload behind it and only `free`
        // is ever rendered. Measured over a whole random 2P game: **the maximum
        // fee anywhere in the corpus was zero.**
        if state["prompt"]["fee"]["total"].as_u64().unwrap_or(0) > 0
            || rows.iter().any(|r| r["fee"]["total"].as_u64().unwrap_or(0) > 0)
        {
            capture(&mut out, &mut seen, "fee_nonzero", &state);
        }
        // One payload per QUEUED CHOICE the walk meets. `Deciding::Choice`
        // outranks the phase and the client words each head separately, so a
        // corpus with one head in it leaves the other six rendered by the raw
        // key fallback. These are captured OPPORTUNISTICALLY and not owed: the
        // walk is deterministic, so the file set is still a function of the
        // seeds, but which heads a random game meets is not something to assert
        // — the ones it does not reach are named in the M2.5 record instead.
        if let Some(head) = state["frame"]["decision"]["deciding"]["Choice"]["head"].as_str() {
            capture(&mut out, &mut seen, &format!("choice_{head}"), &state);
        } else if let Some(obj) =
            state["frame"]["decision"]["deciding"]["Choice"]["head"].as_object()
        {
            if let Some(head) = obj.keys().next() {
                capture(&mut out, &mut seen, &format!("choice_{head}"), &state);
            }
        }
        if kind == "retreat" && state["prompt"]["remaining"] == 0 {
            capture(&mut out, &mut seen, "retreat_answered", &state);
        }

        // The one payload a walk that only ever answers the game never
        // produces. It applies no move, and the walk continues from the state
        // it already holds.
        //
        // **`hidden_hand` used to be captured here too, and it is retired with
        // this milestone** rather than moved. It was the `privacy` toggle — one
        // operator's "hide every hand so I can pass the keyboard over" — which
        // `client::facts_for` no longer consults: a seat's payload masks every
        // hand but its own, always, so toggling privacy over the route now
        // produces a payload byte-identical to the one beside it. The property
        // the fixture carried — a masked hand is a count and no identities —
        // is still on the wire in every `ClientState` in the corpus, as
        // `hand_hidden` on the seats that are not the viewer's, and is asserted
        // corpus-wide in the client's `contract.test.ts`.
        if kind == "options" && !seen.iter().any(|s| s == "refused") {
            seen.push("refused".to_string());

            // A REFUSAL, produced by the real handler, at a live prompt.
            //
            // `messages` is the only field on the payload an untroubled walk
            // never populates, and it is the field the client renders as a
            // warning. The line is a number the grammar lets through (three
            // digits — four is refused at the FENCE, which is a different
            // answer) and the REPL then declines as out of range.
            out.push(("refused".to_string(), api.send_as(mover, "999").await));
            continue;
        }

        let line: String = match kind.as_str() {
            _ if stuck >= 3 => "cancel".to_string(),
            "options" | "pick" => {
                let rows = state["prompt"]["rows"].as_array().expect("rows");
                assert!(!rows.is_empty(), "an answerable prompt must offer something: {state}");
                rows[rng.below(rows.len())]["number"].as_u64().expect("a number").to_string()
            }
            "place_a" => {
                let already = state["prompt"]["selected"].as_array().map_or(0, Vec::len);
                let want = state["prompt"]["max_areas"].as_u64().unwrap_or(0).min(2) as usize;
                if already >= want {
                    "confirm".to_string()
                } else {
                    // Ask the server which area it will take. `place_a` does
                    // not carry the legal set (Option B does), so this probes.
                    //
                    // **Probing in a ROTATED order, not `0..24`.** The first
                    // draft tried the cached hit and then ascending ids, so it
                    // placed in area 0 essentially every turn — and a walk that
                    // always places in the same tile never places where another
                    // seat's President is standing, which is the only way a
                    // placement costs anything. The corpus had no fee above zero
                    // in it, so the client's `pay ¥N (n to Blue)` branch had no
                    // payload behind it at all.
                    // **Tiles holding a PRESIDENT are tried first, and that is
                    // the only steering in this walk.** A placement costs 1 yen
                    // per assistant to each president owner in the area, so a
                    // walk that never places under a President never produces a
                    // fee above zero — measured over a whole random 2P game, the
                    // largest fee anywhere in the corpus was 0, leaving the
                    // client's priced branch rendered by nothing. This reads the
                    // PAYLOAD (`tiles[*].presidents`) rather than restating a
                    // rule, and the server still decides what it will accept.
                    let occupied: Vec<u64> = state["frame"]["board"]["tiles"]
                        .as_array()
                        .map(|tiles| {
                            tiles
                                .iter()
                                .filter(|t| {
                                    !t["presidents"].as_array().map(Vec::is_empty).unwrap_or(true)
                                })
                                .filter_map(|t| t["area"].as_u64())
                                .collect()
                        })
                        .unwrap_or_default();
                    let offset = rng.below(24) as u64;
                    let order: Vec<u64> = occupied
                        .into_iter()
                        .chain(placed_before)
                        .chain((0..24).map(|i| (i + offset) % 24))
                        .collect();
                    let mut landed = None;
                    for area in order {
                        let (status, probe) = api
                            .call_as(
                                Some(mover),
                                "POST",
                                "/api/games/wire/input",
                                Some(json!({ "line": format!("A{area:02}") })),
                            )
                            .await;
                        let grew = probe["prompt"]["kind"] == "place_a"
                            && probe["prompt"]["selected"].as_array().map_or(0, Vec::len) > already;
                        if status == StatusCode::OK && grew {
                            placed_before = Some(area);
                            capture(&mut out, &mut seen, "place_a_selected", &probe);
                            landed = Some(probe);
                            break;
                        }
                    }
                    match landed {
                        Some(next) => {
                            state = next;
                            continue;
                        }
                        // No area will take one — a real state late in a game,
                        // when the assistants are gone. Placing none is legal.
                        None => "confirm".to_string(),
                    }
                }
            }
            "place_b" => {
                // Option B names its own legal areas on the wire, so this one
                // needs no probing.
                let areas = state["prompt"]["areas"].as_array().cloned().unwrap_or_default();
                if state["prompt"]["selected"] != Value::Null || areas.is_empty() {
                    "confirm".to_string()
                } else {
                    let area = areas[rng.below(areas.len())].as_u64().expect("an area id");
                    let picked = api.send_as(mover, &format!("A{area:02}")).await;
                    capture(&mut out, &mut seen, "place_b_selected", &picked);
                    state = picked;
                    continue;
                }
            }
            "retreat" => {
                let max = state["prompt"]["max"].as_u64().unwrap_or(0) as usize;
                if state["prompt"]["remaining"].as_u64().unwrap_or(0) > 0 {
                    rng.below(max + 1).to_string()
                } else {
                    "confirm".to_string()
                }
            }
            "reveal" => panic!("a simulation must never ask for a card reveal: {state}"),
            "game_over" => break,
            other => panic!("unknown prompt kind {other:?}: {state}"),
        };
        state = as_actor(&api, api.send_as(mover, &line).await).await;
    }

    capture(&mut out, &mut seen, "game_over", &state);

    // The corpus's FILE SET is part of the contract — the golden refuses a file
    // nothing emits, and the client imports each one by name — so a shape that
    // is captured opportunistically has to be captured every time or not at
    // all. These say which, at the seeds above, rather than leaving the set to
    // depend on where the walk happened to go.
    for owed in [
        "refused",
        "options_many",
        "options_priced",
        "fee_nonzero",
        "retreat_answered",
    ] {
        assert!(
            seen.iter().any(|s| s == owed),
            "the walk never reached the {owed:?} shape. The corpus's file set must not depend \
             on where a walk happens to go: either move WALK_SEED until it does, or drop the \
             capture and say in `contract.ts` which client branch is uncovered.",
        );
    }

    // A FOUR-PLAYER table, at its opening prompt.
    //
    // The whole corpus above is 2P, and three things exist only away from it:
    // the Canal tile (`board_gen`'s tile set has none at two players, and the
    // client draws it as the one tile that is never an input), four seat
    // colours, and the wider staggered board. The seat-identity defect this
    // lane's review found — a frame built for seat 0 while the prompt carried
    // seat 2's options — was reachable only at 3P or 4P, which is one dropdown
    // away in the client's own setup screen.
    // **One person in all four seats**, which is the CLI's "one operator driving
    // the table" and keeps this table's opening a single identity's sequence.
    // The corpus shape it exists for is the 4P BOARD, which no seat binding
    // changes; the two-people case is the `wire` game above.
    let (status, four) = api
        .call(
            "POST",
            "/api/games",
            Some(json!({
                "players": 4,
                "colors": ["Red", "Blue", "Yellow", "Green"],
                "start_seat": 0,
                "seed": SETUP_SEED,
                "seats": [{ "user": WALKER }, { "user": WALKER }, { "user": WALKER }, { "user": WALKER }],
                "name": "wire4",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{four}");
    for seat in 0..4 {
        let (status, kept) =
            api.call("POST", "/api/games/wire4/keep", Some(json!({ "seat": seat, "card": 1 }))).await;
        assert_eq!(status, StatusCode::OK, "seat {seat}: {kept}");
    }
    out.push(("options_4p".to_string(), api.get("/api/games/wire4").await));
    // The area panel at 4P as well. Only away from two players does the tile set
    // contain a Canal — whose ladder is EMPTY, which is the client's "never an
    // action area" branch — or a Chinatown, whose rung pays `Exchanges`. A
    // corpus of 2P surfaces leaves both rendered by nothing.
    out.push(("areas_4p".to_string(), api.get("/api/games/wire4/areas").await));

    // The log, read AFTER the game is over, so it carries real entries.
    out.push(("log".to_string(), api.get("/api/games/wire/log").await));

    // **A historical position — the M4 replay payload.**
    //
    // Taken at the MIDPOINT of the finished game on purpose. At `k == total`
    // the payload is the live frame by construction, so a client rendering it
    // would prove nothing about the replay path; at `k == 0` the board is
    // empty and every panel is a zero. The midpoint is the only stop that
    // exercises a real position this game has already left behind — which is
    // the whole shape the scrubber renders.
    let total = api.get("/api/games/wire/log").await["total"].as_u64().unwrap_or(0);
    assert!(total > 2, "a finished game's log carries events; got {total}");
    out.push(("at_k".to_string(), api.get(&format!("/api/games/wire/at/{}", total / 2)).await));

    // **The final score, which only a FINISHED game has.** The walk plays to a
    // winner, so this is the one place in the corpus it can come from — and the
    // reason `result` is a 409 while a game is still running rather than an
    // empty payload it could have been faked from.
    out.push(("result".to_string(), api.get("/api/games/wire/result").await));

    // One card index per kind, plus a FILTERED one.
    //
    // Four rather than one because the four kinds group and word their rows
    // differently — an orders index groups by the goods a card wants, a tech
    // index by gear cost — so a corpus with one of them leaves three client
    // branches with no payload. The filtered pair is the same first-sighting
    // rule the prompt shapes follow: `shown < total` is a branch the client
    // renders differently ("36 of 36" against "7 of 36") and one payload per
    // kind never reaches it.
    // **Fetched by a caller who is NOT the acting seat's holder, and that is
    // asserted rather than hoped for** (M6.1, R-4.1). `CardLocation::OwnHand`
    // marks the READER's hand; before the index was scoped to the viewer it
    // marked the MOVER's, and this very fixture froze the leak — the corpus
    // committed at the previous HEAD named both of seat 1's cards to `WALKER`,
    // who holds seat 0. So what these files now pin is an ABSENCE, and an
    // absence is only a pin while the reader is somebody else: if the walk ever
    // ends with `WALKER` to move, these fixtures stop discriminating and this
    // says so instead of going quiet.
    let over = api.get("/api/games/wire").await;
    assert_eq!(
        over["to_move"].as_u64(),
        Some(1),
        "the finished game must leave the OTHER seat to move, or the index fixtures below are \
         taken by the acting seat's own holder and pin nothing",
    );
    for kind in ["orders", "techs", "goals", "sites"] {
        out.push((
            format!("index_{kind}"),
            api.get(&format!("/api/games/wire/index/{kind}")).await,
        ));
    }
    let filtered = api.get("/api/games/wire/index/orders?filter=copper").await;
    assert!(
        filtered["shown"].as_u64() < filtered["total"].as_u64(),
        "the filtered fixture must actually be narrower, or it is a second copy of the first"
    );
    out.push(("index_filtered".to_string(), filtered));

    // The one error shape. Every refusal on this API is `{"error": "..."}`
    // with a status, and a client that has to guess between shapes gets the
    // path it exercises least wrong.
    let (status, err) = api.call("GET", "/api/games/no-such-game", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    out.push(("error".to_string(), err));

    out
}

/// The two payloads the router cannot produce, built in process.
///
/// Both are real states of the served type, and both are unreachable over HTTP
/// from a deterministic game: `Watching` needs a BOT to be the acting seat (and
/// a bot's seed is entropy), and `Locked` needs a driver or engine fault.
fn constructed() -> Vec<(String, Value)> {
    use yokohama_cli::bot::{BotSeats, PROVISIONAL_DEFAULT_BOT};
    use yokohama_cli::repl::Repl;
    use yokohama_cli::session::Session;
    use yokohama_core::{GameConfig, PlayerColor, PlayerId, Rules, StartingOrderChoice};
    use yokohama_web::client::ClientState;

    let build = |bot_seats: Vec<Option<yokohama_core::Contestant>>| {
        let rules = Rules::standard();
        let mut config = GameConfig::for_player_count(2);
        config.player_colors = vec![PlayerColor::Red, PlayerColor::Blue];
        config.start_player = PlayerId::new(0);
        let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)];
        let mut session =
            Session::new_sim(config, SETUP_SEED, choices, rules).expect("a fixture game");
        let bots = BotSeats::new(bot_seats, BOT_SEED);
        session.set_run_seats(bots.seat_provenance());
        Repl::new(session)
    };

    // Seat 0 is the bot and it is seat 0's turn, so the payload is built for
    // seat 1 and carries no answerable list. Deliberately NOT pumped: the pump
    // is what makes this state transient in the served API, and the client
    // still has to render it.
    //
    // The viewer is seat 1 EXPLICITLY now (M6.1 §5.1). Over the router it would
    // be `viewer_seat`'s answer for the person holding seat 1 — the bot holds
    // the acting seat, so a caller falls back to their own lowest — and that is
    // the same seat this constructs, stated rather than derived because there
    // is no table document in process.
    let watching = build(vec![Some(PROVISIONAL_DEFAULT_BOT), None]);
    let watching =
        serde_json::to_value(ClientState::of("wire", &watching, PlayerId::new(1), Vec::new()))
            .expect("a serializable payload");

    let mut locked = build(vec![None, None]);
    locked
        .session
        .lock_down("the engine reported an invariant violation applying that move".to_string());
    let locked =
        serde_json::to_value(ClientState::of("wire", &locked, PlayerId::new(0), Vec::new()))
            .expect("a serializable payload");

    // ---- M3: the explanation surface ----------------------------------------
    //
    // These are built HERE and not in the walk, for the reason the `watching`
    // fixture is: `Games::create` draws its bot seed from OS entropy, so a
    // golden over a router-created bot game would churn on every run. With a
    // fixed `BOT_SEED` and a fixed `SETUP_SEED` the account below is the same
    // account every time, which is what a committed payload has to be.

    let rules = Rules::standard();

    // `why` over a move a bot actually played, at a position where the bot
    // ACTUALLY CHOSE something.
    //
    // **The first draft of this fixture was degenerate, in exactly the way
    // `options_many` exists to prevent.** One pump from the opening produced
    // `forced: true`, `root_candidates: 1`, a one-step continuation and a
    // one-row ranking — so the runner-up table, the margin column and the
    // multi-step line had no payload behind them, and a client could render all
    // three wrong without failing anything. The M2.5 record states this rule as
    // "a corpus of one payload per kind is a corpus in which every main prompt
    // is the one-row 'end the additional-action phase'"; it applies here too.
    //
    // So: BOTH seats are bots and the pump runs once. That is bounded and
    // deterministic without a policy to write — `pump_bots` stops itself at
    // `PUMP_MOVE_CAP` moves across consecutive bot seats — and it leaves a
    // record full of real bot decisions. The scan then takes the first one from
    // the MIDDLE of the game that is genuinely a choice.
    let mut played = build(vec![Some(PROVISIONAL_DEFAULT_BOT), Some(PROVISIONAL_DEFAULT_BOT)]);
    played.pump_bots();
    let bots =
        BotSeats::new(vec![Some(PROVISIONAL_DEFAULT_BOT), Some(PROVISIONAL_DEFAULT_BOT)], BOT_SEED);
    let record = played.session.record().clone();
    let mode = played.session.mode;

    // From the middle rather than the start: an opening explanation is mostly
    // zero terms at `progress_ppm: 0`, and the chart this payload exists to
    // draw is a chart of a position someone had to think about.
    let mut account = None;
    for index in (record.events.len() / 2)..record.events.len() {
        if let Ok(a) = yokohama_cli::why::account_parts(mode, &record, &rules, &bots, Some(index)) {
            if !a.reconstruction.forced && a.explain.ranking.len() >= 3 {
                account = Some(a);
                break;
            }
        }
    }
    let account = account.expect(
        "no bot move in the second half of the pumped game was a real choice with three or more \
         candidates — the fixture would be degenerate, so this refuses rather than committing it",
    );
    let why = serde_json::to_value(yokohama_web::explain::Why::granted(&account, &rules))
        .expect("a serializable account");

    // **A TERMINAL explanation, which is a different picture and not an empty
    // one — and which nothing checked until a review proved it.**
    //
    // `Explanation.terminal` is `null` in both payloads above, so every check
    // in the net skipped straight over the branch: `Widen<Explanation>` is
    // satisfied by `null` without ever reaching `Widen<Terminal>`, a field that
    // serializes to `null` contributes no bytes for the golden to compare, and
    // the client's own test for the branch HAND-AUTHORED its payload — the one
    // thing `contract.ts` forbids. Renaming all five wire fields of `Terminal`
    // left 54 Rust tests, the typecheck and 173 vitest cells green.
    //
    // It is not a hypothetical branch: `DecisionExplanation.explanation` is the
    // explanation of the LEAF the search reached, so a continuation that ends
    // the game reaches a terminal leaf and a real `suggest` near the end of a
    // real game emits one. This plays the pumped game OUT — both seats are
    // bots, so repeated pumps finish it — and explains the finished position
    // through the engine's own entry point.
    let mut finished = played;
    for _ in 0..200 {
        if finished.session.is_over() {
            break;
        }
        finished.pump_bots();
    }
    assert!(finished.session.is_over(), "the both-bot pump must reach a finished game");
    let state = finished.session.state();
    let view = yokohama_core::game::observe(state, state.players[0].id).expect("a seat's view");
    assert!(
        yokohama_core::game::eval::view_is_terminal(&view),
        "a finished game's view is terminal, or this fixture is not the branch it claims"
    );
    let core_terminal = yokohama_core::game::eval::explain(
        &view,
        &rules,
        &yokohama_core::game::eval::WeightTable::V4_STEP9_DRAFT1,
    );
    assert!(core_terminal.terminal.is_some(), "and its explanation carries a standing");
    let terminal = serde_json::to_value(yokohama_web::explain::Explanation::of(&core_terminal))
        .expect("a serializable terminal explanation");

    // A refusal, reached ORGANICALLY: a game with no bot has no bot move to
    // explain, and the browser renders that sentence rather than a status code.
    let no_bots = build(vec![None, None]);
    // A `match` rather than `expect_err`: `WhyAccount` carries a `GameState` and
    // deliberately derives no `Debug`, which `expect_err` would require.
    let refusal = match yokohama_cli::why::account_parts(
        no_bots.session.mode,
        no_bots.session.record(),
        &rules,
        &BotSeats::new(vec![None, None], BOT_SEED),
        None,
    ) {
        Err(r) => r,
        Ok(_) => panic!("a game with no bot seat has no bot move to account for"),
    };
    let why_refused = serde_json::to_value(yokohama_web::explain::Why::refused(&refusal))
        .expect("a serializable refusal");

    vec![
        ("watching".to_string(), watching),
        ("locked".to_string(), locked),
        ("why".to_string(), why),
        ("why_refused".to_string(), why_refused),
        ("explanation_terminal".to_string(), terminal),
        ("why_refusals".to_string(), every_refusal_payload()),
    ]
}

/// **Every refusal `why` can raise, as the browser would receive it.**
///
/// M2.5's lesson, applied to the surface M3 adds: a branch the client renders
/// and nobody has ever seen is exactly the shape of the two defects M2 found by
/// clicking. The prompts needed a NAMED EXEMPTION (`reveal`, live-mode only)
/// because a fixture for it cannot be reached; these need none, because
/// `Refusal` is a plain enum and every variant is constructible.
///
/// **The samples are `explain::refusal_samples()`' and are not restated here.**
/// An adversarial review found three hand-written lists agreeing with each other
/// and none pinned to the enum — this file's own vec was the third. There is one
/// now, in `explain.rs`, and its doc states exactly what the fence catches and
/// what it does not.
fn every_refusal_payload() -> Value {
    use yokohama_web::explain::{refusal_samples, RefusalDto};
    let rows: Vec<RefusalDto> = refusal_samples().iter().map(RefusalDto::of).collect();
    serde_json::to_value(rows).expect("serializable refusals")
}

/// The whole committed corpus, in one place, so the two tests below cannot
/// disagree about what it contains.
async fn corpus() -> Vec<(String, Value)> {
    let mut all = walk().await;
    all.extend(constructed());
    all.push((
        "art_manifest".to_string(),
        serde_json::to_value(
            yokohama_web::art::load(&art_dir()).expect("the committed art manifest"),
        )
        .expect("a serializable manifest"),
    ));
    all.push(("site_space_words".to_string(), site_space_words()));
    // The ONE normalization, applied to everything so no payload can acquire a
    // clock without passing through it. See [`FROZEN_CREATED_AT`].
    for (_, value) in all.iter_mut() {
        freeze_clock(value);
    }
    // Sorted, so the file set is a set and its order is not an accident of the
    // walk. (The names, not the payloads — payload key order is serde's.)
    all.sort_by(|a, b| a.0.cmp(&b.0));
    all
}

/// **The terminal's own words for every printed building-site space.**
///
/// Not a payload any route serves — a CONTRACT, frozen the same way the payloads
/// are, because the client has to word these itself and had been getting them
/// wrong.
///
/// `Index.tsx` used to build the words from the serde field names
/// (`${n} ${kind.replace(/_/g, " ")}`), which disagreed with
/// `card_render::space_reward_words` on **55 of the 96 printed spaces**. Two of
/// those disagreements state a rule wrongly rather than merely reading oddly:
/// `movement` is the ASSISTANT-move icon (`types/board.rs:109-111`) and rendered
/// as "movement", which reads as President movement; and `warehouse_tokens` is a
/// number of TAKES from the warehouse, not a number of tokens in it.
///
/// The client cannot import a Rust function, so the two are held together by
/// this fixture instead: the terminal emits its words here, and
/// `Cards.spaceWords` is asserted against them in `Index.test.tsx`. Neither side
/// can be re-worded while both believe they agree — which is the same shape as
/// the rest of this file, applied to a string the client has to compute.
fn site_space_words() -> Value {
    let rules = Rules::standard();
    let cards: Vec<Value> = rules
        .building_sites
        .iter()
        .enumerate()
        .map(|(index, site)| {
            serde_json::json!({
                "index": index,
                "spaces": site
                    .spaces
                    .iter()
                    .map(yokohama_cli::card_render::space_reward_words)
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    serde_json::json!({ "cards": cards })
}

/// **No payload spells the setup seed, except the one whose job is to.**
///
/// The seed is the FIRST member of the engine's hidden set (`game/observe.rs`):
/// it regenerates the order deck, the tech deck, the boxed pile, the board AND
/// every deal. `games::draw_seed`'s own doc states the threat model — the public
/// board is an injective fingerprint of the seed, so small seeds are
/// "brute-forceable in milliseconds, recovering the entire hidden set". Serving
/// the value removes the need to brute-force anything: `new_game(config, rules,
/// seed)` reproduces the whole hidden set directly, which narrows an opponent's
/// kept starting order from thirty-six cards to two.
///
/// It was on 21 of 41 committed payloads, twice over — as `ClientState.seed`
/// and again as `frame.title.seed` — and rendered by one line of one title
/// strip. Parity with the CLI while one operator plays one bot; a total break
/// of the hidden-information model on the FIRST FRAME the moment there is a
/// second human or a spectator, which `06-decisions-and-open-questions.md`
/// records as the stated future.
///
/// **This is the fence, and it is deliberately mechanism-independent.** The fix
/// is typed — `TitleModel::seed` is an `Option` the web passes `None` — but a
/// future field, a debug route or a renamed key could publish the number again
/// without touching that type. So this searches the SERIALIZED TEXT of every
/// committed payload for the value itself, in the two spellings JSON can hold
/// it in. `preview` is the single exemption and needs no list of its own: the
/// setup flow round-trips the seed from preview back into `create`, so a
/// preview that did not carry it could not deal the game the player chose.
#[tokio::test]
async fn no_committed_payload_carries_the_setup_seed_except_the_preview() {
    let decimal = SETUP_SEED.to_string();
    let mut offenders: Vec<String> = Vec::new();

    for (name, value) in corpus().await {
        if name.starts_with("preview") {
            // The exemption, asserted rather than assumed: if `preview` ever
            // STOPS carrying the seed, this test is silently guarding nothing
            // and the round trip is broken anyway.
            assert!(
                serde_json::to_string(&value).expect("serializable").contains(&decimal),
                "preview must carry the seed it round-trips — {name} does not",
            );
            // The exemption is the SEED FIELD and nothing else. `preview` now
            // carries a whole position beside it, and a frame is served
            // seed-free everywhere else in this API — so the table inside the
            // one exempt payload is held to the same rule the exemption does
            // not cover.
            let table = serde_json::to_string(&value["table"]).expect("serializable");
            assert!(
                !table.contains(&decimal),
                "{name}.table carries the setup seed — the position is served seed-free like every frame",
            );
            continue;
        }
        let text = serde_json::to_string(&value).expect("serializable");
        // As a JSON number and as a JSON string: `WireSeed` serializes to a
        // string, `TitleModel` used to serialize to a number, and a fence that
        // knew only one spelling would have missed the other.
        if text.contains(&decimal) {
            offenders.push(name);
        }
    }

    assert!(
        offenders.is_empty(),
        "the setup seed reaches the client in {} payload(s): {}\n\
         The seed regenerates every deal, both decks, the boxed pile and the board. \
         If a new field genuinely needs it, it needs an owner decision first, not an \
         exemption here.",
        offenders.len(),
        offenders.join(", "),
    );
}

/// **The gate: what the server emits is what the client type-checks against.**
#[tokio::test]
async fn the_committed_wire_fixtures_are_what_the_server_emits() {
    let dir = wire_dir();
    let update = std::env::var_os("UPDATE_WIRE_FIXTURES").is_some();
    if update {
        std::fs::create_dir_all(&dir).expect("the wire directory");
    }

    let mut drifted: Vec<String> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for (name, value) in corpus().await {
        names.push(name.clone());
        let path = dir.join(format!("{name}.json"));
        let emitted = format!("{}\n", serde_json::to_string_pretty(&value).expect("pretty JSON"));
        if update {
            std::fs::write(&path, &emitted).expect("writing a fixture");
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(committed) if committed == emitted => {}
            Ok(committed) => drifted.push(format!(
                "  {name}.json — committed {} bytes, server emits {} bytes; first difference at byte {}",
                committed.len(),
                emitted.len(),
                committed
                    .bytes()
                    .zip(emitted.bytes())
                    .position(|(a, b)| a != b)
                    .unwrap_or_else(|| committed.len().min(emitted.len())),
            )),
            Err(e) => drifted.push(format!("  {name}.json — {e}")),
        }
    }

    // A file nobody emits is a contract the client checks against nothing.
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let file = entry.file_name().to_string_lossy().to_string();
            let Some(stem) = file.strip_suffix(".json") else { continue };
            if !names.iter().any(|n| n == stem) {
                if update {
                    std::fs::remove_file(entry.path()).expect("removing a stale fixture");
                } else {
                    drifted.push(format!("  {file} — committed, but nothing emits it any more"));
                }
            }
        }
    }

    assert!(
        drifted.is_empty(),
        "the wire contract moved:\n{}\n\nIf the change is intended, regenerate and READ THE DIFF:\n  \
         UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures\n\
         then re-run the client's type gate, which checks these files against \
         `web/src/types.ts`:\n  (cd web && npm run typecheck)",
        drifted.join("\n"),
    );

    assert!(!update, "UPDATE_WIRE_FIXTURES was set: fixtures were REWRITTEN, not checked");
}

/// One value per `PromptState` variant, and the ONE place the variant list lives.
///
/// **`prompt_ordinal` below has no `_` arm, so a new variant is a COMPILE ERROR
/// in THIS file** — which is what puts a developer here at all. The first draft
/// of this test claimed that property and did not have it: the only exhaustive
/// match was `PromptState::kind` in the crate itself, so a new variant forced an
/// edit THERE and both hand-written lists here stayed nine items long. A review
/// added a tenth variant and all three tests passed.
fn every_prompt_variant() -> Vec<yokohama_web::client::PromptState> {
    use yokohama_core::{AreaId, PlayerId};
    use yokohama_web::client::PromptState;
    vec![
        PromptState::Options { rows: Vec::new() },
        PromptState::Pick { title: String::new(), rows: Vec::new() },
        PromptState::Retreat {
            area: AreaId::new(0),
            area_label: String::new(),
            max: 0,
            answered: Vec::new(),
            remaining: 0,
        },
        PromptState::PlaceA { selected: Vec::new(), max_areas: 0, fee: None },
        PromptState::PlaceB { areas: Vec::new(), selected: None, fee: None },
        PromptState::Reveal,
        PromptState::GameOver,
        PromptState::Locked { reason: String::new() },
        PromptState::Watching { to_move: PlayerId::new(0) },
    ]
}

/// Where each variant sits in [`every_prompt_variant`]. **No `_` arm, ever.**
fn prompt_ordinal(p: &yokohama_web::client::PromptState) -> usize {
    use yokohama_web::client::PromptState;
    match p {
        PromptState::Options { .. } => 0,
        PromptState::Pick { .. } => 1,
        PromptState::Retreat { .. } => 2,
        PromptState::PlaceA { .. } => 3,
        PromptState::PlaceB { .. } => 4,
        PromptState::Reveal => 5,
        PromptState::GameOver => 6,
        PromptState::Locked { .. } => 7,
        PromptState::Watching { .. } => 8,
    }
}

/// The list above is EVERY variant, exactly once, in order.
///
/// The compile-error match is half the guard and this is the other half: a new
/// arm added to `prompt_ordinal` without a value added to `every_prompt_variant`
/// would otherwise leave the list nine long and every check below still green.
/// **`9` is load-bearing and moves with the enum.**
#[test]
fn the_variant_list_is_every_variant_exactly_once() {
    let all = every_prompt_variant();
    assert_eq!(
        all.len(),
        9,
        "a `PromptState` variant was added or removed: `every_prompt_variant`, this count, and \
         the EXEMPT list in `every_prompt_the_client_can_be_shown_has_a_fixture` move together",
    );
    let ordinals: Vec<usize> = all.iter().map(prompt_ordinal).collect();
    assert_eq!(ordinals, (0..all.len()).collect::<Vec<_>>(), "one value per variant, in order");
}

/// **Every `PromptState` variant is covered, or named as an exemption.**
///
/// Without this the corpus is whatever the walk happened to reach, and a new
/// prompt variant ships with a browser branch nobody rendered and no fixture to
/// render it from. The exemption list is deliberately a list of REASONS: an
/// entry that cannot state why it is unreachable does not belong on it.
///
/// **Read off the COMMITTED files, not off a fresh walk.** Two reasons, and the
/// first is the load-bearing one: the property worth asserting is that the
/// corpus the client type-checks against covers every branch, and that is a
/// statement about the files. (The second is that a second full game costs a
/// full game.)
#[test]
fn every_prompt_the_client_can_be_shown_has_a_fixture() {
    /// Unreachable, with the reason. `reveal` is the only one.
    const EXEMPT: &[(&str, &str)] = &[(
        "reveal",
        "live mode only — this API creates no live games (v1 owner decision), and the \
         whole-game gate panics if a simulation ever asks for a card reveal",
    )];

    // A regeneration run rewrites the very directory this test reads, from a
    // sibling test, with no ordering between them. Rather than race it, say so:
    // a regeneration is not a check, and the run it happens in already fails.
    assert!(
        std::env::var_os("UPDATE_WIRE_FIXTURES").is_none(),
        "regeneration run — re-run WITHOUT UPDATE_WIRE_FIXTURES to check the corpus",
    );

    let dir = wire_dir();
    let mut kinds: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e} — generate the corpus first with \
             `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`", dir.display()))
        .flatten()
    {
        // The directory also holds the client's own `contract.ts`, which is
        // the other half of this contract and not a payload.
        if entry.path().extension().is_none_or(|e| e != "json") {
            continue;
        }
        let text = std::fs::read_to_string(entry.path()).expect("a committed fixture");
        let value: Value = serde_json::from_str(&text).expect("a committed fixture is JSON");
        if let Some(kind) = value["prompt"]["kind"].as_str() {
            kinds.push(kind.to_string());
        }
    }

    // The tags come from the variant list, which the compile-error match above
    // pins to the enum — so this cannot go stale in the direction that would
    // make it vacuous.
    let tags: Vec<&str> =
        every_prompt_variant().iter().map(yokohama_web::client::PromptState::kind).collect();
    let missing: Vec<&str> = tags
        .into_iter()
        .filter(|t| !kinds.iter().any(|k| k == t))
        .filter(|t| !EXEMPT.iter().any(|(e, _)| e == t))
        .collect();
    assert!(
        missing.is_empty(),
        "no fixture covers prompt kind(s) {missing:?} — the client renders a branch for each, \
         and a branch with no payload behind it is a branch nobody has seen. Either extend the \
         walk in `wire_fixtures.rs` or add the tag to EXEMPT with the reason it cannot be \
         reached.",
    );
}

/// **Every `why` refusal the browser can be handed has a payload — with NO
/// exemption list, and that is the point.**
///
/// The sibling test above needs one, because `reveal` genuinely cannot be
/// reached by an API that creates no live games. This one needs none:
/// `why::Refusal` is a plain enum, so `explain::refusal_samples()` constructs
/// all twelve and words each with the CLI's own `Display`. A coverage claim
/// with an empty exemption list is worth strictly more than one with entries,
/// so it is asserted to STAY empty.
///
/// **What this catches, stated precisely.** The corpus on disk is compared
/// against what `refusal_samples()` produces TODAY, so a sample removed, added
/// or re-tagged without regenerating fails here. What it does NOT catch is a
/// variant added to `Refusal` and to `explain::kind_of` but never sampled —
/// `refusal_samples()`' own doc names that hole and why plain Rust cannot close
/// it. An earlier version of this comment claimed a three-link chain that
/// closed it; a reviewer added a thirteenth variant plus a one-line `kind_of`
/// arm and every test here stayed green. The claim was wrong, not the code, and
/// this is the corrected claim.
#[test]
fn every_why_refusal_the_client_can_be_shown_has_a_payload() {
    assert!(
        std::env::var_os("UPDATE_WIRE_FIXTURES").is_none(),
        "regeneration run — re-run WITHOUT UPDATE_WIRE_FIXTURES to check the corpus",
    );

    let path = wire_dir().join("why_refusals.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{}: {e} — generate the corpus first with \
             `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`",
            path.display()
        )
    });
    let rows: Vec<Value> = serde_json::from_str(&text).expect("an array of refusals");

    let mut present: Vec<String> = rows
        .iter()
        .map(|r| r["kind"].as_str().expect("every refusal is tagged").to_string())
        .collect();
    present.sort();
    let unique = {
        let mut u = present.clone();
        u.dedup();
        u
    };
    assert_eq!(present, unique, "a refusal kind appears twice in the corpus");

    let mut declared: Vec<String> =
        yokohama_web::explain::refusal_kinds().iter().map(|s| s.to_string()).collect();
    declared.sort();
    assert_eq!(
        present, declared,
        "the refusal corpus and `explain::refusal_samples()` disagree. Every variant the browser can \
         be handed needs a payload behind it — a branch with no payload is a branch nobody has \
         seen, which is exactly how M2's two client defects shipped.",
    );

    // Anti-vacuity: a corpus of twelve empty sentences would satisfy the set
    // check above and prove nothing about what a reader is shown.
    for row in &rows {
        let message = row["message"].as_str().expect("every refusal carries a sentence");
        assert!(
            message.len() > 20,
            "{} says nothing a player could act on: {message:?}",
            row["kind"]
        );
    }
}
