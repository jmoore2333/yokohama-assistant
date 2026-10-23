# Security S3 — the hosts, the operations and the install-failure path: implementation plan (written 2026-10-20)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the security series' third cycle: the `Host` allow-list with its switch (D4) and the password-oracle throttle (D2) in the layer, the creator's reopen of a seat at an open table and the waiting-table TTL for a dealing table (D9), the service worker's install-failure path for a lapsed clearance (W-3), the private test Worker at `yokohama-test.moore.nyc`, every other row of S3 recorded in the owner's words — and, at the very end and waiting on nobody, `0.7.2` live on the NAS and the Worker (Task 10).

**Architecture:** Four code pieces land in the shapes the design fixes — two in `crates/yokohama-web` (a middleware before routing; a failure counter inside `Games`, so both hosts carry it), one across the layer, the Worker's lifecycle and the client (a seat-scoped route; a one-word widening of each host's "waiting" predicate), and one in the client's service worker and its door hook (a typed `kind` on the message the worker already posts, an election of one door window, one automatic pass per page). The fifth piece is operations: the private Worker's route, record, bucket and first deploy, Access first. The wrap moves the ledger, the spec's review record, the lane, the board and guidingpath.

**Tech Stack:** Rust (axum, tokio; the layer compiled natively and to wasm32), TypeScript (React + vitest + Playwright for `web/`; the Worker shell under `worker/`), Cloudflare (Workers, R2, Access, wrangler), Docker on the NAS.

**Spec:** `docs/plans/2026-10-19-security-s3-design.md` — every rule below argues from it; an implementer reads the design's section the task names before the task.

## Global Constraints

