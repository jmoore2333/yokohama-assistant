//! **The gate: a whole 2-player game played to a winner, over the API. Twice.**
//!
//! This is the web analogue of `yokohama-cli`'s `piped_dogfood`, and it exists
//! for the same reason: a hundred unit tests that each prove one handler
//! answers correctly still do not prove the surface can be PLAYED. Every phase,
//! every queued choice, every collapsed family and every sub-picker has to be
//! reachable and answerable through JSON alone, and the only way to know that
//! is to finish a game without ever touching the terminal.
//!
//! It drives the API the way the dogfood test drives the binary — a seeded
//! RANDOM choice among whatever the prompt offers, which is what makes it
//! explore rather than walk one path — but it reads a typed `PromptState`
//! instead of scraping a screen, so it asserts on structure rather than words.
//!
//! **Two arms, and the second is what M6.1 is for.** The first is one person
//! against the shipped bot: every request under one identity, which is what
//! the surface was until this cycle. The second is TWO PEOPLE — two claimed
//! identities, each request under its own header, alternating with the turn —
//! and on every reply it reads the one thing a single-identity game cannot
//! test at all: that the payload is built for its CALLER, and that the caller's
//! own hand is the only one that is not masked. Both play by the same policy
//! ([`choose`]), so the second arm is the first one with the seats told apart
//! rather than a different walk that happens to agree.
//!
//! **Both arms have the same two guards since M6.5c2**: [`probe_a_placement`]
//! and [`Stuck`]. The vs-bot arm had neither, placed nothing, and livelocked
//! (M6.3 close §6.3). It prints the bot's stream seed, and
//! `YOKOHAMA_WEB_GATE_BOT_SEED=<u64>` pins that seed to reproduce a run.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;
use yokohama_core::GameRecord;
use yokohama_web::store::{Doc, Store};
use yokohama_web::{app, AppState};

/// A tiny deterministic PRNG, so a failure reproduces exactly. SplitMix64 —
/// the same shape the engine uses for its own stream derivation, and no new
/// dependency for a test that only needs to pick an index.
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
    state: Arc<AppState>,
    router: axum::Router,
}

impl Api {
    fn new() -> Self {
        Self::over(Arc::new(AppState::in_memory()))
    }

    fn over(state: Arc<AppState>) -> Self {
        Self { router: app(Arc::clone(&state)), state }
    }

    /// The same store behind a FRESH server — every handle gone, as a
    /// container that slept (M6.5c2): the next request rebuilds each game from
    /// its record.
    fn restarted(&self) -> Self {
        let store: Arc<dyn Store> = Arc::clone(self.state.games.store());
        Self::over(Arc::new(AppState::with_store(store, self.state.identity)))
    }

    /// A call as the vs-bot arm's single player.
    async fn call(&self, method: &str, uri: &str, body: Option<serde_json::Value>) -> (StatusCode, serde_json::Value) {
        self.call_as("red@wire.test", method, uri, body).await
    }

    /// A call under a NAMED identity.
    ///
    /// Every `/api/games` route needs one (M6.1), and since §5.1 every reply is
    /// built for the caller — so which identity a request carries is not a
    /// formality here but the whole subject of the second arm.
    async fn call_as(&self, who: &str, method: &str, uri: &str, body: Option<serde_json::Value>) -> (StatusCode, serde_json::Value) {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header("x-yokohama-claimed-user", who);
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
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|e| panic!("{uri} must answer JSON: {e}: {:?}", String::from_utf8_lossy(&bytes)))
        };
        (status, value)
    }
}

/// One move, chosen from whatever the prompt offers — the policy BOTH arms play
/// by.
///
/// `None` means the prompt says the game is over, which is the caller's signal
/// to leave its loop. Every other shape is answered; an unknown one panics,
/// because a prompt kind no driver knows is a surface a client could not have
/// rendered either.
///
/// **It places NOTHING**, and its callers make up for that with
/// [`probe_a_placement`]. Placing none is legal on its own terms (Option A
/// takes 0–3 areas), and it keeps the policy from having to know which tiles
/// accept an assistant, a fact the engine owns and this test must not restate.
/// The vs-bot arm once leaned on the bot to end the game instead, and
/// livelocked on some OS-entropy seeds until M6.5c2.
fn choose(state: &Value, rng: &mut Rng) -> Option<String> {
    let prompt = &state["prompt"];
    Some(match prompt["kind"].as_str().expect("a prompt kind") {
        // The main list and a sub-list are both "pick a number", which is the
        // whole point of the grouped rows: one interaction shape.
        "options" | "pick" => {
            let rows = prompt["rows"].as_array().expect("rows");
            assert!(!rows.is_empty(), "an answerable prompt must offer something: {state}");
            let row = &rows[rng.below(rows.len())];
            row["number"].as_u64().expect("a number").to_string()
        }
        "place_a" => "confirm".to_string(),
        // Back out; the next pass picks a different entry.
        "place_b" => "cancel".to_string(),
        // Recover none from each area in turn, then commit the retreat.
        "retreat" => {
            if prompt["remaining"].as_u64().unwrap_or(0) > 0 {
                "0".to_string()
            } else {
                "confirm".to_string()
            }
        }
        "reveal" => panic!("a simulation must never ask for a card reveal: {state}"),
        "game_over" => return None,
        other => panic!("unknown prompt kind {other:?}: {state}"),
    })
}

