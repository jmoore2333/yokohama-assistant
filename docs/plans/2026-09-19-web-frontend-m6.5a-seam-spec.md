# Web frontend — M6.5a "the seam": the spec

**Written 2026-09-19 on branch `web-room-m6.5` at `a4d238c`, under the umbrella
`docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` (its §2.1 is this cycle's charter,
its §1 the rulings R-6.5.1–R-6.5.13 that bind it, its §6.1 the open items it must not
re-open).** Every source file cited below is byte-identical to `faf1a88`, M6.4's close HEAD:
the five commits since are documents. M6.5a is the first of M6.5's three sub-cycles (R-6.5.7).
It builds nothing a player sees except one button and one word on `/api/health`; what it
builds is the shape the other two sub-cycles stand on — a table layer that compiles for
`wasm32-unknown-unknown` with the axum server reduced to an adapter over it, proven byte-for-
byte by the wire corpus — and the archive, so that saves are collected on the NAS from this
cycle on.

**The one-sentence exit:** the axum server redeployed on the NAS with every existing corpus
payload byte-identical except `health.json`'s one new key, the wasm check line and the
platform-call scanning cell green in CI, the CLI untouched, core byte-identical.

---

## 0. What is true at HEAD (`a4d238c`; source byte-identical to `faf1a88`)

The cut-surface survey (`.superpowers/sdd/2026-09-19-web-frontend-m6.5-hosting/survey-cut-surface.md`,
gitignored, written at `2f5381a` and restated here where this spec depends on it) is the
line-cited map. Re-read every citation at the HEAD the plan's first task opens.

### 0.1 The crate, by size and by axum reach

`crates/yokohama-web/src/` is 19,463 lines in fifteen files. **Two thirds of each of the three
big files is `#[cfg(test)]`**: `lib.rs` 7,380 lines, first `#[cfg(test)]` at `:2148`;
`tables.rs` 4,849 / `:1698`; `games.rs` 1,593 / `:626`. The logic this cycle moves is about
2,400 lines and the tests that ride on it about 6,800. Five files mention axum — `lib.rs`,
`identity.rs`, `art.rs`, `assets.rs`, `main.rs` — and ten do not; of the ten, `client.rs`,
`explain.rs`, `outcome.rs` and `grammar.rs` make no platform call at all.

### 0.2 The state, and where the layering actually sits

`AppState` (`lib.rs:255-271`) is `rules: Arc<Rules>`, `games: Games`, `identity:
IdentityMode`, `heartbeat: Duration`, `chat: ChatConfig`; its constructors (`:262-322`) all
end in `with_store_and_chat`, which builds `Games::with_limits(rules, store, chat.limits)`
(`games.rs:214`) — so the chat RULE reaches `Games` while the lobby SWITCH stays on
`AppState` (`lib.rs:311-313` says the two are not a duplicate). `Games` (`games.rs:107-187`)
holds `rules`, `store`, `live`, `written`, `table_lock`, `locks`, `deleted`, `signals`,
`chat_last`, `house_lock`, `limits`. **The 24 document methods in `tables.rs`'s `impl Games`
(`:686-1696`) and the 12 in `games.rs` (`:206-593`) ARE the layer**; the axum-shaped state sits
above them. Three of the four helpers every route starts with take `&AppState`, not `&Games`:
`admitted` (`lib.rs:506`), `events_at` (`:1856`), `lobby_chat_on` (`:981`). `ApiError`
(`lib.rs:70-85`) carries `status: axum::http::StatusCode` — the `http` crate's type
re-exported (`http` 1.5.0 in the lock) — and is the error of seven of the eleven helpers,
every handler, and the `Identity` extractor (`identity.rs:149-163`); `From<GameError> for
ApiError` (`lib.rs:141-159`) is the one mapping from the layer's own error (`games.rs:41`,
eight variants).

### 0.3 The handlers

