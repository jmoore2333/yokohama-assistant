# M6.5b "single-player in the browser" — the close

**Written 2026-09-21 on branch `web-room-m6.5b` (stacked on `web-room-m6.5`), the record of record
for the cycle whose authority is the spec `docs/plans/2026-09-20-web-frontend-m6.5b-single-player-spec.md`
(R-6.5b.1–16) and whose plan is `docs/plans/2026-09-20-web-frontend-m6.5b-plan.md` (nine tasks).
Executed in ONE session from `docs/plans/2026-09-20-web-frontend-m6.5b-execution-handover.md` under
the owner's ruling of that handover: Tasks 1 and 9 INLINE by the orchestrator, Tasks 2–8
subagent-driven with Opus as the floor for every implementer and reviewer, a Fable whole-branch
review, spec §6.6's mutation battery, the deploy, the push and the PR. The execution ledger (the
pre-flight scan, every ruling, every review) is the gitignored
`.superpowers/sdd/2026-09-20-web-frontend-m6.5b-plan/progress.md`; §5 below carries every ruling
with its cost so that nothing decided on the owner's behalf lives only there.**

**The headline.** A person opens the site and plays the bot with no server behind the page. The
whole table layer — `api::dispatch` over `Games`, the engine and the production bot — runs as
WebAssembly in a dedicated Web Worker; every local game lives in the browser's IndexedDB; the
first screen lists those games FIRST and keeps listing them when the API cannot be reached; two
tabs on one game are settled by the Web Locks API with a Take over; a running local game can be
saved as a file at any time, a finished one sent to the server's archive, and an abandon sends
it first. The axum host gained two routes — a batch run route the walk now uses everywhere, and
the archive route that strict-replays what a browser sends and files it under a path-safe caller
prefix by the leaf the replay shows. The five fenced crates and the CLI are byte-identical to the
base; `SEMANTICS_REV` is 7; no bot behaviour moved.

---

## 1. What shipped (by task; the rulings each rests on are in §5)

### 1.1 The run route and the leaf (Task 1, R-6.5b.10, R-6.5b.13 — inline)

`GET /api/games/{id}/at?from=&to=` answers the frames of `[from, to)` in one reply, computed by
ONE forward pass: `HistoricalFrame::run` takes `state_at(from)` once and applies one event per
position, and `at(k)` became a thin wrapper over the same `from_state`, so a run's frame IS
`at/{k}`'s by construction and by the cell that compares them BY VALUE over a run whose three
frames all differ. The cap (64) is checked FIRST, the range's 404 is `at/{k}`'s own sentence
extended to name the run (`to = total + 1` is ADMITTED — the live frame — and `to = total + 2`
answers what `at/{total + 1}` answers), and `from > to` is deliberately the same 404. The
admission grid gained the row, so the route is admitted exactly as `at/{k}` is. The client's
`fetchRun` makes ONE run call with `to = next + 1` and pages a longer run in cap-sized calls; the
scrubber keeps `/at/{k}`. At the abandon site the leaf follows the mirror's `over` (R-6.5b.13),
with the one-move-stale residual named in the cell.

### 1.2 The archive route and the caller prefix (Task 2, R-6.5b.6, R-6.5b.15)