/// **The stuck-picker rule, ONE copy for both arms** (M6.5c2; the rule and its
/// measurement are M6.1's). A prompt that does not move is a walk that does not
/// terminate: a pair the server accepted one area at a time can still be
/// unaffordable together, and then Confirm is refused and the same picker comes
/// back unchanged for ever — MEASURED: seats holding `A03, A04` with one yen owed
/// to the President in A03 and no money, refused 40,000 times. So when a
/// prompt comes back unchanged three more times after its first — the FOURTH
/// identical prompt in a row — the answer is to back OUT of the picker rather
/// than to keep pushing. That is `wire_fixtures.rs`'s walk's rule, for its
/// reason.
#[derive(Default)]
struct Stuck {
    repeats: usize,
    last: Value,
}

impl Stuck {
    /// Whether `prompt` is the fourth identical prompt in a row — its first
    /// sighting and three repeats — so the caller backs out with `cancel`.
    fn again(&mut self, prompt: &Value) -> bool {
        if *prompt == self.last {
            self.repeats += 1;
        } else {
            self.repeats = 0;
            self.last = prompt.clone();
        }
        self.repeats >= 3
    }
}

/// **The placement probe, ONE copy for both arms** (M6.5c2). `place_a` does not
/// carry its legal set (Option B does), so this ASKS: tiles holding a
/// President first, so a placement sometimes costs a fee, then the area it
/// last placed in, then a rotated order so the walk does not place in area 0
/// every turn. The server decides what it will accept, and nothing here
/// restates a rule. The reply that GREW the selection, or `None` when the
/// picker already holds two or no area takes one — placing none is legal, so
/// the caller falls through to [`choose`].
async fn probe_a_placement(
    api: &Api,
    who: &str,
    id: &str,
    at: &Value,
    rng: &mut Rng,
    placed_before: &mut Option<u64>,
) -> Option<Value> {
    let already = at["prompt"]["selected"].as_array().map_or(0, Vec::len);
    let want = at["prompt"]["max_areas"].as_u64().unwrap_or(0).min(2) as usize;
    if already >= want {
        return None;
    }
    let occupied: Vec<u64> = at["frame"]["board"]["tiles"]
        .as_array()
        .map(|tiles| {
            tiles
                .iter()
                .filter(|t| !t["presidents"].as_array().map(Vec::is_empty).unwrap_or(true))
                .filter_map(|t| t["area"].as_u64())
                .collect()
        })
        .unwrap_or_default();
    let offset = rng.below(24) as u64;
    let order: Vec<u64> =
        occupied.into_iter().chain(*placed_before).chain((0..24).map(|i| (i + offset) % 24)).collect();
    let uri = format!("/api/games/{id}/input");
    for area in order {
        let (status, probe) =
            api.call_as(who, "POST", &uri, Some(serde_json::json!({ "line": format!("A{area:02}") }))).await;
        let grew = probe["prompt"]["kind"] == "place_a"
            && probe["prompt"]["selected"].as_array().map_or(0, Vec::len) > already;
        if status == StatusCode::OK && grew {
            *placed_before = Some(area);
            return Some(probe);
        }
    }
    None
}

