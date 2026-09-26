# M6.5c "the objects" — the close

**Written 2026-09-24 to 2026-09-26 on branch `web-room-m6.5c` (stacked on `web-room-m6.5b2`; the
cycle's diff base `2621fbe`), the record of record for the cycle whose authority is the spec
`docs/plans/2026-09-22-web-frontend-m6.5c-objects-spec.md` (R-6.5c.1–16, at `50f705f`; read it
through its dated §10) and whose plan is `docs/plans/2026-09-23-web-frontend-m6.5c-plan.md` (sixteen
tasks). Executed in one session from `docs/plans/2026-09-23-web-frontend-m6.5c-execution-handover.md`,
SUBAGENT-DRIVEN for every task by the owner's ruling — a fresh Opus implementer and a fresh Opus
reviewer per task, the orchestrator (Fable) running every step that touches the account, and a
Fable whole-branch review with three Opus area reviewers beside it. The session stopped for two days
at the account's weekly limit (2026-09-24 ~10:58 UTC → 2026-09-26 ~06:04 UTC), with the exit
half-run, and every agent was woken by message when it reset, as the owner asked. The execution
ledger (the pre-flight scan, every ruling, every review, every deploy's readings) is the gitignored
`.superpowers/sdd/2026-09-23-web-frontend-m6.5c-plan/progress.md`; §5 below carries every ruling
with its cost, so nothing decided on the owner's behalf lives only there.**

**The headline.** Hosted multiplayer runs on Cloudflare at `yokohama.moore.nyc`, behind the same
`yokohama` Access application as the NAS. A stateless Worker serves the release as static assets
without being invoked, and runs only for `/api/*`, `/enter` and a missing file: it verifies Access's
assertion, removes every client `x-yokohama-*` header, resolves the caller and forwards by NAME to
one of three SQLite-backed Durable Object classes — a `Table` per table, ONE `House` for the lobby
and the house chat, an `Archive` per caller. Every object runs the SAME table layer the NAS runs, in
its own instance of a WebAssembly module (`crates/yokohama-worker` under a TypeScript shell,
`worker/`), so every payload is the layer's own: the whole committed corpus replayed byte-identical
through the objects in the test pool at every task from Task 12, and live, twice, through the
Worker. A request's writes and the alarm they make due go in ONE synchronous batch; the side
channels follow it — a hibernatable WebSocket carrying the SSE stream's own frames, a durable
ordered slot pushed to the house, and an R2 upload carried out as a receipt. Finished and abandoned
games land in the bucket `yokohama-archive` in the file store's own layout behind a clean off
switch; a table idle for 90 days is saved and expires as its creator's own Abandon behind a 90-day
marker that says why; and a person-only operator surface lists and ends tables. **The five fenced
crates, the CLI and `yokohama-wasm` are byte-identical to the base; `SEMANTICS_REV` is 7; no bot
behaviour moved; the layer changed by exactly the two items R-6.5c.4 names, and no third layer
change was ruled.**

**What is LIVE.** `yokohama-web` version **`6da906a5-a088-442e-bc43-d94904630982`** at
`yokohama.moore.nyc`, deployed 2026-09-26 at 06:38:27 UTC: the twentieth deployment, the fourteenth
being this cycle's first. It is the plain deploy that closed the exit's window, so
`YOKOHAMA_WEB_COUNT_WAKES` is no longer bound. **The code close head is `754b80ae`**; the code it
ships is the fix-wave follow-up `bd8ce497`'s, since the three commits after that (`f2c27a3f`,
`528b1285`, `754b80ae`) move only `web/e2e/` and the Playwright configuration. `deploy/RUNBOOK.md`
§6.8–§6.11 record deployments 14–17; the close's docs commit adds the records of deployments 18, 19
and 20, the live runs and the exit (drafted as §6.12–§6.14; §6.12–§6.14, in the docs commit that follows `754b80ae`). The NAS is untouched: `yokohama-web:0.5.0-m6.5b2.1` at
`yokohama-nas.moore.nyc` (RUNBOOK §3.16), `claimed`, SSE, the directory archive. The branch is **PR
#{{TBD: number}}** against `web-room-m6.5b2`, the eighth of an unmerged stack whose **merge order is
the owner's**.

---

## 1. What shipped (by task, in execution order; the rulings each rests on are in §5)

The order run was the plan's, 1 → 16. Forty-two execution commits took Tasks 1–15
(`f0140e03..d025ba43`), each task one green commit with its red run quoted and every later change a
NEW commit. Task 16 added seven more: the fix wave (`0919931c`, `9630edd4`, `e20c8ff8`), its follow-up
(`bd8ce497`), and three e2e-only commits for the exit spec and R-N7 (`f2c27a3f`, `528b1285`,
`754b80ae`, the code close head). No reported SHA was amended; two unreported ones were, before their
reports, which the rule allows. The branch as a whole, `2621fbe..754b80ae`, is 64 commits with the
spec's and the plan's; at `d025ba43` it was 124 files, +32,391/−617 (the whole-branch review
package's own stat), and the seven commits after it add 75 file-changes, +4,309/−778.

### 1.1 Task 1 — the layer's two changes, and the corpus's two request scripts (R-6.5c.4; spec §5.3)

Commits `ab9c6812` (the green commit — an amend of the unreviewed `c2daf4c6`, comment-only, verified
line by line) + `786d89f7` (the review's comment and doc round). **`Games::archive_over_retry(id)`**
in `games.rs` runs the existing `archive_if_over` against the stored record at the pinned reading,
so the game-over stamp and its retry on a version conflict are the layer's own code; at an open or
dealing table it answers `handle`'s own `Err` (Ruling S-01). **`tables::check_shape`** is `pub`, its
body byte-identical. The emitter writes `web/src/wire-script/walk-host.json` (761 B: three
`…/events` rows positioned by the walk row they follow, and ONE synthesized `GET /api/art` at the
walk's start) and `archive-walk.json` (500,600 B: the eight `archive_*` requests); the committed
corpus is byte-identical. `archive_payloads` posts the walk's table with its lock stripped, because
the lock's salt is OS entropy and a committed script would not be byte-stable (ruled; §5.4).
`cargo test -p yokohama-web` **289 → 293** (278 lib + 2 `browser_played_pin` + 2 gate + 11
`wire_fixtures`).

### 1.2 Task 2 — the module's core: `init`, `dispatch` with recorded bumps, the side channel, the clock (spec §2.2)

Commit `b7a682c8`. `crates/yokohama-worker` is a `cdylib`, `publish = false`, over `yokohama-web`
without its server half, with `debug = []` as its one feature and no axum or tokio in its tree.
`init` rebuilds every static the crate owns (`GAMES`, `STORE`, `SIGNAL`, later `PICKERS` and the
receipts) but never the clock pin (Ruling I-07 refined); `RecordingSignal` returns the layer's
bumps tagged (`bump`, `bump_table`, `bump_house_chat`, `end`) in the reply; `load`/`read` are the
side channel over `record | table | chat | house`; `set_clock` returns the prior setting;
`debug_trap` exists only under `debug`; the platform scan is `yokohama-wasm`'s. The owed-pump
fixture pair is committed once (its bot seed is entropy). **`cargo test -p yokohama-worker` 11**
(new).

### 1.3 Task 3 — the two routes this host answers itself: `create` and `events` (spec §2.2, §2.3)

Commits `61c1687a` (green) + `528369e3` (the caller-first ruling's comment; "nothing stored"
widened to every id) + `a48cf5f5`/`d375391f` (fix round 1). **`create`** checks the CALLER first — a
caller-less body is dispatched unchanged so the layer's own 401 answers — then lets the layer's own
`NewGame` parse decide: a body it refuses goes through unchanged; a parsed non-object is **400** `a
game must be created from a JSON object`; `"id": null` is **400** `a game id, when given, must be a
string`; a string id that is not the object's name is **500** `the create's id does not name the
table it was routed to`; an absent id is inserted TEXTUALLY before the object's final `}`, trailing
whitespace allowed (Review Focus 1), so nothing is re-serialized. **`events`** restates axum's
handler order and its 25-second cap. `from_the_layer` is the ONE drain path every export answers
through. 14 mutation probes caught; worker **24**.

### 1.4 Task 4 — the picker across hibernation (R-6.5c.7; spec §2.4)

Commits `4f6f37d7` + `5011bf99` (fix round 1). `picker.rs` keeps `PICKERS` as `Lines | Dropped`:
a 200 input that leaves the picker open and the RECORD's bytes unchanged appends `(user, line,
expect)`; only a change to the game clears the list (a chat post, an unlock or a watcher leaves it);
the 65th line drops it whole. `restore_picker` re-feeds the lines on a fresh instance behind a
SHA-256 guard on the record, reads a row only in its object form, and never panics on a bad row (the
picker is a convenience whose loss is a 409 on a stale click). Worker **34**; 31 of 33 mutants
caught (2 equivalent), then the fix round's six. The fix wave stamped each row with the
code that gives its lines their meaning (I-85, §1.16).

### 1.5 Task 5 — the house copy: the scrub, and the house's store (R-6.5c.8; spec §2.5)

Commits `163774c3` + `1c958fbc` + `f665035d` (two ruled follow-ups before the review). `house_copy`
scrubs the seed to 0, a lock to zeroed hex of the stored lengths and each keep to presence
(`Some(1)`/`None`); it runs `check_shape` on the stored ORIGINAL first (a document the NAS's
`Games::list` would drop is never a copy the house lists) and then on the copy, through a test seam
`house_copy_with(id, scrub)` that makes the copy-side check falsifiable. The scrub names every
`TableDoc` field, so a new field is a compile error. `put_copy`, `remove_copy` and `house_frame`
complete the house's store; Review Focus 4 (a name with an accent, an emoji and CJK) is pinned
natively. Worker **43**; 34 of 35 mutants caught (one equivalent).

### 1.6 Task 6 — the receipt archive, the stamp, and `expire` (R-6.5c.9, .14, .15; spec §2.6, §2.4 step 1)

Commits `17284a27` + `928387a4` (the ruled `over` follow-up) + `d2218e1c` (fix round 1).
`ReceiptArchive` implements the layer's `Archive`: a put matching a receipt for the EXACT plan (both
keys and both bodies' SHA-256) answers the receipt's outcome; otherwise it records the plan and
answers "deferred to the host". The `test/` root reads the full address — the table's `created_by`
for a server game, the Archive object's caller for a browser save — never the truncated segment.
`stamp_archive` answers `{"stamped", "over"}` (Ruling I3 refined), so the shell can tell an undone
game from a failed stamp; `expire` is the stored creator's own `DELETE`. **The module's Rust half is
DONE here: sixteen exports.** The owed-game-over fixture is cut at the smallest n, 588. Worker
**55**; mutants 34/34, 3/3 and 7/7.

### 1.7 Task 7 — `worker/`: the package, the module build and its factory, the assets, the pool, CI (spec §2.2, §2.3, §3.1, §5.6)

Commits `e1f54ec8` + `87caa769` (fix round 1). `worker/` is its own npm package (Ruling P-3):
wrangler 4.136.3, `@cloudflare/vitest-plugin` 1.2.3, vitest 4.1.11. `scripts/build-module.mjs`
builds the release module (`release-wasm`, wasm-bindgen 0.2.128, the factory, `wasm-opt`) and the
debug one; the factory transform makes each call its own `WebAssembly.Instance` and refuses a glue
whose shape changed; `assemble-assets.mjs` builds `worker/assets/` from `web/dist` and the art and
VERIFIES every precache entry's SHA-256 against the release's own `sw.js`; a refused step leaves
nothing the guard would pass, and an unoptimized build leaves `gen/.unoptimized`, which the guard
refuses by name. CI's `web-server` job gained the worker's steps. `test:scripts` **98** (5 files),
the pool **15**; the assembly 153 entries verified / 156 files / 9,966,268 B; 46 mutants caught.

### 1.8 Task 8 — the client: the WebSocket arm and the outbox's 404 (R-6.5c.6, .14; spec §2.7)

Commit `4e109268`. `web/src/ws.ts` reads `{event, data}` frames, drops everything else (the
edge's `pong` among it), sends `ping` every 30 s, ends the source on an `end` frame and ignores the
close that follows it. `signal.ts`'s third arm probes ONCE, through the source's own `once()`, after
a close it did not make or an open that failed — never after its own (a hidden tab's, a teardown's)
— and climbs the existing ladder; an expired Access session ends the source with the `/enter`
remedy. The outbox reads the archive route's 404 as "no archive" (`rememberArchiveOff`) and forgets
the row. The corpus is untouched. `npm test` **1024/55 → 1044/56**; e2e **61/11** — every cell
passed at this tree, `two_browsers` twice hitting the inherited `CAP = 4000` before passing solo in
4.6 min (2,669 moves); 36 mutants caught.

### 1.9 Task 9 — the table object: storage, the wake, the request path, the traps (spec §2.4)

Commits `0ff17088` + `683f36c6` (fix round 1). `docs` and `meta` are created by the first write
batch, never by the constructor, so a probe leaves `sqlite_master` empty. `fetch` reads the body
last and then runs synchronously: the poison check, the configuration, the module (built lazily),
the wake (each stored document loaded, the picker restored ONCE), the call, and ONE
`transactionSync` batch with any alarm set beside it in the same turn. A trap costs one instance and
persists nothing; two consecutive LOAD traps name the poison (500 `this table's stored <kind> cannot
be loaded`); a successful load resets the count even when the call that follows traps (the spec
over the brief, §5.4). Node **103**, pool **37**; 54 mutants caught. **Measured:** the pool CAN
deliver a nameless alarm (an object reached only through `idFromString`, or named over 1,024 bytes),
which narrows the plan's Pool fact 8 (spec §10).

### 1.10 Task 10 — the table's sockets and transport routes (R-6.5c.6; spec §2.4)

Commits `b6f843c5` + `87d30a14` (fix round 1). `frames.ts` wraps the SSE's own data raw as
`{"event","data"}`; a reply whose bumps name the table sends ONE frame per socket, computed for
that socket's own viewer; a 403/404 is `end {"reason":"gone"}` and a 4403/4404 close whose reason is
the layer's sentence cut at 123 UTF-8 bytes; at ADMISSION the `end` also carries the sentence
(ruled; §5.4); a delete sends `end {"reason":"deleted"}` and a 4404 close `this table was deleted`;
`ping → pong` is the edge's auto-response, and no client message reaches `dispatch`. Sockets are
tagged with the table id (the spec over the plan). Node **110**, pool **58**; 62 of 63 mutants
caught (the survivor equivalent: the pool silently ignores a missing `webSocketMessage`).

### 1.11 Task 11 — the Worker, the first cuts of the house and the archive object, and THE FIRST DEPLOY (R-6.5c.1, .3, .10; spec §2.3, §3.2, §3.3)

Commits `a8e6c8a5` (green) + `756dc759` (fix round 1 — **deployed** as the fourteenth) +
`60346fe4`/`f64310c7` (the deployment's record). `index.ts`'s `handle` runs spec §2.3's order: the
route; the configuration (a bad variable is a 500 naming it on every `/api/*`); the body read once
and bounded at 2 MiB with the NAS's own sentence, read off the NAS live; `/api/art`; identity (the
Access assertion verified RS256 against the team's keys, `aud`, `iss` and expiry, one key refresh
per unknown `kid`); the raw-address rule (an address outside tab, space and visible ASCII is no one,
judged on the RAW claim before any trim, and lowercased by ASCII only — the Kelvin sign read as
another user's `k` was the review's first Important); the strip rule (every client `x-yokohama-*` header);
forward by NAME. A WebSocket upgrade whose `Origin` is present and foreign is refused 403 `this
socket is for the site's own pages` (ruled; the owner may reverse it). Names over 64 bytes go to the
house, so no request can create a nameless object. `smoke-release.mjs` instantiates the release
module once before any deploy; the deploy guard compares the two configurations' object blocks.
`deploy/worker/` and `deploy/modal_app.py` are deleted and RUNBOOK §4 is the record of why; RUNBOOK
§6 is new. Node **227**, pool **98**; 104 of 105 mutants caught (the survivor equivalent).

### 1.12 Task 12 — the house and the slot, and the corpus through the objects (R-6.5c.8; spec §2.5, §5.3)

Commits `b163e64a` (green) + `ebe16d32` (the copy module's cost, documented) + `b565fd2a`/`b24149b5`
(fix round 1 — **deployed** as the fifteenth) + `e06abfd4`/`953abd64` (the deployment's record).
`slot.ts` orders generations by `(at, rand)` and decides a push (the current generation at a higher
revision; else a generation not retired that orders after every retired one); the slot and its
alarm go in the request's batch when the scrubbed copy DIFFERS from the one last put in motion, and
the push follows the reply; an acknowledgement clears the slot only if it still holds that `(g, r)`
and records `acked` only if it advances. The house applies with no `await` between its decision and
its batch, and frames the lobby socket `{seq, chat}`. The copy is computed on a scratch module
instance of its own (18 pages, 1.125 MiB, ~0.15 ms), so a trap never empties the module the frames
read; an owed copy (`copy_owed`) is retried on the ladder and by every request. The house drops its
chat document at its second consecutive load trap rather than stay 500. `/api/art` serves the
manifest projected onto the nine maps the NAS's typed route carries (the raw file had two generator
keys more — a payload that differed on this host, found and ruled host-local). **From here the
pool replays the whole corpus through the objects**: 2,201 rows, 78 named rows equal, the art row
unexempted. Node **248**, pool **121**; 43 mutants caught plus 2 evidence readings.

### 1.13 Task 13 — the archive on R2: the receipt loop, game over, the switch, the bucket (R-6.5c.9, .14; spec §2.6)

Commits `ad476e8f` (green) + `4d984f52` (fix round 1 — **deployed** as the sixteenth, the bucket
bound) + `435e2bfd` (the record). `r2.ts`'s `executePlan` runs `DirArchive::put`'s own order — read
the record key (the same bytes `identical`, others `conflict`), put the table unconditionally, put
the record create-if-absent with `onlyIf: new Headers({"If-None-Match": "*"})` and re-read on a
refusal — and every outcome is logged. Abandon and the browser's save run the receipt loop under
`blockConcurrencyWhile` at ONE pinned reading; game over — the one site that cannot be
re-dispatched — owes its plan in that request's batch, uploads after the reply, and stamps through
`stamp_archive` at the RECORDED reading. The switch's `off` keeps an owed save and re-arms nothing
for it; an abandon while off drops it and logs `archive-owed-dropped`. The pool's replay now serves
the eight `archive_*`: **2,209 rows, 86 named, a third column of 8**. Node **264**, pool **141**; 38
of 44 mutants caught — M30 equivalent, M33–M36 delay-only (ruled), M43 unreachable.

### 1.14 Task 14 — expiry, the marker, and the operator surface (R-6.5c.15, .16; spec §2.4, §2.9)

Commits `24e53754` (green) + `c2e8d084` (fix round 1 — **deployed** as the seventeenth, with TTL 90
and the operator secret) + `959df82e` (the record). One receipt loop, `deleting`, serves the
creator's Abandon, the alarm's expiry and the operator's plain expire; the ending's ONE batch writes
the marker, the lobby REMOVAL, the alarm, and clears the documents, the picker, `archive_owed`,
`held` and `expiry_failed`. The marker answers every request 404 with a head (abandoned, expired
after N days, removed by an operator — dated UTC) and a tail by `kept` for 90 days, an upgrade
included and unupgraded; a create under a marker or an unacknowledged removal is 409. `kept` is
derived without parsing the table document (Rulings I-64, I-65, I-66, I-71). The operator gate
counts only a PERSON on `YOKOHAMA_WEB_ADMINS`; `?force` is the bare key (I-72), ends the object
without the layer and without a save, and leaves no marker. The deploy guard's seventh check keeps
the operator list out of the vars. Node **299/17**, pool **191/9**; mutants 72/72 and 26/26.

### 1.15 Task 15 — the live tiers: Playwright against the Worker, and the live corpus replay (R-6.5c.12; spec §5.3, §5.4)

Commits `0ac38d34` (green) + `b1e749db` (the rulings' reasons beside their code) + `b5eca77f` (fix
round 1: a live call's log withheld) + `d025ba43` (fix round 2: the first live run's three findings
— **deployed** as the eighteenth). `web/e2e/live.ts` builds each live context from Access's own
`CF_Authorization` cookie, fetched once by Node with the credential so Playwright never holds the
secret, plus the claimed identity as a context header, the one header a page-opened WebSocket
upgrade carries (Ruling I-75); every live `request.*` goes through `liveRequest`, which rethrows the
first line only, because Playwright's call log prints the cookie (I-79); tracing is off under LIVE;
cleanup is `afterEach` (I-77). `harness.ts` forgives EXACTLY two known console shapes: the beacon's
CORS pair — the claimed header on the cross-origin beacon request is the harness artifact Task 11
diagnosed — and the deal's designed 409. `worker/scripts/live-replay.mjs` forces the replay's
three ids before and after, reads the house chat's base, replays all three scripts through the
Worker and judges every named row with the named exemptions, counted as spent. Fix round 2 widened
the cycle's `web/` allow-list by ruling (I-81) to `signal.ts`, `api.ts` and `ws.ts`: no stream
opens before the transport is known, and the keep-alive stops at hang-up and sends only while OPEN.
`test:scripts` **328/18**; `test:wasm` **66/5** (34 of them the live tier's node cells);
`npm test` **1048/56**; local e2e **61/61**.

### 1.16 Task 16 — the whole-branch review, the fix wave, the battery, the exit and this close

The review, its rulings and its re-reviews are §4; its rulings with their costs are §5.5; the exit's
readings are §3.3. What landed:

- **`0919931c` — fix wave A** (the Rust crate, `worker/src`, `worker/scripts`, the RUNBOOK and the
  workflow; 32 files, +1,637/−333). The picker row's CODE IDENTITY (I-85): the worker crate's
  `build.rs` digests the sources that give a line its meaning, the row carries it, and
  `restore_picker` clears a row built under other code with nothing applied, logged `picker-dropped`.
  A rejecting object answers the API's 500 `the server could not answer — try again`, logged
  `object-failed` (F-1). A generation's time is `Date.now()` in MILLISECONDS (F-2; measured 5–12 ms
  apart in 20 create → force → re-create sequences). A token with no string `kid` is no one (F-3).
  The `wake {id, wakes}` line (I-84). A browser save whose path names another caller is refused
  (R1's L-1); a dev-door claim that is not an address is no one (L-2, a differential of 60,018
  strings against the layer's `User::parse`, 0 disagreements); `init` refuses anything but an object
  (L-3); and the crate's docs sweep (L-4…L-9; `cargo doc` 4 → 0 warnings). The deploy guard gains
  two FRESHNESS checks, 8 and 9 (R3's M-3); the replay prints its reset recipe and stops on a first
  send's 409 (M-1); the operator list's refusals name an entry by position (M-6); RUNBOOK §6.7
  becomes "The live tiers and the quota guard", §6.4 and §6.8's reading 6 are corrected forward,
  §6.5 gains six log lines, and §1 and the workflow state `wasm-opt` 133 against CI's 132 (I-87);
  R2's five hygiene LOWs. Worker cargo **55 → 59**; `test:scripts` **328 → 351**; the pool **191 →
  194**; the release module **1,220,664 → 1,221,719 B** (the code stamp grew it); new-cell mutants
  27/27. Not done: Task 1's M6 cell, whose home is the fenced `games.rs`.
- **`9630edd4` — wave A's second commit** (I-89): the pool's `picker-dropped` cell over both stamps
  — another code, and none (every row deployments 14–18 wrote, logged `stored: null`); and RUNBOOK
  §6.6's two rollback bullets — a rollback to the eighteenth deployment or earlier is a
  GENERATION-ORDER rollback, and it drops every open picker the newer code stored. The pool **195**.
- **`e20c8ff8` — fix wave B** (`web/src`, `web/e2e`; 15 files, +1,280/−160). The transport wait
  BOUNDED by `BOUND_MS`, the word "reconnecting…" while it runs (I-86); the socket arm's no-poll cell
  (M-2); the three jsdom flakes, each reproduced first and fixed by its cheapest fix; the structural
  cell `web/e2e/live.node.test.ts`, which refuses a live `request.*` outside `liveRequest` and any
  `toBeOK()` (Task 15's R-1); and the opt-in exit spec `web/e2e/exit_game.spec.ts` (I-88).
  `npm test` **1048 → 1056** in 56, green in order twice, shuffled under seeds 22 and 44, and under
  three concurrent suites; `test:wasm` **66 → 72** in 6; `npm run e2e` **61/61** in 6.5 min;
  mutants 8/8. **The fix wave's head.**
- **`bd8ce497` — the follow-up** (I-91; 16 files, +618/−118): the engine's `crates/yokohama-core/src`
  a root of the picker's code (R1's R-1: a line's NUMBER is the engine's legal-list order); two
  config sentences (N-1); `picker-dropped` logged after the request's batch (N-2);
  `yokohama-bridge` in the freshness digest, with a `cargo metadata` cell that walks the module's
  path dependencies (R3's R-N1); any alias of a live context refused (R-N2); the RUNBOOK's
  exit-window steps and the quota query's `date` form (R-N4, R-N5); the exit spec's visibility
  precondition (R-N6); and R2's residual — two held cells settled in `finally`. `test:scripts`
  **352**, `test:wasm` **80**; mutants 10/10. **The shipped code's head: deployment 19 was built from
  it.**
- **`f2c27a3f` — the exit's windows** (`web/e2e` and the Playwright config only): after the exit's
  first run met a covered, hidden window (§3.3), the spec places its three headed windows SIDE BY
  SIDE through the browser protocol — `EXIT_SCREEN=<w>x<h>`, default 1280×832, three 420×820
  columns at 0/426/852 with a 404×730 viewport. `test:wasm` **85** in 6.
- **`528b1285` — R-N7**: a request context MADE from an API context escaped the structural rule, so
  it is refused inside `liveRequest` or out; `browser.newContext()` stays allowed. `test:wasm` **86**.
- **`754b80ae` — the exit's grid, the code close head**: after the exit's second run met Chromium's
  500 px minimum window width, the spec lays a 2×2 GRID when three minimum-width windows cannot share
  the width, under the menu-bar inset read back from the first placement; it refuses any overlap or
  a window off the screen, forgets the seating's transient hidden reports, and checks visibility
  from the start of play. `test:wasm` **92** in 6.
- **The deployments of the exit**: the nineteenth (the window open, `204aba6b…`) and the twentieth
  (the window closed, `6da906a5…`), §3.1; the exit's three runs, the two-browser
  re-run and the wake witness, §3.2 and §3.3.
- **This close's docs commit** the docs commit that follows `754b80ae`: this file, the spec's §10,
  `CLAUDE.md`, the board, the RUNBOOK's records of deployments 18–20, the live runs and the exit, and
  the M6.5c2 handover.

### 1.17 The fences held, and were shown to hold

Read by R1 at the follow-up `bd8ce497` (its re-check, on an export), the head of every file a
deployment ships:

```text
$ git diff --stat 2621fbe -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli crates/yokohama-wasm
(empty)

$ git diff --stat 2621fbe -- crates/yokohama-web/src
 crates/yokohama-web/src/games.rs  | 135 ++++++++++++++++++++++++++++++++++++++
 crates/yokohama-web/src/tables.rs |  25 ++++---
 2 files changed, 152 insertions(+), 8 deletions(-)

$ git diff --stat 2621fbe -- web/src/wire
(empty)

$ git diff --stat 2621fbe -- web/src/wire-script
 web/src/wire-script/archive-walk.json | 98 +++++++++++++++++++++++++++++++++++
 web/src/wire-script/walk-host.json    | 42 +++++++++++++++
 2 files changed, 140 insertions(+)

crates/yokohama-core/src/game/semantics.rs:180: pub const SEMANTICS_REV: u32 = 7;
```

The two layer items are exactly two: `archive_over_retry` with its three cells and a doc paragraph,
called by nothing on the NAS; and `check_shape` made `pub`, its body byte-identical to the base. The
CLI's goldens and `guide_sync` were not a per-edit gate this cycle (ruled: the CLI is fenced
byte-identical), so the empty fence is what establishes the CLI's parity. The three e2e-only commits after
`bd8ce497` touch no fenced path, so these readings hold at `754b80ae` by construction; re-read at `754b80ae` in the final sequence (§2): empty / the two items / empty / the two scripts / 7.

---

## 2. The counts at the close HEAD, in ONE sequence

Every command below ran at the code close head `754b80ae` on the Mac workstation (Darwin 27.0.0, the pinned
1.97.1 toolchain, `wasm-opt` 133) as ONE uninterrupted script on 2026-09-26 from 06:41 UTC — the per-edit gates,
the browser walk, then the fifteen-gate release roster, then the release state restored. The wall is each
command's own; the script's log per row is the evidence.

| command | reading at `754b80ae` | wall |
|---|---|---|
| `cargo test -p yokohama-core` (debug; the twenty-one binaries + the doc-test) | 1577 passed + 29 ignored | 224 s |
| `cargo test -p yokohama-cli` | 851 passed + 2 ignored | 46 s |
| `cargo test -p yokohama-oracle` | 191 passed | 6 s |
| `cargo test -p yokohama-difftest` (debug) | 52 passed + 5 ignored | 20 s |
| `cargo test -p yokohama-core --example eval_probe` | 214 passed | 54 s |
| `cargo test -p yokohama-core --example solver` | 132 passed | 3 s |
| `cargo test -p yokohama-bridge` | 81 passed | 9 s |
| `cargo test -p yokohama-web --lib --test wire_fixtures --test browser_played_pin` | 291 passed | 85 s |
| `cargo test -p yokohama-web --test whole_game` (the gate binary alone, under a 900 s kill) | (not reached) | — |
| `cargo test -p yokohama-wasm` | 5 passed | 7 s |
| `cargo test -p yokohama-worker` | 59 passed | 7 s |
| `cargo test -p yokohama-web --no-default-features --features server --lib identity::` | 5 passed | 16 s |
| `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown` | ok | 4 s |
| `cargo clippy --workspace --all-targets -- -D warnings` | ok | 9 s |
| the four bridge node suites | bridge node suites: 4 passed | 11 s |
| `cd worker && npm run build:module:debug` | ok | 8 s |
| `cd worker && npm run typecheck` | rc=1 in the sequence — `EPERM` opening a `node_modules` file, a macOS transient; re-run alone a minute later: exit 0 | 3 s |
| `cd worker && npm run test:scripts` | vitest 352 passed in 18 files | 8 s |
| `cd worker && npm test` (the pool; the corpus replay inside it) | vitest 195 passed in 9 files | 16 s |
| `cd web && npm run typecheck` (four projects) | typecheck passed | 20 s |
| `cd web && npm test` | vitest 1056 passed in 56 files | 27 s |
| `cd web && npm run test:wasm` | vitest 92 passed in 6 files | 3 s |
| `cd web && npm run e2e` (the local browser walk) | **60 passed of 61** in 6.6 min; the one red, `plays_locally` "a quota failure is APPLIED BUT NOT SAVED" (its `.deck .warn` not found within 10 s under the sequence's load), passed ALONE twice right after (1.8 s, 1.3 s) — a timing flake, recorded for the cleanup cycle beside the Surfaces cell's | 404 s |
| release gate 1 — `fuzz_full_games` | 1 passed | 53 s |
| release gate 2 — `choice_encoding` | 1 passed | 3 s |
| release gate 3 — `observation_honesty` | 2 passed | 4 s |
| release gate 4 — `action_source_equivalence` | 2 passed | 29 s |
| release gate 5 — `action_source_worstcase` | 3 passed | 2 s |
| release gate 6 — `arena_gate` (single-threaded) | 8 passed | 23 s |
| release gate 7 — `policy_diversity` | 4 passed | 7 s |
| release gate 8 — `evaluator_gate` (single-threaded) | 5 passed | 272 s |
| release gate 9 — `architecture_gate` (single-threaded) | 3 passed | 55 s |
| release gate 10 — the core lib's one `#[ignore]` (recovery falsifier) | 1 passed | 30 s |
| release gate 11 — `piped_dogfood` | 2 passed | 12 s |
| release gate 12 — `differential` | 1 passed | 31 s |
| release gate 13 — `routes3_differential` | 1 passed | 5 s |
| release gate 14 — `scoring_differential` | 1 passed | 3 s |
| release gate 15 — `routes3_worstcase` | 2 passed | 4 s |
| `cd web && npm run build` (the release `dist` restored after the e2e; `debug_trap` 0) | rc=1 in the sequence — `EPERM` on the build script's overwrite of the generated `wasm-node/package.json` (a provenance-marked file; the same class as the typecheck's transient); the file removed and the release `dist` rebuilt in the foreground right after, its precache name `yokohama-release-3d7b48a6d2d78e74` the deployed one's, `debug_trap` 0 | 6 s |
| `cd worker && npm run build && node scripts/deploy-guard.mjs` | rc=1 in the sequence — refused BY DESIGN: `web/dist` still held the e2e's DEBUG module because the row before it had failed; re-run in the foreground after the web rebuild: the module 1,221,719 B, 153 assets verified, smoke 200, the guard's nine checks exit 0 | 12 s |

**The fifteen-gate roster's wall in this sequence: 533 s** (8.9 min; the target ≤ 420 s is an M5-era figure, machine-tagged: this is the Mac workstation's reading, recorded beside the earlier ones, never against them). TOTAL wall so far: 1541s (25.7 min) over 40 commands.

---

## 3. The deployments, the live tiers, the exit, and what a game costs

### 3.1 Deployments 14–20

Every deploy was the orchestrator's, from a reviewed and clean tree, `npm run build` in `web/` and
`worker/` first, the deploy guard passing, `wrangler tail` listening, and the credential presented
through a header file of mode 600 that was deleted afterwards. **No reading that differed from the
spike in substance was found.** RUNBOOK §6.8–§6.11 hold deployments 14–17; the close's docs commit
writes 18–20.

| # | when (UTC) | from | version | what it brought | the readings that matter |
|---|---|---|---|---|---|
| **14** | 2026-09-23 | `756dc759` (Task 11) | `8b1bdcf8-610d-4b04-9e21-727e216cd61a` | the Worker; `Table`, `House`, `Archive` in migration `v1` (frozen from here); the static release; identity; the body bound; the Origin check; archive `off` | identity on an upgrade: a person over WARP **101** (the production key source verified a real assertion), the credential with a claimed `.test` user **101**, the credential alone **401** in the layer's `worker`-mode words; `/api/health` `cf-access`/`ws`/`off`; static assets **152 byte-exact + `/index.html` rewritten by the edge**, the service worker installed whole (154 entries for 153 listed + the stamp), **zero** Worker invocations for a real file and **one** for a missing one; the Worker's CPU WITH JWT verification **0–2 ms** (1 ms typical); startup **2 ms**; upload 1260.83 KiB / gzip 484.78 KiB; the Origin check live (same origin 101, foreign 403 unupgraded, a plain GET with a foreign Origin 200) |
| **15** | 2026-09-23/24 (the re-created generation's `at` reads 23:42) | `b24149b5` (Task 12) | `e355c676-cfb2-400b-926d-05def2e725fd` | the house over scrubbed copies; the slot; `/api/art` projected | a LOCKED create with a seed above 2^53 → **201**; both listings carry no seed digits, no hex run ≥ 32, no `salt`/`hash`/`seed` key; a lobby socket received exactly `{"event":"house","data":{"seq":2,"chat":0}}` on a join; delete → re-create at the same id → **201** with a NEW generation; `/api/art`'s nine maps in the struct's order; the tail's `slot-pushed`/`slot-acked` pairs; no `copy-refused`, no trap, no alarm fired (by design); spike row 3 SAME (the lock's salt drawn by the module's `getrandom` inside the object) |
| **16** | 2026-09-24 (its readings ~01:55) | `4d984f52` (Task 13) | `a8e887b5-6212-47ce-a493-abbd40b53cd6` | the bucket `yokohama-archive` bound as `ARCHIVE`; `YOKOHAMA_WEB_ARCHIVE` `r2` | `/api/health` `archive: "r2"`; a locked `.test` game of eight moves, abandoned by its creator → **204**, the pair under `test/rev7/abandoned/` (the record 1,301 B; the table `lock: null`, `unlocked: []`, `archived_at` set), and the record LOADS in the CLI with `log` listing the eight moves; **the conditional put's `Headers` form ACCEPTED by production R2** (`archive-landed`); 40 `ok` outcomes; wrangler's OAuth login accepted the R2 binding |
| **17** | 2026-09-24 ~04:28 | `c2e8d084` (Task 14) | code `eb764c8b-69c6-475c-a30e-8815ff0e690f`; the secret put made **`0b7575a8-7b56-4c53-8619-a82b52b3a63b`**, the one live | `YOKOHAMA_WEB_TABLE_TTL_DAYS` `90`; the secret `YOKOHAMA_WEB_ADMINS` (Ruling P-1); expiry, the marker, the operator surface | the operator listing held one row, the reading's own (no stale table from an earlier deployment existed); four forces, each **204**; an abandoned table's marker sentence **404** and a create under it **409**; the credential claiming the operator's address **403** `this address is not an operator here` (battery 32, live); `?force=false` **400** reaching no object; `?x=1` the layer's **404**; a force on an object holding nothing **204** with no `forced` line; 172 asset files, none re-uploaded; startup 1 ms; 43 tail events |
| **18** | 2026-09-24 ~07:37 | `d025ba43` (Task 15 fix round 2) | **`93f73009-0e3e-4c21-a73f-e164ee5d58a5`** (every event of its tail carries it) | the client's transport wait and keep-alive guard; no crate moved | web precache 153 entries / 9,938,696 B, cache `yokohama-release-b4f28a3400c79f9c`, `debug_trap` 0; the module 1,220,664 B, unchanged; the assembly 153 verified / 156 files / 9,968,109 B; the guard's seven checks; 3 assets uploaded (the client bundle among them), 151 already there; health `cf-access`/`ws`/`r2`; then live runs 2a and 2b (§3.2). **Its tail over those runs:** 9,292 events — `ok` 9,274, `canceled` 9, `responseStreamDisconnected` 4, `unknown` 5, exceptions 0; `operator` ×10, `forced` ×6, `slot-pushed` 394 / `slot-acked` 394 / `slot-ignored` 0, `archive-landed` 3 / `archive-identical` 5 / `archive-conflict` 2 (the replay's own 409 rows) / `archive-failed` 0; `trap`, `load-trap`, `held`, `expired`, `init-refused`, `frame-failed` and `identity-failed` all 0 |
| **19** | 2026-09-24 10:49:45 | `bd8ce497` (the follow-up) | **`204aba6b-2794-4e95-8567-7558cf7c44b2`** (deployment `f41c4d25…`) | the fix wave (F-1, F-2, F-3, I-84, I-85's code stamp, I-86's bounded wait, R1's L-1…L-3); **the exit's window OPEN**: `npx wrangler deploy --var YOKOHAMA_WEB_COUNT_WAKES:on`, after the guard | web precache 153 entries, cache `yokohama-release-3d7b48a6d2d78e74` (9,939,104 B at R2's rebuild of the same release), `debug_trap` 0; the module **1,221,719 B** (`wasm-opt` 133); the assembly 153 verified; the guard's **nine** checks passed, the two freshness checks among them — "built from the sources as they are" and "assembled from the current web/dist and worker/static"; 3 assets uploaded (the client bundle `index-DqiipaaU.js` among them), 151 already there; startup 1 ms; the bindings list shows `env.YOKOHAMA_WEB_COUNT_WAKES ("(hidden)")` — wrangler hides a `--var` value; health 200 `cf-access`/`ws`/`r2`. **Its readings across two days** (§3.2, §3.3): on 2026-09-24 the exit's run 1 (the seating live, then the visibility precondition), the headed walk and the bucket with `why`; on 2026-09-26 the exit's run 2 (the minimum-width overlap), **the `two_browsers` re-run PASSED** (2,510 moves, every console clean but the beacon's), **the exit's run 3 PASSED**, and **the wake witness** — four `wake` lines, 6–9 ms of CPU each, off `tail19b.json` (41,578 events on this version, 0 exceptions, `trap` 0). By design (I-89), every picker row an earlier deployment stored is dropped at its table's next request under this code, logged `picker-dropped` with `stored: null`; the ledger records no such line |
| **20** | 2026-09-26 06:38:27 | `754b80ae` (the code close head; `bd8ce497`'s shipped code) | **`6da906a5-a088-442e-bc43-d94904630982`** | **the exit's window CLOSED**: the plain `npm run deploy`, whose variables name no `YOKOHAMA_WEB_COUNT_WAKES`; the owner's go came after the WARP login was re-authorized | web and worker rebuilt for release, `debug_trap` 0; the module 1,221,719 B (`wasm-opt` 133), unchanged; 153 assets verified; the guard's nine checks; NO asset changed since the nineteenth; startup 2 ms; `YOKOHAMA_WEB_COUNT_WAKES` no longer among the bindings; health 200 `cf-access`/`ws`/`r2`; the tail `tail19b.json` stopped |

**Two readings at the fourteenth that differ from the spike in FORM, both diagnosed by
measurement:**
- **The beacon's CORS error was the spike harness's own.** It sent the credential as CONTEXT-WIDE
  headers, and custom headers on the cross-origin analytics beacon force a CORS preflight the beacon
  host does not answer. A real browser — over WARP, or with the Access cookie — sends no custom
  header and logs nothing; a harness that scopes the credential to `yokohama.moore.nyc` logs nothing
  either (readings 6a and 6b: console `[]`, the beacon 200, `/cdn-cgi/rum` 204). The zone stays as
  the owner keeps it — Web Analytics, Bot Fight Mode and its JavaScript detections ON.
- **The unenrolled vantage met the zone's challenge before Access.** From a throwaway `alpine`
  container on the NAS, curl's default user agent read **403** with `cf-mitigated: challenge` and a
  5,259-byte "Just a moment…" page on `/`, `/api/health` and `/enter`; with a browser's user agent
  and `Accept: text/html` the same paths read **302 → the Access login** (its JWT `auth_status
  NONE`); `workers.dev` **404**; an upgrade with no credential **403**. The NAS host read the same
  from the same vantage, so the shape is the zone's Bot Fight Mode, not this Worker's. Nothing was
  served. **After the two-day pause the workstation met it too**: the WARP client read Connected,
  but its device's Access login had EXPIRED, so every request without the credential answered
  Access's 302 "Please authenticate via the warp client", a curl-shaped one meeting Bot Fight Mode's
  challenge first. The owner re-authorized it on the 26th. Only the operator's readings needed it; the
  exit, the re-run, the deploys and the bucket's sync did not (the ledger's WARP entry).

**Later deployments re-read the rest of the spike:** row 3 (`getrandom` inside an object) at the
fifteenth; the conditional put's form at the sixteenth. Row 4 (a trap live) was never provoked: every
tail read `trap` 0. Row 5, an object's CPU, was read at the exit: **2 ms a move** (the median over
the exit game's 40 move POSTs; the maximum 15 ms) and **6–9 ms a wake**, the stateless Worker's own
half 2 ms. Row 6, hibernation, SAME in substance: the spike saw a wake after 35 s idle, and the exit
saw one after each of four 60 s pauses. **No alarm fired at deployments 14–17**, by design — every
push was acknowledged, and every acknowledgement deletes or re-arms the alarm — and neither the
reading of deployment 18's tail nor that of the exit's names an alarm line; so `ctx.id.name` inside a
production `alarm()` (the plan's Pool fact 8) is still unread live.

### 3.2 The live tiers — the ending state (the owner's direction, 2026-09-24 ~07:00 UTC)

The owner asked that the report state, if the full live test was not re-run, "the consequences and
still be comfortable through either the headed test or otherwise our ending state". The full live
test WAS re-run, on 2026-09-26: this is what ran, and which claim rests on which evidence.

**What ran.**
- **Live run 1** (2026-09-24 06:32–06:45 UTC, the specs at `b5eca77f` against deployment 17's
  `0b7575a8`). Both specs' SUBSTANCE passed and both FAILED their final "no console errors"
  assertion, on three errors that were not the beacon (whose two-line CORS artifact was forgiven 2, 2
  and 2 times per page, as I-76 predicted):
  - `two_browsers`: game `sim-552c786c4ba671ff` **finished — 2,862 moves (2,180 applied events) in
    732 s** (12.2 min against the doubled 40-minute budget; the game ended ~06:44), 0 idle looks, the
    largest hand seen 3. **The watcher held through 2,180 applied moves, asked `/events` ZERO times
    and opened `/stream` ONCE at every one of ~110 checkpoints — the socket never reconnected**; each
    player opened the table's stream twice, both before the first move. It failed on two console
    lines `WebSocket is already in CLOSING or CLOSED state.`
  - `joins_by_link`: table `sim-367fcf21183e035d`, 6 moves across two browsers in 4 s; it failed on a
    **426** from `/api/stream` and a **409** from `…/deal`, each with its console line. **Its 390 px
    readings, printed live:** the who-strip's address 17 px (desktop 17), no sign-out control under
    `cf-access`, the strip 71 px tall (desktop 22) with `flex-wrap: wrap`, the name field 144 px; the
    open table's page overflow **0 px** (gated), its link input 196 px, the copy control's right
    edge 76; the chat row 321 px = label 179 + clock 129 + text 314 (98% of the row, against a 60%
    floor); and at 1280 px the side column's chat row 322 px = 179 + 130 + 315.
  - No "Call log" in the output (the H-1 fix held), no cookie string in the error contexts, and the
    `afterEach` cleanups ran: the operator listing read `[]` afterwards.
  - **Server side** (the Workers Observability query API; the run had no tail): 77,697 `ok`
    invocations, 34 `canceled` — the client's own aborted fetches as pages left — and four
    `responseStreamDisconnected` on the game's stream at 06:32:27, **at the game's START**: the
    players' FIRST sockets, opened at the open table and replaced when it flipped to playing (the
    summary's "a 2, b 2 before the first move"). No `exception` outcome and no `$metadata.error`
    event.
- **The diagnosis** (I-81): the 426 was a CLIENT defect — the hook chose its arm from a transport
  fact whose default is `sse` until `/api/health` answers, so the lobby's house channel was fetched
  as SSE before the facts landed (invisible on the NAS, whose transport IS SSE); the `CLOSING` line
  was a CLIENT defect — the keep-alive interval was cleared only in the `close` handler, so a tick
  between `close()` and the `close` event sent into a closing socket; the 409 was NOT a defect — the
  dealing screen's designed answer for a table that flipped to playing between two requests, which
  Chromium logs as a console error and which the local run misses only because the race resolves the
  other way on the NAS. Fix round 2 (`d025ba43`) fixed the first two with red-first jsdom cells and
  forgives the third exactly — both arms, both modes, counted apart as `dealAnswers`.
- **Live run 2a — `joins_by_link` on deployment 18: PASSED** (1 passed, 4.3 s). Table
  `sim-55b67d78eca70411`, 6 moves in 4 s; the deal's 409 forgiven a 2 / b 0; the beacon's pair a 2 /
  b 2; **no 426 on `/api/stream`** — the transport wait held live; the 390 px readings as run 1's,
  but the who-strip 47 px tall.
- **Live run 2b — the corpus replay, TWICE, on deployment 18: BOTH HOLD**, exit 0. Run 1: the forces
  before `wire2 204, wire 204, wire4 204`; the house chat's base `last 0`; **2,209 of 2,209 rows
  run, 86 named rows compared, 13 exemptions spent** (the lobby filter ×4 with 0 foreign rows
  dropped; five host fields — `health.identity` `claimed → cf-access`, `signal` `sse → ws`,
  `archive` `off → r2`, `me.mode` and `me_signed_out.mode` `claimed → cf-access`; the house chat
  ×2; the two first sends "201, the first send landed"); 0 lobby re-reads of the bound (5 per row,
  500 ms apart); **mismatches none**; the forces after all 204. Run 2: the same, from base `last 1`
  (run 1's own post), the house chat taken back by 1 with 1 other message dropped, and **both first
  sends "200, the once-rule's `identical: true`"** — the repeatability the plan's re-review called
  its HIGH. **The whole corpus is byte-identical live, twice.** Deployment 18's tail over runs 2a and
  2b (§3.1) reads no exception, no trap and no failed archive write.
- **The `two_browsers` LIVE RE-RUN on the fixed client** (2026-09-26 06:12–06:22 UTC, deployment
  19 — I-82's deferred item, run on the fresh UTC day the two-day stop gave): **PASSED**, 1 passed in
  10.1 min. Game `sim-1a8263683952c0b4` **finished in 2,510 moves**, 601 s, 0 idle looks, the
  largest hand seen 3. **The watcher held to 1,886 applied moves, asked `/events` 0 times and opened
  `/stream` ONCE at every checkpoint**; each player opened the table's stream twice, both before the
  first move — no reopen during play. The deal's 409 forgiven 0 times; the beacon's CORS pair a 2 /
  b 2 / c 2; **no other console error on any of the three pages** — the first run's "WebSocket is
  already in CLOSING or CLOSED state" lines are GONE (the keep-alive fix's live witness) and no 426
  appeared (the transport wait's). No "Call log". Its game is the bucket's second finished game.
- **The exit's headed runs and the headed walk, on deployment 19** (§3.3): the lobby's Open-tables
  join, the lobby chat, the watcher's unlock and the 390 px walk; and run 3, a paced game whose
  object HIBERNATED in each of four pauses and came back on the next move.

**What did not run.**
- **The live replay after the fix wave.** It last ran on deployment 18. Deployment 19 moved server
  code — F-1's catch, F-2's generations, the `wake` line, the picker's code stamp — none of which
  changes a payload, and the pool's replay of the whole corpus passed at `bd8ce497` (195 cells, the
  corpus cell among them).

**Which claim rests on which evidence.**

| claim | the evidence | what it does not cover |
|---|---|---|
| A two-person game plays to a winner on the live Worker, and a watcher holds ONE socket with ZERO `/events` | live run 1 (2,862 moves, deployment 17) and the re-run (2,510 moves, deployment 19); both records in the bucket (§3.3) | — |
| A whole live game runs with no unknown console error on the CURRENT client | the re-run, on deployment 19: every console clean but the beacon's forgiven pair | a mid-game edge drop (R3's E-5), which fails the one-socket checks by design |
| No stream or socket opens before the transport is known, and the word is never "live" before a connection | the jsdom cells red-first at `d025ba43` and I-86's at `e20c8ff8` (R3's probe re-run: "reconnecting" from 0 to 4.9 s, the default stream at 5.1 s, then "live"); run 2a live (no 426); the re-run and the exit's runs (no 426) | — |
| No keep-alive is sent into a closing socket | the jsdom cell; the re-run, whose game ended with no `CLOSING` line on any of its three pages | — |
| The deal's designed 409 is forgiven exactly, and nothing else | node cells refusing a 409 elsewhere, another status on `/deal` and a lookalike path; run 2a (forgiven a 2) and the exit's run 3 (x 2) | — |
| The whole corpus is byte-identical live, and the replay is repeatable | run 2b, twice, on deployment 18; the pool's replay at `bd8ce497` | only while `wire`'s records are byte-identical (§7); the reset recipe is RUNBOOK §6.7's "The replay's reset" |
| The lobby's Open-tables join, the lobby chat, the unlock and the watcher work live | the exit's three runs, phase 1, on deployment 19 (§3.3) | — |
| The table object HIBERNATES between moves and comes back, with chat and a watcher | the exit's run 3: four `wake` lines, `wakes` 1 → 4, one on the first POST after each 60 s pause (§3.3) | the wake's cost on a long record, which the exit's 40-move table does not reach |
| A finished game's save lands live, loads in the CLI, and `why` reconstructs a bot move from a bucket record | the bucket synced twice: two finished games under `test/rev7/over/` — run 1's loaded in the CLI, the re-run's listed at 157,033 B; a bot game's `why 0` reconstructed, the honesty gate passed (§3.3) | a game-over save's `archive-stamped` line (neither finished game's run was tailed at its game over) |

**The consequence, stated.** Nothing the owner asked of the live tiers is deferred: the console-clean
whole game, the hibernation witness and the live replay all ran. What stays outside the evidence is
named in §7 — chiefly a phone, a mid-game socket drop at the edge, and a wake on a long record. Note
R3's E-5: the live walk asserts ONE stream open for the watcher, so an edge-side socket drop — a
runtime update restarting the object — fails the run by design; read the tail before believing a
red live run.

### 3.3 The exit (spec §5.5, as Ruling I-88 numbers it, and as the owner directed)

**The instrument** (I-88, numbered with its cost). The headed exit game is a SHORT, PACED
two-person game with a watcher — the wake witness, the per-move wall and CPU, chat, the console —
ABANDONED at its end; it is NOT the whole game spec §5.5 words. The whole live two-person games are
live run 1's and the re-run's, played to a winner, whose records are the finished games in the
bucket. **Cost:** a whole game is not re-played headed — the owner's direction of 07:00 UTC accepts
the headed short test plus the stated ending state, and the quota line is the reason. The spec is
`web/e2e/exit_game.spec.ts`, opt-in (`YOKOHAMA_WEB_EXIT=1` with the live target): three contexts
(`x`, `y`, `w` at `@exit.test`), forty moves at human pace, four long pauses (after moves 5, 15, 25
and 35; `WAKE_PAUSE_MS`, default 45 s) with every request the pages send during them tapped and
printed, table chat and lobby chat, a watcher who must reach every move and hold no hand, the three
pages' visibility as the spec's own precondition (I-91's R-N6), and the abandon with its marker read.
The owner's direction of 07:50 UTC is what it is for — "lag times so hibernation is proven and
bringing DOs back online are proven as well, with chat and the works". The wake count is read off the
`wake {id, wakes}` line (I-84), because `meta.wakes` had no other reading path.

**The window.** Deployment 19 opened it at 10:49:45 UTC on 2026-09-24 with `npx wrangler deploy
--var YOKOHAMA_WEB_COUNT_WAKES:on` after the guard (RUNBOOK §6.7's "The exit's window" — a `--var`
sits beside the file's `vars` and replaces none; the plain `npm run deploy` closes it). It stayed open
across the two-day stop. Every run used `WAKE_PAUSE_MS=60000` — the spike saw eviction after ~35 s
idle.

**Run 1** (2026-09-24 10:50 UTC) — **the seating PASSED LIVE in 5 s.** Table `sim-856c9a6ea7a81c37`
was opened locked by `x@exit.test`; `y@exit.test` read x's lobby line, posted its own, joined through
the lobby's **Open tables** link with the password; both kept; `w@exit.test` unlocked the table and
watched with no hand. **The four assumptions the spec had never run live all held**: the Open-tables
row, the lobby chat, the unlock and the watcher. **Then the spec's OWN PRECONDITION stopped it,
before the pause after move 5**: x's window read hidden at 10:50:28 UTC. The three headed windows
STACKED on the display, and macOS's occlusion marks a covered window hidden whatever Chromium's two
occlusion switches read; a hidden page lets its socket go by design (R-P8). Not a product finding —
R3's R-N6 predicted exactly this. **The fix**, `f2c27a3f`: the spec places its windows side by side
through the browser protocol.

**Run 2** (2026-09-26 06:08 UTC) — **the seating PASSED again in 4 s** (`sim-6a7ae105764b7f76`), and
the same four assumptions held. The windows were placed, but Chromium widens a window to its MINIMUM
width, **500 px**: asked 420×820 at 0, 426 and 852, they read back 500×820 at those lefts and 38 px
down — still overlapping by 74 px, the third running past 1280 — and x read hidden at 06:08:10 UTC.
The precondition stopped the run again; not a product finding. Its failure context named
`CF_Authorization` once — the cookie's NAME in a source comment, with 0 value-shaped matches, so
I-79's fix held on a real failure. **The fix**, `754b80ae`: a 2×2 GRID when three minimum-width
windows cannot share the width, laid UNDER the menu-bar inset read back from the first placement,
every window's bounds read back and the run refused on any overlap or a window off the screen, and
the seating's transient hidden reports forgotten, with visibility checked from the start of play.

**Run 3** (2026-09-26 06:20–06:26 UTC) — **PASSED**, 1 passed in 5.2 min. Table
`sim-6334a4d57da5499f`:
- **Phase 1**, seating, in 4 s: the grid put x and y above and w below x, under a 38 px inset — read
  back x 634×391 at 0,38, y at 640,38, w at 0,435 — each with a 618×301 viewport.
- **Phase 2**, play: **40 moves in 307 s**. The four long pauses lasted **60.0 s** each, after moves
  5, 15, 25 and 35, with nothing asked meanwhile; the move after each pause answered its
  `POST …/input` in **69, 69, 75 and 67 ms**, against an awake median of **58 ms** over the other
  36 — the wake as the page sees it, about 10–15 ms. Table chat: x 10, y 10, and the watcher's panel
  held 20 lines. Lobby chat: x 1, y 1. The watcher was held to 31 applied moves with no hand. All
  three windows read visible at 10 checkpoints. The table's stream opened w 1, x 3 → 3, y 3 → 3: no
  reopen during play.
- **Phase 3**, the abandon: x abandoned the table, its address answered **404** `this table was
  abandoned on 2026-09-26 — its game is in the archive`, x, y and w were told, and the lobby listed it
  no more.
- **Forgiven**: the beacon's pair 2 / 2 / 2; the deal's designed 409 on x twice (the creator's
  page). **Every console clean.** No "Call log".

**The wake witness**, off the tail (`tail19b.json`: 41,578 events on version `204aba6b…`, 1,230 of
them the exit table's): **four `wake` lines, `wakes` 1 → 2 → 3 → 4**, each on the first POST after a
60 s pause.

| the wake | `wakes` | CPU | wall |
|---|---|---|---|
| 06:21:43.726 | 1 | 8 ms | 19 ms |
| 06:23:00.569 | 2 | 7 ms | 18 ms |
| 06:24:17.630 | 3 | 9 ms | 60 ms |
| 06:25:34.840 | 4 | 6 ms | 16 ms |

**The object HIBERNATED in every pause and came back on the next move.** A wake costs the object
**6–9 ms of CPU**, against a **2 ms** median (the maximum 15 ms) over the 40 object-side move POSTs;
the stateless Worker's own half is **2 ms**. The table's other lines: `slot-pushed` 10, `slot-acked`
11, `archive-landed` 1 (the abandon's save), `trap` 0, and 0 exceptions.

**The headed walk at 390 px** — owed across two releases (spec §7 item 2) and TAKEN on deployment 19
on 2026-09-24: headed Chromium at 390×844 @2×, two `.test` contexts, the orchestrator's own eye on
16 + 16 screenshots (the script and its screenshots in the session scratchpad). **No layout defect
seen.**
- **Clean:** the DOOR and its controls ("Play the bot here", the storage sentence, "Back up
  everything", "Restore from a backup" with its file chooser, "Play with others", "Preparing to play
  offline…"), with no overflow; the LOBBY with its back-link ("Play on this device"), "New table",
  its three sections and the folded lobby chat; the CREATE FORM (players, two seats, the table's
  name, your name, the password box, the seed, "Deal"); the OPEN TABLE in the creator's view (the
  seats, the LINK field, "copy", the folded chat, "Abandon" and "Lobby") and in a stranger's ("take a
  seat · password required", the name and password fields, "sit here", "Lobby"); the DEAL (b:
  "Waiting for the others · waiting on walk-a"; a: "Choose your card" with the two cards and their
  keep scores, the marks, the tracks, and the bot's seat that "keeps one of its two cards —
  unseen"); the BOARD (the boards' row scrolling sideways, the panels stacked, the prompt bar); and
  the CHAT PANEL, folded ("show").
- **The five error strips of M6.5b2's list — ONE provoked in the browser:**
  - **PROVOKED — the UNLOCK strip**: the open table's password refusal for a stranger — a wrong
    password at "sit here" → 403 on `/join`, and "wrong password for this table" in orange under the
    buttons — on the screen M6.5b2's list calls the unlock screen. Clean.
  - **NOT provoked — the create form's strip**: a table name with spaces is a legal display name (an
    id is drawn for it), so "bad id with spaces" DEALT a human-vs-bot table, forced afterwards
    through the operator route.
  - **NOT provoked — the deal's strip**: the second keep's 409 came through the API, because the
    page's own guard prevents a second click.
  - **NOT provoked — the chat panel's strip**: the input eluded the script's locator, so the empty
    post's 400 was read through the API; the panel's row geometry at 390 px is the join spec's live
    reading (321 px = 179 + 129 + 314, 98% of the row).
  - **NOT attempted — the open table's own strip.**
- **Console:** the beacon's two-line artifact (the walk sends the claimed header context-wide) and the
  designed refusals (the 403 join, the 409 keep, the 400 chat) — nothing else.

**The bucket and the CLI**, synced with the owner's read-only key in ONE command's environment, on
2026-09-24 (20 files) and again on 2026-09-26:
- **Two FINISHED games** under `test/rev7/over/`: `sim-552c786c4ba671ff.json` — **180,493 B**, live
  run 1's game, **2,180 events**, two human seats, `over: true`, `winner: 0`, `archived_at`
  1790232277 (06:44:37 UTC), no lock or roster in its table; and `sim-1a8263683952c0b4.json` —
  **157,033 B**, the re-run's — each with its table.
- **Abandoned** under `test/rev7/abandoned/`: the exit's run 3 game `sim-6334a4d57da5499f` (3,465 B,
  40 moves) and its table; run 1's `sim-856c9a6ea7a81c37`; two bot games, `exit-bot-1790247093` (no
  bot move) and `exit-bot-1790247143` (the bot in the starting seat, 9 events); the two join-spec
  tables; and Task 13's `t13-abandon-…`.
- The replay's own: `test/rev7/over/wire.json` and `test/browser/red_at_wire.test/…`.
- **The CLI** (2026-09-24): run 1's finished record LOADS (`log` shows its 2,180 lines, the last
  "2179 Blue: End additional-action phase"); the bot record loads and **`why 0` RECONSTRUCTS the
  bot's first move** — seat 1, `v4-step9-draft1@v2{b=4,eval_impl=3,k=1,m=8,…}`, "Only legal
  option", decision seed `0x2e874e72bec9ed69`, the 34-row explain table, TOTAL 15. **The honesty gate
  passed under this build's constants.** Every save the exit made sits under `test/`.

**The whole corpus live** stands on run 2b's two replays on deployment 18 (§3.2); the replay was not
run again at deployment 19.

### 3.4 The quota guard, and what a game costs

Workers Free's daily quotas are shared with the account's other Workers. The orchestrator read the
UTC day's figures from the account's GraphQL analytics before each live run; the HOLD line for this
cycle is **80,000** for EACH guarded figure — the Worker requests and the Durable Object rows written
(the owner, 2026-09-23, mid-Task 12; the plan said 60,000).

| read at (UTC) | the UTC day | Worker requests, account | `yokohama-web` | object rows written | object rows read | object invocations |
|---|---|---|---|---|---|---|
| 2026-09-23, after deployment 14 | 2026-09-23 so far | 22,117 (39 scripts; `yokohama-web` not yet in the rollup) | — | 278 | 281 | 304 |
| 2026-09-24 04:47 | 2026-09-23, CLOSED | ~23,259 | 27 | 371 | 836 | 351 (12 errors — the deliberate refusal readings) |
| 2026-09-24 04:47 | 2026-09-24 so far | ~4,415 | 41 | 217 | 2,034 | 129 (2 errors) |
| 2026-09-24 06:32:29, before live run 1 | 2026-09-24 so far | lost to a wrong result path (the 04:47 figure stands as the pre-run baseline) | — | 238 | 2,060 | — |
| 2026-09-24 06:48, after live run 1 | 2026-09-24 so far | **44,270** | **38,390** | **7,259** | **1,344,253** | 39,234 (13 errors — non-`ok` outcomes, not failures) |
| 2026-09-24 10:16, after run 2a and the replay twice | 2026-09-24 so far | **51,467** | **42,524** (0 errors; the join and the two replays added ~4,100) | **14,939** | **1,562,634** | — |
| 2026-09-26 06:05, before the re-run | 2026-09-26 so far | 7,547 | 0 | 81 | — | — |
| 2026-09-26 06:26, after the re-run and exit run 3 | 2026-09-26 so far | **42,036** | **34,241** | **6,466** | **1,202,112** | 34,883 (18 non-`ok`, the cancels and disconnects kind) |

At 10:16 on the 24th the headroom to the line was about 28,500, and the `two_browsers` re-run
(17,000–38,000) did not fit (I-82); on the 26th's fresh day it did, with room to spare.

**What a live game cost:**
- **A three-page two-browser game**: run 1 (2,862 moves, with the six-move join and the cleanups)
  cost **~38,350 Worker requests ≈ 13.4 per move**, **~7,000 rows written ≈ 2.5 per move** and
  **~1.34 M rows READ ≈ 470 per move**; the re-run (2,510 moves) cost **~32,000 requests ≈ 12.8 per
  move**. That is three live pages each refetching on every frame, plus the harness's own probes
  (`appliedEvents` alone is one GET per move).
- **The exit's paced game**: about **1,400 requests for 40 moves, ≈ 35 per move** — three pages, chat
  both ways, the watcher, and the harness's own reads, spread over pauses.
- The join spec costs about 100 requests and the replay about 2,300 a run.

At these rates Free's day holds about **30 human-paced games of ~250 moves**, and it is the
**REQUESTS that bind first**: 100,000 a day is ≈7,500 harness moves at 13.4 a move (≈2,900 paced
moves at 35), rows read bind second (5 M a day ≈ 10,600 moves at 470), rows written third (100,000
≈ 40,000 moves at 2.5). The spec's §2.8 never estimated rows read:
the orchestrator's whole-branch reading traced it to each request re-reading the three documents up
to three times (F-4, a cleanup-cycle item). **R-6.5c.6's push trigger, read:** at the owner's usage
neither requests nor rows written bind, so push stays deferred; requests — the client's refetch on
every frame — are the lever to watch, rows read the next.

---

## 4. The reviews

Every task had an Opus implementer and a fresh Opus reviewer over a packaged diff (trimmed above
~150 KB); where the review found an Important it ruled, the SAME implementer ran a fix round as a
NEW commit, and the reviewer re-read the round in scope. Every report is a file in the ledger's
directory. Across the fifteen tasks: **0 Critical**; every fix round closed with 0 open; the new
breakage the rounds introduced was comment- or doc-level every time (at most two items in a round),
each closed in the task's own record commit or deferred by name to the whole-branch fix wave.

| task | review (at) | verdict | fix rounds | re-review |
|---|---|---|---|---|
| 1 | `ab9c6812` | Spec ✅, Approved; 0 C / 2 I (plan-mandated) / 8 M / 4 escalations | `786d89f7`, comments and docs only | 9 addressed, 0 open |
| 2 | `b7a682c8` | Spec ✅, Approved; 0 C / 0 I / 5 M / 4 escalations | none (the minors deferred, the escalations ruled forward to Tasks 7 and 11) | — |
| 3 | `528369e3` | Spec ✅, Needs fixes; 0 C / 3 I / 7 M | `a48cf5f5` + `d375391f` | 9 addressed, 0 open; 2 Minor comment imprecisions deferred |
| 4 | `4f6f37d7` | Spec ❌ on one narrow item, Needs fixes; 0 C / 1 I / 4 M; all four deviations sound | `5011bf99` | 5 addressed, 0 open |
| 5 | `f665035d` | Spec ✅, Approved; 0 C / 0 I / 5 M / 4 escalations | none (two ruled follow-ups BEFORE the review) | — |
| 6 | `928387a4` | Spec ✅, Needs fixes; 0 C / 1 I / 5 M / 6 escalations | `d2218e1c` | 6 addressed; 1 Minor (two overstated clauses) deferred |
| 7 | `e1f54ec8` | Spec ✅, Needs fixes; 0 C / 2 I / 3 M | `87caa769` | 5 addressed; 1 Minor deferred |
| 8 | `4e109268` | Spec ✅, Approved; 0 C / 0 I / 4 M / 5 escalations | none | — |
| 9 | `0ff17088` | Spec ✅ (2 ⚠️), Needs fixes; 0 C / 1 I / 6 M / 5 escalations | `683f36c6` | 7 addressed; 1 comment sentence deferred |
| 10 | `b6f843c5` | Spec ✅, Needs fixes; 0 C / 2 I / 5 M / 4 escalations | `87d30a14` | 7 addressed; 1 comment clause deferred |
| 11 | `a8e6c8a5` | Spec ❌ on one item, Needs fixes; 0 C / 3 I / 9 M / 4 escalations | `756dc759`; then `60346fe4` + `f64310c7` (the record) | 11 of 12, and one new latent Minor (N1); all closed by the record's commits; five Minor observations left for this close's docs pass |
| 12 | `ebe16d32` | Spec ✅, Needs fixes; 0 C / 1 I / 9 M; the I-42 adaptation ACCEPTED | `b565fd2a` + `b24149b5`; then `e06abfd4` + `953abd64` (the record, two rounds) | 8 addressed; 1 test-only Minor deferred; the record's six wordings fixed |
| 13 | `ad476e8f` | Spec ❌ on one ruled item, Needs fixes; 0 C / 2 I / 6 M / 3 escalations | `4d984f52`; then `435e2bfd` (the record) | 5 addressed, 0 new |
| 14 | `24e53754` | spec ✅, APPROVED; 0 HIGH / 2 MEDIUM / 10 LOW | `c2e8d084`; then `959df82e` (the record) | all addressed; N-1 (MEDIUM, docs) rode the record; L-11 queued for the close's RUNBOOK pass |
| 15 | `b1e749db` | spec ✅, quality ❌ NOT APPROVED; 1 HIGH / 1 MEDIUM / 9 LOW; the reviewer's own mutants MA, RA, RB SURVIVED | `b5eca77f`; then `d025ba43` (the live run's three findings) | all addressed twice by mechanism; LOWs R-1 and R2-1 to the whole-branch triage |

**Four process lessons the cycle produced, each now a standing rule** (the orchestrator's memory
carries them):
- **An implementer's DONE and a ruling crossed at Task 1**, and the implementer AMENDED a commit about
  to be reviewed — the STOP-on-amend rule went into every later dispatch text. (An UNREPORTED SHA may
  still be amended before its report: the follow-up's `736c3722` became `bd8ce497` that way.)
- **An acceptance reply that asks for anything after a DONE is itself a fix-round dispatch** — say
  "as a new commit" and sequence it. Task 15's implementer left such edits uncommitted under a
  reviewer; and the rule was broken a second time by its own author at the fix wave, when I-89's two
  additions went to wave A after its DONE and after wave B had been handed the tree: wave A committed
  `9630edd4` while wave B held the tree. The files were disjoint, so nothing was lost; wave B checked
  its edits and committed on the moved HEAD.
- **A reviewer's mutants run on an export** (`git archive`) unless the tree has been handed to it, and
  once a fix round is dispatched the tree is the implementer's alone (Task 15).
- **An agent the account's weekly limit stops is WOKEN by message, never respawned** — the owner at
  the reset, 2026-09-26: "please continue, note the three teammates will need to be addressed
  directly so they can resume as is".

### 4.1 The whole-branch review (Fable and three Opus area reviewers, over `2621fbe..d025ba43`)

The package is 2.8 MB over 57 commits — beyond one read — so it was split by area. **The
orchestrator (Fable) read every file of `worker/src` in full** at `d025ba43` (`table.ts` 2,054
lines; the other files about 4,000), the two layer changes, the configurations and the deploy
guard. Beside it: **R1** (Opus) the Rust module, the layer and the fences, on an export; **R2**
(Opus) spec §5.7's mutation battery and the shell's test hygiene, in the real tree, which was its
own; **R3** (Opus) the client, the live tiers, the scripts, the deployment surface and the
documents, on exports.

- **The orchestrator's reading** (`final-review-fable-notes.md`): **F-1 (MEDIUM)** an exception
  escaping an object reached the platform's plain 500 — not the API's `{"error": …}` shape, and with
  no log line; **F-2 (MEDIUM)** a force then a re-create of one id within the SAME SECOND left the
  new table unlisted for 90 days about half the time, because a generation's `at` was the pinned
  reading in seconds and `cmpGen` broke the tie by the random half; **F-3 (LOW)** a token header
  without `kid` would match a kid-less key; **F-4 (LOW, the cleanup cycle)** the three documents
  are re-read up to three times per request — the ~470 rows read per move. Everything else was
  verified sound by mechanism, listed in the notes (the strip rule, identity's order, the Origin
  check, `ended()` before every read, the endings under `blockConcurrencyWhile`, the ending's one
  batch, the acknowledgement rules, `force`, the alarm's three rules, the framing, the house's
  unbroken decide-and-batch, `executePlan`'s order, the strings rule).
- **R1** (`final-review-r1-report.md`): **0 HIGH / 1 MEDIUM / 9 LOW; the fences HOLD.** On an export:
  worker 55, web 278 + 11 + 2, the committed scripts equal a fresh emission, clippy and the `wasm32`
  check clean, three scrub mutants caught. **M-1**: a stored picker row carries no code identity, so a
  line whose MEANING changed between recording and restore — the first CLI-only deploy that alters
  prompt numbering, grammar or picker behaviour while a row is stored — applies and persists a move
  nobody clicked (probe P1: `1` re-fed at the same record, 0 → 1 events). Unreachable at HEAD, the
  CLI byte-identical all cycle; the design case for re-feeding IS a deploy. The LOWs: two callers the
  archive root and the prefix could name differently (L-1); a dev-door caller that is not an address
  reaching a host-written 500 where the layer answers 401 (L-2); `init` accepting the positional
  array (L-3); four false or overstated doc sentences and Tasks 2, 3, 5 and 6's deferred minors
  (L-4…L-9). Escalations: **E-1**, spec §6's "the deployed module is built at 132" is false for every
  deploy; **E-2**, `cargo clippy --target wasm32-unknown-unknown` fails inside the CLI
  (`clippy::drop_non_drop`, `session.rs:439`) — pre-existing, fenced, no gate runs it.
- **R2** (`final-review-r2-report.md`): the battery (§4.4) and **0 HIGH / 0 MEDIUM / 5 LOW** of
  hygiene: `slot.test.ts`'s ten held cells restore without `settle` in `finally` (the hazard did not
  reproduce: eight in-window failures under two mutants all exited); one mislabelled battery-6
  comment; seven weak not-null assertions, each beside a stronger one; a bucket swap with no restore
  handle (harmless); row 17's catcher covers only the table's sites. **76 suite runs, no hang.**
- **R3** (`final-review-r3-report.md`): **2 HIGH / 6 MEDIUM / 13 LOW** and **29 stale counts and
  sentences** for this close's docs pass. **H-1**: spec §6's "built at 132" and two standing
  sentences (RUNBOOK lines 93–94, `test.yml` 77–79) are false — every deployment 14–18 was built with
  `wasm-opt` 133, and the ledger had no ruling for it. **H-2**: fix round 2's transport wait was
  UNBOUNDED, so a first visit whose `/api/health` never settles opened NO stream on the NAS while the
  word read "live" (measured: `b5eca77f` streams=1, `d025ba43` streams=0 over 120 s) — the Task 15
  re-review's "the old code hung the same way" was false. MEDIUM: the live replay repeats only while
  `wire`'s records are byte-identical (M-1); no socket-arm cell pins "no poll while connected" (M-2);
  the deploy guard cannot see stale-but-consistent build output (M-3); CLAUDE.md names none of the
  Worker's gates (M-4); RUNBOOK §6 has no record of the live tiers or the quota guard, and two
  sentences contradict the live tier's design (M-5); a malformed operator list is echoed, address and
  all, in the 500 every `/api/*` answers (M-6). The three inherited jsdom flakes are now NAMED with a
  cheapest fix each (§6.3). Secrets clean: the only `.dev.vars`-shaped value in the diff is the
  credential's public client id, committed by design.

### 4.2 The ONE fix wave, in two sequential dispatches on disjoint files — and its follow-up

Ruled at `d025ba43` (the ledger's "TASK 16 FIX WAVE"): never two implementers in the tree. **Wave A**
— the Rust crate, `worker/src`, `worker/scripts`, `deploy/RUNBOOK.md` and `.github` — took I-85, R1's
L-1…L-9, F-1, F-2, F-3, I-84, R3's M-1, M-3, M-5, M-6 and H-1's RUNBOOK half, and R2's five hygiene
LOWs: **`0919931c`**, then **`9630edd4`** for I-89. **Wave B** — `web/src` and `web/e2e` — took I-86,
R3's M-2, the three flakes, Task 15's R-1 structural cell and the exit spec: **`e20c8ff8`, the fix
wave's head**. The re-reviews then found eight small residuals, and ONE follow-up commit by wave A's
implementer took them all (I-91): **`bd8ce497`**, the head of the shipped code. §1.16 lists what
each commit carried, with its counts.

**Two deviations wave A reasoned and the re-reviews found sound:**
- **The assets' freshness stamp is the copied `sw.js`**, compared byte for byte with the current
  `web/dist/sw.js` — no new file. It is stronger than comparing release names, since the name
  digests only the precache list and a change to `web/src/sw.ts` alone keeps it; and a stamp file
  inside `worker/assets/` would have been uploaded.
- **Two digests, two purposes.** The module's freshness stamp is a node digest over everything the
  module compiles from; the picker's code is a Rust digest over the three roots that give a line its
  meaning. The picker's must stay narrow, so pickers survive a worker-only deploy; the freshness
  one must be complete. The sets nest (R1, twice), so any change that moves the picker's code also
  makes the guard demand a rebuild.

**Not done, by design:** R1's L-9 asked for a cell on `archive_over_retry`'s second `NotFound` arm,
whose home is the fenced `games.rs`, and the arm is unreachable from the worker crate.

### 4.3 The scoped re-reviews, and the re-checks of the follow-up

- **R1, the Rust half, at `e20c8ff8` (on an export): every finding ADDRESSED but one residual.**
  L-2 was proven by a differential — the shell's address rule against the layer's `User::parse` over
  60,018 generated strings, 0 disagreements. I-85 was proven by mechanism: the digest matched an
  independent recompute over 51 files, held across `cargo clean`, moved on an edit or an added file
  and ignored dotfiles; rows under another code or none were cleared with nothing applied; a mutant
  disabling the check reds its cell. **The residual, R-1**: the engine's `crates/yokohama-core/src` was
  not a root, yet the engine's legal-list ORDER numbers the main prompt, and a pure reorder must not
  bump `SEMANTICS_REV` — so a line's meaning could change under an unchanged stamp. **New breakage,
  two LOWs**: N-1, two config sentences; N-2, `picker-dropped` logged before the batch, so a
  dispatch that then trapped would log the same drop twice. **At `bd8ce497`, R-1, N-1, N-2 and R3's
  R-N1 are ADDRESSED**, no new breakage: the digest matches an independent recompute over 113 files,
  an edit to `legal_moves.rs` moves it and its revert restores it, and the module's `cargo metadata`
  graph is {bridge, cli, core, web, worker} with nothing unread. Worker 59, clippy clean, `cargo doc`
  0 warnings, the fences hold.
- **R3, wave A's scripts and docs and all of wave B, at `e20c8ff8` (on an export): every HIGH and
  MEDIUM within the waves' reach ADDRESSED by mechanism** — H-1's wording; H-2 by R3's own probe
  re-run ("reconnecting" from 0 to 4.9 s, the default stream at 5.1 s, then "live"; a retry asks a
  new read); M-1's reset recipe and the stop on a 409; M-2's mutant caught ("expected 2 to be 1"); M-3
  refusing a layer edit and a `web/dist` change on a scratch copy of the real layout; M-5's §6.7
  against the ledger, figure for figure; M-6's positional refusals; and the flakes (App 116/116
  under seeds 22 and 44; the shuffled suite 1056/56). **M-4 — CLAUDE.md — is this close's docs
  pass.** **New: six LOWs, R-N1…R-N6**: the freshness digest omitted `yokohama-bridge`, a path
  dependency of the CLI (and the RUNBOOK's "four crates" was false); the structural cell was defeated
  by an alias (`const api = x.request; api.get(…)`); a FIRST connection reads "reconnecting…" (R-N3,
  RECORDED for this close and for M6.5c2 — `Link` has no "connecting" word); no RUNBOOK steps for the
  exit's wake window; the quota query's documented form was not the one run; and the exit spec never
  read `visibilityState`, so a hidden headed window would fail its one-socket checks by the client's
  own R-P8 rule. **At `bd8ce497`, R-N1, R-N2, R-N4, R-N5 and R-N6 are ADDRESSED** — the alias refused
  even inside `liveRequest`; the bridge in the digest with a `cargo metadata` cell that catches its
  removal (so `test:scripts` now needs `cargo` on `PATH`, which CI has); the window's `--var` open
  and plain-deploy close confirmed in wrangler 4.136.3's own source; the `date`-form query; the
  visibility watch with six cells. **One new LOW, R-N7**: a request context MADE inside `liveRequest`
  (`x.request.newContext()`) passes the rule — refused inside `liveRequest` or out by `528b1285`. R3's docs list after the wave: **8 of its 29 items fixed, 21 open, 5 new** — the
  open ones are this close's docs pass.

### 4.4 The mutation battery (spec §5.7; R2) — at `d025ba43`, and ALL of it again at `e20c8ff8`

**At `d025ba43`** (Ruling I-83: as a review instrument, in parallel with the area reviews, so its
survivors could join the one fix wave): **39 rows, enumerated from the spec at execution** (the
plan's count was 39 as well). **38 CAUGHT by a named cell, 0 SURVIVED, 1 RECORDED — row 4**, a frame
sent before the batch, green as the spec's ruling predicts: the platform's output gate holds every
outgoing message until the batch is durable, so the mutant is equivalent (the M6.5b "uncatchable by
ruling" precedent). **65 mutants in all**: 61 for the 39 rows — a row with more than one plausible
defect got one mutant per defect — plus Task 13's M33–M36. **59 of the 61 row mutants were caught.**
The two green ones:
- **row 4's**, RECORDED as above (a first draft that called `sendFrames` twice was "caught" by the
  double call, not by the order, and was discarded);
- **r17b — the ARCHIVE object's second dispatch taking a reading of its own — EQUIVALENT BY
  MEASUREMENT.** A temporary ticking-clock probe handed out two readings under the mutant and still
  answered the identical 201: a browser save's plan stamps no time and its key covers only keys and
  digests, so the pin at that site is unobservable. RECORDED as Task 13's M30 was — and, since the
  fix wave, written into `archive.ts` and its cell; row 17 is caught by its other two variants.
- **M33–M36 still survive and stay RECORDED** under Task 13's ruling m3: each makes a retry LATER by
  at most one rung (an hour) and never loses one, because any alarm runs every delivery.

**At the fix wave's head, `e20c8ff8`: ALL 39 rows re-run** — a superset of I-83's list, because the
wave moved the code under many mutants and a run costs about 6 s. **38 CAUGHT by the same named
cells, 0 SURVIVED, row 4 RECORDED; 59 of the 61 row mutants caught**; r04, r17b and M33–M36 green as
recorded. Only r33's anchor had moved (L-1 rewrote `ReceiptArchive::root`). **Spot checks of the
wave's new cells, one mutant each: 11 of 11 caught** — I-85's other code and I-89's unstamped row
(each by the Rust cell and by exactly its own pool case), the missing `picker-dropped` line, F-1's
catch removed, F-2's seconds (7 cells), I-84's missing line, M-6's echoes, M-3's stale `web/dist`
and its never-compared stamp, and R3's M-2 poll. **Hygiene:** R2's five LOWs fixed, no weak
assertion left; one residual LOW — two held cells outside `slot.test.ts` without a `settle` in
`finally` — went into the follow-up and was fixed at `bd8ce497`. **76 harness runs and 89 suite
executions, no hang.** Every mutant was applied with an exact-text edit refusing an ambiguous anchor,
run in its own process group with a 300 s kill, restored with `git checkout --`, and byte-verified by
SHA-256 with a clean `git status`.

---

## 5. The rulings taken in execution, with their costs

Every decision taken on the owner's behalf, in the order taken; the ledger carries each with its
`Ruling` line. Two numbering schemes meet here, and both are the ledger's: the pre-flight rulings
cite the scan's row ids (`I-` interface, `S-` self-consistency, `R-` rubric, `Z-` citation, `C-`
constraint); from Task 14 on, the orchestrator numbered its own rulings on from the scan's last
interface row, **I-65 to I-91**. A task review's findings keep the reviewer's labels (`I1`, `M-5`,
`E3` …). **No third layer change was ruled**: the layer moved by exactly R-6.5c.4's two items.

### 5.1 The OWNER's rulings and directions of this cycle (not the orchestrator's)

- **2026-09-23, before execution** (the plan's Global Constraints; the execution handover §2):
  subagent-driven for EVERY task, Opus the floor, Fable the whole-branch review, the orchestrator
  running every step that touches the account; ONE green commit per task with its red run quoted;
  Web Analytics and Bot Fight Mode (with its JavaScript detections) stay ON — zero console errors is
  not the goal, a known recorded error is allowed and an unknown one is not; the TTL is 90 days;
  P-1 approved; the push and the PR pre-authorized; a third layer change the orchestrator's ruling,
  with its architectural reason; the quota guard; an anomaly goes to a research subagent before it
  goes to the owner. No research subagent was dispatched: no trap, `exceededCpu`, hang or unexplained
  5xx appeared in any recorded live reading (the tails of deployments 14–19, the exit's among them;
  the Observability query of live run 1).
- **2026-09-23, mid-Task 12: the quota guard's HOLD line for THIS CYCLE is 80,000**, not 60,000,
  for each guarded figure, since the account's overall use leaves headway; Free's limit stays
  100,000, shared, and the reading is still taken and recorded before each live run.
- **2026-09-24 ~07:00 UTC — the M6.5c2 handover's scope**, in the owner's words: "tidying up,
  arranging the docker vs cloudflare setup in the repo, finalizing that documentation and
  construction, and testing strategy to perhaps start with that at the right time to handle the full
  test suite as we wrap this up with the whole set of modules and docs and repo strategy and
  frontend design tweaking." **And the live tier:** "If we won't be using the full test now we should
  know the consequences and still be comfortable through either the headed test or otherwise our
  ending state in the report." §3.2 is that statement.
- **2026-09-24 ~07:50 UTC**: (1) this cycle's `test/` saves REMAIN in the bucket, with no lifecycle
  rule now; the M6.5c2 handover carries the choice — a periodic script, or an R2 expiry after N days
  on the `test/` prefix — with a recommendation, decided then. (2) The exit "should include lag
  times so hibernation is proven and bringing DOs back online are proven as well, with chat and the
  works" — §3.3's paced game. (3) "Otherwise seems this is ready to move forward."
- **2026-09-26 ~06:0x UTC, at the weekly limit's reset**: "please continue, note the three teammates
  will need to be addressed directly so they can resume as is" — each stopped agent woken by
  message, never respawned.
- **2026-09-26 ~06:3x UTC, at the close-out**, after re-authorizing WARP: clear to deploy, push and
  open the PR. On CI: "CI will likely need to be updated, optimizing for what's reasonable to have in
  github vs being part of the local work". On the README: "a comprehensive update to the readme.md …
  or a detailed readme for the cloudflare version that is linked going through in high detail the
  reasoning and mechanics of the durable object/WASM/cloudflare worker optimization". And any other
  action needed, verified with subagents (the ledger's words for the last). The twentieth deployment
  followed at 06:38:27. A docs and CI agent was dispatched at once for `worker/README.md`, the root
  `README.md` and `.github/workflows/test.yml`; that work's own commit is its record.

### 5.2 The plan's rulings, as executed

| ruling | what it rules | how it was carried out | cost |
|---|---|---|---|
| **P-1** | `YOKOHAMA_WEB_ADMINS` is a Worker SECRET, not a `vars` entry: its value is the owner's address and `wrangler.jsonc` is committed to a repository headed for open source (approved) | set at deployment 17 through stdin; `secret list` names exactly it; the deploy guard's seventh check refuses the key in any `vars` block (Task 14's review made the rule mechanical) | none in code. R3's M-6 found a malformed list's 500 echoed the value to every caller; since fix wave A the refusal names the entry by its position (`YOKOHAMA_WEB_ADMINS entry 2 of 3 is empty`) and echoes nothing |
| **P-2** | the shell pins the module's clock before EVERY call that can reach the layer's clock and records the reading; any reply's plan becomes `archive_owed` | from Task 9; the receipt loop's re-dispatch and the stamp pin the RECORDED reading; battery 17 is its mutant | none. r17b showed the Archive object's pin is unobservable (§4.4) |
| **P-3** | `worker/` is its own npm package with its own lock | Task 7; CI keys the npm cache on both locks | two lock files |

### 5.3 Pre-flight (2026-09-23, before Task 1)

The scan (`preflight-scan.md`) read the plan whole against the spec and the code: **189 rows** (65
interface, 16 self, 16 rubric, 75 citation, 17 constraint), **49 not `none`**, every one ruled by id
before Task 1.

| ruling | what it rules | cost if wrong |
|---|---|---|
| the checkout | the work runs in THIS checkout on `web-room-m6.5c`, no worktree (a worktree only if Task 8 ran beside a Rust task; it did not) | none to the code |
| **I-21 + S-04** | the picker row: `Some(<list>)` when it grew, `Some("[]")` ONLY when a non-empty list was cleared, `None` otherwise; the shell deletes the row on `"[]"`; a row that is not the list's object is NO list and never a panic; `restore_picker` only when a record exists | one extra row deletion per cleared picker |
| **I-43 + I-44 + I-34** | EVERY shell time — `written_at`, alarm deadlines, retry dues, the marker's `at`, the expiry comparison — is the platform's `Date.now()`; the module's pinned reading feeds ONLY what the layer stamps; one exported `nextReading()`; one TTL reader; expiry cells backdate stored times | a cell rewritten |
| **S-12** | Task 12's cells read the revision from `meta` before each hold; `onRow` awaited; the third column sorted; a refused create writes nothing; battery 18 gets its pool cell | — |
| **I-22** | `house_frame(seq: u32, chat: u32)`; the factory's `.d.ts` re-exports wasm-bindgen's own types | a counter above 2^32 |
| **I-42** | a `house_copy` trap in the REQUEST path: the documents still commit, the slot stays, the alarm is armed at +30 s for a fresh instance, the instance is discarded, the reply stands — amending spec §2.4's classification (spec §10) | a lobby row up to 30 s stale after a trap |
| **I-39** | a cell that needs an applied move posts the MEASURED PlaceA sequence (`1`, `A09`, `confirm`) as the acting seat, and asserts non-empty bumps as its premise | — |
| **I-64** | the marker's `kept` is derived WITHOUT parsing the table document: from `events()` read before the delete, the switch, and the delete's plan outcome (refined by I-66 and I-71) | — |
| **I-30** | each class-adding task adds its class to BOTH configurations in ONE migration tag, edited in place until the first deploy and FROZEN after it | — |
| **I-28** | `shellConfig(env): ShellConfig \| { error }` is the contract | — |
| **I-29** | `init` runs inside `Instance.get()`; its throw is a 500 naming the variable, logged `init-refused`, NOT a load trap | — |
| **S-09** | Task 9's verbatim cells made to compile against the shared names | — |
| **R-09** | the nameless-alarm guard is a PURE function with a falsifiable patched-export cell | — |
| **R-10** | every "sends nothing" cell pairs with a POSITIVE control on the same socket | — |
| **R-13, R-06** | every placeholder cell written in full; `partial_of`, copied from a private test module, names its source | — |
| **I-25** | the module boundary is STRINGS; `r2.ts` names the TS `Plan` and `Outcome` | — |
| **I-54** | `resolveIdentity` returns a `Promise` (WebCrypto verifies asynchronously) | — |
| **I-07** (refined before Task 2) | `init` rebuilds every module-scope static the crate owns — but NOT the clock pin, whose only writer is `set_clock` | a cell expecting `init` to unfreeze the clock |
| **I-57** | the delete's batch clears `archive_owed` from the task that first builds it (Task 13) | — |
| **I-36 + I-48** | the house sets the same `ping → pong` auto-response and answers a non-upgrade `GET /api/stream` 426 with the table's sentence | — |
| **I-31** | the runner split: the pool `test/**/*.test.ts`; node `scripts/**/*.test.mjs` and `src/**/*.unit.test.ts` | — |
| **C-11 + C-13** | the orchestrator never edits code: the implementer commits before a deploy; a reading that needs a code change goes back to the resumed implementer as a fix commit | — |
| **I-17, I-56** | implementers read the crate at HEAD, not the plan's Consumes lists | — |
| **C-00** | every commit: the plan's subject, the red run quoted, the EXECUTING agent's own attribution | — |
| **S-01** | `archive_over_retry` at an open or dealing table answers `handle`'s own `Err`; "unfinished" means a playing game not over | — |
| **S-02, S-05, S-07, S-08, C-15** | wording and file lists within the allowed sets | — |
| **R-02, R-03, R-12, R-15** | cross-host duplication (the clock, the platform scan, `error_body`'s restatement, a replay driver per host) is the lane's established shape; each copy names its source | the restatements are cleanup-cycle debt (§6.2) |
| **Z-02, Z-17, Z-23–25, Z-36, Z-41, Z-42** | the plan's line numbers are approximate; implementers read the code | — |
| the CLI suite | NOT a per-edit gate this cycle — the CLI is fenced byte-identical | the CLI's goldens and `guide_sync` were not run; the empty fence is the evidence |

### 5.4 Taken during Tasks 1–15

| task | ruling | what it rules | cost |
|---|---|---|---|
| 1 | the lock stripped | `archive_payloads` posts the walk's table with `lock = None` (its salt is OS entropy, so a regenerated script was never byte-stable), with an anti-vacuity assert that the lock was there; a seeded lock would be a layer change for a test's convenience | the archive walk no longer exercises `check_shape`'s lock-present branch (the lib cells do) |
| 1 | a comment measured false | cell 2's comment claimed a catch it cannot make (the `is_on` mutant survives all three cells); rewritten to what the eviction really buys, the code unchanged | none |
| 1 | the amend | `c2daf4c6` was amended to `ab9c6812` before its review (comment-only, verified); the review continued on the code and was extended to the delta | none to the code; the STOP-on-amend rule went into every later dispatch |
| 1 | **I2** | a false sentence in a new cell's doc is fixed even where the plan gave it verbatim: a sentence a task writes about itself must be true | — |
| 1 | **I3** | `archive_over_retry`'s `Ok(())` means the retry RAN; the HOST reads `archived_at` (refined at Task 6) | one extra table read per stamp |
| 1 | **M1–M4, M8, E4, E1** | comment truth; M8 measured (the 200-move pump cap, then 157 moves to game over after an eviction); `archive_over_retry` named as `archive_if_over`'s third caller; the amend accepted | — |
| 2 | two deviations | an added cell for the house kind (it catches a mutant the brief's cells let survive); the brief's citations corrected to HEAD | none |
| 2 | **E2**, **E3** (→ Task 7) | the shell trims and ASCII-lowercases `YOKOHAMA_WEB_ARCHIVE`; it passes each variable as a string or `null` and refuses a non-string naming the variable, so serde's nameless refusal is unreachable | none |
| 3 | caller first | `create` dispatches an identity-less body UNCHANGED so the layer's own 401 answers before any host-written refusal; pinned by two two-defect probes | only the sentence a caller-less request reads — fail-closed either way |
| 3 | **I-3** | the brief's "nothing stored" read only the object's name; the whole-store listing with an anti-vacuity partner replaces it | none |
| 3 | **M-5** | from Task 4 on, each task's cells live in its own file's test module, not `lib.rs` | a scan reading a test module (checked) |
| 3 | a plan sentence measured false | Review Focus 1's "`curl --data @file` produces a trailing newline" is false — `--data` strips CR and LF; `--data-binary` and most editors keep it (spec §10) | — |
| 4 | four deviations | `Lines \| Dropped`; a cleared restore closes the picker (`repl.sub = None`), so a stale click is the layer's 409; the closure answers the picker; four extra cells | — |
| 4 | **I-1** | a picker row is read only as an OBJECT (a positional array deserialized through serde's `visit_seq`) | — |
| 4 | the wake contract | `restore_picker` once per new instance, after `load`; a non-200, non-trap restore is LOGGED, the row left, and the request goes on to its own dispatch | — |
| 5 | concern 2 | `house_copy` checks the stored ORIGINAL first, so a document the NAS would drop is never listed | one extra `check_shape` per copy |
| 5 | the seam | `house_copy_with(id, scrub)` makes the copy-side check falsifiable with a defective scrub | ten lines of test plumbing |
| 5 | **M2**; escalation 1 | the wrapped refusal sentence accepted; a refused copy leaves the lobby listing the LAST copy it received — accepted and documented, the operator's force the path | a stale row for a corrupted store only |
| 6 | **I3 refined** | `stamp_archive` answers `{"stamped","over"}`; the shell clears on `stamped`, clears on `!stamped && !over` (the undo), retries on `!stamped && over`; a `conflict` receipt clears regardless | one boolean |
| 6 | four deviations | `init` normalizes the `r2` caller through `User::parse`; a caller-less browser save refused (fail-closed); `expire` with no readable creator answers the layer's own refusal; three extra cells | — |
| 6 | **I1** + spec §4 | `expire`'s resident-handle 500 and `root()`'s three refusal details join spec §4's host-written list (spec §10) | a sentence on a list |
| 6 | escalation **e** | a REFINISH can strand a one-person table on this host and never on the NAS (§6.3 item 2): RECORDED, no mechanism this cycle | an operator force for a rare table |
| 6 | note f | spec §2.6's "one clock reading per plan" overgeneralizes (spec §10) | — |
| 7 | deviations | the build commands in `scripts/build-module.mjs`; small additions with cells; wasm-bindgen's `_bg.wasm.d.ts` deleted (measured: it mistyped the import); an absent `preview_urls` keeps an existing setting (measured) | — |
| 7 | **I1**, **I2** | a refused assembly removes its output, and the guard runs the debug-module check; the factory is written AFTER `wasm-opt`; an opt-out build leaves `gen/.unoptimized`, which the guard refuses | one marker file |
| 7 | the release smoke | `scripts/smoke-release.mjs` instantiates the release module and answers `/api/health` 200 before any deploy (Task 11) | — |
| 8 | four deviations | `setLink("reconnecting")` at an unexplained close; CLAUDE.md's "no timer while connected" now holds for the stream arm only; the socket arm shows the generic words for an admission refusal (spec §2.7's trade); Chromium's CONNECTING-close line carried to Task 15 | — |
| 8 | escalation 2 (→ Task 10) | the ADMISSION `end` frame carries `{"reason":"gone","sentence":…}`; the mid-stream `end` stays byte-identical to the SSE's | one field in a frame no SSE emits |
| 9 | a FINDING | the pool CAN deliver a nameless alarm; Ruling R-09's pair kept (spec §10's Pool fact 8) | — |
| 9 | deviations | `wake` returns `BatchParts \| Response`; `commit` takes no id; a module drop on ANY non-trap throw (measured: an object outlives a throw out of `fetch`); a trap while BUILDING the module is not a load trap | — |
| 9 | **⚠️1 / M1** | the SPEC wins over the brief: a successful load resets the count even when the call traps — one batch holding the two deletes | one tiny batch on a rare path |
| 9 | escalations 3, 4 (→ Task 11) | the guard compares the two configurations' object and migration blocks; a name over 64 bytes routes to the house, whose `dispatch` answers the layer's own refusal | garbage probes reach the house object |
| 10 | the socket TAG | the spec wins: the tag is the table id (the plan's `"t"` would have hidden every socket from a spec-written `getWebSockets(id)`) | none |
| 10 | HEAD on `…/events` | keep 405, the spec's letter; axum's `get` route answers HEAD 200 empty, so the spec's "axum's router's answer" is inexact for HEAD (spec §10) | nothing observable |
| 10 | escalation 1 | a framing trap's frames are retried by the NEXT BUMP, parity with the SSE's comment arm; no "frames owed" flag | frames lost to a trap until the next bump |
| 11 | concerns | a create BODY's over-long `id` routes to the house too; two trap sentences join §4; workerd sends a non-ASCII header value as UTF-8, so `forwardable` decides; the reading script sends the credential to the Worker's own origin only | — |
| 11 | **I1** | `forwardable` runs on the RAW claim before any trim, and lowercasing is ASCII-only — stricter than the brief's verbatim code, equal to the NAS | none |
| 11 | **I2** | the source fence reads through TypeScript's own parser (a `/*` inside a line comment had swallowed `table.ts:185-252`, so a planted call there was missed) | — |
| 11 | escalation 1 | **the Origin check**: an upgrade whose `Origin` is present and foreign is 403 `this socket is for the site's own pages`, unupgraded; an absent `Origin` passes. A WebSocket is outside CORS, so a hostile page in a signed-in player's browser could otherwise read a table's turn and timing frames. The owner may reverse it | a legitimate cross-origin client, of which none exists |
| 11 | escalation 3 (→ Task 12) | the HOUSE has no poison count: at its chat document's second consecutive load trap it DELETES the row, logs `house-chat-dropped` and serves on | a lost lobby chat history |
| 11 | the zone's challenge | a reading that differs from the spike in FORM, measured on both hosts; RECORDED, not a spec departure | none to protection |
| 11 | the beacon | the cause was OURS — the harness's context-wide headers — and is fixed by configuration; the zone unchanged (then refined for Playwright by I-75) | — |
| 12 | the late-ack guard | `hasTables` alone, not "AND a slot or a marker": the extra clause made two batteries' catchers vacuous | an ack writing into a not-yet-deleted object — the design |
| 12 | `/api/art` | a payload that DIFFERED on this host (the raw manifest had two generator keys more): served projected onto the nine declared maps; host-local; spec §10 | a JSON parse of ~10 KB per `/api/art` |
| 12 | the I-42 adaptation | the copy is computed on a scratch module instance of its own, so a trap never empties the module the frames read — accepted with its cost written (18 pages, ~0.15 ms, dropped after) | one short-lived instance per copy |
| 12 | **M1** | `copy_owed`: an owed copy is recomputed while owed, retried on the ladder after an alarm-path trap, so spec §2.4's "retries on the alarm or the next request" holds | one meta row on a repeating-trap path |
| 12 | **M2**, **M3**, **M6** | a non-trap throw building the scratch instance counts as a trap; one retry-rung convention; failed pushes made on the table's side, so the pool's output is pristine | — |
| 12 | **M4** (plan-mandated interim) | a same-second re-create could lose the generation's tie; Task 14's 409 narrowed it, and F-2 closed it at the whole-branch review | — |
| 13 | the off-switch drop | `archive-owed-dropped` is logged at the abandon-while-off site too, not only at expiry | — |
| 13 | **I1**, **I2** | the line and its cell; the host's own `failed` detail `<record_key> refused a create as present, then read as absent` named and added to §4's list | — |
| 13 | **m3** | M33–M36 survive, each a retry at most one rung (an hour) late and never lost; a catcher would be fragile — ACCEPTED, recorded | a late retry |
| 13 | escalations E1, E3 | a second consequence of the refinish race (§6.3 item 2); every request retries an owed save while the switch is on, so an R2 outage costs one attempt and one failure batch per request (§7) | — |
| 14 | **I-65** | an expiry's `created_by` is read off the table's HOUSE COPY (the shell's own parsed JSON); an abandon's viewer is the DELETE's caller | a copy the scrub refuses leaves an expiry without a viewer |
| 14 | **I-66** | a `status: "over"` reading counts as a game for `kept` | none observed |
| 14 | **I-67** | the `expiry_failed` meta key drives the 30 s / 2 min / 10 min / hourly backoff (spec §10's key list) | — |
| 14 | **I-68** | three delivery cells run with the TTL reader patched to null — under a TTL every alarm re-arms to everything still due | none |
| 14 | **I-69**, **I-70** | a TTL of 1 reads "after 1 day"; a FORCE logs `archive-owed-dropped` for a save it drops, whatever the switch | — |
| 14 | the deploy note | a table created before the seventeenth deployment carries no expiry alarm until touched; the listing at deployment 17 showed none | — |
| 14 | **I-71** | `kept` reads `archive` whenever the house copy carries `archived_at`, checked BEFORE the switch — the stamp is true of the bucket whatever the switch reads | a marker saying "not kept" over a kept game, for 90 days (avoided) |
| 14 | **I-72** | `?force` is the BARE key; `force` with any value (`false` among them) is a 400 naming the switch, never a force and never a plain expire | one more host-written sentence |
| 14 | **I-73** | the abandon's `deleting` reads the house copy first, so a document whose copy traps answers the trap 500 — accepted: the copy and the layer's delete share ONE `check_shape`, so neither path accepts such a document | a creator reads `trap` in place of the layer's refusal |
| 14 | **I-74** | `GET /api/admin/tables?anything` is unrouted: no query is silently ignored on the operator surface | — |
| 15 | **I-75** | SUPERSEDES Task 11's "per-origin route-based injection, no allow-list" for the Playwright tier ONLY: Playwright's request routing does not reach a page-opened WebSocket upgrade, so the CLAIMED header goes context-wide; the CREDENTIAL never does — each live context takes Access's `CF_Authorization` cookie; and because the claimed header rides the beacon's cross-origin request, the beacon's CORS error recurs and the live harness allow-lists EXACTLY it. Stubbing the beacon was considered and not taken | an unmatched console error would show it |
| 15 | **I-76** | six brief-vs-code points: `liveContextOptions` empty locally; the abandon under LIVE only; one blocked beacon load logs TWO errors and both are forgiven, each tied to the beacon's prefix; the cookie fetched by Node and **tracing OFF under LIVE** (a trace records the secret and the cookie); WebSocket opens counted as the stream count; a bounded lobby re-read | a failing live run has no trace |
| 15 | **I-77** | cleanup in `test.afterEach`, not the brief's `finally` — measured: a timed-out test's `finally` ran after the page closed | — |
| 15 | **I-78** | `testIgnore` for `*.node.test.ts` in the Playwright config (the node cells had made the local run report 0 tests) | — |
| 15 | **I-79** | every live `request.*` goes through `liveRequest`, which rethrows the first line alone: Playwright prints every request header of a thrown API call, the cookie included, to the reporter and to `test-results/` | an Access session on the terminal and on disk after one transport failure (avoided) |
| 15 | **I-80** | two cells for the replay's SECOND run (a message at `seq == base` dropped; a 200 at the wrong leaf refused) | — |
| 15 | fix round 1's LOWs | L-1…L-8 and W-3 fixed; L-9, W-1, W-2 deferred to the whole-branch triage; W-4 recorded (what a live run leaves) | — |
| 15 | **I-81** | the live run's three findings: no stream before the transport is known; no keep-alive into a closing socket; the deal's designed 409 forgiven EXACTLY in both modes — WIDENING the cycle's `web/` allow-list to two of Task 8's client files, because the live tier exists to find exactly these | one more deploy |
| 15 | **I-82** | the owner's 80,000 line: after fix round 2 and deployment 18 only the CHEAP live steps ran that UTC day (`joins_by_link`, the replay twice); `two_browsers` live waits for a fresh day | the ping fix's live witness waited two days; the re-run ran on 2026-09-26 and PASSED (§3.2) |
| 15 | two process rules | an acceptance reply that asks for changes after a DONE is a fix-round dispatch ("as a new commit"); a reviewer's mutants run on an export | — |

### 5.5 Task 16 — the whole-branch review, the fix wave, its follow-up and the exit

| ruling | what it rules | cost |
|---|---|---|
| **I-83** | the spec §5.7 battery runs at `d025ba43`, as a review instrument in parallel with the area reviews, so its survivors join the ONE fix wave; afterwards only the rows whose catchers the wave touched need a re-run at the fix wave's head (R2 re-ran all 39) | none in the event: the whole battery stands at `e20c8ff8` |
| **I-84** | spec §2.4 defines `meta.wakes` and §5.5 wants it "rising across pauses", but no reading path existed: the shell writes `wake {id, wakes}` when a counted wake's batch commits, with a pool cell under the switch and without it | one log line per counted wake, in the exit's window only |
| **I-85** | R1's M-1: a stored picker row carries the CODE IDENTITY of the sources that give a line its meaning, computed by the worker crate's `build.rs`; `restore_picker` CLEARS a row stamped with other code and applies nothing, logged `picker-dropped {id, stored, running}`; the doc sentences and spec §2.4's "today's behaviour after a restart" corrected (spec §10) | every open picker at a deploy that changed those sources starts over — the safe loss |
| R1's LOWs | **L-1** fail-closed: a browser save whose path names another caller than the object's is refused; **L-2** a dev-door claimed value must be an ADDRESS by `User::parse`'s rule, else no one (the layer's 401, not a 500 at an Archive object); **L-3…L-9** a docs and test-minor sweep in the crate, `init`'s positional array REFUSED; the cleanup-cycle debts to the M6.5c2 handover | Task 1's M6 cell not written: its home is the fenced `games.rs` |
| **I-86** | R3's H-2, a NAS behaviour change fix round 2 made and its re-review misjudged: the transport wait is BOUNDED by App's own `BOUND_MS`; the word reads "reconnecting…" while it runs and is never "live" before a connection; after the bound the remembered or default transport is used, and a retry issues a NEW health read once the pending one is older than the bound | on the Worker one 426 per rung while health hangs; on both hosts a first visit reads "reconnecting…" until the first connection (§7) |
| **I-87** | R3's H-1 and R1's E-1: deployments 14–18 (and 19) built the module with `wasm-opt` 133, the workstation's binaryen, where spec §6 says 132 and CI pins 132; the spec's sentence is corrected in its dated §10, RUNBOOK §1 and the workflow record 133 as the workstation's reading and 132 as CI's (CI builds, never deploys), and the close's CI commit `a9c8be27` pinned CI to 133; the Dockerfile's NAS image stays at 132 for the cleanup cycle. No code change | until `a9c8be27`, CI could not reproduce the deployed bytes; the NAS image's pin still differs |
| R3's dispositions | **M-1** the replay prints its reset recipe and stops on a first send's 409; **M-2** a no-poll cell on the socket arm; **M-3** the deploy guard's two freshness checks; **M-4** CLAUDE.md's gates (this close); **M-5** RUNBOOK §6.7 for the live tiers, the quota guard, what a live run leaves and the reset, with §6.4 and §6.8 corrected forward to I-75; **M-6** the operator list's refusals name an entry by POSITION; the three flakes each by its cheapest fix; the LOWs fixed where a sentence was false, else this close's docs pass | — |
| F-1…F-4 | **F-1** a host-written JSON 500 `the server could not answer — try again` and an `object-failed {target, path, error}` line when an object's `fetch` rejects; **F-2** a generation's `at` is `Date.now()` in MILLISECONDS, the shell's own time base (I-43, I-44); **F-3** a token without a string `kid` is no one; **F-4**, the rows-read cost, to the cleanup cycle | F-2 moves no payload: no committed file carries a generation |
| the fix wave's shape | ONE wave in TWO sequential dispatches on disjoint files — A (the crate, `worker/src`, `worker/scripts`, the RUNBOOK, `.github`), then B (`web/src`, `web/e2e`) — never two implementers in the tree; one scoped re-review per area | wall-clock only |
| **I-88** | the exit instrument, numbered with its cost: a SHORT, PACED headed game with a watcher, abandoned at its end, NOT the whole game spec §5.5 words; the whole live game is live run 1's, and the console-clean whole-game witness is the deferred `two_browsers` re-run (§3.3) | a whole game is not re-played headed this session; the owner's 07:00 direction accepts the short test plus the stated ending state, and the quota line is the reason |
| **I-89** | a picker row written BEFORE the code stamp (deployments 14–18) is CLEARED at its first restore under the stamped code, since no code vouches for its meaning; and a ROLLBACK from the fix wave's deployment to the eighteenth or earlier is a GENERATION-ORDER rollback — seconds generations order before every retired milliseconds one, so a re-created table goes unlisted for 90 days — written into RUNBOOK §6.6 with the open-picker drop | every open picker at deployment 19 dropped; a rollback across the fix wave has a cost the RUNBOOK names |
| **I-90** | R1's R-1, N-1 and N-2 go to ONE follow-up commit by wave A's implementer, AFTER R2 releases the tree, "as a new commit", with R1 re-checking the digest cell | — |
| **I-91** | the follow-up carries R1's R-1 (the engine a root of the picker's code), N-1, N-2 (`picker-dropped` after the batch), R3's R-N1 (the bridge in the freshness digest, "five crates"), R-N2 (any `.request` access outside `liveRequest` refused), R-N4 and R-N5 (the RUNBOOK's window steps and the `date`-form query), R-N6 (the exit spec's visibility precondition and the two Chromium switches); **R-N3 RECORDED** for this close — a "connecting" word is an M6.5c2 frontend tweak | every open picker at a deploy that changed the ENGINE starts over too |
| the exit's windows | after run 1 met a covered, hidden headed window on macOS whatever Chromium's switches read, the spec places its three windows side by side through the browser protocol (`f2c27a3f`); after run 2 met Chromium's 500 px minimum window width, a 2×2 grid under the menu-bar inset, every placement read back, and visibility counted from the start of play (`754b80ae`) — both spec-only commits by wave B's implementer; run 3 then PASSED | none to the product |
| R-N7 | R3's re-check at `bd8ce497`: a request context MADE from an API context escaped the rule; it is refused inside `liveRequest` or out, `browser.newContext()` allowed — `528b1285` | — |

---

## 6. The LEAVE list, the owner items, and the amendments

### 6.1 Inherited and not touched

Spec §6's inherited list stands: the whole-game gate's vs-bot arm LIVELOCKS on some OS-entropy
seeds (it fired twice this cycle — at Task 3 and in Task 9's combined run, each passing alone in
about 24 s at the same commit); the e2e's `CAP = 4000` (hit twice at Task 8 by a 4,001-move game
with its end triggers unmet, and passed solo at 2,669 moves); `walk.json`'s 419 KB pretty-printed
diff; "who starts" on the local form; nothing in CI building the self-host image (not rebuilt this
cycle — the workspace gained a member, and `COPY crates/` with `-p` targets suggests the image still
builds, unverified; R3's E-3); and the `npm test` order-dependent flake, which
this cycle named as three and fixed (§6.3 item 6). The layer's positional-array quirk in its own `create`, its mute `eprintln!` and the
NAS's missing TTL are the cleanup cycle's by the execution handover's list. Spec §7's carried items
stand: the clean-browser single-audience reading, the "Lobby" controls under a single-player build,
the no-Web-Locks fallback and background suspension, R-F6's Access re-login in a browser the worker
controls, and the PR #37 fix wave's residuals.

### 6.2 This cycle's own — deferred, each with its reason

Most are design debts rather than defects, and the M6.5c2 handover carries them as its list.

- **The rows-read cost (F-4).** `readDocs` reads all three documents — the record's whole text — up
  to three times per request (the wake on a fresh module, `deleting`, `commit`'s comparison); ~470
  rows read per move live. Read once per request and thread the result.
- **`worker/src/table.ts` is 2,082 lines holding four separable concerns** (Task 10's M5, Task 12's
  M9, Task 13's m4): the transport helpers beside `frames.ts`; the slot half (~330 lines) as a
  `delivery.ts`; the archive half (~380 lines) behind a narrow host interface; and three copies of
  the guarded-call pattern.
- **R1's cleanup debts:** the picker invariant is not structural (`events_inner`,
  `stamp_archive_inner` and `expire_inner`'s no-creator arm reach `Games::handle` outside
  `picker::around`, safe only because one instance holds one table); the scrub's compile-time fence
  is one level deep (`TableDoc`, `Lock` — not `SeatDoc::Human` or `TurnMirror`);
  `ReceiptArchive.pending` keeps only the LAST deferred plan; the restatements (`error_body`,
  `MAX_WAIT_SECONDS`, the house frame, the clock, the platform scan), each pinned; the layer's mute
  `eprintln!` (a logging seam would let `archive_over_retry` report a failed stamp); the worker
  module has no ONE place for its figures (`copyOf`'s doc carries release readings that growth will
  make false silently, the M6.5b2 pattern); and the positional-array quirk in the LAYER's `create`
  (the module's own `init` now refuses the array form).
- **R3's cleanup debts:** `.dev.vars` parsed three times (`live.ts`, `live-replay.mjs`, RUNBOOK
  §6.4's recipe); two origin constants; the replay's five parts in one 649-line script; rules stated
  twice (`HOST_FIELDS`, `order()`, the credential's key names); two parsers of the one precache list;
  the RUNBOOK at ~3,150 lines covering two hosts; the dev door ON in production's vars; the live
  tier's cost.
- **R3's LOWs left open after the wave**, none a false sentence: `read-deploy.mjs` still echoes up to
  200 characters of a non-200 `/sw.js` body (L-3); the POOL's host-field exemption can still be spent
  on an absent field — the presence rule went into the live replay only (L-5); after the first 404 an
  outbox drain or sweep goes on posting the remaining rows (L-6, Task 8's M3 too); `dealAnswers` is
  uncapped, as ruled (L-7); a live create whose reply was never read leaves a table in the shared
  lobby, the exit spec sharing that window (L-9); and the client id's place, the owner's call (L-10).
- **R-N3, the first connection's word**: `Link` has no "connecting", so a FIRST connection reads
  "reconnecting…" (recorded; a wording change for M6.5c2's frontend tweaks).
- **R-N7's other escape**: a computed key held in a variable (`page[k].get(…)`) still passes the
  structural rule — contrived, and left.
- **Task 8's deferred minors and escalations:** a probe answer landing after the attempt was aborted
  is still handed over; battery 37 never reads the link word while the tab is hidden; two fake
  sockets (`signal.test.ts`, `ws.test.ts`); the two self-posting sites (`App.tsx`, `Abandon.tsx`) do
  not remember "off" on a 404 (spec §2.6 scoped one outbox change); the probe is unbounded, as the
  stream arm's open fetch is; the Vite dev proxy has no `ws: true`.
- **Task 1's M6 and M7**: no cell on `archive_over_retry`'s second `NotFound` arm (its home is the
  fenced `games.rs`, and the arm is unreachable from the worker crate); and the new compare loop
  restating `walk.json`'s block — R1's call: leave it, two sites in the file's established shape.
- **Task 14's L-8** (a one-row existence query in `deleting`), untouched by the wave; and its L-4, a
  transitional `forced` line without an `operator` line at the acknowledgement of a pre-Task-14
  removal — none was seen at deployment 17.
- **An observation of this close's:** the operator listing mixes time units — `created_at` is the
  layer's seconds and `last_active` the shell's milliseconds (deployment 17's reading: `1790224338`
  beside `1790224338723`). Both are correct for their source; one payload carrying two units is worth
  one line in the cleanup cycle.
- **Pre-existing and fenced:** `cargo clippy --target wasm32-unknown-unknown` over the worker's graph
  fails inside the CLI (`clippy::drop_non_drop`, `crates/yokohama-cli/src/session.rs:439`); no gate
  runs clippy for that target (R1's E-2).

### 6.3 OWNER ITEMS

1. **The `test/` prefix's lifecycle.** This cycle's live saves REMAIN in the bucket (the owner,
   2026-09-24). The choice — a periodic script, or an R2 object lifecycle rule expiring everything
   under `test/` after N days — is the M6.5c2 handover's, with its recommendation.
2. **The REFINISH consequence** (Task 6's escalation e; Task 13's E1). On this host, and never on
   the NAS, a one-person table can be stranded: an undo DURING the game-over upload's I/O clears
   `archive_owed` with `archived_at` unset while `over/<id>` already holds the first completion; a
   replay to a different end then puts as a `Conflict`, so the creator's Abandon is refused for good
   and expiry goes `held` until an operator's `?force`. Two uploads of DIFFERENT records for one key
   can also overlap and leave the first completion's record beside the second's table. The NAS holds
   its lock across the put and the stamp. The window is tens of milliseconds on a one-person table.
   A stamp-on-conflict-after-first-completion rule is a SPEC decision, not a host fix.
3. **The phones** — the iPhone exit as R-6.5b2.17 words it, and the Android handset as the second
   reading, now against the Worker; recorded, not gated. not taken this session; the owner's, recorded not gated.
4. **The PR merge order.** Eight pull requests, a stack, nothing merged: `web-room` #31 against
   master, then #32 … #37, and this cycle's #{{TBD: number}} against `web-room-m6.5b2`.
5. **The NAS-era RUNBOOK §3 spells the owner's address** as an Access-policy identity in FIVE lines
   committed in earlier cycles — at `bd8ce497`, lines 572 and 581 (§3.7), 658 (§3.8), and 1413 and
   1417 (§3.14's Step 4b). P-1 held for everything this cycle wrote; whether §3 is scrubbed before the
   repository goes public is the owner's.
6. **The jsdom flakes are FIXED** — spec §6's "the `npm test` order-dependent flake" was three, now
   named and closed at `e20c8ff8`: a `Surfaces.test.tsx` cell that was a BUDGET (split into its three
   cases, one render each); a `Lobby.test.tsx` cell that raced a static heading (it now awaits the
   listing's link); and an `App.test.tsx` ORDER dependence (a `resetFactsForTest()` in its describe's
   `beforeEach`; 116/116 under the shuffle seeds 22 and 44 that failed it). `npm test` read 1056/56 in
   order twice, shuffled, and under three concurrent suites.
7. **Pool fact 8, narrowed.** The pool CAN deliver a nameless alarm (Task 9, measured), and
   `ctx.id.name` inside a PRODUCTION `alarm()` is still unread: no alarm fired at deployments 14–17,
   and neither deployment 18's tail nor the exit's names an alarm line. The rule rests on Cloudflare's
   changelog of 2026-03-15, the source fence against `idFromString` and `newUniqueId`, and the logged
   guard.
8. **The `wasm-opt` pins** (I-87): 133 on the workstation that deploys and, since the close's CI commit
   `a9c8be27`, in CI; 132 in the Dockerfile's NAS image. RUNBOOK §1 and the workflow say so; the cleanup
   cycle aligns the Dockerfile.
9. **What the picker's code stamp costs** (I-85, I-89, I-91). A deploy that changes the ENGINE's, the
   CLI's or the layer's sources drops every open picker on the Worker (the player re-opens it;
   `picker-dropped` in the log) — before the stamp, such a deploy could have applied a move nobody
   clicked, unreachable only because those sources stayed byte-identical all cycle. Deployment 19,
   the first with the stamp, drops every picker row stored before it at its table's next request,
   by design (`stored: null`); the ledger's readings name no such row.
10. **A rollback across the fix wave is a GENERATION-ORDER rollback** (I-89; RUNBOOK §6.6). Rolling
    back to the eighteenth deployment or earlier draws generations in SECONDS, which order before
    every retired MILLISECONDS one: a table forced or re-created under the rolled-back code goes
    unlisted until the retired generation is pruned, 90 days after its retirement, while the table
    itself plays by its link; and the rollback drops every open picker the newer code stored, whose
    `code` key the older module's strict read does not know. Rolling forward again needs nothing.
11. **The first connection's word** (R-N3). On a browser with no remembered health answer — a first
    visit, cleared storage, or a storage that refuses — every server-signal screen, on BOTH hosts, now
    reads "reconnecting…" from its first open until it connects: one health round trip, at most 5 s
    when health hangs. A returning visitor sees no change. A "connecting" word is a frontend tweak for
    M6.5c2.
12. **The Origin check** (Task 11) refuses a WebSocket upgrade from another site's page. It is a
    ruling the owner can reverse.
13. **`DEV_DOOR_CLIENT_ID` is committed** in `worker/wrangler.jsonc` — the credential's public half,
    by the spec's design; a `wrangler secret put`, as P-1 did for the operator list, would keep it out
    of an open-source repository at no code cost (R3's L-10).
14. **The dev door is ON in production's vars.** It must go off when the public door goes live
    (R-6.5c.3); a guard check could require it then, or at least print it.
15. **R3's E-3**: whether the cleanup cycle rebuilds and re-measures the self-host image, now that
    the workspace has a member it never built.

**An owner note, not an item**: the WARP client's device login EXPIRED during the two-day stop —
the client read Connected while every request without the credential answered Access's 302 "Please
authenticate via the warp client" — and the owner re-authorized it on the 26th. After a long stop,
re-authenticate WARP before any operator reading: the operator is a person over WARP, and the
credential never is one.

### 6.4 Amendments to the frozen spec and plan (recorded forward, never edited there)

The spec gains a dated **§10, "Corrections recorded in execution (2026-09-24)"**, in this close's
docs commit: every ledger amendment, one row each, with the sentence as the spec reads it, what
execution found, the sentence as ruled, and the ruling — the art bytes, the Origin check, the
raw-address and dev-door address rules, every variable's 500, the object-failed 500, the host-written
class route by route (the fix wave's `object-failed`, L-1, L-3 and positional sentences among it),
the meta keys and the `wake` line, the time base, `house_copy`'s classification, the admission `end`,
the picker's code identity, the marker's `kept`, the expiry's ladder, `?force`, the generation's
milliseconds and its rollback, the clock's scope, the transport wait and the first connection's word,
the live tier's identity design, the exit as I-88 rules it, the `wasm-opt` version, the flakes, and
the plan's own sentences (the plan carries no dated section of its own after its two revisions):
Review Focus 1's curl sentence, Pool fact 8, the 60,000 line, the binaryen pin, `Generation.at`, the
socket tag, the widened `web/` list, the `afterEach` cleanup, and Task 16's "whole game".

---

## 7. What this cycle does NOT claim

Spec §6 is the starting point; each sentence below is it corrected by what execution measured, plus
what the reviews, the deployments, the live runs and the exit added.

**About the door, identity and Access**

- **Nothing is public** (R-6.5c.1). The Worker is behind the hostname-wide `yokohama` application.
  The zero-invocation single-player door is a PROPERTY of the asset configuration, read off the tail
  at deployment 14 (zero Worker invocations for a real file, one for a missing one), not a public
  service; the path-scoped Access door is built and proven locally and not deployed; the card art is
  not published; and on a public door a scanner's misses are Worker invocations.
- **`claimed` is not retired.** On the Worker it is the dev door: the `yokohama-worker` credential
  with `YOKOHAMA_DEV_DOOR=on` may claim any identity at any table, the owner's included — never an
  operator's rights (battery 32, read LIVE at deployment 17) — and since the fix wave only an ADDRESS
  by `User::parse`'s rule. The NAS stays on `claimed`.
- **The raw-address rule was not read live.** An address outside tab, space and visible ASCII — the
  Kelvin sign, a leading no-break space — reads as no one; no Access identity with such an address
  exists to try it with, so the node and pool cells are its evidence. Nor was the operator gate's
  refusal of an unlisted PERSON read live (only the credential's claim, which the gate refuses
  whatever it claims).
- **The zone answers curl before Access.** From outside WARP a curl-shaped request meets Bot Fight
  Mode's 403 challenge, and a browser-shaped one Access's 302; nothing is served either way, on this
  host and on the NAS alike. The owner's own workstation meets Access's 302 whenever its WARP
  device login has expired, as it had after the two-day stop, until the owner re-authorizes it.

**About the platform, the guarantees this host rests on**

- **The frame-after-write property rests on the output gate's documented sentence**; battery row 4
  is RECORDED as uncatchable, and no cell can observe a WebSocket send held.
- **The alarm's atomicity with its batch rests on write coalescing**; the pool's abort cell reads
  workerd's local behaviour, and no cell can read a production crash.
- **CPU.** The Worker's own CPU WITH JWT verification is MEASURED at 0–2 ms (1 ms typical) — inside
  Free's 10 ms. An object's is MEASURED at the exit: 2 ms a move (the median over 40 moves; the maximum
  15 ms) and 6–9 ms a wake. The 30 s assumption was never approached, and no `exceededCpu` was seen
  live; a strict replay of a large untrusted record, the heaviest call an object makes, was not timed
  live.
- **No alarm fired live at deployments 14–17**, by design, and deployment 18's tail names none; so a
  retry's own delivery, a save owed across an eviction and `ctx.id.name` inside a production
  `alarm()` are pool-read only; the exit's tail names no alarm line either.
- **Production R2's refusal of a create-if-absent on a present key was not read.** Its `Headers`
  form was read only as ACCEPTED (deployment 16); the refusal and the re-read that follows it are
  proven against miniflare's R2 by two race cells.
- **Hibernation's wake cost grows with the record**: 19–28 ms at 595 events in the spike, and 6–9 ms
  of CPU (16–60 ms wall) for the exit's 40-move table. A wake on a record of thousands of events — the
  e2e's games reach 2,862 moves — was not measured.

**About what execution found — the claims as they now stand**

- **The beacon.** The spike's CORS error was its HARNESS's: context-wide custom headers on the
  cross-origin beacon request. A real browser — WARP, or the Access cookie — logs nothing. The LIVE
  PLAYWRIGHT tier nevertheless forgives EXACTLY the beacon's two-line CORS pair, because its claimed
  header rides that request (I-75), and EXACTLY the deal's designed 409. "No console error" in the
  live tier means no UNKNOWN console error; the forgiven ones are counted, by shape and by count.
- **The transport wait, and what both hosts now show** (I-81, I-86). The client waits for the
  deployment's transport before choosing an arm, the wait bounded by `BOUND_MS` (5 s) from the
  health read's asking; the word reads "reconnecting…" while it runs, and after the bound the
  remembered or default transport is used. **The consequence, on BOTH hosts**: a browser with no
  remembered health answer — a first visit, cleared storage, or a storage that refuses — shows
  "reconnecting…" on every server-signal screen until the first connection: one health round trip,
  at most 5 s when health hangs; on the NAS a hung health read delays its stream by the bound at
  most, and on the Worker it costs one 426 per rung. A returning visitor with a stored answer sees
  no change. And the word on a FIRST connection is "reconnecting…", since `Link` has no
  "connecting" (R-N3, recorded). **Deployment 18 carried the UNBOUNDED wait** (R3's H-2: no stream
  at all while the word read "live", when health hangs); deployment 19 carries the bounded one. The
  NAS received neither client: it runs its own build.
- **The same-second generation tie** (F-2). At deployments 15–18 a generation's `at` was the pinned
  reading in SECONDS, so a force then a re-create of one id within the same second left the new
  table unlisted for 90 days about half the time — one operator script away. From deployment 19 it
  is `Date.now()` in milliseconds (5–12 ms apart in 20 pool sequences). A generation drawn before
  keeps its seconds `at`, which every milliseconds one orders after — the safe direction; **a
  rollback to deployment 18 or earlier is a GENERATION-ORDER rollback** (I-89; RUNBOOK §6.6): a table
  re-created under the rolled-back code goes unlisted until the retired generation is pruned, 90 days
  after its retirement.
- **The picker's code identity** (I-85, I-89, I-91). Deployments 14–18 re-fed a stored picker line
  with no code identity, safe only because the engine, the CLI and the layer were byte-identical to
  the base at every one of them. From deployment 19 a row carries the digest of those three crates'
  sources; a row with another code, or with none, is cleared with nothing applied and logged
  `picker-dropped` after the request's batch. So every deploy that changes those sources drops the
  open pickers, and a rollback across the fix wave drops the ones the newer code stored.
- **Requests bind first, rows read second.** ~13.4 requests per move in the three-page harness
  (100,000 a day ≈ 7,500 such moves) and ~470 rows read per move (~1.34 M for one 2,862-move game,
  against Free's 5 M a day ≈ 10,600 moves); this cycle reduced neither (F-4 for the reads). Requests
  were ~13.4 per move in that harness, which refetches on every frame on three pages and probes once per
  move itself (12.8 a move in the 2,510-move re-run); the exit's paced game cost about 35 a move
  with three pages and chat both ways. A two-person game without the harness's probes costs less, and
  was not measured on its own. When Free's day IS spent, Cloudflare answers every Worker invocation
  429 (its error 1027) — `/api/*` and the missing-file route — while the static release and single
  player keep working, because the assets are served without an invocation.
- **A console-clean whole game is witnessed once.** The re-run on deployment 19 played 2,510 moves
  with no unknown console error; one game on one machine's Chromium is the evidence, and an edge-side
  socket drop mid-game would still fail the walk by design (R3's E-5).
- **Hibernation is witnessed in one table.** The exit's run 3 saw its object hibernate in each of four
  60 s pauses and wake on the next move, with chat and a watcher; runs 1 and 2 stopped at the spec's
  own visibility precondition before their first long pause (§3.3).
- **The headed walk provoked ONE of the five error strips in a browser** — the unlock strip, the open
  table's password refusal for a stranger (a `/join` 403); the create form's, the deal's and the chat
  panel's were not provoked (a spaced name is legal, the page's guard prevents a second keep, the chat
  input eluded the script; the last two refusals were read through the API), and the open table's own
  strip was not attempted (§3.3). The walk saw no layout defect at 390 px; a stylesheet change in an
  unprovoked strip would still pass every gate.
- **A poisoned or `held` table needs the operator**; nothing sweeps it, and the operator listing
  lags activity that changes no copy (its `last_active` and `bytes` are the last delivery's).
- **The live replay is repeatable only while `wire`'s records are byte-identical.** A workspace
  version bump (each record carries `engine_version`), a walk that plays other moves, or a record
  format change without a rev bump makes the first sends answer 409; the replay then STOPS, names the
  cause and prints the reset — four `wrangler r2 object delete … --remote` lines, run with wrangler's
  login, since the owner's R2 key is read-only (RUNBOOK §6.7, "The replay's reset"). It also needs
  the owner's WARP shell: the force is an operator's action, and the credential is never one. **It
  last ran on deployment 18**; deployment 19's server changes (F-1, F-2, I-84, the code stamp) move
  no payload, and their corpus evidence is the pool's replay at `bd8ce497`.
- **What a live run leaves behind:** two 90-day markers per e2e run at fresh `sim-` ids; `test/`
  saves per e2e run and the replay's `test/browser/…` pair; a PERMANENT house-chat post per replay
  run — "anyone up for a second table?" from `red@wire.test` — and two more per exit run, from x and
  y, each carrying "exit test <stamp>", visible to real people until 500 newer messages trim them;
  and each live table in the live lobby while its run lasts.

**About the archive and the endings**

- **A finished game's save was read live through live run 1's game**: synced at the exit, 180,493 B,
  2,180 events, loaded in the CLI. That run had no tail, so its `archive-stamped` line was not read;
  the abandon's save was read at deployment 16.
- **The refinish** (§6.3 item 2) can strand a one-person table on this host and never on the NAS.
- **An R2 outage costs every request one attempt and one failure batch** while a save is owed and the
  switch is on, the ladder climbing per request (Task 13's E3).
- **The untrusted record on `POST /api/archive` is BOUNDED, not closed**: at worst one trapped
  instance of one caller's archive object, behind Access and the 2 MiB bound.
- **An expired unfinished game is lost when the archive is off**, and so is a game-over save still
  owed from before the switch went off, at an expiry or an abandon while off — the marker says the
  first, the `archive-owed-dropped` line the second.
- **No table expired live.** The expiry, `held`, `expiry-failed`, the expired head, an operator's
  plain expire, the "in the archive" and "keeps no archive" tails and a force's 503 were read in the
  pool only (RUNBOOK §6.11). Tables created before deployment 17 carried no expiry alarm until
  touched; the listing then held none.
- **The house has no poison count**: its chat document is DROPPED at the second consecutive load
  trap, because the lobby chat is not training data; and a refused house copy leaves the lobby
  listing the last copy it received (a corrupted store is the only way there).

**About the instruments**

- **Only two of the e2e's sixty-one cells, in two of its eleven spec files, run live**, by the
  orchestrator, with the credential, plus the opt-in exit spec; CI runs no live tier, and the
  workflow does not say so (R3's docs item 24, open).
- **The deploy guard's build-dependent checks run only where the deploy is made.** CI runs checks
  3–7 over the committed configurations; checks 1, 2, 8 and 9 — the release module, the assets tree,
  and the two freshness checks — need a build and run on the deploying workstation alone.
- **`npm run test:scripts` now needs `cargo` on `PATH`**: the freshness digest's `cargo metadata`
  cell (R-N1). CI has it; a node-only checkout does not.
- **The mutation battery's residue**: row 4 recorded; r17b equivalent by measurement; M33–M36 alive
  and delay-only; the Archive object's clock pin unobservable at its site.
- **The layer's own diagnostics are mute on this host** (`eprintln!` on `wasm32-unknown-unknown`).
  The shell logs what it can observe; RUNBOOK §6.5 is the vocabulary, `object-failed`, `wake` and
  `picker-dropped` among it now.
- **Chromium's two occlusion switches are not enough on macOS.** A covered headed window read HIDDEN
  in the exit's run 1 whatever `--disable-backgrounding-occluded-windows` and
  `--disable-renderer-backgrounding` read (both Playwright 1.62.1 defaults), and Chromium widens a
  window to its minimum width of 500 px, so three side by side still overlapped at 1280 px (run 2).
  The exit spec lays a 2×2 grid when three cannot share the width, reads every placement back, and
  asserts visibility as its own precondition. **Chromium on a socket
  closed while CONNECTING** logs "WebSocket is closed before the connection is established"; believed
  warning-level, which the breakage watch ignores, and unverified live (R3's E-4). **A red live walk
  may be the edge**, not a regression: the walk asserts ONE stream open for the watcher, so a runtime
  update restarting the object fails it by design (R3's E-5).
- **The release roster and the CLI suite WERE run** at the close head, in the one sequence of §2; the fence says why their figures could not have moved.
- **`wasm-opt` 133 built every deployed module**; CI was pinned to 133 in the close's docs and CI commit, so its module now matches the deployed bytes (I-87).
- **Nothing on a phone was measured by this session.** The phones remain the owner's item.

---

## 8. The handover

### 8.1 Into M6.5c2 — the cleanup cycle

**The entry artifact is `docs/plans/2026-09-26-web-frontend-m6.5c2-handover.md`**, written at this
close. It carries the owner's direction of 2026-09-24 in the owner's own words — tidying the
repository; arranging the Docker/NAS setup against the Cloudflare one, what lives where and what
each host owns; finalizing that documentation and its construction; a TESTING STRATEGY for when
each tier runs and what skipping it costs; and the frontend design tweaks — beside the `test/`
lifecycle choice with its recommendation, the design debts all four reviews listed for the cleanup
cycle (§6.2), the owner items carried (§6.3), the traps this cycle paid for, and an execution order.
Nothing the live tiers owed is carried: the deferred two-browser re-run ran and passed. The
owner's close-out direction on CI and the README (§5.1) was started at the close, so M6.5c2 reads
`.github/workflows/test.yml` and both READMEs as that work leaves them.

### 8.2 What M6.5c2 inherits unchanged

The five fenced crates, the CLI, `SEMANTICS_REV` 7 and every bot behaviour. The committed corpus,
byte-identical, and the two host scripts beside it. The Worker, live at `yokohama.moore.nyc`, and
the NAS, live and untouched at `yokohama-nas.moore.nyc`. Workers FREE, the owner's 80,000 line for
live runs as long as the owner keeps it, and the zone as the owner keeps it.

### 8.3 The AI lane

Untouched by this cycle. After the cleanup cycle and the refresh of `CLAUDE.md` and
`guidingpath.md`, it re-enters from `docs/plans/2026-09-11-ai-lane-reentry-handover.md`.
