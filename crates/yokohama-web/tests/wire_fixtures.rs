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
//! **Why the game has no bot.** The bot seed is drawn from OS entropy at
//! FINALIZE — `Games::finalize`'s `BotSeats::new(…, draw_seed())`, which the
//! LAST keep reaches, not `create` and not `config_of` — deliberately, so a
//! game with a bot in it does not reproduce and a golden built from one would
//! churn on every run.
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

/// The walk's table password (M6.2). A constant like the seeds, so the corpus
/// stays a function of what is written in this file — and so
/// [`no_payload_of_this_run_carries_the_password_or_its_hash`] has a literal to
/// search for.
///
/// **What it gates is not what it is worth.** A table password is a shared
/// game password (R-2.8): it decides who may take a seat at a table behind a
/// door Cloudflare Access has already opened, and nobody's identity rests on
/// it. So a readable word is the right literal here, and the fixtures below
/// prove the SALT and the HASH never leave the server whatever the word is.
const WIRE_PASSWORD: &str = "sesame";

/// The WRONG password the walk sends, as a constant for the same reason the
/// right one is: so the pin has a literal to search for.
///
/// **A review found the gap this closes, and it is worth stating exactly.** The
/// pin searched the corpus for [`WIRE_PASSWORD`] alone. Three payloads are
/// produced by requests carrying that literal, so an echo in those is caught —
/// but the wrong-password attempt carries a DIFFERENT string, so a refusal that
/// repeated the attempt back ("wrong password: {given}") would contain the
/// attempt and never the table's own password, and the search could not see it.
/// That is the realistic leak on this route: a sentence quoting what the caller
/// typed, which is exactly what a person then finds in a bug report, a proxy log
/// or a screenshot.
///
/// Two things about the literal, and the second was measured rather than
/// reasoned. It is **not a word**: an accidental occurrence in some unrelated
/// payload would fail the pin for a reason that is not a leak, and a common
/// English word in a corpus with this much prose in it is a real risk. And it
/// **shares no substring with [`WIRE_PASSWORD`]**: the first draft was
/// `"not-sesame-9f3a"`, which contains `"sesame"`, so the two needles were not
/// independent — an echoed ATTEMPT was reported as the table's own password
/// leaking, and the pin could not have told the two apart.
const WIRE_WRONG_PASSWORD: &str = "wrongpass-9f3a";

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
    /// How many of this walk's calls were ALSO answered by the layer's dispatcher
    /// directly and compared byte for byte — see [`Api::call_as`]. The count is
    /// the anti-vacuity of `the_dispatcher_answers_what_the_router_answers`.
    compared: std::cell::Cell<usize>,
}

impl Api {
    fn new() -> Self {
        Self::with_chat(yokohama_web::chat::ChatConfig::default())
    }

    /// The same router with the chat configured rather than defaulted (M6.4).
    ///
    /// One fixture in this corpus is the answer a deployment gives when the
    /// house chat is OFF, and a switch read at startup cannot be flipped over
    /// the wire — so that payload comes from a second state built here rather
    /// than from an extra route on the first.
    fn with_chat(chat: yokohama_web::chat::ChatConfig) -> Self {
        let state = Arc::new(AppState::in_memory_with_chat(chat));
        Self { router: app(Arc::clone(&state)), state, compared: std::cell::Cell::new(0) }
    }

    /// The same router over a real ARCHIVE directory (M6.5b, R-6.5b.6).
    ///
    /// A third state, for the reason the second one exists: `/api/archive` is a
    /// 404 wherever the archive is off, and the primary `Api` keeps `NoArchive`
    /// deliberately — so `health.json` goes on reading `"archive": "off"` and
    /// the cut moves no payload the walk already froze.
    fn with_archive(dir: &std::path::Path) -> Self {
        let state = Arc::new(AppState::with_store_chat_and_archive(
            Arc::new(yokohama_web::store::MemStore::default()),
            yokohama_web::identity::IdentityMode::Claimed,
            yokohama_web::chat::ChatConfig::default(),
            Arc::new(yokohama_web::archive::DirArchive::new(dir)),
        ));
        Self { router: app(Arc::clone(&state)), state, compared: std::cell::Cell::new(0) }
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
        let body_bytes: Option<Vec<u8>> = body.as_ref().map(|v| serde_json::to_vec(v).expect("a body"));
        let req = match &body_bytes {
            Some(b) => req
                .header("content-type", "application/json")
                .body(Body::from(b.clone()))
                .expect("a request"),
            None => req.body(Body::empty()).expect("a request"),
        };
        let res = self.router.clone().oneshot(req).await.expect("the router answers");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        self.compare_with_dispatch(who, method, uri, body_bytes.as_deref(), status, &bytes);
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                panic!("{uri} must answer JSON: {e}: {:?}", String::from_utf8_lossy(&bytes))
            })
        };
        (status, value)
    }

    /// **R-6.5a.14, read at its real size**: the layer's `dispatch`, called
    /// DIRECTLY with the same method, path, query, body and identity, must
    /// answer what the router just answered — status and body, byte for byte.
    /// After M6.5a the router IS the adapter over that same `dispatch`, so what
    /// this proves is that the ADAPTER's request conversion is faithful — the
    /// path/query split, the content type, the body bytes, the identity, the
    /// status round-trip — and not that two implementations agree; a second
    /// host's corpus claim is that host's own cell (M6.5b). Compared HERE, at
    /// capture time and against the same state (SDD ruling P-1): a replay after
    /// the walk would meet a different table (`table_dealing` against a playing
    /// one), and a replay of a 2xx write would apply the move twice. So every
    /// READ (GET/HEAD) and every REFUSAL (4xx) is compared; a write that landed
    /// is not. The three transport routes and `/api/art` are the host's own and
    /// are skipped.
    fn compare_with_dispatch(
        &self,
        who: Option<&str>,
        method: &str,
        uri: &str,
        body: Option<&[u8]>,
        status: StatusCode,
        bytes: &[u8],
    ) {
        let (path, query) = uri.split_once('?').unwrap_or((uri, ""));
        let hosts_own = path.ends_with("/events")
            || path.ends_with("/stream")
            || path == "/api/stream"
            || path == "/api/art";
        if hosts_own || !(method == "GET" || method == "HEAD" || status.is_client_error()) {
            return;
        }
        let user = who.map(|w| yokohama_web::identity::User::parse(w).expect("a valid identity"));
        let sentence = yokohama_web::identity::resolve(self.state.identity, &axum::http::HeaderMap::new())
            .expect_err("no header, no user");
        let identity: yokohama_web::api::Identity<'_> = match &user {
            Some(u) => Ok(u),
            None => Err(sentence.as_str()),
        };
        let host = yokohama_web::api::Host {
            identity_name: self.state.identity.name(),
            signal_kind: yokohama_web::SIGNAL_KIND,
        };
        let out = yokohama_web::api::dispatch(
            &self.state.games,
            &host,
            identity,
            &yokohama_web::api::ApiRequest {
                method,
                path,
                query,
                content_type: body.map(|_| "application/json"),
                body: body.unwrap_or(b""),
            },
        );
        assert_eq!(out.status, status.as_u16(), "{method} {uri}: status through dispatch vs the router");
        assert_eq!(
            out.body.as_bytes(),
            bytes,
            "{method} {uri}: body through dispatch vs the router\n  dispatch: {}\n  router:   {}",
            out.body,
            String::from_utf8_lossy(bytes)
        );
        self.compared.set(self.compared.get() + 1);
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

