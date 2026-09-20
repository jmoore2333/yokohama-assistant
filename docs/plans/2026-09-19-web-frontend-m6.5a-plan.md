# M6.5a "the seam" — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cut one table's logic out of `crates/yokohama-web` into a transport-neutral layer that builds for `wasm32-unknown-unknown` behind a `server` cargo feature, with axum reduced to an adapter over ONE route table, the archive collecting saves on the NAS, and every existing wire payload byte-identical.

**Architecture:** `Games` becomes the layer's state (it absorbs the chat switch and gains a clock, a signal trait object and an archive trait object); a new `api.rs` holds the request/response types, the parsers that reproduce axum's rejection sentences, the path matcher, the per-route functions moved out of `lib.rs`, and `dispatch`; `lib.rs` keeps `AppState`, the three transport routes, the static routes and ONE catch-all adapter under `#[cfg(feature = "server")]`. A new `archive.rs` holds the `Archive` trait, the directory implementation and the shared layout function. The record route and "Save game file" ride on top.

**Tech Stack:** Rust 1.97.1 (pinned), axum 0.8.9 / tokio 1.53 / tokio-stream (now optional), `http` 1.5, `serde_urlencoded` 0.7.1, `getrandom` 0.2 (`js`, wasm-only), React + TypeScript + vitest + Playwright in `web/`.

**Spec:** `docs/plans/2026-09-19-web-frontend-m6.5a-seam-spec.md` (rulings R-6.5a.1–16), under the umbrella `docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` (R-6.5.1–13, §2.1, §6.1). The plan argues from the spec; executors read both. Line citations are at `4dc25a1`, whose sources are byte-identical to `faf1a88` — **re-read every cited range at your own HEAD before editing**, because lines move with every task.

## Global Constraints

- The five fenced crates (`yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge`, `gen-data`) and `crates/yokohama-cli` are NOT touched: `git diff faf1a88 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli` is empty at every commit. `SEMANTICS_REV` stays 7.
- Every existing committed wire payload stays byte-identical except `web/src/wire/health.json`, which gains exactly one key, `archive`. Regenerate at every task that changes a shape (`UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`), then run WITHOUT the variable, read the diff, and put the diff's summary in the commit message (R-3-1).
- Per-edit gates after EVERY task: `cargo test -p yokohama-web` (all three binaries), `cargo clippy --workspace --all-targets -- -D warnings`, and from Task 4 on `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`. Tasks that touch `web/`: `cd web && npm run typecheck && npm test`.
- `Games`' public method signatures do not change (R-6.5a.1). The 145 module cells and the 74 route cells stay green with only the cells this plan names added or moved.
- Stage with explicit paths (`git add <path>`), never `git add -A`. Commit messages end with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.
- Rulings R-6.5.1–13 and R-6.5a.1–16 are settled inputs; a task that finds one unworkable STOPS and reports rather than deviating.
- The layer's refusal sentences are byte-equal to today's (R-6.5a.4): axum's `body_text()` for bad bodies, queries and path parameters; `resolve`'s per-mode 401 sentence; every `GameError` sentence unchanged.
- No `SystemTime`, `Instant`, `std::fs`, `std::env`, `std::thread`, `std::process`, `tokio::` or `axum::` outside `server`-gated code from Task 4 on (the scanning cell holds it).
- `wasm32-unknown-unknown` is installed on the pinned toolchain: `rustup target add wasm32-unknown-unknown --toolchain 1.97.1` (already done on the Mac that wrote this plan).

---

## File structure

| file | responsibility after this plan |
|---|---|
| `crates/yokohama-web/Cargo.toml` | the `server` feature; optional axum/tokio/tokio-stream; `http`, `serde_urlencoded`; the wasm-only `getrandom` line |
| `crates/yokohama-web/src/signal.rs` | the unconditional `Signal` trait and `NoSignal`; `Signals`/`Bump`/`House` under `server` |
| `crates/yokohama-web/src/archive.rs` (new) | `Archive`, `Why`, `paths`, `strip_for_archive`, `NoArchive`; `DirArchive` under `server` |
| `crates/yokohama-web/src/api.rs` (new) | `ApiRequest`, `ApiResponse`, `Host`, `Identity`, the parsers, the matcher, `dispatch`, every per-route function and helper moved out of `lib.rs`, the DTOs (`ListQuery`, `GamesList`, `Keep`, `Join`, `Unlock`, `ChatPost`, `ChatQuery`, `ChatPage`, `Input`, `LogQuery`, `IndexQuery`, `WhyQuery`, `Events`, `Me`, `Labels`, `Health`) |
| `crates/yokohama-web/src/games.rs` | `Games` with `chat`, `clock`, `signal`, `archive`; the game-over archive site in `persist` |
| `crates/yokohama-web/src/tables.rs` | `now_unix` callers on the clock; the abandon archive site in `delete`; `archived_at` in `check_shape`; `TABLE_SCHEMA` 3 |
| `crates/yokohama-web/src/table.rs` | `TableDoc.archived_at`; `now_unix` under `server` |
| `crates/yokohama-web/src/lib.rs` | `pub mod` list; `ApiError` on `http::StatusCode`; under `server`: `AppState`, the adapter, `events`/`stream`/`house_stream`, `art_manifest`, `fallback`, `app()`, `IntoResponse for ApiError`, the constants |
| `crates/yokohama-web/src/identity.rs`, `store.rs`, `chat.rs`, `art.rs`, `assets.rs`, `main.rs` | gated as the spec's §2.1 table says |
| `crates/yokohama-web/tests/wire_fixtures.rs` | `record_409`; the seed pin's second partner; the dispatcher-equals-router cell |
| `web/src/api.ts`, `App.tsx`, `Abandon.tsx`, `Help.tsx`, `types.ts`, `wire/contract.ts`, `web/src/*.test.tsx` | `recordText`, "Save game file", the Abandon label, `Health.archive`, the help sentence, their cells |
| `.github/workflows/test.yml`, `deploy/Dockerfile.runtime`, `deploy/RUNBOOK.md` | the check line; the archive variable documented |

---

### Task 1: The `Signal` trait (R-6.5a.8)