/// The bot's stream seed at `id` — pinned by `YOKOHAMA_WEB_GATE_BOT_SEED` to
/// reproduce a recorded run, else the one the server drew.
///
/// **Why this exists** (M6.5c2, spec §2.6; review o5): the create's seed is
/// fixed and the walk's RNG is fixed, but the BOT's stream seed is drawn from OS
/// entropy at finalize (`tables.rs`'s `draw_seed`), so a run that fails is
/// reproducible only once that third seed is known. It is read off the stored
/// record's `run.seats`, which is what a rebuilt game's bot plays from. It is
/// PINNED by rewriting that record before the first move and restarting the
/// server over the same store, so the next request rebuilds the game from the
/// pinned seed exactly as a container that slept would.
fn bot_stream_seed(api: Api, id: &str) -> (Api, u64) {
    let store: Arc<dyn Store> = Arc::clone(api.state.games.store());
    let (json, version) = store.read(Doc::Record, id).expect("a readable store").expect("a dealt game's record");
    let mut record: GameRecord = serde_json::from_str(&json).expect("the stored record parses");
    let pinned = std::env::var("YOKOHAMA_WEB_GATE_BOT_SEED")
        .ok()
        .map(|s| s.trim().parse::<u64>().expect("YOKOHAMA_WEB_GATE_BOT_SEED is a u64"));
    let Some(seed) = pinned else {
        let drawn = record.run.as_ref().and_then(|run| run.seats.iter().find_map(|s| s.agent_seed));
        return (api, drawn.expect("the bot seat records its stream seed"));
    };
    assert!(record.events.is_empty(), "premise: the seed is pinned before the first move");
    let run = record.run.as_mut().expect("a web game records its seats");
    for seat in run.seats.iter_mut().filter(|s| s.agent_seed.is_some()) {
        seat.agent_seed = Some(seed);
    }
    let rewritten = serde_json::to_string_pretty(&record).expect("a record serializes");
    store.write_if_version(Doc::Record, id, &rewritten, Some(version)).expect("the pinned record lands");
    (api.restarted(), seed)
}

/// **The vs-bot arm's input cap, MEASURED** (M6.5c2, spec §2.6): 50 runs over
/// OS-entropy bot seeds at Task 8's tree (base `605c3ddd`), after the placement
/// probe and the stuck-picker rule — inputs min 136, median 383, max 1,212, and
/// every run's bot seed in the ledger's `task-8-gate-50.txt`. The cap is the
/// next thousand above four times that maximum, so a regression fails in
/// minutes where the old 20,000 cost half an hour (M6.3 close §6.3: 1,813 s).
const VS_BOT_CAP: usize = 5_000;

