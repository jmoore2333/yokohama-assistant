# M6.5b "single-player in the browser" — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (for Task 4) or superpowers:executing-plans (the rest, inline — the owner's R-P11 refinement of 2026-09-20) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A person plays the bot in their browser with no server behind the page: the table layer runs as WebAssembly in a Web Worker, games persist to IndexedDB, the first screen shows local games before the server answers, a local game can be sent to the server's archive, and the axum host gains the batch run route and the archive route.

**Architecture:** A new crate `crates/yokohama-wasm` exports six functions over the host-free layer (`api::dispatch` over `Games` with `MemStore`, `NoSignal`, `NoArchive`, a settable clock); a dedicated Web Worker per tab owns the module, the IndexedDB side channel and the Web Lock; the client's one `call` gains a local arm routed by the `local-` id prefix or an explicit host. Two layer routes land first, on axum: `GET …/at?from=&to=` (one forward replay for a run of frames) and `POST /api/archive` (a strict-replay-verified send under a path-safe caller prefix).

**Tech Stack:** Rust 1.97.1 (pinned), `wasm32-unknown-unknown`, `wasm-bindgen 0.2.128` + `wasm-bindgen-cli 0.2.128`, `js-sys`; React + Vite + vitest (jsdom, and a second node project); Playwright; IndexedDB; the Web Locks API.

**Spec:** `docs/plans/2026-09-20-web-frontend-m6.5b-single-player-spec.md` (R-6.5b.1–15; its §0 is the map of HEAD every task below cites, its §6 the instruments each task ends in). The umbrella is `docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md`; the M6.5a close `docs/plans/2026-09-20-web-frontend-m6.5a-close.md` is what the layer IS.

## Global Constraints

- Branch `web-room-m6.5b`, stacked on `web-room-m6.5`; base for every diff `54a9a1f`. Stage with explicit paths, never `git add -A`. A clean `git status` after every commit.
- **The five fenced crates and the CLI stay byte-identical**: `git diff --stat 54a9a1f -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli` is EMPTY at every commit. `SEMANTICS_REV` 7. No bot behaviour moves.
- **Per-edit gates after EVERY task**: `cargo test -p yokohama-web` (259 at the base: 247 lib + 2 gate + 10 wire — re-count at your HEAD), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`; from Task 3 also `cargo test -p yokohama-wasm` and `cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm`; from Task 4 the three web commands (`cd web && npm run typecheck && npm test`, and `npm run e2e` when a task changes what a browser sees — Tasks 5, 6, 8) and from Task 7 `npm run test:wasm`. The whole-game gate's vs-bot arm livelocks on some seeds: re-run it ALONE before believing a red (M6.4 close §6.3).
- **The corpus is a contract**: regenerate with `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures` ONLY at Tasks 1, 2 and 7, and READ the diff each time — the expected diffs are stated in those tasks; anything else in the diff is a finding, not a fixture to accept. Every request body is `#[serde(deny_unknown_fields)]`.
- **Every 400 sentence the layer's parsers produce is axum's word for word** — a new query struct goes through `parse_query`, a new body through `parse_json`; a range or existence refusal is a 404 in THIS API's words (spec §2.4). Never a silent clamp.
- **The local player is `local@browser`** (R-6.5b.1). A local game's id starts with `local-` (R-6.5b.12). The Worker is the only caller of the module. The seed leaves the browser only inside a record the player sends or saves (R-6.5b.6).
- **`.superpowers/sdd/2026-09-20-web-frontend-m6.5b-plan/`** (gitignored) is the ledger: `progress.md` with one `Ruling:` line per deviation from this plan, and every review report. Every deviation is recorded there before the close reads it.
- Commit messages end with the EXECUTING agent's own attribution lines (the ones its harness names), never the plan author's; the trailers on the plan's own commits are the author's.
- `cargo test` takes ONE positional filter: where a step names several cells, run one command per cell or a shared substring.
- **Revised once before execution** after the owner's pre-execution review (spec §10): the Worker is a serialised actor with ownership from the route, a generation checked in every write, persistence of the documents that exist for held games only, commit-before-reply, storage failure distinct from a trap, a fresh Worker on a trap; the archive put protocol (`Landed` / `Identical` / a conflict); the glue bundled from a source directory; the walk script outside the corpus.

---

## File structure

**Created**
- `crates/yokohama-wasm/Cargo.toml`, `crates/yokohama-wasm/src/lib.rs` — the host: six exports, the clock, the crate's own scan (Task 3).
- `crates/yokohama-web/tests/browser_played_pin.rs` + `tests/fixtures/browser-played-2p.json`, `browser-played-2p.table.json`, `browser-played-2p.provenance.json` — the determinism pin and its committed pair (Task 8).
- `web/src/local/worker.ts` (the Worker), `web/src/local/db.ts` (IndexedDB), `web/src/local/client.ts` (the page side: the Worker singleton, `localCall`, `localRead`, the hold), `web/src/local/facts.ts` (the per-source deployment facts) (Task 4).
- `web/src/local/corpus.node.test.ts`, `web/vitest.node.config.ts` — the wasm corpus replay (Task 7).
- `web/src/test/fakeWorker.ts` — a `Worker` double answering from the committed corpus, for every jsdom cell that plays a local game (Task 4).
- `web/src/OnThisDevice.tsx` — the fifth section (Task 5); `web/src/Door.tsx` — the four door states as the server area's content (Task 5).
- `web/e2e/plays_locally.spec.ts` — the exit, the reload, the API-refused screen, the two tabs (Task 8).

**Modified**
- `crates/yokohama-web/src/client.rs` (`HistoricalFrame::from_state`, `run`), `api.rs` (`RunQuery`, `run`, `ArchivePost`, `archive_post`, two route rows), `tables.rs` (`delete`'s leaf; `check_shape` visibility), `table.rs` (`TableDoc::agrees_with`), `archive.rs` (`caller_segment`, the trait's prefix, `PutError`), `games.rs` (`load_document`, `read_document`, the two `put` call sites), `assets.rs` (the `wasm` arm), `lib.rs` (`AppState::with_clock`, the admission grid row), `tests/wire_fixtures.rs` (the script, the archive `Api`, the frozen clock, the new fixtures).
- `Cargo.toml` (the eighth member), `.gitignore`, `.github/workflows/test.yml`.
- `web/package.json`, `web/src/api.ts`, `web/src/signal.ts`, `web/src/App.tsx`, `web/src/Lobby.tsx`, `web/src/NewGame.tsx`, `web/src/route.ts`, `web/src/types.ts`, `web/src/wire/contract.ts`, `web/src/Help.tsx`, `web/e2e/harness.ts`, `web/playwright.config.ts`.
- `deploy/RUNBOOK.md`, `CLAUDE.md`, `docs/handover.md`, the close.

---

### Task 1: The run route, `HistoricalFrame::run`, `fetchRun` as one call, the delete leaf (R-6.5b.10, R-6.5b.13)

**Files:**
- Modify: `crates/yokohama-web/src/client.rs:584-680` (`HistoricalFrame::at` → `from_state` + `run`)
- Modify: `crates/yokohama-web/src/api.rs:129-162` (the route row), `:1655-1700` (beside `at_k`), `:1774-1860` (the `answer` arm)
- Modify: `crates/yokohama-web/src/tables.rs` (the abandon site in `delete`, the `Why` choice)
- Modify: `crates/yokohama-web/src/lib.rs:2881-2903` (`admission_routes()` gains a row)
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs:1432` (beside `at_k`: three new captures)
- Modify: `web/src/api.ts` (`api.run`, `RUN_CAP`), `web/src/App.tsx:55-62` (`fetchRun`), `web/src/types.ts` (`RunFrames`), `web/src/wire/contract.ts` (the row)
- Test: `crates/yokohama-web/src/lib.rs` test module (route cells), `crates/yokohama-web/src/games.rs` test module (the leaf cell), `web/src/App.test.tsx`

**Interfaces:**
- Produces: `HistoricalFrame::from_state(repl: &Repl, k: usize, state: &GameState, viewer: Viewer) -> HistoricalFrame`; `HistoricalFrame::run(repl: &Repl, from: usize, to: usize, viewer: Viewer) -> Result<Vec<HistoricalFrame>, ReplayError>`; `pub const RUN_CAP: usize = 64;` `pub struct RunQuery { pub from: usize, pub to: usize }`; `pub struct RunFrames { pub frames: Vec<HistoricalFrame> }`; `pub fn run(games: &Games, user: &User, id: &str, q: &RunQuery) -> Result<RunFrames, ApiError>`; the route row `("GET", "/api/games/{id}/at")`; client `api.run(game, from, to): Promise<RunFrames>`, `RUN_CAP = 64`.
- Consumes: `yokohama_core::{state_at, replay_from}` (both `pub`, spec §0), `admitted`, `Games::handle`, `ApiError::{not_found, bad_request}`.

- [ ] **Step 1: Write the failing route cells** (in `lib.rs`'s test module, beside the existing `at_k` cells — find them with `grep -n "at/0\|at_k" crates/yokohama-web/src/lib.rs`):

```rust
/// R-6.5b.10: a run is the positions one by one, BY VALUE, over a run that
/// crosses a turn boundary so its middle frame differs from both neighbours
/// (a run of identical frames would pass a builder that returned the first
/// frame `to - from` times).
#[tokio::test]
async fn a_run_is_the_positions_one_by_one() {
    let (app, id) = playing_game_with_events(6).await; // the module's helper that returns a 2P game past a turn boundary; if none exists, play `input` until `events >= 6`
    let total = events_of(&app, &id).await;
    let (status, run) = get_as(&app, RED, &format!("/api/games/{id}/at?from={}&to={}", total - 3, total)).await;
    assert_eq!(status, StatusCode::OK, "{run}");
    let frames = run["frames"].as_array().expect("frames");
    assert_eq!(frames.len(), 3);
    for (i, k) in (total - 3..total).enumerate() {
        let (_, one) = get_as(&app, RED, &format!("/api/games/{id}/at/{k}")).await;
        assert_eq!(frames[i], one, "frame {k} of the run is at/{k}, by value");
    }
    assert_ne!(frames[0]["frame"]["board"], frames[1]["frame"]["board"], "anti-vacuity: the middle frame differs from the first");
    assert_ne!(frames[1]["frame"]["board"], frames[2]["frame"]["board"], "… and from the last");
}

/// The boundary in numbers (spec §2.4): `to = total + 1` is ADMITTED and its
/// last frame is the live frame `at/{total}`; `to = total + 2` answers what
/// `at/{total + 1}` answers.
#[tokio::test]
async fn a_run_admits_the_live_frame_and_refuses_the_one_past_it() {
    let (app, id) = playing_game_with_events(2).await;
    let total = events_of(&app, &id).await;
    let (status, run) = get_as(&app, RED, &format!("/api/games/{id}/at?from={total}&to={}", total + 1)).await;
    assert_eq!(status, StatusCode::OK, "{run}");
    let (_, live) = get_as(&app, RED, &format!("/api/games/{id}/at/{total}")).await;
    assert_eq!(run["frames"][0], live, "the run's last frame is the live frame");
    let (status, past) = get_as(&app, RED, &format!("/api/games/{id}/at?from={total}&to={}", total + 2)).await;
    let (one_status, one) = get_as(&app, RED, &format!("/api/games/{id}/at/{}", total + 1)).await;
    assert_eq!(status, one_status, "{past} vs {one}");
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(past["error"].as_str().unwrap().starts_with(&format!("this game has {total} events, so its last position is {total}")), "{past}");
    let (status, bad) = get_as(&app, RED, &format!("/api/games/{id}/at?from=2&to=1")).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "from > to is the same 404: {bad}");
}

