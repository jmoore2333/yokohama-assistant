# Web frontend — M6.5 "the hosting move": the spec

**Written 2026-09-19 on branch `web-room-m6.4` at `faf1a88` (M6.4's close HEAD), from the
brainstorm held the same day between the owner and the orchestrating session; landed on branch
`web-room-m6.5` (stacked on `web-room-m6.4`) at `2f5381a`, and corrected once by the adversarial
read of that commit (4 HIGH, 11 MEDIUM, 12 MINOR, 10 OPEN, 71 claims verified true) together with
the cut-surface survey — the fix-wave commit that follows `4051b18`.** M6.5 is the fifth and last
cycle of the room plan (`docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §2) and its entry
artifact is `docs/plans/2026-09-19-web-frontend-m6.5-hosting-handover.md`. **This document
supersedes the handover's charter** (§0.5 says exactly where) and is the authority for the ORDER
and the SEAMS of M6.5; it is an umbrella of THREE sub-cycles, each of which enters through its own
spec and plan under R-P1 and R-P11, the way the room plan was the authority for M6's five. The
first sub-cycle's spec, M6.5a, is written after the owner has read this one.

The change of direction, in one paragraph. The handover's charter moved the room onto a
container (Modal or Cloudflare Containers) fronted by a Worker, with Postgres behind the store
trait and a Durable Object notifier the origin pokes. The brainstorm measured the owner's hint
instead — that a pure-Rust engine compiles to WebAssembly — and found that the engine, the
production bot AND the whole seam the web crate consumes compile to `wasm32-unknown-unknown`
today with no change to any fenced crate (§0.3). On that fact the container disappears: hosted
multiplayer runs inside Durable Objects, where one object per table is the store, the lock and
the signal at once; single-player runs entirely in the browser with no server; and the axum
binary in Docker stays exactly as it is, as the self-host path this cycle does not invest in.

---

## 0. What is true at HEAD (`faf1a88`; byte-identical at `2f5381a` and `4051b18`, which are docs-only)

Every citation below was opened at that HEAD by the adversarial read. Re-read each at the HEAD a
sub-cycle opens; four cycles running, something had moved every time.

### 0.1 The three seams as built

- **Identity** (`crates/yokohama-web/src/identity.rs`): `IdentityMode` (`:32`) is `Claimed`
  | `CfAccess` | `Worker`, read from `YOKOHAMA_WEB_IDENTITY` (the literal at `:29`, read by
  `from_env` at `:55`) with no default. Each mode reads exactly one header (`:47-52`);
  `resolve` (`:127`) is the pure half and the axum extractor — `pub struct Identity(pub User)` at
  `:149`, its `FromRequestParts` impl at `:151` — the transport half. `User::parse` (`:92`) trims
  and lowercases once. Under `worker` the literal `anonymous` counts as absent (`:132`).
- **Store** (`crates/yokohama-web/src/store.rs`): `trait Store: Send + Sync` (`:117`) — `list`
  `:119`, `read` `:124`, `write_if_version` `:129`, `delete` `:138` — over `Doc` (`:36`, four
  variants: `Record`, `Table`, `Chat`, `House`); `Version` is a content digest (`version_of`,
  `:84`); `RESERVED_ID_SUFFIXES = [".table", ".chat"]` (`:157`); `is_safe_id` (`:169`);
  `check_house_id` (`:200`); `FsStore` (`:208`, temp + rename, no lock, `std::fs` throughout and
  `std::process::id()` at `:286`) and `MemStore` (`:398`). **The trait is synchronous.**
- **Signal** (`crates/yokohama-web/src/signal.rs`): `Signals` (`:48`) over
  `tokio::sync::watch` (`:21`) — `Bump { seq, end }` per table (`:26`), `House { seq, chat }`
  (`:42`). The eight bump sites, every one in a success arm: `persist` `games.rs:543`; `create`
  `tables.rs:769`; `chat_append` `:951`; `house_append` `:965`; `flip_to_playing` `:1255`
  (which is `finalize`'s bump too); `join` `:1402`; `keep` `:1504`; `delete` `:1693` (`end`,
  the terminal bump — so SEVEN sites send a frame and the eighth sends `end`).
  `GET …/{id}/stream` and `GET /api/stream` sit in front (`lib.rs:1931`, `:1993`);
  `SIGNAL_KIND = "sse"` (`lib.rs:240`); `HEARTBEAT` 25 s (`lib.rs:248`).

### 0.2 The per-process state, and what an actor does to each

The handover §2.3 enumerated TWELVE pieces of state a second replica breaks, read off `Games`
(`games.rs:110-186`) and `AppState` (`lib.rs:255-271`). On the path this spec chooses there is
no second replica of a table: a Durable Object is a single-threaded actor and every request for
a table id reaches the same object. Row by row, all twelve:

| what | at HEAD | inside a table object |
|---|---|---|
| `live` (`Repl` cache, bounded 64) | a miss is a replay | the one in-memory `Repl`; a miss (wake) is the same replay — see §6.1 O-3 for its cost and H-4 for what it drops |
| `written` (CAS base) | detects a cross-process conflict | no second writer exists; retired |
| `deleted` (tombstones) | a local delete's guard | `deleteAll()` ends the object; retired |
| `table_lock` (global create) | serializes id draws | the id is drawn by the Worker's create handler BEFORE the object is addressed (§2.2); create-if-absent runs inside the object |
| `locks` (per table) | R-2.9's exclusion | the object's input gate |
| `signals` | one process's memory | the object's WebSockets |
| `chat_last` (per-process count) | WRONG with two processes | a `MAX(seq)` over the object's own rows |
| `house_lock` | serializes house appends | the house object's input gate |
| `limits` / `AppState.chat` | launch variables, read once | Worker configuration, one place, every object reading the same values |
| `AppState.identity` | a launch variable, no default | Worker configuration |
| `AppState.heartbeat` | **a field for testability, not configuration** — every production constructor sets `HEARTBEAT`; it exists so a test can build a 50 ms cadence | not carried: a hibernating socket needs no heartbeat, and the client's ping is answered at the edge (§2.3) |
| `TableDoc.unlocked` (`table.rs:211`) | stored, capped at 256, never serialized | a stored roster still; it survives a replica today and an object tomorrow |

### 0.3 What compiles to WebAssembly today (measured 2026-09-19, throwaway crate, not in the tree; reproduced byte-exactly by the adversarial read)

- `cargo check -p yokohama-core --target wasm32-unknown-unknown` fails on exactly ONE thing:
  `getrandom` 0.2.16 refuses the target without its `js` feature (the transitive dependency of
  `rand`, reached by `OsRng` inside `from_os_entropy` at `game/seeds.rs:106-108`). **Enabling
  that feature from a downstream crate unifies onto core's dependency, so core's `Cargo.toml`
  stays byte-identical.** The version is load-bearing: `getrandom` 0.3 renames the feature and
  needs a `RUSTFLAGS` cfg, so a lock bump here is a build decision.
- **The CLI's library half compiles to the target unchanged.** `rustyline` 18.0.1 with the crate's
  own `default-features = false, features = ["buffer-redux"]` (`crates/yokohama-cli/Cargo.toml`)
  selects its TEST terminal on the target (`tty/mod.rs:243`, `#[cfg(any(test, target_arch =
  "wasm32"))]` — a stub, not a designed mode; nothing here calls it), and `yokohama-bridge`'s
  `std::net` compiles (it is never called). No feature flag, no crate split.
- Sizes, `opt-level = "z"`, `lto = true`, `panic = "abort"`, no `wasm-opt` (not installed):

  | linked | raw | gzip | brotli |
  |---|---|---|---|
  | `Rules::standard`, `new_game`, `finalize_setup`, `get_legal_moves`, `apply_move`, `Contestant::PRODUCTION.build` | 744 KB | 213 KB | 161 KB |
  | plus `Session::new_sim`, `Repl::new`, `Repl::step` (whose dispatch reaches `suggest`, `why` and the whole render stack), `to_record_json` | 1.30 MB | 387 KB | 289 KB |
  | plus `screen_model::build_frame`, the web host's principal projection | 1.31 MB | 388 KB | 290 KB |

  The third row is the figure a host bundle carries; `build_frame` adds 3 KB because `step`
  already reaches the frame builders. LTO drops what no root reaches, so a row measures its
  reachable set and nothing more.
- The production bot, native release, `examples/arena -- cell … 2 v4-step9-draft1
  v4-step9-draft1 1 --master 20260919 --no-invariants`: **1,538 µs and 1,483 µs per decision**
  over 220 and 214 decisions (the counts are deterministic in the master and reproduce exactly;
  the walls are load-sensitive — the re-run read 1,672 / 1,576 µs); **328.5 ms per whole 2P
  game with BOTH seats played by the production contestant**, so a hosted table with one bot
  seat is about half that.
- Runtime hazards on the target, in the web crate and in no fenced crate. Two PANIC:
  `table::now_unix` (`table.rs:417-422`) calls `std::time::SystemTime::now()`, which panics on
  `wasm32-unknown-unknown` (its two callers are `tables.rs:746`, a table's `created_at`, and
  `:913`, a message's); and tokio's `time` module, which compiles for the target and panics
  when used. Two ERROR rather than panic: `FsStore`'s `std::fs` and the `std::env` reads in
  `chat.rs:145-147`, `identity.rs:56` and `store.rs:469-471`, which compile and return errors —
  §2.1 puts them behind the `server` feature so the wasm build carries no code that can only
  fail. `signal.rs:21`'s watch channel compiles and is the seam an object replaces.
  `tables::draw_seed` (`tables.rs:213`) goes through `SeedSchedule::from_os_entropy`, which
  `getrandom`'s `js` feature serves from `crypto.getRandomValues` in a browser (proven by the
  spike) and, unproven until §6.1 O-7's spike, in a Worker.
- The web crate itself: of its 19,463 lines, only `lib.rs` (the routes), `identity.rs`,
  `art.rs`, `assets.rs` and `main.rs` mention axum. `games.rs`, `tables.rs`, `chat.rs`,
  `table.rs`, `store.rs`, `signal.rs`, `client.rs`, `explain.rs`, `outcome.rs` and `grammar.rs`
  do not — and `client.rs`, `explain.rs`, `outcome.rs` and `grammar.rs` are pure over `Repl`
  and the documents, with no platform call at all. **Two thirds of each of the three big files
  is `#[cfg(test)]`**: the logic to move in M6.5a is about 2,400 lines and the tests that ride
  on it about 6,800 (the survey's `wc` table). `Cargo.toml` enables SIX tokio features —
  `rt-multi-thread`, `macros`, `net`, `signal`, `sync`, `time` — of which `net`, `signal` and
  `rt-multi-thread` do not build for the target (they pull `mio`, 48 errors) while `sync`,
  `time` and `macros` do, as do `tokio-stream`'s `sync` and `sha2`. **A compile-time check
  therefore proves the absence of axum and of mio-backed tokio, and nothing else** (H-1).

### 0.4 The platform, from the docs (all read 2026-09-19; every figure re-fetched by the adversarial read)

- **Durable Objects run on Workers Free and Paid**; since 2026-07-09 a NEW namespace must be
  SQLite-backed (conditional: an account that already holds a key-value namespace may still
  create them); 10 GB per object on Paid, and the Free plan's storage row is 5 GB in total; SQL,
  point-in-time recovery to any moment in the past 30 days.
  (`developers.cloudflare.com/durable-objects/platform/pricing/`, `/reference/durable-objects-migrations/`)
- **Retention is indefinite.** "A Durable Object fully ceases to exist if, when it shuts down,
  its storage is empty"; once anything is written, only `deleteAll()` removes it.
  (`/durable-objects/best-practices/access-durable-objects-storage/`)
- **Lifecycle**: active → idle in memory (hibernatable after ~10 s when only hibernatable
  sockets are open) → hibernated (memory dropped, WebSockets stay connected at the edge) →
  inactive (evicted 70–140 s after the last event when not hibernatable). Any request or
  socket message re-runs the constructor. Hibernation applies only to WebSockets the object
  SERVES; a held HTTP response keeps it awake. Duration is not billed while hibernatable;
  `setWebSocketAutoResponse` answers ping/pong without waking the object;
  `serializeAttachment` survives hibernation. **In-memory state is reset at every hibernation**
  — see H-4 in §6.1. (`/durable-objects/concepts/durable-object-lifecycle/`,
  `/durable-objects/best-practices/websockets/`, `/durable-objects/api/state/`)
- **Output gates** hold "outgoing network messages (responses, fetch requests)" until pending
  storage writes complete — a reply is durable by construction; a WebSocket `send()` to a third
  party's socket is not named by the gate. (`/durable-objects/best-practices/rules-of-durable-objects/`)
- **Limits**: 128 MB per isolate; Worker bundle 64 MiB UNCOMPRESSED on both plans since
  2026-09-04 (the 3 MB / 10 MB compressed limits are gone); **Worker startup time 1 second**
  ("parse and execute its global scope within 1 second; larger bundles … increase startup
  time" — the one limit a 1.31 MB module is plausibly near, §6.1 O-2); CPU per request 30 s
  default on Paid, raisable to 5 min; the plain-Workers Free row reads 10 ms and neither the
  limits page nor the DO pricing page carries a Durable-Object-specific Free figure. Free plan:
  100,000 DO requests/day, 13,000 GB-s/day; Paid: 1 M requests and 400,000 GB-s a month
  included. WebSocket messages count as requests at a 20:1 inbound discount.
  (`/workers/platform/limits/`, `/durable-objects/platform/pricing/`)
- **Access, two kinds, and the difference is load-bearing (H-3).** Worker-level Access
  (2026-08-14) gives `ctx.access.getIdentity()` with no JWT parsing and an `access.dev` block
  for `wrangler dev` — but its own limitations section says Worker-level policies "do not
  currently support WebSocket connections" and recommends a hostname-based application for
  Workers using WebSockets, including those with Durable Objects; a Worker serving Static
  Assets is not passed `ctx.access`; and `ctx.access` does not propagate across service
  bindings or RPC. A hostname-based (self-hosted) Access application sets
  `Cf-Access-Jwt-Assertion` and `Cf-Access-Authenticated-User-Email` on every request that
  passes it. (`/workers/configuration/cloudflare-access/`)
- **workers-rs** `worker` 0.8.5: `DurableObject` trait with `fetch`, `alarm`,
  `websocket_message`/`websocket_close`/`websocket_error`; `#[durable_object(websocket)]`;
  `State::accept_web_socket`, `get_websockets`, `set_websocket_auto_response`;
  `WebSocket::serialize_attachment`/`deserialize_attachment`; `SqlStorage` on
  `state.storage().sql()`. `worker::Context` is `new`, `wait_until`,
  `pass_through_on_exception`, `props` — **no `ctx.access`**. No tokio runtime ("leave your
  threaded async runtimes at home"). (`docs.rs/worker/0.8.5`)
- **R2**: 10 GB-month, 1 M Class A and 10 M Class B a month free; zero egress; Workers
  binding, S3-compatible API, `wrangler r2 object get`. (`/r2/pricing/`, `/r2/api/`)
- **Data Studio** (beta 2025-10-16) reads and writes a named SQLite-backed object's SQL data
  from the dashboard, `Editor` access on the implementing Worker, audit-logged. This session
  found no CLI path to an object's data; that is an inference from the docs read, not a
  sentence in them. (`/durable-objects/observability/data-studio/`)
- **Containers** are Paid-only, 1–3 s cold starts, fronted by a Durable Object, scale to zero by
  `sleepAfter`. Recorded because the charter named them; this spec does not use them.
- **Local development**: `wrangler dev` persists object state under `.wrangler/state/`.

### 0.5 What this spec supersedes

- **The handover §1 charter**: no Postgres store, no Modal, no Cloudflare Containers, no
  origin-poked notifier, no `worker` identity mode behind a Modal proxy token. `deploy/worker/`
  and `deploy/modal_app.py` are written-and-never-run code for a path this spec retires; M6.5c
  removes them (R-6.5.13) rather than leaving two hosted designs on disk.
- **Room plan R-P5's letter**, by owner ruling R-6.5.2 below; its intent stands.
- **The M6.3 spec §7 notifier design** as a design: its message shape (`Events`, five scalars,
  R-6.4.7's house `{seq, chat}`), its drop-and-recover rule and the client selecting the reader
  from `/api/health` are KEPT; the poke, its secret and the separate object are not needed when
  the origin IS the object.
- **The handover §5's eight leans**: 1 (the schema and what a `Version` becomes), 2 (the
  notifier's shape), 3 (Modal or Containers) and 6 (the caches) are answered by §2; 4 (the
  roster and the password oracles) and 5 (house-chat retention and moderation) are LEAVE items
  on this door by R-6.5.3 and named in §6; 7 (`claimed`) is R-6.5.11; 8 (the exit measurement)
  is §3.3.

---

## 1. What the owner ruled on 2026-09-19 (the settled inputs)

Owner direction. A sub-cycle may ask to revisit one; none is re-litigated inside a sub-cycle.
Numbered `R-6.5.n` — `R-5.n` already means M6.1's undo rulings in the code and `R-6.4.n` is
M6.4's, so the namespace is checked, as the M6.4 close asked.

- **R-6.5.1 — the hosts.** Hosted multiplayer runs on Cloudflare Workers and Durable Objects.
  Single-player runs entirely in the browser as a PRIMARY channel, served from the same Worker
  but executing in WebAssembly with no server round-trip. **The axum binary in Docker stays
  exactly as it is** — the NAS deployment, the self-host path, the development environment for
  the server logic — neither invested in nor removed this cycle; a self-hosted multiplayer
  answer beyond it is revisited later. `wrangler dev` is NOT offered as a self-host path (the
  owner: not an elegant solution), and "browser-only, bring your own Cloudflare for
  multiplayer" is not the endpoint either, though the browser channel may well be the primary
  one.
- **R-6.5.2 — R-P5 amended.** The INTENT stands: state never lives in compute that can be
  thrown away. The LETTER changes: a table's documents live in its Durable Object's SQLite, the
  lobby's index in a house object, and there is no Postgres in this lane. Keeping a database
  beside the objects would re-introduce the cross-process conflict machinery the actor model
  removes.
- **R-6.5.3 — the door.** The hosted deployment sits behind **a hostname-based (self-hosted)
  Cloudflare Access application** with the owner's allow-list, as today — hostname-based and
  not the Worker-level feature, because the transport is WebSockets and §0.4 records that
  Worker-level Access does not support them. Other identity methods may be added later; with
  Access available the cycle focuses on the architecturally interesting and baseline factors.
  Consequently the email roster on the wire (M6.3 §6.3 item 1, R-6.4.2), the `unlock`/`join`
  password oracles and house-chat moderation (R-6.4.14) stay LEAVE items on this door and are
  named in §6.
- **R-6.5.4 — the archive.** Collecting save games matters. A finished game is written to an R2
  bucket by its table object at game over, in the FILE STORE's own layout (`<id>.json` the
  record, `<id>.table.json` the table document) under a `SEMANTICS_REV`-keyed prefix, so an
  `rclone sync` of the bucket is a directory the CLI loads. Chat is excluded. A game id is
  archived AT MOST ONCE (over, or abandoned — R-6.5.5), so there are never versions to
  reconcile on the training side; an on-demand "save to R2" click was considered and rejected
  for exactly that housekeeping. The R2 binding's presence is the switch, reported on
  `/api/health`; **a failed archive at game over is logged and retried and NEVER refuses a
  move** — the object's own storage is the durable copy. The retry has a mechanism on each
  host and they differ: on an object it is an alarm; on the axum host, which has none, it is
  the next abandon of that table or the operator's hand, and `archived_at` staying unset in
  the table document is what says a retry is owed. A later pipeline (Databricks was
  named) is out of scope; the layout is chosen so it can read the bucket. **On the axum host
  the archive is a DIRECTORY** beside the store in the same layout (`YOKOHAMA_WEB_ARCHIVE_DIR`,
  absent = off, reported on `/api/health` the same way), so the NAS collects saves during
  development from M6.5a on, and the bucket in M6.5c is the second implementation of one
  trait — with ONE shared layout function and a cell asserting both implementations write the
  same relative paths for the same table (§6.1 O-9).
- **R-6.5.5 — abandon and save.** The table creator's existing two-click Abandon (M6.2 ruling
  #13; `web/src/Abandon.tsx`; `DELETE /api/games/{id}`, `lib.rs:1049` → `tables.rs:1648`)
  ARCHIVES the table under an `abandoned/` prefix whenever the archive is on and the table has
  at least one event, then deletes it. One control, its label telling the truth from health
  and state; an open or dealing table with no events still just deletes. **The delete AWAITS
  the archive and is REFUSED, with a sentence naming the archive, when the put fails** — the
  save is the point of the control, and on an object a `deleteAll()` racing an in-flight put
  would lose it silently along with the alarm that would have retried it (§6.1 O-4); the
  creator retries. During development this is how saves are collected without playing to
  completion.
- **R-6.5.6 — single-player is entirely local.** The record lives in the browser (IndexedDB);
  a local game never appears under "your games" on the server and needs no sign-in beyond the
  door. "Save game file" is a plain download at game over on BOTH channels. When the archive
  is on, "Send to the archive" posts the record to the Worker, which does not trust it: it
  applies THREE separate checks — `Session::from_record`'s own `semantics_rev` gate, then
  `yokohama_core::replay_strict` (the rules hash, every event through the checked boundary,
  the terminal digest; `pub` at `game/record.rs:478`, reachable from the web crate and called
  by nothing in it today), then a game-over check that neither of the first two makes — and
  only then writes it under a `browser/<user>/` prefix.
- **R-6.5.7 — the order.** M6.5a the seam, M6.5b single-player in the browser, M6.5c the room
  on Durable Objects. Each its own brainstorm → spec → plan → execution under R-P11. The
  alternative (c before b) reaches "online" sooner but stacks the transport work on wasm risks
  that b retires cheaply first.
- **R-6.5.8 — test in the live setup during development.** The owner's experience is that
  `wrangler dev` and production differ in ways that matter. M6.5c deploys to the real account
  behind Access EARLY and repeatedly, not only at the final review — the site is private and
  the only players are the owner's — so a bad assumption is met before the review rather than
  at it. **The first thing the first deploy tests is an Access-protected WebSocket upgrade
  carrying the caller's identity** (§6.1 O-6), because every design sentence in §2.3 and §2.6
  rests on it; the second is a cold start of the 1.31 MB module against the 1 s startup limit
  (O-2).
- **R-6.5.9 — what stays fixed** (room plan §3, restated with what §0.3 measured): the five
  fenced crates byte-identical; `SEMANTICS_REV` 7; the CLI UNTOUCHED (it need not be touched:
  its library half compiles to the target as-is); the server ROUTES, SCOPES and SERIALIZES and
  computes nothing about the game; no bot behaviour moves; **the wire corpus is the contract
  across ALL THREE hosts: every payload a host SERVES is byte-identical to the committed
  one**. The axum server and the object serve the whole corpus; the browser host serves the
  single-player subset, which the M6.5b spec names — a quarter of today's corpus (the lobby,
  join, unlock, spectator and house-chat payloads) has no meaning where there is one tab and
  one game. A host that cannot reproduce the payloads it serves is not done. No count rides in
  this ruling; the corpus grows in M6.5a.
- **R-6.5.10 — the export route.** `GET /api/games/{id}/record` serves the record's bytes to a
  seat holder ONLY once the game is over, because the record carries the setup seed, hidden-set
  member #1, until the last move. Before game over it is a 409 in this API's error shape.
- **R-6.5.11 — `claimed` is kept as the development door** on every SERVER host, and the hosted
  Worker refuses BOTH `X-Yokohama-Claimed-User` and `X-Yokohama-User` from the client — the
  rule, not a copied list: `deploy/worker/src/index.ts:71`'s `CLIENT_MUST_NOT_SET` strips the
  second and not the first, and under `claimed` a forwarded first would be impersonation of any
  seat at any table. On the hosted path the identity mode is `cf-access`, the header the
  hostname-based Access application sets on every request that reaches the Worker through it.
  `ctx.access` is recorded as UNAVAILABLE on this shape (§0.4: not on WebSockets, not behind
  Static Assets, not in workers-rs 0.8.5) rather than as a pending upgrade.
- **R-6.5.12 — the plan.** The owner's deployment runs on Workers Paid (30 s CPU per request,
  1 M object requests a month included). Human-only tables would fit the Free plan; a hosted
  bot seat is not sized for it (§0.3's figure is for two bot seats and a hosted table has one,
  which halves the cost and does not change the ruling) and this spec does not claim it.
