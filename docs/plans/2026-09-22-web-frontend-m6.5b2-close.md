# M6.5b2 "the front door, installable and offline" — the close

**Written 2026-09-22 on branch `web-room-m6.5b2` (stacked on `web-room-m6.5b`, the cycle's diff
base `369a1f2`), the record of record for the cycle whose authority is the spec
`docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md` (R-6.5b2.1–18) and whose plan is
`docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md` (eleven tasks). Executed in one session from
`docs/plans/2026-09-21-web-frontend-m6.5b2-execution-handover.md` under the owner's mid-session
ruling PF-1 — "switch to subagent driven for all the modules unless it's a very trivial mechanical
stage" — so EVERY task was subagent-driven with Opus as the floor for implementers, reviewers and
the fix wave, including the four the plan header had marked inline. The execution ledger (the
pre-flight scan, every ruling, every review) is the gitignored
`.superpowers/sdd/2026-09-21-web-frontend-m6.5b2-plan/progress.md`; §5 below carries every ruling
with its cost so nothing decided on the owner's behalf lives only there.**

**The headline.** The first screen is a DOOR with two ways on — play on this device, or play with
others — and it asks the server for nothing. Behind it the whole release is cached in the browser,
keyed by a digest of its own bytes, so the door and a complete game against the bot work with the
network off; the app installs to a home screen on both platforms and asks for persistent storage at
the first committed move; a game backs up to one envelope file and restores on another device, with
a save this build cannot open KEPT and listed rather than erased; and a game finished offline is
queued with its reason and uploaded when the connection returns. Underneath, three deployment
switches make the app say what it serves: `dev-identity` is a build feature, so a public build
REFUSES `YOKOHAMA_WEB_IDENTITY=claimed` by name; multiplayer is a deployment switch whose OFF does
not serve the game, chat or stream routes at all; and `/enter` is a turnstile that exists only where
something behind it is protected, which is how Access moved off the hostname without the app ever
learning that Access exists. The five fenced crates and the CLI are byte-identical to the base;
`SEMANTICS_REV` is 7; no bot behaviour moved.

**What is LIVE.** `yokohama-web:0.5.0-m6.5b2`, built from `43d54a5`, in container `58ac45ba478a…`
since 2026-09-22 19:08:27.10Z at `yokohama-nas.moore.nyc` — the NAS's new hostname, taken mid-cycle
at Task 4 (`0.5.0-m6.5b2-rc1`) under the hostname-wide Access application the owner ruled to keep
during development. `deploy/RUNBOOK.md` §3.14 (the mid-cycle release and the hostname move) and
§3.15 (this release) are the deployment records.

---

## 1. What shipped (by task, in EXECUTION order; the rulings each rests on are in §5)

The order run was 1 → 3 → 2 → 4 → 5 → 6 → 8 → 7 → 9 → 10 → 11 (ruling PF-2, §5), which is the
plan's order with two swaps, each because a task's own cell needed the other task's screen or
control to exist.

### 1.1 Task 1 — the three switches, the turnstile, `/sw.js`'s header (R-6.5b2.2, .3, .4)

Commits `ecac705` + `6f68b0b` (fix round 1). **`dev-identity`** is a cargo feature: the `"claimed"`
arm of `FromStr for IdentityMode` is a `#[cfg(feature)]` / `#[cfg(not)]` pair, and the refusing arm
names the feature, both production modes and why (`identity.rs`). **Multiplayer** is
`YOKOHAMA_WEB_MULTIPLAYER`, and OFF means the rows LEAVE the route table: `api::routes` filters the
game and chat rows, and the three routes the axum ROUTER owns outside that table (`events`,
`…/stream`, `/api/stream`) are registered conditionally in `lib.rs` — one sweep cell asserts every
pattern and method 404s with the TABLE's own sentence, and the battery's row 11b (below) is the
measurement that a status-only sweep would have passed the router-owned `events` route, which
answers a genuine 404 with `{"error":"no game named \"x\""}`. **`/enter`** is a 302 to `/#/lobby`
served when EITHER multiplayer or the archive is on — `enter_follows_any_protected_route_and_not_multiplayer_alone`
arranges a real `dist` through the new `AppState::dist` seam and asserts `multiplayer off + archive
on` still answers 302, with a `GET /lobby` 200 carrying `<!doctype` as the positive control that
makes the refusals a reading rather than an empty build. `/sw.js` is served `no-store` on the
success arm AND on the miss arm (the latter Task 6's fix of this task's own deferred minor), pinned
by `hashed_assets_are_immutable_the_worker_is_no_store_and_nothing_else_carries_a_header`.
The corpus moved by exactly `health.json` +`multiplayer`. Review 0 Critical / 2 Important / 5 Minor;
the fix round replaced a `set_var` under the threaded runner with the dist-root seam and put the
refusing build shape into CI.

### 1.2 Task 3 — the front door at `#/` (R-6.5b2.1)

Commit `7381189`. `Route` gains `{kind: "door"}` and `parseRoute` maps `/lobby` to the lobby and
EVERYTHING it does not recognise to the door, so the fallback of a total function is now the one
screen that draws with no server behind it. `FrontDoor.tsx` renders, in order, an optional notice,
`OnThisDevice` (MOVED out of `Lobby.tsx`, not copied) and either the `/enter` link or the line
"This deployment plays single player only." The door's ORDER — the local channel first — is pinned by the cell
"puts the channel that needs nothing above the one that needs a login" rather than argued in a
comment — a Task 3 review minor, closed at Task 6, and until it existed a refactor putting the
turnstile on top passed every cell in the file.
Review 0 Critical / 0 Important / 9 Minor; no fix round, the nine routed to Tasks 2 and 6 as
ride-alongs or to this close.

### 1.3 Task 2 — zero Worker invocations, and an expired session that is READ (R-6.5b2.7, .2)

Commit `553e3ee`. Four requests used to leave the first screen and each went a different way:
`/api/me` is gated on a new `Channel` (`channelOf(route)` in `route.ts` — the door, `#/new/local`
and a `local-` table are `"local"`), `/api/art` is deleted for a static `/art/manifest.json` fetch,
`/api/labels` is local-first on the local channel, and `/api/health` is read at the two lazy points
§2.2 names instead of at boot. The cell that holds R-6.5b2.7 renders `<App/>` at `#/` and asserts
ZERO `/api/` requests. **The expired Access session is read rather than guessed**: `serverHeaders()`
and `isAccessChallenge()` are shared across all THREE fetch sites (`call`, `readStream`,
`recordText`), which the battery's rows 10a/10b/10c each break independently. Review 0 Critical /
1 Important / 9 Minor, and the one Important was ruled plan-mandated interim state (§5): with the
boot health read gone, Abandon read the archive fact's `"off"` default until Task 9 gave it a
click-time read.

### 1.4 Task 4 — the hostname move and the MID-CYCLE DEPLOY (R-6.5b2.5, .16)