`POST /api/archive` takes the record and the local table document as two STRINGS and checks, in
order: `semantics_rev` present and equal (or `replay_strict`'s own "missing semantics provenance"
sentence for a `None`); a terminal digest present; `replay_strict` clean; the table parsing,
passing `check_shape` and AGREEING with the record (`TableDoc::agrees_with`: players, colours,
start seat, seed, and every keep the document CLAIMS — a bot seat's `None` is not compared, which
is what lets a real `[Some(n), None]` pair through). Then the leaf IS the `Why` from the REPLAYED
state, never the body, and the put lands under `browser/<caller_segment>/` with the table
re-stripped server-side and the record as sent. `Archive::put` took the prefix (`""` at the
server's own two sites) and a protocol: the table first, the record's path the once-marker,
`Identical` for the same bytes (a half pair repaired), `Conflict` for different bytes — 201, 200
`identical`, 409. The marker read has THREE answers (the fix round): present, absent, and a read
that FAILS for another reason, which answers `Failed` and writes nothing. `caller_segment` is the
readable encoding (`@` → `_at_`, everything outside `[a-z0-9._-]` → `_`, capped at 64) and the
containment cell canonicalises `../../x@y`, `a/b@c` and `..@..` inside the root.

### 1.3 The wasm host (Task 3, R-6.5b.9, R-6.5b.14)

`crates/yokohama-wasm`, a `cdylib` workspace member over `yokohama-web` with
`default-features = false`: six exports (`init` a REBUILD over a fresh `MemStore`; `dispatch`
with `Host { "local", "none" }` and the null identity carrying the layer's own 401 arm so
`/api/me` answers `user: null`; `load` a CREATE that refuses a present document; `read`;
`set_clock` frozen or live, `js_sys::Date::now()` on the target and a ticking counter natively),
each a thin wrapper over a pure `_inner` function the four native cells call under one test lock;
its own scan over its own `src` with `js_sys` the one allowed call; `debug_trap` under the `debug`
feature only. The build: `npm run build:wasm` emits the web glue INTO `web/src/local/wasm/`
(gitignored, bundled by Vite, the `.wasm` a hashed asset) and the Node glue into
`web/wasm-node/` with a CommonJS `package.json`; `build` = `build:wasm && build:client`;
`typecheck` runs the Worker's own project through a guard that refuses out loud when the glue is
absent unless `YOKOHAMA_WASM_OPTIONAL=1`. `.wasm` is served as `application/wasm`. The
self-host `deploy/Dockerfile` builds the module in its Rust stage and hands the glue to the
client stage (verified by `docker build --target client`, 92 s), and `.dockerignore` keeps a
developer's local glue out of the context.

### 1.4 The Worker, IndexedDB, the lock, the client's local arm (Task 4, R-6.5b.4, .5, .12)

`web/src/local/worker.ts` is ONE serialised actor over the module: every message through one
queue; the lock's callback promise held outside it; an acquisition = `takeGeneration` + a full
rebuild (`init()` and `load` of EVERY stored pair) as one queued operation; persistence by
OUTCOME for the games this tab holds (and a create, unconditionally, the table alone) of the
documents that EXIST, in one transaction that CHECKS the tab's generation — the DELETE included —
and answers `stale` as the lock loss; the page's success reply sent AFTER the durable write; a
storage refusal answered `unsaved` and never a trap; a trap answered as a 500 marked `trapped`,
after which the page terminates the Worker, settles every pending call with the restart sentence,
starts a fresh one and RE-ACQUIRES every held id BEFORE answering anything else (Task 8's finding);
the export snapshot ONE message (`pair`). `db.ts` is `yokohama-local` v1 with `docs` keyed
`[doc, id]` and `meta` keyed `id`. `api.ts`'s `call` routes by `host` first and the id's `local-`
prefix second (`isLocal` defined once in `ids.ts`, the Worker's importer too); the five id-less
routes take `host`; `recordText` reads the side channel; `signal.ts`'s local arm arms no timer;
the deployment facts are per source with `signalKind()`'s union untouched.

### 1.5 The first screen (Task 5, R-6.5b.2, R-6.5b.8)

All four door gates became the server area's CONTENT (`Door.tsx`), and "On this device" renders
above them in every state: rows from the local `GET /api/games?scope=mine` re-read on every
local write, "Play the bot here" opening the form fixed to you and the bot (no seed, no id, no
password), a Worker fault a visible line and never an empty list. The health read and the labels
read are BOUNDED at five seconds: a refused OR a hung API shows one "not connected" line where
the four server sections were, and the words on the cards fall back to the Worker. The e2e
playback cell Task 1 had broken (it counted `/at/{k}` requests) was repaired here.

### 1.6 The local game screen (Task 6, R-6.5b.6, R-6.5b.7)

Ownership from the ROUTE id: `useLocalHold` holds before the first read, on every route change
(the gate keys on the id the hold answered for), releases on unmount; "This game is open in
another tab" with Take over, "opened in another tab" with Take back after a loss, and a lost tab
writes nothing (Send and Abandon absent, as the answer bar is). "Save game file" at any point on
a local game; "Send to the archive" on a finished one, dark unless the server keeps an archive,
darkening for the session on a 201 or a 200 `identical`; Abandon archive-first when the server
keeps an archive — a 201 or a 200 proceeds, a 409 or any 4xx keeps the game with the server's
sentence, a network failure keeps it with "the archive could not be reached — the game was kept".
A stored game that no longer rebuilds gets a minimal screen with the engine's sentence, Save and
a plain Abandon (the archive would refuse that record every time). No stream, no strip, no Chat
panel on a local game.

### 1.7 The walk script and the wasm corpus replay (Task 7, spec §5, §6.3)

`web/src/wire-script/walk.json` — OUTSIDE the corpus, so the leakage pins never scan a request
body — records every primary-`Api` call with its own identity and status, bound to its fixture BY
VALUE (a digest of the frozen response, after the fix round took the 653 MB of retained responses
down to 55 MB); 2,197 rows, 74 named. `npm run test:wasm` replays it through the Node module with
the clock frozen, every row executed and its status asserted, every named row compared (the host
fields `identity`, `signal`, `mode` exempted; `archive` compared), and the third column asserted
as an EXACT set of TWENTY files — five more than the plan counted, because the three `events*`
fixtures come from the transport route no host in the layer serves, and `constructed()` returns
six payloads, not the two its doc had claimed. `Api::new` freezes its clock; the regeneration diff
for that was empty, as the spec predicted.

### 1.8 The e2e, the pair and the pin (Task 8, spec §6.4, §6.5)

`web/e2e/plays_locally.spec.ts`, seven cells in real Chromium: a whole game against the bot with
NO `/api/` request at all once the local screen is open, the download equal to the archived
copy under `browser/walker_at_e2e.test/rev7/over/`; a reload from IndexedDB with every committed
move and the replay wall recorded; a create closed before the first card comes back with its
setup; a quota failure "APPLIED BUT NOT SAVED" with the game still exportable; a real trap
recovered by a fresh Worker with play continuing; the first screen with the API refused; two tabs
of one context with the takeover. The pair the wasm bot played — `browser-played-2p.json`
(595 events, 54 KB), its table document (`keeps [1, null]`, `over`, winner the bot) and a sidecar
naming the release, the date, Chromium 151 and the module's SHA-256 — is committed under
`crates/yokohama-web/tests/fixtures/`, outside the corpus's seed pin by design, and
`browser_played_pin.rs` reconstructs every bot event under the native `why` (350 reconstructed,
245 human refusals, 0 unaccounted) with `placement_cap_binds` zero and `agrees_with` accepting
the pair.

### 1.9 The fences held

The five fenced crates and the CLI: `git diff --stat 54a9a1f -- crates/yokohama-core
crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data
crates/yokohama-cli` EMPTY at every commit. `SEMANTICS_REV` 7. The corpus's seed, password and
watcher pins scan `corpus()` only, with no exemption, and the walk script sits outside their scope
by construction — with an anti-vacuity assertion that the script really does contain the seed and
the password.

---

## 2. The counts at the close HEAD, in ONE sequence

At `e15e369` on 2026-09-21, 04:43Z → 04:51Z, one uninterrupted script on the Mac workstation
(Darwin 27.0.0, the pinned 1.97.1 toolchain), the log kept in the ledger's scratchpad:

| command | reading |
|---|---|
| `git diff --stat 54a9a1f -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli` | **EMPTY** |
| `SEMANTICS_REV` | **7** |
| `cargo test -p yokohama-web` | **274** = 260 lib (31.3 s) + 2 `browser_played_pin` (7.6 s) + 2 whole-game gate (62.4 s, both arms) + 10 `wire_fixtures` (27.7 s); 2:24 wall |
| `cargo test -p yokohama-wasm` | **4** |
| `cargo test -p yokohama-cli` | **851** (823 lib + 9 `bga_import` + 18 `guide_sync` + 1 `piped_dogfood`) **+ 2 ignored** — untouched, as the empty diff says |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown` | clean |
| corpus | **94** files under `web/src/wire/` (83 + 3 run + 8 archive) + `web/src/wire-script/walk.json` 419,546 B |
| `npm run typecheck` | exit 0, three projects, the module present |
| `npm test` | **746** in **42** files |
| `npm run test:wasm` | **2** in 1 file |
| `npm run e2e` | **46** in **9** spec files, **4.0 min** of tests (4:12 wall with the debug module build) |

The battery ran one commit earlier (`ee552cf`, §4.5); `c9f989b` between them is the assertion that
closes its survivor and `e15e369` a doc line — neither moves a count above except the lib's 260,
which `c9f989b` does not touch (it is a node test) and the fix wave had already brought there.

---

## 3. The corpus diff

Regenerated at Tasks 1, 2 and 7 only, each diff read: Task 1 added `run.json`,
`run_400_cap.json`, `run_404_range.json` (three); Task 2 added the eight `archive_*` files
(`archive_post`, `archive_post_identical`, `archive_post_abandoned`, `archive_409`,
`archive_400_rev`, `archive_400_digest`, `archive_400_replay`, `archive_400_table`) with
`health.json` UNCHANGED (the primary `Api` keeps `NoArchive`); Task 7 changed NOTHING under
`web/src/wire/` and created `web/src/wire-script/walk.json`. **83 → 94 files; every `ClientState`,
`HistoricalFrame` and `TableView` fixture byte-identical**, which is what proves the cut moved no
wire shape.

---

## 4. The reviews

Every task from 2 to 8 had an Opus implementer, an Opus task review over a packaged diff, and —
where the review found an Important — one fix round with the SAME implementer and a scoped Opus
re-review. Every report is a file in the ledger's directory; the reply channel truncates at about
four thousand characters, which the Task 2 review was the one to prove.

| task | review | fix round | re-review |
|---|---|---|---|
| 1 (inline) | none by ruling; covered by the whole-branch review | — | — |
| 2 | 0 C / 1 I / 5 M — the marker read fell through to a write on a non-NotFound error | `b01fb41`: three answers, a cell with a directory at the record path | ADDRESSED; 1 minor new |
| 3 | 0 C / 1 I / 6 M — `npm run build` needs the toolchain; the self-host Dockerfile's client stage | `af16794`: the Rust stage builds the module, the client stage copies the glue; `docker build --target client` 92 s | ADDRESSED; 4 minor new |
| 4 | 0 C / 6 I / 11 M — the unfenced DELETE; a module fault read as a storage refusal; a dead Worker left installed; `not-held` read as saved; a hold answered before recorded; no one-snapshot export | `4527466`: all six; 1/3/4/6 mutation-proven, 2/5 Worker-side and read | ADDRESSED ×6; 2 minor new |
| 5 | 0 C / 1 I / 8 M — a hung health read never showed "not connected" | `41c00d2`: the read bounded; two cells RED with the bound removed | ADDRESSED; 2 minor new |
| 6 | 0 C / 2 I / 6 M — a stale lock state on a local→local route change; Abandon live after the loss | `3e6f413`: the gate keyed on the answered id; Send and Abandon absent after the loss | ADDRESSED ×2; 1 minor new |
| 7 | 0 C / 1 I / 7 M — retained frozen responses, 653 MB per walk | `ae56532`: a digest of the frozen serialisation; 653 → 55 MB per walk, 2.99 GB → 92 MB for the binary | ADDRESSED; 2 minor new |
| 8 | 0 C / 1 I / 7 M — the reload cell's cold-pump clause | PARKED by ruling (§5) | — |

### 4.4 The whole-branch review (Fable, over `c158857..2e2a0a2`; `review-final-whole-branch.md`)

Verdict **With fixes**: 0 Critical / 3 Important / 11 Minor over one full pass of the code
package (eleven reads) with the fenced diff, `SEMANTICS_REV`, the wasm crate's cells, the pin
(350 bot events; O-3 native 25.4 ms), the archive cells, the corpus replay and 82 local-host
vitest cells re-run by the reviewer. The three Important: the in-flight rule's `not-held` arm
showed a write that landed after the steal as "APPLIED BUT NOT SAVED" with a false reason; no
caching header on `assets/`, so the 758 KB module was re-fetched on every first-screen load; no
CI job ran `cargo test -p yokohama-wasm`. **Ten false or overstated sentences** were found and
each given its true sentence — among them spec §2.3's "the Worker stays lazy" (false at HEAD by
§2.5's own requirement), the corpus replay's "does not serve" (false for the four `why*` files
the pin drives), the Playwright config's "ONCE", the wasm crate's parity claim and "poisoned",
and the restart's "only then". No ruling was reversed; three were annotated. The twenty-six
deferred minors and the one parked line were triaged (one MUST FIX, nine folded into the wave,
the rest LEAVE with §7 sentences).

**The fix wave** (`ee552cf`, Opus, eight items in the review's priority order): a non-GET
answered `not-held` is the LOCK LOSS (`web/src/local/lost.ts`, the sentence shared, a cell);
`Cache-Control: public, max-age=31536000, immutable` on `assets/` only, with a cell asserting
its absence on `index.html`; `cargo test -p yokohama-wasm` in CI's `web-server` job; `NewGame`
keyed by host; the `serverUp` null-guard; `sendToArchive`'s in-flight guard (its first cell
measured VACUOUS under `userEvent.dblClick` and rewritten); `checked_sub` in `agrees_with`; the
six doc sites corrected. The optional Worker twin of item 1 was declined because no cell can see
it. Gates at the wave's head: cargo web 274, wasm 4, clippy, the wasm check, typecheck, vitest
746/42, test:wasm 2, e2e 46/9 in 4 m 47 s; the gate arm livelocked once (1,634 s) and read green
alone — the inherited defect.

**The scoped re-review** (Opus, `final-rereview.md`): all EIGHT items ADDRESSED with file:line — the `not-held` loss raised in `localCall` (`client.ts:305-313`, cells in `client.test.ts` and `api.test.ts`), `Cache-Control` on the success arm for `assets/` paths only with `/api/`, `/art/` and the fallback structurally excluded (`assets.rs:85-129`), the CI line, the `NewGame` remount reading the posted `players: 2`, the `serverUp` guard through the real not-connected arm, the archive guard with both clicks in one `act`, `checked_sub` refused by name, the six doc sites with the sidecar and the restart wording untouched; 0 Critical / 0 Important / 3 Minor new (a stale doc, an over-wide comment, an app-wide rather than per-game guard), all LEAVE.

### 4.6 The headed live review of the deployed site (Opus, Playwright over real Chrome 153, headed; `live-review.md`, screenshots `live-01…10`)

Run against `0.5.0-m6.5b` on the NAS through the tunnel, signed in as a throwaway identity.
**SHIP WITH FIXES — 0 Critical / 1 Important / 3 Minor.** What held, measured: `/api/` requests
during play **0** across a create, the deal, the keep, seven moves, a bot turn, Suggest, Why, the
scrubber and an Undo — by an in-page `fetch`/XHR/EventSource probe AND the MCP's network log;
two tabs both directions correct (the second refused with Take over, took over with every move,
the first read-only with Take back, Abandon absent, Save kept; Take back returned it); the abandon
landed on the NAS at `browser/live-review_at_m65b.test/rev7/abandoned/local-…{.json,.table.json}`
(201; rev 7, 14 events, bot `v4-step9-draft1`); "not connected" at **5,146 ms** on a hung API and
never spuriously; the module one request, 2,842,659 B raw / 727 KB over the wire (zstd), cached
at 8–13 ms afterwards; the API-refused first screen with a real 503 on `/api/*`.
**The Important (I-1): a Chat panel rendered on a LOCAL game's DEAL screen and accepted messages
from `local@browser`** — spec §2.5 "no Chat panel" on a local table; the game screen was clean;
reproduced on two fresh games. Fixed the same night as the point release **`0.5.0-m6.5b.1`**:
`253387d` the RED cell (a local deal over the fake Worker holds no chat `log` region and posts
no chat message to the local host — measured red with the guard mutated to always render),
`9bd1d4b` the guard (`!isLocal(game)` around the deal screen's Chat mount); typecheck 0, vitest
**747**/42; the e2e was not re-run for it (a Dealing-only change whose e2e cells all sign in
hosted). The three Minors LEAVE as owner items: local rows name `local@browser` beside hosted
rows naming a person; the take-over screen names no game, so with two local games you cannot
tell which you are claiming; `why`'s refusal shows the machine tag `no_bot_move_yet` (inherited
from M3). The review's own lesson repeats M6.4's: the vitest and e2e cells covered the GAME
screen's three differences and never the DEAL screen's, and only a headed walk noticed.

### 4.5 The mutation battery (spec §6.6; Opus; `mutation-battery.md`; at `ee552cf`)

Twenty-three mutations, each applied, run against the cell the spec names, and reverted with the
tree verified clean: **19 CAUGHT · 1 SURVIVED · 1 UNCATCHABLE · 1 recorded-not-caught · 1
folded.** Caught: a `fetch` in a local game; an edited event, another rev, a stripped digest, a
disagreeing table each refused by its own sentence; a record not over accepted under
`abandoned/`; the bot's moves reconstructing under native `why` with a perturbed key failing the
pin; the seed re-served under another key caught by the server's value pin; a frame off by one
and the range refusal as a 400 in the run cells; any one door gate restored caught by the boot
cells; the lock not stolen and a takeover that does not rebuild, both caught by the two-tab cell in
real Chromium; the lobby's stream silenced; a server id routed to the Worker; the `wasm` MIME arm
removed; the leaf taken from the body; the raw address as the prefix; the walk's identity ignored
(the first per-seat payload). **Survived: #22** — `archive` exempted in the corpus replay changed
nothing, because both hosts run `NoArchive` and read `"off"`; the hole was in the exemption list's
own accounting, and it was CLOSED the same night at `c9f989b`: the replay now asserts each
exempted field really DIFFERS between the two hosts before stripping it, and re-applied, the
mutation fails by name. **Uncatchable: #16** (a bot pump on a cold GET lost across a reload) by
the parked ruling — the shipped client cannot reach that state (§5). **Recorded, not caught: #10**
(the run builder replaying from scratch per position) — no cell asserts wall, as the spec says.
**Folded: #7 into #2** — `archive_post` holds one replay gate, so a seed mismatch cannot be let
through while an edited event still fails. Two notes: #9's client-contract catcher never ran (the
corpus was not regenerated, so only the server's value pin fired — the two catchers are
deliberately different checks); #13 showed the four door gates are split across the two boot
cells rather than each cell covering all four.

---

## 5. The rulings taken in execution, with their costs

Every decision taken on the owner's behalf, in the order taken; the ledger carries each with its
`Ruling:` line. **Owner rulings of the cycle** (not the orchestrator's): the execution mode
(Tasks 1 and 9 inline, 2–8 SDD, Opus the floor); the spec defaults standing; offline mode as its
own cycle "M6.5b2"; the mobile measurement deferred and then RUN by the owner on an iOS simulator
(§1.4, R-6.5b.5); the self-host Docker path confirmed as a goal after a first "we're not using
Modal" ("there is a goal of a user being able to run locally in docker … if it fits here to check
we can"); the deploy, the push and the PR authorized for the wrap; the hand-off into M6.5b2.

**Pre-flight (before Task 1)**

- **P-1** the review base for Task 1 is `c158857` (docs-only after the plan's `54a9a1f`); the
  fenced-crate diff base stays `54a9a1f`. Cost: none.
- **P-2** the module-dependent type checks run where the module EXISTS and skip OUT LOUD where it
  does not: `tsconfig.worker.json` behind a guard that refuses naming `npm run build:wasm` unless
  `YOKOHAMA_WASM_OPTIONAL=1`; the `web-client` CI job sets that variable and stays toolchain-free;
  the `web-server` job builds the module and runs the typecheck after it; the node replay test
  declares the module's shape locally and lives in the node project. Cost if wrong: a worker type
  error surfaces in `web-server` rather than `web-client`, minutes later; nothing silent.
- **P-3** Task 4 implements the actor RULE, not the plan's sketch, whose `hold` nested the queue
  inside itself (a deadlock) and awaited the lock's lifetime inside the queue. Cost: none; the
  sketch could not run.
- **P-4** a CREATE's 201 is persisted unconditionally as the table alone under generation 0 (a
  fresh id nobody can hold); the route's hold then advances it. Cost if wrong: a create by a tab
  that never opens the game leaves an unheld table on disk, which is R1's own behaviour.
- **P-5** ownership from the ROUTE's table id, never a fetched `ClientState`; a local table's FIRST
  read waits for the hold's answer; `elsewhere` reads nothing. Cost: none; the spec's sentence.
- **P-6** "On this device" rows carry NO Abandon; a local game is abandoned on its own screen,
  where the tab holds it. Refined at Task 5: spec §4's "Abandon still works" for a game that no
  longer rebuilds is satisfied by Task 6's broken-save screen, not by a row control. Cost if
  wrong: one flag later.
- **P-7** the glue is bundled from `web/src/local/wasm/` and the `.wasm` is a hashed asset under
  `dist/assets/`; the three `/wasm/` mentions in the spec, the plan and Task 9's RUNBOOK line are
  superseded (spec §10 R7). Cost: none.
- **P-8** `Archive::put -> Result<PutOutcome, PutError>` (Task 2's body) over the plan's stale
  self-review line; `ArchiveReply.identical` rides every reply and `types.ts` declares it. Cost:
  none.
- **P-11** `localReady` DROPPED — it would have been a page caller of the local `/api/health`,
  which R-6.5b.12 says does not exist; readiness is implied by any local call. Cost: a readiness
  probe, if ever wanted, is a Worker message.
- **P-12** the R-6.5b.13 leaf cell lives in `tables.rs`'s test module beside the M6.5a abandon-site
  cell, where its helpers are. Cost: none.
- Notes P-9 (Task 2's path segment computed from the module's own identity) and P-10 (the vitest
  `exclude` extends `configDefaults`; the `init` export name verified against the emitted glue —
  no clash).

**Task 1 (inline)** — the corpus's range probe is `from=total-1&to=total+2`, not the plan's
`from=0` (the cap is checked FIRST, so a span over the walk's 1,036 events reads the cap's 400);
the run cell's anti-vacuity compares whole `frame`s (a queued-choice event need not move a
piece); the scrubber's 401 cell keeps `/at/{k}`. **Lesson recorded**: Task 1 changed what a
browser DOES and ran no e2e because the plan named the e2e for Tasks 5, 6 and 8 only — the
playback cell was red until Task 5 repaired it. The e2e is owed by any task that changes what a
browser does, not only what it sees.

**Task 2** — the implementer's three deviations STAND: the 409 body edits `engine_version`
(provenance-only; a replaced-and-restamped first event cannot land at the SAME path, so the plan's
helper could not produce a conflict at all); the table-disagreement body edits `start_seat`
(`check_shape` refuses a `players` mismatch first, with its own sentence); `finished_local_pair`
is two-human, with the `[Some(n), None]` acceptance in the `agrees_with` cell where §6.1 puts it
and re-pointed at the committed pair by Task 8. The reviewer corrected the brief's "five
characters" to SIX, the right number.

**Task 3** — no `git push` during the task (an outward action; the CI wall is read at the PR);
nine declared deviations stand (`crate-type = ["cdylib"]` alone — `cargo test` compiles the lib
with `--test` regardless, measured; a malformed identity carries `User::parse`'s sentence as the
axum host's `resolve` does; the scan walks `src/`; the anti-vacuity partner reads CODE lines; the
ticking counter gated off wasm32; the P-2/P-10 shapes; figures in the report). **The Dockerfile
Important** (`npm run build` now needs cargo + wasm-bindgen, breaking the self-contained image's
node-only client stage): ruled a fix of a known shape → CANCELLED on the owner's "we're not using
modal for anything here" → REINSTATED on the owner's clarification that a user running the image
locally in Docker from a checkout IS a goal → fixed (§1.3), verified by one `docker build
--target client` in 92 s; the fix also found that `.gitignore` does not govern a build context.
Cost of the detour: forty minutes and one reverted half-edit.

**Task 4** — the spec-§4 rebuild-refusal cell SPLIT (Task 4 the screen half, Task 5 the listing);
the fetch-at-zero cell at Task 4 stubs the boot reads (the door gates were still gates) and Task 5
tightens it; review finding 6 (plan-mandated: no one-snapshot export message) resolved FOR THE
SPEC with the `pair` message, the one-document `read` removed as dead; two implementer decisions
accepted — a BOUNDED restart (an unbounded one loops when the module cannot load) and a
`wanted`/`inFlight` pair for StrictMode's hold/release/hold.

**Task 5** — the parked ruling refined (P-6 above). **Task 6** — none beyond P-5's mechanics.

**Task 7** — NO Worker-under-node harness for the two Worker rules that rest on reading (a
harness of its own, outside the plan; Task 8's real-browser cells are the spec's instruments);
the node test's typing per P-2; the binding deviation accepted (every `Api`'s rows recorded,
only the primary's emitted, "answered by no `Api` at all" the hard failure) — the plan's own
sentence contradicted its next paragraph; the third column is TWENTY files, derived (§1.7).

**Task 8** — PARKED, one Important, a conflict between two spec sentences: §6.5's reload cell
wants the bot's cold pump preserved, and §2.5's "who starts as today" fixes the start seat while
the server pumps the bot inline after every input, so no local record ends at the bot's turn. The
arm is read code with no production path (§7); the owner item is "who starts" on the local form.
The whole-branch reviewer agreed and sharpened it: unreachable, not merely untested.

**The close phase** — the whole-branch review on Fable (the owner's tiering of 2026-09-20; Opus
the floor); ONE fix wave (eight items, §4.4) and ONE scoped re-review; the battery at the fixed
HEAD, never beside a reviewer; the deploy, the push and the PR on the owner's authorization of
2026-09-20 21:58.

---

## 6. The LEAVE list

### 6.1 Inherited and not touched

The M6.5a close's §6 stands, in particular: the whole-game gate's vs-bot arm LIVELOCKS on some
OS-entropy seeds (M6.4 close §6.3; it fired ONCE in this cycle's close sequence, 1 passed / 1
failed after 1,634 s, and read 2 passed alone in 24.78 s); the e2e stays local; the e2e `CAP`;
`deploy/Dockerfile`'s base tag floats (`rust:1-slim-trixie`) while the wasm-target install
presumes the pinned toolchain resolves first.

### 6.2 This cycle's own (deferred minors the whole-branch review triaged LEAVE; each in the ledger with file:line)

- **Task 2**: no two-defect probe pins the ORDER of the five archive checks (all five are 400s with
  corpus-pinned sentences; only rev-before-replay is load-bearing and its cell orders it); the
  marker read-then-write is not atomic against a concurrent put of the same path (one operator's
  NAS; §7); the abandon site's conflict sentence changed wording; the route takes no seat or
  ownership check (deliberate; §7 states it as a property); `Failed` carries the archive's
  absolute path (the crate's convention).
- **Task 3**: `doc_of` runs after the init `expect` (the Worker always `init`s first); `set_clock`
  saturates a negative or NaN to 0; `typecheck`'s toolchain dependency (this document's CLAUDE.md
  line); the CLI install layer has no registry cache mount; the dockerignore comment's
  "Measured: both"; nothing in CI builds the self-host image (owner item).
- **Task 4**: an acquisition failure posted as a trap; the restart sentence before re-acquisition;
  the invisible no-Web-Locks fallback (§7; owner item); two path-id regexes; lost-handling written
  twice; `ready`'s unhandled rejection; `withUnsaved` dropping warnings with no array (§7);
  `ask`'s leaked pending entry; `takeover` ignoring `held`; a test-only `readGeneration`; two dead
  switch arms; `takeover`'s unconditional `heldByPage.add`.
- **Task 5**: the local form states its seating twice; a cell matching the component's own prose;
  the playback comment's over-claim; the labels bound's timer never cleared and the effect without
  cleanup; a fault hiding an already-loaded list with no retry; the `Lobby` ↔ `OnThisDevice` import
  cycle (safe via hoisting); `reached.current` assigned during render; two cell comments over-claim.
- **Task 6**: any local 5xx enters the broken-save screen (every local 5xx today IS "this record
  cannot be opened"; §7); local faults reported in the archive's words both directions;
  `.decision.locked-out` duplicates `.decision.walking`; the hook releases an id it never held; the
  Game menu on a local screen lists the server's games; the takeover's `setAnswer` without a `live`
  guard.
- **Task 7**: the script-drift message drops the byte offset; `walk.json` is 23,785 pretty-printed
  lines (owner item); the by-value binding scrambles names across byte-equal pairs (documented
  residual); `hosts_own`'s stated reason for `/api/art`; a non-JSON reply throws a bare
  `SyntaxError`; the double execution unstated; two doc-precision notes from the fix round.
- **Task 8**: the restart gate can stay raised if `start()` throws; the sidecar's "same bot"
  sentence is unmeasured (the file is provenance and stays; §7); `npm run e2e` leaves the debug
  module in `web/dist` (RUNBOOK §1); B's first-frame read distinguishes turn and seat only; a
  third copy of the walk loop and `CAP = 4000`; the pin's restated host constants with no drift
  pin and a `not_a_move` arm that never fires.
- **PARKED (Task 8)**: the reload cell's cold-pump clause and §6.6's outcome-vs-method mutation —
  §5's ruling; owner item 5 below.

Fixed in the wave and therefore NOT on this list: `Keep(n - 1)` via `checked_sub`; the wasm
crate's parity claim and "poisoned"; the two orphaned doc comments; `NewGame` keyed by host; the
`serverUp` null-guard; `sendToArchive`'s in-flight guard; the `corpus.node` "does not serve"
sentence; the Playwright "ONCE" comment; the CI line for the wasm crate's cells.

### 6.3 OWNER ITEMS

1. **The deploy** — DONE at this close: `yokohama-web:0.5.0-m6.5b`, container `1a14939b…` started 2026-09-21 04:59:06Z, then the point release `0.5.0-m6.5b.1` (container `0c3e49f6…`, 06:40:19Z) after the headed live review (§4.6); health `archive: dir`, the module served `application/wasm` and `immutable` through the tunnel (RUNBOOK §3.13).
2. **The module's size**: 2.84 MB raw / 758 KB gzip, with no `[profile.release]` tuning
   (`opt-level`, `lto`, `codegen-units`, `wasm-opt`) anywhere in the workspace; the wave's
   `Cache-Control: immutable` on hashed assets makes it a once-per-release download, and a size
   BUDGET belongs to M6.5b2, which caches it.
3. **The first-screen boot instantiates the module on every visit** to list the games on this
   device (R-6.5b.8's guarantee needs the Worker); spec §2.3's "the Worker stays lazy" is false
   at HEAD and the close says so; an IndexedDB-only listing is the alternative.
4. **`walk.json`** is 419 KB / 23,785 lines of pretty-printed JSON rewritten wholesale on every
   corpus regeneration — one row per line would give readable diffs.
5. **"Who starts" on the local form** would make the cold-pump arm of persistence-by-outcome
   reachable and the reload cell able to claim it (§5's parked ruling).
6. **`POST /api/archive` hands an UNTRUSTED `GameRecord` to the engine's setup path**, which is
   not hardened for a hostile config: a crafted record can abort its own request's task (the
   process survives; no lock is held). Behind Access today; a config-shape pre-check in the layer,
   or a hardening in the fenced core, is a decision for the owner.
7. **The no-Web-Locks fallback is invisible**: on a browser without `navigator.locks` the second
   opener silently wins (the generation still fences the first tab's writes).
8. **Nothing in CI builds the self-host image** (`deploy/Dockerfile`), verified once by hand.
9. **Inherited**: the vs-bot gate livelock (M6.4 close §6.3); the Dockerfile's floating base tag.
10. **From the headed live review** (§4.6): local rows and the archived table name `local@browser`
    beside hosted rows naming a person; the take-over screen names no game; `why`'s refusal shows
    the machine tag `no_bot_move_yet` (inherited from M3).

---

## 7. What this cycle does NOT claim

The whole-branch review's sentences, carried as written with the wave's outcome resolved.

- **Two of the Worker's rules rest on READING and no cell executes them**: a module read that
  throws is the trap path and not `unsaved`; a hold released mid-acquisition answers `false`. After
  the wave a third is read-only on the Worker side and pinned on the page side: a write answered
  `not-held` is the lock loss.
- **The cold-pump arm of persistence-by-outcome is read code with no production path**: the shipped
  client seats the person first and the module pumps the bot inline, so a stored local record never
  ends at a bot's turn; the reload cell cannot exercise it and §6.6's outcome-vs-method mutation has
  no catcher.
- **The determinism pin was played by the DEBUG module** (`npm run build:wasm:debug`); the release
  module is the same source without the `debug` feature and is not separately pinned.
- **Two tabs on one game are settled by the Web Locks API where it exists**; on a browser without it
  the second opener silently wins (the generation still fences the first tab's writes). Measured on
  the owner's iOS 27 simulator (Safari, two tabs) and desktop Chromium; not on Android, a physical
  device, older iOS or under background suspension.
- **A local game lives only in this browser's IndexedDB with no persistence grant requested**: the
  browser may evict it (Safari's seven-day script-writable-storage rule among them), and clearing
  site data deletes it. Backup is the player's own send or save. Level 3's charter (M6.5b2) carries
  the grant and a restore.
- **The module (2.84 MB raw / 758 KB gzip) is downloaded and instantiated on every first-screen
  load** to list the games on this device; spec §2.3's "the Worker stays lazy" is not true at HEAD.
  Since the wave the browser caches the module after the first visit (immutable hashed assets);
  before it, every visit fetched it.
- **`POST /api/archive` admits any signed-in caller and files any valid rev-7 record under that
  caller's prefix**; it checks no seat and no ownership. It is the first route on which an untrusted
  `GameRecord` reaches this server's engine, whose setup path is not hardened for a hostile config:
  a crafted record can abort its own request's task (the process survives).
- **The archive's two-file put is not atomic** and its marker read is not atomic against a
  concurrent put of the same path; "at most once" is per path and the caller prefix collides on
  unusual or over-long addresses.
- **A create or a delete that the browser's store refuses is not reported as unsaved** (those
  replies carry no `warnings`).
- **A local 5xx of any cause renders the broken-save screen**, whose Abandon deletes without
  sending.
- **The restart sentence reaches the screen before the re-acquisition completes**; what waits for
  the re-hold is the re-read.
- **`npm run e2e` leaves the debug module** in `web/src/local/wasm/` and `web/dist/`; the deploy
  rebuilds with `npm run build` and checks for `debug_trap` (RUNBOOK §1).
- **The self-host image builds** (measured once, 92 s) **and nothing in CI builds it**; the e2e
  stays local.
- **The three `events*` fixtures are unreplayable by the browser host** — the signal is the
  host's, and nothing in the corpus replay says the browser host's own signalling is right; Task 8's
  cells are where a local game's signalling is exercised.
- **The point release `0.5.0-m6.5b.1` was not re-walked by the e2e** (a Dealing-only change; its
  e2e cells sign in hosted) and not re-reviewed headed; the fix's cell is the claim.
- **The plan's Task 9 carries two stale sentences** (`web/dist/wasm/`; "the Free-tier confirmation
  the owner owes") that this document does not copy.
- **The mutation battery ran at `ee552cf`**, one commit before the close HEAD; the one commit
  between them (`c9f989b`) is the assertion that closes the battery's survivor, itself re-applied
  and shown to fail by name, and `e15e369` is a doc line.

---

## 8. The handover

**Into M6.5b2 "installable and offline"** — the owner's cycle between this one and M6.5c. Its
entry artifact is `docs/plans/2026-09-21-web-frontend-m6.5b2-handover.md`, written at this
close from the MEASURED artifacts: the module's size and hash, the asset graph a release ships,
the boot and replay walls, the IndexedDB layout, the save formats that exist, the R-6.5b.5
measurement, and the charter's requirements as the owner stated them in spec §8. The decisions
that spec takes first are listed there (§4).

**Into M6.5c "the objects"** — carried unchanged from spec §8 and the M6.5a close's handover:
the deployment target is Workers FREE (R-6.5b.16, amending umbrella R-6.5.12), so gameplay,
replay, `suggest`, `why` and the archive's strict replay run INSIDE objects with `/api/archive`
mapped to an object of its own; the layer's `Archive::put` is synchronous and an R2 put is not,
so M6.5c defines a prepare/upload/commit protocol with a receipt before a delete is authorised;
hibernating WebSockets with the picker persisted (umbrella H-4); static assets served without
the Worker (the immutable hashed assets this cycle's wave introduced are what that deployment
serves); the house projection updated only when a visible field changes; the long-save
measurements (cold replay, a whole bot turn, projection, serialisation, `why`, the archive
replay — CPU and wall separately, on the phone and in a live Free-tier object); the browser
crate's global `Games` is the per-tab instance's and never a shared isolate's; R-6.5b.11's two
push shapes. The archive route and trait go to M6.5c unchanged.

**The AI lane** re-enters from `docs/plans/2026-09-11-ai-lane-reentry-handover.md`, untouched by
this cycle.
