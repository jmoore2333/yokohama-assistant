# Web frontend — M6.1 "identity and seats": implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give every request an identity, bind every seat to a person, build every payload for the person asking, and let two people at two browsers play each other on the NAS — with the deal served per seat, the acting-seat fence on the one mutation, the undo rule, and a polled change signal the later transport cycle replaces without touching anything downstream of it.

**Architecture:** Server-first, then client. One `Identity` extractor over one configured mode; the store trait widened to typed documents with a conditional write; a `table` document beside each record holding who sits where and the deal in progress; every game route resolving the caller to a seat and passing SERVER-computed `SeatFacts` into the existing `build_frame`, so masking is the mechanism M5.5 already has; a tiny `events` endpoint the client polls from a visible tab only. `crates/yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge` and `gen-data` are untouched; `crates/yokohama-cli` gains one `pub` (the keep hedge).

**Tech Stack:** axum 0.8 + tokio (server), serde (documents), React 19 + TypeScript + Vite (client), vitest + Testing Library over jsdom (component tests), Playwright against the real release binary (browser walk), the Portainer Docker proxy (deploy).

**Spec:** `docs/plans/2026-09-14-web-frontend-m6.1-identity-and-seats-spec.md` — the authority; this plan argues from it and records where it deviates (§ "Deviations" at the end). The cycle plan above it is `docs/plans/2026-09-14-web-frontend-m6-room-plan.md`, whose §0 carries the owner's rulings R-P1–R-P11.

## Global Constraints

- **Branch:** `web-room`, in the worktree `.claude/worktrees/web-room`. Every command runs from that worktree root unless it says `cd web`. Stage with explicit paths (`git add <path>`), never `git add -A`. Commit messages end with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and the EXECUTING session's `Claude-Session:` trailer (never the planning session's).
- **Blast radius (spec §9.5):** at the close, `git diff master -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data` is EMPTY and `SEMANTICS_REV` is **7**. `crates/yokohama-cli` moves by ONE line (Task 3: `KEEP_SCORE_NOTE` becomes `pub`) and `cargo test -p yokohama-cli` stays green with its goldens and `guide_sync` untouched.
- **Per-edit gate (after EVERY task):** `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings` when `crates/` moved; `cd web && npm run typecheck && npm test` when `web/` moved. **The browser walk (`cd web && npm run e2e`) is RED BY DESIGN from Task 3 through Task 8** — the server's create flow changes at Task 3 and the client learns it at Task 8; the tasks say which. It is green again at Task 8 and is part of the gate for Tasks 8–12.
- **The wire corpus is a contract.** Any task that changes a payload runs `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, then `cargo test -p yokohama-web --test wire_fixtures` WITHOUT the variable, then `cd web && npm run typecheck`, and its report names every fixture file whose diff changed and why. Three tasks regenerate it: 1, 3, 4 (and 10 removes files).
- **No hand-authored payload in a test** (spec §9.2). A component cell renders a fixture from `web/src/wire/` or nothing.
- **Identity in every server test**: the test helpers send `X-Yokohama-Claimed-User: red@wire.test` by default (Task 1); a cell about identity sets its own headers.
- **Every mode reads exactly one header** (spec R-1.2); **no default mode** (R-1.3); **the NAS runs `claimed`** (R-1.4); **not seated is 403** (R-1.5); **the creator holds a seat** (R-1.6); **a hand is shown to its own seat only** (R-1.7); **`suggest` for the acting seat's holder only** (R-1.8); **health names the mode** (R-1.9); **a bot at two players only** (R-P10).
- **Counts at HEAD `fec9bee`** (re-measure at Task 0 and again at Task 12, never copy): `cargo test -p yokohama-web` **82** (76 lib + 1 gate + 5 wire_fixtures), `cd web && npm test` **393** in 24 files, `npm run e2e` **37** (+2 opt-in), wire corpus **44** files.

---

## Execution model

| Seat | Model | Tasks |
|---|---|---|
| Implementer | **Opus** (xhigh) | 0–11 |
| Per-task reviewer | **Opus**, fresh context | every task |
| Orchestrator, adjudication | **Fable** | all |
| Deploy (outward-facing, owner-authorized) | the orchestrator through the Portainer MCP, as M5.5's Task 13 was | 11 |
| The final review against the LIVE version | **Fable**, a CLEAN subagent with the brief in Task 12 | 12 |

No Sonnet or Haiku dispatch (R-P11). The SDD workspace is `.superpowers/sdd/2026-09-14-web-frontend-m6.1-plan/progress.md` in the shape the M5.5 wave used (pre-flight scan, one ruling row per decision, one report and one review per task); `git status --short` must not list it.

**Pre-flight (the orchestrator, before Task 0):** `git status` clean at `fec9bee` or later on `web-room`; `cd web && npm ci` done; `npx playwright install chromium` done; `cargo build --release -p yokohama-web` warm. Record the executing session's `Claude-Session` URL for every commit trailer.

---

## File structure

| File | Responsibility | Task |
|---|---|---|
| `crates/yokohama-web/src/identity.rs` (new) | `IdentityMode`, `User`, `Identity` extractor, header names, normalization | 1 |
| `crates/yokohama-web/src/lib.rs`, `main.rs`, `tests/wire_fixtures.rs`, `tests/play_a_whole_game.rs`, `web/src/types.ts`, `web/src/api.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`, `web/e2e/harness.ts` | `AppState.identity`, `/api/me`, `health.identity`, the 401 on `/api/games/*`, test helpers with the header, the client's header from storage | 1 |
| `crates/yokohama-web/src/store.rs`, `games.rs` | `Doc`, `Version`, `WriteError`, the `Store` trait, `persist` re-expressed | 2 |
| `crates/yokohama-web/src/table.rs` (new), `games.rs`, `lib.rs`, `crates/yokohama-cli/src/wizard.rs` (one `pub`), `tests/wire_fixtures.rs`, `tests/play_a_whole_game.rs`, `web/src/types.ts`, `web/src/api.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts` | The table document; `NewGame` with `seats`; create → dealing; `table`/`deal`/`keep`; finalize and settle; list scoped; delete = creator; 409 while dealing | 3 |
| `crates/yokohama-web/src/client.rs`, `lib.rs`, `tests/wire_fixtures.rs` | `viewer_seat`, server-computed `SeatFacts`, every route seated, the acting-seat fence, `suggest`/`why` for the caller | 4 |
| `crates/yokohama-web/src/grammar.rs`, `lib.rs` | `privacy` out; the undo rule | 5 |
| `crates/yokohama-web/src/lib.rs`, `tests/wire_fixtures.rs`, `web/src/types.ts`, `web/src/api.ts`, `web/src/wire/contract*.ts` | `GET …/events` | 6 |
| `web/src/SignIn.tsx` (+ `.test.tsx`, new), `web/src/api.ts`, `web/src/App.tsx` (+ `.test.tsx`), `web/src/GameMenu.tsx` (+ `.test.tsx`), `web/src/styles.css` | Sign-in, the "You are" strip, the games list as tables | 7 |
| `web/src/NewGame.tsx` (+ `.test.tsx`), `web/src/Dealing.tsx` (+ `.test.tsx`, new), `web/src/App.tsx`, `web/e2e/harness.ts`, `web/src/styles.css` | The per-seat form, the dealing screen, the harness through the new flow | 8 |
| `web/src/signal.ts` (+ `.test.ts`, new), `web/src/App.tsx` (+ `.test.tsx`), `web/src/Help.tsx` (+ `.test.tsx`), `web/src/styles.css` | The visible-tab poll and the walk from it; the help prose | 9 |
| `crates/yokohama-web/src/games.rs`, `lib.rs`, `tests/wire_fixtures.rs`, `tests/play_a_whole_game.rs`, `web/src/types.ts`, `web/src/api.ts`, `web/src/wire/contract*.ts`, `web/e2e/two_browsers.spec.ts` (new), `web/e2e/harness.ts` | `preview` removed; the seed pin without its exemption; the two-identity gate arm; the two-browser walk | 10 |
| `deploy/RUNBOOK.md` (§3.7) | The fourth deployment, `yokohama-web:0.5.0-m6.1` | 11 |
| `docs/plans/2026-09-14-web-frontend-m6.1-close.md` (new), `CLAUDE.md`, `guidingpath.md`, `README.md`, `docs/handover.md` | The final review and the close | 12 |

---

## Plan-level rulings (deviations from the spec's letter, recorded here per R-P11)

- **R-plan-1 — `created_at` is unix seconds (`u64`), not an RFC 3339 string.** The workspace has no date dependency and hand-formatting a calendar date is the kind of arithmetic that ships wrong; the client formats with `Date`. The spec's §4.1 example shows a string; the field's type is this plan's.
- **R-plan-2 — the claimed header is attached whenever the browser holds a stored user, and a user is stored only by the sign-in screen, which exists only in `claimed` mode.** The spec's §7.1 "only when `/api/me` said the mode is `claimed`" is satisfied by construction; a value left in storage from an earlier claimed deployment is ignored by the server under any other mode (R-1.2, pinned).
- **R-plan-3 — the table document mirrors `to_move` and `over` for the LIST.** The spec's §4.4 list carries whose turn it is; computing that needs the state, and replaying every game on every list call is not acceptable. `persist` writes the mirror beside the record; the session stays the truth and no play route reads the mirror.
- **R-plan-4 — `preview` survives until Task 10.** The client's set table needs it until Task 8 rewrites the flow; removing it earlier would delete client cells with nothing to replace them. Its body follows the new `NewGame` shape from Task 3.
- **R-plan-5 — `hidden_hand.json` retires at Task 4 with the fixture that produced it.** Under per-seat views the VIEWER's own hand is never hidden, so the branch that fixture rendered (`Dashboard.test.tsx:225-227`) becomes "another seat shows as a count", rendered from `options.json`, which is now built for a named seat.

---

### Task 0: Pre-flight and baseline

**Files:**
- Create: `.superpowers/sdd/2026-09-14-web-frontend-m6.1-plan/progress.md` (gitignored)

- [ ] **Step 1: Confirm the worktree and the branch**

Run: `git status --short && git branch --show-current && git log --oneline -1`
Expected: no output from status; `web-room`; `fec9bee` or later.

- [ ] **Step 2: Re-count the baselines**

Run: `cargo test -p yokohama-web 2>&1 | grep 'test result'`
Expected: three lines — `76 passed`, `1 passed`, `5 passed`.

Run: `cd web && npm run typecheck && npm test 2>&1 | grep -E 'Test Files|Tests '`
Expected: `24 passed`, `393 passed`.

Run: `ls web/src/wire/*.json | wc -l`
Expected: `44`.

- [ ] **Step 3: Open the SDD ledger**

Write `progress.md` with: the plan path, the branch, the base commit, the three counts above, and an empty "Rulings" table with columns `#`, `task`, `question`, `ruling`, `cost`. Confirm `git status --short` still prints nothing.

---

### Task 1: The identity extractor, `/api/me`, and the 401

**Files:**
- Create: `crates/yokohama-web/src/identity.rs`
- Modify: `crates/yokohama-web/src/lib.rs` (`AppState`, `Health`, router, test helpers), `crates/yokohama-web/src/main.rs`, `crates/yokohama-web/tests/wire_fixtures.rs` (`Api::call`, the corpus list), `crates/yokohama-web/tests/play_a_whole_game.rs` (`Api::call`), `web/src/types.ts`, `web/src/api.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`, `web/e2e/harness.ts`, `web/playwright.config.ts`
- Test: `crates/yokohama-web/src/identity.rs` (unit), `crates/yokohama-web/src/lib.rs` (route cells)

**Interfaces:**
- Produces: `identity::IdentityMode { Claimed, CfAccess, Worker }` with `from_env() -> Result<Self, String>` and `name(&self) -> &'static str`; `identity::User(String)` with `User::parse(&str) -> Result<User, String>`, `as_str()`, `Display`; `identity::resolve(mode, &HeaderMap) -> Result<User, String>`; `identity::Identity(pub User)` implementing `FromRequestParts<Arc<AppState>>` with `Rejection = ApiError` (401); `AppState { rules, games, identity: IdentityMode }`, `AppState::new(mode)`, `AppState::in_memory()` (Claimed), `AppState::in_memory_with(mode)`; `GET /api/me` → `Me { mode: &'static str, user: Option<String> }`; `Health.identity: &'static str`.
- Client: `types.ts` `Me { mode: "claimed" | "cf-access" | "worker"; user: string | null }`, `Health.identity: string`; `api.me()`; `api.ts` exports `CLAIMED_USER_KEY = "yokohama.claimed_user"` and attaches `X-Yokohama-Claimed-User` from `localStorage` when present (R-plan-2).

- [ ] **Step 1: Write the failing unit tests for normalization and resolution**

In a new `crates/yokohama-web/src/identity.rs`, at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                axum::http::HeaderName::from_bytes(k.as_bytes()).expect("a header name"),
                v.parse().expect("a header value"),
            );
        }
        h
    }

    #[test]
    fn an_email_is_normalized_once_and_the_shapes_that_are_not_one_are_refused() {
        assert_eq!(User::parse("  A@Example.COM ").expect("an email").as_str(), "a@example.com");
        for bad in ["", "nobody", "two@@signs", "a@b c", "@example.com", "a@", "a\u{7}@b.c"] {
            assert!(User::parse(bad).is_err(), "{bad:?} must be refused");
        }
        let long = format!("{}@x.y", "a".repeat(252));
        assert!(User::parse(&long).is_err(), "255 bytes is one too many");
    }

    /// R-1.2: a request carrying all three headers with three different values
    /// resolves to the MODE's own, under each mode. A mutation that reads any
    /// header regardless of mode fails all three arms at once.
    #[test]
    fn every_mode_reads_its_own_header_and_only_that_one() {
        let all = headers(&[
            (CLAIMED_HEADER, "claimed@t.test"),
            (CF_ACCESS_HEADER, "access@t.test"),
            (WORKER_HEADER, "worker@t.test"),
        ]);
        assert_eq!(resolve(IdentityMode::Claimed, &all).unwrap().as_str(), "claimed@t.test");
        assert_eq!(resolve(IdentityMode::CfAccess, &all).unwrap().as_str(), "access@t.test");
        assert_eq!(resolve(IdentityMode::Worker, &all).unwrap().as_str(), "worker@t.test");
    }

    #[test]
    fn a_missing_header_is_absent_and_the_workers_anonymous_counts_as_absent() {
        assert!(resolve(IdentityMode::Claimed, &headers(&[])).is_err());
        assert!(resolve(IdentityMode::CfAccess, &headers(&[(CLAIMED_HEADER, "x@y.z")])).is_err(),
            "a forged claimed header on the tunnel path changes nothing");
        assert!(resolve(IdentityMode::Worker, &headers(&[(WORKER_HEADER, "anonymous")])).is_err());
    }

    #[test]
    fn the_mode_comes_from_the_environment_and_has_no_default() {
        assert!(matches!(IdentityMode::from_str("claimed"), Ok(IdentityMode::Claimed)));
        assert!(matches!(IdentityMode::from_str("cf-access"), Ok(IdentityMode::CfAccess)));
        assert!(matches!(IdentityMode::from_str("worker"), Ok(IdentityMode::Worker)));
        let err = IdentityMode::from_str("").unwrap_err();
        assert!(err.contains("claimed") && err.contains("cf-access") && err.contains("worker"), "{err}");
        assert!(IdentityMode::from_str("neon").is_err(), "reserved, not built");
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p yokohama-web identity 2>&1 | tail -5`
Expected: compile error — `identity` is not a module.

- [ ] **Step 3: Write `identity.rs`**

```rust
//! Who a request is from — one configured source, one header, one normalized email.
//!
//! The room's first seam (`docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §1).
//! Every mode reads EXACTLY ONE header and ignores every other identity header
//! (spec R-1.2), so a forged header on a path that does not read it changes
//! nothing. There is no default mode (R-1.3): `main.rs` refuses to start
//! without one, the Worker's own fail-closed shape for its secrets.
//!
//! `claimed` is a development convenience and not a security boundary (R-1.4):
//! anyone past the door can claim any email. The runbook says so beside the
//! mode. Reserved and unbuilt: `cf-access-jwt` and `neon`, each a new arm here.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};

use crate::{ApiError, AppState};

pub const CLAIMED_HEADER: &str = "x-yokohama-claimed-user";
pub const CF_ACCESS_HEADER: &str = "cf-access-authenticated-user-email";
pub const WORKER_HEADER: &str = "x-yokohama-user";
pub const ENV_VAR: &str = "YOKOHAMA_WEB_IDENTITY";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityMode {
    Claimed,
    CfAccess,
    Worker,
}

impl IdentityMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Claimed => "claimed",
            Self::CfAccess => "cf-access",
            Self::Worker => "worker",
        }
    }

    pub fn header(self) -> &'static str {
        match self {
            Self::Claimed => CLAIMED_HEADER,
            Self::CfAccess => CF_ACCESS_HEADER,
            Self::Worker => WORKER_HEADER,
        }
    }

    pub fn from_env() -> Result<Self, String> {
        match std::env::var(ENV_VAR) {
            Ok(v) => Self::from_str(&v),
            Err(_) => Self::from_str(""),
        }
    }
}

impl FromStr for IdentityMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s.trim() {
            "claimed" => Ok(Self::Claimed),
            "cf-access" => Ok(Self::CfAccess),
            "worker" => Ok(Self::Worker),
            other => Err(format!(
                "{ENV_VAR} must be one of claimed, cf-access, worker — got {other:?}. \
                 There is no default: an identity source is a deployment decision."
            )),
        }
    }
}

/// A normalized email: trimmed, lowercased, exactly one `@` with both sides
/// non-empty, no whitespace or control characters, at most 254 bytes (R-1.1).
/// A shape check, not RFC validation. Normalized HERE and nowhere else — this
/// is the key every seat binding is written under.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct User(String);

impl User {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let s = raw.trim().to_ascii_lowercase();
        if s.is_empty() {
            return Err("no email given".to_string());
        }
        if s.len() > 254 {
            return Err("an email is at most 254 bytes".to_string());
        }
        if s.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(format!("{raw:?} contains whitespace or a control character"));
        }
        let mut parts = s.split('@');
        let (local, domain, rest) = (parts.next(), parts.next(), parts.next());
        match (local, domain, rest) {
            (Some(l), Some(d), None) if !l.is_empty() && !d.is_empty() => Ok(Self(s)),
            _ => Err(format!("{raw:?} is not an email: exactly one @ with something on both sides")),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The mode's ONE header, parsed; absent or invalid is `Err` with the sentence
/// the 401 carries. The Worker's `anonymous` — what it sends with no Access
/// policy in front of it — counts as absent.
pub fn resolve(mode: IdentityMode, headers: &HeaderMap) -> Result<User, String> {
    let raw = headers
        .get(mode.header())
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty() && !(mode == IdentityMode::Worker && *s == "anonymous"));
    match raw {
        Some(s) => User::parse(s),
        None => Err(match mode {
            IdentityMode::Claimed => "no signed-in user: send X-Yokohama-Claimed-User".to_string(),
            IdentityMode::CfAccess => "no identity reached the server: this deployment expects \
                                       Cf-Access-Authenticated-User-Email from the Cloudflare tunnel"
                .to_string(),
            IdentityMode::Worker => "no identity reached the server: this deployment expects \
                                     X-Yokohama-User from the edge Worker"
                .to_string(),
        }),
    }
}

/// The extractor every `/api/games` route takes. A missing identity is a 401 in
/// this API's `{"error": …}` shape.
pub struct Identity(pub User);

impl FromRequestParts<Arc<AppState>> for Identity {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, ApiError> {
        resolve(state.identity, &parts.headers)
            .map(Identity)
            .map_err(|message| ApiError { status: StatusCode::UNAUTHORIZED, message })
    }
}
```

In `lib.rs`: add `pub mod identity;`, and change `AppState`:

```rust
pub struct AppState {
    pub rules: Arc<Rules>,
    pub games: Games,
    /// Which header names the caller (R-1.2). Read once at start; no default.
    pub identity: identity::IdentityMode,
}

impl AppState {
    pub fn new(identity: identity::IdentityMode) -> Self {
        Self::with_store(Arc::new(FsStore::new(store::default_dir())), identity)
    }
    /// Backed by memory, in `claimed` mode — the mode every test drives with a header.
    pub fn in_memory() -> Self {
        Self::in_memory_with(identity::IdentityMode::Claimed)
    }
    pub fn in_memory_with(identity: identity::IdentityMode) -> Self {
        Self::with_store(Arc::new(MemStore::default()), identity)
    }
    pub fn with_store(store: Arc<dyn SessionStore>, identity: identity::IdentityMode) -> Self {
        let rules = Arc::new(Rules::standard());
        Self { rules: Arc::clone(&rules), games: Games::new(rules, store), identity }
    }
}
```

Delete `impl Default for AppState` (a default would be a default mode). In `main.rs`, before `AppState::new`:

```rust
    let identity = match yokohama_web::identity::IdentityMode::from_env() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let state = Arc::new(AppState::new(identity));
```

- [ ] **Step 4: Run the unit tests**

Run: `cargo test -p yokohama-web identity 2>&1 | grep 'test result'`
Expected: `4 passed`.

- [ ] **Step 5: Write the failing route cells for the 401, `/api/me` and health**

In `lib.rs`'s `mod tests`, first change the helpers so every request carries the default identity — `get_json`, `post_json` and `raw_post` build their `Request` with `.header(identity::CLAIMED_HEADER, "red@wire.test")` — and add a raw variant that takes explicit headers:

```rust
    /// A request under headers the CELL chooses — for the identity cells only.
    async fn get_with(state: Arc<AppState>, uri: &str, headers: &[(&str, &str)]) -> (StatusCode, serde_json::Value) {
        let mut req = Request::builder().uri(uri);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let res = app(state)
            .oneshot(req.body(Body::empty()).expect("a GET"))
            .await
            .expect("the router answers");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
    }

    #[tokio::test]
    async fn every_game_route_is_401_without_an_identity_and_the_public_ones_are_not() {
        use identity::{IdentityMode as M, CLAIMED_HEADER};
        for uri in ["/api/games", "/api/games/nope", "/api/games/nope/log"] {
            let (status, body) = get_with(Arc::new(AppState::in_memory()), uri, &[]).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}: {body}");
            assert!(body["error"].as_str().unwrap_or("").contains("X-Yokohama-Claimed-User"), "{body}");
        }
        for uri in ["/api/health", "/api/labels", "/api/me"] {
            let (status, _) = get_with(Arc::new(AppState::in_memory()), uri, &[]).await;
            assert_ne!(status, StatusCode::UNAUTHORIZED, "{uri} is public");
        }
        // The mode reads its own header: under cf-access the claimed header is inert.
        let (status, _) = get_with(
            Arc::new(AppState::in_memory_with(M::CfAccess)),
            "/api/games",
            &[(CLAIMED_HEADER, "forged@t.test")],
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn me_answers_without_identity_and_names_the_mode() {
        use identity::{IdentityMode as M, CLAIMED_HEADER, CF_ACCESS_HEADER};
        let (status, body) = get_with(Arc::new(AppState::in_memory()), "/api/me", &[]).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, serde_json::json!({ "mode": "claimed", "user": null }));
        let (_, body) = get_with(Arc::new(AppState::in_memory()), "/api/me", &[(CLAIMED_HEADER, " Red@Wire.TEST ")]).await;
        assert_eq!(body["user"], "red@wire.test", "normalized once, in the extractor");
        let (_, body) = get_with(Arc::new(AppState::in_memory()), "/api/me", &[(CLAIMED_HEADER, "not-an-email")]).await;
        assert_eq!(body["user"], serde_json::Value::Null, "an invalid header resolves to null here, never 401");
        let (_, body) = get_with(Arc::new(AppState::in_memory_with(M::CfAccess)), "/api/me", &[(CF_ACCESS_HEADER, "a@b.c")]).await;
        assert_eq!(body, serde_json::json!({ "mode": "cf-access", "user": "a@b.c" }));
    }
```

And extend `health_reports_the_engines_own_compatibility_keys` with `assert_eq!(body["identity"], "claimed");`.

- [ ] **Step 6: Run them to see them fail**

Run: `cargo test -p yokohama-web --lib 2>&1 | grep -E 'FAILED|panicked|error\[' | head`
Expected: the three new cells fail (401 expected, 200/404 received; `/api/me` is a 404; `identity` absent from health).

- [ ] **Step 7: Wire the routes**

In `lib.rs`: `Health` gains `pub identity: &'static str` (set from `state.identity.name()`); add

```rust
/// What `/api/me` answers. Never a 401: null is the answer that tells the client
/// which screen to show (spec §2.3).
#[derive(Debug, Clone, Serialize)]
pub struct Me {
    pub mode: &'static str,
    pub user: Option<String>,
}

async fn me(State(state): State<Arc<AppState>>, headers: axum::http::HeaderMap) -> Json<Me> {
    Json(Me {
        mode: state.identity.name(),
        user: identity::resolve(state.identity, &headers).ok().map(|u| u.as_str().to_string()),
    })
}
```

route `.route("/api/me", get(me))`, and add `_who: identity::Identity` as the FIRST extractor parameter of every `/api/games` handler (`list_games`, `create_game`, `preview_game`, `get_game`, `delete_game`, `input`, `ports`, `labs`, `areas` via the `surface!` macro, `log`, `at_k`, `suggest`, `why`, `card_index`, `result`). It is unused in this task — the seat scoping comes in Task 4 — so name it `_who` and say so in one comment above `list_games`.

- [ ] **Step 8: Send the header from the two integration harnesses and the client**

`tests/wire_fixtures.rs` and `tests/play_a_whole_game.rs`: in `Api::call`, add `.header("x-yokohama-claimed-user", "red@wire.test")` to the builder. In `wire_fixtures.rs`'s `walk()`, append two captures after `health`: `("me".to_string(), api.get("/api/me").await)` and, built with `Api::call` under NO header, `("me_signed_out".to_string(), …)` — add an `Api::get_anonymous(&self, uri)` that omits the header.

`web/src/api.ts`: 

```ts
/** Where the sign-in screen keeps the claimed user (R-plan-2). Read on every
 *  call; absent means no header. A value is only ever WRITTEN by the sign-in
 *  screen, which only exists in `claimed` mode, and the server ignores this
 *  header under any other mode (spec R-1.2). */
