# Web frontend — M6.5 "the hosting move": the spec

**Written 2026-09-19 on branch `web-room-m6.4` at `faf1a88` (M6.4's close HEAD), from the
brainstorm held the same day between the owner and the orchestrating session.** M6.5 is the
fifth and last cycle of the room plan (`docs/plans/2026-09-14-web-frontend-m6-room-plan.md`
§2) and its entry artifact is `docs/plans/2026-09-19-web-frontend-m6.5-hosting-handover.md`.
**This document supersedes the handover's charter** (§0.5 says exactly where) and is the
authority for the ORDER and the SEAMS of M6.5; it is an umbrella of THREE sub-cycles, each of
which enters through its own spec and plan under R-P1 and R-P11, the way the room plan was the
authority for M6's five. The first sub-cycle's spec, M6.5a, is written beside this one.

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

## 0. What is true at HEAD (`faf1a88`)

Every citation below is at `faf1a88`. Re-read each at the HEAD a sub-cycle opens; four cycles
running, something had moved every time.

### 0.1 The three seams as built

- **Identity** (`crates/yokohama-web/src/identity.rs`): `IdentityMode` (`:32`) is `Claimed`
  | `CfAccess` | `Worker`, read from `YOKOHAMA_WEB_IDENTITY` (`:55`) with no default. Each mode
  reads exactly one header (`:47-52`); `resolve` (`:127`) is the pure half and the axum
  extractor `Identity` (`:151`) the transport half. `User::parse` (`:92`) trims and lowercases
  once. Under `worker` the literal `anonymous` counts as absent (`:132`).
- **Store** (`crates/yokohama-web/src/store.rs`): `trait Store: Send + Sync` (`:117`) — `list`,
  `read`, `write_if_version`, `delete` — over `Doc` (`:36`, four variants: `Record`, `Table`,
  `Chat`, `House`); `Version` is a content digest (`version_of`, `:84`);
  `RESERVED_ID_SUFFIXES = [".table", ".chat"]` (`:157`); `is_safe_id` (`:169`);
  `check_house_id` (`:200`); `FsStore` (`:208`, temp + rename, no lock) and `MemStore`
  (`:398`). **The trait is synchronous.**
- **Signal** (`crates/yokohama-web/src/signal.rs`): `Signals` (`:48`) over
  `tokio::sync::watch` (`:21`) — `Bump { seq, end }` per table (`:26`), `House { seq, chat }`
  (`:42`). The eight bump sites, every one in a success arm: `persist` `games.rs:543`; `create`
  `tables.rs:769`; `chat_append` `:951`; `house_append` `:965`; `flip_to_playing` `:1255`
  (which is `finalize`'s bump too); `join` `:1402`; `keep` `:1504`; `delete` `:1693` (`end`).
  `GET …/{id}/stream` and `GET /api/stream` sit in front (`lib.rs:1931`, `:1993`);
  `SIGNAL_KIND = "sse"` (`lib.rs:240`); `HEARTBEAT` 25 s (`lib.rs:248`).

### 0.2 The per-process state, and what an actor does to each

The handover §2.3 enumerated nine pieces of state a second replica breaks, read off `Games`
(`games.rs:110-186`) and `AppState` (`lib.rs:255-271`). On the path this spec chooses there is
no second replica of a table: a Durable Object is a single-threaded actor and every request for
a table id reaches the same object. Row by row:

| what | at HEAD | inside a table object |
|---|---|---|
| `live` (`Repl` cache, bounded 64) | a miss is a replay | the one in-memory `Repl`; a miss (wake) is the same replay |
| `written` (CAS base) | detects a cross-process conflict | no second writer exists; retired |
| `deleted` (tombstones) | a local delete's guard | `deleteAll()` ends the object; retired |
| `table_lock` (global create) | serializes id draws | ids are drawn by the caller; create-if-absent runs inside the object |
| `locks` (per table) | R-2.9's exclusion | the object's input gate |
| `signals` | one process's memory | the object's WebSockets |
| `chat_last` (per-process count) | WRONG with two processes | a `MAX(seq)` over the object's own rows |
| `house_lock` | serializes house appends | the house object's input gate |
| `limits` / `identity` / `heartbeat` | launch variables | Worker configuration, one place |

`TableDoc.unlocked` (`table.rs:211`) is stored, not per-process, and stays a stored roster.

### 0.3 What compiles to WebAssembly today (measured 2026-09-19, throwaway crate, not in the tree)

- `cargo check -p yokohama-core --target wasm32-unknown-unknown` fails on exactly ONE thing:
  `getrandom` 0.2 refuses the target without its `js` feature (the transitive dependency of
  `rand`, reached by `OsRng` at `game/seeds.rs:48,108`). **Enabling that feature from a
  downstream crate unifies onto core's dependency, so core's `Cargo.toml` stays byte-identical.**
- **The CLI's library half compiles to the target unchanged.** `rustyline` 18 with the crate's
  own `default-features = false, features = ["buffer-redux"]` (`crates/yokohama-cli/Cargo.toml`)
  has a no-terminal fallback on `wasm32-unknown-unknown`, and `yokohama-bridge`'s `std::net`
  compiles (it is never called). No feature flag, no crate split.
- Sizes, `opt-level = "z"`, `lto = true`, `panic = "abort"`, no `wasm-opt` (not installed):

  | linked | raw | gzip | brotli |
  |---|---|---|---|
  | `Rules::standard`, `new_game`, `finalize_setup`, `get_legal_moves`, `apply_move`, `Contestant::PRODUCTION.build` | 744 KB | 213 KB | 161 KB |
  | plus `Session::new_sim`, `Repl::new`, `Repl::step`, `to_record_json` | 1.30 MB | 387 KB | 289 KB |

- The production bot, native release, `examples/arena -- cell … 2 v4-step9-draft1
  v4-step9-draft1 1 --master 20260919 --no-invariants`: **1,538 µs and 1,483 µs per decision**
  over 220 and 214 decisions; **328.5 ms per whole 2P game** of bot thinking.
- Two runtime hazards on the target, both in the web crate and neither in a fenced crate:
  `table::now_unix` (`table.rs:417-422`) calls `std::time::SystemTime::now()`, which PANICS on
  `wasm32-unknown-unknown` — a clock seam; and `signal.rs:21`'s tokio watch channel, which is
  the seam an object replaces. `tables::draw_seed` (`tables.rs:213`) goes through
  `SeedSchedule::from_os_entropy`, which `getrandom`'s `js` feature serves from
  `crypto.getRandomValues` in a browser and in a Worker.
- The web crate itself: of its 19,463 lines, only `lib.rs` (the routes), `identity.rs`,
  `art.rs`, `assets.rs` and `main.rs` mention axum. `games.rs`, `tables.rs`, `chat.rs`,
  `table.rs`, `store.rs`, `client.rs`, `explain.rs`, `outcome.rs` and `grammar.rs` do not.
  `Cargo.toml` enables tokio's `rt-multi-thread`, `net`, `signal` and `time`, none of which
  build for the target; `sync` does.

### 0.4 The platform, from the docs (all read 2026-09-19)

- **Durable Objects run on Workers Free and Paid**; since 2026-07-09 a NEW namespace must be
  SQLite-backed; 10 GB per object; SQL, point-in-time recovery to any moment in the past 30
  days. (`developers.cloudflare.com/durable-objects/platform/pricing/`, `/reference/durable-objects-migrations/`)
- **Retention is indefinite.** "A Durable Object fully ceases to exist if, when it shuts down,
  its storage is empty"; once anything is written, only `deleteAll()` removes it.
  (`/durable-objects/best-practices/access-durable-objects-storage/`)
- **Lifecycle**: active → idle in memory (hibernatable after ~10 s when only hibernatable
  sockets are open) → hibernated (memory dropped, WebSockets stay connected at the edge) →
  inactive (evicted 70–140 s after the last event when not hibernatable). Any request or
  socket message re-runs the constructor. Hibernation applies only to WebSockets the object
  SERVES; a held HTTP response keeps it awake. Duration is not billed while hibernatable;
  `setWebSocketAutoResponse` answers ping/pong without waking the object;
  `serializeAttachment` survives hibernation. (`/durable-objects/concepts/durable-object-lifecycle/`,
  `/durable-objects/best-practices/websockets/`, `/durable-objects/api/state/`)
- **Limits**: 128 MB per isolate; Worker bundle 64 MiB UNCOMPRESSED on both plans since
  2026-09-04 (the 3 MB / 10 MB compressed limits are gone); CPU per request 30 s default on
  Paid, raisable to 5 min; the plain-Workers Free row reads 10 ms and this session did not find
  a Durable-Object-specific Free figure. Free plan: 100,000 DO requests/day, 13,000 GB-s/day;
  Paid: 1 M requests and 400,000 GB-s a month included. WebSocket messages count as requests.
  (`/workers/platform/limits/`, `/durable-objects/platform/pricing/`)
- **Access on a Worker**: `ctx.access.getIdentity()` gives the signed-in email with no JWT
  parsing (2026-08-14); `wrangler dev` can simulate it with an `access.dev` block; the
  `Cf-Access-Jwt-Assertion` and `Cf-Access-Authenticated-User-Email` headers remain.
  (`/workers/configuration/cloudflare-access/`)
- **workers-rs** `worker` 0.8.5: `DurableObject` trait with `fetch`, `alarm`,
  `websocket_message`/`websocket_close`/`websocket_error`; `#[durable_object(websocket)]`;
  `SqlStorage` on `state.storage().sql()`; no tokio runtime ("leave your threaded async
  runtimes at home"). (`docs.rs/worker/0.8.5`)
- **R2**: 10 GB-month, 1 M Class A and 10 M Class B a month free; zero egress; Workers
  binding, S3-compatible API, `wrangler r2 object get`. (`/r2/pricing/`, `/r2/api/`)
- **Data Studio** (beta 2025-10-16) reads and writes a named SQLite-backed object's SQL data
  from the dashboard, Workers Platform Admin role, audit-logged; there is no CLI path to an
  object's data. (`/durable-objects/observability/data-studio/`)
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
- **The handover §5's leans** on questions 1, 2, 3 and 6 (schema, notifier shape, Modal vs
  Containers, the caches) are answered by §2 below; 4 and 5 (roster, moderation) are LEAVE
  items on this door by R-6.5.3; 7 (`claimed`) is R-6.5.11; 8 (the exit measurement) is §3.3.

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
- **R-6.5.3 — the door.** The hosted deployment sits behind Cloudflare Access with the owner's
  allow-list, as today. Other identity methods may be added later; with Access available the
  cycle focuses on the architecturally interesting and baseline factors. Consequently the email
  roster on the wire (M6.3 §6.3 item 1, R-6.4.2), the `unlock`/`join` password oracles and
  house-chat moderation (R-6.4.14) stay LEAVE items on this door and are named in §6.
- **R-6.5.4 — the archive.** Collecting save games matters. A finished game is written to an R2
  bucket by its table object at game over, in the FILE STORE's own layout (`<id>.json` the
  record, `<id>.table.json` the table document) under a `SEMANTICS_REV`-keyed prefix, so an
  `rclone sync` of the bucket is a directory the CLI loads. Chat is excluded. A game id is
  archived AT MOST ONCE (over, or abandoned — R-6.5.5), so there are never versions to
  reconcile on the training side; an on-demand "save to R2" click was considered and rejected
  for exactly that housekeeping. The R2 binding's presence is the switch, reported on
  `/api/health`; a failed archive is logged and retried by alarm and NEVER refuses a move —
  the object's own storage is the durable copy. A later pipeline (Databricks was named) is out
  of scope; the layout is chosen so it can read the bucket. **On the axum host the archive
  is a DIRECTORY** beside the store in the same layout (`YOKOHAMA_WEB_ARCHIVE_DIR`, absent =
  off, reported on `/api/health` the same way), so the NAS collects saves during development
  from M6.5a on, and the bucket in M6.5c is the second implementation of one trait.
- **R-6.5.5 — abandon and save.** The table creator's existing two-click Abandon (M6.2 ruling
  #13; `web/src/Abandon.tsx`; `DELETE /api/games/{id}`, `lib.rs:1049` → `tables.rs:1648`)
  ARCHIVES the table under an `abandoned/` prefix whenever the archive is on and the table has
  at least one event, then deletes it. One control, its label telling the truth from health
  and state; an open or dealing table with no events still just deletes. During development
  this is how saves are collected without playing to completion.
- **R-6.5.6 — single-player is entirely local.** The record lives in the browser (IndexedDB);
  a local game never appears under "your games" on the server and needs no sign-in beyond the
  door. "Save game file" is a plain download at game over on BOTH channels. When the archive
  is on, "Send to the archive" posts the record to the Worker, which does not trust it: it
  replays the record through the same WebAssembly engine under the strict replay (rules hash,
  every event through the checked boundary, terminal digest) and refuses a record that is not
  over, then writes it under a `browser/<user>/` prefix.
- **R-6.5.7 — the order.** M6.5a the seam, M6.5b single-player in the browser, M6.5c the room
  on Durable Objects. Each its own brainstorm → spec → plan → execution under R-P11. The
  alternative (c before b) reaches "online" sooner but stacks the transport work on wasm risks
  that b retires cheaply first.
- **R-6.5.8 — test in the live setup during development.** The owner's experience is that
  `wrangler dev` and production differ in ways that matter. M6.5c deploys to the real account
  behind Access EARLY and repeatedly, not only at the final review — the site is private and
  the only players are the owner's — so a bad assumption is met before the review rather than
  at it.
- **R-6.5.9 — what stays fixed** (room plan §3, restated with what §0.3 measured): the five
  fenced crates byte-identical; `SEMANTICS_REV` 7; the CLI UNTOUCHED (it need not be touched:
  its library half compiles to the target as-is); the server ROUTES, SCOPES and SERIALIZES and
  computes nothing about the game; no bot behaviour moves; **the wire corpus is the contract
  across ALL THREE hosts** — the axum server, the browser and the object must produce the same
  82 payloads byte-for-byte, and a host that cannot is not done.
- **R-6.5.10 — the export route.** `GET /api/games/{id}/record` serves the record's bytes to a
  seat holder ONLY once the game is over, because the record carries the setup seed, hidden-set
  member #1, until the last move. Before game over it is a 409 in this API's error shape.
- **R-6.5.11 — `claimed` is kept as the development door** on every SERVER host, refused when the
  Worker's Access identity is present (the Worker strips the header exactly as
  `deploy/worker/src/index.ts` already does for `X-Yokohama-User`). On the hosted path the
  identity mode is `cf-access` — the header Access sets on every request that reaches a Worker
  route through it — with `ctx.access.getIdentity()` as the upgrade once workers-rs exposes it.
