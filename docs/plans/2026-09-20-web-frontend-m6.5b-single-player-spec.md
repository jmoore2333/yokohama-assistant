# M6.5b "single-player in the browser" — the spec

**Written 2026-09-20 on branch `web-room-m6.5b` (stacked on `web-room-m6.5` at `adfcd31`, the
M6.5a deploy record), from the brainstorm with the owner that same day, under the umbrella
`docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` (R-6.5.1–13) and from the M6.5a close
`docs/plans/2026-09-20-web-frontend-m6.5a-close.md` and its entry handover
`docs/plans/2026-09-20-web-frontend-m6.5b-handover.md`. This document is M6.5b's authority; the
plan is written from it (R-6.5.7, R-P11); the close is its record.**

**The headline.** A person opens the site and plays the bot with no server behind the page: the
whole table layer (`api::dispatch` over `Games`, the engine and the production bot inside it)
runs as WebAssembly in a Web Worker in their own browser, every game they play there lives in
their browser's IndexedDB, and the first screen shows those games FIRST and keeps showing them
when the server cannot be reached. A finished or abandoned local game can be sent to the
server's archive, where a strict replay decides what it is; a running one can be saved as a
file at any time. Nothing hosted moves; the axum binary gains two routes.

**Read order for the session that writes the plan:** §1 (the rulings — every one is an owner
decision or a stated default the owner did not object to), §2 (the mechanism, with the file
and line each piece lands on), §6 (the instruments and the mutation battery — the plan's tasks
end in these), §7 (the order of the work), §8 (what is not claimed).

---

## 0. What is true at HEAD (`adfcd31`), and what this spec relies on

- **The layer is host-free and proven under axum** (M6.5a close §1). `api::dispatch(games,
  host, identity, req) -> ApiResponse` over `ApiRequest { method, path, query, content_type,
  body }`; 28 routes; `Games::with_parts(rules, store, chat, signal, clock, archive)`;
  `MemStore`, `NoSignal`, `NoArchive` unconditional; `cargo check -p yokohama-web
  --no-default-features --target wasm32-unknown-unknown` passes and is in CI. Nothing RUNS in
  wasm yet (close §7).
