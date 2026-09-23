# M6.5c "the objects" — the spec

**Written 2026-09-22 on branch `web-room-m6.5c` (stacked on `web-room-m6.5b2` at `2621fbe`, the
M6.5c handover's HEAD; the branch's first commit `952a68c` only ignores `.dev.vars`), from the
brainstorm held with the owner that evening, from the entry handover
`docs/plans/2026-09-22-web-frontend-m6.5c-handover.md`, and from a PLATFORM SPIKE the owner
authorized mid-brainstorm and that ran on the real account before a word of the design was written
(§0.3).** This document is M6.5c's authority under the umbrella
`docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` (R-6.5.1–13, as amended by R-6.5b.16 and
R-6.5b2.2); the plan is written from it; the close is its record. Where it and the umbrella's §2–§5
disagree, this document is the later ruling and §0.5 says exactly where.

**The headline.** Hosted multiplayer moves onto Cloudflare, at `yokohama.moore.nyc`, behind the same
Access application as today. A stateless Worker serves the release as static assets without being
invoked, and runs only for `/api/*` and `/enter`: it validates Access's assertion, resolves the
caller, and forwards to a Durable Object by name. One object per table is that table's store, lock
and signal; one house object is the lobby's projection and the house chat; one archive object per
caller strict-replays what a browser sends. Every object runs the SAME table layer the NAS runs —
`api::dispatch` over `Games` — in its own instance of a WebAssembly module, so every payload is the
layer's own and a trap costs one instance, never a table. The signal is a hibernatable WebSocket
carrying the same frames the SSE stream carries today. Finished and abandoned games land in an R2
bucket in the file store's own layout. **No code outside the new host changes except one named
public function in the layer and one transport arm in the client**; the NAS, the browser module, the
CLI and the five fenced crates stay as they are. (The runbook, the CI workflow and `CLAUDE.md` move
with the cycle, and `deploy/worker/` and `deploy/modal_app.py` are deleted — §3.3.)

**Read order for the session that writes the plan:** §1 (the rulings), §2 (the mechanism), §5 (the
instruments and the battery — the plan's tasks end in these), §3 (the deployment and the first
deploy), §6 (what is not claimed), §7 (the handover's opening items, each placed).

---

## 0. What is true at HEAD, and what this spec relies on

### 0.1 HEAD

- Branch `web-room-m6.5c` at `952a68c` over `2621fbe`. Seven PRs open and stacked, nothing merged,
  merge order the owner's (handover §1). **LIVE on the NAS: `yokohama-web:0.5.0-m6.5b2.1`** at
  `yokohama-nas.moore.nyc` (RUNBOOK §3.16), `identity: claimed`.
- `SEMANTICS_REV` 7; `crates/yokohama-core`, `-oracle`, `-difftest`, `-bridge`, `gen-data` and
  `crates/yokohama-cli` byte-identical to `369a1f2` (M6.5b2 close §1.11).
- **`yokohama.moore.nyc` has NO DNS record.** The dead tunnel CNAME
  (`6a6c468e-5e1e-4a7c-88a6-4e2fe07da1fc.cfargotunnel.com`, proxied, ttl 1) was removed for the
  spike and not restored; the name is held for this cycle's Worker (§0.3, the readings file).
- `deploy/worker/` and `deploy/modal_app.py` are still on disk (R-6.5.13 removes them, §3.3).

### 0.2 The layer's surface the new host consumes (opened at `2621fbe`)

The new host drives the layer through surfaces that are ALREADY public; that is what makes R-6.5c.4
(changes stay in the new host) cheap.

- **The front door**: `api::dispatch(games, host, identity, req) -> ApiResponse`
  (`crates/yokohama-web/src/api.rs:1984`) over `ApiRequest { method, path, query, content_type,
  body }` (`:34`); `Host { identity_name, signal_kind, serves_games, multiplayer_offered }` (`:54`);
  the route table `api::routes(serves_games)` (`:149`).
- **What `dispatch` does NOT serve**, because axum's ROUTER owns it: `GET /api/games/{id}/events`,
  `GET /api/games/{id}/stream`, `GET /api/stream` (`lib.rs:883-885`), `GET /api/art` (answered by
  the adapter before `dispatch`, `lib.rs:757`) and `GET /enter` (`lib.rs:845`). The new host serves
  these itself — the transport routes through the public `api::events_at(games, id, user) ->
  Result<Events, ApiError>` (`api.rs:1641`), whose `Events` (`api.rs:1545`) carries `your_turn`, so
  **a frame is per viewer**.
- **The state**: `Games::with_parts(rules, store, chat, signal, clock, archive)` (`games.rs:287`);
  `Games::handle(id) -> Result<Arc<Mutex<Repl>>, GameError>` is **public** (`games.rs:391`) and the
  `Repl`'s `sub: SubPrompt` is a public field (`crates/yokohama-cli/src/repl.rs:43`, the enum at
  `:190`, which has **no derives at all**); `Games::load_document` / `read_document`
  (`games.rs:352`, `:359`) are the side channel `yokohama-wasm` already uses.
- **The seams**: `Store` is synchronous (`store.rs:121`); `Signal` is four methods over `&str`
  (`signal.rs:38`); `Archive::put(prefix, why, id, record_json, table_json) -> Result<PutOutcome,
  PutError>` is synchronous (`archive.rs:93`), with `PutOutcome::{Landed, Identical}` (`:44`),
  `PutError::{Conflict, Failed}`, and ONE layout function `archive::paths` (`:119`).
- **The two archive sites**: game over — `persist` (`games.rs:485`) calls the PRIVATE
  `archive_if_over` (`:682`), which stamps `archived_at` only after a landed put and logs a failure;
  abandon — `Games::delete` (`tables.rs:1655`) puts BEFORE unlinking and REFUSES the delete on any
  put error. The browser's save — `api::archive_post` (`api.rs:1915`) — runs `check_pair` and puts
  under `browser/<caller_segment>/`.
- **The move**: `api::input(games, user, id, &Input)` is public (`api.rs:1031`); it persists only
  when the event count or the lock changed, so a picker click that applies nothing exists ONLY in
  the resident `Repl` (`Input::expect`'s doc, `api.rs:990-1011`).
- **The table document**: `TableDoc` (`table.rs:170`) with public `seed`, `keeps`, `lock`,
  `unlocked`; `TableDoc::view(user)` (`table.rs:371`) reads the lock and each keep for their
  **presence only** (`locked: self.lock.is_some()`, `chosen: … k.is_some()`); `Lock { salt, hash }`
  (`table.rs:104`). The lobby is `Games::list(user, scope, limit, before)` (`tables.rs:1556`) over
  `Doc::Table` alone, each row `doc.view(user)`.
- **Identity**: the layer's three modes (`identity.rs`) — `claimed` (`X-Yokohama-Claimed-User`, the
  `dev-identity` feature), `cf-access` (`Cf-Access-Authenticated-User-Email`), `worker`
  (`X-Yokohama-User`, under which the literal `anonymous` is ABSENT, `:162`).
- **The browser host** `crates/yokohama-wasm` keeps ONE global `Games` (`src/lib.rs`, `static
  GAMES`) — right for one tab, and the M6.5b close §8 warning for a shared isolate.

### 0.3 What the platform spike MEASURED (2026-09-22, owner-authorized, throwaway, torn down)