- **R-6.5.13 — the retired code goes.** M6.5c deletes `deploy/worker/` and `deploy/modal_app.py`
  and rewrites `deploy/RUNBOOK.md` §4 as the record of why they were retired; the NAS sections
  §0–§3 and §5 stay, because the NAS stays. The one thing carried out of `deploy/worker/`
  before it goes is its strip rule, widened as R-6.5.11 says.

---

## 2. The architecture: three hosts over one layer

### 2.1 The layer, and where the cut runs

**The table layer** is one table's logic, transport-neutral, no axum, no mio-backed tokio, no
clock of its own: the three documents (`table.rs`, `chat.rs`, the record), admission
(`lib.rs:506`'s `admitted` and `:563`'s `spectators_write_nothing`), the viewer projection
(`client.rs`), the move path over `Repl::step` behind `grammar::is_gameplay_line` (which is
also what keeps `Repl::step("save …")` and its `std::fs` unreachable, `grammar.rs:62-95`), undo
under R-P6/R-5.1/R-5.2, the acting-seat fence (R-4.3), deal / keep / join / unlock / chat, the
`Events` frame (`lib.rs:1816` `events_of`), the four card indexes, `suggest`/`why`
(`explain.rs`), the result (`outcome.rs`), and the archive as a trait.

**The cut runs THROUGH `Games`, not around it** (the survey's finding, correcting this spec's
first draft). At HEAD `Games` is not a wrapper: all 24 document methods in `tables.rs`'s
`impl Games` (`:686-1696`) ARE the layer, and the axum-shaped state sits ABOVE it in
`AppState` (`lib.rs:255-271`: `rules`, `games`, `identity`, `heartbeat`, `chat`). Three of the
helpers every route starts with take `&AppState` rather than `&Games` (`admitted` `:506`,
`events_at` `:1856`, `lobby_chat_on` `:981`), and the layer's refusal type `ApiError`
(`lib.rs:76-85`) carries `axum::http::StatusCode`. So the shape M6.5a builds is:

- the layer's state is `Games` plus the `rules`/`chat` half of `AppState` (the chat rule is
  already copied into `Games::limits`, `lib.rs:311-313`), and the three helpers take it;
- `ApiError` is re-typed on the `http` crate's `StatusCode` directly — the SAME type axum
  re-exports (`http` 1.5.0 in the lock), so the `IntoResponse` impl stays under `server` and
  nothing above the type changes;
- the seven `lib.rs` helpers and `input`'s thirteen steps (`lib.rs:1103-1284`: the grammar
  fence, admission, the spectator refusal, the seat match, the undo rule, the acting-seat
  fence, the `expect` check, `step`, `persist`, the warnings line, the `APPLIED BUT NOT SAVED`
  arm, the prompt-suffix strip, `ClientState::of`) move down beside the layer's methods —
  seventeen of the twenty-eight handler bodies carry logic beyond one `Games` call, and `input`
  is the one that has to move whole;
- `AppState` stays as the axum wrapper holding `identity`, `heartbeat` and the router;
  `Games` stays a NAME and its method signatures stay, which is what leaves the 145 module
  cells and the 74 route cells (all of which drive `app(state)` through `oneshot`) green
  untouched — the route cells are behaviour assertions over the wire and do not know how the
  body is layered;
- axum and tokio go behind a `server` cargo feature (default on) as OPTIONAL dependencies,
  because cargo features are additive and tokio's `time` builds for the target only to panic;
  `FsStore`, the `std::env` readers, `signal.rs`, `identity.rs`'s extractor, `art.rs`,
  `assets.rs` and the router go with them, so that the wasm build carries no code that can only
  fail (§6.1 O-10);
