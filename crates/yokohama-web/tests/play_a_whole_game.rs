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

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;
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
    router: axum::Router,
}

impl Api {
    fn new() -> Self {
        Self { router: app(Arc::new(AppState::in_memory())) }
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
/// **It places NOTHING**, and its callers make up for that separately. Placing
/// none is legal on its own terms (Option A takes 0–3 areas) and it keeps the
/// policy from having to know which tiles accept an assistant — a fact the
/// engine owns and this test must not restate. The vs-bot arm can afford it
/// because the bot drives the game to an end regardless; the two-people arm
/// cannot, and probes for a placement itself.
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

    while state["over"] != serde_json::Value::Bool(true) {
        inputs += 1;
        assert!(inputs < 20_000, "a game must finish inside the cap; last state: {state}");
        turns_seen = turns_seen.max(state["frame"]["decision"]["turn_number"].as_u64().unwrap_or(0));

        let Some(line) = choose(&state, &mut rng) else { break };

        let (status, next) =
            api.call("POST", "/api/games/gate/input", Some(serde_json::json!({ "line": line }))).await;
        assert_eq!(status, StatusCode::OK, "input {line:?} was refused: {next}");
        state = next;
    }

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


/// **The gate's second arm: two PEOPLE play to a winner, and neither one is ever
/// handed the other's hand.**
///
/// The first arm is one identity against a bot, which is what this surface was
/// before M6.1; it cannot say anything at all about whose payload is whose,
/// because there is only ever one caller. This arm drives the SAME policy under
/// two claimed identities — every request carrying its own header, alternating
/// with the turn — and reads two things off every single reply:
///
/// 1. **`seat` is the REQUESTER's**, not the game's idea of who is interesting.
///    A payload built for "whoever is to move" would pass every cell in this
///    crate that only ever asks as the mover, and would hand a watching player
///    the acting seat's frame.
/// 2. **`hand_hidden` is false at the caller's seat and true at every other
///    one** — asserted as an EQUALITY against `seat != mine`, not as a pair of
///    spot checks, so a server that masked everything and a server that masked
///    nothing both fail. Mutation-proven by flipping `facts_for`'s own test.
///
/// **It probes a placement, and it has to.** [`choose`] answers every placement
/// picker with `confirm` on an empty selection, which is legal and keeps the
/// policy from restating a rule the engine owns. The first arm can afford that
/// because the BOT drives the game to an end regardless; a bot-free game in
/// which nobody ever places never ends at all — measured on the fixture walk at
/// turn 3,644, with ten cards still in the deck, every seat still holding eight
/// assistants and nobody above zero VP. So this arm asks the server which areas
/// it will take, exactly as `wire_fixtures.rs`'s walk does, and the count at the
/// bottom is what says it really did.
#[tokio::test]
async fn two_people_play_to_a_winner_and_each_sees_only_their_own_hand() {
    let api = Api::new();
    let (red, blue) = ("red@wire.test", "blue@wire.test");
    let (status, table) = api
        .call_as(
            red,
            "POST",
            "/api/games",
            Some(serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
                "seed": 20260914u64, "name": "two",
                "seats": [{ "user": red }, { "user": blue }],
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{table}");
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

    let mut rng = Rng(0x5457_4F50_4C41_5945); // "TWOPLAYE"
    let mut inputs = 0usize;
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
    // **A prompt that does not move is a walk that does not terminate.** The
    // probe below selects areas one at a time and the SERVER decides what it
    // will accept — but a pair it accepted one id at a time can still be
    // unaffordable together, and then Confirm is refused and the same picker
    // comes back unchanged for ever. MEASURED: seats holding `A03, A04` with one
    // yen owed to the President in A03 and no money, refused 40,000 times. So
    // three identical prompts in a row means the answer is to back OUT of the
    // picker rather than to keep pushing — the same rule, and the same reason,
    // as `wire_fixtures.rs`'s walk.
    let mut stuck = 0usize;
    let mut last_prompt = Value::Null;
    let (_, mut state) = api.call_as(red, "GET", "/api/games/two", None).await;

    while state["over"] != Value::Bool(true) {
        inputs += 1;
        assert!(inputs < 40_000, "a game must finish inside the cap; last state: {state}");
        let to_move = state["to_move"].clone();
        let mover = if to_move == 0 { red } else { blue };
        let (status, mine) = api.call_as(mover, "GET", "/api/games/two", None).await;
        assert_eq!(status, StatusCode::OK, "{mine}");
        assert_eq!(mine["seat"], to_move, "the payload is the requester's seat: {mine}");

        let players = mine["frame"]["dashboard"]
            .as_array()
            .expect("a dashboard")
            .iter()
            .find_map(|p| p.get("Players"))
            .expect("a Players panel");
        let rows = players["players"].as_array().expect("one row per seat");
        assert_eq!(rows.len(), 2, "both seats are on the panel: {players}");
        for p in rows {
            assert_eq!(
                p["hand_hidden"],
                p["seat"] != mine["seat"],
                "{mover} is shown seat {} with hand_hidden {}: {p}",
                p["seat"],
                p["hand_hidden"],
            );
        }

        if mine["prompt"] == last_prompt {
            stuck += 1;
        } else {
            stuck = 0;
            last_prompt = mine["prompt"].clone();
        }
        if stuck >= 3 {
            let (status, next) = api
                .call_as(mover, "POST", "/api/games/two/input", Some(serde_json::json!({ "line": "cancel" })))
                .await;
            assert_eq!(status, StatusCode::OK, "backing out of a stuck picker: {next}");
            state = next;
            continue;
        }

        // **The placement probe.** `place_a` does not carry its legal set (Option
        // B does), so this ASKS — tiles holding a President first, so a placement
        // sometimes costs a fee, then a rotated order so the walk does not place
        // in area 0 every turn. The server decides what it will accept; nothing
        // here restates a rule.
        if mine["prompt"]["kind"] == "place_a" {
            let already = mine["prompt"]["selected"].as_array().map_or(0, Vec::len);
            let want = mine["prompt"]["max_areas"].as_u64().unwrap_or(0).min(2) as usize;
            if already < want {
                let occupied: Vec<u64> = mine["frame"]["board"]["tiles"]
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
                let order: Vec<u64> = occupied
                    .into_iter()
                    .chain(placed_before)
                    .chain((0..24).map(|i| (i + offset) % 24))
                    .collect();
                let mut landed = None;
                for area in order {
                    let (status, probe) = api
                        .call_as(
                            mover,
                            "POST",
                            "/api/games/two/input",
                            Some(serde_json::json!({ "line": format!("A{area:02}") })),
                        )
                        .await;
                    let grew = probe["prompt"]["kind"] == "place_a"
                        && probe["prompt"]["selected"].as_array().map_or(0, Vec::len) > already;
                    if status == StatusCode::OK && grew {
                        placed_before = Some(area);
                        selections += 1;
                        landed = Some(probe);
                        break;
                    }
                }
                if let Some(next) = landed {
                    state = next;
                    continue;
                }
                // No area will take one — a real state late in a game, when the
                // assistants are gone. Placing none is legal, so fall through.
            }
        }

        let Some(line) = choose(&mine, &mut rng) else { break };
        let (status, next) = api
            .call_as(mover, "POST", "/api/games/two/input", Some(serde_json::json!({ "line": line })))
            .await;
        assert_eq!(status, StatusCode::OK, "input {line:?} as {mover} was refused: {next}");
        state = next;
    }

    // The probe really did run, rather than falling through to `confirm` every
    // time — without this the paragraph above about a game that never places
    // would be describing something this arm does not do.
    assert!(selections > 0, "the arm never selected an area, so it never exercised a placement picker");

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