Commits `afa186a` + `b6cb059` (fix round 1) — **docs only; no source file was touched**, which is
the task's own premise that the app does not know about Access. `yokohama-web:0.5.0-m6.5b2-rc1` was
built from `553e3ee` and started on the NAS; the CNAME `yokohama-nas.moore.nyc` was created at
22:20:13Z and the ingress added, verified **302 and never 200 from an UNENROLLED vantage** (a
throwaway alpine container on the NAS, because this workstation is WARP-enrolled and reads 200 from
every path); the old `yokohama.moore.nyc` ingress block was retired to the catch-all 404 and the
name is HELD for the Worker. The owner refined the ruling twice mid-task (§5): the path-scoped
Access application of spec §3.2 is NOT deployed, the hostname-wide one stays for development, and
the public-path 200 readings are **deferred by owner decision** and recorded as the 302s they are.
The review was scoped to the record's accuracy and found 0 Critical / 3 Important / 5 Minor, ALL
record-accuracy — the 403's place in the sequence, an unsourced restart timestamp, and a mechanism
(the device-bound token) stated as a cause when only the refusal had been measured. `RUNBOOK` §3.14
is the record, and it now says what was not measured.

### 1.5 Task 5 — what the release CONTAINS (R-6.5b2.15, .18)

Commits `df3aa8f` + `d0363e2` + `0b4a08e` (two fix rounds). `[profile.release-wasm]` is added to the
ROOT `Cargo.toml` (`inherits = "release"`, `opt-level = "z"`, fat LTO, one codegen unit) and
`[profile.release]` is never written, so the fifteen-gate roster, `cargo build --release`, the
server binary and the Docker image build exactly what they built before. `npm run build:wasm` runs
`wasm-opt -Oz --strip-debug --strip-producers` over both emitted modules and REFUSES when
`wasm-opt` is absent, naming `brew install binaryen` (`YOKOHAMA_WASM_OPT_OPTIONAL=1` turns that into
a build that says out loud it produced an unoptimized module). The help pictures went 4.21 MB of PNG
to 764,742 B of WebP, and Fraunces and JetBrains Mono are self-hosted through the HASHED `/assets/`
pipeline with their OFL texts committed beside them and copies shipped in `dist/fonts/` — a
licensing position taken conservatively (§5). The binaryen tarball is CHECKSUMMED in both
`deploy/Dockerfile` and `.github/workflows/test.yml`, the hash measured three times independently.
Review 0 Critical / 0 Important / 7 Minor; the reviewer exercised all four arms of `wasm-opt.mjs`,
checked the woff2 magic and the OFL texts. **This task's round 2 found that the workflow had not
PARSED since Task 1's fix round** — a bare scalar ending in a colon reads as a mapping key, so
GitHub would have reported an invalid workflow and run no job on every commit from `6f68b0b` through
`d0363e2`. Two reviews had read that file without parsing it; the lesson is §6.3's.

### 1.6 Task 6 — the precache generator and the service worker (R-6.5b2.8)

Commits `108cbec` + `480753c` (fix round 1). `buildPrecache` walks the FINISHED `dist` tree plus the
art manifest — **Vite's manifest is not consulted at all**, and the pre-implementation review's
measurement is why: it names `assets/index-<hash>.js` and its stylesheet and neither the WebAssembly
module nor the Worker chunk, so the generator and its own completeness test would have agreed on a
release with no game engine in it. The generator REFUSES a release with no module, no shell, no art,
a bypassed path, a duplicate or either ceiling exceeded, and the cache NAME is a digest of every
entry's bytes, so a re-cut card busts the cache although its filename never changes. The service
worker installs all-or-nothing with `cache: "reload"` and a per-body digest, never calls
`skipWaiting()` by itself or `clients.claim()` at all, and on activate RETAINS the previous
generation and deletes only what is older. **The worker is RUN, not scanned**: `sw.node.test.ts`
bundles `src/sw.ts` through the generator's own esbuild path and drives `install`, `activate`,
`fetch` and `message` against fakes with a real WebCrypto digest — the fix round's work, and it is what turns
five of the spec's mutations from "no catcher" into named red cells. It also found the defect that
would have outlived one transient failure: a `??=`-memoised REJECTION of `caches.open`. Review
0 Critical / 2 Important / 7 Minor.

### 1.7 Task 8 — backup and restore (R-6.5b2.10, .11, .12)

Commits `9841a48` + `813e5c5` (fix round 1). `crates/yokohama-web/src/backup.rs` is the ONE
validator and it is NOT behind the `server` feature, because the browser host calls it:
`archive_post`'s own checks, order and sentences MOVED there verbatim, and the route now calls
`check_pair`. The refusal ORDER was pinned through `dispatch` BEFORE the checks moved
(`the_archive_route_keeps_its_statuses_sentences_and_refusal_order`, adjacent-pair probes carrying
two defects each) and every existing archive cell stayed green without being edited, which is what
the plan asked for. One envelope holds one game or the whole device; a restore is ONE IndexedDB
transaction with the compatibility row as a presence bit, so a half-present pair is a CLASH and not
an empty slot, identical bytes read "already on this device" and change nothing, and a record at
another `SEMANTICS_REV` RESTORES and lands as a row that says why rather than being refused — the id
contract `TableDoc::agrees_with` cannot supply lives in Rust as `check_addressable`, with
`a_restored_game_must_be_addressable_as_a_local_one` as its positive control. Review: spec met on
every load-bearing claim, 0 Critical / 2 Important / 10 Minor.

### 1.8 Task 7 — installable and persistent (R-6.5b2.9)

Commits `b8128ac` + `1b60c41` (fix round 1). `install.ts` captures `beforeinstallprompt` and calls
`preventDefault()` (an unprevented event is spent by Chromium's own bar, leaving the door a button
with nothing behind it), and `installState()` is the ruled precedence — standalone or `appinstalled`
→ installed; dismissed → none; a captured prompt → available; iOS Safari not standalone →
instructions. Persistent storage is requested at the FIRST COMMITTED MOVE, never at boot. The fix
round closed the finding a reader of this lane should keep: an absent `localStorage` read as
ALREADY-DISMISSED (`undefined !== null`), so no install affordance would ever have drawn on an iOS
device with cookies blocked — closed by REMOVING the copy of the accessor that had the defect
(`browserStorage.ts`, whose `readItem` normalises every unreadable storage to `null`), with thirteen
cells red first. Review 0 Critical / 2 Important / 8 Minor.

### 1.9 Task 9 — the outbox, and Abandon's three states (R-6.5b2.13, .14)

Commits `852ab54` + `0621601` (fix round 1). `yokohama-local` goes to VERSION 2 for one new store,
`outbox`, whose row is a tagged union — pending with its reason, or sent with its receipt — because
"what this game is waiting to do" and "what landed" are one statement about one game. A game
finished offline is queued and retried on the browser's `online` event, at the next launch and on
the player's button; **Abandon does not join the queue** and has three answers, not two: send,
delete, or ASK — "unknown" is not "off", which the battery's row 14 breaks. The drain is one serial
queue with a per-row `attempt` and its own catch, so one game whose record traps the module cannot
head-of-line-block every other row, and ONE `localPair` per attempt, so the receipt is checked
against the bytes that are posted. Review 0 Critical / 2 Important / 8 Minor; the automatic upload
at game over was judged spec-conformant and is a PRODUCT NOTE the owner should read (§6.3).

