# M6.5b "single-player in the browser" — the spec

**Written 2026-09-20 on branch `web-room-m6.5b` (stacked on `web-room-m6.5` at `adfcd31`, the
M6.5a deploy record), from the brainstorm with the owner that same day, under the umbrella
`docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` (R-6.5.1–13) and from the M6.5a close
`docs/plans/2026-09-20-web-frontend-m6.5a-close.md` and its entry handover
`docs/plans/2026-09-20-web-frontend-m6.5b-handover.md`. This document is M6.5b's authority; the
plan is written from it (R-6.5.7, R-P11); the close is its record. Revised twice after an Opus
adversarial read of the first draft against HEAD (2 Critical / 10 Important / 13 Minor, 41 claims
verified true) and that reviewer's scoped re-read of the fix wave (24 addressed, 1 partly, 0 not;
1 new Critical, 3 new Important, 6 new Minor, all fixed in the second wave), and a third time
after the owner's own lightweight-agent read (one execution-critical boundary both prior reviews
missed, one under-specified lifecycle, one unspecified failure surface, one supersession gap, six
one-sentence items), and a fourth time after the owner's pre-execution review of the PLAN
(`docs/plans/2026-09-20-web-frontend-m6.5b-review-free-tier-offline-saves.md`: eight corrections,
two of them reproduced; every one verified here and taken — §10 is the disposition of each); the
two Opus reports are in the gitignored `.superpowers/sdd/2026-09-20-web-frontend-m6.5b/` — §9 and
§10 list what the four reads changed.**

**The headline.** A person opens the site and plays the bot with no server behind the page: the
whole table layer (`api::dispatch` over `Games`, the engine and the production bot inside it)
runs as WebAssembly in a Web Worker in their own browser, every game they play there lives in
their browser's IndexedDB, and the first screen shows those games FIRST and keeps showing them
when the API cannot be reached. A finished or abandoned local game can be sent to the server's
archive, where a strict replay decides what it is; a running one can be saved as a file at any
time. Nothing hosted moves; the axum binary gains two routes.

**Read order for the session that writes the plan:** §1 (the rulings — every one is an owner
decision or a stated default the owner did not object to), §2 (the mechanism, with the file
and line each piece lands on), §6 (the instruments and the mutation battery — the plan's tasks
end in these), §7 (the order of the work), §8 (what is not claimed), §9 (what the review
changed, so a sentence that reads oddly can be traced).

---

## 0. What is true at HEAD (`adfcd31`), and what this spec relies on

Every line below was opened by the review; the ones it corrected are marked.

- **The layer is host-free and proven under axum** (M6.5a close §1). `api::dispatch(games,
  host, identity, req) -> ApiResponse` (`api.rs:1749`) over `ApiRequest { method, path, query,
  content_type: Option<_>, body }`; a route table of 28 rows (`api.rs:129-162`);
  `Games::with_parts(rules, store, chat, signal, clock, archive)` in that order
  (`games.rs:287-294`); `MemStore`, `NoSignal`, `NoArchive` unconditional; `cargo check -p
  yokohama-web --no-default-features --target wasm32-unknown-unknown` passes and is in CI.
  Nothing RUNS in wasm yet (close §7).
- **The record carries the seed and the seed is the whole hidden set** (`game/observe.rs`).
  A hosted client can never hold a record; a local client holds only its own.
- **A person may hold two seats** (`tables.rs:3368`, `a_person_may_take_two_seats_and_a_named_
  seat_is_taken`). It is what would let a local hot-seat table exist (§8 declines to offer
  one); it is NOT why the corpus walk can run locally — that is the export's identity parameter
  (§2.1). *(corrected: M-8)*
- **`is_safe_id`** (`store.rs:173`): first character alphanumeric, then alphanumerics, `.`,
  `_`, `-`; at most 64; no reserved suffix. So `local-<16 hex>` is a legal id and `_local-…`
  is not. **`NewGame.id`'s doc (`tables.rs:158-161`) says "the browser never does" set it** —
  R-6.5b.12 makes that false and the line moves with it. *(M-7)*
- **`User::parse`** (`identity.rs:106-125`) refuses an empty, over-long, whitespace-bearing or
  malformed address and NOTHING else: `/`, `.` and `..` pass. An address is not path-safe,
  and today no path is built from one. *(C-1)*
- **`draw_seed`** (`tables.rs:212`) is `SeedSchedule::from_os_entropy()`, i.e. `rand`'s
  `OsRng`, i.e. `getrandom`, whose `js` feature the M6.5a manifest enables for `wasm32` — the
  one blocker the umbrella measured (§0.3).
- **The historical frame** is `HistoricalFrame::at(repl, k, viewer)` (`client.rs:601`) over
  `yokohama_core::state_at(record, k, rules)` (`record.rs:561`), which is `rebuild_initial_
  state` + `replay_from(&initial, &events[..k])`; `replay_from` is `pub` (`record.rs:384`,
  `&[RecordedEvent]`) and everything `at` needs beyond the state is on the `repl` or computed
  from the state (`client.rs:659-660`). **`at/{k}` refuses a position past the end with a 404
  in THIS API's words** (`api.rs:1669-1673`: "this game has {total} events, so its last
  position is {total} — there is no position {k}"); a non-numeric `k` is a 400 in axum's.
  *(I-7)* **The client's `fetchRun`** (`App.tsx:55-62`) asks `/at/{k}` for every `k` in
  `[prev, next]` INCLUSIVE plus `/log`, so an opponent's fifteen-event turn is seventeen
  requests and sixteen replays of the whole record from its start.
- **The client draws nothing until the door answers, through FOUR gates** (`App.tsx:922-926`):
  `door.error` → an error screen; `!door.me` → `null`; `claimed` with no user → the `SignIn`
  form; no user under another mode → the door sentence. **With every `fetch` failing it is the
  FIRST gate that fires**, because `useMe` (`whoami.ts:50-58`) sets `error` on a rejected
  `api.me()`. *(I-3)* The four deployment facts are module-level values in `api.ts` set from
  ONE health read (`App.tsx:577-584`).
- **`api.ts` holds TWO `fetch` calls** — `call` (`:251`, behind which the `api` object's
  members sit) and `recordText`'s own (`:366`, which reads the body as text and never parses
  it, M6.5a §1.9) — and `sse.ts`'s `readStream` (`:102`) is the THIRD, reached from
  `usePolled`'s `open()` (`signal.ts:260-292`), which branches on `signalKind() === "held"` at
  `:270`. `open()` holds a `Source<T>` (`signal.ts:101-121`: `key`, `once`, `stream`, `frames?`)
  and knows no id and no host. *(I-4, I-6)*
- **The static server's MIME map** (`assets.rs:49-63`, `content_type`) has no `wasm` arm — a
  `.wasm` file would be served as `application/octet-stream`, which
  `WebAssembly.instantiateStreaming` refuses; the cell to extend is at `assets.rs:124`.
- **The lobby** (`Lobby.tsx:258-313`) is one `usePolled` over `fetchLists` (`:143`, `active`
  hard-coded `true`) with four sections: Your games, Open tables, Games in progress, Lobby chat.
  `api.art()` and `api.labels()` fall back to `null` on refusal (`App.tsx:561-565`) and every
  consumer degrades to raw serde names and no pictures. *(M-4)*
- **`wasm-bindgen` is `0.2.128` in the lock** (`Cargo.lock:821`; `js-sys 0.3.105`, `getrandom
  0.2.16`), the version `wasm-bindgen-cli` must match.
- **The store**: `Store::write_if_version(doc, id, json, expected)` (`store.rs:133`) with
  `expected: None` CREATES and refuses a present document — there is no versionless
  overwrite. **`Games` caches a resident `Repl` in `live` and its base version in `written`**
  (`games.rs:126-134`), and the only eviction, `evict_for_test` (`:744`), is `#[cfg(test)]`.
  **A cold `live_handle` PUMPS the bot** (`games.rs:405-424`): after `Session::from_record`,
  `bring_alive` plays a bot to move and `persist`s if it did — so a GET can change the record.
  *(C-2, I-10)*
- **`mirror_turn`** (`games.rs:710-718`) swallows its own write failure by design, so
  `TableDoc.turn` is a cache that may be one move stale. *(M-3)*
- **The archive** (M6.5a §1.7): the TRAIT is `put(&self, why, id, record_json, table_json)`
  (`archive.rs:44`) and `paths(rev, why, id)` is called INSIDE each implementation
  (`archive.rs:138`), not by the route; `DirArchive::write_new` (`:120-133`) joins the computed
  relative path onto the root with no traversal guard, because today the only string in that
  path is a game id under `is_safe_id`. **`delete` chooses the leaf by STATUS** (`tables.rs:
  1677-1697`): a finished game with no stamp is archived under `abandoned/` — the M6.5a deploy
  review's finding, fixed here (R-6.5b.13). *(I-5, C-1)*
