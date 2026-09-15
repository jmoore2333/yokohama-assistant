# Web frontend — M6.3 "transport and spectating": implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the client's poll with a server-pushed signal that a browser reads through one held connection, so that no tab on the NAS polls, and open every playing or finished game to a masked spectator who watches it walk — the room plan's M6.3 exit: no polling on the NAS; a spectator link shows a game walking.

**Architecture:** Server-first, then the measurement, then the client. `Games` gains a `watch` channel per table and one for the house, bumped after every write; `GET …/{id}/stream` writes the existing `Events` object on connect and on every bump as server-sent events with a heartbeat, and `GET /api/stream` writes a sequence number; the client's `usePolled` keeps M6.2's `Source` signature and reads the stream through `fetch` with the identity header (the browser's `EventSource` cannot carry it), reconnecting with the ladder it already has. A `Viewer` enum threads a spectator through the facts, the live frame and the replay frame — a non-acting seat's frame with every hand hidden and the one hand panel dropped — behind an admission function that opens the read routes to any signed-in caller at a playing or finished table once the table's password, if any, has been presented once through `unlock`. One CLI touch: the card index's seat becomes optional. The Durable Object notifier is designed in the spec and not built.

**Tech Stack:** axum 0.8 `response::sse` + tokio `sync` + `tokio-stream` (`WatchStream`), React 19 + TypeScript (a `ReadableStream` SSE reader), vitest + jsdom (a scripted fake `fetch`), Playwright against the release binary (three browsers), the Portainer Docker proxy (an rc deploy for the measurement, then the release).

**Spec:** `docs/plans/2026-09-15-web-frontend-m6.3-transport-and-spectating-spec.md` — the authority; this plan argues from it and records where it deviates. It executes AFTER M6.2 ships; its entry artifact is M6.2's close, and **every line citation below is at `48ae908` and MUST be re-read at Task 0 against M6.2's close HEAD** — the `games.rs` cut (M6.2 Task 1) moved the table lifecycle into `tables.rs`, and this plan names the M6.2 locations where it knows them.

## Global Constraints

- **Branch:** `web-room-m6.2` continues (the M6.2 close HEAD is the base; the owner may rename the branch), main checkout, explicit `git add` paths, the `Co-Authored-By` and executing `Claude-Session` trailers on every commit.
- **Blast radius (spec §9.6):** the five fenced crates EMPTY in `git diff master`; `SEMANTICS_REV` **7**; `crates/yokohama-cli`'s diff is EXACTLY the `Option` widening of `Repl::index_placements_for` and its one cell (Task 5), goldens and `guide_sync` untouched.
- **Per-edit gate:** `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings` when `crates/` moved (plus `cargo test -p yokohama-cli` at Task 5); `cd web && npm run typecheck && npm test` and `npm run e2e` when `web/` moved. The e2e is RED BY DESIGN from Task 7 (the hook stops polling) until Task 7's own end (the server stream exists from Task 2, so the walk is green again inside the same task).
- **The wire corpus is a contract**: Tasks 2, 4, 6 regenerate (`UPDATE_WIRE_FIXTURES=1 …`, then without, then `npm run typecheck`), every diff read and reported.
- **The spec's rulings**: SSE held to a measurement with the held request as fallback (R-3.1); no polling (R-3.2); the stream is per table (R-3.3); the house stream is a sequence only (R-3.4); who may watch (R-3.5); what a spectator gets (R-3.6) and is (R-3.7); one CLI touch (R-3.8); the lobby's `playing` scope (R-3.9); the notifier designed only (R-3.10); the dependencies (R-3.11); the heartbeat 25 s as an `AppState` field (R-3.12); `table` open to anyone at playing/over (R-3.13); `Last-Event-ID` ignored, `after` inert (R-3.14).
- **Counts** are M6.2's close figures, re-measured at Task 0 and Task 11, never copied.

---

## Execution model

As M6.2's: Opus implementers and per-task reviewers, Fable orchestrating and running the final review clean; the SDD workspace `.superpowers/sdd/2026-09-15-web-frontend-m6.3-plan/progress.md`, gitignored. Two outward-facing steps need the owner: the rc deploy for the measurement (Task 3) and the release deploy (Task 10).

---

## File structure

| File | Responsibility | Task |
|---|---|---|
| `crates/yokohama-web/src/signal.rs` (new), `games.rs`, `tables.rs`, `lib.rs`, `Cargo.toml` | `Signals`, `Bump`, the bump sites, `tokio` `sync`, `tokio-stream` | 1 |
| `crates/yokohama-web/src/lib.rs`, `tests/wire_fixtures.rs` | `GET …/{id}/stream`, `GET /api/stream`, `AppState.heartbeat`, `Events.status = "open"`, `Health.signal`, the on-connect frame, the `end` frame | 2 |
| `deploy/RUNBOOK.md` (§3.9's first half) | The rc deploy and the tunnel measurement; the R-3.1 decision | 3 |
| `crates/yokohama-web/src/client.rs`, `lib.rs`, `table.rs`, `tables.rs`, `tests/wire_fixtures.rs` | `Viewer`, `admitted`, the spectator projection, `unlock`, `TableDoc.unlocked`, `TableView.can_watch`, `table` at playing/over for anyone, the `playing` scope | 4 |
| `crates/yokohama-cli/src/repl.rs`, `crates/yokohama-web/src/lib.rs` | `index_placements_for(Option<PlayerId>)`; the web passes `None` for a spectator | 5 |
| `crates/yokohama-web/tests/wire_fixtures.rs`, `web/src/types.ts`, `web/src/api.ts`, `web/src/wire/contract*.ts` | The spectator fixtures, the stream-frame compare, the watcher's-email pin, the client types | 6 |
| `web/src/signal.ts` (+ `.test.ts`), `web/src/sse.ts` (+ `.test.ts`, new), `web/src/App.tsx`, `web/src/Dealing.tsx`, `web/src/OpenTable.tsx`, `web/src/Lobby.tsx` | The fetch-based reader; the poll leaves; the four consumers on the stream; the lobby on the house stream | 7 |
| `web/src/App.tsx`, `web/src/Decision.tsx`, `web/src/walk.ts` (+ `.test.ts`), `web/src/Lobby.tsx`, `web/src/Unlock.tsx` (+ `.test.tsx`, new), `web/src/styles.css` | The spectator screen, the walk's arrival, Games in progress, the unlock face | 8 |
| `crates/yokohama-web/tests/play_a_whole_game.rs`, `web/e2e/two_browsers.spec.ts` | The third identity that watches; the watching browser and the no-`/events` read | 9 |
| `deploy/RUNBOOK.md` (§3.9), `deploy/cloudflared-config.yml` | The release, the repeated measurement, the heartbeat sentence | 10 |
| `docs/plans/<date>-web-frontend-m6.3-close.md`, `CLAUDE.md`, `guidingpath.md`, `README.md`, `docs/handover.md` | The review and the close | 11 |

---

## Plan-level rulings (deviations from the spec's letter, recorded here per R-P11)

- **R-plan-1 — the measurement is Task 3, not the plan's first task.** The spec's R-3.1 holds `GET …/{id}/stream` through the tunnel, and there is no stream to hold until Tasks 1–2 build it; a release-candidate image (`0.5.0-m6.3-rc1`, owner-authorized) carries the server half to the NAS for the reading, and the client is written after the decision, against whichever reader won.
- **R-plan-2 — `admitted` returns `(TableDoc, Viewer)` and the routes that need a handle fetch it after**, exactly as `events` does at HEAD; the nine `seated` call sites become `admitted` + `handle` pairs.
- **R-plan-3 — the stand-in seat is `(current_player + 1) % players`**, one line, and the cells prove the three consequences at 2P, 3P and 4P rather than the choice of seat.
- **R-plan-4 — `Health.signal` reads `"sse"` or `"held"` from `AppState.signal_kind`**, set at construction from the R-3.1 decision as a compile-time constant in `lib.rs` (`const SIGNAL_KIND: &str`), not an environment variable: the fallback is a code decision recorded in the close, and a deployment cannot flip it.
- **R-plan-5 — under the held-request fallback, Task 7's reader is the SAME `fetch` to `…/events?wait=25`** and Task 2's `?wait` arm ships regardless (it costs one cell), so the fallback is a one-line change in the reader and not a second plan.

---

### Task 0: Pre-flight and baseline

- [ ] **Step 1: Re-read every citation.** From the M6.2 close HEAD: `grep -n 'fn seated\|fn events\|fn viewer_seat\|fn facts_for\|pub fn of\|pub fn at\|fn prompt_of_seat\|fn mirror_turn\|fn persist\|fn join\|fn keep\|fn finalize\|fn flip_to_playing\|fn delete\|fn create' crates/yokohama-web/src/*.rs`; `grep -n 'index_placements' crates/yokohama-cli/src/repl.rs`; `grep -n 'usePolled\|useSignal' web/src/*.ts web/src/*.tsx`. Write the current lines into the SDD ledger's pre-flight table beside the spec's numbers.

- [ ] **Step 2: Re-count.** `cargo test -p yokohama-web 2>&1 | grep 'test result'`; `cd web && npm test 2>&1 | grep -E 'Test Files|Tests '`; `npm run e2e 2>&1 | tail -2`; `ls web/src/wire/*.json | wc -l`. Record them.

- [ ] **Step 3: Open the ledger** with the plan path, the base commit, the counts, and the empty rulings table.

---

### Task 1: The channels — `signal.rs`, the bump sites, the dependencies (spec §2, R-3.11)

**Files:**
- Create: `crates/yokohama-web/src/signal.rs`
- Modify: `crates/yokohama-web/Cargo.toml`, `crates/yokohama-web/src/games.rs` (`Games.signals`, `persist`, `delete`), `crates/yokohama-web/src/tables.rs` (`create`, `join`, `keep`, `finalize`, `flip_to_playing`), `crates/yokohama-web/src/lib.rs` (`pub mod signal;`)
- Test: `crates/yokohama-web/src/signal.rs`, `crates/yokohama-web/src/tables.rs`, `crates/yokohama-web/src/games.rs`

**Interfaces:**
- Produces: `signal::Bump { seq: u64, end: bool }`; `signal::Signals` with `bump_table(&self, id)`, `bump(&self, id)`, `end(&self, id)`, `subscribe(&self, id) -> watch::Receiver<Bump>`, `subscribe_house(&self) -> watch::Receiver<u64>`; `Games.signals: Signals` (`pub(crate)`), `Games::signals(&self) -> &Signals`.

- [ ] **Step 1: The dependencies**

In `Cargo.toml`, `tokio`'s features gain `"sync"`, and after the `sha2` paragraph:

```toml
# M6.3 (owner-ruled 2026-09-15, spec R-3.11): the change signal. `tokio::sync::watch`
# is the in-process channel one bump per table rides on (it coalesces, which is the
# property a "something changed" signal wants), and `tokio-stream`'s `sync` feature
# turns a receiver into the `Stream` axum's SSE response consumes.
tokio-stream = { version = "0.1", features = ["sync"] }
```

- [ ] **Step 2: Write the failing unit tests in `signal.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_subscriber_before_the_first_bump_wakes_on_it_and_reads_the_latest_seq() {
        let s = Signals::default();
        let mut rx = s.subscribe("t");
        assert_eq!(rx.borrow().seq, 0);
        s.bump_table("t");
        s.bump_table("t");
        s.bump_table("t");
        assert!(rx.has_changed().unwrap(), "three bumps, one wake");
        let b = *rx.borrow_and_update();
        assert_eq!(b.seq, 3);
        assert!(!b.end);
        assert!(!rx.has_changed().unwrap(), "coalesced: nothing further pending");
    }

    #[test]
    fn bump_reaches_the_house_and_bump_table_does_not() {
        let s = Signals::default();
        let mut house = s.subscribe_house();
        s.bump_table("t");
        assert!(!house.has_changed().unwrap());
        s.bump("t");
        assert!(house.has_changed().unwrap());
        assert_eq!(*house.borrow_and_update(), 1);
    }

    #[test]
    fn end_sends_the_terminal_bump_removes_the_sender_and_bumps_the_house() {
        let s = Signals::default();
        let mut rx = s.subscribe("t");
        let mut house = s.subscribe_house();
        s.end("t");
        assert!(rx.borrow_and_update().end, "the terminal bump");
        assert!(house.has_changed().unwrap());
        // The sender is gone: the receiver reports the channel closed.
        assert!(rx.has_changed().is_err() || s.senders() == 0);
        // A later subscribe makes a fresh sender that has seen nothing.
        let fresh = s.subscribe("t");
        assert_eq!(fresh.borrow().seq, 0);
        assert!(!fresh.borrow().end);
    }
}
```

- [ ] **Step 3: `signal.rs`**

```rust
//! **The change signal (M6.3, spec §2): one `watch` channel per table, one for
//! the house.** A bump is sent AFTER the write it announces lands, never before,
//! so a subscriber that wakes and reads sees the new state. `watch` coalesces:
//! a subscriber that missed three bumps sees one — the property wanted per
//! table, and the reason the house channel carries a bare sequence (R-3.4).

use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::watch;

use crate::store::GameId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bump {
    pub seq: u64,
    /// Sent once, by `end`: the table was deleted and every stream on it closes.
    pub end: bool,
}

#[derive(Default)]
pub struct Signals {
    tables: Mutex<HashMap<GameId, watch::Sender<Bump>>>,
    house: Mutex<Option<watch::Sender<u64>>>,
}

impl Signals {
    fn table_sender(&self, id: &str) -> watch::Sender<Bump> {
        let mut tables = self.tables.lock().expect("signals lock");
        tables
            .entry(id.to_string())
            .or_insert_with(|| watch::channel(Bump::default()).0)
            .clone()
    }

    fn house_sender(&self) -> watch::Sender<u64> {
        let mut house = self.house.lock().expect("signals lock");
        house.get_or_insert_with(|| watch::channel(0).0).clone()
    }

    /// The table's channel only (a keep that does not finalize).
    pub fn bump_table(&self, id: &str) {
        let tx = self.table_sender(id);
        tx.send_modify(|b| b.seq += 1);
    }

    /// The table's channel AND the house.
    pub fn bump(&self, id: &str) {
        self.bump_table(id);
        self.house_sender().send_modify(|n| *n += 1);
    }

    /// The terminal bump, then the sender is REMOVED so every stream on the
    /// table ends; the house is bumped so a lobby drops the row.
    pub fn end(&self, id: &str) {
        let tx = self.tables.lock().expect("signals lock").remove(id);
        if let Some(tx) = tx {
            tx.send_modify(|b| {
                b.seq += 1;
                b.end = true;
            });
        }
        self.house_sender().send_modify(|n| *n += 1);
    }

    pub fn subscribe(&self, id: &str) -> watch::Receiver<Bump> {
        self.table_sender(id).subscribe()
    }

    pub fn subscribe_house(&self) -> watch::Receiver<u64> {
        self.house_sender().subscribe()
    }

    #[cfg(test)]
    pub fn senders(&self) -> usize {
        self.tables.lock().expect("signals lock").len()
    }
}
```

(`watch::Sender::send_modify` mutates and notifies even with no receivers; `subscribe()` on a sender is what makes the "subscribe before the first bump" cell pass. Note `tokio::sync::watch` is `Send + Sync`, so `Games` stays `Send + Sync`.)

- [ ] **Step 4: The bump sites, each with its gated cell**

`Games` gains `pub(crate) signals: Signals` (`Signals::default()` in `new`) and `pub fn signals(&self) -> &Signals`. Then, AFTER the write in each: `create` → `self.signals.bump(&doc.id)` after `write_table` succeeds; `join` → `self.signals.bump(id)` after its write; `keep` → `self.signals.bump_table(id)` after the keep's write (and, when it finalizes, `finalize`'s own bump follows); `finalize` and `flip_to_playing` → `self.signals.bump(id)` after the table flips to `Playing`; `persist` → `self.signals.bump(id)` after `mirror_turn` returns; `delete` → `self.signals.end(id)` after `store.delete` and the map removals.

The cells (in `tables.rs` / `games.rs` beside each subject, one per site): subscribe first, perform the write, assert `rx.has_changed()` and that reading the table document (or the record) after the wake shows the write; and ONE cell using the gated store (the one beside `a_delete_cannot_interleave_with_the_keep_that_finalizes`) that parks the record write inside `finalize` and asserts, from another thread, that the receiver has NOT changed while parked and HAS once the gate opens — the "a bump precedes no write" proof (mutation 5's partner). For `delete`, the cell asserts the receiver's value has `end: true` and the house receiver changed.

- [ ] **Step 5: Run and commit**

Run: `cargo test -p yokohama-web 2>&1 | grep 'test result' && cargo clippy --workspace --all-targets -- -D warnings`

```bash
git add crates/yokohama-web/Cargo.toml Cargo.lock crates/yokohama-web/src/signal.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/src/lib.rs
git commit -m "feat(web): the change signal — one watch channel per table and one for the house (tokio sync, tokio-stream), bumped after every write site and ended on delete; a bump never precedes its write, proven with the gated store (spec §2)"
```

---

### Task 2: The streams — `GET …/{id}/stream`, `GET /api/stream`, the heartbeat, `events` at `open`, `health.signal` (spec §3.1–§3.3, R-3.12, R-3.14)

**Files:**
- Modify: `crates/yokohama-web/src/lib.rs` (`AppState.heartbeat`, `AppState.signal_kind`, `Health.signal`, `events`, the two stream handlers, the router), `crates/yokohama-web/tests/wire_fixtures.rs` (regenerate; the stream-frame compare)
- Test: `crates/yokohama-web/src/lib.rs`

**Interfaces:**
- Produces: `AppState { …, heartbeat: Duration /* 25 s */, signal_kind: &'static str }` with `AppState::in_memory_with_heartbeat(Duration)`; `Health.signal: &'static str`; `Events.status` may read `"open"`; `GET /api/games/{id}/stream` and `GET /api/stream` as `text/event-stream`; `fn events_of(repl: &Repl, table: &TableDoc, viewer: Viewer-or-seat) -> Events` shared by `events` and the stream (Task 4 threads `Viewer` through it; until then it takes the `PlayerId` viewer as `events` computes it today); `GET …/{id}/events?wait=N` (R-plan-5).

- [ ] **Step 1: Write the failing route cells in `lib.rs`'s `mod tests`**

```rust
    async fn stream_frames(state: Arc<AppState>, uri: &str, who: &str, read_for: std::time::Duration) -> (StatusCode, String) {
        let res = app(state)
            .oneshot(Request::builder().uri(uri).header(identity::CLAIMED_HEADER, who)
                .header("accept", "text/event-stream").body(Body::empty()).expect("a GET"))
            .await.expect("the router answers");
        let status = res.status();
        let mut body = res.into_body().into_data_stream();
        let mut text = String::new();
        let deadline = tokio::time::Instant::now() + read_for;
        use tokio_stream::StreamExt;
        while let Ok(Some(Ok(chunk))) = tokio::time::timeout_at(deadline, body.next()).await {
            text.push_str(&String::from_utf8_lossy(&chunk));
        }
        (status, text)
    }

    #[tokio::test]
    async fn the_stream_writes_the_current_events_on_connect_then_one_frame_per_bump_and_an_end_on_delete() {
        let state = Arc::new(AppState::in_memory_with_heartbeat(std::time::Duration::from_millis(50)));
        // a playing 2P table by red and blue, both kept (use the lib helpers or three calls)
        …create, keep, keep as in the existing persist cells…
        let s = Arc::clone(&state);
        let reader = tokio::spawn(async move {
            stream_frames(s, "/api/games/wire/stream", WHO, std::time::Duration::from_millis(400)).await
        });
        tokio::time::sleep(std::time::Duration::from_millis(60)).await;
        // one move by the acting seat through the games layer, then a delete
        …post one legal input as the acting seat…
        state.games.delete("wire", &identity::User::parse(WHO).unwrap()).unwrap();
        let (status, text) = reader.await.unwrap();
        assert_eq!(status, StatusCode::OK);
        let frames: Vec<&str> = text.split("\n\n").filter(|f| f.starts_with("event:")).collect();
        assert!(frames[0].contains("event: signal") && frames[0].contains("\"events\":"), "on connect: {text}");
        assert!(frames.iter().any(|f| f.contains("event: signal") && f.contains("\"events\":1")), "the move's frame: {text}");
        assert!(frames.last().unwrap().contains("event: end") && frames.last().unwrap().contains("deleted"), "{text}");
        assert!(text.contains("\n:"), "at least one heartbeat comment in 400 ms at 50 ms: {text}");
        assert!(!text.contains("\nid:") && !text.contains("\nretry:"), "no id or retry line (R-3.14): {text}");
        let data: Vec<&str> = text.lines().filter(|l| l.starts_with("data: ")).collect();
        let first: serde_json::Value = serde_json::from_str(&data[0][6..]).expect("one-line JSON");
        let (st, events) = get_with(Arc::clone(&state), "/api/games/wire/events", &[(identity::CLAIMED_HEADER, WHO)]).await;
        let _ = st; // the table is deleted by now; compare the FIRST frame with a fresh events read made BEFORE the delete instead — restructure so the events read happens right after connect
        assert_eq!(first["status"], "playing");
    }

    #[tokio::test]
    async fn the_house_stream_writes_nothing_on_connect_and_a_seq_per_table_write() {
        let state = Arc::new(AppState::in_memory_with_heartbeat(std::time::Duration::from_millis(50)));
        let s = Arc::clone(&state);
        let reader = tokio::spawn(async move {
            stream_frames(s, "/api/stream", WHO, std::time::Duration::from_millis(300)).await
        });
        tokio::time::sleep(std::time::Duration::from_millis(60)).await;
        …one create by WHO…
        let (status, text) = reader.await.unwrap();
        assert_eq!(status, StatusCode::OK);
        let frames: Vec<&str> = text.split("\n\n").filter(|f| f.starts_with("event: house")).collect();
        assert_eq!(frames.len(), 1, "exactly the create's bump, none on connect: {text}");
        assert!(frames[0].contains("\"seq\":"));
    }

    #[tokio::test]
    async fn events_answers_an_open_table_with_zero_events_and_status_open() {
        …create an open table by WHO; GET /events as WHO → 200 {"events":0,"status":"open","to_move":null,"your_turn":false}…
    }

    #[tokio::test]
    async fn health_names_the_signal() {
        let (_, v) = get_json("/api/health").await;
        assert_eq!(v["signal"], "sse");
    }
```

(Restructure the first cell so the "first frame equals a fresh `events` read" compare is made against an `events` call issued right after the reader connected and before the move; the sketch above marks where.)

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p yokohama-web --lib the_stream_`
Expected: no route, no `in_memory_with_heartbeat`.

- [ ] **Step 3: The handlers**

In `lib.rs`:

```rust
pub const SIGNAL_KIND: &str = "sse";   // R-plan-4; "held" if R-3.1's measurement falls back
pub const HEARTBEAT: std::time::Duration = std::time::Duration::from_secs(25);

pub struct AppState {
    pub rules: Arc<Rules>,
    pub games: Games,
    pub identity: identity::IdentityMode,
    /// The SSE comment cadence (R-3.12). A field so a test can read heartbeats
    /// off the wire at 50 ms; nothing on a built `Sse` is inspectable.
    pub heartbeat: std::time::Duration,
}
// `new`, `in_memory`, `in_memory_with`, `with_store` set `heartbeat: HEARTBEAT`;
// add `pub fn in_memory_with_heartbeat(d: Duration) -> Self`.

fn events_of(repl: &yokohama_cli::repl::Repl, viewer: yokohama_core::PlayerId) -> Events {
    let s = repl.session.state();
    let over = repl.session.is_over();
    Events {
        events: repl.session.record().events.len(),
        to_move: Some(s.current_player),
        status: if over { "over" } else { "playing" },
        your_turn: !over && s.current_player == viewer,
    }
}

fn events_at(state: &AppState, id: &str, user: &identity::User) -> Result<Events, ApiError> {
    let table = state.games.table(id, user)?;          // Task 4 replaces this with `admitted`
    match table.status {
        crate::table::TableStatus::Open => Ok(Events { events: 0, to_move: None, status: "open", your_turn: false }),
        crate::table::TableStatus::Dealing => Ok(Events { events: 0, to_move: None, status: "dealing", your_turn: false }),
        crate::table::TableStatus::Playing => {
            let handle = state.games.handle(id)?;
            let repl = handle.lock().expect("a game");
            let viewer = viewer_of(&repl, &table, user)?;
            Ok(events_of(&repl, viewer))
        }
    }
}

async fn events(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    QueryArgs(q): QueryArgs<EventsQuery>,
) -> ApiResult<Events> {
    if let Some(wait) = q.wait {                       // R-plan-5: the held-request arm
        let wait = wait.min(25);
        let mut rx = state.games.signals().subscribe(&id);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(wait), rx.changed()).await;
    }
    Ok(Json(events_at(&state, &id, &user)?))
}

async fn stream(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<axum::response::sse::Sse<impl futures_core::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>, ApiError> {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use tokio_stream::StreamExt;
    let first = events_at(&state, &id, &user)?;         // the admission IS the events route's
    let rx = state.games.signals().subscribe(&id);
    let s = Arc::clone(&state);
    let user = user.clone();
    let id2 = id.clone();
    let frames = tokio_stream::wrappers::WatchStream::new(rx).map(move |bump| {
        if bump.end {
            return Ok(Event::default().event("end").data("{\"reason\":\"deleted\"}"));
        }
        match events_at(&s, &id2, &user) {
            Ok(e) => Ok(Event::default().event("signal").json_data(e).expect("Events serializes")),
            Err(_) => Ok(Event::default().event("end").data("{\"reason\":\"gone\"}")),
        }
    });
    let _ = first;   // `WatchStream::new` yields the current value at once, which IS the on-connect frame
    Ok(Sse::new(frames).keep_alive(KeepAlive::new().interval(state.heartbeat)))
}

async fn house_stream(
    identity::Identity(_user): identity::Identity,
    State(state): State<Arc<AppState>>,
) -> axum::response::sse::Sse<impl futures_core::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use tokio_stream::StreamExt;
    let rx = state.games.signals().subscribe_house();
    let frames = tokio_stream::wrappers::WatchStream::from_changes(rx)
        .map(|seq| Ok(Event::default().event("house").data(format!("{{\"seq\":{seq}}}"))));
    Sse::new(frames).keep_alive(KeepAlive::new().interval(state.heartbeat))
}
```

(`futures_core` is a transitive dependency of axum; if naming it needs a direct dependency, add `futures-core = "0.3"` beside `tokio-stream` with one sentence in the paragraph — or write the return type as `impl IntoResponse`, which is simpler and what axum's own SSE example does.) The stream on a table that ends with `end` must ALSO close: `WatchStream` ends when the sender is dropped, which `Signals::end` does after sending, so the `end` frame is the last item. `EventsQuery` gains `#[serde(default)] pub wait: Option<u64>` (validated ≤ 25 → 400 above). `Health` gains `signal: SIGNAL_KIND`. The router gains `.route("/api/games/{id}/stream", get(stream))` and `.route("/api/stream", get(house_stream))`.

- [ ] **Step 4: The wire — `health.json` re-cut, `events_open.json`, and the frame compare**

`wire_fixtures.rs`: capture `events_open` on the open table before the join (`api.get("/api/games/wire/events")` as WALKER right after create); add a test `the_stream_s_first_frame_is_the_events_route_s_reply_by_value`: through a playing table, open the stream with the `accept` header, read up to the first blank line, parse the `data:` line as JSON, and `assert_eq!` it to `api.get("/api/games/wire/events")` parsed — value equality, never bytes (spec §8). Regenerate: `health.json` gains `"signal":"sse"`, `events_open.json` is new; nothing else changes.

- [ ] **Step 5: Run and commit**

```bash
git add crates/yokohama-web/src/lib.rs crates/yokohama-web/Cargo.toml Cargo.lock crates/yokohama-web/tests/wire_fixtures.rs web/src/wire/health.json web/src/wire/events_open.json
git commit -m "feat(web): GET …/{id}/stream and GET /api/stream — server-sent events over the watch channels, the current Events on connect, one frame per bump, an end frame on delete, a heartbeat read off the wire at a test cadence, no id or retry line (R-3.14); events answers an open table; health names the signal (spec §3.1–§3.3)"
```

---

### Task 3: The measurement — an rc deploy and ten minutes through the tunnel (R-3.1, R-plan-1)

**Files:**
- Modify: `deploy/RUNBOOK.md` (the first half of §3.9: the rc deploy and the two readings)

- [ ] **Step 1: Owner authorization for `yokohama-web:0.5.0-m6.3-rc1`.** Build and deliver per §1–§3 of the runbook; the container's `Env` unchanged. This image carries the stream and NOT the client change, so the live site keeps polling meanwhile and nobody's game is disturbed.

- [ ] **Step 2: Reading A — with the heartbeat.** From the workstation, WARP on, against a playing table the owner names:

```bash
curl -sN -H "X-Yokohama-Claimed-User: $ME" -H 'Accept: text/event-stream' \
  "https://yokohama.moore.nyc/api/games/$ID/stream" | ts '[%H:%M:%.S]' | tee /tmp/stream-a.log
```

Hold ten minutes. At minute nine, make one move in that game from a browser. Record: the on-connect frame's timestamp, every heartbeat comment's timestamp (expected every 25 s), whether the frame at minute nine ARRIVED, and whether the connection was still open at minute ten.

- [ ] **Step 3: Reading B — without the heartbeat.** Deploy nothing: set `AppState.heartbeat` cannot be changed live, so run reading B against a LOCAL release build reached through a temporary tunnel route? No — the spec's reading is of the EDGE's behaviour on an idle stream, and the edge cannot be reached without the tunnel. So: run reading B on the rc image with `curl` and a request that the server answers but never bumps — a stream on an OPEN table that nobody joins — and compare heartbeat arrival with reading A's; then, to measure the drop, temporarily build `-rc2` with `HEARTBEAT = 3600 s`, deploy it, hold the stream, and record when the edge closes it (expected near 100 s). Redeploy `-rc1` after. Both rc deploys are owner-authorized.

- [ ] **Step 4: The decision.** If reading A held ten minutes with the frame arriving: **SSE ships**; `SIGNAL_KIND` stays `"sse"`. If the stream dropped with the heartbeat on: **the held request ships** — `SIGNAL_KIND = "held"`, and Task 7's reader uses `…/events?wait=25`. Record both readings, verbatim timestamps, in the ledger and in `deploy/RUNBOOK.md` §3.9 (first half); the close's §2 carries them.

---

### Task 4: The spectator on the server — `Viewer`, `admitted`, the projection, `unlock`, `table` at playing/over, the `playing` scope (spec §3.4, §3.5, §4, R-3.5–R-3.7, R-3.9, R-3.13)

**Files:**
- Modify: `crates/yokohama-web/src/client.rs` (`Viewer`, `facts_for`, `ClientState::of`, `HistoricalFrame::at`, `prompt_of_viewer`), `crates/yokohama-web/src/lib.rs` (`admitted`, the nine call sites, `events_at`, `unlock`, the router), `crates/yokohama-web/src/table.rs` (`unlocked`, `can_watch`), `crates/yokohama-web/src/tables.rs` (`unlock`, `table_for_anyone`, `Scope::Playing`, `check_shape`), `crates/yokohama-web/tests/wire_fixtures.rs` (regenerate)
- Test: `crates/yokohama-web/src/client.rs`, `lib.rs`, `tables.rs`, `table.rs`

**Interfaces:**
- Produces: `client::Viewer { Seat(PlayerId), Spectator }` (`Copy`); `client::facts_for(repl, viewer: Viewer)`; `ClientState::of(id, repl, viewer: Viewer, messages)` and `HistoricalFrame::at(repl, k, viewer: Viewer)` with `seat: Option<PlayerId>`; `client::stand_in(state) -> PlayerId`; `lib::admitted(state, id, user) -> Result<(TableDoc, Viewer), ApiError>`; `Games::unlock(id, user, password) -> Result<TableView, GameError>`; `TableDoc.unlocked: Vec<String>`, `TableDoc::can_watch(&self, user) -> bool`; `TableView.can_watch: bool`; `Scope::Playing`; `POST /api/games/{id}/unlock` with `Unlock { password: String }`.

- [ ] **Step 1: The document and the view**

`table.rs`: `TableDoc` gains `#[serde(default)] pub unlocked: Vec<String>`; `TableView` gains `pub can_watch: bool`; in `view()`: `can_watch: self.can_watch(user)`, with

```rust
    /// A seat holder always; anyone at a table with no lock; a caller on the
    /// `unlocked` list (M6.3, R-3.5).
    pub fn can_watch(&self, user: &User) -> bool {
        !self.seats_of(user).is_empty()
            || self.lock.is_none()
            || self.unlocked.iter().any(|u| u == user.as_str())
    }
```

`check_shape` (`tables.rs`) gains: `unlocked.len() <= 256` and each entry parses as a `User`. Cells in `table.rs`: the schema-1 read cell extends to `unlocked: []`; `can_watch`'s three arms; the view carries `can_watch` and never `unlocked` (by key).

- [ ] **Step 2: `unlock`, `table_for_anyone` widened, the third scope (`tables.rs`)**

```rust
pub const UNLOCKED_CAP: usize = 256;

impl Games {
    pub fn unlock(&self, id: &str, user: &User, password: &str) -> Result<TableView, GameError> {
        let lock = self.lock_for(id);
        let _serialized = lock.lock().expect("table lock");
        let (mut doc, version) = self
            .read_table(id)?
            .ok_or_else(|| GameError::NotFound(format!("no game named {id:?}")))?;
        match doc.status {
            TableStatus::Open => return Err(GameError::Open),
            TableStatus::Dealing => return Err(GameError::Dealing),
            TableStatus::Playing => {}
        }
        if doc.lock.is_none() || doc.can_watch(user) {
            return Ok(doc.view(user));                       // nothing to unlock, or already unlocked
        }
        if !doc.check_password(password.trim()) {
            return Err(GameError::Forbidden("wrong password for this table".to_string()));
        }
        if doc.unlocked.len() >= UNLOCKED_CAP {
            return Err(GameError::Conflict(
                "this table has as many watchers as it can remember".to_string(),
            ));
        }
        doc.unlocked.push(user.as_str().to_string());
        match self.write_table(&doc, Some(version)) {
            Ok(_) => Ok(doc.view(user)),
            Err(WriteError::Conflict { .. }) => Err(GameError::Conflict(format!("{id:?} changed while this unlock was in flight"))),
            Err(WriteError::Io(e)) => Err(GameError::Server(e)),
        }
    }
}
```

`table_for_anyone` (M6.2 Task 5) widens per R-3.13: `if doc.status != TableStatus::Dealing || !doc.seats_of(user).is_empty() { return Ok(doc) }` — i.e. open, playing and over read for anyone; dealing for seat holders. `Scope` gains `Playing` (`"playing"`), keeping `status == Playing` (over included, since over is a mirror fact on a playing table). Cells: unlock's seven answers (404; 409 at open and at dealing carrying the status; no-lock 200 with no write — assert the store's version unchanged; wrong password 403; success 200 with `can_watch` true and the email on the document; the 257th a 409; a seat holder's 200 no-op); dedup (unlocking twice leaves one entry); the two-watcher cell (green unlocks, blue is still refused); `table_for_anyone` at playing for a stranger 200 with `can_watch: false` at a locked table; the `playing` scope lists a playing and an over table and not an open or dealing one.

- [ ] **Step 3: `Viewer` and the projection (`client.rs`)**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Viewer {
    Seat(PlayerId),
    /// A signed-in caller with no seat, admitted at a playing or finished table
    /// (M6.3, R-3.5). Every hand hidden, no options, `seat: null` on the wire.
    Spectator,
}

/// The seat a spectator's frame is BUILT for (spec §4, R-plan-3): any seat that
/// is not acting. `build_dashboard` uses the seat only for its own hand panel,
/// which the facts hide and the projection then drops; `build_decision_for`
/// returns the not-deciding arm for any non-acting seat.
pub fn stand_in(state: &GameState) -> PlayerId {
    PlayerId::new((state.current_player.0 + 1) % state.players.len() as u8)
}

pub fn facts_for(repl: &Repl, viewer: Viewer) -> Vec<SeatFacts> {
    repl.session
        .state()
        .players
        .iter()
        .map(|p| SeatFacts {
            is_bot: repl.bots().is_bot(p.id),
            hand_hidden: match viewer {
                Viewer::Seat(s) => p.id != s,
                Viewer::Spectator => true,
            },
        })
        .collect()
}

fn built_for(state: &GameState, viewer: Viewer) -> PlayerId {
    match viewer {
        Viewer::Seat(s) => s,
        Viewer::Spectator => stand_in(state),
    }
}

/// A spectator's dashboard carries no `Hand` panel: the builder emits the
/// stand-in seat's panel hidden (spec §0.1), which would single one seat out.
fn drop_hand_for(viewer: Viewer, frame: &mut FrameModel) {
    if matches!(viewer, Viewer::Spectator) {
        frame.dashboard.retain(|p| !matches!(p, screen_model::Panel::Hand(_)));
    }
}

fn prompt_of_viewer(repl: &Repl, viewer: Viewer) -> PromptState {
    match viewer {
        Viewer::Seat(s) => prompt_of_seat(repl, s),
        Viewer::Spectator => {
            if repl.session.record().lockdown.is_none() && !repl.session.is_over() {
                PromptState::Watching { to_move: repl.session.state().current_player }
            } else {
                prompt_of(repl)
            }
        }
    }
}
```

`ClientState.seat` and `HistoricalFrame.seat` become `Option<PlayerId>`; `of` and `at` take `viewer: Viewer`, build with `built_for(state, viewer)` and `facts_for(repl, viewer)`, call `drop_hand_for(viewer, &mut frame)`, set `seat: match viewer { Viewer::Seat(s) => Some(s), Viewer::Spectator => None }`, `your_turn: !over && matches!(viewer, Viewer::Seat(s) if s == state.current_player)`, `prompt: prompt_of_viewer(repl, viewer)`. (`Panel::Hand`'s exact variant name: read `screen_model::Panel` — the M6.1 gate matches `p.get("Hand")` on the JSON, so the variant is `Hand`.) `viewer_seat` is unchanged and wrapped: `viewer_of` (`lib.rs`) returns `Viewer::Seat(…)`.

Cells in `client.rs`: over a playing 2P, 3P and 4P `Repl` (the M6.1 helpers), `ClientState::of(…, Viewer::Spectator, …)` has `seat: None`, `your_turn: false`, prompt `Watching`, every `Players` row `hand_hidden: true`, NO `Hand` panel, `frame.decision.viewer_is_deciding == false`, `frame.decision.options.is_empty()`, and `frame.decision.seat == current_player`; the same for `at(…, k, Viewer::Spectator)` at `k = total` and `k = 0`; a BEFORE-drop assertion that `build_frame` for `stand_in` with all-hidden facts carries a `Hand` panel with `hidden: true` and `cards: []` (the mechanism, pinned so the drop is known to be removing something real); over → prompt `GameOver` and `scores` present; the corpus byte-identity for seated payloads is the wire test's job (Step 6).

- [ ] **Step 4: `admitted` and the nine call sites (`lib.rs`)**

```rust
/// Every session route starts here (M6.3): the TABLE first, as `seated` read
/// it, then WHO the caller is at it. No handle — a route that needs one asks
/// `Games::handle` after, which is what lets `events` and `stream` answer a
/// table with no record.
fn admitted(
    state: &AppState,
    id: &str,
    user: &identity::User,
) -> Result<(crate::table::TableDoc, client::Viewer), ApiError> {
    use crate::table::TableStatus as S;
    let table = state.games.table_for_anyone(id, user)?;        // 404 / 403 not seated at dealing
    if !table.seats_of(user).is_empty() {
        // The viewer seat needs the state at playing; at open/dealing the routes that
        // reach a handle refuse first, so `Seat(lowest)` is a placeholder they never read.
        let seat = match table.status {
            S::Playing => {
                let handle = state.games.handle(id)?;
                let repl = handle.lock().expect("a game");
                client::viewer_seat(&table, user, repl.session.state()).expect("a seat holder has a viewer seat")
            }
            _ => table.seats_of(user)[0],
        };
        return Ok((table, client::Viewer::Seat(seat)));
    }
    match table.status {
        S::Open | S::Dealing => Err(ApiError { status: StatusCode::FORBIDDEN, message: "you are not seated at this table".into(), table_status: None }),
        S::Playing if table.can_watch(user) => Ok((table, client::Viewer::Spectator)),
        S::Playing => Err(ApiError { status: StatusCode::FORBIDDEN, message: "this table is locked — unlock it first".into(), table_status: None }),
    }
}
```

(Reading the handle inside `admitted` for a seat holder at `playing` is what `seated` did; the lock is released before returning.) Then every `seated(&state, &id, &user)?` becomes `let (table, viewer) = admitted(&state, &id, &user)?; let handle = state.games.handle(&id)?;` and the route reads `viewer` instead of `viewer_of(…)`. **The write routes refuse a `Spectator` before touching the handle**: `input`, `keep`, `deal`, `join` (already status-gated), `suggest`, `delete_game` — each gains, right after `admitted`, `if matches!(viewer, client::Viewer::Spectator) { return Err(ApiError { status: FORBIDDEN, message: "you are not seated at this table", .. }) }` (for `suggest` the existing R-1.8 sentence is kept for a non-acting HOLDER; a spectator gets the not-seated one). `events_at` (Task 2) reads `admitted` and, at `Playing`, `events_of(&repl, viewer)` with `your_turn` false for a spectator. `card_index` passes `match viewer { Seat(s) => Some(s), Spectator => None }` to the CLI (Task 5 makes that compile — do Tasks 4 and 5 in one implementer dispatch, or land Task 5 first; the plan orders them 4 then 5 so the CLI diff is its own commit, and Task 4's commit may leave `card_index` passing `Some(stand_in)` with a `// Task 5` comment for exactly one commit). `why` is unchanged. `unlock`:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unlock { pub password: String }

async fn unlock(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    JsonBody(req): JsonBody<Unlock>,
) -> ApiResult<crate::table::TableView> {
    Ok(Json(state.games.unlock(&id, &user, &req.password)?))
}
```

Router: `.route("/api/games/{id}/unlock", post(unlock))`. `list_games` parses `"playing"` too.

Cells in `lib.rs`: the whole-table admission cell of spec §3.5 — five caller rows × four statuses (open, dealing, playing, over) × every route, asserting the table's cell, and asserting `stream`'s status equals `events`'s in EVERY cell (the read of `stream` is its status code alone here — the router answers before the body streams); the write sweep as a spectator at playing and at over (403 each, the handle never touched: use a store that panics on `Doc::Record` reads for the sweep's spectator calls — the write routes must refuse BEFORE `handle`); `card_index` as a spectator: no `OwnHand` location (after Task 5); `why` as a spectator equals `why` as the non-acting player (byte-equal JSON).

- [ ] **Step 5: Regenerate and read the diff**

Expected: every `ClientState` and `HistoricalFrame` fixture BYTE-IDENTICAL (the `Option` serializes as the number); every `TableView` fixture gains `"can_watch": …`. Anything else is a finding.

- [ ] **Step 6: Run and commit**

```bash
git add crates/yokohama-web/src/client.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/table.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/tests/wire_fixtures.rs web/src/wire
git commit -m "feat(web): the spectator — Viewer threads a seat or a watcher through the facts, the live frame and the replay frame (a non-acting seat's frame with every hand hidden and the hand panel dropped); admitted() opens the read routes to any signed-in caller at a playing or finished table once its password has been presented through POST …/{id}/unlock; table reads for anyone but at dealing; the playing scope (spec §3.4, §3.5, §4)"
```

---

### Task 5: The one CLI touch — `index_placements_for(Option<PlayerId>)` (R-3.8)

**Files:**
- Modify: `crates/yokohama-cli/src/repl.rs` (`index_placements`, `index_placements_for`), `crates/yokohama-web/src/lib.rs` (`card_index`)
- Test: `crates/yokohama-cli/src/repl.rs` (one cell), `crates/yokohama-web/src/lib.rs`

- [ ] **Step 1: The failing CLI cell**

In `repl.rs`'s tests, over a `Repl` at a state where `current_player` holds order cards (the existing index cells have one — reuse their fixture):

```rust
    #[test]
    fn a_spectators_index_carries_no_own_hand_row() {
        let repl = …the existing fixture with cards in the acting seat's hand…;
        let with = repl.index_placements();
        assert!(with.iter().any(|(_, l)| matches!(l, CardLocation::OwnHand)), "the fixture holds cards");
        let without = repl.index_placements_for(None);
        assert!(without.iter().all(|(_, l)| !matches!(l, CardLocation::OwnHand)));
        assert_eq!(
            without.len(),
            with.iter().filter(|(_, l)| !matches!(l, CardLocation::OwnHand)).count(),
            "None removes exactly the OwnHand rows and nothing else",
        );
    }
```

- [ ] **Step 2: The widening**

```rust
    pub fn index_placements(&self) -> Vec<(CardRef, CardLocation)> {
        self.index_placements_for(Some(self.session.state().current_player))
    }

    /// `me` is the seat whose hand is marked `OwnHand`; `None` (a spectator,
    /// M6.3) marks no hand at all.
    pub fn index_placements_for(&self, me: Option<PlayerId>) -> Vec<(CardRef, CardLocation)> {
        …unchanged, except the hand block becomes…
        if let Some(me) = me {
            if !self.seat_display().hides_hand(me) {
                if let Some(p) = state.players.iter().find(|p| p.id == me) {
                    for &id in &p.order_cards_in_hand {
                        placed.push((CardRef::Order(id), CardLocation::OwnHand));
                    }
                }
            }
        }
```

`card_index` in `lib.rs` passes `match viewer { Viewer::Seat(s) => Some(s), Viewer::Spectator => None }` and Task 4's comment goes.

- [ ] **Step 3: Prove the CLI moved nothing**

Run: `cargo test -p yokohama-cli 2>&1 | grep 'test result' && cargo test -p yokohama-web --lib card_index && git diff --stat master -- crates/yokohama-cli`
Expected: `823 passed` lib (822 + this cell), `9`, `18`, `1 + 2 ignored`; the CLI diff is `repl.rs` alone.

```bash
git add crates/yokohama-cli/src/repl.rs crates/yokohama-web/src/lib.rs
git commit -m "feat(cli): Repl::index_placements_for takes Option<PlayerId> — None marks no hand, for a spectator's card index; index_placements delegates with Some(current_player) as before; goldens and guide_sync untouched (R-3.8)"
```

---

### Task 6: The wire — spectator fixtures, the watcher's-email pin, the client's types and calls (spec §8, §5.4)

**Files:**
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs`, `web/src/wire/*.json`, `web/src/types.ts`, `web/src/api.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`

**Interfaces:**
- Produces (client): `Events.status` gains `"open"`; `TableView.can_watch: boolean`; `ClientState.seat: PlayerId | null`, `HistoricalFrame.seat: PlayerId | null`; `Health.signal: "sse" | "held"`; `UnlockRequest { password: string }`; `api.unlock(game, password)`, `api.list(scope: "mine" | "open" | "playing", before?)`; `wire.spectator`, `wire.spectatorAtK`, `wire.spectatorOver`, `wire.gamesPlaying`, `wire.tableWatchable`, `wire.unlock403`, `wire.unlock409Open`, `wire.watch403Locked`, `wire.eventsOpen`, `wire.houseFrame` (not a route file — see below).

- [ ] **Step 1: The walk gains a third identity**

`const GREEN: &str = "green@wire.test";`. After the table is playing (after the last keep): `watch_403_locked` = GET `/api/games/wire` as GREEN (403, the lock); `unlock_403` = POST `/unlock` `{"password":"open"}` as GREEN; `unlock_409_open` = POST `/unlock` on the SECOND table `wire2` while it is open (create `wire2` with an open seat for this — M6.2's walk made it a bot table; make it `{"user": WALKER}, {"open": true}` so it stays open, and let `games_next`/`gamesNext` keep working); then POST `/unlock` `{"password": WIRE_PASSWORD}` as GREEN → 200 with `can_watch: true`; `table_watchable` = GET `/table` as a FOURTH identity `STRANGER` at the locked playing table (200, `can_watch: false`); `spectator` = GET `/api/games/wire` as GREEN; `spectator_at_k` = GET `/at/1` as GREEN; `games_playing` = GET `?scope=playing` as GREEN; `spectator_over` is captured in the walk's game-over branch (where `game_over.json` is captured) as GREEN. The value pin: `no_payload_built_for_someone_else_carries_a_watchers_email` — after green unlocks, every payload the walk captured for WALKER or OTHER is searched for `GREEN`'s text; the anti-vacuity partner is the table document carrying it in `unlocked`.

- [ ] **Step 2: Regenerate and read the diff**

Expected new: the nine files above; changed: `games.json`/`games_next.json` only if the second table's shape moved (report it). Every `ClientState`/`HistoricalFrame` fixture for a seated caller is byte-identical to Task 4's regeneration.

- [ ] **Step 3: The client's types and contract**

`types.ts`: `Events.status: "open" | "dealing" | "playing" | "over"`; `TableView.can_watch: boolean`; `ClientState.seat: PlayerId | null` and `HistoricalFrame.seat: PlayerId | null` with a doc line each ("`null` for a spectator (M6.3)"); `Health.signal`. `api.ts`: `UnlockRequest`, `api.unlock: (game, password: string) => call<TableView>("POST", `/api/games/${id(game)}/unlock`, { password })`, `api.list`'s scope union gains `"playing"`. `contract.ts`/`contract.test.ts`: the new files in `wire` and `KEYS` (`"can_watch"` on `TABLE`, `"signal"` on `HEALTH`), rows for each new fixture, and a cell that `wire.spectator.seat === null` and every `Players` row in it has `hand_hidden: true` and no `Hand` panel is in its dashboard.

Run: `cd web && npm run typecheck`
Expected: it FAILS on every reader of `state.seat` that assumed a number — that list is Task 8's input; record the file:line pairs in the ledger. Then, in this task, make the typecheck green by the narrowest means: where `state.seat` is passed to a `PlayerId` parameter, guard with `state.seat !== null ? … : …` and leave a `// M6.3 Task 8` marker; Task 8 replaces every marker with the spectator behaviour.

- [ ] **Step 4: Commit**

```bash
git add crates/yokohama-web/tests/wire_fixtures.rs web/src/wire web/src/types.ts web/src/api.ts web/src/wire/contract.ts web/src/wire/contract.test.ts
git commit -m "test(web): the wire walk gains a third identity that unlocks and watches — the spectator's live frame, replay frame and finished frame, the playing scope, the locked-table refusals, and the watcher's-email pin; the client's types make seat nullable and health carry the signal (spec §8)"
```

---

### Task 7: The client's reader — the poll leaves, the stream is read through `fetch`, the four consumers move (spec §5.1, R-3.2)

**Files:**
- Create: `web/src/sse.ts`, `web/src/sse.test.ts`
- Modify: `web/src/signal.ts`, `web/src/signal.test.ts`, `web/src/App.tsx`, `web/src/Dealing.tsx`, `web/src/OpenTable.tsx`, `web/src/Lobby.tsx`, `web/src/api.ts` (the header helper exported)
- Test: `web/src/sse.test.ts`, `web/src/signal.test.ts`

**Interfaces:**
- Produces: `sse.ts` — `interface Frame { event: string; data: string }`, `parseSse(): (chunk: string) => Frame[]` (a stateful line parser), `readStream(url: string, headers: HeadersInit, signal: AbortSignal, onFrame: (f: Frame) => void): Promise<"ended" | "closed">` (resolves `"closed"` when the body ends without an `end` frame, `"ended"` on `event: end`; rejects with `ApiError` on a non-200); `usePolled` unchanged in signature, reading `source.stream` under `SIGNAL_KIND === "sse"` and `source.once`-through-`?wait=25` under `"held"`, with `SIGNAL_KIND` read ONCE from `/api/health` by `whoami.ts`'s door (it already fetches health? — if not, `api.health()` at boot in `App`, stored in a module-level `let signalKind`), `api.ts` exports `identityHeaders(): HeadersInit` (the same header `call` attaches).

- [ ] **Step 1: `sse.ts` with its cells**

```ts
import { ApiError, identityHeaders } from "./api";

export interface Frame { event: string; data: string }

/** A stateful parser over the text/event-stream grammar we emit: `event:` and
 *  `data:` lines, a blank line ends a frame, a line starting with `:` is a
 *  comment (the heartbeat) and is dropped. Multi-line `data:` is joined with
 *  `\n` per the spec, though this server writes one line. */
export function parseSse(): (chunk: string) => Frame[] {
  let buffer = "";
  let event = "message";
  let data: string[] = [];
  return (chunk: string) => {
    buffer += chunk;
    const out: Frame[] = [];
    let nl: number;
    while ((nl = buffer.indexOf("\n")) >= 0) {
      const line = buffer.slice(0, nl).replace(/\r$/, "");
      buffer = buffer.slice(nl + 1);
      if (line === "") {
        if (data.length) out.push({ event, data: data.join("\n") });
        event = "message";
        data = [];
      } else if (line.startsWith(":")) {
        /* heartbeat */
      } else if (line.startsWith("event:")) {
        event = line.slice(6).trim();
      } else if (line.startsWith("data:")) {
        data.push(line.slice(5).replace(/^ /, ""));
      }
    }
    return out;
  };
}

export async function readStream(
  url: string,
  signal: AbortSignal,
  onFrame: (f: Frame) => void,
): Promise<"ended" | "closed"> {
  const res = await fetch(url, {
    headers: { ...identityHeaders(), accept: "text/event-stream" },
    signal,
  });
  if (!res.ok || !res.body) {
    let message = `${res.status}`;
    try { message = ((await res.json()) as { error?: string }).error ?? message; } catch { /* not JSON */ }
    throw new ApiError(message, res.status);
  }
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  const parse = parseSse();
  for (;;) {
    const { value, done } = await reader.read();
    if (done) return "closed";
    for (const f of parse(decoder.decode(value, { stream: true }))) {
      onFrame(f);
      if (f.event === "end") return "ended";
    }
  }
}
```

`sse.test.ts`: `parseSse` over a chunk split mid-line and mid-frame yields the frames whole; a heartbeat line yields nothing; `readStream` over a fake `fetch` whose body is a `ReadableStream` scripted with two frames and an `end` resolves `"ended"` after two `onFrame` calls; a 403 body `{"error":"…"}` rejects with an `ApiError` carrying the sentence and status; the request carried the identity header and `accept: text/event-stream` (assert on the stub's captured `init`).

- [ ] **Step 2: `usePolled` re-plumbed**

Inside the effect, replace the `ask`/`schedule(VISIBLE_MS)` cadence with:

```ts
    let controller: AbortController | null = null;
    const open = async () => {
      if (stopped || !visible() || !src.current) return;
      controller = new AbortController();
      try {
        if (signalKind() === "held") {
          // R-plan-5: the same fetch, one frame per request, reopened at once.
          const v = await src.current.once();     // `once` for a game is `api.events(game, undefined, 25)` — see api.ts
          if (stopped) return;
          failures.current = 0; setLink("live"); latest.current(v);
          schedule(0);
          return;
        }
        setLink("live");
        const outcome = await readStream(src.current.stream, controller.signal, (f) => {
          if (stopped) return;
          if (f.event === "signal" || f.event === "house") {
            failures.current = 0;
            latest.current(JSON.parse(f.data) as T);
          } else if (f.event === "end") {
            const { reason } = JSON.parse(f.data) as { reason: "deleted" | "gone" };
            ended.current?.({ reason });
          }
        });
        if (stopped || outcome === "ended") return;
        throw new Error("the stream closed");          // fall to the ladder
      } catch (e) {
        if (stopped || (e instanceof DOMException && e.name === "AbortError")) return;
        if (e instanceof ApiError && (e.status === 404 || e.status === 403)) {
          ended.current?.({ reason: e.status === 404 ? "gone" : "refused", sentence: e.message });
          return;
        }
        setLink("reconnecting");
        const wait = nextDelay(failures.current);
        failures.current += 1;
        schedule(wait);
      }
    };
    const schedule = (ms: number) => { if (timer) clearTimeout(timer); timer = setTimeout(() => void open(), ms); };
    const onVisibility = () => {
      if (visible()) { setLink("refreshing"); schedule(0); }
      else { controller?.abort(); controller = null; if (timer) { clearTimeout(timer); timer = null; } }
    };
    document.addEventListener("visibilitychange", onVisibility);
    if (visible()) schedule(0);
    return () => { stopped = true; controller?.abort(); if (timer) clearTimeout(timer); document.removeEventListener("visibilitychange", onVisibility); };
```

(`src` is the `useRef` holding `source`; `VISIBLE_MS` is deleted with the poll; `BACKOFF_MS`/`nextDelay` stay.) `signalKind()` reads a module-level value set from `/api/health` at boot: `api.ts` gains `let kind: "sse" | "held" = "sse"; export const signalKind = () => kind; export const setSignalKind = (k) => { kind = k }`, and `App` calls `setSignalKind(health.signal)` where it already fetches `labels`/`art` at boot (add `api.health()` there). The house `Source` for the lobby: `{ key: "lobby", once: fetchLists, stream: "/api/stream" }` — under SSE the `house` frames carry `{seq}`, not the lists, so `Lobby`'s `onValue` must refetch: change the lobby's hook use to `usePolled<{seq:number}>(…, () => void fetchLists().then(setLists))` with an initial `fetchLists()` on mount; under `held` the once-path returns the lists directly — make `once` for the lobby `api.list`-based and let `onValue` accept either by refetching in both arms (simplest: the lobby's `onValue` ALWAYS refetches, ignoring the value; one extra list read under `held`).

`signal.test.ts`: rewrite the poll cells against a fake `fetch` that returns a scripted `ReadableStream` (from `sse.test.ts`'s helper): (1) connected → two frames → two `onValue` calls, and **no timer is armed while connected** (a `vi.spyOn(globalThis, "setTimeout")` that throws while `link === "live"`); (2) hidden aborts the reader (the stub's `signal.aborted` is true), visible again opens a new one and `link` read `"refreshing"` in between; (3) the body closing without `end` → `"reconnecting"`, the ladder's first delay, a new open; (4) an `end` frame → `onEnd({reason:"deleted"})` and no reopen; (5) a 404 open → `onEnd({reason:"gone", sentence})`; (6) the stream request carried the identity header; (7) under `setSignalKind("held")`, `once` is called, then called again at once after it resolves, and never `fetch` on `/stream`.

- [ ] **Step 3: The consumers**

`App.tsx`'s in-game `useSignal` is unchanged in shape (the hook changed under it); `Dealing.tsx`'s `useSignal(game, true, …)` likewise; `OpenTable.tsx`'s `usePolled` likewise; `Lobby.tsx` per Step 2. `api.events` gains an optional `wait` argument (`?wait=25`) for the held arm. The e2e turns green again here: run it.

Run: `cd web && npm run typecheck && npm test 2>&1 | grep -E 'Tests ' && npm run e2e 2>&1 | tail -3`
Expected: all green; the two-browser cell's wall drops (record it in the ledger against the M6.1 close's 11.8 min).

```bash
git add web/src/sse.ts web/src/sse.test.ts web/src/signal.ts web/src/signal.test.ts web/src/api.ts web/src/App.tsx web/src/Dealing.tsx web/src/OpenTable.tsx web/src/Lobby.tsx
git commit -m "feat(web client): the poll leaves — usePolled reads …/{id}/stream through fetch with the identity header (EventSource cannot carry it), one connection per visible tab, the ladder on failure, an end frame on delete, the held-request arm behind health's signal; the lobby listens to /api/stream (spec §5.1, R-3.2)"
```

---

### Task 8: The spectator screen, the walk's arrival, Games in progress, the unlock face (spec §5.2, §5.3, R-3.9)

**Files:**
- Create: `web/src/Unlock.tsx`, `web/src/Unlock.test.tsx`
- Modify: `web/src/App.tsx` (+ `.test.tsx`), `web/src/Decision.tsx` (+ `.test.tsx`), `web/src/walk.ts` (+ `.test.ts`), `web/src/Lobby.tsx` (+ `.test.tsx`), `web/src/styles.css`

**Interfaces:**
- Produces: `walk.ts` — `hasOpponentStep(steps: Step[], viewer: PlayerId | null): boolean` (`null` → every step counts); `App.tsx` — `type Arrival = { via: "reply" } | { via: "signal"; viewer: PlayerId | null }`, `landed(game, prev, next, arrival: Arrival = { via: "reply" })`; `Unlock` props `{ game: string; table: TableView; onUnlocked: () => void; onLeave: () => void; who?: ReactNode }`.

- [ ] **Step 1: The walk's arrival**

`walk.ts`: `viewer: PlayerId | null` and `steps.some((s) => s.moverIsBot || viewer === null || s.mover !== viewer)`; a cell that `hasOpponentStep(steps, null)` is true for a run of purely-human steps by one mover. `App.tsx`: `landed`'s fourth parameter becomes `arrival: Arrival = { via: "reply" }` and the gate line `const plays = arrival.via === "reply" ? hasBotStep(r) : hasOpponentStep(r.steps, arrival.viewer);`; the in-game signal consumer calls `landed(game, state.events, fresh.events, { via: "signal", viewer: fresh.seat })` (was `fresh.seat` bare); every other call site passes nothing. Cell (`App.test.tsx`): render `wire.spectator` through `renderApp`, script a signal frame with `events + 1` and a `/at/{k}` reply that is a HUMAN mover's step, and assert the walk starts (`.turn-account .lit` or whatever the account's lit-row class is — read `TurnAccount.tsx` for it) — the mutation-10 cell.

- [ ] **Step 2: The spectator screen**

`App.tsx`: `state.seat === null` selects: the "You are" strip reads "You are watching" (the `whoStrip` node gains the branch); `<Decision …/>` is rendered with a new prop `spectating={state.seat === null}`. `Decision.tsx`: when `spectating`, render ONLY the `watching` line (`:132`'s block) and none of the answer bar, Undo, or the Suggest entry; a cell over `wire.spectator` asserts no `button` named Undo, no Suggest, and the line `Seat 0 is deciding.` (whichever seat the fixture's `to_move` is). The `:397` template stays as it is (unreachable for a `watching` prompt; §5.2). Every `// M6.3 Task 8` marker from Task 6 is replaced by the spectator behaviour (a null seat draws no own-hand strip, no "your turn" affordance).

- [ ] **Step 3: Arriving as a watcher — `Unlock.tsx` and the route**

`App.tsx`'s `open(id)`: after `api.table(id)`, if `doc.status === "playing"` and `doc.my_seats.length === 0` and `!doc.can_watch` → `show(null); setUnlock({ id, table: doc }); return;`; otherwise fall through to `api.get(id)` (a spectator's frame comes back with `seat: null`). `Unlock.tsx`:

```tsx
export function Unlock({ game, table, onUnlocked, onLeave, who }: {…}) {
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const submit = async () => {
    setBusy(true); setError(null);
    try { await api.unlock(game, password.trim()); onUnlocked(); }
    catch (e) { setError(e instanceof ApiError ? e.message : String(e)); }
    finally { setBusy(false); }
  };
  return (
    <div className="setup unlock">
      {who && <div className="who-strip">{who}</div>}
      <h2>{table.name ?? table.id}</h2>
      <p className="lede">{table.players}P &middot; {tableWord(table)} &middot; {seatNames(table)} &middot; password required to watch</p>
      <div className="field"><label htmlFor="watch-password">Password</label>
        <input id="watch-password" type="password" value={password} onChange={(e) => setPassword(e.target.value)} /></div>
      <button type="button" className="primary" disabled={busy || !password.trim()} onClick={() => void submit()}>Watch</button>
      <button type="button" className="ghost" onClick={onLeave}>Lobby</button>
      {error && <div className="error">{error}</div>}
    </div>
  );
}
```

`onUnlocked` re-runs `open(id)`. `Unlock.test.tsx` over `wire.tableWatchable`: the face renders the name and seats; a wrong password (`wire.unlock403`) shows the sentence; a 200 calls `onUnlocked`.

- [ ] **Step 4: Games in progress**

`Lobby.tsx`: a third section over `api.list("playing")` (fetched with the other two in `fetchLists`), rows through the same `row`, lock marker when `locked`; a cell over `wire.gamesPlaying`.

- [ ] **Step 5: Run and commit**

Run: `cd web && npm run typecheck && npm test 2>&1 | grep -E 'Tests ' && npm run e2e 2>&1 | tail -3`

```bash
git add web/src/App.tsx web/src/App.test.tsx web/src/Decision.tsx web/src/Decision.test.tsx web/src/walk.ts web/src/walk.test.ts web/src/Lobby.tsx web/src/Lobby.test.tsx web/src/Unlock.tsx web/src/Unlock.test.tsx web/src/styles.css
git commit -m "feat(web client): the spectator screen — no answer bar, 'You are watching', every served run walks because the walk's arrival now carries the viewer separately from 'via the signal'; the unlock face for a locked table; Games in progress in the lobby (spec §5.2, §5.3, R-3.9)"
```

---

### Task 9: The gates — the third identity that watches; the watching browser and the no-`/events` read (spec §9.4)

**Files:**
- Modify: `crates/yokohama-web/tests/play_a_whole_game.rs`, `web/e2e/two_browsers.spec.ts`

- [ ] **Step 1: The gate's third identity**

In `two_people_play_to_a_winner_and_each_sees_only_their_own_hand`, after both keeps: `green` reads `GET /api/games/two` → 403 (locked); `POST /unlock {"password":"sesame"}` → 200; then inside the move loop, after EVERY applied move, green reads `GET /api/games/two` and `GET /at/{k}` for the current `k`, and asserts: `seat` is null; every `Players` row `hand_hidden: true`; no `Hand` panel; and `frame.board`, `frame.title` and the dashboard MINUS `Hand` panels are `==` to red's frame at the same `k` (fetch red's `at/{k}` and strip its `Hand` panel before comparing; `decision` excluded). Keep the per-reply assertions for red and blue.

- [ ] **Step 2: The watching browser**

`two_browsers.spec.ts`: a third context `c` signs in as `c@e2e.test` and goes to `/#/t/${id}` after the table is dealt (the M6.2 re-point created it WITH the password `sesame`; if the cell's create does not, add the password box); `c` sees the unlock face, types `sesame`, clicks Watch, and sees the board with "You are watching". Start a request log on `c`: `const events: string[] = []; c.on("request", (r) => { const p = new URL(r.url()).pathname; if (p.endsWith("/events")) events.push(p); });` and `let streams = 0; c.on("request", (r) => { if (new URL(r.url()).pathname.endsWith("/stream")) streams += 1; })`. Then inside the loop, after each `step`: `await expect(c.locator(".turn-account .lit")).toHaveAttribute("data-event", String(expectedIndex), { timeout: 15_000 })` (read `TurnAccount.tsx` for the row's real attribute; add `data-event` on the row if it has none — a product attribute like `data-busy`, recorded in the close); `expect(await ownHand(c)).toEqual([])`; and every 20 moves, assert `events.length === 0` and `streams === 1` while `await c.evaluate(() => document.visibilityState)` is `"visible"`. At the end record `Date.now() - started` in the test's output line beside the moves count.

Run: `cd web && npm run e2e 2>&1 | tail -4`
Expected: `39 passed`; the two-browser wall printed (compare with 11.8 min in the ledger).

```bash
git add crates/yokohama-web/tests/play_a_whole_game.rs web/e2e/two_browsers.spec.ts web/src/TurnAccount.tsx
git commit -m "test(web): a third identity watches the two-identity game at every position and sees the players' board and no hand; a third browser unlocks, watches the two-browser game walk, and issues no request to /events on one open stream (spec §9.4)"
```

---

### Task 10: Release `yokohama-web:0.5.0-m6.3` and repeat the measurement (spec §10)

- [ ] **Step 1: Owner authorization.** Build and deliver per the runbook; the container `Env` unchanged.
- [ ] **Step 2: Repeat reading A against the deployed stream** (the `curl -N` of Task 3 for ten minutes with a move at minute nine) and record it beside the rc reading.
- [ ] **Step 3: Verify from the shell**: the on-connect frame and a heartbeat within 25 s; `/api/health` reads `"signal":"sse"` (or `"held"`).
- [ ] **Step 4: Record §3.9** in the runbook (both halves: the rc readings and the release) and add the heartbeat sentence to `deploy/cloudflared-config.yml`'s Yokohama ingress comment.

```bash
git add deploy/RUNBOOK.md deploy/cloudflared-config.yml
git commit -m "docs(deploy): the sixth deployment — yokohama-web:0.5.0-m6.3 (transport and spectating), the tunnel measurement before and after, the heartbeat sentence on the ingress"
```

---

### Task 11: The final review against the live site, and the close

- [ ] **Step 1: The mutation battery** (spec §9.5, twelve mutations), one at a time on the local build, first failing assertion verbatim, reverted.
- [ ] **Step 2: The live review**: a CLEAN Fable subagent from THREE browsers as three people — two playing, one arriving by link, unlocking, watching a full turn with the network panel open (zero `/events`, one `/stream`), in both layouts.
- [ ] **Step 3: The counts in one sequence**, the blast-radius diff (five crates empty; the CLI's diff exactly `repl.rs`), `SEMANTICS_REV` 7.
- [ ] **Step 4: The close** in the M6.2 close's shape (§1–§8), with the R-3.1 readings in §2 and the two-browser cell's new wall beside the old; then the doc pass (`CLAUDE.md`, one `guidingpath.md` row, `README.md`, `docs/handover.md` Track 3 pointing at M6.4).

```bash
git add docs/plans/<date>-web-frontend-m6.3-close.md CLAUDE.md guidingpath.md README.md docs/handover.md
git commit -m "docs(web): M6.3 close — no polling on the NAS, a spectator link shows a game walking; the reviews' findings; the handover into M6.4"
```

---

## Deviations from the spec (recorded per R-P11; the close carries the costs)

- R-plan-1 through R-plan-5 above. The close lists any taken in execution beside them.

## Self-review (run before the plan is handed over)

- **Spec coverage.** §2 → Task 1; §3.1–§3.3, R-3.12, R-3.14 → Task 2; R-3.1 → Tasks 3 and 10; §3.4, §3.5, §4, R-3.5–R-3.7, R-3.9 (server), R-3.13 → Task 4; R-3.8 → Task 5; §8, §5.4 → Task 6; §5.1, R-3.2 → Task 7; §5.2, §5.3, R-3.9 (client) → Task 8; §6 → Tasks 1, 4, 6, 9; §7 (the notifier) → the spec alone, by R-3.10; §9.1–§9.3 → Tasks 1–8's cells; §9.4 → Task 9; §9.5 → Task 11; §9.6 → every task's gate; §10 → Task 10; §11, §12 → the close.
- **Placeholders.** The `…` ellipses stand for existing code or for a fixture the M6.1 cells already build, and each says which; the one open reading (`TurnAccount`'s lit-row attribute) names the file to read and the fallback (add `data-event`, record it).
- **Type consistency.** `Signals::{bump_table, bump, end, subscribe, subscribe_house}` in Task 1 are what Tasks 2 and 4 call; `events_at(state, id, user)` in Task 2 is what `stream` and `events` share and what Task 4 re-points onto `admitted`; `admitted(state, id, user) -> (TableDoc, Viewer)` is what every route destructures; `ClientState::of(id, repl, viewer: Viewer, messages)` and `HistoricalFrame::at(repl, k, viewer: Viewer)` are what the handlers and the wire test call; `index_placements_for(Option<PlayerId>)` in Task 5 is what `card_index` calls in Task 4's re-point; `readStream(url, signal, onFrame)` in Task 7 is what `usePolled` calls; `Arrival` in Task 8 is what the in-game consumer passes to `landed`.