export const CLAIMED_USER_KEY = "yokohama.claimed_user";

export function claimedUser(): string | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage.getItem(CLAIMED_USER_KEY);
  } catch {
    return null;
  }
}
```

and in `call()`:

```ts
  const headers: Record<string, string> = {};
  if (body !== undefined) headers["content-type"] = "application/json";
  const who = claimedUser();
  if (who) headers["x-yokohama-claimed-user"] = who;
  const res = await fetch(path, { method, headers, body: body === undefined ? undefined : JSON.stringify(body) });
```

plus `me: () => call<Me>("GET", "/api/me")`. `types.ts`: `Me` and `Health.identity`. `contract.ts`: import `me.json`/`me_signed_out.json`, `_me: Widen<Me> = me`, `_meOut: Widen<Me> = meSignedOut`, `KEYS.me = ["mode", "user"]`, `HEALTH` gains `"identity"`, `wire.me`, `wire.meSignedOut`. `contract.test.ts`: two rows `["me", wire.me, KEYS.me]`, `["me_signed_out", wire.meSignedOut, KEYS.me]`.

`web/playwright.config.ts`: the `webServer.env` gains `YOKOHAMA_WEB_IDENTITY: "claimed"`. `web/e2e/harness.ts` `startGame`: before `page.goto("/")`, `await page.addInitScript(() => localStorage.setItem("yokohama.claimed_user", "walker@e2e.test"));` — a stand-in until Task 8 signs in through the screen.

- [ ] **Step 9: Regenerate the corpus and run every gate**

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures 2>&1 | tail -3 && cargo test -p yokohama-web 2>&1 | grep 'test result' && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -2`
Expected: `health.json` gains `"identity": "claimed"`; `me.json` and `me_signed_out.json` new; nothing else moves (report the diff); `79 passed` lib (76 + 3), `1 passed`, `5 passed`; clippy clean.

Run: `cd web && npm run typecheck && npm test 2>&1 | grep -E 'Tests ' && npm run e2e 2>&1 | tail -4`
Expected: typecheck clean; 393 + 2 = **395** passed (the two contract rows); the browser walk green (37).

- [ ] **Step 10: Commit**

```bash
git add crates/yokohama-web/src/identity.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/main.rs crates/yokohama-web/tests/wire_fixtures.rs crates/yokohama-web/tests/play_a_whole_game.rs web/src/types.ts web/src/api.ts web/src/wire/contract.ts web/src/wire/contract.test.ts web/src/wire/health.json web/src/wire/me.json web/src/wire/me_signed_out.json web/e2e/harness.ts web/playwright.config.ts
git commit -m "feat(web): identity — one configured mode reading one header, a 401 on every game route, /api/me, and health naming the mode"
```

---

### Task 2: The store seam — typed documents and a conditional write

**Files:**
- Modify: `crates/yokohama-web/src/store.rs`, `crates/yokohama-web/src/games.rs` (`Games.written`, `create`, `handle`, `persist`, `delete`), `crates/yokohama-web/src/lib.rs` (`with_store`'s type)
- Test: `crates/yokohama-web/src/store.rs`, `crates/yokohama-web/src/games.rs`

**Interfaces:**
- Produces: `store::Doc { Record, Table }`; `store::Version(u64)` (`Copy`, `Eq`); `store::WriteError { Conflict { current: Option<Version> }, Io(String) }` with `Display`; `trait Store: Send + Sync { fn list(&self, doc: Doc) -> Result<Vec<GameId>, String>; fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String>; fn write_if_version(&self, doc: Doc, id: &str, json: &str, expected: Option<Version>) -> Result<Version, WriteError>; fn delete(&self, id: &str) -> Result<(), String>; }`; `store::version_of(json: &str) -> Version` (the digest, `pub`); `FsStore` and `MemStore` implementing it; `SessionStore` is GONE.
- `Games::persist` unchanged in signature; `Games::written: Mutex<HashMap<GameId, Version>>`.

- [ ] **Step 1: Write the failing store cells**

Replace the store's test module with cells over BOTH implementations (a `fn both() -> Vec<(&'static str, Arc<dyn Store>)>` returning a `MemStore` and an `FsStore` over a `tempdir` made with `std::env::temp_dir().join(format!("yokohama-store-{}-{}", std::process::id(), name))`, removed at the end):

```rust
    #[test]
    fn a_create_refuses_a_present_document_and_a_stale_version_conflicts() {
        for (name, store) in both("cas") {
            let v1 = store.write_if_version(Doc::Record, "g", "{\"a\":1}", None).expect(name);
            match store.write_if_version(Doc::Record, "g", "{\"a\":2}", None) {
                Err(WriteError::Conflict { current: Some(c) }) => assert_eq!(c, v1, "{name}"),
                other => panic!("{name}: a second create must conflict, got {other:?}"),
            }
            let v2 = store.write_if_version(Doc::Record, "g", "{\"a\":2}", Some(v1)).expect(name);
            assert_ne!(v1, v2);
            match store.write_if_version(Doc::Record, "g", "{\"a\":3}", Some(v1)) {
                Err(WriteError::Conflict { current: Some(c) }) => assert_eq!(c, v2, "{name}: names the current"),
                other => panic!("{name}: a stale version must conflict, got {other:?}"),
            }
            let (json, v) = store.read(Doc::Record, "g").expect(name).expect("present");
            assert_eq!((json.as_str(), v), ("{\"a\":2}", v2));
        }
    }

    #[test]
    fn the_two_kinds_are_separate_documents_under_one_id() {
        for (name, store) in both("kinds") {
            store.write_if_version(Doc::Record, "g", "{\"r\":1}", None).expect(name);
            assert!(store.read(Doc::Table, "g").expect(name).is_none(), "{name}");
            store.write_if_version(Doc::Table, "g", "{\"t\":1}", None).expect(name);
            assert_eq!(store.list(Doc::Table).expect(name), vec!["g".to_string()]);
            store.write_if_version(Doc::Record, "h", "{\"r\":2}", None).expect(name);
            assert_eq!(store.list(Doc::Table).expect(name), vec!["g".to_string()], "{name}: h has no table");
            assert_eq!(store.list(Doc::Record).expect(name), vec!["g".to_string(), "h".to_string()]);
            store.delete("g").expect(name);
            assert!(store.read(Doc::Record, "g").expect(name).is_none() && store.read(Doc::Table, "g").expect(name).is_none());
        }
    }

    #[test]
    fn identical_bytes_from_two_writers_carry_one_version() {
        assert_eq!(version_of("{\"x\":1}"), version_of("{\"x\":1}"));
        assert_ne!(version_of("{\"x\":1}"), version_of("{\"x\":2}"));
    }
```

Keep `only_safe_ids_are_usable_as_storage_keys`, `a_refused_id_never_reaches_the_filesystem`, `an_absent_directory_is_an_empty_store_rather_than_an_error`, `concurrent_writers_never_publish_a_torn_record` and `an_unreadable_entry_is_present_not_absent`, re-pointed at the new signatures (`read(Doc::Record, …)`, `write_if_version(…, None)`; the unreadable-entry cell now asserts that `write_if_version(Doc::Record, id, …, None)` on an unreadable file is `Err(WriteError::Io(_))`, never a silent create).

- [ ] **Step 2: Run to see them fail**

Run: `cargo test -p yokohama-web store 2>&1 | grep -E 'error\[' | head -3`
Expected: `Doc`, `Version`, `WriteError`, `version_of` not found.

- [ ] **Step 3: Write the trait and both stores**

```rust
/// Which document under a game id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Doc {
    /// The `GameRecord` JSON — `<id>.json`.
    Record,
    /// The table document (`table.rs`) — `<id>.table.json`.
    Table,
}

impl Doc {
    fn suffix(self) -> &'static str {
        match self {
            Self::Record => ".json",
            Self::Table => ".table.json",
        }
    }
}

/// An opaque version of a stored document. For the file and memory stores it is
/// the digest of the exact bytes, so two writers of identical bytes agree; a
/// Postgres store would carry a row version. Callers compare and pass it back;
/// they never construct one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version(u64);

pub fn version_of(json: &str) -> Version {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    json.hash(&mut h);
    Version(h.finish())
}