### 1.10 Task 10 — the six browser cells (spec §6.3)

Commits `6d28efa` + `729d646` (fix round 1). One new spec file, `web/e2e/offline.spec.ts`, 765
lines: a whole game played offline on a page the worker controls with not one `/api/` request of
that page's answered across the WHOLE cell; an install verified by bytes on a persistent profile
closed and reopened; three releases shipped in sequence with the offer, the click, and the retention
rule (release 2 activates and release 1's cache is RETAINED; release 3 activates and release 1's is
gone — the orchestrator's ruling, §5); a restore on a FRESH profile; the clash; and the outbox
draining on the browser's own `online` event with "exactly one POST" re-read at the END, after the
poll that would have stopped at the first. The fix round moved the `dist` restoration AND its proof
INSIDE the `finally` (a proof below it never runs when the cell fails, and `git status` is no
evidence for a gitignored directory) and named cell 4's real witness. Review: all six cells as
specified, 0 Critical / 2 Important / 6 Minor.

### 1.11 The fences held, and were shown to hold

`git diff --stat 369a1f2` over `crates/yokohama-core`, `crates/yokohama-oracle`,
`crates/yokohama-difftest`, `crates/yokohama-bridge`, `crates/gen-data` and `crates/yokohama-cli` is
**EMPTY** at the close HEAD. `SEMANTICS_REV` is **7**. The committed wire corpus moved by exactly
one key (§3). `[profile.release]` was never written. Every 400 sentence is still axum's own.

---

## 2. The counts at the close HEAD, in ONE sequence

Taken at `0fc229f` — the code head of this cycle, the deploy's own docs commit — on 2026-09-22 on
the **Mac workstation** (Darwin 27.0.0, the pinned 1.97.1 toolchain), as ONE uninterrupted script
whose per-command logs are in the session scratchpad; **total wall 2,409 s**, of which the failed
first pass of the web gate was 1,596 s. **Wall figures are this machine's**, and the four marked
*(warm)* ran against a cache this sequence's earlier commands had already filled, so they are
green readings and not cost measurements. The docs commit that carries this close moves no count:
it touches `CLAUDE.md`, `docs/handover.md`, this file, one comment block in `deploy/Dockerfile` and
one shell step in `.github/workflows/test.yml`.

| command | reading | wall |
|---|---|---|
| `git diff --stat 369a1f2 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli` | **EMPTY** | — |
| `SEMANTICS_REV` | **7** | — |
| `cargo test -p yokohama-core` | **1576 + 29 ignored** across the twenty-one binaries, plus the crate's doc-test (cargo's aggregate reads 1577) | 194 s |
| `cargo test -p yokohama-cli` | **851 + 2 ignored** (823 lib + 9 `bga_import` + 18 `guide_sync` + 1 `piped_dogfood`) | 35 s |
| `cargo test -p yokohama-oracle` | **191** | 3 s |
| `cargo test -p yokohama-difftest` | **52 + 5 ignored** | 20 s |
| `cargo test -p yokohama-core --example eval_probe` | **214** | 53 s |
| `cargo test -p yokohama-core --example solver` | **132** | 5 s |
| `cargo test -p yokohama-bridge` | **81** (57 lib + 3 bin + 9 `feed_preprocess` + 12 relay) | 8 s |
| `cargo test -p yokohama-web` — **first pass FAILED** | the vs-bot gate arm LIVELOCKED (below) | 1,596 s |
| `cargo test -p yokohama-web` — **re-run ALONE at the same commit** | **289** = **275** lib + **2** `browser_played_pin` + **2** whole-game gate + **10** `wire_fixtures` | 103 s |
| `cargo test -p yokohama-wasm` | **5** | 4 s |
| `cargo test -p yokohama-web --no-default-features --features server --lib identity::` (the refusing build CI runs) | **5** passed, 270 filtered out, the refusing branch taken | 4 s |
| the four bridge node suites (`outbox_race`, `wrapper_transparency`, `safe_table_parity`, `connected_indicator`) | all four green, 155 `[PASS]` lines | 9 s |
| `cargo clippy --workspace --all-targets -- -D warnings` | **clean** | 3 s *(warm)* |
| `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown` | **clean** | 1 s *(warm)* |
| `npm run build:wasm` | `wasm-opt 132`: **1,783,361 → 1,175,223 B** (−34.1%) on both emitted modules | 2 s *(warm)* |
| `npm run typecheck` | **clean, four projects** (app, node, service worker, and the guarded worker check) | 4 s |
| `npm test` | **988 in 54 files** | 10 s |
| `npm run test:wasm` | **17 in 4 files** | 2 s |
| `npm run build` | **153 entries, 9,935,340 B** under the release ceilings (11,900,000 / 1,400,000); module **1,175,223 B**; `sw.js` 27,220 B; **131 art files**; cache name `yokohama-release-19e8532f977bcba2` | 6 s *(warm)* |
| `npm run e2e` | **53 in 11 spec files**, 7.4 min of tests | 450 s |

**The web gate's first pass is the documented vs-bot livelock, and it is read before it is
believed.** `a_whole_two_player_game_against_the_bot_plays_through_the_api` panicked at
`play_a_whole_game.rs:189` — the 20,000-input cap — after **1,558.60 s**, and the other arm
(`two_people_play_to_a_winner_and_each_sees_only_their_own_hand`, the two-identity one) passed.
Because cargo stops at the first failing binary, `wire_fixtures` never ran in that pass. The re-run
of the WHOLE crate at the same commit read **289 passed** with the gate binary's two arms in
**24.07 s**. The defect is pre-existing and diagnosed in the M6.3 close §6.3 — the arm's driving
policy places nothing and leans on the bot to end the game — and it is an owner item, not a finding
of this cycle.

**The release the build produced is the one that is deployed.** Every figure the generator printed
here — 153 entries, 9,935,340 B, the 1,175,223 B module, `sw.js` at 27,220 B and the cache name
`yokohama-release-19e8532f977bcba2` — equals the deploy's own build from `43d54a5` (§4.5), and the
deploy read that cache name back off the wire.

**The five fenced crates and the CLI are byte-identical to `369a1f2`**, so every figure on the
fifteen-gate release roster is unchanged by construction. **That is a statement about the fence and
not a run**: no roster gate was executed in this sequence.

---

## 3. The corpus diff, and the two fixture pairs beside it

`git diff --stat 369a1f2 -- web/src/wire/` at the close HEAD:

```
 web/src/wire/contract.ts | 2 +-
 web/src/wire/health.json | 1 +
 2 files changed, 2 insertions(+), 1 deletion(-)
```

**Exactly one key on one payload** — `health.json` gained `"multiplayer": true`, and `contract.ts`
gained `multiplayer` in its `HEALTH` root-key set so the client's own type declares it. The corpus
stays at **94 files**. Every `ClientState`, `HistoricalFrame`, `TableView`, `DealView` and
`archive_*` fixture is BYTE-IDENTICAL, which is what proves that three deployment switches, a front
door, a service worker, a backup envelope and an outbox moved no wire shape at all.