- **The record carries the seed and the seed is the whole hidden set** (`game/observe.rs`;
  CLAUDE.md's play-payload rule). A hosted client can never hold a record; a local client
  holds only its own.
- **A person may hold two seats** (`tables.rs:3361`, `a_person_may_take_two_seats_and_a_named_
  seat_is_taken`), which is what lets a bot-free deterministic walk run at the local host.
- **`is_safe_id`** (`store.rs:173`): first character alphanumeric, then alphanumerics, `.`,
  `_`, `-`; at most 64; no reserved suffix. So `local-<16 hex>` is a legal id and `_local-…`
  is not.
- **`draw_seed`** (`tables.rs:212`) is `SeedSchedule::from_os_entropy()`, i.e. `rand`'s
  `OsRng`, i.e. `getrandom`, whose `js` feature the M6.5a manifest enables for `wasm32` — the
  one blocker the umbrella measured (§0.3).
- **The historical frame** is `HistoricalFrame::at(repl, k, viewer)` (`client.rs:601`) over
  `yokohama_core::state_at(record, k, rules)` (`record.rs:561`), which is `rebuild_initial_
  state` + `replay_from(&initial, &events[..k])`; `replay_from` is `pub` (`record.rs:384`).
  **The client's `fetchRun`** (`App.tsx:55`) asks `/at/{k}` for every `k` in `[prev, next]`
  plus `/log`, so an opponent's fifteen-event turn is sixteen requests and fifteen replays of
  the whole record from its start.
- **The client draws nothing until `/api/me` answers** (`App.tsx:923`, `if (!door.me) return
  null;`), and the three deployment facts (`signalKind`, `lobbyChat`/`chatText`,
  `archiveKind`) are module-level values set from ONE health read (`App.tsx:580-584`).
  `api.ts`'s `call` (`:249`) is the module's one `fetch` for the API; `sse.ts`'s `readStream`
  is the second, reached from `usePolled`'s `open()` (`signal.ts:260-292`), which branches on
  `signalKind()` into the held arm or the stream.
- **The static server's MIME map** (`assets.rs:49`, `content_type`) has no `wasm` arm — a
  `.wasm` file would be served as `application/octet-stream`, which
  `WebAssembly.instantiateStreaming` refuses.
- **The lobby** (`Lobby.tsx:246-313`) is one `usePolled` over `fetchLists` with four sections:
  Your games, Open tables, Games in progress, Lobby chat.
- **`wasm-bindgen` is `0.2.128` in the lock** (pulled by `getrandom`'s `js` feature), which
  is the version `wasm-bindgen-cli` must match.
- **The archive** (M6.5a §1.7): `Archive { put(why, id, record_json, table_json), kind() }`,
  `Why::{Over, Abandoned}`, ONE layout function `paths(rev, why, id)`, `DirArchive` refuses an
  existing path. **`delete` chooses the leaf by STATUS** (`tables.rs`, the abandon site): a
  finished game with no stamp is archived under `abandoned/` — the M6.5a deploy review's
  finding, fixed here (R-6.5b.13).

---

## 1. The rulings (owner, 2026-09-20, in the brainstorm's order)

- **R-6.5b.1 — the local player is `local@browser`.** One fixed address, never a person.
  `dispatch` is handed it for every local call; the local table document's `created_by` and
  seats carry it; `GET /api/me` served locally reports it. It appears nowhere else: the
  server's archive attributes a browser game to the SERVER's caller (R-6.5.6's
  `browser/<user>/` prefix), and no listing on any server ever prints it.
- **R-6.5b.2 — the client survives having no server (level 2), and the offline install is a
  later, isolated update (level 3).** If the health read fails, the first screen shows the
  local section and one "not connected" line where the four server sections would be, and a
  local game plays. What is NOT in M6.5b: a service worker, an installable app, offline
  delivery of the page, the module and the art. The owner wants that ("with the ability to
  cache bust what's there for later patching … installable/offline for single player") as
  its own update after M6.5b; §8 carries it as the first follow-up with its shape.
- **R-6.5b.3 — the name for save-game review rides in the table document, never in the
  record.** The record is the engine's format (its provenance names controllers, not people)
  and is what the terminal loads; "Save game file" stays the record alone. A local game keeps
  a local TABLE document beside its record, whose seats carry the display name the browser
  remembers, and "Send to the archive" sends both — the same pair the NAS archive writes for
  a hosted game. The fenced record format does not move.
- **R-6.5b.4 — the module runs in a dedicated Web Worker, one per tab** (a stated default the
  owner did not object to). The Worker owns the module, the local `Games`, the IndexedDB
  side channel and the lock; the page speaks to it in one request and one reply shape.
  Reason: the replay on load rebuilds a state from seed and events, grows with the record,
  and would freeze the screen on the main thread; `suggest` and `why` run the same machinery.
- **R-6.5b.5 — two tabs on one local game are settled by the Web Locks API at OPEN time**, with
  a Take over control ("Web lock is ideal, we can build for that"). A tab holds an exclusive
  lock on the game id while the game is open; a second tab opening it sees "This game is open
  in another tab" and Take over, which steals the lock and tells the first tab to stop. One
  writer at a time, told before a move rather than after. `navigator.locks` exists in
  dedicated workers and in every browser the site is played on, Chrome on Android included.
- **R-6.5b.6 — a local game can be abandoned AND saved, through the server's archive, and the
  server decides what it received.** "Abandon and save" on a local game posts the record and
  the local table document to `POST /api/archive` BEFORE the local delete. The server parses
  both, gates `semantics_rev`, requires a terminal digest, replays strictly, and **chooses the
  leaf from the replayed state — `over/` when the game is over, `abandoned/` otherwise — never
  from what the client says.** This AMENDS umbrella R-6.5.6's fourth check ("game over"): a
  partial record is accepted, because a partial record carries a digest too (the store stamps
  one at every write and the Worker writes records through the same function) and strict
  replay verifies it as it does a finished one. The umbrella §4 fence "the seed never leaves
  the browser except inside a finished record" is re-worded: **the seed leaves the browser
  only inside a record the player sends to the archive or saves as a file, and never on any
  wire payload.**
- **R-6.5b.7 — when the archive cannot be reached, the abandon is refused and the game kept,
  and a local game's record can be saved as a file at ANY point.** "The archive could not be
  reached — the game was kept" is the server's own rule for a failed put, applied at the
  client. "Save game file" on a local game is offered while it runs, since it is the player's
  own seed on the player's own disk (R-6.5b.6's fence). A hosted game's file save stays at
  game over (R-6.5.10 is untouched — there the record crosses a wire to other parties).
- **R-6.5b.8 — a fifth section, "On this device", at the top of the first screen, and the
  architectural guarantee that local games are reachable there with the server down.** The
  section's rows come from the local dispatch's own `GET /api/games?scope=mine`; its action is
  "Play the bot here". The first screen renders from the browser's state FIRST and the server
  sections fill in when the server answers (§2.5); the cell that holds it: with every `fetch`
  failing, the section renders with its games and Play the bot here works.
- **R-6.5b.9 — the host is a new crate, `crates/yokohama-wasm`** (approach 1 of three). A
  `cdylib` over `yokohama-web` with `default-features = false`, built with `wasm-bindgen`.
  `yokohama-web` stays exactly what M6.5a made it: a library that knows no host, with its two
  region kinds and its scanning cell untouched. Ruled out: a `wasm` feature on the layer crate
  (bindgen glue and a `cdylib` crate type inside the layer, a third region kind); a raw ABI
  with hand-written glue (`getrandom`'s `js` feature already pulls `wasm-bindgen` in, so the
  glue is generated regardless).
- **R-6.5b.10 — the batch run route lands FIRST, on the axum host, as Task 1.** One new route
  answers the frames of `[from, to)` in one reply computed by ONE forward pass, the client's
  `fetchRun` makes one call, and `/at/{k}` stays for the scrubber. The owner's reasoning:
  it should come in "before we get that first build up of the package" — proven by the
  instruments M6.5a left (the corpus, the route cells, the dispatcher cell) and inherited by
  every host, and the O-3 replay measurement then reads the shipped path. Its fence against
  drift: one route, one builder, one client call, nothing in a fenced crate. It is deployable
  to the NAS on its own if the owner wants the walk faster there before the rest lands.
- **R-6.5b.11 — push instead of fetch is M6.5c's, named here and not designed here.** In the
  object, (1) push each seat's `ClientState` on its socket instead of a signal followed by a
  GET, and (2) push chat text instead of a count followed by a GET (a per-socket queue
  coalesces nothing, so M6.4's reason for the count does not apply there). Carried in §8 for
  the M6.5c spec. The wasm-in-a-Worker shape changes nothing about a hosted move's cost: a
  hosted frame is computed where the record is.
- **R-6.5b.12 — a local game's id starts with `local-`** (a stated default). The Worker draws
  `local-<16 hex>` and supplies it on every create; the client's one predicate `isLocal(id)`
  is that prefix, DEFINED once in `api.ts`, and every reader (`call`, `recordText`, the
  transport arm, the screen's source record) goes through it — one definition, never a flag
  kept in sync. **At its real size:** a server table
  CAN be given that prefix by a hand-written request (the prefix is `is_safe_id`-legal and
  the layer reserves nothing), in which case the page routes it to the Worker, which answers
  404. A confusion the browser's own client never causes, never a leak. A reservation in
  `is_safe_id` was considered and rejected: the local host runs the same function.
- **R-6.5b.13 — `delete` chooses the archive leaf by the mirror's truth.** In `tables.rs`'s
  abandon site, `Why::Over` when `doc.turn` says the game is over, `Why::Abandoned` otherwise
  — one line and one cell. The M6.5a deploy review found a finished game with no stamp
  (finished before that release, or whose game-over put failed) archived under `abandoned/`;
  the leaf lied and the archived table's `turn.over` told the truth. Fixed here rather than as
  a hot change on a reviewed branch.
- **R-6.5b.14 — `.wasm` is served as `application/wasm`.** One arm in `assets::content_type`
  and one assertion in its cell; without it `instantiateStreaming` refuses the module.

---

## 2. The mechanism

### 2.1 `crates/yokohama-wasm`

A `cdylib` (`crate-type = ["cdylib"]`), `publish = false`, depending on `yokohama-web =
{ path, default-features = false }`, `wasm-bindgen` (the lock's `0.2.128`), `js-sys` (the
clock). It holds ONE `Games`, built once at `init()`:

```
Games::with_parts(Arc::new(Rules::standard()), Arc::new(MemStore::default()),
                  ChatConfig::default(), Arc::new(NoSignal), <clock>, Arc::new(NoArchive))
```

and exports exactly these:

- `init()` — builds the above; a second call is a no-op.
- `dispatch(user: string, method: string, path: string, query: string, content_type:
  string, body: Uint8Array) -> { status: number, body: string }` — builds an `ApiRequest` and
  calls `api::dispatch` with `Host { identity_name: "local", signal_kind: "none" }` and
  `Identity = Ok(&User::parse(user))`. **The identity is a parameter so a test can play the
  corpus walk as its own five people** (§6.3); the Worker passes `local@browser` and nothing
  but the Worker can reach the module.
- `load(doc: "record" | "table" | "chat", id: string, text: string)` — writes the document
  into the store with no version check, for the Worker's boot (§2.2). It bypasses `Games` on
  purpose: a stored pair is a state the player was shown, and `Games` will rebuild the `Repl`
  from it on first touch through `Session::from_record` exactly as the axum host rebuilds an
  absent handle (`games.rs:406`).
- `read(doc, id) -> string | undefined` — the store's exact bytes, for the side channel.
- `set_clock(seconds: number | undefined)` — a frozen reading, or `undefined` for the JS
  clock (`js_sys::Date::now() / 1000`). The `Clock` handed to `with_parts` reads an
  `AtomicU64` mode + value; nothing in the layer knows which.

The chat config is the default; `chat_append` and the house routes answer, and nothing on
the client asks them for a local game (§2.5). `NoArchive` is the local archive: `/api/health`
served locally reads `"archive": "off"`, and R-6.5b.6's send goes to the SERVER's route.

**Build.** `cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm`, then
`wasm-bindgen --target web --out-dir web/public/wasm/ …` (and `--target nodejs --out-dir
web/wasm-node/` for §6.3). Both output directories are gitignored; `web/package.json` gains
`build:wasm` and `build` runs it before `vite build`. `wasm-bindgen-cli` is installed at the
lock's version (`cargo install wasm-bindgen-cli --version 0.2.128 --locked`), and the plan's
first task records the module's size (raw and gzip) beside the umbrella's 1.30 MB / 387 KB —
RECORDED, never gated. The `web-server` CI job, which already installs the pinned toolchain
with the target, builds the module and runs §6.3's replay; the `web-client` job stays
node-only.

### 2.2 The Worker (`web/src/local/worker.ts`)

One dedicated Worker per tab, started lazily on the first local call and kept for the page's
life. It loads the module from `/wasm/`, calls `init()`, opens IndexedDB (`yokohama-local`,
one object store `docs` keyed by `[doc, id]` holding the text), `load`s every stored pair, and
only then answers messages.

- **Messages.** Request `{ seq, method, path, query, body }` (body a string or `null`;
  content type `application/json` when a body is present) → reply `{ seq, status, body }`.
  The same five fields as `ApiRequest` and the two of `ApiResponse`; the Worker adds
  `local@browser` and nothing else.
- **Persistence is the Worker's, after the reply.** After any reply to a `POST` or `DELETE`
  on a game path, the Worker `read`s that game's record and table and writes both to
  IndexedDB in one transaction, or removes both after a 204 on `DELETE`. The module never
  knows. **Written after the acknowledged write and never before**: what IndexedDB holds is
  always a state the player was shown, which is what makes §4's trap recovery honest.
- **The lock.** `navigator.locks.request("yokohama:local:<id>", { mode: "exclusive" }, …)`
  is taken when the page opens a local game and released when it leaves; the second tab's
  `request` with `{ ifAvailable: true }` failing is what "This game is open in another tab"
  reads, and Take over is `{ steal: true }`, whose effect on the first holder (its promise
  rejects with `AbortError`) is what turns that tab read-only (§4).
- **Ids.** The Worker rewrites a create's body to carry `id: "local-<16 hex>"` drawn from
  `crypto.getRandomValues`; the seed is NOT supplied — `draw_seed` inside the module draws it
  through `getrandom`'s `js` feature, and it is never typed (umbrella §3.2).

### 2.3 The client's local arm

- `api.ts`: `isLocal(id)` is the `local-` prefix. `call` routes to the Worker when the path's
  game id is local and to `fetch` otherwise; under the Worker arm `identityHeaders()` is not
  read. `recordText` reads the record for a local game from the Worker's side channel (a
  `read` message), never from the `…/record` route, which the local dispatch still answers
  exactly as the corpus pins it (a 409 while running — umbrella §2.5).
- `signal.ts`: `open()` gains a third arm for `signalKind() === "none"`: no stream, no held
  request, nothing to wait for — the bot's trailing run returns inside the reply, as the
  whole-game gate plays it today. The `"sse" | "held"` union at `api.ts:193` gains `"none"`.
- **Deployment facts per source.** The three module-level values become a small record keyed
  by source (`server` | `local`): the local record is a constant (`signal: "none"`, no lobby
  chat, `archive: "off"`); the server record is set from the health read as today. A game
  screen reads its own source's record through `isLocal(id)`.
- `App.tsx`'s `fetchRun` makes ONE call to the run route (§2.4) plus `/log`, on every host —
  and PAGES a run longer than the route's cap in cap-sized calls (a spectator reconnecting
  after a long absence is the case; one call is the common one).

### 2.4 The two server routes (the layer; unconditional; both hosts)

- **`GET /api/games/{id}/at?from=&to=`** (R-6.5b.10) — the `HistoricalFrame`s of every `k` in
  `[from, to)`, as `{ frames: [...] }`, each frame byte-equal to `/at/{k}`'s. Built by
  `HistoricalFrame::run(repl, from, to, viewer)`: `state_at(from)` once, then `replay_from`
  ONE event at a time with a frame built from each intermediate state, so `HistoricalFrame::
  at` becomes `from_state(state_at(k))` and the two share one builder. Admission is `at`'s
  (`admitted`, the viewer's own seat). `from <= to <= len` or a 400 in axum's words for a bad
  number; a run longer than **64** frames is a 400 naming the cap (never a silent clamp — the
  M6.3 `wait` rule). `EventsQuery`'s sibling, `RunQuery`, carries `deny_unknown_fields`.
- **`POST /api/archive`** (R-6.5b.6) — body `{ "record": "<text>", "table": "<text>" }`, both
  strings, both parsed by the SERVER. In order: `record.semantics_rev == SEMANTICS_REV` or
  400 "this record is at semantics rev N; this server archives rev 7"; a `terminal_digest`
  present or 400 "the record carries no terminal digest"; `replay_strict` clean or 400 "the
  record does not replay: <the engine's sentence>"; the table document parses and `check_
  shape`s against the record (players, seats, keeps) or 400; then the leaf from the replayed
  state (`over/` when `is_over`, else `abandoned/`) and `archive.put` under the prefix
  `browser/<caller>/` — `paths(rev, why, id)` gains a prefix argument with `""` for the
  server's own games, so it stays the ONE layout function (umbrella R-6.5.4) — answering 201
  `{ "leaf": "over" | "abandoned", "path": "…" }`; an existing path is 409 "already
  archived"; no archive on this host is 404 "this server keeps no archive". The route is in
  the layer so a Worker host in M6.5c serves it unchanged; under `NoArchive` (the local host)
  it is the 404 by construction.

### 2.5 The first screen (R-6.5b.2, R-6.5b.8)

- The `if (!door.me) return null` at `App.tsx:923` goes. The app renders its frame, then the
  local section as soon as the Worker has loaded its store, then the door and the server
  sections as `/api/me` and `/api/health` answer. The sign-in form under `claimed` mode is the
  server area's content, not a gate on the page.
- **"On this device"** is the first section: rows from the local `GET /api/games?scope=mine`
  (the lobby's own row shape and wording — "your turn", "over · you won"), each a link to
  `#/t/local-…`; the action "Play the bot here" opens the New-table form fixed to two seats,
  you and the bot, with colours and who starts as today (3P and 4P bots are out of scope,
  R-P10), and its submit goes through the Worker.
- **The server sections** stay the one `usePolled` they are; when the health read fails they
  are replaced by one "not connected" line, and the archive-dependent controls (Send to the
  archive, "and save") are dark.
- **A local game's screen** is the game screen with three differences the id decides: no
  stream and no live strip; "Save game file" at any point (R-6.5b.7); Abandon reading
  "Abandon and save" when the SERVER's archive is on. No Chat panel (the house is the
  server's and a local table has one person); no Unlock, no watchers; undo is the CLI's own,
  as at a one-person table.
- **"Send to the archive"** on a finished local game's header, dark unless the server's
  health reported an archive; on 201 it says where the copy went.

### 2.6 What a local game never does

Talk to the server for play (§6.2's `fetch` spy), carry the seed on any wire payload (the
corpus pins hold — the local dispatch is the same code), or leave the device except by the
player's own send or save (R-6.5b.6).

---

## 3. What a player sees, in one paragraph each

**First screen.** "On this device" at the top with your local games and Play the bot here;
under it the lobby as before. Server down: the same top section, one line saying "not
connected" below it. **Playing.** The table as always; the bot's turn walks; nothing on the
network. **Two tabs.** The second says the game is open elsewhere and offers Take over; the
first goes read-only with Take back. **Finished.** Save game file, Send to the archive (when
the server keeps one). **Abandoning.** "Abandon and save" sends the record first and refuses
if it cannot; a running game can be saved as a file first.

---

## 4. Error handling

- **The module fails to load**: the local section says so in one line naming the reason;
  the server sections are untouched.
- **A trap inside `dispatch`**: the Worker answers a 500 in the API's `{"error": …}` shape,
  reloads the module and `load`s the stored pairs, and the page's strip says "reloaded from
  the last saved move". Nothing acknowledged is lost (§2.2's write-after rule).
- **IndexedDB refuses a write** (quota, a private window): the move is shown with the
  server's own sentence, "APPLIED BUT NOT SAVED" — one rule, one wording, both hosts.
- **The lock is stolen**: the losing tab's game goes read-only with "opened in another tab"
  and Take back; a move attempted after the loss is refused in the page before it reaches the
  Worker.
- **The archive's refusals** are shown as sent (rev, no digest, replay failed, table shape,
  already archived) plus "the archive could not be reached". The game stays in every case
  but one: "already archived" on an abandon means an earlier send landed and only the delete
  failed, so the abandon proceeds.
- **No server**: the one line, the local facts, the archive controls dark.

---

## 5. The corpus

- `run.json` — the run route at the walk's mid-game stop, `[k-3, k)`, and `run_400_cap.json`.
- `archive_post.json` (201 for a finished record, `leaf: "over"`), `archive_post_abandoned.
  json` (201 for a partial record, `leaf: "abandoned"`), and the four refusals `archive_400_
  rev.json`, `archive_400_digest.json`, `archive_400_replay.json`, `archive_409.json` — the
  walk's own record, edited in memory for each.
- **`walk.json` — NEW KIND: the walk's request script**, emitted by `wire_fixtures.rs` beside
  the payloads: for every `call_as`, `{ who, method, path, query, body, fixture }` in order.
  It is the input §6.3 replays through the module. Committed and read like the rest; a
  regeneration rewrites it with the payloads.
- `health.json` is unchanged for the axum host; the local host's health is
  `identity: "local"`, `signal: "none"`, `archive: "off"` — the three host-parameterized
  fields R-6.5.9 names, and the ONLY fields §6.3 exempts.
- Every `ClientState`, `HistoricalFrame` and `TableView` fixture stays byte-identical — the
  cut proves nothing moved on the wire.

---

## 6. The instruments, and the mutation battery

### 6.1 Layer cells (`yokohama-web`)

- The run route: frames equal `/at/{k}`'s BY VALUE over a run with a change in its middle
  (anti-vacuity: the run spans a turn boundary and its middle frame differs from both
  neighbours); `from > to`, `to > len` and a run over the cap as 400s in axum's words; a
  spectator admitted exactly as at `at/{k}` (one grid row).
- `POST /api/archive`: the four refusals by four DIFFERENT sentences; the leaf chosen from the
  replayed state with a body whose table says the opposite; a partial record ACCEPTED under
  `abandoned/`; once per id (409); the `browser/<user>/` prefix through `paths()`; a record
  round-tripped through `JSON.parse`-shaped rounding (its seed altered) refused by the replay.
- R-6.5b.13: a finished unstamped game deleted lands under `over/`.
- R-6.5b.14: `content_type("x.wasm") == "application/wasm"` in the existing cell.

### 6.2 Client cells (vitest)

- `isLocal` routes `call` to the Worker and everything else to `fetch`; a **`fetch` spy at
  zero across a whole local game, the stream reader included**, with `POST /api/archive` and
  the server's health read as the ONLY named exemptions (umbrella §4).
- The boot path with EVERY fetch failing renders "On this device" with its games and Play the
  bot here works — R-6.5b.8's guarantee, the cell that must fail under the pre-M6.5b
  `return null`.
- The lock's three screens (open, opened elsewhere, taken over) and the refused move after a
  loss.
- Save game file at any point on a local game and only at game over on a hosted one.
- A refused abandon keeps the game; "already archived" proceeds.
- `recordText` for a local game reads the side channel and never parses (the existing cell's
  local twin).

### 6.3 The wasm corpus replay (the browser host's corpus claim)

A vitest with `@vitest-environment node` (`web/src/local/corpus.test.ts`) loads the module
built for Node, calls `set_clock` with `FROZEN_CREATED_AT` (the one constant `freeze_clock`
writes into every `created_at` and page cursor, so the replies need no normalisation — and if
the walk's listing order turns out to have depended on DISTINCT real stamps, the replay applies
the same normalisation and the plan records it), and replays `walk.json`
through `dispatch` with each call's OWN identity, asserting every captured reply deep-equals
the committed file it named — the whole corpus except `health.json`'s three host fields.
Because it is the same walk through the compiled module, it is also a bot-free determinism
claim across the compilation boundary. It REFUSES loudly when the module is absent, naming
`npm run build:wasm`; `YOKOHAMA_WASM_OPTIONAL=1` turns absence into a skip that says so (the
BGA-corpus shape). **CI:** the `web-server` job builds the module and runs the replay; the
`web-client` job sets the opt-out, so there it is a skip that says so and never a silent green.

### 6.4 The determinism pin (umbrella §2.5, O-3, O-11)

A record played by the wasm bot IN A REAL BROWSER — produced by §6.5's exit walk and
COMMITTED at `crates/yokohama-web/tests/fixtures/browser-played-2p.json` with a sidecar
naming the release, the date, the browser and the module's hash — is driven through the
layer's own `why` route at EVERY bot event natively; a refusal fails it, and the search
counters the payload carries are read with `placement_cap_binds` asserted zero as the scope
partner. The same cell times `Session::from_record` on that record natively, and the e2e
records the Worker's replay wall on reload (O-3), both RECORDED. Regeneration is one
documented command.

### 6.5 e2e (real Chromium, the release binary)

- **The exit**: a whole game against the bot in the browser with `/api/games` requests counted
  ZERO after load, off the page's request log (the M6.3 shape); the download; Send to the
  archive against the binary started with a temporary `YOKOHAMA_WEB_ARCHIVE_DIR`, the archived
  file under `browser/<user>/rev7/over/` byte-equal to the download.
- Two TABS of ONE context (IndexedDB is per profile; two contexts would prove nothing) with
  the takeover and the first tab read-only.
- The first screen with the server refused (a route that 503s `/api/*`): "On this device"
  renders, a local game plays.

### 6.6 The mutation battery (the spec's own, run by an agent at the close)

The umbrella §4's M6.5b list, and: the run builder replaying from scratch per step (RECORDED
by the O-3 timing, not caught — no cell asserts wall); a frame off by one in the run (§6.1);
the `return null` restored (§6.2); the lock not stolen (§6.5); a server id routed to the
Worker (§6.2); the `wasm` MIME arm removed (§6.1); the leaf taken from the body (§6.1); a
partial record refused (§6.1); the walk's identity ignored by `dispatch` (§6.3 fails on the
first per-seat payload).