A throwaway Worker `yokohama-spike` — one SQLite-backed class, the real module (`yokohama-wasm` at
`2621fbe`, `release-wasm` + `debug`, `wasm-opt` **133** on this workstation against CI's pinned
132: 1,783,435 → 1,175,167 B), `workers_dev` and preview URLs off, observability on — deployed
ONCE as the custom domain `yokohama.moore.nyc` (already a destination of the `yokohama` Access
application), measured, and deleted with its namespace and domain. The full record, with every
figure's method, is the gitignored `docs/temp/m6.5c-spike-readings-2026-09-22.md`; its source stays
in the session scratchpad and is not committed.

| # | What | Reading |
|---|---|---|
| P | Protection, from an unenrolled vantage (alpine on the NAS) | every path — `/`, `/sw.js`, `/api/health`, `/enter`, the object routes — **302 to the Access login** (`auth_status: NONE`, `is_warp: false`); a WebSocket upgrade with no credential **403**; `yokohama-spike.bdvc.workers.dev` **404** |
| 1 | **O-6**: identity on HTTP and on a WebSocket upgrade | over WARP: `Cf-Access-Authenticated-User-Email` and an `email`-bearing JWT on HTTP, **on the 101 of an HTTP/1.1 upgrade**, and in a socket opened from PAGE JavaScript (headless Chromium). With the `yokohama-worker` **Service Auth** credential: `email: null`, JWT `common_name` = the credential's client id, and Access sets `CF_Authorization` (HttpOnly, SameSite=None, ~30 days); **Service Auth outranks the WARP identity**; with the credential's headers removed after the first navigation, **the cookie alone** carries it on a page-opened upgrade and on `fetch` |
| 2 | **O-2**: size and startup | Total Upload **1,172.84 KiB / gzip 453.90 KiB**; **Worker Startup Time 1–3 ms** across five deploys (the module is compiled at load and instantiated per object on first use) |
| 3 | **O-7**: `getrandom` in an object | `create` inside the object drew `sim-c7e7f98029f58c13` and its setup seed through `crypto.getRandomValues`; **20 ms CPU**. `nodejs_compat` must stay OFF: `getrandom` 0.2's `js` detection takes Node's `require` path when `process.versions.node` exists |
| 4 | **O-1**: a trap | `debug_trap` → `RuntimeError: unreachable`, caught at the JavaScript boundary; the instance discarded; the next request re-instantiated and reloaded from SQLite and replayed correctly; 1 ms. **It needs a factory-wrapped glue**: wasm-bindgen's `--target web` output keeps `let wasm` at module scope and `initSync` returns early once it is set (`yokohama_wasm.js:460`), so a trapped instance could never be replaced and every object in an isolate would share ONE `GAMES` |
| 5 | **CPU on Free, and O-3** | one object invocation — a cold replay of 595 events plus seven bot-move `why` reconstructions — **`cpuTime` 376 ms, outcome `ok`**, on Workers Free. The cold path alone for the committed FINISHED 595-event game (a fresh instance, the 54,103 B record and 554 B table loaded from SQLite, `Session::from_record`, the frame): **19 / 27 / 22 / 28 ms**. `why` ≈ 50 ms per bot move. The routing Worker: **0–1 ms**. One instance's linear memory: 1,441,792 B after a replay, 1,900,544 B after the `why` run. CPU read from `wrangler tail --format json` (`cpuTime`), because timers do not advance while a Worker computes |
| 6 | **Hibernation** | a hibernatable socket with an attachment (`viewer`, the Access identity, `openedAtWake: 1`): after **35 s idle** the next message found **wake 2** — the object evicted and its constructor re-run — with the socket still open and **the attachment intact**; `setWebSocketAutoResponse(ping → pong)` answered before and after |
| 7 | **Static assets** | of the 153 precache entries, **152 byte-exact** (200, type essence, SHA-256) under the defaults; **`/index.html` answered `307 /`** under the default `html_handling` — the service worker fetches it with `redirect: "error"` (`web/src/sw.ts:190-191`), so its install would FAIL; `not_found_handling: "single-page-application"` answered **200 HTML for a missing `/assets/*.js` and `/sw2.js`**. **The configuration that is right**: `html_handling: "none"`, `not_found_handling: "none"`, a `_redirects` line `/ /index.html 200`, `_headers` (§2.3) — `/` and `/index.html` 200, a missing asset **404**, headers exact, **zero Worker invocations** for every static path (read off the tail). The art is NOT in `web/dist` (28 files): the asset directory must be assembled from `dist` and `crates/yokohama-web/assets/art/` (154 files, 9.8 MB). **The service worker installed completely from the Worker host**: active, `yokohama-release-d62d6763649a749f`, 154 entries, `/index.html` cached clean at 2,387 B, the door reading "Ready to play offline" |
| 8 | **The edge rewrites HTML** (handover §4 item 27) | zone `moore.nyc` (Free Website plan): **Bot Fight Mode ON, JavaScript detections ON, Web Analytics auto-install ON**. JS detections inject a `challenge-platform` script into navigations and into plain `curl`/Node fetches of `/` (3,325–3,692 B, varying, against the built 2,387 B) but **not into the service worker's own fetch** — which is why the NAS install succeeded. The Web Analytics beacon (`static.cloudflareinsights.com`) **fails CORS and logs a console error on every page load**. Headless Chromium was not challenged by Bot Fight Mode |

**Not measured by the spike:** whether a `ping` during hibernation wakes the object (the two cases
give the same wake count in its reading); a non-WARP browser; a phone; R2; alarms; object-to-object
calls; a WebSocket upgrade carrying Playwright's context headers (§5.4 measures it first).

### 0.4 The account and the door

- **Workers FREE** (R-6.5b.16), account BDVC. **Account-wide usage, last 7 days** (GraphQL
  analytics, read 2026-09-22): **17k–33k Worker requests/day** against Free's 100,000, and **300–1,800
  Durable Object requests/day** against 100,000 — both quotas SHARED with the account's ~97 other
  Workers; static-asset requests count against neither. Twenty SQLite-backed namespaces exist
  (Free allows 100 classes).
- **The door**: the `yokohama` self-hosted Access application (`11cbcf68…`, audience `5fa7a989…`)
  with three destinations — `yokohama.moore.nyc`, `yokohama-cli.moore.nyc`,
  `yokohama-nas.moore.nyc` — and three policies: `sbaitso-friendly` and `yokohama-players` (email
  lists; allow) and, added by the owner this evening, **"yokohama ST bypass"**, decision
  `non_identity` (Service Auth), precedence 3, including the service credential **`yokohama-worker`**
  (created 2026-09-23T01:01Z, expires 2031). Its client id and secret live in the repository root's
  `.dev.vars`, **gitignored since `952a68c`**.
- **wrangler 4.135.0** is logged in by OAuth on this workstation with `workers`, `workers_routes`
  and `workers_scripts` write — and **no R2 scope**, so the bucket is created through the account
  API or by the owner (§3.1).

### 0.5 What this spec supersedes or amends

- **Umbrella §2.1's host table** names the Durable Objects host "`yokohama-worker` (new;
  workers-rs)". **R-6.5c.2 replaces workers-rs with a TypeScript shell over a wasm-bindgen module**;
  the crate keeps the name `yokohama-worker`.
- **Umbrella §2.3's H-4 options** (a `SubPrompt` row, the socket attachment, or the client
  re-opening the picker) — **R-6.5c.7 takes a fourth**, host-local re-fed input lines.
- **Umbrella §2.4's "an index row per table"** — **R-6.5c.8**: the house holds SCRUBBED COPIES of
  table documents and lists them with the layer's own `list`, so there is no row type to drift.