`web/src/wire-script/` gained **two files and nothing else**: `backup-envelope.json` and
`backup-report.json`, Task 8's committed pair, regenerated only by the named Rust test. They sit
outside `web/src/wire/` deliberately — `walk.json` established that directory as the one the
corpus's seed and password pins scan, and an envelope carries a record.

---

## 4. The reviews

Every task had an Opus implementer, an Opus review over a packaged diff, and — where the review
found an Important — a fix round with the SAME implementer (woken by message, never replaced; the
owner's ruling, §5) and a scoped Opus re-review. Every report is a file in the ledger's directory.

| task | review | fix rounds | re-review |
|---|---|---|---|
| 1 | 0 C / 2 I / 5 M — `set_var` under the threaded runner; the refusing build named by no CI job | `6f68b0b`: a dist-root seam on `AppState`; one CI step; five minors | ADDRESSED 7/7, 0 open |
| 3 | 0 C / 0 I / 9 M | none needed; nine routed to Tasks 2/6 or here | — |
| 2 | 0 C / 1 I / 9 M — Abandon reading the archive fact's default on a first visit | RULED plan-mandated interim state; carried to Task 9 | — |
| 4 | 0 C / 3 I / 5 M — all record-accuracy (the 403's place; an unsourced timestamp; a mechanism stated as a cause) | `b6cb059`: docs only, pathspec commit, concurrent with Task 5 | ADDRESSED 8/8, 0 open |
| 5 | 0 C / 0 I / 7 M (the reviewer exercised all four `wasm-opt.mjs` arms) | `d0363e2` (the hashed font pipeline, ruled), `0b4a08e` (the tarball checksum, the OFL texts in `dist`, the workflow parse) | ADDRESSED ×2, 0 open |
| 6 | 0 C / 2 I / 7 M — five §6.5 mutations survive because `sw.ts` is untested; a vacuous substitution check | `480753c`: `sw.node.test.ts` bundles and RUNS the worker; `checkSubstitution` made non-vacuous | ADDRESSED 7/7, 0 open |
| 8 | 0 C / 2 I / 10 M — `deletePair` orphans the compat row; the exporter's rev stamped into every game | `813e5c5`: both, plus nine minors; each mutation-proven | ADDRESSED 11/11, 0 open |
| 7 | 0 C / 2 I / 8 M — an absent `localStorage` read as already-dismissed; no in-memory dismissal | `1b60c41`: one page flag and a shared `browserStorage.ts`; thirteen cells RED first | ADDRESSED 9/9, 0 open |
| 9 | 0 C / 2 I / 8 M — a rejected read aborts the whole drain; the pair read twice per game | `0621601`: a per-game `attempt` with its own catch; one snapshot per attempt | ADDRESSED 10/10, 0 open |
| 10 | 0 C / 2 I / 6 M — the restoration's proof below the `finally`; cell 4's assertion satisfied by the deal screen | `729d646`: both inside the `finally`, proven by two mutations; the real witness named | ADDRESSED 6/6, 0 open |

### 4.1 The whole-branch review (Fable, over `369a1f2..729d646`; `final-review.md`)

Twenty-six commits, 144 files, +17,911/−668, read in ten passes by area — the hunks and then the
file at HEAD where a hunk was cut. Verdict **With fixes**: **0 Critical / 5 Important / 12 Minor**,
plus **12 false or stale sentences** and a triage of the ledger's ~31 deferred lines (8
fix-before-merge, 11 already closed at HEAD, the rest LEAVE with reasons). The reviewer re-ran 18
focused Rust cells, the refusing build's 5, the wasm crate's 5, 219 jsdom cells in 13 files and the
driven worker's 11 rather than trusting the reports' gate tables.

The five Important: a one-click "Remove" erasing the only copy of a game this build cannot open; a
stream held across an expired Access session reading "reconnecting…" for ever although the
classifier reached it; the expired-session link drawn on four error strips of about seventeen; the
IndexedDB v2 bump making a CLIENT rollback a browser-side schema rollback with no record saying so;
and `backup.ts` opening a connection per call and closing none on a path that runs on EVERY local
write.

**The method that found the false sentences is worth carrying**: try the mechanism a sentence
promises rather than re-read the sentence. Two of the twelve were about the same claim from opposite
sides — the code really did classify the edge's 401 on all three fetch paths, and no consumer read
the classification, so the sentence "a change made in `api.ts` alone would leave a reconnect
reporting an ordinary network failure" was true about `api.ts` and false about the screen.

### 4.2 The ONE fix wave (Opus, `729d646..43d54a5`, five commits)

`47621ae` · `a77bb1c` · `5ea38e6` · `be50fbd` · `43d54a5`. **Important 5 of 5 closed**; false
sentences **10 of 12** (the two left are frozen-spec wording, amended in §6.4 below); ledger
fix-before-merge **8 of 8**; Minors 7 of 12 done or recorded, 5 left with their reasons in §6.2.

Three of the closures are mechanisms rather than words. "Remove" became `Abandon`'s two-click,
table-naming shape with the in-flight ref every delete in this client carries. The expired session
now ENDS the stream with the sentence and its link (`signal.ts:272` and `:390`, over one predicate
`isSessionExpired` in `api.ts`), and the link reaches **sixteen** error strips across thirteen files
— fenced by a source scan rather than a cell per host, because a cell per host would only say the
five hosts the review happened to name were fixed, and its own premise asserts the scan really read
five named files. `backup.ts` got `withDb` around all six `openDb` sites, with a cell that read
"expected +0 to be 5" before the fix.

The wave also RE-MEASURED the module in one place. Five documents carried Task 5's figures after
Task 8's `restore_read` grew the module; the root `Cargo.toml`'s `release-wasm` comment is now the
ONE place they are measured and the other four cite it.

### 4.3 The scoped re-review (Opus, `fix-wave-rereview.md`)

Important 5 addressed / 0 left / 0 unaddressed; false sentences 10/2/0; ledger 8/0/0; Minors
8/4/0. No new Critical or Important. **One new Minor**: the CI identity step captures its output
under `set -e`, so a genuine failure aborts before the echo — paid in this close's docs commit
(§6.5). Six observations, including `deploy/Dockerfile`'s stale Task-5 module figure, also paid
here.

### 4.4 The mutation battery (spec §6.5; Opus; `battery-report.md`; at `43d54a5`)

Spec §6.5's text was read AS IT STANDS and split on its own semicolons: **22 mutations, 30 probes**
(three rows carry an explicit "any one of" clause and one names a mechanism with two hiding places).
**22 of 22 CAUGHT, 0 with no catcher, 0 survived.** Every mutation was applied to real source, shown
to fail by a NAMED cell with its verbatim failing line, and reverted; the tree ended byte-identical
to `43d54a5` and `git diff 43d54a5` was empty.

Four readings the close carries because they change how the instruments read:

1. **Row 1 is caught and the residue is narrower than the Task 6 review's "PARTIAL".** An entry
   dropped from the LIST fails `precache.node.test.ts:137`, which asserts the art count at exactly
   131 against a real build. What survives is a different mutation — a depth-capped manifest WALK —
   and it is invisible only because every `{file}` in today's manifest sits at depth 2.
2. **Row 16's catcher is the BUILD gate, not a test cell.** A precache list taken from Vite's
   manifest instead of the tree is caught by `buildPrecache`'s `need("WebAssembly module")` reached
   through `npm run build:client`, which STOPS and names the module; row 16b is the cell that holds
   the refusal itself. The real-build node cell does its own walk and cannot see a mutation inside
   `scripts/precache.mjs`.
3. **Row 13's blast radius is 37 cells across three files** — the archive fact's lazy read removed.
   That is the sign that §2.2's "three read points and no boot read" is load-bearing at three call
   sites, not one.
4. **Rows 11a and 11b fail differently**, and both halves of the sweep's assertion earn their keep:
   11a on the STATUS (400 where 404 was owed), 11b on the BODY.

The two mutations the whole-branch review asked to be ADDED because nothing failed at the time —
a `usePolled` consumer handed an `ACCESS_CHALLENGE` reader, and `forgetUnplayable` reached on the
first click — were measured in the fix wave and are both CAUGHT by cells that fail by name.

### 4.5 The deploy (Opus; `deploy-report.md`; RUNBOOK §3.15)

`yokohama-web:0.5.0-m6.5b2` from `43d54a5`, image `sha256:c802a036c2cb…`, 24,466,536 B, container
`58ac45ba478a…` started 19:08:27.10Z, downtime under 15.1 s. Read back through the tunnel:
`multiplayer: true`, `archive: dir`, `semantics_rev: 7`, `/sw.js` 200 `no-store` 27,220 B with its
cache name `yokohama-release-19e8532f977bcba2` equal to the build's printed name, the manifest
`application/manifest+json`, the woff2 and the module `immutable`, `/fonts/fraunces/OFL.txt`
`text/plain; charset=utf-8`, `/enter` **302 → `/#/lobby`**, `/api/games` 401, `/` serving this
build's own hashed assets. The cloudflared config was re-synced and `cloudflared-nas` restarted
(owner-authorized), both files now hashing equal.

**The vantage lesson, recorded because it looks exactly like a broken deploy**: at 19:08:37Z every
path read 302 while `warp-cli` reported Connected — WARP up, Access session lapsed, the challenge's
meta JWT decoding to `auth_status: NONE`. An origin-side pass taken from inside the container during
the gap agrees with the tunnel pass field for field, which is what makes the lesson checkable.

---

## 5. The rulings taken in execution, with their costs

Every decision taken on the owner's behalf, in the order taken; the ledger carries each with its
`Ruling:` line.

### 5.1 The OWNER's rulings of this cycle (not the orchestrator's)

- **PF-1, the execution mode.** *"switch to subagent driven for all the modules … unless it's a very
  trivial mechanical stage."* Every task is SDD with Opus the floor, including 3, 5, 7 and 11, which
  the plan header had marked inline. Cost if wrong: wall-clock only.
- **PF-8, the mid-cycle deploy, refined twice.** First: *"for task 4 for now you can deploy under
  the base url and that will have all paths protected — do not yet publish the new nas name."* Then,
  at ~17:36 local: *"yokohama-nas.moore.nyc is now covered under the full access policy that
  yokohama.moore.nyc is covered under — we'll be leaving the access policies covering the root
  domain throughout the development process."* So the NAS took the new hostname behind the
  HOSTNAME-WIDE application; the spec §3.2 path-scoped door stays the DESIGN, built and proven
  locally, and its live deployment is the owner's to schedule. **Cost, and it is paid in §7: the
  zero-login single-player claim is not SHOWN live this cycle** — during development no path is
  public, so the plan's "public paths 200 with no session" readings are deferred by owner decision
  and recorded as the 302s they are.
- **The cloudflared restart is authorized standing.** *"you can restart cloudflared at any time"* —
  so the comment-only config re-sync rode this cycle's release deploy rather than a restart for two
  comments. Cost if wrong: none; the ingress blocks were identical before the sync.
- **The orchestrator runs NOTHING in the tree while an implementer works, and arms no timers.**
  *"you're conflicting with the in progress implementation agent that noticed you changing its work.
  Stop and wait for it to report back, no timer needed."* The agent's message is the signal.
- **A dead agent is WOKEN by message, not replaced by a fresh spawn**, and finished agents are not
  stopped while a fix round might still need them. FOUR agents died mid-cycle, on API 5xx or on the
  account's usage window: two were replaced by fresh agents BEFORE this ruling (a Task 6 reviewer
  and a Task 7 re-reviewer, neither of which had written a file), one recovered on its own once the
  API settled, and one had already committed and written its report when it went.