`app()` (`lib.rs:2116-2146`) registers 26 routes → 30 handler functions over 28 bodies.
**Seventeen bodies carry logic beyond extraction, one `Games` call and `Json`** (the survey's
§1.2 table); the thin four are `create_game`, `delete_game`, `deal`, `unlock`; the public
four (`health` `:363`, `me` `:391`, `labels` `:429`, `art_manifest` `:2087`) call no `Games`
at all, and `me` is the only route that reads headers outside the `Identity` extractor
(`:384-390`). **`input` (`:1103-1284`) is the one that has to move whole**, thirteen steps in
order: the grammar fence (`:1109`), `admitted` (`:1112`), `spectators_write_nothing` (`:1117`),
the viewer→seat `match` (`:1120`), `handle` + lock (`:1124`), the undo rule (`:1143`, R-P6,
BEFORE the fence by R-5.1), the acting-seat fence (`:1190`, R-4.3, never dropping), the
`expect` check (`:1198`, 409), `before`/`locked_before` (`:1212`), `step` (`:1214`),
`persist` on an events-or-lockdown change with the `APPLIED BUT NOT SAVED` arm (`:1234`),
the prompt-suffix strip (`:1253`), `ClientState::of` and the warnings (`:1272`). Nothing in
the thirteen is axum except `ApiError`'s status type and the `Json` wrapper. The three
transport routes — `events` (`:1783`, the `?wait` arm's `tokio::time::timeout` at `:1804`),
`stream` (`:1931`, SSE, the `Err` split at `:1955-1968`) and `house_stream` (`:1993`) — and the
static/art/asset routes are the ones that are axum by nature. The two body/query extractors
(`JsonBody` `:190`, `QueryArgs` `:212`) wrap axum's `Json` and `Query` and answer every
rejection as a 400 in this API's shape with axum's own `body_text()`; **no committed fixture
and no lib cell pins those sentences** (grep, 0 hits), but every client that meets one reads
it.

### 0.4 The platform calls, outside `#[cfg(test)]`

| file | call | disposition in this cycle |
|---|---|---|
| `table.rs:417-422` | `now_unix` → `SystemTime::now()` (callers `tables.rs:746`, `:913`) | the clock seam, §3 |
| `signal.rs:21` | `tokio::sync::watch` | behind the `Signal` trait, §2.4 |
| `store.rs:237-382`, `:286`, `:469-471` | `FsStore`'s `std::fs`, `std::process::id`, `default_dir`'s `std::env` | `server`, §2.3 |
| `chat.rs:145-147`, `identity.rs:56` | `std::env::var` | `server`, §2.3 |
| `identity.rs:20-22`, `:149-163` | the axum extractor | `server` |
| `tables.rs:213` (`create` `:697`, `:709`; `finalize` `:1195`) | `draw_seed` → `SeedSchedule::from_os_entropy` | unchanged; `getrandom`'s `js` feature, §2.5 |
| `tables.rs:1565`, `:1576`, `:1587` | `eprintln!` in `list` | unchanged — VERIFIED on the target: a `wasm32-unknown-unknown` module calling `eprintln!` has no imports and returns normally under Node (the write is discarded, nothing traps) |
| `games.rs:21`, `tables.rs:21`, `store.rs:213`, `:399` | `std::sync::{Arc, Mutex}` | unchanged (single-threaded on the target; compiles) |

`Cargo.toml` enables six tokio features; `net`, `signal`, `rt-multi-thread` fail the target,
`sync`, `time`, `macros` build (`time` panics at runtime). The umbrella §0.3 has the
measurements.

### 0.5 The test surface

219 lib cells: **all 74 in `lib.rs` drive the router through `app(state)` and
`tower::ServiceExt::oneshot`** (63 through a helper alone, 12 also arranging or asserting
through `Games`; the helpers `get_json` `:2161` … `house_frames_after` `:6115`); the 145
module cells construct `Games`/`AppState`/documents directly through `games.rs:1452`'s
`test_support`. `tests/wire_fixtures.rs` builds `Api { router }` from
`AppState::in_memory_with_chat` (`:155`) and drives `oneshot` (`:191`); `freeze_clock`
(`:247-266`) normalises `created_at` before comparison; `UPDATE_WIRE_FIXTURES` at `:2231` is
asserted UNSET at the end of the compare cell (`:2286`). `tests/play_a_whole_game.rs` is the
same shape over `AppState::in_memory()` (`:60`). **A cut that keeps `Games`' method
signatures and `app()`'s behaviour leaves every one of them green untouched.**

### 0.6 The client's edges this cycle touches

`web/src/api.ts`'s `call` (`:233-284`) is the module's only `fetch`, parses every body as
JSON after reading the status, and is the only thing outside `sse.ts` that fetches; the
`api` object's 24 members are all `call`. `Health` (`web/src/types.ts:447-465`) is
type-checked against `web/src/wire/health.json` by `contract.ts` (`:110`, `:330`, `:772`).
The finished game is `state.over` (`App.tsx:1255`, the header reads "finished"); `Abandon`
(`web/src/Abandon.tsx:21-56`) calls `api.remove` and reports through `onError(sentence)`.

### 0.7 The store and the table document

`Store` (`store.rs:117`) is synchronous over `Doc` (`:36`, four kinds); `FsStore::delete`
(`:378-388`) removes record, table and chat. `TableDoc` (`table.rs:170-187`) is `schema`,
`id`, `name`, `created_by`, `created_at`, `players`, `colors`, `start_seat`, `seats`,
`keeps`, `seed`, `status`, `turn`, `lock` (`#[serde(default)]`), `unlocked`
(`#[serde(default)]`); `TABLE_SCHEMA = 2` (`tables.rs:52`, its doc: additive fields with
defaults, the number is what lets an OLDER server refuse a newer document). `mirror_of`
(`games.rs:91-98`) reads `over` and `winner` off the session; `persist` (`:363-546`) writes the
record under compare-and-swap, then `mirror_turn` (`:549`), then `bump` (`:543`).
`Games::delete` (`tables.rs:1648-1696`) is creator-only, tombstones, `store.delete`, then
clears `written`/`live`/`chat_last` and `signals.end`.

---

## 1. Rulings (R-6.5a.n; the namespace is free)

- **R-6.5a.1 — the cut runs through `Games`, and `Games` is the layer's state.** `Games`
  absorbs `AppState.chat` (so `lobby_chat_on` reads `games.chat.lobby`) and gains a `clock`
  (§3), a `signal` (§2.4) and an `archive` (§4); `AppState` keeps `games`, `identity` and
  `heartbeat`, and is `server`-only. `admitted`, `events_at`, `lobby_chat_on` and the other
  helpers take `&Games`. **`Games`' public method signatures do not change**, which is what
  keeps the 145 module cells green; where a method needs the clock or the archive it reads
  them off `self`.
- **R-6.5a.2 — `ApiError` is re-typed on the `http` crate's `StatusCode`** (a direct
  `http = "1"` dependency; the same type axum re-exports, so `From<GameError>` and every
  helper compile unchanged); `impl IntoResponse for ApiError` moves under `server`.