- the clock is a seam the layer takes (`now_unix`'s two callers), and **it has its own fence,
  because the compile check cannot see it** (H-1): a source-scanning test in the crate, in the
  shape of `web/e2e/every_spec_can_skip.spec.ts`, reads the crate's non-test source at run time
  and names any `SystemTime`, `Instant`, `std::fs`, `std::env`, `std::thread` or `std::process`
  outside the `server`-gated modules;
- the CI line `cargo check -p yokohama-web --no-default-features --target
  wasm32-unknown-unknown` holds the DEPENDENCY fence (no axum, no mio) and the scanning cell
  holds the PLATFORM-CALL fence; neither is the other's fallback;
- `wire_fixtures` survives the clock seam untouched because `freeze_clock` (`tests/
  wire_fixtures.rs:247-266`) already normalises `created_at` to a constant before comparison.

It is `crates/yokohama-web` with a feature, NOT a new crate: no file moves and `git blame`
survives. The M6.5a spec carries the exact task list; this section is its charter.

**Three hosts:**

| host | crate | identity | store | signal | who runs it |
|---|---|---|---|---|---|
| axum (NAS, Docker, self-host) | `yokohama-web` (feature `server`) | `claimed` / `cf-access` | `FsStore` | SSE over tokio watch | unchanged |
| browser (single-player) | `yokohama-wasm` (new; `wasm-bindgen`) | none needed | IndexedDB, via the client | none (one tab, one game) | M6.5b |
| Durable Objects (hosted multiplayer) | `yokohama-worker` (new; workers-rs) | `cf-access` under a hostname-based Access application | the object's SQLite | hibernating WebSockets | M6.5c |

### 2.2 The route to object map (M6.5c)

The Worker resolves `/api/games/{id}/…` to `env.TABLE.idFromName(id)`. **There is no lookup
table FOR ROUTING: a table's id is its address** (the house's index, §2.4, is a lookup table
for LISTING and nothing else). A link for an id never created reaches an object with empty
storage, which answers 404 and, having written nothing, ceases to exist — probing creates no
garbage.