---

## 7. The order of the work (the plan's tasks, named)

1. The run route, `HistoricalFrame::run`, `fetchRun` as one call, `run.json`; R-6.5b.13's
   leaf fix. Deployable alone.
2. `POST /api/archive` with the prefix argument on `paths()`; its corpus files.
3. `crates/yokohama-wasm`, the build scripts, CI, the `.wasm` MIME arm; the size recorded.
4. The Worker: messages, IndexedDB, the lock; `api.ts`'s local arm; `signal.ts`'s `none`
   arm; per-source deployment facts.
5. The first screen: the boot path, "On this device", Play the bot here.
6. The local game screen's three differences; Save game file; Abandon and save; Send to the
   archive.
7. `walk.json` from `wire_fixtures.rs`; the Node replay.
8. The e2e walk; the committed browser-played record; the determinism pin; O-3 recorded.
9. Docs: the help panel, RUNBOOK §1 (the wasm build line) and §3.13, CLAUDE.md, the board;
   the close.

Process: R-P11 as refined 2026-09-20 — small mechanical tasks inline with the per-edit gates
after each, one Opus review at the structural mid-point (after Task 4) and one whole-branch
review before the release; full SDD only for a task the plan marks large or judgment-heavy
(Task 4 is the candidate).

---

## 8. What this spec does NOT claim, and the LEAVE list

- **Not installable, not offline.** Level 3 — a service worker caching the shell, the module
  and the art with a cache keyed to the release and bustable for a patch, an update prompt,
  about 8 MB per browser — is the FIRST follow-up after M6.5b, its own brainstorm and spec.
- **Nothing hosted.** No Workers, no objects. M6.5c inherits R-6.5b.11's two push shapes and
  the archive route unchanged.
- **Two seats only**, you and the bot. A local hot-seat human-versus-human is free in the
  layer and is not offered.
- **The corpus replay proves the walk, not the bot**; the bot's determinism is proven on ONE
  committed browser-played record (§6.4). O-11 stands: 2P only.
- **The lock is per browser profile.** Two profiles on one machine are two IndexedDBs: no
  conflict, and no sharing.
- **A local game's data leaves the device only by the player's own send or save.** There is
  no sync and no backup; clearing site data deletes the games.
- **O-2 and O-7** (module start-up in a Worker; `getrandom` inside a Cloudflare Worker) are
  M6.5c's; M6.5b measures the browser Worker's start-up and records it.
- Two cosmetic items carried from the M6.5a deploy review: an already-archived finished
  game's row reads "Abandon and save"; "and save" on a just-dealt table (M6.5a §6.3 item 4).