- **The branch is `security-s3`, stacked on `security-auth` at `4ce7493b`** (design §4 D-12). Every commit lands on it; the PR targets `security-auth`. Stage with explicit paths (`git add <path>`), never `git add -A`.
- **The suites per edit** (`docs/web-lane.md` §2, named in each task's verify step): `cargo test -p yokohama-web` (the lib, `start_refusal`, `wire_fixtures`), `cargo test -p yokohama-wasm`, `cargo test -p yokohama-worker`, `cargo test -p yokohama-web --features server identity::` where the task touches identity (none here), `cargo clippy --workspace --all-targets -- -D warnings`; in `web/`: `npm run typecheck`, `npx vitest run`, `npm run test:wasm` where the wasm tier is touched, `npm run e2e` where an e2e cell is added; in `worker/`: `npm run typecheck`, `npm run test:scripts`, `npm test` (the pool). The roster crates (`yokohama-core`, `yokohama-cli`) are not touched; no roster run is owed.
- **The wire corpus** (`web/src/wire/*.json`) pins the layer's output byte for byte: a task that changes a sentence or a field regenerates with `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures` and READS the diff into its report (the lesson M6.1).
- **TDD, test-first, every commit** (the repo's history is strictly TDD); no `#[ignore]`d test anywhere in this cycle; `cargo fmt` is never run; every invisible non-ASCII code point in code is a `\u` escape.
- **Sentences:** one line each, the house voice; a caller-supplied value repeated in a sentence goes through `crate::echo::echo` (64 characters). Every `ApiError` is capped at construction (`ApiError::capped`).
- **No `Retry-After` header anywhere** (design D-7): the layer and the Worker's reply carry no header channel.
- **The front door costs no Worker invocation** (design §3.1 rule 7): nothing on the install or update path requests anything under `/api`.
- **No exploit detail in any committed file;** the ledger's titles stay neutral; the gitignored report is never cited by content.
- **Dispatch rules (the owner's standing rules):** every implementer and reviewer runs on an EXPORT (`git archive`) of the branch head, never in the orchestrator's tree; every agent writes its full report to a transcript file and replies with the path plus three lines; no agent runs `rm -rf`, `rm -r` or a bulk `rm -f`; no agent waits on events or arms a watcher; no peer messaging — every message routes through the orchestrator. Every implementer PROBES the brief's code before reporting (runs the named cells, reads the cited lines) and reports `DONE` or `DONE_WITH_CONCERNS` with each concern quoted; a fix report quotes its hunks.
- **Tiers** (the owner's ruling for S3, 2026-10-20): implementers as each task's header says (`impl: sonnet` for a precise brief, `impl: opus` where judgment is needed); every task reviewer `review: opus`; the orchestrator runs the plan's "before" review (executing the first tasks' cells in an export — no mutants ahead of the tasks, the owner's pre-flight rule); the whole-branch "after" review is FABLE's, dispatched on an export of the final head, with the orchestrator sequencing its fix round and the scoped re-review.
- **Commit messages** in the house style (a prose subject naming the change, its rule and its catcher), ending with the attribution trailer the session carries.

## Review Focus

Five inputs the design implies that a person will meet, each pinned by a cell in the task that owns it:

1. **A `Host` header a real proxy or browser sends in a form the list did not spell** — uppercase, a trailing dot, a port, an IPv6 literal in brackets — must pass when the name is listed (Task 1: `a_listed_name_passes_in_every_spelling_a_client_sends`); and **a NAS deploy whose create body forgets the list** answers every tunnelled request 421 while the loopback smoke passes — the smoke's hostname probe and the recipe's required tunnel reading are the catchers (Task 1, R-S3-14).
2. **A friend's right password during a stranger's wrong-password burst at the same table** must pass; the counter is theirs, not the table's (Task 2: `two_callers_at_one_table_count_apart`).
3. **A creator's reopen racing the join that fills the last seat**: whichever lands first decides, and the loser reads a sentence, never a torn table (Task 3: `a_reopen_and_the_filling_join_land_in_either_order_and_the_table_is_whole`).
4. **A creator's own un-kept table against a bot** — full at create, so `Dealing` at once — now expires on the waiting-table TTL (7 days) instead of the idle one (90): a consequence of "a table that has not started", stated in the docs and pinned (Task 4: `a_dealing_table_with_a_bot_seat_is_waited_on_the_open_ttl`).
5. **A keyless build whose install is challenged** (a self-host on the open door behind some other edge) must land on `failed` with the sentence and "Try again", never a hang or a loop (Task 6: cell 4).

And one election case: **the door tab in the background while a game tab is focused** — the door is elected, never the game tab (Task 5: `the_election_prefers_a_focused_door_then_a_visible_door_then_any_door_and_never_a_game`).

---

## Before Task 1 (the orchestrator)

- [ ] On `security-s3` at `4ce7493b`, commit the design and this plan as the branch's first commit: `git add docs/plans/2026-10-19-security-s3-design.md docs/plans/2026-10-19-security-s3-plan.md`.
- [ ] The "before" review: in an export of the head, run the suites the Global Constraints name once (the baseline counts go in the SDD ledger); read Task 1's and Task 2's cells against the design's §3.3–§3.4 and correct the plan at the source before any dispatch. No mutants are run here (the owner's pre-flight rule).
- [ ] Open the SDD ledger at `.superpowers/sdd/2026-10-19-security-s3-plan/progress.md` (gitignored), one row per task with its tier, its export path, its transcript path and its verdict.

---

### Task 1: D4 — the `Host` allow-list on the NAS binary · impl: sonnet · review: opus

Design §3.4, D-8 and R-S3-14. The binary answers only the host names it is told to; the default set is loopback plus a specific bind address; an unlisted name is 421 with one sentence; the list is printed at start and a malformed one refuses to start. A second variable, `YOKOHAMA_WEB_HOSTNAMES_ENABLED`, switches the check: absent or `on` is on, `off` answers any `Host` and says so at start, anything else refuses to start by name. Every committed deploy file sets `on` beside the list. The smoke takes the hostname and probes with it; the NAS recipe's tunnel reading becomes a required step.

**Files:**
- Create: `crates/yokohama-web/src/hostnames.rs`
- Modify: `crates/yokohama-web/src/lib.rs` (the `AppState` struct near line 464 and its constructors near 696; `app()` at 1331–1350; a new middleware beside `response_headers` at 1296)
- Modify: `crates/yokohama-web/src/main.rs:100–130` (read the variable; refuse at start; print the list)
- Test: `crates/yokohama-web/src/hostnames.rs` (unit cells), `crates/yokohama-web/src/lib.rs` (route cells, the test module's helpers `request_raw` at 3423 extended with a `Host` header), `crates/yokohama-web/tests/start_refusal.rs` (three process cells)
- Modify: `deploy/smoke-image.sh` (an optional third argument, the hostname; the `get` helper at 71–77 already takes curl arguments, so `-H "Host: …"` rides it; the `docker run` at 99 gains `-e YOKOHAMA_WEB_HOSTNAMES_ENABLED=on -e "YOKOHAMA_WEB_HOSTNAMES=$hostname"` when the argument is given)
- Docs: `deploy/README.md` §4 (the paragraph at 257–259, the `docker run` example at 209 and the environment table at 234–235 gain BOTH variables) and §2 (the LAN note: list the browsers' name, or set the switch `off`), the root `README.md` (its self-host lines at 229 and 394 carry both), `deploy/Dockerfile.runtime` (a comment beside the `ENV` block at line 75; no value), `deploy/update.sh` (both variables documented beside the others it passes), `deploy/RUNBOOK.md` §3 (the create body's `Env` array at 244–254 gains `YOKOHAMA_WEB_HOSTNAMES_ENABLED=on` and `YOKOHAMA_WEB_HOSTNAMES=yokohama-nas.moore.nyc`; the recipe's reading gains the two `docker exec` lines of design §3.4 and the tunnel's `/api/health` as a REQUIRED step before a deploy is called done, marked "from 0.7.2")

**Interfaces:**
- Produces: `pub struct Hostnames` with `pub fn from_env_values(enabled: Option<&str>, raw: Option<&str>, bind: &str) -> Result<Hostnames, String>`, `pub fn is_on(&self) -> bool`, `pub fn admits(&self, authority: &str) -> bool` (always true when off), `pub fn listed(&self) -> Vec<String>` (the printed order: the defaults, then the listed), `pub fn start_line(&self) -> String` (on: `yokohama-web answers host names: …`; off: `yokohama-web answers any host name — YOKOHAMA_WEB_HOSTNAMES_ENABLED is off`), `pub const HOSTNAMES_VAR: &str = "YOKOHAMA_WEB_HOSTNAMES"`, `pub const HOSTNAMES_ENABLED_VAR: &str = "YOKOHAMA_WEB_HOSTNAMES_ENABLED"`, `pub const NOT_FOR_THIS_HOST: &str` (the sentence's fixed part); `AppState.hostnames: Hostnames`; `AppState::with_hostnames(self, h: Hostnames) -> Self` (the builder-step shape of `with_clock`); `ApiError::misdirected(message) -> ApiError` (status 421).
- Consumes: `crate::echo::echo`, `api::error_value`, `AppState`'s existing builder steps.

- [ ] **Step 1: Write the failing unit cells in `hostnames.rs`**

```rust
#[test] fn the_default_set_is_loopback_plus_a_specific_bind_address() {
    // bind "0.0.0.0" → ["localhost", "127.0.0.1", "::1"]; bind "10.0.0.5" → those plus "10.0.0.5"; bind "::" adds nothing
}
#[test] fn a_listed_name_passes_in_every_spelling_a_client_sends() {
    // list "yokohama-nas.moore.nyc": admits "yokohama-nas.moore.nyc", "YOKOHAMA-NAS.moore.nyc", "yokohama-nas.moore.nyc.", "yokohama-nas.moore.nyc:8080"
    // default set: admits "localhost:9999", "[::1]:8080", "127.0.0.1"
}
#[test] fn an_unlisted_name_an_empty_authority_and_a_different_name_with_a_listed_port_are_refused() {
    // "attacker.example", "attacker.example:8080", "" → false
}
#[test] fn a_malformed_list_is_refused_naming_the_entry() {
    // "a,,b" → the empty entry; "with space" → the entry; "http://x" → the entry (holds "/"); "x:8080" → the entry (a port is not a name; a colon is admitted only for an IPv6 literal); "::1" → Ok
}
#[test] fn the_switch_absent_or_on_is_on_and_off_admits_any_host() {
    // (None, None, "0.0.0.0") → is_on true, admits("attacker.example") false; (Some("on"), …) the same; (Some("off"), …) → is_on false, admits("attacker.example") true, listed() empty, start_line names the variable
}
#[test] fn a_malformed_switch_value_is_refused_naming_it() {
    // Some("yes"), Some("") → Err naming HOSTNAMES_ENABLED_VAR and the value; Some("ON "), Some(" Off") → Ok, read trimmed and case-insensitive
    // exactly as Multiplayer::from_env_value reads (lib.rs:372 — "a rule that held for two of them would be a rule nobody could remember"; pre-flight ruling P-12)
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p yokohama-web hostnames::`
Expected: FAIL to compile (no module).

- [ ] **Step 3: Implement `hostnames.rs`**

`Hostnames` is `Off` or `On { names }`, the names lower-cased, no trailing dot, IPv6 literals without brackets. `admits` is true when off; when on it lowers the authority, strips a trailing dot and one `:port` (an authority starting with `[` ends its host at `]`), and looks the host up. `from_env_values`: the switch first, after `trim().to_ascii_lowercase()` as `Multiplayer::from_env_value` reads it — `None | Some("on")` on, `Some("off")` off, anything else (the empty string included) `Err` naming `HOSTNAMES_ENABLED_VAR` and the value through `echo` (the shape of `Multiplayer::from_env_value`, `lib.rs:374`); off ignores the list; on with `None` is the default set; on with `Some(raw)` is the default set plus each comma-separated entry trimmed and checked — refused as `Err(format!("{HOSTNAMES_VAR}: \"{entry}\" is not a host name — …"))` through `echo` when empty, holding whitespace, `/`, `@`, `#`, or a `:` that does not parse as `std::net::Ipv6Addr`. Register `pub mod hostnames;` in `lib.rs`.

- [ ] **Step 4: Run the unit cells to verify they pass**

Run: `cargo test -p yokohama-web hostnames::`
Expected: PASS, 6 cells.

- [ ] **Step 5: Write the failing route cells in `lib.rs`'s test module**

**Every request a test sends through `app()` carries `Host: localhost` from this task on** (pre-flight ruling P-1): the in-process requests the crate's cells build today — `request_as`, `request_raw`, `request_raw_with_body`, `post_as`, `get_as` and every ad-hoc `Request::builder()` that reaches `app()` in `lib.rs`'s, `api.rs`'s and `assets.rs`'s test modules and in `tests/wire_fixtures.rs` and `tests/play_a_whole_game.rs` — carry NO `Host` header, so with the check on by default in every constructor each would read 421; a browser always sends one, so the fixtures sending one is the honest shape. Add `.header(http::header::HOST, "localhost")` to each (the suite names every miss as a 421), and add a helper `request_with_host(state, who, method, uri, host: Option<&str>)` beside `request_raw` (line 3423): the same request with the `Host` header when `host` is `Some` and with none when it is `None` — the ONLY path that omits it. (`tests/start_refusal.rs`'s raw helpers already write `Host: 127.0.0.1:{port}`, in the default set with the port ignored.) Cells:

```rust
#[tokio::test] async fn an_unlisted_host_is_421_before_routing_with_the_header_set_and_no_store_under_api() {
    // state with_hostnames(default, bind "0.0.0.0") and with_headers(one pair)
    // GET /api/health with Host attacker.example → 421, body error_value's JSON whose "error" holds NOT_FOR_THIS_HOST and the echoed host; the pair present; cache-control no-store
    // GET / with the same Host → 421, the pair present, no cache-control
    // HEAD /api/health with the same Host → 421
}
#[tokio::test] async fn a_listed_host_answers_every_route_as_before_and_the_port_is_ignored() {
    // GET /api/health with Host localhost → 200 as today; with Host localhost:9999 → 200; with Host 127.0.0.1 → 200
}
#[tokio::test] async fn a_request_with_no_host_header_is_421() {}
#[tokio::test] async fn the_bind_address_is_admitted_only_when_specific() {
    // bind "10.0.0.5": Host 10.0.0.5 → 200; bind "0.0.0.0": Host 10.0.0.5 → 421
}
#[tokio::test] async fn with_the_switch_off_a_foreign_host_answers_200_and_nothing_is_refused() {
    // with_hostnames(Hostnames::from_env_values(Some("off"), None, "0.0.0.0")): GET /api/health with Host attacker.example → 200; GET / → 200; no Host header → 200
}
```

- [ ] **Step 6: Run them to verify they fail**

Run: `cargo test -p yokohama-web --features server an_unlisted_host_is_421`
Expected: FAIL (200 where 421 is expected; `with_hostnames` missing).

- [ ] **Step 7: Implement the middleware and the state**

`AppState.hostnames: Hostnames` (every constructor builds the default set from its bind — the in-memory fixtures with bind `127.0.0.1`; `AppState::production` from the environment); `async fn host_allowed(State(state), req, next) -> Response` reads `req.headers().get(http::header::HOST)`, else the request URI's authority (`req.uri().authority()` — where an HTTP/2 `:authority` surfaces; pre-flight ruling P-2), neither → refused, answers `(StatusCode::MISDIRECTED_REQUEST, Json(api::error_value(&ApiError::misdirected(format!("{NOT_FOR_THIS_HOST} \"{}\"", echo(host))))))` or runs `next`; layered in `app()` BEFORE `response_headers` in the builder so the refusal carries the header set and `no-store` under `/api` (the cell in Step 5 is the catcher of the order). `ApiError::misdirected` beside `too_many_requests` at `lib.rs:184`.

- [ ] **Step 8: Run the lib suite to verify it passes**

Run: `cargo test -p yokohama-web --features server`
Expected: PASS; the count rises by the new cells and nothing else moves (the wire fixtures unchanged — no payload changed; the test requests now carry `Host: localhost`, so the committed corpus IS the design's "whole route table under one listed host, byte-identical" cell).

- [ ] **Step 9: Write the failing process cells in `tests/start_refusal.rs`**

```rust
#[test] fn a_malformed_hostname_list_exits_one_before_it_listens_naming_the_entry() {
    // YOKOHAMA_WEB_HOSTNAMES="good.example,bad name" → exit 1, stderr holds "bad name"; no listening line
}
#[test] fn the_binary_answers_its_listed_hostname_and_refuses_another_with_421() {
    // YOKOHAMA_WEB_HOSTNAMES_ENABLED=on, YOKOHAMA_WEB_HOSTNAMES="yokohama-nas.moore.nyc", a free port: GET /api/health with Host yokohama-nas.moore.nyc → 200; with Host attacker.example → 421; stdout's start line names "yokohama-nas.moore.nyc"
}
#[test] fn a_malformed_switch_value_exits_one_and_off_answers_any_host_saying_so() {
    // YOKOHAMA_WEB_HOSTNAMES_ENABLED=yes → exit 1, stderr names the variable and "yes"; YOKOHAMA_WEB_HOSTNAMES_ENABLED=off, a free port: GET /api/health with Host attacker.example → 200; stdout holds "answers any host name"
}
```

(The file's `Server` helper spawns the binary with the cell's own environment; follow its two existing request cells for the socket shape.)

- [ ] **Step 10: Run them to verify they fail**

Run: `cargo test -p yokohama-web --features server --test start_refusal hostname`
Expected: FAIL (the process starts on the malformed list; both hosts answer 200).

- [ ] **Step 11: Implement `main.rs`'s read and the start line**

Read `YOKOHAMA_WEB_HOSTNAMES_ENABLED` and `YOKOHAMA_WEB_HOSTNAMES` beside `YOKOHAMA_WEB_HOST` (line 106); `Hostnames::from_env_values(enabled.as_deref(), raw.as_deref(), &host)`; on `Err(e)` print `e` to stderr and `exit(1)` before the bind (the identity's shape at 108–114); on `Ok` print `hostnames.start_line()` before the listening line and pass it to the state with `with_hostnames`.

- [ ] **Step 12: Run the process cells and the whole crate**

Run: `cargo test -p yokohama-web --features server --test start_refusal` then `cargo test -p yokohama-web` and `cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS; clippy clean.

- [ ] **Step 13: The docs**

`deploy/smoke-image.sh` FIRST, since the docs cite it: an optional third argument `hostname`; when given, the `docker run` at 99 adds `-e YOKOHAMA_WEB_HOSTNAMES_ENABLED=on -e "YOKOHAMA_WEB_HOSTNAMES=$hostname"` and two probes follow the health read — `/api/health` with `-H "Host: $hostname"` expecting 200 and with `-H "Host: smoke-foreign.example"` expecting 421 — each a named pass or fail line; without it the script prints `smoke: the host probe was not asked for (pass a hostname as the third argument)` and runs as today. Run it once against a locally built image both ways and paste both tails in the report.

`deploy/README.md` §4: the `docker run` example at 209 gains `-e YOKOHAMA_WEB_HOSTNAMES_ENABLED=on -e YOKOHAMA_WEB_HOSTNAMES=<your hostname>`; the environment table at 234–235 gains two rows (the switch: `on`, `off`, the default, the start line; the list: its grammar, its default set, 421); after the paragraph at 257, one paragraph saying why both exist and the trap (a container reached through a tunnel with no list answers every request 421 while a loopback smoke passes). §2: one sentence telling a LAN self-hoster to list the name or address their browsers use, or to set the switch `off` when the boundary is not wanted. §5: the smoke's third argument. The root `README.md`: the self-host lines at 229 and 394 carry both variables. `deploy/Dockerfile.runtime`: a comment beside the `ENV` block: both variables are the operator's, set at run (no default here), with the README's section named. `deploy/update.sh`: both documented beside the others it passes through. `deploy/RUNBOOK.md` §3: the create body's `Env` array gains both; the recipe gains the two `docker exec … wget` lines (design §3.4) and the tunnel's `/api/health` as a REQUIRED step before a deploy is called done — a record for the NEXT deployment, marked "from 0.7.2".

- [ ] **Step 14: Commit**

```bash
git add crates/yokohama-web/src/hostnames.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/main.rs crates/yokohama-web/tests/start_refusal.rs deploy/smoke-image.sh deploy/README.md README.md deploy/Dockerfile.runtime deploy/update.sh deploy/RUNBOOK.md
git commit -m "web: the Host allow-list with its switch (security S3, D4; R-S3-14) — YOKOHAMA_WEB_HOSTNAMES_ENABLED on by default and explicit in every deploy file, YOKOHAMA_WEB_HOSTNAMES loopback plus a specific bind address by default, 421 before routing on an unlisted name with the header set kept, the state printed at start and a malformed value refusing to start; the smoke's hostname probe; the deploy docs and the NAS create body with the tunnel reading required"
```

**Review focus for the reviewer:** the middleware's order (the 421 carries the header set); the mutant "compare the whole authority, port included" reads red at `a_listed_host_answers_every_route_as_before_and_the_port_is_ignored`; the mutant "defaults without the bind address" reads red at `the_bind_address_is_admitted_only_when_specific`; the mutant "an absent switch reads off" reads red at `the_switch_absent_or_on_is_on_and_off_admits_any_host` AND at `an_unlisted_host_is_421_before_routing…` (whose state names no switch); every `AppState` literal in the crate carries the field (grep `AppState {`); the smoke's two new probes each fail when the image's check is broken (run the smoke against an image built with the middleware removed, once).

---

### Task 2: D2 — the password oracle's throttle inside `Games` · impl: opus · review: opus

Design §3.3 and D-7. A failure counter keyed by (table, caller key), 20 wrong passwords per 10 seconds, failures only, a success clears, a refusal counts nothing, entries live only while their window is open, 429 with one sentence and no header. Inside `Games` so the Worker's Table object carries it too.

**Files:**
- Create: `crates/yokohama-web/src/guesses.rs`
- Modify: `crates/yokohama-web/src/games.rs` (`GameError` at 45 gains a variant; the `Games` struct at ~421 gains the field; EVERY `Games { … }` literal — the S1c lesson: grep them all)
- Modify: `crates/yokohama-web/src/lib.rs` (the `From<GameError> for ApiError` arm at ~253)
- Modify: `crates/yokohama-web/src/tables.rs` (`Games::unlock` at 1413–1432; `Games::join` at 1636–1690, the password arms at 1674–1686)
- Test: `guesses.rs` (unit), `tables.rs` or `lib.rs` (route cells through `request_raw_with_body`), `crates/yokohama-worker/src/lib.rs` (one cell through its `call` helper)
- Client: `web/src/OpenTable.test.tsx` (the join form's strip on a 429), the unlock face's test file (the builder finds the component that calls `api.unlock`, `web/src/api.ts:1115`, and its test)
- Docs: `docs/web-lane.md` §8 (the password-oracles row), `deploy/README.md` §2 (one sentence: under the open door the counter is per name)

**Interfaces:**
- Produces: `pub struct Guesses` (Default); `pub const WRONG_PASSWORDS_PER_WINDOW: u32 = 20`; `pub const WINDOW_SECS: u64 = 10`; `pub const TOO_MANY_WRONG_PASSWORDS: &str = "too many wrong passwords for this table — wait ten seconds and try again"`; `pub enum Guess { Refused, Wrong, Right }`; `impl Guesses { pub fn guess(&self, table: &str, caller: &str, now: u64, correct: bool) -> Guess; pub fn live(&self, now: u64) -> usize }` — ONE acquisition of the counter's lock per guess (P-17, after the review: the design's "the count-then-write is one operation under the counter's own lock", so the serialization no longer leans on the table lock, which is keyed by the request's spelling where the counter is keyed by the stored id); `GameError::TooMany(String)` → `ApiError::too_many_requests`; `Games.guesses: Guesses` (crate-visible).
- Consumes: `(self.clock)()` for `now` (seconds — the unit `table::now_unix` and both wasm hosts hand the layer; the implementer confirms the Worker's `CLOCK.now` reading is seconds before writing the window arithmetic, `crates/yokohama-worker/src/lib.rs:105–120`).

- [ ] **Step 1: Write the failing unit cells in `guesses.rs`**

```rust
#[test] fn the_twenty_first_failure_in_a_window_is_refused_and_the_twentieth_is_not() {}
#[test] fn a_clear_resets_the_window_and_the_next_failure_counts_from_one() {}
#[test] fn two_callers_at_one_table_count_apart() {}
#[test] fn one_caller_at_two_tables_counts_apart() {}
#[test] fn a_refused_check_counts_nothing_so_the_window_lapses_on_schedule() {
    // 20 failures at t=0; check at t=5 refused ×2; at t=9 still refused; at t=10 check passes and live() reads 0 —
    // AND the entry's count read white-box (a #[cfg(test)] reader) is still 20 after the two refusals: under a fixed
    // window a counted refusal is invisible from outside, so the black-box line alone cannot see that mutant (P-14)
}
#[test] fn a_failure_inside_an_open_window_never_moves_its_start() {
    // nineteen failures at t=0, the twentieth at t=9; a check at t=9 refused; at t=10 Ok — a sliding window would refuse until t=19 (P-17; the review's one surviving mutant)
}
#[test] fn the_window_arithmetic_neither_overflows_nor_lapses_early_at_the_clocks_ends() {} // P-13
#[test] fn a_lapsed_entry_is_pruned_on_the_next_check_whatever_its_key() {
    // failures under key A at t=0; a check under key B at t=10 → the map's length read RAW is 0 (`live` prunes for
    // itself, so `live() == 0` cannot see a `check` that never prunes; P-14)
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p yokohama-web guesses::`
Expected: FAIL to compile.

- [ ] **Step 3: Implement `guesses.rs`**

`Guesses { windows: Mutex<HashMap<(String, String), Window>> }`, `Window { started: u64, failures: u32 }`. `guess(table, caller, now, correct)`: ONE lock acquisition — drop every entry whose `now.saturating_sub(started) >= WINDOW_SECS` (the addition form overflows at the clock's end and a saturating add lapses a window started there at once; P-13); when the key's window holds `failures >= WRONG_PASSWORDS_PER_WINDOW` answer `Refused` whatever `correct` says (a full window hides the compare's result); else when `correct` is false start a window at `now` if none — **a window starts at the FIRST failure and never moves; a failure inside an open window increments and never restarts it** — and answer `Wrong`; when `correct` is true remove the key and answer `Right`. `live`: prune, then the map's length. The check and the count are one operation under the counter's own lock (the S1b lesson; P-17). Register `pub mod guesses;`.

- [ ] **Step 4: Run the unit cells to verify they pass**

Run: `cargo test -p yokohama-web guesses::`
Expected: PASS, 8 cells.

- [ ] **Step 5: Write the failing route cells (in `lib.rs`'s test module, through the router)**

```rust
#[tokio::test] async fn the_twenty_first_wrong_password_at_a_join_is_429_with_the_sentence_and_no_header() {
    // a locked open 3-seat table; 20 × POST …/join {"seat":2,"password":"wrong"} as "guest@t.test" → 403 "wrong password for this table"; the 21st → 429, body "error" == TOO_MANY_WRONG_PASSWORDS, no retry-after header
}
#[tokio::test] async fn the_right_password_at_the_twentieth_clears_and_a_friend_passes_during_a_strangers_burst() {
    // 19 wrong as guest, then the right one → 200 and the seat taken; then 20 wrong as "other@t.test" at the same table → the 21st 429 for other, while "friend@t.test" with the right password → 200
}
#[tokio::test] async fn the_window_lapses_after_ten_seconds_on_the_tables_clock() {
    // Games::with_clock(frozen at T); 20 wrong → the 21st 429; clock to T+10 → a wrong one is 403 again
}
#[tokio::test] async fn the_unlock_route_counts_the_same_way() {
    // a locked PLAYING table; 20 wrong unlocks → the 21st 429 with the sentence
}
#[tokio::test] async fn a_wrong_join_and_a_wrong_unlock_by_one_caller_at_one_table_count_together() {
    // design §3.3 "both routes share the counter" (pre-flight ruling P-4). Games::with_clock frozen; a locked 2-seat OPEN table;
    // 10 wrong joins as "guest" → 403; "friend" joins with the right password and both seated players keep → Playing;
    // 10 wrong unlocks as "guest" → 403; the 21st wrong unlock → 429 with the sentence (10 + 10 + 1 under one key).
    // If the route module has no helper that carries a table from Open to Playing, the cell lives in tables.rs against Games directly, same arithmetic.
}
```

- [ ] **Step 6: Run them to verify they fail**

Run: the five cells by name — the filter `wrong_password` matches only the first (P-15): `cargo test -p yokohama-web --features server -- the_twenty_first_wrong_password the_right_password_at_the_twentieth the_window_lapses_after_ten_seconds the_unlock_route_counts a_wrong_join_and_a_wrong_unlock`
Expected: FAIL (every attempt 403).

- [ ] **Step 7: Wire the counter into `join` and `unlock`**

In both, after the table is read under the lock and immediately before the compare (a request the status match, the seat rungs or an absent/empty password refuse is no guess and neither checks nor counts): one private helper the two routes share — `let now = (self.clock)(); match self.guesses.guess(&doc.id, user.as_str(), now, doc.check_password(given)) { Guess::Refused => Err(GameError::TooMany(TOO_MANY_WRONG_PASSWORDS.to_string())), Guess::Wrong => Err(the existing Forbidden), Guess::Right => Ok(()) }` (the STORED document's id, never the request's spelling — a case-insensitive file store aliases `GUARDED` to `guarded`; one clock reading; the compare's result computed before the call and hidden by a `Refused` answer; P-16, P-17). `GameError::TooMany(m)` maps to `ApiError::too_many_requests(m)` in `lib.rs`. The field: `guesses: Guesses::default()` in every `Games` literal (`with_parts` builds it as `Self { … }` inside `impl Games` — grep `Games {` AND `Self {` in `games.rs`).

- [ ] **Step 8: Run the lib suite and regenerate nothing**

Run: `cargo test -p yokohama-web` and `cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS; `wire_fixtures` unchanged (the 429's body is `error.json`'s shape with a new sentence, which no committed fixture carries — confirm, and if a fixture DOES move, read the diff and say so in the report).

- [ ] **Step 9: The Worker's cell**

In `crates/yokohama-worker/src/lib.rs`'s test module, through `call(...)`: a locked open table; 20 wrong joins as one address → 403; the 21st → 429 with the sentence. Run: `cargo test -p yokohama-worker twenty_first`. Expected: PASS (the layer's counter runs inside the object with no shell change).

- [ ] **Step 10: The client's two cells**

`web/src/OpenTable.test.tsx`: a 429 from `api.join` with the sentence lands in the join form's error strip, in the server's words, and the form stays usable. The unlock face's test: the same for `api.unlock`. Run: `npx vitest run OpenTable` and the unlock face's file. Expected: PASS without a client code change (the forms draw `ApiError.message`) — if a form swallows a 429 into a generic line, fix it to draw the sentence, test-first.

- [ ] **Step 11: The docs**

`docs/web-lane.md` §8, the password-oracles row: one sentence that since S3 the layer counts wrong passwords per table and caller, 20 per 10 s, on both hosts, and the NAS hostname is covered by it. `deploy/README.md` §2: under the open door the counter is per NAME, a name is free, and the LAN is the trusted network by the door's own rule.

- [ ] **Step 12: Commit**

```bash
git add crates/yokohama-web/src/guesses.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/tables.rs crates/yokohama-worker/src/lib.rs web/src/OpenTable.test.tsx docs/web-lane.md deploy/README.md
git commit -m "web: the password oracle's throttle inside Games (security S3, D2) — twenty wrong passwords per table and caller per ten seconds, failures only, a success clears, a refusal counts nothing, entries live only while their window is open; 429 in one sentence and no header, on both hosts"
```

**Review focus for the reviewer:** one mutant per value class, each red at a named cell — key by caller alone (`one_caller_at_two_tables_counts_apart`), key by table alone (`two_callers_at_one_table_count_apart` and the friend cell), 19 or 21 (`the_twenty_first…`), never prune (`a_lapsed_entry_is_pruned…`), never clear (`a_clear_resets…`), count refusals (`a_refused_check_counts_nothing…`), one counter per route (`a_wrong_join_and_a_wrong_unlock_by_one_caller_at_one_table_count_together`); the count of `Games {`/`Self {` literals carrying the field equals the count in the tree.

---

### Task 3: D9 — the seat the creator reopens, at `Open` only · impl: opus · review: opus

Design §3.2 rule 1, D-4, D-15 and R-S3-13. A seat-scoped route the creator posts at an open table; refused at `Dealing` and `Playing`, on the creator's own seat, on a bot's and on an open one; the kicked person is a would-be joiner afterwards; the client's control on the open table's roster and nowhere else.

**Files:**
- Modify: `crates/yokohama-web/src/api.rs` (a `Reopen` body beside `Keep` at 1011; `pub fn reopen` beside `keep` at 1281; `routes()` at 232 and the hand-kept copy at 4132; the dispatcher arm beside `/keep` at 2658)
- Modify: `crates/yokohama-web/src/tables.rs` (`Games::reopen` beside `join` at 1636)
- Modify: `web/src/api.ts` (`reopen` beside `keep` at 1128), `web/src/OpenTable.tsx` (the seat rows at 350), `web/src/OpenTable.test.tsx`, `web/e2e/two_browsers.spec.ts`
- Test: `tables.rs` (every arm), `lib.rs` (the route through the router), `crates/yokohama-worker/src/lib.rs` (one cell through `call`), the client cells, one e2e cell
- Docs: `docs/web-lane.md` §13 (the API map gains the row), §4 (the `admitted` bullet, one sentence)

**Interfaces:**
- Produces: `POST /api/games/{id}/reopen` with body `{ "seat": n }` (0-BASED as `Keep`/`Join` send it — `PlayerId::try_new` accepts 0..3 and indexes `seats` directly; P-18), answering the creator's `TableView` (200); `Games::reopen(&self, id: &str, user: &User, seat: PlayerId) -> Result<TableView, GameError>`; the sentences — `ONLY_CREATOR_REOPENS = "only the table's creator can reopen a seat"` (403), `DEAL_HAS_BEGUN = "the deal has begun — abandon the table and set it again"` (409), `GAME_HAS_STARTED = "the game has started — a seat at play is a player"` (409), `OWN_SEAT = "you cannot reopen your own seat — abandon the table instead"` (409), `NO_JOINER = "that seat holds no joiner to reopen"` (409, a bot's seat and an open seat alike), `no seat N at this table` (400, echoed `n`); `api.reopen(game: string, seat: number): Promise<TableView>` in `web/src/api.ts`.
- Consumes: `TableDoc.is_creator`, `SeatDoc::{Human, Bot, Open}`, `Games::with_table_lock`, `read_table`, `write_table`, `signal.bump`, `doc.view`.

- [ ] **Step 1: Write the failing layer cells in `tables.rs`'s test module**

```rust
#[test] fn a_creator_reopens_a_joiners_seat_at_an_open_table_and_the_table_stays_open() {
    // 3 seats: creator seat 1, "guest" joins seat 2; reopen 2 → seats[1] Open, status Open, the view's seats[1].kind "open"; the signal bumped once
}
#[test] fn a_reopen_is_refused_by_everyone_but_the_creator() {} // the joiner and a stranger → Forbidden(ONLY_CREATOR_REOPENS)
#[test] fn a_reopen_at_dealing_and_at_playing_is_refused_with_its_sentence() {} // Conflict(DEAL_HAS_BEGUN), Conflict(GAME_HAS_STARTED)
#[test] fn the_creators_own_seat_a_bots_seat_and_an_open_seat_are_not_reopened() {} // OWN_SEAT, NO_JOINER, NO_JOINER
#[test] fn a_seat_number_past_the_table_is_a_bad_request() {} // seat 4 at a 3-seat table → BadRequest
#[test] fn a_kicked_joiner_is_a_would_be_joiner_afterwards_and_may_join_again() {} // table_for_anyone admits them; join seat 2 again → 200
#[test] fn a_reopen_and_the_filling_join_land_in_either_order_and_the_table_is_whole() {
    // order A: the join that fills seat 3 first → Dealing; then reopen 2 → DEAL_HAS_BEGUN; check_shape ok
    // order B: reopen 2 first; then the join for seat 3 → the table Open with seats 2 and 3 as the join left them; check_shape ok
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p yokohama-web reopen`
Expected: FAIL to compile (`Games::reopen` missing).

- [ ] **Step 3: Implement `Games::reopen` in `tables.rs`**

`require_table`; `with_table_lock`; read; `is_creator` FIRST else `Forbidden(ONLY_CREATOR_REOPENS)` at every status (the status-leak ruling #8 — a stranger learns no status from a refusal; the catchers are `a_reopen_is_refused_by_everyone_but_the_creator`'s dealing arm on a real filled table, the stranger sweep and the admission grid — the seat holder's open-table loop skips /reopen; P-19); then the status match (`Open` proceeds; `Dealing` → `Conflict(DEAL_HAS_BEGUN)`; `Playing` → `Conflict(GAME_HAS_STARTED)`); the seat by `doc.seats.get(n)` else `BadRequest`; `Human { user, .. }` equal to the creator → `OWN_SEAT`; `Bot` or `Open` → `NO_JOINER`; write `SeatDoc::Open`; `write_table(&doc, Some(version))`, a `Conflict` on a lost race in `join`'s sentence shape ("changed while this reopen was in flight"); `signal.bump(id)`; `Ok(doc.view(user, self.archive_on()))`.

- [ ] **Step 4: Run the layer cells to verify they pass**

Run: `cargo test -p yokohama-web reopen`
Expected: PASS, 7 cells.

- [ ] **Step 5: Write the failing route cells**

In `api.rs`'s test module: the route table's hand-kept copy at 4132 gains `("POST", "/api/games/{id}/reopen")` (the drift detector); in `lib.rs`'s test module, through `request_raw_with_body`: the creator's reopen → 200 with the `TableView`; a joiner's → 403 with the sentence; a body `{"seat": 4}` → 400 (`PlayerId::try_new`) and `{"seat": 3}` at a 3-seat table → 400 naming the seat; `{"seat": 0}` is the creator's own seat → 409 OWN_SEAT (P-18); a stranger's reopen at a DEALING table → 403, never 409 (P-19); a body with an unknown field → 400 (`deny_unknown_fields`).

- [ ] **Step 6: Run them to verify they fail**

Run: `cargo test -p yokohama-web --features server reopen`
Expected: FAIL (404 `no such endpoint`; the route-table cell red).

- [ ] **Step 7: Implement the API**

`pub struct Reopen { pub seat: u8 }` with `deny_unknown_fields`; `pub fn reopen(games, user, id, req: &Reopen) -> Result<TableView, ApiError>` parsing the seat as `keep` does (`api.rs:1281–1293`); the row in `routes()`; the dispatcher arm beside `/keep`.

- [ ] **Step 8: Run the lib suite, the wire fixtures and the worker crate**

Run: `cargo test -p yokohama-web`, then add in `crates/yokohama-worker/src/lib.rs` one cell through `call(...)` (the creator's reopen at an open table → 200; the joiner's → 403) and run `cargo test -p yokohama-worker reopen`; then `cargo clippy --workspace --all-targets -- -D warnings`.
Expected: PASS; `wire_fixtures` unchanged (the answer is `TableView`, a committed shape; the refusals are `error.json`'s shape) — if `access-paths.mjs`'s drift cell (`worker/npm run test:scripts`) reads the new segment, it is under the `/api/games` prefix and lists nothing new; confirm and say so.

- [ ] **Step 9: Write the failing client cells in `OpenTable.test.tsx`**

- the creator sees a "Reopen" control beside a joiner's human seat, and none beside their own seat (`you`), a bot's, or an open one;
- a joiner (not `mine`) sees no control at any seat;
- the click posts `/api/games/<id>/reopen` with `{ seat: n }` and redraws the roster from the answer (the seat now open);
- a refusal's sentence (a 409 `DEAL_HAS_BEGUN`) lands in the screen's error strip in the server's words.

- [ ] **Step 10: Run them to verify they fail**

Run: `cd web && npx vitest run OpenTable`
Expected: FAIL (no control).

- [ ] **Step 11: Implement `api.reopen` and the control**

`web/src/api.ts`: `reopen: (game: string, seat: number) => call<TableView>("POST", `/api/games/${id(game)}/reopen`, { seat })`. `OpenTable.tsx`: in the seat rows at 350, when `table.mine && s.kind === "human" && !s.you`, a button "Reopen" whose handler awaits `api.reopen(table.id, i)` (0-based; P-18), sets the table from the answer, and on an `ApiError` puts `e.message` in the general error strip (`setError`, not the Abandon strip) and re-reads the table ONCE on a 409 exactly as `join`'s 409 path does — a re-read that finds the table dealing hands over to the dealing screen, where the Abandon control now is (P-21, after the review). The Reopen control stays off the dealing screen; `Dealing.tsx` GAINS the creator's Abandon control under design §3.2 rule 3 — it had none at HEAD (the ~255 comment is about the lobby row) — `mayAbandon`-gated, the open table's wiring and sentence, with `Dealing.test.tsx` cells (P-20). `docs/web-lane.md` §16's metered ledger gains the reopen row (`lane_document`'s `the_ledger_names_every_route`).

- [ ] **Step 12: Run the client suites**

Run: `cd web && npm run typecheck && npx vitest run`
Expected: PASS.

- [ ] **Step 13: The e2e cell**

In `web/e2e/two_browsers.spec.ts`, following its two-context shape: the first browser sets a 3-seat open table; the second joins seat 2; the first clicks Reopen on seat 2; the second browser's table screen offers Join again at seat 2 and shows no hand. Run: `cd web && npm run e2e -- two_browsers`. Expected: PASS.

- [ ] **Step 14: The docs and the commit**

`docs/web-lane.md` §13: the row for `POST …/reopen` (the creator, an open table, the write limiter counts it); §4's `admitted` bullet: one sentence that `reopen` is creator-only at `Open`. Then:

```bash
git add crates/yokohama-web/src/api.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/src/lib.rs crates/yokohama-worker/src/lib.rs web/src/api.ts web/src/OpenTable.tsx web/src/OpenTable.test.tsx web/e2e/two_browsers.spec.ts docs/web-lane.md
git commit -m "web: the creator reopens a joiner's seat at an open table (security S3, D9; R-S3-13) — POST …/reopen, creator-only, refused at dealing and playing, on the creator's own seat and on a seat that holds no joiner; the kicked person a would-be joiner afterwards; the control on the open table's roster"
```

**Review focus for the reviewer:** the `Dealing` refusal is reached by a cell with a REAL dealing table (the join that fills the last seat), not a hand-built doc; the own-seat refusal covers a creator's second hot-seat seat; the e2e cell reads the second browser, not only the first.

---

### Task 4: D9 — a table that has not started expires on the waiting-table TTL · impl: opus · review: opus

Design §3.2 rule 2, D-5 and D-16. Both hosts read `Dealing` beside `Open` for the waiting-table TTL; the NAS settles before it reads, the Worker does not and ends the unread corner; the knob's sentence changes everywhere it is stated.

**Files:**
- Modify: `crates/yokohama-web/src/sweep.rs` (`ending` at 164–176; the loop at 222–241 reads through `Games::settle` instead of `read_table`), `crates/yokohama-web/src/api.rs:71–78` and `:433–440` (doc sentences)
- Modify: `worker/src/table/lifecycle.ts:382–387` (`waiting`), `worker/src/marker.ts` (the doc at 113–129), `worker/src/env.ts:97–100`, `worker/src/config.ts:42–45`
- Test: `sweep.rs` (the `ending` cells at ~427–440 and a settle-first cell), `worker/test/expiry.test.ts`
- Docs: `deploy/README.md:309`, `docs/web-lane.md` (every sentence stating the open-table TTL — grep `OPEN_TABLE_TTL` and `waiting for players`), and §4: one sentence recording D-16's difference

**Interfaces:**
- Produces: `sweep::ending` returns `Waited` for `TableStatus::Dealing` past the open days as it does for `Open`; the Worker's `waiting(id)` is true for a copy whose `status` is `"open"` or `"dealing"`.
- Consumes: `Games::settle(&self, id) -> Result<Option<(TableDoc, Version)>, GameError>` (crate-visible, `tables.rs:1456`).

- [ ] **Step 1: Write the failing `sweep.rs` cells**

```rust
#[test] fn a_dealing_table_is_waited_on_the_open_ttl_and_expired_on_the_table_ttl_like_an_open_one() {}
#[test] fn a_dealing_table_with_a_bot_seat_is_waited_on_the_open_ttl() {} // a 2-seat table, creator + bot, un-kept
#[test] fn the_sweep_settles_a_repairable_dealing_table_before_it_reads_its_status_and_keeps_it() {
    // the fixture `tables.rs`'s settle cells build (a record present, the flip never landed), past the open TTL: the sweep's pass repairs it to Playing and ends nothing; NO sweep line names it (a table the pass keeps gets no line; P-22)
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p yokohama-web sweep::`
Expected: FAIL (the first two read `None` at 7–8 days where `Waited` is expected — the old dealing arm is the table TTL; the third: the pass ENDS the repairable table where the real code keeps it; P-22).

- [ ] **Step 3: Implement**

`ending`: `matches!(status, TableStatus::Open | TableStatus::Dealing) && past(open_days)` → `Waited`. The loop at 222: read each id through `games.settle(&id)` (the repairing read; a `Some` carries the doc); the rest unchanged. The two `api.rs` doc sentences: "a table that has not started — open, or dealing with a keep outstanding".

- [ ] **Step 4: Run the lib suite**

Run: `cargo test -p yokohama-web` and clippy.
Expected: PASS.

- [ ] **Step 5: Write the failing Worker cell in `worker/test/expiry.test.ts`**

Following the file's open-table `waited` cell: a 2-seat table filled by a second identity (status `dealing`, no keep), the clock past 7 days → the table ends with reason `waited` and `ttl_days` 7; a dealing table past 1 day does not end. And the corner (D-16): the same dealing table with no move made, past the open TTL → ended `waited` (the Worker does not settle before it reads; the cell's doc says so and cites the design).

- [ ] **Step 6: Run it to verify it fails**

Run: `cd worker && npx vitest run test/expiry`
Expected: FAIL (the dealing table survives 7 days).

- [ ] **Step 7: Implement `waiting`**

`lifecycle.ts:386`: `const status = factsOf(made.copy).status; return status === "open" || status === "dealing";` and the method's doc; `marker.ts`'s doc at 113–129; `env.ts:97–100` and `config.ts:42–45`: "a table that has not started".

- [ ] **Step 8: Run the Worker suites**

Run: `cd worker && npm run typecheck && npm test && npm run test:scripts`
Expected: PASS.

- [ ] **Step 9: The docs and the commit**

`deploy/README.md:309`: "a table that has not started — open, or dealing with a keep outstanding — once it has stood idle …". `docs/web-lane.md`: every sentence stating the TTL, and §4: "Since S3 a dealing table expires on the waiting-table TTL on both hosts; the NAS settles a repairable table before it reads, the Worker does not, so a finalized-but-unflipped table nobody opened for the whole TTL ends on the Worker (design D-16)."

```bash
git add crates/yokohama-web/src/sweep.rs crates/yokohama-web/src/api.rs worker/src/table/lifecycle.ts worker/src/marker.ts worker/src/env.ts worker/src/config.ts worker/test/expiry.test.ts deploy/README.md docs/web-lane.md
git commit -m "web, worker: a table that has not started expires on the waiting-table TTL (security S3, D9) — dealing beside open on both hosts, the NAS sweep settling before it reads, the Worker ending the unread corner by design; the knob's sentence at every site"
```

**Review focus for the reviewer:** the settle-first cell uses a REAL repairable fixture (the one `tables.rs`'s settle cells build), not a doc edited by hand; the Worker cell's clock moves the TTL, not the cell's expectations.

---

### Task 5: W-3 — the service worker names the challenge and elects one door window · impl: opus · review: opus

Design §3.1 steps 1–2 and 7, D-1, D-13. A challenged precache fetch is a typed failure; the message carries `kind`; one door window is elected for `"challenged"`, every other window gets `"failed"`; the install path requests nothing under `/api`.

**Files:**
- Create: `web/src/edgeChallenge.ts` — `export function isEdgeChallengeResponse(status: number, cfMitigated: string | null): boolean` (status 403 and `cfMitigated === "challenge"`), the ONE predicate both the page and the worker import (design §3.1 step 1's "kept equal by a cell" becomes one source)
- Modify: `web/src/turnstile.ts:233–235` (`isEdgeChallenge` calls the shared predicate), `web/src/sw.ts` (`installOne` at 197–227; the install's catch at ~176–190; `tellTheClients` at 248–256), `web/src/sw.node.test.ts` (the harness's `matchAll` fake at 217–219 gains clients with `url`, `focused`, `visibilityState`; the two `posted` expectations at 313 and 473 gain `kind: "failed"`)
- Test: `web/src/sw.node.test.ts`, `web/src/edgeChallenge.test.ts`

**Interfaces:**
- Produces: the message `{ yokohama: "install-failed", kind: "challenged" | "failed", reason: string }`; `export type InstallFailedKind = "challenged" | "failed"` in `web/src/swMessages.ts` (a new, tiny module both `sw.ts` and `swRegister.ts` import, so the page's reader and the worker's writer share the type); the challenged reason `"<path>: the edge asked for a check before it would answer"`; the election: among windows whose LOAD address is the door's — Chromium's `WindowClient.url` does not follow a hash change (measured by the Task 5 review and the Task 6 implementer in Playwright's Chromium), so a tab that entered at the door and moved into a game can be elected and a tab that arrived on a game link and moved to the door is not elected until it reloads; the page's own route check (P-26) is the load-bearing fence and the election only picks ONE window (P-27) — the focused one, else a visible one, else the first; with none, every window gets `"failed"`.
- Consumes: `self.clients.matchAll({ includeUncontrolled: true, type: "window" })` (`WindowClient.url`, `.focused`, `.visibilityState`); the door's URL shape — the implementer reads how `App.tsx` computes `route.kind === "door"` (its `route` parser) and writes `isDoorUrl(url: string): boolean` in `swMessages.ts` from the same rule, with a cell — restated for the worker's bundle (`route.ts` imports React): an `http(s)` URL whose path is the app's document (never `/enter`, `/api/…`, `/cdn-cgi/…` or another server path) and whose hash is empty or parses to the door; `about:blank` and server paths are never the door; equality with `parseRoute` pinned over a table of hashes (P-23).

- [ ] **Step 1: Write the failing cells**

`edgeChallenge.test.ts`: the predicate is true only for 403 with `challenge`, false for 403 without, for 200 with, for 429 with.

`sw.node.test.ts`, in "the worker's install":

```ts
it("names the edge's challenge as its own kind, to one door window, and every other window gets failed", ...)
  // boot with answer: "/art/manifest.json" → { status: 403, body: "", type: "text/html", headers: { "cf-mitigated": "challenge" } }
  // matchAll fake: [game tab focused, door tab visible, door tab hidden]
  // posted: the visible door gets kind "challenged" with reason "/art/manifest.json: the edge asked for a check before it would answer"; the other two get kind "failed" with the same reason; the install rejects; no generation left behind
it("the_election_prefers_a_focused_door_then_a_visible_door_then_any_door_and_never_a_game", ...)
  // three boots, each list ordered ADVERSARIALLY with the wanted window last (a first-match election would otherwise pass): [visible door, focused door] → the focused; [hidden door, visible door] → the visible; [focused game, hidden door] → the hidden door (P-25)
it("with no door open every window gets failed and nothing is challenged", ...)
it("a 403 without the header is a plain failure to every window", ...)
it("the install path requests nothing under /api", ...) // harness.asked has no path starting with "/api"
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cd web && npx vitest run -c vitest.node.config.ts sw.node` — the default config EXCLUDES `*.node.test.ts`; `npm run test:wasm` is that project — and `npx vitest run edgeChallenge swMessages` (P-24)
Expected: FAIL (no `kind`; no module).

- [ ] **Step 3: Implement**

`edgeChallenge.ts` and `swMessages.ts` (the type, `isDoorUrl`). `sw.ts`: `class EdgeChallenged extends Error` thrown by `installOne` when `isEdgeChallengeResponse(response.status, response.headers.get("cf-mitigated"))`; the install's catch derives `kind` from `error instanceof EdgeChallenged` (a deadline abort stays `"failed"`); `tellTheClients(reason, kind)` elects as the Interfaces block says and posts. `turnstile.ts`'s `isEdgeChallenge` delegates to the shared predicate.

- [ ] **Step 4: Run the client suites**

Run: `cd web && npm run typecheck && npx vitest run && npm run test:wasm` (P-24)
Expected: PASS; the two older `posted` expectations updated with `kind: "failed"`.

- [ ] **Step 5: Commit**

```bash
git add web/src/edgeChallenge.ts web/src/edgeChallenge.test.ts web/src/swMessages.ts web/src/sw.ts web/src/sw.node.test.ts web/src/turnstile.ts
git commit -m "web: the service worker names the edge's challenge (security S3, W-3) — a typed kind on install-failed, one door window elected for it (focused, visible, any) and every other window told failed; one predicate shared with the page; the install path requests nothing under /api, pinned"
```

**Review focus for the reviewer:** the election is decided by the WORKER (the page cannot see its siblings); a game tab is never elected even when focused; the sw bundle still builds under `scripts/precache.mjs` (esbuild) with the two new imports — run `npm run build` once in the export and read that `dist/sw.js` carries the predicate.

---

### Task 6: W-3 — the door passes the challenge and retries on its own · impl: opus · review: opus

Design §3.1 steps 3–6, D-2, D-3, D-14, D-17. On a `"challenged"` message the door runs `passChallenge()` once per page and retries through its existing path; a rejected pass lands on today's `failed`; a click resets the counter; nothing under `/api` is requested.

**Files:**
- Modify: `web/src/swRegister.ts` (`useReadiness`: the message reader at 266–272; the retry at 396–440 split into the public `retry` and an internal one), `web/src/swRegister.test.ts` (the `fakeHost`/`message` harness at 255–272)
- Modify: `web/e2e/offline.spec.ts` (one cell beside cells 8 and 9)
- Docs: `docs/web-lane.md` §8 (the W-3 row and the "Still owed" paragraphs), §15 (the W-3 bullet)

**Interfaces:**
- Consumes: `passChallenge(): Promise<void>` (`turnstile.ts:186`, rejects `EDGE_CHECK` with no handler, `EDGE_CHECK_REPEATED` on a failed check); the message's `kind` (Task 5); `FakeRegistration`/`FakeWorker`/`fakeHost` in the test file.
- Produces: `useReadiness` behaviour — on `kind: "challenged"` the reader FIRST checks the page's own route (`parseRoute(location.hash).kind === "door"` at message time: the worker's election reads `WindowClient.url`, whose tracking of a hash navigation is unmeasured, so the page is the fence — a non-door page treats `challenged` as `failed` and runs no pass; P-26); then, on the door with no automatic pass yet on this hook instance: `passedOnce.current = true`, `passChallenge().then(retryInternal, (e) => { failure/reason = the message's reason; readiness "failed" })`; on `kind: "challenged"` after a COMPLETED pass: `failed` with the reason; DURING the pass: the same pass (P-28); `retry` (the UI's) sets `passedOnce.current = false` then runs `retryInternal`; `retryInternal` is today's retry body (`update()` with an active worker, `register` otherwise).

- [ ] **Step 1: Write the failing cells in `swRegister.test.ts`**

Each with `setChallengeHandler(...)` set in the cell and reset in `afterEach` (`resetTurnstileForTests`):

```ts
it("passes the challenge once and retries through update() when a worker is active, never reading failed", ...)
  // handler resolves; registration with .active; message kind "challenged" → passChallenge called once (a spy on the handler), reg.update called once, readiness never "failed"
it("passes once and registers anew when the first install failed", ...) // no active worker → host.register called a second time
it("a rejected pass lands on failed with the reason and no retry", ...) // handler rejects → readiness "failed", reason the message's, update/register not called
it("with no overlay mounted the door lands on failed at once", ...) // setChallengeHandler(null) → "failed", no retry
it("a second challenged message on the same page draws the button and runs no second pass", ...) // after one pass: handler spy still 1, readiness "failed"
it("a click on Try again resets the counter so its own challenged retry may pass once more", ...) // result.current.retry(); then a challenged message → the handler spy reads 2
it("a challenged message at a page whose route is not the door lands on failed and runs no pass", ...) // P-26: location.hash "#/t/abc"; the handler spy reads 0
it("kind failed takes today's path", ...) // the existing 'names the failure…' cell asserts kind: "failed" in its message
it("the automatic pass and retry request nothing under /api", ...) // vi.spyOn(globalThis, "fetch"): no call whose URL path starts with /api
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cd web && npx vitest run swRegister`
Expected: FAIL (`kind` ignored; readiness `failed` at once).

- [ ] **Step 3: Implement**

In `useReadiness`: a `passedOnce = useRef(false)`; the message reader branches on `said.kind`; the retry body moves to `retryInternal` (a ref-held callback so the message handler, registered once, reaches the latest); `retry` resets the ref and calls it. No new readiness STATE, but a `passing` ref (P-28): during the pass the platform marks the dying worker redundant and `settle` would read `failed` behind the overlay, so the ref holds the readiness until the pass settles; a `challenged` that arrives DURING the pass is handed the same pass (a click then gives one retry, not two); `failed` is set on a rejected pass, an absent handler, a non-door page (P-26) or a challenge after a completed pass.

- [ ] **Step 4: Run the client suites**

Run: `cd web && npm run typecheck && npx vitest run`
Expected: PASS.

- [ ] **Step 5: The e2e cell**

In `offline.spec.ts`, following cell 9's `context.route` shape: on the SECOND release's install (the update path, a controlled browser), one precache fetch is answered 403 with `cf-mitigated: challenge`; the door (keyless) reads `failed` with the sentence "the edge asked for a check" and offers "Try again"; the route then answers normally and the click lands the release. Run: `cd web && npm run e2e -- offline`. Expected: PASS. (This proves the typed kind reaching the door through a real browser and the keyless landing; the pass itself is unit-proven — design D-17.)

- [ ] **Step 6: The docs and the commit**

`docs/web-lane.md` §15's W-3 bullet: the mechanism built (Tasks 5–6, one sentence each), Bot Fight Mode kept on, the live reading owed to the public day's window; §8's W-3 row and the "Still owed" paragraph: the same, and "W-3's other half" struck from the owed list with the reading named as owed.

```bash
git add web/src/swRegister.ts web/src/swRegister.test.ts web/e2e/offline.spec.ts docs/web-lane.md
git commit -m "web: the door passes the edge's challenge and retries on its own (security S3, W-3) — one automatic pass per page through the overlay it already has, the existing retry after it, a rejected or absent pass landing on today's failed, a click resetting the counter; nothing under /api; the lane's §8 and §15"
```

**Review focus for the reviewer:** the loop: a `challenged` recheck after the automatic pass must NOT open the widget (cell 5) — mutate the counter to per-attempt and read it red; the click-reset must not be reachable from the automatic path (mutate `retryInternal` to reset and read cell 5 red).

---

### Task 7: the private test Worker — `yokohama-web-private` at `yokohama-test.moore.nyc` · code half impl: sonnet · review: opus · operations: the orchestrator

Design §3.5 and D-9. Access first over the API (R-S3-12, with the typed skip), then the route and the record, the bucket, the first deploy with the deploy guard and without the secrets guard, later deploys through `deploy:private`.

**Files (the code half):**
- Modify: `worker/wrangler.private.jsonc` (`"routes": [{ "pattern": "yokohama-test.moore.nyc", "custom_domain": true }]` beside `workers_dev`; the header's sentence about the first deploy kept true), `worker/scripts/deploy-guard.mjs:335–341` (`routes: ["yokohama-test.moore.nyc"]`), `worker/scripts/deploy-guard.test.mjs` (the private file's record cell: a private fixture whose routes line names the hostname passes; one naming `elsewhere.example` is refused with the record's sentence — the pattern of the cell at ~1037)
- Docs: `worker/RUNBOOK.md` §26 (the recipe and the record), `worker/README.md` (one line naming the private Worker beside production), `docs/web-lane.md` §1 (the hosts paragraph)

- [ ] **Step 1 (code): the failing guard cell**

Run: `cd worker && npm run test:scripts -- deploy-guard` after adding the cell. Expected: FAIL (the record's `routes` is `null`).

- [ ] **Step 2 (code): the file and the record; run the scripts suite**

Run: `cd worker && npm run test:scripts && npm run typecheck`. Expected: PASS. Commit:

```bash
git add worker/wrangler.private.jsonc worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs worker/README.md docs/web-lane.md
git commit -m "worker: the private Worker's hostname (security S3) — yokohama-test.moore.nyc as its custom domain, the deploy guard's record holding the file to it"
```

- [ ] **Step 3 (operations, the orchestrator): Access first — DONE by the owner, read back**

The owner added `yokohama-test.moore.nyc` to the `yokohama` Access application in the Zero Trust dashboard on 2026-10-20 once the outage lifted, and the orchestrator read it back over the API at 04:4xZ: `self_hosted_domains` and `destinations` both list it, `updated_at` `2026-10-20T04:41:55Z`, the `aud` `5fa7a989…` the private file's `ACCESS_AUD`, the session 24 h. **This step is now a READ-BACK only:** GET the application (`/accounts/{account}/access/apps/11cbcf68-96cd-4310-baee-f2ade812155d`) and confirm the hostname is still in both lists before Step 4's route goes live; on a 504 retry once, then take the recorded read-back as the fact. The typed skip (R-S3-12) stays for a read that finds the hostname GONE.

- [ ] **Step 4 (operations): the bucket**

`wrangler r2 bucket create yokohama-archive-private`; the `test/` prefix lifecycle as production's (`worker/RUNBOOK.md` records production's rule — copy it); read back.

- [ ] **Step 5 (operations): the keyless build and the first deploy**

`cd web && npm run build` (restores the release client; the e2e's debug build sits in `web/dist` today); the assets copied to `worker/assets` by the lane's release steps (`worker/RUNBOOK.md` §23 stage 1 names them); then `cd worker && node scripts/deploy-guard.mjs --config wrangler.private.jsonc && wrangler deploy --config wrangler.private.jsonc`. The secrets guard is NOT run (a Worker that does not exist holds no secret). No operator secret unless the owner asks for one.

- [ ] **Step 6 (operations): the readings**

Over WARP as the owner: `/api/health` (identity `worker`, source `access`, version), a table dealt and a move made, the lobby socket open; from a device off WARP: Access's login. Every later deploy: `npm run deploy:private`.

- [ ] **Step 7 (operations): the record**

`worker/RUNBOOK.md` §26: the recipe (Steps 3–6 in order, with the keyless-build rule for the private Worker after the public day) and the record (the version, the readings, the times). Commit the docs.

---

### Task 8: the release number and the suites one sequence · the orchestrator (a sonnet implementer for the bump is acceptable)

Design D-10; the lane's §7. `0.7.1` → `0.7.2`, by the bump commit `3991feb3`'s file list: `Cargo.toml` (`[workspace.package] version`), `Cargo.lock`, `web/package.json` and its lock, `worker/package.json` and its lock, the wire fixtures that carry the version (`web/src/wire/health.json`; `web/src/wire-script/*.json` regenerated — the version and nothing else, read in the diff), `deploy/Dockerfile.runtime`'s example tag.

- [ ] Bump; run `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures` and read the diff (version fields only); `cargo test -p yokohama-web the_two_npm_packages_carry_the_workspace_version`.
- [ ] Run the suites ONE sequence at the head and record the counts in the SDD ledger and the close: `cargo test -p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`, clippy; `web/`: typecheck, vitest, `test:wasm`, `e2e`; `worker/`: typecheck, `test:scripts`, the pool.
- [ ] Commit: `release: 0.7.2 — security S3 (…)` in the house style.

---

### Task 9: the wrap — the records · the orchestrator

Design D-11 and §1. No code.

- [ ] `docs/security-ledger.md`: the header's "Updated" line; B11 and D22 → `dismissed` with R-S3-1's words; D19, D21, D30, D31 → `recorded` with R-S3-2's; D17's cycle column `S4` with R-S3-3's; D32 → `accepted` with R-S3-4's; D2, D4, D9 → `fixed <hash>` with their catchers named (the cells of Tasks 1–4) and the live column "the window's" where a reading is owed; the AUTH row's note: passkeys `dismissed` (R-S3-6).
- [ ] `docs/plans/2026-10-08-security-remediation-spec.md` §9: "§9.9 — 2026-10-19/20, S3's brainstorm, design and plan" — the rulings R-S3-1–R-S3-13 by pointer to the design, the decisions D-1–D-17 by pointer, the builder check's seven questions and seven corrections by pointer, and the close's readings; then the sentence that the file is FROZEN at S3's close as §9 says.
- [ ] `docs/plans/2026-10-19-security-s3-design.md` §7.3: the close — the commits, the counts, what was read live and what was not (the success path's live reading, the test Worker if skipped).
- [ ] `docs/handover.md`: Track 4's header and body → S3's state (landed on `security-s3`; the PR; what the public day's window owes: W-3's live reading, stage 2); the owner checklist (the test Worker's deploy if skipped; the PR merges).
- [ ] `guidingpath.md`: one row dated 2026-10-20 in the decision log's form (the rulings by id, the design and plan by path, the five pieces one line each, the counts).
- [ ] `CLAUDE.md`: no new standing rule unless the run produced one (the test: a sentence that changes what the next agent does).
- [ ] Commit the records: `git add docs/security-ledger.md docs/plans/2026-10-08-security-remediation-spec.md docs/plans/2026-10-19-security-s3-design.md docs/handover.md guidingpath.md`.

---

## After the tasks (the orchestrator)

- [ ] **The whole-branch "after" review by FABLE** on an export of the final head (the owner's ruling, 2026-10-20): the vacuity lens first (does each sentence the cycle wrote about itself have a mechanism), then every mutant the tasks' review-focus lines name, run once more against the final head, then the lane's rows; its report to a transcript file; findings fixed by the orchestrator or a sonnet implementer as NEW commits with their catchers, a scoped re-review of the fix round, and the review recorded in the design's §7.
- [ ] **D32's record** (the owner's ruling, 2026-10-20, during this plan's hand-over): `/api/me` added to the zone's one rate-limit rule at the edge, since a bound inside the Worker spares no invocation and the edge's refusal costs none; the two testers asked to open the front page once; the ledger's D32 row and `worker/RUNBOOK.md` §23's D32 bullet say so (Task 9 carries the ledger's half; the runbook's half is written at the change).
- [ ] **The PR** `security-s3` → `security-auth`, its description in the house style with the attribution trailer; the owner merges the stack on their word (design D-12).
- [ ] **The board and the memory**: the session's close on `docs/handover.md`; the scratchpad's exports named for the owner to remove (never removed by an agent).
- [ ] **Then Task 10, the window — at the very end, programmatic, waiting on nobody** (the owner, 2026-10-20: "the NAS deploy will need to be layered at the very end of the plan, so it doesn't get hung waiting for me, unless it can be done over portainer and configured programatically" — it can: S2's NAS window, `deploy/RUNBOOK.md` §3.22, was driven from the workstation end to end).

---

### Task 10: the window — `0.7.2` on the NAS, then the Worker · the orchestrator, programmatic · at the very end

Design D-10, R-S3-14 and R-S3-15; the lane's §7 (a stack that merges at its wrap deploys the branch commit the tag names; the release is published before either deploy). **No step waits on the owner:** the build and the context are the workstation's, the image build and the container swap go through Portainer's Docker proxy (the recipe `deploy/RUNBOOK.md` §1, §2, §3 and §3.22's record), and every reading is taken from the workstation over WARP (`curl`) or by an exec inside a container. The owner's own browser read is welcome and asynchronous. **If Portainer, the LAN file server or WARP is unreachable, the window is SKIPPED as a typed, recorded deferral** and the owner's checklist carries it (the fallback item below).

- [ ] **Step 1: the tag and the release.** On the branch head after the Fable review's fix round and the scoped re-review: the annotated tag `v0.7.2` (`Yokohama CLI v0.7.2 — security S3: …`) on that commit, the branch and the tag pushed, `release.yml` run to completion (the CLI's gate `piped_dogfood` in it); the run id recorded.
- [ ] **Step 2: the NAS build.** The musl binary per §1 (`rust-lld`; the size and the `file` line recorded), the NAS's dist with `YOKOHAMA_SITE_URL=https://yokohama-nas.moore.nyc` (§3.22's values), the context cut with `COPYFILE_DISABLE=1` (entries counted, zero `._`, `.dev.vars` absent) and served from a scratch directory on the LAN (the port recorded); the image `yokohama-web:0.7.2-s3` built through Portainer's Docker proxy from that context.
- [ ] **Step 3: the smoke, with the hostname.** `bash deploy/smoke-image.sh yokohama-web:0.7.2-s3 18080 yokohama-nas.moore.nyc` against the built image (locally or on the NAS as §3.22 did): the health read, the header set, AND the two host probes (200 with the hostname, 421 with the foreign name) — pasted into the record.
- [ ] **Step 4: the swap.** `yokohama-web` stopped and deleted; the new container created from §3's body — the `Env` array with the FOUR existing entries PLUS `YOKOHAMA_WEB_HOSTNAMES_ENABLED=on` and `YOKOHAMA_WEB_HOSTNAMES=yokohama-nas.moore.nyc`, `cf-internal`, `unless-stopped`, the volume, the hardened `HostConfig` (`CapDrop ALL`, `no-new-privileges`, 512 MiB) — started; the `HostConfig` and the `Env` read back through Portainer; the start lines in its log (`answers host names: …` naming the hostname; `listening on http://0.0.0.0:8080`).
- [ ] **Step 5: the readings, all from the workstation.** Over WARP: `/api/health` (`server_version 0.7.2`, `semantics_rev 7`, `identity cf-access`, the TTLs), `/api/me` naming the owner, the header set on `/` and `/api/health`; by `docker exec` inside `yokohama-web` (`wget`): `127.0.0.1:8080/api/health` with `Host: yokohama-nas.moore.nyc` 200 and with `Host: smoke-foreign.example` 421; the negative probe from the terminal container as §3.22 did; D2 on the NAS: 21 wrong passwords at a scratch locked table over WARP → 20 × 403 then 429 with the sentence; D9: a scratch table's seat reopened through the API; the sweep's log line unchanged in shape. A STOP for the owner only on a reading that contradicts a cell.
- [ ] **Step 6: the Worker.** `cd web && npm run build` (keyless, stage 1's shape), the assets copied as `worker/RUNBOOK.md` §23 stage 1 names, `cd worker && npm run deploy`; the update reading (`/api/health` version, `/sw.js` naming the new release), D2 and D9 read through the API over WARP as on the NAS, the W-3 message shape read in a browser's console if the owner is at hand (optional; the live reading of the pass itself is the public day's — design D-17).
- [ ] **Step 7: the records.** `deploy/RUNBOOK.md` §3.23 (the twentieth deployment: every figure above, the rollback line — `yokohama-web:0.6.3-s2` stays on the NAS); `worker/RUNBOOK.md` §27 (the thirty-eighth deployment); `docs/web-lane.md` §1's host paragraph (both hosts at `0.7.2`); the ledger's live column for D2, D4, D9; the board; one commit on the branch, and the PR's description amended with the window's line.
- [ ] **The fallback, if skipped (the owner's item):** the create body carries both variables; the smoke runs with the hostname as its third argument; the site is opened through the tunnel BEFORE the deploy is called done — a body that forgets the list answers 421 to everything while the loopback smoke passes. Written on the board's owner checklist at the close.

## Pre-flight record (the orchestrator, 2026-10-20)

The "before" review, run from the ledger's scan (one row per pair of tasks sharing a file or an interface, one per task against itself) and a read of Tasks 1–2's cells against the design's §3.3–§3.4 and the code at `302babdb`. Corrections written INTO the tasks above (the plan's text is the single source a brief is cut from):

- **P-1 (Task 1, a plan defect):** the crate's in-process test requests carry no `Host` header, so the check on by default would turn every existing route cell 421; every request through `app()` now carries `Host: localhost`, and `request_with_host(.., None)` is the one path without — Step 5 says so. Cost if wrong: mechanical edits in test modules.
- **P-2 (Task 1):** an absent `Host` header falls back to the URI's authority before the refusal (HTTP/2's `:authority` surfaces there under hyper's auto builder); the no-header cell builds a request with neither. Cost if wrong: one line.
- **P-3 (Task 1):** three process cells, as Step 9 lists them (the Files line said two).
- **P-4 (Task 2):** the design's "both routes share the counter" cell was missing; added beside `the_unlock_route_counts_the_same_way`, with its placement rule. Cost if wrong: one cell.
- **P-5 (Task 2):** `Games`'s literal is `Self { … }` in `with_parts`; the grep names both spellings.
- **P-6 (Task 3):** the hand-kept route copy is at `api.rs:4132` (the plan said 4118); every brief says probe every cited line.
- **P-7 (the design, D-2 against D-14):** D-2 reads "one automatic pass per install attempt" and D-14 "the pass counter is the page's" — D-14 is the later decision (the builder check's question 4) and §3.1 step 4 agrees; Task 6 follows D-14. Recorded for the design's §7 at the wrap.
- **P-8 (Task 1 Step 13, operations):** Docker's daemon is down on the workstation at the run's start; the implementer starts Docker Desktop once (`open -a Docker`, up to 90 s) for the smoke's two runs; failing that, the two smoke runs are NOT taken by the task and the window's Step 3 takes them (recorded in the task's report as a concern).
- **P-12 (Task 1, after the implementer's DONE_WITH_CONCERNS — concern 2):** the plan's cell refused `"ON "`, while the design says the switch "reads as the layer's other switches do" and `Multiplayer::from_env_value` trims and lower-cases for a stated reason (one operator's `docker run` line); the design wins — the switch is read trimmed and case-insensitive, `""` still refused. The implementer's strict reading followed the plan's cell exactly; corrected by a new commit before the review. Cost if wrong: an operator's `ON` accepted instead of refused by name.
- **P-13 (Task 2, after its DONE_WITH_CONCERNS):** the window test is `now.saturating_sub(started) >= WINDOW_SECS` — the plan's `started + WINDOW_SECS <= now` panics in debug near `u64::MAX`; pinned by a seventh unit cell.
- **P-14 (Task 2):** the refused-check and prune cells read the count and the map white-box, because under a fixed window a counted refusal is invisible from outside and `live` prunes for itself.
- **P-15 (Task 2):** Step 6's filter named one cell of five; the five are named.
- **P-16 (Task 2):** the counter's key is the stored document's id and both routes share one private helper, one clock reading per guess.
- **P-17 (Task 2, after its review):** one lock acquisition per guess through `Guesses::guess(.., correct) -> Guess`, so the counter's own lock is the serialization (the review's Minor 2, plan-mandated by the split interface); the window's anchor pinned by a cell (Minor 1, the one surviving mutant).
- **P-18 (Task 3, the implementer's pre-code probe):** the wire seat is 0-based as `Keep`/`Join` send it; the 400/409 cells re-cut.
- **P-19 (Task 3):** the creator check runs first, then status, then seat — the design's order and the status-leak ruling #8; three existing sweeps are the catchers.
- **P-20 (Task 3):** the dealing screen had no Abandon control at HEAD, so design rule 3's "added there" applies and Task 3 builds it; §16's metered ledger gains the reopen row.
- **P-21 (Task 3, after its review):** a reopen's 409 re-reads the table once as `join`'s does (the review's Minor 3); P-19's catcher list trimmed (the seat holder's loop skips /reopen).
- **P-22 (Task 4, after its DONE_WITH_CONCERNS):** Step 2's predicted reds and Step 1's "the log line is the kept one" corrected — the old code reads `None`, and a kept table gets no line.
- **P-23 (Task 5):** `isDoorUrl` also refuses non-`http(s)` URLs and the server's own paths (`/enter`, `/api/…`, `/cdn-cgi/…`) — the orchestrator's hash-only restatement would have elected them.
- **P-24 (Task 5):** `sw.node.test.ts` runs under `vitest.node.config.ts` (`npm run test:wasm`), not the default config, which excludes `*.node.test.ts`.
- **P-25 (Task 5):** the election cell orders each window list adversarially.
- **P-26 (Task 6):** the door's reader checks its own route before an automatic pass — the page is the fence while `WindowClient.url`'s hash tracking is unmeasured.
- **P-27 (Task 5, after its review):** the election reads load addresses; the page's route fence is load-bearing; the reverse corner recorded; D-13's premise amended at the wrap.
- **P-28 (Task 6, after its DONE_WITH_CONCERNS):** a `passing` ref holds the readiness while the pass runs (the dying worker goes redundant mid-pass), and a mid-pass challenge joins the same pass.
- **R-8 supersedes the next bullet: every export builds in its OWN `target/`** (cargo keys a workspace crate by its path relative to the workspace root, so a shared target runs the other tree's artifacts as fresh — found by Task 2's implementer).
- **Exports share the checkout's `target/`** (WITHDRAWN by R-8, kept for the record) (a symlink inside each export; 51 GiB free beside an 87 GiB target): cargo's lock serializes the builds and path-keyed artifacts coexist. The baseline counts are in the SDD ledger.

## Self-review (run by the author, 2026-10-20)

- **Spec coverage:** §3.1 → Tasks 5–6 (steps 1–7 all placed; rule 7's cell in both); §3.2 rule 1 → Task 3, rule 2 → Task 4, rule 3 → BUILT by Task 3 (P-20: the dealing screen had no Abandon control at HEAD — the self-review's "verified, no build" was wrong); §3.3 → Task 2; §3.4 → Task 1; §3.5 → Task 7; D-10 → Task 8; D-11 → Task 9; D-12 → Before Task 1 and After. The design's "the client's 429 does not route through the stream's end" → Task 2 Step 10's note (the join is a `call`, not a stream; by construction).
- **Step scan:** every code step names its signature and file; every test step names its cells and assertions; no step says "handle" or "appropriate".
- **Type consistency:** `Hostnames`/`with_hostnames`/`misdirected` (Task 1) used nowhere else; `Guesses`/`GameError::TooMany` (Task 2) used in Tasks 2 only; `Reopen`/`Games::reopen`/`api.reopen` (Task 3) match across layer, Worker cell and client; `InstallFailedKind`/`isDoorUrl`/`isEdgeChallengeResponse` (Task 5) consumed by Task 6 under the same names.
- **Review Focus:** six lines, each with its cell in the owning task.
- **Proportion:** the plan is shorter than the code it describes; bodies appear only where a signature and a cell leave a choice (the election order, the window arithmetic).