| route (`lib.rs:2116` `app()`) | answered by |
|---|---|
| `/api/health`, `/api/me`, `/api/labels`, `/api/art`, the static client and art | the Worker |
| `/api/games` GET (`scope=mine\|open\|playing`), `/api/stream`, `/api/chat` GET/POST | the house object |
| `/api/games` POST | the Worker's create handler draws the ID (today's independent 64-bit draw, `tables.rs:709`) before the object is addressed; the table object initialises create-if-absent inside its single thread, drawing the SETUP seed (`:697`) and later the BOT seed (`finalize`, `:1195`) itself through `draw_seed`, then pushes its row |
| `…/{id}` GET/DELETE, `table`, `deal`, `keep`, `join`, `unlock`, `chat`, `input`, `ports`, `labs`, `areas`, `log`, `events`, `stream`, `at/{k}`, `suggest`, `why`, `index/{kind}`, `result`, `record` (R-6.5.10) | the table object |

### 2.3 The table object

- **Storage**: three rows or tables in its SQLite — the record (the same JSON
  `Session::to_record_json` writes, digest-stamped), the table document, the chat messages
  (`(seq, created_at, author_kind, author_user, author_name, text)` with `last` kept as a
  counter that survives a trim, R-6.4.9), plus `unlocked`, `archived_at`. `Version` and the
  compare-and-swap are not needed inside the object; the layer keeps `check_shape` on read and
  write because every stored field is still an input, and every bound it checks returns `Err`
  rather than reaching a constructor that panics (§6.1 O-1).