- **R-6.5.12 — the plan.** The owner's deployment runs on Workers Paid (30 s CPU per request,
  1 M object requests a month included). Human-only tables would fit the Free plan; a hosted
  bot seat is not sized for it and this spec does not claim it.
- **R-6.5.13 — the retired code goes.** M6.5c deletes `deploy/worker/` and `deploy/modal_app.py`
  and rewrites `deploy/RUNBOOK.md` §4 as the record of why they were retired; the NAS sections
  §0–§3 and §5 stay, because the NAS stays.

---

## 2. The architecture: three hosts over one layer

### 2.1 The layer

**The table layer** is one table's logic, transport-neutral, no tokio, no axum, no clock of
its own: the three documents (`table.rs`, `chat.rs`, the record), admission (`lib.rs:506`'s
`admitted` and `spectators_write_nothing`), the viewer projection (`client.rs`), the move path
over `Repl::step` behind `grammar::is_gameplay_line`, undo under R-P6/R-5.1/R-5.2, the acting-
seat fence (R-4.3), deal / keep / join / unlock / chat, the `Events` frame (`lib.rs:1816`
`events_of`), the four card indexes, `suggest`/`why` (`explain.rs`), the result
(`outcome.rs`), and the archive as a trait. It takes a `Clock` and a per-table `Store`-shaped
document set, and returns typed results the hosts serialize. It is `crates/yokohama-web` with
axum and tokio behind a `server` cargo feature (default on), NOT a new crate: the split is a
feature so that no file moves and `git blame` survives, and the CI line
`cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown` is what
holds it (M6.5a spec §4). **`Games` stays** as the axum wrapper around the layer, holding the
file store, the per-table locks and the watch channels it holds at HEAD.