- **Umbrella §2.3's pending-deliveries LIST** for "the three pushes that have no next bump" —
  **one durable slot per table** (R-6.5c.8), since only the newest copy ever matters.
- **Umbrella §2.3's "delete removing the house row BEFORE `deleteAll()`"** — **eventual** toward
  the lobby: a tombstone and its alarm outlive the documents until the house confirms (R-6.5c.8).
- **Umbrella §2.6's `claimed` door "as a query parameter that the Worker refuses whenever Access
  identified the caller"** — made exact by **R-6.5c.3**: honoured only when Access passed the
  request on the Service Auth credential and named no one, behind a Worker variable.
- **Umbrella R-6.5.12 and §5's "Workers Paid"** stay amended by R-6.5b.16 (Free); **R-6.5c.5**
  records the CPU assumption the owner ruled and the spike measured.
- **Umbrella §5's "one Worker (`yokohama-web`)"** is kept as the name.

---

## 1. The rulings (the owner, 2026-09-22, in the brainstorm's order)

Each is an owner decision, or a recommendation the owner approved by name. A sub-cycle may ask to
revisit one; none is re-litigated inside it.

- **R-6.5c.1 — at the close, `yokohama.moore.nyc` is private; the Worker is built for the public
  door.** Owner: *"it will eventually leave access but not at the end of 6.5c … we will deploy to
  yokohama.moore.nyc and it will be access protected but we will plan for the front door mechanics
  and the rest to not be protected eventually."* So the hostname-wide `yokohama` application stays
  in front; nothing is public and **the card art is not published this cycle** (`ASSETS.md`); and
  the Worker is built so that going public later is a CONFIGURATION change — single-player paths are
  static assets that never invoke it, the protected prefixes are exactly R-6.5b2.2's, and identity
  fails closed (R-6.5c.3). This decides R-6.5b2.6 for this cycle.
- **R-6.5c.2 — the host is a TypeScript shell over a per-object wasm-bindgen module, not
  workers-rs.** Chosen on evidence read from workers-rs 0.8.6's own tree (`86092a8`): hibernatable
  WebSockets present but exercised by one echo test and one "set and read back the auto-response
  pair" test, no test that hibernates and wakes an object, no README section; a panic "will
  terminate the isolate" without `--panic-unwind`, which needs NIGHTLY and a rebuilt `std` against
  this repository's pinned stable 1.97.1; RPC "experimental". R2 and SQLite are proven there, and
  the testing and portability arguments were a wash (workers-rs tests its own objects in TypeScript
  over Miniflare too). The shell uses the platform's first-hand API; the module is the browser's
  build pipeline reused. **Amends umbrella §2.1.**