- **Memory**: the `Repl`, rebuilt from the record through `Session::from_record` on every wake
  — the same cold-miss path `Games::handle` (`games.rs:269`) takes today, at a DIFFERENT
  FREQUENCY: at HEAD the cache is warm for the life of the process, and an object hibernates
  ten seconds after its last event, so on a turn-based game the rebuild happens on most moves.
  Two consequences the spec names rather than assumes: the replay's cost per wake grows with
  the record and is UNMEASURED (§6.1 O-3); and **an open multi-step picker — `Repl::sub`,
  which is not in the `GameRecord` — is dropped at every hibernation, not at a rare cold
  miss** (H-4). `Input::expect` turns the stale answer into a 409 rather than a wrong move
  (its own doc measures the case: a client holding `{"kind":"place_a","selected":[3,7]}`
  clicked area 3 and a rebuilt `Repl` applied option 3), but a player who thinks for a minute
  inside a placement picker would meet that 409 on most turns. M6.5c's spec decides among
  persisting `SubPrompt` as a per-object row beside the record (the lean: it is the object's
  own state and survives socket loss too; the M6.1 reason for keeping it out of the RECORD does
  not apply to a row that is not the record), carrying it on the socket attachment, or having
  the client re-open the picker; doing nothing is not among the options.
- **The signal**: one hibernatable WebSocket per visible tab (R-P8 holds: a hidden tab closes
  its socket; an idle open one costs nothing), `serializeAttachment` carrying the viewer (seat
  or spectator) so a wake needs no handshake, `setWebSocketAutoResponse` for the client's
  ping. Seven of the eight bump sites send the `Events` frame to every socket; the eighth,
  delete, sends the `end` frame and closes them. **Two properties, held two ways.** The
  mover's own reply is durable BY CONSTRUCTION: DO output gates hold the response until the
  storage write is confirmed. The opponent's frame not overtaking the write is held BY
  DISCIPLINE — the write is awaited before the send — and that is exactly what the four M6.3
  and M6.4 gated cells police today, so §4's M6.5c battery carries the same cell shape over
  the object's storage rather than claiming the problem away.
- **Lifecycle**: ten seconds after the last event the object hibernates between moves; two
  minutes after the last socket closes it is evicted; days later the next request re-runs the
  constructor over the same storage. The lobby says "waiting on <name>" from the row the table
  pushed at its last bump. The alarm API is the future hook for a turn notification (R-P3 keeps
  it out of scope) and for retention (a `deleteAll()` N days after archiving — a policy, not
  built).
- **The archive** (R-6.5.4, R-6.5.5): at game over, `put` the two documents to the bucket; on
  failure set `archived_at = NULL` and an alarm; the put is idempotent. At abandon the put is
  awaited and a failure REFUSES the delete, because the alarm cannot outlive `deleteAll()`.

### 2.4 The house object

One named object. It holds an index row per table — id, label, status, players, seats (kind,
name, and under this door the email, which is how `scope=mine` is answered), `to_move`,
`over`, `winner`, a `locked` BOOLEAN, `created_at`, `updated_at` — the house chat, and the
lobby's sockets. **It is a projection, never an authority**, in exactly the role
`TurnMirror` (`table.rs`) plays today: a table object pushes its row at each bump site (one
object-to-object call), a failed push leaves the row stale until the next bump heals it, and
every click resolves against the table object. **The password never reaches the house**: the
salt and hash live only in the table document; `join` and `unlock` are requests to the table
object, which checks them itself and writes its own roster. Delete runs `deleteAll()` on the
table, then removes the house row.

**Its load, in numbers, the way the room plan §1 wrote the polling arithmetic.** A live 2P
game bumps about once per applied move — the e2e's longest ran 1,704 applied moves in 457 s,
under four a second at the fastest this lane has ever played — so N concurrent live tables
push at most 4N rows a second; each push is one row upsert under the input gate, well under a
millisecond of work. The lobby reads three listings per house frame (M6.4's LEAVE item) from
however many tabs sit on the lobby, and each is one SQL query over an index that at a
thousand tables is a few hundred kilobytes. Against a single object's request budget (Paid: a
million a month included, then $0.15 per million) that is one house object for tens of
concurrent tables and every lobby tab the owner's allow-list can produce, with the 20:1
WebSocket-message discount unused. A public door would revisit this number; this door does
not reach it.

### 2.5 The browser host (M6.5b)

