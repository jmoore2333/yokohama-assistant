# M6.5a "the seam" — close

**Executed 2026-09-20 on branch `web-room-m6.5` (stacked on `web-room-m6.4` at `faf1a88`), from
the plan `docs/plans/2026-09-19-web-frontend-m6.5a-plan.md` under the spec
`docs/plans/2026-09-19-web-frontend-m6.5a-seam-spec.md` (R-6.5a.1–16) and the umbrella
`docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` (R-6.5.1–13). Close HEAD:
`42832f8` — `346e7ef` plus one test-only commit (§4.4's adjudicated residual). This is the record of record for M6.5a; the M6.5b entry artifact is
`docs/plans/2026-09-20-web-frontend-m6.5b-handover.md`.**

**The headline.** One table's logic is out from under axum. `crates/yokohama-web` builds for
`wasm32-unknown-unknown` with `--no-default-features` (a CI check line proves the dependency
fence; a source-scanning cell proves the platform-call fence), every game and house route is a
typed `pub fn` over `&Games` behind ONE route table and ONE `dispatch` in a new `api.rs`, and
axum is an adapter of about forty lines plus the three transport routes. The change signal, the
clock and the archive are seams on `Games` — a trait, a closure and a trait — with a no-op, a
ticking counter and a directory as this cycle's implementations. A finished game is archived
ONCE under `rev7/over/`, an abandoned one under `rev7/abandoned/`, a seat holder can download
the finished record as the store's exact bytes, and the client saves it as a file without ever
parsing it. Every committed wire payload is byte-identical except `health.json`, which gained
`"archive"`, and `record_409.json`, which is new. The five fenced crates and the CLI are
untouched; `SEMANTICS_REV` is 7; no bot behaviour moved.

**How it was run.** Tasks 1–2 under subagent-driven development (an Opus implementer and an
Opus reviewer each; Task 1 took two fix rounds over one cell). The owner then ruled, after Task
2: *"move to inline execution instead of subagent driven, these tasks are small enough we should
be able to handle the review phase more broadly than with such minor edits back and forth."*
Tasks 3–11 were executed inline by the orchestrator with the per-edit gates after every task,
one Opus review at the structural mid-point (Tasks 3–6) and one Opus whole-branch review before
the release; the fifteen-mutation battery by an Opus agent. The ledger of every ruling is
`.superpowers/sdd/2026-09-19-web-frontend-m6.5a-plan/progress.md` (gitignored; §5 below is its
public copy).

---

## 1. What shipped

### 1.1 The signal is a trait (Task 1, R-6.5a.8)

`signal::Signal { bump, bump_table, bump_house_chat, end }` — four methods over `&str`, exactly
`Signals`' four at HEAD, unconditional; `NoSignal` the unconditional no-op; `Signals` (the tokio
watch channels, unchanged in behaviour) implements it under `server`, and `subscribe`,
`subscribe_house` and `senders` stay on `Signals` for the transport routes alone. `Games` holds
`signal: Arc<dyn Signal>`; the eight bump sites did not move. `AppState` holds the concrete
`Signals` and hands `Games` a clone — the streams are the only readers of a bump. Seventeen
cells (not the plan's four) subscribed through the deleted `Games::signals()` and now build
their own `Signals` through one helper, `test_support::watched_games`; every gated proof (a bump
FOLLOWS its write) is unchanged. The fifth cell the spec asked for,
`every_bump_site_answers_under_no_signal`, drives all four trait methods on the no-op through
real bump sites — the plan's body (a `chat_last` read) reached none and was superseded by the
spec (§5, ruling T1).

### 1.2 The clock is a seam (Task 2, R-6.5a.6)

`games::Clock = Arc<dyn Fn() -> u64 + Send + Sync>`, on `Games`, read at the two stamp sites
(`tables.rs`'s `create` and `chat_append`); `with_parts` takes it; `with_clock` is the builder
step a cell pins a stamp with. Both sites are mutation-proven (a `now_unix()` restored at
either fails the cell on that site's assertion). `AppState` passes `table::now_unix`, which is
`server`-only since Task 4. **A `Games` built without a host reads a TICKING clock** (§5, P-7):
`Games::ticking_clock` returns 1, 2, 3 per reading with no platform call; two list cells that
had slept 1.1 s for a wall-clock second dropped their sleeps.

### 1.3 `Games` carries the chat facts (Task 3, R-6.5a.1)

`Games.chat: ChatConfig` (pub) replaces the private `limits`; `AppState.chat` is gone;
`admitted`, `lobby_chat_on` and `events_at` take `&Games`. No cell moved.

### 1.4 The `server` feature and the two fences (Task 4, R-6.5a.2, .5, .6, .7)

`default = ["server"]`, `server = ["dep:axum", "dep:tokio", "dep:tokio-stream"]`, the three
made optional; `http = "1"` direct and `ApiError.status: http::StatusCode`; `getrandom = {
version = "0.2", features = ["js"] }` under `[target.'cfg(target_arch = "wasm32")']`, the one
manifest line about wasm; `[[bin]] required-features = ["server"]`. Gated IN PLACE, per item:
every axum item in `lib.rs` (68 items, the `surface!` macro and its three invocations, the
imports), `identity::from_env` and the extractor, `FsStore` and `default_dir`,
`ChatConfig::from_env`, everything in `signal.rs` after `NoSignal`, `table::now_unix`, and
`art.rs`/`assets.rs` whole. **The source convention** (written into `lib.rs`'s module doc): a
`server`-only region begins with `#[cfg(feature = "server")]` (one per item) and ends with
`// ---- end server ----` at column 0; a whole-`server` file begins with the inner attribute.
`platform_calls_stay_behind_the_server_feature` reads the crate's source cut at each file's
test module and names any of eight patterns (`SystemTime`, `Instant::now`, `std::fs`,
`std::env`, `std::thread`, `std::process`, `tokio::`, `axum::`) outside a region; RED before the
gating named 31 sites (the Task 4 commit message says 33 — a miscount, §4.3 M4), GREEN after
with 57 gated hits. `cargo check -p yokohama-web --no-default-features --target
wasm32-unknown-unknown` passes and joins CI's `web-server` job, which now installs the PINNED
toolchain with the target (§5, P-3). `Cargo.lock` grew by `http` and the wasm-only
`getrandom`/`js-sys`/`wasm-bindgen` graph, never built natively (§5, P-2).

### 1.5 `api.rs` — the layer's front door (Task 5, R-6.5a.3, .4)

`ApiRequest { method, path, query, content_type, body }`, `ApiResponse { status, body }`,
`Host { identity_name, signal_kind }`, `Identity<'a> = Result<&'a User, &'a str>`; the route
table `routes()` (28 rows at the close, `app()`'s old order plus `record`); a segment matcher
with HEAD → the GET arm, 405 with an empty body for a known path under another method, the JSON
404 for no match; `dispatch` over `answer`, whose fall-through checks identity FIRST so a game
route given the host's `Err` sentence answers that sentence as a 401 — in every mode, pinned.
**The parsers make the same calls axum's extractors make**: `parse_json` — `mime` for the
content-type rule, `serde_path_to_error` over `serde_json`'s deserializer, the error classified
into the DATA or the SYNTAX sentence, `end()` for trailing input; `parse_query` —
`serde_path_to_error` over `serde_urlencoded::Deserializer::new(form_urlencoded::parse(..))`,
which is why its sentence carries the field's path (the plan's `from_str` sketch would not
have matched — R-plan-3); `parse_path_usize` for `{k}`, whose sentence says `u64` because
axum's path deserializer visits a `usize` as one (the plan said `usize`; the pin caught it).
Every sentence is pinned WORD FOR WORD against axum's own `body_text()` in lib.rs's server
cells — both JSON arms, the missing content type (the fix wave's third arm — the first draft pinned it to a literal, §4.3 I5), the query, and
the path against a throwaway axum router (a `Path` cannot be extracted outside one).
`Health`, `Me`, `Labels` and `labels`' body MOVED verbatim (field order is wire order). The
adapter resolves identity with `resolve`, hands bytes and text to the layer, answers `/api/art`
itself (the manifest reads the gated `art` module) and keeps axum's 2 MiB body limit.

### 1.6 Every route in the layer; `app()` is an adapter (Task 6, R-6.5a.3, .14)

Twenty handler bodies, the nine helpers (`admitted` first) and fourteen DTOs moved from
`lib.rs` into `api.rs` as `pub fn`s over `&Games` — `Identity(user)` → `user: &User`,
`Path(id)` → `id: &str`, `JsonBody(req)`/`QueryArgs(q)` → `&T`, `Ok(Json(v))` → `Ok(v)`, 201 and
204 to the dispatch arm — and the `surface!` macro became `fn surface<T>`. **The mid-point
review diffed all twenty-nine against their pre-images after the prescribed rewrites and found
them identical**; `dispatch`'s arm order (identity, then body or query, then the route) matches
the extractor order every moved handler had. `app()` is `events`, `stream`, `house_stream`,
`/api/{*rest}` and `/api/games` on the adapter, and `lib.rs` re-exports the DTOs for the cells,
the corpus and the client contract. **`the_dispatcher_answers_what_the_router_answers`**
compares AT CAPTURE TIME inside the walk's `call_as` — every GET/HEAD and every 4xx, never a
landed write, never the transport routes or `/api/art` (§5, P-1) — 234 pairs over the
74-payload walk, all equal, floor 210. **Read that cell at its real size** (§4.3 I3): after
Task 6 both sides of every pair are the adapter's own `dispatch` call, so what it proves is
that the ADAPTER's request conversion is faithful — path/query split, content type, body bytes,
status round-trip — not that two implementations agree; spec §6.3's mutation 4 is caught by the
corpus compare and the route cells, and §4.5 records it so.

### 1.7 The archive (Tasks 7–8, R-6.5a.9, .10, .11)

`archive::Archive { put(why, id, record_json, table_json), kind() }`, `Why::{Over, Abandoned}`,
ONE layout function `paths(rev, why, id)` → `rev<N>/<why>/<id>.json` + `.table.json`,
`strip_for_archive` (lock and watcher roster removed, `archived_at` kept), `NoArchive`
(`"off"`) unconditional, `DirArchive` under `server` — temp + rename, REFUSES an existing path,
`YOKOHAMA_WEB_ARCHIVE_DIR` unset = off, a directory that cannot be created is an `Err` `main`
exits 1 on, naming the variable. `TableDoc.archived_at: Option<u64>` with `#[serde(default)]`,
`TABLE_SCHEMA` 3, bounded in `check_shape` at `MAX_CREATED_AT`, a schema-2 document reads with
`None`. `Games` holds `archive: Arc<dyn Archive>` (`with_parts` is six parameters; `with_archive`
the builder step). `AppState::new` returns `Result` and builds the archive from the environment;
`in_memory*` keep none. `Health.archive = games.archive.kind()`. **The two sites**: in `persist`,
after `mirror_turn` and before the bump, an unstamped table whose session is over is
`put(Over)` with the record bytes JUST WRITTEN; on success `archived_at` is stamped (retried
once on a version conflict); on failure the move is still answered and the stamp stays unset —
the retry mark. In `delete`, under the table's lock, after the creator check and BEFORE the
tombstone, a playing table with at least one event and no stamp is `put(Abandoned)`; a failed
put REFUSES the delete with "the archive could not be written (<why>) — the table was not
deleted"; open, dealing, moveless and already-stamped tables delete as before. Five cells over a
recording/refusing fake, the finished game a second `Repl` over the very game pumped as two bots
and persisted through `Games` (2 persists, 1 put). The client's contract lines for
`Health.archive` landed in the same commit (§5, P-4).

### 1.8 The record route (Task 9, R-6.5a.12, .14)

`GET /api/games/{id}/record`: `admitted` first (a spectator is 403 "the record is the players' —
watch, download nothing"), the handle's `is_over` (a running game is 409 "the record is
available once the game is over"), then the store's `Doc::Record` STRING as the body — never
re-serialized, so the digest the store stamped is the one the file carries. `record_409.json`
is captured mid-walk beside `refused`; the seed pin's SECOND partner plays the walk on its own
state and reads the seed in the 200 body — the corpus-wide search is looking for a string a
route really emits.

### 1.9 The client (Task 10, R-6.5a.13)

`api.recordText(id)` reads the body as TEXT and never parses it (`GameRecord.seed` is a u64 that
`JSON.parse` would round), with `call`'s refusal shape; **Save game file** on the finished-game
header downloads the very string as `<id>.json`; `archiveKind()`/`setArchiveKind` read
`Health.archive` at boot; the Abandon control reads "Abandon and save" at a PLAYING table when
the archive is on — the near truth, since `TableView` carries no event count — with the
accessible name unchanged; the help panel's saving section says where an abandoned game goes.
Six cells (vitest 634 → 640, `api.test.ts` new), and the vs-bot e2e walk asserts the saved
bytes equal `GET …/record`'s body fetched raw from the page (172 moves, 18 s).

### 1.10 Deployment documents (Task 11)

`deploy/Dockerfile.runtime` names `YOKOHAMA_WEB_ARCHIVE_DIR` beside the identity and chat
variables; `deploy/RUNBOOK.md` §3's create body carries it as its second `Env` entry
(`/data/archive`, the store's own volume) and §3.12 is the placeholder for the
`0.5.0-m6.5a` record, written at the deploy — **which this session did not perform** (§7).

### 1.11 The fences held

`git diff faf1a88 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest
crates/yokohama-bridge crates/gen-data crates/yokohama-cli` is EMPTY at the close HEAD;
`SEMANTICS_REV` 7; `guide_sync` 18 unchanged; every `Games` public method signature is what it
was; the corpus is byte-identical except as §3 says, proved by the compare cell after every
task and by a regeneration whose diff was read at Tasks 5 (empty), 6 (empty), 7 (`health.json`
+1 key) and 9 (`record_409.json` new), exactly R-plan-2's four readings.

---

## 2. The counts at the close HEAD, in ONE sequence

Measured twice as one uninterrupted script each time — at `3f420d8` (the mid-point fix wave,
06:22–06:36Z) and at `346e7ef` (the whole-branch fix wave; the log is
`.superpowers/sdd/2026-09-19-web-frontend-m6.5a-plan/counts-346e7ef.log`). The two readings
agree on every figure; the second is the close's. The close HEAD `42832f8` is `346e7ef` plus
one commit that changes two conditions inside ONE test cell (§4.4 NEW-1) — `cargo test -p
yokohama-web` (259) and clippy were re-run at it; every other figure below is unchanged by
construction (no production code, no client file, no wire payload moved).

| gate | reading |
|---|---|
| `cargo test -p yokohama-web` | **259** = **247** lib + **2** whole-game gate + **10** `wire_fixtures` (M6.4: 230 = 219 + 2 + 9; +29) |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown` | clean (**the new gate**, Task 4 on, in CI's `web-server` job) |
| `cargo test -p yokohama-cli` | **851** (823 lib + 9 `bga_import` + 18 `guide_sync` + 1 `piped_dogfood`) **+ 2 ignored** — unchanged: the CLI is untouched |
| `git diff --stat faf1a88 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli` | empty |
| `SEMANTICS_REV` | 7 |
| corpus (`web/src/wire/*.json`) | **83** (82 + `record_409`); `git diff --stat faf1a88 -- web/src/wire/` = `health.json` +1, `record_409.json` +3, `contract.ts` +2/−1 |
| `npm run typecheck` | clean |
| `npm test` | **640** in **38** files (M6.4: 634 in 37; +6) |
| `npm run e2e` | **39** in **8** spec files, **11.1 min** at `346e7ef` (the vs-bot game 533 moves; the two-browser game 2,735 loop iterations, 0 idle looks, 494 s, the watcher 0 `/events` and 1 stream; 11.3 min at `3f420d8`) |

The whole-game gate's vs-bot arm livelocked ONCE in a combined `cargo test -p yokohama-web` run
this cycle (>10 min, killed) and passed alone every time (27 s, 61 s, 35 s, 37 s, 36 s) and in
both counts sequences (28.9 s) — R-plan-6, the M6.4 close §6.3's first owner item, unchanged.

---

## 3. The corpus diff

Two files moved in 82 → 83: `health.json` gained exactly `"archive": "off"` (Task 7);
`record_409.json` is new, `{"error": "the record is available once the game is over"}` (Task 9).
Every other committed payload is byte-identical to `faf1a88`'s — every `ClientState`, every
`TableView`, every `HistoricalFrame`, every 4xx — which is what proves the cut moved no wire
shape. The client's `HEALTH` root-key row and `types.ts`'s `Health` gained `archive` in the same
commit as the fixture (P-4), so `npm run typecheck` was green at every commit.

---

## 4. The reviews

### 4.1 The per-task reviews (Tasks 1–2, Opus)

Task 1: spec ❌ (one) — the `NoSignal` cell called only `chat_last`, which reaches no bump
site; RULED against the plan's body (§5, T1), fixed, then a false doc sentence in the fix
("reached here and nowhere else in this binary") caught in the scoped re-review and fixed in a
second round; six Minors deferred (§6.2). Task 2: spec ✅, Approved, three Minors deferred.

### 4.2 The process change (owner, after Task 2)

Quoted above. From Task 3 the orchestrator implemented inline and ran the gates itself; review
became the mid-point review (§4.3) and the whole-branch review (§4.4).

### 4.3 The mid-point review (Opus, over `1d3b1aa..340b5c1` — Tasks 3–6) and its fix wave

0 Critical / 5 Important / 8 Minor; range quality Needs fixes; every one of the twenty moved
handlers and nine helpers verified identical to its pre-image; dispatch's refusal order
verified against every pre-image signature; the catch-all beside the transport routes builds.

- **I1** `JsonBody` was NOT deleted though the Task 6 commit message and the ledger said it
  was (a `pub` item raises no dead-code warning). **I2** the scanning cell cut at the first
  `#[cfg(test)]` ANYWHERE, not the first top-level one — `games.rs:671` and `signal.rs:127`
  carry inner ones, so `games.rs:675-706` and `signal.rs:128-169` were never scanned and a
  `SystemTime` planted at `games.rs:705` was MISSED (measured). **I3** the dispatcher cell
  compares `dispatch` with itself (§1.6). **I4** `AppState::with_store_and_chat`'s doc described
  a duplication Task 3 removed, and `AppState.rules` had zero readers. **I5** the missing-
  content-type sentence was pinned to a literal, not against axum. **M1** 405s lost axum's
  `Allow` header (`any()` skips it). **M2** an oversize body's sentence changed ("the body could
  not be read: …" for axum's "Failed to buffer the request body: length limit exceeded"; same
  status, nothing pins either). **M3** HEAD `/api/art` returned a body. **M4** Task 4's commit
  message said RED named 33 sites; the cell's algorithm at `a9a3913` names 31 — the orchestrator
  miscounted the printed lines; the examples named are all correct. **M5** two moved DTO docs
  still cited `QueryArgs`. **M6** `Games::rules()` undocumented. **M7** the route-table cell's
  premise ("every pattern app() registers") stale. **M8** an ungated item INSIDE a `server`
  region reads as gated — latent (every item in `lib.rs`'s 516-line region is genuinely
  `server`), a hazard for the next region author.

**The fix wave (`3f420d8`)**: I1 `JsonBody` and its impl deleted, its story re-homed on
`QueryArgs` (the one axum extractor left, for `events?after=&wait=`); I2 the cell split on
`"\n#[cfg(test)]"` (a first fix — superseded by §4.4's rewrite); I3 the two docs say the cell
pins the adapter; I4 `AppState.rules` removed and the constructor doc corrected; I5 the missing-
content-type sentence became the JSON pin's third arm; M1 the adapter writes axum's `Allow` header
on a 405 from `api::allowed_methods` (pinned at `GET,HEAD` and `GET,HEAD,DELETE`); M3 HEAD
`/api/art` strips its body; M5–M7 the three sentences corrected; and the battery's survivor
(§4.5 #13) got its catcher. Recorded, not fixed: M2, M4 (31 not 33), M8 (re-raised in §4.4 and
fixed there). lib 247 (+1). The commit message's "same 57 gated hits" is itself off: the cell's
algorithm counts **64** at `3f420d8` — 57 at `340b5c1` plus `archive.rs`'s seven — the
whole-branch review's measurement.

### 4.4 The whole-branch review (Opus, over `712ddf7..3f420d8`; `review-final-whole-branch.md`)

**Ready to merge WITH FIXES: 0 Critical / 2 Important / 11 Minor**; every deferred minor in the
ledger triaged (two FIX BEFORE MERGE, twelve LEAVE with a reason each); the wasm check line and
the scanning cell's algorithm re-run independently over the committed source (0 escapes, 64 gated
hits).

- **IMP-1** — the game-over stamp retry's stated reason was FALSE: a lost stamp cannot make the
  next abandon's put a refusal, because the abandon puts under `abandoned/`, a different path;
  what it does is archive the same game a SECOND time under the other leaf — and the trait doc's
  "a game id is archived at most once" is true only per `(id, reason)` and only through the
  callers' stamp. Nothing lost, nothing leaked; a correctness-of-the-record sentence.
- **IMP-2** — the scanning cell gated by REGION, not item: 984 lines sat inside
  `#[cfg(feature = "server")]` … `// ---- end server ----` spans (534 in `lib.rs`), and
  `lib.rs:387`'s `pub use api::{…}` was already an UNGATED item inside one — so the mid-point's
  "latent" M8 was present at HEAD, and a `SystemTime` on such an item would escape both fences.
- **Minors**: the stamp write's `Io` arm silently swallowed; the 64-not-57 figure; two
  constructors both claiming to be "the one"; a stale `QueryArgs` attribution in a test doc; the
  `watched_games` universal (deferred from Task 1 — FIX BEFORE MERGE); `recordText` inserted
  between `chatPath`'s doc and `chatPath`; HEAD stripped only on `dispatch`'s `Ok` arm; the
  object URL revoked synchronously with a detached anchor; `record_409.json` the one refusal
  fixture untyped in `contract.ts`; the oversize-body sentence (record); `matches` binding an
  empty segment (`is_safe_id` refuses it; noted as the one place the layer's matcher and matchit
  could differ).

**The fix wave (`346e7ef`)**, one commit, then the single scoped re-review the process
allows: IMP-1 — both sentences rewritten (per `(id, reason)`; a lost stamp is a DUPLICATE, not a
refusal) and the stamp write's `Io` arm and a vanished table logged; IMP-2 — the `pub use` block
moved above the imports and **the cell rewritten to ENFORCE the convention**: inside a region
every column-0 item must carry its own attribute or is named ("ungated item inside a server
region"), an item runs to the next column-0 `}` unless its first line ends in `;`, and the
top-level test module is cut OUT rather than everything after it, so a platform call written
below `mod tests` — the battery's residual — is read too. RED measured twice by planting and
reverse-editing (an ungated `SystemTime` item inside `lib.rs`'s import region; one appended below
`table.rs`'s tests), GREEN at HEAD, the module doc's convention paragraph says "enforced". The
eight minors above fixed; the three "record" items recorded here. lib 247 / wire 10 unchanged.

**The scoped re-review** (the same reviewer, over `3f420d8..346e7ef`): **all ten ADDRESSED**,
nothing regressed, the cell's algorithm re-run independently (0 escapes, 64 gated hits, RED on both
plants); **one new Minor** (NEW-1) — the item-span rule read a one-line item ending in `}`
(`impl std::error::Error for GameError {}`) and a const array closing with `];` as still OPEN, so
the column-0 items after them were swallowed — harmless at HEAD (neither file has a region), but
a gated one-line item inside a region would have re-opened IMP-2's hole. **Adjudicated and FIXED
in `42832f8`** (§5): two conditions in the cell — an item closes on its own line when it ends in
`;` or `}`, an open one closes at a column-0 `}`, `]` or `)` — GREEN at HEAD and RED on two plants
including the new Minor's own shape (a gated one-line struct followed by an ungated `SystemTime`
item). That commit is the close HEAD; it changes one test cell and nothing else, and
`cargo test -p yokohama-web` and clippy were re-run at it.

### 4.5 The mutation battery (Opus, spec §6.3, fifteen; `mutation-battery.md`)

**At `57f1f6b` (Task 11's head): 14 of 15 caught, 1 survived. At `3f420d8` (the fix wave): 15
of 15.** Each row planted the smallest edit that realises the defect, ran the predicted
catcher and any other cell that might fire, and reverted; the report carries the diff, the
command and the first line of every assertion. What fired, with the spec's prediction marked:

| # | planted | caught by | vs. the spec |
|---|---|---|---|
| 1 | `SystemTime::now()` in `tables.rs` | the scanning cell; the wasm check line STAYS GREEN (H-1 re-measured) | YES — and at `3f420d8` the same plant in `games.rs`'s formerly unscanned span is caught too; reverting the cell's split made it invisible again |
| 2 | `axum::http::StatusCode` in `api.rs` | the check line | YES |
| 3 | `tokio::time::sleep` in `client.rs` | the scanning cell AND the check line | PARTLY — the spec said the check line "would be green: `time` builds"; tokio is OPTIONAL now, so it is red too |
| 4 | the ADAPTER mapping a 403 to 200 | `the_dispatcher_answers_what_the_router_answers` | the spec's sentence names a dispatcher arm; a change INSIDE `api.rs` moves both sides (§1.6, §4.3 I3) and is caught by the corpus compare and the route cells; the adapter divergence is what this cell catches |
| 5 | a re-worded JSON sentence | the word-for-word pin (and, at `3f420d8`, the missing-content-type arm — the literal-only cell stays green when source and literal are edited together; only the axum-derived arm fails) | YES |
| 6 | `record` re-serializing | the record cell's byte compare | YES |
| 7 | `record` answering 200 before over | the walk's own inline 409 assertion, in all three wire cells, before the seed pin's search runs; the record cell's 409 arm | PARTLY — the spec's "the seed pin's partner reads the seed on a running game's wire" is never reached because the walk aborts first |
| 8 | `lock` left in the archived document | the strip pin | YES |
| 9 | `delete` proceeding past a failed put | the refusal arm ("delete returned Ok"), one assertion before "the table stays" | YES (one assertion earlier) |
| 10 | `put(Over)` on every persist | the archives-once cell | YES |
| 11 | `archived_at` unbounded | the `check_shape` cell | YES |
| 12 | `(self.clock)()` → `crate::table::now_unix()` | `with_clock`'s pin AND the wasm check line (`now_unix` is `server`-gated); the scanning cell stays green — `now_unix()` matches none of its eight patterns | PARTLY — two catchers, but not the two the spec named |
| 13 | `DirArchive::put` hand-building its paths | **SURVIVED at `57f1f6b`** on the `over/` leaf (the layout cell tests `paths()` directly; the directory cell asserted literals only under `abandoned/`); CAUGHT at `3f420d8` by the directory cell's two added `over/` assertions | NO → YES |
| 14 | HEAD mapped to 405 | the HEAD cell | YES |
| 15 | a 401 sentence of the layer's own | the 401 pin | YES |

**Residual blind spot the re-run found, recorded and LEFT (§6.2):** a platform call appended
BELOW a file's top-level `mod tests` escapes both halves of the fence — the scanning cell stops
at the module and the check line cannot see a `SystemTime`. It is held today by the convention
that `mod tests` is the last item in every file of this crate.

---

## 5. The rulings taken in execution, with their costs

Every ruling below is in the ledger with its `Ruling:` line; this is the public copy.

- **P-1 (Task 6)** — the dispatcher comparison runs AT CAPTURE TIME inside `Api::call_as`, for
  every GET/HEAD and every 4xx, skipping landed writes, the three transport routes and
  `/api/art`; the floor MEASURED then set (234 → 210). *Why:* the plan's sketch replayed after
  the walk against end-state (a `table_dealing` capture against a playing table) and would have
  double-applied every write; the owner-relayed note found the same two defects. *Cost if wrong:*
  a cell that proves less than the spec's sentence — and the mid-point review found exactly that
  (I3): it proves the adapter, not two hosts.
- **P-2 (Tasks 4, 5)** — `Cargo.lock` staged with every manifest change. *Cost:* none on
  behaviour.
- **P-3 (Task 4)** — CI's `web-server` job installs the PINNED toolchain 1.97.1 WITH the wasm
  target (`dtolnay/rust-toolchain@master`, `toolchain:`/`targets:` inputs) rather than
  `@stable` plus a bare `rustup target add`, which could land the target on a toolchain cargo
  never uses (the owner-relayed note, item 3). The other two jobs keep `@stable` (not this
  plan's scope — an owner item, §6.3). *Cost if wrong:* one CI-only red, one YAML line.
- **P-4 (Task 7 ↔ 10)** — Task 7 carries the client's `Health.archive` type and `HEALTH` row, so
  the typecheck is green at every commit. *Cost:* two lines moved between tasks.
- **P-5 (Task 6, contingency)** — explicit per-pattern registrations if the catch-all conflicted.
  NOT needed: `matchit` prefers the specific routes and the router built.
- **P-6 (Task 7)** — `archive.rs`'s `server` region ends with the marker. *Cost:* one comment
  line.
- **P-7 (Task 4)** — the host-less default clock TICKS (1, 2, 3) rather than the plan's
  constant 0. *Why:* a constant made every test table's `created_at` a tie and two newest-first
  list cells went red; a counter keeps the claim about the sort's first key and lets the cells
  drop their 1.1 s sleeps. *Cost if wrong:* test stamps are small integers, not unix seconds; a
  cell wanting a time-looking stamp uses `with_clock`.
- **T1 (Task 1)** — the plan's `NoSignal` cell body superseded by spec §2.4: the cell drives
  real bump sites under `NoSignal` and requires each to answer, crossing all four trait methods.
  *Cost:* one cell shaped differently from the plan's text.
- **R-plan-3 corrections** — `parse_query` goes through `serde_path_to_error` and
  `form_urlencoded` (the plan's `from_str` would have dropped the field path); the `{k}` sentence
  says `u64` (the plan said `usize`). The pins decided both.
- **The process change** (owner, after Task 2) — inline execution with broad review; recorded
  in the ledger and in the standing memory of how this project dispatches.
- **The deploy was NOT performed** by the executing session (§7): the owner's message at the
  close asked for a PR, a push and the M6.5b handover, and a deploy is an outward-facing step
  the message did not name.
- **The whole-branch review's two Importants and the deferred minors it marked FIX BEFORE MERGE
  were fixed in ONE wave** (`346e7ef`), per the process (one wave, one scoped re-review); its
  three "record" items are in §6.2 and §7, not in code.
- **The re-review's one new Minor was FIXED after the process's one re-review** (`42832f8`),
  adjudicated by the orchestrator rather than parked: the scanning cell is the fence M6.5b's
  gated items will lean on, a known blind spot in it is load-bearing, and the change is two
  conditions in one test cell measured by its own RED/GREEN readings. *Cost if wrong:* one
  test-cell edit no reviewer read.

### Deviations from the plan's letter, recorded here

- Task 1: seventeen cells re-pointed (the plan said four); `signal.rs`'s module doc kept.
- Task 2: the brief's creator email would have been refused by R-1.6; the cell also covers the
  chat-append stamp.
- Task 4: `serde_urlencoded` landed in Task 5 with its first use, not in Task 4's manifest text;
  `Doc::suffix` carries a no-server `allow(dead_code)` rather than a gate line (a gate line
  opens a region the scanning cell holds until a marker); the `surface!` macro escaped the
  per-item regex on first pass and the wasm check named it.
- Task 6: `JsonBody` — see §4.3 I1: the commit message claimed a deletion that had not
  happened; deleted in the fix wave.
- Task 8: `play_to_the_end`/`one_move_in` build a SECOND `Repl` over the very game (`Session`
  is not `Clone`; a `Repl` learns its bots at construction); the game-over stamp is retried once
  on a version conflict (a put that landed without its stamp would make the next abandon's put a
  refusal under `DirArchive`'s once-rule); the "once" floor is ≥ 2 persists, because a both-bot
  pump plays a run of up to 200 moves.
- Task 10: the anchor's click is intercepted in the cell (jsdom cannot navigate) and its
  `download`/`href` read off the spy; the Blob is read through `FileReader`.

---

## 6. The LEAVE list

### 6.1 Inherited and not touched

The M6.4 close §6.3's seven owner items — the vs-bot whole-game gate's livelock (it fired once
this cycle in a combined run, >10 min, and passed alone every time: 27 s, 61 s, 35 s, 37 s), the
e2e `CAP`, the disk, the house chat's undeletable lines, the lobby's three reads per house
message, the per-process chat state, WARP — stand.

### 6.2 This cycle's own (deferred minors, from the ledger)

- Task 1: the installed `NoSignal` is a stated premise in `every_bump_site_answers_under_
  no_signal` (an explicit `with_parts(.., Arc::new(NoSignal))` would make it an assertion);
  "the only cell reaching all four methods" is true but rots silently; "Every cell that watches a
  bump uses `watched_games`" is false as a universal (route cells watch via `AppState`); "the
  nearest other" is singular where three cells reach three.
- Task 2: the clock cell pins `with_clock` rather than `with_parts`; its `1_700_000_000` is one
  second below `freeze_clock`'s floor, contradicting the report's rationale for the value.
- Mid-point M2: an oversize body's 400 sentence under the adapter differs from `JsonBody`'s;
  nothing pins either. **M8**: the region convention's blind spot — an ungated item placed
  inside a `#[cfg(feature = "server")]` … `// ---- end server ----` span reads as gated; marker
  placement is load-bearing, and the next region author should close a region after every item
  that is genuinely `server`.
- Spec §10's edge: a vs-bot game undone past its end and finished again keeps its FIRST
  completion in the archive. **And its sibling, found by the whole-branch review (IMP-1)**: a
  game-over put that LANDS while its `archived_at` stamp does not (the stamp is retried once and
  every failure logged) would be archived a SECOND time under `abandoned/` at the next abandon
  — a duplicate, never a loss. "At most once" is per `(id, reason)` in the archive and per game
  id only through the stamp.
- The battery's residual and the whole-branch review's IMP-2 are CLOSED by the final fix wave
  (the cell enforces per-item gating and reads below the test module); what remains of the
  fence's honesty is in its own doc: the top-level test module is the one span it does not read.
- The 405 `Allow` header is back (M1 fixed); the oversize-body sentence under the adapter differs
  from `JsonBody`'s ("the body could not be read: …" for axum's "Failed to buffer the request
  body: length limit exceeded"), same 400 — not one of R-6.5a.4's four, nothing pins either.
- `api::matches` binds an empty path segment (`/api/games//table` reaches `get_table` with
  `id == ""`, which `is_safe_id` refuses): the one place the layer's matcher and matchit could
  answer differently; no leak, no panic.
- The Task 4 and the fix-wave commit messages carry two miscounts corrected here: RED named 31
  sites (not 33); the gated hits at `3f420d8` were 64 (not 57).

### 6.3 OWNER ITEMS

1. **Deploy `yokohama-web:0.5.0-m6.5a`** with `YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive`
   (RUNBOOK §3, §3.12) and read the two files of an abandoned and a finished game off the volume.
2. **The PR stack** — #31 → #34 unmerged; M6.5a's PR against `web-room-m6.4`.
3. **CI's other two jobs** still install `@stable` while `rust-toolchain.toml` pins 1.97.1 (P-3
   changed only the `web-server` job).
4. **"Abandon and save" is the near truth** (R-6.5a.13): a just-dealt table with no move reads
   "and save" and saves nothing; adding an event count to `TableView` would move eight fixtures.
5. **The record route and the Access door** — a seat holder can now download a finished game's
   record, seed included; behind the owner's allow-list that is the owner's own game; on the
   hosted path it is a decision M6.5c inherits with the email roster (M6.3 §6.3 item 1).

---

## 7. What this cycle does NOT claim

- **Nothing runs in wasm.** The check line proves the crate BUILDS for the target without
  `server`, and the scanning cell proves no platform call survives outside it. Running it is
  M6.5b.
- **The dispatcher is proven against ONE host.** The corpus, the 74 route cells and the
  adapter-conversion cell drive `dispatch` under axum; the browser host's corpus claim is
  M6.5b's own cell.
- **Not deployed.** The NAS runs `0.5.0-m6.4.1`; the archive directory has collected nothing;
  `/api/health` there does not yet say `"archive"`. §6.3 item 1.
- **The archive has no alarm on this host**; the retry at game over is the next abandon or the
  operator, and `archived_at` unset is the only sign.
- **The mutation battery's mutation 4** is caught by the corpus compare and the route cells,
  not by the cell the spec named (§4.5).
- **No sizing, no date**; the counts in §2 are the counts.

---

## 8. The handover into M6.5b

`docs/plans/2026-09-20-web-frontend-m6.5b-handover.md` — what the layer IS now with signatures,
what M6.5b builds (umbrella §3.2), the decisions its spec takes at its start (the local identity,
the JS clock, the Worker, the two tabs, the bot seed, a local abandon), what is owed from this
close (the deploy first), the open items carried, and the process as the owner refined it.