- **The Android emulator reading is SKIPPED** (plan Step 3's "second reading"): the owner tests on a
  physical Android handset, and BOTH phone readings — the iPhone exit of R-6.5b2.17 and the handset
  — are recorded here as OPENING ITEMS for M6.5c rather than gates of this cycle. Cost: none;
  R-6.5b2.17 already said "recorded, not gated".

### 5.2 Pre-flight (before Task 1)

- **PF-2, the execution order** 1 → 3 → 2 → 4 → 5 → 6 → 8 → 7 → 9 → 10 → 11. Task 2's
  "the first screen asks the server for nothing" cell renders `<App/>` at `#/`, which at the plan's
  order was the LOBBY and legitimately fetches; and Task 7's iOS cell asserts a button Task 8 builds.
  Cost if wrong: none visible; every dependency the plan states still holds.
- **PF-3** Task 1 also edits `crates/yokohama-wasm/src/lib.rs`'s `HOST` (the plan's file list omits
  it; it does not compile otherwise). **PF-4** Task 1 also declares `Health.multiplayer` in
  `types.ts` and `contract.ts`, or its own typecheck gate is red by construction. Cost of both:
  none.
- **PF-5** `api::routes(serves_games)` filters the game and chat rows; the three stream routes are
  the axum ROUTER's and are gated there. Cost if wrong: the sweep cell catches it.
- **PF-6** Task 6's real-build cell lives in the NODE project, because CI's toolchain-free
  `web-client` job has no module to build against — a cell that never runs is the class this repo's
  gate rule forbids.
- **PF-7** (a trap, not a deviation) the e2e's no-worker fixture must reach contexts the specs
  create themselves, not only Playwright's own fixtures.

### 5.3 Taken during execution

- **Task 1, I2 closed NOW rather than routed to the close**: the refusing build shape gets a CI step
  and a CLAUDE.md row the moment it exists, because "a suite no documented gate line names never
  runs" starts biting then. Cost: one CI line.
