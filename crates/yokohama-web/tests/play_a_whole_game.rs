//! **The gate: a whole 2-player game, human vs the shipped bot, over the API.**
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

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
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

    async fn call(&self, method: &str, uri: &str, body: Option<serde_json::Value>) -> (StatusCode, serde_json::Value) {
        let req = Request::builder().method(method).uri(uri);
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

#[tokio::test]
async fn a_whole_two_player_game_against_the_bot_plays_through_the_api() {
    let api = Api::new();

    // The deal preview a client shows before anyone chooses. A fixed seed so a
    // failure here is reproducible; the SERVER's own default is OS entropy.
    let (status, preview) = api
        .call(
            "POST",
            "/api/games/preview",
            Some(serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
                "seed": 20260830u64, "keeps": [1, 1],
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["deals"].as_array().expect("deals").len(), 2);
    assert_eq!(
        preview["deals"][0]["cards"].as_array().expect("two dealt cards").len(),
        2,
        "each seat is dealt two order cards: {preview}",
    );

    // Seat 1 is the shipped bot; seat 0 is us.
    let (status, mut state) = api
        .call(
            "POST",
            "/api/games",
            Some(serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
                "seed": 20260830u64, "keeps": [1, 1], "bot_seats": [1],
                "name": "gate",
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{state}");
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

        let prompt = &state["prompt"];
        let line: String = match prompt["kind"].as_str().expect("a prompt kind") {
            // The main list and a sub-list are both "pick a number", which is
            // the whole point of the grouped rows: one interaction shape.
            "options" | "pick" => {
                let rows = prompt["rows"].as_array().expect("rows");
                assert!(!rows.is_empty(), "an answerable prompt must offer something: {state}");
                let row = &rows[rng.below(rows.len())];
                row["number"].as_u64().expect("a number").to_string()
            }
            // Place nothing: legal on its own terms (Option A takes 0-3 areas),
            // and it keeps the driver from having to know which tiles accept an
            // assistant — a fact the engine owns and this test must not restate.
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
            "game_over" => break,
            other => panic!("unknown prompt kind {other:?}: {state}"),
        };

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
    assert_eq!(list, serde_json::json!(["gate"]));
}