#[tokio::test]
async fn a_whole_two_player_game_against_the_bot_plays_through_the_api() {
    let api = Api::new();

    // Seat 1 is the shipped bot; seat 0 is us. A fixed seed so a failure here is
    // reproducible; the SERVER's own default is OS entropy. **A create opens a DEALING table
    // and nothing is playable until the last keep** (M6.1 §4.3) — so the gate
    // drives the whole opening sequence, which is itself part of what "can this
    // surface be played" means. The two cards a seat chooses between are served
    // by `GET …/deal` and read there (`the_deal_serves_the_callers_own_cards…`);
    // this arm keeps card 1 without looking, because what it is measuring is the
    // GAME.
    let (status, table) = api
        .call(
            "POST",
            "/api/games",
            Some(serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
                "seed": 20260830u64,
                "seats": [{ "user": "red@wire.test" }, { "bot": true }],
                "name": "gate",
                "id": "gate",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{table}");
    assert_eq!(table["status"], "dealing", "a create opens a table, not a game: {table}");

    let (status, kept) = api
        .call("POST", "/api/games/gate/keep", Some(serde_json::json!({ "seat": 0, "card": 1 })))
        .await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    assert_eq!(kept["status"], "playing", "the last keep starts the game: {kept}");

    let (api, bot_seed) = bot_stream_seed(api, "gate");
    // Printed FIRST, so a run that never finishes still names its seed.
    eprintln!("gate: start bot_seed={bot_seed}");

    let (status, mut state) = api.call("GET", "/api/games/gate", None).await;
    assert_eq!(status, StatusCode::OK, "{state}");
    assert_eq!(state["seat"], 0, "the viewing seat is the human one");
    assert_eq!(state["mode"], "sim");
    assert!(
        !state["frame"]["board"]["tiles"].as_array().expect("tiles").is_empty(),
        "a created game carries a board",
    );

    let mut rng = Rng(0x594F_4B4F_5745_4200); // "YOKOWEB"
    let mut inputs = 0usize;
    let mut turns_seen = 0u64;
    let mut selections = 0usize;
    let mut backed_out = 0usize;
    let mut stuck = Stuck::default();
    let mut placed_before: Option<u64> = None;

    while state["over"] != serde_json::Value::Bool(true) {
        inputs += 1;
        assert!(inputs < VS_BOT_CAP, "a game must finish inside the cap (bot seed {bot_seed}); last state: {state}");
        turns_seen = turns_seen.max(state["frame"]["decision"]["turn_number"].as_u64().unwrap_or(0));

        // **The two guards the two-people arm has** (M6.5c2, spec §2.6;
        // review o5). Without them this arm placed nothing and leaned on the
        // bot to end the game, which on some OS-entropy bot seeds it never did
        // (M6.3 close §6.3: turn 7,295, no end trigger advancing).
        if stuck.again(&state["prompt"]) {
            let (status, next) =
                api.call("POST", "/api/games/gate/input", Some(serde_json::json!({ "line": "cancel" }))).await;
            assert_eq!(status, StatusCode::OK, "backing out of a stuck picker (bot seed {bot_seed}): {next}");
            backed_out += 1;
            state = next;
            continue;
        }
        if state["prompt"]["kind"] == "place_a" {
            // Bound first, so no borrow of `state` outlives the probe.
            let probed = probe_a_placement(&api, "red@wire.test", "gate", &state, &mut rng, &mut placed_before).await;
            if let Some(next) = probed {
                selections += 1;
                state = next;
                continue;
            }
        }

        let Some(line) = choose(&state, &mut rng) else { break };

        let (status, next) =
            api.call("POST", "/api/games/gate/input", Some(serde_json::json!({ "line": line }))).await;
        assert_eq!(status, StatusCode::OK, "input {line:?} was refused (bot seed {bot_seed}): {next}");
        state = next;
    }
    eprintln!(
        "gate: done bot_seed={bot_seed} inputs={inputs} events={} turns={turns_seen} selections={selections} backed_out={backed_out}",
        state["events"],
    );

    assert!(
        selections > 0,
        "the vs-bot arm never selected an area, so the placement probe the livelock fix rests on never ran",
    );
    assert_eq!(state["over"], true, "the game ends");
    let scores = state["scores"].as_array().expect("a finished game reports scores");
    assert_eq!(scores.len(), 2, "both seats are scored: {state}");
    assert!(turns_seen > 1, "the game ran for more than one turn (saw {turns_seen})");
    assert_eq!(
        state["prompt"]["kind"], "game_over",
        "and the prompt says so rather than offering a move",
    );

    // It survives a round trip through storage: the handle is dropped and the
    // game is rebuilt from its GameRecord alone, which is the property the
    // hosted plan depends on (a container that sleeps loses its memory).
    let (status, reloaded) = api.call("GET", "/api/games/gate", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reloaded["over"], true);
    assert_eq!(reloaded["scores"], state["scores"], "a reloaded game scores the same");

    let (status, list) = api.call("GET", "/api/games", None).await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&str> = list["games"]
        .as_array()
        .expect("a games list")
        .iter()
        .map(|g| g["id"].as_str().expect("an id"))
        .collect();
    assert_eq!(ids, ["gate"]);
}


/// **What a watcher's payload must be, wherever it came from** (M6.3 §9.4).
///
/// A spectator holds no seat, so there is no seat to unmask: `seat` is null,
/// every Players row says its hand is hidden, and there is no `Hand` panel on
/// the frame at all. The three statements are checked separately because a
/// server could make any two of them true and leak through the third.
fn assert_watching(payload: &Value, at: &str) {
    assert_eq!(payload["seat"], Value::Null, "a watcher holds no seat ({at}): {payload}");
    let panels = payload["frame"]["dashboard"].as_array().expect("a dashboard");
    let players = panels.iter().find_map(|p| p.get("Players")).expect("a Players panel");
    let rows = players["players"].as_array().expect("one row per seat");
    assert_eq!(rows.len(), 2, "both seats are on a watcher's panel ({at}): {players}");
    for p in rows {
        assert_eq!(p["hand_hidden"], true, "every hand is hidden from a watcher ({at}): {p}");
    }
    assert!(
        panels.iter().all(|p| p.get("Hand").is_none()),
        "a watcher is handed no Hand panel ({at}): {payload}",
    );
}

/// **The public half of a frame: what a watcher and a player must AGREE on.**
///
/// The `Hand` panels come out because a watcher has none — that is
/// [`assert_watching`]'s claim, not this one. `hand_hidden` is normalised for
/// the same reason and no other: it is the masking statement itself, asserted
/// on both sides already, so leaving it in would make this compare re-state a
/// difference it knows about while hiding every difference it does not. What is
/// left is the game.
fn public_dashboard(frame: &Value) -> Vec<Value> {
    frame["frame"]["dashboard"]
        .as_array()
        .expect("a dashboard")
        .iter()
        .filter(|p| p.get("Hand").is_none())
        .map(|p| {
            let mut p = p.clone();
            if let Some(rows) = p.pointer_mut("/Players/players").and_then(Value::as_array_mut) {
                for r in rows.iter_mut() {
                    r["hand_hidden"] = Value::Bool(true);
                }
            }
            p
        })
        .collect()
}

/// One position, read by the watcher and by a player, and compared.
///
/// `open_hands` counts the player's frames that carried a hand with cards in it
/// BEFORE the strip — the anti-vacuity partner for the compare, without which
/// "the two dashboards agree once the hands are taken out" is a sentence about
/// payloads that may never have differed.
async fn watch_position(api: &Api, green: &str, player: &str, k: u64, open_hands: &mut usize) {
    let (status, live) = api.call_as(green, "GET", "/api/games/two", None).await;
    assert_eq!(status, StatusCode::OK, "{live}");
    assert_watching(&live, &format!("live, {k} events"));

    let (status, watched) = api.call_as(green, "GET", &format!("/api/games/two/at/{k}"), None).await;
    assert_eq!(status, StatusCode::OK, "{watched}");
    assert_watching(&watched, &format!("at {k}"));

    let (status, played) = api.call_as(player, "GET", &format!("/api/games/two/at/{k}"), None).await;
    assert_eq!(status, StatusCode::OK, "{played}");
    if played["frame"]["dashboard"]
        .as_array()
        .expect("a dashboard")
        .iter()
        .filter_map(|p| p.get("Hand"))
        .any(|h| h["cards"].as_array().is_some_and(|c| !c.is_empty()))
    {
        *open_hands += 1;
    }

    // **The same GAME, and `decision` is excluded by construction**: the
    // player's is built for the player's seat and the watcher has none, which is
    // a difference the spec names rather than a finding.
    assert_eq!(
        watched["frame"]["board"], played["frame"]["board"],
        "a watcher and a player see the same board at {k}",
    );
    assert_eq!(
        watched["frame"]["title"], played["frame"]["title"],
        "a watcher and a player see the same title at {k}",
    );
    assert_eq!(
        public_dashboard(&watched),
        public_dashboard(&played),
        "a watcher and a player see the same dashboard at {k}, once the hands are out",
    );
}

/// **The gate's second arm: two PEOPLE play to a winner, and neither one is ever
/// handed the other's hand.**
///
/// The first arm is one identity against a bot, which is what this surface was
/// before M6.1; it cannot say anything at all about whose payload is whose,
/// because there is only ever one caller. This arm drives the SAME policy under
/// two claimed identities — every request carrying its own header, alternating
/// with the turn — and holds every payload to [`assert_built_for`]: `seat` is
/// the seat the REQUESTER holds, `hand_hidden` is false at exactly that seat and
/// true at every other, and the one open Hand panel on the frame is that seat's.
///
/// **The expected seat is passed IN, never read off the payload**, and that is
/// the whole discrimination. Stated against the payload's own `seat` it is a
/// tautology; stated against who asked, a server keyed on
/// `state.current_player` — ignoring the header entirely — fails.
///
/// **So the arm asks as the NON-MOVER too, once per turn**, and that is the
/// assertion a mover-keyed server actually trips. A walk that only ever asks as
/// the mover cannot tell the two servers apart, because for the mover the two
/// answers are the same seat. Mutation-proven: `client::viewer_seat` returning
/// `Some(state.current_player)` unconditionally fails the non-mover read and
/// nothing else in this arm.
///
/// **And the POST replies are checked, not only the GETs.** A move's reply is a
/// whole `ClientState` built for its caller (§5.1) — it is the payload the
/// client renders next — so leaving it unread would have left the busiest
/// surface on this route unasserted.
///
/// **And a THIRD identity watches the whole thing** (M6.3 §9.4). Green holds no
/// seat at this table and was never named at it: the table is locked, so its
/// first read is refused, `unlock` buys it the right to read, and from there it
/// is served every position the two players are — `GET …/{id}` live and
/// `at/{k}` at each `k` — with `seat: null`, every hand hidden and no `Hand`
/// panel at all. The two halves of that are one sentence read from both ends:
/// what a watcher sees is the same GAME (`board`, `title` and the dashboards
/// with the hands taken out are EQUAL to a player's at the same position) and
/// none of the hands. A single-identity arm cannot say either, and neither can
/// the two-player arm alone: both of its callers hold a seat.
///
/// **It probes a placement, and it has to.** [`choose`] answers every placement
/// picker with `confirm` on an empty selection, which is legal and keeps the
/// policy from restating a rule the engine owns. A bot-free game in which
/// nobody ever places never ends at all — measured on the fixture walk at turn
/// 3,644, with ten cards still in the deck, every seat still holding eight
/// assistants and nobody above zero VP — and the first arm, which once leaned
/// on the BOT to end the game instead, livelocked (M6.3 close §6.3). So both
/// arms ask the server which areas it will take through [`probe_a_placement`],
/// exactly as `wire_fixtures.rs`'s walk does, and the count at the bottom is
/// what says this one really did.
#[tokio::test]
async fn two_people_play_to_a_winner_and_each_sees_only_their_own_hand() {
    let api = Api::new();
    let (red, blue) = ("red@wire.test", "blue@wire.test");
    // **Blue ARRIVES rather than being named** (M6.2 §8.3). The table is opened
    // with seat 1 OPEN and a password on it, so the path this arm walks in is
    // the lobby's: a person nobody named at create time finds the table under
    // `scope=open`, is refused by the lock, and is seated by the join that
    // fills the last seat — which is what flips the table to `dealing`. Nothing
    // after the keeps moves, and it is still the whole subject of this arm:
    // two identities, each served the seat it holds and no other.
    let (status, table) = api
        .call_as(
            red,
            "POST",
            "/api/games",
            Some(serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
                "seed": 20260914u64, "name": "two", "id": "two",
                "seats": [{ "user": red }, { "open": true }],
                "password": "sesame",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{table}");
    assert_eq!(table["status"], "open", "a table with an open seat opens OPEN: {table}");

    // Blue was told nothing about this table. The open scope is every table
    // waiting for players and not only the ones you were sent a link to
    // (R-2.7), so this is the arrival that needs no invitation.
    let (status, listed) = api.call_as(blue, "GET", "/api/games?scope=open", None).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert!(
        listed["games"].as_array().expect("a page of tables").iter().any(|t| t["id"] == "two"),
        "blue finds the open table without having been named at it: {listed}",
    );

    // The lock from the OUTSIDE: a join carrying no password is refused, and
    // the refusal is about the table rather than about who is asking.
    let (status, refused) = api
        .call_as(blue, "POST", "/api/games/two/join", Some(serde_json::json!({ "seat": 1 })))
        .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "a locked table refuses a join with no password: {refused}",
    );
    let (status, joined) = api
        .call_as(
            blue,
            "POST",
            "/api/games/two/join",
            Some(serde_json::json!({ "seat": 1, "name": "Blue", "password": "sesame" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{joined}");
    assert_eq!(joined["status"], "dealing", "the join that fills the last seat flips it: {joined}");
    assert_eq!(joined["seats"][1]["name"], "Blue", "the name rides with the join: {joined}");

    // Each seat chooses for ITSELF, over a deal it fetched on its own. The last
    // keep is what starts the game.
    for (user, seat) in [(red, 0), (blue, 1)] {
        let (status, deal) = api.call_as(user, "GET", "/api/games/two/deal", None).await;
        assert_eq!(status, StatusCode::OK, "{deal}");
        assert_eq!(deal["seat"], seat, "a caller is served the seat it holds: {deal}");
        let (status, kept) = api
            .call_as(user, "POST", "/api/games/two/keep", Some(serde_json::json!({ "seat": seat, "card": 1 })))
            .await;
        assert_eq!(status, StatusCode::OK, "{kept}");
    }

    // **The watcher arrives.** Green was told nothing about this table and holds
    // no seat at it. A locked table refuses its read outright — and the refusal
    // is about the TABLE, not about which seat is asking — so `unlock` is the
    // whole of the ladder between a link and a game somebody is here to watch.
    let green = "green@wire.test";
    let (status, refused) = api.call_as(green, "GET", "/api/games/two", None).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "a locked table refuses a stranger's read before the password: {refused}",
    );
    // **The SENTENCE, because the status alone does not witness the lock**
    // (final review M-3). An UNLOCKED playing table refuses a stranger with the
    // same 403 until they are admitted, so a cell that read only the number
    // would pass against a table that had no password at all — and the `unlock`
    // below answers 200 at such a table too. This is the one assertion in the
    // arm that can tell the two apart.
    assert_eq!(
        refused["error"], "this table is locked — unlock it first",
        "the refusal is about the LOCK, not about which seat is asking: {refused}",
    );
    let (status, unlocked) = api
        .call_as(green, "POST", "/api/games/two/unlock", Some(serde_json::json!({ "password": "sesame" })))
        .await;
    assert_eq!(status, StatusCode::OK, "the password buys the right to read: {unlocked}");

    // ---- M6.4: the three of them talk before the first move (spec §9.4) ----
    let (status, m) = api
        .call_as(green, "POST", "/api/games/two/chat", Some(serde_json::json!({ "text": "hello from the rail", "name": "Green" })))
        .await;
    assert_eq!(status, StatusCode::CREATED, "the watcher's one write: {m}");
    assert_eq!(m["author"]["kind"], "watcher", "{m}");
    for (user, seat, text) in [(red, 0, "gl"), (blue, 1, "hf")] {
        let (status, m) = api
            .call_as(user, "POST", "/api/games/two/chat", Some(serde_json::json!({ "text": text })))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        assert_eq!((m["author"]["kind"].as_str(), m["author"]["seat"].as_u64()), (Some("seat"), Some(seat)), "{m}");
    }
    let (_, m) = api.call_as(blue, "GET", "/api/games/two/chat?after=2", None).await;
    assert_eq!(m["messages"][0]["author"]["name"], "Blue", "the seat's stored name from the join: {m}");
    for who in [red, blue, green] {
        let (status, page) = api.call_as(who, "GET", "/api/games/two/chat", None).await;
        assert_eq!(status, StatusCode::OK, "{who}: {page}");
        assert_eq!(page["messages"].as_array().map(Vec::len), Some(3), "{who} reads all three: {page}");
        let (_, e) = api.call_as(who, "GET", "/api/games/two/events", None).await;
        assert_eq!(e["chat"], 3, "{who}'s frame carries the count: {e}");
    }

    // **Every payload this arm sees goes through here, and the seat it is
    // checked against is passed IN.**
    //
    // Reading the expected seat off the payload — `hand_hidden == (p.seat !=
    // payload.seat)` — is a tautology about one object: a server that built
    // every reply for `state.current_player` and ignored the header would
    // satisfy it on every line. `who` and `seat` come from what this test KNOWS
    // about the table it created, so the two servers are distinguishable.
    let assert_built_for = |payload: &Value, who: &str, seat: u64| {
        assert_eq!(payload["seat"], seat, "{who} holds seat {seat}, and was served: {payload}");
        let panels = payload["frame"]["dashboard"].as_array().expect("a dashboard");
        let players = panels.iter().find_map(|p| p.get("Players")).expect("a Players panel");
        let rows = players["players"].as_array().expect("one row per seat");
        assert_eq!(rows.len(), 2, "both seats are on the panel: {players}");
        for p in rows {
            assert_eq!(
                p["hand_hidden"],
                p["seat"] != seat,
                "{who} holds seat {seat} and is shown seat {} with hand_hidden {}: {p}",
                p["seat"],
                p["hand_hidden"],
            );
        }
        // And the one hand drawn OPEN on the frame is this caller's own. The
        // `hand_hidden` rows above are the Players panel's summary; the Hand
        // panel is the card list itself, which is where identities would leak.
        let hands: Vec<&Value> = panels.iter().filter_map(|p| p.get("Hand")).collect();
        assert_eq!(hands.len(), 1, "{who}: exactly one Hand panel, got {}", hands.len());
        assert_eq!(hands[0]["seat"], seat, "{who} holds seat {seat}: {}", hands[0]);
        assert_eq!(hands[0]["hidden"], false, "{who}'s own hand is open: {}", hands[0]);
    };

    let mut rng = Rng(0x5457_4F50_4C41_5945); // "TWOPLAYE"
    let mut inputs = 0usize;
    // The turn the non-mover was last read on, so that read happens once per
    // turn rather than once per move — it is the assertion a mover-keyed
    // server trips, and it costs a request.
    let mut watched: Option<u64> = None;
    // How many times the server ACCEPTED an area into a placement picker. Not
    // the same as an assistant reaching the board — a selection can still be
    // cancelled or refused at Confirm — which is why the claim below is the
    // weaker and true one. What says assistants really were placed is that the
    // game ENDED: a bot-free game in which nobody places never does.
    let mut selections = 0usize;
    // The area the server last accepted an assistant in, tried first next time.
    // A CACHE over answers it already gave, never a rule: a miss falls back to
    // asking about every area in turn.
    let mut placed_before: Option<u64> = None;
    // The stuck-picker rule — [`Stuck`].
    let mut stuck = Stuck::default();
    // The position green has already been read at, so the watcher's reads are
    // one per APPLIED MOVE rather than one per loop pass — see below.
    let mut watched_position: Option<u64> = None;
    let mut positions_watched = 0usize;
    // How many of red's replay frames carried an OPEN hand before the strip.
    // Without it "the dashboards agree once the hands are taken out" is a claim
    // about two payloads that may never have differed.
    let mut open_hands_seen = 0usize;
    let (_, mut state) = api.call_as(red, "GET", "/api/games/two", None).await;

    while state["over"] != Value::Bool(true) {
        inputs += 1;
        assert!(inputs < 40_000, "a game must finish inside the cap; last state: {state}");
        let seat = state["to_move"].as_u64().expect("a seat to move");
        let (mover, watcher, watcher_seat) =
            if seat == 0 { (red, blue, 1) } else { (blue, red, 0) };

        // **Green reads the same position the players are at, every time one is
        // reached.**
        //
        // Keyed on the EVENT COUNT and not on the loop pass, for a reason that
        // is about cost rather than about the claim: a picker step applies no
        // event, and `at/{k}` replays `k` events from setup, so a read per pass
        // would re-replay the same position several times a turn for the whole
        // game. Every position both players pass through is still read — this
        // loop cannot advance the count without coming back round — and the
        // last one is read after the loop, where the game is over.
        let k = state["events"].as_u64().expect("an event count");
        if watched_position != Some(k) {
            watched_position = Some(k);
            positions_watched += 1;
            watch_position(&api, green, red, k, &mut open_hands_seen).await;
        }
        let (status, mine) = api.call_as(mover, "GET", "/api/games/two", None).await;
        assert_eq!(status, StatusCode::OK, "{mine}");
        assert_built_for(&mine, mover, seat);

        // **The read a mover-keyed server fails**, once per turn. For the mover
        // "the seat you hold" and "the seat to move" are the same number, so
        // every assertion above passes either way; the WATCHER's payload is
        // where they come apart, and its `seat` must be the watcher's own.
        if watched != Some(seat) {
            watched = Some(seat);
            let (status, theirs) = api.call_as(watcher, "GET", "/api/games/two", None).await;
            assert_eq!(status, StatusCode::OK, "{theirs}");
            assert_built_for(&theirs, watcher, watcher_seat);
            assert_eq!(
                theirs["your_turn"], false,
                "{watcher} is not to move, and the payload says so: {theirs}",
            );
        }

        if stuck.again(&mine["prompt"]) {
            let (status, next) = api
                .call_as(mover, "POST", "/api/games/two/input", Some(serde_json::json!({ "line": "cancel" })))
                .await;
            assert_eq!(status, StatusCode::OK, "backing out of a stuck picker: {next}");
            assert_built_for(&next, mover, seat);
            state = next;
            continue;
        }

        // **The placement probe** — [`probe_a_placement`].
        if mine["prompt"]["kind"] == "place_a" {
            let probed = probe_a_placement(&api, mover, "two", &mine, &mut rng, &mut placed_before).await;
            if let Some(next) = probed {
                selections += 1;
                assert_built_for(&next, mover, seat);
                state = next;
                continue;
            }
            // No area will take one — a real state late in a game, when the
            // assistants are gone. Placing none is legal, so fall through.
        }

        let Some(line) = choose(&mine, &mut rng) else { break };
        let (status, next) = api
            .call_as(mover, "POST", "/api/games/two/input", Some(serde_json::json!({ "line": line })))
            .await;
        assert_eq!(status, StatusCode::OK, "input {line:?} as {mover} was refused: {next}");
        // **The move's own reply, which is what the client renders next.** It is
        // built for its CALLER, so it is the mover's seat even when the move
        // ended the turn and `to_move` has already moved on.
        assert_built_for(&next, mover, seat);
        state = next;
    }

    // The probe really did run, rather than falling through to `confirm` every
    // time — without this the paragraph above about a game that never places
    // would be describing something this arm does not do.
    assert!(selections > 0, "the arm never selected an area, so it never exercised a placement picker");

    // **The last position, where the game is over.** The loop reads a position
    // on its way round and so cannot read the one it leaves on; this is that
    // one, and it is also the only read here taken at a FINISHED table, which
    // is a state a watcher is admitted to for its own reasons.
    let last = state["events"].as_u64().expect("an event count");
    watch_position(&api, green, blue, last, &mut open_hands_seen).await;
    assert!(
        positions_watched > 1,
        "the watcher read {positions_watched} positions, so it never followed a game",
    );
    assert!(
        open_hands_seen > 0,
        "no player frame in the whole game carried an open hand, so stripping the hands \
         out of the compare above proved nothing",
    );

    // **`events.status` reads `over`, and nothing else in this crate asserts
    // that** — the client's poll branches on it to stop walking, and until this
    // line the field could have said anything at the end of a game.
    let (status, e) = api.call_as(red, "GET", "/api/games/two/events", None).await;
    assert_eq!(status, StatusCode::OK, "{e}");
    assert_eq!(e["status"], "over", "{e}");
    // And the final score answers the seat that did not ask for it last.
    let (status, result) = api.call_as(blue, "GET", "/api/games/two/result", None).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["standings"].as_array().map(Vec::len), Some(2), "{result}");
}
