# M6.5b "single-player in the browser" — the entry handover

**Written 2026-09-20 on branch `web-room-m6.5` at the M6.5a close, by the session that executed
M6.5a inline — for the session that opens M6.5b.** Under R-6.5.7 each sub-cycle is its own
brainstorm → spec → plan → execution; **the M6.5b spec is written at ITS start from this
document and the M6.5a close**, `docs/plans/2026-09-20-web-frontend-m6.5a-close.md`. Nothing of
M6.5b exists yet; every sentence below is about what M6.5a left and what the umbrella
(`docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md`) already ruled for M6.5b.

## 0. Read order (the session's first fifteen minutes)

1. `CLAUDE.md` — the Web (server) and Web (browser half) bullets, the `crates/yokohama-web`
   architecture bullet, the M6.5 sentence in "Where things stand".
2. `docs/handover.md` Track 3.
3. The umbrella spec — §1 (R-6.5.1–13), **§2.5 the browser host** (the paragraph M6.5b is
   built from), §3.2 (M6.5b's builds and exit), §4 (the mutations M6.5b must catch), §6.1
   (the open items, O-2, O-3, O-7, O-11, O-12 are M6.5b's).
4. The M6.5a close — §1 (what the layer IS now, with signatures), §5 (the rulings and their
   costs), §6 (the LEAVE list and the owner items), §7 (what it does NOT claim).
5. This document's §2 and §3.

## 1. What M6.5a built, as M6.5b consumes it

Everything below is in `crates/yokohama-web` at the close HEAD and builds for
`wasm32-unknown-unknown` with `--no-default-features` (the CI check line; the scanning cell
`platform_calls_stay_behind_the_server_feature` is the platform-call fence beside it).

- **The layer's front door** — `api::dispatch(games: &Games, host: &Host, identity: Identity<'_>,
  req: &ApiRequest<'_>) -> ApiResponse`, with `ApiRequest { method, path, query, content_type,
  body: &[u8] }`, `ApiResponse { status: u16, body: String }` (the body a JSON string in this
  API's shape, or empty for 204/405/HEAD), `Host { identity_name: &'static str, signal_kind:
  &'static str }`, and `Identity<'a> = Result<&'a User, &'a str>` — **identity is the host's
  sentence, carried**: a game route given `Err(msg)` answers 401 with `msg`. HEAD reaches the
  GET arm and leaves with an empty body. A known path under another method is 405 (the axum
  adapter adds axum's `Allow` header — the layer's `ApiResponse` carries no headers); a path
  that matches nothing is the JSON 404 `{"error":"no such endpoint: <path>"}`.
- **The route table** — `api::routes()`, 28 rows in `app()`'s old order: the four public
  routes (`health`, `me`, `labels`, `art` — the last answered by the HOST, listed for the
  405/404 arithmetic), every game and house route, and `GET /api/games/{id}/record`. The three
  transport routes (`…/events`, `…/stream`, `/api/stream`) are NOT in the layer: a host writes
  its own, or has none (the browser host has none — one tab, one game, the bot's run returns in
  the same call, umbrella §2.5).
- **The parsers** — `api::parse_json` and `api::parse_query` make the SAME calls axum's
  extractors make (`mime`, `serde_path_to_error` over `serde_json` with `end()`,
  `serde_urlencoded::Deserializer` over `form_urlencoded::parse`), so every 400 sentence is
  axum's word for word and is PINNED against axum's own `body_text()` in lib.rs's server
  cells; `{k}` in `…/at/{k}` parses through `parse_path_usize`, whose sentence names the type
  `u64` because axum's does. **A host that converts a request faithfully gets those
  sentences for free; a host that pre-parses gets different ones and fails the corpus.**
- **`Games` is the layer's state** — built by `Games::with_parts(rules, store, chat:
  ChatConfig, signal: Arc<dyn Signal>, clock: Clock, archive: Arc<dyn Archive>)`; every public
  method signature is what it was before M6.5a. `Games::new`/`with_limits` fill `NoSignal`, a
  TICKING clock (1, 2, 3 per reading — `Games::ticking_clock`, no platform call) and
  `NoArchive`; `with_clock`/`with_archive` are builder steps.
- **The three seams as traits/types** — `signal::Signal { bump, bump_table, bump_house_chat,
  end }` (`NoSignal` unconditional; `Signals` and the streams under `server`); `games::Clock =
  Arc<dyn Fn() -> u64 + Send + Sync>` (the two stamp sites read it; `table::now_unix` is
  `server`-only — **a wasm host passes a clock from JS**, since `SystemTime` panics on the
  target and the scanning cell refuses it anyway); `archive::Archive { put(why, id,
  record_json, table_json), kind() }` with `Why::{Over, Abandoned}`, ONE layout function
  `archive::paths(rev, why, id) -> (record_path, table_path)`, `strip_for_archive(&TableDoc)`
  (lock and watcher roster removed), `NoArchive` (`"off"`), and `DirArchive` under `server`
  (`"dir"`, `YOKOHAMA_WEB_ARCHIVE_DIR`).
- **The store** — `store::Store`/`Doc`/`MemStore`/`is_safe_id` unconditional; `FsStore` and
  `default_dir` under `server`. **`MemStore` is the store the browser host starts from**
  (spec R-6.5a.5); persistence to IndexedDB is a SIDE CHANNEL of the record's bytes, never a
  second `Store` implementation the browser has to keep consistent (umbrella §2.5).
- **The archive sites** (R-6.5a.11) — at game over in `persist` (best effort, the stamp
  `archived_at` is the retry mark) and at abandon in `delete` (awaited; a failed put REFUSES the
  delete). `TableDoc.archived_at: Option<u64>`, `TABLE_SCHEMA` 3, a schema-2 document reads
  with `None`.
- **The record route** — `GET /api/games/{id}/record`: a seat holder's FINISHED game as the
  store's exact bytes, 409 "the record is available once the game is over" while running, 403
  "the record is the players' — watch, download nothing" for a spectator. **Under the browser
  host the route still answers exactly as the corpus pins it** (`record_409.json`), while the
  local persistence and "Save game file" read the bytes directly (umbrella §2.5).
- **The client's edges** — `api.recordText(id)` reads the record as TEXT and never parses it
  (R-6.5.6: a `u64` seed does not survive `JSON.parse`); "Save game file" on the finished-game
  header downloads it as `<id>.json`; `Health.archive: "off" | "dir" | "r2"` is read at boot
  into `archiveKind()` and the Abandon control reads "Abandon and save" at a playing table
  when it is on; `web/src/wire/contract.ts`'s `HEALTH` row carries `archive`.
- **The corpus** — 83 committed payloads under `web/src/wire/`; `health.json` carries
  `"archive": "off"` (the host-parameterized payload R-6.5.9's second class names, with
  `identity` and `signal`); `record_409.json` is the 409. `the_dispatcher_answers_what_the_
  router_answers` runs inside the walk's `call_as` and proves the axum ADAPTER's request
  conversion is faithful (status and body byte-equal through `dispatch` for every read and
  refusal); **it is not a second host** — the browser host's corpus claim is M6.5b's own cell.

## 2. What M6.5b builds (umbrella §3.2, restated with the seam it lands on)

1. **`yokohama-wasm`** — a new crate (or a `wasm` feature of `yokohama-web`; the spec decides)
   exposing ONE function, request-in / response-out over the same method, path, query and
   body: it builds an `ApiRequest`, resolves a LOCAL identity (see §3.1), and calls
   `api::dispatch` over a `Games` built with `MemStore`, `NoSignal`, a JS clock and
   `NoArchive`. `Host { identity_name: <the local mode's name>, signal_kind: <none> }` — both
   values are the M6.5b spec's to name, and `/api/health` served locally is the payload whose
   `identity`, `signal` and `archive` fields the spec lists as host-parameterized (R-6.5.9).
2. **The local arm in `web/src/api.ts`** — `call` (the module's one `fetch` for the API) and
   `recordText` dispatch to the wasm function when the game is local; `usePolled`'s `open()`
   (`signal.ts`) needs a third arm (no stream, no held request); `identityHeaders()` is skipped
   under the local arm; **`/api/health` is answered locally too**, or the three module-level
   deployment facts keep the server's values (umbrella §2.5).
3. **Persistence** — the record's bytes to IndexedDB after every applied move, read back as
   bytes, `Session::from_record` on reload; the two-tab rule (Web Locks, or a revision check
   with a defined second-tab screen — O-12); how the lobby re-finds local games.
4. **The Web Worker decision** — taken at the spec's start (the lean: yes), because the
   replay-on-load cost (O-3) is the unmeasured quantity and `suggest`/`why` run the same
   machinery.
5. **"Play the bot here"** beside "New table"; **"Send to the archive"** on the finished-game
   screen, dark until the server's health reports an archive, and its receiving route
   `POST /api/archive` on the axum host with the FOUR checks of R-6.5.6 (the `semantics_rev`
   gate, the digest PRESENT, `replay_strict`, game over) writing under a `browser/<user>/`
   prefix through the same `Archive` trait — a THIRD `Why`, or a prefix argument, is the
   spec's call; `archive::paths` stays the one layout function.
6. **The determinism pin** — a record played by the wasm bot reconstructs byte-identically
   under the NATIVE `why` (`record::state_at` + the same `decide` at the same `(bot_seed,
   events.len())`), with `placement_cap_binds == 0` as its scope partner (umbrella §0.3, O-11);
   the same run records the wasm replay's wall (O-3).
7. **The browser host's corpus subset** — named in the spec, regenerated against the local
   dispatch, byte-identical to the committed files except the host-parameterized fields the
   spec lists.
8. **The exit** — a whole game against the bot in the browser with the page's own request log
   showing ZERO `/api/games` calls after load; the save downloaded and reconstructed natively
   under `why` with zero refusals; shipped on the NAS (this host needs no server).

## 3. Decisions the M6.5b spec takes at its start (each reshapes the dispatch if retrofitted)

1. **The local identity.** `dispatch` takes `Result<&User, &str>` and every game route reads the
   `User` for seats and admission. The browser host has no door; the spec names the one local
   `User` (a fixed address the lobby never lists, or the browser's remembered display name
   normalized) and rules whether `me` reports it. A `TableDoc` written locally carries that
   address in `created_by` and `seats` — and a record archived by "Send to the archive" is
   written under the SERVER's caller, not this one.
2. **The clock.** A JS clock through `with_parts` (`js_sys::Date::now() / 1000`, or a clock
   handed in on each call); `created_at` and a message's stamp read it. The ticking default is
   for tests.
3. **The Worker and the two tabs** (§2.4 above; O-12).
4. **The bot's seed.** `Games::create` draws the bot seed from OS entropy through `draw_seed`,
   which is `rand`'s `OsRng` — the umbrella measured `getrandom`'s `js` feature as the one
   blocker and M6.5a's manifest carries it under `[target.'cfg(target_arch = "wasm32")']`;
   the spec records that a local game's seed is drawn the same way and never typed.
5. **What "abandon" means locally** — with `NoArchive`, `delete` archives nothing; a local
   game's Abandon reads "Abandon" (the label reads `archiveKind()`, which the local health
   answers `"off"`). If a local abandon should offer "Send to the archive" instead, that is a
   spec sentence.

## 4. Owed from M6.5a's close, in order

1. **The NAS deploy of `yokohama-web:0.5.0-m6.5a`** (owner's authorization; the recipe is
   `deploy/RUNBOOK.md` §3 with the create body's second `Env` entry
   `YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive`, and §3.12 is the placeholder for its record): after
   the swap, `/api/health` reads `"archive": "dir"`; abandon one throwaway table with a move in
   it and read `/data/archive/rev7/abandoned/<id>.json` + `.table.json` off the volume; play one
   game to the end and read the two files under `over/`. The M6.5a close §7 says NOT deployed.
2. **The pull-request stack** — PR #31 → #32 → #33 → #34 are unmerged; M6.5a's PR targets
   `web-room-m6.4` in the same shape; merge order is the owner's.
3. **Re-run the whole-game gate alone before believing a red** (R-plan-6; it livelocked once in
   this cycle's combined runs and passed alone every time).

## 5. Open items carried into M6.5b

- Umbrella §6.1: **O-2** (the 1 s startup against a 1.31 MB module — a Worker measurement),
  **O-3** (replay on every wake — measured by the determinism pin's run), **O-7** (`getrandom`
  `js` inside a Worker — proven in the browser path only when the module runs), **O-11** (the
  placement reservoir's width-dependent `usize` draw — unreached at 2P, the pin's scope
  partner), **O-12** (two tabs on one record).
- M6.5a spec §10: an archived vs-bot game undone past its end and finished again keeps its
  FIRST completion in the archive (the stamp stays set; `DirArchive` refuses a second put).
- The M6.5a close §6 LEAVE list — the region convention's blind spot (an ungated item INSIDE a
  `server` region reads as gated; marker placement is load-bearing), the 405 `Allow` header and
  the oversize-body sentence under the adapter, and the seven owner items inherited from the
  M6.4 close §6.3 (the vs-bot gate livelock first among them).

## 6. Process (R-P11 as refined 2026-09-20)

The owner ruled mid-cycle: **small mechanical tasks execute INLINE by the orchestrator with the
per-edit gates run after every task, and review is BROAD — one fresh-eyes Opus review at the
structural mid-point and one whole-branch review at the end — never per-task fix rounds over
minor edits.** Full subagent-driven development (Opus implementer + Opus reviewer per task) is
reserved for tasks that are large or judgment-heavy. Every agent writes its full report to a
file in the SDD workspace (`.superpowers/sdd/<plan-basename>/`) and replies with the path plus
three lines; a subagent runs long commands in the FOREGROUND; never follow up an implementer
without `git log -1` and `git status`; never rewrite a reviewed SHA. The corpus is regenerated
at every shape-changing task and its diff READ.

## 7. Facts the M6.5b session should not re-derive

- `wasm32-unknown-unknown` is installed on the pinned toolchain (`rustup target add … --toolchain
  1.97.1`); CI's `web-server` job installs 1.97.1 WITH the target and runs the check line.
- axum 0.8.9's rejection sentences: the JSON data/syntax pair, the missing-content-type
  sentence, the query sentence WITH its field path, and the path sentence naming `u64` — all
  reproduced in `api.rs` and pinned in lib.rs; a host that hands `dispatch` the raw bytes and
  the raw query gets them unchanged.
- `Games::ticking_clock` is the test default; `AppState` passes `now_unix`; the two stamp sites
  are `tables.rs`'s `create` and `chat_append`.
- The corpus is 83 files; `health.json` is the only host-parameterized payload today.
- The counts at the M6.5a close (one sequence at `346e7ef`; the close HEAD `42832f8` adds one test-only commit, re-run for the web crate): web **259** (247 lib + 2 gate + 10 wire) · CLI **851** + 2 ignored, untouched · vitest **640** in 38 · e2e **39** in 8 (11.1 min) · corpus **83** · the five fenced crates and the CLI byte-identical to `faf1a88` · `SEMANTICS_REV` 7. Its §2 is the table.
