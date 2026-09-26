# M6.5c "the objects" — the spec

**Written 2026-09-22 on branch `web-room-m6.5c` (stacked on `web-room-m6.5b2` at `2621fbe`, the
M6.5c handover's HEAD; the branch's first commit `952a68c` only ignores `.dev.vars`), from the
brainstorm held with the owner that evening, from the entry handover
`docs/plans/2026-09-22-web-frontend-m6.5c-handover.md`, and from a PLATFORM SPIKE the owner
authorized mid-brainstorm that ran on the real account before the design was written (§0.3).
Revised the same night — this is the second text — after an Opus adversarial read of the first
(`f80e2f7`: 0 CRITICAL / 5 HIGH / 16 MEDIUM / 16 MINOR, 47 claims verified true, every finding
accepted) and after four owner additions made while that read ran (R-6.5c.14–16 and the single-player
invocation points); then touched up after a scoped re-review of that revision (`8dc9096`: of the 37,
32 addressed and 5 partly; new, 0 CRITICAL / 1 HIGH / 3 MEDIUM / 18 MINOR, every finding accepted).
§9 traces every change to its reason.** This document is M6.5c's authority under
the umbrella `docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` (R-6.5.1–13, as amended by
R-6.5b.16 and R-6.5b2.2); the plan is written from it; the close is its record. Where it and the
umbrella disagree, this document is the later ruling and §0.6 says exactly where.

**The headline.** Hosted multiplayer moves onto Cloudflare at `yokohama.moore.nyc`, behind the same
Access application as today. A stateless Worker serves the release as static assets without being
invoked, and runs only for `/api/*`, `/enter` and a missing file: it validates Access's assertion,
resolves the caller, and forwards to a Durable Object by name. One object per table is that table's
store, lock and signal; one house object is the lobby's projection and the house chat; one archive
object per caller strict-replays what a browser sends. Every object runs the SAME table layer the
NAS runs — `api::dispatch` over `Games` — in its own instance of a WebAssembly module, so every
payload is the layer's own and a trap costs one instance, never a table. The signal is a
hibernatable WebSocket carrying the same frames the SSE stream carries. Finished and abandoned games
land in an R2 bucket in the file store's own layout — behind a switch that turns the archive off
cleanly — and a table left without activity for a configured number of days is saved and expired.
**The code that changes outside the new host is named, and it is small**: two additions to the
layer (§1, R-6.5c.4), one transport arm and one outbox branch in the client, and two new request
scripts written by the corpus emitter; the NAS, the browser module, the CLI and the five fenced
crates stay as they are. (The runbook, the CI workflow and `CLAUDE.md` move with the cycle, and
`deploy/worker/` and `deploy/modal_app.py` are deleted — §3.3.)

**Read order for the session that writes the plan:** §1 (the rulings), §2 (the mechanism), §5 (the
instruments and the battery — the plan's tasks end in these), §3 (the deployment and the first
deploy), §6 (what is not claimed), §7 (the handover's opening items, each placed), §9 (what the
review changed).

---

## 0. What is true at HEAD, and what this spec relies on

### 0.1 HEAD

- Branch `web-room-m6.5c` over `2621fbe`. Seven PRs open and stacked, nothing merged, merge order
  the owner's (handover §1). **LIVE on the NAS: `yokohama-web:0.5.0-m6.5b2.1`** at
  `yokohama-nas.moore.nyc` (RUNBOOK §3.16), `identity: claimed`.
- `SEMANTICS_REV` 7; `crates/yokohama-core`, `-oracle`, `-difftest`, `-bridge`, `gen-data` and
  `crates/yokohama-cli` byte-identical to `369a1f2` (M6.5b2 close §1.11; the review re-read it:
  `git diff --stat` empty).
- **`yokohama.moore.nyc` has NO DNS record.** The dead tunnel CNAME
  (`6a6c468e-5e1e-4a7c-88a6-4e2fe07da1fc.cfargotunnel.com`, proxied, ttl 1) was removed for the
  spike and not restored; the name is held for this cycle's Worker.
- `deploy/worker/` and `deploy/modal_app.py` are still on disk (R-6.5.13 removes them, §3.3).

### 0.2 The layer's surface the new host consumes (opened at `2621fbe`; the review verified each)

- **The front door**: `api::dispatch(games, host, identity, req) -> ApiResponse`
  (`crates/yokohama-web/src/api.rs:1984`) over `ApiRequest { method, path, query, content_type,
  body }` (`:34`); `Host { identity_name, signal_kind, serves_games, multiplayer_offered }` (`:54`);
  the route table `api::routes(serves_games)` (`:149`); `api::parse_query` (`:126`). The body parser
  `api::parse_json` (`:97`) is public and is axum's `Json` extractor call for call; the create body
  `tables::NewGame` (`tables.rs:140`) is public and fenced by `deny_unknown_fields`, and its `id` is
  `#[serde(default)] Option<String>` (`:162`) — so a body carrying `"id": null` reads as ABSENT and
  `create` draws its own id (`:715`). The route matcher binds a path segment RAW, with no
  percent-decoding, because an id is `is_safe_id`'s ASCII (`api.rs:196-210`). `ApiError`'s three
  fields are public (`lib.rs:117-129`); the body builder `error_response` is `pub(crate)`
  (`api.rs:80`).
- **What `dispatch` does NOT serve**, because axum's ROUTER owns it: `GET /api/games/{id}/events`,
  `GET /api/games/{id}/stream`, `GET /api/stream` (`lib.rs:883-885`), `GET|HEAD /api/art` (answered
  by the adapter before `dispatch`, `lib.rs:757-765`) and `GET /enter` (`lib.rs:845`, the route at
  `:888`). This host serves them itself — the transport routes through the public
  `api::events_at(games, id, user) -> Result<Events, ApiError>` (`api.rs:1641`), whose `Events`
  (`:1545`) carries `your_turn`, so **a frame is per viewer**. Axum's `events` handler
  (`lib.rs:547-603`) refuses unknown query keys and a `wait` above 25 with a 400 and HOLDS on `wait`;
  its `stream` handler ENDS a connection whose admission changes under it with
  `end {"reason":"gone"}` on a 403 or 404 and writes a comment on any other failure
  (`lib.rs:677-690`); the SSE frames are `format!`ed (`:721-724`).
- **The body bound**: the axum adapter reads at most **2 MiB** and answers `400` `"the body could not
  be read: …"` above it (`lib.rs:734-740`).
- **The state**: `Games::with_parts(rules, store, chat, signal, clock, archive)` (`games.rs:287`);
  `Games::handle(id)` is **public** (`:391`) and the `Repl`'s `sub: SubPrompt` is a public field
  (`crates/yokohama-cli/src/repl.rs:43`; the enum at `:190` has **no derives at all**);
  `Games::load_document` / `read_document` (`games.rs:352`, `:359`) are the side channel
  `yokohama-wasm` uses. A cold `handle` PUMPS an owed bot move and persists it (`lib.rs:520-538`).
- **The seams**: `Store` is synchronous (`store.rs:121`); `Signal` is four methods — `bump(id)`,
  `bump_table(id)`, `bump_house_chat()` (no id), `end(id)` (`signal.rs:38-50`); `Archive::put(prefix,
  why, id, record_json, table_json) -> Result<PutOutcome, PutError>` is synchronous (`archive.rs:93`),
  `PutOutcome::{Landed, Identical}` (`:44`), `PutError::{Conflict, Failed}` (`:62`), ONE layout
  `archive::paths` (`:119`), and `strip_for_archive` (`:160`) keeps every field but the lock and the
  watcher roster.
- **`DirArchive::put`'s order** (`archive.rs:256-283`): READ the record path — the same bytes are
  `Identical`, different bytes `Conflict`, absent the only road to a write — then OVERWRITE the table
  ("overwriting a half pair's table is the repair"), then write the record.
- **The two archive sites**: game over — `persist` (`games.rs:485`) calls the PRIVATE
  `archive_if_over` (`:682`), which stamps `archived_at` from the clock INTO the table body it puts
  (`:690`), stamps the stored table only after a landed put, logs a failure, and retries its stamp
  once on a version conflict (`:692-724`); abandon — `Games::delete` (`tables.rs:1655`) checks that
  the caller is SEATED and is the CREATOR (`:1660-1668`), stamps the archived copy from the clock
  (`:1700`), puts BEFORE the tombstone (`:1724`) and the unlink (`:1725`), and REFUSES the delete on
  any put error — so a deferred put leaves its first run side-effect-free. **The creator is always
  seated**: `config_of` refuses a create whose creator holds no seat (`tables.rs:271-274`) and `join`
  fills only an `Open` seat, so `delete(id, created_by)` passes both checks for the table's whole
  life. The browser's save — `api::archive_post` (`api.rs:1915`) — runs `check_pair_with` and puts
  under `browser/<caller_segment>/`, where `caller_segment` is the encoded address TRUNCATED to 64
  characters (`archive.rs:139-153`).
- **The move**: `api::input` is public (`api.rs:1031`) and persists only when the event count or the
  lock changed (`:1157`); a picker click that applies nothing exists ONLY in the resident `Repl`
  (`Input::expect`'s doc, `:989-1011`). A table chat post bumps the table (`chat_append` →
  `bump_table`, `tables.rs:958`).
- **The table document**: `TableDoc` (`table.rs:170`) with public `seed`, `keeps`, `lock`,
  `unlocked`; `TableDoc::view(user)` (`:371`) reads the lock and each keep for their PRESENCE only;
  `Lock { salt, hash }` (`:104`). **`check_shape` (`tables.rs:362`, `pub(crate)`) accepts a keep only
  as `None | Some(1 | 2)`** (`:444-449`, "the dealt cards are numbered 1 and 2") and accepts a
  zeroed 32/64-hex placeholder lock (measured by the review). The lobby is `Games::list(user, scope,
  limit, before)` (`:1556`) over `Doc::Table` alone, SKIPPING any document `check_shape` refuses
  (`:1593-1596`), 20 rows to a page by default (`LIST_LIMIT_DEFAULT`, `:680`).
- **A stored record carries no clock**: its fields are `engine_version`, `rules_hash`, `seed`,
  `config`, `choices`, `events`, `semantics_rev`, `terminal_digest` and `run` (read off the committed
  `tests/fixtures/browser-played-2p.json`), so a bot-free game replayed from the same requests writes
  the same record bytes at any hour.
- **Identity**: the layer's three modes (`identity.rs`), each reading exactly one header; under
  `worker` the literal `anonymous` is ABSENT (`:162`).
- **The layer's diagnostics are `eprintln!`, which is a SILENT SINK on `wasm32-unknown-unknown`**
  (measured by the review: `list`'s skip arm ran in the browser module, printed nothing, did not
  trap).
- **The browser host** `crates/yokohama-wasm` keeps ONE global `Games` (`src/lib.rs:38`) and a
  whole-second clock `(Date.now() / 1000.0) as u64` (`:62-63`); `set_clock` is one global setting
  (`:209-218`).
- **The corpus emitter** strips every host-owned path from the request script
  (`crates/yokohama-web/tests/wire_fixtures.rs:2131`, `hosts_own` at `:215-220`: `/events`,
  `/stream`, `/api/stream`, `/api/art`), and the eight `archive_*` fixtures come from a second `Api`
  whose requests are never written out (`:1735-1739`); the M6.5b node replay lists those twelve in its
  `NOT_SERVED` set (`web/src/local/corpus.node.test.ts:141`). The walk script has 2,197 rows, 74
  named, and **no DELETE rows**; its creates carry fixed ids (`wire2` at row 0, `wire` at row 1,
  `wire4` at row 2175), and no row sends a page cursor (the lobby reads are `scope=open`,
  `scope=mine&limit=1` and `scope=playing`, rows 4, 11 and 26). The second `Api`'s first send ASSUMES
  AN EMPTY ARCHIVE — it asserts a 201 at `browser/red_at_wire.test/rev7/over/wire.json`
  (`wire_fixtures.rs:1761-1764`; "each must meet an EMPTY archive or the first send is the once-rule's
  200", `:1822-1823`); and `art_manifest.json` is CONSTRUCTED from the art directory, never requested
  (`:2077-2082`).

### 0.3 What the platform spike MEASURED (2026-09-22, owner-authorized, throwaway, torn down)

A throwaway Worker `yokohama-spike` — one SQLite-backed class, the real module (`yokohama-wasm` at
`2621fbe`, `release-wasm` + `debug`, `wasm-opt` **133** on this workstation against CI's pinned
132: 1,783,435 → 1,175,167 B), `workers_dev` and preview URLs off, observability on,
`compatibility_date` 2026-09-15 with no flags — deployed ONCE as the custom domain
`yokohama.moore.nyc`, measured, and deleted with its namespace and domain. The full record is the
gitignored `docs/temp/m6.5c-spike-readings-2026-09-22.md` (corrected in place where the review
showed a reading overstated); its source stays in the session scratchpad and is not committed.

| # | What | Reading |
|---|---|---|
| P | Protection, from an unenrolled vantage (alpine on the NAS) | every path — `/`, `/sw.js`, `/api/health`, `/enter`, the object routes — **302 to the Access login** (`auth_status: NONE`, `is_warp: false`); a WebSocket upgrade with no credential **403**; `yokohama-spike.bdvc.workers.dev` **404** |
| 1 | **O-6**: identity on HTTP and on a WebSocket upgrade | over WARP: `Cf-Access-Authenticated-User-Email` and an `email`-bearing JWT on HTTP, **on the 101 of an HTTP/1.1 upgrade**, and in a socket opened from PAGE JavaScript (headless Chromium). With the `yokohama-worker` **Service Auth** credential: `email: null`, JWT `common_name` = the credential's client id, and Access sets `CF_Authorization` (HttpOnly, SameSite=None, ~30 days); **Service Auth outranks the WARP identity**; with the credential's headers removed after the first navigation, **the cookie alone** carries it on a page-opened upgrade and on `fetch` |
| 2 | **O-2**: size and startup | Total Upload **1,172.84 KiB / gzip 453.90 KiB**; **Worker Startup Time 1–3 ms** across five deploys |
| 3 | **O-7**: `getrandom` in an object | `create` inside the object drew `sim-c7e7f98029f58c13` and its setup seed through `crypto.getRandomValues`; **20 ms CPU**. The spike ran with **`nodejs_compat` ON** — compatibility dates from 2026-08-04 enable it by default — and the locked `getrandom` 0.2.16 tries Web Crypto FIRST (`js.rs:71-81`), so Node compatibility is not an obstacle (the review also created a table from the module under Node 26 ESM) |
| 4 | **O-1**: a trap | `debug_trap` → `RuntimeError: unreachable`, caught at the JavaScript boundary; the instance discarded; the next request re-instantiated, reloaded from SQLite and replayed correctly; 1 ms. **It needs a factory-wrapped glue**: wasm-bindgen's `--target web` output keeps `let wasm` at module scope and `initSync` returns early once it is set, so a trapped instance could never be replaced and every object in an isolate would share ONE `GAMES` |
| 5 | **CPU on Free, and O-3** | one object invocation — a cold replay of 595 events plus seven bot-move `why` reconstructions — **`cpuTime` 376 ms, outcome `ok`**, on Workers Free. The cold path alone for the committed FINISHED 595-event game (a fresh instance, the 54,103 B record and 554 B table loaded from SQLite, `Session::from_record`, the frame): **19 / 27 / 22 / 28 ms**. `why` ≈ 50 ms per bot move. The routing Worker — which DECODED the JWT and did not verify it — **0–1 ms**. One instance's linear memory: 1,441,792 B after a replay, 1,900,544 B after the `why` run. CPU read from `wrangler tail --format json` (`cpuTime`) |
| 6 | **Hibernation** | a hibernatable socket with an attachment (`viewer`, the Access identity, `openedAtWake: 1`): after **35 s idle** the next message found **wake 2** — the object evicted and its constructor re-run — with the socket still open and **the attachment intact**; `setWebSocketAutoResponse(ping → pong)` answered before and after |
| 7 | **Static assets** | of the 153 precache entries, **152 byte-exact** (200, type essence, SHA-256) under the defaults; **`/index.html` answered `307 /`** under the default `html_handling` — the service worker fetches it with `redirect: "error"` (`web/src/sw.ts:190-191`), so its install would FAIL; `not_found_handling: "single-page-application"` answered **200 HTML for a missing `/assets/*.js` and `/sw2.js`**. **The right configuration**: `html_handling: "none"`, `not_found_handling: "none"`, a `_redirects` line `/ /index.html 200`, `_headers` (§2.3) — `/` and `/index.html` 200, headers exact, **zero Worker invocations** for `/`, `/index.html`, `/sw.js`, a hashed bundle and the art; **a MISSING path is answered BY THE WORKER** (the platform: "If no matching asset is found and a Worker script is present, the request will be processed by the Worker"). The art is NOT in `web/dist`: the asset directory is assembled from `dist` and `crates/yokohama-web/assets/art/` (154 files, 9.8 MB). **The service worker installed completely from the Worker host**: active, `yokohama-release-d62d6763649a749f`, 154 entries, `/index.html` cached clean at 2,387 B, the door reading "Ready to play offline" |
| 8 | **The edge rewrites HTML** (handover §4 item 27) | zone `moore.nyc` (Free Website plan): **Bot Fight Mode ON, JavaScript detections ON, Web Analytics auto-install ON**. JS detections inject a `challenge-platform` script into navigations and into plain `curl`/Node fetches of `/` (3,325–3,692 B, varying, against the built 2,387 B) but **not into the service worker's own fetch** — which is why the NAS install succeeded. The Web Analytics beacon **fails CORS and logs a console error on every page load**. Headless Chromium was not challenged by Bot Fight Mode |

**Not measured by the spike:** whether a `ping` during hibernation wakes the object; a non-WARP
browser; a phone; R2; alarms; object-to-object calls; the cost of VERIFYING the JWT.

### 0.4 Measured after the spike (2026-09-22)

- **Playwright's context headers reach a WebSocket upgrade opened from page JavaScript** (Chromium
  151.0.7922.34, a local Node upgrade server, no deploy): `X-Yokohama-Claimed-User` and
  `CF-Access-Client-Id`, set as `extraHTTPHeaders`, both arrived on the handshake. So the test
  harness can be several people on sockets as well as on fetches with no client change (R-6.5c.3).
- **The platform's output gate** holds "any new outgoing network messages … until the write has
  completed … it is impossible for anything else in the world to observe a premature confirmation"
  (Cloudflare, *Durable Objects: Easy, Fast, Correct — Choose three*); the rules page's own example
  names "responses, fetch requests". This spec relies on the gate for the frame-after-write property
  (§2.4) and records the WebSocket case as resting on that sentence (§6).