/// Replace every `created_at` with [`FROZEN_CREATED_AT`], wherever it is nested
/// — **and the same clock where it is spelled inside a list CURSOR**.
///
/// A walk rather than a list of paths: the field rides on `TableView`, which
/// appears at the root of two fixtures, inside `deal.table`, and once per row of
/// `games.games` — and a list of those would be a fourth statement of the wire
/// shape to keep true.
///
/// **`next` is the second clock on this wire and it does not look like one**
/// (M6.2). A page cursor is `<created_at>,<id>`, so `games_next.json` would
/// otherwise carry a wall-clock timestamp inside a string and churn on every
/// regeneration exactly as `created_at` would have. The same rule applies to it
/// and for the same reason, and the same anti-vacuity rides along: the
/// timestamp half is REFUSED if it is not a plausible clock, so a cursor that
/// stopped carrying one fails here rather than being normalized away. The ID
/// half is left alone — it is the row the page ended at, which is the whole
/// content of the field.
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
                } else if key == "next" {
                    // `null` is the last page and carries no clock at all.
                    if let Some(cursor) = v.as_str() {
                        let (at, id) = cursor.split_once(',').unwrap_or_else(|| {
                            panic!("a page cursor is <created_at>,<id>: {cursor:?}")
                        });
                        let seen: u64 = at.parse().unwrap_or_else(|_| {
                            panic!("a page cursor's first half is a unix timestamp: {cursor:?}")
                        });
                        assert!(
                            seen > 1_700_000_000,
                            "a page cursor reads {seen}, which is not a clock — freezing it \
                             would hide a cursor that has stopped carrying one",
                        );
                        *v = json!(format!("{FROZEN_CREATED_AT},{id}"));
                    }
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

/// **The walk's third identity: nobody's seat, and from M6.3 the one who
/// knocks.**
///
/// It was called `STRANGER` until this cycle, and the rename IS the cycle: a
/// watcher is exactly a stranger who presented the password (R-3.5), so one
/// person does the whole arc — reads the open table before anyone has sat at
/// it, is refused at the deal, is refused at the locked game, gets the password
/// wrong, then gets it right and watches to the end. Keeping it one person is
/// also what puts a NAME in the table document's `unlocked` list for
/// [`a_silent_watcher_is_named_nowhere_and_a_posting_one_only_as_an_author`] to
/// search for.
///
/// **From M6.4 they also SPEAK** (R-6.4.12), which is what narrowed that pin: a
/// watcher who posts is named as the author of what they posted and nowhere
/// else, and the two other identities below are the halves of that claim the
/// allowance must not cover.
const GREEN: &str = "green@wire.test";

/// **A FOURTH identity, who never knocks** (M6.3).
///
/// `table_watchable` is the payload the client draws the UNLOCK FACE from — a
/// locked, playing table read by somebody who may see that it exists and may
/// not watch it (R-3.13) — so it has to be read by a caller whose `can_watch`
/// stays false. [`GREEN`]'s turns true a few lines later, and a fixture cut
/// from that caller would describe the other half of the branch while being
/// named after this one.
const STRANGER: &str = "grey@wire.test";

/// **A FIFTH identity, who knocks and then says NOTHING** (M6.4, R-6.4.12).
///
/// [`GREEN`] is now a watcher who POSTS, so its address is legitimately on this
/// wire as the author of one message — which means green alone can no longer
/// witness "a watcher is not named to the players". This one can: it presents
/// the password, is written into the table document's `unlocked` list exactly
/// as green is, and then makes no other request for the rest of the walk. Every
/// payload captured after that point is built while this person is watching,
/// and none of them may spell this address anywhere at all.
///
/// It is therefore the needle whose ABSENCE is the claim, and
/// [`a_silent_watcher_is_named_nowhere_and_a_posting_one_only_as_an_author`]
/// proves the search can find it by replaying an unlock of its own.
const QUIET: &str = "quiet@wire.test";

/// **An OPEN, LOCKED table with one seat still to fill** (M6.2, §7).
///
/// It used to bind both seats at the create. It binds one, and [`OTHER`] takes
/// the other through `POST …/join` — which is what puts the room's own opening
/// on this corpus: a table somebody has to be let into, a password that has to
/// be right, and a join that flips the table to `dealing` in the same write.
/// Everything after that point in the walk is the table it always was.
///
/// NO bot seats, here or at `wire2`. See the module doc: a bot's seed is OS
/// entropy and a golden built over one would churn on every run. Two PEOPLE
/// instead, which is also what makes the per-seat deal fixtures below real.
fn new_game_body() -> Value {
    json!({
        "players": 2,
        "colors": ["Red", "Blue"],
        "start_seat": 0,
        "seed": SETUP_SEED,
        "seats": [{ "user": WALKER }, { "open": true }],
        "name": "wire",
        "id": "wire",
        // Written onto the creator's seats and onto no other (R-2.3), so the
        // corpus carries a NAMED seat beside an email-only one.
        "display_name": "Red",
        "password": WIRE_PASSWORD,
    })
}

/// A SECOND open table, unlocked, so two lobby fixtures mean something.
///
/// `games_open` is the lobby's "tables I could sit at", and a page of one table
/// cannot show the lock marker against anything — both rows would read the same
/// way whether the client rendered the flag or ignored it. `games_next` needs a
/// second row for a different reason: a page of one at `limit=1` is the only way
/// `next` is ever `Some`.
///
/// **It is created BEFORE `wire`, and the order is load-bearing.** The listing
/// is `(created_at desc, id asc)` over unix SECONDS, so two tables opened in one
/// second are separated by their ids and `wire` sorts before `wire2`; opened in
/// two seconds, the NEWER sorts first. Creating `wire2` first makes `wire` the
/// first row under both — which is what keeps `games_next`'s page and its cursor
/// a function of this file rather than of where a second boundary happened to
/// fall. The other order is correct roughly 999 runs in 1000, which is the worst
/// kind of correct for a committed golden.
///
/// Nobody ever joins it and it never deals: it exists to be listed.
fn second_table_body() -> Value {
    json!({
        "players": 2,
        "colors": ["Red", "Blue"],
        "start_seat": 0,
        "seed": SETUP_SEED,
        "seats": [{ "user": WALKER }, { "open": true }],
        "name": "wire2",
        "id": "wire2",
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
    walk_with(&api).await
}

/// The walk over a caller's [`Api`], so a cell can read what the walk counted.
async fn walk_with(api: &Api) -> Vec<(String, Value)> {

    // **The opening sequence, and every payload it produces** (M6.1 §4). A
    // create opens a DEALING table; each seat fetches its own two cards and
    // keeps; the last keep starts the game. Six shapes the client draws exist
    // only inside that sequence, so they are captured as they go past rather
    // than reconstructed afterwards.
    // The second table first — see [`second_table_body`] for why the ORDER of
    // these two creates is what keeps the lobby fixtures deterministic.
    let (status, second) = api.call("POST", "/api/games", Some(second_table_body())).await;
    assert_eq!(status, StatusCode::CREATED, "{second}");
    let (status, table_created) = api.call("POST", "/api/games", Some(new_game_body())).await;
    assert_eq!(status, StatusCode::CREATED, "{table_created}");
    assert_eq!(table_created["status"], "open", "a table with an open seat opens OPEN");

    // **The room's opening, and every payload it produces** (M6.2 §7). A table
    // with an open seat is something a person who is not seated at it can read,
    // follow a link to, be refused by, and then join — and each of those is a
    // shape the client draws and nothing else in this corpus carries.

    // A caller with no seat reads the open table. The one route R-2.7 widens: a join link
    // is an address, so the person following one sees what they are joining —
    // with `my_seats` EMPTY, which is impossible at any other status.
    let (status, table_open) =
        api.call_as(Some(GREEN), "GET", "/api/games/wire/table", None).await;
    assert_eq!(status, StatusCode::OK, "an open table reads for a stranger: {table_open}");
    assert_eq!(
        table_open["my_seats"].as_array().map(Vec::len),
        Some(0),
        "a stranger holds no seat here: {table_open}",
    );

    // **The chat is NARROWER than the table** (M6.4, spec §3.2), captured at the
    // one status where the two visibly part company. The same caller, one
    // request later: `…/table` answers because a join link is an address and the
    // person following one has to see what they are joining (R-2.7); `…/chat`
    // takes no such widening, because a room is readable by the people
    // `admitted` admits and not by everyone who can see the door. Committed for
    // the SENTENCE — "you are not seated at this table" is a different thing to
    // do next from the lock's, which the walk captures below from a caller who
    // CAN knock.
    let (status, chat_403_not_seated) =
        api.call_as(Some(GREEN), "GET", "/api/games/wire/chat", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{chat_403_not_seated}");

    // **The signal at the table's FIRST status** (M6.3 §3.3). A seat holder
    // watching an open table for its seats to fill reads the same endpoint the
    // dealing screen and the game read, and gets a reading rather than the 409
    // captured just below — which is the whole point of the `open` arm, and is
    // a different fixture from `events_dealing` because `status` is the field
    // the client branches the screen on.
    let events_open = api.get("/api/games/wire/events").await;
    assert_eq!(events_open["status"], "open", "{events_open}");

    // The lobby's OTHER list: tables I could sit at, whoever made them. Read by
    // the stranger, because that is who the list is for, and carrying both
    // tables — one locked and one not, which is the only way the lock marker on
    // a row is rendered by a payload rather than by nothing.
    let games_open = api.get_as(GREEN, "/api/games?scope=open").await;

    // A SEATED caller's session route, while the table is still open. A 409
    // carrying `status: "open"` — the same shape as the dealing 409 below and a
    // different word, so the client draws the setup screen it is actually at.
    let (status, open_409) = api.call("GET", "/api/games/wire", None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{open_409}");

    // **The three ways a join is refused, in the order §3.2 refuses them.**
    // Each is the plain `ApiErrorBody` the client already renders, so what these
    // files pin is the SENTENCE: "403" alone leaves a person with nothing to do
    // next, and these three are three different things to do.
    let (status, join_403_locked) = api
        .call_as(Some(OTHER), "POST", "/api/games/wire/join", Some(json!({ "seat": 1 })))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{join_403_locked}");
    let (status, join_403_password) = api
        .call_as(
            Some(OTHER),
            "POST",
            "/api/games/wire/join",
            Some(json!({ "seat": 1, "password": WIRE_WRONG_PASSWORD })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{join_403_password}");
    // The seat the CREATOR holds, asked for with the right password: the seat
    // check runs in front of the password check, so this is a conflict about
    // the seat and says so.
    let (status, join_409_taken) = api
        .call_as(
            Some(OTHER),
            "POST",
            "/api/games/wire/join",
            Some(json!({ "seat": 0, "password": WIRE_PASSWORD })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{join_409_taken}");

    // And the join that works. It fills the LAST open seat, so it flips the
    // table to `dealing` in the same write — which is why this reply is the
    // corpus's `table_dealing`.
    let (status, table_dealing) = api
        .call_as(
            Some(OTHER),
            "POST",
            "/api/games/wire/join",
            Some(json!({ "seat": 1, "name": "Blue", "password": WIRE_PASSWORD })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{table_dealing}");
    assert_eq!(table_dealing["status"], "dealing", "the filling join flips: {table_dealing}");

    // The same join again, at a table that is no longer open: a 409 carrying
    // the status, not a 403 about the seat.
    let (status, join_409_not_open) = api
        .call_as(
            Some(OTHER),
            "POST",
            "/api/games/wire/join",
            Some(json!({ "seat": 1, "password": WIRE_PASSWORD })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{join_409_not_open}");

    // The lobby's FIRST list, paged: `wire` and `wire2` are both this caller's,
    // so a page of one leaves a row over and `next` is the only non-null cursor
    // anywhere in this corpus. See [`freeze_clock`] on the clock inside it.
    let games_next = api.get("/api/games?scope=mine&limit=1").await;
    assert!(
        games_next["next"].is_string(),
        "a page of one over two tables has a next: {games_next}",
    );

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
    // **The signal, while the table is DEALING**, and the 409 just above is what
    // makes this fixture worth committing: every route that needs a SESSION
    // refuses here, and this one answers. The setup screen therefore polls the
    // same endpoint the game does (spec §6) and gets a reading rather than an
    // error to branch on. Taken before any keep, so `status` is `dealing`,
    // `events` is 0 and `to_move` is null.
    let events_dealing = api.get("/api/games/wire/events").await;

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
        api.call_as(Some(GREEN), "GET", "/api/games/wire/deal", None).await;
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

    // **The watcher's opening, and every payload it produces** (M6.3 §3.4,
    // §3.5). The table is playing and LOCKED, so a caller with no seat at it
    // meets three different answers before being let in — and the client draws
    // a different face from each.

    // The game itself, to somebody who has presented nothing. A 403 that says
    // what to do about it, which is the whole difference between this sentence
    // and the not-seated one: this caller CAN do something next.
    let (status, watch_403_locked) = api.call_as(Some(GREEN), "GET", "/api/games/wire", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{watch_403_locked}");

    // The TABLE, which R-3.13 opened to anybody at `playing`, so a would-be
    // watcher can read `locked` and `can_watch` before asking for a password.
    // **Read by [`STRANGER`] and not by [`GREEN`]**: this is the payload the
    // unlock face is drawn from, and GREEN's `can_watch` turns true four
    // requests from here.
    let (status, table_watchable) =
        api.call_as(Some(STRANGER), "GET", "/api/games/wire/table", None).await;
    assert_eq!(status, StatusCode::OK, "{table_watchable}");
    assert_eq!(table_watchable["can_watch"], false, "seen, not watchable: {table_watchable}");
    assert_eq!(
        table_watchable["my_seats"].as_array().map(Vec::len),
        Some(0),
        "a watcher holds no seat: {table_watchable}",
    );

    // The wrong password. The sentence is the point — "403" alone leaves a
    // person unable to tell a typo from a table that is not theirs — and the
    // attempt itself is [`WIRE_WRONG_PASSWORD`], so
    // [`no_payload_of_this_run_carries_the_password_or_its_hash`] searches this
    // refusal for the string the caller typed as well as for the table's own.
    let (status, unlock_403) = api
        .call_as(
            Some(GREEN),
            "POST",
            "/api/games/wire/unlock",
            Some(json!({ "password": WIRE_WRONG_PASSWORD })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{unlock_403}");

    // An unlock at a table with nothing to watch yet. `wire2` is still open —
    // nobody ever joins it — so this is a 409 carrying the STATUS the client
    // branches the screen on, the same two-key shape as the dealing and open
    // 409s above and a different answer from either 403.
    let (status, unlock_409_open) = api
        .call_as(
            Some(GREEN),
            "POST",
            "/api/games/wire2/unlock",
            Some(json!({ "password": WIRE_PASSWORD })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{unlock_409_open}");

    // And the knock that works. **Not committed as a fixture**: the reply is a
    // `TableView`, which `table.json` already carries, and what is new about it
    // is the VALUE of one boolean — so it is asserted here rather than frozen as
    // a second copy of a shape the corpus has.
    let (status, unlocked) = api
        .call_as(
            Some(GREEN),
            "POST",
            "/api/games/wire/unlock",
            Some(json!({ "password": WIRE_PASSWORD })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{unlocked}");
    assert_eq!(unlocked["can_watch"], true, "the knock is what turns this over: {unlocked}");
    assert_eq!(
        unlocked["my_seats"].as_array().map(Vec::len),
        Some(0),
        "and a watcher is still nobody's seat: {unlocked}",
    );

    // **And a SECOND knock, from somebody who then says nothing** (M6.4,
    // R-6.4.12). This is the only request [`QUIET`] ever makes: from here to the
    // end of the walk there are two people on the rail, one of whom speaks, and
    // every payload below is captured with both of them watching. The reply is
    // not committed — it is the same `TableView` shape `unlocked` just asserted
    // — because what this request exists to produce is a NAME in the table
    // document that no payload may carry.
    let (status, quiet_knock) = api
        .call_as(
            Some(QUIET),
            "POST",
            "/api/games/wire/unlock",
            Some(json!({ "password": WIRE_PASSWORD })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "the silent watcher knocks: {quiet_knock}");
    // **And it is on the roster of the table THIS corpus is cut from**, read off
    // the store rather than inferred from the 200. Without this line the pin
    // below would search every payload for somebody who might not be watching at
    // all, and pass for that reason: its own partner proves the SEARCH can find
    // the string, and only this proves the WALK put it there.
    assert!(
        api.state
            .games
            .store()
            .read(yokohama_web::store::Doc::Table, "wire")
            .expect("a readable store")
            .expect("the walk's table document")
            .0
            .contains(QUIET),
        "the silent watcher's knock did not reach the walk's own table document",
    );

    // The lobby's THIRD list: every game under way, whoever made it (R-3.9),
    // read by the watcher who just got in.
    //
    // **ONE row, on purpose.** `wire4` below is a second playing table, and two
    // tables created in different seconds sort by recency while two created in
    // the same second sort by id — so a list taken after it exists would swap
    // its rows between runs. That is the hazard [`second_table_body`]'s doc
    // records from the other side, and the answer here is to take the list while
    // there is only one game under way. The `can_watch: false` face the client
    // renders on a row it may not open is on `table_watchable` instead.
    let games_playing = api.get_as(GREEN, "/api/games?scope=playing").await;
    assert_eq!(
        games_playing["games"].as_array().map(Vec::len),
        Some(1),
        "one game is under way: {games_playing}",
    );

    let created = api.get("/api/games/wire").await;

    let mut out: Vec<(String, Value)> = vec![
        ("table_dealing".to_string(), table_dealing),
        ("table".to_string(), table),
        // ---- M6.2: the open table, the two lists, and the four refusals ----
        ("table_open".to_string(), table_open),
        ("games_open".to_string(), games_open),
        ("games_next".to_string(), games_next),
        ("open_409".to_string(), open_409),
        ("join_403_locked".to_string(), join_403_locked),
        ("join_403_password".to_string(), join_403_password),
        ("join_409_taken".to_string(), join_409_taken),
        ("join_409_not_open".to_string(), join_409_not_open),
        // ---- M6.3: the watcher's four refusals-and-readings ----------------
        ("watch_403_locked".to_string(), watch_403_locked),
        ("table_watchable".to_string(), table_watchable),
        ("unlock_403".to_string(), unlock_403),
        ("unlock_409_open".to_string(), unlock_409_open),
        ("games_playing".to_string(), games_playing),
        ("deal".to_string(), deal),
        ("deal_waiting".to_string(), deal_waiting),
        ("dealing_409".to_string(), dealing_409),
        ("games".to_string(), games_list),
        ("not_seated_403".to_string(), not_seated_403),
        // ---- M6.4: the chat's own not-seated refusal, from the OPEN table ---
        ("chat_403_not_seated".to_string(), chat_403_not_seated),
        ("events_open".to_string(), events_open),
        ("events_dealing".to_string(), events_dealing),
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

    let mut state = as_actor(api, created).await;

    // ---- M6.4: the table's chat, BEFORE the first move --------------------
    //
    // The order is the whole reason this block is here and not after the loop
    // (R-plan-6): `events.json` is captured inside the walk, and its fifth field
    // is the table's newest message number. Taken with nothing said yet it would
    // read 0 — a constant a client could not tell from a stub — so the three
    // posts go first and the signal fixture carries a real count.
    //
    // Three authors, because the arms are what the client branches on: two
    // SEATED callers, whose names the table settles (R-6.4.2), and the WATCHER
    // who knocked a few requests ago, who carries the name they sent.
    let (status, chat_post) = api
        .call_as(
            Some(WALKER),
            "POST",
            "/api/games/wire/chat",
            Some(json!({ "text": "good luck" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{chat_post}");
    let (status, blue_post) = api
        .call_as(Some(OTHER), "POST", "/api/games/wire/chat", Some(json!({ "text": "you too" })))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{blue_post}");
    assert_eq!(blue_post["author"]["name"], "Blue", "the seat's stored name, from the join");
    let (status, chat_post_watcher) = api
        .call_as(
            Some(GREEN),
            "POST",
            "/api/games/wire/chat",
            Some(json!({ "text": "watching from the rail", "name": "Green" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{chat_post_watcher}");
    assert_eq!(chat_post_watcher["author"]["kind"], "watcher");

    // The page, and the same page from a cursor. `chat_after` is the delta the
    // client actually fetches once it is running — one message rather than
    // three — and a corpus with only the full page in it would leave that read
    // rendered by nothing.
    let chat = api.get("/api/games/wire/chat").await;
    assert_eq!(chat["last"], 3, "{chat}");
    let chat_after = api.get("/api/games/wire/chat?after=2").await;
    assert_eq!(chat_after["messages"].as_array().map(Vec::len), Some(1), "{chat_after}");
    assert_eq!(chat_after["last"], 3, "a cursor answers the DOCUMENT's last: {chat_after}");

    // The two 400s, which are two different things to fix: a message that is too
    // long, and a seated caller claiming a name their seat has already settled.
    let (status, chat_400_long) = api
        .call_as(
            Some(WALKER),
            "POST",
            "/api/games/wire/chat",
            Some(json!({ "text": "x".repeat(501) })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{chat_400_long}");
    let (status, chat_400_seated_name) = api
        .call_as(
            Some(WALKER),
            "POST",
            "/api/games/wire/chat",
            Some(json!({ "text": "hi", "name": "Not Red" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{chat_400_seated_name}");

    // The LOCK's refusal, from the one caller who has not knocked — the other
    // half of the pair whose first is `chat_403_not_seated` at the open table.
    // Same route, same status, a sentence that names something this reader can
    // do about it.
    let (status, chat_403_locked) =
        api.call_as(Some(STRANGER), "GET", "/api/games/wire/chat", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{chat_403_locked}");

    // **The house**: red posts, blue reads. Every author there is a watcher —
    // the house has no seats — so this is also the only `Watcher` author in the
    // corpus whose address is a PLAYER's, which is what keeps the client from
    // reading the arm as "somebody with no seat at this table".
    let (status, house_chat_post) = api
        .call_as(
            Some(WALKER),
            "POST",
            "/api/chat",
            Some(json!({ "text": "anyone up for a second table?", "name": "Red" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{house_chat_post}");
    let house_chat = api.get_as(OTHER, "/api/chat").await;
    assert_eq!(house_chat["last"], 1, "{house_chat}");

    for (name, v) in [
        ("chat", chat),
        ("chat_after", chat_after),
        ("chat_post", chat_post),
        ("chat_post_watcher", chat_post_watcher),
        ("chat_400_long", chat_400_long),
        ("chat_400_seated_name", chat_400_seated_name),
        ("chat_403_locked", chat_403_locked),
        ("house_chat", house_chat),
        ("house_chat_post", house_chat_post),
    ] {
        out.push((name.to_string(), v));
    }

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
            // The signal at a position with a real event count behind it.
            //
            // **Read by OTHER rather than by the mover**, which is the
            // whole point of the fixture: `your_turn` is the one field whose
            // value depends on who asked, and the client's poll exists to run
            // while it is somebody ELSE's turn. Captured at the opening
            // instead, `events` would be 0 and the count would be a constant
            // the client could not tell from a stub.
            if !seen.iter().any(|s| s == "events") {
                let e = api.get_as(OTHER, "/api/games/wire/events").await;
                capture(&mut out, &mut seen, "events", &e);
            }
        }

        // **The watcher's LIVE frame** (M6.3, R-3.7), taken at the first
        // position with an ASSISTANT actually on the board.
        //
        // The condition is not decoration. A spectator's payload is a picture
        // of a table nobody at it is playing, and what it has to show is the
        // table: taken at the first prompt with three rows — where `suggest`
        // and `events` are captured — the board still had no assistant and no
        // President on it, measured, so the fixture the client renders its
        // watching screen from would have been a picture of nothing. The board
        // is read off the PAYLOAD rather than counted here, which is the same
        // rule the placement arm below follows: the engine owns that fact.
        if !seen.iter().any(|s| s == "spectator")
            && state["frame"]["board"]["tiles"].as_array().is_some_and(|tiles| {
                tiles.iter().any(|t| {
                    t["assistants"]
                        .as_array()
                        .is_some_and(|rows| rows.iter().any(|r| r[1].as_u64().unwrap_or(0) > 0))
                })
            })
        {
            let w = api.get_as(GREEN, "/api/games/wire").await;
            assert!(w["seat"].is_null(), "a watcher holds no seat: {w}");
            capture(&mut out, &mut seen, "spectator", &w);
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

        // **The two refusals a seated caller meets by acting out of turn**
        // (spec §8). Both are the plain `ApiErrorBody` the client already
        // renders, so what these files pin is the WORDS and the status rather
        // than a new shape — and the words are the whole point: "403" alone
        // tells a player nothing about what to do next.
        //
        // Taken once the walk has crossed a TURN BOUNDARY, which is what makes
        // them real: until seat 1 is to move, `WALKER` holds the acting seat
        // and neither refusal exists. `events > 0` is the same condition read
        // from the other side — `undo_allowed` answers "nothing to undo" on an
        // empty log, which is a different sentence about a different thing.
        //
        // **The two sentences differ because the two checks are ORDERED**
        // (R-5.1): `undo` is refused by R-P6's rule, which runs in FRONT of the
        // acting-seat fence, so the player hears about the button they pressed;
        // every other line falls through to the fence, which names whose turn
        // it is. A server that dropped the ordering would answer both of these
        // with one sentence, and these two files are what would say so.
        //
        // Neither request mutates: both are refused before `Repl::step`.
        if state["to_move"].as_u64() == Some(1)
            && state["events"].as_u64().unwrap_or(0) > 0
            && !seen.iter().any(|s| s == "wrong_seat_403")
        {
            seen.push("wrong_seat_403".to_string());
            seen.push("undo_refused_403".to_string());
            let (status, wrong_seat) = api
                .call_as(Some(WALKER), "POST", "/api/games/wire/input", Some(json!({ "line": "1" })))
                .await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{wrong_seat}");
            out.push(("wrong_seat_403".to_string(), wrong_seat));

            let (status, undo_refused) = api
                .call_as(
                    Some(WALKER),
                    "POST",
                    "/api/games/wire/input",
                    Some(json!({ "line": "undo" })),
                )
                .await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{undo_refused}");
            out.push(("undo_refused_403".to_string(), undo_refused));
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

            // **The record route while the game RUNS** (M6.5a, R-6.5a.14): the
            // 409 the client would render if it asked early. The 200 body is
            // NEVER committed — it is the record, seed and all; the seed pin
            // reads it as its second anti-vacuity partner instead.
            let (status, record_409) = api.call("GET", "/api/games/wire/record", None).await;
            assert_eq!(status, StatusCode::CONFLICT, "{record_409}");
            out.push(("record_409".to_string(), record_409));
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
        state = as_actor(api, api.send_as(mover, &line).await).await;
    }

    capture(&mut out, &mut seen, "game_over", &state);

    // **The same finished game, read by the WATCHER.** A spectator's payload at
    // game over is the one place the client has to draw a standing for somebody
    // with no seat in it: `scores` is present, `seat` is null, and the prompt is
    // `game_over` rather than `watching` — which is a different branch from the
    // live spectator frame captured in the loop above.
    let spectator_over = api.get_as(GREEN, "/api/games/wire").await;
    assert!(spectator_over["seat"].is_null(), "a watcher holds no seat: {spectator_over}");
    assert!(!spectator_over["scores"].is_null(), "a finished game reports its standing");
    out.push(("spectator_over".to_string(), spectator_over));

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
        "wrong_seat_403",
        "undo_refused_403",
        "spectator",
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
                "id": "wire4",
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

    // **The same stop as `at_k`, as a RUN of three ending there** (M6.5b,
    // R-6.5b.10): `[k-3, k)`, so the run's last frame is the position BEFORE
    // `at_k`'s and the three are real positions this game left behind. And the
    // two refusals the client renders — the cap as a 400 (never a clamp), the
    // range as `at/{k}`'s own 404 extended to name the run. The range probe
    // starts at `total - 1`: the cap is checked FIRST, so a span from zero over
    // this game's whole record would read the cap's sentence instead.
    out.push((
        "run".to_string(),
        api.get(&format!("/api/games/wire/at?from={}&to={}", total / 2 - 3, total / 2)).await,
    ));
    let (status, cap) = api.call("GET", "/api/games/wire/at?from=0&to=65", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{cap}");
    out.push(("run_400_cap".to_string(), cap));
    let (status, range) = api
        .call("GET", &format!("/api/games/wire/at?from={}&to={}", total - 1, total + 2), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{range}");
    out.push(("run_404_range".to_string(), range));

    // **The same position, replayed by the WATCHER** (M6.3, R-3.7). The SAME
    // `k` as the fixture above, deliberately: the pair then differs in exactly
    // one thing — who asked — so what the second one is missing is the mask and
    // not the position. A spectator's sweep over `0..=total` harvests nothing at
    // any stop, and `seat: null` is how the payload says so.
    let spectator_at_k =
        api.get_as(GREEN, &format!("/api/games/wire/at/{}", total / 2)).await;
    assert!(spectator_at_k["seat"].is_null(), "a watcher holds no seat: {spectator_at_k}");
    assert_eq!(
        spectator_at_k["index"], total / 2,
        "the watcher's replay is the same stop as the seat's, or the pair is not a pair",
    );
    out.push(("spectator_at_k".to_string(), spectator_at_k));

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

    // **What a deployment with the house chat OFF answers** (M6.4, R-6.4.6),
    // from a SECOND state, because the switch is read once at startup and
    // nothing over the wire can flip it.
    //
    // A 404 and not a 403: with the lobby chat off the route is not something
    // this deployment refuses you, it is something this deployment does not
    // have. The client reads the same fact off `/api/health` and draws no panel
    // at all — this is the sentence for whoever asks anyway, and the payload
    // that says the answer is an ordinary `ApiErrorBody` rather than an empty
    // page a client would render as a quiet room.
    let off = Api::with_chat(yokohama_web::chat::ChatConfig {
        lobby: false,
        ..yokohama_web::chat::ChatConfig::default()
    });
    let (status, house_chat_off_404) = off.call("GET", "/api/chat", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{house_chat_off_404}");
    out.push(("house_chat_off_404".to_string(), house_chat_off_404));

    out.extend(archive_payloads(api).await);

    out
}

/// **What the archive route answers a browser** (M6.5b, R-6.5b.6), captured
/// from a THIRD `Api` whose archive is a real directory.
///
/// The record and the table document are this walk's own — read off the primary
/// state's store, which is where a finished game's bytes are — so the body is a
/// real pair and not a construction, and the path in the reply is a function of
/// the walk's id and the walk's caller: `browser/red_at_wire.test/rev7/over/wire.json`.
async fn archive_payloads(api: &Api) -> Vec<(String, Value)> {
    use yokohama_web::store::Doc;
    let store = api.state.games.store();
    let (record_text, _) = store.read(Doc::Record, "wire").expect("a store").expect("the record");
    let (table_text, _) = store.read(Doc::Table, "wire").expect("a store").expect("the table");

    let dir = archive_tempdir();
    let archived = Api::with_archive(&dir);
    let body = |r: &str, t: &str| Some(json!({ "record": r, "table": t }));
    let mut out = Vec::new();

    let (status, archive_post) = archived.call("POST", "/api/archive", body(&record_text, &table_text)).await;
    assert_eq!(status, StatusCode::CREATED, "{archive_post}");
    assert_eq!(
        archive_post["path"], "browser/red_at_wire.test/rev7/over/wire.json",
        "the caller prefix is the ENCODED address (R-6.5b.15)"
    );

    // The same bytes again: the retry, answered 200 `identical` rather than a
    // 409, which is what makes a lost reply safe for a client to act on.
    let (status, archive_identical) =
        archived.call("POST", "/api/archive", body(&record_text, &table_text)).await;
    assert_eq!(status, StatusCode::OK, "{archive_identical}");

    // A DIFFERENT record under the same id. `engine_version` is provenance only
    // and `replay_strict` never reads it, so this record replays, is still over,
    // and therefore collides at the very same path — which is what a conflict is.
    let (status, archive_409) = archived
        .call("POST", "/api/archive", body(&with_another_engine_version(&record_text), &table_text))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{archive_409}");

    // A PARTIAL record — the Abandon-and-save shape — under `abandoned/`, the
    // leaf the REPLAY shows and not the one the body claims.
    let (status, archive_post_abandoned) =
        archived.call("POST", "/api/archive", body(&partial_of(&record_text, 5), &table_text)).await;
    assert_eq!(status, StatusCode::CREATED, "{archive_post_abandoned}");
    assert_eq!(archive_post_abandoned["leaf"], "abandoned");

    // The four refusals the client's error strip renders. (The fifth — a record
    // with no `semantics_rev` at all — is `replay_strict`'s own sentence carried
    // verbatim, and is pinned in the layer's cells rather than committed here.)
    let refusals = [
        ("archive_400_rev", edited_record(&record_text, |o| { o.insert("semantics_rev".into(), 6.into()); }), table_text.clone()),
        ("archive_400_digest", edited_record(&record_text, |o| { o.remove("terminal_digest").expect("a digest"); }), table_text.clone()),
        ("archive_400_replay", edited_record(&record_text, |o| {
            let events = o.get_mut("events").expect("events").as_array_mut().expect("a list");
            assert!(events.len() > 1, "anti-vacuity: a one-event record cannot be re-ordered");
            events[0] = events[1].clone();
        }), table_text.clone()),
        // `players` cannot carry this one: `check_shape` refuses a count that
        // disagrees with the seats, which is a DIFFERENT sentence. The start
        // seat is in range at a 2-seat table and wrong against the record.
        ("archive_400_table", record_text.clone(), edited_record(&table_text, |o| { o.insert("start_seat".into(), 1.into()); })),
    ];
    for (name, r, t) in refusals {
        let (status, v) = archived.call("POST", "/api/archive", body(&r, &t)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{name}: {v}");
        out.push((name.to_string(), v));
    }

    out.extend([
        ("archive_post".to_string(), archive_post),
        ("archive_post_identical".to_string(), archive_identical),
        ("archive_post_abandoned".to_string(), archive_post_abandoned),
        ("archive_409".to_string(), archive_409),
    ]);
    std::fs::remove_dir_all(&dir).ok();
    out
}

/// A directory of this run's own. `walk_with` is driven by several tests, and
/// each must meet an EMPTY archive or the first send is the once-rule's 200
/// instead of the 201 the fixture is.
fn archive_tempdir() -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "yokohama_wire_archive_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temp directory");
    dir
}

/// One key of a JSON document changed, everything else left exactly as it was.
fn edited_record(text: &str, f: impl FnOnce(&mut serde_json::Map<String, Value>)) -> String {
    let mut v: Value = serde_json::from_str(text).expect("the document is JSON");
    f(v.as_object_mut().expect("an object"));
    v.to_string()
}

/// A record that REPLAYS, is still over, and differs in bytes — the conflict
/// case. See [`archive_payloads`] for why it is this field and not an event.
fn with_another_engine_version(record: &str) -> String {
    edited_record(record, |o| {
        o.insert("engine_version".into(), "0.0.0-elsewhere".into());
    })
}

/// The first `events` moves of a record, re-stamped so it carries the digest of
/// the state it really folds to — a game abandoned part way through.
fn partial_of(record_text: &str, events: usize) -> String {
    use yokohama_core::{state_at, GameRecord};
    let mut rec: GameRecord = serde_json::from_str(record_text).expect("a record");
    assert!(rec.events.len() > events, "anti-vacuity: the partial must be shorter");
    rec.events.truncate(events);
    let state = state_at(&rec, events, &Rules::standard()).expect("the partial replays");
    rec.stamp_terminal_digest(&state);
    serde_json::to_string(&rec).expect("a record")
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
    use yokohama_web::client::{ClientState, Viewer};

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
        serde_json::to_value(ClientState::of("wire", &watching, Viewer::Seat(PlayerId::new(1)), Vec::new()))
            .expect("a serializable payload");

    let mut locked = build(vec![None, None]);
    locked
        .session
        .lock_down("the engine reported an invariant violation applying that move".to_string());
    let locked =
        serde_json::to_value(ClientState::of("wire", &locked, Viewer::Seat(PlayerId::new(0)), Vec::new()))
            .expect("a serializable payload");

    // ---- M3: the explanation surface ----------------------------------------
    //
    // These are built HERE and not in the walk, for the reason the `watching`
    // fixture is: the last keep's `finalize` draws the bot seed from OS
    // entropy, so a golden over a router-created bot game would churn on every
    // run. With a fixed `BOT_SEED` and a fixed `SETUP_SEED` the account below is
    // the same account every time, which is what a committed payload has to be.

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

/// **No payload spells the setup seed. There is no longer an exception.**
///
/// The seed is the FIRST member of the engine's hidden set (`game/observe.rs`):
/// it regenerates the order deck, the tech deck, the boxed pile, the board AND
/// every deal. `tables::draw_seed`'s own doc states the threat model — the public
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
/// committed payload for the value itself, in the spellings JSON can hold it in.
///
/// **The one exemption is gone, and that is what this task is** (M6.1 §8).
/// `POST /api/games/preview` was the last surface that carried the seed: it
/// drew one, dealt from it, and the client sent the value back into `create`,
/// so the round trip could not have worked without it. The setup flow is
/// create-then-deal now — the table document holds the seed on the SERVER and
/// no client ever sees it — so this reads every committed payload with no list
/// of exceptions at all.
///
/// **And an exemption-free search needs an anti-vacuity partner**, because a
/// search for a value that is nowhere passes exactly as loudly as a search that
/// is looking for the wrong thing. The partner is below: the same string, found
/// in the table document the store holds for a table opened from the same body.
///
/// It covers the DECIMAL form, which is the one this codebase writes; the hex
/// arm guards a spelling nothing produces today and is a fence rather than a
/// measurement, which is said here rather than left to be inferred.
#[tokio::test]
async fn no_committed_payload_carries_the_setup_seed() {
    let decimal = SETUP_SEED.to_string();
    // The two spellings a number reaches JSON in — `WireSeed` serializes to a
    // string and `TitleModel` used to serialize to a number, so a fence that
    // knew only one would have missed the other — and the HEX form beside them,
    // which is how an id or a key has spelled a seed on this wire before:
    // default game ids were `sim-{seed:016x}` until they were drawn
    // independently, and a `GET /api/games` then handed the hidden set to
    // anyone who could list.
    let hex = format!("{SETUP_SEED:016x}");
    let mut offenders: Vec<String> = Vec::new();

    for (name, value) in corpus().await {
        let text = serde_json::to_string(&value).expect("serializable");
        if text.contains(&decimal) || text.contains(&hex) {
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

    // **The anti-vacuity partner: the search cannot pass by searching for
    // nothing.** The same seed IS in the table document the store holds for a
    // table opened from the walk's own body — so the string above is a string
    // this codebase really does write down somewhere, and `contains` really
    // does find it. Without this the whole test passes just as well against a
    // seed no route was ever given.
    let api = Api::new();
    let (status, created) = api.call("POST", "/api/games", Some(new_game_body())).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let (doc, _) = api
        .state
        .games
        .store()
        .read(yokohama_web::store::Doc::Table, "wire")
        .expect("a readable store")
        .expect("the table document the create wrote");
    assert!(
        doc.contains(&decimal),
        "the seed is in the table document and nowhere on the wire — this one is not, so the \
         search above was looking for a string nothing writes",
    );
    // And the create's own reply, which IS on the wire, is not.
    assert!(
        !serde_json::to_string(&created).expect("serializable").contains(&decimal),
        "the create that wrote that document answered with the seed in it",
    );

    // **The SECOND anti-vacuity partner** (M6.5a, R-6.5a.14): the record route's
    // 200 body DOES carry the seed — it IS the record — and it is never
    // committed. A finished game is needed, and the walk's own is one at its
    // end; so the walk runs once more here, on its own state.
    let api = Api::new();
    let _ = walk_with(&api).await;
    let (status, record) = api.call("GET", "/api/games/wire/record", None).await;
    assert_eq!(status, StatusCode::OK, "{record}");
    assert!(
        serde_json::to_string(&record).expect("serializable").contains(&decimal),
        "the record carries the seed; the search above would otherwise be looking for nothing",
    );
    assert!(
        !wire_dir().join("record.json").exists(),
        "a record fixture would put the seed in the corpus",
    );
}

/// **No payload carries the table password, its salt or its hash** (M6.2 §5.7).
///
/// The sibling fence above is about the setup seed; this is the same shape of
/// claim about the second secret a table document now holds. The threat is
/// plainer and the reach is wider: a salt and a hash in a browser are a password
/// to attack OFFLINE, at whatever rate the attacker's hardware allows and with
/// nothing on this server able to notice, and people reuse passwords. So
/// `TableDoc.lock` never leaves the document and `TableView` carries the boolean
/// `locked` in its place.
///
/// **Three searches, over two different runs, because the values are not the
/// same kind of thing.**
///
/// The PASSWORDS — the table's own and the wrong one the walk attempts — are
/// constants of this file, so both are searched for across the whole committed
/// corpus: every payload of a full walk, including the four join refusals, where
/// a server that echoed either back would be found. **Two literals and not one,
/// because a refusal quotes the ATTEMPT and never the table's password**; the
/// one-literal version of this test could not see the leak it was written to
/// catch, which a review found and [`WIRE_WRONG_PASSWORD`] records.
///
/// The SALT and the HASH are drawn per table, so a corpus produced by one `Api`
/// says nothing about them: this opens a SECOND table and searches THAT api's
/// own answers for THAT table's values. Three surfaces, and they are the three a
/// lock can reach — the create's own reply, the table view, and a lobby row
/// (`view()` builds all three, so a field added there would ride on each).
///
/// **The anti-vacuity partner is the document.** A search for a value that is
/// nowhere passes exactly as loudly as a search looking for the wrong thing, so
/// the salt and the hash are READ OUT of the stored document before they are
/// searched for — they are strings this codebase really does write down, and
/// their lengths are pinned (16 bytes of salt and a SHA-256, in hex) so a lock
/// that silently became empty would fail here rather than make every search
/// trivially pass. **Neither password has such a partner and neither can have
/// one**: nothing stores either of them, which is the property — so those two
/// searches rest on the salt and the hash having proved the mechanism finds what
/// is there, and on the mutations recorded in the task report.
#[tokio::test]
async fn no_payload_of_this_run_carries_the_password_or_its_hash() {
    // This run's payloads, taken before the golden compares them — the same
    // corpus the committed files are cut from.
    let run = corpus().await;

    let api = Api::new();
    let (status, created) = api.call("POST", "/api/games", Some(new_game_body())).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let (doc, _) = api
        .state
        .games
        .store()
        .read(yokohama_web::store::Doc::Table, "wire")
        .expect("a readable store")
        .expect("the table document the create wrote");
    let doc: Value = serde_json::from_str(&doc).expect("the table document is JSON");
    let salt = doc["lock"]["salt"].as_str().expect("the document carries the salt").to_string();
    let hash = doc["lock"]["hash"].as_str().expect("the document carries the hash").to_string();
    assert_eq!(salt.len(), 32, "16 random bytes, in hex");
    assert_eq!(hash.len(), 64, "a SHA-256, in hex");

    let mut offenders: Vec<String> = Vec::new();
    for (name, value) in &run {
        let text = serde_json::to_string(value).expect("serializable");
        // **BOTH literals, and the second is the one a review had to add.** A
        // refusal that quotes the attempt back leaks what the caller TYPED, not
        // what the table holds — and people type their other passwords into
        // these boxes. See [`WIRE_WRONG_PASSWORD`].
        for (what, needle) in
            [("the password", WIRE_PASSWORD), ("a password ATTEMPT", WIRE_WRONG_PASSWORD)]
        {
            if text.contains(needle) {
                offenders.push(format!("{name} carries {what}"));
            }
        }
    }
    for (name, uri) in [
        ("the create's own reply", None),
        ("the table view", Some("/api/games/wire/table")),
        ("an open lobby row", Some("/api/games?scope=open")),
    ] {
        let value = match uri {
            None => created.clone(),
            Some(uri) => api.get(uri).await,
        };
        let text = serde_json::to_string(&value).expect("serializable");
        for (what, needle) in [
            ("the password", WIRE_PASSWORD),
            ("a password ATTEMPT", WIRE_WRONG_PASSWORD),
            ("the salt", &salt),
            ("the hash", &hash),
        ] {
            if text.contains(needle) {
                offenders.push(format!("{name} carries {what}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "a table password reached the client: {}\n\
         The salt and the hash are an offline attack on a password people reuse, and the \
         password itself is the thing this server was trusted not to repeat. `locked` is the \
         boolean that crosses instead; if a new field genuinely needs more, it needs an owner \
         decision first, not an exemption here.",
        offenders.join(", "),
    );
}

/// **A silent watcher is named NOWHERE, and a posting one only as the author of
/// what they posted** (M6.4, R-6.4.12; narrowing M6.3's pin).
///
/// The third secret a table document holds, after the seed and the lock, is
/// `unlocked` — the list of people who have presented this table's password.
/// It is not a game secret and it is somebody's business all the same: it is
/// the roster of who is looking over the players' shoulders, and the players
/// did not invite them. `TableView` carries the one boolean the caller needs
/// about THEMSELVES (`can_watch`) and the list itself never crosses (R-3.5).
///
/// **M6.3 stated that as "no payload names a watcher", and M6.4 makes that
/// sentence false in one place on purpose.** A watcher may now speak, and a
/// message is signed — so [`GREEN`]'s address is legitimately on the wire, as
/// `author.user` of the one message green sent. The narrowing is not "green is
/// exempt": it is a PATH rule, checked per hit. An address may appear at
/// `<…>.author.user` beside `kind: "watcher"`, and nowhere else, and only for
/// the identity that actually posted.
///
/// **Which is why there are three needles and not two.** The claim M6.3 made
/// needs a witness that still has no reason to be on the wire at all, so
/// [`QUIET`] knocks at the same table, is written into the same `unlocked` list,
/// and then says nothing for the rest of the walk: every payload after that
/// point is built while it is watching, and none of them may spell it. [`GREEN`]
/// exercises the allowance. [`STRANGER`] rides along as the second realistic
/// leak on these routes — a view or a refusal that echoed its READER's address
/// back, a shape no other test would see, since every other payload is read by
/// somebody who is named in it anyway.
///
/// **The search walks the tree rather than the serialized text**, because the
/// allowance is about WHERE a hit is and a text search has no position. It is
/// no weaker for it: a string VALUE containing the address anywhere inside it is
/// a hit, exactly as `contains` was, and an object KEY containing it is a hit
/// too — which the text search also caught and a naive walk would not.
#[tokio::test]
async fn a_silent_watcher_is_named_nowhere_and_a_posting_one_only_as_an_author() {
    /// Every path at which `needle` appears — inside a string value, or as an
    /// object key. The path is the steps taken to reach it, array indices
    /// included, so the caller can read the hit's SIBLINGS back out.
    fn find(v: &Value, path: &mut Vec<String>, needle: &str, hits: &mut Vec<Vec<String>>) {
        match v {
            Value::String(s) if s.contains(needle) => hits.push(path.clone()),
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    path.push(i.to_string());
                    find(item, path, needle, hits);
                    path.pop();
                }
            }
            Value::Object(map) => {
                for (k, item) in map {
                    path.push(k.clone());
                    // A KEY spelling the address is a hit at the key's own path.
                    // It can never be `allowed` — the rule requires the last step
                    // to be `user` — which is the answer a map keyed by email
                    // should get.
                    if k.contains(needle) {
                        hits.push(path.clone());
                    }
                    find(item, path, needle, hits);
                    path.pop();
                }
            }
            _ => {}
        }
    }

    /// The value at a path [`find`] produced, so a hit can be checked against
    /// its siblings. Every step exists by construction — the path was walked
    /// out of this very value.
    fn at_path<'a>(v: &'a Value, path: &[String]) -> &'a Value {
        let mut here = v;
        for step in path {
            here = match here {
                Value::Array(items) => {
                    let i: usize = step.parse().expect("an array step is an index");
                    &items[i]
                }
                _ => &here[step.as_str()],
            };
        }
        here
    }

    let mut offenders: Vec<String> = Vec::new();
    let mut allowed_hits = 0usize;
    for (name, value) in corpus().await {
        for (who, needle, may_post) in [
            ("the watcher", GREEN, true),
            ("a watcher who never spoke", QUIET, false),
            ("a caller who never knocked", STRANGER, false),
        ] {
            let mut hits: Vec<Vec<String>> = Vec::new();
            find(&value, &mut Vec::new(), needle, &mut hits);
            for path in hits {
                let allowed = may_post
                    && path.last().map(String::as_str) == Some("user")
                    && path.len() >= 2
                    && path[path.len() - 2] == "author";
                if allowed {
                    // **The sibling `kind` must say watcher.** A seat author
                    // carrying this address would be a seat green does not hold
                    // — a different and worse leak than the one the allowance
                    // covers, and one that would otherwise pass on its path
                    // alone.
                    let author = at_path(&value, &path[..path.len() - 1]);
                    assert_eq!(
                        author["kind"],
                        "watcher",
                        "{name} names {who} as a {} author at {} — the allowance is for the \
                         author of a message a watcher sent, not for a seat",
                        author["kind"],
                        path.join("."),
                    );
                    allowed_hits += 1;
                } else {
                    offenders.push(format!("{name} names {who} at {}", path.join(".")));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a watcher is named where they did not speak: {}\n\
         `unlocked` is the roster of everyone looking over these players' shoulders, and \
         `can_watch` — one boolean, about the reader — is what crosses instead. The ONE \
         allowance is `author.user` beside `kind: \"watcher\"`, on a message that watcher \
         actually sent.",
        offenders.join(", "),
    );

    // **Anti-vacuity, first half: the allowance is EXERCISED.** A path rule that
    // nothing in the corpus ever satisfies is a rule that has not been read —
    // green posted one message, and it is served back on the page, on the page
    // from a cursor, and as the post's own reply.
    assert!(
        allowed_hits >= 2,
        "green's post should appear at `author.user` in more than one payload; found \
         {allowed_hits} — either the chat fixtures stopped carrying an author, or the \
         allowance above is describing a shape this corpus does not have",
    );

    // **The anti-vacuity partner: the search cannot pass by searching for
    // nothing.** The same shape as the seed pin's — the string IS written down,
    // in the table document, and `contains` really does find it there. Without
    // this the assertion above passes just as well against an address no route
    // was ever given.
    //
    // A table of two BOUND seats rather than the walk's open one, because what
    // this needs is a PLAYING table in as few requests as possible: `unlock`
    // refuses at `open` and at `dealing`, and there is nothing to watch at
    // either.
    let api = Api::new();
    let (status, created) = api
        .call(
            "POST",
            "/api/games",
            Some(json!({
                "players": 2,
                "colors": ["Red", "Blue"],
                "start_seat": 0,
                "seed": SETUP_SEED,
                "seats": [{ "user": WALKER }, { "user": OTHER }],
                "id": "watched",
                "password": WIRE_PASSWORD,
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    for (seat, who) in [(0, WALKER), (1, OTHER)] {
        let (status, kept) = api
            .call_as(
                Some(who),
                "POST",
                "/api/games/watched/keep",
                Some(json!({ "seat": seat, "card": 1 })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "seat {seat}: {kept}");
    }
    let (status, unlocked) = api
        .call_as(
            Some(GREEN),
            "POST",
            "/api/games/watched/unlock",
            Some(json!({ "password": WIRE_PASSWORD })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{unlocked}");

    let (doc, _) = api
        .state
        .games
        .store()
        .read(yokohama_web::store::Doc::Table, "watched")
        .expect("a readable store")
        .expect("the table document the unlock wrote");
    assert!(
        doc.contains(GREEN),
        "the unlock did not write the watcher down — so the search above was looking for a \
         string nothing in this system records, and would pass however the wire behaved",
    );
    // And the unlock's own reply, which IS on the wire, does not name them.
    assert!(
        !serde_json::to_string(&unlocked).expect("serializable").contains(GREEN),
        "the request that added a watcher to the roster answered with the roster in it",
    );

    // **Anti-vacuity for the SILENT watcher, which is the needle the whole
    // narrowing rests on** (M6.4). Green's absence is no longer the claim —
    // green is allowed to appear — so the partner above cannot carry this half.
    //
    // The same table, knocked at a second time, by the address the corpus search
    // must find nothing of. It is replayed HERE rather than read off the walk's
    // own store, which is unreachable from this test: the pin owns its premise
    // or it has none.
    let (status, quiet_reply) = api
        .call_as(
            Some(QUIET),
            "POST",
            "/api/games/watched/unlock",
            Some(json!({ "password": WIRE_PASSWORD })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{quiet_reply}");
    let (doc, _) = api
        .state
        .games
        .store()
        .read(yokohama_web::store::Doc::Table, "watched")
        .expect("a readable store")
        .expect("the table document the unlock wrote");
    assert!(
        doc.contains(QUIET),
        "the silent watcher's knock did not write them down — so the search above was looking \
         for a string nothing in this system records, and would pass however the wire behaved",
    );
    assert!(
        !serde_json::to_string(&quiet_reply).expect("serializable").contains(QUIET),
        "the request that added a watcher to the roster answered with the roster in it",
    );
}

/// **The gate: what the server emits is what the client type-checks against.**
/// **The stream's first frame IS the `events` route's reply, compared by
/// VALUE** (M6.3 §3.1, spec §8).
///
/// The two transports have to answer the same object or the client's compare
/// is against two different things — and "the same object" is a claim the
/// crate's own cells can make only about the function both call. This one makes
/// it about the WIRE: one router, one table, one frame read out of a
/// `text/event-stream` body and one JSON reply, parsed and compared as values.
///
/// **By value and never by bytes.** The framing differs by construction — the
/// stream writes `event: signal` and a compact one-line `data:`, the route
/// writes a pretty-printed body — so a byte compare would be asserting that
/// two different serializers agree about whitespace. What must agree is the
/// object.
///
/// It also reads the frame's SHAPE off the wire, which is where §3.1's
/// formatting rules are observable at all: exactly one `data:` line (the JSON
/// is one line, so a payload that grew a newline would split into two and a
/// browser would join them with one), and no `id:` or `retry:` line (R-3.14).
#[tokio::test]
async fn the_stream_s_first_frame_is_the_events_route_s_reply_by_value() {
    let api = Api::new();
    let (status, table) = api
        .call(
            "POST",
            "/api/games",
            Some(json!({
                "players": 2,
                "colors": ["Red", "Blue"],
                "start_seat": 0,
                "seed": SETUP_SEED,
                "seats": [{ "user": WALKER }, { "user": WALKER }],
                "id": "streamed",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{table}");
    for seat in 0..2 {
        let (status, kept) = api
            .call("POST", "/api/games/streamed/keep", Some(json!({ "seat": seat, "card": 1 })))
            .await;
        assert_eq!(status, StatusCode::OK, "seat {seat}: {kept}");
    }

    let res = api
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/games/streamed/stream")
                .header("x-yokohama-claimed-user", WALKER)
                .header("accept", "text/event-stream")
                .body(Body::empty())
                .expect("a GET"),
        )
        .await
        .expect("the router answers");
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("content-type").and_then(|v| v.to_str().ok()),
        Some("text/event-stream"),
    );

    // Read only as far as the first frame's terminator, then DROP the body:
    // this stream is open until the table ends, so a read to completion would
    // never return.
    use tokio_stream::StreamExt;
    let mut body = res.into_body().into_data_stream();
    let mut text = String::new();
    while !text.contains("\n\n") {
        let chunk = tokio::time::timeout(std::time::Duration::from_secs(5), body.next())
            .await
            .expect("the first frame arrives")
            .expect("the stream is open")
            .expect("a chunk");
        text.push_str(&String::from_utf8_lossy(&chunk));
    }
    let frame = text.split("\n\n").next().expect("a first frame");
    drop(body);

    assert!(frame.starts_with("event: signal\n"), "{frame:?}");
    let data: Vec<&str> = frame.lines().filter_map(|l| l.strip_prefix("data: ")).collect();
    assert_eq!(data.len(), 1, "the JSON is ONE line: {frame:?}");
    assert!(
        !frame.lines().any(|l| l.starts_with("id:") || l.starts_with("retry:")),
        "R-3.14: {frame:?}",
    );

    let framed: Value = serde_json::from_str(data[0]).expect("one line of JSON");
    let polled = api.get("/api/games/streamed/events").await;
    assert_eq!(framed, polled, "the stream and the route answer the same object");
}

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

/// R-6.5a.14: the layer's dispatcher answers what the router answers, for every
/// committed non-transport READ and REFUSAL. **What that pins is the axum
/// ADAPTER's conversion** — both sides of every pair are the same `dispatch`
/// behind the same layer, so a change inside `api.rs` moves both and is caught
/// by the corpus compare and the route cells instead (spec §6.3 mutation 4's
/// real catchers). The comparison itself runs inside [`Api::call_as`] at capture
/// time (SDD ruling P-1); this cell walks the corpus and asserts how many pairs
/// were compared, so a `call_as` that stopped comparing fails here rather than
/// going quiet.
#[tokio::test]
async fn the_dispatcher_answers_what_the_router_answers() {
    let api = Api::new();
    let payloads = walk_with(&api).await;
    let compared = api.compared.get();
    // Measured 2026-09-20 at the M6.5a Task 6 head: 234 pairs over a walk of 74
    // payloads (every read and refusal, some payloads several reads), all equal.
    // The floor is nine tenths of that reading, so a walk that grows keeps
    // passing and one that lost a tenth of its reads does not.
    const FLOOR: usize = 210;
    assert!(
        compared >= FLOOR,
        "the walk compared only {compared} calls through dispatch (over {} payloads) — the floor is {FLOOR}",
        payloads.len()
    );
}

/// **Every committed message carries the FROZEN clock, which is what proves the
/// field is spelled right** (M6.4).
///
/// [`freeze_clock`] pins a clock by KEY NAME, and `chat::Message` names its
/// stamp `created_at` for exactly that reason — a field called `at` would be a
/// wall clock inside a committed golden, rewriting six files on every
/// regeneration and failing the very next run with nothing having changed. That
/// is a property of a NAME, so nothing about the type can hold it: this reads
/// the files.
///
/// **Its anti-vacuity is the count.** A sweep that found no stamps at all would
/// pass exactly as loudly, so the number of clocks read is asserted — EIGHT,
/// which is the page's three, the one message a cursor leaves, the two posts'
/// own replies and the house's pair. A regeneration that emptied the chat
/// corpus fails here rather than going quiet, and so does one that stopped
/// serving a message's author its own page.
#[test]
fn every_chat_message_carries_the_frozen_clock() {
    assert!(
        std::env::var_os("UPDATE_WIRE_FIXTURES").is_none(),
        "regeneration run — re-run WITHOUT UPDATE_WIRE_FIXTURES to check the corpus",
    );

    fn stamps(v: &Value, found: &mut Vec<u64>) {
        match v {
            Value::Object(map) => {
                for (k, item) in map {
                    if k == "created_at" {
                        found.push(item.as_u64().unwrap_or_else(|| {
                            panic!("a message's clock is a unix timestamp: {item}")
                        }));
                    } else {
                        stamps(item, found);
                    }
                }
            }
            Value::Array(items) => items.iter().for_each(|i| stamps(i, found)),
            _ => {}
        }
    }

    let dir = wire_dir();
    let mut found: Vec<u64> = Vec::new();
    for name in
        ["chat", "chat_after", "chat_post", "chat_post_watcher", "house_chat", "house_chat_post"]
    {
        let path = dir.join(format!("{name}.json"));
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "{}: {e} — generate the corpus first with \
                 `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`",
                path.display()
            )
        });
        let before = found.len();
        stamps(&serde_json::from_str::<Value>(&text).expect("a committed fixture is JSON"), &mut found);
        assert!(
            found.len() > before,
            "{name}.json carries no `created_at` at all — a message whose stamp is spelled \
             something else is a wall clock inside a committed golden",
        );
    }

    assert_eq!(found.len(), 8, "one clock per committed message: {found:?}");
    for seen in &found {
        assert_eq!(
            *seen, FROZEN_CREATED_AT,
            "a committed message carries a live clock: the freezer pins `created_at` by key \
             name, so this reads as a field spelled something the walk does not normalize",
        );
    }
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