`yokohama-wasm` exposes ONE function, request-in / response-out over the same method, path,
query and body the HTTP client already sends, so `web/src/api.ts`'s `call` (`:233`, the
module's only `fetch`, behind which all 24 members of the `api` object sit) dispatches to it
when the game is local and to `fetch` otherwise, and every component above `api.ts` is
unchanged. **What that one-file claim leaves out, and the M6.5b spec has to carry (the
survey's §5.4):** `sse.ts:102`'s `readStream` is the client's second `fetch` and `usePolled`'s
`open()` (`signal.ts:270-292`) branches on `signalKind()` into the held request or the stream,
so a local game needs a third arm there rather than a `Source` change; the three module-level
deployment facts (`signalKind`, `lobbyChat`, `chatText`) are all set from ONE `api.health()`
at `App.tsx:561-563`, so the local dispatch must answer `/api/health` too or the browser host
keeps the server's values; and `identityHeaders()` (`api.ts:176`) is read by `call` and by
`readStream`, so under the local arm `call` skips it rather than each caller knowing. The
record is persisted to IndexedDB after every applied move, the same "nothing exists only in
memory" rule; a reload rebuilds the `Repl` from it. There is no signal: one tab, one game, and
the bot's trailing run returns in the same call. The lobby gains "Play the bot here" beside
"New table". **The load-bearing pin**: a record played by the wasm bot must reconstruct
byte-identically under the NATIVE `why` (`record::state_at` + the same `decide` at the same
`(bot_seed, events.len())` key), which is the determinism claim WebAssembly has to earn before
anything hosted depends on it — and the same run measures the replay cost in wasm that §6.1
O-3 asks for.

### 2.6 The client's transport seam (M6.5c)

`usePolled(source, active, onValue, onEnd?)` (`web/src/signal.ts:170`) keeps its signature and
`Source` (`:101`) is UNCHANGED — its `stream` field already carries a URL a WebSocket fits.
What changes is `signalKind`'s union at `api.ts:193` (`"sse" | "held"` gains `"ws"`, set from
`/api/health` exactly as the M6.3 spec §7 chose) and `usePolled`'s `open()`, which gains its
third arm. Identity on a WebSocket: the browser's `WebSocket` cannot set a header either, so
under a hostname-based Access application the identity is the Access cookie on the upgrade
(untested here, §6.1 O-6, and the first thing R-6.5.8's early deploy tests), and the `claimed`
development door passes its email as a query parameter that the Worker refuses whenever
Access identified the caller (R-6.5.11). On reconnect the object writes the current `Events`
frame and the client's on-connect compare (M6.3 §5.1) is the recovery, since the count is
cumulative (R-3.14).

---

## 3. The three sub-cycles

Each: brainstorm → spec → plan → execution under R-P11 → deploy → review against the live
site → close → handover. Exit criteria are what the close must SHOW.

### 3.1 M6.5a — the seam

**Builds:** the cut of §2.1 (the layer's state, `ApiError` on `http::StatusCode`, the helpers
and `input` moved down, the `server` feature with its optional dependencies and gated modules);
the clock seam with its source-scanning fence; the archive as a trait with the directory
implementation (R-6.5.4) and abandon-and-save's semantics in `delete` (R-6.5.5, awaiting and
refusing); `GET …/record` (R-6.5.10); "Save game file" on the finished-game screen; the CI
line for the dependency fence.
**Exit:** the axum server redeployed on the NAS with **every EXISTING corpus payload
byte-identical except `health.json`, which gains exactly one key (`archive`)** — regenerate,
run without the variable, read the diff, report it: the proof that nothing moved on the wire.
The record route joins the corpus by its 409 alone: **a finished record is never committed
as a fixture**, because it carries the setup seed and the value pin
`no_committed_payload_carries_the_setup_seed` searches the whole corpus; instead that pin
gains a SECOND anti-vacuity partner — the route's 200 body — beside the table-document read
it already carries (`tests/wire_fixtures.rs`, the partner CLAUDE.md names), asserting IN-TEST
that the record does carry the seed and that no committed file does. `cargo check …
--no-default-features --target wasm32-unknown-unknown` green in CI; the scanning cell green;
the CLI untouched; core byte-identical.

### 3.2 M6.5b — single-player in the browser

**Builds:** `yokohama-wasm` (§2.5); the local arm in `api.ts` AND in `usePolled`, with
`/api/health` answered locally; IndexedDB persistence; "Play the bot here"; "Send to the
archive" (R-6.5.6) on the finished-game screen, dark until a host reports an archive; the
determinism pin; the corpus subset the browser host serves, named and regenerated against it.
**Exit:** a whole game against the bot played in the browser with the network tab showing
ZERO `/api/games` calls after load (counted off the page's own request log, the way the M6.3
e2e counts `/events`); every payload the local arm serves byte-identical to the committed one;
the save downloaded and reconstructed natively under `why` with zero refusals, and the wasm
replay's wall recorded; shipped on the NAS, because this host needs no server.

### 3.3 M6.5c — the room on Durable Objects