**Files:**
- Modify: `crates/yokohama-web/src/signal.rs` (whole non-test half, `:1-112`)
- Modify: `crates/yokohama-web/src/games.rs:9-12` (imports), `:107-187` (the struct), `:206-247` (constructors and accessors)
- Modify: `crates/yokohama-web/src/lib.rs:255-322` (`AppState` gains `signals: Arc<Signals>`; constructors pass it into `Games`)
- Modify: every `state.games.signals()` in `lib.rs` (`:1800`, `:1937`, `:2000` and the test module's uses — `grep -n "signals()" crates/yokohama-web/src/lib.rs`) → `state.signals`
- Test: `crates/yokohama-web/src/signal.rs` (test module), `crates/yokohama-web/src/games.rs` (test module)

**Interfaces:**
- Produces: `pub trait Signal: Send + Sync { fn bump(&self, id: &str); fn bump_table(&self, id: &str); fn bump_house_chat(&self); fn end(&self, id: &str); }`; `pub struct NoSignal;`; `impl Signal for Signals`; `Games::with_parts(rules, store, limits, signal: Arc<dyn Signal>) -> Games`; `AppState.signals: Arc<Signals>`.
- Consumes: nothing new.

- [ ] **Step 1: Write the failing cells**

In `signal.rs`'s test module add:

```rust
#[test]
fn no_signal_accepts_every_call_and_keeps_nothing() {
    let s = NoSignal;
    s.bump("a");
    s.bump_table("a");
    s.bump_house_chat();
    s.end("a");
    // The trait object shape the layer holds.
    let boxed: std::sync::Arc<dyn Signal> = std::sync::Arc::new(NoSignal);
    boxed.bump("b");
}

#[test]
fn signals_through_the_trait_are_the_same_bumps() {
    let s = std::sync::Arc::new(Signals::default());
    let rx = s.subscribe("t");
    let dynamic: std::sync::Arc<dyn Signal> = s.clone();
    dynamic.bump("t");
    assert_eq!(rx.borrow().seq, 1, "a bump through the trait reaches the same channel");
    dynamic.end("t");
    assert!(rx.borrow().end);
}
```

In `games.rs`'s test module add:

```rust
#[test]
fn a_games_built_with_no_signal_still_answers_every_document_call() {
    let rules = Arc::new(Rules::standard());
    let store: Arc<dyn Store> = Arc::new(crate::store::MemStore::default());
    let games = Games::with_parts(rules, store, ChatLimits::default(), Arc::new(crate::signal::NoSignal));
    // Any document call that reaches a bump site is enough: `chat_last` on a
    // missing table reads zero and nothing panics on the way.
    assert_eq!(games.chat_last("nothing"), 0);
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p yokohama-web --lib no_signal signals_through with_no_signal`
Expected: FAIL to compile — `Signal`, `NoSignal`, `with_parts` do not exist.

- [ ] **Step 3: Implement the trait**

Replace the top of `signal.rs` (keep the test module) with:

```rust
//! The change signal, as a trait (M6.5a, R-6.5a.8): the layer's eight bump sites
//! call four methods over `&str`; `Signals` (tokio watch channels) implements
//! them under the `server` feature and `NoSignal` is the unconditional no-op.
//! Subscribing is a transport concern and stays on `Signals`.

pub trait Signal: Send + Sync {
    fn bump(&self, id: &str);
    fn bump_table(&self, id: &str);
    fn bump_house_chat(&self);
    fn end(&self, id: &str);
}

/// The signal of a host with nothing to wake.
pub struct NoSignal;

impl Signal for NoSignal {
    fn bump(&self, _id: &str) {}
    fn bump_table(&self, _id: &str) {}
    fn bump_house_chat(&self) {}
    fn end(&self, _id: &str) {}
}
```

Keep everything from `use std::collections::HashMap;` down unchanged for now (Task 4 gates it), rename the four inherent methods' bodies into an `impl Signal for Signals` block so `Signals` keeps `subscribe`, `subscribe_house`, `senders` and the private senders as inherent methods:

```rust
impl Signal for Signals {
    fn bump_table(&self, id: &str) {
        let tx = self.table_sender(id);
        tx.send_modify(|b| b.seq += 1);
    }
    fn bump(&self, id: &str) {
        self.bump_table(id);
        self.house_sender().send_modify(|h| h.seq += 1);
    }
    fn bump_house_chat(&self) {
        self.house_sender().send_modify(|h| h.chat += 1);
    }
    fn end(&self, id: &str) {
        let tx = self.tables.lock().expect("signals lock").remove(id);
        if let Some(tx) = tx {
            tx.send_modify(|b| {
                b.seq += 1;
                b.end = true;
            });
        }
        self.house_sender().send_modify(|h| h.seq += 1);
    }
}
```

In `games.rs`: change the field to `signal: Arc<dyn Signal>` (import `crate::signal::{NoSignal, Signal}`), add

```rust
pub fn with_parts(
    rules: Arc<Rules>,
    store: Arc<dyn Store>,
    limits: ChatLimits,
    signal: Arc<dyn Signal>,
) -> Self {
    Self { rules, store, live: Mutex::new(HashMap::new()), written: Mutex::new(HashMap::new()),
           table_lock: Mutex::new(()), locks: Mutex::new(HashMap::new()),
           deleted: Mutex::new(std::collections::HashSet::new()), signal,
           chat_last: Mutex::new(HashMap::new()), house_lock: Mutex::new(()), limits }
}
```

make `with_limits` call `with_parts(rules, store, limits, Arc::new(NoSignal))`, delete `pub fn signals(&self)`, and rename the eight call sites `self.signals.` → `self.signal.` (`games.rs:543`; `tables.rs:769, :951, :965, :1255, :1402, :1504, :1693`).

In `lib.rs`: `AppState` gains `pub signals: Arc<signal::Signals>`; `with_store_and_chat` builds `let signals = Arc::new(signal::Signals::default());` and `Games::with_parts(rules, store, chat.limits, signals.clone())`; every `state.games.signals()` becomes `state.signals` (including the test module's).

- [ ] **Step 4: Run the whole crate**

Run: `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 219 + 3 lib cells pass (the four gated bump-ordering cells still subscribe through `state.signals`), 2 gate, 9 wire; clippy clean.

- [ ] **Step 5: Commit**

```bash
git add crates/yokohama-web/src/signal.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/src/lib.rs
git commit -m "feat(web): the change signal is a trait — Signals under the server, NoSignal for the rest (R-6.5a.8)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: The clock seam (R-6.5a.6, first half)

**Files:**
- Modify: `crates/yokohama-web/src/games.rs` (struct: `clock: Clock`; `with_parts` takes it; `pub type Clock = Arc<dyn Fn() -> u64 + Send + Sync>`)
- Modify: `crates/yokohama-web/src/tables.rs:746` (`created_at: (self.clock)()`), `:913` (`created_at: (self.clock)()`); remove the `now_unix` import at `:42-44`
- Modify: `crates/yokohama-web/src/lib.rs` constructors (pass `Arc::new(crate::table::now_unix)`)
- Test: `crates/yokohama-web/src/tables.rs` test module

**Interfaces:**
- Produces: `pub type Clock`; `Games::with_parts(rules, store, limits, signal, clock: Clock)`; `Games::with_clock(self, clock: Clock) -> Self` (a builder step for tests).
- Consumes: Task 1's `with_parts`.

- [ ] **Step 1: Write the failing cell**

In `tables.rs`'s test module, beside the existing create cells (use the module's own helpers for a `User` and a `NewGame` body — `test_support` in `games.rs:1452`):

```rust
#[test]
fn a_table_is_stamped_by_the_clock_the_games_was_built_with() {
    let games = test_support::games_in_memory().with_clock(Arc::new(|| 1_700_000_000));
    let creator = User::parse("clock@wire.test").unwrap();
    let view = games.create(&test_support::two_player_request(&creator), &creator).unwrap();
    let (doc, _) = games.read_table(&view.id).unwrap().unwrap();
    assert_eq!(doc.created_at, 1_700_000_000, "created_at is the injected clock's reading");
}
```

(If `test_support` lacks a two-player request helper, use the body the nearest existing create cell builds; do not add a second helper.)

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p yokohama-web --lib stamped_by_the_clock`
Expected: FAIL to compile — `with_clock` does not exist.

- [ ] **Step 3: Implement**

In `games.rs`:

```rust
pub type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;
```

field `clock: Clock`; `with_parts(..., signal, clock: Clock)`; `with_limits` passes `Arc::new(crate::table::now_unix)` (this still compiles unconditionally until Task 4 gates `now_unix`; Task 4 moves the default into `AppState`); and

```rust
pub fn with_clock(mut self, clock: Clock) -> Self {
    self.clock = clock;
    self
}
```

In `tables.rs:746` and `:913` replace `now_unix()` with `(self.clock)()`.

- [ ] **Step 4: Run the crate**

Run: `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings`
Expected: green; `wire_fixtures` unchanged because `freeze_clock` normalises `created_at` (`tests/wire_fixtures.rs:247-266`).

- [ ] **Step 5: Commit**

```bash
git add crates/yokohama-web/src/games.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/src/lib.rs
git commit -m "feat(web): the clock is a seam on Games — two callers, one injected reading (R-6.5a.6)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: `Games` absorbs the chat switch; the helpers take `&Games` (R-6.5a.1)

**Files:**
- Modify: `crates/yokohama-web/src/games.rs` (field `chat: ChatConfig` replaces `limits: ChatLimits`; `limits()` reads `self.chat.limits`; `with_parts` takes `chat: ChatConfig`)
- Modify: `crates/yokohama-web/src/lib.rs`: `admitted` (`:506`), `lobby_chat_on` (`:981`), `events_at` (`:1856`) take `&Games`; every `state.chat.limits` → `state.games.chat.limits`; `AppState.chat` removed; `health` reads `state.games.chat`
- Test: none new — the existing cells are the assertion

**Interfaces:**
- Produces: `Games.chat: ChatConfig` (pub); `fn admitted(games: &Games, id: &str, user: &User) -> Result<(TableDoc, Viewer), ApiError>`; `fn events_at(games: &Games, id: &str, user: &User) -> Result<Events, ApiError>`; `fn lobby_chat_on(games: &Games) -> Result<(), ApiError>`.

- [ ] **Step 1: Make the mechanical change**

`Games::with_parts(rules, store, chat: ChatConfig, signal, clock)`; `with_limits(rules, store, limits)` builds `ChatConfig { lobby: true, limits }` (its callers are tests). `AppState::with_store_and_chat` passes `chat` into `Games` and no longer stores it. Every `&state` argument to the three helpers becomes `&state.games`; inside them `state.games.` → `games.` and `state.chat.` → `games.chat.`.

- [ ] **Step 2: Run the crate**

Run: `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings`
Expected: green, no cell moved. If a `lib.rs` cell reads `state.chat`, change it to `state.games.chat` — list each in the commit message.

- [ ] **Step 3: Commit**

```bash
git add crates/yokohama-web/src/games.rs crates/yokohama-web/src/lib.rs
git commit -m "refactor(web): Games carries the chat config; the three route helpers take &Games (R-6.5a.1)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 4: `ApiError` on `http`, the `server` feature, the gating, the check line and the scanning fence (R-6.5a.2, R-6.5a.5, R-6.5a.6 second half, R-6.5a.7)

**Files:**
- Modify: `crates/yokohama-web/Cargo.toml`
- Modify: `crates/yokohama-web/src/lib.rs` (imports; `ApiError.status: http::StatusCode`; `#[cfg(feature = "server")]` on every axum item; `AppState` passes the default clock)
- Modify: `crates/yokohama-web/src/identity.rs` (`from_env`, the extractor and the axum imports under `server`; `resolve` takes `&http::HeaderMap`)
- Modify: `crates/yokohama-web/src/store.rs` (`FsStore`, `default_dir` under `server`)
- Modify: `crates/yokohama-web/src/chat.rs` (`ChatConfig::from_env` under `server`)
- Modify: `crates/yokohama-web/src/signal.rs` (everything but the trait and `NoSignal` under `server`)
- Modify: `crates/yokohama-web/src/table.rs:417-422` (`now_unix` under `server`)
- Modify: `crates/yokohama-web/src/art.rs`, `assets.rs` (`#![cfg(feature = "server")]` at the top), `lib.rs`'s `pub mod art; pub mod assets;` gated
- Modify: `crates/yokohama-web/src/games.rs` (`with_limits` no longer references `now_unix`: it takes the clock from the caller — tests pass `Arc::new(|| 0)` through `test_support`)
- Modify: `.github/workflows/test.yml:41-46`
- Test: `crates/yokohama-web/src/lib.rs` test module (the scanning cell)

**Interfaces:**
- Produces: feature `server` (default); `ApiError { status: http::StatusCode, .. }`; `identity::resolve(mode, &http::HeaderMap)`.
- Consumes: Tasks 1–3.

- [ ] **Step 1: Write the failing scanning cell**

In `lib.rs`'s test module (it runs under the default feature; that is fine — it reads source, not features):

```rust
/// R-6.5a.6's fence. The wasm check line cannot see a `SystemTime` (it compiles
/// for the target and panics there), so the crate's own source is read at run
/// time, cut at each file's first top-level `#[cfg(test)]`, and any platform
/// call outside the `server`-gated files or a `#[cfg(feature = "server")]`
/// block is named. Two anti-vacuity partners: the gated files must exist, and
/// at least one gated site must match, so a pattern that matches nothing is
/// caught as loudly as a call that escaped.
#[test]
fn platform_calls_stay_behind_the_server_feature() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let gated_files = ["art.rs", "assets.rs", "main.rs"];
    let patterns = ["SystemTime", "Instant::now", "std::fs", "std::env", "std::thread",
                    "std::process", "tokio::", "axum::"];
    let mut escaped: Vec<String> = Vec::new();
    let mut gated_hits = 0usize;
    for name in gated_files {
        assert!(src.join(name).exists(), "gated file {name} is missing — update the list");
    }
    for entry in std::fs::read_dir(&src).expect("src is readable") {
        let path = entry.expect("an entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let text = std::fs::read_to_string(&path).expect("readable");
        let non_test = text.split("#[cfg(test)]").next().unwrap_or("");
        let whole_file_gated = gated_files.contains(&name.as_str())
            || non_test.trim_start().starts_with("#![cfg(feature = \"server\")]");
        // A block-gated region starts at a line reading `#[cfg(feature = "server")]`
        // and ends at the next line that starts at column 0 with `#[cfg(` or
        // `pub mod`/`mod` or `// ---- end server ----`; the source keeps those
        // regions contiguous by convention (see lib.rs's layout after M6.5a).
        let mut depth_gate = false;
        for (n, line) in non_test.lines().enumerate() {
            if line.trim_start().starts_with("#[cfg(feature = \"server\")]") {
                depth_gate = true;
                continue;
            }
            if depth_gate && line.starts_with("// ---- end server ----") {
                depth_gate = false;
                continue;
            }
            let in_gate = whole_file_gated || depth_gate;
            for p in patterns {
                if line.contains(p) && !line.trim_start().starts_with("//") {
                    if in_gate {
                        gated_hits += 1;
                    } else {
                        escaped.push(format!("{name}:{}: {}", n + 1, line.trim()));
                    }
                }
            }
        }
    }
    assert!(gated_hits > 0, "no gated site matched any pattern — the patterns are stale");
    assert!(escaped.is_empty(), "platform calls outside the server feature:\n{}", escaped.join("\n"));
}
```

Convention this cell enforces (write it into `lib.rs`'s module doc): a `server`-only region inside a mixed file begins with a line `#[cfg(feature = "server")]` and ends with a comment line `// ---- end server ----` at column 0; whole `server` files begin with `#![cfg(feature = "server")]`.

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p yokohama-web --lib platform_calls_stay_behind`
Expected: FAIL naming `table.rs:418 SystemTime`, `store.rs` `std::fs` lines, `chat.rs:145-147`, `identity.rs:56`, `signal.rs:21`, and every `axum::` in `lib.rs`/`identity.rs`.

- [ ] **Step 3: Cargo.toml**

```toml
[features]
default = ["server"]
server = ["dep:axum", "dep:tokio", "dep:tokio-stream"]

[dependencies]
# … existing yokohama-core, yokohama-cli, serde, serde_json, sha2 unchanged …
# M6.5a (R-6.5a.2): the layer's refusal type is typed on the `http` crate directly —
# the same type axum re-exports — so the layer compiles without axum.
http = "1"
# M6.5a (R-6.5a.4): the layer parses query strings with the crate axum's `Query`
# uses, so the rejection sentences are byte-equal by construction.
serde_urlencoded = "0.7"
axum = { version = "0.8", optional = true }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net", "signal", "sync", "time"], optional = true }
tokio-stream = { version = "0.1", features = ["sync"], optional = true }

# M6.5a (R-6.5a.7): the ONE blocker between this crate and wasm32-unknown-unknown
# is getrandom's `js` feature, reached through rand from core's OsRng. Enabling it
# here unifies onto core's dependency and leaves core's manifest byte-identical.
# Version-pinned: getrandom 0.3 renames the feature and needs a RUSTFLAGS cfg.
[target.'cfg(target_arch = "wasm32")'.dependencies]
getrandom = { version = "0.2", features = ["js"] }

[[bin]]
name = "yokohama-web"
path = "src/main.rs"
required-features = ["server"]
```

- [ ] **Step 4: Gate, file by file**

- `lib.rs`: `use http::StatusCode;` replaces `use axum::http::StatusCode;` in `ApiError`; wrap the axum `use` lines, `JsonBody`, `QueryArgs`, `IntoResponse for ApiError`, `AppState`, every handler, `SIGNAL_KIND`, `HEARTBEAT`, `MAX_WAIT_SECONDS`, `app()`, `fallback` in `#[cfg(feature = "server")]` … `// ---- end server ----` regions (one region for the block of handlers is fine); `pub mod art;`/`pub mod assets;` gated. `AppState::with_store_and_chat` passes `Arc::new(crate::table::now_unix)` as the clock.
- `identity.rs`: `use http::HeaderMap;` for `resolve`; `from_env` and the extractor (with `axum::extract::FromRequestParts`, `Parts`) in a gated region.
- `store.rs`: `FsStore` (struct, impls, `path`, `read_locked`, …) and `default_dir` in a gated region; `PathBuf`/`Path` imports move with them.
- `chat.rs`: `ChatConfig::from_env` gated (`from_env_values` stays).
- `signal.rs`: everything after `NoSignal` gated.
- `table.rs`: `now_unix` gated.
- `art.rs`, `assets.rs`: `#![cfg(feature = "server")]` as the first line.
- `games.rs`: `with_limits` takes no clock from `table` — it calls `with_parts(rules, store, ChatConfig{lobby: true, limits}, Arc::new(NoSignal), Arc::new(|| 0))`; production clocks come from `AppState`.

- [ ] **Step 5: Verify both ways**

Run: `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings && cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`
Expected: all green; the scanning cell passes; the check line finishes with no error. If the check line reports a type or import that leaks, gate it and add it to the scanning cell's patterns if it is a platform call.

- [ ] **Step 6: CI**

In `.github/workflows/test.yml`, the `web-server` job:

```yaml
  web-server:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: rustup target add wasm32-unknown-unknown
      - run: cargo test -p yokohama-web --lib --test wire_fixtures
      # M6.5a (R-6.5a.7): the layer builds for the browser and the Worker without
      # its server half. This proves the DEPENDENCY fence (no axum, no mio); the
      # platform-call fence is `platform_calls_stay_behind_the_server_feature`,
      # which runs in the line above.
      - run: cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown
```

- [ ] **Step 7: Commit**

```bash
git add crates/yokohama-web/Cargo.toml crates/yokohama-web/src/lib.rs crates/yokohama-web/src/identity.rs crates/yokohama-web/src/store.rs crates/yokohama-web/src/chat.rs crates/yokohama-web/src/signal.rs crates/yokohama-web/src/table.rs crates/yokohama-web/src/art.rs crates/yokohama-web/src/assets.rs crates/yokohama-web/src/games.rs .github/workflows/test.yml
git commit -m "feat(web): the server feature — axum and tokio optional, ApiError on the http crate, the layer builds for wasm32; the platform-call scanning fence (R-6.5a.2, .5, .6, .7)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 5: `api.rs` — the request types, the parsers, the matcher, the four public routes and the adapter (R-6.5a.3, R-6.5a.4)

**Files:**
- Create: `crates/yokohama-web/src/api.rs`
- Modify: `crates/yokohama-web/src/lib.rs` (`pub mod api;`; `health`, `me`, `labels`, `art_manifest` handlers deleted; the adapter added; `app()` routes `/api/health`, `/api/me`, `/api/labels`, `/api/art` through the adapter)
- Test: `crates/yokohama-web/src/api.rs` test module; `lib.rs` test module (HEAD, 405, 404, the rejection pins)

**Interfaces:**
- Produces:
  ```rust
  pub struct ApiRequest<'a> { pub method: &'a str, pub path: &'a str, pub query: &'a str,
                              pub content_type: Option<&'a str>, pub body: &'a [u8] }
  pub struct ApiResponse { pub status: u16, pub body: String }   // body "" for 204/405/HEAD
  pub struct Host { pub identity_name: &'static str, pub signal_kind: &'static str }
  pub type Identity<'a> = Result<&'a User, &'a str>;
  pub fn dispatch(games: &Games, host: &Host, identity: Identity<'_>, req: &ApiRequest<'_>) -> ApiResponse;
  pub fn parse_json<T: DeserializeOwned>(content_type: Option<&str>, body: &[u8]) -> Result<T, ApiError>;
  pub fn parse_query<T: DeserializeOwned>(query: &str) -> Result<T, ApiError>;
  pub(crate) fn error_response(e: ApiError) -> ApiResponse;
  pub(crate) fn ok_json<T: Serialize>(status: u16, v: &T) -> ApiResponse;
  ```
  and the four public route functions `health(games, host) -> Health`, `me(host, identity) -> Me`, `labels(games) -> Labels`, `art_manifest() -> Result<ArtManifest, ApiError>` (the last stays in `lib.rs` under `server`, since `art` is gated — the matcher's `/api/art` entry is registered by the adapter, not the layer; see Step 4).
- Consumes: Tasks 1–4.

- [ ] **Step 1: Write the failing cells**

In `api.rs`'s test module:

```rust
#[test]
fn a_missing_content_type_is_axums_sentence() {
    let e = parse_json::<crate::api::Input>(None, br#"{"line":"1"}"#).unwrap_err();
    assert_eq!(e.status, http::StatusCode::BAD_REQUEST);
    assert_eq!(e.message, "Expected request with `Content-Type: application/json`");
}

#[test]
fn an_unknown_key_in_a_body_is_axums_sentence() {
    let e = parse_json::<crate::api::Input>(Some("application/json"), br#"{"line":"1","nope":1}"#).unwrap_err();
    assert!(e.message.starts_with("Failed to deserialize the JSON body into the target type: "), "{}", e.message);
    assert!(e.message.contains("unknown field `nope`"), "{}", e.message);
}

#[test]
fn a_bad_query_is_axums_sentence() {
    let e = parse_query::<crate::api::ListQuery>("limit=abc").unwrap_err();
    assert!(e.message.starts_with("Failed to deserialize query string: "), "{}", e.message);
}

#[test]
fn the_matcher_finds_every_route_in_apps_order() {
    // Every pattern app() registers, in its order; a route missing here is a
    // route the dispatcher cannot serve.
    let expected = [
        ("GET", "/api/health"), ("GET", "/api/me"), ("GET", "/api/labels"), ("GET", "/api/art"),
        ("GET", "/api/games"), ("POST", "/api/games"),
        ("GET", "/api/games/{id}"), ("DELETE", "/api/games/{id}"),
        ("GET", "/api/games/{id}/table"), ("GET", "/api/games/{id}/deal"),
        ("POST", "/api/games/{id}/keep"), ("POST", "/api/games/{id}/join"),
        ("POST", "/api/games/{id}/unlock"),
        ("GET", "/api/games/{id}/chat"), ("POST", "/api/games/{id}/chat"),
        ("POST", "/api/games/{id}/input"),
        ("GET", "/api/games/{id}/ports"), ("GET", "/api/games/{id}/labs"), ("GET", "/api/games/{id}/areas"),
        ("GET", "/api/games/{id}/log"),
        ("GET", "/api/chat"), ("POST", "/api/chat"),
        ("GET", "/api/games/{id}/at/{k}"), ("GET", "/api/games/{id}/suggest"),
        ("GET", "/api/games/{id}/why"), ("GET", "/api/games/{id}/index/{kind}"),
        ("GET", "/api/games/{id}/result"),
    ];
    let table = crate::api::routes();
    let names: Vec<(&str, &str)> = table.iter().map(|r| (r.method, r.pattern)).collect();
    assert_eq!(names, expected.to_vec());
}
```

And in `lib.rs`'s test module (server), the three pins against axum's own text and the HEAD/405/404 cells:

```rust
#[tokio::test]
async fn the_layers_json_rejection_is_axums_word_for_word() {
    use axum::extract::FromRequest;
    let req = axum::http::Request::builder()
        .method("POST").uri("/x")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(r#"{"line":"1","nope":1}"#)).unwrap();
    let axum_text = match axum::Json::<api::Input>::from_request(req, &()).await {
        Err(rej) => rej.body_text(),
        Ok(_) => panic!("axum accepted an unknown key"),
    };
    let ours = api::parse_json::<api::Input>(Some("application/json"), br#"{"line":"1","nope":1}"#).unwrap_err();
    assert_eq!(ours.message, axum_text);
}

#[tokio::test]
async fn the_layers_query_rejection_is_axums_word_for_word() {
    use axum::extract::FromRequestParts;
    let (mut parts, _) = axum::http::Request::builder().uri("/x?limit=abc").body(()).unwrap().into_parts();
    let axum_text = match axum::extract::Query::<api::ListQuery>::from_request_parts(&mut parts, &()).await {
        Err(rej) => rej.body_text(),
        Ok(_) => panic!("axum accepted limit=abc"),
    };
    assert_eq!(api::parse_query::<api::ListQuery>("limit=abc").unwrap_err().message, axum_text);
}

#[tokio::test]
async fn head_on_a_get_route_is_200_with_an_empty_body() {
    let (status, body) = raw(app(Arc::new(AppState::in_memory())), "HEAD", "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_empty());
}

#[tokio::test]
async fn a_known_path_under_the_wrong_method_is_405_and_an_unknown_path_is_the_json_404() {
    let app = app(Arc::new(AppState::in_memory()));
    let (status, _) = raw(app.clone(), "DELETE", "/api/health", None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    let (status, body) = raw(app, "GET", "/api/nothing", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, br#"{"error":"no such endpoint: /api/nothing"}"#);
}
```

(`raw` is the existing `raw_post`-style helper at `lib.rs:2186` generalised to take a method; add it beside it.)

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p yokohama-web --lib axums_sentence matcher_finds head_on_a_get wrong_method`
Expected: FAIL to compile (`api` does not exist).

- [ ] **Step 3: Write `api.rs`**

```rust
//! The table layer's one route table (M6.5a, R-6.5a.3). Every host — the axum
//! adapter today, the browser and the Durable Object later — converts its
//! request into an [`ApiRequest`], resolves identity ITS way, and calls
//! [`dispatch`]; the three transport routes (`events`, `stream`, `/api/stream`)
//! and the static routes are the ones a host writes itself.
//!
//! The parsers reproduce axum's rejection sentences (R-6.5a.4) and are pinned
//! against axum's own text in lib.rs's server-only cells.

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::games::Games;
use crate::identity::User;
use crate::ApiError;

pub struct ApiRequest<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub query: &'a str,
    pub content_type: Option<&'a str>,
    pub body: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiResponse {
    pub status: u16,
    pub body: String,
}

#[derive(Debug, Clone, Copy)]
pub struct Host {
    pub identity_name: &'static str,
    pub signal_kind: &'static str,
}

pub type Identity<'a> = Result<&'a User, &'a str>;

pub(crate) fn ok_json<T: Serialize>(status: u16, v: &T) -> ApiResponse {
    ApiResponse { status, body: serde_json::to_string(v).expect("every payload serializes") }
}

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

pub fn parse_json<T: DeserializeOwned>(content_type: Option<&str>, body: &[u8]) -> Result<T, ApiError> {
    let is_json = content_type
        .and_then(|ct| ct.parse::<mime::Mime>().ok())
        .map(|m| m.type_() == "application" && (m.subtype() == "json" || m.suffix().is_some_and(|s| s == "json")))
        .unwrap_or(false);
    if !is_json {
        return Err(ApiError::bad_request("Expected request with `Content-Type: application/json`"));
    }
    let mut de = serde_json::Deserializer::from_slice(body);
    match serde_path_to_error::deserialize::<_, T>(&mut de) {
        Ok(v) => Ok(v),
        Err(e) => {
            let inner = e.inner();
            Err(ApiError::bad_request(if inner.is_data() {
                format!("Failed to deserialize the JSON body into the target type: {e}")
            } else {
                format!("Failed to parse the request body as JSON: {e}")
            }))
        }
    }
}

pub fn parse_query<T: DeserializeOwned>(query: &str) -> Result<T, ApiError> {
    serde_urlencoded::from_str::<T>(query)
        .map_err(|e| ApiError::bad_request(format!("Failed to deserialize query string: {e}")))
}
```

**On the two sentences' exact shape:** axum 0.8's `JsonDataError` wraps a `serde_path_to_error::Error<serde_json::Error>` and prints `"Failed to deserialize the JSON body into the target type: {path}: {inner}"`; its `JsonSyntaxError` prints `"Failed to parse the request body as JSON: {inner}"`; the missing-content-type sentence is `"Expected request with `Content-Type: application/json`"`. **Read `axum-0.8.9/src/extract/rejection.rs` and `src/json.rs` in the cargo registry and match them exactly, including whether the path prefix appears** — the pins in Step 1 are what decide; adjust the format string, not the pin. Add `serde_path_to_error = "0.1"` and `mime = "0.3"` to `Cargo.toml` (both already in the lock through axum; both build for wasm).

Then the matcher and the four public routes:

```rust
pub struct Route {
    pub method: &'static str,
    pub pattern: &'static str,
}

/// `app()`'s table, in its order (R-6.5a.3). `/api/art` is listed so the 405/404
/// arithmetic is one table, but its body lives in the server (art is gated).
pub fn routes() -> Vec<Route> {
    [
        ("GET", "/api/health"), ("GET", "/api/me"), ("GET", "/api/labels"), ("GET", "/api/art"),
        ("GET", "/api/games"), ("POST", "/api/games"),
        ("GET", "/api/games/{id}"), ("DELETE", "/api/games/{id}"),
        ("GET", "/api/games/{id}/table"), ("GET", "/api/games/{id}/deal"),
        ("POST", "/api/games/{id}/keep"), ("POST", "/api/games/{id}/join"),
        ("POST", "/api/games/{id}/unlock"),
        ("GET", "/api/games/{id}/chat"), ("POST", "/api/games/{id}/chat"),
        ("POST", "/api/games/{id}/input"),
        ("GET", "/api/games/{id}/ports"), ("GET", "/api/games/{id}/labs"), ("GET", "/api/games/{id}/areas"),
        ("GET", "/api/games/{id}/log"),
        ("GET", "/api/chat"), ("POST", "/api/chat"),
        ("GET", "/api/games/{id}/at/{k}"), ("GET", "/api/games/{id}/suggest"),
        ("GET", "/api/games/{id}/why"), ("GET", "/api/games/{id}/index/{kind}"),
        ("GET", "/api/games/{id}/result"),
    ]
    .into_iter()
    .map(|(method, pattern)| Route { method, pattern })
    .collect()
}

/// Segment-wise match; `{name}` binds one segment. Returns the bound values in
/// pattern order.
fn matches<'p, 'a>(pattern: &'p str, path: &'a str) -> Option<Vec<&'a str>> {
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

#[derive(Debug, Clone, Serialize)]
pub struct Health {
    pub server_version: &'static str,
    pub semantics_rev: u32,
    pub rules_hash: String,
    pub identity: &'static str,
    pub signal: &'static str,
    pub lobby_chat: bool,
    pub chat_keep: Option<u64>,
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

#[derive(Debug, Clone, Serialize)]
pub struct Me {
    pub mode: &'static str,
    pub user: Option<String>,
}

pub fn me(host: &Host, identity: Identity<'_>) -> Me {
    Me { mode: host.identity_name, user: identity.ok().map(|u| u.as_str().to_string()) }
}
```

(`Games` needs `pub fn rules(&self) -> &Rules` if it lacks one — add it.) Move `Labels`, `key_of` and the body of `labels` (`lib.rs:407-500`) here as `pub fn labels(games: &Games) -> Labels`, replacing `state.rules` with `games.rules()`.

Then `dispatch`, with the four public arms now and the rest added in Task 6:

```rust
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
            let u = identity.map_err(unauthorized)?; // Task 6 adds the game and house arms below this line
            let _ = (u, b);
            Err(ApiError::not_found(format!("no such endpoint: {}", req.path)))
        }
    }
}
```

- [ ] **Step 4: The adapter in `lib.rs` (server)**

```rust
/// The ONE piece of axum between a request and the layer (R-6.5a.3). Identity
/// is resolved here with today's extractor logic and handed to the layer as the
/// host's sentence; the body and query reach the layer as bytes and text.
async fn adapter(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
) -> Response {
    let (parts, body) = req.into_parts();
    let bytes = match axum::body::to_bytes(body, 1 << 20).await {
        Ok(b) => b,
        Err(e) => return ApiError::bad_request(format!("the body could not be read: {e}")).into_response(),
    };
    let resolved = identity::resolve(state.identity, &parts.headers);
    let identity: api::Identity<'_> = match &resolved {
        Ok(u) => Ok(u),
        Err(m) => Err(m.as_str()),
    };
    let content_type = parts.headers.get(http::header::CONTENT_TYPE).and_then(|v| v.to_str().ok());
    let request = api::ApiRequest {
        method: parts.method.as_str(),
        path: parts.uri.path(),
        query: parts.uri.query().unwrap_or(""),
        content_type,
        body: &bytes,
    };
    if request.path == "/api/art" && parts.method == axum::http::Method::GET {
        return match art_manifest().await {
            Ok(j) => j.into_response(),
            Err(e) => e.into_response(),
        };
    }
    let host = api::Host { identity_name: state.identity.name(), signal_kind: SIGNAL_KIND };
    let out = api::dispatch(&state.games, &host, identity, &request);
    let mut response = Response::new(axum::body::Body::from(out.body.clone()));
    *response.status_mut() = StatusCode::from_u16(out.status).expect("a known status");
    if !out.body.is_empty() {
        response.headers_mut().insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("application/json"),
        );
    }
    response
}
```

In `app()`, replace the four public `.route(...)` lines with `.route("/api/health", get(adapter)).route("/api/me", get(adapter)).route("/api/labels", get(adapter)).route("/api/art", get(adapter))` for now (Task 6 collapses the rest into `.route("/api/{*rest}", any(adapter))` once every game route has moved; until then the explicit registrations keep axum's own 405 behaviour for the routes still on handlers). Delete the `health`, `me`, `labels` handlers and their DTOs from `lib.rs` (`pub use api::{Health, Me, Labels};` keeps their paths for tests).

**Body size:** axum's default body limit is 2 MiB; `1 << 20` here is 1 MiB and every body this API takes is under a kilobyte. Keep 2 MiB (`2 << 20`) to change nothing.

- [ ] **Step 5: Run everything**

Run: `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings && cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`
Expected: green including the new cells; `wire_fixtures` byte-identical (`health.json`, `me.json`, `me_signed_out.json`, `labels.json` unchanged — the compare cell is the assertion). The 401 sweep is unchanged because `me` never refuses and the other three take no identity.

- [ ] **Step 6: Commit**

```bash
git add crates/yokohama-web/Cargo.toml crates/yokohama-web/src/api.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/games.rs
git commit -m "feat(web): api.rs — the layer's request types, axum-equal parsers, the route table and dispatch, with the four public routes through the adapter (R-6.5a.3, R-6.5a.4)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 6: Move every game and house route into the layer; `app()` becomes the adapter plus the transport routes (R-6.5a.3)

**Files:**
- Modify: `crates/yokohama-web/src/api.rs` (the helpers `admitted`, `NOT_SEATED`, `spectators_write_nothing`, `holder_of`, `status_refusal`, `undo_allowed`, `author_of`, `lobby_chat_on`, `events_of`, `events_at`; the DTOs; the per-route functions; `dispatch`'s arms)
- Modify: `crates/yokohama-web/src/lib.rs` (delete the moved handlers; keep `events`, `stream`, `house_stream` calling `api::events_at`; `app()` = transport routes + `/api/{*rest}` adapter + fallback)
- Test: `crates/yokohama-web/tests/wire_fixtures.rs` (the dispatcher-equals-router cell); `lib.rs` test module (the 401 pin)

**Interfaces:**
- Produces, all `pub` in `api.rs`, each returning `Result<T, ApiError>` where the handler returned `ApiResult<T>`:
  ```rust
  pub fn admitted(games: &Games, id: &str, user: &User) -> Result<(TableDoc, Viewer), ApiError>;
  pub fn events_of(repl: &Repl, viewer: Viewer, chat: u64) -> Events;
  pub fn events_at(games: &Games, id: &str, user: &User) -> Result<Events, ApiError>;
  pub fn list_games(games: &Games, user: &User, q: &ListQuery) -> Result<GamesList, ApiError>;
  pub fn create_game(games: &Games, user: &User, req: &NewGame) -> Result<TableView, ApiError>;   // 201
  pub fn get_table(games: &Games, user: &User, id: &str) -> Result<TableView, ApiError>;
  pub fn deal(games: &Games, user: &User, id: &str) -> Result<DealView, ApiError>;
  pub fn keep(games: &Games, user: &User, id: &str, req: &Keep) -> Result<TableView, ApiError>;
  pub fn join(games: &Games, user: &User, id: &str, req: &Join) -> Result<TableView, ApiError>;
  pub fn unlock(games: &Games, user: &User, id: &str, req: &Unlock) -> Result<TableView, ApiError>;
  pub fn chat_read(games: &Games, user: &User, id: &str, q: &ChatQuery) -> Result<ChatPage, ApiError>;
  pub fn chat_post(games: &Games, user: &User, id: &str, req: &ChatPost) -> Result<Message, ApiError>; // 201
  pub fn house_chat_read(games: &Games, user: &User, q: &ChatQuery) -> Result<ChatPage, ApiError>;
  pub fn house_chat_post(games: &Games, user: &User, req: &ChatPost) -> Result<Message, ApiError>;  // 201
  pub fn get_game(games: &Games, user: &User, id: &str) -> Result<ClientState, ApiError>;
  pub fn delete_game(games: &Games, user: &User, id: &str) -> Result<(), ApiError>;                 // 204
  pub fn input(games: &Games, user: &User, id: &str, req: &Input) -> Result<ClientState, ApiError>;
  pub fn ports/labs/areas(games: &Games, user: &User, id: &str) -> Result<PortsPanel|LabsPanel|AreasPanel, ApiError>;
  pub fn log(games: &Games, user: &User, id: &str, q: &LogQuery) -> Result<LogPanel, ApiError>;
  pub fn at_k(games: &Games, user: &User, id: &str, k: usize) -> Result<HistoricalFrame, ApiError>;
  pub fn suggest(games: &Games, user: &User, id: &str) -> Result<SuggestResponse, ApiError>;
  pub fn why(games: &Games, user: &User, id: &str, q: &WhyQuery) -> Result<Why, ApiError>;
  pub fn card_index(games: &Games, user: &User, id: &str, kind: &str, q: &IndexQuery) -> Result<IndexModel, ApiError>;
  pub fn result(games: &Games, user: &User, id: &str) -> Result<Outcome, ApiError>;
  ```
- Consumes: Task 5's types and parsers.

- [ ] **Step 1: Write the failing cells**

In `tests/wire_fixtures.rs`, after the compare cell:

```rust
/// R-6.5a.14: the layer's dispatcher answers what the router answers, for every
/// committed non-transport payload — the cross-host contract in miniature,
/// under axum, before any second host exists. Replays each payload's request
/// through `api::dispatch` directly and compares status and body byte for byte
/// with the router's answer captured in the same walk.
#[tokio::test]
async fn the_dispatcher_answers_what_the_router_answers() {
    assert!(std::env::var_os("UPDATE_WIRE_FIXTURES").is_none(),
        "regeneration run — re-run WITHOUT UPDATE_WIRE_FIXTURES to check the corpus");
    let api = Api::new();
    let mut compared = 0usize;
    for (name, request, router_status, router_body) in api.walk_with_requests().await {
        if request.path.ends_with("/events") || request.path.ends_with("/stream") || request.path == "/api/stream" {
            continue;
        }
        let host = yokohama_web::api::Host { identity_name: "claimed", signal_kind: yokohama_web::SIGNAL_KIND };
        let user = request.who.as_deref().map(|w| yokohama_web::identity::User::parse(w).unwrap());
        let identity = match &user { Some(u) => Ok(u), None => Err("no signed-in user: send X-Yokohama-Claimed-User") };
        let out = yokohama_web::api::dispatch(&api.state.games, &host, identity, &yokohama_web::api::ApiRequest {
            method: &request.method, path: &request.path, query: &request.query,
            content_type: request.body.as_ref().map(|_| "application/json"),
            body: request.body.as_deref().unwrap_or(b""),
        });
        assert_eq!(out.status, router_status.as_u16(), "{name}: status through dispatch vs router");
        assert_eq!(out.body.as_bytes(), router_body.as_slice(), "{name}: body through dispatch vs router");
        compared += 1;
    }
    assert!(compared >= 60, "the walk compared only {compared} payloads — the corpus is larger than that");
}
```

`walk_with_requests` is a variant of `walk()` (`tests/wire_fixtures.rs:471`) that records, for every call it makes, `(name, Recorded { who, method, path, query, body }, status, raw body bytes)` — build it by threading a recorder through `Api::call_as` (add a `record: RefCell<Vec<…>>` field to `Api`, filled by `call_as`, and have `walk_with_requests` return `walk()`'s corpus zipped with the recorder). **The dispatcher replays against the SAME state the router mutated**, so for a write route the replay would apply the move twice; therefore the replay compares READ routes only (GET) and the 4xx refusals, and asserts `compared >= 60` — the read half of the corpus. Write routes are covered by the router cells and the corpus itself.

In `lib.rs`'s test module:

```rust
#[tokio::test]
async fn an_err_identity_through_dispatch_is_the_extractors_401_sentence_in_every_mode() {
    for mode in [identity::IdentityMode::Claimed, identity::IdentityMode::CfAccess, identity::IdentityMode::Worker] {
        let sentence = identity::resolve(mode, &http::HeaderMap::new()).unwrap_err();
        let state = AppState::in_memory_with(mode);
        let host = api::Host { identity_name: mode.name(), signal_kind: SIGNAL_KIND };
        let out = api::dispatch(&state.games, &host, Err(&sentence), &api::ApiRequest {
            method: "GET", path: "/api/games", query: "", content_type: None, body: b"" });
        assert_eq!(out.status, 401);
        assert_eq!(out.body, serde_json::json!({ "error": sentence }).to_string());
    }
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p yokohama-web --test wire_fixtures dispatcher_answers` and `cargo test -p yokohama-web --lib err_identity_through_dispatch`
Expected: FAIL — `/api/games` is not dispatched yet (the layer answers the placeholder 404).

- [ ] **Step 3: Move the helpers and the handler bodies**

For each handler in `lib.rs`, in this order — `admitted` and the ten helpers first (`lib.rs:506-700`, `:951-990`, `:1816-1930`), then `list_games` (`:725`), `create_game` (`:762`), `get_table` (`:777`), `deal` (`:787`), `keep` (`:1019`), `join` (`:821`), `unlock` (`:852`), `chat_read` (`:910`), `chat_post` (`:931`), `house_chat_read` (`:990`), `house_chat_post` (`:1006`), `get_game` (`:1034`), `delete_game` (`:1049`), `input` (`:1103`), `ports`/`labs`/`areas` (`:1328-1346`, as one `fn surface<T>(games, user, id, build: fn(&GameState, &Rules) -> T)`), `log` (`:1660`), `at_k` (`:2050`), `suggest` (`:1387`), `why` (`:1484`), `card_index` (`:1551`), `result` (`:1620`) — cut the body into `api.rs` as the `pub fn` in the Interfaces block, applying exactly these rewrites and nothing else:
  - `state.games.X` → `games.X`; `&state` → `games` where a helper is called; `state.rules` → `games.rules()`; `state.chat` → `games.chat`;
  - `Ok(Json(v))` → `Ok(v)`; `Ok((StatusCode::CREATED, Json(v)))` → `Ok(v)` (the status lives in `dispatch`); `Ok(StatusCode::NO_CONTENT)` → `Ok(())`;
  - `StatusCode::X` → `http::StatusCode::X` (already the same type);
  - the DTO structs (`ListQuery`, `GamesList`, `Keep`, `Join`, `Unlock`, `ChatPost`, `ChatQuery`, `ChatPage`, `Input`, `LogQuery`, `IndexQuery`, `WhyQuery`, `Events`, `EventsQuery`) move with their `#[serde(deny_unknown_fields)]` untouched; `lib.rs` re-exports them (`pub use api::{…}`) so `tests/` and the client-contract names keep compiling.

Then the arms of `answer` (Task 5), replacing its `_` placeholder: the `let u = identity.map_err(unauthorized)?;` line stays FIRST in the fallthrough so the public four never touch it, and each arm below reads `u` and the bound segments `b`:

```rust
        "/api/games" if req.method == "GET" => list_games(games, u, &parse_query(req.query)?).map(|v| ok_json(200, &v)),
        "/api/games" => create_game(games, u, &parse_json(req.content_type, req.body)?).map(|v| ok_json(201, &v)),
        "/api/games/{id}" if req.method == "GET" => get_game(games, u, b[0]).map(|v| ok_json(200, &v)),
        "/api/games/{id}" => delete_game(games, u, b[0]).map(|()| ApiResponse { status: 204, body: String::new() }),
        "/api/games/{id}/table" => get_table(games, u, b[0]).map(|v| ok_json(200, &v)),
        "/api/games/{id}/deal" => deal(games, u, b[0]).map(|v| ok_json(200, &v)),
        "/api/games/{id}/keep" => keep(games, u, b[0], &parse_json(req.content_type, req.body)?).map(|v| ok_json(200, &v)),
        "/api/games/{id}/join" => join(games, u, b[0], &parse_json(req.content_type, req.body)?).map(|v| ok_json(200, &v)),
        "/api/games/{id}/unlock" => unlock(games, u, b[0], &parse_json(req.content_type, req.body)?).map(|v| ok_json(200, &v)),
        "/api/games/{id}/chat" if req.method == "GET" => chat_read(games, u, b[0], &parse_query(req.query)?).map(|v| ok_json(200, &v)),
        "/api/games/{id}/chat" => chat_post(games, u, b[0], &parse_json(req.content_type, req.body)?).map(|v| ok_json(201, &v)),
        "/api/games/{id}/input" => input(games, u, b[0], &parse_json(req.content_type, req.body)?).map(|v| ok_json(200, &v)),
        "/api/games/{id}/ports" => surface(games, u, b[0], yokohama_cli::screen_model::build_ports).map(|v| ok_json(200, &v)),
        "/api/games/{id}/labs" => surface(games, u, b[0], yokohama_cli::screen_model::build_labs).map(|v| ok_json(200, &v)),
        "/api/games/{id}/areas" => surface(games, u, b[0], yokohama_cli::screen_model::build_areas).map(|v| ok_json(200, &v)),
        "/api/games/{id}/log" => log(games, u, b[0], &parse_query(req.query)?).map(|v| ok_json(200, &v)),
        "/api/chat" if req.method == "GET" => house_chat_read(games, u, &parse_query(req.query)?).map(|v| ok_json(200, &v)),
        "/api/chat" => house_chat_post(games, u, &parse_json(req.content_type, req.body)?).map(|v| ok_json(201, &v)),
        "/api/games/{id}/at/{k}" => {
            let k = parse_path_usize("k", b[1])?;
            at_k(games, u, b[0], k).map(|v| ok_json(200, &v))
        }
        "/api/games/{id}/suggest" => suggest(games, u, b[0]).map(|v| ok_json(200, &v)),
        "/api/games/{id}/why" => why(games, u, b[0], &parse_query(req.query)?).map(|v| ok_json(200, &v)),
        "/api/games/{id}/index/{kind}" => card_index(games, u, b[0], b[1], &parse_query(req.query)?).map(|v| ok_json(200, &v)),
        "/api/games/{id}/result" => result(games, u, b[0]).map(|v| ok_json(200, &v)),
```

**The order inside an arm matters and is today's**: `dispatch` resolves identity (401) BEFORE parsing the body or query, because axum runs the `Identity` extractor before `JsonBody`/`QueryArgs` in every handler's argument list — check each handler's argument order at HEAD (`events` at `:1783` lists `State` first, then `Identity`; the order of the two extractors that can REFUSE is what matters, and identity is first in all of them). `parse_path_usize(name, raw)` produces axum's `PathRejection` text for a non-integer — read `axum-0.8.9/src/extract/path/mod.rs` (`ErrorKind::ParseErrorAtKey`'s `Display`) and reproduce it; add a `lib.rs` server cell pinning it against `axum::extract::Path::<(String, usize)>::from_request_parts` on `/api/games/x/at/abc`, in the shape of Task 5's pins.

Path segments arrive percent-encoded; axum's `Path` decodes them. The matcher binds the raw segment; decode each bound value with `percent_encoding::percent_decode_str(x).decode_utf8_lossy()` ONLY if a route cell at HEAD sends an encoded id — grep `lib.rs`'s tests for `%`; if none, bind raw and record that the ids this API accepts are `is_safe_id`'s ASCII set, which needs no decoding.

`app()` becomes:

```rust
pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/games/{id}/events", get(events))
        .route("/api/games/{id}/stream", get(stream))
        .route("/api/stream", get(house_stream))
        .route("/api/{*rest}", axum::routing::any(adapter))
        .route("/api/games", axum::routing::any(adapter))
        .with_state(state)
        .fallback(fallback)
}
```

(`/api/games` has no `{*rest}` tail and needs its own line; `/api/health` etc. are under `{*rest}`.) `events`, `stream` and `house_stream` call `api::events_at(&state.games, …)`.

- [ ] **Step 4: Run everything, then the corpus**

Run: `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings && cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`
Expected: every route cell green through the adapter; `wire_fixtures` byte-identical (the compare cell); the new cells green. Then `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures` and `git diff --stat web/src/wire/` — **expected: empty**. If any file differs, the dispatcher answered differently from the router at HEAD; fix the dispatcher, never the fixture.

- [ ] **Step 5: Commit**

```bash
git add crates/yokohama-web/src/api.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/tests/wire_fixtures.rs
git commit -m "refactor(web): every game and house route moves into the layer; app() is the three transport routes and one adapter; the dispatcher answers what the router answers (R-6.5a.3)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 7: The archive — module, `archived_at`, the health key (R-6.5a.9, R-6.5a.10, R-6.5a.13's health half)

**Files:**
- Create: `crates/yokohama-web/src/archive.rs`
- Modify: `crates/yokohama-web/src/table.rs:170-187` (`archived_at`), `crates/yokohama-web/src/tables.rs:52` (`TABLE_SCHEMA = 3`), `:362-450` (`check_shape` bounds `archived_at`), `:723-741` (`create` sets `archived_at: None`)
- Modify: `crates/yokohama-web/src/games.rs` (`archive: Arc<dyn Archive>` on `Games`; `with_parts` takes it; `with_archive(self, a)`)
- Modify: `crates/yokohama-web/src/api.rs` (`Health.archive`)
- Modify: `crates/yokohama-web/src/lib.rs` (`AppState::new` builds `DirArchive` from `YOKOHAMA_WEB_ARCHIVE_DIR`, `NoArchive` otherwise; `in_memory*` use `NoArchive`)
- Modify: `crates/yokohama-web/src/main.rs` (a bad archive directory exits 1 naming the variable)
- Modify: `web/src/wire/health.json` (regenerated: `"archive": "off"`)
- Test: `archive.rs` test module; `tables.rs` test module; `wire_fixtures` regenerated

**Interfaces:**
- Produces:
  ```rust
  pub enum Why { Over, Abandoned }
  pub trait Archive: Send + Sync {
      fn put(&self, why: Why, id: &str, record_json: &str, table_json: &str) -> Result<(), String>;
      fn kind(&self) -> &'static str;
  }
  pub fn paths(rev: u32, why: Why, id: &str) -> (String, String);
  pub fn strip_for_archive(doc: &TableDoc) -> String;
  pub struct NoArchive;                    // kind() == "off"
  pub struct DirArchive { dir: PathBuf }   // server; kind() == "dir"
  impl DirArchive { pub fn new(dir: impl Into<PathBuf>) -> Self; pub fn from_env() -> Result<Option<Self>, String> }
  Games::with_archive(self, Arc<dyn Archive>) -> Self;  Games.archive: Arc<dyn Archive>
  TableDoc.archived_at: Option<u64>
  ```
- Consumes: Tasks 1–4.

- [ ] **Step 1: Write the failing cells**

`archive.rs` test module:

```rust
#[test]
fn the_layout_is_one_function_for_both_reasons() {
    assert_eq!(paths(7, Why::Over, "sim-1"), ("rev7/over/sim-1.json".into(), "rev7/over/sim-1.table.json".into()));
    assert_eq!(paths(7, Why::Abandoned, "sim-1"), ("rev7/abandoned/sim-1.json".into(), "rev7/abandoned/sim-1.table.json".into()));
}

#[test]
fn a_stripped_table_carries_no_lock_and_no_watcher() {
    let mut doc = crate::table::test_support::two_player_doc("sim-2", "a@x.test", "b@x.test"); // the module's own fixture helper; reuse the one the lock cells use
    doc.lock = Some(crate::table::Lock::new("hunter2"));
    doc.unlocked = vec!["watcher@x.test".to_string()];
    let (salt, hash) = { let l = doc.lock.as_ref().unwrap(); (l.salt.clone(), l.hash.clone()) };
    let json = strip_for_archive(&doc);
    assert!(!json.contains(&salt) && !json.contains(&hash), "the salt and hash left with the lock");
    assert!(!json.contains("watcher@x.test"), "the watcher's address left with the roster");
    assert!(json.contains("a@x.test") && json.contains("b@x.test"), "anti-vacuity: the seats are still there");
    assert!(json.contains("\"archived_at\""));
}

#[cfg(feature = "server")]
#[test]
fn the_directory_archive_writes_both_files_and_refuses_a_second_put() {
    let dir = std::env::temp_dir().join(format!("yokohama_archive_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let a = DirArchive::new(&dir);
    a.put(Why::Abandoned, "sim-3", "{\"record\":1}", "{\"table\":1}").unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("rev7/abandoned/sim-3.json")).unwrap(), "{\"record\":1}");
    assert_eq!(std::fs::read_to_string(dir.join("rev7/abandoned/sim-3.table.json")).unwrap(), "{\"table\":1}");
    let second = a.put(Why::Abandoned, "sim-3", "{\"record\":2}", "{\"table\":2}");
    assert!(second.is_err(), "a second put of the same game is refused, never versioned");
    assert_eq!(std::fs::read_to_string(dir.join("rev7/abandoned/sim-3.json")).unwrap(), "{\"record\":1}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn no_archive_is_off() {
    assert_eq!(NoArchive.kind(), "off");
}
```

`tables.rs` test module:

```rust
#[test]
fn a_schema_two_table_document_reads_with_archived_at_absent_and_a_bound_is_kept_on_write() {
    let (doc, _) = /* read a document the existing schema-2 cell writes by hand — reuse `the_m6_1_document_reads_unchanged`'s JSON, with "schema": 2 */;
    assert_eq!(doc.archived_at, None);
    let mut bad = doc.clone();
    bad.archived_at = Some(crate::chat::MAX_CREATED_AT + 1);
    assert!(check_shape(&bad).unwrap_err().contains("archived_at"));
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p yokohama-web --lib archive schema_two_table_document`
Expected: FAIL to compile.

- [ ] **Step 3: Implement `archive.rs`**

```rust
//! The archive (M6.5a, R-6.5a.9): a finished or abandoned game's record and
//! table document, written once, in the file store's own layout under a
//! `SEMANTICS_REV` prefix, so an `rclone sync` of the bucket M6.5c adds is a
//! directory the CLI loads. One layout function for every implementation.

use std::sync::Arc;

use crate::table::TableDoc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    Over,
    Abandoned,
}

impl Why {
    fn dir(self) -> &'static str {
        match self {
            Self::Over => "over",
            Self::Abandoned => "abandoned",
        }
    }
}

pub trait Archive: Send + Sync {
    fn put(&self, why: Why, id: &str, record_json: &str, table_json: &str) -> Result<(), String>;
    fn kind(&self) -> &'static str;
}

pub fn paths(rev: u32, why: Why, id: &str) -> (String, String) {
    let base = format!("rev{rev}/{}/{id}", why.dir());
    (format!("{base}.json"), format!("{base}.table.json"))
}

/// The stored document with the lock and the watcher roster removed.
pub fn strip_for_archive(doc: &TableDoc) -> String {
    let mut copy = doc.clone();
    copy.lock = None;
    copy.unlocked = Vec::new();
    serde_json::to_string_pretty(&copy).expect("a table document serializes")
}

pub struct NoArchive;

impl Archive for NoArchive {
    fn put(&self, _why: Why, _id: &str, _record: &str, _table: &str) -> Result<(), String> {
        Err("no archive is configured".to_string())
    }
    fn kind(&self) -> &'static str {
        "off"
    }
}

/// `Games` asks this before calling `put`, so `NoArchive::put` is never reached
/// on the happy path and its `Err` is a guard, not a failure.
pub fn is_on(a: &Arc<dyn Archive>) -> bool {
    a.kind() != "off"
}

#[cfg(feature = "server")]
pub use dir::DirArchive;

#[cfg(feature = "server")]
mod dir {
    use std::path::{Path, PathBuf};

    use super::{paths, Archive, Why};

    pub const ENV_VAR: &str = "YOKOHAMA_WEB_ARCHIVE_DIR";

    pub struct DirArchive {
        dir: PathBuf,
    }

    impl DirArchive {
        pub fn new(dir: impl Into<PathBuf>) -> Self {
            Self { dir: dir.into() }
        }

        /// `None` when the variable is unset (the archive is off); `Err` when it
        /// is set to a path that cannot be created — a bad value exits 1 naming
        /// the variable, like the chat variables.
        pub fn from_env() -> Result<Option<Self>, String> {
            let Some(raw) = std::env::var_os(ENV_VAR) else { return Ok(None) };
            let dir = PathBuf::from(raw);
            std::fs::create_dir_all(&dir)
                .map_err(|e| format!("{ENV_VAR}={} cannot be created: {e}", dir.display()))?;
            Ok(Some(Self { dir }))
        }

        fn write_new(path: &Path, text: &str) -> Result<(), String> {
            if path.exists() {
                return Err(format!("{} already exists — a game is archived once", path.display()));
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
            }
            let tmp = path.with_extension("json.tmp");
            std::fs::write(&tmp, text).map_err(|e| format!("write {}: {e}", tmp.display()))?;
            std::fs::rename(&tmp, path).map_err(|e| format!("rename {}: {e}", path.display()))
        }
    }

    impl Archive for DirArchive {
        fn put(&self, why: Why, id: &str, record_json: &str, table_json: &str) -> Result<(), String> {
            let (record, table) = paths(yokohama_core::SEMANTICS_REV, why, id);
            Self::write_new(&self.dir.join(&record), record_json)?;
            Self::write_new(&self.dir.join(&table), table_json)
        }
        fn kind(&self) -> &'static str {
            "dir"
        }
    }
}
```

`lib.rs`: `pub mod archive;`. `table.rs`: add `#[serde(default)] pub archived_at: Option<u64>,` after `unlocked`. `tables.rs`: `TABLE_SCHEMA = 3` (update its doc: "3 since M6.5a, `archived_at`"); in `check_shape` add

```rust
if let Some(at) = doc.archived_at {
    if at > crate::chat::MAX_CREATED_AT {
        return Err(format!("archived_at {at} is past {}", crate::chat::MAX_CREATED_AT));
    }
}
```

and `create` sets `archived_at: None`. `games.rs`: field `archive: Arc<dyn Archive>`, `with_parts(..., archive)`, `with_limits` passes `Arc::new(NoArchive)`, `with_archive`. `api.rs`'s `Health` gains `pub archive: &'static str` = `games.archive.kind()`. `AppState::new` (server): `let archive: Arc<dyn Archive> = match archive::DirArchive::from_env() { Ok(Some(d)) => Arc::new(d), Ok(None) => Arc::new(NoArchive), Err(e) => return Err(e) }` — make `AppState::new` return `Result<Self, String>` and `main.rs` exit 1 on `Err` printing it, the shape `ChatConfig::from_env` already has.

- [ ] **Step 4: Regenerate the corpus and read the diff**

Run: `cargo test -p yokohama-web` (expect `wire_fixtures`' compare cell RED on `health.json` only), then `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, then `git diff web/src/wire/`.
Expected diff: `health.json` gains `"archive": "off"` and NOTHING else changes. Then `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings && cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown` green.

- [ ] **Step 5: Commit**

```bash
git add crates/yokohama-web/src/archive.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/main.rs crates/yokohama-web/src/table.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/api.rs web/src/wire/health.json
git commit -m "feat(web): the archive trait, its directory implementation and one layout function; archived_at at table schema 3; health reports archive (R-6.5a.9, .10) — corpus: health.json +archive, nothing else

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 8: The two archive sites and their failure rules (R-6.5a.11)

**Files:**
- Modify: `crates/yokohama-web/src/games.rs:363-546` (`persist`: the game-over site after `mirror_turn`)
- Modify: `crates/yokohama-web/src/tables.rs:1648-1696` (`delete`: the abandon site before `store.delete`)
- Test: `games.rs` and `tables.rs` test modules

**Interfaces:**
- Consumes: Task 7's `Archive`, `Why`, `is_on`, `strip_for_archive`, `TableDoc.archived_at`.
- Produces: nothing new on the wire; `GameError::Server` sentence `"the archive could not be written (<why>) — the table was not deleted"`.

- [ ] **Step 1: Write the failing cells**

A recording archive fake in `games.rs`'s `test_support`:

```rust
pub(crate) struct RecordingArchive {
    pub puts: Mutex<Vec<(crate::archive::Why, String, String, String)>>,
    pub fail: bool,
}
impl crate::archive::Archive for RecordingArchive {
    fn put(&self, why: crate::archive::Why, id: &str, r: &str, t: &str) -> Result<(), String> {
        if self.fail { return Err("disk on fire".to_string()); }
        self.puts.lock().unwrap().push((why, id.to_string(), r.to_string(), t.to_string()));
        Ok(())
    }
    fn kind(&self) -> &'static str { "recording" }
}
```

Cells (use the module's helpers to build a vs-bot game and play it to the end — `play_to_the_end` exists for the whole-game cells; if it does not, drive `repl.step` with the first option until `is_over`, the way `tests/play_a_whole_game.rs:112` chooses):

```rust
#[test]
fn a_game_that_ends_is_archived_once_with_the_stored_bytes() {
    let archive = Arc::new(RecordingArchive { puts: Mutex::new(vec![]), fail: false });
    let games = games_in_memory().with_archive(archive.clone());
    let id = a_vs_bot_game_played_to_the_end(&games);          // persists on every move
    let puts = archive.puts.lock().unwrap();
    assert_eq!(puts.len(), 1, "one put at game over, not one per move");
    assert_eq!(puts[0].0, crate::archive::Why::Over);
    let (stored, _) = games.store().read(Doc::Record, &id).unwrap().unwrap();
    assert_eq!(puts[0].2, stored, "the archived record is the stored bytes");
    let (doc, _) = games.read_table(&id).unwrap().unwrap();
    assert!(doc.archived_at.is_some());
    assert!(!puts[0].3.contains("\"lock\":{"), "the archived table document is stripped");
}

#[test]
fn a_failing_archive_at_game_over_still_answers_the_move() {
    let archive = Arc::new(RecordingArchive { puts: Mutex::new(vec![]), fail: true });
    let games = games_in_memory().with_archive(archive);
    let id = a_vs_bot_game_played_to_the_end(&games);          // every persist returned Ok
    let (doc, _) = games.read_table(&id).unwrap().unwrap();
    assert_eq!(doc.archived_at, None, "unset says a retry is owed");
}

#[test]
fn abandoning_a_playing_table_with_events_archives_then_deletes() {
    let archive = Arc::new(RecordingArchive { puts: Mutex::new(vec![]), fail: false });
    let games = games_in_memory().with_archive(archive.clone());
    let (id, creator) = a_two_person_game_with_one_move(&games);
    games.delete(&id, &creator).unwrap();
    let puts = archive.puts.lock().unwrap();
    assert_eq!(puts.len(), 1);
    assert_eq!(puts[0].0, crate::archive::Why::Abandoned);
    assert!(games.read_table(&id).unwrap().is_none(), "deleted after the put");
}

#[test]
fn abandoning_an_open_table_or_an_archived_one_archives_nothing() {
    let archive = Arc::new(RecordingArchive { puts: Mutex::new(vec![]), fail: false });
    let games = games_in_memory().with_archive(archive.clone());
    let (open_id, creator) = an_open_table(&games);
    games.delete(&open_id, &creator).unwrap();
    let finished = a_vs_bot_game_played_to_the_end(&games);      // archived at over (1 put)
    games.delete(&finished, &creator_of(&games, &finished)).unwrap();
    assert_eq!(archive.puts.lock().unwrap().len(), 1, "the open table and the already-archived one added nothing");
}

#[test]
fn an_abandon_whose_put_fails_is_refused_and_the_table_stays() {
    let archive = Arc::new(RecordingArchive { puts: Mutex::new(vec![]), fail: true });
    let games = games_in_memory().with_archive(archive);
    let (id, creator) = a_two_person_game_with_one_move(&games);
    let err = games.delete(&id, &creator).unwrap_err();
    assert!(matches!(err, GameError::Server(ref m) if m.starts_with("the archive could not be written (") && m.ends_with("— the table was not deleted")), "{err}");
    assert!(games.read_table(&id).unwrap().is_some(), "nothing deleted");
    assert!(games.store().read(Doc::Record, &id).unwrap().is_some(), "the record is intact");
    assert!(games.handle(&id).is_ok(), "no tombstone");
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p yokohama-web --lib archived_once failing_archive abandoning an_abandon_whose`
Expected: FAIL — no put ever happens / the delete succeeds.

- [ ] **Step 3: Implement the two sites**

In `persist`, after `self.mirror_turn(id, repl);` and before `self.signal.bump(id);`:

```rust
        self.archive_if_over(id, repl, &json);
```

with

```rust
    /// R-6.5a.11's game-over site. Best effort: a failure is logged and leaves
    /// `archived_at` unset, which is what says a retry is owed; the move was
    /// already written and is answered 200 regardless.
    fn archive_if_over(&self, id: &str, repl: &Repl, record_json: &str) {
        if !crate::archive::is_on(&self.archive) || !repl.session.is_over() {
            return;
        }
        let Ok(Some((mut doc, version))) = self.read_table(id) else { return };
        if doc.archived_at.is_some() {
            return;
        }
        doc.archived_at = Some((self.clock)());
        let table_json = crate::archive::strip_for_archive(&doc);
        match self.archive.put(crate::archive::Why::Over, id, record_json, &table_json) {
            Ok(()) => {
                let _ = self.write_table(&doc, Some(version));
            }
            Err(e) => eprintln!("archive of {id:?} at game over failed: {e}"),
        }
    }
```

(`json` is the record text `persist` already built for the store write — pass it rather than re-serializing, so the archived bytes are the stored bytes.)

In `delete`, after the creator check and before `self.deleted…insert`:

```rust
        if crate::archive::is_on(&self.archive) && doc.status == TableStatus::Playing && doc.archived_at.is_none() {
            let record = self.store.read(Doc::Record, id).map_err(GameError::Server)?;
            if let Some((record_json, _)) = record {
                let events = serde_json::from_str::<GameRecord>(&record_json)
                    .map(|r| r.events.len())
                    .map_err(|e| GameError::Server(format!("{id:?}'s record does not parse: {e}")))?;
                if events > 0 {
                    let mut stamped = doc.clone();
                    stamped.archived_at = Some((self.clock)());
                    let table_json = crate::archive::strip_for_archive(&stamped);
                    self.archive
                        .put(crate::archive::Why::Abandoned, id, &record_json, &table_json)
                        .map_err(|why| GameError::Server(format!(
                            "the archive could not be written ({why}) — the table was not deleted"
                        )))?;
                }
            }
        }
```

- [ ] **Step 4: Run everything**

Run: `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings && cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`
Expected: green; corpus byte-identical (the wire walk runs with `NoArchive`, so no put and no `archived_at` on the wire — `TableView` does not carry it).

- [ ] **Step 5: Commit**

```bash
git add crates/yokohama-web/src/games.rs crates/yokohama-web/src/tables.rs
git commit -m "feat(web): the two archive sites — game over best-effort with archived_at as the retry mark, abandon awaits the put and refuses on failure (R-6.5a.11)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 9: `GET /api/games/{id}/record` (R-6.5a.12, R-6.5a.14)

**Files:**
- Modify: `crates/yokohama-web/src/api.rs` (`routes()` gains `("GET", "/api/games/{id}/record")` after `result`; `pub fn record(games, user, id) -> Result<String, ApiError>`; the dispatch arm answers the raw string with status 200)
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs` (`record_409` captured mid-walk; the seed pin's second partner)
- Test: `lib.rs` test module (403 spectator, 409 running, 200 bytes equal); `api.rs` route-table cell updated

**Interfaces:**
- Produces: `pub fn record(games: &Games, user: &User, id: &str) -> Result<String, ApiError>` returning the stored record JSON; `ApiResponse { status: 200, body: <the string> }`.

- [ ] **Step 1: Write the failing cells**

`lib.rs` test module (use `a_finished_two_person_game` at `:3633` and `start_locked_as`/the watcher helpers from the M6.3 cells):

```rust
#[tokio::test]
async fn the_record_route_is_409_while_running_403_for_a_watcher_and_the_stored_bytes_when_over() {
    let (app, state, id) = a_two_person_game_in_play().await;   // reuse the M6.1 helper that returns a playing game and its two identities
    let (status, body) = get_as(app.clone(), "red@wire.test", &format!("/api/games/{id}/record")).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "the record is available once the game is over");
    let (id, app, state) = a_finished_two_person_game().await;
    // a watcher of a finished game
    let (status, body) = get_as(app.clone(), "green@wire.test", &format!("/api/games/{id}/record")).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    // a seat holder gets the store's exact bytes
    let (status, raw) = raw(app, "GET", &format!("/api/games/{id}/record"), Some("red@wire.test")).await;
    assert_eq!(status, StatusCode::OK);
    let (stored, _) = state.games.store().read(yokohama_web::store::Doc::Record, &id).unwrap().unwrap();
    assert_eq!(raw, stored.as_bytes(), "the body is the stored record, not a re-serialization");
    assert!(stored.contains("\"terminal_digest\""), "anti-vacuity: the stored record carries its digest");
}
```

(Extend `raw` to take an optional identity; the finished-game helper must give a watcher — if `a_finished_two_person_game` has none, unlock as `green@wire.test` the way the M6.3 spectator cells do.)

`tests/wire_fixtures.rs`: in `walk()`, right after the first move is applied (the `at_k` capture is a good neighbour), add `("record_409".to_string(), api.get_status_and_body("GET", &format!("/api/games/{id}/record")).await)` — capture the 409 body the way the other 4xx fixtures are captured (`join_403_locked` at the M6.2 sites). Extend the seed pin:

```rust
    // The SECOND anti-vacuity partner (M6.5a, R-6.5a.14): the record route's 200
    // body DOES carry the seed — it is the record — and is never committed.
    let (id, _) = api.a_finished_game().await;               // the walk's own finished table, or a fresh bot-free one played out
    let (status, raw) = api.raw("GET", &format!("/api/games/{id}/record")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&raw).contains(&decimal), "the record carries the seed; the pin above would be searching for nothing");
    assert!(!std::path::Path::new("web/src/wire/record.json").exists(), "a record fixture would put the seed in the corpus");
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p yokohama-web --lib the_record_route` and `cargo test -p yokohama-web --test wire_fixtures`
Expected: FAIL — 404 from the dispatcher; the route-table cell fails on the missing entry.

- [ ] **Step 3: Implement**

```rust
pub fn record(games: &Games, user: &User, id: &str) -> Result<String, ApiError> {
    let (_table, viewer) = admitted(games, id, user)?;
    if viewer == crate::client::Viewer::Spectator {
        return Err(ApiError {
            status: http::StatusCode::FORBIDDEN,
            message: "the record is the players' — watch, download nothing".to_string(),
            table_status: None,
        });
    }
    let handle = games.handle(id)?;
    let over = handle.lock().expect("a game").session.is_over();
    if !over {
        return Err(ApiError {
            status: http::StatusCode::CONFLICT,
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
```

dispatch arm: `"/api/games/{id}/record" => record(games, u, b[0]).map(|json| ApiResponse { status: 200, body: json }),`. (`Viewer` needs `PartialEq` if it lacks it — derive it.) Update the route-table cell's expected list.

- [ ] **Step 4: Regenerate the corpus and read the diff**

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures && git status --short web/src/wire/`
Expected: exactly one new file, `web/src/wire/record_409.json`, and no modified file. Then the full gate: `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings && cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`.

- [ ] **Step 5: Commit**

```bash
git add crates/yokohama-web/src/api.rs crates/yokohama-web/src/client.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/tests/wire_fixtures.rs web/src/wire/record_409.json
git commit -m "feat(web): GET …/record — a seat holder's finished game as the store's exact bytes, 409 while running, 403 for a watcher; record_409 joins the corpus and the seed pin gains its second partner (R-6.5a.12, .14)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 10: The client — `recordText`, "Save game file", the Abandon label, `Health.archive`, the help sentence (R-6.5a.13)

**Files:**
- Modify: `web/src/api.ts` (`recordText`; `Health.archive` read at boot into a module getter `archiveKind()`), `web/src/types.ts:447-465` (`archive: "off" | "dir" | "r2"`), `web/src/wire/contract.ts` (the `HEALTH` root-key set gains `archive`)
- Modify: `web/src/App.tsx` (the finished-game header at `:1255`: a "Save game file" button beside "finished"), `web/src/Abandon.tsx` (the label), `web/src/Help.tsx` (one sentence in the lobby section)
- Test: `web/src/api.test.ts` (or the nearest existing api cell file), `web/src/App.test.tsx`, `web/src/Abandon.test.tsx`, `web/src/Help.test.tsx`, `web/src/wire/contract.test.ts`
- Modify: the e2e spec that plays the vs-bot game to a winner (grep `web/e2e` for `winner`): one assertion

**Interfaces:**
- Produces: `api.recordText(game: string): Promise<string>`; `archiveKind(): "off" | "dir" | "r2"`; `setArchiveKind`.

- [ ] **Step 1: Write the failing cells**

`web/src/App.test.tsx` (beside the game-over cells):

```tsx
it("offers Save game file only when the game is over, and hands the record text to the download untouched", async () => {
  const parse = vi.spyOn(JSON, "parse");
  const recordText = vi.spyOn(api, "recordText").mockResolvedValue('{"seed":18446744073709551615,"events":[]}');
  const create = vi.fn().mockReturnValue("blob:record");
  Object.defineProperty(URL, "createObjectURL", { value: create, configurable: true });
  Object.defineProperty(URL, "revokeObjectURL", { value: vi.fn(), configurable: true });
  renderGameAt(wire.gameOver);                       // the existing helper that mounts App on a fixture
  const button = await screen.findByRole("button", { name: /save game file/i });
  await userEvent.click(button);
  expect(recordText).toHaveBeenCalledWith(wire.gameOver.id);
  const blob = create.mock.calls[0][0] as Blob;
  expect(await blob.text()).toBe('{"seed":18446744073709551615,"events":[]}');
  expect(parse.mock.calls.every(([s]) => s !== '{"seed":18446744073709551615,"events":[]}')).toBe(true);
  renderGameAt(wire.clientState);                     // a running game
  expect(screen.queryByRole("button", { name: /save game file/i })).toBeNull();
});
```

`web/src/Abandon.test.tsx`:

```tsx
it("reads Abandon and save at a playing table when the archive is on, and Abandon otherwise", () => {
  setArchiveKind("dir");
  render(<Abandon table={wire.table} onDone={() => {}} onError={() => {}} />);   // wire.table is playing
  expect(screen.getByRole("button", { name: /abandon/i })).toHaveTextContent("Abandon and save");
  cleanup();
  setArchiveKind("off");
  render(<Abandon table={wire.table} onDone={() => {}} onError={() => {}} />);
  expect(screen.getByRole("button", { name: /abandon/i })).toHaveTextContent(/^Abandon$/);
  cleanup();
  setArchiveKind("dir");
  render(<Abandon table={wire.tableOpen} onDone={() => {}} onError={() => {}} />);
  expect(screen.getByRole("button", { name: /abandon/i })).toHaveTextContent(/^Abandon$/);
});
```

`web/src/wire/contract.test.ts`: the `health` root-key row gains `archive` (the existing row shape). `web/src/Help.test.tsx`: the help panel renders a sentence containing "abandon" and "saved" (match the wording you write in Step 3).

- [ ] **Step 2: Run them to verify they fail**

Run: `cd web && npm run typecheck; npm test -- App Abandon contract Help`
Expected: typecheck FAILS on `Health.archive` missing from `types.ts` against the regenerated `health.json`; the cells FAIL.

- [ ] **Step 3: Implement**

`types.ts`: `archive: "off" | "dir" | "r2";` with a doc line. `api.ts`:

```ts
/** Which archive this deployment writes finished and abandoned games to (M6.5a,
 *  R-6.5a.13). Read once from `/api/health` at boot; `"off"` until then. */
let archive: "off" | "dir" | "r2" = "off";
export const archiveKind = (): "off" | "dir" | "r2" => archive;
export const setArchiveKind = (k: "off" | "dir" | "r2"): void => { archive = k; };

/** The record as TEXT, never parsed (umbrella R-6.5.6): `GameRecord.seed` is a
 *  u64 and `JSON.parse` would round it. Same status handling as `call`. */
export async function recordText(game: string): Promise<string> {
  const res = await fetch(`/api/games/${id(game)}/record`, { headers: { ...identityHeaders() } });
  const text = await res.text();
  if (!res.ok) {
    let message = `HTTP ${res.status}`;
    try { const parsed = JSON.parse(text); if (parsed && typeof parsed === "object" && "error" in parsed) message = String(parsed.error); } catch { /* the sentence above stands */ }
    const e = new ApiError(res.status, message);
    if (res.status === 401) unauthorized?.();
    throw e;
  }
  return text;
}
```

(Match `call`'s `ApiError` constructor and `unauthorized` hook exactly as `api.ts:260-272` has them.) Add `recordText` to the `api` object. In `App.tsx` where `health` is read (`:561-563`): `setArchiveKind(h.archive)`. In the header at `:1255`, when `state.over`:

```tsx
<button
  type="button"
  className="ghost"
  onClick={() => void saveGameFile(state.id)}
>
  Save game file
</button>
```

with

```ts
async function saveGameFile(game: string): Promise<void> {
  const text = await api.recordText(game);
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = `${game}.json`;
  a.click();
  URL.revokeObjectURL(url);
}
```

(errors surface through the screen's existing error strip — wrap in the same `try`/`setError` the other actions use). `Abandon.tsx`: the first button's text becomes `{archiveKind() !== "off" && table.status === "playing" ? "Abandon and save" : "Abandon"}` (keep the `aria-label`). `Help.tsx`, lobby section: "Abandon deletes a table you opened; when this server keeps an archive, a game with moves in it is saved there first, and a finished game can be saved to a file from its own screen."

- [ ] **Step 4: Run the client gates and the e2e assertion**

Run: `cd web && npm run typecheck && npm test`
Expected: green, counts up by the cells added. Then add to the vs-bot winner spec:

```ts
const [download] = await Promise.all([
  page.waitForEvent("download"),
  page.getByRole("button", { name: "Save game file" }).click(),
]);
const path = await download.path();
const bytes = fs.readFileSync(path!);
const served = await page.request.get(`/api/games/${gameId}/record`, { headers: identity });
expect(bytes.equals(Buffer.from(await served.body()))).toBe(true);
```

Run: `cd web && npm run e2e` (needs the release binary and Chromium; `YOKOHAMA_WEB_E2E_OPTIONAL=1` is NOT set).
Expected: green, 39 → 39 (one assertion added inside an existing test).

- [ ] **Step 5: Commit**

```bash
git add web/src/api.ts web/src/types.ts web/src/wire/contract.ts web/src/wire/contract.test.ts web/src/App.tsx web/src/App.test.tsx web/src/Abandon.tsx web/src/Abandon.test.tsx web/src/Help.tsx web/src/Help.test.tsx web/e2e/<the spec>
git commit -m "feat(web client): Save game file reads the record as text and downloads it; Abandon says 'and save' when the archive is on; Health.archive in the contract (R-6.5a.13)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 11: Deployment and documentation of the variable (spec §4, §9)

**Files:**
- Modify: `deploy/Dockerfile.runtime:60-70` (the comment block names `YOKOHAMA_WEB_ARCHIVE_DIR`, default unset = off)
- Modify: `deploy/RUNBOOK.md` §3's create body (the `Env` line gains `YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive`) and a §3.12 placeholder heading "0.5.0-m6.5a — written at the deploy"
- Modify: `docs/cli-guide.md`/`.html`: NOT touched (no CLI change); `guide_sync` stays as is.

- [ ] **Step 1: Write the two documents' lines**

Dockerfile.runtime comment: "`YOKOHAMA_WEB_ARCHIVE_DIR` (M6.5a, R-6.5.4): unset, no archive; set, the directory where a finished or abandoned game's record and table document are written once, in the store's own layout under `rev<N>/over/` and `rev<N>/abandoned/`. On the NAS it is `/data/archive`, on the same volume as the store." RUNBOOK §3: the create body's `Env` becomes `["YOKOHAMA_WEB_IDENTITY=claimed", "YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive"]` with one sentence saying why.

- [ ] **Step 2: Gate and commit**

Run: `cargo test -p yokohama-cli --test guide_sync` (unchanged; proves no CLI doc drift) and `cargo test -p yokohama-web`.

```bash
git add deploy/Dockerfile.runtime deploy/RUNBOOK.md
git commit -m "docs(deploy): YOKOHAMA_WEB_ARCHIVE_DIR on the NAS create body and in the image's variable notes (M6.5a)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

## Plan-level rulings

- **R-plan-1 — task order is dependency order**: 1 (signal) → 2 (clock) → 3 (chat on Games) → 4 (feature + fence) → 5 (api.rs + public routes) → 6 (every route) → 7 (archive module) → 8 (archive sites) → 9 (record route) → 10 (client) → 11 (deploy docs). 7–9 could follow 4 directly, but the route table must exist before the record route, so 9 stays after 6.
- **R-plan-2 — the corpus is regenerated at Tasks 5, 6, 7 and 9 and the diff read each time**; the expected diffs are: none, none, `health.json` +1 key, `record_409.json` new. Any other diff is a defect in the task, never a fixture to accept.
- **R-plan-3 — reproduce, do not re-word.** Where a sentence in this plan differs from axum's actual text at the pinned version, the pin cell decides and the plan's string is corrected in the commit that lands it.
- **R-plan-4 — `Games`' signatures are frozen for this cycle**; a task that needs one to change stops and reports (R-6.5a.1).
- **R-plan-5 — the wasm target check is a gate from Task 4 on**, run locally by every task after it and in CI.
- **R-plan-6 — the whole-game gate's vs-bot arm may livelock on an OS-entropy seed** (M6.3 close §6.3, item 1); a red there is re-run alone once before it is believed, as the M6.4 counts did.

## Close checklist (the orchestrator's, after Task 11)

1. Whole-branch review (Fable) BEFORE the release (R-order-1), one fix wave, one scoped re-review.
2. The mutation battery of spec §6.3 (fifteen), each recorded with the catcher that fired.
3. Counts in ONE sequence: `cargo test -p yokohama-web` (three binaries), clippy, the wasm check, `cargo test -p yokohama-cli` (unchanged 851 + 2), `cd web && npm run typecheck && npm test && npm run e2e`, the corpus count (83), the five-crate and CLI diffs by command.
4. Deploy `yokohama-web:0.5.0-m6.5a` on the NAS with the archive directory; abandon a throwaway table with events and read its two files off the volume; play one game to the end and read its two files; runbook §3.12.
5. The close document and the M6.5b handover; CLAUDE.md's counts and the Web bullets; guidingpath; the board.

## Self-review

- **Spec coverage:** R-6.5a.1 → Tasks 1–3, 5–6; .2 → 4; .3 → 5–6; .4 → 5 (+ the path pin in 6); .5 → 4; .6 → 2 and 4; .7 → 4; .8 → 1; .9 → 7; .10 → 7; .11 → 8; .12 → 9; .13 → 10 (health half in 7); .14 → 6, 7, 9; .15 → the global constraints and the close checklist; .16 → the close checklist. Spec §6.3's mutations 1–3 → Task 4's cell and the check line; 4 → Task 6; 5 → Task 5; 6–7 → Task 9; 8–11, 13 → Tasks 7–8; 12 → Tasks 2 and 4; 14–15 → Tasks 5–6.
- **Placeholders:** the two handler-body moves (Task 6) are specified by line range and rewrite rule rather than repeated code, deliberately, since the code exists at HEAD and the rule is mechanical; the axum rejection strings are marked to be matched against the registry source, with the pin as the arbiter; helper names in Task 8's cells (`a_vs_bot_game_played_to_the_end`, `a_two_person_game_with_one_move`, `an_open_table`, `creator_of`) name the shapes the existing `test_support` and the M6.1/M6.2 cells already build — the implementer reuses those, adding a thin wrapper only if none matches.
- **Type consistency:** `Signal`'s four methods (Task 1) are what `games.rs`/`tables.rs` call in Tasks 1, 8; `Clock` (Task 2) is what Tasks 7–8 read as `(self.clock)()`; `Games::with_parts(rules, store, chat, signal, clock, archive)` ends with six parameters by Task 7 and `with_limits` fills the last three with `NoSignal`, `|| 0`, `NoArchive`; `api::dispatch`'s `Identity<'_>` (Task 5) is what Task 6's cells and the adapter pass; `ApiResponse.status: u16` everywhere; `Health.archive: &'static str` (Rust) ↔ `"off" | "dir" | "r2"` (TS) with `"dir"`/`"off"` the only values this cycle produces.