- **The corpus** is `corpus()` = `walk()` + `constructed()` + `art_manifest` + `site_space_
  words` (`wire_fixtures.rs:1710-1720`); `constructed()` is `watching.json` and `locked.json`,
  "the two payloads the router cannot produce"; `me_signed_out.json` is captured with NO
  identity (`:899`, `get_anonymous`); `house_chat_off_404.json` from a SECOND router built with
  `lobby: false` (`:1510`); `api::me` reports `host.identity_name` as `mode` (`api.rs:286-288`),
  so `me.json` and `me_signed_out.json` are host-parameterized. `freeze_clock` (`:337-375`)
  normalises every `created_at` and the page cursor to `FROZEN_CREATED_AT = 1_789_000_000`
  (`:318`); the walk's `Api` uses the REAL clock (`lib.rs:385`), and the committed listings show
  `wire` before `wire2` at an identical stamp — the id tie-break, which a frozen clock
  reproduces by construction. The cell `the_committed_wire_fixtures_are_what_the_server_emits`
  (`:2364-2377`) names as orphaned any `*.json` in the directory that `corpus()` does not emit.
  *(I-1, M-10)*
- **The `why` payload carries all fifteen counters** including `placement_cap_binds`
  (`web/src/wire/why.json`, `explain.rs:279`). **The store stamps a terminal digest at EVERY
  write**: `Session::to_record_json` (`session.rs:419-427`) is the one writer both web sites
  go through (`games.rs:611`, `tables.rs:1215`), so a PARTIAL record carries one, and
  `replay_strict` (`record.rs:478-508`) checks the rules hash, then `semantics_rev` (a `None`
  is its own failure: "missing semantics provenance …"), then every event with invariants,
  then the digest. *(M-12)*
- **The scanning cell** `platform_calls_stay_behind_the_server_feature` reads
  `crates/yokohama-web/src` only (`lib.rs:6187`); a new crate is outside it. *(M-11)*
- **`vitest.config.ts`** applies `setupFiles: ["./src/test/setup.ts"]` to every file, and that
  file touches `Element` and `window` at module scope; `include` is `src/**/*.test.ts(x)`. The
  `web-server` CI job installs the pinned toolchain with the wasm target and runs two cargo
  commands; it has no node. *(I-8)*

---

## 1. The rulings (owner, 2026-09-20, in the brainstorm's order; the review's additions last)

- **R-6.5b.1 — the local player is `local@browser`.** One fixed address, never a person.
  `dispatch` is handed it for every local call; the local table document's `created_by` and
  seats carry it; `GET /api/me` served locally reports it. It appears nowhere else: the
  server's archive attributes a browser game to the SERVER's caller (R-6.5.6's `browser/…/`
  prefix, encoded per R-6.5b.15), and no listing on any server ever prints it.
- **R-6.5b.2 — the client survives having no reachable API (level 2), and the offline install
  is a later, isolated update (level 3).** If the health read fails — the page already loaded,
  the API refusing or not answering — the first screen shows the local section and one "not
  connected" line where the four server sections would be, and a local game plays. **What
  level 2 does NOT cover: the origin itself down**, where the page, the module and the art
  cannot be fetched at all. That is level 3 — a service worker, an installable app, offline
  delivery — which the owner wants ("with the ability to cache bust what's there for later
  patching … installable/offline for single player") as its own update after M6.5b; §8
  carries it as the first follow-up with its shape. *(M-5)*
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
  writer at a time, told before a move rather than after. **And the lock is only half of it:
  a tab that acquires the lock REBUILDS its store from IndexedDB at that moment (§2.2), because
  a Worker that loaded its pairs at boot is stale by every move another tab has played since.**
  `navigator.locks` is on `WorkerNavigator`; support (Chrome 69, Firefox 96, Safari 15.4,
  Chrome on Android) is from compatibility tables and is MEASURED on the owner's phone in
  Task 4, per R-6.5.8. *(C-2)*
- **R-6.5b.6 — a local game can be abandoned AND saved, through the server's archive, and the
  server decides what it received.** "Abandon and save" on a local game posts the record and
  the local table document to `POST /api/archive` BEFORE the local delete. The server parses
  both, gates `semantics_rev`, requires a terminal digest, replays strictly, checks the table
  against the record, and **chooses the leaf from the replayed state — `over/` when the game
  is over, `abandoned/` otherwise — never from what the client says.** This AMENDS three
  umbrella sentences, named: (1) R-6.5.6's fourth check ("game over") — a partial record is
  ACCEPTED, because it carries a digest (§0) that strict replay verifies as it does a finished
  one; (2) umbrella §4's second M6.5b bullet ("as is a record not over … by four different
  sentences") — the refusals are now rev, digest, replay and table shape, and a record not
  over lands under `abandoned/`; (3) umbrella §4's "the seed never leaves the browser except
  inside a finished record" — re-worded: **the seed leaves the browser only inside a record
  the player sends to the archive or saves as a file, and never on any wire payload.** *(I-9)*
- **R-6.5b.7 — when the archive cannot be reached, the abandon is refused and the game kept,
  and a local game's record can be saved as a file at ANY point.** "The archive could not be
  reached — the game was kept" is the server's own rule for a failed put, applied at the
  client. "Save game file" on a local game is offered while it runs, since it is the player's
  own seed on the player's own disk (R-6.5b.6's fence). **This amends umbrella R-6.5.6's
  "'Save game file' is a plain download at game over on BOTH channels."** R-6.5.10 is
  untouched, because the local save never goes through `GET …/{id}/record`, which the local
  dispatch still answers with a 409 while the game runs; a hosted game's file save stays at
  game over. *(I-9)*
- **R-6.5b.8 — a fifth section, "On this device", at the top of the first screen, and the
  architectural guarantee that local games are reachable there with the API unreachable.**
  The section's rows come from the local dispatch's own `GET /api/games?scope=mine`; its
  action is "Play the bot here". The first screen renders from the browser's state FIRST and
  the server sections fill in when the server answers (§2.5); the cell that holds it: with
  every `fetch` failing — refused AND never answering, two code paths — the section renders
  with its games and Play the bot here works. *(I-3)*
- **R-6.5b.9 — the host is a new crate, `crates/yokohama-wasm`** (approach 1 of three), a
  WORKSPACE MEMBER so the lint gate covers it. A `cdylib` over `yokohama-web` with
  `default-features = false`, built with `wasm-bindgen`. `yokohama-web` stays exactly what
  M6.5a made it: a library that knows no host, with its two region kinds and its scanning cell
  untouched. Ruled out: a `wasm` feature on the layer crate (bindgen glue and a `cdylib` crate
  type inside the layer, a third region kind); a raw ABI with hand-written glue (`getrandom`'s
  `js` feature already pulls `wasm-bindgen` in, so the glue is generated regardless).
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
- **R-6.5b.12 — a local game's id starts with `local-`, and the calls that carry no id are
  routed by the caller's intent.** The Worker draws `local-<16 hex>` and supplies it on every
  create; the client's one predicate `isLocal(id)` is that prefix, DEFINED once in `api.ts`,
  and every reader that HAS an id (`call`, `recordText`, the transport arm, the screen's
  source record) goes through it — one definition of what a local GAME is. **Five routes carry
  no id** — `POST /api/games`, `GET /api/games?scope=mine`, `GET /api/health`, `GET /api/me`,
  `GET /api/labels` — and for those `call` takes an explicit `host: "local" | "server"`
  argument, defaulting to the id-derived answer. **Three have page callers** (the create, the
  local listing, the `labels` fallback); `/api/health` and `/api/me` are ANSWERED locally (§6.3
  replays them) but have no local caller in the page — the local deployment facts are a
  constant and the door is the server's — and the implementer should not invent one to
  justify the argument. The create is
  routed by the caller's intent because that is what it is. **At its real size:** a server
  table CAN be given the prefix by a hand-written request (the prefix is `is_safe_id`-legal
  and the layer reserves nothing), in which case the page routes it to the Worker, which
  answers 404. A confusion the browser's own client never causes, never a leak. *(I-2)*
- **R-6.5b.13 — `delete` chooses the archive leaf by the mirror's truth, and the residual is
  named.** In `tables.rs`'s abandon site, `Why::Over` when `doc.turn` says the game is over,
  `Why::Abandoned` otherwise — one line and one cell. The M6.5a deploy review found a finished
  game with no stamp archived under `abandoned/`; the leaf lied and the archived table's
  `turn.over` told the truth. **The residual:** `turn` is a cache `mirror_turn` may leave one
  move stale when its write conflicts (§0), so the leaf can still lie in exactly that race; the
  alternative, `Games::handle(id).is_over` (a rebuild and replay under the table lock), was
  declined for its cost and its lock order, and the cell claims what is true: the leaf follows
  the mirror. *(M-3)*