**Three hosts:**

| host | crate | identity | store | signal | who runs it |
|---|---|---|---|---|---|
| axum (NAS, Docker, self-host) | `yokohama-web` (feature `server`) | `claimed` / `cf-access` | `FsStore` | SSE over tokio watch | unchanged |
| browser (single-player) | `yokohama-wasm` (new; `wasm-bindgen`) | none needed | IndexedDB, via the client | none (one tab, one game) | M6.5b |
| Durable Objects (hosted multiplayer) | `yokohama-worker` (new; workers-rs) | `cf-access` under Access | the object's SQLite | hibernating WebSockets | M6.5c |

### 2.2 The route to object map (M6.5c)

The Worker resolves `/api/games/{id}/…` to `env.TABLE.idFromName(id)`. **There is no lookup
table: a table's id is its address.** A link for an id never created reaches an object with
empty storage, which answers 404 and, having written nothing, ceases to exist — probing creates
no garbage.

| route (`lib.rs:2116` `app()`) | answered by |
|---|---|
| `/api/health`, `/api/me`, `/api/labels`, `/api/art`, the static client and art | the Worker |
| `/api/games` GET (`scope=mine\|open\|playing`), `/api/stream`, `/api/chat` GET/POST | the house object |
| `/api/games` POST | the Worker draws the id (`draw_seed`'s derivation, 64-bit, independent of the setup seed), then the table object initialises create-if-absent inside its single thread and pushes its row |
| `…/{id}` GET/DELETE, `table`, `deal`, `keep`, `join`, `unlock`, `chat`, `input`, `ports`, `labs`, `areas`, `log`, `events`, `stream`, `at/{k}`, `suggest`, `why`, `index/{kind}`, `result`, `record` (R-6.5.10) | the table object |

### 2.3 The table object

- **Storage**: three rows or tables in its SQLite — the record (the same JSON
  `Session::to_record_json` writes, digest-stamped), the table document, the chat messages
  (`(seq, created_at, author_kind, author_user, author_name, text)` with `last` kept as a
  counter that survives a trim, R-6.4.9), plus `unlocked`, `archived_at`. `Version` and the
  compare-and-swap are not needed inside the object; the layer keeps `check_shape` on read and
  write because every stored field is still an input.
- **Memory**: the `Repl`, rebuilt from the record through `Session::from_record` on every wake
  — the same cold-miss path `Games::handle` (`games.rs:269`) takes today. The three things that
  do not survive a rebuild are the three M6.1 named — `Repl::sub`, `Repl::privacy`, and a move
  whose write failed — and `Input::expect` already handles a client holding a stale prompt.
- **The signal**: one hibernatable WebSocket per visible tab (R-P8 holds: a hidden tab closes
  its socket; an idle open one costs nothing), `serializeAttachment` carrying the viewer (seat
  or spectator) so a wake needs no handshake, `setWebSocketAutoResponse` for the client's
  ping. Each of the eight bump sites sends the `Events` frame to every socket; a delete sends
  the `end` frame and closes them. **A frame follows its write and never precedes it**: the
  write is awaited before the send, and DO output gates hold the HTTP reply until storage
  confirms, so an acknowledged move is durable by construction.
- **Lifecycle**: ten seconds after the last event the object hibernates between moves; two
  minutes after the last socket closes it is evicted; days later the next request re-runs the
  constructor over the same storage. The lobby says "waiting on <name>" from the row the table
  pushed at its last bump. The alarm API is the future hook for a turn notification (R-P3 keeps
  it out of scope) and for retention (a `deleteAll()` N days after archiving — a policy, not
  built).
- **The archive** (R-6.5.4, R-6.5.5): at game over and at abandon, `put` the two documents to
  the bucket; on failure set `archived_at = NULL` and an alarm; the put is idempotent.

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
table, then removes the house row. The house is the one global serialization point and at this
scale is nowhere near a limit.

### 2.5 The browser host (M6.5b)

`yokohama-wasm` exposes ONE function, request-in / response-out over the same method, path,
query and body the HTTP client already sends, so `web/src/api.ts`'s `call` (`:233`) dispatches
to it when the game is local and to `fetch` otherwise, and every component above `api.ts` is
unchanged. The record is persisted to IndexedDB after every applied move, the same "nothing
exists only in memory" rule; a reload rebuilds the `Repl` from it. There is no signal: one tab,
one game, and the bot's trailing run returns in the same call. The lobby gains "Play the bot
here" beside "New table". **The load-bearing pin**: a record played by the wasm bot must
reconstruct byte-identically under the NATIVE `why` (`record::state_at` + the same `decide` at
the same `(bot_seed, events.len())` key), which is the determinism claim WebAssembly has to
earn before anything hosted depends on it.

### 2.6 The client's transport seam (M6.5c)

`usePolled(source, active, onValue, onEnd?)` (`web/src/signal.ts`) keeps its signature; `Source`
(`:101`) gains a `ws` reader beside `sse` and `held`, selected by `/api/health`'s `signal`
field exactly as the M6.3 spec §7 chose. Identity on a WebSocket: the browser's `WebSocket`
cannot set a header either, so under Access the identity is the Access cookie on the upgrade,
and the `claimed` development door passes its email as a query parameter that the Worker
refuses whenever Access identified the caller (R-6.5.11). On reconnect the object writes the
current `Events` frame and the client's on-connect compare (M6.3 §5.1) is the recovery, since
the count is cumulative (R-3.14).

---

## 3. The three sub-cycles

Each: brainstorm → spec → plan → execution under R-P11 → deploy → review against the live
site → close → handover. Exit criteria are what the close must SHOW.

### 3.1 M6.5a — the seam

**Builds:** the table layer cut out of `Games` behind the `server` feature (§2.1); a `Clock`
seam replacing `now_unix`'s direct call; the archive as a trait with the directory
implementation (R-6.5.4) and abandon-and-save's semantics in `delete` (R-6.5.5);
`GET …/record` (R-6.5.10); "Save game file" on the finished-game screen; the CI line for the
wasm check.
**Exit:** the axum server redeployed on the NAS with **every EXISTING corpus payload
byte-identical except `health.json`, which gains exactly one key (`archive`)** — regenerate,
run without the variable, read the diff, report it: the proof that nothing moved on the wire.
The record route joins the corpus by its 409 alone: **a finished record is never committed
as a fixture**, because it carries the setup seed and the value pin
`no_committed_payload_carries_the_setup_seed` searches the whole corpus; instead that pin
gains the route's 200 body as its anti-vacuity partner, asserting IN-TEST that the record does
carry the seed and that no committed file does. `cargo check … --no-default-features
--target wasm32-unknown-unknown` green in CI; the CLI untouched; core byte-identical.

### 3.2 M6.5b — single-player in the browser

**Builds:** `yokohama-wasm` (§2.5); the local arm in `api.ts`; IndexedDB persistence;
"Play the bot here"; "Send to the archive" (R-6.5.6) on the finished-game screen, dark until a
host reports an archive; the wasm-side `SystemTime` replaced through the clock seam; the
determinism pin.
**Exit:** a whole game against the bot played in the browser with the network tab showing
ZERO `/api/games` calls after load (counted off the page's own request log, the way the M6.3
e2e counts `/events`); the save downloaded and reconstructed natively under `why` with zero
refusals; shipped on the NAS, because this host needs no server.

### 3.3 M6.5c — the room on Durable Objects

**Builds:** `yokohama-worker` (the Worker, the table object, the house object, §2.2–§2.4); the
`ws` transport arm (§2.6); the R2 archive with alarm retry; Access in front; the hostname move
(the tunnel that serves the ttyd terminal holds `yokohama.moore.nyc` today — `deploy/worker/
wrangler.jsonc`'s own warning); the runbook's new §6; the deletion of the retired code
(R-6.5.13); e2e under `wrangler dev` AND against the live deployment (R-6.5.8).
**Exit — a number, not a demonstration:** two people play a whole game through the hosted path
and a third watches; the table object is OBSERVED hibernating between moves (the dashboard's
duration metric, or the object's own constructor counting its wakes); the wall from a move to
the other browser's screen through the object measured the way M6.3 measured it through the
tunnel; a finished game and an abandoned one land in the bucket and load in the CLI; and the
corpus byte-identical against the Worker.

---

## 4. Fences, and the mutations each sub-cycle must catch

Named here so each sub-cycle's spec carries the battery; the wording of a cell is the
sub-cycle's.

- **M6.5a**: any wire change fails `wire_fixtures` (regenerate → non-empty diff); a `SystemTime`
  call anywhere in the layer fails the wasm check line; a `tokio::` or `axum::` path reachable
  without the `server` feature fails the same line; `GET …/record` at a running game is a 409
  and at a finished one its body is the exact bytes the store holds (a digest compare, so a
  re-serialization that dropped the digest fails); the seed pin's anti-vacuity partner (§3.1)
  fails if the route's 200 body stops carrying the seed, and the pin itself fails if any
  committed file starts to; abandon of a playing table with events calls the archive exactly
  once, an open one never, and an archive failure still deletes and still answers 200.
- **M6.5b**: the local arm makes no `fetch` (a `fetch` spy at zero across a whole game); a
  record with one event edited is REFUSED by "Send to the archive" (strict replay); a record
  not over is refused; the wasm bot's moves reconstruct under native `why` (a mutation that
  perturbs the decision stream key must fail it); the seed never leaves the browser except
  inside a finished record.
- **M6.5c**: a spectator's socket message that is a move is refused before the record is
  touched (the M6.3 sweep, over WebSockets); a client-supplied identity header or query
  parameter is ignored whenever Access identified the caller; the house never holds a hash
  (a value pin over the house's rows for this run's salt and hash, the M6.2 shape); a frame
  never overtakes its write (the M6.3 gated-cell shape, over the object's storage); the
  corpus byte-identical against the Worker; the object hibernates (a wake counter that MUST
  increase across a ten-second pause).

---

## 5. Deployment (M6.5c's, recorded here so the earlier sub-cycles plan for it)

- Workers Paid; one Worker (`yokohama-web`) serving the client and art as static assets, with
  two SQLite-backed object classes (`Table`, `House`) and one R2 bucket bound; Access on the
  hostname with the owner's allow-list; `cf-access` identity.
- The hostname takeover is the step that takes the ttyd tunnel's route away: give ttyd its new
  hostname first and verify it (`wrangler.jsonc`'s own note).
- Early and repeated deploys (R-6.5.8); the runbook records each with the container-era
  numbering continued.
- The NAS keeps serving the axum host throughout; nothing here retires it.

---

## 6. What this spec does NOT claim

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
- Whether workers-rs exposes `ctx.access` was not checked; R-6.5.11 does not depend on it.

---

## 7. Read order for the session that opens M6.5a

1. `CLAUDE.md` — the Web (server) and Web (browser half) bullets and the `crates/yokohama-web`
   architecture bullet.
2. `docs/handover.md` Track 3.
3. This document, §0–§2.
4. `docs/plans/2026-09-19-web-frontend-m6.5-hosting-handover.md` §2 (the seams as M6.4 left
   them) and §6 (the process).
5. The M6.5a spec, written beside this one, then its plan.