- **R-6.5c.3 — identity, in order.** (1) The Worker VALIDATES Access's assertion
  (`Cf-Access-Jwt-Assertion`: RS256 against the team's keys, `aud`, `iss`, expiry), so a missed
  Access destination fails closed rather than trusting a bare header. (2) An assertion that names
  a PERSON (`email`) is the user; every client identity header and `?as=` is ignored (R-6.5.11).
  (3) An assertion that names ONLY the `yokohama-worker` Service Auth credential (`common_name`
  equal to its configured client id, no `email`), with the Worker variable `YOKOHAMA_DEV_DOOR=on`,
  honours the `claimed` identity — the header on a request, `?as=` on an upgrade (§2.3). (4)
  Anything else has no identity. The owner put the credential on the `yokohama` application as
  Service Auth ("so it shouldn't pass identity"). **The dev door is off whenever the public door
  goes live**; the NAS stays on `claimed`.
- **R-6.5c.4 — changes stay in the new host.** Owner: *"we are trying to not make changes to other
  modules to not increase debt and complexity where it's unneeded"*; the NAS/Docker host is
  reconciled with the Worker in a cleanup cycle AFTER M6.5c, and then the project refocuses on the
  computer player. Refined the same evening: *"yokohama-web is new within the broader cycle and
  isn't being used anywhere in production, so if we find an architectural issue that we should or
  need to address, we can."* So: a Cloudflare-specific problem gets a solution inside
  `crates/yokohama-worker` and `worker/`; the layer changes only for a named architectural reason,
  never for convenience; the CLI and the five fenced crates stay untouched. **This cycle's layer
  change is ONE public function (R-6.5c.9); its client change is ONE transport arm (§2.7).**
- **R-6.5c.5 — Workers Free, and an object invocation gets 30 s of CPU.** Owner: *"historically,
  for DOs on free tier or paid, the start point for CPU time in a DO is 30 seconds. We should build
  with that being the assumption and … we'll find out from the logging if we're wrong."* The
  documentation contradicts itself (the limits table's unqualified 30 s against its FAQ's "the same
  per invocation CPU limits as any Workers do", 10 ms on Free); **the spike's 376 ms invocation
  completed `ok`** (§0.3 row 5), and the live logs keep reading it. The Worker itself stays inside
  Free's 10 ms by never loading the module (0–1 ms measured).
- **R-6.5c.6 — the transport is signal-plus-fetch over hibernatable WebSockets.** The socket carries
  the same `Events` frame, house frame and `end` frame the SSE stream carries; the client fetches
  exactly as it does now. R-6.5b.11's push shapes are DEFERRED, with a trigger: if the exit's
  measured requests per game (§2.8) show the shared daily quota binding at the owner's usage.
- **R-6.5c.7 — an open picker survives hibernation by re-fed input lines, host-local.** Owner: *"I
  do expect hibernation to kick in almost every turn, most players take several seconds to take a
  turn."* Serializing `SubPrompt` was weighed and declined: it has no derives
  (`repl.rs:190`), so it would change the CLI and add a stored format coupled to an internal type.
  Mechanism §2.4.
- **R-6.5c.8 — the house is a second layer instance over scrubbed table copies.** One durable,
  revisioned slot per table feeds it, retried by alarm; delete is eventual toward the lobby.
  Mechanism §2.5.
- **R-6.5c.9 — the archive's asynchronous put is a receipt, inside the new crate**, with abandon and
  the browser's save re-dispatched under `blockConcurrencyWhile`, browser saves in one archive
  object per caller, and game over completed through **the one named layer addition,
  `Games::archive_over_retry(id)`** — a public function over the existing private `archive_if_over`,
  so the stamp is written by the layer's own code and never re-implemented in the host (the
  architectural reason R-6.5c.4 asks for). Mechanism §2.6.
- **R-6.5c.10 — static assets exactly as the spike proved** (§0.3 row 7): the assembled directory,
  `html_handling: "none"`, `not_found_handling: "none"`, `_redirects` and `_headers`. It re-proves
  handover §4 item 8 on this host.
- **R-6.5c.11 — the outbox's automatic upload is kept** (handover §4 item 6): with the Worker
  reporting `archive: "r2"`, a signed-in player's finished local games land under `browser/<them>/`
  in the bucket at game over, as M6.5b2 built it.
- **R-6.5c.12 — four tiers of test** (§5): native Rust for host logic; Cloudflare's vitest pool
  inside workerd for the shell and the battery; the corpus replayed against the objects at every
  task and against the LIVE Worker at the exit; the multiplayer Playwright specs against the LIVE
  Worker through the credential.
- **R-6.5c.13 — the spike ran before the spec** (§0.3), which is R-6.5.8's "test in the live setup"
  taken one step earlier than the umbrella planned; its first-deploy items (O-6, O-2) are MEASURED
  here and re-read by the plan's first deploy with the real code.

---

## 2. The mechanism

### 2.1 The pieces, and one request's path

| piece | what it is | answers |
|---|---|---|
| **the Worker** `yokohama-web` | stateless; `worker/src/index.ts` | `/enter`; routing and identity for every `/api/*`; never loads the module |
| **static assets** | the platform's asset layer | everything that is not `/api/*` or `/enter`, with **no invocation** |
| **`Table`** | SQLite-backed; one per table id, `idFromName(id)` | `POST /api/games` (create) and every `/api/games/{id}/…` route, including the three transport routes |
| **`House`** | SQLite-backed; ONE, `idFromName("house")` | `GET /api/games?scope=…`, `GET/POST /api/chat`, `GET /api/stream`, `GET /api/health`, `GET /api/me`, `GET /api/labels` |
| **`Archive`** | SQLite-backed class, stores nothing; `idFromName("archive:" + email)` | `POST /api/archive` |
| **the bucket** `yokohama-archive` | R2 | the archive, in `archive::paths`' layout |
| **the module** `crates/yokohama-worker` | a `cdylib` over `yokohama-web` with `default-features = false`, built like the browser's | the layer, inside each object |

**One move, end to end:**

1. The Worker validates the assertion and resolves the caller (§2.3), strips every client
   `x-yokohama-*` header, sets `X-Yokohama-User`, and forwards to `TABLE.idFromName(id)`.
2. The table object creates its module instance if it has none — `createModule` (§2.2), `init`,
   `load` of each stored document, then the picker's re-feed (§2.4).
3. `dispatch` answers with the reply AND what the layer bumped (§2.2).
4. The shell writes each changed document to SQLite. **Synchronous: the request has awaited
   nothing so far**, so no other request has interleaved (§2.4's input-gate rule).
5. The reply returns. The output gate holds it until the writes are durable.
6. Then the side channels, in this order: `await storage.sync()` (§2.4); one `Events` frame per
   socket, each computed for that socket's viewer; the house copy pushed if it changed (§2.5); an
   owed archive uploaded (§2.6).

### 2.2 The module: `crates/yokohama-worker`

A new workspace member, `crate-type = ["cdylib"]`, `publish = false`, over
`yokohama-web = { default-features = false }` — `yokohama-wasm`'s shape (R-6.5b.9), because the
browser crate is a HOST and so is this one; the browser module is not touched and does not grow.
Built by `worker/`'s `build:module` exactly as `npm run build:wasm` builds the browser's:
`cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker`,
`wasm-bindgen 0.2.128 --target web`, the FACTORY transform, `wasm-opt -Oz --strip-debug
--strip-producers` at the pinned binaryen **132** (the workstation's 133 is a local difference,
§6).

**The factory transform** (`worker/scripts/factory-glue.mjs`, the spike's `build-glue.mjs` made a
tool): wasm-bindgen's glue becomes `export function createModule(compiled)` returning the exports
over ITS OWN `WebAssembly.Instance`. It refuses loudly if an `export` survives or the expected
exports are missing, so a wasm-bindgen upgrade that changes the glue's shape fails the build rather
than shipping a shared instance. **Each object owns one instance; each instance owns one global
`Games`** — so the browser crate's global-`Games` shape is correct here too, because no instance is
ever shared.

**Its exports** (every one a thin wrapper over a native `_inner` function, `yokohama-wasm`'s rule,
so everything that can be wrong is testable under `cargo test`):

- `init(config_json)`: the host facts and parts. `Host { identity_name: "cf-access",
  signal_kind: "ws", serves_games: true, multiplayer_offered: true }`; `MemStore`; the chat
  configuration from the Worker's variables (`YOKOHAMA_WEB_LOBBY_CHAT`, `…_CHAT_KEEP`,
  `…_CHAT_TEXT`, the NAS's names and defaults, a bad value refused at `init` naming the variable);
  the clock `Date.now()` unless a frozen value is configured (test configurations only, §5.3); the
  `RecordingSignal`; and the `ReceiptArchive` when the bucket is bound, else `NoArchive`.
- `load(doc, id, text)` / `read(doc, id)` — the side channel, over `record | table | chat | house`
  (the browser's allows two; this host has all four).
- `dispatch(user, method, path, query, content_type, body) -> Reply { status, body, bumps,
  archive, picker }`: `bumps` the `RecordingSignal`'s calls during this dispatch (`bump`,
  `bump_table`, `bump_house_chat`, `end`, each with its id); `archive` the `ReceiptArchive`'s plan
  when a put was deferred (§2.6); `picker` the lines to persist (§2.4).
- `events(id, user)`: `api::events_at` serialized — the transport routes' payload, byte-identical
  to the axum host's `events` route and SSE frame.
- `create(user, body, id)`: `POST /api/games` with the id the Worker chose (§2.3) — the body is
  parsed as a `serde_json::Value` (which keeps a `u64` exact), `id` inserted when absent and
  REFUSED when present and different, then dispatched.
- `house_copy(id) -> json | null`: the table's stored document, scrubbed (§2.5).
- `restore_picker(id, lines_json)`, `receipt(plan_key, outcome_json)`, `stamp_archive(id)` (calls
  `Games::archive_over_retry`), `set_clock(seconds?)`, and `debug_trap` under the `debug` feature
  only (the battery's trap cells; never in the deployed module).

**The crate holds its own `Arc<MemStore>`** — the one it hands `Games::with_parts` — so it can
replace and remove documents through the public `Store` trait (`write_if_version`, `delete`) where
`load` would refuse a present one: the house's copies (§2.5) are replaced that way, and the exports
`put_copy(id, json)` and `remove_copy(id)` are the house's. No layer change is needed for it.

**`RecordingSignal`** implements the layer's `Signal` by appending to a per-dispatch buffer that
`dispatch` drains into the reply. It knows no socket: sending is the shell's, AFTER the write, which
is what keeps "a bump follows its write" true by construction rather than by care.

**The house frame** is `{"seq": N, "chat": M}` — the axum host's `House` (`signal.rs:92`) lives in
the server-only half of that file, so this crate declares its own two-field struct and a cell
compares its serialization with the SSE frame's.

### 2.3 The Worker

**Routing.** `run_worker_first: ["/api/*", "/enter"]`; everything else is the asset layer's.

| path | goes to |
|---|---|
| `GET /enter` | a 302 to `/#/lobby` (the turnstile; served because multiplayer is on) |
| `POST /api/games` | `Table` named by the body's `id` when it names one (read by parsing the body; the ORIGINAL bytes are forwarded), else by a drawn `sim-<16 hex>` from `crypto.getRandomValues`; the id rides in `X-Yokohama-Table-Id` to the module's `create` |
| `/api/games/{id}` and below, incl. `…/events` and a `…/stream` upgrade | `Table` named `id` |
| `GET /api/games`, `/api/chat`, `/api/stream`, `/api/health`, `/api/me`, `/api/labels` | `House` |
| `POST /api/archive` | `Archive` named `"archive:" + email` |
| `GET /api/art` | the static manifest, `env.ASSETS.fetch("/art/manifest.json")` (§4 names its bytes) |
| any other `/api/*` | `House`, whose `dispatch` answers the layer's own 404 or 405 — never a sentence the Worker writes |

**Identity** (R-6.5c.3) lives in `worker/src/identity.ts`, one pure function over (headers, url,
the team's keys, configuration) — tested without a network (§5.2). The keys come from
`https://bdvc.cloudflareaccess.com/cdn-cgi/access/certs`, cached per isolate; `aud` is the
application's audience; both are Worker variables (`ACCESS_TEAM_DOMAIN`, `ACCESS_AUD`), as is the
credential's client id (`DEV_DOOR_CLIENT_ID`) — none of them is a secret. **The strip rule carried
out of `deploy/worker/` (R-6.5.13), widened as R-6.5.11 says**: every client header beginning
`x-yokohama-` is removed before forwarding — `deploy/worker/src/index.ts:71` stripped
`x-yokohama-user` and not `x-yokohama-claimed-user`, which under `claimed` was impersonation of any
seat. The objects take the caller from `X-Yokohama-User` — the contract the layer's `worker` mode
names, which only the Worker can satisfy because the objects are reachable ONLY through it — and
hand it to `dispatch` as the identity, exactly as `yokohama-wasm` hands `local@browser`; the
literal `anonymous`, like an absent header, is no identity.

**Static assets** (R-6.5c.10), assembled by `worker/scripts/assemble-assets.mjs` into the gitignored
`worker/assets/` from `web/dist` and `crates/yokohama-web/assets/art/` (under `/art/`), plus two
committed files:

```
_redirects      /  /index.html  200
_headers        /assets/*   Cache-Control: public, max-age=31536000, immutable
                /sw.js      Cache-Control: no-store
                /art/*      Cache-Control: public, max-age=3600
```

— the axum host's cache policy (`assets.rs`, `art.rs`) restated for the platform, and every one of
them read back off the wire by the spike. The assembly refuses when `web/dist` has no `sw.js` or no
module, or when the art directory is empty, so a Worker cannot ship a release that is not the
precache's.

### 2.4 The table object

**Storage** — two tables, created on the FIRST WRITE and never by the constructor:

- `docs(kind TEXT, json TEXT, PRIMARY KEY(kind))`: `record` (the exact text
  `Session::to_record_json` wrote), `table`, `chat`, and the host's own `picker`.
- `meta(k TEXT PRIMARY KEY, v)`: `revision` (the house push counter), `wakes`, `house_slot`,
  `archive_owed`, `tombstone`, `load_traps`.

**Probing creates no garbage** (umbrella §2.2): an id that was never created reaches an object whose
constructor writes nothing and whose `GET` answers the layer's 404, so the object ceases to exist
when it goes idle. The spike's constructor did NOT honour this (it counted wakes unconditionally);
a cell holds it (§5.2). `wakes` is incremented only by an object that already holds a table — the
counter the exit reads for "observed hibernating".

**The wake path.** The constructor reads nothing it does not need. The first request that needs
the layer creates the instance (`createModule`, `init`), `load`s each `docs` row, and calls
`restore_picker`. Measured cost for a finished 595-event game: 19–28 ms (§0.3); on a turn-based game
this runs on almost every move (R-6.5c.7's premise).

**The request path, and the input gate.** A request's read → dispatch → write stretch contains NO
`await`: `sql.exec` is synchronous and so is the module. **Every side channel follows every storage
write of the request**, and they are ordered: first `await this.ctx.storage.sync()` — a STORAGE
await, which keeps the input gate closed — so the writes are durable before any frame leaves; then
the frames; then the house push and the archive upload, which are non-storage I/O and DO open the
gate (§0.4 of the umbrella). That ordering is the M6.3 rule "a bump follows its write and never
precedes it" in this platform's terms, and the battery arranges its violation (§5.7).

**Sockets.** A `…/{id}/stream` upgrade is admitted by `events(id, user)`: on 200 the object
`acceptWebSocket`s with the tag `id` and the attachment `{ user }`, and sends that frame at once
(M6.3's first frame); on 403 or 404 it ACCEPTS and then closes with code **4403** or **4404** and the
layer's sentence as the reason, because the browser's `WebSocket` reports a refused upgrade as an
anonymous failure that a client cannot tell from the network. On each bump that names this table
the object computes `events(id, user)` for every socket's attachment and sends it; on `end` it sends
the `end` frame and closes with **4404**. `setWebSocketAutoResponse(ping → pong)` answers the
client's keep-alive without waking the object (§2.7).

**Traps** (O-1). Every module call is wrapped. A `WebAssembly.RuntimeError` discards the instance
and answers **500** in the API's `{"error": …}` shape; **a trapped request persisted nothing**, so
its move was never acknowledged. A trap during LOAD increments `load_traps`; at **2** consecutive,
the object stops re-instantiating and answers every request **500 naming the stored document that
will not load** — a poison pill named rather than a loop — and recovery is an operator's (Data
Studio, RUNBOOK §6). Any successful load resets the count.

**The picker across hibernation** (R-6.5c.7), all in `crates/yokohama-worker`:

- After a `POST …/{id}/input` dispatch that answered 200 and **bumped nothing**, the crate reads
  `games.handle(id)` and `repl.sub` (both public, §0.2). A picker still open appends `(user, line)`
  to the id's list; a closed one clears it. **Any bump clears it** — a move applied, an undo, a
  keep, a delete.
- The list rides out in `Reply.picker` and the shell writes it as the `picker` row.
- On every new instance, after `load`, `restore_picker` re-feeds each line through `api::input` as
  its user, discarding the replies; if a re-fed line bumps, fails, or leaves `sub` closed, the list
  is cleared and the picker starts empty — exactly today's behaviour after a restart, with
  `Input::expect` turning a stale answer into a 409 rather than a wrong move.
- Bounded at 64 lines: past it nothing more is recorded, and the next rebuild starts empty.

**Delete, and the tombstone** (R-6.5c.8, R-6.5c.9). `DELETE /api/games/{id}` runs under
`blockConcurrencyWhile`: the archive's receipt loop (§2.6); then the layer's own `delete` commits and
its `end` bump fires; the shell removes the `docs` rows and writes `tombstone = revision` and a
REMOVAL into `house_slot` in one synchronous batch; the reply is 204; every socket gets `end`. The
alarm then delivers the removal; **only after the house acknowledges it does the object
`deleteAlarm()` and `deleteAll()`**, so nothing a retry needs is lost to the delete. While a
tombstone stands, every route answers the layer's 404 and a `create` at the id is refused, 409, "this
name is still being cleared — try again shortly".

**Alarms.** One per object, re-armed after each run: deliver a pending `house_slot`; complete an
`archive_owed` upload; finish a tombstone. Backoff 30 s, 2 min, 10 min, then hourly, reset by any
success.

### 2.5 The house object

**It is a second layer instance whose store holds SCRUBBED COPIES of table documents and the house
chat.** `GET /api/games?scope=…` is `dispatch` over those copies — the layer's own `Games::list`
(`tables.rs:1556`) and `TableDoc::view` (`table.rs:371`), the code the NAS runs — so a lobby page is
byte-identical by construction and there is no second row type to drift. `/api/chat`, `/api/health`,
`/api/me` and `/api/labels` are `dispatch` too.

**The scrub** (`house_copy`, native Rust): `seed` → 0; a `lock` → a placeholder of zeroed salt and
hash of the stored lengths (so `locked` still reads true and `check_shape` still passes); each
`keeps[i]` → `Some(0)` or `None` (presence only); everything else as stored. Two cells: for every
viewer kind — a seat holder, the creator, a stranger, a stranger on the `unlocked` roster —
`house_copy(doc).view(u) == doc.view(u)`, over tables at every status and lock state; and a VALUE
pin that no house copy carries this run's seed (decimal and hex), password, salt or hash, with the
anti-vacuity partner that the table's own stored document does carry them (M6.2's shape).

**The durable slot.** When a table's `house_copy` differs from the copy last acknowledged, the table
writes `house_slot = {generation, revision, copy | removal}` in the SAME synchronous batch as its
documents, `revision` incremented. `generation` is drawn once, at create, and kept in `meta`; it
exists because `deleteAll()` erases the revision counter with everything else, so a table
re-created at the same id starts counting again. After the reply the table calls the house over RPC;
the house holds `{generation, revision}` per id and applies a push only when its generation is NEW
for that id, or equal with a HIGHER revision — a removal leaves the id's generation and revision
behind as a marker, so a late push of the removed table is ignored while a re-created table's first
push is accepted. It acknowledges; the table clears the slot on the acknowledgement and arms the
alarm on anything else. **So two interleaved pushes cannot regress a row, and a deleted table cannot
return as a ghost**, and the umbrella's three deliveries with no next bump — the create, the final
move, the delete's removal — are simply the slot's contents when the alarm fires.

**The lobby socket** (`GET /api/stream`) is the house's: accepted with the attachment `{ user }`,
sent `{"seq", "chat"}` on every change — `seq` the house's own counter, raised by every accepted
table copy and removal; `chat` the house chat's `last`, raised by `bump_house_chat`. Silent on connect, as the
SSE lobby stream is (`from_changes`).

**Load** (umbrella §2.4's arithmetic, restated for a projection): one push per applied move at most,
and only when a visible field changed; a copy is ~600 B; the house's cold path loads every copy
(a thousand tables ≈ 600 KB) into its `MemStore`.

### 2.6 The archive on R2

**The bucket** `yokohama-archive`, bound as `ARCHIVE`. Keys are `archive::paths(rev, prefix, why,
id)` exactly — `rev7/{over,abandoned}/<id>.json` and `.table.json`, and `browser/<caller_segment>/…`
for a browser's save — so `rclone sync` of the bucket is a directory the CLI loads (R-6.5.4), and
ONE layout function serves the directory and the bucket (O-9; a cell asserts both write the same
relative paths for the same table). `/api/health` reports `archive: "r2"`.

**The receipt** (R-6.5c.9). `ReceiptArchive` implements the layer's `Archive`. Its `put` looks for a
receipt matching the EXACT plan — both paths and the SHA-256 of both bodies; with one it returns the
receipt's outcome; without one it records the plan and returns `PutError::Failed("deferred to the
host")`. The shell carries the plan out: **the table body first, then the record, which is the
once-marker** (the directory archive's rule, `archive.rs`'s trait doc); each a create-if-absent put
(`onlyIf: { etagDoesNotMatch: "*" }`); on "already there" it reads the object and compares bytes —
equal is `Identical`, different is `Conflict`; any other failure is `Failed` with R2's message. The
outcome becomes the receipt.

- **One clock reading per plan.** The layer stamps `archived_at` from its clock INSIDE the table
  body it puts (`tables.rs:1700`; `archive_if_over`), and a Worker's `Date.now()` advances across
  the upload's I/O — so a re-dispatch would stamp a different time, its body's digest would differ,
  and the receipt would never match: an abandon would defer for ever. So the shell records the
  clock reading with the plan and pins the module's clock to it (`set_clock`) for every dispatch
  that drives that plan — the re-dispatch and `stamp_archive` alike — and releases it after. The
  bodies, and therefore the receipt, match byte for byte.
- **Abandon** (R-6.5.5) and **the browser's save** (`POST /api/archive`): under
  `blockConcurrencyWhile`, so nothing interleaves with the upload, the first `dispatch` is
  side-effect-free — `delete` puts BEFORE anything is unlinked (`tables.rs:1655`), and
  `archive_post` puts after its checks and before its reply — and refuses as deferred; the shell
  uploads; **the SAME request is dispatched again with the receipt**, and the layer answers it: a
  delete that proceeds, or its own refusal naming the archive (R-6.5.5's sentence, with R2's reason
  inside it); a 201, a 200 `identical`, or a 409. **Every sentence a client reads is the layer's.**
  A browser's save costs its strict replay twice (≈ 20–30 ms each for 595 events, §0.3). A delete
  that owes no archive — an open or dealing table, one with no events, one already stamped —
  completes in its FIRST dispatch and is never re-dispatched.
- **Game over** is the one site that cannot be re-dispatched — a move is not idempotent. The move
  is answered 200 as today; the layer's own `archive_if_over` meets a deferred put, logs it and
  leaves `archived_at` unset — R-6.5.4's "a retry is owed". The shell writes `archive_owed`, then
  uploads after the reply; on success it installs the receipt and calls `stamp_archive(id)`, which
  is **`Games::archive_over_retry(id)`**: the new public function that runs the existing
  `archive_if_over` against the stored record, whose put now answers from the receipt, so the
  stamp — and its retry on a version conflict — is the layer's own code; the shell then writes the
  stamped table row and clears `archive_owed` in one batch, exactly as after any dispatch. On
  failure the alarm retries. If the game is no longer over when a retry runs (an undo at a one-person table),
  `archive_if_over` does nothing and `archive_owed` clears; a later completion puts afresh and, the
  path holding different bytes, is a `Conflict` the layer logs — the archive keeps the first
  completion, as the directory archive does.
- **One archive object per caller** — `idFromName("archive:" + address)`, the address normalized as
  `User::parse` normalizes it (trimmed, lowercased) — serializes that caller's
  sends and puts the strict replay of an untrusted record (handover §4 item 9) inside an object's
  30 s. Such a record can at worst trap ONE instance of ONE caller's archive object; its storage is
  never written. Access in front, and only the allow-list and the credential reach it.

### 2.7 The client

**One transport arm, and nothing else.** `signalKind()` (`web/src/api.ts:309`) widens from
`"sse" | "held"` to `"sse" | "held" | "ws"`, set from `/api/health` exactly as the M6.3 spec chose.
`usePolled`'s `open()` (`web/src/signal.ts:294`) gains its third arm: a `WebSocket` to the source's
existing `stream` URL (`wss:` for `https:`), whose messages feed the SAME frame handler the stream
reader feeds — `Events` frames, house frames, and `end`. A close with **4403/4404** is the arm's
refusal, handled exactly as the stream's 403/404 is (it ends the source); any other close is a
failure on the existing ladder (`BACKOFF_MS`, `nextDelay`, `Link`, `signal.ts:47-87`), which was
written for a transport with no automatic retry and needs none of its own. Every **30 s** the arm
sends `ping`, answered by the object's auto-response without waking it, which keeps the connection
under the edge's 100 s idle cut (M6.3's heartbeat reason, measured there at 125 s).

The door, single player, the offline release and the service worker are unchanged; the Worker
serves the same `dist` (§0.3 row 7). The NAS keeps `"sse"` and the browser host `"none"`.

### 2.8 The budget

The shared daily quotas are the constraint, and the Worker-request quota binds first: 67k–83k of
headroom a day at this week's usage (§0.4); static assets are free. **Estimate** (to be MEASURED at
the exit): a two-person game of ~400–600 events is ~900 moves' POSTs plus ~2 GETs per bump for each
other screen — **~2,000–4,000 requests**, each one Worker invocation and one object request; so
**~15–30 games a day** in today's headroom. Object requests (100k) are not close: the account uses
1–2%. Rows written (100k/day): ~1–3 per move. R2: saves are ~55 KB; 10 GB and 1 M writes a month.
The exit counts requests per game off the analytics; R-6.5c.6's push is the lever if that count
binds.

---

## 3. Deployment

### 3.1 The infrastructure

- **One Worker, `yokohama-web`**, the custom domain `yokohama.moore.nyc` (it creates the DNS record
  the name now lacks), `workers_dev: false`, `preview_urls: false`, `observability.enabled`,
  **`nodejs_compat` off**, a pinned `compatibility_date`.
- **Three SQLite-backed classes** in one migration: `Table`, `House`, `Archive`.
- **The bucket `yokohama-archive`**, created through the account API (the wrangler token has no R2
  scope, §0.4) and bound as `ARCHIVE`.
- **Variables**: `ACCESS_TEAM_DOMAIN`, `ACCESS_AUD`, `DEV_DOOR_CLIENT_ID`, `YOKOHAMA_DEV_DOOR`
  (`on` during this cycle), and the three chat variables at the NAS's defaults. No secrets.
- **The configuration lives in `worker/wrangler.jsonc`**; the `yokohama` Access application keeps
  its three destinations and three policies unchanged.

### 3.2 The first deploy and after (R-6.5.8)

**The first deploy is the skeleton, and it re-reads the spike with the real code before anything
else lands on it**: the Worker, the three classes, the static assets, identity; then — (1) the
unenrolled protection reading (302 / 403 / `workers.dev` answering nothing); (2) identity on a
WebSocket upgrade as a person over WARP and as the credential by cookie alone; (3) **whether
Playwright's context headers reach a page-opened upgrade** — the one reading the spike did not take,
which decides how the harness's `claimed` identity rides a socket (§5.4); (4) the static-asset
readings of §0.3 row 7, including the service worker's complete install; (5) cold start and CPU
off the tail. Every later task deploys when it changes what the Worker does, and records what it
read.

### 3.3 The runbook, and the retired code

`deploy/RUNBOOK.md` gains **§6, the Worker**: the build (`worker/`'s `build:module` and
`build:assets`), the deploy, the bucket, the variables, the credential's playbook (where it lives,
how the harness presents it, how to rotate it), reading CPU off the tail, Data Studio for a poisoned
table, and each deploy's record with the container-era numbering continued. **§4 is rewritten as the
record of why `deploy/worker/` and `deploy/modal_app.py` were retired** — the hosted charter they
served was superseded by the umbrella (§0.5 there) — and both are deleted (R-6.5.13), the strip rule
carried out first (§2.3). §0–§3 and §5 stay: the NAS stays.

### 3.4 The NAS

Unchanged: `yokohama-nas.moore.nyc`, `claimed`, SSE, the directory archive (R-6.5.1). The cleanup
cycle after M6.5c (R-6.5c.4) is where the two hosts are reconciled.

---

## 4. The wire and the corpus

**R-6.5.9: every payload a host SERVES is byte-identical to the committed one**, except the fields
its spec names. For this host:

- **The second class — host-parameterized fields, named, not exempted silently**: `health.json`'s
  `identity` (`"cf-access"`), `signal` (`"ws"`) and `archive` (`"r2"`); `me.json` and
  `me_signed_out.json`'s `mode` (`"cf-access"`). The replay asserts each named field really DIFFERS
  on this host, the M6.5b battery's lesson (an exemption that exempts nothing hides a regression).
- **`GET /api/art`** is served as the static manifest's bytes; the axum route re-serializes the
  same manifest. The replay compares it by VALUE (the M6.5b2 cell's rule for the two sources) and
  names it here; no client has read the route since M6.5b2.
- **The three transport payloads** (`events.json`, `events_open.json`, `events_dealing.json`) are
  served by this host (the axum host's router serves them; the browser host does not) and must be
  byte-identical.
- **No new payload and no changed payload.** The socket carries existing frames; the only new wire
  words are the close codes 4403/4404, which carry the layer's sentences as reasons.

---

## 5. The tests

### 5.1 Rust — `cargo test -p yokohama-worker` (per-edit, native)

- The factory's premise, in Rust terms: two `init`s in one process are two `Games` only because
  the module is instantiated twice — so the Rust cells cover the `_inner` functions and the
  FACTORY cell lives in the pool (§5.2).
- `RecordingSignal`: every layer bump site's call arrives in the reply, in order, and nothing is
  recorded for a refused request.
- `ReceiptArchive`: no receipt → the plan and `Failed("deferred…")`; a receipt for the exact plan →
  its outcome; a receipt whose bodies' digests differ → treated as absent.
- The picker: append on an open picker with no bump; clear on a bump and on a closed picker; the
  64-line bound; `restore_picker` rebuilding a `PlaceA` picker mid-selection so the next line
  commits the move the player intended (umbrella §4's H-4 acceptance test, natively); a re-feed that
  would bump clears instead.
- The scrub: `view` equality for every viewer kind at every status and lock state; the value pin
  and its anti-vacuity partner.
- `create`: an absent id filled; a different id refused; a `u64` seed in the body survives exactly.
- The chat variables: defaults, a bad value refused naming it.
- The crate's platform scan (`yokohama-wasm`'s `this_host_makes_no_platform_call_but_the_clock`
  shape).
- **In the layer, ONE cell for the ONE addition**: `archive_over_retry` stamps a finished game whose
  first put failed, does nothing for a stamped or unfinished one, and retries its stamp on a version
  conflict — over the existing test archive and store.

### 5.2 The shell — Cloudflare's vitest pool (per-edit, `worker/`)

`@cloudflare/vitest-pool-workers`, running inside workerd with real SQLite, alarms
(`runDurableObjectAlarm`), WebSockets and a local R2, over the BUILT module. Cells:

- **Routing**: every row of §2.3's table, including the Worker's own 404 and `/enter`.
- **Identity** (a pure function, fed crafted assertions and a test key set): a person; the
  credential with the door on and off; an assertion for another audience, an expired one, a forged
  signature; **every client `x-yokohama-*` header stripped**, including `x-yokohama-claimed-user` —
  the header `deploy/worker/` missed.
- **The factory**: two instances share nothing; a trapped instance leaves another usable; a
  re-instantiated one reloads from SQLite.
- **The table object**: probing an unknown id leaves storage EMPTY; a spectator's move is refused
  before the record is read; the picker survives a fresh instance over the same storage; a trap
  persists nothing; two load-traps yield the named poison 500; the tombstone's 404s and its create
  refusal; `deleteAll()` only after the house's acknowledgement.
- **Sockets**: the first frame on accept; one frame per socket per bump, each for its own viewer;
  4403/4404 closes carrying the layer's sentences; `end` on delete.
- **The house**: a lower revision after a higher one leaves the row at the higher; after a removal, a
  late push of the REMOVED generation is ignored while a re-created table's first push (a new
  generation, revision 1) is accepted; a failed push is retried from the slot by the alarm until
  acknowledged; a listing page equals the committed fixture.
- **The archive**: abandon with a failing put leaves the object and its storage in place with the
  layer's refusal; abandon with a landing put deletes; a browser save answers the layer's
  201/200/409; a chat post arriving during a slow house push or R2 put lands (the input gate opens,
  and the request's own writes are already done).

### 5.3 The corpus against the objects (per-task in the pool; ONCE live at the exit)

The walk's request script (`web/src/wire-script/walk.json`, 2,197 rows, 74 named) is replayed with
its statuses asserted and every named row deep-equal to its committed file, §4's second class
excepted and each exemption proven to exempt something.

- **In the pool**, against the objects with `X-Yokohama-User` set as the Worker would set it, and
  the module clock frozen at the corpus's `1_789_000_000` — the node replay's arrangement (M6.5b
  spec §6.3), one host further.
- **Live, at the exit**, through the Worker with the credential and the dev door, the created ids
  (`wire`, `wire2`, …) cleared before and deleted after so the owner's lobby is left as it was; the
  clock cannot be frozen live, so `created_at` and the page cursor are normalized as
  `tests/wire_fixtures.rs`'s `freeze_clock` normalizes them before comparison.

### 5.4 Playwright — the multiplayer specs against the LIVE Worker

`two_browsers.spec.ts` and `joins_by_link.spec.ts` gain a configuration that targets
`https://yokohama.moore.nyc` (`YOKOHAMA_WEB_E2E_TARGET=live`): each context presents the credential
and its own `X-Yokohama-Claimed-User` as context headers. **Whether those headers reach a
page-opened WebSocket upgrade is §3.2's third reading**, and it decides the one open mechanism: if
they do, nothing more is needed; if they do not, the Worker binds the claimed identity to an
HttpOnly `yokohama_dev_as` cookie on the first credentialed request that carries the header, honoured
only beside the credential's assertion. Every other spec stays on the local axum binary, unchanged;
the two specs keep their local configuration too.

### 5.5 The exit (umbrella §3.3 — numbers, not a demonstration)

On the live Worker: two people play a whole game and a third watches, HEADED; the table object
**observed hibernating** between moves (`wakes` rising across pauses, read from its own storage);
the move-to-other-screen wall measured as M6.3 measured it through the tunnel, with the cold-path
CPU beside it from the tail; a finished game and an abandoned one in the bucket, `rclone`-synced and
**loaded in the CLI with `why` reconstructing a bot move**; the whole corpus byte-identical live
(§5.3); requests per game counted off the analytics (§2.8).

### 5.6 Where each tier runs

Per-edit locally: §5.1, §5.2 and §5.3's pool replay, beside `CLAUDE.md`'s existing web gates.
**CI runs §5.1 and §5.2** — the `web-server` job builds the worker module as it builds the browser's
and runs both — and the lint gate covers the new crate as a workspace member. **§5.3-live, §5.4 and
§5.5 stay local**, because they need the credential; written into the job, as the e2e's locality is.
Any edit to a workflow file gets the `yaml.safe_load` parse line in the gate (handover §4 item 12).

### 5.7 The mutation battery (at the reviewed head, each reverted after its run)

Umbrella §4's M6.5c list, and what the spike added. Each must be CAUGHT by a named cell:

1. a spectator's move reaches the record (§5.2);
2. a client `x-yokohama-claimed-user` (or `x-yokohama-user`, or `?as=`) honoured beside a person's
   assertion (§5.2);
3. the house copy keeps the hash, the salt or the seed (§5.1 value pin);
4. a frame sent before `storage.sync()` (the gated arrangement: a write held, the socket read QUIET
   at the instant a hoisted send would be loud — M6.3's shape);
5. the house accepts a lower revision (§5.2);
6. the slot cleared before the acknowledgement (§5.2);
7. the corpus drifts on this host (§5.3);
8. the wake counter stops counting (§5.2 — a counter that MUST rise across a pause);
9. the picker not re-fed after a fresh instance (§5.1, §5.2);
10. a failed abandon put deletes anyway (§5.2);
11. `deleteAll()` before the house acknowledges (§5.2);
12. the factory transform bypassed — the stock glue, one instance per isolate (§5.2);
13. `html_handling` back to its default, so `/index.html` redirects (§5.2 static cell, and the live
    install reading);
14. `not_found_handling: "single-page-application"`, so a missing asset answers HTML (§5.2);
15. the dev door honoured with no credential assertion (§5.2);
16. probing writes storage (§5.2);
17. the receipt loop's clock left unpinned, so the re-dispatch defers again (§5.2's abandon cells);
18. the house applying an older generation's push after a removal — a ghost row (§5.2).

---

## 6. What this spec does NOT claim, and the LEAVE list

- **Nothing is public** (R-6.5c.1). The zero-invocation single-player door is a PROPERTY of the
  asset configuration, read off the tail by the spike, and not a public service; the path-scoped
  Access application of R-6.5b2.2 is still built-and-proven-locally, not deployed; the art is not
  published.
- **`claimed` is not retired.** It is the NAS's mode, and on the Worker it is the development door
  behind the credential. A holder of the credential can claim any identity at any table, including
  the owner's — the trust the NAS gives every allow-listed person today, narrowed to one secret.
- **The CPU assumption rests on one measurement** (376 ms, `ok`) and the owner's experience; the
  documentation contradicts itself, and a cycle that meets `exceededCpu` in the logs reopens it.
- **Hibernation's wake cost grows with the record.** Measured at 595 events; the e2e's own games
  reach 1,700 applied moves, which is an estimate of ~3× until the exit reads it.
- **The untrusted record on `POST /api/archive`** (handover §4 item 9) is BOUNDED — one instance of
  one caller's archive object, behind Access — not closed.
- **A poisoned table is named, not repaired.** Recovery is Data Studio (RUNBOOK §6).
- **Push (R-6.5b.11) is not built**; its trigger is the exit's measured count.
- **The zone is not changed.** Web Analytics auto-install logs a CORS error on every page load, and
  JS detections and Bot Fight Mode stay on: these are the owner's zone settings (§7).
- **`wasm-opt` 133 on this workstation against CI's 132.** The deployed module is built at 132 in the
  plan's pipeline; the spike's 133 figures are the spike's.
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
| 6 the automatic upload | DECIDED, R-6.5c.11 |
| 7 the "Lobby" controls under a single-player build | CARRIED to the public-door step (this Worker offers multiplayer) |
| 8 the SPA-fallback cells | TAKEN, R-6.5c.10, cells 13–14 of §5.7 |
| 9 the untrusted `GameRecord` | BOUNDED, §2.6 |
| 10 no-Web-Locks, background suspension | CARRIED |
| 11 the §6.1 measurements | MEASURED by the spike (O-1, O-2, O-3, O-6, O-7, CPU); a whole bot turn and `why` live at the exit |
| 12 the CI workflow parse check | TAKEN as a plan gate: any workflow edit gets the `yaml.safe_load` line |
| 13–18 | CARRIED as recorded there (18's items are §6's inherited list) |
| 19 R-F6 not measured live | CARRIED to the owner: an Access re-login in a browser the worker controls needs a non-WARP person |
| 20–25 the fix wave's residuals | CARRIED |
| 26 the TDD commit shape | ASKED at the plan |
| 27 the edge rewrites `index.html` | EXPLAINED by the spike (§0.3 row 8); the zone settings are OWNER items |

**Owner items this spec adds**: the zone's Web Analytics auto-install (off for this hostname, or
accept the console error — a "zero console errors" live review needs a decision), JavaScript
detections and Bot Fight Mode (they did not challenge the harness, §0.3); the bucket's creation if
the account API is not used.

---

## 8. Read order for the session that writes the plan

1. This document, §1, §2, §5.
2. `docs/temp/m6.5c-spike-readings-2026-09-22.md` (gitignored; §0.3's source) — and the scratchpad
   spike source if it survives, as the shape the shell's first cut can start from.
3. The umbrella §1 and §4; the M6.5b spec §2.1–§2.2 (the browser host this module is the sibling
   of) and §6.3 (the node corpus replay §5.3 extends).
4. `CLAUDE.md`'s gate commands; re-count at your own HEAD.