- **R-6.5b.14 — `.wasm` is served as `application/wasm`.** One arm in `assets::content_type`
  and one assertion in its cell.
- **R-6.5b.16 — the hosted deployment target is Workers FREE (owner, 2026-09-20, confirming
  what the pre-execution review reported), which AMENDS umbrella R-6.5.12's "Workers Paid".**
  Nothing in M6.5b moves for it — this cycle deploys to the NAS — but M6.5c inherits it as a
  constraint rather than a claim: on Free the routing Worker has a 10 ms CPU budget per request
  and a SQLite-backed object 30 s, so gameplay, replay, `suggest`, `why` and the archive's
  strict replay run INSIDE objects (`/api/archive` mapped to an object of its own), static
  assets are served without the Worker, and the daily Free quotas (requests, GB-s, rows read
  and written, 5 GB) are BUDGETED before the first deploy and MEASURED in the live setup
  (R-6.5.8). §8 carries the list.
- **R-6.5b.15 — the archive's caller prefix is a path-safe ENCODING of the address, never the
  address.** `archive::caller_segment(user) -> String`: the normalised address with `@`
  replaced by `_at_` and every character outside `[a-z0-9._-]` replaced by `_`, capped at 64
  characters. **The property that holds under the cap**: the segment is at least five
  characters of `[a-z0-9._-]` with no `/` (a local part and a domain are each non-empty and the
  separator is four characters, and the cap can only shorten), so it can be neither empty, `.`
  nor `..`, and no address can name a path outside the root; a cell asserts that `../../x@y`,
  `a/b@c` and `..@..` all land inside it (§6.1). Readable on purpose: the owner reviews saves by
  who sent them, and a hash would hide that. **It collides, and §8 says so**: `a/b@c` and
  `a_b@c` render alike, and two addresses longer than the cap can share a prefix — and with it
  each other's "already archived" refusal. A hash is the one-line alternative if the bucket must
  hold no addresses. *(C-1, N-6)*

---

## 2. The mechanism

### 2.1 `crates/yokohama-wasm`