**Builds:** `yokohama-worker` (the Worker, the table object, the house object, §2.2–§2.4);
the `ws` transport arm (§2.6); the R2 archive with alarm retry; the hostname-based Access
application in front; the hostname move (the tunnel that serves the ttyd terminal holds
`yokohama.moore.nyc` today — `deploy/worker/wrangler.jsonc`'s own warning); the runbook's new
§6; the deletion of the retired code (R-6.5.13); e2e under `wrangler dev` AND against the live
deployment (R-6.5.8), the Access-protected WebSocket upgrade and the cold start FIRST; the
`SubPrompt` decision (H-4) and the panic policy (O-1) settled in its spec.
**Exit — a number, not a demonstration:** two people play a whole game through the hosted path
and a third watches; the table object is OBSERVED hibernating between moves (the dashboard's
duration metric, or the object's own constructor counting its wakes); the wall from a move to
the other browser's screen through the object measured the way M6.3 measured it through the
tunnel, and the replay-on-wake cost recorded beside it; a finished game and an abandoned one
land in the bucket and load in the CLI; and the whole corpus byte-identical against the
Worker.

---

## 4. Fences, and the mutations each sub-cycle must catch

Named here so each sub-cycle's spec carries the battery; the wording of a cell is the
sub-cycle's. **"Regenerate the corpus and read the diff" is a task discipline, not a
mutation catcher** (M-7): M6.5a and M6.5b both change the wire on purpose, so a non-empty
diff is the expected outcome of the work and the reading of it is the check.

- **M6.5a**: a `SystemTime`, `Instant`, `std::fs`, `std::env`, `std::thread` or `std::process`
  call planted in a non-`server` module fails the scanning cell (and NOT the wasm check line,
  which is blind to it — H-1); an `axum::` or mio-backed `tokio::` path reachable without the
  `server` feature fails the check line; `GET …/record` at a running game is a 409 and at a
  finished one its body is the exact bytes the store holds (a digest compare, so a
  re-serialization that dropped the digest fails); the seed pin's second partner fails if the
  route's 200 body stops carrying the seed, and the pin itself fails if any committed file
  starts to; abandon of a playing table with events calls the archive exactly once, an open
  one never, a failed archive REFUSES the delete and leaves the table, and a game-over
  archive failure refuses nothing; the two archive implementations write the same relative
  paths (O-9).
- **M6.5b**: the local arm makes no `fetch` (a `fetch` spy at zero across a whole game,
  including the stream reader); a record with one event edited is REFUSED by "Send to the
  archive" (strict replay), as is a record not over and a record at another `semantics_rev`,
  by three different sentences; the wasm bot's moves reconstruct under native `why` (a
  mutation that perturbs the decision stream key must fail it); the seed never leaves the
  browser except inside a finished record; every payload the local arm serves is
  byte-identical to the committed one.
- **M6.5c**: a spectator's socket message that is a move is refused before the record is
  touched (the M6.3 sweep, over WebSockets); a client-supplied `X-Yokohama-Claimed-User` OR
  `X-Yokohama-User` header, or the `claimed` query parameter, is ignored whenever Access
  identified the caller; the house never holds a hash (a value pin over the house's rows for
  this run's salt and hash, the M6.2 shape); a frame never overtakes its write (the M6.3
  gated-cell shape, over the object's storage); the whole corpus byte-identical against the
  Worker; the object hibernates (a wake counter that MUST increase across a ten-second pause);
  an abandon whose put fails leaves the object and its storage in place.

---

## 5. Deployment (M6.5c's, recorded here so the earlier sub-cycles plan for it)

- Workers Paid; one Worker (`yokohama-web`) serving the client and art as static assets, with
  two SQLite-backed object classes (`Table`, `House`) and one R2 bucket bound; **a
  hostname-based (self-hosted) Access application** on the hostname with the owner's
  allow-list; `cf-access` identity.
- The hostname takeover is the step that takes the ttyd tunnel's route away: give ttyd its new
  hostname first and verify it (`wrangler.jsonc`'s own note).
- Early and repeated deploys (R-6.5.8); the runbook records each with the container-era
  numbering continued.
- The NAS keeps serving the axum host throughout; nothing here retires it.

---

## 6. What this spec does NOT claim, and what it leaves open

- No sub-cycle after M6.5a is specified; §3's "Builds" lines are charters.
- No sizing and no date.
- Nothing hosted is verified: every platform fact in §0.4 is a docs reading of 2026-09-19 and
  every wasm figure in §0.3 is a throwaway crate outside the tree. The determinism of the wasm
  bot (§2.5) is a CLAIM until M6.5b's pin holds.
- The Free-plan CPU figure for a Durable Object was not found (§0.4); R-6.5.12 sizes for Paid.
- The email roster, the password oracles, house-chat moderation and rate limits are LEAVE items
  on this door (R-6.5.3), with the handover's owner items 1–7 carried unchanged: the vs-bot gate
  livelock, the e2e `CAP = 4000`, the disk/trace item, the house chat's lack of delete (which
  the house object's SQLite makes one statement, still not built), three listing reads per
  house message, and WARP.
- `wasm-opt` was not run; the sizes are without it.

### 6.1 Open items the sub-cycles must settle (the adversarial read's OPEN list, each with what settles it)

- **O-1 — a panic inside an object.** The spike's profile is `panic = "abort"`; the docs say
  in-memory state is not preserved across a crash. A panic that recurs on every wake — say from
  a malformed stored row — would make a table unreadable, including for its own delete. What
  settles it: the M6.5c spec rules the panic policy (unwind with `catch_unwind` at the object
  boundary, or abort with a poison flag in storage) and confirms that every bound `check_shape`
  checks returns `Err` before any constructor that panics is reached.
- **O-2 — the 1 s startup limit** against a 1.31 MB module whose constructor re-runs at every
  wake. What settles it: R-6.5.8's first deploy reads `wrangler deploy --outdir bundled/
  --dry-run`'s `Total Upload` and measures a cold start.
- **O-3 — replay on every wake.** `Session::from_record` REBUILDS a state from seed + events;
  it does not deserialize one. At HEAD that is paid once per process; in an object it is paid
  on most moves, and the e2e's own games reach 1,700 applied moves. What settles it: time it
  natively on a long record (the NAS pull recipe gives one) and in wasm (M6.5b's determinism
  run gives that for free); if it is the per-move latency the opponent feels, the M6.5c spec
  weighs a stored state snapshot against the provenance the replay provides.
- **O-4 — `deleteAll()` racing an in-flight put** — RULED here: R-6.5.5 awaits and refuses.
- **O-5 — the house object's load** — SIZED here, §2.4; a public door revisits it.
- **O-6 — the Access cookie on a WebSocket upgrade.** Same-origin makes it plausible; nothing
  here has tested it, and H-3's limitation means the answer depends on the kind of Access in
  front. What settles it: the first thing R-6.5.8's first deploy does.
- **O-7 — `getrandom`'s `js` feature inside a Worker.** Proven in the browser path by the
  spike; unproven in a Worker, where it reaches `crypto` through the wasm-bindgen glue
  workers-rs generates, and version-pinned (0.2.16 in the lock; 0.3 renames the feature). What
  settles it: a `worker`-crate spike calling `SeedSchedule::from_os_entropy()` under
  `wrangler dev`, before M6.5c's plan is written — `create` depends on it.
- **O-8 — the three checks behind "Send to the archive"** — NAMED here, R-6.5.6; the strict
  replay's cost in the Worker is O-3's quantity.
- **O-9 — two archive implementations, one layout** — RULED here, R-6.5.4: one shared layout
  function and a cell over both.
- **O-10 — `FsStore` under `--no-default-features`** — RULED here, §2.1: behind `server`.

---

## 7. Read order for the session that opens M6.5a

1. `CLAUDE.md` — the Web (server) and Web (browser half) bullets and the `crates/yokohama-web`
   architecture bullet.
2. `docs/handover.md` Track 3.
3. This document, §0–§2 and §6.1.
4. The cut-surface survey and the adversarial read, in the gitignored workspace
   `.superpowers/sdd/2026-09-19-web-frontend-m6.5-hosting/` (`survey-cut-surface.md`,
   `review-umbrella-spec.md`) while they exist; the survey's §1–§7 is the line-cited map the
   M6.5a spec is written from, and everything of it this spec depends on is restated in §2.1.
5. `docs/plans/2026-09-19-web-frontend-m6.5-hosting-handover.md` §2 (the seams as M6.4 left
   them) and §6 (the process).
6. The M6.5a spec, then its plan.