- **The review's measurements** (its report, `.superpowers/sdd/2026-09-22-web-frontend-m6.5c-spec/
  review-spec.md`, gitignored): a `keeps → Some(0)` copy drops out of `GET /api/games` silently
  through the committed module; a `serde_json::Value` round trip of a create body turns a
  duplicate-key 400 into a 201 and moves an unknown-field column (`Cargo.lock:554` has neither
  `preserve_order` nor `arbitrary_precision`); `replay_strict` costs 0.63–0.95× `Session::from_record`
  natively on the 595-event fixture.
- **The scoped re-review's** (`rereview-spec.md` in the same folder): through `api::dispatch`, the first
  revision's textual-insertion rule answered `"id": null` with **201 naming an id no object held**
  (object `sim-00000000000000aa`, reply `sim-f8539ce029926c0a`), and moved a missing field's column from
  **68 to 96**; §0.4's WebSocket-header reading reproduced under Playwright 1.62.1. Cloudflare's test
  page says `evictDurableObject` "waits up to 30 seconds for in-flight requests to drain" and
  `abortAllDurableObjects()` tears instances down "without waiting for in-flight requests to drain";
  both helpers, `runDurableObjectAlarm` and `runInDurableObject` ship in `@cloudflare/vitest-pool-workers`
  0.22.0 and in `@cloudflare/vitest-plugin` 1.2.3, the package that page now documents (read off both
  tarballs, 2026-09-22).
- **The plan's review measured the pool** (`.superpowers/sdd/2026-09-23-web-frontend-m6.5c-plan/review-plan.md`;
  `@cloudflare/vitest-plugin` 1.2.3, workerd 2026-09-21):
  - the pool's RPC resolver REFUSES an own property (`test-internal.mjs:510-521`), so a method patched on
    a live INSTANCE makes every RPC call throw rather than hang; a patch on the PROTOTYPE works, and it
    survives `abortAllDurableObjects()`;
  - a stub taken before `abortAllDurableObjects()` is dead after it;
  - Durable Object storage and R2 are isolated per test FILE, not per test;
  - a WebSocket close reason longer than 123 UTF-8 bytes throws (`websocket_close_reason_byte_limit`,
    on at this compatibility date).

### 0.5 The account and the door

- **Workers FREE** (R-6.5b.16), account BDVC. **Account-wide usage, last 7 days** (GraphQL,
  2026-09-22): **17k–33k Worker requests/day** against Free's 100,000 and **300–1,800 Durable Object
  requests/day** against 100,000 — both SHARED with the account's ~97 other Workers; static-asset
  requests count against neither. Rows written (Free: 100,000/day) are shared with twenty other
  SQLite namespaces and were not read.
- **The door**: the `yokohama` self-hosted Access application (`11cbcf68…`, audience `5fa7a989…`),
  destinations `yokohama.moore.nyc`, `yokohama-cli.moore.nyc`, `yokohama-nas.moore.nyc`; policies
  `sbaitso-friendly` and `yokohama-players` (email lists; allow) and, added by the owner this evening,
  **"yokohama ST bypass"**, `non_identity` (Service Auth), precedence 3, including the service
  credential **`yokohama-worker`** (created 2026-09-23T01:01Z, expires 2031), whose client id and
  secret live in the repository root's `.dev.vars`, **gitignored since `952a68c`**.
- **wrangler 4.135.0** when this was written (**4.136.3** since 2026-09-23), logged in by OAuth with
  `workers`, `workers_routes`, `workers_scripts` write. Its scope list names no R2 read or write
  scope, **but the login reaches R2**: `wrangler r2 bucket list` and `wrangler r2 bucket info
  yokohama-archive` both answered on 2026-09-23 (§9.5). The bucket was created through the account
  API before that was known (§3.1).

### 0.6 What this spec supersedes or amends

- **Umbrella §2.1's host table** ("`yokohama-worker` (new; workers-rs)") — **R-6.5c.2**: a
  TypeScript shell over a wasm-bindgen module; the crate keeps the name.
- **Umbrella §2.2's route map** puts `/api/health`, `/api/me`, `/api/labels` on the Worker — here
  they are the **house's**, so the Worker never loads the module (§2.3).
- **Umbrella §2.3's H-4 options** — **R-6.5c.7** takes a fourth: host-local re-fed input lines.
- **Umbrella §2.3's pending-deliveries LIST** — **one durable slot per table** (R-6.5c.8).
- **Umbrella §2.3's "delete removing the house row BEFORE `deleteAll()`"** — **eventual** toward the
  lobby, through a MARKER that outlives the documents (R-6.5c.8).
- **Umbrella §2.4's "an index row per table"** — **scrubbed copies** listed by the layer's own `list`
  (R-6.5c.8).
- **Umbrella R-6.5.4's "the R2 binding's presence is the switch"** — **R-6.5c.14**: an explicit
  variable, with the binding required when it says `r2`.
- **Umbrella §2.6's `claimed` door "as a query parameter"** — **R-6.5c.3**: the header, on requests
  and upgrades alike; `?as=` is not built.
- **Umbrella §3.3's "e2e under `wrangler dev` AND against the live deployment"** — **R-6.5c.12**:
  Cloudflare's vitest pool (workerd, the runtime `wrangler dev` runs) for the object tier, and the
  live deployment for the browser tier.
- **Umbrella §4's H-4 test, "paused across an observed wake"** — run natively over a rebuilt `Repl`
  and in the pool over `evictDurableObject` (§5); the live exit reads real wakes.
- **Umbrella R-6.5.12 / §5's "Workers Paid"** stay amended by R-6.5b.16 (Free); **R-6.5c.5** records
  the CPU assumption. **Umbrella §5's Worker name `yokohama-web`** is kept.

---

## 1. The rulings (the owner, 2026-09-22, in the brainstorm's order)

Each is an owner decision, or a recommendation the owner approved by name. A sub-cycle may ask to
revisit one; none is re-litigated inside it.

- **R-6.5c.1 — at the close, `yokohama.moore.nyc` is private; the Worker is built for the public
  door.** Owner: *"it will eventually leave access but not at the end of 6.5c … we will deploy to
  yokohama.moore.nyc and it will be access protected but we will plan for the front door mechanics
  and the rest to not be protected eventually."* The hostname-wide `yokohama` application stays in
  front; nothing is public and **the card art is not published this cycle**; the Worker is built so
  that going public later is a CONFIGURATION change — static single player, R-6.5b2.2's protected
  prefixes plus `/api/admin` (R-6.5c.16), identity that fails closed. Decides R-6.5b2.6 for this
  cycle.
- **R-6.5c.2 — the host is a TypeScript shell over a per-object wasm-bindgen module, not
  workers-rs.** Chosen on evidence read from workers-rs 0.8.6's own tree (`86092a8`): hibernatable
  WebSockets present but exercised by one echo test and one "set and read back the auto-response
  pair" test, no hibernate-and-wake test, no README section; a panic "will terminate the isolate"
  without `--panic-unwind`, which needs NIGHTLY and a rebuilt `std` against this repository's pinned
  stable 1.97.1; RPC "experimental". R2 and SQLite are proven there; the testing and portability
  arguments were a wash. **Amends umbrella §2.1.**
- **R-6.5c.3 — identity, in order.** (1) The Worker VALIDATES `Cf-Access-Jwt-Assertion` (RS256
  against the team's keys, `aud`, `iss`, expiry), so a missed Access destination fails closed. (2) An
  assertion naming a PERSON (`email`) is the user; every client `x-yokohama-*` header is removed and
  ignored (R-6.5.11). (3) An assertion naming ONLY the `yokohama-worker` Service Auth credential
  (`common_name` equal to its configured client id, no `email`), with `YOKOHAMA_DEV_DOOR=on`, honours
  the `X-Yokohama-Claimed-User` header — **on requests and on WebSocket upgrades alike**, since a
  harness's context headers reach an upgrade (§0.4); nothing else carries a claimed identity, and no
  `?as=` parameter is built. (4) Anything else has no identity. **The dev door is off whenever the
  public door goes live** and never grants operator rights (R-6.5c.16); the NAS stays on `claimed`.
- **R-6.5c.4 — changes stay in the new host.** Owner: *"we are trying to not make changes to other
  modules to not increase debt and complexity where it's unneeded"*; the NAS/Docker host is
  reconciled with the Worker in a cleanup cycle AFTER M6.5c, and then the project refocuses on the
  computer player. Refined: *"yokohama-web is new within the broader cycle and isn't being used
  anywhere in production, so if we find an architectural issue that we should or need to address, we
  can."* A Cloudflare-specific problem gets a solution inside `crates/yokohama-worker` and `worker/`;
  the layer changes only for a named architectural reason; the CLI and the five fenced crates stay
  untouched. **This cycle's layer changes are TWO, each for a reason no host-side code can supply**:
  `Games::archive_over_retry(id)` — the game-over stamp must be the layer's own code (R-6.5c.9); and
  `check_shape` from `pub(crate)` to `pub` — so the house refuses, loudly, a copy its lobby would drop
  silently on a host where the layer's own diagnostics are mute (R-6.5c.8). Expiry and the operator's
  expire need NO layer change: they are the layer's own `delete` run as the table's `created_by`,
  which is always seated (§0.2) — the owner's own words for it, "the layer's own `delete` as
  `created_by`". (The first two texts added a `Games::expire` on the premise that a creator could hold
  no seat; the re-review showed the layer forbids that state, §9.2.)
  **The corpus emitter** gains two request scripts (§5.3); **the client** gains one transport arm
  (§2.7) and one outbox branch (R-6.5c.14). **A third layer change**, if execution finds a reason for
  one, is the EXECUTING ORCHESTRATOR's ruling, not a return to the owner (the owner, 2026-09-23). It
  lands with its architectural reason in the ledger, and the close lists it for the whole-branch
  review.
- **R-6.5c.5 — Workers Free, and an object invocation gets 30 s of CPU.** Owner: *"historically, for
  DOs on free tier or paid, the start point for CPU time in a DO is 30 seconds. We should build with
  that being the assumption and … we'll find out from the logging if we're wrong."* The documentation
  contradicts itself; **the spike's 376 ms invocation completed `ok`**. The Worker stays inside
  Free's 10 ms by never loading the module; **the JWT VERIFICATION it adds is unmeasured** and the
  first deploy reads it (§3.2).
- **R-6.5c.6 — the transport is signal-plus-fetch over hibernatable WebSockets.** The socket carries
  the SSE stream's own frames in a JSON framing (§2.4); the client fetches exactly as it does now.
  R-6.5b.11's push shapes are DEFERRED, with a trigger: the exit's measured requests AND rows written
  per game (§2.8) binding the shared daily quotas at the owner's usage.
- **R-6.5c.7 — an open picker survives hibernation by re-fed input lines, host-local.** Owner: *"I
  do expect hibernation to kick in almost every turn, most players take several seconds to take a
  turn."* Serializing `SubPrompt` was declined: it has no derives, so it would change the CLI and add
  a stored format coupled to an internal type. The lines are cleared only when the GAME changes
  (§2.4) — a chat post, an unlock or a watcher arriving leaves them.
- **R-6.5c.8 — the house is a second layer instance over scrubbed table copies**, fed by one durable
  slot per table, ordered by `(generation, revision)`, with the alarm armed in the same write; every
  deletion is eventual toward the lobby. Mechanism §2.5.
- **R-6.5c.9 — the archive's asynchronous put is a receipt inside the new crate**, carried out in
  the directory archive's own order, re-dispatched at one pinned clock reading for abandon and the
  browser's save, completed at game over through `Games::archive_over_retry`, with browser saves in
  one archive object per caller and every `.test` address's saves under a `test/` prefix. Mechanism
  §2.6.
- **R-6.5c.10 — static assets exactly as the spike proved** (§0.3 row 7), assembled and VERIFIED
  against the release's own precache list (§2.3).
- **R-6.5c.11 — the outbox's automatic upload is kept** (handover §4 item 6).
- **R-6.5c.12 — four tiers of test** (§5): native Rust for host logic; Cloudflare's vitest pool inside
  workerd for the shell and the battery; the corpus replayed against the objects at every task and
  against the LIVE Worker at the exit; the multiplayer Playwright specs against the LIVE Worker
  through the credential — the live tiers with `.test` identities, cleaned up after themselves.
- **R-6.5c.13 — the spike ran before the spec** (§0.3), R-6.5.8's "test in the live setup" taken one
  step earlier; the first deploy re-reads it with the real code.
- **R-6.5c.14 — the archive has a clean off switch.** Owner: *"if a future user doesn't want to r2
  backup the game saves, the paths don't dead end or fail … we want to gather saves since it helps
  with our development of the computer player but I can see future users and even myself wanting to
  stop."* A RUNTIME variable (chosen over a build flag), `YOKOHAMA_WEB_ARCHIVE = r2 | off`, absent =
  off — the NAS's `YOKOHAMA_WEB_ARCHIVE_DIR` in Worker form. **Amends umbrella R-6.5.4's "the R2
  binding's presence is the switch."** Mechanism §2.6.
- **R-6.5c.15 — a table expires after a period without activity.** Owner: *"without some garbage
  collection this could get out of hand."* `YOKOHAMA_WEB_TABLE_TTL_DAYS`, absent = never. Expiry is
  the creator's own Abandon run on their behalf — saved first when the archive is on — and a returning
  player is TOLD it expired; the save is not re-imported. Mechanism §2.4.
- **R-6.5c.16 — a minimal operator surface, no UI.** Two routes behind Access plus an address
  allow-list, `YOKOHAMA_WEB_ADMINS`, used from a curl playbook; only a PERSON's assertion is an
  operator. The operator's `?force` also clears the fixed ids the live corpus replay uses, which makes
  that replay repeatable (§5.3). Mechanism §2.9.

---

## 2. The mechanism

### 2.1 The pieces, and one request's path

| piece | what it is | answers |
|---|---|---|
| **the Worker** `yokohama-web` | stateless; `worker/src/index.ts` | routing, identity and the body bound for every `/api/*`; `/enter`; a missing file's 404; never loads the module |
| **static assets** | the platform's asset layer | every file of the release, with **no invocation** |
| **`Table`** | SQLite-backed; one per table id, `idFromName(id)` | `POST /api/games` (create) and every `/api/games/{id}/…` route, the transport routes included; expiry; the operator's expire |
| **`House`** | SQLite-backed; ONE, `idFromName("house")` | `GET /api/games?scope=…`, `GET/POST /api/chat`, `GET /api/stream`, `GET /api/health`, `GET /api/me`, `GET /api/labels`, the operator's listing, and any `/api/*` no other row names |
| **`Archive`** | SQLite-backed class that writes nothing; `idFromName("archive:" + address)` | `POST /api/archive` |
| **the bucket** `yokohama-archive` | R2, bound as `ARCHIVE` when `YOKOHAMA_WEB_ARCHIVE=r2` | the archive, in `archive::paths`' layout |
| **the module** `crates/yokohama-worker` | a `cdylib` over `yokohama-web` with `default-features = false` | the layer, inside each object |

**One move, end to end:**

1. The Worker bounds the body (§2.3), validates the assertion and resolves the caller, strips every
   client `x-yokohama-*` header, sets `X-Yokohama-User`, and forwards to `TABLE.idFromName(id)`.
2. The table object creates its module instance if it has none — `createModule` (§2.2), `init`,
   `load` of each stored document, then the picker's re-feed (§2.4).
3. `dispatch` answers with the reply AND what the layer bumped (§2.2).
4. The shell writes each changed document to SQLite — and, in the SAME synchronous batch, any slot,
   owed archive, marker or alarm that the change makes due. **The request has awaited nothing**, so
   no other request has interleaved.
5. The reply returns; the platform's output gate holds it until the batch is durable.
6. Then the side channels: one frame per socket, each for its own viewer (held by the output gate as
   well, §0.4); the house copy pushed if the slot holds one; an owed archive uploaded.

### 2.2 The module: `crates/yokohama-worker`

A new workspace member, `crate-type = ["cdylib"]`, `publish = false`, over
`yokohama-web = { default-features = false }` — `yokohama-wasm`'s shape (R-6.5b.9), because the
browser crate is a HOST and so is this one; the browser module is not touched and does not grow.
Built by `worker/`'s `build:module` exactly as `npm run build:wasm` builds the browser's:
`cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker`,
`wasm-bindgen 0.2.128 --target web`, the FACTORY transform, `wasm-opt -Oz --strip-debug
--strip-producers` at the pinned binaryen **132**.

**The factory transform** (`worker/scripts/factory-glue.mjs`): wasm-bindgen's glue becomes
`export function createModule(compiled)` returning the exports over ITS OWN `WebAssembly.Instance`,
the WHOLE glue body wrapped so every piece of module-scope state is per instance. It refuses loudly
if an `export` survives or an expected export is missing, so a wasm-bindgen upgrade that changes the
glue's shape fails the build instead of shipping a shared instance. **Each object owns one instance;
each instance owns one global `Games`.**

**The crate holds its own `Arc<MemStore>`** — the one it hands `Games::with_parts` — so it can
replace and remove documents through the public `Store` trait (`write_if_version`, `delete`) where
`load` would refuse a present one.

**Its exports** — every one a thin wrapper over a native `_inner` function, so everything that can
be wrong is testable under `cargo test`:

- `init(config_json)`: `Host { identity_name: "cf-access", signal_kind: "ws", serves_games: true,
  multiplayer_offered: true }`; `MemStore`; the chat configuration from the Worker's variables
  (`YOKOHAMA_WEB_LOBBY_CHAT`, `…_CHAT_KEEP`, `…_CHAT_TEXT`, the NAS's names and defaults through
  `ChatConfig::from_env_values`, a bad value refused at `init` naming the variable); the whole-second
  clock `yokohama-wasm` uses, unless a frozen value is configured (test configurations only); the
  `RecordingSignal`; the `ReceiptArchive` when `YOKOHAMA_WEB_ARCHIVE=r2`, else `NoArchive`.
- `load(doc, id, text)` / `read(doc, id)` — the side channel, over `record | table | chat | house`.
- `dispatch(user, method, path, query, content_type, body) -> Reply { status, body, bumps, archive,
  picker }`: `bumps` the layer's signal calls during this dispatch, each tagged — `{"bump": id}`,
  `{"bump_table": id}`, `{"bump_house_chat": null}`, `{"end": id}`; `archive` the `ReceiptArchive`'s
  plan when a put was deferred (§2.6); `picker` the lines to persist (§2.4).
- `events(id, user, query)`: `api::parse_query::<EventsQuery>` then `api::events_at`, serialized —
  the transport routes' payload, byte-identical to the axum host's `events` route and SSE frame. **The
  `wait` cap is NOT the layer's**: axum's `events` handler refuses a `wait` above 25 itself
  (`lib.rs:586-590`, behind `server`, sentence `"a request can be held for at most 25 seconds, not
  {wait}"`), so this export restates that check and its sentence, in the handler's order — the
  identity, the query's parse, the cap, then `events_at`. A missing identity is
  `identity::resolve`'s own `worker`-mode sentence (`identity.rs:157-172`), since the objects read
  `X-Yokohama-User`. A refusal's body is `error_response`'s shape RESTATED here — `{"error": …}`, plus `"status"` when
  `table_status` is set — because that builder is `pub(crate)` and making it public would be a layer
  change for convenience (§0.2); a cell pins the restatement equal to `dispatch`'s body for the same
  refusal.
- `create(user, body, id)` (§2.3's create row) — the body is first parsed as the layer's own
  `NewGame` through the layer's own `api::parse_json` (§0.2), and only that decides:
  - **a body the layer would refuse is dispatched UNCHANGED**, so the layer's own 400 answers it
    byte for byte — a duplicate key, an unknown field, a missing field (whose column the first text's
    insertion moved, §0.4), a non-string `id`;
  - **a body that PARSES but is not a JSON object** is refused **400** `"a game must be created from a
    JSON object"` (§4): serde's derived `Deserialize` accepts a struct as a positional ARRAY — the plan's
    review measured one answering 201 on the layer directly, a pre-existing quirk of the NAS too, left
    for the cleanup cycle — and the Worker's router reads `id` only from an object;
  - **a body whose `id` is a string** is the name the Worker routed by, and is dispatched unchanged —
    once the array arm above holds, an OBJECT body is read the same way by both parsers, so one whose
    `id` is NOT the object's name cannot arrive through the Worker; it is still refused 500 (§4),
    so a routing defect is loud instead of a 201 for an object that holds nothing;
  - **a body with no `id` key** gets `,"id":"<id>"` inserted TEXTUALLY before its final `}` and is
    dispatched (a body that parses always has a member before it — `seats` is required);
  - **a body with `"id": null`** — which the layer reads as absent, so it would draw an id of its own
    after the Worker had already named the object — is refused **400** `"a game id, when given, must
    be a string"` (host-written, §4).

  Nothing is re-serialized (§0.4); the second look, at whether an `id` key is present, only decides.
  Every 201 therefore names the object that holds the table (a cell).
- `house_copy(id) -> json | null`: the table's stored document, scrubbed (§2.5), refused with its
  reason when `check_shape` refuses the copy.
- `put_copy(id, json)` / `remove_copy(id)` (the house's); `restore_picker(id, lines_json)`;
  `receipt(plan_key, outcome_json)`; `stamp_archive(id)` (calls `Games::archive_over_retry`);
  `expire(id)` — host-local: it reads the stored table's `created_by` and dispatches the layer's own
  `DELETE /api/games/{id}` as that user, so the answer, the archive rule and the leaf are exactly the
  creator's Abandon; `set_clock(seconds?)` returning the PRIOR setting so a caller can restore it; and
  `debug_trap` under the `debug` feature only (the battery's trap cells).

**`RecordingSignal`** implements the layer's `Signal` by appending to a per-dispatch buffer that
`dispatch` drains into the reply. It knows no socket: sending is the shell's, after the write.

**The house frame** `{"seq": N, "chat": M}` — the axum host's `House` lives in the server-only half
of `signal.rs` (`:91-96`), so this crate declares its own two-field struct and a cell compares its
serialization with the SSE frame's `format!`.

### 2.3 The Worker

**Routing.** `run_worker_first: ["/api/*", "/enter"]`; every other path is the asset layer's, and
the asset layer hands the Worker only a path it cannot find.

| path | goes to |
|---|---|
| a non-`/api`, non-`/enter` path the asset layer cannot find | **404**, an empty body — never `index.html` (host-written, §4) |
| `GET /enter` | a 302 to `/#/lobby` (the turnstile; served because multiplayer is on) |
| `POST /api/games` | `Table` named by the body's `id` when the body is a JSON object whose `id` is a string, else by a drawn `sim-<16 hex>` from `crypto.getRandomValues`; the ORIGINAL bytes are forwarded to the module's `create`, the id percent-encoded in `X-Yokohama-Table-Id` (a header carries only Latin-1 and a body may name any string; the layer refuses an id outside `is_safe_id`'s set with its own 400) |
| `/api/games/{id}` and below — `…/events`, a `…/stream` upgrade | `Table` named by the segment exactly as the layer binds it — raw, never percent-decoded (§0.2) |
| `GET /api/games`, `/api/chat`, `/api/stream`, `/api/health`, `/api/me`, `/api/labels` | `House` |
| `POST /api/archive` | with an identity, `Archive` named `"archive:" + address`, the address normalized as `User::parse` normalizes it; with none, `House`, whose `dispatch` answers the layer's 401 |
| `GET|HEAD /api/art` | the static manifest's bytes, `env.ASSETS.fetch("/art/manifest.json")` (§4) |
| `/api/admin/*` | §2.9 |
| any other `/api/*` | `House`, whose `dispatch` answers the layer's own 404 or 405 |

**The body bound.** A request body over **2 MiB** is refused at the Worker with `400` and `"the body
could not be read: length limit exceeded"` in the layer's shape — the NAS adapter's rule and, by
intent, its sentence (`lib.rs:734-740`); the first deploy reads the NAS's actual text once and pins
the Worker's to it. The Worker reads the body once, so a forwarded body is the bytes it bounded.

**The transport routes' parity** (§0.2): `GET …/events` parses its query through the layer's own
`parse_query`, so an unknown key is the layer's 400, and a `wait` above 25 is the axum handler's 400
restated in the module (§2.2); it answers AT ONCE —
`wait` is validated and never held. The client DOES send it: the game and deal sources' `once()` is
`api.events(game, undefined, 25)` (`web/src/signal.ts:476`), and §2.7's probe calls `once()`; an
immediate answer is what that probe needs, and a held request would keep an object invocation open
for nothing (§4). A non-`GET` on
`…/events` or `…/stream` is a **405** with an empty body, axum's router's answer. A `GET …/stream`
that is not an upgrade is **426** `"this host streams over WebSocket"` (host-written). An upgrade with
no identity is the layer's 401 (a browser cannot read it; §2.7's probe does).

**Identity** (R-6.5c.3) is one pure function in `worker/src/identity.ts` over (headers, the team's
keys, configuration), tested without a network (§5.2). The keys come from
`https://<ACCESS_TEAM_DOMAIN>/cdn-cgi/access/certs`, cached per isolate; `ACCESS_AUD` is the
application's audience; `DEV_DOOR_CLIENT_ID` is the credential's client id — variables, none a
secret. **The strip rule carried out of `deploy/worker/` (R-6.5.13), widened as R-6.5.11 says**:
every client header beginning `x-yokohama-` is removed before forwarding —
`deploy/worker/src/index.ts:71` stripped `x-yokohama-user` and not `x-yokohama-claimed-user`, which
under `claimed` was impersonation of any seat. The objects take the caller from `X-Yokohama-User` —
the contract the layer's `worker` mode names, which only the Worker can satisfy because the objects
are reachable ONLY through it — and hand it to `dispatch` as the identity, as `yokohama-wasm` hands
`local@browser`; the literal `anonymous`, like an absent header, is no identity.

**Static assets** (R-6.5c.10), assembled by `worker/scripts/assemble-assets.mjs` into the gitignored
`worker/assets/` from `web/dist` and `crates/yokohama-web/assets/art/` (under `/art/`), plus two
committed files:

```
_redirects      /  /index.html  200
_headers        /assets/*   Cache-Control: public, max-age=31536000, immutable
                /sw.js      Cache-Control: no-store
                /art/*      Cache-Control: public, max-age=3600
```

— the axum host's cache policy (`assets.rs:106-108`, `art.rs:125`) restated for the platform. **The
assembly reads the release's own precache list out of `dist/sw.js` and verifies every entry's
SHA-256 against the assembled file**, refusing the build on a missing or different file — so the
assembled tree IS the release the service worker installs, not merely a tree that has a `sw.js`.

### 2.4 The table object

**Storage** — two tables, created on the FIRST WRITE and never by the constructor:

- `docs(kind TEXT PRIMARY KEY, json TEXT NOT NULL, written_at INTEGER NOT NULL)`: `record` (the
  exact text `Session::to_record_json` wrote), `table`, `chat`, and the host's own `picker`.
  `written_at` is the object's clock at the write, so **activity costs no row of its own**: the last
  activity is the newest `written_at` of `record`, `table` and `chat`.
- `meta(k TEXT PRIMARY KEY, v TEXT NOT NULL)`: `generation` and `revision` (§2.5); `slot` (the
  pending house delivery); `acked` (the `(generation, revision)` and digest last acknowledged);
  `archive_owed` (a deferred plan with its clock reading); `marker` (§ below); `held` (an expiry the
  archive refused as a `Conflict`, § below); `load_traps`; and `wakes` — incremented only when
  `YOKOHAMA_WEB_COUNT_WAKES=on` AND the object already holds a table, so a probe of an unknown id
  still writes nothing (the exit's instrument, off otherwise, because on a turn-based table it costs a
  row written on almost every move).

**Probing creates no garbage** (umbrella §2.2): the constructor writes nothing, an id never created
answers the layer's 404, and the object ceases to exist when it goes idle. A cell reads
`sqlite_master` to prove it (§5.2).

**The wake path.** The first request that needs the layer creates the instance, `load`s each `docs`
row, and calls `restore_picker`. Measured for a finished 595-event game: 19–28 ms (§0.3); on a
turn-based table this runs on almost every move (R-6.5c.7's premise).

**The request path, the gates, and the alarm.** A request's read → dispatch → write stretch contains
NO `await`: `sql.exec` is synchronous and so is the module. Every write the request makes, and
**every `setAlarm` its writes make due, go in ONE synchronous batch** — the SQL inside one
`ctx.storage.transactionSync`, the `setAlarm` issued beside it in the same turn before any `await`
(§6 says what that rests on) — so a platform shutdown
between the batch and a side channel's outcome (a deploy, which the lifecycle page says stops an
object "immediately") leaves the alarm to finish the work. The side channels follow the batch: frames
(the output gate holds them until the batch is durable, §0.4); then the house push and the archive
upload, which are non-storage I/O and DO let other requests interleave — which is why every piece of
state they read back is keyed (§2.5, §2.6). **One alarm per object**, set to the earliest of: the
slot's retry, the owed archive's retry, the expiry deadline (unless `held`), the marker's end. Who
sets it:

- **a write that makes a delivery due** — a changed house copy (§2.5), an owed archive — sets it to
  that retry (+30 s) in its own batch; `setAlarm` REPLACES the earlier setting, so a table played
  every few seconds holds one pending alarm, not one per move;
- **the success that discharges it** — an acknowledged slot, a landed upload — re-arms it to the next
  deadline in the success's own batch, so a discharged retry does not wake the object for nothing;
- **nothing else**: a write that makes nothing due — a move that leaves the copy alone, a chat post —
  leaves the alarm where it is. The expiry deadline only moves LATER, so an alarm that fires before
  `last activity + TTL` re-reads the last activity and re-arms itself to it.

Backoff for retries: 30 s, 2 min, 10 min, then hourly, reset by a success.

**Sockets.** A `…/{id}/stream` upgrade is admitted by `events(id, user, "")`:

- on **200** the object `acceptWebSocket`s with the tag `id` and the attachment `{ user }`, and sends
  that frame at once (M6.3's first frame);
- on **403 or 404** it ACCEPTS, then sends the SSE's own `end` frame `{"reason":"gone"}` and closes
  with **4403** or **4404** and the layer's sentence as the reason, truncated at a UTF-8 boundary to 123
  bytes (the platform throws above it, §0.4) — because a browser reports a refused upgrade as an
  anonymous failure it cannot tell from the network;
- on any other status it answers that status unupgraded, as the layer answered it.

**The framing.** Every message the object sends is `{"event": <name>, "data": <the SSE frame's data,
byte-identical>}`, with `<name>` one of the SSE's own event names — `signal`, `house`, `end` — so the
client's arm unwraps it into the `(event, data)` pair its stream reader already produces (§2.7).
On each reply whose bumps name this table — ONE frame per socket per reply, however many bumps it carries — the object computes `events(id, user, "")` for every socket's
attachment: a 200 is sent as `signal`; a **403/404 is `end {"reason":"gone"}` and a 4403/4404 close**
(a stranger holding an OPEN table's stream when the last seat fills, `lib.rs:635-636`); any other
failure sends NOTHING (the SSE's comment). A delete sends `end {"reason":"deleted"}` and closes
**4404**. `setWebSocketAutoResponse(ping → pong)` answers the client's keep-alive without waking the
object. **No other client message reaches `dispatch`**: `webSocketMessage` ignores it — a spectator's
socket can carry no move.

**Traps** (O-1). Every module call is wrapped. A `WebAssembly.RuntimeError` discards the instance;
inside `dispatch` it answers **500** `"the table's engine stopped — try again"` in the API's shape
(host-written) and **a trapped dispatch persisted nothing**, so its move was never acknowledged. A
trap in a SIDE CHANNEL — a socket's `events`, `house_copy`, `stamp_archive` — happens after the
request's batch and reply; the instance is discarded and the work retries on the alarm or the next
request. A trap during LOAD increments `load_traps`; at **2** consecutive, the object stops
re-instantiating and answers every request **500** `"this table's stored <kind> cannot be loaded"`
(host-written) — the poison pill named rather than a loop; the alarm stops re-arming for anything but
a marker's end; recovery is the operator's force expire (§2.9). Any successful load resets the count.

**The picker across hibernation** (R-6.5c.7), all in `crates/yokohama-worker`:

- After a `POST …/{id}/input` dispatch that answered 200 and **left the RECORD's bytes unchanged**,
  the crate reads `games.handle(id)` and `repl.sub` (both public). A picker still open appends
  `(user, line)` to the list; a closed one clears it.
- **The list is cleared ONLY when the game changes — the record's bytes changed (a move, an undo, a
  finalize) or the table is deleted.** A chat post, an unlock, a keep that does not finalize and a
  watcher arriving all leave it: M6.4 holds the stream open during the player's own turn precisely so
  chat arrives while they think (R-6.4.4), and a line from anyone must not cost them their picker.
- The list rides out in `Reply.picker` and is written as the `picker` row in the batch.
- On every new instance, after `load`, `restore_picker` re-feeds each line through `api::input` as
  its user, discarding the replies; if a re-fed line changes the record, fails, or leaves `sub`
  closed, the list is CLEARED and the picker starts empty — today's behaviour after a restart, with
  `Input::expect` turning a stale answer into a 409 rather than a wrong move.
- **Bounded at 64 lines: the 65th CLEARS the list** (nothing partial is ever re-fed).

**Delete, expiry and the MARKER** (R-6.5c.8, R-6.5c.15). Every way a table ends through the layer —
the creator's Abandon, expiry, the operator's plain expire — ends the same way:

1. **The layer deletes, as the creator.** Abandon is `DELETE /api/games/{id}` from the creator;
   expiry and the operator's plain expire are the SAME request dispatched as the stored table's
   `created_by` (the module's host-local `expire`, §2.2) — always seated, so it passes both of
   `delete`'s checks (§0.2) — and the archive rule and leaf are the layer's in all three. Each runs
   under `blockConcurrencyWhile`, through the receipt loop when a save is owed (§2.6). A put that
   fails REFUSES the abandon and the operator's plain expire (R-6.5.5), each answering the layer's
   sentence; for expiry it leaves the table in place to retry on the alarm — **no game with moves is
   lost while the archive is on** — except a `Conflict`, which no
   retry can clear (the record key already holds another game under this id): the object writes
   `held`, stops arming the expiry deadline, and logs it, and the table waits for the operator's
   `?force` (§2.9).
2. **One batch**: the `docs` rows are removed; `marker = {at, reason: "abandoned" | "expired" |
   "operator", ttl_days?, kept: "archive" | "nothing" | "off"}`; the slot takes a REMOVAL; any
   `archive_owed` is cleared (a delete that saved has superseded it); the alarm is set. `kept` is
   `archive` when the table was already stamped or this delete's save landed or was `Identical`;
   `nothing` when there was no game to keep (an open or dealing table, or no move made); `off` when
   there was a game and the archive is off.
3. The reply (204 for an abandon); every socket gets `end {"reason":"deleted"}` and a 4404 close.
4. The alarm delivers the removal to the house (§2.5).
5. **For 90 days the marker answers every request 404** with a host-written sentence of a HEAD and
   a TAIL — the head *"this table was abandoned on 2026-10-22"*, *"this table expired on 2026-10-22
   after 30 days without activity"* or *"this table was removed by an operator on 2026-10-22"* (UTC
   dates); the tail by `kept`: *"— its game is in the archive"*, *"— no move had been made, so there
   was nothing to keep"* or *"— this server keeps no archive, so its game was not kept"*. A 404
   rather than a 410, because the client already ends a source on a 404 and shows the server's
   sentence; the walk script holds no request to a deleted id (§0.2), so no committed payload meets
   these sentences.
6. At +90 days — and only once the house has acknowledged the removal — `deleteAlarm()` and
   `deleteAll()`: the object ceases to exist. A returning player after that reads the layer's own
   "no game named …".

**A create at an id whose object holds a marker or an unacknowledged removal is 409** *"this name
was used recently — try another"* — so a new table's first copy can never overwrite a removal the
house has not yet received. Hosted creates draw random ids, so only a caller that names its own id
meets it (the tests).

**Expiry** (R-6.5c.15) with `YOKOHAMA_WEB_TABLE_TTL_DAYS` set: when the alarm finds `now ≥ last
activity + TTL`, the object runs step 1 as the creator. Finished games expire on the same clock
(their last activity is the final move; their record reached the archive at game over, and "Save game
file" works until expiry); open and dealing tables expire with nothing to save. With the archive OFF
an expired UNFINISHED game is simply gone — a hosted game has no "Save game file" before game over
(R-6.5.10) — and the marker says so; a game-over save still OWED from when the archive was on goes
with it, and the shell logs that (§2.6). A POISONED table does not expire on its own (its load
traps), nor does a `held` one; the operator's force is their path.

### 2.5 The house object

**A second layer instance whose store holds SCRUBBED COPIES of table documents and the house chat.**
`GET /api/games?scope=…` is `dispatch` over those copies — the layer's own `Games::list` and
`TableDoc::view`, the code the NAS runs — so a lobby page is byte-identical by construction.
`/api/chat`, `/api/health`, `/api/me`, `/api/labels` and every unrouted `/api/*` are `dispatch` too.

**The scrub** (`house_copy`, native Rust): `seed` → 0; a `lock` → a placeholder of zeroed hex of the
stored lengths, so `locked` still reads true; **each `keeps[i]` → `Some(1)` or `None`** — presence
only, and `1` because `check_shape` accepts only `None | Some(1 | 2)` and `list` silently drops what
it refuses (§0.4); everything else as stored. **`house_copy` then runs the newly public `check_shape`
on the copy and refuses one it rejects**, with its reason, which the shell logs (§2.8) — so a scrub
defect is loud at the source instead of a row that silently leaves the lobby. Cells: the scrubbed
copies of tables at every status and lock state, loaded into a `MemStore`, produce through
`Games::list` exactly the rows the originals produce, for every viewer kind — a seat holder, the
creator, a stranger, a stranger on the `unlocked` roster; and a VALUE pin that no house copy carries
this run's seed (decimal and hex), password, salt, hash or any keep other than `1`, with the
anti-vacuity partner that the table's own stored document DOES carry the decimal seed, the salt, the
hash and a keep of 2 — the password and the hex seed are absence-only needles, since the stored
document never holds either (the lock is `Lock { salt, hash }`, the seed a decimal number).

**The durable slot** (R-6.5c.8). Each table holds a `generation` — `(created_at, 64 random bits)`,
drawn once at create and ORDERED by that pair — and a `revision` counter.

- **When a change makes the table's scrubbed copy differ from the one it last put in motion** (the
  slot's pending body if the slot is occupied, else the acknowledged copy), the table increments
  `revision` and writes `slot = {generation, revision, copy | removal, last_active, bytes}` and its
  alarm in the request's batch (§2.4).
- **After the reply it pushes the slot over RPC.** On an acknowledgement of `(g, r)` it clears the
  slot **only if the slot still holds `(g, r)`**, and records `acked` **only if `(g, r)` is newer
  than the `acked` it holds** — acknowledgements can arrive out of order, and a regressed `acked`
  would make a later change back to that copy (an undo) look already delivered. A slot rewritten
  meanwhile — a newer copy, or a delete's REMOVAL — stays, and its own push or the alarm delivers it.
  A stale acknowledgement can therefore never erase a newer delivery. The clear, the `acked` and the
  alarm's re-arm (§2.4) are one batch.
- **The house keeps, per id, the CURRENT `(generation, revision)` and the set of RETIRED
  generations** (pruned after 90 days). It applies a push when its generation is current and its
  revision higher; or when the id has no current generation and the pushed one is not retired and
  orders after every retired one. A removal retires its generation and deletes the row. So a late push
  of a removed table is ignored in EITHER order — before or after a re-created table's first push —
  and two interleaved pushes cannot regress a row.
- The umbrella's three deliveries with no next bump — the create, the final move, a removal — are
  simply the slot's contents when the alarm fires. The copy changes when anything in the scrubbed
  document changes, `archived_at` included, so the game-over stamp is a push with no visible change —
  one row, and correct. The copy is the TABLE document, so a move that leaves it alone — most moves;
  the turn passing, a keep, a join and game over change it — makes no delivery at all.
- **`last_active` and `bytes` ride in the slot**, so the house knows them as of the last delivery:
  activity that changes no copy (a chat post, a move within a turn) does not refresh them. The
  operator's listing says so (§2.9); expiry never reads them — it reads the table's own `written_at`.

**The lobby socket** (`GET /api/stream`) is the house's: accepted with the attachment `{ user }`,
sent `{"event":"house","data":{"seq","chat"}}` on every change — `seq` the house's own counter,
raised by every applied copy and removal; `chat` the house chat's `last`, raised by
`bump_house_chat`. Silent on connect, as the SSE lobby stream is (`from_changes`).

**Load** (umbrella §2.4's arithmetic, restated for a projection): at most one push per change of a
table's scrubbed copy; a copy is ~600 B; the house's cold path loads every copy (a thousand tables ≈
600 KB) into its `MemStore`. The copies are bounded by expiry (R-6.5c.15) and the retired sets by
their 90-day pruning.

### 2.6 The archive on R2

**The switch** (R-6.5c.14): `YOKOHAMA_WEB_ARCHIVE = r2 | off`, absent = off.

- **`r2`**: the `ReceiptArchive` over the bucket bound as `ARCHIVE`; `/api/health` reports
  `archive: "r2"`.
- **off**: `NoArchive`, and every path takes the layer's EXISTING off-branch — `/api/health` reports
  `archive: "off"`; game over archives nothing and arms nothing; a hosted Abandon deletes without a
  save and its label reads "Abandon" (the label already follows health); `POST /api/archive` is the
  layer's 404 `"this server keeps no archive"`; the client shows no send control, the outbox never
  enters, and a local Abandon deletes (it always re-reads health first).
- **A bad value, or `r2` with no `ARCHIVE` binding**: every `/api/*` answers **500** naming the
  variable (host-written) — the Worker's form of the NAS refusing to start; static single player is
  unaffected.
- **Switched off after being on**: an owed game-over save is KEPT and not retried while off (the
  alarm does not re-arm for it); switching back on resumes it at the object's next request or alarm
  — at the latest its expiry, whose delete saves first; an expiry WHILE OFF takes the owed save with
  the table, and the shell logs it (§2.4, §6). Archived games are untouched.
- **The client's dead end, closed** (the one outbox change): a 404 from `POST /api/archive` — the
  layer's "this server keeps no archive" — is read as the archive being OFF: the remembered health
  fact becomes `off` and the outbox forgets the row. At HEAD, `reasonFor` (`web/src/outbox.ts:134`)
  files that 404 as "offline" and retries it on every reconnect until the remembered health expires.

**Keys** are `archive::paths(rev, prefix, why, id)` exactly — `rev7/{over,abandoned}/<id>.json` and
`.table.json`; `browser/<caller_segment>/…` for a browser's save — so a sync of the bucket by any S3
client (the exit uses `aws s3 sync`, §7) is a directory the CLI loads (R-6.5.4), and ONE layout function serves the directory and the bucket (O-9;
a cell asserts both write the same relative paths). **A plan whose table's `created_by` (a server
game) or whose CALLER'S ADDRESS (a browser save) ends in `.test` — the reserved top-level domain every
test identity uses — is written under the root `test/` instead of the bucket's top**, so the live
test tiers never write into the owner's training data (§5). The rule reads the full normalized
address — for a browser save the one the Archive object is named by — never the 64-character
`caller_segment`, which truncation can strip of its `.test` (§0.2). **A reply's path is relative to
the root**, as a NAS reply's is relative to `YOKOHAMA_WEB_ARCHIVE_DIR`: the committed
`archive_post.json`'s `browser/red_at_wire.test/rev7/over/wire.json` names the key
`test/browser/red_at_wire.test/rev7/over/wire.json` on this host (§4).

**The receipt** (R-6.5c.9). `ReceiptArchive` implements the layer's `Archive`. Its `put` looks for a
receipt matching the EXACT plan — both keys and the SHA-256 of both bodies; with one it returns the
receipt's outcome; without one it records the plan and returns `PutError::Failed("deferred to the
host")`. **The shell carries a plan out in `DirArchive::put`'s own order** (§0.2):

1. `get` the RECORD key: the same bytes are `Identical`; different bytes `Conflict`; absent,
   continue;
2. put the TABLE key **unconditionally** — overwriting a half pair's table is the repair, and a
   re-plan's new `archived_at` lands instead of conflicting;
3. put the RECORD key create-if-absent; if the precondition fails (a concurrent writer), re-read it:
   the same bytes `Identical`, different `Conflict`;
4. any other failure is `Failed` with R2's message.

The outcome becomes the receipt; the shell LOGS every outcome (the layer's own log lines are mute on
this host, §0.2). The conditional put's exact form (`onlyIf` with an `If-None-Match: *` header, or
the object form) is read off the first deploy that touches R2 (§3.2).

**One clock reading per plan.** The layer stamps `archived_at` from its clock INSIDE the table body
it puts, and the clock advances across an upload's I/O — so a re-dispatch would stamp a different
second, its body's digest would differ, and the receipt would never match. The shell records the
clock reading with the plan and pins the module's clock to it (`set_clock`) for every dispatch that
drives that plan — the re-dispatch and `stamp_archive` alike — and then **restores the PRIOR setting**
(`set_clock` returns it), so a test configuration's frozen clock survives.

- **Abandon** (R-6.5.5) and **the browser's save**: under `blockConcurrencyWhile`, the first
  `dispatch` is side-effect-free and refuses as deferred; the shell uploads; the SAME request is
  dispatched again with the receipt, and the layer answers it — a delete that proceeds, or its own
  refusal naming the archive with R2's reason inside it; a 201, a 200 `identical`, or a 409. A delete
  that owes no save — open, dealing, no events, already stamped — completes in its FIRST dispatch. A
  browser's save costs its strict replay twice (≈ 20–30 ms each for 595 events).
- **Game over** is the one site that cannot be re-dispatched — a move is not idempotent. The move is
  answered 200 as today; the layer's `archive_if_over` meets a deferred put, and `archived_at` stays
  unset — R-6.5.4's "a retry is owed". The shell writes `archive_owed` (the plan and its clock
  reading) and the alarm in the move's batch, uploads after the reply, and on success installs the
  receipt and calls `stamp_archive(id)` — **`Games::archive_over_retry(id)`**, the new public function
  that runs the existing `archive_if_over` against the stored record at the pinned reading, so the
  stamp, and its retry on a version conflict, is the layer's own code; the stamped table row and the
  cleared `archive_owed` are one batch. On failure the alarm retries, re-planning at the RECORDED
  reading after an eviction. If the game is no longer over when a retry runs (an undo at a one-person
  table), `archive_if_over` does nothing and `archive_owed` clears; a later completion puts afresh
  and — the record key holding different bytes — is a `Conflict` the shell logs: the archive keeps the
  first completion, as the directory archive does. **A `Conflict` is final at every site**: game over
  logs it and clears `archive_owed`; an abandon is refused with it inside the layer's sentence; an
  expiry is `held` (§2.4).
- **One archive object per caller** serializes that caller's sends and puts the strict replay of an
  untrusted record (handover §4 item 9) inside an object's 30 s, behind Access and the 2 MiB bound; a
  hostile record can at worst trap ONE instance of ONE caller's archive object, whose storage is never
  written.

### 2.7 The client

**One transport arm.** `signalKind()` (`web/src/api.ts:309`) widens to `"sse" | "held" | "ws"`, set
from `/api/health`. `usePolled`'s `open()` (`web/src/signal.ts:294`) gains a third arm:

- a `WebSocket` on the source's existing `stream` URL (`wss:` for `https:`);
- each message parsed as `{event, data}` and handed to the SAME handler the stream reader feeds; a
  message that is not that JSON — the auto-response's `pong` — is dropped;
- **an `end` frame ends the source** exactly as the stream's `end` does, and **the close that follows
  it is ignored, whatever its code** — this host always sends `end` before its 4403/4404 close (§2.4),
  so reading the close as a second refusal would end the source twice; the close's reason carries the
  layer's sentence for a reader of the logs;
- **a close the arm did not initiate, with no `end` before it, or an open that fails, is followed by
  ONE probe: the source's own `once()`** — never a close the arm made itself (a hidden tab's, R-P8, or
  a teardown's), exactly as the stream reader returns silently on its own abort (`signal.ts:378`) —
  a plain fetch through `api.ts`'s `call` and `serverHeaders()`. An expired Access session is then
  classified exactly as the SSE arm classifies it (`isSessionExpired`, `web/src/api.ts:245-267`) and
  ends the source with the `/enter` remedy (`signal.ts:390-396`); an answer is delivered as a frame (a
  free resync) and the arm climbs the existing ladder (`BACKOFF_MS`, `nextDelay`, `Link`,
  `signal.ts:47-87`). Without the probe, a browser that hides every refused upgrade (the WHATWG spec:
  a failed handshake closes 1006 and "user agents must not convey any failure information to
  scripts") would read "reconnecting…" for ever — M6.5b2's final-review defect I-2, reintroduced;
- every **30 s** the arm sends `ping`, answered at the edge without waking the object, which keeps
  the connection under the edge's 100 s idle cut (measured in M6.3 at 125 s).

**One outbox branch** (R-6.5c.14, §2.6). The door, single player, the offline release and the service
worker are otherwise unchanged; the Worker serves the same `dist` (§0.3 row 7). The NAS keeps `"sse"`
and the browser host `"none"`.

### 2.8 The budget, and what single player costs

**Single player** (the owner's question): browsing the door and playing a local game to the end
invoke the Worker **zero** times — every file is a static asset (measured, §0.3 row 7) and a local
game makes no server call (R-6.5b2.7). The exceptions are all at the END of a game: the health read
at game over (skipped when the remembered answer is under an hour old, M6.5b2 §2.2) and the outbox's
upload (R-6.5c.11) — **two invocations per finished local game when the upload lands**; an upload
that fails is retried on every `online` event and every launch until it lands or is forgotten
(`web/src/outbox.ts:417`, `:522`), one invocation each. **"Abandon and save" is two** — the forced
health read (`Abandon.tsx:163`) and the send (`:110-126`); a plain local Abandon, one. The first
visit's login is Access, not the Worker. A request for a file
that does not exist invokes the Worker once (§2.3); for the public door, a scanner's misses are
Worker invocations, and that is where the path-scoped door's rate question begins (§6).

**Multiplayer, estimated** (MEASURED at the exit): a two-person game of ~400–600 events is ~900
input POSTs plus ~2 GETs per bump for each other screen — **~2,000–4,000 requests**, each one Worker
invocation and one object request — **~17–41 games a day** in this week's Worker headroom (67k–83k).
**Rows written** (Free: 100,000/day, shared). A move that leaves the house copy alone — most of a
turn's moves — writes **~1–2 rows**: the record, and an open picker's row cleared. A move that changes
the table document — the turn passing, a keep, a join, game over — also delivers a copy, and writes
**~9–12**: the record, the table, `revision`, the slot, its alarm (billed as a row), and on the
acknowledgement `acked`, the slot's clear and the alarm's re-arm, plus the house's copy and its
`(generation, revision)`. A picker click that applies nothing writes the `picker` row, and a chat post
its document. With ~900 input POSTs and ~75–100 turn changes a game, that is **~2,000–3,000 rows a
game**, **~33–50 games a day** — the same order as requests, so the exit counts both (§5.5) and the
estimate is only a reason to measure. Object requests (100,000) are not close: the account uses 1–2%.
R2: saves are ~55 KB; 10 GB and 1 M writes a month. R-6.5c.6's push is the lever if a measured count
binds.

**Logging.** With observability on, the shell logs to Workers Logs: every receipt outcome, stamp,
slot push and acknowledgement, trap, poison, expiry, marker end, operator action, and every copy
`house_copy` refuses. The layer's own `eprintln!` lines are mute on this host (§0.2), and §6 says so.

### 2.9 The operator surface (R-6.5c.16)

- **The gate**: `/api/admin/*` requires an assertion naming a PERSON whose address is in
  `YOKOHAMA_WEB_ADMINS` (comma-separated). An identity reached through the dev door NEVER counts, even
  when its claimed address is on the list. With the variable absent, the routes are unrouted `/api/*`
  and answer the layer's ordinary 404 — a clean off-path. A signed-in non-operator is **403**
  `"this address is not an operator here"` (host-written). `/api/admin` joins R-6.5b2.2's protected
  prefixes for the public door.
- **`GET /api/admin/tables`** — the house answers from its store: every table it holds a copy of,
  dealing ones included, each with id, name, status, players, `created_at`, `last_active`, `bytes`
  (the table object's SQLite size, carried in the slot), over and winner, `archived_at`. An
  operator-only payload outside the corpus. `last_active` and `bytes` are as of the table's last
  delivery (§2.5), so a table talked in but not moved in reads older than it is; the tables the alarm
  could not end — poisoned or `held` — are named in the log.
- **`POST /api/admin/tables/{id}/expire`** — the table object expires NOW by §2.4's path (reason
  `operator`, the save first when owed, the marker after).
- **`?force` ends the object without the layer and without a save, whatever it holds** — a poisoned
  table, a `held` one, any table, or a marker: the lobby removal through the slot (unless the house
  already acknowledged one), then `deleteAlarm()` and `deleteAll()` once the house acknowledges, with
  a log line. **It leaves no marker.** It answers **204** once the storage is gone — at once, writing
  nothing, when the object holds nothing — and **503** `"the lobby has not confirmed the removal — the
  alarm will finish it"` (host-written) when the push fails; until then a create at the id is the
  marker's 409 (§2.4). This is also what makes the live corpus replay repeatable: its fixed ids are
  forced before and after each run (§5.3).
- **No UI**: RUNBOOK §6 carries the curl playbook (WARP authenticates the owner's shell as the owner —
  a request that does NOT present the credential, since Service Auth outranks WARP, §0.3 row 1).

---

## 3. Deployment

### 3.1 The infrastructure

- **One Worker, `yokohama-web`**, the custom domain `yokohama.moore.nyc` (it creates the DNS record
  the name now lacks), `workers_dev: false`, `preview_urls: false`, `observability.enabled`, a pinned
  `compatibility_date` (Node compatibility follows the date's default; §0.3 row 3 shows it is no
  obstacle).
- **Three SQLite-backed classes** in one migration: `Table`, `House`, `Archive`.
- **The bucket `yokohama-archive`**, CREATED 2026-09-23 through the account API (before wrangler's
  login was found to reach R2, §0.5) — location ENAM, storage class Standard, PRIVATE (its `r2.dev` URL disabled, no
  custom domain) — and bound as `ARCHIVE`.
- **Variables**: `ACCESS_TEAM_DOMAIN`, `ACCESS_AUD`, `DEV_DOOR_CLIENT_ID`, `YOKOHAMA_DEV_DOOR`
  (`on` during this cycle), `YOKOHAMA_WEB_ARCHIVE` (`r2`), `YOKOHAMA_WEB_TABLE_TTL_DAYS` (**`90`**, the
  owner's value, 2026-09-23), `YOKOHAMA_WEB_ADMINS` (the owner's address — set as a Worker SECRET so the
  address stays out of the repository, which the owner approved on 2026-09-23; its value is not a
  secret and no code differs), `YOKOHAMA_WEB_COUNT_WAKES` (off except in the exit's window), and the
  three chat variables at the NAS's defaults.
- **The configuration lives in `worker/wrangler.jsonc`**; the `yokohama` Access application keeps its
  destinations and policies unchanged.

### 3.2 The first deploy and after (R-6.5.8)

**The first deploy is the skeleton, and it re-reads the spike with the real code before anything else
lands on it**: the Worker, the three classes, the static assets, identity, the body bound — then (1)
the unenrolled protection reading (302 / 403 / `workers.dev` answering nothing); (2) identity on an
upgrade as a person over WARP and as the credential with a claimed header; (3) the static-asset
readings of §0.3 row 7, including the service worker's complete install and a missing file's 404; (4)
cold start, and **the Worker's CPU with JWT verification**, off the tail; (5) the NAS's body-limit
sentence, read once and pinned. The first deploy that touches R2 reads the conditional put's form.
Every later task deploys when it changes what the Worker does, and records what it read.

### 3.3 The runbook, and the retired code

`deploy/RUNBOOK.md` gains **§6, the Worker**: the build (`worker/`'s `build:module` and
`build:assets`), the deploy, the bucket, every variable and what its absence means, the credential's
playbook (where it lives, how the harness presents it, how to rotate it), the operator's curl
playbook (§2.9) including the live replay's force of its three ids (§5.3), reading CPU and the logs off
the tail, and each deploy's record with the
container-era numbering continued; plus **"to deploy with no archive"** (R-6.5c.14) and **"to keep
every table"** (R-6.5c.15, the TTL unset). **§4 is rewritten as the record of why `deploy/worker/` and
`deploy/modal_app.py` were retired** and both are deleted (R-6.5.13), the strip rule carried out first
(§2.3). §0–§3 and §5 stay: the NAS stays.

### 3.4 The NAS

Unchanged: `yokohama-nas.moore.nyc`, `claimed`, SSE, the directory archive, no expiry (R-6.5.1). The
cleanup cycle after M6.5c (R-6.5c.4) reconciles the two hosts, and may give the NAS a TTL.

---

## 4. The wire and the corpus

**R-6.5.9: every payload a host SERVES is byte-identical to the committed one**, except what its spec
names. For this host:

- **The second class — host-parameterized fields**: `health.json`'s `identity` (`"cf-access"`),
  `signal` (`"ws"`) and `archive` (`"r2"`); `me.json` and `me_signed_out.json`'s `mode`
  (`"cf-access"`). The replay asserts each named field really DIFFERS on this host (the M6.5b
  battery's lesson); `multiplayer` does not differ and is not named.
- **`GET /api/art`** is served as the static manifest's bytes; the axum route re-serializes the same
  manifest; the replay compares it by VALUE. No client has read the route since M6.5b2.
- **The three transport payloads** (`events.json`, `events_open.json`, `events_dealing.json`) and the
  **eight `archive_*` payloads** are served by this host and must be byte-identical — reached by the
  two request scripts the corpus emitter now writes (§5.3).
- **`?wait` on `…/events`** is validated and answered at once, not held (§2.3).
- **The socket's framing** `{"event","data"}` carries the SSE frames' data byte-identically; the
  close codes 4403/4404 carry the layer's sentences.
- **An archive reply's `path` is relative to the archive's root** (§2.6): on this host a `.test`
  caller's root is `test/`, so `archive_post.json` is byte-identical here and names the key under
  `test/` — as a NAS reply names a file under `YOKOHAMA_WEB_ARCHIVE_DIR` without spelling the
  directory.
- **Host-written statuses and sentences — named, never the layer's**: the missing-file 404 (§2.3);
  the 2 MiB body refusal (intended identical to the NAS's, pinned at the first deploy); the stream's
  426; the create body's `"id": null` 400, its non-object 400 `"a game must be created from a JSON
  object"`, and its routing-defect 500 `"the create's id does not name
  the table it was routed to"` (§2.2; unreachable through the Worker, loud if it ever is not); the
  trap 500 and the poison 500 (§2.4); a marker's
  404 sentences — three heads and three tails — and its 409 (§2.4); the archive misconfiguration 500
  (§2.6); the operator's 403, the force's 503 and the operator payload (§2.9). Everything else a
  client reads is the layer's.

---

## 5. The tests

### 5.1 Rust — `cargo test -p yokohama-worker` (per-edit, native), and the layer's two

- **`RecordingSignal`**: every layer bump site's call arrives in the reply, tagged and in order — the
  eight sites (`games.rs:668`; `tables.rs:776, 958, 972, 1262, 1409, 1511, 1742`) — including a cold
  handle's bot pump before a later refusal (`lib.rs:520-538`), which is recorded because it happened.
- **`ReceiptArchive`**: no receipt → the plan and `Failed("deferred…")`; a receipt for the exact plan
  → its outcome; a receipt whose digests differ → absent; the `test/` root for a `.test` creator and a
  `.test` caller — **including a `.test` address longer than the 64-character segment**, whose
  truncated segment has lost its `.test` — and the bucket's top otherwise.
- **The picker**: append on an open picker with the record unchanged; clear when the record changes;
  a CHAT POST between two picker clicks leaves the list; the 65th line clears; `restore_picker`
  rebuilding a `PlaceA` picker mid-selection so the next line commits the move the player intended
  (umbrella §4's H-4 acceptance, natively); a re-fed line that would change the record clears instead.
- **The scrub**: through `Games::list` over a `MemStore` of copies, the same rows for every viewer
  kind at every status and lock state — **a table with a kept human seat included** — plus the value
  pin (keeps among its needles) and its anti-vacuity partner; `house_copy` refusing a copy
  `check_shape` rejects.
- **`create`**: an absent id inserted textually and **every 201 naming the object it was sent to**;
  `"id": null` refused with the host's 400 (it answered 201 for an object that held nothing, §0.4);
  each refused body forwarded UNCHANGED — its answer the very bytes `dispatch` gives the original body
  (the layer's 400s are axum's word for word since M6.5a) — for a duplicate key, an unknown field, a
  missing field (at the original column), a non-string `id`, a non-object and an unparseable body; a
  `u64` seed exact.
- **`events`**: an unknown key is the layer's 400 and a `wait` above 25 the axum handler's, restated
  (§2.2) — each byte-identical to what the axum host answers — and in the handler's ORDER, pinned by
  adjacent-pair probes carrying two defects each (no identity AND an unknown key; an unknown key AND
  `wait=26`; `wait=26` AND an unknown id); a refusal's body equals `dispatch`'s body for the same
  refusal (the restated `error_response`, §2.2).
- **The chat variables** (defaults; a bad value refused naming it) and **the crate's platform scan**
  (`yokohama-wasm`'s `this_host_makes_no_platform_call_but_the_clock` shape).
- **In the layer**: `archive_over_retry` stamps a finished game whose first put failed, does nothing
  for a stamped or unfinished one, and retries its stamp on a version conflict; `check_shape` is
  reachable as `pub` (a compile-time fact, pinned by the crate's use of it).
- **`expire`** (host-local): for a table at every status, the reply, the archive plan and the leaf are
  exactly those of the creator's own `DELETE` — the same request, as `created_by`.

### 5.2 The shell — Cloudflare's vitest pool (per-edit, `worker/`)

Cloudflare's Workers Vitest integration — `@cloudflare/vitest-pool-workers` (0.22.0 on 2026-09-22;
≥ 0.16.20 for `evictDurableObject`) or `@cloudflare/vitest-plugin` (1.2.3), the name the test-API page
now documents; both carry the four helpers these cells use (§0.4), and the plan pins one — inside
workerd with real SQLite, alarms (`runDurableObjectAlarm`), WebSockets and a local R2, over the BUILT
module. **The archive cells run with a clock that TICKS per reading** (the layer's `ticking_clock`
shape), so an unpinned re-dispatch really does stamp a different `archived_at`. **Time-driven cells
move the STORED times, never the clock** — workerd's `Date.now()` cannot be advanced — by backdating
`written_at`, the marker's `at` or a retry's due time through `runInDurableObject`, then running
`runDurableObjectAlarm`. Cells:

- **Routing and the Worker**: every row of §2.3's table — the missing file's empty 404 and never
  `index.html`; `/enter`; the body bound at 2 MiB + 1; the transport routes' 405s and 426.
- **Identity** (a pure function, crafted assertions, a test key set): a person; a person PLUS a
  claimed header (ignored); the credential with the door on and off; another audience; expired; a
  forged signature; **every client `x-yokohama-*` header stripped**, `x-yokohama-claimed-user`
  included.
- **The factory**: two instances share nothing; a trapped instance leaves another usable; a
  re-instantiated one reloads from SQLite.
- **The table object**: probing an unknown id leaves `sqlite_master` EMPTY; a spectator's move is
  refused (the "before the record is read" half rests on the native M6.3 sweep); a socket message that
  is a move reaches nothing; the picker survives `evictDurableObject` with a chat post between the
  clicks; a trapped dispatch persists nothing; two load traps yield the named poison 500.
- **The slot**: a lower revision after a higher one leaves the row at the higher; **a stale
  acknowledgement of `r` after the slot was rewritten to `r+1` or a REMOVAL leaves the slot holding
  it**; an acknowledgement of `r` arriving after `r+1`'s leaves `acked` at `r+1`, and a later change
  back to `r`'s copy is still delivered; a late push of a removed generation is ignored BOTH before
  and after a re-created table's first push; a failed push is retried by the alarm; **the alarm is set
  in the batch**: the house's push handler held pending by a gate on `House.prototype.apply` — the
  PROTOTYPE, since the pool refuses an instance's own property and a patched instance makes the RPC
  throw, which the mutant survives (§0.4, measured true/true) — then `abortAllDurableObjects()`, which
  unlike `evictDurableObject` does not drain in-flight work, so the push never fails and never
  succeeds; the gate restored at once, fresh stubs taken, and `runDurableObjectAlarm` on the table
  delivers the slot (under the mutant it returns `false` and the lobby never lists the table — measured
  true/false); a move that changes no copy leaves the alarm where it was.
- **Sockets**: the first frame on accept; one frame per socket per REPLY — a finalizing keep's two
  bumps are one frame — each for its own viewer; the
  framing; `end {"reason":"gone"}` with 4403/4404 at admission and mid-stream; `end` on delete;
  nothing sent on another failure.
- **The archive**: abandon with a failing put leaves the object and its storage in place with the
  layer's refusal; abandon with a landing put deletes and leaves the marker; **a HALF PAIR (table
  landed, record failed) then a second abandon LANDS**; **a landed-but-unstamped game-over copy
  retried at a NEW reading answers `Identical` and stamps**; a browser save answers the layer's
  201/200/409; the re-dispatch's `archived_at` equals the first's; a chat post arriving during a slow
  house push or R2 put lands.
- **Game over**: `archive_owed` → upload after the reply → receipt → the stamped row and a cleared
  `archive_owed`; the alarm's retry after a failing put; a retry after `evictDurableObject` at the
  recorded reading; the "no longer over" clear.
- **The switch**: `off` — health off, game over puts nothing and arms nothing, abandon deletes without
  a put, the archive route 404s; `r2` without a binding and a bad value refuse naming the variable; an
  owed save is not retried while off and resumes when on; an expiry while off takes the owed save and
  logs it.
- **Expiry and the marker**: a write does not expire an active table (an alarm that fires early
  re-reads the last activity and re-arms to the new deadline); an idle table expires at the TTL, saved
  first, as its creator; a failed save at expiry retries and the table stays; **a `Conflict` at
  expiry writes `held` and stops arming the expiry deadline**; archive off expires without a save;
  **the marker's tail by `kept`** — a saved game, a game stamped at game over (whose delete puts
  nothing), an open table, a dealing one, a playing one with no move, archive off — under each of the
  three heads; a create under the marker, AND a create while a removal is unacknowledged, 409;
  `deleteAll()` only after the house's acknowledgement and the 90 days; no TTL, no expiry.
- **The operator**: a non-operator 403; a credential-claimed operator address refused; the variable
  absent → 404; the listing includes a dealing table; expire; **force** on a poisoned table, a `held`
  one, a live table and a marker — each leaving NO marker and an empty `sqlite_master`, a create at the
  id then succeeding — on an object holding nothing (204, nothing written), and with the house's push
  failing (503, a create meanwhile 409, the alarm finishing it).
- **The client's two changes** (in `web/`'s own `npm test`, over jsdom, not the pool): the socket arm
  — an `end` frame ends the source ONCE and its following close is ignored; a close with no `end`
  before it, or a failed open, sends ONE probe, and an Access 401 on it ends the source with the
  `/enter` remedy; a close the arm made itself (the tab hidden, the component unmounted) sends none;
  `pong` dropped; the 30 s `ping` — and the outbox, whose archive-route 404 turns the remembered
  archive fact `off` and forgets the row.

### 5.3 The corpus against the objects (per-task in the pool; ONCE live at the exit)

**The corpus emitter** (`crates/yokohama-web/tests/wire_fixtures.rs`, test code) writes two more
request scripts beside `walk.json`, and changes no committed payload: `walk-host.json` — the
host-owned requests: `…/events` at the walk positions where it captured `events.json`,
`events_open.json` and `events_dealing.json`, each positioned by the walk row it follows, and ONE
`GET /api/art` row the emitter SYNTHESIZES at the walk's start, because `art_manifest.json` is built
from the art directory and was never requested (§0.2); and `archive-walk.json` — the second `Api`'s
requests that produce the eight `archive_*` payloads. The node replay keeps them in `NOT_SERVED`,
unchanged; this host's replay serves them, and derives its own third column from the file set as the
node replay does.

- **In the pool**, against the objects with `X-Yokohama-User` set as the Worker would set it, and
  the module clock frozen at the corpus's `1_789_000_000` — the node replay's arrangement (M6.5b spec
  §6.3), one host further; statuses asserted, every named row deep-equal to its committed file, §4's
  second class excepted and each exemption proven to exempt something. Each test file gets an empty
  bucket, which `archive-walk.json`'s first send assumes (§0.2).
- **Live, at the exit**, through the Worker with the credential and the dev door, as the walk's own
  `.test` identities — **repeatable, and safe to re-run after a run that failed half way**:
  - **Before the run and after it, the operator's `?force` on `wire`, `wire2` and `wire4`** (§2.9),
    sent from the owner's shell WITHOUT the credential, so WARP makes it a person's assertion — the
    credential is never an operator (R-6.5c.16). Force leaves no marker, so nothing a run leaves
    behind blocks the next one's creates; the harness refuses to start unless all three answer 204,
    which also catches an unset `YOKOHAMA_WEB_ADMINS` (404) and a shell that is not the owner's (403).
  - **The lobby payloads**: `games` compared after filtering to the replay's own ids, and `next`
    compared only where the committed cursor names one of them (`games_next.json`) — the owner's own
    tables and the e2e's share this lobby, 20 to a page (§0.2). The replay's tables are the newest on
    it, so they lead every page, and no walk row sends a cursor for the replay to rewrite (§0.2).
  - **`created_at` and cursors** normalized as `freeze_clock` normalizes them — the clock cannot be
    frozen live.
  - **The house chat** is shared by every run and every real user, so its two payloads
    (`house_chat_post.json`, `house_chat.json`) are compared after taking `seq` and `last` back by the
    house's `last` read at the run's start, with the page filtered to this run's own messages — a
    named host-parameterized filter, like the lobby's. A person talking in the lobby DURING a run
    moves `last` and fails that run; the next run is clean.
  - **`archive-walk.json`'s two FIRST sends** — the finished record (`archive_post.json`, 201,
    `"identical": false`) and the partial one (`archive_post_abandoned.json`, 201, `leaf:
    "abandoned"`) — land only the first time the bucket sees them; on every later run the once-rule
    answers each with **200 and the same body but `"identical": true`** (for the first, that is
    `archive_post_identical.json` exactly; the second has no committed file for it). The replay
    accepts, for those two rows only, either the committed 201 or that 200 — every other field
    compared — and every other row compares as in the pool (`archive_post_identical.json` and
    `archive_409.json` are the same on every run). The game-over put of `wire` is `Identical` on a
    re-run, because a record carries no clock and the walk seats no bot (§0.2).
  - Every save lands under the `test/` root (§2.6).

### 5.4 Playwright — the multiplayer specs against the LIVE Worker

`two_browsers.spec.ts` and `joins_by_link.spec.ts` gain a configuration that targets
`https://yokohama.moore.nyc` (`YOKOHAMA_WEB_E2E_TARGET=live`): each context presents the credential
and its own `X-Yokohama-Claimed-User` at `@e2e.test` as context headers — which reach fetches and
page-opened upgrades alike (§0.4). Each spec deletes the tables it created at its end (saves under
`test/`). Every other spec stays on the local axum binary, unchanged; the two keep their local
configuration too.

### 5.5 The exit (umbrella §3.3 — numbers, not a demonstration)

On the live Worker, **in a window with no deploy** (a deploy stops every object and terminates every
socket, which would read as hibernation): two people play a whole game and a third watches, HEADED;
the table object **observed hibernating** between moves — `wakes` rising across pauses with
`YOKOHAMA_WEB_COUNT_WAKES=on` for the window; the move-to-other-screen wall measured as M6.3 measured
it, with the cold-path CPU beside it from the tail; a finished game and an abandoned one in the
bucket, synced by an S3 client (§7) and **loaded in the CLI with `why` reconstructing a bot move**; the whole
corpus byte-identical live (§5.3); **requests AND rows written per game** counted off the analytics
(§2.8).

### 5.6 Where each tier runs

Per-edit locally: §5.1, §5.2 and §5.3's pool replay, beside `CLAUDE.md`'s existing web gates. **CI
runs §5.1 and §5.2** — the `web-server` job builds the worker module as it builds the browser's and
runs both — and the lint gate covers the new crate as a workspace member. **§5.3-live, §5.4 and §5.5
stay local**, because they need the credential; written into the job, as the e2e's locality is. Any
edit to a workflow file gets the `yaml.safe_load` parse line in the gate (handover §4 item 12).

### 5.7 The mutation battery (at the reviewed head, each reverted after its run)

Each must be CAUGHT by a named cell, or be recorded with its disposition:

1. a spectator's move reaches the record (§5.2);
2. a client `x-yokohama-claimed-user` honoured beside a person's assertion (§5.2);
3. the house copy keeps the hash, the salt, the seed or a keep other than `1` (§5.1);
4. **a frame sent before the batch — RECORDED, uncatchable by the platform's guarantee**: the output
   gate holds every outgoing message until the batch is durable (§0.4), so the mutant is equivalent
   (the M6.5b battery's "uncatchable by ruling" precedent);
5. the house accepts a lower revision (§5.2);
6. the slot cleared before the acknowledgement (§5.2);
7. the corpus drifts on this host — the walk, `walk-host.json` or `archive-walk.json` (§5.3);
8. the wake counter stops counting (§5.2, over `evictDurableObject`);
9. the picker not re-fed after a fresh instance (§5.1, §5.2);
10. a failed abandon put deletes anyway (§5.2);
11. `deleteAll()` before the house acknowledges (§5.2);
12. the factory transform bypassed — the stock glue, one instance per isolate (§5.2);
13. `html_handling` back to its default, so `/index.html` redirects (§5.2, and the live install
    reading);
14. `not_found_handling: "single-page-application"`, so a missing file answers HTML (§5.2);
15. the dev door honoured with no credential assertion (§5.2);
16. probing writes storage (§5.2);
17. the receipt loop's clock left unpinned (§5.2's ticking-clock archive cells);
18. the house applying a removed generation's push, in either order (§5.2);
19. the scrub back to `Some(0)` — the kept table leaves the lobby (§5.1);
20. the R2 order reversed — the table create-if-absent (§5.2's half-pair cell);
21. a chat post clearing the picker (§5.1, §5.2);
22. an acknowledgement clearing a newer slot (§5.2);
23. the socket arm laddering without the probe — "reconnecting…" for ever (a client cell: a refused
    upgrade, then an Access 401 on the probe, ends the source with the remedy);
24. a mid-stream admission loss sending nothing (§5.2);
25. the alarm armed only after a failure (§5.2's `abortAllDurableObjects` cell — the first text's
    `evictDurableObject` cell could not see it, because graceful eviction drains the push);
26. the create body round-tripped through a `Value` (§5.1's duplicate-key cell);
27. an owed save retried while the archive is off (§5.2);
28. the outbox filing the archive route's 404 as offline (a client cell);
29. an expiry that deletes before its save lands (§5.2);
30. an ACTIVE table expires — `written_at` not advanced by a write, or an alarm that expires without
    re-reading the last activity (§5.2);
31. the marker dropped — a returning player reads the bare "no game named" (§5.2);
32. a dev-door identity granted operator rights (§5.2);
33. `.test` saves written to the owner's paths — the rule read off the truncated segment, so a long
    `.test` address loses it (§5.1);
34. `"id": null` forwarded to the layer — a 201 naming an id no object holds (§5.1);
35. `acked` recorded without the advance check — a late acknowledgement regresses it, and an undo
    back to that copy is never delivered (§5.2);
36. a create allowed while a removal is unacknowledged — the new table's first copy overwrites the
    removal and the house keeps the old generation (§5.2's force-with-a-failing-push cell);
37. the socket arm probing after a close it made itself — a hidden tab fetches (a client cell);
38. the close after an `end` frame read as a second refusal — the source ends twice (a client cell);
39. a `Conflict` at expiry retried for ever — `held` not written (§5.2).

---

## 6. What this spec does NOT claim, and the LEAVE list

- **Nothing is public** (R-6.5c.1). The zero-invocation single-player door is a PROPERTY of the asset
  configuration, read off the tail by the spike, not a public service; the path-scoped Access
  application of R-6.5b2.2 is still built-and-proven-locally, not deployed; the art is not published;
  and on a public door a missing file is a Worker invocation, so a scanner's misses are the rate
  question that door will have to answer.
- **`claimed` is not retired.** It is the NAS's mode, and on the Worker it is the development door
  behind the credential. A holder of the credential can claim any identity at any table, including
  the owner's — the trust the NAS gives every allow-listed person today, narrowed to one secret — but
  never an operator's rights.
- **The CPU assumption rests on one measurement** (376 ms, `ok`) and the owner's experience; the
  documentation contradicts itself, and `exceededCpu` in the logs reopens it. The Worker's own CPU
  with JWT verification is unmeasured until the first deploy. **An anomaly in execution is NOT
  presumed to be this limit** (the owner, 2026-09-23): a dedicated research subagent diagnoses it,
  and only a fix that needs a substantial departure from this spec goes back to the owner.
- **The frame-after-write property rests on the output gate's documented sentence** ("any new
  outgoing network messages … held back until the write has completed"); the rules page's example
  names responses and fetches, not WebSocket sends by name, and no cell can observe it.
- **The alarm's atomicity with its batch rests on the platform's write coalescing.** The SQL writes
  go in one `transactionSync` and the `setAlarm` beside it in the same synchronous turn, with no
  `await` between; the storage page says writes issued without an intervening `await` "will
  automatically be submitted atomically" and that `setAlarm` takes `put`'s options, but it does not
  name a `transactionSync` and an alarm together. The pool's `abortAllDurableObjects` cell reads
  workerd's local behaviour; no cell can read a production crash.
- **Hibernation's wake cost grows with the record.** Measured at 595 events; the e2e's own games reach
  1,700 applied moves, an estimate of ~3× until the exit reads it.
- **The untrusted record on `POST /api/archive`** (handover §4 item 9) is BOUNDED — one instance of
  one caller's archive object, behind Access and the 2 MiB bound — not closed.
- **A poisoned table is named and forced out by an operator, not repaired**; so is a `held` one —
  an id whose archive key already holds another game. Hosted creates draw 64-bit random ids, so only a
  caller that names its own id can reach `held`.
- **The layer's own diagnostics are mute on this host**: `eprintln!` is a silent sink on
  `wasm32-unknown-unknown`. The shell logs what it can observe (§2.8); a logging seam in the layer is
  a candidate for the cleanup cycle, not this one.
- **An expired unfinished game is lost when the archive is off**, and so is a game-over save still
  owed from before the switch went off. The marker says the first and the log the second; the
  operator chose both switches.
- **The operator listing lags** activity that changes no copy (chat, a move within a turn): its
  `last_active` and `bytes` are the last delivery's (§2.5). Expiry does not read them.
- **The live corpus replay needs the owner's WARP shell** for its cleanup: the force is an operator's
  action, and the credential is never an operator (§5.3). It is repeatable only from a machine where
  the owner is signed in through WARP.
- **Push (R-6.5b.11) is not built**; its trigger is the exit's measured counts.
- **The zone is not changed** (the owner, 2026-09-23): Web Analytics auto-install and Bot Fight Mode —
  its JavaScript detections with it — stay ON. The beacon's CORS error is DIAGNOSED at the first deploy;
  it is fixed by configuration if the cause is ours to fix, and otherwise the live tests account for
  exactly that error. A "zero console errors" reading is not the goal: a known, recorded error is
  allowed; an unknown one is not.
- **`wasm-opt` 133 on this workstation against CI's 132.** The deployed module is built at 132.
- **Inherited and untouched**: the vs-bot gate livelock, the e2e `CAP = 4000`, `walk.json`'s 419 KB
  pretty-printed diff, "who starts" on the local form, nothing in CI building the self-host image,
  the `npm test` order-dependent flake.

---

## 7. The handover's opening items, placed

| handover §4 | here |
|---|---|
| 1 the phones | OWNER, against the Worker, recorded not gated |
| 2 the headed walk | TAKEN at the exit on the Worker (same `dist`): the door and its controls at 390 px, the five error strips |
| 3 the clean-browser single-audience reading | CARRIED: it tests the path-scoped design, which R-6.5c.1 does not deploy |
| 4 the public Worker's configuration | DECIDED, R-6.5c.1 |
| 5 `cf-access` in place of `claimed` | TAKEN on the Worker, R-6.5c.3; the NAS stays `claimed` |
| 6 the automatic upload | DECIDED, R-6.5c.11 — with R-6.5c.14's off switch and the outbox's 404 fix |
| 7 the "Lobby" controls under a single-player build | CARRIED to the public-door step (this Worker offers multiplayer) |
| 8 the SPA-fallback cells | TAKEN, R-6.5c.10, battery rows 13–14 |
| 9 the untrusted `GameRecord` | BOUNDED, §2.6 |
| 10 no-Web-Locks, background suspension | CARRIED |
| 11 the §6.1 measurements | MEASURED by the spike (O-1, O-2, O-3, O-6, O-7, CPU); a whole bot turn and `why` live at the exit |
| 12 the CI workflow parse check | TAKEN as a plan gate |
| 13–18 | CARRIED as recorded there (18's items are §6's inherited list) |
| 19 R-F6 not measured live | CARRIED to the owner: an Access re-login in a browser the worker controls needs a non-WARP person |
| 20–25 the fix wave's residuals | CARRIED |
| 26 the TDD commit shape | ASKED at the plan |
| 27 the edge rewrites `index.html` | EXPLAINED by the spike (§0.3 row 8); the zone settings are OWNER items |

**Owner items this spec adds — ANSWERED 2026-09-23 unless marked open:**
- **The zone**: Web Analytics auto-install and Bot Fight Mode (with its JavaScript detections) stay ON.
  The beacon's CORS error is diagnosed at the first deploy and fixed by configuration if it can be,
  else accounted for by the live tests (§6); this is settled before §5.4's live tier.
- **The TTL**: 90 days.
- **The operator list**: the owner's address, as a Worker secret. It must name the owner for the live
  replay's cleanup, §5.3.
- **The bucket**: created 2026-09-23 (§3.1).
- **The read-only R2 key for the exit's sync**: CREATED by the owner on 2026-09-23 in the dashboard
  (Object Read, `yokohama-archive` only), its two values in the gitignored `.dev.vars` as
  `R2_ACCESS_KEY_ID` and `R2_SECRET_ACCESS_KEY`. It was verified that day: it lists the bucket, and it
  is refused a write and any other bucket. It was not minted through the account API, whose reply
  would put the secret in a session transcript. The exit syncs with the AWS CLI's `aws s3 sync`;
  `rclone` is not installed on the workstation.
- **OPEN, optional**: a lifecycle rule on the bucket's `test/` prefix, where every live tier's saves
  accumulate (the e2e's game-over and abandon saves carry fresh ids on every run).

---

## 8. Read order for the session that writes the plan

1. This document, §1, §2, §5.
2. The review's report and the re-review's (`review-spec.md` and `rereview-spec.md` in
   `.superpowers/sdd/2026-09-22-web-frontend-m6.5c-spec/`, gitignored) and
   `docs/temp/m6.5c-spike-readings-2026-09-22.md` (§0.3's source), and the scratchpad spike source if
   it survives, as the shape the shell's first cut can start from.
3. The umbrella §1 and §4; the M6.5b spec §2.1–§2.2 (the browser host this module is the sibling of)
   and §6.3 (the node corpus replay §5.3 extends); the M6.5b2 spec §2.2 and §2.6 (the health read and
   the outbox R-6.5c.14 touches).
4. `CLAUDE.md`'s gate commands; re-count at your own HEAD.

---

## 9. What the reviews changed (so a sentence can be traced to its reason)

### 9.1 The first read (`f80e2f7` → `8dc9096`)

The Opus adversarial read of `f80e2f7` (its report is in §8's second item; every finding carried evidence
it gathered, several measured through the committed module), and the owner's four additions while it
ran.

| finding | what changed here |
|---|---|
| 1 HIGH — `keeps → Some(0)` fails `check_shape`; the kept table leaves the lobby (measured) | §2.5 scrubs to `Some(1)`; the scrub cell runs through `Games::list`; `house_copy` runs `check_shape` (made `pub`, R-6.5c.4); battery 19 |
| 2 HIGH — the R2 order was the reverse of `DirArchive`'s; abandon refusable for ever | §2.6 ports the order (record read, table overwrite, record create-if-absent); half-pair and re-plan cells; battery 20 |
| 3 HIGH — a chat post's bump cleared the picker | §2.4 clears only when the record changes; the chat-between-clicks cells; battery 21 |
| 4 HIGH — a stale acknowledgement could erase a newer slot or a removal | §2.5 clears only a matching `(g, r)` and compares against the pending copy; battery 22 |
| 5 HIGH — the socket arm could not see an expired Access session | §2.7's one probe through `once()`; battery 23 |
| 6 — the `sync()` step duplicated the output gate; its mutant was equivalent | §2.4 relies on the gate; battery 4 recorded uncatchable; §6 names the WebSocket case |
| 7 — the clock-pin mutant survived a one-second clock | §5.2's archive cells tick per reading and compare `archived_at` |
| 8 — twelve served payloads had no instrument | §5.3's `walk-host.json` and `archive-walk.json` from the emitter |
| 9 — the `Value` round trip changed the layer's answers (measured) | §2.2 `create` inserts the id textually; battery 26 |
| 10 — `?as=` had no sender | §0.4's measurement; R-6.5c.3 takes the header on upgrades; `?as=` not built |
| 11 — the frame vocabulary was incomplete and untyped | §2.4's framing, mid-stream `gone`, silent failures, ignored client messages; §2.7 drops `pong` |
| 12 — the layer's `eprintln!` is mute on wasm | §2.8's shell logging; §6 |
| 13 — no body bound at the Worker | §2.3's 2 MiB bound |
| 14 — the alarm armed only after a failure | §2.4 arms it in the batch; battery 25 |
| 15 — generations had no order | §2.5's ordered generations and retired sets; both-order cells |
| 16 — host-written sentences contradicted "every sentence is the layer's" | §4 lists every host-written status and sentence |
| 17 — the live tiers wrote into production | §2.6's `test/` prefix; §5.3's filter, cleanup and marker handling; §5.4's cleanup |
| 18 — rows written were underestimated | §2.8 re-estimates; `written_at` replaces an activity row; wakes behind a variable; the exit counts rows |
| 19 — the `nodejs_compat` reason was false, and the spike ran with it on | §0.3 row 3 and §3.1 corrected; the readings file corrected |
| 20 — no cell covered the game-over archive path | §5.2's game-over cells |
| 21 — the missing-file 404 was the Worker's | §0.3 row 7; §2.3's first routing row; §2.8 |
| 22–37 (minor) | `meta` keys (22); tagged bumps (23); §2.8's arithmetic (24); the JWT-verify CPU (25); the glue cited by construct (26); the conditional put read at deploy (27); "the 65th clears" (28); `set_clock` restores (29); §0.6's three umbrella amendments (30); side-channel traps (31); the pump in the signal cell (32); keeps in the value pin (33); ignored socket messages (34); transport parity (35); the assembly verifies every digest (36); "when the copy changes" (37) |
| escalation — deploys terminate sockets and re-run constructors | §5.5's no-deploy window |
| owner — the archive switch | R-6.5c.14; §2.6; §2.7's outbox branch; battery 27–28 |
| owner — single player's invocation points | §2.8 |
| owner — garbage collection | R-6.5c.15 (expiry, the marker); `Games::expire` (DROPPED at §9.2 #4); battery 29–31 |
| owner — the operator surface | R-6.5c.16; §2.9; battery 32 |

### 9.2 The scoped re-review (`8dc9096` → this text)

The same reviewer, read-only, on the fix wave: the five HIGH fixed in MECHANISM (each tried, not
re-read), §0.4's WebSocket reading reproduced; findings 8, 9, 14, 17 and 18 of §9.1 PARTLY addressed,
the rest addressed; and twenty-two new findings, all accepted. Its report is in §8's second item too.

| finding | what changed here |
|---|---|
| 1 HIGH — the live replay could not run twice: the cleanup left 90-day markers on its fixed ids, and `archive-walk.json`'s first send assumes an empty bucket | §2.9's `?force` leaves no marker and clears one; §5.3 forces the three ids before and after each run and accepts the once-rule's 200 for the first sends (corrected at §9.3); R-6.5c.16; §6; also the lobby filter's `next` rule, found while fixing it |
| 2 MEDIUM — `"id": null` answered 201 for a table no object held (measured) | §2.2 `create` parses as the layer's `NewGame` first and refuses `"id": null` 400; §4; battery 34 |
| 3 MEDIUM — battery 25's cell was vacuous: `evictDurableObject` drains the push | §5.2's `abortAllDurableObjects` cell; battery 25 re-pointed |
| 4 MEDIUM — `Games::expire`'s reason was false: `config_of` forbids a creator without a seat | R-6.5c.4 now TWO layer changes; expiry is the creator's own `delete` (§2.2's host-local `expire`, §2.4 step 1); the unreachable cell removed |
| 5 — a missing field's column still moved | §2.2 forwards every refused body unchanged; §5.1 |
| 6 — `error_response` is `pub(crate)` | §2.2 restates its shape, pinned against `dispatch` |
| 7 — "no client of this host sends `wait`" was false | §2.3's reason corrected (`signal.ts:476`) |
| 8 — the probe fired on the arm's own closes | §2.7; battery 37 |
| 9 — a delete ended the source twice | §2.7 ignores the close after `end`; battery 38 |
| 10 — `wakes` lost its qualifier | §2.4 restored |
| 11 — "an ordinary move does NOT re-arm it" contradicted the slot's batch; the rows undercounted a delivering move | §2.4's three alarm rules; §2.5's one-batch acknowledgement; §2.8 re-derived per kind of move (turn counts read off the 595-event fixture: 92 placements) |
| 12 — `acked` could regress | §2.5 records it only when newer; §5.2; battery 35 |
| 13 — the marker's sentences missed the saved-nothing and stamped-at-game-over endings | §2.4's `kept` and the head-and-tail sentences; §4; §5.2 |
| 14 — the `test/` rule read the truncated segment; the reply's path named no key | §2.6 reads the full address and defines the root; §4; battery 33 |
| 15 — a `Conflict` at expiry retried hourly for ever | §2.4's `held`; §2.6 "final at every site"; §6; battery 39 |
| 16 — expiry while the archive is off discarded an owed save | §2.4, §2.6 and §6 say so; §5.2 |
| 17 — `/api/art` had no walk row to follow | §5.3: the emitter synthesizes it |
| 18 — the listing's `last_active` and `bytes` lag | §2.5, §2.9, §6 say so |
| 19 — single player's accounting under-listed | §2.8: Abandon and save is two; failed sends retry per `online` and launch |
| 20 — the expiry cells needed a clock the test controls | §5.2 moves the STORED times through `runInDurableObject` |
| 21 — battery 30's mutant was the design | battery 30 reworded |
| 22 — no route for an unidentified `POST /api/archive` | §2.3 routes it to the house (the layer's 401) |
| found while fixing — a body's id reaches a header, which carries only Latin-1 | §2.3's create row percent-encodes it; the path id is bound raw, as the layer binds it |
| found while fixing — a create while a removal is unacknowledged would overwrite it | §2.4's 409 rule; battery 36 |

### 9.3 Found while writing the plan (after `cf86c0e`)

Read against the code while the plan was drafted; each is a sentence of this document that was false
or incomplete, corrected here.

| finding | what changed here |
|---|---|
| The `wait` cap was called the layer's 400: it is axum's `events` HANDLER's (`lib.rs:586-590`, behind `server`), so `parse_query` never refuses `wait=26` | §2.2's `events` restates the cap, its sentence and the handler's order; §2.3; §5.1's cell pins the order by adjacent-pair probes |
| The live archive rule named `archive_identical.json`, which does not exist (the file is `archive_post_identical.json`), and missed that the walk's SECOND first send — the partial record, `archive_post_abandoned.json` — also turns into the once-rule's 200 on a re-run, with no committed file for that answer | §5.3's rule covers both first sends: the committed 201, or the same body at 200 with `"identical": true`; §9.2 row 1 |
| §2.4's "one synchronous batch" did not say how an alarm joins SQL writes, and nothing recorded what that rests on | §2.4 names `transactionSync` plus a `setAlarm` in the same turn; §6 records the reliance on write coalescing |
| The live replay's house-chat payloads cannot repeat: the lobby chat is shared by every run and every real user, so `seq` and `last` climb run over run | §5.3's live bullets normalize them against the house's `last` read at the run's start |
| `create`'s string-id branch trusted routing silently: a mismatch would answer 201 for an object holding nothing | §2.2's rule refuses it 500, named in §4 (the plan's Ruling P-4) |

### 9.4 Found by the plan's adversarial review (after `cca0aed`)

The plan's review (`.superpowers/sdd/2026-09-23-web-frontend-m6.5c-plan/review-plan.md`) MEASURED the pool
and the layer; five of its escalations were this document's sentences.

| finding | what changed here |
|---|---|
| §5.2's battery-25 technique was vacuous a second time: a method patched on a live instance makes the pool's RPC THROW (it refuses an own property), so correct code and mutant both answered `true` | §5.2 prescribes a gate on the PROTOTYPE, restored before the alarm, with fresh stubs (measured `true`/`false`); §0.4 records the pool facts |
| §2.2's "a body the two parsers would read differently fails the layer's parse" was false: the layer accepts a positional ARRAY (measured 201) | §2.2 refuses a non-object body 400, named in §4; the NAS's own quirk is left for the cleanup cycle |
| §2.5's anti-vacuity partner claimed the stored document carries the password and the hex seed; it holds neither | §2.5 names the real partner — decimal seed, salt, hash, a keep of 2 — and the other two as absence-only needles |
| §7 put the zone's Web Analytics decision before the exit, but §5.4's live tier already asserts no console error | §7 needs it before §5.4 |
| §2.4's close "with the layer's sentence as the reason" throws above 123 UTF-8 bytes at this compatibility date | §2.4 truncates the reason, and answers any other admission status unupgraded |
| §5.2's "one frame per socket per bump" sends a finalizing keep's two bumps (`bump_table` then `bump`, `tables.rs:1507-1511`) as two identical frames | §5.2 says one frame per socket per REPLY |

### 9.5 The plan's scoped re-review, and the owner's answers (after `ad8caeb`)

The plan's scoped re-review (`.superpowers/sdd/2026-09-23-web-frontend-m6.5c-plan/rereview-plan.md`)
found one sentence of this document out of date. The owner's answers the same day added four more.

| finding | what changed here |
|---|---|
| §0.5 said wrangler's login has no R2 scope. Its scope list names none, but `wrangler r2 bucket list` and `wrangler r2 bucket info yokohama-archive` both answered on 2026-09-23 (the re-review's m-j, then measured) | §0.5 corrected; §3.1's reason for the API-made bucket restated |
| The exit's sync named `rclone`, which the workstation does not have | §2.6's keys, §5.5 and §7 say any S3 client; the exit uses the AWS CLI |
| The read-only R2 key was an OPEN owner item | §7: created by the owner and verified |
| R-6.5c.4 did not say who rules on a third layer change | R-6.5c.4: the executing orchestrator, with its architectural reason recorded and listed at the close |
| §6 let an `exceededCpu` reopen the CPU assumption without saying who diagnoses it | §6: a dedicated research subagent; only a substantial departure from this spec goes back to the owner |

---

## 10. Corrections recorded in execution (2026-09-24 to 2026-09-26)

M6.5c was executed from this text at `50f705f` (the execution ledger is
`.superpowers/sdd/2026-09-23-web-frontend-m6.5c-plan/progress.md`, gitignored; the close
`docs/plans/2026-09-26-web-frontend-m6.5c-close.md` §5 carries every ruling with its cost). Execution
found sentences above that were false, incomplete, or overtaken by a ruling. **They are recorded here
and not edited in place: read each sentence of §0–§9 as amended by its row below.** Each row quotes
the sentence as it reads (so it can be searched for), says what execution found, gives the sentence
as ruled, and names the ruling by the ledger's label — the pre-flight scan's row ids, a task
review's finding labels, or the orchestrator's execution rulings I-65–I-91. The plan's own sentences,
which have no dated section after its two revisions, are in §10.8.

### 10.1 The Worker's routes and identity (§2.3)

| § | the sentence as it reads | what execution found | as ruled | ruling |
|---|---|---|---|---|
| §2.3, the routing table; §4 | "`GET\|HEAD /api/art` \| the static manifest's bytes, `env.ASSETS.fetch("/art/manifest.json")`"; and §4: "`GET /api/art` is served as the static manifest's bytes; the axum route re-serializes the same manifest; the replay compares it by VALUE." | The file carries ELEVEN top-level keys — the nine maps and the generator's `area_duplicates` and `provenance` — while the NAS's typed route (`Json<art::ArtManifest>`) and the committed `art_manifest.json` carry nine. The two did not agree by value, and the Task 11 cell (the raw file against the Worker) could not see it: a payload that differed on this host. | `GET /api/art` answers the manifest PROJECTED onto the nine maps `ArtManifest` declares — `orders`, `techs`, `sites`, `achievements`, `areas`, `five_power`, `mercenaries`, `backs`, `tokens`, in the struct's order. It equals the NAS's answer by VALUE, not by bytes (a JavaScript object orders integer-like keys numerically). The pool compares it with the COMMITTED payload by value and by key set, and the corpus replay's art row is deep-equal and UNEXEMPTED. | Task 12's finding, ruled host-local |
| §2.3, the routing table | "`/api/games/{id}` and below … \| `Table` named by the segment exactly as the layer binds it — raw, never percent-decoded" | An object named over 1,024 bytes has no `ctx.id.name`, and the layer's `is_safe_id` accepts at most 64 bytes. | A path segment — or a create body's `id` — that is empty, over 64 UTF-8 bytes, or holds a lone surrogate is routed to `House`, whose `dispatch` answers the layer's own refusal: no request can create a nameless object, and no host-written sentence answers it. | Task 9's escalation 4; Task 11's concern 1 |
| §2.3, "The transport routes' parity" | "A non-`GET` on `…/events` or `…/stream` is a **405** with an empty body, axum's router's answer." | axum's `get` route answers HEAD with 200 and an empty body. | This host answers HEAD on `…/events` **405**, the sentence's letter. No client and no committed row sends HEAD, so nothing observable differs. | Task 10 |
| §2.3, new | — | A WebSocket upgrade is outside CORS: without a check, a hostile page in a signed-in player's browser could open an authenticated socket and read a table's turn and timing frames. The NAS's stream over `fetch` was CORS-protected. | A WebSocket upgrade — on `…/{id}/stream` or `/api/stream` — whose `Origin` header is PRESENT and is not the request URL's own origin is refused **403** `this socket is for the site's own pages`, unupgraded. An absent `Origin` (curl, the deploy readings) passes; a plain request is never checked. Read live at deployment 14 (same origin 101, foreign 403, a plain GET with a foreign `Origin` 200). The owner may reverse it. | Task 11's review, escalation 1 |
| §2.3, "**Identity**"; §2.3's archive row | "the address normalized as `User::parse` normalizes it" | JavaScript's `trim()` strips non-ASCII whitespace and `toLowerCase()` maps U+212A KELVIN SIGN to `k`, so an Access-verified `Kevin@x.test` would have been forwarded as another user's `kevin@x.test`; workerd sends a non-ASCII header value as UTF-8 rather than refusing it. | The address is judged on the RAW `email` claim — and the raw claimed header — BEFORE any trim: an address outside tab, space and visible ASCII is NO ONE (the layer's 401, as on the NAS); lowercasing is ASCII-only, as `User::parse`'s is. | Task 2's note E1; Task 11's review I1 |
| §1 R-6.5c.3 (3); §2.3 "**Identity**" | "honours the `X-Yokohama-Claimed-User` header" | A dev-door claim that is not an address reached a host-written 500 at an Archive object under `r2`, where the layer answers 401. | A claimed value is an identity only if it is an ADDRESS by `User::parse`'s rule — at most 254 bytes, no whitespace or control character, exactly one `@` with something on both sides (`isAddress`, the one function the operator list's check shares); anything else is no one, and the layer's 401 answers. A differential of the shell's rule against `User::parse` over 60,018 generated strings read 0 disagreements. | R1's L-2 (fix wave A, `0919931c`) |
| §2.6, "A bad value, or `r2` with no `ARCHIVE` binding" | "every `/api/*` answers **500** naming the variable (host-written)" | The rule was written for the archive switch; the shell reads other variables too, and wrangler's `vars` can hand it a number or a boolean. A malformed operator list was echoed, value and all, in that 500. | EVERY variable the shell reads — the archive switch (trimmed and ASCII-lowercased first), `YOKOHAMA_WEB_TABLE_TTL_DAYS`, `YOKOHAMA_WEB_ADMINS`, `YOKOHAMA_DEV_DOOR` and `YOKOHAMA_WEB_COUNT_WAKES` (`on` or `off`), the test clock, and the chat values through the module's `init` — is refused the same way: **500** on every remaining `/api/*`, naming the variable; a non-string value is refused naming it; the operator list's refusals name an entry by its POSITION and echo no value — `YOKOHAMA_WEB_ADMINS entry <i> of <n> is empty`, `YOKOHAMA_WEB_ADMINS entry <i> of <n> is not an address — one @ with something on both sides, no spaces or control characters, and at most 254 bytes`. | Task 2's E2 and E3; I-29; R3's M-6 (fix wave A) |
| new | — | An exception escaping an object (a non-trap throw) reached the platform's own plain 500 — not the API's `{"error": …}` shape — with no log line. | A rejection from any object's `fetch` answers **500** `the server could not answer — try again` in the API's shape, logged `object-failed {target, path, error}`. A trap never reaches it: each object answers its own trap 500. | F-1 (the orchestrator's whole-branch reading; fix wave A) |
| §1 R-6.5c.3 (1); §2.3 "**Identity**" | "The Worker VALIDATES `Cf-Access-Jwt-Assertion` (RS256 against the team's keys, `aud`, `iss`, expiry)" | A token header with no `kid` would have matched a key with none; Access's set holds no such key today. | A token whose header names no string `kid` is no one, before any key is looked up. | F-3 (fix wave A) |

### 10.2 §4's host-written statuses and sentences, as they stand

§4 lists the host-written class as it was designed. Execution added to it; the table below is the
whole class at the close, route by route, each marked with whether §4 named it. Everything else a
client reads is still the layer's.

| where | route or event | status | the sentence (or body) | named in §4? | ruling |
|---|---|---|---|---|---|
| the Worker | a non-`/api`, non-`/enter` path with no file | 404 | empty | yes | — |
| the Worker | any method but GET or HEAD on `/enter` or `/api/art` | 405 | empty | no — §2.3 names the 405 for the transport routes only | Task 11 |
| the Worker | a body over 2 MiB | 400 | `the body could not be read: length limit exceeded` — the NAS's own sentence, read off the NAS live on 2026-09-23 | yes | — |
| the Worker | a bad variable | 500 | a sentence naming the variable (§10.1); for the operator list, `YOKOHAMA_WEB_ADMINS entry <i> of <n> is empty` and `YOKOHAMA_WEB_ADMINS entry <i> of <n> is not an address — one @ with something on both sides, no spaces or control characters, and at most 254 bytes` | the archive's only | E2, E3, I-29, R3's M-6 |
| the Worker | a WebSocket upgrade from another site's page | 403 | `this socket is for the site's own pages` | no | Task 11, escalation 1 |
| the Worker | an exception escaping an object (a rejected `fetch` of any object) | 500 | `the server could not answer — try again`, logged `object-failed` | no | F-1 |
| the Worker | `/api/admin/*` for a signed-in non-operator | 403 | `this address is not an operator here` | yes | — |
| the Worker | `POST /api/admin/tables/{id}/expire?force=<any value>` | 400 | `?force takes no value — send ?force alone, or no query for an expire that saves first` | no | I-72 |
| `Table` | a GET of `…/stream` that is not an upgrade | 426 | `this host streams over WebSocket` | yes | — |
| `Table` | a trapped dispatch | 500 | `the table's engine stopped — try again` | yes | — |
| `Table` | a second consecutive load trap | 500 | `this table's stored <kind> cannot be loaded` | yes | — |
| `Table` | a create under a marker or an unacknowledged removal | 409 | `this name was used recently — try another` | yes | — |
| `Table` | every request while a marker stands, an upgrade included (answered unupgraded) | 404 | a HEAD — `this table was abandoned on <YYYY-MM-DD>`, `this table expired on <YYYY-MM-DD> after <N> days without activity` (`after 1 day` for a TTL of 1), `this table was removed by an operator on <YYYY-MM-DD>` — then a TAIL — ` — its game is in the archive`, ` — no move had been made, so there was nothing to keep`, ` — this server keeps no archive, so its game was not kept`: nine sentences in all | yes, but for the singular day and §10.3's `kept` reading | I-69; I-71 |
| `Table` | a force whose removal the house has not confirmed | 503 | `the lobby has not confirmed the removal — the alarm will finish it` | yes | — |
| `Table` | a deleted table's sockets | `end` frame, then a 4404 close | `{"reason":"deleted"}`, then the close reason `this table was deleted` | the frame and the code yes; the close reason no | Task 10 |
| `Table` | a socket refused at ADMISSION (a 403 or 404) | `end` frame, then a 4403/4404 close | `{"reason":"gone","sentence":<the layer's sentence>}` — the SENTENCE field is this host's; the mid-stream `end` stays the SSE's own `{"reason":"gone"}` | no | Task 8's review, escalation 2 |
| `House` | a trapped dispatch | 500 | `the lobby's engine stopped — try again` | no | Task 11 |
| `House` | `/api/stream`: a non-GET; a GET that is not an upgrade | 405; 426 | empty; `this host streams over WebSocket` (the table's sentence) | the 426's sentence yes; the house route is new | I-36, I-48 |
| `Archive` | a trapped dispatch | 500 | `the archive's engine stopped — try again` | no | Task 11 |
| the module, `create` | a parsed body that is not a JSON object; `"id": null`; a string id that is not the object's name | 400; 400; 500 | `a game must be created from a JSON object`; `a game id, when given, must be a string`; `the create's id does not name the table it was routed to` | yes | — |
| the module, `expire` | the resident-handle arm: a table gone, or naming no creator, under a resident game | 500 | `"<id>" has a game in play but no table document that names its creator` — unreachable through the Worker, loud if it ever is not, in the create routing-defect's class | no | Task 6's review, I1 |
| the module, the receipt archive's root | a put it cannot root | inside the layer's own "the archive could not be written …" | four details: `a browser's save is filed by its Archive object's address, and this instance was given none`; `the table document names no creator`; `the archive keeps no root for the prefix "<prefix>"`; and `a browser's save is filed by its Archive object's address, and this save was sent by another caller` — the last for a browser save whose path names another caller than the object's, unreachable through the Worker, which names the Archive object by the address it forwards | no | Task 6's review, I1; R1's L-1 (fix wave A) |
| the module, `init` | a configuration that is JSON but not an object | 500 with the sentence, logged `init-refused` (I-29) | `the module's init configuration is the object of its five keys — archive, caller, lobby_chat, chat_keep and chat_text` — it echoes nothing, since the configuration carries the Archive object's address; unreachable through the Worker, whose shell builds the object | no | R1's L-3 (fix wave A) |
| the shell, R2 | a create-if-absent refused as present, then re-read as absent | inside the layer's refusal of the delete (a `failed` receipt's `detail`) | `<record_key> refused a create as present, then read as absent` | no | Task 13's review, I2 |

### 10.3 The table object (§2.4)

| § | the sentence as it reads | what execution found | as ruled | ruling |
|---|---|---|---|---|
| §2.4, "**Storage**", the `meta` keys | "`meta(k TEXT PRIMARY KEY, v TEXT NOT NULL)`: `generation` and `revision` (§2.5); `slot` …; `acked` …; `archive_owed` …; `marker` …; `held` …; `load_traps`; and `wakes` — incremented only when `YOKOHAMA_WEB_COUNT_WAKES=on` AND the object already holds a table" | Three keys the mechanism needs are not on the list, and `wakes` had no reading path at all — not a route, a listing field, or a log line. | The keys are also **`poison`** — the KIND whose load trapped (the plan's Task 9); **`copy_owed`** — the `house_copy` traps in a row while a copy is owed, which drives the copy's retry ladder (§10.4); and **`expiry_failed`** — an idle table's expiries in a row that did not end it, which drives the expiry's 30 s / 2 min / 10 min / hourly backoff and is removed by the ending's batch. `held` is on the list as written. **`wakes` is read through the shell's log line `wake {id, wakes}`**, written when a counted wake's batch commits. | the plan's Task 9; Task 12's review M1; I-67; I-84 (the line landed at fix wave A, `0919931c`) |
| §2.4, "**Storage**" | "`written_at` is the object's clock at the write" | "The object's clock" had two candidates: the platform's clock, and the module's pinned reading (P-2), which in the pool is frozen ~13 days behind workerd's clock, so an alarm computed from it is overdue at once. | EVERY time the SHELL keeps — `written_at`, every alarm deadline, every retry's due time, the marker's `at`, the expiry comparison, and a generation's time (§10.4) — is the platform's `Date.now()`, in milliseconds. The module's pinned reading, in whole seconds, feeds ONLY what the layer stamps. The shell's test clock is one exported reader patched at module scope; the TTL is one exported reader. | I-43, I-44, I-34 |
| §2.4, "**Traps**" | "A trap in a SIDE CHANNEL — a socket's `events`, `house_copy`, `stamp_archive` — happens after the request's batch and reply; the instance is discarded and the work retries on the alarm or the next request." | `house_copy` is not a side channel: the slot and its alarm ride the REQUEST's batch, so the copy is computed before the batch. And a framing trap has no request of its own to retry on. | `house_copy` is computed IN the request path, BEFORE the batch, on a scratch module instance of its own, so a trap never empties the module the frames read. On a trap the documents still commit, the slot is left as it was, `copy_owed` is written with the alarm on its ladder, a request that finds a copy owed recomputes it, and the reply stands. Any non-trap throw building that instance counts as a trap. For FRAMES, "the next request" means the next BUMP: frames a trap lost are not re-sent until the next change, as the SSE's comment arm drops them. | I-42, as adapted at Task 12 (M1, M2); Task 10's escalation 1 |
| §2.4, "**Traps**" | "A trap during LOAD increments `load_traps` … Any successful load resets the count." | A trap while BUILDING the module (its instantiation or `init`) is not a stored document's fault; and the plan's brief would have written the reset only in a batch a trapped call never commits. | LOAD means `load` and `restore_picker`'s re-feed; a trap building the module is logged `trap` and is not a load trap. The sentence holds as written: a successful load resets the count even when the call that follows it traps (one batch holding the two deletes). | Task 9; Task 9's review ⚠️1/M1 (the spec over the brief) |
| §2.4, "**Sockets**" | "on **403 or 404** it ACCEPTS, then sends the SSE's own `end` frame `{"reason":"gone"}` and closes with **4403** or **4404** and the layer's sentence as the reason" | A browser cannot read a close reason, so the socket arm showed generic words where the stream arm shows the layer's sentence. | At ADMISSION the `end` frame is `{"reason":"gone","sentence":<the layer's sentence>}`; the MID-STREAM `end` stays byte-identical to the SSE's `{"reason":"gone"}`. A 403 at admission also reads `reason: "gone"`, as a 404 does; no consumer branches on `reason` today. | Task 8's review, escalation 2; R3's L-1 |
| §2.4, "**The picker across hibernation**" | "if a re-fed line changes the record, fails, or leaves `sub` closed, the list is CLEARED and the picker starts empty — today's behaviour after a restart, with `Input::expect` turning a stale answer into a 409 rather than a wrong move." | For the "changes the record" arm the sentence is false: by then the line has APPLIED and persisted a move, which a restart never does (probe P1: `1` re-fed at the same record, 0 → 1 events). What kept a re-fed line from applying a move was "the same code", and nothing checked it — while the design case for re-feeding is a DEPLOY. `Input::expect` does not guard the opening line, which is sent at the main prompt. And the ENGINE also gives a line its meaning: its legal-list order numbers the main prompt, and a pure reorder must not bump `SEMANTICS_REV`. | A picker row carries the CODE IDENTITY of the sources that give a line its meaning — a SHA-256 over every file under `crates/yokohama-core/src`, `crates/yokohama-cli/src` and `crates/yokohama-web/src` (dotfiles skipped), computed by the worker crate's build script. `restore_picker` CLEARS a row stamped with other code, or with none (every row deployments 14–18 wrote), and applies nothing; the shell logs `picker-dropped {id, stored, running}` (`stored: null` for an unstamped row) after the request's batch commits, and the player re-opens the picker. Under the row's code, a line re-fed at the row's record does again what it did when recorded; a line that fails or closes the picker clears the list and applies nothing, as a restart leaves it. So every deploy that changes those sources drops the open pickers — the safe loss — and a rollback across the fix wave drops the ones the newer code stored. | I-85 (R1's M-1); I-89; I-91 (R1's R-1, N-2) |
| §2.4, "**Delete, expiry and the MARKER**", step 1 | "expiry and the operator's plain expire are the SAME request dispatched as the stored table's `created_by`" | The module's `expire` reads the creator itself; the SHELL needs the creator too, for its reading of the table before the delete, and "the table document is a string end to end". | The shell reads an expiry's creator off the table's HOUSE COPY — its own already-parsed, seed-zeroed JSON — never the table document; an abandon's viewer is the DELETE's own caller. A document whose copy traps answers the abandon with the trap 500 where it used to reach the layer's refusal: the copy and the layer's delete share ONE `check_shape`, so neither accepts such a document. | I-65; I-73 |
| §2.4, step 1 | "for expiry it leaves the table in place to retry on the alarm" | The retry had no schedule of its own. | A failed expiry retries on the 30 s / 2 min / 10 min / hourly ladder through `expiry_failed`. An hourly `expiry-failed` line is read by its BODY: a body naming the DOCUMENT (it does not parse, or is malformed) means an operator's force; the archive's refusal means wait — a force during an R2 outage would drop the game. | I-67; Task 14's review N-1 |
| §2.4, step 2 | "`kept` is `archive` when the table was already stamped or this delete's save landed or was `Identical`; `nothing` when there was no game to keep (an open or dealing table, or no move made); `off` when there was a game and the archive is off." | Deriving `kept` must not parse the table document; the first derivation read "playing" only, and read the switch before the stamp — so a game stamped while the switch was on read "not kept" after it went off. | `kept` reads `archive` WHENEVER the table's house copy carries `archived_at` — the layer's own stamp, true of the bucket whatever the switch reads — checked BEFORE the switch; then a plan that landed or was `Identical` is `archive`; a reading (`events(id, creator, "")`, taken before the delete) of `playing` OR `over` with a move made is a game; no game is `nothing`; a game while the switch is off is `off`. | I-64, I-66, I-71 |
| §2.4, step 5 | "*"this table expired on 2026-10-22 after 30 days without activity"*" | — | A TTL of 1 reads `after 1 day`. | I-69 |
| §2.4, "**Expiry**" | "when the alarm finds `now ≥ last activity + TTL`, the object runs step 1 as the creator" | A table created before the deployment that set the TTL has no alarm armed for its expiry. | A table from before the TTL's deployment carries no expiry alarm until its next write or an alarm for other work; the deployment's readings force every stale table the listing shows (at deployment 17 it showed none). | Task 14's deploy note |
| §2.9, the operator's `?force` | "**`?force` ends the object without the layer and without a save, whatever it holds**" | `?force=false` would have forced, dropping a save the operator meant to keep; a query on the listing was silently ignored. | `?force` is the BARE key only — `?force` or `?force=` forces; `force` with ANY value is **400** naming the switch (§10.2), never a force and never a plain expire; `GET /api/admin/tables` with any query is unrouted, the layer's 404. A force logs `archive-owed-dropped` for a save it drops, whatever the switch reads. | I-72; I-74; I-70 |
| §2.8, "**Logging**" | "With observability on, the shell logs to Workers Logs: every receipt outcome, stamp, slot push and acknowledgement, trap, poison, expiry, marker end, operator action, and every copy `house_copy` refuses." | The mechanism needed more lines than the categories name. | Beyond those categories the shell also logs `object-failed` (F-1), `wake` (I-84), `picker-dropped` (I-85), `slot-failed`, `slot-ignored`, `expiry-failed`, `expiry-blocked`, `held`, `restore-refused`, `frame-failed`, `frame-carried-bumps`, `alarm-without-name`, `alarm-failed`, `house-chat-dropped`, `archive-owed-kept`, `archive-owed-dropped`, `identity-failed` and `init-refused`. RUNBOOK §6.5 is the vocabulary, each line with its fields. | Tasks 9–14; fix wave A |

### 10.4 The house, the slot and the archive (§2.5, §2.6)

| § | the sentence as it reads | what execution found | as ruled | ruling |
|---|---|---|---|---|
| §2.5, "**The durable slot**" | "Each table holds a `generation` — `(created_at, 64 random bits)`, drawn once at create and ORDERED by that pair" | `created_at` is the layer's whole-second reading. A re-create of an id in the SAME SECOND as its retired generation ordered before it half the time, and the house then ignored — and acknowledged — every push of the new table: unlisted for 90 days. A force then a create is one operator script, and the live replay forces and re-creates seconds apart. | A generation is `(the create's Date.now() in MILLISECONDS, 64 random bits)` — the shell's own time base — so a tie needs the same millisecond (20 create → force → re-create sequences in the pool read the two generations 5–12 ms apart). No committed payload carries a generation. A generation drawn in seconds before the change keeps its `at`, and every milliseconds one orders after it; so a ROLLBACK to a version that draws seconds is a generation-order rollback — a table re-created under it goes unlisted until the retired generation is pruned, 90 days after its retirement (RUNBOOK §6.6). | F-2 (the orchestrator's whole-branch reading); Task 12's review M4; I-89 |
| §2.5, "**The durable slot**" | "The clear, the `acked` and the alarm's re-arm (§2.4) are one batch." | A straggling acknowledgement can arrive after the removal's `deleteAll()`. | An acknowledgement writes only while the object's SQL tables still exist, so a straggler recreates nothing; a batch with nothing to write is skipped. | Task 12's ruling (`hasTables`) |
| §2.5, "**The scrub**" | "**`house_copy` then runs the newly public `check_shape` on the copy and refuses one it rejects**" | A stored document `check_shape` rejects only in a field the scrub REPLACES (a keep outside 1–2, a lock that is not hex) scrubbed into a listable copy the NAS's `Games::list` would drop. | `house_copy` runs `check_shape` on the stored ORIGINAL first and refuses it, then scrubs and checks the copy; the refusal is `"<id>'s house copy is malformed, so the house would not list it: <why>"`. A refused copy leaves the house listing the LAST copy it received — only a corrupted store reaches it — and the operator's force is the path. | Task 5's concern 2; Task 5's review M2 and escalation 1 |
| §2.4 "**Traps**"'s retry; §2.5 | "the work retries on the alarm or the next request" (for a copy) | One alarm retry, and a trap in it armed nothing, so a table whose copy trapped twice after its create stayed unlisted for good — the invisible failure. | An owed copy (`copy_owed`) is recomputed by every request while owed and retried on the push's ladder after an alarm-path trap, so the sentence holds. | Task 12's review M1 |
| §2.5, new | — | The house has no poison count. | At the second consecutive load trap of the house's stored `chat` document the house DELETES that row, logs `house-chat-dropped`, and serves without it: the lobby chat is not training data, and every table copy is re-pushable, so the lobby never stays 500. | Task 11's review, escalation 3 |
| §2.6, "**One clock reading per plan**" | "The layer stamps `archived_at` from its clock INSIDE the table body it puts … The shell records the clock reading with the plan and pins the module's clock to it (`set_clock`) for every dispatch that drives that plan" | The layer stamps `archived_at` only at the two SERVER sites — game over, and a delete that saves. A BROWSER's save is filed as sent and stamps no time: its plan carries no reading, and a pin at the Archive object's second dispatch is unobservable (battery r17b, equivalent by measurement). | One clock reading per SERVER plan. A browser save's plan is reading-free; the Archive object still pins its dispatches, and no cell can see that pin. | Task 6's review M4 and note f; R2's r17b |
| §2.6, "**The receipt**" | "The conditional put's exact form (`onlyIf` with an `If-None-Match: *` header, or the object form) is read off the first deploy that touches R2 (§3.2)." | — | The form that shipped is `onlyIf: new Headers({"If-None-Match": "*"})`, ACCEPTED by production R2 at deployment 16. The REFUSAL on a present key was read against miniflare's R2 only. | Task 13; deployment 16 |
| §2.6, "**Game over**" | "on success installs the receipt and calls `stamp_archive(id)`" | `Ok(())` from the retry only means it RAN; "not stamped" alone cannot tell an undone game from a failed stamp, so the alarm would retry for ever. | `stamp_archive` answers `{"stamped": bool, "over": bool}`; the shell clears `archive_owed` on `stamped`, clears it on `!stamped && !over` (the undo), and retries on `!stamped && over`; a `conflict` receipt clears regardless. | I3 (Task 1's review), refined at Task 6 |
| §2.6, "**Switched off after being on**" | "an expiry WHILE OFF takes the owed save with the table, and the shell logs it" | An ABANDON while off takes it the same way. | An abandon while the switch is off also drops an owed save, logged `archive-owed-dropped` after the committed write; with the switch on the delete's own save supersedes it and nothing is logged. | Task 13's ruling and review I1 |
| §2.6, "**Keys**" | "for a browser save the one the Archive object is named by" | The archive's root read the object's caller and the layer's prefix the sender's, and nothing checked that they agree. | A browser prefix whose segment is not `caller_segment` of the object's own caller is refused, fail-closed, with `a browser's save is filed by its Archive object's address, and this save was sent by another caller`; nothing is deferred. Segments collide by design, so a match proves the segment, not the sender's spelling. | R1's L-1 (fix wave A) |

### 10.5 The client (§2.7)

| § | the sentence as it reads | what execution found | as ruled | ruling |
|---|---|---|---|---|
| §2.7, "**One transport arm**" | "`signalKind()` (`web/src/api.ts:309`) widens to `"sse" \| "held" \| "ws"`, set from `/api/health`." | The fact's default is `sse` until `/api/health` answers, so on the Worker the lobby's house channel was fetched as SSE before the facts landed — a 426 and a console error, live (run 1). The first fix waited without a bound, so a health read that never settles opened NO stream on the NAS while the word read "live". | No stream or socket opens before the deployment's transport is known: the arm waits for App's health read, BOUNDED by App's own `BOUND_MS`; the word reads "reconnecting…" while the wait runs and is never "live" before Connected; after the bound the remembered or default transport is used, and a retry issues a NEW health read once the pending one is older than the bound. On the Worker a hung health read costs one 426 per rung; on the NAS a stream opens up to the bound later than it used to. As landed: the bound runs from the health read's ASKING, and an answer that lands after it is still written down. R3's probe at the fix wave's head read "reconnecting" from 0 to 4.9 s, the default stream at 5.1 s, then "live". | I-81; I-86 (R3's H-2; fix wave B, `e20c8ff8`) |
| §2.7 | "every **30 s** the arm sends `ping`" | The interval was cleared only by the `close` event, so a tick between `close()` and that event sent into a closing socket and Chromium logged an error (live run 1). | The keep-alive stops at hang-up, and a tick sends only while the socket is OPEN. | I-81 |
| §5.1 (the word tied to Connected); §2.7 | the socket arm's words are the stream arm's (`Link`) | `Link` has no "connecting" word, and the bounded wait speaks while a transport is being asked. | **Recorded, not changed**: on a browser with no remembered health answer — a first visit, cleared storage, or a storage that refuses — every server-signal screen on BOTH hosts reads "reconnecting…" from its first open until it connects: one health round trip, at most 5 s when health hangs; a returning visitor sees no change. The word on a FIRST connection is therefore "reconnecting…"; a "connecting" word is a wording change for the cleanup cycle. | R3's R-N3; I-91 |

### 10.6 The tests and the exit (§0.4, §5)

| § | the sentence as it reads | what execution found | as ruled | ruling |
|---|---|---|---|---|
| §5.3, the live half | "**repeatable, and safe to re-run after a run that failed half way**" | Repeatable only while `wire`'s records are byte-identical. Each record carries `engine_version`, so a workspace version bump, a walk that plays other moves, or a record-format change without a rev bump makes the two FIRST sends answer 409 — the walk's own 409 row is built exactly that way — and the next row's status STOPS the run. Only deleting the four keys under `test/browser/red_at_wire.test/rev7/…` and their tables recovers it, with wrangler's login (the owner's R2 key is read-only). | Repeatable while `wire`'s records are unchanged. After such a change a first send's 409 STOPS the replay, which names the cause and prints its reset — the four `npx wrangler r2 object delete "yokohama-archive/test/browser/red_at_wire.test/rev7/{over,abandoned}/wire{,.table}.json" --remote` lines, run with wrangler's own login (RUNBOOK §6.7, "The replay's reset") — and then the replay again. Run twice on deployment 18, both HELD — the second run's first sends answered `200` with `identical: true`. | R3's M-1 (fix wave A) |
| §5.3, the live half | "**The lobby payloads**: `games` compared after filtering to the replay's own ids …" | A lobby GET sent right after a create, a join or a keep can reach the house BEFORE the table's slot push, which rides `waitUntil` after its reply. | The replay re-reads the four lobby rows a BOUNDED number of times — 5, 500 ms apart — prints the bound and the count spent, and a page still differing after them is a MISMATCH. Both live runs spent 0. | I-76 (6) |
| §0.4; §5.4 | §0.4: "Playwright's context headers reach a page-opened upgrade"; §5.4: "each context presents the credential and its own `X-Yokohama-Claimed-User` at `@e2e.test` as context headers — which reach fetches and page-opened upgrades alike (§0.4). Each spec deletes the tables it created at its end" | Context-wide credential headers would carry the Service Auth secret to third-party hosts (the analytics beacon's), and their CORS preflight is itself the beacon's console error. A ROUTE-BASED header injection keeps a header to one origin, but **Playwright's request routing does not reach a page-opened WebSocket upgrade** — and the socket is the live tier's point. A Playwright trace records an API context's headers and a page's cookies; a thrown API call's "Call log" prints every request header, the cookie included, to the reporter and to `test-results/`. | The CREDENTIAL never goes context-wide: each live context takes Access's own `CF_Authorization` cookie, fetched ONCE by Node's `fetch` with the credential and seeded through an in-memory storage state, so Playwright never holds the secret. The CLAIMED header goes context-wide, the one header an upgrade must carry; it is no secret. Because it rides the beacon's cross-origin request, the beacon's CORS error recurs, and the live harness forgives EXACTLY its two console lines (tied to the beacon's stable URL prefix), and the deal's designed 409, and nothing else. Tracing is OFF under the live target; every live `request.*` goes through one helper that rethrows the first line alone; cleanup is `test.afterEach`. | I-75, I-76, I-77, I-79; Task 11's ruling (4), superseded for the Playwright tier only |
| §5.5 | "two people play a whole game and a third watches, HEADED; the table object **observed hibernating** between moves — `wakes` rising across pauses …; … a finished game and an abandoned one in the bucket … **loaded in the CLI with `why` reconstructing a bot move**" | A whole headed game at human pace is hours; the finished game's evidence already existed from live run 1 (2,862 moves, played to a winner on the live Worker); `wakes` had no reading path (§10.3); and on macOS a COVERED headed window reads hidden whatever Chromium's two occlusion switches read, so stacked windows let their sockets go by design (R-P8). | The exit is an opt-in spec, `web/e2e/exit_game.spec.ts`, run HEADED with the tail open between two deploys: three contexts, a SHORT game (forty moves) at HUMAN pace with four pauses longer than the object's idle eviction, table chat and lobby chat, a watcher throughout, the console watched on every page, the three pages' VISIBILITY asserted as the spec's own precondition, the three windows placed SIDE BY SIDE through the browser protocol, and an abandon at its end. The FINISHED game in the bucket is live run 1's; `why` runs on a short human-vs-bot game created through the API and abandoned; `wakes` is read off the `wake` line. Its cost: a whole game is not re-played headed. Runs 1 and 2 (deployment 19) passed their seating live and stopped at the visibility precondition — stacked windows, then Chromium's 500 px minimum width — so the spec lays a 2×2 grid when three windows cannot share the width. **Run 3 PASSED** (2026-09-26): 40 moves in 307 s, four 60 s pauses each followed by a `wake` line (`wakes` 1 → 4) and the move's POST in 67–75 ms against a 58 ms median, table chat 10 and 10 with the watcher reading all 20, lobby chat 1 and 1, the watcher's one stream, every console clean but the two forgiven shapes (the beacon's lines, the deal's designed 409), and the abandon's marker read. The bucket's two finished games are live run 1's and the two-browser re-run's; `why 0` reconstructed a bot move. | I-88; I-84; I-91 (R3's R-N6); `f2c27a3f`; `754b80ae`; the owner's direction of 2026-09-24 ("lag times so hibernation is proven … with chat and the works") |
| §5.6 | "**§5.3-live, §5.4 and §5.5 stay local**, because they need the credential; written into the job, as the e2e's locality is." | The workflow did not mention the live tiers. | As it stands: the workflow still does not say it (R3's docs item 24, open after the fix wave); the live tiers and the exit need the credential and run on the owner's workstation alone. | R3's stale-sentence list, item 24 |
| §5.7, row 17 | "the receipt loop's clock left unpinned (§5.2's ticking-clock archive cells)" | The Archive object's own pin cannot be observed (§10.4). | Row 17 is caught at the table's sites by two variants; the third, the Archive object's, is EQUIVALENT by measurement and recorded, as Task 13's M30 was. Row 4 is recorded as §5.7 says; the battery read 38 caught, 0 survived, 1 recorded, at `d025ba43` and again, all 39 rows, at the fix wave's head `e20c8ff8`; r17b's equivalence is now written into `archive.ts` and its cell. | R2; I-83 |

### 10.7 What §6 did not claim, and §7's owner items

| § | the sentence as it reads | as it stands | ruling |
|---|---|---|---|
| §6; also §2.2 ("`wasm-opt -Oz …` at the pinned binaryen **132**") | "**`wasm-opt` 133 on this workstation against CI's 132.** The deployed module is built at 132." | Every deployed module — deployments 14 onward — was built with `wasm-opt` **133**, the workstation's binaryen (1,854,072 → 1,220,664 B). CI builds with 132 and never deploys, so it cannot reproduce the deployed bytes; the cleanup cycle aligns the two pins. | I-87 (R3's H-1; R1's E-1) CI was pinned to 133 in the close's CI commit `a9c8be27`, so its module now matches the deployed bytes; the Dockerfile's NAS image stays at 132 for the cleanup cycle. |
| §6, the CPU | "The Worker's own CPU with JWT verification is unmeasured until the first deploy." | MEASURED at deployment 14 off the tail: **0–2 ms** per `/api/*` request, 1 ms typical, WITH the JWT verification — inside Free's 10 ms. Startup 1–2 ms at every deployment that read it. An object's CPU, read at the exit: **2 ms** a move (the median over 40 move POSTs; the maximum 15 ms) and **6–9 ms** a wake. | deployment 14; the exit |
| §6, the zone | "The beacon's CORS error is DIAGNOSED at the first deploy; it is fixed by configuration if the cause is ours to fix, and otherwise the live tests account for exactly that error." | DIAGNOSED at deployment 14: the cause was the spike HARNESS's context-wide headers — ours. A real browser logs nothing, and the reading scripts send the credential to the Worker's origin only. The live Playwright tier still forgives exactly that error, because its claimed header must be context-wide (§10.6). The zone is unchanged. | Task 11's diagnosis; I-75 |
| §6, hibernation's cost | "Measured at 595 events; the e2e's own games reach 1,700 applied moves" | The live two-browser games reached **2,862** moves (2,180 applied events) and **2,510**. A wake of the exit's 40-move table cost **6–9 ms of CPU, 16–60 ms wall**, and the page saw the first POST after a 60 s pause answer in 67–75 ms against a 58 ms median; a wake on a record of thousands of events was not measured. | live run 1; the re-run; the exit's run 3 |
| §6, inherited | "the `npm test` order-dependent flake" | Three NAMED flakes, each reproduced first and FIXED at `e20c8ff8` by its cheapest fix: a budget (a `Surfaces.test.tsx` cell at 2,087 ms alone, split into its three cases), a race in a test (`Lobby.test.tsx`, now awaiting the listing's link), and an order dependence (`App.test.tsx`, reproduced under shuffle seeds 22 and 44, fixed by one `resetFactsForTest()`). `npm test` read 1056/56 in order twice, shuffled, and under three concurrent suites. | R3's L-11, L-12, L-13; fix wave B |
| §7, item 2 | "TAKEN at the exit on the Worker (same `dist`): the door and its controls at 390 px, the five error strips" | TAKEN on deployment 19 at 390×844, headed, two `.test` contexts: the door, the lobby, the create form, the open table in both views, the deal, the board and the folded chat panel read clean, and no layout defect was seen. ONE of the five strips was provoked in the browser — the unlock strip, the open table's password refusal for a stranger (a `/join` 403, "wrong password for this table"). Not provoked: the create form's (a spaced name is a legal display name), the deal's (the page's guard prevents a second keep; its 409 was read through the API) and the chat panel's (the input eluded the script; the empty post's 400 was read through the API); the open table's own strip was not attempted. | the exit (the close §3.3) |
| §7, item 11 | "a whole bot turn and `why` live at the exit" | `why 0` reconstructed a bot move from a record synced out of the bucket at the exit — `v4-step9-draft1`, "Only legal option", the 34-row explain table — and the honesty gate passed under this build's constants. A bot turn's CPU was not read live. | the exit |
| §7, owner items | "**OPEN, optional**: a lifecycle rule on the bucket's `test/` prefix" | ANSWERED 2026-09-24: no rule now; this cycle's `test/` saves remain; the cleanup cycle chooses between a periodic script and an R2 expiry on the prefix, with a recommendation. | the owner, 2026-09-24 |

### 10.8 The plan's sentences (`docs/plans/2026-09-23-web-frontend-m6.5c-plan.md` at `50f705f`)

| where | the sentence as it reads | as it stands | ruling |
|---|---|---|---|
| Review Focus 1 | "`{"players":2,…}\n`, which `curl --data @file` and most editors produce" | `curl --data @file` STRIPS carriage returns and newlines (measured, curl 8.7.1); `--data-binary` and most editors keep them. The mechanism — whitespace after the closing brace — is pinned; a live reproduction uses `--data-binary`. | Task 3's finding |
| Pool facts, 8 | "**The pool cannot see the production case** (re-review m-b, measured): it reports the name everywhere, even through `idFromString`." | MEASURED at Task 9: in the pool an object reached ONLY through `idFromString`, or named over 1,024 bytes, has NO `ctx.id.name`, so the pool CAN deliver a nameless alarm, and a cell does. What the pool cannot show is a production alarm's name, and no alarm fired live at deployments 14–18. | Task 9's finding; Ruling R-09's pair kept |
| Global Constraints, the quota guard | "If either is above **60,000** (Free's limit is 100,000), it HOLDS the run until the UTC day rolls over." | **80,000** for this cycle, for each guarded figure. | the owner, 2026-09-23; I-82 |
| Tech Stack | "`wasm-opt` (binaryen **132**)" | CI's pin until the close's CI commit `a9c8be27` moved it to 133; the workstation that deployed ran 133 throughout (§10.7). | I-87 |
| Names shared across tasks, Task 12 | "`type Generation = { at: number; rand: string }` — `at` is the create's P-2 reading (equal to the table's `created_at` …)" | `at` is the create's `Date.now()` in milliseconds (§10.4). | F-2 |
| Task 10 | the socket accepted "with `ctx.acceptWebSocket(server, ["t"])`" | The tag is the table id, as spec §2.4 says: a later `getWebSockets(id)` written from the spec would have found no socket. | Task 10's review M1 |
| Global Constraints, "`web/` changes are only these" | the WebSocket arm, the outbox branch, their tests, the live configuration, the allow-list | WIDENED for Task 15's fix round 2 to `web/src/signal.ts`, `api.ts` and `ws.ts` and their test files — the three findings live run 1 made. | I-81 |
| Task 15 | the live cleanup "in a finally" | `test.afterEach` (measured: a timed-out test's `finally` runs after its page has closed). | I-77 |
| Task 16, item 3 | "**Two people play a whole game and a third watches, HEADED**" | The exit as ruled (§10.6). | Task 16's exit instrument |