#[tokio::test]
async fn a_run_over_the_cap_is_a_400_naming_it() {
    let (app, id) = playing_game_with_events(1).await;
    let (status, v) = get_as(&app, RED, &format!("/api/games/{id}/at?from=0&to=65")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{v}");
    assert_eq!(v["error"], "a run is at most 64 positions — ask for from=0&to=64 and page");
    let (status, v) = get_as(&app, RED, &format!("/api/games/{id}/at?from=x&to=1")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(v["error"].as_str().unwrap().starts_with("Failed to deserialize query string"), "axum's words: {v}");
}
```

`get_as`, `RED`, `playing_game_with_events` and `events_of` are the module's existing helpers or one-line wrappers over them — read the `at_k` cells and reuse EXACTLY their helper names (`grep -n "async fn get_as\|fn events_of\|async fn playing" crates/yokohama-web/src/lib.rs`); if `playing_game_with_events` does not exist, write it as "create the 2P table `red`/`blue`, both keep card 1, then `POST …/input` `"1"` alternating seats until `events >= n`" using the helpers the existing whole-game cells use.

Add the admission row in `admission_routes()` (`lib.rs:2881-2903`): `("GET", "/at?from=0&to=1", None, Kind::Read),` beside `("GET", "/at/0", None, Kind::Read)` — the grid then proves the run route is admitted exactly as `at/{k}` is (six tables × three identities) with no new cell.

In `games.rs`'s test module, beside the M6.5a archive-site cells (`grep -n "play_to_the_end" crates/yokohama-web/src/games.rs`):

```rust
/// R-6.5b.13: a FINISHED game with no stamp deleted by its creator lands under
/// `over/`, because the mirror says over — and the cell's doc says the mirror
/// can be one move stale in one race, which is the residual the spec names.
#[test]
fn a_finished_unstamped_game_deleted_lands_under_over() {
    let games = games();
    let red = who("red@t.test");
    let id = games.create(&two_people("late", CORPUS_SEED), &red).expect("a table").id;
    games.keep(&id, &red, PlayerId::new(0), 1, None).unwrap();
    games.keep(&id, &who("blue@t.test"), PlayerId::new(1), 1, None).unwrap();
    let (_, repl) = play_to_the_end(&games, &id); // archive OFF, so no stamp lands
    assert!(repl.session.is_over());
    let rec = RecordingArchive::recording();
    let games = games.with_archive(rec.clone());
    games.delete(&id, &red).expect("the creator's delete");
    let puts = rec.puts.lock().unwrap();
    assert_eq!(puts.len(), 1);
    assert_eq!(puts[0].0, crate::archive::Why::Over, "a finished game is archived as OVER at its delete, not as abandoned");
}
```

(`play_to_the_end` and `RecordingArchive` are `test_support`'s, M6.5a §1.7; `with_archive` consumes and returns `Games`.)

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p yokohama-web --lib a_run_` then `cargo test -p yokohama-web --lib a_finished_unstamped`
Expected: the three run cells FAIL with 404 "no such endpoint" (the route does not exist); the leaf cell FAILS on `Why::Abandoned`. (The cap cell's `to=65` on a one-event game reads the cap's 400 by the precedence the handler states.)

- [ ] **Step 3: Split `HistoricalFrame::at` into `from_state` + `at`, add `run`**

In `client.rs`, replace `pub fn at(...)` with:

```rust
/// Project `repl`'s record at `k`. **The caller range-checks `k` first** (the
/// doc that was here is unchanged: `state_at`'s error cannot tell past-the-end
/// from a failing event).
pub fn at(repl: &Repl, k: usize, viewer: Viewer) -> Result<Self, yokohama_core::ReplayError> {
    let state = yokohama_core::state_at(repl.session.record(), k, repl.session.rules())?;
    Ok(Self::from_state(repl, k, &state, viewer))
}

/// The frames of every position in `[from, to)`, by ONE forward pass: the
/// state at `from` once, then one event applied per position (R-6.5b.10).
/// `at(k)` for each `k` would replay the record from its start `to - from`
/// times; this replays it once. The caller range-checks `from` and `to`.
pub fn run(repl: &Repl, from: usize, to: usize, viewer: Viewer) -> Result<Vec<Self>, yokohama_core::ReplayError> {
    let record = repl.session.record();
    let rules = repl.session.rules();
    let mut state = yokohama_core::state_at(record, from, rules)?;
    let mut frames = Vec::with_capacity(to.saturating_sub(from));
    for k in from..to {
        frames.push(Self::from_state(repl, k, &state, viewer));
        if k + 1 < to {
            state = yokohama_core::replay_from(&state, &record.events[k..k + 1], rules)?;
        }
    }
    Ok(frames)
}

/// The projection of one position, given its state — what `at` and `run` share.
pub fn from_state(repl: &Repl, k: usize, state: &yokohama_core::GameState, viewer: Viewer) -> Self {
    let session = &repl.session;
    let record = session.record();
    let total = record.events.len();
    let rules = session.rules();
    let seat = built_for(state, viewer);
    // (the body of the old `at` from `let legal = …` to the end, with `&state`
    //  → `state` and `Ok(Self {…})` → `Self {…}`; nothing else changes)
    …
}
```

Move the old body verbatim; the only edits are the borrow of `state` and dropping the `Ok`.

- [ ] **Step 4: The route**

In `api.rs`, beside `EventsQuery`:

```rust
/// The batch of positions `[from, to)` (R-6.5b.10). Both required — a run has
/// two ends — and `deny_unknown_fields` through `parse_query`, so a misspelled
/// bound is a 400 in axum's words rather than a run from zero.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunQuery {
    pub from: usize,
    pub to: usize,
}

/// The most positions one call answers. Above it the client pages; a clamp
/// would read as "the run ended" (the M6.3 `wait` rule).
pub const RUN_CAP: usize = 64;

#[derive(Debug, Clone, Serialize)]
pub struct RunFrames {
    pub frames: Vec<client::HistoricalFrame>,
}

/// `GET /api/games/{id}/at?from=&to=` — the frames of `[from, to)` in one reply.
///
/// **The boundary in numbers** (spec §2.4): a game with `total` events has
/// `total + 1` positions, `0..=total`, so the largest admitted `to` is
/// `total + 1` — the client's every call sends exactly that — and the 404
/// begins at `to = total + 2`, with `at/{k}`'s own sentence extended to name
/// the range. `from > to` is DELIBERATELY the same 404 rather than a third
/// answer. Admitted as `at_k` is; the lock is held across the body for the
/// same reason `at_k` holds it.
pub fn run(games: &Games, user: &User, id: &str, q: &RunQuery) -> Result<RunFrames, ApiError> {
    let (_table, viewer) = admitted(games, id, user)?;
    let handle = games.handle(id)?;
    let repl = handle.lock().expect("a game");
    let total = repl.session.record().events.len();
    // The cap FIRST (spec §2.4): a run longer than the cap is a malformed ask
    // whatever `total` is, so a one-event game asked for `to=65` reads this
    // sentence and not the range's.
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
        ApiError::server_error(format!("replaying this game failed at event {}: {}", e.event_index, e.message))
    })?;
    Ok(RunFrames { frames })
}
```

In `routes()`, insert `("GET", "/api/games/{id}/at"),` immediately BEFORE `("GET", "/api/games/{id}/at/{k}")`. In `answer`, insert before the `at/{k}` arm:

```rust
"/api/games/{id}/at" => {
    run(games, u, b[0], &parse_query(req.query)?).map(|v| ok_json(200, &v))
}
```

`matches` is segment-wise (spec §0), so the five-segment pattern and the six-segment one coexist and `app()`'s catch-all reaches it with no axum registration. Re-export `RunQuery`, `RunFrames`, `RUN_CAP` from `lib.rs` beside `EventsQuery` (`grep -n "pub use api::" crates/yokohama-web/src/lib.rs`).

- [ ] **Step 5: The leaf at the abandon site**

In `tables.rs`'s `delete`, replace `crate::archive::Why::Abandoned` in the `put` call with a computed `why`:

```rust
// R-6.5b.13: the leaf follows the MIRROR's truth. `turn` is a cache
// `mirror_turn` may leave one move stale when its write conflicts, so in
// exactly that race the leaf can still lie; `Games::handle(id).is_over` would
// not, at the cost of a rebuild and a replay under this lock, and was declined.
let why = if doc.turn.as_ref().is_some_and(|t| t.over) {
    crate::archive::Why::Over
} else {
    crate::archive::Why::Abandoned
};
```

and pass `why`. The comment above the site ("Abandon and save …") gains one sentence: "a finished game with no stamp lands under `over/`".

- [ ] **Step 6: Run the crate, then the corpus**

Run: `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings && cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`
Expected: the four new cells pass; the admission grid passes with its new row; 247 + 4 lib.

Add the three captures in `wire_fixtures.rs` beside `at_k` (`:1432`):

```rust
out.push(("at_k".to_string(), api.get(&format!("/api/games/wire/at/{}", total / 2)).await));
// R-6.5b.10: the same stop as `at_k`, as a run of three ending there, and
// the two refusals the client renders.
out.push(("run".to_string(), api.get(&format!("/api/games/wire/at?from={}&to={}", total / 2 - 3, total / 2)).await));
let (status, cap) = api.call("GET", "/api/games/wire/at?from=0&to=65", None).await;
assert_eq!(status, StatusCode::BAD_REQUEST, "{cap}");
out.push(("run_400_cap".to_string(), cap));
let (status, range) = api.call("GET", &format!("/api/games/wire/at?from=0&to={}", total + 2), None).await;
assert_eq!(status, StatusCode::NOT_FOUND, "{range}");
out.push(("run_404_range".to_string(), range));
```

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures && git status --short web/src/wire/`
Expected diff: THREE new files (`run.json`, `run_400_cap.json`, `run_404_range.json`), nothing else. Any other change is a finding.

- [ ] **Step 7: The client — `api.run`, `fetchRun` as one paged call, the types**

`web/src/types.ts`, beside `HistoricalFrame`:

```ts
/** The frames of `[from, to)` in one reply (R-6.5b.10). */
export interface RunFrames {
  frames: HistoricalFrame[];
}
```

`web/src/wire/contract.ts`: `import run from "./run.json";` and a row `const RUN = ["frames"] as const;` with its `KeysOf<RunFrames, typeof RUN>` line and `run: RUN` in the map, in the shape the file uses for every other payload (copy `at_k`'s three lines).

`web/src/api.ts`, in the `api` object after `at`:

```ts
/** The most positions one run call answers (the server's `RUN_CAP`); above
 *  it `fetchRun` pages. */
export const RUN_CAP = 64;
…
  /** The frames of `[from, to)` — one forward replay on the server instead of
   *  one `at/{k}` per position (R-6.5b.10). `to` may be `events + 1`: the run
   *  ends on the live frame. */
  run: (game: string, from: number, to: number) =>
    call<RunFrames>("GET", `/api/games/${id(game)}/at?from=${from}&to=${to}`),
```

`web/src/App.tsx:55-62` — replace `fetchRun`:

```ts
async function fetchRun(game: string, prev: number, next: number): Promise<Run> {
  // The route is half-open and the run ends on the LIVE frame, so `to` is
  // `next + 1`; a run longer than the cap is paged (a spectator reconnecting
  // after a long absence — one call is the common case).
  const frames: HistoricalFrame[] = [];
  for (let from = prev; from <= next; from += RUN_CAP) {
    const to = Math.min(next + 1, from + RUN_CAP);
    frames.push(...(await api.run(game, from, to)).frames);
  }
  const log = await api.log(game, next - prev);
  return buildRun(prev, frames, log);
}
```

(import `RUN_CAP` and `HistoricalFrame`.) In `web/src/App.test.tsx` find the cells that stub `api.at` for a walk (`grep -n "api.at\|/at/" web/src/App.test.tsx`) and re-point them at `api.run` answering `run.json`'s frames; add one cell: a run of 70 positions makes TWO `run` calls (`from=0,to=64` then `from=64,to=71`) and one `log` call — the paging claim.

Run: `cd web && npm run typecheck && npm test`
Expected: green; the walk cells now spy `run`, not `at`.

- [ ] **Step 8: Commit**

```bash
git add crates/yokohama-web/src/client.rs crates/yokohama-web/src/api.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/games.rs crates/yokohama-web/tests/wire_fixtures.rs web/src/wire/run.json web/src/wire/run_400_cap.json web/src/wire/run_404_range.json web/src/api.ts web/src/App.tsx web/src/App.test.tsx web/src/types.ts web/src/wire/contract.ts
git commit -m "feat(web): GET …/at?from=&to= — a run of frames by one forward replay, the client's walk as one paged call; delete's leaf follows the mirror (R-6.5b.10, R-6.5b.13)"
```

---

### Task 2: `POST /api/archive` — the caller prefix on the trait, `caller_segment`, `agrees_with`, the five checks (R-6.5b.6, R-6.5b.15)

**Files:**
- Modify: `crates/yokohama-web/src/archive.rs` (the trait, `paths`, `caller_segment`, `PutError`, `DirArchive::put`, `NoArchive::put`)
- Modify: `crates/yokohama-web/src/games.rs` (`archive_if_over`'s `put` call), `crates/yokohama-web/src/tables.rs` (`delete`'s `put` call; `check_shape` → `pub(crate)`)
- Modify: `crates/yokohama-web/src/table.rs` (`TableDoc::agrees_with`)
- Modify: `crates/yokohama-web/src/api.rs` (`ArchivePost`, `ArchiveReply`, `archive_post`, the route row and arm)
- Modify: `crates/yokohama-web/src/lib.rs` (re-exports; `AppState::with_store_chat_and_archive` is already there)
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs` (a second `Api` with a `DirArchive`; seven captures)
- Test: `archive.rs`, `table.rs`, `lib.rs` test modules

**Interfaces:**
- Produces: `pub enum PutOutcome { Landed, Identical }`, `pub enum PutError { Conflict(String), Failed(String) }` (+ `Display`); `trait Archive { fn put(&self, prefix: &str, why: Why, id: &str, record_json: &str, table_json: &str) -> Result<PutOutcome, PutError>; fn kind(&self) -> &'static str; }` — **the table is written first, the record's path is the once-marker, an identical record answers `Identical` and repairs a half pair, a different record is `Conflict`** (spec §2.4); `pub fn paths(rev: u32, prefix: &str, why: Why, id: &str) -> (String, String)`; `pub fn caller_segment(user: &User) -> String`; `TableDoc::agrees_with(&self, record: &GameRecord) -> Result<(), String>`; `pub struct ArchivePost { pub record: String, pub table: String }`; `pub struct ArchiveReply { pub leaf: &'static str, pub path: String }`; `pub fn archive_post(games: &Games, user: &User, req: &ArchivePost) -> Result<ArchiveReply, ApiError>`; the row `("POST", "/api/archive")`.
- Consumes: `yokohama_core::{GameRecord, replay_strict, is_game_over, SEMANTICS_REV, StartingOrderChoice}`, `store::is_safe_id`, `archive::{is_on, strip_for_archive}`, `tables::check_shape`.

- [ ] **Step 1: Write the failing cells**

In `archive.rs`'s test module:

```rust
#[test]
fn a_caller_segment_is_path_safe_and_readable() {
    let seg = |s: &str| caller_segment(&crate::identity::User::parse(s).unwrap());
    assert_eq!(seg("<the owner's address>"), "<the owner's address, transliterated>"); // scrubbed 2026-09-28 (R-6.5c2.16)
    assert_eq!(seg("A/B@c"), "a_b_at_c", "a slash is replaced, the address lowercased");
    assert_eq!(seg("../../x@y"), ".._.._x_at_y");
    assert_eq!(seg("..@.."), ".._at_..");
    let long = format!("{}@{}", "a".repeat(200), "b".repeat(50));
    assert_eq!(seg(&long).len(), 64, "capped");
    for s in ["../../x@y", "..@..", &long, "a/b@c"] {
        let v = seg(s);
        assert!(!v.is_empty() && v != "." && v != ".." && !v.contains('/'), "{v:?}");
        assert!(v.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-')), "{v:?}");
    }
}

#[test]
fn the_layout_takes_a_prefix_and_both_implementations_agree_under_one() {
    assert_eq!(paths(7, "", Why::Over, "sim-1"), ("rev7/over/sim-1.json".into(), "rev7/over/sim-1.table.json".into()));
    assert_eq!(
        paths(7, "browser/a_at_b/", Why::Abandoned, "local-1"),
        ("browser/a_at_b/rev7/abandoned/local-1.json".into(), "browser/a_at_b/rev7/abandoned/local-1.table.json".into())
    );
}

#[cfg(feature = "server")]
#[test]
fn a_traversal_shaped_caller_writes_inside_the_root() {
    let dir = std::env::temp_dir().join(format!("yokohama_archive_root_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let a = DirArchive::new(&dir);
    let root = std::fs::canonicalize(&dir).unwrap();
    for who in ["../../x@y", "a/b@c", "..@.."] {
        let seg = caller_segment(&crate::identity::User::parse(who).unwrap());
        let prefix = format!("browser/{seg}/");
        a.put(&prefix, Why::Over, "local-1", "{\"r\":1}", "{\"t\":1}").unwrap();
        let (record, _) = paths(yokohama_core::SEMANTICS_REV, &prefix, Why::Over, "local-1");
        let written = std::fs::canonicalize(dir.join(&record)).unwrap();
        assert!(written.starts_with(&root), "{who:?} wrote {written:?} outside {root:?}");
    }
    // The put protocol (spec §2.4): the same bytes again is IDENTICAL, different bytes a CONFLICT.
    let seg = caller_segment(&crate::identity::User::parse("a/b@c").unwrap());
    let prefix = format!("browser/{seg}/");
    assert!(matches!(a.put(&prefix, Why::Over, "local-1", "{\"r\":1}", "{\"t\":1}"), Ok(PutOutcome::Identical)));
    let again = a.put(&prefix, Why::Over, "local-1", "{\"r\":2}", "{\"t\":2}");
    assert!(matches!(again, Err(PutError::Conflict(_))), "{again:?}");
    // A half pair — the table landed, the record did not — is REPAIRED by the retry.
    let (record, table) = paths(yokohama_core::SEMANTICS_REV, &prefix, Why::Abandoned, "local-2");
    std::fs::create_dir_all(dir.join(&table).parent().unwrap()).unwrap();
    std::fs::write(dir.join(&table), "{\"t\":9}").unwrap();
    assert!(matches!(a.put(&prefix, Why::Abandoned, "local-2", "{\"r\":9}", "{\"t\":9}"), Ok(PutOutcome::Landed)));
    assert_eq!(std::fs::read_to_string(dir.join(&record)).unwrap(), "{\"r\":9}");
    std::fs::remove_dir_all(&dir).ok();
}
```

In `table.rs`'s test module (it has `fn locked_doc()`-style builders; add a record builder):

```rust
fn record_for(doc: &TableDoc) -> yokohama_core::GameRecord {
    use yokohama_core::{GameConfig, PlayerId, Rules, StartingOrderChoice};
    let mut config = GameConfig::for_player_count(doc.players);
    config.player_colors = doc.colors.clone();
    config.start_player = PlayerId::new(doc.start_seat);
    // A human keep `Some(n)` is `Keep(n - 1)`; a bot's `None` is whatever the
    // bot kept — `Keep(1)` here — and is NOT compared.
    let choices = doc.keeps.iter().map(|k| StartingOrderChoice::Keep(k.map(|n| n - 1).unwrap_or(1))).collect();
    let mut rec = yokohama_core::GameRecord::new(doc.seed, config, choices, &Rules::standard());
    rec.semantics_rev = Some(yokohama_core::SEMANTICS_REV);
    rec
}

#[test]
fn a_table_agrees_with_its_record_and_a_bot_keep_is_not_compared() {
    let mut doc = playing_doc(); // the module's 2P builder with seats [Human, Bot]; add one if only all-human builders exist
    doc.keeps = vec![Some(2), None];
    assert!(doc.agrees_with(&record_for(&doc)).is_ok(), "the real local pair: one human keep, one bot None");
    let rec = record_for(&doc);
    let mut d = doc.clone(); d.players = 3;
    assert!(d.agrees_with(&rec).unwrap_err().contains("players"));
    let mut d = doc.clone(); d.colors.reverse();
    assert!(d.agrees_with(&rec).unwrap_err().contains("colors"));
    let mut d = doc.clone(); d.start_seat = 1;
    assert!(d.agrees_with(&rec).unwrap_err().contains("start_seat"));
    let mut d = doc.clone(); d.seed += 1;
    assert!(d.agrees_with(&rec).unwrap_err().contains("seed"));
    let mut d = doc.clone(); d.keeps[0] = Some(1);
    assert!(d.agrees_with(&rec).unwrap_err().contains("keep"), "a CLAIMED keep that disagrees is refused");
}
```

(`GameRecord::new(seed, config, choices, &rules)` is the constructor `record.rs`'s own tests use, spec §0 — check its exact parameter order there and match it.)

In `lib.rs`'s test module, the route cells over an `AppState::with_store_chat_and_archive(MemStore, Claimed, ChatConfig::default(), Arc::new(DirArchive::new(tempdir)))`:

```rust
/// R-6.5b.6: the five refusals by five DIFFERENT sentences, the leaf from the
/// replayed state, a partial record ACCEPTED, once per path, the prefix, the
/// server-side strip.
#[tokio::test]
async fn the_archive_route_checks_and_files_a_browser_game() {
    let dir = tempdir("archive_route");
    let app = archive_app(&dir); // AppState with a DirArchive over `dir`, claimed identity
    let (record_text, table_text) = finished_local_pair(); // a 2P you+bot game played to the end in process (Games::create + keep + pump, then the store's two strings — the helper `play_to_the_end` in games::test_support is the model; this one returns the STRINGS)
    let body = |r: &str, t: &str| json!({ "record": r, "table": t });

    // The accepted send, and where it landed.
    let (status, v) = post_as(&app, RED, "/api/archive", body(&record_text, &table_text)).await;
    assert_eq!(status, StatusCode::CREATED, "{v}");
    assert_eq!(v["leaf"], "over");
    assert_eq!(v["path"], "browser/red_at_wire.test/rev7/over/local-pair.json");
    let landed = std::fs::read_to_string(dir.join("browser/red_at_wire.test/rev7/over/local-pair.json")).unwrap();
    assert_eq!(landed, record_text, "the record lands as sent — its bytes are the digest's");
    let table_landed = std::fs::read_to_string(dir.join("browser/red_at_wire.test/rev7/over/local-pair.table.json")).unwrap();
    assert!(!table_landed.contains("hunter2-salt"), "re-stripped server-side: the lock the body carried is gone");

    // The same bytes again: 200 `identical`, never a 409 — a lost reply's retry is safe to act on.
    let (status, v) = post_as(&app, RED, "/api/archive", body(&record_text, &table_text)).await;
    assert_eq!(status, StatusCode::OK, "{v}");
    assert_eq!(v["identical"], true);
    // A DIFFERENT record under the same id: the 409 the client keeps the game on.
    let (status, v) = post_as(&app, RED, "/api/archive", body(&with_first_event_replaced_and_restamped(&record_text), &table_text)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{v}");
    assert_eq!(v["error"], "a different record is archived under this id");

    // A PARTIAL record is accepted under abandoned/ — the leaf from the state.
    let partial = partial_of(&record_text, 5); // truncate to 5 events and re-stamp the digest via state_at + stamp_terminal_digest
    let (status, v) = post_as(&app, RED, "/api/archive", body(&partial, &table_text)).await;
    assert_eq!(status, StatusCode::CREATED, "{v}");
    assert_eq!(v["leaf"], "abandoned");

    // The five refusals, five sentences. (A fresh id each time so the once-rule does not fire first.)
    let refusals = [
        (with_rev(&record_text, 6), &table_text, "this record is at semantics rev 6; this server archives rev 7"),
        (without_rev(&record_text), &table_text, "missing semantics provenance"),
        (without_digest(&record_text), &table_text, "the record carries no terminal digest"),
        (with_first_event_replaced(&record_text), &table_text, "the record does not replay"),
        (record_text.clone(), &with_players(&table_text, 3), "the table document does not agree with the record"),
    ];
    let mut sentences = std::collections::BTreeSet::new();
    for (r, t, expect) in refusals {
        let (status, v) = post_as(&app, RED, "/api/archive", body(&r, t)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{v}");
        let s = v["error"].as_str().unwrap().to_string();
        assert!(s.starts_with(expect), "{s:?} does not start with {expect:?}");
        sentences.insert(s);
    }
    assert_eq!(sentences.len(), 5, "five different sentences");
}

#[tokio::test]
async fn the_archive_route_is_a_404_where_no_archive_is_kept_and_a_401_with_no_identity() {
    let app = app_in_memory(); // NoArchive
    let (record_text, table_text) = finished_local_pair();
    let (status, v) = post_as(&app, RED, "/api/archive", json!({ "record": record_text, "table": table_text })).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{v}");
    assert_eq!(v["error"], "this server keeps no archive");
    let (status, _) = post_anonymous(&app, "/api/archive", json!({ "record": "{}", "table": "{}" })).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
```

The five body-editing helpers (`with_rev`, `without_rev`, `without_digest`, `with_first_event_replaced`, `with_players`) parse the text with `serde_json::from_str::<Value>`, edit one key, and re-serialize with `to_string`; `partial_of` deserializes to `GameRecord`, truncates `events`, computes `state_at(&rec, n, &rules)` and calls `rec.stamp_terminal_digest(&state)` (`record.rs:302`), then serializes. The pair's table document carries a `lock` (`Lock::new("hunter2")`, whose salt the strip assertion searches for by the hex the test reads off `doc.lock.salt` — replace `"hunter2-salt"` with that value).

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p yokohama-web --lib a_caller_segment the_layout_takes a_traversal a_table_agrees the_archive_route`
Expected: FAIL to compile (`caller_segment`, `PutError`, the prefix, `agrees_with`, the route do not exist).

- [ ] **Step 3: `archive.rs`**

```rust
/// What a put did (spec §2.4): wrote both files, or found the SAME record already there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PutOutcome {
    Landed,
    /// The record at this path holds these very bytes: a retry after a lost
    /// reply. A half pair (the table alone) is repaired on the way to `Landed`.
    Identical,
}

/// Why a put was refused: a DIFFERENT record is at this path, or the write failed.
#[derive(Debug)]
pub enum PutError {
    Conflict(String),
    Failed(String),
}
impl std::fmt::Display for PutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict(p) => write!(f, "{p} holds a different record — a game is archived once"),
            Self::Failed(e) => f.write_str(e),
        }
    }
}

pub trait Archive: Send + Sync {
    /// `prefix` is `""` for the server's own games and `browser/<segment>/`
    /// (R-6.5b.15) for a browser game sent by a caller; it is the FIRST path
    /// component and every implementation joins it through [`paths`]. The
    /// TABLE is written first and the RECORD's path is the once-marker.
    fn put(&self, prefix: &str, why: Why, id: &str, record_json: &str, table_json: &str) -> Result<PutOutcome, PutError>;
    fn kind(&self) -> &'static str;
}

/// The ONE layout: `<prefix>rev<N>/<over|abandoned>/<id>.json` + `.table.json`.
pub fn paths(rev: u32, prefix: &str, why: Why, id: &str) -> (String, String) {
    let base = format!("{prefix}rev{rev}/{}/{id}", why.dir());
    (format!("{base}.json"), format!("{base}.table.json"))
}

/// R-6.5b.15: a path-safe, READABLE rendering of a caller's address. `@` →
/// `_at_`, anything outside `[a-z0-9._-]` → `_`, capped at 64. The segment is
/// at least five characters of that alphabet with no `/` (both halves of an
/// address are non-empty and the separator is four characters; the cap only
/// shortens), so it is never empty, `.` or `..`. It COLLIDES (`a/b` and `a_b`;
/// two addresses past the cap), which spec §8 records.
pub fn caller_segment(user: &crate::identity::User) -> String {
    let mut out = String::new();
    for c in user.as_str().chars() {
        match c {
            '@' => out.push_str("_at_"),
            c if c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-') => out.push(c),
            _ => out.push('_'),
        }
    }
    out.truncate(64);
    out
}
```

`NoArchive::put` takes the new signature and answers `Err(PutError::Failed("no archive is configured".into()))`. `DirArchive::put`:

```rust
fn put(&self, prefix: &str, why: Why, id: &str, record_json: &str, table_json: &str) -> Result<PutOutcome, PutError> {
    let (record, table) = paths(yokohama_core::SEMANTICS_REV, prefix, why, id);
    let record_path = self.dir.join(&record);
    // The record is the marker: the same bytes are a retry, different bytes a conflict.
    if let Ok(existing) = std::fs::read_to_string(&record_path) {
        return if existing == record_json { Ok(PutOutcome::Identical) } else { Err(PutError::Conflict(record_path.display().to_string())) };
    }
    // Table FIRST (overwriting a half pair's table — a repair), then the record.
    Self::write_over(&self.dir.join(&table), table_json)?;
    Self::write_over(&record_path, record_json)?;
    Ok(PutOutcome::Landed)
}
```

with `write_over` the old `write_new` minus its existence check (temp + rename, `Failed(…)` for the mkdir/write/rename errors). The existing directory cell's "a second put is refused" arm becomes the `Identical`/`Conflict` pair above.

The two call sites: `games.rs` `archive_if_over` — `self.archive.put("", Why::Over, id, record_json, &table_json)`; its `Err(e)` arm logs `{e}` (Display). `tables.rs` `delete` — `.put("", why, id, &record_json, &table_json).map_err(|why| GameError::Server(format!("the archive could not be written ({why}) — the table was not deleted")))`.

- [ ] **Step 4: `TableDoc::agrees_with`** (in `table.rs`, `impl TableDoc`):

```rust
/// R-6.5b.6 check (4): this document describes `record`'s game. Compared:
/// the player count, the colours, the start seat, the seed, and every keep
/// the document CLAIMS (`Some(n)`, 1-based) against `choices[i] == Keep(n - 1)`.
/// **A `None` keep is not compared**: a bot seat's keep lives in the record and
/// never here (`create` fills `keeps` with `None`, only a seat holder may
/// `keep`, `choices_of` recomputes the bot's at finalize), so every real local
/// pair reads `[Some(n), None]` against two `Keep`s.
pub fn agrees_with(&self, record: &yokohama_core::GameRecord) -> Result<(), String> {
    if usize::from(self.players) != usize::from(record.config.player_count) {
        return Err(format!("players: the table says {}, the record {}", self.players, record.config.player_count));
    }
    if self.colors != record.config.player_colors {
        return Err("colors: the table and the record disagree".to_string());
    }
    if self.start_seat != record.config.start_player.0 {
        return Err(format!("start_seat: the table says {}, the record {}", self.start_seat, record.config.start_player.0));
    }
    if self.seed != record.seed {
        return Err("seed: the table and the record disagree".to_string());
    }
    for (i, keep) in self.keeps.iter().enumerate() {
        if let Some(n) = keep {
            let expected = yokohama_core::StartingOrderChoice::Keep(n - 1);
            if record.choices.get(i) != Some(&expected) {
                return Err(format!("keep: seat {i} kept card {n} by the table and not by the record"));
            }
        }
    }
    Ok(())
}
```

(`GameConfig`'s fields are `player_count`, `player_colors`, `start_player: PlayerId` with a public `.0` — `tables.rs:752-754` reads them exactly so; `Keep(n - 1)` is the expression `choices_of` uses at `tables.rs:506`, whose operand type it settles.)

- [ ] **Step 5: The route** (in `api.rs`, beside `record`):

```rust
/// What a browser sends to the archive (R-6.5b.6): the record and the local
/// table document as TWO STRINGS, parsed here and never by the client.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivePost {
    pub record: String,
    pub table: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchiveReply {
    pub leaf: &'static str,
    pub path: String,
    /// The record at that path already held these bytes (a retry): 200, not 201.
    pub identical: bool,
}

/// `POST /api/archive`. In order: the rev, the digest, the strict replay, the
/// table's shape and its agreement with the record, then the leaf FROM THE
/// REPLAYED STATE, then one put under `browser/<caller_segment>/` with the
/// table re-stripped server-side. 404 where this host keeps no archive.
pub fn archive_post(games: &Games, user: &User, req: &ArchivePost) -> Result<ArchiveReply, ApiError> {
    use crate::archive::{caller_segment, is_on, paths, strip_for_archive, PutError, Why};
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
        None => {
            // `replay_strict`'s own sentence, carried verbatim.
            let e = yokohama_core::replay_strict(&record, games.rules()).expect_err("a record with no rev is refused");
            return Err(ApiError::bad_request(e.message));
        }
    }
    if record.terminal_digest.is_none() {
        return Err(ApiError::bad_request("the record carries no terminal digest"));
    }
    let state = yokohama_core::replay_strict(&record, games.rules())
        .map_err(|e| ApiError::bad_request(format!("the record does not replay: at event {}, {}", e.event_index, e.message)))?;
    let doc: crate::table::TableDoc = serde_json::from_str(&req.table)
        .map_err(|e| ApiError::bad_request(format!("the table document does not parse: {e}")))?;
    crate::tables::check_shape(&doc)
        .map_err(|why| ApiError::bad_request(format!("the table document is malformed: {why}")))?;
    doc.agrees_with(&record)
        .map_err(|why| ApiError::bad_request(format!("the table document does not agree with the record: {why}")))?;
    if !crate::store::is_safe_id(&doc.id) {
        return Err(ApiError::bad_request("the table's id is not a safe name"));
    }
    let why = if yokohama_core::is_game_over(&state) { Why::Over } else { Why::Abandoned };
    let prefix = format!("browser/{}/", caller_segment(user));
    let (path, _) = paths(yokohama_core::SEMANTICS_REV, &prefix, why, &doc.id);
    match games.archive.put(&prefix, why, &doc.id, &req.record, &strip_for_archive(&doc)) {
        Ok(PutOutcome::Landed) => Ok(ArchiveReply { leaf: why.dir(), path, identical: false }),
        Ok(PutOutcome::Identical) => Ok(ArchiveReply { leaf: why.dir(), path, identical: true }),
        Err(PutError::Conflict(_)) => Err(ApiError { status: StatusCode::CONFLICT, message: "a different record is archived under this id".to_string(), table_status: None }),
        Err(PutError::Failed(e)) => Err(ApiError::server_error(format!("the archive could not be written: {e}"))),
    }
}
```

`ArchiveReply` gains `pub identical: bool`; the `answer` arm answers **201 when `!identical` and 200 when `identical`**:

```rust
"/api/archive" => archive_post(games, u, &parse_json(req.content_type, req.body)?)
    .map(|v| ok_json(if v.identical { 200 } else { 201 }, &v)),
}
```

`Why::dir` becomes `pub(crate)`; `tables::check_shape` becomes `pub(crate)`; `Games::archive` is already `pub(crate)`; `GameRecord.terminal_digest` is the `Option<String>` field `stamp_terminal_digest` fills (confirm the name at `record.rs:149-190`). Route row `("POST", "/api/archive")` after `("POST", "/api/chat")`; `answer` arm:

```rust
"/api/archive" => archive_post(games, u, &parse_json(req.content_type, req.body)?).map(|v| ok_json(201, &v)),
```

Re-export `ArchivePost`, `ArchiveReply`, `PutOutcome`, `PutError` from `lib.rs`. (Drop the earlier `answer` arm sketch above that always answered 201.)

- [ ] **Step 6: Run the crate; the corpus on a SECOND `Api`**

Run: `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings && cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`
Expected: all new cells pass; M6.5a's archive cells pass with `""` prefixes.

In `wire_fixtures.rs`: add `Api::with_archive(dir: &Path) -> Self` building `AppState::with_store_chat_and_archive(Arc::new(MemStore::default()), IdentityMode::Claimed, ChatConfig::default(), Arc::new(yokohama_web::archive::DirArchive::new(dir)))`. At the END of `walk_with` (after `game_over`/`result` are captured, where the walk's game is finished), read the pair off the PRIMARY api — `let (_, record_v) = api.call_as(Some(RED_SEAT_HOLDER), "GET", "/api/games/wire/record", None).await;` (the record route answers the seat holder at game over; use the identity the walk's seat 0 plays as) and `let table_text = serde_json::to_string(&api.state.games.read_table("wire").unwrap().unwrap().0).unwrap();` — then on `let archived = Api::with_archive(&tempdir_for_this_run())`:

```rust
let record_text = serde_json::to_string(&record_v).unwrap();
let body = |r: &str, t: &str| Some(json!({ "record": r, "table": t }));
let (status, archive_post) = archived.call("POST", "/api/archive", body(&record_text, &table_text)).await;
assert_eq!(status, StatusCode::CREATED, "{archive_post}");
let (status, archive_identical) = archived.call("POST", "/api/archive", body(&record_text, &table_text)).await;
assert_eq!(status, StatusCode::OK, "{archive_identical}");
let (status, archive_409) = archived.call("POST", "/api/archive", body(&with_first_event_replaced_and_restamped(&record_text), &table_text)).await;
assert_eq!(status, StatusCode::CONFLICT, "{archive_409}");
let partial = partial_of(&record_text, 5);
let (status, archive_post_abandoned) = archived.call("POST", "/api/archive", body(&partial, &table_text)).await;
assert_eq!(status, StatusCode::CREATED, "{archive_post_abandoned}");
// the four 400s, each on a body edited one way (the same five helpers as lib.rs's cell, copied here — the test binaries do not share code)
…
out.extend([
    ("archive_post".to_string(), archive_post),
    ("archive_post_identical".to_string(), archive_identical),
    ("archive_post_abandoned".to_string(), archive_post_abandoned),
    ("archive_409".to_string(), archive_409),
    ("archive_400_rev".to_string(), archive_400_rev),
    ("archive_400_digest".to_string(), archive_400_digest),
    ("archive_400_replay".to_string(), archive_400_replay),
    ("archive_400_table".to_string(), archive_400_table),
]);
```

The `path` in `archive_post.json` reads `browser/red_at_wire.test/rev7/over/wire.json` — deterministic. `Api::call_as` on the second instance records its own script (Task 7 keeps only the primary's).

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures && git status --short web/src/wire/`
Expected diff: EIGHT new files, `health.json` UNCHANGED (the primary `Api` keeps `NoArchive`). (`with_first_event_replaced_and_restamped` replaces the first event with a different legal move and re-stamps the digest through `state_at` + `stamp_terminal_digest`, so the record REPLAYS and differs — the conflict case; the plain `with_first_event_replaced` of the 400 cell leaves the digest stale, which is the replay-refusal case.) Then `cd web && npm run typecheck` — add `ArchiveReply` to `types.ts` (`{ leaf: "over" | "abandoned"; path: string }`) and the `archive_post` row to `contract.ts`; the refusals are `{ error }` and join the untyped-refusal list beside `record_409`.

- [ ] **Step 7: Commit**

```bash
git add crates/yokohama-web/src/archive.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/src/table.rs crates/yokohama-web/src/api.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/tests/wire_fixtures.rs web/src/wire/archive_*.json web/src/types.ts web/src/wire/contract.ts
git commit -m "feat(web): POST /api/archive — a browser game strict-replayed, filed under a path-safe caller prefix by the leaf the replay shows; the archive trait takes the prefix (R-6.5b.6, R-6.5b.15)"
```

---

### Task 3: `crates/yokohama-wasm` — the six exports, the clock, its own scan; the build; the `.wasm` MIME arm; CI (R-6.5b.9, R-6.5b.14)

**Files:**
- Create: `crates/yokohama-wasm/Cargo.toml`, `crates/yokohama-wasm/src/lib.rs`
- Modify: `Cargo.toml` (members), `.gitignore`, `crates/yokohama-web/src/games.rs` (`load_document`, `read_document`), `crates/yokohama-web/src/assets.rs:49-63,124-130`, `web/package.json`, `.github/workflows/test.yml`
- Test: `crates/yokohama-wasm/src/lib.rs` test module, `assets.rs` cell

**Interfaces:**
- Produces (JS, via wasm-bindgen): `init(): void`; `dispatch(user: string | undefined, method: string, path: string, query: string, content_type: string | undefined, body: Uint8Array): Reply` where `Reply` has getters `status: number`, `body: string`; `load(doc: string, id: string, text: string): void` (throws on a bad doc name or a present document); `read(doc: string, id: string): string | undefined`; `set_clock(seconds: number | undefined): void`. Rust: `Games::load_document(&self, doc: Doc, id: &str, text: &str) -> Result<(), String>`, `Games::read_document(&self, doc: Doc, id: &str) -> Result<Option<String>, String>`. Scripts: `npm run build:wasm` → `web/src/local/wasm/yokohama_wasm.js` + `_bg.wasm` + `.d.ts` (target `web`, gitignored, bundled by Vite) and `web/wasm-node/` (target `nodejs`, with a CommonJS `package.json`); `npm run build:wasm:debug` is the same with the `debug` feature (the e2e's trap hook). Rust: `dispatch_inner`, `load_inner`, `read_inner` are the pure functions the exports wrap and the native cells call.
- Consumes: `yokohama_web::{api::{dispatch, ApiRequest, Host}, games::Games, store::{Doc, MemStore}, chat::ChatConfig, signal::NoSignal, archive::NoArchive, identity::User}`, `yokohama_core::Rules`.

- [ ] **Step 1: The two `Games` accessors and the MIME arm, with their cells**

In `games.rs` (unconditional, beside `rules()`):

```rust
/// A host's side channel INTO the store (R-6.5b.4): a document written with no
/// version — a CREATE, refused if present — for a Worker filling a fresh store
/// at boot or on a lock acquisition. Bypasses this type on purpose: the pair
/// is a state the player was shown, and the `Repl` is rebuilt from it on first
/// touch exactly as an absent handle is.
pub fn load_document(&self, doc: Doc, id: &str, text: &str) -> Result<(), String> {
    self.store.write_if_version(doc, id, text, None).map(|_| ()).map_err(|e| e.to_string())
}

/// The side channel OUT: the store's exact bytes, for persistence and the file save.
pub fn read_document(&self, doc: Doc, id: &str) -> Result<Option<String>, String> {
    self.store.read(doc, id).map(|r| r.map(|(text, _)| text))
}
```

(`write_if_version` returns `Result<Version, WriteError>` — check `store.rs:133` and map accordingly.) Cell in `games.rs`: `load_document` twice on one id → the second is `Err`; `read_document` returns the first's text byte-for-byte.

In `assets.rs`: `Some("wasm") => "application/wasm",` and in `content_types_cover_what_a_vite_build_emits`: `assert_eq!(content_type(Path::new("wasm/yokohama_wasm.wasm")), "application/wasm");`.

Run: `cargo test -p yokohama-web --lib load_document content_types` — the MIME assertion FAILS first (`application/octet-stream`), then passes with the arm.

- [ ] **Step 2: The crate**

`Cargo.toml` (workspace): add `"crates/yokohama-wasm",` to `members`. `.gitignore`: add

```
# M6.5b: the browser host's module, built by `npm run build:wasm` (two targets). The web
# glue is generated INTO a source directory so Vite bundles it (spec §2.1, R7).
web/src/local/wasm/
web/wasm-node/
```

`crates/yokohama-wasm/Cargo.toml`:

```toml
[package]
name = "yokohama-wasm"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true
# Bundles nothing third-party, but the layer it wraps is `publish = false` for ASSETS.md's reason.
publish = false

[lib]
# `cdylib` is the module; `rlib` is what lets `cargo test -p yokohama-wasm` run the
# crate's own cells natively (the scan, the identity mapping, the clock modes).
crate-type = ["cdylib", "rlib"]

[dependencies]
# The layer WITHOUT its server half (R-6.5b.9): axum, tokio and tokio-stream never enter this graph.
yokohama-web = { path = "../yokohama-web", default-features = false }
yokohama-core = { path = "../yokohama-core" }
serde_json.workspace = true
# The lock's version (M6.5a pulled it in through getrandom's `js` feature); `wasm-bindgen-cli` matches it.
wasm-bindgen = "=0.2.128"
js-sys = "0.3"

[features]
# The e2e build's trap hook (spec §6.5): never in the release module.
debug = []
```

`crates/yokohama-wasm/src/lib.rs`:

```rust
//! The browser host (M6.5b, R-6.5b.9): ONE `Games` over `MemStore`, `NoSignal`,
//! `NoArchive` and a settable clock, behind six `wasm-bindgen` exports. This is
//! HOST code — its platform calls are `js_sys::Date::now` and the bindgen glue
//! and nothing else (the scan below holds that) — and it is the only caller of
//! the layer in the browser; the Worker (`web/src/local/worker.ts`) is the only
//! caller of this.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use wasm_bindgen::prelude::*;
use yokohama_web::api::{self, ApiRequest, Host};
use yokohama_web::games::Games;
use yokohama_web::identity::User;
use yokohama_web::store::{Doc, MemStore};

/// The one `Games`. `init()` REPLACES it (a rebuild, never a once): the Worker
/// calls `init` at boot, on every lock acquisition and after a trap, and
/// follows each with `load`s (spec §2.2).
static GAMES: Mutex<Option<Games>> = Mutex::new(None);

/// The clock: frozen to `FROZEN` when `FROZEN_ON`, else live (`Date.now()` on
/// the target; a ticking counter natively, for the crate's own cells and the
/// workspace lint gate).
static FROZEN_ON: AtomicBool = AtomicBool::new(false);
static FROZEN: AtomicU64 = AtomicU64::new(0);
static TICKS: AtomicU64 = AtomicU64::new(0);

fn now() -> u64 {
    if FROZEN_ON.load(Ordering::Relaxed) {
        return FROZEN.load(Ordering::Relaxed);
    }
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        TICKS.fetch_add(1, Ordering::Relaxed) + 1
    }
}

const HOST: Host = Host { identity_name: "local", signal_kind: "none" };

#[wasm_bindgen]
pub fn init() {
    let games = Games::with_parts(
        Arc::new(yokohama_core::Rules::standard()),
        Arc::new(MemStore::default()),
        yokohama_web::chat::ChatConfig::default(),
        Arc::new(yokohama_web::signal::NoSignal),
        Arc::new(now),
        Arc::new(yokohama_web::archive::NoArchive),
    );
    *GAMES.lock().expect("the host's games") = Some(games);
}

#[wasm_bindgen]
pub struct Reply {
    status: u16,
    body: String,
}

#[wasm_bindgen]
impl Reply {
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> u16 { self.status }
    #[wasm_bindgen(getter)]
    pub fn body(&self) -> String { self.body.clone() }
}

/// The layer's front door, one call. `user` absent is the host's own 401
/// sentence carried into `dispatch` (so `/api/me` answers `user: null` as the
/// corpus captures it and the 401 sweep has an arm); the Worker always passes
/// `local@browser`, and a test passes the corpus walk's own five people.
///
/// **Every export is a thin wrapper over a pure-Rust `_inner` function** (spec
/// §2.1): the native cells call the inner ones, because `wasm-bindgen`'s
/// non-wasm stubs cannot make a `JsValue`.
pub fn dispatch_inner(user: Option<&str>, method: &str, path: &str, query: &str, content_type: Option<&str>, body: &[u8]) -> (u16, String) {
    let guard = GAMES.lock().expect("the host's games");
    let games = guard.as_ref().expect("init() before dispatch()");
    let parsed = user.map(User::parse);
    let identity: api::Identity<'_> = match &parsed {
        Some(Ok(u)) => Ok(u),
        Some(Err(_)) | None => Err("no identity given"),
    };
    let req = ApiRequest { method, path, query, content_type, body };
    let res = api::dispatch(games, &HOST, identity, &req);
    (res.status, res.body)
}

#[wasm_bindgen]
pub fn dispatch(user: Option<String>, method: &str, path: &str, query: &str, content_type: Option<String>, body: &[u8]) -> Reply {
    let (status, body) = dispatch_inner(user.as_deref(), method, path, query, content_type.as_deref(), body);
    Reply { status, body }
}

fn doc_of(name: &str) -> Result<Doc, String> {
    match name {
        "record" => Ok(Doc::Record),
        "table" => Ok(Doc::Table),
        other => Err(format!("no such document kind: {other:?} (record | table)")),
    }
}

pub fn load_inner(doc: &str, id: &str, text: &str) -> Result<(), String> {
    let guard = GAMES.lock().expect("the host's games");
    let games = guard.as_ref().expect("init() before load()");
    games.load_document(doc_of(doc)?, id, text)
}

pub fn read_inner(doc: &str, id: &str) -> Result<Option<String>, String> {
    let guard = GAMES.lock().expect("the host's games");
    let games = guard.as_ref().expect("init() before read()");
    games.read_document(doc_of(doc)?, id)
}

#[wasm_bindgen]
pub fn load(doc: &str, id: &str, text: &str) -> Result<(), JsValue> {
    load_inner(doc, id, text).map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn read(doc: &str, id: &str) -> Result<Option<String>, JsValue> {
    read_inner(doc, id).map_err(|e| JsValue::from_str(&e))
}

/// The e2e build's trap hook (spec §6.5, `--features debug`): a panic under
/// `panic = "abort"` is a trap that leaves `GAMES` locked — the shape a fresh
/// Worker recovers from and a re-`init()` cannot.
#[cfg(feature = "debug")]
#[wasm_bindgen]
pub fn debug_trap() {
    let _guard = GAMES.lock().expect("the host's games");
    panic!("debug_trap");
}

#[wasm_bindgen]
pub fn set_clock(seconds: Option<f64>) {
    match seconds {
        Some(s) => { FROZEN.store(s as u64, Ordering::Relaxed); FROZEN_ON.store(true, Ordering::Relaxed); }
        None => FROZEN_ON.store(false, Ordering::Relaxed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cells share the global `GAMES` and clock; they run one at a time.
    static SERIAL: Mutex<()> = Mutex::new(());

    #[test]
    fn a_missing_identity_is_the_layers_own_401_and_me_answers_null() {
        let _s = SERIAL.lock().unwrap();
        init();
        let (status, _) = dispatch_inner(None, "GET", "/api/games", "", None, b"");
        assert_eq!(status, 401);
        let (status, body) = dispatch_inner(None, "GET", "/api/me", "", None, b"");
        assert_eq!(status, 200);
        assert_eq!(body, r#"{"mode":"local","user":null}"#);
        let (_, body) = dispatch_inner(Some("local@browser"), "GET", "/api/health", "", None, b"");
        assert!(body.contains(r#""identity":"local""#) && body.contains(r#""signal":"none""#) && body.contains(r#""archive":"off""#), "{body}");
    }

    #[test]
    fn init_is_a_rebuild_and_load_is_a_create() {
        let _s = SERIAL.lock().unwrap();
        init();
        load_inner("table", "local-1", "{\"schema\":3}").unwrap();
        assert!(load_inner("table", "local-1", "{\"schema\":3}").is_err(), "a present document is refused");
        assert_eq!(read_inner("table", "local-1").unwrap().as_deref(), Some("{\"schema\":3}"));
        assert!(load_inner("chat", "local-1", "{}").is_err(), "only record | table");
        init();
        assert_eq!(read_inner("table", "local-1").unwrap(), None, "init() replaced the store");
    }

    #[test]
    fn the_clock_freezes_and_ticks() {
        let _s = SERIAL.lock().unwrap();
        init();
        set_clock(Some(1_789_000_000.0));
        assert_eq!(now(), 1_789_000_000);
        set_clock(None);
        let a = now(); let b = now();
        assert!(b > a, "the native clock ticks");
    }

    /// The crate's own scan (spec §2.1): this is host code and its ONLY platform
    /// calls are `js_sys` and the bindgen glue; the eight patterns the layer's
    /// scan names are refused here too.
    #[test]
    fn this_host_makes_no_platform_call_but_the_clock() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).unwrap();
        let cut = src.split("#[cfg(test)]").next().unwrap();
        for pat in ["SystemTime", "Instant::now", "std::fs", "std::env", "std::thread", "std::process", "tokio::", "axum::"] {
            assert!(!cut.contains(pat), "{pat} in the host crate");
        }
        assert!(cut.contains("js_sys::Date::now"), "anti-vacuity: the one allowed call is there");
    }
}
```

(`SERIAL` is what keeps three cells that reset one global from racing; the inner functions are what lets them run natively at all.)

- [ ] **Step 3: Build natively and for the target**

Run: `cargo test -p yokohama-wasm && cargo clippy --workspace --all-targets -- -D warnings`
Expected: four cells pass; clippy clean — RECORD in `progress.md` that `wasm-bindgen` and `js-sys` now build natively in the workspace (M6.5a's P-2 superseded) and the added wall of `cargo clippy --workspace`.

Run: `rustup target list --installed | grep wasm32-unknown-unknown; cargo install wasm-bindgen-cli --version 0.2.128 --locked; cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm && ls -la target/wasm32-unknown-unknown/release/yokohama_wasm.wasm`
Expected: the module builds. RECORD its size and `gzip -c … | wc -c` in `progress.md` beside the umbrella's 1.30 MB / 387 KB.

`web/package.json` scripts — the glue INTO `src/local/wasm/` (Vite bundles it and hashes the `.wasm` as an asset; a static import from `public/` fails Rollup, reproduced), `build:wasm` BEFORE `tsc` (the `.d.ts` must exist for the typecheck), the Node glue given a CommonJS package scope:

```json
"build:wasm": "cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm --manifest-path ../Cargo.toml && wasm-bindgen --target web --out-dir src/local/wasm ../target/wasm32-unknown-unknown/release/yokohama_wasm.wasm && wasm-bindgen --target nodejs --out-dir wasm-node ../target/wasm32-unknown-unknown/release/yokohama_wasm.wasm && node -e \"require('fs').writeFileSync('wasm-node/package.json', JSON.stringify({type:'commonjs'}))\"",
"build:wasm:debug": "cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm --features debug --manifest-path ../Cargo.toml && wasm-bindgen --target web --out-dir src/local/wasm ../target/wasm32-unknown-unknown/release/yokohama_wasm.wasm",
"build": "npm run build:wasm && tsc -b && vite build",
"typecheck": "tsc -b --noEmit && tsc -p tsconfig.node.json && tsc -p tsconfig.worker.json --noEmit",
```

`web/tsconfig.worker.json`: `{ "extends": "./tsconfig.json", "compilerOptions": { "lib": ["ES2022", "WebWorker"], "types": [] }, "include": ["src/local/worker.ts", "src/local/db.ts", "src/local/wasm/*.d.ts"] }` (exclude `src/local/worker.ts` and `db.ts` from the app project's `include`). `web/vitest.config.ts`: add `exclude: ["src/local/*.node.test.ts", "node_modules/**"]` so the jsdom run never asks for the module.

Run: `cd web && rm -rf src/local/wasm wasm-node && npm run build:wasm && ls src/local/wasm wasm-node && npm run typecheck && npm run build && ls dist/assets | grep -i wasm`
Expected: `yokohama_wasm.js`, `yokohama_wasm_bg.wasm`, `.d.ts` in both directories; `dist/assets/` carries a hashed `yokohama_wasm_bg-*.wasm`; a CLEAN checkout with both directories deleted builds by these commands alone (the R7 acceptance).

- [ ] **Step 4: CI**

In `.github/workflows/test.yml`, the `web-server` job, after the toolchain step:

```yaml
      # M6.5b: the browser host's module is built here — this is the job that has the
      # toolchain and the target — and the corpus replay through it runs here too
      # (Task 7). `wasm-bindgen-cli` is a from-source build of about two minutes,
      # cached by version; the release module is a second cargo target per run. The
      # job's wall is expected to roughly DOUBLE (spec §2.1).
      - uses: actions/cache@v4
        with:
          path: ~/.cargo/bin/wasm-bindgen
          key: wasm-bindgen-cli-0.2.128
      - run: test -x ~/.cargo/bin/wasm-bindgen || cargo install wasm-bindgen-cli --version 0.2.128 --locked
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: web/package-lock.json
      - run: npm ci
        working-directory: web
      - run: npm run build:wasm
        working-directory: web
```

(Task 7 appends `npm run test:wasm`.) Push and read the job's wall; RECORD it.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock .gitignore crates/yokohama-wasm crates/yokohama-web/src/games.rs crates/yokohama-web/src/assets.rs web/package.json .github/workflows/test.yml
git commit -m "feat(wasm): crates/yokohama-wasm — the browser host's six exports over the layer, its own scan, the two-target build; .wasm served as application/wasm (R-6.5b.9, R-6.5b.14)"
```

---

### Task 4: The Worker, IndexedDB, the lock; the client's local arm; the transport's `host` arm; per-source facts (R-6.5b.4, R-6.5b.5, R-6.5b.12) — **the SDD task**

**Files:**
- Create: `web/src/local/db.ts`, `web/src/local/worker.ts`, `web/src/local/client.ts`, `web/src/local/facts.ts`, `web/src/test/fakeWorker.ts`
- Modify: `web/src/api.ts` (`isLocal`, `LOCAL_PREFIX`, `call` with `host?`, `recordText`, `labels(host?)`, `list(scope, before, host?)`, `create(req, host?)`, `health(host?)`, `me(host?)`, `archive`), `web/src/signal.ts` (`Source.host?`, the arm), `web/src/App.tsx:844` (`useSignal` passes `host`), `web/src/types.ts`
- Test: `web/src/local/client.test.ts`, `web/src/local/db.test.ts` (with `fake-indexeddb` — add it as a dev dependency), `web/src/api.test.ts`, `web/src/signal.test.ts`, `web/src/App.test.tsx`

**Interfaces:**
- Produces: `LOCAL_PREFIX = "local-"`, `isLocal(id: string): boolean`; `call<T>(method, path, body?, host?: "local" | "server")`; `type Host = "local" | "server"`; `facts(source: Host): DeploymentFacts` with `DeploymentFacts = { signal: "sse" | "held" | "none"; archive: ArchiveKind; lobbyChat: boolean; chatText: number }` and `setServerFacts(h: Health)`; `Source.host?: "local"`; Worker messages `Req = { seq: number } & ({ kind: "call"; method: string; path: string; query: string; body: string | null } | { kind: "read"; doc: "record" | "table"; id: string } | { kind: "hold"; id: string } | { kind: "release"; id: string } | { kind: "takeover"; id: string })`, replies `{ seq: number } & ({ status: number; body: string } | { text: string | null } | { held: boolean } | { error: string })`, and one unsolicited `{ kind: "lost"; id: string }`; page side `localCall(method, path, body?): Promise<{ status: number; text: string }>`, `localRead(doc, id): Promise<string | null>`, `hold(id): Promise<boolean>`, `takeover(id): Promise<void>`, `release(id): Promise<void>`, `onLost(fn: (id: string) => void): () => void`, `onLocalWrite(fn: () => void): () => void`, `localReady(): Promise<void>`.
- Consumes: Task 3's module at `/wasm/yokohama_wasm.js`; the corpus for the fake.

- [ ] **Step 1: The IndexedDB module and its cells** (`web/src/local/db.ts`)

```ts
// The Worker's side channel (spec §2.2): one database, one store `docs` keyed
// by `[doc, id]`, holding the layer's documents as opaque TEXT. Their own schema
// is the layer's (`TABLE_SCHEMA`), so a version bump here is only ever a new
// store.
export type DocKind = "record" | "table";
const NAME = "yokohama-local";
const VERSION = 1;

export function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(NAME, VERSION);
    req.onupgradeneeded = () => {
      req.result.createObjectStore("docs", { keyPath: ["doc", "id"] });
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

const tx = (db: IDBDatabase, mode: IDBTransactionMode) => db.transaction("docs", mode).objectStore("docs");
const done = <T>(r: IDBRequest<T>) =>
  new Promise<T>((resolve, reject) => { r.onsuccess = () => resolve(r.result); r.onerror = () => reject(r.error); });

export const readAll = (db: IDBDatabase) =>
  done(tx(db, "readonly").getAll()) as Promise<{ doc: DocKind; id: string; text: string }[]>;

/** Both documents of one game in ONE transaction, so a reader never sees a
 *  record without its table. */
export function writePair(db: IDBDatabase, id: string, record: string, table: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const t = db.transaction("docs", "readwrite");
    t.objectStore("docs").put({ doc: "record", id, text: record });
    t.objectStore("docs").put({ doc: "table", id, text: table });
    t.oncomplete = () => resolve();
    t.onerror = () => reject(t.error);
    t.onabort = () => reject(t.error);
  });
}

export function deletePair(db: IDBDatabase, id: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const t = db.transaction("docs", "readwrite");
    t.objectStore("docs").delete(["record", id]);
    t.objectStore("docs").delete(["table", id]);
    t.oncomplete = () => resolve();
    t.onerror = () => reject(t.error);
  });
}
```

Cells (`db.test.ts`, `import "fake-indexeddb/auto"` at the top): a pair written reads back as the same two strings; a delete removes both; a `writePair` whose second put throws (spy `objectStore.put` to throw on the second call) leaves NEITHER document (the transaction aborts).

- [ ] **Step 2: The Worker** (`web/src/local/worker.ts`; a module Worker) — **one serialised actor** (spec §2.2, R3): every message runs through one queue; the lock's callback promise is held OUTSIDE it; ownership is a generation stored in IndexedDB and checked in every write's transaction (R2); persistence is of the documents that EXIST, for HELD games only, BEFORE the reply (R1, R2, R4); a trap is answered `trapped` and the PAGE recreates the Worker (R5).

`db.ts` gains a second store `meta` keyed by `id` holding `{ id, generation }`, and the two pair writers take the generation:

```ts
/** The pair — or the table alone while dealing — written ONLY if this tab's
 *  generation is the stored one, in the SAME transaction (spec §2.2, R2). */
export function writeDocs(db: IDBDatabase, id: string, generation: number, docs: { record?: string; table: string }): Promise<"written" | "stale"> {
  return new Promise((resolve, reject) => {
    const t = db.transaction(["docs", "meta"], "readwrite");
    let outcome: "written" | "stale" = "written";
    const meta = t.objectStore("meta").get(id);
    meta.onsuccess = () => {
      const stored = (meta.result as { generation: number } | undefined)?.generation ?? 0;
      if (stored !== generation) { outcome = "stale"; t.abort(); return; }
      try {
        t.objectStore("docs").put({ doc: "table", id, text: docs.table });
        if (docs.record !== undefined) t.objectStore("docs").put({ doc: "record", id, text: docs.record });
      } catch (e) { t.abort(); reject(e); }
    };
    t.oncomplete = () => resolve(outcome);
    t.onabort = () => (outcome === "stale" ? resolve("stale") : reject(t.error ?? new Error("aborted")));
    t.onerror = () => reject(t.error);
  });
}
/** Advance the game's generation (an acquisition) and return the new value. */
export function takeGeneration(db: IDBDatabase, id: string): Promise<number> { … get, +1, put, resolve on complete … }
export function deletePair(db: IDBDatabase, id: string): Promise<void> { … delete both docs and the meta row; oncomplete/onabort/onerror … }
```

The Worker:

```ts
/// <reference lib="webworker" />
import { openDb, readAll, writeDocs, deletePair, takeGeneration } from "./db";
import init, * as wasm from "./wasm/yokohama_wasm.js";   // generated INTO src/local/wasm by build:wasm; Vite bundles it

const LOCAL_USER = "local@browser";
const LOCAL_PREFIX = "local-";
type Req = { seq: number } & (
  | { kind: "call"; method: string; path: string; query: string; body: string | null }
  | { kind: "read"; doc: "record" | "table"; id: string }
  | { kind: "hold"; id: string } | { kind: "takeover"; id: string } | { kind: "release"; id: string });

let db: IDBDatabase;
const lastWritten = new Map<string, string>();
const held = new Map<string, { generation: number; release: () => void }>();
let queue: Promise<unknown> = Promise.resolve();
/** Everything runs through here, one at a time (R3). */
const serial = <T>(f: () => Promise<T>): Promise<T> => { const p = queue.then(f, f); queue = p.catch(() => {}); return p; };

const post = (m: unknown) => (self as unknown as Worker).postMessage(m);
const drawId = () => { const b = new Uint8Array(8); crypto.getRandomValues(b); return LOCAL_PREFIX + [...b].map((x) => x.toString(16).padStart(2, "0")).join(""); };
const gameOf = (path: string) => { const m = /^\/api\/games\/([^/]+)/.exec(path); return m ? decodeURIComponent(m[1]!) : null; };

/** A FRESH `Games` from the committed pairs, and the write cache RESEEDED from them (R3). */
async function rebuild(): Promise<void> {
  wasm.init();
  lastWritten.clear();
  const rows = await readAll(db);
  for (const r of rows) wasm.load(r.doc, r.id, r.text);
  const byId = new Map<string, { record?: string; table?: string }>();
  for (const r of rows) byId.set(r.id, { ...(byId.get(r.id) ?? {}), [r.doc]: r.text });
  for (const [id, d] of byId) lastWritten.set(id, (d.record ?? "") + "\u0000" + (d.table ?? ""));
}

/** The documents that EXIST (the table alone while dealing, R1), for a game this
 *  tab HOLDS, committed BEFORE the page is answered (R2, R4). */
async function persist(id: string, deleted: boolean): Promise<"saved" | "unsaved" | "not-held"> {
  const h = held.get(id);
  if (!h) return "not-held";
  if (deleted) { await deletePair(db, id); lastWritten.delete(id); return "saved"; }
  const table = wasm.read("table", id);
  if (table == null) return "saved";                       // nothing to keep (the delete's own path handles removal)
  const record = wasm.read("record", id) ?? undefined;
  const key = (record ?? "") + "\u0000" + table;
  if (lastWritten.get(id) === key) return "saved";
  const outcome = await writeDocs(db, id, h.generation, { record, table });
  if (outcome === "stale") { held.delete(id); post({ kind: "lost", id }); return "not-held"; }
  lastWritten.set(id, key);
  return "saved";
}

async function call(req: Extract<Req, { kind: "call" }>): Promise<Record<string, unknown>> {
  let body = req.body;
  if (req.method === "POST" && req.path === "/api/games" && body) body = JSON.stringify({ ...(JSON.parse(body) as object), id: drawId() });
  const bytes = body === null ? new Uint8Array() : new TextEncoder().encode(body);
  const reply = wasm.dispatch(LOCAL_USER, req.method, req.path, req.query, body === null ? undefined : "application/json", bytes);   // a trap throws here → the catch in onmessage
  const id = gameOf(req.path) ?? (req.method === "POST" && req.path === "/api/games" && reply.status === 201 ? (JSON.parse(reply.body) as { id: string }).id : null);
  let saved: "saved" | "unsaved" | "not-held" = "saved";
  if (id) {
    try { saved = await persist(id, req.method === "DELETE" && reply.status === 204); }
    catch (e) { saved = "unsaved"; }                        // a storage failure is NOT a trap (R4)
  }
  return { status: reply.status, body: reply.body, saved };
}

function hold(id: string, steal: boolean): Promise<boolean> {
  if (held.has(id)) return Promise.resolve(true);
  return new Promise((resolve) => {
    const opts: LockOptions = steal ? { mode: "exclusive", steal: true } : { mode: "exclusive", ifAvailable: true };
    navigator.locks.request(`yokohama:local:${id}`, opts, async (lock) => {
      if (lock === null) { resolve(false); return; }
      // Inside the queue: the generation advances and the store rebuilds before this tab answers for the game.
      await serial(async () => { const generation = await takeGeneration(db, id); await rebuild(); await new Promise<void>((release) => { held.set(id, { generation, release }); resolve(true); }); });
    }).catch((e: unknown) => {
      if (e instanceof DOMException && e.name === "AbortError") { held.delete(id); post({ kind: "lost", id }); } else resolve(false);
    });
  });
}

const ready = (async () => { await init(); db = await openDb(); await rebuild(); })();

self.onmessage = (e: MessageEvent<Req>) => {
  const req = e.data;
  void serial(async () => {
    try {
      await ready;
      switch (req.kind) {
        case "call": post({ seq: req.seq, ...(await call(req)) }); break;
        case "read": post({ seq: req.seq, text: wasm.read(req.doc, req.id) ?? null }); break;
        case "hold": post({ seq: req.seq, held: await hold(req.id, false) }); break;
        case "takeover": post({ seq: req.seq, held: await hold(req.id, true) }); break;
        case "release": held.get(req.id)?.release(); held.delete(req.id); post({ seq: req.seq, released: true }); break;
      }
    } catch (err) {
      // A trap (or a module that never loaded): answered, then the PAGE recreates this Worker (R5).
      post({ seq: req.seq, status: 500, body: JSON.stringify({ error: `the local host stopped: ${String(err)}` }), trapped: true });
    }
  });
};
```

(`hold`'s callback holds the lock's promise open OUTSIDE the queue — the `serial` inside it returns once `held.set` has run, and the lock's own callback keeps waiting on `release`; the acquisition itself is serialised with every call. The in-flight rule falls out of `writeDocs`: a call that dispatched under an ownership since stolen finds `"stale"` in its write transaction and is answered as the loss.)

- [ ] **Step 3: The page side** (`web/src/local/client.ts`) — the Worker singleton, request/reply by `seq`, `trapped` → terminate and recreate (R5), Worker `error`/`messageerror` and a failed start → a visible sentence, never a pending list:

```ts
type Reply = { seq: number } & Record<string, unknown>;
let worker: Worker | null = null; let seq = 0;
const pending = new Map<number, { resolve: (r: Reply) => void; reject: (e: Error) => void }>();
const lostHandlers = new Set<(id: string) => void>(); const writeHandlers = new Set<() => void>(); const faultHandlers = new Set<(sentence: string) => void>();
const heldByPage = new Set<string>();           // what to re-acquire after a restart

function fail(sentence: string) { for (const p of pending.values()) p.reject(new Error(sentence)); pending.clear(); faultHandlers.forEach((f) => f(sentence)); }

function start(): Worker {
  if (worker) return worker;
  worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  worker.onerror = (e) => fail(`the local host failed to start: ${e.message}`);
  worker.onmessageerror = () => fail("the local host sent something this page could not read");
  worker.onmessage = (e: MessageEvent<Reply & { kind?: string; id?: string; trapped?: boolean }>) => {
    if (e.data.kind === "lost" && e.data.id) { heldByPage.delete(e.data.id); lostHandlers.forEach((f) => f(e.data.id!)); return; }
    const p = pending.get(e.data.seq); pending.delete(e.data.seq);
    if (e.data.trapped) { restart(e.data); return; }
    p?.resolve(e.data);
  };
  return worker;
}

/** R5: a trapped module cannot recover in place — terminate, settle, restart, re-hold. */
function restart(last: Reply) {
  worker?.terminate(); worker = null;
  fail("the local host restarted from the last saved move");
  const w = start();
  for (const id of [...heldByPage]) void ask({ kind: "hold", id });
  void last; void w;
}

function ask<T extends Record<string, unknown>>(msg: Record<string, unknown>): Promise<T> {
  const w = start(); const mine = ++seq;
  return new Promise((resolve, reject) => { pending.set(mine, { resolve: (r) => resolve(r as unknown as T), reject }); w.postMessage({ seq: mine, ...msg }); });
}

export async function localCall(method: string, path: string, body?: unknown): Promise<{ status: number; text: string; saved: boolean }> {
  const [p, q = ""] = path.split("?", 2) as [string, string?];
  const r = await ask<{ status: number; body: string; saved: "saved" | "unsaved" | "not-held" }>({ kind: "call", method, path: p, query: q, body: body === undefined ? null : JSON.stringify(body) });
  if (method !== "GET") writeHandlers.forEach((f) => f());
  return { status: r.status, text: r.body, saved: r.saved === "saved" };
}
export const localRead = async (doc: "record" | "table", id: string) => (await ask<{ text: string | null }>({ kind: "read", doc, id })).text;
export const hold = async (id: string) => { const ok = (await ask<{ held: boolean }>({ kind: "hold", id })).held; if (ok) heldByPage.add(id); return ok; };
export const takeover = async (id: string) => { await ask({ kind: "takeover", id }); heldByPage.add(id); };
export const release = async (id: string) => { heldByPage.delete(id); await ask({ kind: "release", id }); };
export const onLost = (f: (id: string) => void) => { lostHandlers.add(f); return () => lostHandlers.delete(f); };
export const onLocalWrite = (f: () => void) => { writeHandlers.add(f); return () => writeHandlers.delete(f); };
export const onLocalFault = (f: (s: string) => void) => { faultHandlers.add(f); return () => faultHandlers.delete(f); };
export const localReady = () => localCall("GET", "/api/health").then(() => undefined);
```

`call`'s local arm reads `saved` and, when false, appends the server's own "APPLIED BUT NOT SAVED" to the reply's `warnings` the way the axum host does (find the sentence with `grep -rn "APPLIED BUT NOT SAVED" crates/yokohama-web/src` and reuse the exact words). The route effect at `App.tsx:503` becomes `if (!door.me?.user && !(route.kind === "table" && isLocal(route.id))) return;`, and ownership is taken from the ROUTE's table id (Task 6's `useLocalHold(route.kind === "table" ? route.id : null)`), the dealing screen included — never from `state?.id`.

- [ ] **Step 4: `api.ts` — the local arm, the per-source facts** (`web/src/local/facts.ts` and `api.ts`)

`facts.ts`:

```ts
import type { Health } from "../types";
export type Host = "local" | "server";
export interface DeploymentFacts { signal: "sse" | "held" | "none"; archive: "off" | "dir" | "r2"; lobbyChat: boolean; chatText: number }
const LOCAL: DeploymentFacts = { signal: "none", archive: "off", lobbyChat: true, chatText: 500 }; // the chat facts are unread (spec §2.1)
let server: DeploymentFacts = { signal: "sse", archive: "off", lobbyChat: true, chatText: 500 };
export const facts = (source: Host): DeploymentFacts => (source === "local" ? LOCAL : server);
export const setServerFacts = (h: Health): void => { server = { signal: h.signal, archive: h.archive, lobbyChat: h.lobby_chat, chatText: h.chat_text }; };
```

In `api.ts`: `signalKind`, `archiveKind`, `lobbyChat`, `chatText` become reads of `facts("server")` (their setters call `setServerFacts` — keep the four exported names so nothing else moves; the `"sse" | "held"` union at `:193` is untouched because `facts("server").signal` is narrowed by `setServerFacts`'s input type `Health`). Add:

```ts
export const LOCAL_PREFIX = "local-";
/** ONE definition of what a local GAME is (R-6.5b.12). */
export const isLocal = (id: string): boolean => id.startsWith(LOCAL_PREFIX);

const gameIdOf = (path: string): string | null => { const m = /^\/api\/games\/([^/?]+)/.exec(path); return m ? decodeURIComponent(m[1]!) : null; };

async function call<T>(method: string, path: string, body?: unknown, host?: Host): Promise<T> {
  const local = host === "local" || (host === undefined && (() => { const g = gameIdOf(path); return g !== null && isLocal(g); })());
  let status: number; let text: string; let statusText = "";
  if (local) {
    ({ status, text } = await localCall(method, path, body));
  } else {
    const headers: Record<string, string> = { ...identityHeaders() };
    if (body !== undefined) headers["content-type"] = "application/json";
    const res = await fetch(path, { method, headers, body: body === undefined ? undefined : JSON.stringify(body) });
    status = res.status; statusText = res.statusText; text = await res.text();
  }
  return settle<T>(status, statusText, text); // the existing parse-and-refuse tail of `call`, extracted verbatim
}
```

`api.labels()` is wrapped by the boot effect in Task 5 with a five-second bound (`Promise.race` against a timer) before the Worker fallback — a hanging read never rejects. `recordText(game)`: `if (isLocal(game)) { const t = await localRead("record", game); if (t === null) throw new ApiError("no such local game", 404, null); return t; }` before the `fetch`. `api.labels(host?)`, `api.list(scope, before, host?)`, `api.create(req, host?)`, `api.health(host?)`, `api.me(host?)` pass `host` through; add `archive: (record: string, table: string) => call<ArchiveReply>("POST", "/api/archive", { record, table }, "server")`.

`signal.ts`: `Source<T>` gains `host?: "local";` and `open()`'s first lines become:

```ts
if (src.current.host === "local") {
  // R-6.5b.12 / spec §2.3: nothing to wait for — the bot's run returns inside the reply.
  setLink("live");
  return;
}
```

(before the `signalKind() === "held"` branch; no timer, no fetch). `App.tsx:844`'s `useSignal(...)` source gains `host: isLocal(state.id) ? "local" : undefined`.

- [ ] **Step 5: The fake Worker and the cells**

`web/src/test/fakeWorker.ts`: a class with `postMessage(msg)` that answers by `seq` from a table of `(method, path) → committed fixture` the test supplies (`import health from "../wire/health.json"` etc.), posting `{ seq, status, body: JSON.stringify(fixture) }` on the next microtask, `{ held: true }` to `hold`, and exposing `posted: Req[]` for assertions; `vi.stubGlobal("Worker", FakeWorker)` in the cells that use it.

Cells:
- `api.test.ts`: `call("GET", "/api/games/local-1")` reaches the fake Worker and NOT `fetch` (fetch is absent in the setup — a call to it throws, so this is a crash-or-pass cell); `call("GET", "/api/games/sim-1")` reaches `fetch` (stub one); the five id-less callers with `"local"` reach the Worker; `recordText("local-1")` returns the Worker's `read` text unparsed; `archive(...)` posts to `/api/archive` on the server with the two strings as the body's two fields.
- `signal.test.ts`: a `Source` with `host: "local"` arms NO timer (`vi.getTimerCount() === 0` across sixty fake seconds) and calls neither `fetch` nor `readStream`; a `Source` without it still opens the stream (the existing cell, unchanged).
- `App.test.tsx`: **the `fetch` spy at zero across a whole local game** — mount `App` with the fake Worker answering `me`, `health`, `table`, `deal`, the `ClientState` fixtures and `run.json` for the walk; play three inputs; assert `fetch` was never called (it is absent, so any call throws into the error strip — assert the strip is empty), with `POST /api/archive` and `health` the only paths a later cell exempts.
- `client.test.ts` (with `fake-indexeddb` NOT needed — the Worker is faked): `hold` returns what the Worker says; an unsolicited `lost` reaches `onLost`; every non-GET call fires `onLocalWrite`.
- `client.test.ts`: a reply carrying `trapped: true` terminates the Worker, rejects every pending call with the restart sentence, starts a fresh Worker and re-`hold`s every game the page held (assert the fake's constructor ran twice and the second instance received `hold` for the held id); a Worker `error` event reaches `onLocalFault` with its sentence; a call whose reply says `saved: "unsaved"` surfaces "APPLIED BUT NOT SAVED" in the frame's `warnings`.
- **The rebuild-refusal surface** (spec §4): the fake Worker answers `GET /api/games/local-1` with a 500 whose body is `{"error": "this save was recorded under game-semantics revision 6, but this build plays revision 7 — …"}` (the sentence `Session::from_record` gives, spec §0); the game screen shows that sentence in its strip, the "reloaded from the last saved move" wording does NOT appear (a refusal is a reply, never a trap), and "On this device" still lists the game with its Abandon control.

Run: `cd web && npm run typecheck && npm test`
Expected: green; count RECORDED (640 + the new cells).

- [ ] **Step 6: The one manual measurement** (R-6.5b.5, spec §8): `npm run build && cargo run --release -p yokohama-web` with the identity variable, open the site on the owner's phone or a phone emulator that runs real Chrome, and confirm `navigator.locks` exists in a dedicated Worker there. RECORD the browser and version in `progress.md`.

- [ ] **Step 7: Commit**

```bash
git add web/src/local web/src/test/fakeWorker.ts web/src/api.ts web/src/api.test.ts web/src/signal.ts web/src/signal.test.ts web/src/App.tsx web/src/App.test.tsx web/src/types.ts web/package.json web/package-lock.json
git commit -m "feat(web client): the local host — a Worker per tab over the module with IndexedDB and the Web Lock; call's local arm by prefix or intent; the transport's host arm; per-source deployment facts (R-6.5b.4, R-6.5b.5, R-6.5b.12)"
```

---

### Task 5: The first screen — the four door gates become content, "On this device", Play the bot here, `labels` with its fallback (R-6.5b.2, R-6.5b.8)

**Files:**
- Create: `web/src/OnThisDevice.tsx`, `web/src/Door.tsx`
- Modify: `web/src/App.tsx:555-600` (the boot effects), `:905-935` (the gates), the setup/lobby render; `web/src/Lobby.tsx` (the fifth section mounted FIRST; the "not connected" line); `web/src/NewGame.tsx` (the local shape); `web/src/route.ts` (`{ kind: "new"; local?: true }` ↔ `#/new/local`)
- Test: `web/src/OnThisDevice.test.tsx`, `web/src/App.test.tsx`, `web/src/Lobby.test.tsx`, `web/src/NewGame.test.tsx`, `web/src/route.test.ts`

**Interfaces:**
- Produces: `<OnThisDevice onOpen={(id) => …} />` (rows from `api.list("mine", undefined, "local")`, re-read on `onLocalWrite`, the "Play the bot here" button navigating to `{ kind: "new", local: true }`); `<Door door={door} />` (the four states as content); `route.ts`'s `parseRoute("#/new/local") → { kind: "new", local: true }`; `NewGame` prop `local?: boolean` (two seats, me + bot, the submit through `api.create(req, "local")`); App state `serverUp: boolean | null`.
- Consumes: Task 4's `api`, `onLocalWrite`, `localReady`, `facts`.

- [ ] **Step 1: The failing cells**

`App.test.tsx`:

```tsx
it("renders On this device with its games when every fetch REFUSES (the door.error arm)", async () => {
  vi.stubGlobal("Worker", fakeWorkerAnswering({ "GET /api/games?scope=mine": gamesLocal, "GET /api/health": healthLocal, "GET /api/labels": labels }));
  vi.stubGlobal("fetch", vi.fn(async () => { throw new TypeError("Failed to fetch"); }));
  render(<App />);
  expect(await screen.findByRole("heading", { name: "On this device" })).toBeInTheDocument();
  expect(screen.getByRole("link", { name: /local-1/ })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Play the bot here" })).toBeEnabled();
  expect(screen.getByText("not connected")).toBeInTheDocument();
});

it("renders On this device when the server never answers (the null arm)", async () => {
  vi.stubGlobal("Worker", fakeWorkerAnswering({ … }));
  vi.stubGlobal("fetch", vi.fn(() => new Promise(() => {}))); // never settles
  render(<App />);
  expect(await screen.findByRole("heading", { name: "On this device" })).toBeInTheDocument();
});

it("keeps the words on the cards when the server's labels read fails: the Worker answers them", async () => { … assert `Labels` came from the fake Worker's `GET /api/labels` after `fetch` rejected … });
```

(`gamesLocal` is `games.json` with its ids rewritten to `local-…` in the test — the ONE place a fixture is edited, and it is edited for its ids only, said in a comment.) `route.test.ts`: `#/new/local` round-trips. `NewGame.test.tsx`: with `local`, the seat rows are fixed to "me" and "the bot", the players select is absent, and the submit calls `api.create` with `host: "local"`.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd web && npm test -- App OnThisDevice route NewGame`
Expected: FAIL — the app renders `null`/the error screen; no such heading.

- [ ] **Step 3: Implement**

`Door.tsx`: a component rendering, for `door.error` → `<div className="setup door-error">{door.error}</div>`; `!door.me` → `<p className="muted">connecting…</p>`; `claimed` with no user → `<SignIn …/>`; no user → the door sentence; else `null` — the SAME four bodies `App.tsx:922-926` return today, moved. In `App.tsx` delete the four `return`s and render `<Door door={door} />` INSIDE the server area of the lobby/setup screen; every hook stays above. `serverUp` is set from the health effect (`then → true`, `catch → false`; `null` until then); the game/table screens for a SERVER game still require `door.me?.user` (pass `signedIn` down; a server game with no user shows the door's content in the game area). `App.tsx`'s labels effect becomes:

```ts
const bounded = <T,>(p: Promise<T>, ms: number) => Promise.race([p, new Promise<T>((_, reject) => setTimeout(() => reject(new Error("timed out")), ms))]);
bounded(api.labels(), 5_000).catch(() => api.labels("local")).then(setLabels).catch(() => setLabels(null));
```

and a cell pins the timeout arm: a server `labels` that never settles yields the Worker's words within the bound (fake timers). A Worker/IndexedDB fault reaches "On this device" through `onLocalFault` and renders as one line naming it, never as an empty list.

`Lobby.tsx`: render `<OnThisDevice onOpen={…} />` FIRST, then `<h2>Lobby</h2>` and, when `serverUp === false`, one `<p className="muted">not connected</p>` in place of the four sections (the `usePolled` stays mounted and keeps trying; the sections come back when a read lands and `serverUp` flips). `OnThisDevice.tsx`:

```tsx
export function OnThisDevice({ onOpen }: { onOpen: (id: string) => void }) {
  const [games, setGames] = useState<TableView[] | null>(null);
  const read = useCallback(() => api.list("mine", undefined, "local").then((l) => setGames(l.games)).catch(() => setGames([])), []);
  useEffect(() => { void read(); return onLocalWrite(() => void read()); }, [read]);
  return (
    <section className="saved on-this-device">
      <h3>On this device</h3>
      <button type="button" className="primary" onClick={() => navigate({ kind: "new", local: true })}>Play the bot here</button>
      {games === null ? <p className="muted">loading…</p> : games.length === 0 ? <p className="muted">no game on this device yet.</p> : <ul>{games.map((t) => rowFor(t, onOpen))}</ul>}
    </section>
  );
}
```

with `rowFor` reusing `Lobby`'s row builder (export it from `Lobby.tsx` as `lobbyRow(t, abandon: boolean)` so the wording is one function). `route.ts`: `{ kind: "new"; local?: true }`, `#/new/local`. `NewGame.tsx`: `local` fixes players to 2 and the rows to me + bot, hides the password and open-seat controls, and submits with `"local"`.

- [ ] **Step 4: Run, then the browser**

Run: `cd web && npm run typecheck && npm test && npm run e2e`
Expected: green; the existing e2e walks still pass (they sign in and use the lobby; the door is now content, not a gate). RECORD counts.

- [ ] **Step 5: Commit**

```bash
git add web/src/Door.tsx web/src/OnThisDevice.tsx web/src/OnThisDevice.test.tsx web/src/App.tsx web/src/App.test.tsx web/src/Lobby.tsx web/src/Lobby.test.tsx web/src/NewGame.tsx web/src/NewGame.test.tsx web/src/route.ts web/src/route.test.ts
git commit -m "feat(web client): the first screen renders from the browser first — the four door gates become the server area's content, On this device, Play the bot here, labels with a Worker fallback (R-6.5b.2, R-6.5b.8)"
```

---

### Task 6: The local game screen — no stream, Save game file at any point, Abandon and save through the archive, Send to the archive, the lock's three screens (R-6.5b.6, R-6.5b.7)

**Files:**
- Modify: `web/src/App.tsx` (the header controls `:1280-1310`; the Abandon path; `useLocalHold`), `web/src/Abandon.tsx` (the local arm), `web/src/Help.tsx`
- Create: `web/src/local/useLocalHold.ts`, `web/src/LockScreen.tsx`
- Test: `web/src/App.test.tsx`, `web/src/Abandon.test.tsx`, `web/src/Help.test.tsx`, `web/src/local/useLocalHold.test.ts`

**Interfaces:**
- Produces: `useLocalHold(id: string | null): { state: "held" | "elsewhere" | "lost" | "none"; takeover(): Promise<void> }`; `<LockScreen state onTakeover />`; `Abandon` prop `local?: boolean` (posts `api.archive(record, table)` BEFORE `api.remove` when `facts("server").archive !== "off"`; a failed post refuses with "the archive could not be reached — the game was kept" or the server's sentence; a 201 or a 200 `identical` proceeds to the delete, a 409 KEEPS the game); the "Send to the archive" button.
- Consumes: Task 4's `hold`, `takeover`, `release`, `onLost`, `localRead`, `api.archive`, `facts`.

- [ ] **Step 1: The failing cells**

`App.test.tsx`: "Save game file" is present on a RUNNING local game and absent on a running server game; "Send to the archive" is present on a finished local game only when `facts("server").archive !== "off"`, posts the record text and table text from the Worker's `read` UNPARSED, shows the reply's path, and then goes dark for the session; a lock `state: "elsewhere"` renders "This game is open in another tab" with Take over and NO answer bar; after `onLost` fires the screen is read-only with Take back and a click on an option does not reach the Worker. `Abandon.test.tsx`: local + archive on → `POST /api/archive` then DELETE, in that order; a 201 AND a 200 `identical` both proceed to the DELETE; the post failing (network) → no DELETE and the sentence; **a 409 ("a different record is archived under this id") → NO DELETE**, the sentence shown, the game kept (R6 — the earlier draft deleted on it). `useLocalHold.test.ts`: `hold` on mount, `release` on unmount and on id change.

- [ ] **Step 2: Run them to verify they fail** — `cd web && npm test -- App Abandon useLocalHold` — FAIL.

- [ ] **Step 3: Implement**

`useLocalHold.ts`:

```ts
export function useLocalHold(id: string | null) {
  const [state, setState] = useState<"held" | "elsewhere" | "lost" | "none">("none");
  useEffect(() => {
    if (!id || !isLocal(id)) { setState("none"); return; }
    let live = true;
    void hold(id).then((ok) => live && setState(ok ? "held" : "elsewhere"));
    const off = onLost((lostId) => lostId === id && live && setState("lost"));
    return () => { live = false; off(); void release(id); }; // navigating away RELEASES (spec §2.2)
  }, [id]);
  const take = useCallback(async () => { if (id) { await takeover(id); setState("held"); } }, [id]);
  return { state, takeover: take };
}
```

In `App.tsx`: `const lock = useLocalHold(state?.id ?? null);` — when `lock.state` is `"elsewhere"` or `"lost"` render `<LockScreen state={lock.state} onTakeover={lock.takeover} />` in place of the decision zone and make `send` a no-op that sets the strip to "opened in another tab"; the header's "Save game file" condition becomes `state.over || isLocal(state.id)`; a "Send to the archive" button when `state.over && isLocal(state.id) && facts("server").archive !== "off" && !sent` doing

```ts
const record = await localRead("record", state.id); const table = await localRead("table", state.id);
const r = await api.archive(record!, table!); setSent(true); setNotice(r.identical ? `already archived under ${r.path}` : `archived under ${r.path}`);
```

`Abandon.tsx`: the `abandon` function, when `local && facts("server").archive !== "off"`, first `api.archive(await localRead("record", id), await localRead("table", id))` inside a `try` whose catch calls `onError(...)` with the server's own sentence for a 4xx (a 409 included — a different record under this id is NOT a reason to delete this one) or "the archive could not be reached — the game was kept" otherwise, and returns without removing; only a resolved call (201, or 200 `identical`) proceeds to `api.remove`. The label reads "Abandon and save" for a local game under the same condition. `Help.tsx`: one paragraph under the saving section — a game on this device is saved as a file at any time, sent to the server's archive when finished, or abandoned and saved; the panel's `Help.test.tsx` walks it for the glued-letter rule as every paragraph is.

- [ ] **Step 4: Run, then the browser** — `cd web && npm run typecheck && npm test && npm run e2e` — green; RECORD counts.

- [ ] **Step 5: Commit**

```bash
git add web/src/App.tsx web/src/App.test.tsx web/src/Abandon.tsx web/src/Abandon.test.tsx web/src/Help.tsx web/src/Help.test.tsx web/src/LockScreen.tsx web/src/local/useLocalHold.ts web/src/local/useLocalHold.test.ts
git commit -m "feat(web client): a local game's screen — Save game file at any point, Abandon and save and Send to the archive through the server, the lock's three screens (R-6.5b.6, R-6.5b.7)"
```

---

### Task 7: `walk.json` from the walk; the frozen clock in `Api::new`; the wasm corpus replay as its own project (spec §5, §6.3)

**Files:**
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs` (`ScriptRow`, the recorder in `call_as`, `walk()`'s return, `corpus()`'s association and emission; `Api::new` with the frozen clock), `crates/yokohama-web/src/lib.rs` (`AppState::with_clock`)
- Create: `web/vitest.node.config.ts`, `web/src/local/corpus.node.test.ts`
- Modify: `web/package.json` (`test:wasm`), `.github/workflows/test.yml` (the run line), `web/src/wire/contract.ts` (`walk.json` is NOT a payload — exclude it from the root-key rows the way `site_space_words` is)
- Test: the replay itself

**Interfaces:**
- Produces: `web/src/wire-script/walk.json` — OUTSIDE the corpus directory (R8: the script carries request bodies with the setup seed and the passwords every leakage pin scans `corpus()` for) — `[{ who: string | null, method, path, query, body: object | null, status: number, fixture: string | null }]` in call order, the PRIMARY `Api`'s calls only; `AppState::with_clock(self, clock: Clock) -> Self`; `npm run test:wasm`.
- Consumes: Task 3's node build at `web/wasm-node/yokohama_wasm.js`.

- [ ] **Step 1: The recorder and the frozen clock**

In `lib.rs` (server): `pub fn with_clock(self, clock: games::Clock) -> Self { Self { games: self.games.with_clock(clock), ..self } }`. In `wire_fixtures.rs`: `Api` gains `script: std::cell::RefCell<Vec<ScriptRow>>` with

```rust
#[derive(Clone, serde::Serialize)]
struct ScriptRow {
    who: Option<String>,
    method: String,
    path: String,
    query: String,
    body: Option<Value>,
    status: u16,
    /// Bound after the walk, by VALUE (see `corpus`); `null` for an intermediate.
    fixture: Option<String>,
    /// Not emitted: the response, frozen, for the association.
    #[serde(skip)]
    response: Value,
}
```

`call_as` pushes a row after parsing the value (`response` = the value with `freeze_clock` applied to a clone). `Api::new`/`with_chat`/`with_archive` build the state then `.with_clock(Arc::new(|| FROZEN_CREATED_AT))` — **the regeneration diff for this change is expected EMPTY** (spec §6.3; a non-empty diff is a finding about the tie assumption). `walk()` returns `(Vec<(String, Value)>, Vec<ScriptRow>)`; `corpus()` takes both, and after `freeze_clock` on every payload:

```rust
// Bind each walk payload to the FIRST unbound script row whose frozen response
// equals it. A walk payload no row produced is a hard failure: it belongs in
// `constructed()` or on a second `Api`, and must be listed as such.
for (name, value) in &walk_payloads {
    let row = script.iter_mut().find(|r| r.fixture.is_none() && r.response == *value)
        .unwrap_or_else(|| panic!("{name}.json is not the response of any primary-Api call; list it in the third column"));
    row.fixture = Some(name.clone());
}
// NOT into `all`: the script is written to its OWN directory by the compare cell
// (`web/src/wire-script/walk.json`), so no leakage pin over `corpus()` ever reads a
// request body (R8). The compare cell writes it under `UPDATE_WIRE_FIXTURES` and
// otherwise asserts the committed script equals the emitted one, byte for byte.
```

(the `house_chat_off_404` and the eight `archive_*` payloads come from second `Api`s whose scripts are dropped, so they are never bound — they are third-column files by construction.) Add to `the_committed_wire_fixtures_are_what_the_server_emits` the same write-or-compare over `wire_script_dir().join("walk.json")`, and one assertion that the script's serialised text CONTAINS the setup seed and the walk's password — the anti-vacuity that says the pins' scope really excludes it on purpose.

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures && git status --short web/src/wire/ web/src/wire-script/`
Expected diff: `web/src/wire-script/walk.json` NEW, NOTHING under `web/src/wire/` (the frozen clock changes no committed payload). The seed and password pins stay green with no exemption.

- [ ] **Step 2: The replay**

`web/vitest.node.config.ts`:

```ts
import { defineConfig } from "vitest/config";
// The wasm corpus replay: Node, no DOM, no shared setup file (spec §6.3, I-8).
export default defineConfig({ test: { environment: "node", include: ["src/local/*.node.test.ts"] } });
// (`vitest.config.ts` EXCLUDES this glob — Task 3 — so `npm test` never asks for the module.)
```

`package.json`: `"test:wasm": "vitest run -c vitest.node.config.ts"`. `web/src/local/corpus.node.test.ts`:

```ts
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { describe, expect, it } from "vitest";

const wire = path.resolve(__dirname, "../wire");
const script = path.resolve(__dirname, "../wire-script/walk.json");
const modulePath = path.resolve(__dirname, "../../wasm-node/yokohama_wasm.js");
const have = fs.existsSync(modulePath);
if (!have && !process.env.YOKOHAMA_WASM_OPTIONAL) {
  throw new Error("the browser host's module is not built — run `npm run build:wasm` (or set YOKOHAMA_WASM_OPTIONAL=1 to skip, which tests NOTHING)");
}

const HOST_FIELDS: Record<string, string[]> = { health: ["identity", "signal"], me: ["mode"], me_signed_out: ["mode"] };
const NOT_SERVED = ["art_manifest", "site_space_words", "watching", "locked", "house_chat_off_404",
  "archive_post", "archive_post_identical", "archive_post_abandoned", "archive_409", "archive_400_rev", "archive_400_digest", "archive_400_replay", "archive_400_table"];

describe.skipIf(!have)("the browser host serves the corpus", () => {
  it("replays the walk through the module and every named reply equals its file", () => {
    const wasm = createRequire(import.meta.url)(modulePath) as typeof import("../../wasm-node/yokohama_wasm");
    wasm.init();
    wasm.set_clock(1_789_000_000);
    const rows = JSON.parse(fs.readFileSync(script, "utf8")) as Row[];
    const named = new Set<string>();
    for (const row of rows) {
      const body = row.body === null ? new Uint8Array() : new TextEncoder().encode(JSON.stringify(row.body));
      const reply = wasm.dispatch(row.who ?? undefined, row.method, row.path, row.query, row.body === null ? undefined : "application/json", body);
      expect(reply.status, `${row.method} ${row.path}`).toBe(row.status); // the status of EVERY row, named or not
      if (row.fixture === null) continue;                       // every row is REPLAYED; only a named one is compared
      named.add(row.fixture);
      const committed = JSON.parse(fs.readFileSync(path.join(wire, `${row.fixture}.json`), "utf8"));
      const got = reply.body ? JSON.parse(reply.body) : null;
      const strip = (v: Record<string, unknown>) => { const c = { ...v }; for (const f of HOST_FIELDS[row.fixture!] ?? []) delete c[f]; return c; };
      expect(strip(got), `${row.fixture} (${row.method} ${row.path})`).toEqual(strip(committed));
    }
    // The derived three columns: every committed file is named, host-parameterized, or listed as not served.
    const all = fs.readdirSync(wire).filter((f) => f.endsWith(".json")).map((f) => f.slice(0, -5));
    const third = all.filter((n) => !named.has(n)).sort();
    expect(third).toEqual([...NOT_SERVED].sort());
    for (const n of Object.keys(HOST_FIELDS)) expect(named.has(n), `${n} is replayed with its fields exempted`).toBe(true);
    // And the local host keeps no archive.
    const r = wasm.dispatch("local@browser", "POST", "/api/archive", "", "application/json", new TextEncoder().encode(JSON.stringify({ record: "{}", table: "{}" })));
    expect(r.status).toBe(404);
    expect(JSON.parse(r.body).error).toBe("this server keeps no archive");
  });
});
```

`Row` is the `ScriptRow` shape; `archive` is deliberately NOT in `HOST_FIELDS` (both hosts read `"off"` — M-1). CI: append `- run: npm run test:wasm` (working-directory `web`) to the `web-server` job.

Run: `cd web && npm run build:wasm && npm run test:wasm`
Expected: 1 passed. RECORD the wall. Also `cargo test -p yokohama-web` (the wire binary now emits 12: 10 + `walk` handling inside the existing two + the dispatcher cell unchanged — re-count).

- [ ] **Step 3: Commit**

```bash
git add crates/yokohama-web/tests/wire_fixtures.rs crates/yokohama-web/src/lib.rs web/src/wire-script/walk.json web/vitest.node.config.ts web/src/local/corpus.node.test.ts web/package.json .github/workflows/test.yml
git commit -m "test(web): walk.json — the walk's own request script, bound to its fixtures by value; the corpus replayed through the compiled module in Node as its own project; the walk's clock frozen (spec §5, §6.3)"
```

---

### Task 8: The e2e walk, the committed browser-played pair, the determinism pin, O-3 recorded (spec §6.4, §6.5)

**Files:**
- Create: `web/e2e/plays_locally.spec.ts`, `crates/yokohama-web/tests/browser_played_pin.rs`, `crates/yokohama-web/tests/fixtures/browser-played-2p.json`, `…table.json`, `…provenance.json`
- Modify: `web/e2e/harness.ts` (`startGame(page, { local: true })`), `web/playwright.config.ts` (`YOKOHAMA_WEB_ARCHIVE_DIR` in `env`)

**Interfaces:**
- Produces: the pair and its sidecar (`{ release, date, browser, module_sha256 }`); the pin as a per-edit integration test; `YOKOHAMA_WEB_UPDATE_PIN=1 npm run e2e -- plays_locally` as the regeneration command. The e2e's module is the `debug` build (`npm run e2e` runs `build:wasm:debug` before `vite build`); the RELEASE module never carries `debug_trap` (Task 9's RUNBOOK line says which build ships).
- Consumes: everything above.

- [ ] **Step 1: The spec file** (`web/e2e/plays_locally.spec.ts`, with the `test.skip(!haveBrowser, …)` guard `every_spec_can_skip.spec.ts` enforces):

```ts
test("a whole game against the bot plays in the browser with zero /api/games calls after load", async ({ page, context }) => {
  const watch = watchForBreakage(page, [{ status: 404, urlIncludes: "/api/archive" }]);
  const apiCalls: string[] = [];
  page.on("request", (r) => { const p = new URL(r.url()).pathname; if (p.startsWith("/api/") && p !== "/api/archive") apiCalls.push(p); });
  const game = await startGame(page, { local: true });          // Play the bot here → the form → Deal → keep
  expect(game.id.startsWith("local-")).toBe(true);
  const before = apiCalls.length;                                // the sign-in and the lobby's server reads happen before the game screen
  const pick = rng(0x4c4f_4341); let moves = 0; let screen = await readScreen(page);
  while (!screen.over && moves < CAP) { const via = await act(page, screen, pick); if (via === null) throw await deadEnd(page, game, moves); moves += 1; screen = await readScreen(page); }
  expect(apiCalls.slice(before), "no /api/ request of ANY kind once the local game is on screen").toEqual([]);
  // The download is the Worker's bytes; the archive copy equals it.
  const [download] = await Promise.all([page.waitForEvent("download"), page.getByRole("button", { name: "Save game file" }).click()]);
  const saved = fs.readFileSync(await download.path()!, "utf8");
  await page.getByRole("button", { name: "Send to the archive" }).click();
  await expect(page.getByText(/archived under browser\//)).toBeVisible();
  const archived = fs.readFileSync(path.join(archiveDir(), `browser/${SEGMENT}/rev7/over/${game.id}.json`), "utf8");
  expect(archived).toBe(saved);
  if (process.env.YOKOHAMA_WEB_UPDATE_PIN) {
    fs.writeFileSync(PIN_RECORD, saved);
    fs.writeFileSync(PIN_TABLE, fs.readFileSync(path.join(archiveDir(), `browser/${SEGMENT}/rev7/over/${game.id}.table.json`), "utf8"));
    fs.writeFileSync(PIN_SIDECAR, JSON.stringify({ release: RELEASE, date: new Date().toISOString().slice(0, 10), browser: browserName(), module_sha256: sha256(WASM) }, null, 2));
  }
  expect(watch.failures).toEqual([]);
});

test("a reload mid-play comes back from IndexedDB with the bot's cold pump preserved, and the replay wall is recorded", …);
test("a game closed before its first card is chosen comes back on reload with its setup (the table-only pair)", …);   // R1
test("a quota failure is APPLIED BUT NOT SAVED and the game stays exportable", async ({ page }) => {
  await page.addInitScript(() => { /* in every Worker: */ const orig = IDBObjectStore.prototype.put; let n = 0; IDBObjectStore.prototype.put = function (...a) { if (++n > 4) throw new DOMException("quota", "QuotaExceededError"); return orig.apply(this, a as never); }; });
  … play until the strip reads "APPLIED BUT NOT SAVED"; "Save game file" still downloads the record …
});
test("a real trap is recovered by a fresh local host, play continuing from the last committed move", async ({ page }) => {
  // built with `npm run build:wasm:debug`; the Worker exposes `debug_trap` through a `{ kind: "debug_trap" }` message under that build only
  … play two moves; send the trap; expect the strip "the local host restarted from the last saved move"; play on; the two moves are still on the log …
});
test("the first screen renders On this device with the API refused", async ({ page }) => {
  await page.route("**/api/**", (r) => r.fulfill({ status: 503, body: "down" }));
  await page.goto(BASE_URL);
  await expect(page.getByRole("heading", { name: "On this device" })).toBeVisible();
  await expect(page.getByText("not connected")).toBeVisible();
  // the words: a local game's option row carries a label, not a serde tag
});
test("two tabs: B booted first, A plays, B takes over and sees A's moves; A is read-only", async ({ page: a, context }) => {
  const b = await context.newPage(); await b.goto(BASE_URL);        // B's Worker boots on the lobby
  const game = await startGame(a, { local: true }); … play four moves on A …
  await b.goto(`${BASE_URL}#/t/${game.id}`);
  await expect(b.getByText("This game is open in another tab")).toBeVisible();
  await b.getByRole("button", { name: "Take over" }).click();
  await expect(b.locator(".log")).toContainText(lastMoveTextOnA);   // B's first frame shows A's moves
  await expect(a.getByText("opened in another tab")).toBeVisible();
  await expect(a.locator("button.option")).toHaveCount(0);
});
```

`SEGMENT` is `caller_segment` of the sign-in email the harness uses (`red_at_wire.test` if it signs in as `red@wire.test`); `archiveDir()` is a sibling of `gamesDir()` set on the `webServer` `env` as `YOKOHAMA_WEB_ARCHIVE_DIR`. `startGame(page, { local: true })` clicks "Play the bot here" instead of "New table" and expects the id off the create's 201 to start with `local-` — the create is a Worker call, so it is read off the SCREEN's route (`#/t/local-…`) rather than a response.

- [ ] **Step 2: Produce and commit the pair**

Run: `cd web && YOKOHAMA_WEB_UPDATE_PIN=1 npm run e2e -- plays_locally && git status --short ../crates/yokohama-web/tests/fixtures/`
Expected: the three files new. RECORD the game's length, the module hash and the O-3 wall the reload cell printed.

- [ ] **Step 3: The pin** (`crates/yokohama-web/tests/browser_played_pin.rs`):

```rust
//! The determinism pin (M6.5b, spec §6.4): a pair played by the WASM bot in a
//! real browser reconstructs, bot move by bot move, under the native `why`.
use std::sync::Arc;
use yokohama_web::api::{dispatch, ApiRequest, Host};
use yokohama_web::games::Games;
use yokohama_web::identity::User;
use yokohama_web::store::{Doc, MemStore};

const RECORD: &str = include_str!("fixtures/browser-played-2p.json");
const TABLE: &str = include_str!("fixtures/browser-played-2p.table.json");

#[test]
fn the_browser_played_pair_reconstructs_under_the_native_why() {
    let games = Games::with_parts(
        Arc::new(yokohama_core::Rules::standard()), Arc::new(MemStore::default()),
        yokohama_web::chat::ChatConfig::default(), Arc::new(yokohama_web::signal::NoSignal),
        Arc::new(|| 1_789_000_000), Arc::new(yokohama_web::archive::NoArchive),
    );
    let record: yokohama_core::GameRecord = serde_json::from_str(RECORD).unwrap();
    let doc: yokohama_web::table::TableDoc = serde_json::from_str(TABLE).unwrap();
    let id = doc.id.clone();
    games.load_document(Doc::Record, &id, RECORD).unwrap();
    games.load_document(Doc::Table, &id, TABLE).unwrap();
    let user = User::parse("local@browser").unwrap();
    let host = Host { identity_name: "local", signal_kind: "none" };
    let rules = yokohama_core::Rules::standard();
    // O-3: the rebuild's wall, RECORDED.
    let t = std::time::Instant::now();
    let _ = yokohama_cli::session::Session::from_record(record.clone(), rules.clone()).unwrap();
    eprintln!("Session::from_record over {} events: {:?}", record.events.len(), t.elapsed());
    let mut reconstructed = 0;
    for e in 0..record.events.len() {
        let mover = yokohama_core::state_at(&record, e, &rules).unwrap().current_player;
        let is_bot = record.run.as_ref().map_or(false, |r| r.seats[mover.0 as usize].controller != "human");
        if !is_bot { continue; }
        let req = ApiRequest { method: "GET", path: &format!("/api/games/{id}/why"), query: &format!("event={e}"), content_type: None, body: b"" };
        let res = dispatch(&games, &host, Ok(&user), &req);
        assert_eq!(res.status, 200, "event {e}: {}", res.body);
        let v: serde_json::Value = serde_json::from_str(&res.body).unwrap();
        assert!(v.get("ok").is_some(), "event {e} was REFUSED: {}", res.body);
        assert_eq!(v["ok"]["counters"]["placement_cap_binds"], 0, "event {e}: the scope partner");
        reconstructed += 1;
    }
    assert!(reconstructed > 50, "anti-vacuity: {reconstructed} bot events reconstructed");
}
```

(`record.run` / `seats[i].controller` — read the `RunProvenance` type in `yokohama_core` for the exact field names; the archived record in RUNBOOK §3.12 shows `"run": {"schema": 1, "seats": [{"controller": "human", …}, {"controller": "v4-step9-…"}]}`. The `why` payload's `ok.counters` shape is `web/src/wire/why.json`'s.)

Run: `cargo test -p yokohama-web --test browser_played_pin -- --nocapture`
Expected: 1 passed; the wall printed and RECORDED.

- [ ] **Step 4: Commit**

```bash
git add web/e2e/plays_locally.spec.ts web/e2e/harness.ts web/playwright.config.ts crates/yokohama-web/tests/browser_played_pin.rs crates/yokohama-web/tests/fixtures/
git commit -m "test(web): the exit — a whole local game with zero /api/games calls, the archive copy equal to the download, the reload, the API-refused screen, two tabs; the browser-played pair committed and reconstructed under the native why (spec §6.4, §6.5)"
```

---

### Task 9: Docs and the close

**Files:**
- Modify: `crates/yokohama-web/src/tables.rs:158-161` (the `NewGame.id` doc line), `deploy/RUNBOOK.md` (§1 the wasm build line; §3.13 the deploy record at the deploy), `CLAUDE.md` (the Web bullets: `npm run test:wasm`, `cargo test -p yokohama-wasm`, the pin binary, the counts; the M6.5b sentence in Where-things-stand), `docs/handover.md` (Track 3), the close `docs/plans/2026-09-20-web-frontend-m6.5b-close.md` (new), the M6.5c entry handover (new)

- [ ] **Step 1: The doc line** — `tables.rs:158-161`: "Tests and the fixture harness set it, and since M6.5b the browser's Worker sets it on every local create (`local-<16 hex>`, R-6.5b.12)."
- [ ] **Step 2: RUNBOOK §1** — after the musl line: the wasm build (`rustup target add wasm32-unknown-unknown`, `cargo install wasm-bindgen-cli --version 0.2.128 --locked`, `cd web && npm run build`, which now runs `build:wasm`), and that `web/dist/wasm/` rides in the context tarball unchanged in shape. §3.13 placeholder "written at the deploy".
- [ ] **Step 3: CLAUDE.md** — the Web (server) bullet gains the pin binary and `cargo test -p yokohama-wasm` with counts; the Web (browser half) bullet gains `npm run test:wasm` as the FOURTH command with its count; the Where-things-stand M6.5b sentence.
- [ ] **Step 4: The counts in ONE sequence** (web crate, the wasm crate, clippy, the wasm check, the CLI's 851 + 2 untouched, the fenced diff EMPTY, corpus count, typecheck, vitest, test:wasm, e2e with the wall) — RECORDED in the close §2.
- [ ] **Step 5: The mutation battery** (spec §6.6) by an Opus agent, report in the ledger; then the whole-branch Opus review and its one fix wave; the close written; **the M6.5c entry handover carrying spec §8's "carried to M6.5c" list verbatim** — the Free-tier confirmation the owner owes before R-6.5.12 is amended, gameplay and archive verification inside objects, the async archive seam (a prepare/upload/commit protocol with a receipt before any delete), hibernating WebSockets with the picker persisted, static assets outside the Worker, the house projection on visible change only, the long-save measurements, the browser crate's global as per-tab only; the board.
- [ ] **Step 6: Commit** — `docs(web): M6.5b close …`, PR against `web-room-m6.5`.

---

## Self-review (run by the plan's author before handing over)

**Spec coverage.** R-6.5b.1 (Task 3/4: `local@browser` in the Worker and the pin) · .2 (Task 5) · .3 (Task 2's table string; Task 6's send) · .4 (Task 4) · .5 (Task 4 + Task 8's two-tab cell; the phone measurement) · .6 (Tasks 2, 6) · .7 (Task 6) · .8 (Task 5) · .9 (Task 3) · .10 (Task 1) · .11 (the close's handover to M6.5c — Task 9) · .12 (Tasks 4, 5) · .13 (Task 1) · .14 (Task 3) · .15 (Task 2). §2.2's rebuild-on-acquire, release-on-navigation, the in-flight rule (Tasks 4, 6). §4's rebuild-refusal surface: the Worker answers the layer's 500 with `Session::from_record`'s sentence and does NOT enter the trap path — Task 4's `handle` returns the reply as-is (a 500 from `dispatch` is a reply, not a throw); the game screen shows the sentence; RECORD a cell for it in Task 4's `App.test.tsx` (a fixture `error.json`-shaped 500 on `GET …/{id}` renders the sentence and the row stays listed). §5's three columns (Task 7). §6.1–§6.6 (Tasks 1, 2, 4, 5, 6, 7, 8, 9).

**Placeholders.** Two steps say "the module's existing helper" (`playing_game_with_events`, `finished_local_pair`) and give the helper's definition if absent — acceptable, since the reader is told exactly what to write. Task 4's Worker leaves `lostWhileInFlight`'s placement to the implementer with the rule stated; Task 8's reload cell body is named by its assertions rather than written — the SDD implementer of Task 4 and the author of Task 8 write them from the spec's sentences. No "TBD".

**Type consistency.** `Archive::put(prefix, why, id, record, table) -> Result<(), PutError>` in Tasks 2 and 3's NoArchive; `paths(rev, prefix, why, id)` in Tasks 2 and 8; `RunQuery { from, to }` / `api.run(game, from, to)` / `RUN_CAP` in Tasks 1 and 7's replay (the `run.json` row); `Reply { status, body }` getters in Tasks 3, 4 and 7; `load(doc, id, text)` / `read(doc, id)` with `"record" | "table"` in Tasks 3, 4, 7 and 8; `facts(source)` in Tasks 4, 5, 6; `Source.host?: "local"` in Task 4 and `useSignal`.