- **Task 1, the implementer's M5 deviation STANDS**: `Health.lobby_chat = chat.lobby &&
  host.serves_games`, not `&& multiplayer_offered` — the field reports whether the house chat ROUTES
  answer, and on every axum host the two spellings agree. Cost if wrong: one token, and the corpus
  replay would say so.
- **Task 3, the §2.1/§4 wording seam STANDS as built**: the door's heading is "On this device" and
  "Play on this device" is the lobby's back-link. Cost if wrong: two strings.
- **Task 2, the one Important is plan-mandated INTERIM STATE**: Abandon reading `archiveKind()`'s
  `"off"` default for a browser that has never entered the lobby is the plan's own interim between
  Tasks 2 and 9, because adding a read at local-game-open would break R-6.5b2.7's accounting. It
  SHIPPED to the NAS in the mid-cycle deploy (owner: no games of consequence, in-person testing
  only) and RUNBOOK §3.14 records it as a known state. Cost if wrong: one abandoned local game
  during development, not sent to the archive. **Closed at Task 9, both lines, with cells.**
- **Task 4's fix round runs CONCURRENTLY with Task 5's implementer** — docs-only, two files Task 5
  never touches, committed with the PATHSPEC form so the shared index cannot sweep another agent's
  staged files. Cost if wrong: one commit to untangle.
- **Attribution, correcting my own dispatches**: the plan says commit messages end with the
  EXECUTING agent's attribution. Six commits (`ecac705`, `6f68b0b`, `7381189`, `553e3ee`, `afa186a`,
  `b6cb059`) were written by Opus agents carrying the orchestrator's Fable line because my dispatch
  told them to. Those SHAs are reviewed and were NOT amended ("new commit, never amend a reviewed
  SHA"); from Task 5 on every implementer signs as itself. Cost: none beyond this record.
- **Task 5, SPEC OVER MY OWN RULING**: the fonts go through the HASHED `/assets/` pipeline (spec
  §2.7), not `web/public/fonts/` — hashed names get `immutable`, unhashed ones get no cache header
  at all. Cost if wrong: two files moved back.
- **Task 5, the PNG help intermediates move OUT of `web/public/`** — a gitignored PNG there is still
  copied into `dist/` by Vite, which is exactly the 4.2 MB the WebP cut removes. Cost: none.
- **Task 5, SECURITY**: the pinned binaryen tarball is downloaded with no checksum in two files, so
  its SHA-256 is pinned in BOTH and verified before extraction, failing loudly on mismatch. Measured
  three times independently. Cost if wrong: none; a mismatch fails the build by name.
- **Task 5, the OFL texts SHIP with the built artifact** — copies under `web/public/fonts/<family>/`
  so `dist/` and the runtime image carry the grant, the source copy staying beside the font. A
  licensing position taken conservatively rather than parked. Cost if wrong: two duplicate files.
- **Task 8, I-2**: a game's per-game `semantics_rev`/`rules_hash` come from its COMPATIBILITY ROW
  where one exists and are `null` otherwise; the envelope header is the exporter's. Cost if wrong:
  two nullable fields.
- **Task 8, M-2**: a game with a compatibility row is listed ONCE, under "Will not open here", not
  also as a playable row. Cost if wrong: one filter. (It has a consequence the fix round recorded:
  the click route to I-1's defect is now reachable only by address — the rule is held, not the
  route.)
- **Task 10, cell 3**: spec §6.3's "the old cache is gone after activation" is the PRE-REVISION
  sentence; the revised §2.3 governs (activate RETAINS the previous generation and deletes only
  older ones). The cell asserts release 1's cache retained after release 2 activates and gone after
  release 3. Cost if wrong: one assertion flipped.
- **No second fix wave**: the re-review's two one-liners (the workflow's echo-before-fail, the
  Dockerfile's stale figure) were PARKED as residuals that could ride this close's docs commit. Both
  were taken (§6.5). Cost if wrong: one CI job's failure output missing until the next touch.
- **The close's counts sequence does NOT re-run the fifteen-gate release roster** — the five fenced
  crates and the CLI are byte-identical to `369a1f2` (§1.11), so every roster figure is unchanged by
  construction. This is a statement about the fence, not a run: nothing on that roster was executed
  in this cycle's close sequence.

---

## 6. The LEAVE list, the owner items, and the amendments

### 6.1 Inherited and not touched

The M6.5b close's §6 stands, in particular: the whole-game gate's vs-bot arm LIVELOCKS on some
OS-entropy seeds (it fired THREE times in this cycle — during Task 6, during Task 8 and in this close's own counts
sequence, reading `2 passed` alone at the same commit in 24.00 s, 26.33 s and 24.07 s — and did not
fire in the battery run or in the fix wave's six runs); the e2e stays local, so a LAYOUT claim from this lane is still checked by
hand on one machine; the e2e's `CAP = 4000`; `walk.json`'s 419 KB pretty-printed diff; "who starts"
on the local form; and nothing in CI building the self-host image.

### 6.2 This cycle's own — deferred minors the reviews triaged LEAVE, each with its reason

- **`onBlocked` on the page-side `openDb()` calls** (`backup.ts`, `outbox.ts`): while an M6.5b tab
  holds version 1 open, "Back up everything" hangs with `busy` set and no sentence of its own. The
  fix is not the helper next door — it is a parameter through five public entry points and state in
  two components — and the Worker's own notice naming the other tab is on the same screen. Recorded
  for a later touch.
- **A compatibility row outlives its reason after a future rev bump**: a game restored at rev N as
  unplayable keeps its row after the device moves to rev N+1, and `OnThisDevice` hides it from the
  playable list on the strength of it. Recorded in `web/src/local/db.ts` ON `restoreOne` — the
  function the fix will be about — with both candidate fixes and the reason it is not done now: no
  rev N+1 record exists to restore, so the state cannot be reached and a fix cannot be tested.
- **No cell restores a game the CLIENT drew.** Every restore cell uses the Rust-emitted fixture;
  the TypeScript `LOCAL_PREFIX`/`LOCAL_USER` are pinned only indirectly. A coverage addition rather
  than a finding.
- **`channelOf`'s bare `return "multiplayer"` has no direct cell** (the route parse is covered), and
  **the "leaves the Worker's words alone" cell lost its anti-vacuity partner.**
- **`test/platform.ts` copies prototype methods detached from their receiver** (unreachable today);
  **`storage.ts`'s `askedListeners` is declared below its closures** (works by hoisting, reads
  oddly); **`app_with`/`app_with_multiplayer` are `async fn`s that await nothing** (clippy is clean
  with `-D warnings`).
- **Task 8's `M-7`**: a NEWER envelope version is refused with serde's unknown-field sentence rather
  than "this is envelope version N" — a version-first parse for when the version ever bumps
  (M6.5c).
- **The e2e no longer clicks "Send to the archive"** (four jsdom cells cover the control), and the
  send control's darkening after a landed send holds per page load only — after a reload the button
  returns on an archived game and a click buys the server's `identical`.
- **`precache.ts` counts digests with a regex that depends on esbuild's pretty-print whitespace**
  (it fails CLOSED and reads 153/153 at HEAD); **the e2e guard sees `browser.newContext(` and not
  `browser.newPage()`** (no spec uses it today); **cell 2's "answered nothing from the network" has
  no anti-vacuity partner** and is a bonus assertion beside the cell's real claim; **`CAP` and
  `REV = "rev7"` are restated** in the e2e (a rev bump must find them).
- **A CI cache hit on `~/.local/bin/wasm-opt` skips the tarball checksum** — the guarantee is about
  the download, and Actions cache scoping bounds who can write the key.
- **`deploy/Dockerfile`'s base tag still floats** (`rust:1-slim-trixie`) and the AppleDouble
  sidecars macOS `tar` writes into every image cut from this workstation are pre-existing since
  RUNBOOK §3.5 — measured this deploy (141 sidecars beside 132 real art files), harmless because the
  precache list is generated from the workstation's `dist` where they do not exist, and recorded
  rather than fixed so this image stays comparable with every prior record.

### 6.3 OWNER ITEMS

1. **The HEADED walk of the deployed site is owed across TWO releases** (§3.14's and §3.15's). Both
   deploy agents were blocked by the same thing — Claude-in-Chrome reporting "extension is not
   connected" / `list_connected_browsers` empty. M6.4 §4.5 is the standing argument that a headless
   pass does not substitute: four headless reviews reported "zero console errors" on a screen that
   was 404ing for a favicon. **The list this cycle owes a headed eye**, all at phone width: the
   DOOR and the lobby's back-link (M6.2 found exactly this class on a new screen); the door's new
   backup and restore controls, which no width gate covers; and the five error strips the Refusal
   sweep touched mid-flow — the deal, the open table, the create form, the unlock screen and the
   chat panel — since jsdom cannot see a stylesheet and thirteen screens changed.
2. **The clean-browser single-audience reading** (one login at `/enter`, then every destination
   without navigating to them) is still owed. The enrolled workstation cannot take it: every read
   from here is an authenticated one, and the device-bound token confounds the measurement.
3. **The phone.** The iPhone exit of R-6.5b2.17 — install to the home screen, airplane mode, play,
   close the browser entirely, reopen, resume, finish, export, then an update and a restore on a
   fresh profile — and the Android handset as the second reading. Both are OPENING ITEMS for M6.5c
   by the owner's ruling. **Nothing on a phone was measured by this session**, so "installs and runs
   offline" is a claim this cycle's Chromium cells make and this deployment does not.
4. **`npm test` has a pre-existing order-dependent flake** (reproduced at `afa186a` with the
   cycle's work stashed, so it is not this cycle's). It did not fire in any of the fix wave's six
   full runs or in the close sequence.
5. **Under a `VITE_MULTIPLAYER=false` build, four "Lobby"-labelled controls on LOCAL screens lead to
   a lobby whose game routes do not exist** — its own refusal and one link back, so nobody is stuck,
   but it is a product call the spec did not make.
6. **Every commit from `6f68b0b` through `d0363e2` carried an unparseable workflow**, so no CI
   reading in that window was a reading of anything. Fixed at `0b4a08e`; the lesson is that any edit
   to a workflow file gets a local parse check in the gate
   (`python3 -c "import yaml; yaml.safe_load(open('.github/workflows/test.yml'))"`), which this
   close's docs commit ran.
7. **A finished LOCAL game uploads itself to the archive at game over**, wherever the deployment
   keeps one, with nobody clicking anything — and `reconsiderAll()` uploads every finished local
   game on the device the first time an archive is discovered, including games finished before this
   release. Spec-conformant (§2.6 rule 2, §6.3 cell 6) and the e2e caught it as an unexpected
   `POST /api/archive`, so the walk was re-shaped and not the behaviour. **A player can notice a
   game they did not send appearing under `browser/<them>/`** — which is why it is here in the
   player's terms and not only in the ledger.
8. **The cloudflared config re-sync is DONE** — carried by this cycle's deploy under the owner's
   standing restart authorization, the old file kept as `config.yml.bak-2026-09-22`, both copies
   now hashing equal and `cloudflared-nas` back with four connections by 19:16Z. It closes the
   Task 4 item. One consequence: **the NAS's `config.yml` is now root-owned**, which works and
   matches the older backups, and which a future in-place editor will meet.
9. **The server binary SHRANK 50,904 B** this release (6,591,376 vs §3.14's 6,642,280) and nobody
   has explained it. It was CHECKED to be from this tree (three strings only this release has), not
   diagnosed. Low risk; worth one look if anyone tracks binary size.

### 6.4 Amendments to the frozen spec (recorded here, never edited there)

Two sentences in `2026-09-21-web-frontend-m6.5b2-offline-spec.md` are wrong at HEAD and the document
is a frozen dated artifact:

- **§2.3's "the app then shows one quiet line offering a reload"** — the offer renders on the DOOR
  only (`FrontDoor.tsx:116-119`, which says so in as many words), so a player who keeps a game tab
  open never sees it; the waiting worker activates by itself once every old page closes, which §2.3
  itself calls ordinary. A deliberate narrowing, correct as a policy, and the spec's sentence is
  wider than the code.
- **§3.2 step 3's "ONE Access application on the new hostname, with FIVE DESTINATIONS"**, which then
  enumerates six. `RUNBOOK` §3.14 and `deploy/cloudflared-config.yml` carry the six.

A third naming difference belongs here too: **the spec names `archiveKind()` in four places** and
Task 9 retired it for `useArchiveFact()` when it unified the fact's two accessors (whole-branch
review Minor M-9). The mechanism is the spec's; the name is not.

### 6.5 The two parked one-liners, both paid in this close's docs commit

- `.github/workflows/test.yml` captured the identity run into `out=$(… 2>&1)` under `set -e`, so a
  genuine failure aborted the step BEFORE the `echo` that prints the output. It now prints on both
  paths, and the file was re-parsed with `yaml.safe_load` after the edit.
- `deploy/Dockerfile`'s cross-host comment carried Task 5's module figures as if current. It now
  names them as Task-5-era readings of a module that has since grown, cites the root `Cargo.toml`'s
  `release-wasm` comment as the ONE place the current figures are measured, and keeps the sentence
  that is load-bearing: a ceiling must clear the LARGER of the two hosts, because the Docker stage
  is what a deployment ships and it has not been re-measured since Task 5.

---

## 7. What this cycle does NOT claim

Spec §7 is the starting point; each sentence below is it corrected by what execution measured, plus
what the reviews and the deploy added.

**About the door and Access**

- **The zero-login single-player claim is not SHOWN LIVE.** The path-scoped Access application of
  spec §2.1/§3.2 is BUILT and proven locally — `/enter`'s turnstile, the switches and the sweep are
  Task 1's cells — but by the owner's ruling the NAS stays behind the HOSTNAME-WIDE application
  throughout development, so no path on the deployed site is public and the plan's "public paths 200
  with no session" readings were **deferred by owner decision and recorded as the 302s they are**.
  What was measured live is that `/enter` answers **302 → `/#/lobby`** to an authenticated caller.
- **Access is not made secure by this cycle.** `claimed` still authenticates the door and not the
  seat, and the NAS keeps it. What changed is that a build can now REFUSE it by name. A missed
  Access application on a new protected route would still be open, because `claimed` cannot fail
  closed; moving to `cf-access` is what makes it, and is deliberately not taken here.
- **The clean-browser single-audience reading was not taken** (§6.3 item 2), so "one login, then
  every destination" is a property of the configuration read from the Access API and of one partial
  in-session pass, not of an independent measurement.
- **`/api/me` stays served with multiplayer OFF**, which spec §2.1 lists among the protected paths.
  It is consistent with §3.1's list (game, chat, stream) and deliberate — the identity read is how a
  browser learns there is nobody to be — and `api.rs`'s field doc carries it.

**About the phone and the browsers**

- **Nothing on a phone was measured by this session.** The exit is the owner's, on the owner's own iPhone,
  and the Android handset is the second reading; both are M6.5c opening items. The six offline cells
  prove the mechanism in **Chromium**, on a desktop, on one machine.
- **The seven-day WebKit eviction rule and the separate storage container an installed iOS app
  gets** are read from WebKit's documentation, not measured. A persistence grant reduces eviction;
  it does not prevent a player clearing site data, and no wording says otherwise.
- **Background suspension is not measured.** Whether a suspended tab keeps its Web Lock is still
  unknown; the exit closes and reopens the browser, which is a different thing. Two tabs are still
  settled by the Web Locks API where it exists and the no-Web-Locks fallback is still invisible.

**About the instruments**

- **The blocked IndexedDB upgrade is pinned in jsdom against `fake-indexeddb`**, not across two real
  tabs holding two versions. What the cell arranges is a real version-1 holder; what it does not
  exercise is a real M6.5b Worker on a real second tab.
- **The door's LAYOUT is unmeasured by any width gate.** M5's twelve fit cells and the width gate
  cover the table and the game screen; the door, its install affordance and the backup and restore
  controls are outside all of them, and one Task 7 reading of 0 px overflow at 390 px was taken by
  hand and is not gated.
- **The sixteen-site `Refusal` sweep is unseen by any CSS-aware instrument.** Every host kept its
  own class and the rendered element is the same `div`, `npm test` and the e2e are green — and no
  assertion in this repository can see a stylesheet.
- **Cell 2's "answered nothing from the network" is a bonus assertion with no anti-vacuity
  partner**; the cell's real claim is the install verified by bytes on a reopened persistent
  profile.
- **Row 1 of the battery leaves a residue**: a depth-capped manifest WALK is invisible today because
  every art entry sits at depth 2. The mutation §6.5 words — an entry dropped from the list — is
  caught.
- **Row 16's catcher is the BUILD**, not a test cell (§4.4). It is a per-edit gate and an e2e and
  deploy prerequisite, so it is not a check that never runs; it is not the node cell the plan
  expected.

**About what ships**

- **The `.txt` content type moves the server and the client TOGETHER.** The two shipped
  `dist/fonts/<family>/OFL.txt` copies now go out `text/plain; charset=utf-8`, and the precache
  install compares the served type against the expected one — so a PARTIAL deploy (new client, old
  server) would fail the install on those two files. They are in one commit and the image always
  ships both.
- **The Docker-built module has not been re-measured since Task 5.** The cross-host gap measured
  then was 3,833 B with the SAME `wasm-opt 132` on both (1,173,055 B in the image against the
  workstation's 1,169,222 B), the optimizer converged, and the residue is what `cargo` and
  `wasm-bindgen` emit on x86_64-linux versus aarch64-darwin. Recorded, not diagnosed. A ceiling must
  clear the larger figure.
- **The build variance this cycle records, so two readings do not look contradictory**: spec §0.3's
  nine Task-5 builds and the workstation's own readings differ by ordinary build variance — the
  **109 bytes** on the same asset that §0.3 already names, and the **6 bytes** between the deploy's
  `gzip -9` of the module (456,054 B) and the `Cargo.toml` comment's (456,048 B) for the SAME
  1,175,223 raw bytes. The module GREW 5,996 B between Task 5's figure and the fix-wave head because
  Task 8 added `restore_read`; that is growth, not variance, and the re-measurement is why five
  documents stopped disagreeing.
- **`POST /api/archive` still hands an untrusted `GameRecord` to the engine's setup path** (M6.5b
  close §6.3 item 6). This cycle keeps it behind the door rather than closing it, and R-6.5b2.14 is
  the ruling that uploading requires passing the door.
- **Deferring the install under Save-Data is a LEAVE item.** The charter's "the art waits" cannot
  coexist with an all-or-nothing release; deferring the whole install is the coherent version and is
  not built.
- **The service worker's update offer renders on the door only**, and a finished local game uploads
  itself — both §6.4's and §6.3 item 7's, restated here because they are the two things a player can
  notice that the spec's own words do not describe.

---

## 8. The handover

### 8.1 Into M6.5c "the objects" — the opening items this cycle hands over

The entry artifact for M6.5c is **this document**. Its plan and spec take these first:

1. **The two phone readings** (§6.3 item 3), which are the owner's and are the only WebKit evidence
   this lane will have: the iPhone exit as R-6.5b2.17 words it, and the Android handset as the
   second reading. Recorded, not gated.
2. **The public Worker's configuration (R-6.5b2.6)** — whether it serves multiplayer behind Access
   or single player alone, and therefore **whether the card art becomes public**. Both switches are
   built and proven locally; which way the Worker runs has not been decided.
3. **`cf-access` (or another identity method) in place of `claimed`**, which is what makes a
   path-scoped door fail closed, and with it the live deployment of the path-scoped Access
   application this cycle designed and did not deploy.
4. **The archive moving to R2**, inheriting this cycle's OUTBOX and the existing layout function
   unchanged: a browser-sent save already lands under `browser/<caller>/rev7/{over,abandoned}/` and
   a bucket writes the same relative paths. The layer's `Archive::put` is synchronous and an R2 put
   is not, so M6.5c still owes the prepare/upload/commit protocol with a receipt before a delete is
   authorised.
5. **The SPA-fallback cells must be RE-PROVEN on the Worker.** `/sw.js` 404ing rather than answering
   HTML, and `/enter` reaching the origin rather than the fallback, are proven here against axum's
   own map; Workers' static assets have their own `not_found_handling`, so a green cell in this
   cycle is not evidence about that one.
6. **The untrusted `GameRecord` on the archive route** (M6.5b close §6.3 item 6), which moves from
   behind a development door to a public one the moment the Worker is public.
7. **The no-Web-Locks fallback**, still invisible, and background suspension, still unmeasured.
8. **The measurements the umbrella spec's §6.1 still owes**: cold replay, a whole bot turn, `why`
   and the archive's strict replay inside a live Free-tier object.

### 8.2 What M6.5c inherits unchanged

The five fenced crates, the CLI, `SEMANTICS_REV` 7 and every bot behaviour. The deployment target is
Workers FREE (R-6.5b.16, amending umbrella R-6.5.12). The static assets a Worker deployment serves
are exactly this cycle's precache inventory — hashed, `immutable`, and now digest-keyed — which is
what makes "static assets served without the Worker" a shape that already exists rather than one
M6.5c has to invent. The archive route and trait go over unchanged. Hibernating WebSockets with the
picker persisted (umbrella H-4), the house projection updated only when a visible field changes, and
R-6.5b.11's two push shapes are all still M6.5c's.

### 8.3 The AI lane

Untouched by this cycle; it re-enters from `docs/plans/2026-09-11-ai-lane-reentry-handover.md`.