- **R-6.5a.3 — ONE route table, in the layer.** A new module `api.rs` holds `ApiRequest
  { method, path, query, content_type, body }`, `ApiResponse { status, body }` (the body a
  JSON string in this API's shape), a `Host { identity_name, signal_kind }` of host
  facts, and `dispatch(games, host, user: Option<&User>, &ApiRequest) -> ApiResponse` over a
  path matcher for every route in `app()` EXCEPT the three transport routes (`events`,
  `stream`, `house_stream`) and the static, art and asset routes. Each handler body becomes a
  typed `pub fn` in the layer (`api::input(games, user, id, &Input) -> Result<ClientState,
  ApiError>` and its siblings), and `dispatch` is the parse-then-call layer over them.
  **axum's `app()` becomes an ADAPTER**: it registers the transport and static routes
  explicitly, and ONE catch-all for `/api/{*rest}` that resolves the identity with the
  extractor, converts the request, calls `dispatch`, and writes the response — so the 74
  route cells and the corpus drive the layer's dispatcher through the only axum code that
  remains. Unknown path → the 404 `fallback` (`lib.rs:2104`) answers today; known path with a
  wrong method → axum's 405 shape, reproduced. **HEAD is GET with the body removed**: axum's
  `get` routes serve HEAD that way today (its own doc says so, `routing/method_routing.rs:55`),
  no fixture and no client sends one, and a proxy or health checker might — so the adapter maps
  HEAD to the GET arm and strips the body, and a cell pins `HEAD /api/health` at 200 with an
  empty body through the adapter. **Identity is the host's sentence, carried, not the layer's,
  reproduced**: `dispatch` takes `identity: Result<&User, &str>`; a game route given `Err(msg)`
  answers 401 with `msg` in this API's shape, and the axum adapter passes `resolve`'s own
  message (`identity.rs:127-145`, one sentence per mode) — so the 401 sweep stays byte-equal
  through the adapter and a direct-dispatch cell pins the same text. What an identity-less
  host passes is that host's spec (M6.5b). **Why Level 2 now and not per-route functions
  alone**: M6.5b's browser host is "one function, request-in / response-out over the same
  method, path, query and body" (umbrella §2.5), and building that dispatcher HERE means the
  corpus proves it under axum before any second host exists; the alternative leaves three
  route tables to drift.
- **R-6.5a.4 — the rejection sentences are reproduced, not re-worded.** The layer's parsers
  (`serde_json` for bodies with the content-type rule, `serde_urlencoded` 0.7.1 for queries
  — the crate axum's `Query` uses — and the path matcher for `{k}`) answer a bad body, a bad
  query and a bad path parameter as 400s whose text is byte-equal to what `JsonBody`,
  `QueryArgs` and `Path` answer at HEAD (axum's `body_text()` prefixes: "Failed to deserialize
  the JSON body into the target type: …", "Failed to parse the request body as JSON: …",
  "Expected request with `Content-Type: application/json`", "Failed to deserialize query
  string: …", and the path rejection's), and a `server` cell pins each against axum's own
  rejection for a canonical bad input so an axum upgrade that rewords is caught. No fixture
  pins them today (§0.3); this cycle adds none, because a fixture of a malformed request is a
  fixture of axum's prose.
- **R-6.5a.5 — the `server` cargo feature.** `default = ["server"]`; `server = ["dep:axum",
  "dep:tokio", "dep:tokio-stream"]` with all three made OPTIONAL (cargo features are additive,
  and tokio's `time` builds for the target only to panic, so the dependency itself is what is
  gated). Behind it: the router, the handlers for the transport and static routes, the
  extractors, `main.rs` (`required-features = ["server"]`), `signal.rs`'s tokio implementation,
  `identity.rs`'s extractor and `from_env`, `chat.rs`'s `from_env`, `FsStore` and
  `default_dir`, `art.rs`, `assets.rs`, and `table::now_unix`. `MemStore` stays unconditional:
  it is the store a wasm host's tests and M6.5b's browser host start from.
- **R-6.5a.6 — the clock is a seam with its own fence** (umbrella H-1). `Games.clock:
  Arc<dyn Fn() -> u64 + Send + Sync>`, defaulting to `now_unix` under `server` and injected
  by tests; the two callers (`tables.rs:746`, `:913`) read it. The fence is the source-
  scanning cell of §6.2, because the compile check cannot see a `SystemTime`.
- **R-6.5a.7 — the CI line** `cargo check -p yokohama-web --no-default-features --target
  wasm32-unknown-unknown` joins the `web-server` job (after `rustup target add
  wasm32-unknown-unknown`), and `yokohama-web`'s `Cargo.toml` gains `getrandom = { version =
  "0.2", features = ["js"] }` under `[target.'cfg(target_arch = "wasm32")'.dependencies]`,
  inert on native, version-pinned (0.3 renames the feature). The line proves the DEPENDENCY
  fence (no axum, no mio); §6.2's cell proves the PLATFORM-CALL fence; neither is the
  other's fallback.
- **R-6.5a.8 — the signal is a trait.** `signal::Signal { fn bump(&self, id: &str); fn
  bump_table(&self, id: &str); fn bump_house_chat(&self); fn end(&self, id: &str) }` — four
  methods over `&str` and nothing else, exactly the four signatures `Signals` has at HEAD
  (`signal.rs:68-87`), so the trait is unconditional while `Bump`, `House`,
  `subscribe`/`subscribe_house` and `senders` stay on `Signals` under `server`, reached by the
  transport routes only. On `Games` as `Arc<dyn Signal + Send + Sync>`; `Signals` (the tokio
  watch channels, unchanged in behaviour) implements it under `server`; `NoSignal` is the
  unconditional no-op. The eight bump sites call the trait and do not move. This is room plan R-P9's third seam made a
  type, and the object host implements it in M6.5c.
- **R-6.5a.9 — the archive is a trait with two implementations this cycle** (umbrella
  R-6.5.4, R-6.5.5, O-9): `archive::Archive { fn put(&self, why: Why /*Over | Abandoned*/,
  id, record_json: &str, table_json: &str) -> Result<(), String>; fn kind(&self) ->
  &'static str }`; `DirArchive` under `server` (`YOKOHAMA_WEB_ARCHIVE_DIR`, absent = off, a
  bad value exits 1 naming the variable, like the chat variables); `NoArchive` unconditional
  (`kind() == "off"`). **One layout function** `archive::paths(rev: u32, why, id) ->
  (record_path, table_path)` = `rev<N>/over/<id>.json` + `rev<N>/over/<id>.table.json` (or
  `abandoned/`), used by `DirArchive` now and the bucket in M6.5c, with a cell over both. **The
  archived table document is the stored one with `lock` and `unlocked` REMOVED**, and a value
  pin asserts neither this run's salt, its hash, nor a watcher's address is in the archived
  bytes (the M6.2 and M6.3 pin shapes). Chat is not archived.
- **R-6.5a.10 — `archived_at` and `TABLE_SCHEMA` 3.** `TableDoc` gains `archived_at:
  Option<u64>` with `#[serde(default)]`, bounded in `check_shape` against `MAX_CREATED_AT`;
  `TABLE_SCHEMA` becomes 3; a schema-2 document reads unchanged (a cell reads one).
- **R-6.5a.11 — the two archive sites and their two failure rules.** At game over: in
  `persist`, after `mirror_turn`, when the mirror's `over` is newly true and `archived_at` is
  `None`, the record bytes just written and the stripped table document are `put(Over)`; on
  success `archived_at` is written; **on failure the move is still answered 200 and the
  failure is logged**, and the retry is the next abandon or the operator (the umbrella's
  R-6.5.4 sentence). At abandon: in `delete`, under the table's own lock, when the archive is
  on, the table is `Playing` (over or not), the record has at least one event and
  `archived_at` is `None`, `put(Abandoned)` runs BEFORE `store.delete`; **on failure the delete
  is REFUSED** with `GameError::Server("the archive could not be written (<why>) — the table
  was not deleted")`, nothing tombstoned, nothing removed. A table already archived (at over,
  or by an earlier failed delete's successful put) is deleted without a second put. An open
  or dealing table, or a playing one with no events, deletes as today. **One edge, ruled and
  recorded rather than solved**: at a vs-bot table today's CLI undo (R-P6) can un-finish a
  game after its end, and `archived_at` STAYS SET — the archived copy is the game's FIRST
  completion, a later ending is not archived, and `DirArchive`'s refusal to overwrite is what
  makes that a rule instead of a race. Changing undo is not this cycle's; §10 carries it.
- **R-6.5a.12 — `GET /api/games/{id}/record`** (umbrella R-6.5.10). Admission: a SEAT HOLDER
  at the table (`admitted` → `Viewer::Seat`; a spectator is refused 403 with "the record is
  the players' — watch, download nothing"); a running game is a 409 "the record is available
  once the game is over" in this API's shape; a finished game answers 200 with the EXACT bytes
  the store holds for `Doc::Record` (not a re-serialization: a digest compare in-test proves
  it), `content-type: application/json`. Lockdown does not fence it (the M3 lockdown rule:
  projection, not computation).
- **R-6.5a.13 — "Save game file"** on the finished-game screen (beside the final score),
  reading the record as TEXT through a second entry beside `call` that never parses it
  (`api.recordText(id)`), and handing it to the browser as a download named `<id>.json`.
  The record is opaque to the client from this cycle on (umbrella R-6.5.6). The help panel
  gains its sentence. `Health` gains `archive: "off" | "dir"` (M6.5c adds `"r2"`), read at
  boot beside the chat facts; nothing in the client reads it this cycle except the help
  panel's sentence about where an abandoned game goes, and the Abandon control's label, which
  reads "Abandon and save" when the archive is on and the table's status is `playing`
  (`TableView` carries no event count, so a just-dealt table with no move yet reads "and
  save" and saves nothing — the server's rule is the truth, the label is the near truth, and
  adding a count to `TableView` would move eight fixtures for one word), "Abandon" otherwise.
- **R-6.5a.14 — the corpus.** `health.json` gains exactly `archive`; `record_409.json` joins
  (captured mid-walk, the walker asking); the 200 body is NEVER committed; the seed pin gains
  its second partner (the umbrella §3.1); and a NEW cell, `the_dispatcher_answers_what_the_
  router_answers`, replays every committed non-transport payload's request through
  `api::dispatch` directly and asserts byte-equality with the router's answer — the cross-host
  contract in miniature, under axum, before any second host exists.
- **R-6.5a.15 — what stays fixed.** The five fenced crates byte-identical; `SEMANTICS_REV` 7;
  the CLI UNTOUCHED (measured: its library half compiles for the target as-is); no bot
  behaviour moves; `wire_fixtures`, `play_a_whole_game` and every one of the 219 lib cells
  stay green with only the cells this spec names added or moved; the wire byte-identical
  except as R-6.5a.14 says.
- **R-6.5a.16 — process.** R-P11 as refined by the M6.4 close §6 and the umbrella's read
  order: Opus implementers and per-task Opus reviewers, one Fable whole-branch review before
  the release, the corpus regenerated at EVERY shape-changing task (R-3-1) and its diff read,
  every agent reporting to a transcript file, no reviewed SHA rewritten, the counts in one
  sequence, the NAS deploy, a live check, the close.

---

## 2. The layer

### 2.1 Modules after the cut

| module | feature | holds |
|---|---|---|
| `api.rs` (new) | — | `ApiRequest`, `ApiResponse`, `Host`, `dispatch`, the path matcher, the parsers of R-6.5a.4, and the typed per-route functions moved out of `lib.rs` (the eleven helpers of the survey's §1.4 among them) |
| `games.rs`, `tables.rs`, `chat.rs`, `table.rs`, `client.rs`, `explain.rs`, `outcome.rs`, `grammar.rs` | — | as today, with `Games` widened by R-6.5a.1; `now_unix`'s callers on the clock |
| `store.rs` | mixed | `Store`, `Doc`, `Version`, `MemStore`, `is_safe_id`, `check_house_id` unconditional; `FsStore`, `default_dir` under `server` |
| `identity.rs` | mixed | `IdentityMode`, `User`, `resolve`, `from_env_value` unconditional; `from_env`, the `Identity` extractor under `server` |
| `signal.rs` | mixed | the `Signal` trait and `NoSignal` unconditional; `Signals`, `Bump`, `House` under `server` |
| `archive.rs` (new) | mixed | `Archive`, `Why`, `paths`, `NoArchive`, `strip_for_archive` unconditional; `DirArchive` under `server` |
| `lib.rs` | mixed | the `pub mod` list unconditional; under `#[cfg(feature = "server")]` blocks IN PLACE (no file move, so `git blame` and the 5,200 test lines stay where they are): `AppState`, `app()`, the adapter handler, the transport handlers (`events`, `stream`, `house_stream`), `IntoResponse for ApiError`, `SIGNAL_KIND`, `HEARTBEAT`, `MAX_WAIT_SECONDS` |
| `art.rs`, `assets.rs`, `main.rs` | `server` | as today |

### 2.2 `dispatch`, precisely

- Identity is resolved BY THE HOST and passed in as `Result<&User, &str>`: the axum adapter
  runs `resolve` and passes the user or its per-mode sentence; every game route answers an
  `Err` as the 401 the extractor raises today, byte-equal. The public four take no identity
  (`me` reports the `Err` as `user: null` exactly as `lib.rs:384-390` does).
- HEAD reaches the GET arm and leaves with an empty body (R-6.5a.3).
- `me` answers from `host.identity_name` and the passed user; `health` from `Games` (rules
  hash, chat facts, `archive.kind()`), `host.signal_kind` and `host.identity_name`, plus the
  crate version and `SEMANTICS_REV`.
- The matcher is a list of `(method, pattern, handler)` in `app()`'s order; a path that
  matches a pattern under another method is 405 with axum's empty body; a path that matches
  nothing is the JSON 404 `fallback` writes.
- Query strings reach the typed structs through `serde_urlencoded` exactly as `Query<T>`
  would; bodies through `serde_json` with the content-type rule; `deny_unknown_fields`
  keeps refusing an unknown key as a 400.
- `dispatch` holds no lock across the call it does not already hold today: the per-route
  functions take and drop `Games`' locks exactly where the handlers do at HEAD, and
  `suggest`/`why` still drop the handle before searching.

### 2.3 What `server` gates, and why each

`FsStore` and the env readers compile on the target and can only fail there; gating them
makes the wasm build carry nothing that cannot run (umbrella O-10). The transport routes
gate themselves through tokio. `now_unix` gates so that the ONLY `SystemTime` in the crate
sits in a `server` file, which is what the scanning cell asserts.

### 2.4 The `Signal` trait

The eight call sites (`games.rs:543`; `tables.rs:769`, `:951`, `:965`, `:1255`, `:1402`,
`:1504`, `:1693`) become `self.signal.bump(id)` etc. The four gated cells that prove a bump
follows its write keep their receivers: they build `Games` with a `Signals` and subscribe to
it, so the proof is unchanged; a fifth cell builds `Games` with `NoSignal` and asserts every
route still answers.

### 2.5 `getrandom`

`draw_seed`'s three call sites are unchanged. The target-gated dependency of R-6.5a.7 is the
only manifest line that mentions wasm.

---

## 3. The clock

`Games.clock` replaces the two direct calls. `AppState`'s production constructors pass
`now_unix`; `in_memory*` constructors pass it too (the corpus is already clock-normalised by
`freeze_clock`, so no committed byte moves); a new `Games::with_clock` lets a cell pin a
`created_at` of its choosing, which is the anti-vacuity partner of the seam.

---

## 4. The archive

- **`Why::Over`** at `persist`, **`Why::Abandoned`** at `delete`, the rules of R-6.5a.11.
- **`strip_for_archive(&TableDoc) -> String`** serializes the document with `lock: None` and
  `unlocked: []`; `archived_at` is included so an archived table says when.
- **`DirArchive::put`** writes both files under `dir/rev<SEMANTICS_REV>/<why>/`, temp + rename
  each, and refuses (returns `Err`) rather than overwriting an existing record file for the
  same id and `why` — a second put of the same game is a bug the caller made, and the archive
  says so instead of versioning.
- **`/api/health`'s `archive`** is `games.archive.kind()`.
- **`deploy/RUNBOOK.md` §3** gains the variable on the NAS's create body:
  `YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive`, on the same named volume, and §3.12 is this
  cycle's deployment record; `deploy/Dockerfile.runtime` documents the variable beside the
  others it names.

---

## 5. The record route and the button

- Route: `GET /api/games/{id}/record`, in `dispatch` and therefore in `app()` through the
  adapter; admission and sentences per R-6.5a.12; the body is the stored string.
- Client: `api.recordText(id)` (a `fetch` that reads `res.text()` after the same status check
  `call` makes, and never parses), the button under the final score when `state.over`, the
  download via an object URL, the help sentence. Component cells: the button is absent while
  the game runs; present and downloading the exact text the api returned when over (a spy on
  the anchor's `download` and `href`); the text is never passed through `JSON.parse` (a spy).
- e2e: the spec that plays the vs-bot game to a winner gains one assertion — the download's
  bytes equal `GET …/record`'s body.

---

## 6. Fences, cells and the mutations that must be caught

### 6.1 Cells this cycle adds (names indicative; the plan fixes them)

- `api.rs`: `the_dispatcher_answers_what_the_router_answers` (in `wire_fixtures`, R-6.5a.14);
  a 405 for a known path under a wrong method; the JSON 404 for an unknown path; HEAD on a GET
  route at 200 with an empty body; the 401 for an `Err` identity byte-equal to `resolve`'s
  sentence in all three modes; the three rejection pins against axum's own text (R-6.5a.4).
- `games.rs`/`tables.rs`: `Games::with_clock` pins `created_at` (§3); `NoSignal` answers every
  route (§2.4); at-over archives once and never twice; at-over failure answers 200 and logs;
  abandon of a playing table with events archives once then deletes; abandon of an open or
  dealing table, or of an already-archived one, archives nothing; abandon whose put fails is
  refused, the table and its documents intact (a failing `Archive` fake); `archived_at` is
  written on success and bounded on read.
- `archive.rs`: the layout function's paths for both `Why`s; `DirArchive` writes both files
  and refuses a second put; the strip pin (salt, hash, watcher address absent; seats and
  keeps present — the anti-vacuity half); `NoArchive::kind() == "off"`.
- `table.rs`: a schema-2 document reads with `archived_at: None`; schema 3 stamped on write.
- The record route: 403 for a spectator, 409 while running, 200 bytes equal to the store's
  (digest compare), the seed pin's second partner (`wire_fixtures`).
- `lib.rs` (server): `platform_calls_stay_behind_the_server_feature` (§6.2); the 401 sweep
  and the stranger sweep unchanged and still green through the adapter.
- Client: the Save cells of §5; the `Health` contract row for `archive`; the Abandon label's
  two wordings.

### 6.2 The scanning cell

`platform_calls_stay_behind_the_server_feature` reads every `src/**/*.rs` at run time (from
`CARGO_MANIFEST_DIR`, so a file added later is scanned without being listed), cuts each at
its first top-level `#[cfg(test)]`, and asserts that outside the `server`-gated files and
`#[cfg(feature = "server")]` blocks there is no `SystemTime`, `Instant::now`, `std::fs`,
`std::env`, `std::thread`, `std::process`, `tokio::` or `axum::`. Two anti-vacuity partners:
it asserts the gated set is non-empty and that at least one gated site DOES match (so the
pattern is live), and it fails if a listed gated file does not exist. It is the shape of
`web/e2e/every_spec_can_skip.spec.ts`, which reads its directory at run time for the same
reason.

### 6.3 The mutations that must be caught (recorded in the close with the catcher that fired)

1. A `SystemTime::now()` planted in `tables.rs` → §6.2's cell red; the CI check line stays
   green, and the close says so (H-1).
2. An `axum::http::StatusCode` re-imported into `api.rs` → the check line red.
3. A `tokio::time::sleep` planted in `client.rs` → §6.2's cell red (the check line would be
   green: `time` builds).
4. A handler in the dispatcher answering 200 where the router answered 403 → the
   dispatcher-equals-router cell red.
5. A re-worded JSON rejection → the pin against axum red.
6. The record route serving `serde_json::to_string(record)` instead of the stored bytes → the
   digest compare red.
7. The record route answering 200 before game over → its 409 cell red and the seed pin's
   partner reads the seed on a running game's wire.
8. `lock` left in the archived table document → the strip pin red.
9. `delete` deleting before `put` → the failing-archive cell finds the table gone.
10. `put(Over)` called on every persist → the archives-once cell red.
11. `archived_at` unbounded → the `check_shape` cell red.
12. The clock seam bypassed (`now_unix` called directly) → `with_clock`'s pin red AND §6.2
    red, two catchers.
13. A second `Why` path in `DirArchive` not derived from `archive::paths` → the layout cell red.
14. The adapter answering HEAD with 405 → the HEAD cell red.
15. The dispatcher refusing an `Err` identity with a sentence of its own → the 401 pin red.

(Regenerating the corpus and reading the diff is a DISCIPLINE, §7, not a mutation catcher —
the same note the adversarial read made about the umbrella's §4.)

---

## 7. The wire

`health.json` +1 key (`archive`); `record_409.json` new; every other committed payload
byte-identical; the seed pin's second partner; the dispatcher cell. Regenerate with
`UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures` and READ THE DIFF at
every task that changes a shape (R-3-1).

---

## 8. Byte-identical

`crates/yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge`,
`gen-data`: empty diff against `faf1a88`. `crates/yokohama-cli`: empty diff. `SEMANTICS_REV`
7. The close records the five-crate diff and the CLI diff by command.

---

## 9. Deployment

`yokohama-web:0.5.0-m6.5a` on the NAS from the close HEAD, the create body of §3.7 plus
`YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive`; verified through the tunnel; one throwaway table
abandoned with events and its two files read off the volume, one game played to the end
against the bot and its two files read; the runbook's §3.12. CI: the `web-server` job gains
the target and the check line.

---

## 10. What this spec does NOT claim

- Nothing runs in wasm this cycle; the check line proves the crate BUILDS for the target
  without `server`, and §6.2 proves no platform call survives outside it. Running it is
  M6.5b.
- The dispatcher is proven against the corpus and the 74 route cells, not against a second
  host.
- The archive's retry at game over on the axum host is the next abandon or the operator;
  no alarm exists on this host.
- An archived vs-bot game that is undone past its end and finished again keeps its FIRST
  completion in the archive (R-6.5a.11); the second ending is in the store and not in the
  archive.
- No sizing; no date; the counts in the close are the counts.

---

## 11. Read order for the plan writer

1. This spec, §0–§2.
2. The umbrella's §2.1 and §6.1.
3. The survey's §1.3 (the thirteen steps), §1.4 (the helpers), §2 (the `impl Games` table),
   §4 (the test surface), while it exists.
4. `crates/yokohama-web/src/lib.rs:70-160`, `:186-240`, `:255-322`, `:506-700`, `:1103-1284`,
   `:2116-2146`; `games.rs:107-230`, `:363-560`; `tables.rs:686-800`, `:1648-1696`;
   `store.rs:117-160`; `table.rs:170-215`, `:410-425`; `tests/wire_fixtures.rs:130-300`,
   `:2220-2300`.
5. `web/src/api.ts:170-290`, `web/src/types.ts:447-470`, `web/src/Abandon.tsx`,
   `web/src/App.tsx:1240-1280`.