A `cdylib` (`crate-type = ["cdylib"]`), `publish = false`, a workspace member, depending on
`yokohama-web = { path, default-features = false }`, `wasm-bindgen` (the lock's `0.2.128`),
`js-sys` (the clock). It holds ONE `Games` at a time, built by `init()`:

```
Games::with_parts(Arc::new(Rules::standard()), Arc::new(MemStore::default()),
                  ChatConfig::default(), Arc::new(NoSignal), <clock>, Arc::new(NoArchive))
```

and exports exactly these:

- `init()` — builds a FRESH `Games` over a fresh `MemStore`, replacing any earlier one. It is
  "rebuild", not "once": the Worker calls it at boot, on acquiring a lock (§2.2) and after a
  trap (§4), and each time follows it with `load`s. *(C-2)*
- `dispatch(user: string | null, method: string, path: string, query: string, content_type:
  string | null, body: Uint8Array) -> { status: number, body: string }` — builds an
  `ApiRequest` and calls `api::dispatch` with `Host { identity_name: "local", signal_kind:
  "none" }` and `Identity = Ok(&User::parse(user))`, or `Err("no identity given")` when
  `user` is `null`, so `/api/me` answers `user: null` as the corpus captures it
  (`me_signed_out.json` is a 200 — `/api/me` never refuses) and the 401 sweep has an arm it can
  reach. *(N-5)* **The identity is a parameter so a test can play the corpus walk
  as its own five people** (§6.3); the Worker passes `local@browser` and nothing but the Worker
  can reach the module. *(I-1)*
- `load(doc: "record" | "table", id: string, text: string)` — writes the document into the
  store with `expected: None`, which is a CREATE; it is only ever called on a fresh store
  (§2.2), so it never meets a present document and the `Store` trait needs no versionless
  overwrite. It bypasses `Games` on purpose: a stored pair is a state the player was shown,
  and `Games` rebuilds the `Repl` from it on first touch through `Session::from_record`
  exactly as the axum host does (`games.rs:406`). *(C-2, M-10)*
- `read(doc: "record" | "table", id) -> string | undefined` — the store's exact bytes.
- `set_clock(seconds: number | undefined)` — a frozen reading, or `undefined` for the JS
  clock. The `Clock` handed to `with_parts` is an `AtomicU64`-backed closure like
  `Games::ticking_clock`; on `wasm32` the live reading is `js_sys::Date::now() / 1000`, on
  every other target (the crate's own unit cells, the workspace lint gate) it is the ticking
  clock, gated by `#[cfg(target_arch = "wasm32")]`.

**The chat config is the default**, so the local `/api/health` reports `lobby_chat: true`,
`chat_keep: 500`, `chat_text: 500` — byte-equal to the committed `health.json` (§6.3 relies on
it) — and NOTHING in the client reads the local host's chat facts: the lobby's panel is the
server's house, and a local table has no panel (§2.5). *(M-2)* `NoArchive` is the local
archive: the local health reads `"archive": "off"`, which is what the committed file reads
too, and R-6.5b.6's send goes to the SERVER's route.

**The fences on the new crate.** It is outside `yokohama-web`'s scanning cell by construction
(§0) and is HOST code: its platform calls are `js_sys::Date::now` and the bindgen glue, and
nothing else. It carries its own copy of the scan over its own `src` with `js_sys` the one
allowed pattern, so a `std::fs` or `std::env` in the host crate is named the way one in the
layer is. *(M-11)* `wasm-bindgen` and `js-sys` are plain dependencies and compile natively
(documented panicking stubs), which the workspace lint gate exercises for the first time —
recorded in Task 3, and the M6.5a close's P-2 ("never built natively") is superseded.

**Build.** `cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm`, then
`wasm-bindgen --target web --out-dir web/src/local/wasm/ …` — INTO A SOURCE DIRECTORY, gitignored,
so Vite bundles the glue and emits the `.wasm` as a hashed asset (a static import of a file under
`public/` fails Rollup's resolution — the pre-execution review reproduced it, R7) — and `--target
nodejs --out-dir web/wasm-node/` followed by writing `web/wasm-node/package.json` as
`{"type":"commonjs"}`, because the Node glue is CommonJS and `web/package.json` is `"type":
"module"` (§6.3). `build:wasm` runs BEFORE `tsc`, since the glue's `.d.ts` must exist for the
typecheck; the Worker compiles under its own `tsconfig.worker.json` (`lib: ["ES2022",
"WebWorker"]`) rather than by adding worker globals to the window project. The crate's exports
are thin wrappers over pure-Rust inner functions (`dispatch_inner`, `load_inner`, …) that the
native cells test — `wasm-bindgen`'s non-wasm stubs cannot make a `JsValue` — and the cells that
touch the global `Games` serialise on one test lock. `wasm-bindgen-cli` is installed at the
lock's version (`cargo install wasm-bindgen-cli --version 0.2.128 --locked`), **a from-source
build of about two minutes that CI caches by version** (`actions/cache` on `~/.cargo/bin/
wasm-bindgen`), and the release wasm build is a second cargo target the job pays once per run —
so the `web-server` job compiles three ways (the debug tests, the wasm check, the release
module) plus node, and its wall is expected to roughly DOUBLE; recorded at Task 3. *(M-13)* Task 3 records the module's size (raw and gzip) beside the umbrella's 1.30 MB / 387 KB
— RECORDED, never gated.

### 2.2 The Worker (`web/src/local/worker.ts`)

One dedicated Worker per tab, started lazily on the first local call and kept for the page's
life. It loads the module from `/wasm/`, opens IndexedDB (`yokohama-local`, schema version 1,
one object store `docs` keyed by `[doc, id]` holding the text; documents are opaque and their
own schema is the layer's `TABLE_SCHEMA`, so an IndexedDB upgrade is only ever a new store),
then `init()`s and `load`s every stored pair before it answers anything.

- **Messages.** Request `{ seq, method, path, query, body }` (body a string or `null`;
  content type `application/json` when a body is present) → reply `{ seq, status, body }`.
  The same fields as `ApiRequest` and `ApiResponse`; the Worker adds `local@browser` and
  nothing else.
- **Persistence is the Worker's, decided by OUTCOME and not by method, for the documents that
  EXIST, for the games this tab HOLDS, and BEFORE the page is answered.** After any call on a
  game this tab holds, the Worker `read`s the game's record and table and writes what exists —
  the table alone while the game is dealing (`create` writes no record; the record appears at
  the last keep, `tables.rs:1791`), the pair once playing — in one transaction when the bytes
  differ from what it last wrote, or removes both after a 204 on `DELETE`. One rule, no method
  list — it covers the move, the bot's pump on a cold read (§0), the cold-rebuild repair and
  anything a later cycle adds. **A call on a game this tab does NOT hold is answered from its
  store and never written back** — a second tab's cold GET pumps the bot in ITS store and shows
  it, and the next holder's rebuild re-derives the same moves from the `(bot_seed, events.len())`
  key. **The page's success reply is sent AFTER the durable write completes**: a module reply is
  provisional, and "saved" means the transaction landed — never "the player was shown it". A
  write the store REFUSES (quota, a private window) is answered with the reply marked unsaved,
  which the page shows as "APPLIED BUT NOT SAVED" (§4); it is not a trap. The write carries the
  tab's ownership GENERATION and the transaction checks it against the stored one, so a stale
  holder whose JS keeps running after a steal cannot commit. *(I-10, R1, R2, R4)*
- **The Worker is one serialised actor.** Every message runs through ONE operation queue —
  dispatch, persist, rebuild, hold, release — so a rebuild cannot replace the store under a
  persist and two holds cannot interleave; the lock's callback promise is held OUTSIDE the
  queue (a queue that awaited a lock's lifetime would answer nothing else). `lastWritten` is
  cleared and reseeded from the loaded pairs on every rebuild, so a take-back followed by an
  undo cannot skip a write because an older cache still matched. The record and table are
  read for export as one snapshot inside the queue. *(R3)*
- **The lock, and the rebuild it triggers.** Ownership is derived from the ROUTE's table id —
  the dealing screen included — and acquired BEFORE any call on that game, never from a
  `ClientState` already fetched. Opening a local game is `navigator.locks.request
  ("yokohama:local:<id>", { mode: "exclusive", ifAvailable: true }, …)`; when the lock is
  unavailable the callback is invoked with `null` (it does not reject), and that `null` is
  what "This game is open in another tab" reads. Take over is a second `request` with
  `{ steal: true }`, whose effect on the first holder — its request's promise rejects with an
  `AbortError` — is what turns that tab read-only (§4). **On every acquisition (the first open,
  a Take over, a Take back) the Worker `init()`s a fresh `Games` and `load`s EVERY stored pair
  from IndexedDB — not only the acquired game's, since `init()` emptied the store and every
  other local game in this tab would otherwise 404 until reopened; loading bytes is cheap and
  the `Repl` rebuild stays lazy per game on first touch — before it answers a call for that
  game**, so the writer always starts from the last state ANY tab acknowledged; the boot-time
  load is only the first of these. The acquisition also ADVANCES the game's ownership generation
  in IndexedDB (a small `meta` store), which every later write checks in its own transaction —
  the Web Locks specification warns that a stolen-from holder may keep executing, and a JS
  notification cannot fence a running wasm call or a queued write; the generation can. *(R2)* **What releases the lock:** the page keeps the request's
  callback promise open for as long as the game screen is mounted and resolves it when the
  screen unmounts, so navigating away (to the lobby, to another game) RELEASES; and the Worker
  keeps the set of ids this tab holds, so a re-request for one of them short-circuits rather
  than telling the player their own tab has the game open. A reply produced by a call that was
  in flight when the lock was lost — under any number of prior holders, since Take over and
  Take back can alternate — is not written back and is answered to the page as the lock loss,
  so nothing is shown that will not be kept. *(C-2, M-9, owner read item 2)*
- **Ids.** The Worker rewrites a create's body to carry `id: "local-<16 hex>"` drawn from
  `crypto.getRandomValues`; the seed is NOT supplied — `draw_seed` inside the module draws it
  through `getrandom`'s `js` feature, and it is never typed (umbrella §3.2).

### 2.3 The client's local arm

- `api.ts`: `isLocal(id)` is the `local-` prefix. `call(method, path, body?, host?)` routes to
  the Worker when `host` is `"local"` or, absent that, when the path's game id is local; the
  five id-less local callers (R-6.5b.12) pass `"local"`. Under the Worker arm
  `identityHeaders()` is not read. `recordText` reads the record for a local game from the
  Worker's side channel (a `read` message), never from the `…/record` route, which the local
  dispatch still answers exactly as the corpus pins it (a 409 while running — umbrella §2.5).
  `api.labels()` is read from the SERVER first, BOUNDED at five seconds, and from the Worker
  when that read fails OR times out (a hanging read never rejects, so `.catch` alone would
  never fall back — the pre-execution review's point); and once a local game is on screen the
  Worker is ready anyway and answers `labels` directly. The words on the cards survive an
  unreachable API, and the Worker stays lazy — a visitor who only plays hosted games never
  downloads the module. The route effect's guard at `App.tsx:503` (`if (!door.me?.user)
  return;`) admits a LOCAL table id signed out, since a local game needs no door. A Worker that
  fails to start, a module that fails to load, or an IndexedDB that will not open is a VISIBLE
  line in the local section naming the reason — never an empty list.
  Routing it locally ALWAYS was considered and declined for that boot cost (a stated default
  the owner may reverse; §2.1 would then record the module's cost at every page load). The art
  manifest is the host's (`/api/art` is not in the layer) and degrades to the typed fallback when
  it cannot be fetched; level 3 is what fixes that. *(I-2, M-4, N-1)*
- `signal.ts`: `Source` gains one optional field, `host?: "local"`, and `open()` gains a third
  arm for it: no stream, no held request, nothing to wait for — the bot's trailing run
  returns inside the reply, as the whole-game gate plays it today. **This AMENDS umbrella
  §2.5's "a local game needs a third arm there rather than a `Source` change"**: `open()` holds
  a `Source` and nothing else (§0), so the source is the only place the host can be said, and
  it is the shape M6.3's `frames?: "value" | "wake"` (R-7.1) already used. `useSignal` sets it
  from `isLocal(id)`; the lobby's and the open table's sources never do, so the server's
  streams stay open while a local game is on screen — a cell (§6.2). The `"sse" | "held"`
  union at `api.ts:193` is untouched. *(I-6)*
- **Deployment facts per source.** The module-level values become a record keyed by source
  (`server` | `local`) whose `signal` field is typed `"sse" | "held" | "none"`: the local record
  is a constant (`signal: "none"`, `archive: "off"`, the chat facts unread); the server record
  is set from the health read as today, and `signalKind()` becomes a read of the SERVER
  record with its `"sse" | "held"` union untouched — the transport arm never asks it about a
  local game, since it reads `Source.host` instead. A game
  screen reads its own source's record through `isLocal(id)` — with ONE named cross-read: a
  local game's Abandon label reads the SERVER's `archive`, because the send goes there.
- `App.tsx`'s `fetchRun` makes ONE call to the run route (§2.4) with `to = next + 1` (the
  route is half-open, `fetchRun`'s range is inclusive) plus `/log`, on every host — and PAGES
  a run longer than the route's cap in cap-sized calls (a spectator reconnecting after a long
  absence is the case; one call is the common one).

### 2.4 The two server routes (the layer; unconditional; both hosts)

- **`GET /api/games/{id}/at?from=&to=`** (R-6.5b.10) — the `HistoricalFrame`s of every `k` in
  `[from, to)`, as `{ frames: [...] }`, each frame byte-equal to `/at/{k}`'s. Built by
  `HistoricalFrame::run(repl, from, to, viewer)`: `state_at(from)` once, then `replay_from`
  ONE event at a time with a frame built from each intermediate state by
  `HistoricalFrame::from_state`, which `at` becomes a thin wrapper over (`at(k) = from_state
  (state_at(k))`), so the two share one builder. Admission is `at`'s. `RunQuery { from:
  usize, to: usize }` with `deny_unknown_fields` through `parse_query`, so a non-numeric or
  unknown field is a 400 in axum's words. **The boundary, in numbers, because the client
  contract depends on it:** a game with `total` events has `total + 1` POSITIONS, `0..=total`,
  and `at/{total}` is the live frame; the run is half-open, so **the largest admitted `to` is
  `total + 1`** — which is exactly what `fetchRun` sends (`to = next + 1`, `next` the reply's
  event count, so the run ends on the live frame) — and **the 404 begins at `to = total + 2`**,
  with the sentence `at/{k}` already gives for a position past the end, extended to name the
  range, so the two routes refuse one condition one way; `from > to`, a malformed range rather
  than a missing position, is DELIBERATELY the same 404 rather than a third answer. **The cap is
  checked FIRST**: a run longer than 64 is a malformed ask and is the 400 whatever `total` is, so
  a one-event game asked for `to=65` reads the cap's sentence and not the range's. *(owner
  read, item 1 — both prior reviews had "to > len" with `len` the event count, which would have
  refused every legitimate client call while the pairing cell passed on the same wrong
  boundary.)* A run longer than **64** frames is a 400
  naming the cap (never a silent clamp — the M6.3 `wait` rule). *(I-7)*
- **`POST /api/archive`** (R-6.5b.6) — body `ArchivePost { record: String, table: String }`
  with `deny_unknown_fields`, both strings parsed by the SERVER. In order: (1) a
  `semantics_rev` PRESENT and equal to `SEMANTICS_REV`, else 400 "this record is at semantics
  rev N; this server archives rev 7" or, for a `None`, `replay_strict`'s own "missing
  semantics provenance" sentence carried verbatim; (2) a `terminal_digest` present, else 400
  "the record carries no terminal digest"; (3) `replay_strict` clean, else 400 "the record does
  not replay: <the engine's sentence>"; (4) the table document parses, passes `check_shape`,
  and AGREES with the record (a new `TableDoc::agrees_with(&GameRecord)`; nothing at HEAD
  compares the two) — `doc.players` equals the record config's player count, `doc.colors ==
  record.config.player_colors`, `doc.start_seat == record.config.start_player`, `doc.seed ==
  record.seed`, and **for every seat the document actually claims a keep for** (`keeps[i] ==
  Some(n)`, 1-based) the record reads `choices[i] == Keep(n - 1)` (0-based, the `- 1` of
  `choices_of` at `tables.rs:506`). **A `None` keep is NOT compared**: a bot seat's keep lives
  in the record and never in the table document — `create` fills `keeps` with `None`
  (`tables.rs:756`), only a seat holder may `keep` (`:1463`), and `choices_of` recomputes the
  bot's from `bot_keep` at finalize (`:499`) — so every real local pair reads `keeps = [Some(n),
  None]` against two `Keep`s, and a rule that compared the `None` would 400 every real send
  *(N-4)* — else 400;
  then **the leaf IS the `Why`**: `Why::Over` iff the replayed state `is_over`, else
  `Why::Abandoned`, never read from the body; then `archive.put(prefix, why, id, record,
  strip_for_archive(&doc))` — the table re-stripped SERVER-side from the parsed document, as
  `delete` does, so a crafted body carrying a lock or a watcher roster never lands verbatim
  (the record string is put as sent, since its bytes are the digest's) — with the prefix
  `browser/<caller_segment>/` (R-6.5b.15), answering 201 `{ "leaf":
  "over" | "abandoned", "path": "…" }`; an existing path is 409 "already archived"; no archive
  on this host is 404 "this server keeps no archive" — under `NoArchive` (the local host) the
  route is that 404 by construction. *(M-12)*
- **The trait changes, and it is a cross-cycle edit named as one:** `Archive::put(&self,
  prefix: &str, why: Why, id: &str, record_json: &str, table_json: &str)` and `paths(rev,
  prefix, why, id)`, with `""` the server's own games' prefix at both call sites
  (`persist`, `delete`); `NoArchive`, `DirArchive` and M6.5c's R2 implementation carry it, and
  the umbrella's O-9 cell ("the two implementations write the same relative paths") is
  extended to a NON-EMPTY prefix. `paths` stays the ONE layout function (umbrella R-6.5.4).
  *(I-5)*
- **The put protocol, because a 409 must never delete the only complete copy** (R6). An
  implementation writes the TABLE first and the RECORD second, and **the record's path is the
  once-marker**. `put` answers `Landed` when it wrote both; **`Identical` when the record at
  that path already holds the SAME bytes** (a retry after a lost reply — and a half pair, a
  table with no record, is REPAIRED by writing the record and re-writing the table); and
  `Err(Conflict)` when the record at that path holds DIFFERENT bytes. The route answers 201 for
  `Landed`, **200 with `identical: true` for `Identical`**, and 409 "a different record is
  archived under this id" for the conflict. The client proceeds to its local delete on 201 or
  200 and KEEPS the game on 409 — a generic 409 is not proof the pair landed, and the earlier
  draft deleted on it. The two-file put is still not atomic (a crash between the two writes
  leaves a table alone), which the repair on retry is for; §8 carries it.
- **"At most once", at its real size.** An implementation refuses a DIFFERENT record at an
  existing PATH, and the path is `prefix/rev/leaf/id`. The client's controls keep one local game to one leaf: Send to
  the archive is offered only at game over (`over/`) and Abandon and save only while the game
  runs (`abandoned/`), and an abandoned game is deleted locally, so it cannot be sent again.
  What the route does NOT prevent: the same save sent by two different callers lands twice,
  under two prefixes. Accepted and recorded: the training side dedups by digest. *(M-6)*

### 2.5 The first screen (R-6.5b.2, R-6.5b.8)

- **All FOUR door gates at `App.tsx:922-926` stop being gates on the page.** `door.error`, a
  null `door.me`, the `SignIn` form and the door sentence become the content of the server
  AREA; the page's frame and "On this device" render above them in every one of those four
  states, as soon as the Worker has loaded its store. *(I-3)*
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
  health reported an archive; on 201 (or a 200 `identical`) it says where the copy went AND
  goes dark for the rest of the session (the local document cannot be stamped — the send never goes through the module);
  after a reload the control is live again and a second click answers the route's 409, shown as
  "already archived" (§8). *(owner read item 5)*

### 2.6 What a local game never does

Talk to the server for play (§6.2's `fetch` spy), carry the seed on any wire payload (the
corpus pins hold — the local dispatch is the same code), or leave the device except by the
player's own send or save (R-6.5b.6).

---

## 3. What a player sees, in one paragraph each

**First screen.** "On this device" at the top with your local games and Play the bot here;
under it the lobby as before. The API unreachable: the same top section, one line saying "not
connected" below it, the words on the cards intact and the pictures gone until the server is
back. **Playing.** The table as always; the bot's turn walks; nothing on the network.
**Two tabs.** The second says the game is open elsewhere and offers Take over; the first goes
read-only with Take back; whoever holds the lock sees every move the other tab played.
**Finished.** Save game file, Send to the archive (when the server keeps one).
**Abandoning.** "Abandon and save" sends the record first and refuses if it cannot; a running
game can be saved as a file first.

---

## 4. Error handling

- **The module fails to load**: the local section says so in one line naming the reason;
  the server sections are untouched.
- **A trap inside `dispatch`**: a trapped module holds its own mutex and TRAPS AGAIN on
  `init()` — reproduced by the pre-execution review with this crate's exact `Mutex<Option<_>>`
  shape under `panic = "abort"` (R5) — so recovery is a FRESH INSTANCE: the Worker answers the
  call as a 500 in the API's `{"error": …}` shape marked `trapped`, the page terminates that
  Worker, settles every pending request as "the local host restarted", starts a new Worker
  (which rebuilds from the committed pairs), re-acquires the game's ownership, and only then
  says "reloaded from the last saved move". Nothing committed is lost (§2.2's
  commit-before-reply rule), and nothing a stale tab held is kept (§2.2's rebuild rule).
  Worker `error` and `messageerror` events and a module that fails to load are the visible line
  of §2.3, not a permanently pending list.
- **A stored local game that no longer rebuilds** — a later `SEMANTICS_REV` bump or a rules-hash
  change, which by design stop a save resuming: `Session::from_record` REFUSES with an `Err`
  (`games.rs:406`, `GameError::Server`), which is a 500 carrying the engine's own sentence and
  NOT a trap, so the trap recovery above is not entered and its "reloaded" strip never shows.
  The row stays listed (the listing reads table documents), "Save game file" still works (it
  reads bytes) and Abandon still works (`delete` reads the store and rebuilds nothing); the
  game screen shows the sentence naming the revision. *(owner read item 3)*
- **IndexedDB refuses a write** (quota, a private window): the reply is answered marked
  unsaved and the move is shown with the server's own sentence, "APPLIED BUT NOT SAVED" — one
  rule, one wording, both hosts — and a storage failure is DISTINGUISHED from a trap (R4): the
  Worker keeps running, the game stays exportable, and the next successful write clears it.
- **The lock is stolen**: the losing tab's game goes read-only with "opened in another tab"
  and Take back; a move attempted after the loss is refused in the page before it reaches the
  Worker, and a reply that was in flight is not written back (§2.2).
- **The archive's refusals** are shown as sent (rev, no digest, replay failed, table shape,
  already archived) plus "the archive could not be reached". The game stays in every case
  but one: "already archived" on an abandon means an earlier send landed and only the delete
  failed, so the abandon proceeds.
- **No reachable API**: the one line, the local facts, the archive controls dark, the words
  intact (`labels` falls back to the Worker), the pictures gone.

---

## 5. The corpus

- `run.json` — the run route at the walk's mid-game stop, `[k-3, k)`, three frames — and
  `run_400_cap.json`, `run_404_range.json`.
- `archive_post.json` (201 for a finished record, `leaf: "over"`), `archive_post_abandoned.
  json` (201 for a partial record, `leaf: "abandoned"`), and the five refusals `archive_400_
  rev.json`, `archive_400_digest.json`, `archive_400_replay.json`, `archive_400_table.json`,
  `archive_409.json` — the walk's own record, edited in memory for each.
- **`walk.json` — NEW KIND: the walk's request script, and it lives OUTSIDE the corpus**, at
  `web/src/wire-script/walk.json`, written by the same test under the same variable with its
  own staleness check — because the script carries REQUEST bodies (the setup seed, the table
  passwords) and every leakage pin scans `corpus()` for exactly those strings; a script inside
  `web/src/wire/` would fail the pins or force an exemption that weakens them (R8). For every
  `call_as` of the PRIMARY `Api`, `{ who, method, path, query, body, status, fixture }` in
  order, where **`fixture` is nullable** — most rows are intermediates and assertions (the dispatcher cell counts 234
  compared pairs over a 74-payload walk), every row is REPLAYED so the module's state keeps
  step with the walk's, and only a named row is compared. `call_as` does not know the
  fixture's name — it is bound at the `out.push` sites — so the walk records each call into a
  side list that those sites annotate — bound BY VALUE after the walk (the first unbound row whose frozen
  response equals the payload), with the residual named: two calls with byte-equal responses are
  interchangeable for the replay, and the status rides the row so a replay compares it too; the
  alternative (a name parameter on `call_as`, ~120 call sites) is declined. **The side list is per `Api` instance and only the primary's rows
  are emitted**: `house_chat_off_404.json` is captured through `call_as` on a SECOND `Api`
  built with `lobby: false` (`wire_fixtures.rs:1510-1513`), and it belongs in the third column
  below, not in the script. It is the input §6.3 replays through the module. *(M-10, N-2, N-3)*
- **Every committed file is in exactly ONE of three columns, and the classification is
  DERIVED, not written** (the replay asserts it, so a new fixture cannot join the corpus
  without landing in a column): *(I-1)*

  | column | files | rule |
  |---|---|---|
  | replayed byte-for-byte | every file `walk.json` names, except the next row's | the default |
  | host-parameterized, the fields NAMED | `health.json`: `identity`, `signal` (`archive` is ASSERTED — both hosts read `"off"`, so exempting it would hide a local host that reported `"dir"`); `me.json`, `me_signed_out.json`: `mode` | R-6.5.9's second class |
  | not served by this host | `art_manifest.json` (the host's), `site_space_words.json` (a contract, not a payload), `watching.json` and `locked.json` (constructed, not routed), `house_chat_off_404.json` (a second `ChatConfig`) | exactly the files `walk.json` does not name |

  *(M-1)*
- Every `ClientState`, `HistoricalFrame` and `TableView` fixture stays byte-identical — the
  cut proves nothing moved on the wire.

---

## 6. The instruments, and the mutation battery

### 6.1 Layer cells (`yokohama-web`)

- The run route: frames equal `/at/{k}`'s BY VALUE over a run with a change in its middle
  (anti-vacuity: the run spans a turn boundary and its middle frame differs from both
  neighbours); a non-numeric field as a 400 in axum's words; **`to = total + 1` is ADMITTED and
  its last frame equals `at/{total}` BY VALUE** (the live frame — the arm the client's every
  call depends on), and **`to = total + 2` answers what `at/{total + 1}` answers** (the 404, the
  pairing that makes it a claim); a run over the cap as the 400 naming it; a spectator admitted
  exactly as at `at/{k}` (one grid row). *(I-7, owner read item 1)*
- `POST /api/archive`: the five refusals by five DIFFERENT sentences, including the `None`
  rev; the leaf chosen from the replayed state with a body whose table says the opposite; a
  partial record ACCEPTED under `abandoned/`; once per path (409); the `browser/<segment>/`
  prefix through `paths()`; a record with its seed altered (the `JSON.parse` rounding shape)
  refused by the replay.
- **Containment (R-6.5b.15)**: for `../../x@y`, `a/b@c` and `..@..` the written path is a
  descendant of the archive root, asserted on the `DirArchive` by canonicalising both; and
  `caller_segment` never yields an empty, `.` or `..` segment. *(C-1)*
- The trait's prefix: O-9's "same relative paths" cell over both implementations under a
  NON-EMPTY prefix as well as the empty one. *(I-5)*
- R-6.5b.13: a finished unstamped game deleted lands under `over/` when the mirror says over —
  and the cell's doc names the mirror residual. *(M-3)*
- R-6.5b.14: `content_type("x.wasm") == "application/wasm"` in the existing cell.
- `TableDoc::agrees_with`: each of the five fields disagreeing is refused by name, **and the
  committed browser-played pair of §6.4 — one human, one bot, `keeps = [Some(n), None]` — is
  ACCEPTED**, which is the arm that keeps the five refusals from passing on a synthetic all-human
  pair while the only real input is refused. *(N-4)*

### 6.2 Client cells (vitest)

- `isLocal` routes `call` to the Worker and everything else to `fetch`; the five id-less
  callers pass `host: "local"`; a **`fetch` spy at zero across a whole local game, the stream
  reader and `recordText` included**, with `POST /api/archive` and the server's health read as
  the ONLY named exemptions (umbrella §4). *(I-2, I-4)*
- The boot path with every fetch REFUSING (the `door.error` arm) AND with every fetch never
  answering (the null arm) both render "On this device" with its games and Play the bot here
  works — R-6.5b.8's guarantee, two code paths, the cells that must fail under the pre-M6.5b
  gates. *(I-3)*
- The lobby's source still opens its stream while a local game is on screen. *(I-6)*
- The lock's three screens (open, opened elsewhere, taken over) and the refused move after a
  loss.
- Save game file at any point on a local game and only at game over on a hosted one.
- A refused abandon keeps the game; "already archived" proceeds.
- `recordText` for a local game reads the side channel and never parses (the existing cell's
  local twin).

### 6.3 The wasm corpus replay (the browser host's corpus claim)

Its OWN vitest project — `web/vitest.node.config.ts`, environment `node`, NO `setupFiles`,
`include: ["src/local/*.node.test.ts"]` — and its own script, **`npm run test:wasm`**, listed
in CLAUDE.md beside the three web commands with its own count, so `npm test` keeps its
meaning and its count and the module is a dependency of one command rather than of a
per-edit gate. It loads the module built for Node, calls `set_clock(1_789_000_000)` — the
`FROZEN_CREATED_AT` the corpus writes, under which every stamp ties and the listing falls to
the same id tie-break the committed files already show — and replays `walk.json` through
`dispatch` with each call's OWN identity (`null` for the anonymous call), executing EVERY row
and asserting, for every row that names a fixture, that the reply in §5's first column
deep-equals the committed file, the second column equal outside its named fields, and that the
third column is exactly the files the script does not name. Because it
is the same walk through the compiled module, it is also a bot-free determinism claim across
the compilation boundary. It REFUSES loudly when the module is absent, naming `npm run
build:wasm`; `YOKOHAMA_WASM_OPTIONAL=1` turns absence into a skip that says so. **CI:** the
`web-server` job gains `setup-node` and `npm ci`, builds the module (§2.1's cached CLI) and
runs `npm run test:wasm`; the `web-client` job is untouched. *(I-8)*

**And one change to the axum walk itself, taken here:** `Api::new` in `wire_fixtures.rs`
builds its `Games` with `with_clock` at the same constant, so the walk's listings no longer
depend on two creates landing inside one wall-clock second — the review found that a straddled
second boundary would sort `wire2` first and turn the corpus red on unchanged code. The two
hosts then agree by construction rather than by coincidence. **The regeneration diff for that
change is expected EMPTY** — the committed listings already show the id tie-break, and
`1_789_000_000` clears `freeze_clock`'s floor — so a non-empty diff there is a finding about the
tie assumption, not a fixture to accept. *(review claim 37, N-9)*

### 6.4 The determinism pin (umbrella §2.5, O-3, O-11)

A PAIR played by the wasm bot IN A REAL BROWSER — the record AND its local table document,
exactly what "Send to the archive" sends — produced by §6.5's exit walk and COMMITTED at
`crates/yokohama-web/tests/fixtures/browser-played-2p.{json,table.json}` with a sidecar naming
the release, the date, the browser and the module's hash. **It carries a seed, deliberately**
— it is a record, and the seed is what the replay needs — and it lives under
`crates/yokohama-web/tests/fixtures/`, outside `web/src/wire/`, which is the ONLY directory the
corpus's seed value-pin scans (`wire_fixtures.rs`, `corpus()`), so the fence cuts as intended in
both directions and nobody should move the pair into the corpus. The pin builds a `Games` over a
`MemStore`, `load`s the pair, and drives the layer's own `why` route through `dispatch` as
`local@browser` at EVERY bot event natively; a refusal fails it, and the fifteen counters the
payload carries are read with `placement_cap_binds` asserted zero as the scope partner. The
same cell times `Session::from_record` on that record natively, and the e2e records the
Worker's replay wall on reload (O-3), both RECORDED. Regeneration is one documented command.

### 6.5 e2e (real Chromium, the release binary)

- **The exit**: a whole game against the bot in the browser with EVERY `/api/` request other
  than `/api/archive` counted ZERO from the moment the local game screen is open, off the
  page's request log (the M6.3 shape; wider than "no `/api/games`" — streams, other play
  routes and Worker-originated requests are in the count); the download; Send to the
  archive against the binary started with a temporary `YOKOHAMA_WEB_ARCHIVE_DIR` (the Playwright
  `webServer` block takes an `env` map), the archived file under `browser/<segment>/rev7/
  over/` byte-equal to the download.
- **Real-Worker fault cells**, because a fake Worker proves page routing and nothing about
  commit ordering, ownership or recovery: a quota failure injected by overriding
  `IDBObjectStore.prototype.put` in the page's Worker (Playwright `addInitScript`) shows "APPLIED
  BUT NOT SAVED" and the game stays exportable; a real trap raised through a `debug_trap` export
  the e2e build enables (cargo feature `debug`, never in the release module) is recovered by a
  fresh Worker with play continuing from the last committed move; a create closed before the
  first card is chosen comes back on reload with its exact setup (the table-only pair, R1).
- **Two TABS of ONE context** (IndexedDB is per profile; two contexts would prove nothing):
  B opens the lobby first, A plays a run of moves, B opens the game and takes over — **B's first
  frame shows A's moves** — and A is read-only with Take back. *(C-2)*
- **The reload cell**: a local game reloaded mid-play, at a position where the bot is to move,
  comes back with the bot's cold pump PRESERVED across the reload (the outcome-triggered
  write-back of §2.2) and the replay wall recorded (O-3). *(N-7)*
- The first screen with the API refused (a route that 503s `/api/*`): "On this device"
  renders, a local game plays, the option words are intact.
- Any NEW spec file carries the file-level `test.skip(!haveBrowser, …)` guard that
  `every_spec_can_skip.spec.ts` enforces.

### 6.6 The mutation battery (the spec's own, run by an agent at the close)

The umbrella §4's M6.5b list AS AMENDED by R-6.5b.6 — a `fetch` at any point of a local game
(§6.2); a record with one event edited, a record at another rev, a record with its digest
stripped, a table that disagrees, each refused by its own sentence (§6.1); a record not over
ACCEPTED under `abandoned/` (§6.1); a `JSON.parse` round trip refused (§6.1); the wasm bot's
moves reconstructing under native `why` with a perturbed stream key failing it (§6.4); the seed
on no wire payload (the corpus pins) — and: the run builder replaying from scratch per step
(RECORDED by the O-3 timing, not caught — no cell asserts wall); a frame off by one in the run
(§6.1); the run's range refusal as a 400 (§6.1's pairing); any one of the four door gates
restored (§6.2, both arms); the lock not stolen, and a takeover that does NOT rebuild its
store (§6.5); persistence triggered by method rather than outcome (a bot pump on a cold GET
lost across a reload — §6.5's reload cell); the lobby's stream silenced by a local game (§6.2);
a server id routed to the Worker (§6.2); the `wasm` MIME arm removed (§6.1); the leaf taken
from the body (§6.1); the caller prefix built from the raw address (§6.1's containment);
`archive` exempted in the replay (§6.3); the walk's identity ignored by `dispatch` (§6.3 fails
on the first per-seat payload). *(I-9, and every review item with a catcher)*

---

## 7. The order of the work (the plan's tasks, named)

1. The run route, `HistoricalFrame::from_state` + `run`, `fetchRun` as one paged call,
   `run.json` and its two refusals; R-6.5b.13's leaf fix with the residual in the cell's doc.
   Deployable alone.
2. `POST /api/archive`: `ArchivePost`, the five checks, `TableDoc::agrees_with`, the leaf as
   the `Why`; the TRAIT's prefix and `paths(rev, prefix, why, id)`; `caller_segment` and the
   containment cell; O-9 under a non-empty prefix; the corpus files.
3. `crates/yokohama-wasm` as a workspace member, the six exports, the crate's own scan, the
   build scripts and `.gitignore` lines, CI (the cached CLI, the node steps), the `.wasm` MIME
   arm; the size and the native build of the bindgen crates recorded.
4. The Worker: messages, IndexedDB, the lock with the rebuild-on-acquire, the in-flight rule,
   persistence by outcome; `api.ts`'s local arm with `host?`; `signal.ts`'s `Source.host?`;
   per-source deployment facts with the one named cross-read. **The SDD candidate.**
5. The first screen: the four gates, "On this device", Play the bot here, `labels` with its
   Worker fallback.
6. The local game screen's three differences; Save game file; Abandon and save; Send to the
   archive.
7. `walk.json` through `corpus()` with the side list; the frozen clock in `Api::new`; the
   node vitest project, `npm run test:wasm`, the derived three-column assertion.
8. The e2e walk; the committed browser-played PAIR; the determinism pin over `dispatch`; O-3
   recorded; the takeover cell.
9. Docs: the help panel, `NewGame.id`'s doc line, RUNBOOK §1 (the wasm build line) and §3.13,
   CLAUDE.md (the fourth web command `npm run test:wasm` and its count, and `cargo test -p
   yokohama-wasm` as a per-edit line with its count — the crate's own scan is a test nothing
   runs until a documented command names it), the board; the close.

Process: R-P11 as refined 2026-09-20 — small mechanical tasks inline with the per-edit gates
after each, one Opus review at the structural mid-point (after Task 4) and one whole-branch
review before the release; full SDD only for a task the plan marks large or judgment-heavy
(Task 4).

---

## 8. What this spec does NOT claim, and the LEAVE list

- **Not installable, not offline.** Level 3 — a service worker caching the shell, the module
  and the art with a cache keyed to the release and bustable for a patch, an update prompt,
  about 8 MB per browser — is the FIRST follow-up after M6.5b, its own brainstorm and spec.
  Level 2 is "the API unreachable with the page loaded", never "the origin down". **Its charter
  is written down now** (the pre-execution review's §3, carried as requirements): a coherent
  release cached whole (HTML, JS, glue, module, art, manifest, labels) before "available offline"
  is said; release-keyed asset paths with an update installed and verified before the switch,
  never mid-move, and eviction that cannot touch IndexedDB; a persistent-storage request when a
  player starts relying on local saves, with the grant inspected and backup offered; **a
  browser backup and restore format** carrying the record AS A STRING beside its table document
  with a versioned envelope, validated in Rust, refusing to overwrite silently — today a
  downloaded save is a CLI recovery path and the browser has no restore; incompatible saves
  kept listed and downloadable, never migrated or erased; the origin kept stable across the
  NAS-to-Workers move, since a different hostname is different storage. Its exit: airplane mode
  → close the browser → reopen → resume → finish → export, then an update and a restore on a
  fresh profile, on the owner's own phone.
- **Nothing hosted.** No Workers, no objects. M6.5c inherits R-6.5b.11's two push shapes and
  the archive route and trait unchanged.
- **Two seats only**, you and the bot. A local hot-seat human-versus-human is free in the
  layer and is not offered.
- **The corpus replay proves the walk, not the bot**; the bot's determinism is proven on ONE
  committed browser-played pair (§6.4). O-11 stands: 2P only.
- **The lock is per browser profile.** Two profiles on one machine are two IndexedDBs: no
  conflict, and no sharing.
- **"At most once" is per path.** One save sent by two callers lands under two prefixes;
  the training side dedups by digest. **And the caller prefix collides** (R-6.5b.15): an unusual
  address, or two longer than 64 characters, can share a directory and each other's 409.
- **The leaf at an abandon follows the mirror**, which can be one move stale in one race
  (R-6.5b.13).
- **The archive route's two-file put is not atomic**: the table is written first, so a crash
  between the two writes leaves a table with no record, which the next retry REPAIRS (§2.4);
  the operator's sign is a `.table.json` with no sibling. A client never deletes on a 409.
- **"Send to the archive" after a reload can be clicked again** and answers 409; the local
  document carries no stamp.
- **A local game's data leaves the device only by the player's own send or save.** There is
  no sync and no backup; clearing site data deletes the games.
- **The Web Locks facts are from the specification and compatibility tables until Task 4
  measures them on the owner's phone.**
- **O-2 and O-7** (module start-up in a Cloudflare Worker; `getrandom` there) are M6.5c's;
  M6.5b measures the browser Worker's start-up and records it.
- Two cosmetic items carried from the M6.5a deploy review: an already-archived finished
  game's row reads "Abandon and save"; "and save" on a just-dealt table (M6.5a §6.3 item 4).
- **Carried to the M6.5c handover, not this cycle's** (the pre-execution review's §1 and §4):
  the deployment target is Workers FREE — **confirmed by the owner 2026-09-20, R-6.5b.16, which
  amends umbrella R-6.5.12** — and on Free the routing Worker has a 10 ms budget and the object 30 s, so gameplay, replay,
  `suggest`, `why` and the archive's strict replay run INSIDE objects, `/api/archive` mapped to
  an object of its own; the layer's `Archive::put` is synchronous and an R2 put is not, so
  M6.5c defines a prepare/upload/commit protocol with a receipt before a delete is authorised;
  hibernating WebSockets with the picker persisted (umbrella H-4); static assets served without
  the Worker; the house projection updated only when a visible field changes; long-save
  measurements of cold replay, a whole bot turn, projection, serialisation, `why` and the
  archive replay, CPU and wall separately, on the phone and in a live Free-tier object; the
  browser crate's global `Games` is the per-tab instance's and never a shared isolate's.

---

## 9. What the review changed (so a sentence can be traced to its reason)

The first draft's mechanism survived; what the read found was the next step down from
several right-altitude sentences, and three inferences drawn from true citations.

- **C-1** the caller prefix was a raw email in a filesystem path → R-6.5b.15, §6.1's
  containment cell.
- **C-2** a second tab's Worker was stale by every move the first tab played → `init()` is a
  rebuild, the Worker rebuilds on every lock acquisition, the in-flight rule, §6.5's takeover
  cell.
- **I-1** "the whole corpus" was unreachable five ways → §5's derived three columns, the
  `null` identity, `archive` asserted (M-1).
- **I-2** five calls carry no id → `host?` on `call`, R-6.5b.12 narrowed to games.
- **I-3** four door gates, and the first is the one that fires offline → §2.5, §6.2's two arms.
- **I-4** three `fetch` sites, not two → §0.
- **I-5** the prefix is the TRAIT's, not `paths()`'s → §2.4.
- **I-6** `open()` cannot know its source → `Source.host?`, the umbrella §2.5 amendment
  named, the lobby-stream cell.
- **I-7** the range refusal was a 400 where `at/{k}` answers 404 → §2.4, §6.1's pairing.
- **I-8** the replay could not run under the shared vitest setup and CI had no node → its
  own project and script, `web-server` gains node, the CLI cached (M-13).
- **I-9** two umbrella sentences contradicted and unnamed → R-6.5b.6 (three amendments),
  R-6.5b.7 (R-6.5.6, not R-6.5.10).
- **I-10** the bot's pump on a cold GET missed the write-back → persistence by outcome.
- **M-2 … M-12** as marked in place; **claim 37** the walk's latent clock flake → the frozen
  clock in `Api::new`.

The scoped re-read of that wave found one Critical the wave had introduced and nine smaller
things; all are in the second wave:

- **N-4** check (4) compared a bot seat's `None` keep against the record's `Keep` and would have
  refused every real send → the rule compares only the keeps the document claims, and §6.1's
  cell ACCEPTS the committed browser-played pair.
- **N-1** `labels` local ALWAYS made the Worker eager on every page load → server first, Worker
  on failure; the Worker stays lazy (a default the owner may reverse).
- **N-2, N-3** the third column's rule was false for `house_chat_off_404.json`, and most rows
  name no fixture → the side list is per primary `Api`; `fixture` nullable; every row replayed.
- **N-5** the anonymous arm is a 200, not a 401 → §2.1.
- **N-6** the cap could cut inside `_at_`; the encoding collides → the property that survives
  the cap, the collisions in §8.
- **N-7** §6.6 pointed at a reload cell §6.5 did not list → listed.
- **N-8** the new crate's scan had no command → Task 9's CLAUDE.md line.
- **N-9** the clock change's regeneration diff is expected empty → said.
- **N-10** `from > to` as a 404 is deliberate → said.

The owner's own lightweight-agent read of the twice-revised spec (every citation it checked
verified true) found what both Opus passes had missed, and the third wave carries it:

- **Item 1 (execution-critical)** the run route's 404 boundary was one position too tight
  against the client's own `to = next + 1` → §2.4's boundary in numbers, §6.1's two arms
  (`total + 1` admitted, `total + 2` the 404).
- **Item 2** the lock lifecycle: `init()` after an acquisition reloads EVERY pair; navigating
  away RELEASES and a re-request by the holder short-circuits; the in-flight rule holds under
  any number of prior holders → §2.2.
- **Item 3** a stored game that no longer rebuilds after a later rev bump is an `Err`, not a
  trap; the surface is defined → §4.
- **Item 4** the entry handover's superseded sentences, named here since the handover is in
  the read order: **its §2 item 5 ("the FOUR checks of R-6.5.6 … game over … under a
  `browser/<user>/` prefix") is superseded by R-6.5b.6, R-6.5b.15 and §2.4 (five checks, the
  leaf from the replayed state, the encoded prefix), and its §3 item 5 ("a local game's Abandon
  reads 'Abandon'") by R-6.5b.6 and §2.5 ("Abandon and save" through the server's archive).**
- **Item 5** Send to the archive goes dark on 201 and 409s after a reload (§2.5, §8); the route
  re-strips the table server-side (§2.4); the two-file put is not atomic (§8); two of the five
  id-less routes have no page caller (R-6.5b.12); the per-source record's `signal` union is the
  wider one and `signalKind()`'s is untouched (§2.3); the CI job's wall roughly doubles (§2.1).
- **Context** the committed pair's seed sits outside the corpus pin's scan on purpose (§6.4).

---

## 10. The pre-execution review's dispositions (the owner's fourth read, of the PLAN)

`docs/plans/2026-09-20-web-frontend-m6.5b-review-free-tier-offline-saves.md` — advisory, at
`6bc137f`. Each finding was verified against the code here before it was taken; none was wrong.

| finding | verified by | disposition |
|---|---|---|
| **R1** a dealing local game has only a table and the Worker persisted nothing without a pair | `tables.rs:1791` "no record until the last keep" | TAKEN — §2.2 persists the documents that exist; the plan's `persist` and the reload cell |
| **R2** a cold GET pumps and persists before ownership; a stolen-from holder may keep running | `games.rs:405-424`; the Web Locks spec's `steal` note | TAKEN — §2.2: ownership from the ROUTE id before any call, write-back only for held games, a generation checked in the write's transaction |
| **R3** an async `onmessage` is not an actor; `lastWritten` survives a rebuild | the plan's Worker sketch | TAKEN — §2.2: one operation queue, the lock's promise outside it, the cache reseeded on rebuild |
| **R4** "written after the acknowledged reply" vs commit-before-success; storage failure treated as a trap | the spec's own §2.2 sentence; the sketch's catch-all | TAKEN — §2.2 commit-before-reply; §4 a storage failure is marked unsaved, never a trap; the transaction helpers abort explicitly |
| **R5** re-`init()` on a trapped instance traps again | the review's reproduction (this crate's mutex shape, `panic = "abort"`) | TAKEN — §4: a fresh Worker, pending requests settled, ownership re-acquired |
| **R6** a 409 authorised a local delete; a half pair or a different record both 409 | `archive.rs:138-140` writes the record first | TAKEN — §2.4's put protocol: table first, the record the marker, `Identical` as a 200, repair on retry, a different record a 409 the client keeps the game on |
| **R7** the static import of the glue from `public/` fails Rollup; the jsdom glob matches the node suite; the Node glue is CommonJS under `"type": "module"`; native `JsValue`; parallel tests on the global | the review's reproduction; `vitest.config.ts`; `web/package.json:6` | TAKEN — §2.1 and §6.3: glue into `web/src/local/wasm/`, `build:wasm` before `tsc`, a Worker tsconfig, the jsdom exclude, the node `package.json`, inner functions, a test lock |
| **R8** `walk.json` inside `corpus()` carries the seed and passwords the leakage pins scan for | `wire_fixtures.rs:1812-1830`, `corpus()` | TAKEN — §5: the script lives in `web/src/wire-script/`, outside every pin's scan, with its own staleness check |
| §3's two level-2 details: a hanging `/api/labels` never falls back; `App.tsx:503` gates local routes on the door | `App.tsx:561`, `:503` | TAKEN — §2.3 |
| §5's notes: the cap cell's precedence; single `cargo test` filters; the no-network claim widened; real-Worker fault cells; attribution trailers | the plan | TAKEN — §2.4 cap-before-range; the plan's commands; §6.5 |
| §1 Free tier vs R-6.5.12's Paid; offline reopening not delivered; no browser restore | the umbrella `:354`; §8 | TAKEN as R-6.5b.16 (the owner confirmed Free the same day; R-6.5.12 amended) and CARRIED — §8: the level-3 charter written down; restore named as part of it |
| §4 the M6.5c optimisation list; the async archive seam | — | CARRIED — §8, into the M6.5c entry handover |