#[derive(Debug)]
pub enum WriteError {
    /// The document is not at `expected`: `current` is what is there now, or
    /// `None` when the document is absent and a replace was asked for.
    Conflict { current: Option<Version> },
    Io(String),
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict { current: Some(_) } => write!(f, "the stored document changed under this writer"),
            Self::Conflict { current: None } => write!(f, "the document is absent"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

pub trait Store: Send + Sync {
    /// Every id holding a `doc`, sorted.
    fn list(&self, doc: Doc) -> Result<Vec<GameId>, String>;
    /// The document and its version, or `None` when absent. An unreadable
    /// present file is an `Err`, never `None`.
    fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String>;
    /// `expected: None` creates and refuses a present document; `Some(v)`
    /// replaces only if the current version is `v`. Atomic within a process;
    /// across processes it DETECTS a conflict and does not prevent one — the
    /// property `Games::persist` has always had, restated at the seam.
    fn write_if_version(&self, doc: Doc, id: &str, json: &str, expected: Option<Version>) -> Result<Version, WriteError>;
    /// Both documents.
    fn delete(&self, id: &str) -> Result<(), String>;
}
```

`FsStore` gains `lock: std::sync::Mutex<()>` (held across the read-compare-write of `write_if_version`); `path(&self, doc, id)` appends `doc.suffix()`; `list(doc)` keeps files whose name ends with `doc.suffix()` and — for `Doc::Record` — does NOT end with `.table.json`, stripping the suffix to the id and keeping `is_safe_id` ones; `read` returns `Ok(None)` on `NotFound`, `Err` on any other error, else `Some((json, version_of(&json)))`; `write_if_version` reads the current (`Ok(None)`/`Some`/`Err(Io)`), compares to `expected` (absent+None → write; Some(v)==current → write; else `Conflict { current }`), then the existing temp-and-rename write; `delete` removes both files, `Err` only when NEITHER existed. `MemStore` holds `BTreeMap<(Doc, GameId), String>` with the same rules (`Doc` needs `Ord` — derive `PartialOrd, Ord`).

- [ ] **Step 4: Re-express `Games`**

`Games.store: Arc<dyn Store>`, `written: Mutex<HashMap<GameId, Version>>`. `create`: the `exists` check becomes `self.store.read(Doc::Record, &id)?.is_some()` — NOTE: `read` returns `Err` for an unreadable file, which `create` propagates as its refusal, so `create_refuses_a_name_whose_record_is_unreadable` keeps passing. `handle`: `let (json, version) = self.store.read(Doc::Record, id)?.ok_or_else(|| format!("no game named {id:?}"))?;` and `written.insert(id, version)`. `persist`: the three guards in the same order —

```rust
        let record = repl.session.record();
        let events = record.events.len();
        let stored = self.store.read(Doc::Record, id).ok().flatten();
        let mut expected: Option<Version> = None;
        if let Some((json, current)) = &stored {
            if let Ok(on_disk) = serde_json::from_str::<GameRecord>(json) {
                if on_disk.seed != record.seed { /* the identity refusal, unchanged */ }
                match written.get(id) {
                    Some(base) if base == current => expected = Some(*current),
                    Some(_) => { /* the superseded refusal, unchanged */ }
                    None => {
                        if on_disk.events.len() > events { /* the backwards refusal, unchanged */ }
                        expected = Some(*current);
                    }
                }
            } else {
                expected = Some(*current);
            }
        }
        let json = repl.session.to_record_json()?;
        let version = match self.store.write_if_version(Doc::Record, id, &json, expected) {
            Ok(v) => v,
            Err(WriteError::Conflict { .. }) => {
                return Err(self.conflict(id, format!("{id:?} changed under this handle between the compare and the write")))
            }
            Err(WriteError::Io(e)) => return Err(e),
        };
        written.insert(id.to_string(), version);
```

`delete`: `self.store.delete(id)?` then the three map removals. `Games::list()` → `self.store.list(Doc::Record)` for now (Task 3 moves it to `Doc::Table`). `AppState::with_store(store: Arc<dyn Store>, …)`; `lib.rs` imports `Store` instead of `SessionStore`. Every `games.rs` test that constructed a `MemStore` or called `store.write(id, json)` now calls `write_if_version(Doc::Record, id, json, None)`; every `store.read(id)` becomes `store.read(Doc::Record, id).unwrap().unwrap().0`.

- [ ] **Step 5: Run every server gate**

Run: `cargo test -p yokohama-web 2>&1 | grep -E 'test result|FAILED' && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -1`
Expected: lib **78 + 3 = 81**? — count it: the store module's cells go from 6 to 8 (three new, one retired: `the_memory_store_round_trips_and_refuses_what_the_fs_store_refuses` folds into `both()`); record the number. The seventeen persist cells in `games.rs` — `a_divergent_write_of_the_same_length_is_refused`, `a_stale_handle_cannot_erase_a_move_another_writer_already_stored`, `an_undo_reaches_the_store`, `an_in_flight_write_cannot_land_on_a_different_game_that_took_the_name`, `a_handle_whose_write_was_refused_is_not_left_resident`, `a_delete_is_not_undone_by_a_move_already_in_flight`, `two_creates_of_one_name_cannot_both_succeed`, `create_refuses_a_name_whose_record_is_unreadable` among them — all pass UNCHANGED in what they assert. The gate and the corpus are unchanged.

- [ ] **Step 6: Commit**

```bash
git add crates/yokohama-web/src/store.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/lib.rs
git commit -m "refactor(web): the store seam — typed documents (record, table) and write_if_version; persist's three guards re-expressed through it, byte-for-byte in what they refuse"
```

### Task 3: Tables and the deal

**Files:**
- Create: `crates/yokohama-web/src/table.rs`
- Modify: `crates/yokohama-web/src/games.rs` (`NewGame`, `config_of`, `Games`, `preview`), `crates/yokohama-web/src/lib.rs` (`ApiError`, routes, handlers, tests), `crates/yokohama-cli/src/wizard.rs:1384` (`pub const KEEP_SCORE_NOTE`), `crates/yokohama-web/tests/wire_fixtures.rs`, `crates/yokohama-web/tests/play_a_whole_game.rs`, `web/src/types.ts`, `web/src/api.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`
- Test: `crates/yokohama-web/src/table.rs`, `games.rs`, `lib.rs`

**Interfaces (produced):**

```rust
// table.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeatDoc { Human { user: String }, Bot }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TableStatus { Dealing, Playing }

/// R-plan-3: two facts of the session, mirrored for the list.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnMirror { pub to_move: u8, pub over: bool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDoc {
    pub schema: u32,                 // 1
    pub id: String,
    pub name: Option<String>,
    pub created_by: String,
    pub created_at: u64,             // unix seconds (R-plan-1)
    pub players: u8,
    pub colors: Vec<PlayerColor>,
    pub start_seat: u8,
    pub seats: Vec<SeatDoc>,
    pub keeps: Vec<Option<u8>>,      // 1 | 2 on a human seat once chosen; bots stay None
    pub seed: u64,                   // never on the wire
    pub status: TableStatus,
    pub turn: Option<TurnMirror>,    // Some once playing
}

impl TableDoc {
    pub fn seats_of(&self, user: &User) -> Vec<PlayerId>;
    pub fn distinct_humans(&self) -> usize;
    pub fn all_humans_kept(&self) -> bool;
    pub fn next_unkept_seat_of(&self, user: &User) -> Option<PlayerId>;
    pub fn is_creator(&self, user: &User) -> bool;
    pub fn view(&self, user: &User) -> TableView;
}

#[derive(Debug, Clone, Serialize)]
pub struct TableView {
    pub id: String, pub name: Option<String>, pub created_by: String, pub created_at: u64,
    pub players: u8, pub colors: Vec<PlayerColor>, pub start_seat: u8,
    pub seats: Vec<SeatDoc>, pub chosen: Vec<bool>, pub my_seats: Vec<PlayerId>,
    pub status: TableStatus, pub to_move: Option<PlayerId>, pub over: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DealView {
    pub table: TableView,
    pub seat: Option<PlayerId>,
    pub cards: Option<Vec<DealCard>>,
    pub keep_note: &'static str,
    pub frame: HistoricalFrame,
}

// games.rs
#[derive(Debug, Clone, Deserialize)] #[serde(deny_unknown_fields)]
pub struct SeatSpec { #[serde(default)] pub user: Option<String>, #[serde(default)] pub bot: bool }
pub struct NewGame { players, colors, #[serde(default)] start_seat, #[serde(default)] seed: Option<WireSeed>, seats: Vec<SeatSpec>, #[serde(default)] name: Option<String> }

pub enum GameError { NotFound(String), Dealing, Forbidden(String), Conflict(String), BadRequest(String), Server(String) }
impl From<GameError> for ApiError;   // 404 / 409 (+ "status":"dealing") / 403 / 409 / 400 / 500

impl Games {
    pub fn create(&self, req: &NewGame, creator: &User) -> Result<TableView, GameError>;
    pub fn table(&self, id: &str, user: &User) -> Result<TableDoc, GameError>;      // settles; 403 if not seated
    pub fn handle(&self, id: &str) -> Result<Arc<Mutex<Repl>>, GameError>;          // Dealing while dealing
    pub fn deal(&self, id: &str, user: &User) -> Result<DealView, GameError>;
    pub fn keep(&self, id: &str, user: &User, seat: PlayerId, card: u8) -> Result<TableView, GameError>;
    pub fn list_for(&self, user: &User) -> Result<Vec<TableView>, String>;
    pub fn delete(&self, id: &str, user: &User) -> Result<(), GameError>;
}
```

`ApiError` gains `pub table_status: Option<&'static str>` (serialized as `"status"` beside `"error"` when `Some`), and `ApiError::dealing()`.

- [ ] **Step 1: Make the hedge public**

`crates/yokohama-cli/src/wizard.rs:1384`: `const KEEP_SCORE_NOTE` → `pub const KEEP_SCORE_NOTE` with one doc line: `/// Served by \`yokohama-web\` beside the deal (M6.1), so the browser carries no copy.` Run `cargo test -p yokohama-cli 2>&1 | grep 'test result'` — every line unchanged (850 + 2 ignored across the binaries), `guide_sync` untouched.

- [ ] **Step 2: Write the failing table cells**

`table.rs` tests:

```rust
    fn doc() -> TableDoc {
        TableDoc {
            schema: 1, id: "t".into(), name: None, created_by: "a@t.test".into(), created_at: 0,
            players: 3, colors: vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow], start_seat: 0,
            seats: vec![SeatDoc::Human { user: "a@t.test".into() }, SeatDoc::Human { user: "b@t.test".into() }, SeatDoc::Human { user: "a@t.test".into() }],
            keeps: vec![None, None, None], seed: 7, status: TableStatus::Dealing, turn: None,
        }
    }

    #[test]
    fn seats_of_a_user_are_theirs_and_in_order_and_a_stranger_has_none() {
        let a = User::parse("a@t.test").unwrap();
        assert_eq!(doc().seats_of(&a), vec![PlayerId::new(0), PlayerId::new(2)]);
        assert!(doc().seats_of(&User::parse("nobody@t.test").unwrap()).is_empty());
        assert_eq!(doc().distinct_humans(), 2);
    }

    #[test]
    fn the_next_unkept_seat_is_the_lowest_of_mine_and_none_once_all_mine_have_kept() {
        let a = User::parse("a@t.test").unwrap();
        let mut d = doc();
        assert_eq!(d.next_unkept_seat_of(&a), Some(PlayerId::new(0)));
        d.keeps[0] = Some(1);
        assert_eq!(d.next_unkept_seat_of(&a), Some(PlayerId::new(2)), "hot-seat: the next of mine");
        d.keeps[2] = Some(2);
        assert_eq!(d.next_unkept_seat_of(&a), None);
        assert!(!d.all_humans_kept(), "b has not kept");
        d.keeps[1] = Some(1);
        assert!(d.all_humans_kept());
    }

    #[test]
    fn the_view_carries_no_seed_and_no_keeps_and_marks_a_bot_chosen_from_the_start() {
        let mut d = doc();
        d.seats[1] = SeatDoc::Bot;
        d.keeps[0] = Some(2);
        let v = serde_json::to_value(d.view(&User::parse("a@t.test").unwrap())).unwrap();
        assert_eq!(v["chosen"], serde_json::json!([true, true, false]));
        assert_eq!(v["my_seats"], serde_json::json!([0, 2]));
        assert_eq!(v["status"], "dealing");
        assert_eq!(v["to_move"], serde_json::Value::Null);
        let text = v.to_string();
        assert!(!text.contains("seed") && !text.contains("keeps"), "{text}");
        assert_eq!(v["seats"][1], serde_json::json!({ "kind": "bot" }));
    }

    #[test]
    fn the_document_round_trips_through_json() {
        let d = doc();
        let back: TableDoc = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(serde_json::to_string(&back).unwrap(), serde_json::to_string(&d).unwrap());
    }
```

- [ ] **Step 3: Run to see them fail; write `table.rs`**

Run: `cargo test -p yokohama-web table 2>&1 | grep -E 'error\[' | head -2` → the module does not exist. Then write the types above with these bodies:

```rust
impl TableDoc {
    pub fn seats_of(&self, user: &User) -> Vec<PlayerId> {
        self.seats.iter().enumerate()
            .filter(|(_, s)| matches!(s, SeatDoc::Human { user: u } if u == user.as_str()))
            .map(|(i, _)| PlayerId::new(i as u8)).collect()
    }
    pub fn distinct_humans(&self) -> usize {
        self.seats.iter().filter_map(|s| match s { SeatDoc::Human { user } => Some(user.as_str()), SeatDoc::Bot => None })
            .collect::<std::collections::BTreeSet<_>>().len()
    }
    pub fn all_humans_kept(&self) -> bool {
        self.seats.iter().zip(&self.keeps).all(|(s, k)| matches!(s, SeatDoc::Bot) || k.is_some())
    }
    pub fn next_unkept_seat_of(&self, user: &User) -> Option<PlayerId> {
        self.seats_of(user).into_iter().find(|p| self.keeps[usize::from(p.0)].is_none())
    }
    pub fn is_creator(&self, user: &User) -> bool { self.created_by == user.as_str() }
    pub fn view(&self, user: &User) -> TableView {
        TableView {
            id: self.id.clone(), name: self.name.clone(), created_by: self.created_by.clone(),
            created_at: self.created_at, players: self.players, colors: self.colors.clone(),
            start_seat: self.start_seat, seats: self.seats.clone(),
            chosen: self.seats.iter().zip(&self.keeps).map(|(s, k)| matches!(s, SeatDoc::Bot) || k.is_some()).collect(),
            my_seats: self.seats_of(user), status: self.status,
            to_move: self.turn.map(|t| PlayerId::new(t.to_move)), over: self.turn.map(|t| t.over),
        }
    }
}

pub fn now_unix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
```

`PlayerId`'s inner field: check `crates/yokohama-core/src/types/ids.rs` — the code above uses `p.0`; if the field is private use `PlayerId::index()` or whatever accessor `client.rs` already uses (`usize::from(acting.0)` at `client.rs:228` says `.0` is reachable).

- [ ] **Step 4: Write the failing `Games` cells**

In `games.rs` tests, the helpers become:

```rust
    fn who(s: &str) -> User { User::parse(s).expect("an email") }
    fn req_seeded(name: &str, seed: u64) -> NewGame {
        NewGame {
            players: 2, colors: vec![PlayerColor::Red, PlayerColor::Blue], start_seat: 0,
            seed: Some(WireSeed(seed)),
            seats: vec![SeatSpec { user: Some("red@t.test".into()), bot: false }, SeatSpec { user: None, bot: true }],
            name: Some(name.to_string()),
        }
    }
    fn two_people(name: &str, seed: u64) -> NewGame {
        let mut r = req_seeded(name, seed);
        r.seats[1] = SeatSpec { user: Some("blue@t.test".into()), bot: false };
        r
    }
    /// create → keep for every human → the playing handle, as the API does it.
    fn playing(games: &Games, req: &NewGame, users: &[&str]) -> Arc<Mutex<Repl>> {
        let creator = who(users[0]);
        let id = games.create(req, &creator).expect("a table").id;
        for u in users {
            let user = who(u);
            while let Some(seat) = games.table(&id, &user).expect("seated").next_unkept_seat_of(&user) {
                games.keep(&id, &user, seat, 1).expect("a keep");
            }
        }
        games.handle(&id).expect("playing")
    }
```

and these cells (the existing seventeen persist cells switch their `games.create(&req(..)).expect(..)` to `playing(&games, &req(..), &["red@t.test"])`, which returns the handle they used to get):

```rust
    #[test]
    fn create_refuses_in_the_order_a_client_reads_and_binds_the_creator() {
        let games = games();
        let red = who("red@t.test");
        let mut r = req("order");
        r.players = 5;
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("2 to 4"));
        let mut r = req("order");
        r.seats[0] = SeatSpec { user: Some("other@t.test".into()), bot: false };
        let e = games.create(&r, &red).unwrap_err().to_string();
        assert!(e.contains("creator"), "R-1.6: {e}");
        let mut r = req("order");
        r.seats[1] = SeatSpec { user: Some("x@t.test".into()), bot: true };
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("exactly one"));
        let mut r = req("order");
        r.seats = vec![SeatSpec { user: None, bot: true }, SeatSpec { user: None, bot: true }];
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("human"));
        let v = games.create(&req("order"), &red).expect("a table");
        assert_eq!(v.status, TableStatus::Dealing);
        assert_eq!(v.my_seats, vec![PlayerId::new(0)]);
        assert_eq!(v.chosen, vec![false, true], "the bot is chosen from the start");
        assert!(games.store().read(Doc::Record, "order").unwrap().is_none(), "no record until the last keep");
    }

    #[test]
    fn a_bot_is_refused_at_three_and_four_players_with_the_measured_sentence() {
        let games = games();
        for n in [3u8, 4] {
            let mut r = req("bot3");
            r.players = n;
            r.colors = vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green][..n as usize].to_vec();
            r.seats = (0..n).map(|i| if i == 1 { SeatSpec { user: None, bot: true } } else { SeatSpec { user: Some("red@t.test".into()), bot: false } }).collect();
            let e = games.create(&r, &who("red@t.test")).unwrap_err().to_string();
            assert_eq!(e, "the bot is measured at two players only", "{n}P");
        }
    }

    #[test]
    fn the_deal_serves_the_callers_own_cards_and_never_the_others() {
        let games = games();
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        let id = games.create(&two_people("deal", CORPUS_SEED), &red).unwrap().id;
        let pending = new_game(GameConfig::for_player_count(2), &Rules::standard(), CORPUS_SEED).unwrap();
        let ids = |cards: &Vec<DealCard>| cards.iter().map(|c| c.card.id).collect::<Vec<_>>();
        let d = games.deal(&id, &red).expect("red's deal");
        assert_eq!(d.seat, Some(PlayerId::new(0)));
        assert_eq!(ids(d.cards.as_ref().unwrap()), pending.deals[0].to_vec());
        assert!(!pending.deals[1].iter().any(|b| ids(d.cards.as_ref().unwrap()).contains(b)), "disjoint by construction, asserted anyway");
        assert_eq!(d.keep_note, yokohama_cli::wizard::KEEP_SCORE_NOTE);
        let text = serde_json::to_string(&d).unwrap();
        assert!(!text.contains(&CORPUS_SEED.to_string()), "no seed on the wire");
        assert!(d.frame.labs.boards.iter().all(|b| b.slots.is_empty()), "withheld until the choice");
        games.keep(&id, &red, PlayerId::new(0), 1).unwrap();
        let d = games.deal(&id, &red).unwrap();
        assert_eq!((d.seat, d.cards.is_none()), (None, true), "all of red's seats have kept");
        let d = games.deal(&id, &blue).unwrap();
        assert_eq!(ids(d.cards.as_ref().unwrap()), pending.deals[1].to_vec());
    }

    #[test]
    fn keep_is_fenced_by_holder_and_by_once_and_the_last_keep_finalizes() {
        let games = games();
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        let id = games.create(&two_people("keep", ARGMAX_IS_CARD_2), &red).unwrap().id;
        assert!(matches!(games.keep(&id, &blue, PlayerId::new(0), 1), Err(GameError::Forbidden(_))));
        assert!(matches!(games.keep(&id, &red, PlayerId::new(0), 3), Err(GameError::BadRequest(_))));
        assert!(matches!(games.handle(&id), Err(GameError::Dealing)));
        games.keep(&id, &red, PlayerId::new(0), 2).unwrap();
        assert!(matches!(games.keep(&id, &red, PlayerId::new(0), 1), Err(GameError::Conflict(_))));
        let v = games.keep(&id, &blue, PlayerId::new(1), 1).unwrap();
        assert_eq!(v.status, TableStatus::Playing);
        assert_eq!(v.to_move, Some(PlayerId::new(0)));
        let (json, _) = games.store().read(Doc::Record, &id).unwrap().expect("the record exists now");
        let record: GameRecord = serde_json::from_str(&json).unwrap();
        assert!(record.terminal_digest.is_some());
        let handle = games.handle(&id).unwrap();
        let pending = new_game(GameConfig::for_player_count(2), &Rules::standard(), ARGMAX_IS_CARD_2).unwrap();
        assert_eq!(handle.lock().unwrap().session.state().players[0].order_cards_in_hand, vec![pending.deals[0][1]], "red kept card 2");
        assert!(matches!(games.keep(&id, &red, PlayerId::new(0), 1), Err(GameError::Conflict(_))), "a playing table takes no keep");
    }

    #[test]
    fn the_bot_keeps_its_own_argmax_at_finalize() {
        let games = games();
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, ARGMAX_IS_CARD_2).unwrap();
        assert_eq!(bot_keep(&pending.deals[1], &rules), 2, "the premise: at this seed the argmax is card 2");
        let handle = playing(&games, &req_seeded("argmax", ARGMAX_IS_CARD_2), &["red@t.test"]);
        assert_eq!(handle.lock().unwrap().session.state().players[1].order_cards_in_hand, vec![pending.deals[1][1]]);
    }

    #[test]
    fn a_hot_seat_creator_is_served_seat_zero_then_seat_one() {
        let games = games();
        let red = who("red@t.test");
        let mut r = two_people("hot", CORPUS_SEED);
        r.seats[1] = SeatSpec { user: Some("red@t.test".into()), bot: false };
        let id = games.create(&r, &red).unwrap().id;
        assert_eq!(games.deal(&id, &red).unwrap().seat, Some(PlayerId::new(0)));
        games.keep(&id, &red, PlayerId::new(0), 1).unwrap();
        assert_eq!(games.deal(&id, &red).unwrap().seat, Some(PlayerId::new(1)));
        let v = games.keep(&id, &red, PlayerId::new(1), 1).unwrap();
        assert_eq!(v.status, TableStatus::Playing);
    }

    #[test]
    fn a_table_left_dealing_with_every_keep_present_settles_on_the_next_read() {
        // Arm 1: the record exists and the status was never flipped.
        let games = games();
        let red = who("red@t.test");
        let id = games.create(&req_seeded("settle1", CORPUS_SEED), &red).unwrap().id;
        games.keep(&id, &red, PlayerId::new(0), 1).unwrap();
        let (json, v) = games.store().read(Doc::Table, &id).unwrap().unwrap();
        let mut doc: TableDoc = serde_json::from_str(&json).unwrap();
        doc.status = TableStatus::Dealing;
        doc.turn = None;
        games.store().write_if_version(Doc::Table, &id, &serde_json::to_string(&doc).unwrap(), Some(v)).unwrap();
        games.evict_for_test(&id);
        assert_eq!(games.table(&id, &red).unwrap().status, TableStatus::Playing, "settled on read");
        // Arm 2: every keep present and NO record (a crash between the two writes).
        let games = games();
        let id = games.create(&req_seeded("settle2", CORPUS_SEED), &red).unwrap().id;
        let (json, v) = games.store().read(Doc::Table, &id).unwrap().unwrap();
        let mut doc: TableDoc = serde_json::from_str(&json).unwrap();
        doc.keeps[0] = Some(1);
        games.store().write_if_version(Doc::Table, &id, &serde_json::to_string(&doc).unwrap(), Some(v)).unwrap();
        let t = games.table(&id, &red).unwrap();
        assert_eq!(t.status, TableStatus::Playing);
        assert!(games.store().read(Doc::Record, &id).unwrap().is_some(), "finalized on read");
    }

    #[test]
    fn the_list_is_the_tables_i_sit_at_newest_first_with_whose_turn_it_is() {
        let games = games();
        let (red, blue, green) = (who("red@t.test"), who("blue@t.test"), who("green@t.test"));
        games.create(&two_people("first", CORPUS_SEED), &red).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let handle = playing(&games, &req_seeded("second", CORPUS_SEED), &["red@t.test"]);
        drop(handle);
        let mine = games.list_for(&red).unwrap();
        assert_eq!(mine.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), vec!["second", "first"], "newest first");
        assert_eq!((mine[0].status, mine[0].to_move, mine[0].over), (TableStatus::Playing, Some(PlayerId::new(0)), Some(false)));
        assert_eq!(games.list_for(&blue).unwrap().len(), 1);
        assert!(games.list_for(&green).unwrap().is_empty());
        // A record with no table document is not a game this API serves (R-P7).
        games.store().write_if_version(Doc::Record, "legacy", "{}", None).unwrap();
        assert!(games.list_for(&red).unwrap().iter().all(|t| t.id != "legacy"));
        assert!(matches!(games.handle("legacy"), Err(GameError::NotFound(_))));
    }

    #[test]
    fn the_turn_mirror_follows_every_persisted_move() {
        let games = games();
        let red = who("red@t.test");
        let handle = playing(&games, &two_people("mirror", CORPUS_SEED), &["red@t.test", "blue@t.test"]);
        let before = games.table("mirror", &red).unwrap().turn.unwrap();
        {
            let mut r = handle.lock().unwrap();
            // Walk red's turn to its end with the sole options the opening offers.
            for _ in 0..40 {
                if r.session.state().current_player != PlayerId::new(0) { break; }
                let rows = r.option_rows();
                let n = yokohama_cli::screen_model::menu_number(rows.len() - 1);
                r.step(&n.to_string());
                games.persist("mirror", &r).unwrap();
            }
        }
        let after = games.table("mirror", &red).unwrap().turn.unwrap();
        assert_ne!(before, after, "the mirror moved with the turn");
        assert_eq!(after.to_move, 1);
    }

    #[test]
    fn delete_is_the_creators() {
        let games = games();
        let (red, blue) = (who("red@t.test"), who("blue@t.test"));
        let id = games.create(&two_people("del", CORPUS_SEED), &red).unwrap().id;
        assert!(matches!(games.delete(&id, &blue), Err(GameError::Forbidden(_))));
        games.delete(&id, &red).unwrap();
        assert!(matches!(games.table(&id, &red), Err(GameError::NotFound(_))));
    }
```

`Games::evict_for_test(&self, id)` is a `#[cfg(test)] pub fn` that removes the live handle (so the settle arm exercises the cold path). The existing cells `the_bot_keeps_the_higher_scored_card_and_the_human_keeps_theirs`, `a_keep_on_a_bot_seat_and_a_missing_keep_on_a_human_seat_are_refused_by_name` and `the_created_game_holds_the_bots_own_keep_at_both_seeds` are RETIRED — their claims live in the cells above (`the_bot_keeps_its_own_argmax_at_finalize`, `keep_is_fenced…`); `the_preview_is_the_position_after_setup`, `a_bot_seats_deal_never_reaches_the_wire` and `the_preview_withholds_the_laboratory_and_the_achievements` are re-pointed at the new `NewGame` body and stay until Task 10 (R-plan-4).

- [ ] **Step 5: Run to see them fail, then implement**

Run: `cargo test -p yokohama-web --lib games 2>&1 | grep -E 'error\[' | head -3` → `SeatSpec`, `GameError`, `table` not found.

Implement in `games.rs`:

`config_of(req: &NewGame, creator: &User) -> Result<(GameConfig, Vec<SeatDoc>), String>` — the checks in this order, each its own sentence: players 2–4 (`"{} players — the game is 2 to 4"`); colours one per seat and distinct (existing sentences); `start_seat` in range (existing); `seats.len() == n` (`"{} seat entries for {n} seats"`); each entry exactly one of `user`/`bot` (`"seat {i}: exactly one of user or bot"`); each `user` through `User::parse` (its sentence prefixed `"seat {i}: "`); at least one human (`"at least one seat must stay human"`); the creator among the humans (`"the creator must hold a seat — {creator} is not seated"`); a bot at n > 2 → exactly `"the bot is measured at two players only"`.

`Games` gains `table_lock: Mutex<()>` (rename `create_lock`). `create`: under the lock, `config_of`, the seed and id as today, `read(Doc::Table, &id)?.is_some() || read(Doc::Record, &id)?.is_some()` → `"a game named {id:?} already exists"`, build the `TableDoc` (`keeps: vec![None; n]`, `status: Dealing`, `turn: None`, `created_at: now_unix()`), `write_if_version(Doc::Table, &id, &json, None)` (a `Conflict` is the duplicate sentence), clear the tombstone, return `doc.view(creator)`.

`read_table(&self, id) -> Result<Option<(TableDoc, Version)>, GameError>` (parse failure → `Server`). `settle(&self, id) -> Result<Option<(TableDoc, Version)>, GameError>`: read; if `Dealing && all_humans_kept()`: under `table_lock`, re-read; if the record exists → `status = Playing`, `turn = mirror_of(&rebuilt session)` (rebuild through `handle_inner`), write `Some(v)`; else → `finalize(doc, v)`. `table(&self, id, user)`: `settle` → `None` → `NotFound("no game named …")`; `seats_of(user).is_empty()` → `Forbidden("you are not seated at this table")`; else the doc.

`finalize(&self, mut doc, v) -> Result<(TableDoc, Version), GameError>` (caller holds `table_lock`): `config` from the doc; `bots = BotSeats::new(seats.iter().map(|s| matches!(s, Bot).then_some(PROVISIONAL_DEFAULT_BOT)).collect(), draw_seed())`; `pending = new_game(config.clone(), rules, doc.seed)`; choices per seat: bot → `Keep(bot_keep(deal) - 1)`, human → `Keep(keeps[i].unwrap() - 1)`; `Session::new_sim`, `set_run_seats`, `Repl::new`, `bring_alive`; `json = repl.session.to_record_json()`; `write_if_version(Doc::Record, &id, &json, None)` (a `Conflict` here means the record already exists — treat as the settle arm: continue); `written.insert(id, version)`; `doc.status = Playing; doc.turn = Some(mirror_of(&repl))`; `write_if_version(Doc::Table, …, Some(v))`; insert the live handle with `entry().or_insert` + `evict_down_to`. `mirror_of(repl) -> TurnMirror { to_move: state.current_player.0, over: session.is_over() }`.

`handle(&self, id)`: resident → Ok; else `settle(id)?` → `None` → `NotFound`; `Dealing` → `Err(GameError::Dealing)`; else the existing rebuild path (`read(Doc::Record)` → `written` → `from_record` → pump → persist) with `String` errors mapped to `Server`.

`deal(&self, id, user)`: `table(id, user)?`; `Playing` → `Conflict("this table is playing")`; `seat = doc.next_unkept_seat_of(user)`; `pending = new_game(config, rules, doc.seed)`; the throwaway session and frame EXACTLY as `preview` builds them today (move that body into `fn set_table(doc, bots, rules) -> Result<HistoricalFrame, String>` and call it from both `preview` and `deal` until Task 10 deletes `preview`); `cards = seat.map(|s| pending.deals[s].map(DealCard {..}))`; `keep_note: yokohama_cli::wizard::KEEP_SCORE_NOTE`.

`keep(&self, id, user, seat, card)`: under `table_lock`: read; `Playing` → `Conflict`; the seat is the caller's (`seats_of(user).contains(&seat)`) → else `Forbidden("seat {n} is not yours")`; `keeps[seat].is_some()` → `Conflict("seat {n} has already chosen")`; `card` in 1..=2 → else `BadRequest("keep {card} — the dealt cards are numbered 1 and 2")`; set; if `all_humans_kept()` → `finalize` else write `Some(v)`; return the view.

`list_for(&self, user)`: `store.list(Doc::Table)` → read each doc (skip unparseable with no error), keep those where `seats_of(user)` is non-empty, sort by `created_at` descending then id, map to views. `persist` additionally: after the record write, `if let Some((mut doc, v)) = read_table(id)? { doc.turn = Some(mirror_of(repl)); let _ = write_if_version(Doc::Table, id, json, Some(v)); }` — a conflict on the mirror is IGNORED (the record is the truth; the list may read one move stale until the next persist). `delete(&self, id, user)`: `table(id, user)?`, `is_creator` or `Forbidden("only {created_by} can delete this table")`, then the existing removals. `Games::list()` is deleted.

`GameError` → `ApiError`: `NotFound` 404; `Dealing` → `ApiError { status: CONFLICT, message: "this table is still dealing", table_status: Some("dealing") }`; `Forbidden` 403; `Conflict` 409; `BadRequest` 400; `Server` 500. `ApiError::into_response` emits `{"error": m}` or `{"error": m, "status": s}`.

- [ ] **Step 6: Route it**

`lib.rs`: `create_game(State, Identity(user), JsonBody<NewGame>) -> Result<(StatusCode, Json<TableView>), ApiError>` → 201; `list_games(State, Identity(user)) -> ApiResult<GamesList>` where `GamesList { games: Vec<TableView> }`; `get_table(State, Identity, Path)` at `GET /api/games/{id}/table`; `deal(…)` at `GET /api/games/{id}/deal`; `keep(State, Identity, Path, JsonBody<Keep>)` at `POST /api/games/{id}/keep` with `#[derive(Deserialize)] #[serde(deny_unknown_fields)] pub struct Keep { pub seat: u8, pub card: u8 }`; `delete_game(State, Identity, Path)`. Every handler that calls `state.games.handle(&id)` maps its error with `ApiError::from` (which is what makes a dealing table answer 409 on `ports`, `labs`, `areas`, `log`, `at/{k}`, `index`, `result`, `input`, `suggest`, `why` and the state). `preview_game` stays and takes `Identity` (R-plan-4).

Route cells in `lib.rs` tests (each with a real router and two identities `red@wire.test` / `blue@wire.test`; write `post_as(state, user, uri, body)`, `get_as(state, user, uri)` and `delete_as(state, user, uri)` helpers beside `get_with` — each takes the shared `Arc<AppState>` so one game is driven by several identities):

```rust
    #[tokio::test]
    async fn the_create_flow_deals_per_seat_and_the_state_is_409_until_the_last_keep() {
        let state = Arc::new(AppState::in_memory());
        let body = serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "20260831", "name": "flow",
            "seats": [{ "user": "red@wire.test" }, { "user": "blue@wire.test" }],
        });
        let (status, table) = post_as(&state, "red@wire.test", "/api/games", body).await;
        assert_eq!(status, StatusCode::CREATED, "{table}");
        assert_eq!(table["status"], "dealing");
        assert!(!table.to_string().contains("20260831"), "no seed on the wire: {table}");
        let (status, body) = get_as(&state, "red@wire.test", "/api/games/flow").await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body, serde_json::json!({ "error": "this table is still dealing", "status": "dealing" }));
        let (status, _) = get_as(&state, "red@wire.test", "/api/games/flow/ports").await;
        assert_eq!(status, StatusCode::CONFLICT, "every session route");
        let (status, deal) = get_as(&state, "blue@wire.test", "/api/games/flow/deal").await;
        assert_eq!(status, StatusCode::OK, "{deal}");
        assert_eq!(deal["seat"], 1);
        assert_eq!(deal["cards"].as_array().unwrap().len(), 2);
        let (status, err) = post_as(&state, "blue@wire.test", "/api/games/flow/keep", serde_json::json!({ "seat": 0, "card": 1 })).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{err}");
        let (status, _) = post_as(&state, "blue@wire.test", "/api/games/flow/keep", serde_json::json!({ "seat": 1, "card": 1 })).await;
        assert_eq!(status, StatusCode::OK);
        let (status, err) = get_as(&state, "green@wire.test", "/api/games/flow/deal").await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{err}");
        let (status, view) = post_as(&state, "red@wire.test", "/api/games/flow/keep", serde_json::json!({ "seat": 0, "card": 2 })).await;
        assert_eq!((status, view["status"].as_str()), (StatusCode::OK, Some("playing")), "{view}");
        let (status, game) = get_as(&state, "red@wire.test", "/api/games/flow").await;
        assert_eq!(status, StatusCode::OK, "{game}");
        assert_eq!(game["events"], 0);
        let (status, list) = get_as(&state, "blue@wire.test", "/api/games").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(list["games"][0]["my_seats"], serde_json::json!([1]));
        let (status, _) = get_as(&state, "green@wire.test", "/api/games").await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn the_old_body_keys_are_refused_by_name_and_delete_is_the_creators() {
        let state = Arc::new(AppState::in_memory());
        let old = serde_json::json!({ "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "keeps": [1, null], "bot_seats": [1] });
        let (status, err) = post_as(&state, "red@wire.test", "/api/games", old).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(err["error"].as_str().unwrap().contains("keeps"), "{err}");
        let body = serde_json::json!({ "players": 2, "colors": ["Red", "Blue"], "name": "own",
            "seats": [{ "user": "red@wire.test" }, { "user": "blue@wire.test" }] });
        post_as(&state, "red@wire.test", "/api/games", body).await;
        let (status, _) = delete_as(&state, "blue@wire.test", "/api/games/own").await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, _) = delete_as(&state, "red@wire.test", "/api/games/own").await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }
```

Re-point the existing route cells that create a game (`live_router`, `a_deleted_game_is_gone_from_the_list_and_from_the_store`, the M3 cells, `every_payload_shows_the_hand_of_the_seat_it_offers_moves_for`, the replay cells, `the_live_payload_counts_its_events`, `an_answer_to_a_prompt_the_game_is_no_longer_asking_is_refused`) through a shared `async fn start(state, name, seed, seats) -> ()` helper that creates and keeps for every human as its own identity; `the_preview_serves_the_set_table_and_refuses_a_keep_on_a_bot_seat` becomes `the_preview_serves_the_set_table_and_withholds_the_bots_deal` (the "refuses a keep on a bot seat" half is now `keep`'s 403 above); `a_seed_survives_the_preview_round_trip_past_the_double_limit` is re-pointed at `create` with a big `seed` string and `deal` asserting the dealt ids match `new_game` at that seed.

- [ ] **Step 7: Re-point the two integration harnesses and regenerate the corpus**

`wire_fixtures.rs`: `Api::call` takes the identity as a parameter (`call_as(user, method, uri, body)`; `call`/`get`/`send` default to `red@wire.test`; `send_as(user, line)`). `new_game_body()` → `"seats": [{"user": "red@wire.test"}, {"user": "blue@wire.test"}]`, no `keeps`/`bot_seats`. The walk: `create` (201, captured as `table_dealing`), `GET …/table` as blue (captured as `table` after finalize), `GET …/deal` as red (captured as `deal`), `POST …/keep` as red `{seat:0, card:1}`, `GET …/deal` as red again (captured as `deal_waiting`), `GET /api/games/wire` as red while dealing (captured as `dealing_409`, status asserted 409), keep as blue, then `GET /api/games/wire` as red is the first `state`. **Every `send` in the walk goes as the ACTING seat**: `send_as(if state["to_move"] == 0 { "red@wire.test" } else { "blue@wire.test" }, line)` — a helper `acting(&state) -> &str`. `preview_bot`'s body: `"seats": [{"user": "red@wire.test"}, {"bot": true}]`. `games` (the list as red, captured after the first keep so it carries a dealing table) and `not_seated_403` (`GET /api/games/wire/deal` as `green@wire.test`) join the corpus. The gate `play_a_whole_game.rs`: preview body updated; create with `"seats": [{"user": "red@wire.test"}, {"bot": true}]`, expect 201 `status: dealing`, then `POST …/keep {seat:0, card:1}` → `status: playing`, then `GET /api/games/gate` is the first `state`; the loop is unchanged.

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures 2>&1 | tail -3 && cargo test -p yokohama-web 2>&1 | grep 'test result' && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -1`
Expected: new files `table_dealing.json`, `table.json`, `deal.json`, `deal_waiting.json`, `dealing_409.json`, `games.json`, `not_seated_403.json`; `preview.json`/`preview_bot.json` re-cut (same shape, new body); every `ClientState` fixture BYTE-IDENTICAL to before (the viewer is still the acting seat until Task 4 — say so in the report); gate green; clippy clean.

- [ ] **Step 8: The client's wire half**

`types.ts`: `SeatDoc = { kind: "human"; user: string } | { kind: "bot" }`, `TableStatus = "dealing" | "playing"`, `TableView`, `DealView`, `GamesList { games: TableView[] }`, `KeepRequest { seat: number; card: 1 | 2 }`; `NewGameRequest` in `api.ts` → `{ players, colors, start_seat, seed?, seats: ({ user: string } | { bot: true })[], name? }`; `api.create` returns `TableView`, `api.list` returns `GamesList`, `api.table(game)`, `api.deal(game)`, `api.keep(game, seat, card)`. `contract.ts`: imports and `Widen` assignments for the seven new fixtures, `KEYS.table = ["id","name","created_by","created_at","players","colors","start_seat","seats","chosen","my_seats","status","to_move","over"]`, `KEYS.dealView = ["table","seat","cards","keep_note","frame"]`, `KEYS.gamesList = ["games"]`, `wire.tableDealing`, `wire.table`, `wire.deal`, `wire.dealWaiting`, `wire.games`, `wire.dealing409` (as `ApiErrorBody & { status: string }`), `wire.notSeated403`; `contract.test.ts` rows for each. `NewGame.tsx` keeps compiling: change its `base()` to send `seats` (seat 0 `{ user: "me" }`… NO — the form is rewritten in Task 8; here change only what `tsc` demands: `api.preview` still takes `NewGameRequest`, so `base()` builds `seats: Array.from({length: players}, (_, i) => i === botSeat ? { bot: true } : { user: claimedUser() ?? "" })` and `Start` calls `api.create(...)` then `api.keep(...)` for every `chosen` seat and finally `api.get(id)` before `onStarted` — a MINIMAL bridge so the component suite stays green until Task 8 replaces the screen; its cells that read `keeps` off the create body change to reading the `keep` posts (`stub.matching("POST", "/keep")`). `App.test.tsx:1044`'s preview stub stays.

Run: `cd web && npm run typecheck && npm test 2>&1 | grep -E 'Tests |FAIL'`
Expected: typecheck clean; **395 + 7 contract rows = 402** passed (or the number measured; record it). `npm run e2e` is RED from here (the harness's `startGame` presses buttons the bridge no longer wires the same way) — expected, stated in Global Constraints.

- [ ] **Step 9: Commit**

```bash
git add crates/yokohama-web/src/table.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/lib.rs crates/yokohama-cli/src/wizard.rs crates/yokohama-web/tests/wire_fixtures.rs crates/yokohama-web/tests/play_a_whole_game.rs web/src/types.ts web/src/api.ts web/src/NewGame.tsx web/src/NewGame.test.tsx web/src/wire/contract.ts web/src/wire/contract.test.ts web/src/wire/
git commit -m "feat(web): tables and the per-seat deal — create opens a dealing table, each seat keeps from its own view, the last keep finalizes; the list is the tables you sit at; the seed leaves the create flow"
```

### Task 4: Per-seat views and the route fences

**Files:**
- Modify: `crates/yokohama-web/src/client.rs` (`viewer_seat`, `facts_for`, `ClientState::of`, `HistoricalFrame::at`; delete `viewing_seat`, `replay_seat`), `crates/yokohama-web/src/lib.rs` (a `seated` helper, every session handler, `suggest`, `why`, tests), `crates/yokohama-web/tests/wire_fixtures.rs` (the in-process builds pass a viewer; `hidden_hand` retired), `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`, `web/src/Dashboard.test.tsx`
- Test: `crates/yokohama-web/src/client.rs`, `lib.rs`

**Interfaces:**
- Produces: `client::viewer_seat(table: &TableDoc, user: &User, state: &GameState) -> Option<PlayerId>`; `client::facts_for(repl: &Repl, viewer: PlayerId) -> Vec<SeatFacts>`; `ClientState::of(id, repl, viewer: PlayerId, messages)`; `HistoricalFrame::at(repl, k, viewer: PlayerId)`; in `lib.rs`, `async fn seated(state: &AppState, id: &str, user: &User) -> Result<(Arc<Mutex<Repl>>, TableDoc), ApiError>` and `fn viewer_of(repl: &Repl, table: &TableDoc, user: &User) -> Result<PlayerId, ApiError>` (403 when `None`).

- [ ] **Step 1: Write the failing client cells**

```rust
    #[test]
    fn the_viewer_is_the_acting_seat_when_mine_else_my_lowest_else_none() {
        let (red, blue, green) = (User::parse("red@t.test").unwrap(), User::parse("blue@t.test").unwrap(), User::parse("green@t.test").unwrap());
        let mut doc = table_of(&["red@t.test", "blue@t.test", "red@t.test"]);  // a 3P hot-seat for red
        let state = state_with_current(PlayerId::new(2));
        assert_eq!(viewer_seat(&doc, &red, &state), Some(PlayerId::new(2)), "acting and mine");
        assert_eq!(viewer_seat(&doc, &blue, &state), Some(PlayerId::new(1)), "not acting: my lowest");
        assert_eq!(viewer_seat(&doc, &green, &state), None);
        doc.seats[2] = SeatDoc::Bot;
        assert_eq!(viewer_seat(&doc, &red, &state), Some(PlayerId::new(0)), "a bot is acting: my lowest");
    }

    #[test]
    fn the_facts_hide_every_hand_but_the_viewers_bots_included() {
        let repl = two_human_repl();   // the corpus-seed 2P game with no bots
        let f = facts_for(&repl, PlayerId::new(1));
        assert_eq!(f.iter().map(|s| s.hand_hidden).collect::<Vec<_>>(), vec![true, false]);
        assert!(f.iter().all(|s| !s.is_bot));
        let payload = ClientState::of("t", &repl, PlayerId::new(1), Vec::new());
        assert_eq!(payload.seat, PlayerId::new(1));
        let hands: Vec<(u8, bool, usize)> = payload.frame.dashboard.iter().filter_map(|p| match p {
            Panel::Hand(h) => Some((h.seat.0, h.hidden, h.cards.len())), _ => None }).collect();
        assert_eq!(hands, vec![(1, false, 1)], "the viewer's own hand, open, and no other Hand panel");
    }
```

`table_of`, `state_with_current` and `two_human_repl` are test helpers in `client.rs` (`two_human_repl` builds exactly as `wire_fixtures.rs`'s `build(vec![None, None])` does). Check which panels `build_frame` emits for the non-viewer seats — `build_dashboard(state, rules, seats, seat)` builds ONE Hand panel (the viewer's) and a Players panel carrying `hand_hidden` per seat; the cell above asserts what it finds and the implementer corrects the expectation to the model's real shape, recording it in the ledger.

- [ ] **Step 2: Implement `client.rs`**

```rust
/// The seat a payload is built FOR (spec §5.1): the acting seat when the caller
/// holds it, else the caller's lowest seat, else `None` — which the route turns
/// into a 403 (R-1.5). Replaces `viewing_seat`, which followed the mover, and
/// `replay_seat`, which existed to give an UNAUTHENTICATED replay route one
/// stable seat; an authenticated route's stable seat is the caller's own.
pub fn viewer_seat(table: &TableDoc, user: &User, state: &GameState) -> Option<PlayerId> {
    let mine = table.seats_of(user);
    if mine.contains(&state.current_player) {
        return Some(state.current_player);
    }
    mine.first().copied()
}

/// R-1.7: a hand is shown to its own seat only, bots included, hot-seat included.
/// This is the SERVER's answer to `build_frame`'s `seats` and it replaces
/// `Repl::seat_facts()` in this crate: the REPL's privacy flag is one operator's
/// toggle at one screen, and a per-seat payload has no use for it.
pub fn facts_for(repl: &Repl, viewer: PlayerId) -> Vec<SeatFacts> {
    repl.session.state().players.iter()
        .map(|p| SeatFacts { is_bot: repl.bots().is_bot(p.id), hand_hidden: p.id != viewer })
        .collect()
}
```

`ClientState::of(id, repl, viewer, messages)`: `let seat = viewer;` and `build_frame(state, rules, &facts_for(repl, viewer), seat, …)`; `your_turn: !over && state.current_player == viewer`; `prompt: prompt_of_seat(repl, viewer)`. `HistoricalFrame::at(repl, k, viewer)`: `seat = viewer`, `&facts_for(repl, viewer)`. Delete `viewing_seat` and `replay_seat` and their docs; the `Watching` variant's doc loses its "bot-fallback arm" sentence and gains "reached when the caller's seat is not the acting one".

- [ ] **Step 3: Write the failing route cells, then wire the handlers**

```rust
    #[tokio::test]
    async fn every_session_route_is_403_for_a_signed_in_stranger() {
        let state = Arc::new(AppState::in_memory());
        start(&state, "fence", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        for uri in ["/api/games/fence", "/api/games/fence/table", "/api/games/fence/ports", "/api/games/fence/labs",
                    "/api/games/fence/areas", "/api/games/fence/log", "/api/games/fence/at/0", "/api/games/fence/events",
                    "/api/games/fence/suggest", "/api/games/fence/why", "/api/games/fence/index/orders", "/api/games/fence/result"] {
            let (status, body) = get_as(&state, "green@wire.test", uri).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{uri}: {body}");
            assert_eq!(body["error"], "you are not seated at this table");
        }
        let (status, _) = post_as(&state, "green@wire.test", "/api/games/fence/input", serde_json::json!({ "line": "1" })).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn input_is_the_acting_seats_holders_and_names_whose_turn_it_is() {
        let state = Arc::new(AppState::in_memory());
        start(&state, "acting", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        let (status, body) = post_as(&state, "blue@wire.test", "/api/games/acting/input", serde_json::json!({ "line": "1" })).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "it is seat 0's turn (red@wire.test)");
        let (status, body) = post_as(&state, "red@wire.test", "/api/games/acting/input", serde_json::json!({ "line": "1" })).await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    #[tokio::test]
    async fn a_payload_for_one_seat_never_carries_another_seats_cards() {
        let state = Arc::new(AppState::in_memory());
        start(&state, "mask", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        for (user, mine) in [("red@wire.test", 0u64), ("blue@wire.test", 1)] {
            let (_, game) = get_as(&state, user, "/api/games/mask").await;
            assert_eq!(game["seat"], mine);
            let players = game["frame"]["dashboard"].as_array().unwrap().iter().find_map(|p| p.get("Players")).unwrap();
            for p in players["players"].as_array().unwrap() {
                assert_eq!(p["hand_hidden"], p["seat"] != mine, "{user}: {p}");
            }
            for p in game["frame"]["dashboard"].as_array().unwrap().iter().filter_map(|p| p.get("Hand")) {
                assert_eq!(p["seat"], mine, "the one Hand panel is the viewer's");
                assert_eq!(p["hidden"], false);
            }
        }
        let (_, as_blue) = get_as(&state, "blue@wire.test", "/api/games/mask").await;
        assert_eq!(as_blue["your_turn"], false);
        assert_eq!(as_blue["prompt"]["kind"], "watching");
    }

    #[tokio::test]
    async fn a_replay_sweep_as_each_caller_shows_only_that_callers_hands() {
        let state = Arc::new(AppState::in_memory());
        start(&state, "sweep", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        // Play red's turn to its end so the sweep spans a turn boundary.
        play_turn(&state, "sweep", "red@wire.test").await;
        let (_, game) = get_as(&state, "blue@wire.test", "/api/games/sweep").await;
        let total = game["events"].as_u64().unwrap() as usize;
        assert!(total > 0);
        for (user, mine) in [("red@wire.test", 0u64), ("blue@wire.test", 1)] {
            let mut seen = std::collections::BTreeSet::new();
            for k in 0..=total {
                let (status, frame) = get_as(&state, user, &format!("/api/games/sweep/at/{k}")).await;
                assert_eq!(status, StatusCode::OK);
                assert_eq!(frame["seat"], mine, "{user} at {k}");
                for h in frame["frame"]["dashboard"].as_array().unwrap().iter().filter_map(|p| p.get("Hand")) {
                    seen.insert(h["seat"].as_u64().unwrap());
                }
            }
            assert_eq!(seen, std::collections::BTreeSet::from([mine]), "{user}: one seat's hands across the sweep");
        }
    }

    #[tokio::test]
    async fn suggest_answers_the_acting_seats_holder_only() {
        let state = Arc::new(AppState::in_memory());
        start(&state, "advise", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        let (status, body) = get_as(&state, "blue@wire.test", "/api/games/advise/suggest").await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "it is not your turn");
        let (status, body) = get_as(&state, "red@wire.test", "/api/games/advise/suggest").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["ok"]["seat"], 0);
    }
```

`play_turn(state, id, user)` posts the LAST option row's number repeatedly as `user` until `to_move` changes (bounded at 40 posts) — the same loop `the_turn_mirror_follows_every_persisted_move` uses. Retire `a_replay_sweep_never_unmasks_more_than_one_seat` and `the_replay_pin_does_not_move_when_the_turn_does` (their property is now the sweep cell above, per caller); re-point `every_payload_shows_the_hand_of_the_seat_it_offers_moves_for` to request as the acting seat's holder at every step.

Wire: in `lib.rs`

```rust
/// Resolve the caller to a seated table and a live handle: 404 for no table, 403
/// for a stranger (R-1.5), 409 while dealing. Every session route starts here.
fn seated(state: &AppState, id: &str, user: &User) -> Result<(Arc<Mutex<Repl>>, TableDoc), ApiError> {
    let table = state.games.table(id, user)?;
    let handle = state.games.handle(id)?;
    Ok((handle, table))
}

fn viewer_of(repl: &Repl, table: &TableDoc, user: &User) -> Result<PlayerId, ApiError> {
    client::viewer_seat(table, user, repl.session.state())
        .ok_or_else(|| ApiError { status: StatusCode::FORBIDDEN, message: "you are not seated at this table".into(), table_status: None })
}
```

`get_game`, `input`, the `surface!` macro, `log`, `at_k`, `card_index`, `result`, `why`: `Identity(user)` → `seated` → lock → `viewer_of` → build with the viewer. `input` additionally, after the grammar check and before `step`: `if repl.session.state().current_player != viewer { return Err(ApiError { status: FORBIDDEN, message: format!("it is seat {}'s turn ({})", acting.0, holder_email(&table, acting)), .. }) }` where `holder_email` reads `table.seats[acting]` (`Human { user }` → the email; `Bot` → `"the bot"`). `suggest`: after `seated`, if `state.current_player` is not among `table.seats_of(&user)` → 403 `"it is not your turn"`; the existing refusal ladder follows. `create_game`/`keep` are unchanged. `ClientState::of` call sites pass the viewer; `HistoricalFrame::at(&repl, k, viewer)`.

`wire_fixtures.rs`: every in-process `ClientState::of("wire", &repl, …)` passes a viewer (`PlayerId::new(1)` for `watching`, `PlayerId::new(0)` for `locked`); the walk's GETs go as the acting seat's holder (the `acting(&state)` helper), so `options.json` and its siblings are built for seat 0 or 1 as the turn stands; the `privacy` send and the `hidden_hand` capture are DELETED (the walk's owed list drops `"hidden_hand"`). `contract.ts`/`contract.test.ts`: remove `hiddenHand` everywhere (`_hiddenHand`, `_pHiddenHand`, `wire.hiddenHand`, `EVERY_STATE`). `Dashboard.test.tsx:225-227`: the cell becomes `it("another seat's hand shows as a count and never as cards", …)` over `wire.options`: find the Players panel, assert every player with `seat !== wire.options.seat` has `hand_hidden === true` and the rendered row for it reads the count with no card face, and the viewer's own Hand panel renders its cards.

- [ ] **Step 4: Regenerate, run every gate, commit**

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures 2>&1 | tail -3 && cargo test -p yokohama-web 2>&1 | grep -E 'test result|FAILED' && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -1 && cd web && npm run typecheck && npm test 2>&1 | grep -E 'Tests |FAIL'`
Expected: `hidden_hand.json` deleted; every `ClientState` fixture re-cut — in each, `frame.dashboard`'s Players rows carry `hand_hidden: true` for every seat but `seat`; `at_k.json` built for seat 0 (the caller); report which fixtures changed and confirm `deck_remaining`, `events`, `prompt` did not; gate green; the component suite at the measured count.

```bash
git add crates/yokohama-web/src/client.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/tests/wire_fixtures.rs web/src/wire/ web/src/wire/contract.ts web/src/wire/contract.test.ts web/src/Dashboard.test.tsx
git commit -m "feat(web): per-seat views — every game route resolves the caller to a seat, masks every other hand through SeatFacts, fences input to the acting seat's holder, and answers suggest for that holder only"
```

---

### Task 5: The grammar and the undo rule

**Files:**
- Modify: `crates/yokohama-web/src/grammar.rs` (`HEADS`, `refusal`), `crates/yokohama-web/src/lib.rs` (`input`, tests)
- Test: `grammar.rs`, `lib.rs`

**Interfaces:**
- Produces: `grammar::is_undo(line: &str) -> bool` (`undo` | `back`); in `lib.rs`, `fn undo_allowed(repl: &Repl, table: &TableDoc, viewer: PlayerId) -> Result<(), String>`.

- [ ] **Step 1: Write the failing cells**

`grammar.rs`: extend the existing allowlist cell — `assert!(!is_gameplay_line("privacy"))` and `assert!(refusal("privacy").contains("no privacy toggle"))`; `assert!(is_undo("undo") && is_undo("back") && !is_undo("1"))`.

`lib.rs`:

```rust
    #[tokio::test]
    async fn undo_is_admitted_mid_turn_and_refused_once_the_turn_has_passed() {
        let state = Arc::new(AppState::in_memory());
        start(&state, "undo", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        // One event of red's turn, then undo it: admitted, and the log shrinks by one.
        let (_, before) = get_as(&state, "red@wire.test", "/api/games/undo").await;
        let n0 = before["events"].as_u64().unwrap();
        let line = first_option(&before);
        let (_, after) = post_as(&state, "red@wire.test", "/api/games/undo/input", serde_json::json!({ "line": line })).await;
        assert!(after["events"].as_u64().unwrap() > n0, "the option applied: {after}");
        assert_eq!(after["to_move"], 0, "and the turn is still red's — pick a first option that does not end it");
        let (status, undone) = post_as(&state, "red@wire.test", "/api/games/undo/input", serde_json::json!({ "line": "undo" })).await;
        assert_eq!(status, StatusCode::OK, "{undone}");
        assert_eq!(undone["events"].as_u64().unwrap(), n0);
        // Play the turn out; blue is to move; red's undo is refused.
        play_turn(&state, "undo", "red@wire.test").await;
        let (status, body) = post_as(&state, "red@wire.test", "/api/games/undo/input", serde_json::json!({ "line": "undo" })).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "undo is not offered once your turn has passed");
        // Blue has not moved yet; blue's undo would cross red's turn boundary: refused too.
        let (status, _) = post_as(&state, "blue@wire.test", "/api/games/undo/input", serde_json::json!({ "line": "undo" })).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn undo_at_a_table_with_one_person_unwinds_the_bot_run_as_before() {
        let state = Arc::new(AppState::in_memory());
        start_vs_bot(&state, "undo-bot", 20260830, "red@wire.test").await;
        // Play red's whole turn; the bot then moves inside the same reply.
        let after = play_turn(&state, "undo-bot", "red@wire.test").await;
        assert_eq!(after["to_move"], 0, "the bot has played and it is red again");
        let n = after["events"].as_u64().unwrap();
        let (status, undone) = post_as(&state, "red@wire.test", "/api/games/undo-bot/input", serde_json::json!({ "line": "undo" })).await;
        assert_eq!(status, StatusCode::OK, "{undone}");
        assert!(undone["events"].as_u64().unwrap() < n, "the trailing bot run and red's move are gone");
    }

    #[tokio::test]
    async fn privacy_is_refused_at_every_table() {
        let state = Arc::new(AppState::in_memory());
        start(&state, "priv", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        let (status, body) = post_as(&state, "red@wire.test", "/api/games/priv/input", serde_json::json!({ "line": "privacy" })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("no privacy toggle"), "{body}");
    }
```

`first_option(&state)` returns the FIRST row's number of an `options` prompt; `start_vs_bot` creates `seats: [{user}, {bot: true}]` and keeps once.

- [ ] **Step 2: Implement**

`grammar.rs`: remove `"privacy"` from `HEADS`; `refusal`: when `line.trim() == "privacy"`, `"a seated table has no privacy toggle: every seat sees its own hand and no other"`; add `pub fn is_undo(line: &str) -> bool { matches!(line.trim(), "undo" | "back") }`.

`lib.rs`, in `input` after the acting-seat fence:

```rust
    if grammar::is_undo(&req.line) {
        undo_allowed(&repl, &table, viewer).map_err(|m| ApiError { status: StatusCode::FORBIDDEN, message: m, table_status: None })?;
    }
```

```rust
/// R-P6. ONE distinct person at the table (vs-bot, or one person in every human
/// seat): today's undo, untouched. Otherwise: only while the caller's own turn is
/// still in progress — the trailing event's mover is the caller's seat AND the
/// current player is still that seat. The mover of event `len-1` is
/// `state_at(record, len-1).current_player`, replay-derived, the CLI's own
/// attribution. Undo never crosses another person's turn boundary.
fn undo_allowed(repl: &Repl, table: &TableDoc, viewer: PlayerId) -> Result<(), String> {
    if table.distinct_humans() <= 1 {
        return Ok(());
    }
    let record = repl.session.record();
    let len = record.events.len();
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
```

- [ ] **Step 3: Run every gate, commit**

Run: `cargo test -p yokohama-web 2>&1 | grep -E 'test result|FAILED' && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -1`
Expected: green; the corpus unchanged (no payload moved).

```bash
git add crates/yokohama-web/src/grammar.rs crates/yokohama-web/src/lib.rs
git commit -m "feat(web): privacy leaves the grammar; undo is admitted mid-turn and refused once the turn has passed at a table with more than one person"
```

---

### Task 6: `GET …/events` — the signal, polled form

**Files:**
- Modify: `crates/yokohama-web/src/lib.rs` (`Events`, `EventsQuery`, handler, route, tests), `crates/yokohama-web/tests/wire_fixtures.rs` (captures `events`, `events_dealing`), `web/src/types.ts`, `web/src/api.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`

**Interfaces:**
- Produces: `Events { events: usize, to_move: Option<PlayerId>, status: &'static str, your_turn: bool }` at `GET /api/games/{id}/events?after=N`; `EventsQuery { after: Option<usize> }` (`deny_unknown_fields`); client `api.events(game, after?)` → `Events`; `types.ts` `Events { events: number; to_move: number | null; status: "dealing" | "playing" | "over"; your_turn: boolean }`.

- [ ] **Step 1: Write the failing cell**

```rust
    #[tokio::test]
    async fn events_answers_in_four_fields_at_every_status_and_fences_its_query() {
        let state = Arc::new(AppState::in_memory());
        let body = serde_json::json!({ "players": 2, "colors": ["Red", "Blue"], "seed": "20260831", "name": "sig",
            "seats": [{ "user": "red@wire.test" }, { "user": "blue@wire.test" }] });
        post_as(&state, "red@wire.test", "/api/games", body).await;
        let (status, e) = get_as(&state, "blue@wire.test", "/api/games/sig/events").await;
        assert_eq!(status, StatusCode::OK, "{e}");
        assert_eq!(e, serde_json::json!({ "events": 0, "to_move": null, "status": "dealing", "your_turn": false }));
        post_as(&state, "red@wire.test", "/api/games/sig/keep", serde_json::json!({ "seat": 0, "card": 1 })).await;
        post_as(&state, "blue@wire.test", "/api/games/sig/keep", serde_json::json!({ "seat": 1, "card": 1 })).await;
        let (_, e) = get_as(&state, "red@wire.test", "/api/games/sig/events?after=0").await;
        assert_eq!(e, serde_json::json!({ "events": 0, "to_move": 0, "status": "playing", "your_turn": true }));
        let (_, e) = get_as(&state, "blue@wire.test", "/api/games/sig/events").await;
        assert_eq!(e["your_turn"], false);
        let (status, err) = get_as(&state, "blue@wire.test", "/api/games/sig/events?afer=0").await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{err}");
        let (status, _) = get_as(&state, "green@wire.test", "/api/games/sig/events").await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
```

(The `over` arm is asserted by the whole-game gate: at the end of Task 10's two-identity arm, `events.status == "over"`.)

- [ ] **Step 2: Implement**

```rust
/// The signal (room plan §1, R-P9): a few bytes, no frame built, the lock held
/// only to read three fields. `after` is the resume cursor every later transport
/// reuses; this cycle validates it and answers at once whether or not `events`
/// moved. A dealing table answers without a session.
#[derive(Debug, Clone, Serialize)]
pub struct Events { pub events: usize, pub to_move: Option<PlayerId>, pub status: &'static str, pub your_turn: bool }

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventsQuery { #[serde(default)] pub after: Option<usize> }

async fn events(State(state): State<Arc<AppState>>, identity::Identity(user): identity::Identity, Path(id): Path<String>, QueryArgs(_q): QueryArgs<EventsQuery>) -> ApiResult<Events> {
    let table = state.games.table(&id, &user)?;
    if table.status == table::TableStatus::Dealing {
        return Ok(Json(Events { events: 0, to_move: None, status: "dealing", your_turn: false }));
    }
    let handle = state.games.handle(&id)?;
    let repl = handle.lock().expect("a game");
    let viewer = viewer_of(&repl, &table, &user)?;
    let s = repl.session.state();
    let over = repl.session.is_over();
    Ok(Json(Events {
        events: repl.session.record().events.len(),
        to_move: Some(s.current_player),
        status: if over { "over" } else { "playing" },
        your_turn: !over && s.current_player == viewer,
    }))
}
```

Route `.route("/api/games/{id}/events", get(events))`. `wire_fixtures.rs`: capture `events_dealing` (as red, right after create) and `events` (as blue, at the first `options_many` sighting — a real mid-game reading). Client: `types.ts` `Events`; `api.events`; `contract.ts` `_events`, `_eventsDealing`, `KEYS.events = ["events","to_move","status","your_turn"]`, `wire.events`, `wire.eventsDealing`; two `contract.test.ts` rows.

- [ ] **Step 3: Regenerate, gate, commit**

Run: the corpus regeneration and both suites as in Task 4's Step 4.
Expected: `events.json`, `events_dealing.json` new; nothing else moves.

```bash
git add crates/yokohama-web/src/lib.rs crates/yokohama-web/tests/wire_fixtures.rs web/src/types.ts web/src/api.ts web/src/wire/contract.ts web/src/wire/contract.test.ts web/src/wire/events.json web/src/wire/events_dealing.json
git commit -m "feat(web): GET …/events — the change signal in four fields with a resume cursor, answered at once, seated only"
```

### Task 7: The client — sign-in, the "You are" strip, the games list as tables

**Files:**
- Create: `web/src/SignIn.tsx`, `web/src/SignIn.test.tsx`, `web/src/whoami.ts` (+ `.test.ts`)
- Modify: `web/src/api.ts` (`setClaimedUser`, `clearClaimedUser`), `web/src/App.tsx` (+ `.test.tsx`), `web/src/GameMenu.tsx` (+ `.test.tsx`), `web/src/NewGame.tsx` (the saved-games section reads `GamesList`), `web/src/test/harness.tsx` (`bootstrapReply` answers `/api/me` and the list), `web/src/styles.css`

**Interfaces:**
- Produces: `api.setClaimedUser(email)`, `api.clearClaimedUser()`; `whoami.ts`: `type Door = { mode: Me["mode"]; user: string | null }`, `useMe(): { me: Me | null; error: string | null; refresh(): void }`; `<SignIn onSignedIn={(email) => void} />`; `App` renders, in order: nothing until `/api/me` answers, `SignIn` when `mode === "claimed" && user === null`, a `.door-error` screen when `user === null` under another mode, else the existing screens; the banner gains `<span className="meta who">You are {email}</span>` and, in claimed mode, a `sign out` ghost button. `GameMenu` and `NewGame`'s saved list render `TableView` rows: `name ?? id`, `players`P, status, `your turn` / `waiting on <email or "the bot">` / `over`.

- [ ] **Step 1: Write the failing cells**

`SignIn.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SignIn } from "./SignIn";
import { CLAIMED_USER_KEY } from "./api";

describe("the sign-in screen", () => {
  it("stores the typed email under the key api.ts reads and hands it up", async () => {
    const onSignedIn = vi.fn();
    render(<SignIn onSignedIn={onSignedIn} />);
    const user = userEvent.setup();
    await user.type(screen.getByLabelText("Email"), "  Red@Wire.TEST ");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBe("red@wire.test");
    expect(onSignedIn).toHaveBeenCalledWith("red@wire.test");
  });

  it("refuses a value with no @ and stores nothing", async () => {
    render(<SignIn onSignedIn={vi.fn()} />);
    const user = userEvent.setup();
    await user.type(screen.getByLabelText("Email"), "nobody");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    expect(screen.getByText(/is not an email/)).toBeInTheDocument();
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBeNull();
  });
});
```

`App.test.tsx` (a new `describe("the door")`):

```tsx
  it("shows sign-in when the door is claimed and nobody is signed in, then the games list", async () => {
    const stub = stubApi((call) => {
      if (call.path.endsWith("/api/me")) return localStorage.getItem(CLAIMED_USER_KEY) ? wire.me : wire.meSignedOut;
      if (call.path.endsWith("/api/games")) return wire.games;
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/api/art")) return wire.artManifest;
      return undefined;
    });
    render(<App />);
    await screen.findByRole("heading", { name: "Sign in" });
    expect(stub.matching("GET", "/api/games")).toHaveLength(0);
    await userEvent.setup().type(screen.getByLabelText("Email"), wire.me.user!);
    await userEvent.setup().click(screen.getByRole("button", { name: "Sign in" }));
    await screen.findByText(`You are ${wire.me.user}`);
    expect(stub.matching("GET", "/api/games/").length).toBe(0);
    expect(stub.matching("GET", "/api/me").length).toBeGreaterThanOrEqual(2);
  });

  it("names the door rather than offering a form when a non-claimed deployment has no user", async () => {
    stubApi((call) => (call.path.endsWith("/api/me") ? { mode: "cf-access", user: null } : undefined));
    render(<App />);
    await screen.findByText(/Cf-Access-Authenticated-User-Email/);
    expect(screen.queryByLabelText("Email")).toBeNull();
  });

  it("a 401 mid-game returns to sign-in", async () => {
    const stub = await renderApp(wire.options, (call) =>
      call.path.endsWith("/input") ? { status: 401, body: { error: "no signed-in user: send X-Yokohama-Claimed-User" } } : undefined,
    );
    await userEvent.setup().click(firstOptionButton());
    await screen.findByRole("heading", { name: "Sign in" });
    expect(stub.matching("POST", "/input")).toHaveLength(1);
  });
```

`firstOptionButton()` is `() => document.querySelector<HTMLButtonElement>("button.option:not([disabled])")!`, defined once at the top of `App.test.tsx`.

`GameMenu.test.tsx`: one cell rendering the menu with `/api/games` answering `wire.games` and asserting each row's text carries the table's `name ?? id`, its status word, and `your turn` on the row whose `to_move` is in `my_seats`; the switch click calls `api.get(id)` as before.

`harness.tsx` `bootstrapReply`: `/api/me` → `wire.me`; the list → `wire.games`; `renderApp` seeds `localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!)` before rendering, and finds the game to open by the button named `state.id` — which the list now renders from `wire.games`, so `renderApp` answers `GET /api/games` with a `GamesList` whose first table has `id: state.id` (`{ games: [{ ...wire.games.games[0], id: state.id }] }` — a fixture with ONE field overridden, allowed here because the override is the identity join `renderApp` has always made; say so in its comment).

- [ ] **Step 2: Run to see them fail; implement**

`api.ts`:

```ts
export function setClaimedUser(email: string): void {
  try { localStorage.setItem(CLAIMED_USER_KEY, email); } catch { /* storage refused: the header is simply not sent */ }
}
export function clearClaimedUser(): void {
  try { localStorage.removeItem(CLAIMED_USER_KEY); } catch { /* nothing to clear */ }
}
```

`whoami.ts`:

```ts
import { useCallback, useEffect, useState } from "react";
import { api, ApiError } from "./api";
import type { Me } from "./types";

/** `/api/me`, once at mount and again on demand. `null` until it answers. */
export function useMe(): { me: Me | null; error: string | null; refresh: () => void } {
  const [me, setMe] = useState<Me | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  useEffect(() => {
    let live = true;
    api.me().then((m) => live && setMe(m)).catch((e) => live && setError(e instanceof ApiError ? e.message : String(e)));
    return () => { live = false; };
  }, [tick]);
  return { me, error, refresh: useCallback(() => setTick((t) => t + 1), []) };
}

/** The sentence the error screen shows for a null user under a non-claimed door. */
export function doorSentence(mode: Me["mode"]): string {
  return mode === "cf-access"
    ? "This deployment expects Cloudflare Access to name you (Cf-Access-Authenticated-User-Email), and nothing reached the server. Open the site through its public hostname."
    : "This deployment expects the edge Worker to name you (X-Yokohama-User), and nothing reached the server.";
}
```

`SignIn.tsx`: a `.setup` block with `<h2>Sign in</h2>`, a `<label htmlFor="email">Email</label><input id="email" …/>`, the same shape check as the server's R-1.1 (`/^[^\s@]+@[^\s@]+$/` after trim + lowercase; on failure `<div className="error">{value} is not an email</div>`), `setClaimedUser(normalized)` then `onSignedIn(normalized)`, and one line of prose: "This is a development door: the name you type is the name you play under. Behind Cloudflare Access it is still Access that lets you in."

`App.tsx`: `const door = useMe();` at the top; before the `if (!state)` branch: `if (door.error) return <div className="setup door-error">{door.error}</div>; if (!door.me) return null; if (door.me.mode === "claimed" && !door.me.user) return <SignIn onSignedIn={() => door.refresh()} />; if (!door.me.user) return <div className="setup door-error">{doorSentence(door.me.mode)}</div>;`. The banner (both the game screen's and the set-up screens') gains `<span className="meta who">You are {door.me.user}</span>` and, when `mode === "claimed"`, `<button className="ghost" onClick={() => { clearClaimedUser(); door.refresh(); }}>sign out</button>`. In `send`'s catch (and the scrub's, and `GameMenu`'s `said`): when the `ApiError` message starts with `no signed-in user`, call `door.refresh()` (which re-reads `/api/me`, now null, and routes to sign-in) — thread `onSignedOut={door.refresh}` into `GameMenu`.

`GameMenu.tsx` and `NewGame.tsx`'s saved list: `api.list()` is a `GamesList`; each row is `<button>{t.name ?? t.id}</button> <span className="muted">{t.players}P · {tableWord(t)}</span>` where `tableWord` is `over ? "over" : status === "dealing" ? "dealing" : my_seats.includes(to_move) ? "your turn" : "waiting on " + holder(t, to_move)` and `holder` reads `seats[to_move]` (`human` → `user`, `bot` → `"the bot"`); put `tableWord`/`holder` in `whoami.ts`… no — in a new `tables.ts` beside it (one responsibility: words about a `TableView`), with three cells.

- [ ] **Step 3: Gate and commit**

Run: `cd web && npm run typecheck && npm test 2>&1 | grep -E 'Tests |FAIL'`
Expected: green at the measured count; e2e still red (Task 8).

```bash
git add web/src/SignIn.tsx web/src/SignIn.test.tsx web/src/whoami.ts web/src/whoami.test.ts web/src/tables.ts web/src/tables.test.ts web/src/api.ts web/src/App.tsx web/src/App.test.tsx web/src/GameMenu.tsx web/src/GameMenu.test.tsx web/src/NewGame.tsx web/src/test/harness.tsx web/src/styles.css
git commit -m "feat(web client): the door — sign-in under a claimed deployment, the You-are strip, a 401 returning to sign-in, and the games list as the tables you sit at"
```

---

### Task 8: The client — the per-seat form and the dealing screen

**Files:**
- Create: `web/src/Dealing.tsx`, `web/src/Dealing.test.tsx`
- Modify: `web/src/NewGame.tsx` (+ `.test.tsx`), `web/src/App.tsx` (+ `.test.tsx`: opening a game by id reads `/table` first), `web/e2e/harness.ts` (`startGame` through sign-in → seats → deal → keep), `web/src/styles.css`, `web/src/Help.tsx` (+ `.test.tsx`)

**Interfaces:**
- `NewGame` props: `{ art, labels, sections, onToggleSection, me: string, onCreated: (table: TableView) => void, onOpen: (id: string) => void }`; it posts §4.2's body and calls `onCreated` with the 201.
- `Dealing` props: `{ game: string; me: string; art; labels; sections; onToggleSection; onPlaying: () => void }`; it fetches `/deal`, draws the set table with the caller's cards, posts `/keep`, re-fetches `/deal` after every keep, and polls `/events` every 3 s while `status === "dealing"` (the poll hook arrives in Task 9; here a plain `setInterval` inside `Dealing` that Task 9 replaces — say so in a comment); on `status !== "dealing"` it calls `onPlaying`.
- `App`: `open(id)` = `api.table(id)` → `dealing` ? show `<Dealing>` : `show(await api.get(id), false)`; `onCreated` → `open(table.id)`.

- [ ] **Step 1: Write the failing cells**

`NewGame.test.tsx` (the file is rewritten; the seed cells that matter are kept in spirit):

```tsx
  it("posts one seat entry per seat, me by default, and the bot only at two players", async () => {
    const stub = stubApi((call) => {
      if (call.method === "POST" && call.path.endsWith("/api/games")) return { status: 201, body: wire.tableDealing };
      if (call.path.endsWith("/api/games")) return wire.games;
      return undefined;
    });
    const onCreated = vi.fn();
    render(<Host me="red@wire.test" onCreated={onCreated} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Deal" }));
    const body = stub.matching("POST", "/api/games")[0]!.body as { seats: unknown[]; keeps?: unknown };
    expect(body.seats).toEqual([{ user: "red@wire.test" }, { bot: true }]);
    expect("keeps" in body).toBe(false);
    expect(onCreated).toHaveBeenCalledWith(wire.tableDealing);
  });

  it("at three players every seat defaults to me and no bot is offered", async () => {
    stubApi(() => undefined);
    render(<Host me="red@wire.test" onCreated={vi.fn()} />);
    await userEvent.setup().selectOptions(screen.getByLabelText("Players"), "3");
    const seats = screen.getAllByLabelText(/^Seat \d/);
    expect(seats).toHaveLength(3);
    for (const s of seats) expect(within(s.parentElement!).queryByRole("option", { name: "bot" })).toBeNull();
    expect(screen.getByText(/measured at two players only/)).toBeInTheDocument();
  });

  it("a seat set to another email posts that email", async () => {
    const stub = stubApi((call) => call.method === "POST" ? { status: 201, body: wire.tableDealing } : wire.games);
    render(<Host me="red@wire.test" onCreated={vi.fn()} />);
    const user = userEvent.setup();
    await user.selectOptions(screen.getByLabelText("Seat 1"), "email");
    await user.type(screen.getByLabelText("Seat 1 email"), "Blue@Wire.test");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    expect((stub.matching("POST", "/api/games")[0]!.body as { seats: unknown[] }).seats[1]).toEqual({ user: "blue@wire.test" });
  });

  it("a big seed survives to the create body as a string", async () => { /* the BIG_SEED cell, re-pointed at the create body's `seed` */ });
```

`Dealing.test.tsx`:

```tsx
  it("draws my two cards over the set table, posts my keep, and re-asks for the next of my seats", async () => {
    let deals = [wire.deal, wire.dealWaiting];
    const stub = stubApi((call) => {
      if (call.path.endsWith("/deal")) return deals.shift() ?? wire.dealWaiting;
      if (call.path.endsWith("/keep")) return wire.table;
      if (call.path.endsWith("/events")) return wire.eventsDealing;
      return undefined;
    });
    render(<Host game="wire" me="red@wire.test" onPlaying={vi.fn()} />);
    const picks = await screen.findAllByRole("button", { name: /^keep O\d\d/ });
    expect(picks).toHaveLength(2);
    expect(screen.getByText(wire.deal.keep_note.trim().replace(/\s+/g, " ").slice(0, 30), { exact: false })).toBeInTheDocument();
    await userEvent.setup().click(picks[1]!);
    expect(stub.matching("POST", "/keep")[0]!.body).toEqual({ seat: wire.deal.seat, card: 2 });
    await screen.findByText(/waiting on/);
    expect(stub.matching("GET", "/deal")).toHaveLength(2);
  });

  it("opens the game when the signal says playing", async () => {
    vi.useFakeTimers();
    const onPlaying = vi.fn();
    let status = "dealing";
    stubApi((call) => {
      if (call.path.endsWith("/deal")) return wire.dealWaiting;
      if (call.path.endsWith("/events")) return { ...wire.eventsDealing, status };
      return undefined;
    });
    render(<Host game="wire" me="red@wire.test" onPlaying={onPlaying} />);
    await act(async () => { await vi.advanceTimersByTimeAsync(3100); });
    expect(onPlaying).not.toHaveBeenCalled();
    status = "playing";
    await act(async () => { await vi.advanceTimersByTimeAsync(3100); });
    expect(onPlaying).toHaveBeenCalledTimes(1);
    vi.useRealTimers();
  });

  it("never draws another seat's cards: the DOM holds exactly my two card faces", async () => {
    stubApi((call) => call.path.endsWith("/deal") ? wire.deal : call.path.endsWith("/events") ? wire.eventsDealing : undefined);
    const { container } = render(<Host game="wire" me="red@wire.test" onPlaying={vi.fn()} />);
    await screen.findAllByRole("button", { name: /^keep/ });
    expect(container.querySelectorAll(".deal .card-face.order:not(.back)")).toHaveLength(2);
    expect(container.querySelectorAll(".deal-seat.human .card-face.back")).toHaveLength(0);
  });
```

In both test files `Host` is a wrapper holding `useSections()` exactly as `NewGame.test.tsx`'s `Host` does today, passing `sections`/`onToggleSection` down and forwarding the other props.

`App.test.tsx`: `it("opening a saved game reads its table first and shows the dealing screen when it is dealing", …)` — `/table` answers `wire.tableDealing` → the dealing screen's heading `Choose your card` appears and `GET /api/games/<id>` is never called; and the mirror cell where `/table` answers `wire.table` → `api.get` is called and the game opens.

- [ ] **Step 2: Implement**

`NewGame.tsx` is the FORM only now (the set table moved to `Dealing.tsx`): players, seed, name, and one `.field` per seat — `<label htmlFor={`seat-${i}`}>Seat {i}</label><select id=…>` with options `me` / `email` / (`bot` when `players === 2`), and when `email` a text input labelled `Seat {i} email`; the lede reads "Seat one bot at two players; at three and four everyone is a person — the bot is measured at two players only." The body: `seats: rows.map((r) => r.kind === "bot" ? { bot: true } : { user: r.kind === "me" ? me : normalize(r.email) })`; `Deal` posts and calls `onCreated(table)`. The saved list stays (Task 7's rows) and its row click calls `onOpen(id)`. `turn`, `passing`, `chosen`, `preview`, `DealBacks` for humans, the keep-hedge copy and `api.preview` LEAVE this file.

`Dealing.tsx`: on mount and after every keep, `api.deal(game)`; render the M5.5 set table (`Board` over `deal.frame.frame.board`, `TableSections` with `game={null} state={null} view={deal.frame}`) — copied from the old `NewGame` set-table branch; the side column lists the other seats from `deal.table.seats` × `deal.table.chosen` as "chosen" / "waiting" (a bot: "keeps one of its two cards — unseen" with `DealBacks`); the deck holds `deal.cards` as two `.deal-pick` buttons (`aria-label={`keep ${pad(c.card.id)} for seat ${deal.seat}`}`) whose click posts `api.keep(game, deal.seat!, n)` then re-fetches `/deal`; `deal.keep_note` rendered under them (whitespace collapsed); when `deal.seat === null`: `<p>waiting on {names of unkept humans}</p>`; a `setInterval(3000)` calling `api.events(game)` while mounted, `onPlaying()` when `status !== "dealing"`. `DealBacks` moves here.

`App.tsx`: `const open = useCallback(async (id: string) => { const t = await api.table(id); if (t.status === "dealing") setDealing(id); else { setDealing(null); show(await api.get(id), false); } }, [show])` with `const [dealing, setDealing] = useState<string | null>(null)`; the `!state` branch renders `<Dealing … onPlaying={() => void open(dealing)} />` when `dealing`, else `<NewGame … onCreated={(t) => void open(t.id)} onOpen={(id) => void open(id)} />`. `GameMenu`'s `onSwitch` goes through `open` too (a dealing table in the menu opens its dealing screen).

`e2e/harness.ts` `startGame`: `page.goto("/")` → `getByRole("heading", {name: "Sign in"})` → fill `Email` with `walker@e2e.test` → click `Sign in` → `findByRole("heading", {name: "New game"})` → players select → the `created` response wait on `POST …/api/games` (201) → click `Deal` → read `id` from the 201 body; then `for (;;)`: wait for either the game banner or a `keep` button; click the first `keep …` button; break when the end-trigger strip is visible. The `seed` for the returned `Game` comes from `page.request` — NO: the seed never leaves the server now; drop `seed` from `Game` and print `id` alone in every `console.log` that named it (the `deadEnd` message included). Delete the `addInitScript` stand-in from Task 1. The 3P/4P walks (`the_screen_itself.spec.ts`, `the_table.spec.ts` width cells) set every seat to `me` by default, so the loop above keeps for every seat in turn — this is the hot-seat sequence by construction.

`Help.tsx` "Starting a game": rewrite to the new flow (sign in; one row per seat; the table opens dealing; each seat sees only its own two cards; "waiting on" until everyone has chosen; the bot keeps unseen). `Help.test.tsx`'s glued-letter rule runs over it as it does over every paragraph.

- [ ] **Step 3: Gate, including the browser walk, and commit**

Run: `cd web && npm run typecheck && npm test 2>&1 | grep -E 'Tests |FAIL' && npm run e2e 2>&1 | tail -6`
Expected: component suite green at the measured count; the browser walk GREEN again — 36 browser cells + the structural one; the whole-game cell prints its move count.

```bash
git add web/src/NewGame.tsx web/src/NewGame.test.tsx web/src/Dealing.tsx web/src/Dealing.test.tsx web/src/App.tsx web/src/App.test.tsx web/src/Help.tsx web/src/Help.test.tsx web/e2e/harness.ts web/src/styles.css
git commit -m "feat(web client): the per-seat form and the dealing screen — each seat keeps from its own view, the hot-seat pass retires, and the browser walk signs in and deals through the new flow"
```

---

### Task 9: The client — the visible-tab poll and the walk from it

**Files:**
- Create: `web/src/signal.ts`, `web/src/signal.test.ts`
- Modify: `web/src/App.tsx` (+ `.test.tsx`), `web/src/Dealing.tsx` (uses the hook), `web/src/styles.css` (`.link`)

**Interfaces:**
- `signal.ts`: `VISIBLE_MS = 3000`; `BACKOFF_MS = [2000, 4000, 8000, 16000, 30000]`; `nextDelay(failures: number): number`; `type Link = "live" | "refreshing" | "reconnecting"`; `useSignal(game: string | null, active: boolean, onSignal: (e: Events) => void): Link`.
- `App`: `landed(game, prev, next: number)` (was `replied: ClientState`); a `.link` strip in the banner reading `refreshing…` / `reconnecting…` and nothing when live.

- [ ] **Step 1: Write the failing cells**

`signal.test.ts`:

```ts
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useSignal, nextDelay, BACKOFF_MS, VISIBLE_MS } from "./signal";
import { stubApi } from "./test/harness";
import { wire } from "./wire/contract";

function visibility(state: "visible" | "hidden") {
  Object.defineProperty(document, "visibilityState", { value: state, configurable: true });
  document.dispatchEvent(new Event("visibilitychange"));
}

describe("the poll", () => {
  beforeEach(() => { vi.useFakeTimers(); visibility("visible"); });
  afterEach(() => { vi.useRealTimers(); });

  it("asks every three seconds while visible and active, and not at all when inactive", async () => {
    const stub = stubApi((c) => (c.path.endsWith("/events") ? wire.events : undefined));
    const { rerender } = renderHook(({ active }) => useSignal("wire", active, () => {}), { initialProps: { active: true } });
    await act(async () => { await vi.advanceTimersByTimeAsync(VISIBLE_MS * 3 + 10); });
    expect(stub.matching("GET", "/events")).toHaveLength(3);
    rerender({ active: false });
    await act(async () => { await vi.advanceTimersByTimeAsync(VISIBLE_MS * 3); });
    expect(stub.matching("GET", "/events")).toHaveLength(3);
  });

  it("asks nothing from a hidden tab, then once on return with a refreshing state (R-P8)", async () => {
    const stub = stubApi((c) => (c.path.endsWith("/events") ? wire.events : undefined));
    const { result } = renderHook(() => useSignal("wire", true, () => {}));
    visibility("hidden");
    await act(async () => { await vi.advanceTimersByTimeAsync(VISIBLE_MS * 5); });
    expect(stub.matching("GET", "/events")).toHaveLength(0);
    act(() => visibility("visible"));
    expect(result.current).toBe("refreshing");
    await act(async () => { await vi.advanceTimersByTimeAsync(10); });
    expect(stub.matching("GET", "/events")).toHaveLength(1);
    expect(result.current).toBe("live");
  });

  it("backs off on failure and says reconnecting, then recovers", async () => {
    let fail = true;
    const stub = stubApi((c) => (c.path.endsWith("/events") ? (fail ? { status: 502, body: { error: "gone" } } : wire.events) : undefined));
    const { result } = renderHook(() => useSignal("wire", true, () => {}));
    await act(async () => { await vi.advanceTimersByTimeAsync(VISIBLE_MS + 10); });
    expect(result.current).toBe("reconnecting");
    await act(async () => { await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! + 10); });
    expect(stub.matching("GET", "/events")).toHaveLength(2);
    fail = false;
    await act(async () => { await vi.advanceTimersByTimeAsync(BACKOFF_MS[1]! + 10); });
    expect(result.current).toBe("live");
    expect(nextDelay(0)).toBe(BACKOFF_MS[0]);
    expect(nextDelay(99)).toBe(BACKOFF_MS[BACKOFF_MS.length - 1]);
  });
});
```

`App.test.tsx` (`describe("the opponent's turn arrives by the signal")`):

```tsx
  it("walks the new events when the count grows, and reloads without a walk when it shrinks", async () => {
    vi.useFakeTimers();
    let events = wire.watching.events;
    const stub = await renderApp(wire.watching, (call) => {
      if (call.path.endsWith("/events")) return { events, to_move: 0, status: "playing", your_turn: false };
      if (call.path.endsWith(`/api/games/${wire.watching.id}`)) return { ...wire.watching, events };
      return undefined;
    });
    events = wire.watching.events + 2;
    await act(async () => { await vi.advanceTimersByTimeAsync(VISIBLE_MS + 10); });
    await waitFor(() => expect(stub.matching("GET", "/at/").length).toBeGreaterThanOrEqual(3));
    expect(screen.getByText(/skip/)).toBeInTheDocument();
    events = wire.watching.events;
    await act(async () => { await vi.advanceTimersByTimeAsync(VISIBLE_MS * 3); });
    expect(stub.matching("GET", `/api/games/${wire.watching.id}`).length).toBeGreaterThanOrEqual(3);
    vi.useRealTimers();
  });
```

(`wire.watching` is the seat-1 payload while seat 0 is to move — the one shape where `your_turn` is false at rest and the poll is active. `renderApp` must NOT answer `/events` itself; it is the cell's.)

- [ ] **Step 2: Implement `signal.ts`**

```ts
import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import type { Events } from "./types";

export const VISIBLE_MS = 3000;
export const BACKOFF_MS = [2000, 4000, 8000, 16000, 30000] as const;
export type Link = "live" | "refreshing" | "reconnecting";

export function nextDelay(failures: number): number {
  return BACKOFF_MS[Math.min(Math.max(failures, 0), BACKOFF_MS.length - 1)]!;
}

/** The polled form of the change signal (room plan §1). Asks `/events` every
 *  `VISIBLE_MS` while the tab is VISIBLE and `active`; never from a hidden tab
 *  (R-P8) — on return it asks once under "refreshing"; on a failure it backs
 *  off under "reconnecting". M6.3 replaces the inside of this hook; nothing
 *  downstream of `onSignal` changes. */
export function useSignal(game: string | null, active: boolean, onSignal: (e: Events) => void): Link {
  const [link, setLink] = useState<Link>("live");
  const failures = useRef(0);
  const latest = useRef(onSignal);
  latest.current = onSignal;

  useEffect(() => {
    if (!game || !active) return;
    let timer: ReturnType<typeof setTimeout> | null = null;
    let stopped = false;
    const visible = () => document.visibilityState === "visible";

    const ask = async () => {
      if (stopped || !visible()) return;
      try {
        const e = await api.events(game);
        if (stopped) return;
        failures.current = 0;
        setLink("live");
        latest.current(e);
        schedule(VISIBLE_MS);
      } catch {
        if (stopped) return;
        setLink("reconnecting");
        const wait = nextDelay(failures.current);
        failures.current += 1;
        schedule(wait);
      }
    };
    const schedule = (ms: number) => {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => void ask(), ms);
    };
    const onVisibility = () => {
      if (visible()) {
        setLink("refreshing");
        schedule(0);
      } else if (timer) {
        clearTimeout(timer);
        timer = null;
      }
    };
    document.addEventListener("visibilitychange", onVisibility);
    if (visible()) schedule(VISIBLE_MS);
    return () => {
      stopped = true;
      if (timer) clearTimeout(timer);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [game, active]);

  return link;
}
```

`App.tsx`: `landed` takes `next: number`; callers pass `replied.events`. Add:

```tsx
  const link = useSignal(
    state?.id ?? null,
    !!state && !state.over && !state.your_turn && !busy,
    useCallback(async (e: Events) => {
      if (!state) return;
      const game = state.id;
      if (e.events === state.events) return;
      setBusy(true);
      try {
        const fresh = await api.get(game);
        if (gameId.current !== game) return;
        setState(fresh);
        if (e.events > state.events) await landed(game, state.events, fresh.events);
        else { setRun(null); setWalk(null); setFresh(false); }
      } catch (err) {
        if (gameId.current === game) setError(err instanceof ApiError ? err.message : String(err));
      } finally {
        setBusy(false);
      }
    }, [state, landed]),
  );
```

and in the banner `{link !== "live" && <span className="meta link" role="status">{link}…</span>}`. `Dealing.tsx`: replace its `setInterval` with `useSignal(game, true, (e) => { if (e.status !== "dealing") onPlaying(); })` and render the same `.link` strip.

- [ ] **Step 3: Gate and commit**

Run: `cd web && npm run typecheck && npm test 2>&1 | grep -E 'Tests |FAIL' && npm run e2e 2>&1 | tail -4`
Expected: green; the browser walk green (a vs-bot game never activates the poll — its `your_turn` is true at every rest — so this task's browser evidence is Task 10's two-browser cell).

```bash
git add web/src/signal.ts web/src/signal.test.ts web/src/App.tsx web/src/App.test.tsx web/src/Dealing.tsx web/src/styles.css
git commit -m "feat(web client): the polled signal — three seconds from a visible tab, nothing from a hidden one, refreshing on return, reconnecting with backoff; the opponent's turn walks from it and an undo reloads without a walk"
```

### Task 10: `preview` retires; the seed pin loses its exemption; the two-identity gate arm; the two-browser walk

**Files:**
- Modify: `crates/yokohama-web/src/games.rs` (delete `Preview`, `PreviewError`, `preview`, the old `Deal`; `set_table` stays for `deal`), `crates/yokohama-web/src/lib.rs` (delete `preview_game`, its route and its three cells), `crates/yokohama-web/tests/wire_fixtures.rs` (no preview captures; the seed pin), `crates/yokohama-web/tests/play_a_whole_game.rs` (the second arm), `web/src/types.ts` (delete `Preview`, the old `Deal`), `web/src/api.ts` (delete `preview`), `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`, `web/src/App.test.tsx:1044`, `web/e2e/harness.ts` (`signIn(page, email)`, `openFromList`)
- Create: `web/e2e/two_browsers.spec.ts`

- [ ] **Step 1: The seed pin without its exemption**

Rename `no_committed_payload_carries_the_setup_seed_except_the_preview` → `no_committed_payload_carries_the_setup_seed`, delete the `preview` arm, and add the anti-vacuity partner:

```rust
    // The search cannot pass by searching for nothing: the same seed IS in the
    // table document the memory store holds for the walk's game.
    let (doc, _) = api.state.games.store().read(yokohama_web::store::Doc::Table, "wire").expect("readable").expect("the walk's table");
    assert!(doc.contains(&decimal), "the seed is in the table document and nowhere on the wire");
```

(`Api` keeps its `Arc<AppState>` as `state` so the test can reach the store.) Also search each payload for the seed's HEX form (`format!("{:016x}", SETUP_SEED)`) beside the decimal.

- [ ] **Step 2: The second gate arm**

In `play_a_whole_game.rs`:

```rust
#[tokio::test]
async fn two_people_play_to_a_winner_and_each_sees_only_their_own_hand() {
    let api = Api::new();
    let (red, blue) = ("red@wire.test", "blue@wire.test");
    let (status, table) = api.call_as(red, "POST", "/api/games", Some(serde_json::json!({
        "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": 20260914u64, "name": "two",
        "seats": [{ "user": red }, { "user": blue }],
    }))).await;
    assert_eq!(status, StatusCode::CREATED, "{table}");
    for (user, seat) in [(red, 0), (blue, 1)] {
        let (status, _) = api.call_as(user, "POST", "/api/games/two/keep", Some(serde_json::json!({ "seat": seat, "card": 1 }))).await;
        assert_eq!(status, StatusCode::OK);
    }
    let mut rng = Rng(0x5457_4F50_4C41_5945); // "TWOPLAYE"
    let mut inputs = 0usize;
    let (_, mut state) = api.call_as(red, "GET", "/api/games/two", None).await;
    while state["over"] != serde_json::Value::Bool(true) {
        inputs += 1;
        assert!(inputs < 40_000, "a game must finish inside the cap; last state: {state}");
        let mover = if state["to_move"] == 0 { red } else { blue };
        let (_, mine) = api.call_as(mover, "GET", "/api/games/two", None).await;
        assert_eq!(mine["seat"], state["to_move"], "the payload is the requester's seat");
        let players = mine["frame"]["dashboard"].as_array().unwrap().iter().find_map(|p| p.get("Players")).unwrap();
        for p in players["players"].as_array().unwrap() {
            assert_eq!(p["hand_hidden"], p["seat"] != mine["seat"], "{mover}: {p}");
        }
        let line = choose(&mine, &mut rng);   // the same policy as the first arm, factored out
        let (status, next) = api.call_as(mover, "POST", "/api/games/two/input", Some(serde_json::json!({ "line": line }))).await;
        assert_eq!(status, StatusCode::OK, "{next}");
        state = next;
    }
    let (_, e) = api.call_as(red, "GET", "/api/games/two/events", None).await;
    assert_eq!(e["status"], "over");
    let (status, _) = api.call_as(blue, "GET", "/api/games/two/result", None).await;
    assert_eq!(status, StatusCode::OK);
}
```

Factor the first arm's `match prompt["kind"]` into `fn choose(state: &Value, rng: &mut Rng) -> String` so both arms play by one policy. **This arm places nothing either** (the first arm's `"place_a" => "confirm"`), and its own doc must say what the first arm's does: a bot-free game that never places may not end — so this arm PROBES a placement exactly as `wire_fixtures.rs`'s walk does (copy its `place_a` arm: tiles with a President first, then a rotated order, `A{area:02}`, cache the hit). Say so in the test's doc.

- [ ] **Step 3: Remove `preview`**

Delete `preview_game`, the route, `Preview`, `PreviewError`, `preview`, the old `Deal`, the two `preview` corpus captures and the `"preview"` handling in the seed pin, the three `lib.rs` preview cells (`the_preview_serves_the_set_table_and_withholds_the_bots_deal`, `a_preview_the_server_cannot_build_is_a_500_and_a_bad_request_is_still_a_400` — its 500-vs-400 discrimination moves onto `deal`: a `deal` for a table whose `seed` cannot deal is a 500, asserted by writing a table document with `players: 9` directly into the store and asking for its deal), and the three `games.rs` preview cells whose claims now live on `deal` (`the_deal_serves_the_callers_own_cards_and_never_the_others` already asserts the withholding; add `the_deal_is_the_position_after_setup` asserting `frame.index == 0 && frame.total == 0`). Client: `Preview`, old `Deal`, `api.preview`, `_preview`, `_previewBot`, `KEYS.preview`, `KEYS.deal` (the OLD deal), `wire.preview`, `wire.previewBot`, their `contract.test.ts` rows and `App.test.tsx:1044`'s stub go.

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures 2>&1 | tail -3 && cargo test -p yokohama-web 2>&1 | grep -E 'test result|FAILED' && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -1 && cd web && npm run typecheck && npm test 2>&1 | grep -E 'Tests |FAIL'`
Expected: `preview.json`, `preview_bot.json` removed by the regeneration; the gate reads `2 passed`; everything else green.

- [ ] **Step 4: The two-browser walk**

`web/e2e/harness.ts`: `export async function signIn(page: Page, email: string)` (goto `/`, the sign-in form, wait for the `New game` heading) and `export async function openFromList(page: Page, id: string)` (the saved-games row named `id`). `startGame` uses `signIn`.

`web/e2e/two_browsers.spec.ts`:

```ts
import { expect, test, type Page } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { act, deadEnd, openFromList, readScreen, rng, signIn, watchForBreakage } from "./harness";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

const CAP = 4000;

/** The card ids in a page's OWN hand panel — the one place another seat's cards could leak. */
async function ownHand(page: Page): Promise<string[]> {
  return page.locator("aside.side .hand .card-face.order").evaluateAll((els) =>
    els.map((e) => e.getAttribute("data-card") ?? e.textContent ?? ""),
  );
}

test("two people at two browsers play to a winner and neither sees the other's hand", async ({ browser }) => {
  const a = await (await browser.newContext()).newPage();
  const b = await (await browser.newContext()).newPage();
  const watchA = watchForBreakage(a);
  const watchB = watchForBreakage(b);

  await signIn(a, "a@e2e.test");
  await a.getByLabel("Seat 1").selectOption("email");
  await a.getByLabel("Seat 1 email").fill("b@e2e.test");
  const created = a.waitForResponse((r) => /\/api\/games$/.test(new URL(r.url()).pathname) && r.request().method() === "POST");
  await a.getByRole("button", { name: "Deal" }).click();
  const id = String(((await (await created).json()) as { id: string }).id);
  await a.getByRole("button", { name: /^keep O\d\d/ }).first().click();
  await expect(a.getByText(/waiting on b@e2e\.test/)).toBeVisible();

  await signIn(b, "b@e2e.test");
  await openFromList(b, id);
  await b.getByRole("button", { name: /^keep O\d\d/ }).first().click();
  await expect(b.getByText(/end triggers:/)).toBeVisible();
  await expect(a.getByText(/end triggers:/)).toBeVisible({ timeout: 10_000 }); // the poll opened it

  const pick = rng(0x5457_4f50);
  let moves = 0;
  for (;;) {
    const pages = { a, b };
    const screens = { a: await readScreen(a), b: await readScreen(b) };
    if (screens.a.over || screens.b.over) break;
    const turn = screens.a.inert === null ? "a" : screens.b.inert === null ? "b" : null;
    if (turn === null) {
      // Neither screen is answerable: wait one poll for the mover's screen to refresh.
      await a.waitForTimeout(3500);
      moves += 1;
      if (moves > CAP) throw await deadEnd(a, { id }, moves);
      continue;
    }
    // The cross-check every move: the hands are disjoint, and non-empty on the mover.
    const [handA, handB] = [await ownHand(a), await ownHand(b)];
    expect(handA.filter((c) => handB.includes(c)), "no card is in both hands").toEqual([]);
    const via = await act(pages[turn], screens[turn], pick);
    if (via === null) throw await deadEnd(pages[turn], { id }, moves);
    moves += 1;
    if (moves > CAP) throw await deadEnd(a, { id }, moves);
  }
  // Anti-vacuity for the leak check (spec §9.4 item 8): the selector finds the OWN cards.
  expect((await ownHand(a)).length + (await ownHand(b)).length).toBeGreaterThan(0);
  await expect(a.getByText("The game is over.")).toBeVisible({ timeout: 10_000 });
  await expect(b.getByText("The game is over.")).toBeVisible({ timeout: 10_000 });
  expect(watchA.failures).toEqual([]);
  expect(watchB.failures).toEqual([]);
});
```

`Screen.inert` already distinguishes `watching`; a "watching" screen is the waiting seat's. The hand selector: `Dashboard.tsx`'s Hand panel renders `.card-face.order` per card — give each a `data-card={id}` attribute in `Cards.tsx`'s `OrderCardView` (one attribute, so the walk reads an identifier rather than prose). `every_spec_can_skip.spec.ts` passes by construction (the guard line and the import are present).

- [ ] **Step 5: Run everything and commit**

Run: `cd web && npm run e2e 2>&1 | tail -6`
Expected: 38 browser cells + the structural one, the new one printing its move count.

```bash
git add crates/yokohama-web/src/games.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/tests/wire_fixtures.rs crates/yokohama-web/tests/play_a_whole_game.rs web/src/types.ts web/src/api.ts web/src/App.test.tsx web/src/Cards.tsx web/src/wire/ web/src/wire/contract.ts web/src/wire/contract.test.ts web/e2e/harness.ts web/e2e/two_browsers.spec.ts
git commit -m "feat(web): preview retires and the seed never crosses the wire; the whole-game gate gains its two-identity arm; two browsers play to a winner and neither sees the other's hand"
```

---

### Task 11: Deploy `yokohama-web:0.5.0-m6.1` to the NAS

**Files:**
- Modify: `deploy/RUNBOOK.md` (§3.7, a record), `deploy/Dockerfile.runtime` (one comment beside `ENV`: the identity mode is NOT baked in; the container's create body carries it)

- [ ] **Step 1: Build and deliver by reference** — `deploy/RUNBOOK.md` §1 (the musl build), §2 (the context), §3.5 (the Portainer recipe). Tag `0.5.0-m6.1`.

- [ ] **Step 2: Create the container with the mode** — §3's body plus `"Env": ["YOKOHAMA_WEB_IDENTITY=claimed"]`; the same network, restart policy and volume. Stop and remove the `0.5.0-m5.5.4` container first; the volume carries over (its records have no table documents, so the list is empty by R-P7 — say so in §3.7).

- [ ] **Step 3: Verify from the shell (WARP passes Access)**

```bash
curl -sS https://yokohama.moore.nyc/api/health              # "identity":"claimed"
curl -sS -o /dev/null -w '%{http_code}\n' https://yokohama.moore.nyc/api/games   # 401
curl -sS -H 'X-Yokohama-Claimed-User: jamoore@gmail.com' https://yokohama.moore.nyc/api/games  # {"games":[]}
```

- [ ] **Step 4: Write §3.7** — the record: the tag, the image digest, the container id, the Env line, the three readings, R-1.4's two sentences (`cf-access` trusts the tunnel because 8080 is unpublished; `claimed` authenticates the door and not the seat), and the rollback (`0.5.0-m5.5.4` is still an image on the NAS; its client knows nothing of `/table`, so a rollback is a whole-image rollback).

- [ ] **Step 5: Commit**

```bash
git add deploy/RUNBOOK.md deploy/Dockerfile.runtime
git commit -m "docs(deploy): the fourth deployment — yokohama-web:0.5.0-m6.1 with YOKOHAMA_WEB_IDENTITY=claimed, verified from the shell"
```

---

### Task 12: The final review against the live site, and the close

**Files:**
- Create: `docs/plans/2026-09-14-web-frontend-m6.1-close.md`
- Modify: `CLAUDE.md` (the Web (server) and Web (browser half) count lines; the `crates/yokohama-web` architecture bullet gains the identity/table/seat paragraph), `guidingpath.md` (one decision-log row: M6.1 shipped, R-P1–R-P11 by reference, no bot behaviour moved, `SEMANTICS_REV` 7), `README.md` (the web section's create flow), `docs/handover.md` (Track 3: M6.1 done, M6.2 next from the room plan)

- [ ] **Step 1: The final review brief (a CLEAN Fable subagent, no prior context)**

Against `https://yokohama.moore.nyc`, in BOTH layouts (above and below 1201 px; `innerWidth` is the truth, not `resize_window`'s return), from TWO browsers as two people: sign in; create a two-person table; keep from each; play four turns alternating; confirm the other's screen advances within ~4 s of a move and the turn account walks; undo mid-turn as the mover and see the other screen reload; try undo after the turn passes and read the refusal; open the log, the replay, `why` and `suggest` from both seats; sign out and back in as a third email and confirm the table is not listed and its URL answers 403 through the client's error strip. Then the mutation battery from spec §9.4 on the LOCAL build (each mutation applied, the named cell run, reverted): the extractor reading any header; `hand_hidden` false for a non-viewer; `input` without the acting-seat check; the undo rule with the `current_player` clause removed; `deal` indexing the highest seat; `write_if_version` writing on a mismatch; the seed re-added to `TableView`; the e2e hand selector changed to match nothing. Record which cell caught each, and any that did not.

- [ ] **Step 2: Write the close** — on M5.5's close shape: §1 what shipped (routes, documents, fences), §2 the counts at the close HEAD (every gate re-run, ONE sequence), §3 the corpus diff summary across Tasks 1/3/4/6/10, §4 the review's findings and the mutations caught (and the ones not, as cells added), §5 the rulings taken in execution with their costs (from the SDD ledger), §6 the LEAVE list (inherited items from the M6 handover §4 that this cycle did not touch, plus its own), §7 what this cycle does NOT claim (spec §11 restated at the close HEAD), §8 the handover into M6.2: the room plan's §2 M6.2 charter, and what M6.1 built that it starts from (`TableDoc` gains `open`; `Identity` is the seam a join link lands on; the bot rule).

- [ ] **Step 3: The four tracked documents** — `CLAUDE.md`'s count lines re-counted at HEAD; `guidingpath.md`'s row appended (never edited); `README.md`'s create-flow sentence; `docs/handover.md` Track 3 rewritten (not appended).

- [ ] **Step 4: Commit and archive the SDD workspace**

```bash
git add docs/plans/2026-09-14-web-frontend-m6.1-close.md CLAUDE.md guidingpath.md README.md docs/handover.md
git commit -m "docs(web): M6.1 close — identity and seats shipped and live; the review's findings; the handover into M6.2"
tar -czf docs/temp/sdd-archive/2026-09-14-web-frontend-m6.1-plan.tar.gz -C .superpowers/sdd 2026-09-14-web-frontend-m6.1-plan
```

---

## Deviations from the spec (recorded per R-P11; the close carries the costs)

- **R-plan-1** `created_at` is unix seconds. **R-plan-2** the claimed header is attached whenever stored. **R-plan-3** the table document mirrors `to_move`/`over`. **R-plan-4** `preview` survives to Task 10. **R-plan-5** `hidden_hand.json` retires at Task 4.
- Spec §5.3 lists `GET …/{id}/table` as new; this plan also makes `Games::list()` return `TableView`s inside `{ "games": [...] }` (spec §4.4 says "→ `{ "games": [ …TableView… ] }`" — consistent).
- Spec §9.3's two-browser cell searches "the DOM for the other's card art and text"; this plan scopes the search to the Hand panels through a `data-card` attribute, because a port card is public and would legitimately appear on both screens (the spec's sentence would have failed on a true positive).
- Spec §4.2's `seats` entries are written `{ "user": "…" } | { "bot": true }`; `SeatSpec` accepts both fields and refuses anything but exactly one, so `{ "bot": false }` alone is a 400 rather than a human seat with no email.
- The `Dealing` screen's poll is a plain interval in Task 8 and the shared hook in Task 9; the two-task shape exists so that Task 8 can be reviewed as the deal alone.

## Self-review (run before the plan is handed over)

- **Spec coverage:** §2 → Task 1; §3 → Task 2; §4 → Task 3 (+10 for the preview's removal); §5 → Tasks 4 and 5; §6 → Task 6 (+9); §7 → Tasks 7–9; §8 → Tasks 1, 3, 4, 6, 10; §9.1–9.3 → the cells named in each task, §9.3's two arms in Task 10; §9.4 → Task 12's battery; §9.5 → the Global Constraints line; §10 → Task 11; §11 → Task 12's close §7; §12 → the implementer's read order at the top of the SDD ledger.
- **Placeholder scan:** none; the one "count it" instruction (Task 2 Step 5) asks for a measurement, not a value.
- **Type consistency:** `User`, `Identity`, `IdentityMode` (Task 1) are what Tasks 3–6 import; `Doc`, `Version`, `WriteError`, `Store` (Task 2) are what Task 3's `Games` calls; `TableDoc`, `TableView`, `DealView`, `SeatDoc`, `TableStatus`, `TurnMirror`, `GameError`, `SeatSpec` (Task 3) are what Tasks 4–6 and 10 use; `viewer_seat`, `facts_for`, `seated`, `viewer_of` (Task 4) are what Tasks 5 and 6 call; `Events` (Task 6) is what `signal.ts` (Task 9) consumes; `CLAIMED_USER_KEY`, `claimedUser`, `setClaimedUser`, `clearClaimedUser` (Tasks 1, 7) are what `SignIn`, `App` and the e2e harness use; `TableView`/`DealView`/`GamesList` in `types.ts` (Task 3) are what Tasks 7–8 render.

