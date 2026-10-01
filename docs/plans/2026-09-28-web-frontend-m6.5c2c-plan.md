# M6.5c2 cycle C — the hosts and the close: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan
> task-by-task. The execution shape is §"Execution shape" below, which records the owner's standing answers: every
> task subagent-driven, an Opus implementer and an Opus reviewer per task, Fable for the whole-branch review at the
> cycle's close, and EVERY account step — a deploy, a secret, the bucket, a live run — the ORCHESTRATOR's alone. Steps
> use checkbox (`- [ ]`) syntax for tracking. **Read the spec first** (revision 2.7: §10.7 is this cycle's rulings,
> §4 the cycle map with row D, §2.1 "The Worker's two doors", §2.5's Worker half, §2.7–§2.11 as amended, §3's close);
> this plan argues from it and does not restate its reasons. **Read §"Couplings between tasks" before any task**: an
> implementer sees only its own task, and the couplings are what no single task section can carry. **Every task
> that changes what the client shows loads `frontend-design:frontend-design` before it writes a line** (R-6.5c2.18;
> Tasks 10 and 11).

**Goal:** The two hosts as the web lane leaves them — the Worker with its two doors and its open door's refusals, the
refinish rule reachable on the Worker, the module's invariants structural and its restatements imported, the shell's
and the scripts' measured debts closed, the build's pins aligned and CI lean, the workspace at 0.6.0, the lobby's
finished games folded, the client's carried fixes and its a11y pass landed, each host's documents beside its code —
and then the close: the review, the battery, ONE recorded sequence, both hosts deployed (the NAS on `cf-access` for
the first time), the live tiers run, the close document written, PR #41 opened and the handover into cycle D.

**Architecture:** The layer (`crates/yokohama-web`) is FINISHED but for four named touches (Tasks 6 and 9); the
client is finished but for two screen tasks (10, 11). This cycle's code is the WORKER's — the TypeScript shell
(`worker/src/`) and the module (`crates/yokohama-worker`) — the browser host's two restatements, the scripts, the
Dockerfile and the workflows. Change runs from the doors outward: the door variable and the `claimed` identity kind
(Task 1) before the refusals that read the door (Task 2); the receipt's held text (Task 3) before the module's
invariants (Task 4), which edit the same files; the shell's and the layer's small debts (5, 6) and the scripts' LOWs
(7); the build and CI (8) before the bump (9), so the bump's commit is the first the new jobs run against when the
branch is pushed; the client (10, 11) after the bump, so its one e2e literal is in; the host documents (12) after
every code task, so they describe the final code and the NAS recipe reads `cf-access` before the deploy that follows
it; the close (13) last.

**Tech Stack:** TypeScript in `worker/` (the shell; vitest's node cells and the POOL — `@cloudflare/vitest-plugin`
1.2.3 inside workerd, now TWO projects); Rust 1.97.1 for the module (`wasm-bindgen 0.2.128`, `release-wasm`,
`wasm-opt` 133) and the layer's named touches; React 18 + TypeScript in `web/` (vitest over jsdom; Playwright over
Chromium against the release binary); Docker (buildx, `linux/amd64` emulated on this arm64 workstation); GitHub
Actions; wrangler 4.136.3 and the owner's Cloudflare account (the close only).

**Spec:** `docs/plans/2026-09-26-web-frontend-m6.5c2-spec.md` at **revision 2.7** (`a855b379`) — §2.1 "The Worker's
two doors" whole, §2.4 as amended (the versioned refusal sentence; the key NOT taken), §2.5's Worker half, §2.6 (the
restatements IMPORTED, the whole-game gate's return), §2.7 as amended, §2.8 as amended, §2.9 as amended (CI stays
lean; the Docker workflow's narrowed trigger), §2.10, §2.11 as amended (what C keeps, what D takes), §3's close as
amended (steps 1–6 and the close document; the screenshots, the merge and the tag are D's), §4 rows C and D, §8.2,
§10.7. The entry artifact is `docs/plans/2026-09-28-web-frontend-m6.5c2c-handover.md` (§1 the readings at
`3ed27f07`, §2 the client as it IS with every accessible name the live specs read, §3 cycle A's items marked, §4
the rules, §5 what B does not claim, §7 C's standing items).

**How this plan was built, and what is measured.** It is assembled from the retired single plan's drafts D6 (retired
Tasks 17–19), D7 (20–23) and D8 (24–28), under the gitignored
`.superpowers/sdd/2026-09-26-web-frontend-m6.5c2-plan/drafts/`, renumbered here (Appendix A is the map), and from the
spec's revision 2.7. **What the drafts measured, in `git archive` exports at `1e16fe27`:** D6 ran all three of its tasks
end to end over stand-ins for cycle A's layer — the two-project pool (two workerds, `--project open`), the refinish
race in both shapes (each RED for the right reason at the M6.5c code, `uploads.size === 2`), the `URLSearchParams`
round trip of `a+b@x.test` and `ben moore`, and two mutants (a conflict read through `stampArm`; the upgrade's
parameter read undecoded); D7 measured the rows a warm request reads (32, of which 30 are `hasTables`' scan of
`sqlite_master`; the memo takes it to 2), the scrub fence's compile errors (E0027, E0004) and TS7016; D8 measured
the bump's lock-file ripple (`cargo update --workspace --offline`: 9 lines; `npm version`: six lines, the third-party
`0.5.0` untouched), BuildKit's empty `ARG`, and that GitHub's `bash -e` swallows a red `cargo`'s output inside a
substitution. **What has changed since the drafts, each carried in its task:** cycle A landed the layer's exports
(`api::error_body` with its foreign-bot arm, `api::check_wait`, `signal::house_frame`, `games::HostClock`,
`platform::{CALLS, calls_in}`, `identity::admit`, `PutError::Conflict { held }`, `RunHost`, the log seam) and the
corpus's identity and seed tables; cycle B rebuilt every setup screen, so the accessible names the live specs read are
handover §2's; the `table.ts` split, the read-once thread, the replay's split and the `.dev.vars` module are DROPPED
(R-6.5c2.1 as amended), so every draft reference to `delivery.ts`, `saves.ts`, `guarded.ts`, `transport.ts`,
`dev-vars.mjs` and `replay/*.mjs` maps back to `table.ts`, `live-replay.mjs` and `live.ts` as they stand; and revision
2.7 moved the lane document, the top-level refresh, the screenshots, the merge and the tag to cycle D. **Unmeasured by
anyone at the seams**, and named in Review Focus: the claim's carriers against cycle A's REAL `resolve`, the held text
against A's real `archive_if_over`, the smoke script against the real image, and the Finished fold in a browser.

**A plan sized to be read whole.** The drafts' code is NOT restated where it defines no contract: each step names the
draft's section and line range that holds it and states what to CHANGE from the draft. Code appears inline only where
it defines a contract another task or a reviewer reads — the shared names, the cells' shapes, the guard's rules, the
smoke script's three claims, the close's sequence. The drafts are the implementer's raw material, never its authority:
where a draft and the tree at `3ed27f07` disagree, the tree wins, and where a draft and the spec at revision 2.7
disagree, the spec wins.

---

## Global Constraints

- **Branch `web-room-m6.5c2c`**, stacked on `web-room-m6.5c2b`; **the base for every code diff is `3ed27f07`**, cycle
  B's reviewed code head (`23735672` is B's close commit, documents only; `a855b379` is this branch's first commit, the
  spec's revision 2.7; this plan's commit is its second). Every `file:line` below was read at `3ed27f07` unless it
  says "D6/D7/D8 base", which means `1e16fe27` (the drafts' base). Cycles A and B moved most files the drafts cite:
  **find by content, never by number.**
  - Stage with explicit paths, never `git add -A`. `git status` is clean after every commit.
  - **Nothing is pushed and nothing touches the account until Task 13.** The push and PR #41 against
    `web-room-m6.5c2b`, the deploys of both hosts and the live tiers are **PRE-AUTHORIZED** (the owner, 2026-09-27:
    *"prior authorization for deployment both on the NAS and cloudflare, and for a push and PR at your discretion"*,
    held through B by R-B5 and spent here) — and every one of them is the ORCHESTRATOR's step in Task 13, never an
    implementer's.
- **Fences** (spec §5). At every commit this prints nothing:
  `git diff --stat 3ed27f07 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli`.
  `SEMANTICS_REV` stays **7**. **The layer moves in NAMED places only:** `crates/yokohama-web/src/bots.rs`,
  `api.rs`'s and `explain.rs`'s foreign-bot sentence sites, `games.rs`'s test module and `log.rs`'s print-scan list
  (Task 6); `lib.rs`'s test module (Task 9's version pin). `crates/yokohama-wasm/src/lib.rs` moves in Task 4 alone.
  A task that finds it needs any other layer change STOPS and asks the ORCHESTRATOR, who rules with a `Ruling:` line
  in the ledger.
- **The committed corpus is regenerated ONLY by Task 9, and only by the version**: `web/src/wire/health.json`,
  `web/src/wire-script/archive-walk.json`, `web/src/wire-script/backup-envelope.json`, each read as a diff of `0.5.0`
  → `0.6.0` and nothing else. No task adds, removes or otherwise changes a corpus file; `wire_fixtures` stays at
  **13 tests** and its file set unchanged. A task that believes it needs another corpus change is a FINDING for the
  orchestrator.
- **Per-edit gates after EVERY task. Re-count at your own HEAD; a figure in any document is a reading, not a
  constant. Counts below are DELTAS over your own HEAD's reading; the readings at `3ed27f07` are handover §1's:**
  web 339 (lib 320 + pin 4 + gate 2 + `wire_fixtures` 13), wasm 7, worker 64; `npm test` 1195 in 64, `test:wasm` 104
  in 9, e2e 68 in 13; `worker/` `test:scripts` 352 in 18, the pool 197 in 9.
  - **Rust:** `cargo test -p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`; `cargo clippy --workspace
    --all-targets -- -D warnings`; `cargo check -p yokohama-web --no-default-features --target
    wasm32-unknown-unknown`; `cargo test -p yokohama-web --no-default-features --features server --lib identity::`
    (8); `cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker`; `cargo doc -p
    yokohama-worker --no-deps` (0 warnings). Task 9 alone also runs the CLI, the oracle, difftest, the bridge and the
    two example targets (every crate compiles at the new version).
  - **`worker/`:** **`npm run build:module:debug` FIRST** whenever `crates/yokohama-worker` or the layer changed
    (Tasks 1, 3, 4, 6, 9), or the pool tests a stale module GREEN; then `npm run typecheck`, `npm run test:scripts`,
    `npm test`. **From Task 1 on, `npm test` runs BOTH pool projects** (`worker` over `wrangler.test.jsonc`, `open`
    over `wrangler.open.test.jsonc`); `npx vitest run --project open` runs one.
  - **`web/`:** `npm run typecheck`, `npm test`, `npm run test:wasm` (after `npm run build:wasm` when the module
    changed: Tasks 4, 6, 9).
  - **`cd web && npm run e2e`** is owed by every task that changes what a browser DOES or SHOWS — **Tasks 1** (the
    socket's URL), **10** and **11** — and by Task 9 for `offline.spec.ts` alone (it restores the regenerated
    envelope). SEQUENCED: never two Playwright runs at once. 6–8 min; run it **in the background with its output to a
    file in the ledger and POLL that file** (`tail -3` every minute or two) — **never a Monitor, never an event wait,
    never a re-arm** (an implementer stalled five hours on one in cycle A). It leaves the DEBUG module in
    `web/src/local/wasm/` and `web/dist/`; Task 13 rebuilds before any deploy. `plays_locally`'s quota cell is a
    KNOWN flake (a seed whose first option opens a sub-picker): re-run it alone, then believe the suite.
  - **A red whole-game gate** (`cargo test -p yokohama-web --test play_a_whole_game`) is a FINDING, not a re-run: it
    reads 2 passed in ~25 s since cycle A.
  - **Docker** (Task 8, and the battery's smoke row): Docker Desktop on this workstation; the `linux/amd64` build is
    EMULATED and long — background it to a file and poll. Check `df -h` first: **≥ 20 GiB free** before Task 8's
    build and before every e2e (a failed Playwright run writes a multi-GB trace).
- **The strings rule holds:** the shell never `JSON.parse`s a record or a table document; the held text (Task 3)
  travels as a string end to end and is compared in Rust. **The two time bases hold** (the shell's milliseconds; the
  module's seconds) — Task 5 changes a LISTING's unit, never a stored value.
- **Test data never uses the owner's surname or first name** (R-6.5c2.16): every test person is `Ben Castle` in some
  spelling, `Ada Bell`, or an address under `wire.test` / `e2e.test` / `t.test` / `readme.test`. The hostnames under
  `moore.nyc` are real and stay. **The owner's email address goes into no document**; a deployment reading is
  recorded with "the owner's address" in the value's place.
- **Every INVISIBLE non-ASCII code point in code is a `\u` escape** (an edit tool decodes a typed `\uXXXX` — three
  implementers hit it); a visible glyph may be raw. Task 12 states the rule once in the documents.
- **The ledger** is `.superpowers/sdd/2026-09-28-web-frontend-m6.5c2c-plan/` (gitignored). `progress.md` holds the
  SDD identity line first, one line per task and one `Ruling:` line per deviation from this plan. **Every agent writes
  its full report to a file there and replies with the PATH and three lines**; a reviewer's report is ordered
  Verdict → Issues → Assessment, each finding compact (`file:line`, defect, why, fix).
- **Commit messages** quote the task's red runs in the body and end with the EXECUTING agent's own attribution lines.
- **A reviewer's mutants run on an EXPORT** (`git archive HEAD | tar -x -C <dir>`; ONE shared `CARGO_TARGET_DIR` under
  the session's scratchpad for every Rust export), never in the tree an implementer holds; a mutated file is restored
  with `cp` and `touch`, never `mv` from an older copy. While an implementer works, nothing runs in its tree and no
  timer is armed.
- **Entry for the executing session.** Read, in order: `CLAUDE.md`; `docs/handover.md`'s "Updated" line; the spec's
  §10.7 and §4; the handover; this plan WHOLE; the ledger's `progress.md`. Confirm `git branch --show-current` prints
  `web-room-m6.5c2c`, `git status --short` prints nothing, and `git log --oneline -1` is at or after this plan's
  commit. Then Task 1 Step 0, which records the baseline every delta in this plan is read against.

---

## Review Focus

The spec implies these five inputs, and no cell would meet them unless someone put it there. Each line says what a
person would expect; its test lives in the owning task and is marked **(Review Focus N)** there.

1. **A claimed name carrying `+` or a space, over the WebSocket upgrade** (`a+b@x.test`, `ben moore`). Expected: the
   socket's caller is the same key the fetch header gives — `%2B` never becomes a space, and a space travels as `+`
   and comes back a space — against cycle A's REAL `resolve`, not a stand-in. (Task 1: `door.test.ts`'s Review Focus 1
   cell in the `open` project; `ws.test.ts`'s encoding cell.)
2. **A Worker deployed with no `YOKOHAMA_WEB_IDENTITY`.** Expected: every `/api/*` request answers the host-written
   500 naming the variable, and the static release is served — never a default door, never a platform 500. (Task 1:
   `worker.test.ts`'s cell with an env override at `handle`; Task 2: the deploy guard refuses the configuration.)
3. **A game-over conflict whose held text is NOT a record** — truncated, or another game's. Expected: the save is
   final, nothing retries hourly for ever, nothing throws inside an alarm, and a same-game conflict alone stamps.
   (Task 3: `settledArm`'s rows, the module's `held`-beside-a-conflict parse cell, and the two race cells.)
4. **The self-host image started without a door, and a NAME sent to it under `cf-access`.** Expected: the container
   exits naming the variable; under `cf-access` a `Cf-Access-Authenticated-User-Email` that is not an address reads as
   no one. (Task 8: `deploy/smoke-image.sh`'s step 1 and its `ben` probe, run on the locally built image.)
5. **A finished LOCKED table in the lobby's Finished group, seen by a stranger.** Expected: the row reads "View" with
   the padlock, and lands on the unlock face, never the game. (Task 10: the jsdom cell over a locked corpus table with
   `over: true` — one field moved, named.)

---

## Execution shape (the owner's standing answers)

- **Mode: SUBAGENT-DRIVEN, EVERY TASK** (the owner, 2026-09-21, standing; R-6.5c2.18). Each task gets a fresh **Opus**
  implementer and a fresh **Opus** task reviewer; **Fable** does the whole-branch review in Task 13. Folding two small
  tasks into one execution unit is the orchestrator's call (spec §9); each task keeps its own green commit either way.
- Tests are written first and RUN RED; the red run is quoted in the commit body and in the ledger. Every commit passes
  every per-edit gate.
- Standing process rules: never follow up an implementer's DONE without reading HEAD first; a reviewed SHA is never
  amended — a fix is a NEW commit, and the acceptance reply that asks for it says "as a new commit"; while an
  implementer works, run nothing in its tree and arm no timers; wake a stalled agent by message rather than spawning
  another; check the ledger within ~15 minutes of any long run; a reviewer's mutants run on an export.
- **The orchestrator edits no code.** Its steps are Task 1 Step 0's baseline reading, the pictures' reading at Tasks
  10 and 11, and Task 13 whole: the review dispatches, the battery, the ONE sequence, the deploys, the secret, the
  bucket, the live tiers, the close document, the push and PR #41 (pre-authorized), the handover into D, the board
  and the memory.
- **Rulings at planning (2026-09-28, the orchestrator, under the owner's rulings R-6.5c2.20–.27 in spec §10.7):**
  - **R-C1:** the spec's twelve tasks are THIRTEEN here — its item 5 "the small debts, shell and layer" is two tasks
    (5 the shell's, in TypeScript; 6 the layer's, in Rust), because one reviewer's gate should not straddle two
    languages and two crates' suites.
  - **R-C2:** the base for every diff is `3ed27f07`; the spec's revision 2.7 (`a855b379`) and this plan are the
    branch's first two commits.
  - **R-C3:** with `dev-vars.mjs` and the replay's split DROPPED, L-9's `awaitCreate`/`abandonOpened` live in
    `web/e2e/live.ts` as it stands, and L-5's presence rule is stated in `worker/test/corpus.test.ts` beside the live
    replay's own statement — two statements, accepted under R-6.5c2.1 as amended, and each named as the other's twin.
  - **R-C4:** with the read-once thread DROPPED, L-8 is the one-row existence query the spec names: `hasDoc(sql,
    kind)` in `storage.ts`, and `deleting` reads no document of its own.
  - **R-C5:** the versioned refusal sentence's shape (spec §2.4 as amended): where two labels tie, each gains
    `.<controller version>` (`V4.2`); where those tie too (a knob-only difference, which only a grandfathered seat
    the web never deals could show), each gains its parameters rendered `k=v,…` in parentheses.
  - **R-C6:** the Finished group's DOM is `section.lobby-section.finished-tables` holding ONE `details.fold` whose
    `summary` carries the `h3` "Finished" and the count; it is rendered only when a finished game the caller does not
    sit at exists (an empty fold is clutter); its rows are `li` rows whose one link is the table's name, as the other
    three lists' are.
  - **R-C7:** the Finished group's browser witness rides `web/e2e/plays_a_whole_game.spec.ts`, whose first cell already
    plays a game to a winner: a second identity then opens the lobby at 1440 and 390.
  - **R-C8:** the two-defect probe cycle A said `events` lacks is ONE line in the module's
    `events_refuses_in_axums_handler_order`: `wait=26` from a caller with NO SEAT at an existing table answers the
    cap's 400, never the admission's 403 (the cap before the admission).
  - **R-C9:** Task 8's Docker workflow triggers as spec §2.9 as amended rules — pull requests, master pushes and
    dispatch over the web-lane inputs only — and Task 13 records its first walls; the trigger drops to master and
    dispatch if a cold build reads over fifteen minutes.

---

## Names shared across tasks

An implementer sees only its own task. **These names are the contract between tasks and are used verbatim.** A task
that finds the code forces a different shape STOPS and asks the ORCHESTRATOR, who amends this table with a `Ruling:`
line. It never renames on its own.

### What cycles A and B produced, consumed here as it IS (read off the tree, never off this table)

| name | shape | where |
|---|---|---|
| `yokohama_web::identity::{IdentityMode, resolve, admit, User, CLAIMED_HEADER, WORKER_HEADER, CF_ACCESS_HEADER}` | `IdentityMode::{Open, CfAccess, Worker}` with `name()` (`"open"`, `"cf-access"`, `"worker"`) and `header()`; `resolve(mode, &HeaderMap) -> Result<User, String>`; `admit(mode, user)`: under `Open` every key, under the other two an email or the ABSENT sentence; `User::parse`, `User::is_email`, `User::as_str` | Tasks 1, 2 |
| `yokohama_web::api::{error_body, error_value, NOT_AN_OBJECT, MAX_WAIT_SECONDS, check_wait}` | `error_body(&ApiError) -> String` (three arms: `status`, `foreign_bot`); `check_wait(Option<u64>) -> Result<(), ApiError>` | Task 4 |
| `yokohama_web::signal::house_frame(seq: u64, chat: u64) -> String` | `{"seq":…,"chat":…}` | Task 4 |
| `yokohama_web::games::{HostClock, RunHost}`; `Games::with_parts(rules, store, chat, signal, clock, archive, run_host)`; `Games::with_log` | `HostClock::{new (const), set, now(live), tick}`; `RunHost::{Server, Worker, Browser}` with `run_id()` | Tasks 4, 6 |
| `yokohama_web::platform::{CALLS, calls_in}` | the eight patterns; `calls_in(file, text) -> Vec<String>` | Task 4 |
| `yokohama_web::archive::PutError::Conflict { key, held: Option<String> }`; `Games::archive_if_over`'s same-game rule | at `Why::Over`, a `Conflict` whose `held` parses to a record with the current record's `seed`, `config` and `choices` stamps as `Landed`; `None`, a text that does not parse, or another game's record stamps nothing | Task 3 |
| `yokohama_web::bots::{BotDescriptor, label, label_of, production, recorded, foreign, cannot_continue, ForeignBot}` | `label(&BotDescriptor) -> String` ("V4"); `foreign(record) -> Vec<(u8, BotDescriptor)>`; `cannot_continue(played, runs) -> String` | Task 6 |
| `yokohama_web::client::without_version_drift`; the fixture pair `crates/yokohama-web/tests/fixtures/browser-played-2p.{json,table.json}` at `engine_version` `0.5.0` | the drift filter, and the pin that goes LIVE at the bump | Task 9 |
| the module: `init_inner(config)`, `dispatch_inner`, `identity_of`, `identity_inner`, `events_inner`, `from_the_layer(call)`, `ReplyData { status, body, bumps, archive, picker, dropped, logs }`, `host_refusal`, `error_body` (the module's copy), `now()`/`set_clock`, `HOST`; the test helpers `config(archive)`, `r2(caller)`, `serial()` | as at `3ed27f07` | Tasks 1–4 |
| `worker/src/`: `ShellConfig`, `shellConfig`, `moduleConfig`, `ROWS`; `Identity` (`person` \| `dev` \| `none`), `resolveIdentity`, `forwardable`, `asciiLower`, `isAddress`; `operatorGate(admins, identity)`; `executePlan`, `Outcome`, `Plan`, `Stamp`, `stampArm`, `parseStamp`, `logOutcome`; `Table.settleUpload`, `Table.deleting`, `Table.end`, `Table.force`, `House.listing`; `storage.ts`'s `hasTables`, `readDocs`, `getMeta`, `setMeta`, `MetaKey` | as at `3ed27f07` | Tasks 1–5 |
| `worker/test/helpers.ts`: `RED`, `BLUE`, `LOCAL`, `OWED`, `call`, `tableStub`, `seedPlaying`, `settle`, `owedGameOver`, `holding`, `docOf`, `bucketText`, `logsDuring`, `backdate`, `markerOf`, `DAY_MS`, `operate`, `archiveOff`; `worker/test/gates.ts`: `swapBucket`, `heldPuts`, `failingPuts`, `holdApply`; `worker/test/walk.ts`: `WALK`, `WALK_HOST`, `ARCHIVE_WALK`, `walk` | the pool's harness | Tasks 1, 3, 5, 7 |
| `web/src/api.ts`: `claimedUser`, `identityHeaders`, the module-private `identityMode()` (the value `useIdentityMode` returns), `serverFacts`, `ApiError` | | Task 1 |
| `web/src/tables.ts`: `type Section = "mine" \| "open" \| "playing"`, `rowMeta`, `rowAction`, `RowAction`, `seatChips`, `tableWord`; `web/src/Lobby.tsx`'s three sections `section.your-games` / `.open-tables` / `.playing-tables`, its `row(t, section)`; `styles.css`'s `details.fold` rules (R-B16) | the lobby as B left it | Task 10 |
| `web/e2e/setup_screens.spec.ts`: `DESKTOP`, `PHONE`, `noSidewaysScroll`, `look`; `web/e2e/harness.ts`: `signIn`, `nameThisBrowser`, `watchForBreakage`, `WALKER`; `web/e2e/live.ts`: `LIVE_ORIGIN`, `credential`, `liveRequest`, `liveContextOptions`, `abandonAsCreator`; `web/e2e/sw.ts`: `test`, `expect` | the browser harness | Tasks 7, 10, 11 |
| B's accessible names (handover §2) | "Playing as <name>", "switch player" / "change name", `.identity-pill`; "Your tables" / "Open tables" / "Being played"; row buttons `"<label>: <table name>"`; "Join as <Colour>", "Table password", "Invite link", "Copy link" → "Copied"; "You are spectating"; "How to play", "Close" | Tasks 10, 11 |

### What cycle C produces (the contract between its own tasks)

| name | shape | task |
|---|---|---|
| `Env.YOKOHAMA_WEB_IDENTITY?: string` (first in `ROWS`); `type Door = "worker" \| "open"` (`config.ts`); `ShellConfig.door: Door` (its FIRST field); `moduleConfig` sends `"identity"` as the init object's first key | the shell's door | 1 |
| `worker/src/identity.ts`: `Identity` gains `{ kind: "claimed"; name: string }`; `CLAIMED_USER_HEADER = "X-Yokohama-Claimed-User"`; `CLAIMED_PARAM = "claimed"`; `claimedIdentity(raw: string \| null): Identity` (forwardable → trimmed, ASCII-lowercased, else `none`); `callerName(identity): string \| undefined` | the open-door claim | 1 |
| `worker/src/table.ts`: `forwardedCaller(headers, config): string \| undefined` — `X-Yokohama-User` under `worker`, `X-Yokohama-Claimed-User` under `open`, `undefined` with a refused configuration; read by `table.ts`, `house.ts`, `archive.ts` | the objects' one read of the caller | 1 |
| the module: `config::Door { Worker, Open }` with `mode() -> IdentityMode` and `wire_name()`; `config::IDENTITY_VAR`; `Config.door`; `lib.rs`'s `DOOR` static, `door()`, `host()` (replacing `HOST`); the six-key init sentence; the test helper `door_config(archive, door)` | the module's door | 1 |
| `web/src/api.ts`: `identityParams(): Record<string, string>` — `{ claimed: <stored name> }` under `open`, else `{}`; `ws.ts` sets it on the upgrade URL through `URLSearchParams` | the client's parameter | 1 |
| `worker/wrangler.open.test.jsonc`; `worker/vitest.config.ts`'s `test.projects` `worker` (excludes `test/open/**`) and `open` (only `test/open/**/*.test.ts`); `worker/test/open/door.test.ts` with its helpers `through(path, sent)`, `claimedAt(path, name)`, `socket(res)`, `reaching(f)`, `yokohamaHeaders(req)`, `noCaller()` | the two pool projects | 1 |
| `config.ts`: `openDoorRefusals(c)` (the two sentences, verbatim in Task 2); `operatorGate(door: Door, admins, identity)`; `deploy-guard.mjs` exports `doorLine(root): string \| undefined` beside `configFailures` and `deployGuard`; `receipt.rs`'s `is_test_identity(address)` and the test helper `r2_at(caller, door)` | the refusals | 2 |
| `r2.ts`: `Outcome` gains `held?: string`; `settledArm(outcome, stamp \| undefined): "stamped" \| "no-longer-over" \| "retry" \| "final"`; `receipt.rs`'s `Outcome.held: Option<String>` (`#[serde(default)]`, refused beside anything but a conflict); `worker/test/gates.ts`'s `heldInTurn()` | the refinish on the Worker | 3 |
| the module: `pub(crate) enum Scope<'a> { Dispatch, Table(&'a str) }`; `from_the_layer(scope, call)`; `picker::watching(games, id, call) -> (T, Option<String>)`; `picker::clear_if`; `receipt.rs`'s `SECOND_PLAN`; `house.rs`'s `scrub` naming `SeatDoc`, `TurnMirror` and `BotDescriptor`; `worker/scripts/module-figures.mjs` (`moduleFigures(bytes) -> { bytes, gzip9, pages, segments, dataBytes, dataSection }`) and the root `Cargo.toml`'s Worker paragraph | the invariants, the figures | 4 |
| `storage.ts`: `forgetTables(sql)`, `hasDoc(sql, kind): boolean`, `MetaKey` + `"forced"`; `house.ts`'s listing in unix SECONDS; the log line `removal-ended {id}`; `worker/test/reads.test.ts` | the shell's debts | 5 |
| `bots::distinguishing_labels(played: &BotDescriptor, runs: &BotDescriptor) -> (String, String)` | the versioned sentence | 6 |
| `web/e2e/harness.ts`: `DEAL_ANSWERS_PER_PAGE = 1`; `web/e2e/live.ts`: `type Opening = { id: Promise<string \| null> }`, `awaitCreate(page): Opening`, `abandonOpened(opening: Opening \| null)`; `worker/scripts/read-deploy.mjs`: `deployedRelease(base, pairs, fetcher = fetch)` exported, `answerShape(status, contentType, bytes)`; `worker/test/corpus.test.ts`'s `hostFields`/`withoutHostFields` | the LOWs | 7 |
| `deploy/smoke-image.sh <image> [port]` (mode 755); `.github/workflows/docker.yml` job `image`; `.github/workflows/test.yml` jobs `test`, `web-server`, `worker`, `web-client` | build and CI | 8 |
| `the_two_npm_packages_carry_the_workspace_version` (`crates/yokohama-web/src/lib.rs`); the workspace and both npm packages at `0.6.0` | the bump | 9 |
| `tables.ts`: `Section` gains `"finished"`; `rowAction(t, "finished")` → `View`; `Lobby.tsx`'s `section.lobby-section.finished-tables` > `details.fold` > `summary` (`h3` "Finished" + the count) | the fold | 10 |
| `worker/RUNBOOK.md` with `§N` = the old `deploy/RUNBOOK.md §6.N`, deployments recorded from `§15`; `deploy/RUNBOOK.md` §6 as the relocation map; `deploy/README.md` with the anchors `#1-access-first-the-repositorys-default` and `#2-the-open-door-a-home-network-a-lan-or-a-vpn`; `worker/README.md` `#15-the-two-doors-access-or-an-open-door` | the documents | 12 |

---

## File structure

**Created**
- `worker/wrangler.open.test.jsonc`, `worker/test/open/door.test.ts` (Task 1)
- `worker/test/reads.test.ts` (Task 5)
- `worker/scripts/module-figures.mjs`, `module-figures.test.mjs` (Task 4)
- `deploy/smoke-image.sh`, `.github/workflows/docker.yml` (Task 8)
- `deploy/README.md`, `worker/RUNBOOK.md` (Task 12)
- `docs/plans/<the day>-web-frontend-m6.5c2c-close.md`, `docs/plans/<the day>-web-frontend-m6.5c2d-handover.md`
  (Task 13)
- in the ledger (gitignored): `task-<n>-look/*.png` for Tasks 10 and 11, `task-<n>-e2e.txt` for every e2e run,
  `task-8-docker-build.log`, `close-sequence.sh`, `sequence.tsv`, `battery.md`, the review reports

**Modified** (principal files; each task's own list is the authority)
- `crates/yokohama-worker/{Cargo.toml, src/config.rs, src/lib.rs, src/events.rs, src/receipt.rs}`; `worker/src/{env.ts,
  config.ts, identity.ts, index.ts, table.ts, house.ts, archive.ts}` and their `.unit.test.ts`; `worker/wrangler.jsonc`,
  `wrangler.test.jsonc`, `vitest.config.ts`; `worker/test/{worker,factory,table}.test.ts`; `worker/scripts/smoke-release.*`;
  `web/src/{api.ts, ws.ts, ws.test.ts}` (Task 1)
- `worker/src/{config.ts, admin.ts, index.ts, env.ts}`, `worker/wrangler.jsonc`, `worker/scripts/deploy-guard.{mjs,test.mjs}`,
  `crates/yokohama-worker/src/receipt.rs`, `worker/test/{open/door,worker}.test.ts` (Task 2)
- `worker/src/{r2.ts, r2.unit.test.ts, table.ts}`, `crates/yokohama-worker/src/receipt.rs`, `worker/test/{gates.ts,
  archive.test.ts}` (Task 3)
- `crates/yokohama-worker/src/{lib.rs, picker.rs, events.rs, receipt.rs, house.rs}`, `crates/yokohama-wasm/src/lib.rs`,
  `Cargo.toml` (root), `worker/src/table.ts` (`copyOf`'s doc), `worker/README.md` §6.3 (Task 4)
- `worker/src/{storage.ts, table.ts, house.ts, admin.ts}`, `worker/test/{operator,slot}.test.ts` (Task 5)
- `crates/yokohama-web/src/{bots.rs, api.rs, explain.rs, games.rs, log.rs}` (Task 6)
- `worker/scripts/read-deploy.{mjs,test.mjs}`, `worker/test/corpus.test.ts`, `web/e2e/{harness.ts, live.ts,
  two_browsers,joins_by_link,exit_game}.spec.ts`, `web/e2e/{harness,live}.node.test.ts` (Task 7)
- `deploy/Dockerfile`, `deploy/Dockerfile.runtime`, `.github/workflows/test.yml` (Task 8)
- `Cargo.toml`, `Cargo.lock`, `web/package*.json`, `worker/package*.json`, `crates/yokohama-web/src/lib.rs`, the three
  corpus files, `deploy/Dockerfile.runtime`, `web/e2e/plays_locally.spec.ts` (Task 9)
- `web/src/{tables.ts, Lobby.tsx, Help.tsx, styles.css}` and their tests; `web/e2e/plays_a_whole_game.spec.ts`,
  `web/e2e/setup_screens.spec.ts` (Task 10)
- `web/src/{App.tsx, OnThisDevice.tsx, OpenTable.tsx, Lobby.tsx, GameMenu.tsx, FrontDoor.tsx, Band.tsx, NewGame.tsx,
  styles.css}` and their tests; `web/e2e/offline.spec.ts` (Task 11)
- `deploy/RUNBOOK.md`, `deploy/cloudflared-config.yml`, `worker/README.md`, `README.md`, `docs/handover.md`,
  `guidingpath.md` (one row), the two frozen plans (the exception's two lines), `web/src/whoami.ts` (no code: its
  sentence is made true by `deploy/README.md`'s existence) (Task 12)
- `deploy/RUNBOOK.md` §3.17, `worker/RUNBOOK.md` §15–§17, `CLAUDE.md` (the truth pass), `guidingpath.md` (the cycle
  row), `docs/handover.md`, the spec's §10.8 (Task 13)

---

## Task order and why

```
1 ─► 2 ─► 3 ─► 4 ─► 5 ─► 6 ─► 7 ─► 8 ─► 9 ─► 10 ─► 11 ─► 12 ─► 13 (the close, ORCHESTRATOR)
```

1. **1 first:** the door variable, the `claimed` kind, `forwardedCaller` and the `open` pool project are what Task 2's
   refusals and cells read; Task 1 also carries the cycle's baseline (Step 0).
2. **2 before 3:** both edit `receipt.rs`; 2's `is_test_identity` and `r2_at` land before 3 changes `Outcome`.
3. **3 before 4:** 4's `Scope` re-threads every `from_the_layer` call in `receipt.rs`, `events.rs` and `picker.rs`,
   and 4's one-plan rule edits `put` — after 3's `held` has landed in the same functions.
4. **4 before 5:** 5's `reads.test.ts` is the pool's; 4's module changes are built first (`build:module:debug`).
5. **5 and 6 and 7** depend on nothing later; 6 is Rust in the layer, 7 is node cells — each after 5 so the shell's
   files are quiet.
6. **8 before 9:** the bump's commit is the first one the reshaped CI runs against (at Task 13's push); 8's smoke
   script is measured on an image built at 0.5.0 and re-run by `docker.yml` at 0.6.0.
7. **9 before 10 and 11:** the client's one e2e literal (`plays_locally.spec.ts`) moves with the bump; the client
   tasks then run the walk over a corpus whose `server_version` is 0.6.0.
8. **10 before 11:** both edit `Lobby.tsx` and `styles.css`; 10 adds the fourth section, 11's "Copied" fix finds the
   copy handler by content.
9. **12 after every code task:** the documents describe the final code, and the NAS recipe reads `cf-access` before
   Task 13 follows it.
10. **13 last.**

---

## Couplings between tasks (binding; the later task's implementer does what its item says)

- **Task 1 → Task 2: `Door`, `ShellConfig.door`, the `claimed` kind, `callerName`, `door_config`, the `open` project
  and `door.test.ts`'s helpers.** Task 2 appends a describe to `door.test.ts` and edits `shellConfig`'s body, which
  Task 1 wrote with `door` as its first field; Task 2 inserts `openDoorRefusals(config)` between the object and the
  `return`.
- **Task 1 → Task 3: `r2(caller)` gains `"identity": "worker"`.** Task 3's `receipt.rs` cells call `r2` as Task 1
  leaves it, and Task 2's `r2_at(caller, door)` where a door matters.
- **Task 1 → Task 4: `config(archive)` delegates to `door_config(archive, "worker")`.** Task 4's picker and one-plan
  cells call `config("off")` / `r2(...)` as they stand; they never build an init JSON of their own.
- **Task 1 → Task 8: BOTH pool projects run in CI's `worker` job** (`npm test` runs both). The job's comment says so.
- **Task 2 → Task 8: the guard's per-door rules** are what `npm run test:scripts` runs in that job; the pool's second
  configuration is what check 5 and 6 read alongside the first.
- **Task 3 → Task 4: `receipt.rs`'s `put`** carries Task 3's `held` in its `Conflict` arm; Task 4's `SECOND_PLAN`
  edits the `pending` write BELOW that arm. Both edit `parse_outcome`'s neighbourhood: find by content.
- **Task 3 → Task 5: `settleUpload`'s arms.** Task 5 edits `deleting` and `end`; Task 3 edits `settleUpload`. Same file,
  different functions.
- **Task 4 → Task 12: the figures' ONE place** (the root `Cargo.toml`) is what `worker/README.md` §6.3 cites; Task 12
  checks the citation survived and does not restate a number.
- **Task 5 → Task 12: the listing's seconds and the `forced` / `removal-ended` lines** are the vocabulary
  `worker/RUNBOOK.md` §5 (the old §6.5) and `worker/README.md` §10.2 carry; Task 12 edits both.
- **Task 6 → nobody, but the corpus:** the versioned sentence changes NO committed payload (the sample is V2 vs V4);
  Task 6 asserts `git diff --stat -- web/src/wire` prints nothing.
- **Task 8 → Task 9: `deploy/Dockerfile.runtime`.** Task 8 rewrites its identity sentence (`:44-52` at the D8 base);
  Task 9 changes its example tag (`:18`). Find by content.
- **Task 9 → Task 13: the live replay's reset recipe is OWED** (trap 19): `archive-walk.json`'s records carry
  `0.6.0`, so the bucket's four `test/browser/red_at_wire.test/…` keys go before the first live send.
- **Task 9 → Tasks 10, 11: the corpus's `health.json` reads `0.6.0`.** Nothing a screen shows changes; the fake Worker
  serves it.
- **Task 10 → Task 11: `Lobby.tsx`.** Task 10 adds the `finished-tables` section after `.playing-tables` and filters
  `playing` by `!t.over`; Task 11's copy-refusal fix edits the `copy` handler (`setCopied` / `setError`). Task 11 finds
  by content and leaves Task 10's section alone.
- **Task 10 → Task 11: `Help.tsx`.** Task 10 rewrites the "Being played" sentences for the fold; Task 11 touches
  `Help.tsx` nowhere.
- **Task 12 → Task 13: the NAS recipe** (`deploy/RUNBOOK.md` §1–§3 as Task 12 leaves it) is what E.6 follows;
  `worker/RUNBOOK.md` §15 is where the twenty-first deployment is recorded; `deploy/RUNBOOK.md` §3.17 the fourteenth
  NAS deployment.
- **Every task → Task 13: the battery.** Each task's report names its catchers by cell name; Task 13's battery rows
  cite them.

---

### Task 1: The Worker's two doors (spec §2.1 "The Worker's two doors"; R-6.5c2.5; review R2.2-M2, R2.2-m1, R2.2-m2, R2.2-m3, R2.2-m6; Review Focus 1, 2) — from D6 retired Task 17

**Files:**
- Modify, the module: `crates/yokohama-worker/Cargo.toml` (the layer dependency gains `features = ["open-door"]`);
  `src/config.rs` (the `identity` key FIRST, `IDENTITY_VAR`, `Door`, `Config.door`, the six-key sentence);
  `src/lib.rs` (the import drops `IdentityMode` and `WORKER_HEADER` as fixed values; a `DOOR` static and `door()`
  after `SIGNAL`; the `HOST` const becomes `fn host()` whose `identity_name` is the door's wire name; `init_inner`
  sets the door; `identity_of` reads `door().mode()` in BOTH arms; `dispatch_inner` passes `&host()`; the test
  module's `config` delegates to `door_config`, the two init literals gain `"identity":"worker"` first, the array
  cell's sentence names six keys; three new cells); `src/events.rs` (its doc names `host()`); `src/receipt.rs` (the
  test helper `r2` sends `"identity": "worker"` first).
- Modify, the shell: `worker/src/env.ts` (`YOKOHAMA_WEB_IDENTITY?: string` before `ACCESS_TEAM_DOMAIN`);
  `worker/src/config.ts` (`Door`, `ShellConfig.door` first, `ROWS`, `door()`, `shellConfig`, `moduleConfig` sends
  `identity` first); `worker/src/identity.ts` (`Identity` + `claimed`; `CLAIMED_USER_HEADER`, `CLAIMED_PARAM`; the
  dev door reads the header through its constant; `claimedIdentity`, `callerName`); `worker/src/index.ts` (the
  imports; doc steps 5 and 6; the caller BY DOOR; the ONE header set; the archive object named through
  `callerName`); `worker/src/table.ts` (`forwardedCaller` beside `USER_HEADER`, the attachment's doc, the caller
  read); `worker/src/house.ts` and `worker/src/archive.ts` (the caller read through `forwardedCaller`);
  `worker/scripts/smoke-release.mjs` and its test (the smoke's configuration names the worker door).
- Modify, the configurations: `worker/wrangler.jsonc` and `worker/wrangler.test.jsonc` (each gains
  `"YOKOHAMA_WEB_IDENTITY": "worker"` FIRST in `vars`, with D6's comments); `worker/vitest.config.ts` (two projects).
- Create: `worker/wrangler.open.test.jsonc`; `worker/test/open/door.test.ts`.
- Modify, the pool: `worker/test/worker.test.ts` (+2 cells); `worker/test/factory.test.ts` and
  `worker/test/table.test.ts` (each configuration literal gains `identity: "worker"`).
- Modify, the node cells: `worker/src/config.unit.test.ts` (`env()`; a door describe; the not-a-string row; the
  order cell; the module configuration); `worker/src/identity.unit.test.ts` (two describes);
  `worker/src/marker.unit.test.ts` (the `ShellConfig` literal gains `door`, or `npm run typecheck` fails).
- Modify, the client: `web/src/api.ts` (`identityParams()` after `identityHeaders`); `web/src/ws.ts` (the doc, the
  import, `readSocket`'s doc, the URL); `web/src/ws.test.ts` (a describe).

**Interfaces:**
- Consumes: cycle A's `IdentityMode::{Open, Worker}`, `admit`, `resolve`, `CLAIMED_HEADER`, `WORKER_HEADER`, the
  mode rule (a plain name under `Worker` reads as ABSENT in the absent sentence), `User::parse`; the client's
  `claimedUser`, `identityHeaders`, the module-private `identityMode()`.
- Produces (verbatim, the shared names): `Env.YOKOHAMA_WEB_IDENTITY`; `Door`; `ShellConfig.door`; `Identity`'s
  `claimed` kind; `CLAIMED_USER_HEADER`, `CLAIMED_PARAM`, `claimedIdentity`, `callerName`; `forwardedCaller`;
  `config::Door`, `IDENTITY_VAR`, `DOOR`, `door()`, `host()`, `door_config`; the init object's `"identity"` key;
  `identityParams`; `wrangler.open.test.jsonc`; the two projects; `door.test.ts`'s helpers.

**Why the objects read the door's OWN header** (D6, kept): an object hands the module a raw `user` string, and the
module's `identity_of` puts it under the header the door's mode reads; the header between the shell and an object is
the shell's own convention. Reading it by door (`forwardedCaller`) buys one thing: were the Worker and an object ever
to disagree about the door, a claim would reach NO caller rather than be read by the `worker` mode as a verified
identity. Every client `x-yokohama-*` header is still stripped.

- [ ] **Step 0 (ORCHESTRATOR, before Task 1's dispatch): the cycle's baseline.** At `HEAD` (this plan's commit), run
  every per-edit suite of Global Constraints ONCE, record each count in `progress.md` with the machine tag and the
  date, and `df -h` (≥ 20 GiB). Every delta in this plan is read against this line. Also confirm the two BGA captures
  are present (`bridge-captures/895724349.ndjson`, `896199544.ndjson`), which Task 9's CLI run needs.

- [ ] **Step 1: The module's cells** — `drafts/D6-tasks-17-19.md:209-320`: replace the test helper `config` with the
  `config`/`door_config` pair; add `"identity":"worker",` as the first key of the two init literals; make the array
  cell's expected sentence the six-key one ("…is the object of its six keys — identity, archive, caller, lobby_chat,
  chat_keep and chat_text"); add the three cells `the_open_door_resolves_a_claimed_name_and_names_itself_open`,
  `init_takes_the_workers_two_doors_and_no_default` and
  `a_value_no_header_can_carry_is_judged_under_the_doors_own_mode` after the health cell. **Change from the draft:**
  the test person is `"  Ben   Castle "` → `ben castle`, never a surname of the owner's. In `receipt.rs`, `r2` gains
  `"identity": "worker"` as its `json!`'s first key. Run `cargo test -p yokohama-worker`: every cell that calls
  `init_inner` panics on `unknown field `identity``, and the array cell on the five-key sentence. Quote it.

- [ ] **Step 2: The module takes the door** — `drafts/D6-tasks-17-19.md:331-592`, as drafted, with these
  corrections: `identity_of`'s fallback arm reads `admit(door().mode(), user)` and its header arm
  `resolve(door().mode(), &headers)` — cycle A's `admit` is the function (the draft's `Task 2` stand-in is the tree's
  real `identity::admit`); `host()` builds `Host { identity_name: door().wire_name(), signal_kind: "ws", serves_games:
  true, multiplayer_offered: true }`; `init_inner` stores the door BEFORE it builds the `Games` and only after
  `config::parse` accepted the configuration (a refused init changes nothing — the second cell holds it). Run:

```bash
cargo test -p yokohama-worker && cargo clippy -p yokohama-worker --all-targets -- -D warnings && cargo doc -p yokohama-worker --no-deps
```

  Expected: green (+3), clippy clean, 0 doc warnings.

- [ ] **Step 3: The shell's node cells** — `drafts/D6-tasks-17-19.md:602-718`: `config.unit.test.ts`'s door describe
  (`worker` and `open` accepted; ` Open ` folded as every switch is; `cf-access` and `claimed` refused naming both
  values — `YOKOHAMA_WEB_IDENTITY must be worker or open — got "cf-access"`; ABSENT refused naming the variable and
  that there is no default; the not-a-string row; the order cell: the door is the FIRST field refused, before the
  archive); the module-configuration cell (`identity` first); `identity.unit.test.ts`'s two describes
  (`claimedIdentity`: `" Ben  Castle "` → `{kind: "claimed", name: "ben  castle"}` — trimmed and ASCII-lowercased,
  internal spaces LEFT (the layer collapses them; the shell judges what a header can carry and no more); a Kelvin
  sign, a `\r`, a no-break space → `none`; `null` → `none`; `callerName` over the four kinds); `marker.unit.test.ts`'s
  literal gains `door: "worker"`. Run `cd worker && npx vitest run -c vitest.scripts.config.ts src/config.unit.test.ts
  src/identity.unit.test.ts`: red on every new cell. Quote it.

- [ ] **Step 4: The shell reads the door and the claim** — `drafts/D6-tasks-17-19.md:732-868`, as drafted. Green:
  `npm run test:scripts` (+N, counted) and `npm run typecheck` (the marker literal).

- [ ] **Step 5: The pool's two projects, and their cells** — `drafts/D6-tasks-17-19.md:876-1300`:
  - `wrangler.jsonc` and `wrangler.test.jsonc` gain the door FIRST in `vars`, with the draft's comments;
  - `wrangler.open.test.jsonc` as the draft gives it (`name: "yokohama-web-open-test"`; the same assets block, object
    classes, migrations and bucket; `vars` of five: the door `open`, the archive, the TTL, the test clock, the wakes —
    NO Access variables and NO dev door);
  - `vitest.config.ts` whole, as the draft measured it (`test.projects`: `worker` over `wrangler.test.jsonc` with
    `exclude: [...configDefaults.exclude, "test/open/**"]`; `open` over `wrangler.open.test.jsonc` with `include:
    ["test/open/**/*.test.ts"]`; both through `cloudflareTest`);
  - `test/open/door.test.ts` with its helpers (`through`, `claimedAt`, `socket`, `reaching`, `yokohamaHeaders`,
    `noCaller`) and four cells: a claim by the header on a fetch is the caller, no assertion read, the objects handed
    it under `X-Yokohama-Claimed-User` alone; **(Review Focus 1)** `a+b@x.test` and `ben castle` reach a table's
    socket exactly as the fetch caller (`claimedAt` builds the URL through `URLSearchParams`); a claimed parameter no
    header can carry (a non-ASCII value) is no caller — the layer's own 401, never a platform 500; the claim rides
    ONE carrier per request (the header on a fetch, the parameter on an upgrade, never the other);
  - `worker.test.ts` gains **(Review Focus 2)** "an unset `YOKOHAMA_WEB_IDENTITY` answers 500 naming it on every
    `/api/*` path" (an env override at `handle`: `{...env, YOKOHAMA_WEB_IDENTITY: undefined}`; the static release
    still served) and "under `worker` an upgrade's claimed parameter is no caller, and never outranks a person";
  - `factory.test.ts` and `table.test.ts`'s configuration literals gain `identity: "worker"`;
  - `smoke-release.mjs` and its test name the worker door.

  **Change from the draft:** the test people are `ben castle` and `a+b@x.test`; nothing else. Then
  `npm run build:module:debug && npm test`: the `open` project's cells red (the shell forwards no claim yet), the two
  `worker.test.ts` cells red. Quote both.

- [ ] **Step 6: The Worker carries the claim, and the objects read it by door** — `drafts/D6-tasks-17-19.md:1317-1396`
  as drafted (`index.ts`'s caller by door; the one header set through `callerName`; the archive object named through
  it; `forwardedCaller` in `table.ts`, imported by `house.ts` and `archive.ts`; the two attachment docs). Green:
  `npm test` (both projects, +6), `npm run typecheck`, `npm run test:scripts`.

- [ ] **Step 7: The client's cells** — `drafts/D6-tasks-17-19.md:1405-1453` in `ws.test.ts`: under `open` with a stored
  name the upgrade's URL carries `claimed=` with `a+b@x.test` as `a%2Bb%40x.test` and `ben castle` as `ben+castle`;
  under `cf-access` and `worker` no parameter; with no stored name none. **Change from the draft:** the mode is read
  from `serverFacts().identity` through the module-private `identityMode` — export a `identityParams()` from `api.ts`
  that reads it, and stub `serverFacts` as `api.test.ts`'s cells do. Red: `npx vitest run src/ws.test.ts`. Then build —
  `drafts/D6-tasks-17-19.md:1461-1507`: `identityParams` in `api.ts` (after `identityHeaders`, with the draft's doc:
  one stored name, one statement of it; nothing under any other door); `ws.ts`'s doc paragraph, its import, and
  `for (const [key, value] of Object.entries(identityParams())) url.searchParams.set(key, value);` before `new
  WebSocket(url.href)`. Green: `cd web && npm run typecheck && npm test && npm run test:wasm`.

- [ ] **Step 8: The per-edit gates** (Global Constraints), every one; the fence prints nothing. Name every count that
  moved: worker +3, the pool +6 (across two projects), `test:scripts` +N, `npm test` +N.

- [ ] **Step 9: The browser walk.** This task changes what a browser DOES (the socket's URL), so the suite is owed:
  `cd web && npm run e2e` in the background to `task-1-e2e.txt`, POLLED. Expected: 68 in 13. The local walk serves
  `sse` and `open`, so it proves no regression rather than the parameter (the parameter's witness is the pool's Review
  Focus 1 cell; its live reading is Task 13's).

- [ ] **Step 10: Commit.** The body quotes Steps 1, 3, 5 and 7's red runs.

```bash
git add crates/yokohama-worker/Cargo.toml crates/yokohama-worker/src/config.rs crates/yokohama-worker/src/lib.rs \
  crates/yokohama-worker/src/events.rs crates/yokohama-worker/src/receipt.rs \
  worker/src/env.ts worker/src/config.ts worker/src/config.unit.test.ts worker/src/identity.ts worker/src/identity.unit.test.ts \
  worker/src/index.ts worker/src/table.ts worker/src/house.ts worker/src/archive.ts worker/src/marker.unit.test.ts \
  worker/scripts/smoke-release.mjs worker/scripts/smoke-release.test.mjs \
  worker/wrangler.jsonc worker/wrangler.test.jsonc worker/wrangler.open.test.jsonc worker/vitest.config.ts \
  worker/test/open/door.test.ts worker/test/worker.test.ts worker/test/factory.test.ts worker/test/table.test.ts \
  web/src/api.ts web/src/ws.ts web/src/ws.test.ts
git commit -m "feat(worker): the two doors — YOKOHAMA_WEB_IDENTITY with no default, the open door's claim by header on a fetch and by claimed= on an upgrade, the objects reading their door's one header, and the pool's second project"
```

---

### Task 2: The open door's refusals (spec §2.1 "What is refused under `open`", "The archive's `test/` root", "The deploy guard, per door"; R-6.5c2.5; review R2.2-M1, R2.2-m4, R2.2-m5; cycle A's R2.2-m4 item) — from D6 retired Task 18

**Files:**
- Modify: `worker/src/config.ts` (`openDoorRefusals`, called at the end of `shellConfig`; the doc); `worker/src/admin.ts`
  (`operatorGate(door, admins, identity)`, door-aware; a type-only `Door` import); `worker/src/index.ts` (the gate's
  call and doc step 6a); `worker/src/env.ts` (`DEV_DOOR_CLIENT_ID`'s doc names it a SECRET in production);
  `worker/wrangler.jsonc` (`DEV_DOOR_CLIENT_ID` and its comment LEAVE `vars`; the header comment counts it among the
  absent, beside `YOKOHAMA_WEB_ADMINS`); `worker/wrangler.open.test.jsonc` (one comment sentence: the guard holds it);
  `worker/scripts/deploy-guard.mjs` (the header's checks 3, 5, 6, 7; `doorOf`, `doorRules`, `DOORS`, `doorLine`; checks
  5 and 6 over BOTH pool files; check 7 for two secrets, production's file alone) and `deploy-guard.test.mjs` (the
  fixtures lay `wrangler.open.test.jsonc`; the rewritten cells; the new cells); `crates/yokohama-worker/src/receipt.rs`
  (the module doc's test-root paragraph; `is_test_identity`; `root` through it; the test helper `r2` becomes a wrapper
  over `r2_at`; one cell); `worker/src/config.unit.test.ts`, `worker/src/admin.unit.test.ts` (the gate's first
  argument on every call, one new cell); `worker/test/open/door.test.ts` (a describe); `worker/test/worker.test.ts`
  (one cell).

**Interfaces:**
- Consumes (Task 1): `ShellConfig.door`, `Door`, the `claimed` kind, `callerName`, the `open` project and
  `door.test.ts`'s helpers, `door_config`. From cycle A: the key grammar, under which `a.test` and `joiner` are legal
  names.
- Produces: `operatorGate(door: Door, admins: string[] | null, identity: Identity): Gate`; `deploy-guard.mjs`'s
  `doorLine(root): string | undefined`; `receipt.rs`'s `is_test_identity(address)` and the test helper
  `r2_at(caller, door)`; the two refusal sentences below, verbatim.

**The two sentences** (the shared names; `config.unit.test.ts` pins them):
- `YOKOHAMA_WEB_ADMINS is set beside YOKOHAMA_WEB_IDENTITY=open — a claimed name is never an operator: delete the
  operator list (wrangler secret delete YOKOHAMA_WEB_ADMINS), or deploy the worker door`
- `YOKOHAMA_DEV_DOOR=on beside YOKOHAMA_WEB_IDENTITY=open — the dev door is the worker door's test instrument, and
  under open a claim needs no credential: set YOKOHAMA_DEV_DOOR to off, or deploy the worker door`

- [ ] **Step 1: The run-time refusals' cells** — `drafts/D6-tasks-17-19.md:1611-1764`: `config.unit.test.ts`'s describe
  (the list refused naming both variables and echoing NOTHING — `boss@t.test` never appears; the dev door refused
  when ` ON `; both kept under `worker`, the client id ignored under `open`; the two-defect probe: the list is named
  BEFORE the dev door); `admin.unit.test.ts` (every `operatorGate(` gains `"worker", ` first; the new cell: `off`
  under `open` for a claim, a listed person and no one; a claim under `worker` is `refused`);
  `door.test.ts`'s describe "what the open door refuses": `/api/admin/tables` answers the layer's 404 whatever name is
  claimed (`boss@t.test`, `ben`, none), reaching the house alone; `?force` 404; a plain name ending `.test` files its
  saves OUTSIDE `test/` (an `a.test` creator's abandon lands at `rev7/abandoned/…`, read by `bucketText`, and NOT under
  `test/rev7/…`); `worker.test.ts`: under `worker`, `YOKOHAMA_WEB_ADMINS` beside an env override to `open` answers the
  500 naming both (at `handle`). Red. Quote it.

- [ ] **Step 2: The refusals, the gate, and the call** — `drafts/D6-tasks-17-19.md:1784-1874` as drafted
  (`openDoorRefusals` before `shellConfig`; `shellConfig`'s body builds the object then calls it; `operatorGate`
  door-aware; `index.ts`'s call `operatorGate(config.door, config.admins, identity)` and doc 6a). Green: the unit
  cells; the pool's `.test` cell stays red until Step 4.

- [ ] **Step 3: The module's test root — its cell** — `drafts/D6-tasks-17-19.md:1880-1931`: in `receipt.rs`, `r2`
  becomes `r2_at(caller, "worker")`; the cell `a_plain_name_never_takes_the_test_root`: under the `open` door, a
  creator `a.test` and a browser caller `joiner.test` each plan under `rev7/…` with no `test/` root, while
  `red@t.test` still takes it; the truncation premise cell (battery 33) stands. Red.

- [ ] **Step 4: The test root takes an email** — `drafts/D6-tasks-17-19.md:1940-1972`: the module doc's paragraph;
  `is_test_identity(address) = address.contains('@') && address.ends_with(TEST_DOMAIN)` with its doc; `root`'s two
  sites through it. Then `cargo test -p yokohama-worker` (+1) and `cd worker && npm run build:module:debug && npm
  test` — the `open` project's `.test` cell passes.

- [ ] **Step 5: The guard's cells** — `drafts/D6-tasks-17-19.md:1972-2118`: the fixtures lay both pool files; the
  rewritten cells at the draft's `:203-208` and `:256-261`; the new cells, one per rule: a configuration with NO door
  fails naming the variable and "no default"; a door outside the two fails quoting it; under `worker`, `workers_dev`
  `true` fails and `ACCESS_TEAM_DOMAIN`/`ACCESS_AUD` absent fail; under `open`, `workers_dev` `true` PASSES,
  `preview_urls` `true` still fails, and `YOKOHAMA_DEV_DOOR: "on"` in `vars` fails; `DEV_DOOR_CLIENT_ID` in
  production's `vars` fails (check 7's second secret) while the pool files keep theirs; checks 5 and 6 read the `open`
  file too (a drifted `assets` block there fails); `doorLine` prints `wrangler.jsonc deploys the worker door —
  Cloudflare Access, verified by the shell`. Red: `npx vitest run -c vitest.scripts.config.ts scripts/deploy-guard.test.mjs`.

- [ ] **Step 6: The guard, per door, and the committed configuration** — `drafts/D6-tasks-17-19.md:2134-2334` as
  drafted (`DOORS`, `doorOf`, `doorRules`, the two pool files in checks 5 and 6, check 7's two keys over production's
  file alone, `doorLine`, the header's checks 3, 5, 6, 7 reworded); `wrangler.jsonc` loses `DEV_DOOR_CLIENT_ID` (the
  value stays in the git history and in RUNBOOK's records, accepted by R-6.5c2.5; the SECRET is put at Task 13 E.3, in
  the same deploy that drops the var); `env.ts`'s doc. Green: `npm run test:scripts` (+N), `node
  scripts/deploy-guard.mjs` on the tree fails ONLY its build-dependent checks (1, 2, 8, 9: no `gen/` release module,
  no assembled assets) and prints the door line — record the output.

- [ ] **Step 7: The per-edit gates**, the fence, the counts as deltas. The browser suite is NOT owed (nothing a
  browser does or shows moved).

- [ ] **Step 8: Commit.** The body quotes Steps 1, 3 and 5's reds.

```bash
git add worker/src/config.ts worker/src/config.unit.test.ts worker/src/admin.ts worker/src/admin.unit.test.ts worker/src/index.ts \
  worker/src/env.ts worker/wrangler.jsonc worker/wrangler.open.test.jsonc worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs \
  crates/yokohama-worker/src/receipt.rs worker/test/open/door.test.ts worker/test/worker.test.ts
git commit -m "feat(worker): what the open door refuses — the operator surface off, the operator list and the dev door refused beside open, the test/ root by an @, the deploy guard per door, and DEV_DOOR_CLIENT_ID out of vars"
```

---

### Task 3: The refinish rule on the Worker (spec §2.5 "On the Worker…", "The archive's pair", "A pool cell reproduces the race"; R-6.5c2.11; review M3, o1, o2; cycle A's `settleUpload` item; Review Focus 3) — from D6 retired Task 19

**Files:**
- Modify: `worker/src/r2.ts` (`Outcome` gains `held?: string`; `executePlan` hands a conflict's held text on;
  `settledArm` after `stampArm`; `logOutcome`'s doc); `worker/src/r2.unit.test.ts` (the conflicts carry `held`; two
  describes); `crates/yokohama-worker/src/receipt.rs` (`Outcome.held`; the `Conflict` arm hands it on; `parse_outcome`
  refuses `held` beside anything but a conflict; `stamp_archive_inner`'s doc; two cells); `worker/src/table.ts`
  (`settledArm` replaces `stampArm` in the import; `settleUpload`'s doc and its `conflict` arm, which goes; the arm
  through `settledArm`; `stamp` reads a NON-200 stamp reply as no stamp; `stamp`'s doc); `worker/test/gates.ts`
  (`heldInTurn()` after `heldPuts`); `worker/test/archive.test.ts` (imports; the two endings and `playAll` after
  `oneSeatGame`; the conflict cell as an `it.each`; two race cells; one foreign-stamp cell).

**Interfaces:**
- Consumes (cycle A, the contract these cells stand on): `PutError::Conflict { key, held: Option<String> }`, with the
  Worker's `receipt.rs` arm at `held: None` today; the layer's same-game rule in `Games::archive_if_over` (reached
  through `archive_over_retry`): at `Why::Over`, a `Conflict` whose held text parses to a `GameRecord` with the current
  record's `seed`, `config` and `choices` stamps `archived_at` exactly as `Ok(Landed)` does; `None`, a text that does
  not parse, or another game's record stamps nothing; the rule lives at the game-over site alone.
- Consumes (Task 1): the module's door; `r2()` with `identity`.
- Produces: `Outcome.held?` (both sides); `settledArm(outcome, stamp | undefined)`; `heldInTurn()`; the log line
  `stamp-refused {id, status}`.

**Confirmed at the M6.5c code (D6, review o2):** `deliverArchive` keys each upload in flight by its plan's key; two
completions of one game are two records, two keys; the overlapping cell asserts both uploads in flight
(`uploads.size === 2`); `deliverArchive` runs outside `blockConcurrencyWhile`, so an undo interleaves.

**Why a conflict whose stamp does not land is FINAL** (spec §10.5): read through `stampArm`, another game's
`{"stamped": false, "over": true}` is `retry`, hourly for ever. `settledArm` keeps every conflict final except the one
the layer stamped.

- [ ] **Step 1: The shell's cells** — `drafts/D6-tasks-17-19.md:2450-2528`: the imports gain `settledArm`; the conflict
  cell's title ends "— and hands those bytes on, unparsed" and its outcome carries `held: OTHER_SEED` (a record text
  whose seed is above 2^53, so it survives only unparsed); the race rows carry `held`; the describe "what an upload and
  its stamp leave together" (**Review Focus 3**: a conflict the stamp landed on is `stamped`; a conflict with
  `{stamped: false, over: true}`, `{stamped: false, over: false}` or `undefined` is `final`; every other outcome reads
  as `stampArm` does; an undefined stamp on a landed upload is `retry`); the `logOutcome` describe (the line names the
  detail and never the held record). Red: `npx vitest run -c vitest.scripts.config.ts src/r2.unit.test.ts`.

- [ ] **Step 2: The bucket side hands the held record on** — `drafts/D6-tasks-17-19.md:2539-2609` as drafted
  (`Outcome`'s doc; `executePlan`'s `against(held)`; `settledArm`; `logOutcome`'s doc). Green.

- [ ] **Step 3: The module's cells** — `drafts/D6-tasks-17-19.md:2615-2677`: a receipt carrying `held` beside a
  `conflict` reaches the layer as `PutError::Conflict { held: Some(text) }` (read through a `stamp_archive_inner` over
  the owed fixture whose held text IS the same game's earlier completion: `stamped: true`); `held` beside `landed`,
  `identical` or `failed` is refused in `parse_outcome`'s sentence ("a receipt carries the held record only beside a
  conflict"); a `held` that does not parse as a record, or another game's, leaves `stamped: false, over: true`
  (**Review Focus 3**). Red: `cargo test -p yokohama-worker`.

- [ ] **Step 4: The receipt carries it** — `drafts/D6-tasks-17-19.md:2687-2756` as drafted (`Outcome.held` with
  `#[serde(default)]`; the `Conflict` arm `held: receipt.held`; `parse_outcome`'s refusal; `stamp_archive_inner`'s doc
  rewritten for the three arms). Green (+2).

- [ ] **Step 5: The race, pinned — the pool's cells** — `drafts/D6-tasks-17-19.md:2756-3010`: `gates.ts`'s
  `heldInTurn()`; `archive.test.ts`'s two endings (the walk's last two moves from the decision two undos reach:
  `1`, `0`, `confirm`, `1` replays the walk's own record; `1`, `1`, `confirm`, `1` is a different finished record —
  D6 measured both with the built module) and `playAll`; the conflict cell as an `it.each` over the two endings; the
  two race cells — an undo during a held game-over put, then the OTHER ending: (a) the first upload settles before the
  second starts; (b) BOTH in flight at once (`heldInTurn`, `uploads.size === 2` asserted). Each ends with the table
  STAMPED (`archived_at` set), the archive holding the FIRST completion's record, the table abandonable (a creator's
  DELETE 200) and expiring normally (`backdate` + the alarm → the marker). **Plus one cell this plan adds (cycle A's
  item):** an owed game-over table whose record names a FOREIGN bot — the fixture pair's record with its bot seat's
  `"version":2` text-replaced to `3` (`replaceOnce`; the strings rule) — whose alarm's `stamp_archive` answers the
  409: the shell logs `stamp-refused {id, status: 409}`, throws nothing, keeps `archive_owed` with `attempt + 1` and
  re-arms the alarm (final-for-now: bounded by Abandon or expiry, spec §2.4). Build the module first; red on the race
  cells (D6 measured: B's conflict final, the table unstamped, the Abandon refused for good, the expiry `held`) and on
  the foreign cell (a throw inside the alarm). Quote them.

- [ ] **Step 6: The conflict goes to the stamp** — `drafts/D6-tasks-17-19.md:3027-3077` as drafted (`settledArm`
  imported; the `conflict` arm deleted; `const arm = settledArm(outcome, stamp)`; the docs of `settleUpload` and
  `stamp`) — PLUS: in `stamp`, a reply whose status is not 200 becomes `stamp = undefined` after
  `log("stamp-refused", { id, status })`, and `parseStamp` runs on a 200 alone. Green: `npm test` (both projects),
  `npm run typecheck`, `npm run test:scripts`.

- [ ] **Step 7: The per-edit gates**, the fence, the counts (worker +2, the pool +N, `test:scripts` +N). The browser
  suite is not owed.

- [ ] **Step 8: Commit.** The body quotes Steps 1, 3 and 5's reds.

```bash
git add worker/src/r2.ts worker/src/r2.unit.test.ts worker/src/table.ts crates/yokohama-worker/src/receipt.rs \
  worker/test/gates.ts worker/test/archive.test.ts
git commit -m "feat(worker): the refinish rule reaches the Worker — a conflict hands its held record to the module, the layer stamps a same-game conflict, every other conflict stays final, a refused stamp is retried and never thrown, and the race is pinned both ways"
```

---

### Task 4: The module's invariants, the restatements imported, the figures in one place, and the `events` probe (spec §2.6 "The restatements", §2.7 as amended; R1's cleanup debts 1–4 and 6; cycle A's import-switch item and its `events` item; R-C8) — from D7 retired Task 22, minus what Task 20's split would have moved

Five debts made STRUCTURAL, each held by the compiler or a cell where a comment held it before — and the module's
copies of five layer items deleted for the layer's exports.

1. **The picker's rule on every call.** `events_inner`, `stamp_archive_inner` and `expire_inner`'s no-creator arm reach
   `Games::handle` outside `picker::around`, and a handle one of them REBUILDS pumps an owed bot move and persists it —
   a record change no list is cleared for. **Decision (D7): clear the list whenever the record changes, at the ONE
   drain every layer call passes through** (`from_the_layer`), which takes a `Scope` — never route those calls
   through `around`, which is keyed by a request and applies the wrong rule (a rebuilt `Repl` starts with its picker
   CLOSED, so the whole entry goes, a `Dropped` one included). The residual, named: a rebuild at the SAME record
   leaves a stale list the record rule cannot see; unreachable, since nothing in this host evicts a resident handle.
2. **The scrub's fence all the way down:** `scrub` names every field of each `SeatDoc` variant, of `TurnMirror` and of
   its `bot: Option<BotDescriptor>` — MEASURED (D7): a field added at any depth is E0027, a new variant E0004, and the
   fence must be one exhaustive `match` (a nested `if let` trips `clippy::collapsible_match`).
3. **One plan per call:** a second deferred put of ANOTHER key in one call is REFUSED (`SECOND_PLAN`), the first kept —
   not a list, which would move `Reply.archive`, `meta.archive_owed` and the receipt loop for a case no layer site
   reaches.
4. **The restatements imported** — the module's `error_body`, `MAX_WAIT_SECONDS`, `house_frame_inner`, the clock and
   the scan become the layer's `api::error_body`, `api::check_wait`, `signal::house_frame`, `games::HostClock` and
   `platform::calls_in`; and the BROWSER host's clock and scan switch too (spec §2.6 as corrected in revision 2.5:
   "stated once" is false otherwise).
5. **The figures in one place:** the Worker module's figures join the browser module's in the root `Cargo.toml`'s
   `release-wasm` comment, read by a committed script; `copyOf`'s doc (in `table.ts`, since the split is dropped) and
   `worker/README.md` §6.3 cite that place.

**Files:**
- Modify: `crates/yokohama-worker/src/lib.rs` (`Scope`; `from_the_layer(scope, call)`; `dispatch_inner`'s call;
  `error_body` re-exported from the layer; the clock through `HostClock`; the scan through `platform::calls_in`; the
  module doc; `a_refusal_body_is_the_one_dispatch_writes`'s doc); `src/picker.rs` (the module doc's rule; `clear_if`
  and `watching` beside `around`; `restore_picker_inner`'s call; two cells); `src/events.rs` (the cap through
  `api::check_wait`; the call's scope; the doc; the two-defect line in `lib.rs`'s order cell); `src/receipt.rs`
  (`SECOND_PLAN`; `put`'s pending write; `pending`'s doc; the scopes of `stamp_archive_inner` and `expire_inner`'s arm;
  the undo cell's direct call; one cell); `src/house.rs` (`scrub`; its imports; `house_frame_inner` as the export; the
  frame cell pins BYTES); `crates/yokohama-wasm/src/lib.rs` (the clock and `set_clock` through `HostClock`; the scan
  through `calls_in`; the crate doc); `Cargo.toml` (root: the Worker module's paragraph in the `release-wasm`
  comment); `worker/src/table.ts` (`copyOf`'s doc cites the one place); `worker/README.md` (§6.3's sentence).
- Create: `worker/scripts/module-figures.mjs`, `worker/scripts/module-figures.test.mjs`.

**Interfaces:**
- Consumes: the five exports (§"Names shared", read off the tree in Step 1); Tasks 1–3's `config`, `door_config`,
  `r2`, `r2_at`; `ReplyData.logs` and `drain_logs()`; `TurnMirror.bot: Option<BotDescriptor>` and
  `bots::BotDescriptor`; `SeatDoc::{Human { user, name }, Bot, Open}`.
- Produces: `pub(crate) enum Scope<'a> { Dispatch, Table(&'a str) }`; `from_the_layer(scope, call)`;
  `picker::watching(games, id, call) -> (T, Option<String>)`; `picker::clear_if(changed, id)`; `SECOND_PLAN`;
  `moduleFigures(bytes) -> { bytes, gzip9, pages, segments, dataBytes, dataSection }`; the `Cargo.toml` paragraph.

- [ ] **Step 1: Read the exports off the tree, and STOP if one is not the layer's ONE statement** —
  `drafts/D7-tasks-20-23.md:1764-1792`, against `3ed27f07` rather than "Task 7's commit": for each of
  `api::error_body` (three arms), `api::check_wait`, `signal::house_frame`, `games::HostClock`, `platform::calls_in`,
  confirm its signature, that it compiles WITHOUT `server` (the wasm `cargo check`), and that the layer's own site CALLS
  it. Record the five in the ledger. If `error_body` lacks the `foreign_bot` arm, stop and ask.

- [ ] **Step 2: The picker cells** — `drafts/D7-tasks-20-23.md:1792-1862`: in `picker.rs`'s test module, a cell per
  reaching call: an open picker over a table whose handle is EVICTED and whose record then changes through
  `events_inner` (a cold handle with an owed bot move pumps it), through `stamp_archive_inner`, and through
  `expire_inner`'s no-creator arm — each answers `picker: Some("[]")` and the entry is gone; and a control: a call
  that changes nothing leaves the entry. Red (the entries stay, `picker: None`).

- [ ] **Step 3: The one-plan cell** — `drafts/D7-tasks-20-23.md:1862-1893`: two puts of DIFFERENT plans in one call
  (the second a different id): the second answers `PutError::Failed(SECOND_PLAN)`, the first is what `drain_plan`
  hands over; two puts of the SAME plan still defer one. Red.

- [ ] **Step 4: The figures cells** — `drafts/D7-tasks-20-23.md:1893-1925`: `module-figures.test.mjs` over a
  hand-built minimal module (the magic, a memory section of 3 pages, two data segments) → the six figures; a buffer
  that is not a module refused naming `\0asm`. Red (no script).

- [ ] **Step 5: The `events` two-defect line (R-C8).** In `lib.rs`'s `events_refuses_in_axums_handler_order`, after the
  unknown-id probe: a caller with NO SEAT at an existing dealt table, with `wait=26` — `events_inner("dealt",
  Some("blue@t.test"), "wait=26")` where only `red@t.test` sits — answers the cap's 400, never the admission's 403,
  with the layer's own sentence. It passes at HEAD (the restated cap is before the read); it is the pin Step 8's
  switch to `check_wait` must keep, and a mutant that moves the check after `events_at` turns it red. Record the
  mutant's red in the report.

- [ ] **Step 6: `Scope`, and the drain that applies it** — `drafts/D7-tasks-20-23.md:1957-2023` as drafted (`Scope`
  above `from_the_layer`; the drain's two arms; the doc paragraph; the six call sites take their scopes: `dispatch_inner`
  `Dispatch`, `events_inner` / `restore_picker_inner` / `stamp_archive_inner` / `expire_inner`'s arm / the undo cell
  `Table(id)`). `logs: drain_logs()` is kept as the tree spells it.

- [ ] **Step 7: `watching`, and the rule stated once** — `drafts/D7-tasks-20-23.md:2023-2060` as drafted (`clear_if`;
  `watching` reading `record_of` before and after; `around`'s changed arm through `clear_if`; the module doc's rule).
  Green: Step 2's cells.

- [ ] **Step 8: One plan per call** — `drafts/D7-tasks-20-23.md:2060-2100` as drafted (`SECOND_PLAN` below
  `ANOTHER_CALLER`; `put`'s pending write refuses a second key; the docs). Green: Step 3's cell.

- [ ] **Step 9: The scrub, named all the way down** — `drafts/D7-tasks-20-23.md:2100-2143` as drafted: `seats` and
  `turn` bound; the appended exhaustive matches over `SeatDoc` and `TurnMirror { to_move, over, winner, bot:
  Some(BotDescriptor { controller, version, params }) | None }`; the doc. **Prove the fence in an export** (a field
  `agent_seed: u64` added to `BotDescriptor` → E0027 in `house.rs`; revert). Record it.

- [ ] **Step 10: The restatements, imported** — `drafts/D7-tasks-20-23.md:2143-2368`, with the tree's names (Step 1):
  - the error body: `pub(crate) use yokohama_web::api::error_body;` and the fn deleted; the pin cell's doc;
  - the wait cap: `events.rs`'s `const` goes; `if let Err(e) = api::check_wait(q.wait) { return host_refusal(e); }`;
    the doc; `events_refuses_in_axums_handler_order` holds, Step 5's line included;
  - the house frame: `house_frame_inner` returns `yokohama_web::signal::house_frame(seq, chat)`; `HouseFrame` and
    `use serde::Serialize` go; the frame cell pins BYTES (`{"seq":3,"chat":7}` and the `u32::MAX` pair) instead of
    comparing the export with itself;
  - the clock: `static CLOCK: HostClock = HostClock::new();` replaces `FROZEN_ON`/`FROZEN`/`TICKS`; `now()` is
    `CLOCK.now(|| live())` with `live` the `js_sys::Date::now() / 1000.0` arm on `wasm32` and `CLOCK.tick()` natively;
    `set_clock(seconds)` is `CLOCK.set(seconds.map(|s| s as u64)).map(|s| s as f64)`; the atomics' import goes;
  - the scan: `this_host_makes_no_platform_call_but_the_clock` reads each file's text through
    `yokohama_web::platform::calls_in(&name, &text)` and keeps its own anti-vacuity partners (a file scanned; the
    clock in CODE);
  - **the browser host** (`crates/yokohama-wasm/src/lib.rs`): the same clock switch (`HostClock` in a `static`;
    `set_clock` through `set`), the same scan switch in `this_host_makes_no_platform_call_but_the_clock_and_the_console`
    (its two allowed calls, the clock and the console import, stay its own partners), and the crate doc naming the
    export. **Anti-vacuity, both hosts:** temporarily add `std::fs::read("x")` to a host file in an export and watch
    the switched scan go red; record it.
  - `cargo test -p yokohama-worker`, `-p yokohama-wasm`, the wasm `cargo check`, `cargo doc`, clippy: green.

- [ ] **Step 11: `worker/scripts/module-figures.mjs`** — `drafts/D7-tasks-20-23.md:2404-2500` as drafted (the header,
  `moduleFigures` over the binary format's memory and data sections, `gzip -9` through `zlib`, the CLI entry). Green:
  Step 4's cells; `npm run test:scripts` (+2).

- [ ] **Step 12: Measure the release module, and record it in the ONE place** — `drafts/D7-tasks-20-23.md:2500-2538`:
  `cd worker && npm run build:module && node scripts/module-figures.mjs gen/yokohama_worker_bg.wasm && npm run
  build:module:debug` (the debug module comes BACK before any pool run); the root `Cargo.toml`'s Worker paragraph
  transcribed from the printed JSON with the head, the date and the machine; `copyOf`'s doc in `table.ts` cites it
  (`drafts/D7-tasks-20-23.md:2538-2557`, with `worker/src/table.ts` for `delivery.ts`); `worker/README.md` §6.3's
  sentence ("18 pages or 1.125 MiB (read off the release build)") becomes "read in ONE place, the root `Cargo.toml`'s
  `release-wasm` comment". The comment's head, "The two hosts' modules, and NOTHING else", stays true.

- [ ] **Step 13: The per-edit gates**, in the order that keeps a stale module from answering: the Rust set; `cd web &&
  npm run build:wasm && npm run typecheck && npm test && npm run test:wasm` (the browser host's module moved); `cd worker
  && npm run build:module:debug && npm run typecheck && npm run test:scripts && npm test`. The fence prints nothing;
  the corpus is untouched. Counts: worker +5 (Steps 2, 3, 5's mutant-holding line counted in its cell), wasm ±0
  (its scan cell rewritten, none added), `test:scripts` +2, the pool ±0 (D7 measured the invariants byte-inert on the
  pool).

- [ ] **Step 14: Commit.** The body quotes Steps 2, 3 and 4's reds and Steps 9 and 10's export mutants.

```bash
git add crates/yokohama-worker/src/lib.rs crates/yokohama-worker/src/picker.rs crates/yokohama-worker/src/events.rs \
  crates/yokohama-worker/src/receipt.rs crates/yokohama-worker/src/house.rs crates/yokohama-wasm/src/lib.rs Cargo.toml \
  worker/scripts/module-figures.mjs worker/scripts/module-figures.test.mjs worker/src/table.ts worker/README.md
git commit -m "refactor(worker): the picker's rule on every call, the scrub fenced to the seat and the mirror, one plan per call, the two hosts' restatements replaced by the layer's exports, and the Worker module's figures in one place"
```

---

### Task 5: The shell's small debts — the `hasTables` memo, one time unit, L-8, L-4, the house copy's comments (spec §2.7 as amended; M6.5c close §6.2; F-4 by its measured fix; R-C4; cycle A's house-copy item) — from D7 retired Task 21, minus the read-once thread

**What changes, and why.**
- **The memo (F-4's rows).** MEASURED (D7, A-3): `hasTables` scans the whole of `sqlite_master` inside every `getMeta`
  and every `readDocs` — 30 of a warm request's 32 rows read. A per-object memo, forgotten at `deleteAll()`, takes a
  warm request to 2 rows. The close claims F-4 only by its measured live reading (Task 13).
- **One time unit.** The operator listing carries `created_at` and `archived_at` in the layer's SECONDS and
  `last_active` in the shell's MILLISECONDS (deployment 17: `1790224338` beside `1790224338723`). The listing becomes
  all seconds, `last_active` cut where the row is built; the SLOT keeps milliseconds (nothing stored changes). The
  listing is host-only: no corpus file carries it.
- **L-8.** `deleting` reads every document only to ask whether a record is stored. With the thread dropped (R-C4) it
  asks a one-row existence query, `hasDoc(sql, "record")`.
- **L-4.** `end` logs `forced` for ANY removal acknowledged with no marker — a force, or a removal the pre-marker code
  wrote. A force now flags itself (`meta.forced`, in its batch), so `forced` is exactly an operator's force; a
  markerless removal nobody flagged logs `removal-ended`.
- **The house copy's comments.** `house.ts`'s listing and `lastOf`, and `table.ts`'s `copyFacts`, justify PARSING a copy
  by "carries no seed". Since cycle A a copy also carries `turn.bot.params.weights` above 2^53, which `JSON.parse`
  rounds. Harmless — nothing reads `params`, and no parsed copy is written back — but the premise is incomplete: each
  comment now says so, and a cell pins that the stored copy is the module's text byte for byte.

**Files:**
- Modify: `worker/src/storage.ts` (`hasTables` answers from a `WeakSet<SqlStorage>` once found; `forgetTables`;
  `hasDoc`; `MetaKey` + `"forced"`); `worker/src/table.ts` (`end` calls `forgetTables` after `deleteAll()` and names
  its line; `force`'s batch sets `forced`; `deleting` through `hasDoc`; `copyFacts`'s doc); `worker/src/house.ts`
  (`listing()`'s `last_active` in seconds and its doc; `lastOf`'s doc); `worker/src/admin.ts` (`OperatorRow`'s doc).
- Create: `worker/test/reads.test.ts`.
- Modify: `worker/test/operator.test.ts` (the listing cell's units; a `removal-ended` cell); `worker/test/slot.test.ts`
  (the byte-identical copy cell, if none holds it).

**Interfaces:**
- Consumes: Task 3's `settleUpload` as it stands (a different function of the same file); `helpers.ts`'s
  `seedPlaying`, `call`, `RED`, `logsDuring`, `operate`, `backdate`, `endMarker`.
- Produces: `forgetTables(sql)`, `hasDoc(sql, kind): boolean`, `MetaKey "forced"`, the log line `removal-ended {id}`,
  the listing in unix seconds.

- [ ] **Step 1: The rows-read cell** — `drafts/D7-tasks-20-23.md:1482-1520`: `reads.test.ts` wraps the LIVE instance's
  `sql.exec` (an own property of an object that is no RPC receiver — pool facts 1 and 4) and, over a warm
  `GET …/events`, asserts NO query names `sqlite_master` and the rows read are ≤ 4. Red, MEASURED (D7): six scans, 32
  rows. Also a guard cell: a warm request calls `readDocs` once (a spy at module scope, filtered to the table's own
  `SqlStorage`) — it passes at HEAD and pins that the memo moves no read.

- [ ] **Step 2: The memo** — `drafts/D7-tasks-20-23.md:1520-1565` as drafted (`known: WeakSet<SqlStorage>`; `hasTables`
  answers from it once both tables are found and never remembers an absence; `forgetTables`; `end` forgets right
  after `deleteAll()`). Green: Step 1 (rows ≤ 4, D7 read 2). **The forget has catchers already** (D7 measured 13 cells
  red without it: expiry's marker cell, nine force cells, three slot cells) — the battery takes both rows.

- [ ] **Step 3: The listing's cells** — `drafts/D7-tasks-20-23.md:1190-1252`'s two operator cells: the row's
  `last_active` is whole seconds, equal to `Math.floor(slot.last_active / 1000)`, and `created_at` is unchanged; a
  markerless removal logs `removal-ended {id}` and a force logs `forced {id}` with its `operator` line. Red.

- [ ] **Step 4: The listing in one unit, and L-4** — `drafts/D7-tasks-20-23.md:1383-1463` as drafted (`listing()`'s
  row; `OperatorRow`'s doc; `MetaKey "forced"`; `force`'s batch `setMeta(sql, "forced", String(Date.now()))`; `end`'s
  three lines). Green.

- [ ] **Step 5: L-8.** `hasDoc(sql, kind)`: `hasTables(sql) && sql.exec("SELECT 1 FROM docs WHERE kind = ? LIMIT 1",
  kind).toArray().length === 1`, with its doc; `deleting`'s `readDocs` at its top becomes the one query at the site
  that asks whether a record is stored (find `stored.has("record")` by content). A cell in `reads.test.ts`: a cold
  abandon that saves first reads the documents at most twice (the wake and the batch; MEASURED at the base: 3 with
  `deleting`'s own). Red first (3), then green.

- [ ] **Step 6: The house copy's comments, and their cell.** The three comments (`house.ts`'s listing doc and `lastOf`;
  `table.ts`'s `copyFacts` doc) each say: "a copy carries no seed, and since M6.5c2 a bot descriptor whose `weights`
  is above 2^53, which this parse rounds; it may be parsed because nothing here reads `params` and no parsed copy is
  written back". The cell (in `slot.test.ts` unless one already holds it — check first): the house's stored copy for a
  seeded table is byte-identical to the text the table's `house_copy` answered, `weights` digits intact.

- [ ] **Step 7: The per-edit gates** in `worker/` (no module change: no rebuild owed), the Rust set unmoved, the fence,
  the counts (the pool +5: reads ×3, operator ×2; +1 slot if added). The browser suite is not owed.

- [ ] **Step 8: Commit.** The body quotes Steps 1, 3 and 5's reds.

```bash
git add worker/src/storage.ts worker/src/table.ts worker/src/house.ts worker/src/admin.ts \
  worker/test/reads.test.ts worker/test/operator.test.ts worker/test/slot.test.ts
git commit -m "perf(worker): the tables' existence remembered per object (a warm request reads 2 rows, not 32), the operator listing in one time unit, a one-row record check in deleting, a force that flags itself, and the house copy's parse premise stated whole"
```

---

### Task 6: The layer's carried items — the versioned refusal sentence, the run-id stamping pin, the print scan's blind spot (spec §2.4 as amended, R-6.5c2.21; cycle A's `bots.rs` edges and its `run_id` items; R-C5)

**Files:**
- Modify: `crates/yokohama-web/src/bots.rs` (`distinguishing_labels`; two cells); `crates/yokohama-web/src/api.rs`
  (`live_handle`'s refusal through it — find `cannot_continue(` by content; the test at its `ForeignBot` literal
  unchanged); `crates/yokohama-web/src/explain.rs` (`why`'s foreign refusal through it, the `played` label alone);
  `crates/yokohama-web/src/games.rs` (the test module: one cell); `crates/yokohama-web/src/log.rs` (the print scan's
  list gains `io::stderr(` and `io::stdout(`).

**Interfaces:**
- Consumes: `bots::{BotDescriptor, label, production, foreign, cannot_continue}`; `Contestant::from_parts`;
  `Games::with_parts(…, RunHost)`; `api::input`; `Games::persist`; the corpus's `foreign_bot_409.json` and
  `why_foreign.json` (V2 vs V4 — UNCHANGED by this task).
- Produces: `bots::distinguishing_labels(played: &BotDescriptor, runs: &BotDescriptor) -> (String, String)`.

**The rule (R-C5):** when `label(played) != label(runs)`, the pair is the two labels as today; when they tie, each is
`format!("{label}.{version}")`; when those tie too, each is `format!("{label} ({controller} {params})")` with the
parameters rendered `k=v,…` in `BTreeMap` order. So a same-name re-point reads "This game was played against V4.2.
This server now runs V4.3 and can't continue it." and `why` on its move "This move was made by V4.2, which this server
doesn't run." — the screens keep "V4" (`label` is unchanged), and the corpus samples (V2 vs V4) do not move.

- [ ] **Step 1: The label cells.** In `bots.rs`: `distinguishing_labels` over (V2, V4) is `("V2", "V4")`; over V4's
  descriptor and the same with `version: 3` is `("V4.2", "V4.3")`; over `LOOKAHEAD_V1_DEFAULT`'s descriptor and the
  same with `b: 8` is the parenthesized pair, distinct. Red (no function).

- [ ] **Step 2: The function, and its two callers.** `distinguishing_labels` with the rule above and its doc (the
  same-name re-point edge cycle A carried; the screens keep `label`); `api.rs`'s `live_handle` builds
  `GameError::ForeignBot { played, runs }` from it; `explain.rs`'s two `Why::foreign(&label(d))` become the
  pair's first half against `production()`. Green: `cargo test -p yokohama-web` (+1 cell, 3 rows), and
  **`git diff --stat -- web/src/wire` prints nothing** (`wire_fixtures` regenerates nothing: run it plain, not with
  `UPDATE_WIRE_FIXTURES`).

- [ ] **Step 3: The run-id pin.** In `games.rs`'s test module: a record dealt on a `Games` built with
  `RunHost::Browser` (its JSON taken from the store) is loaded into a `Games` built with `RunHost::Server` over a store
  holding that JSON; one move goes through `api::input`; after `persist`, the stored record's `run.run_id` is STILL
  `web/browser`. **Prove the mutant in an export:** `persist` re-stamping `run_id` from `run_host` → red. It passes
  at HEAD; the mutant's red goes in the report.

- [ ] **Step 4: The print scan's blind spot.** `log.rs`'s scan list gains `"io::stderr("` and `"io::stdout("`. Run the
  cell: if it names an existing site outside the seam, STOP and report it (a finding, not a silent gate); expected: it
  stays green, the seam count 1. **Anti-vacuity in an export:** a `writeln!(std::io::stderr(), "x")` in `games.rs`
  → red naming the line.

- [ ] **Step 5: The per-edit gates:** the Rust set (`cargo test -p yokohama-web` +2), the wasm check, clippy, the
  refusing build (8); `cd worker && npm run build:module:debug && npm test` (the layer moved: the module is rebuilt,
  the corpus through the objects unchanged); `cd web && npm run build:wasm && npm run test:wasm`; the fence prints
  nothing; the corpus diff prints nothing. The browser suite is not owed (no screen changes: the V2/V4 sentence is
  what every screen renders).

- [ ] **Step 6: Commit.** The body quotes Step 1's red and Steps 3 and 4's export mutants.

```bash
git add crates/yokohama-web/src/bots.rs crates/yokohama-web/src/api.rs crates/yokohama-web/src/explain.rs \
  crates/yokohama-web/src/games.rs crates/yokohama-web/src/log.rs
git commit -m "feat(web): the foreign-bot refusal distinguishes a same-name re-point by version, a persist never re-stamps run_id (pinned), and the print scan sees io::stderr writes"
```

---

### Task 7: The scripts' and the live tiers' LOW findings — L-3, L-5, L-7, L-9 (spec §2.8 as amended; R3's §7; R-C3) — from D7 retired Task 23, minus the dropped modules

The four, and where each rule now lives:
- **L-3.** `read-deploy.mjs`'s `deployedRelease` echoes up to 200 characters of a non-200 `/sw.js` body — an Access
  login page or the zone's challenge, which can carry a token. It now names the answer's status, type and size, as
  `live.ts`'s `accessState` does, and never a character of the body.
- **L-5.** The pool's host-field exemption (`corpus.test.ts`'s `HOST_FIELDS`) can be spent on an ABSENT field:
  `expect(undefined).not.toEqual(…)` passes. It now takes the presence rule the live replay states
  (`live-replay.mjs`, "an exempted field must be SENT by this host, as a string, and differ") — stated here as its
  twin, each comment naming the other (R-C3).
- **L-7.** `dealAnswers` is uncapped: ONE designed 409 a page is forgiven, its two lines once each, so a client that
  looped on the refusal fails the walk rather than having its lines forgiven by the hundred.
- **L-9.** A live create's table was known only once the spec READ the reply, so a walk that failed before that left a
  table in the shared lobby. It is now known from the moment the create is SENT (`awaitCreate`), and the `afterEach`
  abandons it whether or not the spec got that far.

**Files:**
- Modify: `worker/scripts/read-deploy.mjs` (`deployedRelease` exported with a `fetcher`, no body echoed;
  `answerShape`); `worker/scripts/read-deploy.test.mjs` (two cells); `worker/test/corpus.test.ts` (`hostFields`,
  `withoutHostFields`, the presence rule, the twin comment); `web/e2e/harness.ts` (`DEAL_ANSWERS_PER_PAGE`; the two
  arms of `watchForBreakage` count per page); `web/e2e/live.ts` (`Opening`, `awaitCreate`, `abandonOpened` after the
  credential's block); `web/e2e/two_browsers.spec.ts`, `joins_by_link.spec.ts`, `exit_game.spec.ts` (the create through
  `awaitCreate`; the `afterEach` through `abandonOpened`); `web/e2e/harness.node.test.ts` (the cap's cells;
  `awaitCreate`/`abandonOpened`'s cells); `web/e2e/live.node.test.ts` (the create's structural cell: every live
  spec's create goes through `awaitCreate`).

**Interfaces:**
- Consumes: `watchForBreakage`'s two arms and `isDealAnswer`/`isDealAnswerLine`; `liveRequest`, `abandonAsCreator`,
  `credential`; `worker/scripts/live-replay.mjs`'s presence rule (its `:431-442` at the D7 base — find by content).
- Produces: `DEAL_ANSWERS_PER_PAGE = 1`; `type Opening = { id: Promise<string | null> }`; `awaitCreate(page):
  Opening` (a `page.waitForResponse` for `POST /api/games` armed BEFORE the click, resolving the reply's `id` or
  `null`); `abandonOpened(opening: Opening | null)` (awaits the id; `abandonAsCreator` where there is one; nothing
  where there is none); `deployedRelease(base, pairs, fetcher = fetch)`; `answerShape(status, contentType, bytes)`;
  `hostFields(fixture, committed, got) -> {field, verdict}[]`, `withoutHostFields(fixture, value)`.

- [ ] **Step 1: The baselines.** `cd worker && npm run test:scripts && npm test` and `cd web && npm run test:wasm`;
  record the three counts.

- [ ] **Step 2: The read-deploy cells (L-3)** — `drafts/D7-tasks-20-23.md:2898-2930`: a 403 `text/html` body carrying
  `DUMMY-TOKEN-1234` is named `/sw.js answered 403 — a text/html; charset=utf-8 body of <n> B` and the token appears
  nowhere; a 302 is named without its `location`; `answerShape(204, null, 0)` reads `a typeless body of 0 B`. Red
  (`deployedRelease` is not exported; the body is echoed).

- [ ] **Step 3: L-3** — `drafts/D7-tasks-20-23.md:3650-3690`'s `deployedRelease` and `answerShape`, WITHOUT the
  `precacheEntriesFrom` import (the one-reader dedup is dropped): `precacheList` stays as it is. Green: `npm run
  test:scripts` (+2).

- [ ] **Step 4: L-5** — `corpus.test.ts`: `hostFields` and `withoutHostFields` defined IN THE FILE (the dropped
  `rules.mjs` would have shared them): for each exempted field of a named fixture, the verdict is `spent` only when
  this host sent it as a string and it differs from the committed one; the row callback asserts every verdict `spent`
  and compares the rest deep-equal (`drafts/D7-tasks-20-23.md:3369-3401`'s loop, with the local functions). The
  comment names `live-replay.mjs`'s rule as its twin, and `live-replay.mjs`'s comment names this one. **Prove it:**
  in an export, drop `identity` from the module's `health` reply (the `Host` struct's field renamed in the JSON) →
  the cell is red naming `health.identity`; at HEAD it was green. Record the red.

- [ ] **Step 5: The live tier's node cells** — `drafts/D7-tasks-20-23.md:2930-3129`, the parts for L-7 and L-9 only:
  in `harness.node.test.ts`, `watchForBreakage` forgives the deal's 409 ONCE a page on each arm and counts a second as
  breakage; `awaitCreate` resolves the id from a `201 {id}` reply, `null` from a refusal; `abandonOpened(null)` does
  nothing; in `live.node.test.ts`, every live spec's `POST /api/games`-triggering click is preceded by an `awaitCreate`
  (read through the TypeScript compiler, as the file reads `liveRequest`). Red.

- [ ] **Step 6: L-7** — `drafts/D7-tasks-20-23.md:3790-3834` as drafted (`DEAL_ANSWERS_PER_PAGE`, the `dealt` counter,
  the two arms). Green: its cells.

- [ ] **Step 7: L-9** — `drafts/D7-tasks-20-23.md:3690-3790`'s `Opening`, `awaitCreate` and `abandonOpened` in
  `live.ts` (WITHOUT the `dev-vars.mjs` import: `LIVE_ORIGIN` and `credential` stay where they are), then
  `:3834-3883`'s three spec edits (the create through `awaitCreate`; `opened: Opening | null`; the `afterEach` through
  `abandonOpened`; `joins_by_link`'s `Page` type import dropped if unused). Green: `npm run test:wasm` (+N), `npm run
  typecheck` (the node project includes `e2e/`), `npx playwright test --list` still lists 68 in 13.

- [ ] **Step 8: The per-edit gates** in `worker/` and `web/`; the Rust set unmoved; the fence. **No live run in this
  task** (the deployed Worker serves the M6.5c layer until Task 13, and the corpus moved in cycle A): the split
  helpers are proven by their node cells; the live specs are RUN at Task 13 F.4. The browser suite is not owed (the
  local walks' behaviour is unchanged: no local spec meets a second 409).

- [ ] **Step 9: Commit.** The body quotes Steps 2, 4 and 5's reds.

```bash
git add worker/scripts/read-deploy.mjs worker/scripts/read-deploy.test.mjs worker/test/corpus.test.ts \
  web/e2e/harness.ts web/e2e/live.ts web/e2e/two_browsers.spec.ts web/e2e/joins_by_link.spec.ts web/e2e/exit_game.spec.ts \
  web/e2e/harness.node.test.ts web/e2e/live.node.test.ts
git commit -m "fix(web): R3's four LOWs — a non-200 sw.js named by shape and never echoed, the pool's host-field exemption spent only on a present field, one designed 409 a page, and a live table known from the moment its create is sent"
```

---

### Task 8: `wasm-opt` 133 in the image, the image smoke-run under both doors, CI's `worker` job, the Docker workflow on web-lane inputs, and the whole-game gate back in CI (R-6.5c2.14; spec §2.9 as amended, §2.6; R-6.5c2.24; review M5, m4; R-C9; Review Focus 4) — from D8 retired Task 24

**Files:**
- Modify: `deploy/Dockerfile` (the false "no build.rs anywhere, checked"; the pin's prose at 133; the 132-era paragraphs
  kept as the record they are, one sentence saying so; the download at `version_133` with CI's hash; the identity
  comment's `claimed` → the open door, the repository's door `cf-access`); `deploy/Dockerfile.runtime` (the same
  identity sentence; its `:18` tag is Task 9's); `.github/workflows/test.yml` (the header comment; the `web-server`
  job's comment; the binaryen comment's two sentences about the Dockerfile's 132; the npm cache keyed by
  `web/package-lock.json` alone in `web-server`; the Worker's steps moved to a new `worker` job with their comments,
  the pool's comment naming BOTH projects; the test-binary guard step and `--lib --test wire_fixtures --test
  browser_played_pin`; `set +e` on the refusing-build step; the gate as `web-server`'s LAST step, bounded).
- Create: `deploy/smoke-image.sh` (mode 755); `.github/workflows/docker.yml`.

**Interfaces:**
- Consumes: the cargo feature `open-door` (a default feature, so the image's `cargo build --release -p yokohama-web`
  compiles it in); `IdentityMode::Open`'s name `open` on `/api/health` and `/api/me`; the mode rule under `cf-access`;
  `User::parse`'s key grammar (`"  Smoke   Runner "` keys as `smoke runner`); `web/scripts/project-url.mjs`'s
  `checkProjectUrl` through `vite.config.ts`'s define; the gate's two arms and cycle A's recorded bounded run
  (`.superpowers/sdd/2026-09-27-web-frontend-m6.5c2a-plan/`, Task 8's report: 50 seeds, min 136 / median 386 / max
  1212 inputs under the 5,000 cap, the pinned-equals-unpinned witness); the refusing build's count (8); Tasks 1–2's
  two pool projects and the guard's per-door cells (what CI's `worker` job runs).
- Produces: `deploy/smoke-image.sh <image> [port]` — exits 0 only when the image refuses to start with no door and
  answers rightly under `cf-access` and `open`, printing the image's size and each container's memory; the job ids
  Task 13 reads to green: `test.yml`'s `test`, `web-server`, `worker`, `web-client`; `docker.yml`'s `image`.

**The CI rule this task obeys (R-6.5c2.24):** nothing is ADDED to `test.yml` beyond the gate's return (~25 s) and the
test-binary guard; the `worker` job is a re-arrangement; `docker.yml` triggers on pull requests, pushes to `master` and
manual dispatch over the image's WEB-LANE inputs only — `deploy/Dockerfile`, `deploy/Dockerfile.dockerignore`,
`deploy/smoke-image.sh`, `crates/yokohama-web/**`, `crates/yokohama-wasm/**`, `web/**`, `Cargo.toml`, `Cargo.lock`,
`rust-toolchain.toml`, `.github/workflows/docker.yml` — and never on a change under `crates/yokohama-core`,
`crates/yokohama-cli` or `crates/yokohama-bridge` alone (a core change that breaks the image is caught at the next
web-lane pull request or by dispatch; the lane document records it in D). Its `concurrency` cancels a superseded run.

- [ ] **Step 1: Read the inputs this task stands on, and STOP if one is missing** — `drafts/D8-tasks-24-28.md:151-171`:
  cycle A's bounded run in its ledger (every seed under the cap; the stream seeds recorded); the refusing build's
  count at HEAD equals the workflow's `8 passed`; the gate's two cell names at HEAD (`grep -n "^async fn "
  crates/yokohama-web/tests/play_a_whole_game.rs`).

- [ ] **Step 2: The failing proof — the refusing-build step prints NOTHING when it fails (A-4)** —
  `drafts/D8-tasks-24-28.md:171-191`: a stub `cargo` that fails, and the step's own script run as GitHub runs it
  (`bash -e`). Expected at HEAD (RED): no output at all, then `exit=101`. Quote it.

- [ ] **Step 3: `test.yml`** — `drafts/D8-tasks-24-28.md:191-403` as drafted, with these changes: the header comment's
  sentence on CI's scope gains R-6.5c2.24's words ("CI runs what a clean checkout can build and what the per-edit
  process does not already run heavily; nothing is added here beyond that"); the `worker` job's pool comment names
  both projects (Task 1); the refusing-build step's count stays `8`. Then Step 6's proofs: the stub-`cargo` run now
  prints the stub's line and the `::error::` annotation before `exit=101`; the YAML parses and has the structure
  `drafts/D8-tasks-24-28.md:416-443` checks (four jobs in order; `web-server`'s last step the gate with
  `timeout-minutes: 20`; the guard step naming the three binaries).

- [ ] **Step 4: `deploy/Dockerfile` and `Dockerfile.runtime`** — `drafts/D8-tasks-24-28.md:443-563` as drafted (133 with
  CI's hash `2dc9c7813f5375db93d96ead4b78222fcc3e2677bbb832297af4797782a37489`; the build.rs sentence; the identity
  comment; the runtime file's sentence).

- [ ] **Step 5: `deploy/smoke-image.sh`** — `drafts/D8-tasks-24-28.md:563-679` as drafted: its three claims (no door →
  a refusal naming the variable; `cf-access` reads ONLY Access's header and only an address; `open` keys a typed name
  as `User::parse` does), read through `/api/health`, `/api/me` and `/api/games`, the static release and one art
  file, each container bound to `127.0.0.1` and removed on exit. **(Review Focus 4)** is its step 1 and its `ben`
  probe under `cf-access`. `chmod 755`.

- [ ] **Step 6: `.github/workflows/docker.yml`** — `drafts/D8-tasks-24-28.md:679-805` with R-C9's trigger: `on.push`
  gains `branches: [master]`; both `paths` lists are the web-lane inputs above (the core, CLI and bridge globs and
  `crates/*/Cargo.toml` REMOVED; `Cargo.lock` kept, since a dependency change moves it); the header comment states the
  narrowing and its reason (a core-only push pays no cold image build; the next web-lane pull request or a dispatch
  catches a break). The structure check (`:805-832`) is edited to the narrowed lists and passes.

- [ ] **Step 7: Build the image locally — the first build since M6.5c added a member (R3's E-3)** —
  `drafts/D8-tasks-24-28.md:832-850`: in the BACKGROUND to `task-8-docker-build.log`, POLLED; `df -h` first. Record
  the wall, `wasm-opt version 133` in the log, the stage's module size and the image size.

- [ ] **Step 8: Smoke-run it** — `drafts/D8-tasks-24-28.md:850-865`: `bash deploy/smoke-image.sh yokohama-web:task8`.
  Expected, in order: the refusal naming `YOKOHAMA_WEB_IDENTITY`; under `cf-access` every `expect` line
  (`health.identity = "cf-access"`, `smoke@ci.test` as Access's header, `null` with the claimed header and with
  `ben`); under `open` (`"open"`, `smoke runner`, `null` with Access's header); two memory lines, one size,
  `smoke: PASS`. Record the readings.

- [ ] **Step 9: The link, through Docker, both ways** — `drafts/D8-tasks-24-28.md:865-882`: the link build re-runs only
  the client stage's tail (`CACHED` above it); `grep -l` names one asset; the `ftp:` build FAILS at `build:client`
  naming `YOKOHAMA_PROJECT_URL`, `exit=1`. Remove the task's images afterwards.

- [ ] **Step 10: The per-edit gates.** No compiled file changed; run them once and confirm each count equals Task 7's
  reading. The browser suite is not owed.

- [ ] **Step 11: Commit.** The body quotes Step 2's red (the empty output, `exit=101`) and Step 8's PASS.

```bash
git add .github/workflows/test.yml .github/workflows/docker.yml deploy/Dockerfile deploy/Dockerfile.runtime deploy/smoke-image.sh
git commit -m "ci(web): the Worker's own job, the self-host image built and smoke-run under both doors on web-lane inputs only, wasm-opt 133 in the image, every layer test binary named, and the whole-game gate back in CI, bounded"
```

---

### Task 9: The version, 0.6.0 (R-6.5c2.19 as amended; spec §5, §8.2; review H3, m3, M4) — from D8 retired Task 25

**Files:**
- Modify: `Cargo.toml` (`[workspace.package] version = "0.6.0"`); `Cargo.lock` (the nine members' lines through
  `cargo update --workspace --offline`, nothing else — MEASURED: `18 +++++++++---------`); `web/package.json`,
  `web/package-lock.json`, `worker/package.json`, `worker/package-lock.json` (through `npm version 0.6.0
  --no-git-tag-version --ignore-scripts`: six lines; the third-party `@cloudflare/kv-asset-handler@0.5.0` UNTOUCHED);
  `crates/yokohama-web/src/lib.rs` (one cell after `health_reports_the_engines_own_compatibility_keys`).
- Regenerate, and READ: `web/src/wire/health.json` (`server_version`), `web/src/wire-script/archive-walk.json` (seven
  embedded records' `engine_version`; the eighth, `archive_409`'s, is `0.0.0-elsewhere` by design),
  `web/src/wire-script/backup-envelope.json` (its `app_version` ONLY, written by a LIB cell).
- Modify: `deploy/Dockerfile.runtime` (the example tag `yokohama-web:0.6.0-m6.5c2`); `web/e2e/plays_locally.spec.ts`
  (the pin sidecar's `release` READ from `web/package.json` rather than written down).

**Records kept as written** (`git grep -F "0.5.0"` afterwards names only these): the fixture pair
`browser-played-2p.{json,provenance.json}`; `worker/test/fixtures/owed-game-over.json` and `owed-pump.json`;
`worker/scripts/smoke-release.test.mjs`'s fake body; the envelope's two embedded records; every RUNBOOK record,
every `guidingpath.md` row, the `docs/plans/` files; `record.rs`'s "never-bumped" sentence inside the fence.
**Outputs another task owns:** `README.md`'s release lines and the RUNBOOK's recipe tag (Task 12); `CLAUDE.md:16`
and the board (Task 13).

**Interfaces:**
- Consumes: `client::without_version_drift` and its `warnings == []` cell over the 0.5.0 pair; `production_bot.json`
  (version-independent; must not move).
- Produces: the workspace, both npm packages and every save written from here at `0.6.0`;
  `the_two_npm_packages_carry_the_workspace_version`; **for Task 13:** the live replay's reset recipe is OWED
  (trap 19).

- [ ] **Step 1: The release-version pin** — `drafts/D8-tasks-24-28.md:942-985`: the cell reads `web/` and `worker/`'s
  `package.json` and lock files against `env!("CARGO_PKG_VERSION")` (three reads per package: the manifest, the lock's
  root, the lock's `packages[""]`). It holds at the base (every number is 0.5.0).

- [ ] **Step 2: The bump, in Cargo** — `drafts/D8-tasks-24-28.md:985-1011`: `Cargo.toml`'s version; `cargo update
  --workspace --offline`; `git diff --stat` reads `Cargo.toml | 2 +-` and `Cargo.lock | 18 +++++++++---------`. Then
  `cargo test -p yokohama-web` goes RED: the new cell (the npm packages at 0.5.0), `health.json`'s `server_version`
  in `wire_fixtures`, the envelope's `app_version` in the lib's backup cell, and `browser_played_pin`'s
  `engine_version` premise if it asserts the running version — quote every red. **Read cycle B's Task 1 cell**
  (`a_rules_hash_drift_reaches_the_wire_as_a_warning`) and its doc: its "exactly one warning" premise stood at
  0.5.0 because no drift line fired; after the bump the drift line fires and the filter drops it, so the count
  still reads one — make the doc SAY that (the C handover §7 named it as going stale), and if the cell's
  assertion rather than its doc goes red, that is a finding for the orchestrator.

- [ ] **Step 3: The npm packages follow the release** — `drafts/D8-tasks-24-28.md:1011-1024`: six `-`/`+` pairs; the
  `kv-asset-handler` grep count `0`.

- [ ] **Step 4: Regenerate, both producers, and READ the diff** — `drafts/D8-tasks-24-28.md:1024-1069`:
  `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures` (it FAILS by design at its end: "fixtures
  were REWRITTEN, not checked"), then `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --lib backup`; the diff
  script asserts exactly three files moved and each by `0.5.0` → `0.6.0` alone (`health.json | 2 +-`,
  `archive-walk.json | 14`, `backup-envelope.json | 2`). **A changed payload beyond these is a FINDING: stop.**
  Then `cargo test -p yokohama-web` green, the whole-game gate included, +1.

- [ ] **Step 5: `Dockerfile.runtime`'s tag and `plays_locally`'s literal** — `drafts/D8-tasks-24-28.md:1069-1101` as
  drafted (the tag `0.6.0-m6.5c2`; the pin's `release` read from `web/package.json` under `YOKOHAMA_WEB_UPDATE_PIN=1`;
  the pin NOT re-cut; `npm run typecheck` its check).

- [ ] **Step 6: Every crate compiled at the new version** — `drafts/D8-tasks-24-28.md:1101-1113`: `cargo test -p
  yokohama-cli` (851 + 2; the captures present), `-p yokohama-core`, `-p yokohama-oracle`, `-p yokohama-difftest`,
  `-p yokohama-bridge`, `--example eval_probe`, `--example solver` — each count equals Step 0's reading; the bridge's
  served `"version"` moves and no bridge cell asserts it (spec §5).

- [ ] **Step 7: The per-edit gates, in the order that keeps a stale module from answering the old version** —
  `drafts/D8-tasks-24-28.md:1113-1132`: the Rust set; `cd web && npm run build:wasm && npm run typecheck && npm test &&
  npm run test:wasm`; `cd worker && npm run build:module:debug && npm run typecheck && npm run test:scripts && npm
  test` (the pool's corpus replay reads the new `health.json` through the rebuilt objects — `server_version` is NOT an
  exempted host field); then `offline.spec.ts` alone: `cd web && npm run build:wasm:debug && npm run build:client &&
  npx playwright test offline` (it restores the regenerated envelope). The whole walk is not owed: no screen displays
  `server_version`.

- [ ] **Step 8: Commit** — the body quotes Step 2's reds and Step 4's stat.

```bash
git add Cargo.toml Cargo.lock web/package.json web/package-lock.json worker/package.json worker/package-lock.json \
  crates/yokohama-web/src/lib.rs web/src/wire/health.json web/src/wire-script/archive-walk.json web/src/wire-script/backup-envelope.json \
  deploy/Dockerfile.runtime web/e2e/plays_locally.spec.ts
git commit -m "chore(release): the workspace and both npm packages at 0.6.0 — one release number, pinned, and the corpus regenerated for it"
```

- [ ] **Step 9: After the commit — the drift filter's pin is LIVE now (A-7)** — `drafts/D8-tasks-24-28.md:1139-1160`: in
  an EXPORT, revert `without_version_drift` to `return warnings;` and run `cargo test -p yokohama-web --lib --test
  browser_played_pin`: exactly the cells cycle A's Task 7 report names go RED, each showing `warning:
  GameRecord.engine_version="0.5.0" does not match running engine version "0.6.0"`. A green run, or any other red, is
  a FINDING for the orchestrator. Delete the export; the result goes in the report.

---

### Task 10: The lobby's "Finished" group (spec §2.2 as amended; R-6.5c2.22, R-6.5c2.23; R-C6, R-C7; Review Focus 5) — this plan's, no draft

**Load `frontend-design:frontend-design` before writing a line.** The screen is built in mock 02's row language and
the app's own stylesheet — the `details.fold` idiom (R-B16), the `:root` tokens, no new colour role, `:focus-visible`
on the summary, `prefers-reduced-motion` honoured, nothing wider than the window at 390 px — and it ships with
pictures at 1440 and 390 beside mock 02, which the reviewer and the orchestrator read before acceptance (cycle D's
polish pass reads them again).

**What it is.** The `playing` scope lists finished games by the status rule (`tables.rs`: "over is not a
`TableStatus`"), and B's lobby shows them under "Being played" with "Spectate". Now "Being played" lists only
UNFINISHED games the caller does not sit at, and the finished ones sit in a **Finished** group below it: ONE
`details.fold`, closed by default, whose `summary` carries the `h3` "Finished" and a count ("3 games"), newest first,
"View" on every row (the padlock where `t.locked && !t.can_watch`, as "Spectate" carried it), rendered only when at
least one such game exists. No wire change, no data change, no time rule (a finished-at time is not on the wire; D's
if the count ever bothers the owner). Your own finished games stay under "Your tables" with "View", as before.

**Files:**
- Modify: `web/src/tables.ts` (`Section` gains `"finished"`; `rowAction(t, "finished")` → `{kind: "go", label:
  "View", primary: false, padlock: t.locked && !t.can_watch}`; `rowMeta` unchanged — `t.over` already reads
  "over · <name> won"); `web/src/tables.test.ts` (+3); `web/src/Lobby.tsx` (the two filters; the fourth section after
  `.playing-tables`; the "Being played" empty sentence unchanged); `web/src/Lobby.test.tsx` (+5); `web/src/styles.css`
  (one appended block "M6.5c2 cycle C: the Finished group" — the summary's row, the count, the fold's rows under
  the existing `.lobby-rows` rules; nothing restated); `web/src/Help.tsx` (its two "Being played" sentences); the Help
  cells that assert them; `web/e2e/harness.ts` (`noSidewaysScroll` and `look` LIFTED from `setup_screens.spec.ts`,
  exported, byte-identical in body); `web/e2e/setup_screens.spec.ts` (imports them); `web/e2e/plays_a_whole_game.spec.ts`
  (the fold's browser witness, appended to the first cell).

**Interfaces:**
- Consumes: `TableView.over`, `.winner`, `.locked`, `.can_watch`, `.my_seats`; `notMine`; `row(t, section)`;
  `rowMeta`; `seatChips`; the corpus's tables (`wire.table`, `wire.tableLocked` or the locked table the corpus holds —
  read `contract.ts` for its name — with `over: true` as ONE field moved, named); `Help.tsx`'s two sentences at its
  `:199` and `:252-253`.
- Produces: `Section "finished"`; `section.lobby-section.finished-tables > details.fold > summary` (the `h3` "Finished"
  and `span.section-hint` "<n> games" / "1 game"); rows `li` inside `ul.lobby-rows` with the one link; the exported
  `noSidewaysScroll(page, what)` and `look(page, name)` in `harness.ts`.

- [ ] **Step 1: The rule's cells** — `tables.test.ts`: `rowAction(over, "finished")` is `View`, not primary, no padlock
  when unlocked; **(Review Focus 5)** the locked corpus table with `over: true` and `can_watch: false` → `View` WITH
  the padlock; a finished table whose caller sits at it is never asked for `"finished"` (a type-level note: the lobby
  filters `notMine` first). Red (the `Section` type refuses `"finished"`).

- [ ] **Step 2: The lobby's cells** — `Lobby.test.tsx`, every payload a corpus table with at most one field moved:
  (a) a finished table not mine is NOT under "Being played" and IS under "Finished", whose `details` is closed
  (`open` false) and whose summary reads "Finished" and "1 game"; (b) opening it reveals the row with the button
  `"View: <name>"` and the meta "2 players · over · <name> won"; (c) two finished tables read "2 games", newest first;
  (d) with NO finished table the section is absent from the DOM; (e) **(Review Focus 5)** the locked finished table's
  View carries `svg.padlock`, and clicking it navigates to `#/t/<id>` (where the unlock face answers — the route is
  the same, the face is the server's `can_watch`). Red.

- [ ] **Step 3: The group.** `Lobby.tsx`: `const finished = playing.filter(notMine).filter((t) => t.over)` and the
  playing list `.filter((t) => !t.over)`; the fourth section rendered when `finished.length > 0`:

```tsx
<section className="lobby-section finished-tables">
  <details className="fold">
    <summary>
      <h3>Finished</h3>
      <span className="section-hint">{finished.length === 1 ? "1 game" : `${finished.length} games`}</span>
    </summary>
    <ul className="lobby-rows">{finished.map((t) => row(t, "finished"))}</ul>
  </details>
</section>
```

  with a comment saying why it is folded (R-6.5c2.22: a list that would grow without end on the NAS) and why it is
  absent when empty (R-C6). The stylesheet's block: the summary as a section head (`display: flex; align-items:
  baseline; gap`), the `h3` inside it inheriting the section head's size, the hint muted, the marker the fold idiom's
  own; nothing that reaches the game screen (`details.fold` is scoped, R-B16). Green: Steps 1–2's cells.

- [ ] **Step 4: The help text.** `Help.tsx`'s two sentences: at `:199`, "…and *Being played*, the ones you can
  spectate; finished games sit folded under *Finished*, with *View*"; at `:252-253`, "*Being played* is every game on
  the server still under way, and **Spectate** beside one opens it; *Finished*, folded below it, is every game already
  over, and **View** opens one — as does a link…". The Help cells that read those sentences are updated; the
  whole-panel cell's `no \bwatch` assertion still holds.

- [ ] **Step 5: The browser witness (R-C7).** Lift `noSidewaysScroll` and `look` into `web/e2e/harness.ts` (exported;
  `setup_screens.spec.ts` imports them; its 68-cell count is unchanged). Then, at the end of `plays_a_whole_game.spec.ts`'s
  first cell, after the winner is read: `await signIn(page, "ada@e2e.test")`; open `#/lobby`; expect
  `section.finished-tables details` present and NOT open; expect "Being played" to hold no row for the game; open the
  fold; expect the button `View: <name>`; `look(page, "lobby-finished-1440")`; `page.setViewportSize(PHONE)` (the
  constant lifted with the helpers, or restated as `{width: 390, height: 844}`); `noSidewaysScroll(page, "the lobby
  with a Finished group")`; `look(page, "lobby-finished-390")`; click View; expect the spectator's finished screen (the
  banner "You are spectating" — the name B gave it — and the winner's mark). Run `npx playwright test
  plays_a_whole_game` alone first (background, polled), then the suite (Step 7).

- [ ] **Step 6: The pictures.** Copy the two `look` shots from `test-results/` into the ledger's `task-10-look/` with
  a README naming mock 02 and what differs (the fold is not in the mock: say what it takes from the mock's rows).
  The reviewer and the orchestrator read them beside mock 02 before acceptance.

- [ ] **Step 7: The suites.** `cd web && npm run typecheck && npm test` (+8: tables 3, Lobby 5; the Help cells moved,
  not added) `&& npm run test:wasm`; the full e2e to `task-10-e2e.txt`, POLLED: 68 in 13 (the witness rides an
  existing cell). The Rust and `worker/` gates unmoved; the fence and the corpus untouched.

- [ ] **Step 8: Commit.** The body quotes Steps 1 and 2's reds.

```bash
git add web/src/tables.ts web/src/tables.test.ts web/src/Lobby.tsx web/src/Lobby.test.tsx web/src/styles.css web/src/Help.tsx web/src/Help.test.tsx \
  web/e2e/harness.ts web/e2e/setup_screens.spec.ts web/e2e/plays_a_whole_game.spec.ts
git commit -m "feat(web): the lobby folds finished games under Finished, with View — Being played lists only games under way"
```

---

### Task 11: The client's carried fixes, polish 1–4 and the a11y pass (R-6.5c2.23; the C handover §7: B's whole-branch review Minors 6, 7, 8; the Opus re-review's one Minor; the wave's concern (a); the a11y list) — this plan's, no draft

**Load `frontend-design:frontend-design` before writing a line.** Every change here ships with pictures at 1440 and
390 in `task-11-look/`; the reviewer and the orchestrator read them.

**The items, each with its site at `3ed27f07`, its fix and its cell:**

1. **The foreign SERVER game's screen takes the lock screen's shape** (Minor 6). `App.tsx`'s `.setup.broken-save`
   block (find `broken-save` by content) draws the un-rebuilt chrome (the who-strip, ghost buttons). Fix: the
   `.band-screen` + `LobbyBand(["Lobby", name])` with `serverCorner` + `.notice-card` shape the lock screen uses; the
   same for the local broken save under `["On this device"]`. Cells: the three route cells in `App.test.tsx` that
   render the foreign screen read the band and the card (the heading "Lobby" / "On this device" at level 2 and the
   refusal sentence inside `.notice-card`); the Abandon control and the way back stay.
2. **The device foot's grid under a failure sentence at 390** (Minor 7). `styles.css`'s `.device-foot` keeps
   `.offline-state` in column 1 beside Back up · Restore; a "Could not prepare to play offline — …" sentence wraps to
   nine lines in ~200 px. Fix: `.device-foot > .offline-state:has(.linkish), .device-foot > .offline-state.failed {
   grid-column: 1 / -1 }` — the component adds `failed` to the state's class when its text is not the one-line
   "Ready…" (a class, so jsdom can assert it; `:has` as belt and braces). Cell: `OnThisDevice.test.tsx` asserts the
   class on the refused state and not on the ready one. Verified in `offline.spec.ts`'s refused-registration cell
   screenshot at 390 (add `look(page, "device-foot-refused-390")` there).
3. **A refused copy that later succeeds leaves the refusal beside "Copied"** (the Opus re-review's Minor; Minor 8's
   sibling), on the open table (`OpenTable.tsx`'s `Invite`: `setCopied(true)` on the fulfilled promise) and in the
   lobby (`Lobby.tsx`'s copy handler: `setCopied(t.id)`). Fix: the success arm also clears the refusal
   (`onRefused(null)` / `setError(null)`). Cells: `OpenTable.test.tsx` and `Lobby.test.tsx`, a refused copy then a
   fulfilled one → "Copied" and NO `.refusal` strip.
4. **The single-player Game menu carries the door's foreign filter** (the wave's concern (a)). `GameMenu.tsx`'s
   "Saved games" under `!multiplayerOffered()` lists every local game, the ones the door sets aside as foreign
   included, which open onto their refusal screen. Fix: one shared stranded predicate in `tables.ts` (lift the test
   from `rowAction`'s first line into `export const isStranded = (t: TableView) => Boolean(t.bot && !t.bot.runs_here
   && !t.over)`, used by `rowAction` and by the menu), plus `unplayableIds()` (`backup.ts`) for the cross-rev rows: the
   menu lists neither. Cell: `App.test.tsx`'s single-player describe — a foreign local table and a cross-rev id are
   absent from the menu, a playable one present, and nothing reaches `fetch`.
5. **Polish 2 — the secondary controls at 390 draw ~30 px.** `styles.css`'s phone breakpoint gives the primaries
   `min-height: 44px` (`.lobby-actions button.primary`, `.state-actions button.ghost`, `.row-action .row-go`); the
   secondary controls do not (`.quiet-link`, `.to-door`, the corner's `.ghost` buttons, the door's `.linkish`). Fix:
   at the phone breakpoint, every pressable control in the setup screens has `min-height: 44px` and a matching
   `padding`. Witness: a 390 cell in `setup_screens.spec.ts`'s lobby cell — every `button` and `a` inside
   `.lobby-actions`, `.lobby-rows` and `.identity-corner` measures `height >= 44`; and the door's "How to play".
6. **Polish 3 — the device card's heading skip.** `OnThisDevice.tsx`'s outbox heading is an `h4` under the card's `h2`
   with no `h3` between. Fix: `h3`, with the stylesheet rule that sized it moved to the new selector. Cell: the
   outbox cell queries `heading, { level: 3 }`.
7. **Polish 4 — the open table's refusal sits far from its field.** `OpenTable.tsx` renders `{error && <Refusal…/>}`
   at the page's bottom, below the chat and the controls. Fix: the strip renders directly after the join block (the
   password field, the seat cards and the "Join as" button), where the join's and the password's refusals belong;
   Abandon's refusal reaches the same strip, one screen-height nearer its control. Cell: DOM order — the strip
   precedes `.open-table-controls` and follows `ol.seat-cards`.
8. **The a11y pass:** (a) the door's Close returns focus to "How to play" (`FrontDoor.tsx`: a ref on the button;
   `setHelp(false)` then `.focus()`); (b) the identity pill's accessible name is "Playing as <name>" (a `.sr-only`
   "Playing as " inside the pill's button, before the name) and its menu's action keeps its name; (c) Escape cancels
   the corner's edit and closes the phone menu (one `onKeyDown` on `.identity-corner`); (d) the project link announces
   its new tab (a `.sr-only` "(opens in a new tab)" inside the anchor); (e) the refused seed's `summary` carries
   `aria-describedby="seed-error"` while the fold is held open (R-B17); (f) "Copied" is announced (done in B's wave;
   its one manual screen-reader check is D's). Cells: one each in `FrontDoor.test.tsx`, `Band.test.tsx` (name; Escape
   twice: editing → not editing; open → closed), `NewGame.test.tsx`; (d)'s in `FrontDoor.test.tsx`.

**Files:** `web/src/App.tsx`, `App.test.tsx`; `OnThisDevice.tsx`, `OnThisDevice.test.tsx`; `OpenTable.tsx`,
`OpenTable.test.tsx`; `Lobby.tsx`, `Lobby.test.tsx`; `GameMenu.tsx`; `tables.ts`, `tables.test.ts`; `FrontDoor.tsx`,
`FrontDoor.test.tsx`; `Band.tsx`, `Band.test.tsx`; `NewGame.tsx`, `NewGame.test.tsx`; `styles.css`;
`web/e2e/setup_screens.spec.ts`, `web/e2e/offline.spec.ts`.

**Interfaces:**
- Consumes: `LobbyBand`, `serverCorner`, `.band-screen`, `.notice-card` (B's Tasks 2, 6, 7); `rowAction`;
  `unplayableIds()`; `multiplayerOffered()`; `noSidewaysScroll`, `look` (Task 10's lift); B's accessible names.
- Produces: `tables.ts`'s `isStranded(t)`; the `.failed` class on `.offline-state`; the `.sr-only` texts "Playing as "
  and "(opens in a new tab)".

- [ ] **Step 1: The cells, item by item, each red before its fix** — items 1–8 above, in that order, one commit per
  item is NOT required: one commit for the task, but each item's red is quoted. A cell that passes at HEAD is a
  finding: say so rather than keep it.

- [ ] **Step 2: The fixes**, item by item, until every cell is green: `npm run typecheck && npm test` (+N, counted per
  item in the report).

- [ ] **Step 3: The browser suite**, with the two new reads (the lobby's 44 px sweep at 390; the device foot's shot):
  `npm run e2e` to `task-11-e2e.txt`, POLLED; expected 68 in 13, the two reads inside existing cells. The Rust and
  `worker/` gates unmoved; the fence and the corpus untouched.

- [ ] **Step 4: The pictures.** `task-11-look/`: the foreign server screen and the local broken save at 1440 and 390
  (jsdom-held: rendered ONCE by a one-off script in an export, as B's Task 8 did, and named as such); the device foot
  refused at 390 (from `offline.spec.ts`'s shot); the open table with a refusal at 390; the lobby's controls at 390.
  A README names each beside its mock (01, 02, 08, 09, 11) and says what differs.

- [ ] **Step 5: Commit.** The body quotes every item's red.

```bash
git add web/src/App.tsx web/src/App.test.tsx web/src/OnThisDevice.tsx web/src/OnThisDevice.test.tsx web/src/OpenTable.tsx web/src/OpenTable.test.tsx \
  web/src/Lobby.tsx web/src/Lobby.test.tsx web/src/GameMenu.tsx web/src/tables.ts web/src/tables.test.ts web/src/FrontDoor.tsx web/src/FrontDoor.test.tsx \
  web/src/Band.tsx web/src/Band.test.tsx web/src/NewGame.tsx web/src/NewGame.test.tsx web/src/styles.css \
  web/e2e/setup_screens.spec.ts web/e2e/offline.spec.ts
git commit -m "fix(web): the foreign screens in the band's shape, the device foot under a failure sentence, a refusal cleared by a later copy, the single-player menu's foreign filter, 44 px controls at 390, the heading order, the open table's refusal by its field, and the a11y pass"
```

---

### Task 12: The hosts' documents (R-6.5c2.15, R-6.5c2.16, R-6.5c2.5, R-6.5c2.25; spec §2.11 as amended, §2.10; the C handover §7's document lines) — from D8 retired Task 26, minus what D takes

**What this task writes, and what it leaves to D.** C keeps the HOST documents: `deploy/README.md` (new, the
self-host guide), `deploy/RUNBOOK.md` rewritten for `cf-access` with its §6 moved verbatim to `worker/RUNBOOK.md`
behind a relocation map, `deploy/cloudflared-config.yml`'s comment, `worker/README.md`'s two doors and its citations,
the root README's host paragraphs and run snippet, the address scrub with its decision-log row, and the stale lines
the C handover names. D takes `docs/web-lane.md`, `CLAUDE.md`'s reduction, `guidingpath.md`'s current state, the
board's history, the README's lane sentences and the seven screenshots (spec §2.11 as amended). So **where D8's
draft points a sentence at `docs/web-lane.md`, this task points it at `CLAUDE.md` as today**, and D re-points it.

**Files:**
- Create: `worker/RUNBOOK.md` (`deploy/RUNBOOK.md` §6, moved by a script, verbatim but for its section references;
  then two sentences corrected in place, below); `deploy/README.md`.
- Modify: `deploy/RUNBOOK.md` (its head; §0 and §2 and §3's recipe and §3.2 for the NAS's `cf-access` door; §6 as
  the relocation map; the five address lines); `deploy/cloudflared-config.yml` (the ingress comment's identity
  sentence — comments only; the ingress does not move); `worker/README.md` (`:24` and the "operating manual" bullet;
  the fourteen `RUNBOOK §6.N` citations; a new §1.5 "The two doors"; the sentences this cycle made false; §1.4's
  screenshot clause; the `src/…` paths checked; `:168`'s mute-layer sentence; §9.4's secrets; §10.5's `test/` root);
  `README.md` (the run snippet; Hosting; the history note; the release lines; Where truth lives);
  `docs/handover.md` (the one transliterated address line); the two frozen plans (the exception's two lines);
  `guidingpath.md` (ONE appended row: the personal-data exception).

**Interfaces:**
- Consumes: the final state of Tasks 1–11 — the door variable and the two doors (1, 2), the refinish on the Worker
  (3), the figures' one place (4), the listing's seconds and the `forced` / `removal-ended` lines (5), `smoke-image.sh`
  and `docker.yml` (8), the version (9), the Finished group (10); `YOKOHAMA_PROJECT_URL` (B); `whoami.ts:131`'s
  sentence "see the self-host guide on identity".
- Produces: `worker/RUNBOOK.md` with `§1`–`§14` = the old `§6.1`–`§6.14`, deployments recorded from `§15` (Task
  13); `deploy/RUNBOOK.md` §6's map; `deploy/README.md` with the anchors `#1-access-first-the-repositorys-default` and
  `#2-the-open-door-a-home-network-a-lan-or-a-vpn`; `worker/README.md` `#15-the-two-doors-access-or-an-open-door`.

- [ ] **Step 1: The move script, run, and proven verbatim** — `drafts/D8-tasks-24-28.md:1204-1340`: save
  `move-section6.py` in THIS cycle's ledger (never committed); run it on `deploy/RUNBOOK.md` at HEAD; install its two
  outputs; prove the move verbatim by re-running it on the parent commit's file and diffing. Its counts (the headings
  renumbered; three NAS references named by file; two "top of §6" → "this runbook"; every `§6.N` → `§N`) are
  recorded. **Then two sentences corrected in place, each with a dated note:** the old `§6.3`'s create sentence
  (`a game must be created from a JSON object` → the layer's `NOT_AN_OBJECT` sentence as `api.rs` spells it) and the
  old `§6.5`'s "The layer's own `eprintln!` lines are mute on this host" → "since M6.5c2 the layer's log lines reach
  this host's log through the log seam, under their own names" (cycle A's item).

- [ ] **Step 2: `deploy/RUNBOOK.md`'s head, §0, §2, §3's recipe, §3.2** — `drafts/D8-tasks-24-28.md:1340-1449` as
  drafted (the two-runbook head; §0's identity bullet for the three modes and the NAS on `cf-access` "since M6.5c2
  (§3.17)"; the bind-address bullet; §2's `<tag>` and the Docker paragraph naming `docker.yml`; §3's `Env` line
  `YOKOHAMA_WEB_IDENTITY=cf-access`; §3.2 rewritten around `/api/me` naming the owner, the credential's state-B
  probe through `worker/RUNBOOK.md` §4's header file, WARP's two failure shapes). **Change from the draft:** §3.2's
  recipe cites `worker/RUNBOOK.md` §4 (the moved credential section), never a `dev-vars.mjs`.

- [ ] **Step 3: `deploy/cloudflared-config.yml`'s comment** — `drafts/D8-tasks-24-28.md:1449-1468` as drafted (the
  origin runs `cf-access`; Access is its door AND its identity; the header is worth what the path to the port is
  worth). Comments only; Task 13 records that the NAS's file needs no re-sync.

- [ ] **Step 4: `deploy/README.md`** — `drafts/D8-tasks-24-28.md:1468-1644`, its seven sections (Access first; the open
  door — a home network, a LAN or a VPN; build the image; run it; check the image before you trust it; behind a
  proxy; what the browser keeps), with these changes: §2 carries **R-B14's one line** (an operator who flips one origin
  from `open` to `cf-access` sees a stored display name in the corner until "change name"); §2's trust model is
  R-6.5c2.3's ("anyone who can reach the server may use any name"); §3 names `YOKOHAMA_PROJECT_URL` as a
  `--build-arg`, and why it is build-time (the precache's digest); §5 runs `deploy/smoke-image.sh` as Task 8 wrote it;
  §1 and §2 together are what `whoami.ts`'s "the self-host guide on identity" names — read that sentence and make
  sure the guide answers it; no sentence names `docs/web-lane.md` (D's): where the draft does, name `CLAUDE.md` or
  the section itself.

- [ ] **Step 5: `worker/README.md`** — `drafts/D8-tasks-24-28.md:1644-1787`: the citations (`:24`, the "operating
  manual" bullet, the `§6.N` → `§N` regex — the `grep` for the OLD forms then prints nothing); §1.5 "The two doors" as
  drafted, with `docs/web-lane.md` §8 replaced by "the close's recorded preconditions" (D's document is not yet
  written); the false sentences: the host table's two identity cells; the dev door's trust sentence; §9.4's variables
  and secrets (`YOKOHAMA_WEB_IDENTITY` names the door; `DEV_DOOR_CLIENT_ID` a secret since M6.5c2, put at the close's
  deploy); §11's "It did not reduce rows read" becomes "**Rows read: a warm request asks `sqlite_master` nothing since
  M6.5c2** — the tables' existence is remembered per object (MEASURED in the pool: 32 → 2 rows a warm request); the
  documents are still read up to three times on a cold path; the live figure per move is re-read at the M6.5c2 close,
  and §5's arithmetic moves with it then"; `:168`'s mute-layer sentence (the log seam, as Step 1's); §1.4's opening
  clause ("…before M6.5c2 redesigned the door, the lobby and the forms; they are recaptured in cycle D"); §10.5's
  `test/` sentence ("a plain name never takes the `test/` root; the prefix's lifecycle rule is set at the M6.5c2
  close (R-6.5c2.12)"); every `src/…` path it names exists at HEAD (the draft's script; no split happened, so none
  should be missing — a missing one is a finding).

- [ ] **Step 6: `README.md`** — `drafts/D8-tasks-24-28.md:1787-1865` with these changes: the run snippet and Hosting as
  drafted (the three hosts, the two guides, the two runbooks); the history note; **the release lines stay at
  v0.5.0 as the CURRENT release and gain one sentence** — "the workspace is at 0.6.0 since M6.5c2 (R-6.5c2.19); the
  `v0.6.0` tag is cut at cycle D's close, after the merge" — because the tag is D's; the Gates paragraph and Where
  truth lives point at `CLAUDE.md` for the lane's suites (D re-points them at `docs/web-lane.md`); the two new
  bullets under Where truth lives name the two guides and the two runbooks.

- [ ] **Step 7: The address scrub** (R-6.5c2.16). Read the address from `git config user.email` into a shell variable
  and never type it: replace it with "the owner's address" in `deploy/RUNBOOK.md`'s five lines (§3.7 twice, §3.8, and
  §3.14's Step 4b twice — find by `grep -n "$OWNER"`), each with a dated note in place ("(the owner's address;
  scrubbed 2026-09-28, R-6.5c2.16)"); in `docs/handover.md`'s Track 3 line the `_at_` transliteration becomes "the
  owner's address segment"; **in the two frozen plans**, `docs/plans/2026-09-14-web-frontend-m6.1-plan.md` (a `curl`)
  and `docs/plans/2026-09-20-web-frontend-m6.5b-plan.md` (a test literal), the two lines are edited to "the owner's
  address" under the recorded exception; `crates/yokohama-web/src/archive.rs` is confirmed clean (cycle A rewrote the
  cell). Then `git grep -n -F "$OWNER"` prints NOTHING, and `git grep -n "_at_gmail"` prints nothing.

- [ ] **Step 8: The exception's decision-log row** — appended to `guidingpath.md`'s table, dated 2026-09-28: the
  personal-data exception R-6.5c2.16 records ("Record a personal-data exception", the owner, 2026-09-26), naming the
  two frozen plans and their two lines, and that the frozen-record rule stands for everything else. (The cycle row is
  Task 13's.)

- [ ] **Step 9: The links resolve, and no current-state sentence says `claimed`** — `drafts/D8-tasks-24-28.md:1865-1896`'s
  link checker over `README.md deploy/README.md deploy/RUNBOOK.md worker/README.md worker/RUNBOOK.md`
  (`links: OK`), and its `awk` over the runbook's current-state part (only §0's rename sentence names `claimed`).

- [ ] **Step 10: The per-edit gates.** No compiled file moved and no code reads these documents; every count equals
  Task 11's reading. The browser suite is not owed.

- [ ] **Step 11: Commit.**

```bash
git add worker/RUNBOOK.md deploy/RUNBOOK.md deploy/README.md deploy/cloudflared-config.yml worker/README.md README.md \
  docs/handover.md docs/plans/2026-09-14-web-frontend-m6.1-plan.md docs/plans/2026-09-20-web-frontend-m6.5b-plan.md guidingpath.md
git commit -m "docs(hosts): a README and a RUNBOOK for each host — the self-host guide leads with Access and gives the open door its own section, RUNBOOK §6 moves verbatim to worker/RUNBOOK.md behind a map, the NAS's recipe takes cf-access, and the owner's address is scrubbed under the recorded exception"
```

---

### Task 13: The cycle's close (ORCHESTRATOR; spec §3 as amended; R-6.5c2.13; R-6.5c2.20; the M6.5c close's order)

No code. **Every step here is the orchestrator's**, and every step that touches the account is PRE-AUTHORIZED (Global
Constraints). The order is the M6.5c close's, so a fix found in review never forces a second deploy or a second live
sequence: review → one fix wave → the scoped re-reviews → the battery → the ONE recorded sequence → the deploys →
the live tiers → the close document → the push and PR #41 → the handover into D. **Not here:** the seven
screenshots, the merge and the `v0.6.0` tag — cycle D's (R-6.5c2.20; §4 row D). The ledger is
`L=.superpowers/sdd/2026-09-28-web-frontend-m6.5c2c-plan`.

**A. The branch is whole**

- [ ] **A.1** `git status` clean; `progress.md` holds a DONE line and a review line for each of Tasks 1–12; HEAD is Task
  12's reviewed commit.
- [ ] **A.2 The fences**, output quoted in the close: the five crates and the CLI (`git diff --stat 3ed27f07 -- crates/yokohama-core
  crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli` prints
  nothing); `SEMANTICS_REV` 7; `git diff --stat 3ed27f07 -- crates/yokohama-web/src` lists ONLY `bots.rs`, `api.rs`,
  `explain.rs`, `games.rs`, `log.rs`, `lib.rs` (Tasks 6 and 9's named places); `git diff --stat 3ed27f07 -- web/src/wire
  web/src/wire-script` lists ONLY the three files Task 9 regenerated.

**B. The review (Fable), one fix wave, the scoped re-reviews**

- [ ] **B.1** One **Fable** reviewer over `git diff 3ed27f07..HEAD`, the spec at revision 2.7, this plan and the ledger,
  working in an EXPORT: the vacuity lens first (every new catcher reverted against its fix — the two doors' cells,
  the refusals, `settledArm`, the picker watch, the memo, the labels, the LOWs, the fold); **the pictures** (Tasks
  10 and 11's `task-<n>-look/`, read beside mocks 01, 02, 08, 09, 11, and any screen the tasks could not photograph
  rendered once in the export); the sentences this plan and the documents (Task 12) write about themselves TRIED
  rather than re-read (the top yield, every cycle: a false sentence a cycle wrote about itself); the strings rule and
  the two time bases; the fences; findings CRITICAL / HIGH / MEDIUM / MINOR / OPEN and "escalations outside the
  brief". Report: `$L/final-review.md`.
- [ ] **B.2** **ONE fix wave**, each fix a NEW commit by an Opus implementer on the orchestrator's dispositions, each
  dispatch saying "as a new commit", disjoint files if parallel; a Worker-side fix runs `build:module:debug` before its
  pool; a fix that changes what a browser shows owes the e2e, sequenced. A triage line per finding (FIX / D / drop,
  with the reviewer's reason), as B's close recorded it.
- [ ] **B.3** The scoped re-reviews of the wave — an Opus one and a Fable one, in parallel over the wave's diff, each in
  its own export (B's precedent). Their head is **the reviewed head** for everything below.

**C. The mutation battery** (spec §3; R-6.5c2.13), at the reviewed head

- [ ] **C.1** An Opus battery agent, on an EXPORT (its own `CARGO_TARGET_DIR`), never the tree: each row applies the
  mutant, sees the named catcher fail BY NAME, reverts, and records the diff it applied. The agent enumerates the
  tasks' reports again at execution — a catcher with no row gets one; a row whose catcher does not fail is a FINDING.
  The door and refinish rows first. Report: `$L/battery.md`.

| # | the mutant | the catcher (the owning task's report names the cell) |
|---|---|---|
| 1 | an unset `YOKOHAMA_WEB_IDENTITY` defaults to `worker` | Task 1: the 500 at `handle` (Review Focus 2) |
| 2 | the shell reads the claim from the header on an upgrade, or the parameter on a fetch | Task 1: one carrier per request |
| 3 | the upgrade's parameter read undecoded | Task 1: `a+b@x.test` / `ben castle` over the socket (Review Focus 1) |
| 4 | a non-ASCII `claimed=` reaches `headers.set` | Task 1: no caller, never a platform 500 |
| 5 | `identity_of`'s fallback mode left at `Worker` under `open` | Task 1: the module's `"ben\r"` cell |
| 6 | `host()` names `cf-access` under `open` | Task 1: `/api/health` and `/api/me` say `open` |
| 7 | `operatorGate` answers `refused` under `open` | Task 2: the layer's 404 whatever name is claimed |
| 8 | `shellConfig` accepts `YOKOHAMA_WEB_ADMINS` beside `open` | Task 2: the 500 naming both |
| 9 | `shellConfig` accepts `YOKOHAMA_DEV_DOOR=on` beside `open` | Task 2: the 500 naming both |
| 10 | `ReceiptArchive::root` files `a.test` under `test/` | Task 2: saves outside `test/` |
| 11 | the guard: an absent door passes | Task 2: the guard's cell |
| 12 | the guard: `workers_dev` passes under `worker` | Task 2 |
| 13 | the guard: `preview_urls` passes under `open` | Task 2 |
| 14 | the guard: `YOKOHAMA_DEV_DOOR=on` passes under `open` | Task 2 |
| 15 | the guard accepts `DEV_DOOR_CLIENT_ID` in `wrangler.jsonc`'s `vars` | Task 2: check 7's second key |
| 16 | a conflict read through `stampArm` | Task 3: `settledArm`'s rows, and the race (Review Focus 3) |
| 17 | `held` accepted beside `landed` | Task 3: `parse_outcome`'s refusal |
| 18 | `executePlan` puts the table before reading the record key | Task 3: the pair stays one game's |
| 19 | a 409 stamp reply handed to `parseStamp` | Task 3: the foreign-stamp cell (a throw) |
| 20 | `events_inner` reaches `Games::handle` outside the watch | Task 4: the picker cell |
| 21 | `ReceiptArchive.pending` keeps the last plan | Task 4: one plan per call |
| 22 | a field added to `BotDescriptor` in the export | Task 4: E0027 in `house.rs` (the compiler) |
| 23 | `std::fs::read` in a host file | Task 4: both hosts' switched scans |
| 24 | the cap check moved after `events_at` | Task 4: R-C8's line |
| 25 | the memo removed | Task 5: the `sqlite_master` cell |
| 26 | `forgetTables` removed from `end` | Task 5: the thirteen catchers D7 measured |
| 27 | `last_active` left in milliseconds | Task 5: the listing cell |
| 28 | `forced` logged for a markerless removal | Task 5: `removal-ended` |
| 29 | `deleting` reads every document | Task 5: the ≤ 2 reads cell |
| 30 | `distinguishing_labels` returns plain labels on a tie | Task 6 |
| 31 | `persist` re-stamps `run_id` | Task 6: the run-id pin |
| 32 | the print scan without `io::stderr(` | Task 6: the `writeln!` probe |
| 33 | `deployedRelease` echoes the body | Task 7: L-3 |
| 34 | the host-field exemption spent on an absent field | Task 7: L-5 |
| 35 | `dealAnswers` uncapped | Task 7: L-7 |
| 36 | a live create without `awaitCreate` | Task 7: L-9's structural cell |
| 37 | a new `crates/yokohama-web/tests/zz_probe.rs` | Task 8: the test-binary guard step (run the step's script) |
| 38 | the refusing-build step without `set +e` | Task 8: the stub-`cargo` proof prints nothing |
| 39 | `docker.yml` triggered by `crates/yokohama-core/**` | Task 8: the structure check |
| 40 | the image with a default door | Task 8: the smoke's step 1 (Docker present, else "not run" recorded) |
| 41 | `web/package.json` at `0.6.1` | Task 9: the version pin |
| 42 | `without_version_drift` reverted | Task 9: `warnings == []` over the 0.5.0 pair, LIVE since the bump |
| 43 | a finished game listed under "Being played" | Task 10 |
| 44 | the Finished row labelled "Spectate" | Task 10 |
| 45 | the locked finished row without the padlock | Task 10 (Review Focus 5) |
| 46 | the door's Close not returning focus | Task 11 |
| 47 | the menu listing a foreign local game | Task 11 |
| 48 | `isStranded` reading `over` only | Task 11 / Task 10's `rowAction` |

**D. The ONE recorded sequence** (R-6.5c2.13; spec §3 steps 1–3), at the reviewed head

- [ ] **D.1** Save `drafts/D8-tasks-24-28.md:2626-2700`'s script as `$L/close-sequence.sh` with `L=` set to THIS
  ledger and run it from the repository root as ONE uninterrupted run, machine-tagged, in the BACKGROUND with its log
  in the ledger; POLL `tail -n 3 "$L/sequence.tsv"`, never a notification. Its rows: the per-edit suites of every
  crate and package (the refusing build, the wasm check, clippy, `cargo doc`, both wasm builds, the four bridge node
  suites, `worker/`'s four — `npm test` runs BOTH pool projects — and `web/`'s five, the e2e among them), then the
  fifteen release gates with their binaries pre-built (core compiled at 0.6.0).
- [ ] **D.2 Read it.** Every `rc` is 0; the gates' walls summed as the roster's and machine-tagged beside M6.5c's 533 s,
  never against it; each count set against Step 0's baseline and the per-task readings; a red row re-run ALONE at the
  same commit before it is believed (the whole-game gate's included), read, never retried into green; `cargo doc`
  0 warnings.

**E. ORCHESTRATOR — the account: the deploys and the bucket** (spec §3 step 4; §2.10)

- [ ] **E.1 Before anything live:** WARP's device login (`warp-cli status`, and a browser-shaped `curl -A 'Mozilla/5.0'`
  of `https://yokohama.moore.nyc/api/health` must print 200 — Access's 302 means the owner re-authorizes, trap 25);
  `cd worker && npx wrangler whoami` names the owner's account; **ask the owner** for `YOKOHAMA_PROJECT_URL` for this
  release, or none.
- [ ] **E.2 The release build, ONCE, for both hosts:** `cd web && npm run build` (with the link in the environment if
  the owner gave one); `grep -c debug_trap src/local/wasm/yokohama_wasm.js` prints 0 (the e2e left the DEBUG module);
  `cd ../worker && npm run build` (module → assets → `smoke:release`). Record the precache generator's four lines and
  the release name.
- [ ] **E.3 The Worker, with `DEV_DOOR_CLIENT_ID` as a SECRET in the same version** (A-1; spec §2.1). From `worker/`,
  the secret goes through a mode-600 file, read from `.dev.vars` by the shell and never printed:

```bash
S="$SCRATCH/dev-door.secrets"; umask 077
id="$(grep '^CF-Access-Client-Id=' ../.dev.vars | cut -d= -f2- | tr -d '[:space:]')"
[ -n "$id" ] || { echo ".dev.vars names no CF-Access-Client-Id" >&2; exit 1; }
printf 'DEV_DOOR_CLIENT_ID=%s\n' "$id" > "$S"; unset id
npx wrangler tail yokohama-web --format json > "$SCRATCH/tail21.json" &     # listening before the deploy
node scripts/deploy-guard.mjs && npx wrangler deploy --secrets-file "$S"
rm -f "$S"
npx wrangler secret list                                                   # names only: DEV_DOOR_CLIENT_ID, YOKOHAMA_WEB_ADMINS
```

  Expected: the guard passes and prints the door `worker`; wrangler prints the new version id (the twenty-first
  deployment); `secret list` names both secrets. This deploy changes the layer's sources, so every open picker is
  dropped at its table's next request, logged `picker-dropped` (I-85, I-91); it restarts every object and closes
  every socket — deploy between games.
- [ ] **E.4 The Worker's readings** (`worker/RUNBOOK.md` §5), over WARP as the owner: `/api/health` (`identity:
  "cf-access"`, `signal: "ws"`, `archive: "r2"`, `server_version: "0.6.0"`); `/api/me` naming the owner (recorded as
  "the owner's address"); the dev door works with the secret (the header file `$H` of `worker/RUNBOOK.md` §4 plus
  `X-Yokohama-Claimed-User: smoke@readme.test` reads `user: "smoke@readme.test"`; the credential alone `null`); the
  served `/sw.js` names E.2's release; the tail's `picker-dropped` lines counted; **the rows read, the F-4 reading the close
  claims**: the account's GraphQL analytics, the Durable Objects rows-read dataset, read before and after a
  counted burst of warm requests exactly as the M6.5c close §3.4 measured it (`worker/RUNBOOK.md` §7, the quota
  guard's recipe), beside M6.5c's ~470 a move.
- [ ] **E.5 The `test/` lifecycle rule** (R-6.5c2.12) — `drafts/D8-tasks-24-28.md:2713-2732` as drafted: re-read R2's
  command reference and `npx wrangler r2 bucket lifecycle add --help` first; **NEVER pass `--force`**; not `lock add`;
  `npx wrangler r2 bucket lifecycle add yokohama-archive expire-test-after-30-days test/ --expire-days 30`, then
  `lifecycle list` read back: the rule enabled, prefix exactly `test/`, 30 days, beside the default multipart-abort
  rule; no expiring rule reads "(all prefixes)". Record both rules.
- [ ] **E.6 The NAS** — `drafts/D8-tasks-24-28.md:2733-2790` as drafted, following `deploy/RUNBOOK.md` §1–§3 as Task 12
  left it: read the running container first (Portainer MCP, environment 2); the tables on the volume as COUNTS (the
  Portainer archive read; the counting script prints counts and never an address); the musl build; the context
  (`COPYFILE_DISABLE=1`; zero `._` entries; the `.dev.vars` 404 check on the file server BEFORE the POST — P-1);
  the image `yokohama-web:0.6.0-m6.5c2` through `dockerProxy` `POST /build`; the swap with `Env` exactly
  `["YOKOHAMA_WEB_IDENTITY=cf-access", "YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive"]`; the listening line and the downtime.
- [ ] **E.7 The NAS is NOT deployed until `/api/me` names the owner** (review R2.2-M3) — the first live reading of the
  layer's `CfAccess` arm on any host: `/api/health` (`cf-access`, `0.6.0`); `/api/me` naming the owner; `?scope=mine`
  listing the owner's tables (E.6's count); the credential's header file reads `user: null` and `/api/games` 401 —
  state B, by design; `/enter` 302; `/` and `/sw.js` E.2's release; memory (`/stats?stream=false`). **`/api/me`
  naming anyone else, or `null` over WARP, is a STOP**: roll back by §5 to `yokohama-web:0.5.0-m6.5b2.1` with the same
  `Env` (that binary knows `cf-access`) and diagnose. With Claude-in-Chrome connected, the owner's own browser opens
  `https://yokohama-nas.moore.nyc/#/lobby`: the corner shows the Access identity, never the stored name (B's Review
  Focus 3, live); a disconnected extension is recorded as "not read".

**F. ORCHESTRATOR — the live tiers** (spec §3 steps 5–6)

- [ ] **F.1 The live replay's reset FIRST** (trap 19; review M4): Task 9 changed `archive-walk.json`'s records, so the
  bucket's four keys under `test/browser/red_at_wire.test/rev7/{over,abandoned}/wire{,.table}.json` go by `npx wrangler
  r2 object delete "yokohama-archive/<key>" --remote`, with wrangler's login.
- [ ] **F.2 The quota guard** (`worker/RUNBOOK.md` §7): the UTC day's Worker requests and Durable Object rows written,
  one GraphQL dataset per call; HOLD above 80,000.
- [ ] **F.3 The live replay, twice, sequentially:** `cd worker && node scripts/live-replay.mjs`; the first run's two first
  sends 201, the second run the once-rule's `identical: true`; each HOLDS, every named row byte-identical.
- [ ] **F.4 The quota guard again, then the live Playwright tier:** `cd web && YOKOHAMA_WEB_E2E_TARGET=live npx
  playwright test two_browsers joins_by_link` — one run at a time; the only forgiven shapes the beacon's CORS pair
  and ONE deal 409 a page (Task 7's cap: a second is breakage); the quota read after (~32,000–38,000 requests a
  game); **the rows read for the game, the F-4 live reading** (E.4's method, the same dataset over the game's
  window), recorded beside M6.5c's ~1.34 M for a 2,862-move game.
- [ ] **F.5 The headed exit game, in a window with no other deploy** (`worker/RUNBOOK.md` §7): open the window with
  `node scripts/deploy-guard.mjs && npx wrangler deploy --var YOKOHAMA_WEB_COUNT_WAKES:on` (the twenty-second
  deployment; secrets are inherited); tail it; play `cd web && WAKE_PAUSE_MS=60000 YOKOHAMA_WEB_EXIT=1
  YOKOHAMA_WEB_E2E_TARGET=live npx playwright test exit_game --headed` (the accessible names B rewrote: handover §2);
  close the window with `npm run deploy` in `worker/` (the twenty-third). Record the wakes, the move wall and CPU
  off the tail, the quota after.

**G. The close document, the records, and the current-state documents** (spec §3 step 7 as amended)

- [ ] **G.1** Write `docs/plans/<the day>-web-frontend-m6.5c2c-close.md` on the M6.5c and B closes' shape: the header
  (the branch, the base `3ed27f07`, the spec at revision 2.7, this plan, the execution shape); the headline and **what
  is LIVE** (the Worker's version ids 21–23; the NAS's `yokohama-web:0.6.0-m6.5c2` on `cf-access`; the `test/` rule);
  §1 what shipped by task, in execution order, and §1.x the fences (A.2's output); §2 the counts in ONE sequence
  (`sequence.tsv` as a table, machine-tagged; the roster's wall beside 533 s); §3 the deployments (both hosts'
  readings; `/api/me`'s shape on the NAS with "the owner's address"), the live tiers (both replays, the two-browser
  game, the exit's wakes), the F-4 live reading, the quota, the lifecycle rule; §4 the reviews (B) and the battery
  (C's table with each row's verdict); §5 every ruling taken in execution with its cost — R-C1–R-C9 and every ledger
  `Ruling:` line; §6 the LEAVE list and the owner items (the phones, after this deploy; the public door's
  preconditions; the polish items 5–9 with the pictures' paths); **§7 what this cycle does NOT claim** — spec §6
  corrected by what execution found, at least: no public door; no open-door Worker deployed (the open door is read in
  the pool's second project, the local e2e and `docker.yml`'s smoke); the NAS's `cf-access` read once, as the owner;
  no not-over foreign game on either host (the refusal and the versioned sentence are cell-read); `run_id` and the
  mirror's `bot` only on games after the deploy; the livelock bound measured over recorded seeds, not proven;
  overlapping uploads' pair consistency best-effort; requests per move unchanged, the rows read reduced by E.4/F.4's
  reading; the address in the git history; the seven screenshots, the lane document, the merge and the tag are D's;
  the phones the owner's; `docker.yml`'s first wall is read at the push (I.1); §8 the handover: cycle D, from the
  document I.3 writes.
- [ ] **G.2** The records and the current-state documents:
  - **`deploy/RUNBOOK.md` §3.17** — the fourteenth NAS deployment on §3.16's shape (the build's figures; the context;
    the image; the swap; E.7's verification with "the owner's address"; the tables' seat counts; memory; the
    cloudflared comment drift needing no re-sync; rollback: not a schema rollback — `TABLE_SCHEMA` stays 3, an older
    binary ignores the mirror's `bot` and reads `run_id`, and shows the drift line on 0.6.0 records; "What this
    deployment does NOT claim").
  - **`worker/RUNBOOK.md` §15–§17** — the twenty-first to twenty-third deployments on §8's and §13–§14's shapes with
    E.3–E.5 and F's readings; §7's `test/` sentence becomes the rule's ("`expire-test-after-30-days` expires them 30
    days after they were written (§15)"); `worker/README.md` §10.5's sentence likewise.
  - **`CLAUDE.md`, the truth pass** (R-6.5c2.25): every sentence C made false, rewritten in place — the NAS's mode and
    build (`cf-access`, `0.6.0-m6.5c2`); the Worker's version (the twenty-third deployment); `RUNBOOK §6` → `worker/RUNBOOK.md
    §N` wherever cited; `DEV_DOOR_CLIENT_ID` a secret; the two pool projects and the `worker` CI job; `docker.yml`; the
    `test/` rule; every count that moved, named with its reading (this close); the Docker image's `wasm-opt` 133; one
    sentence under Conventions stating the escape rule ("every INVISIBLE non-ASCII code point in code is a `\u`
    escape; a visible glyph may be raw"). The web section's REDUCTION to a pointer is D's, not this pass's.
  - **`guidingpath.md`**: the current-state paragraph's M6.5c2 sentence names cycle C CLOSED and D next; ONE appended
    row, `M6.5c2 CYCLE C "THE HOSTS AND THE CLOSE" EXECUTED, CLOSED AND LIVE`, naming the close, the reviewed code
    head, the deployments (the Worker's version ids, the NAS's tag), the rulings R-6.5c2.20–.27 and R-C1–R-C9 by
    number, the fences, and that the merge and the tag follow in D.
  - **the board**, pointing at the close and at D's handover (I.3).
  - **the spec**, gaining `§10.8 Corrections recorded in execution (<the day>)`: one row per ledger amendment — the
    sentence as written, what execution found, the ruling.
- [ ] **G.3** Commit (documents only; the fences re-read):

```bash
git add docs/plans/*-web-frontend-m6.5c2c-close.md docs/plans/2026-09-26-web-frontend-m6.5c2-spec.md deploy/RUNBOOK.md worker/RUNBOOK.md worker/README.md CLAUDE.md guidingpath.md docs/handover.md
git commit -m "docs(close): M6.5c2 cycle C — the hosts and the close: the record of record, both hosts' deployment records, and the current-state documents made true"
```

**H. ORCHESTRATOR — the push, PR #41, and the handover into cycle D** (pre-authorized)

- [ ] **H.1 The push and the PR.** `git push -u origin web-room-m6.5c2c`, then `gh pr create --base web-room-m6.5c2b
  --head web-room-m6.5c2c` with a description that is the cycle in one screen (what landed by task, the readings at the
  reviewed head, what is LIVE, what the cycle does NOT claim, the attribution line). **Read CI to green:** `gh run list
  --branch web-room-m6.5c2c`, then `gh run watch <id>` — `test.yml`'s four jobs, and `docker.yml`'s `image` (the branch
  touches web-lane inputs, so its pull request triggers it): **record its first wall** in the close's §3 (a wall over
  fifteen minutes invokes R-C9's fallback, recorded for D). The spec's §4 says "expected #41"; the number GitHub gives
  is the record.
- [ ] **H.2 Nothing is merged and nothing is tagged**: D's (R-6.5c2.20).
- [ ] **H.3 The handover into cycle D — its planning session's ENTRY.** Write
  `docs/plans/<the day>-web-frontend-m6.5c2d-handover.md`, under 200 lines, every fact read off the TREE at the
  reviewed head, never off this plan; its sections, in order:
  1. **Where cycle C left the tree:** the branch, its HEAD, the PR; every suite's reading; both fence checks;
     **what is LIVE on both hosts** (the version ids, the tags, the doors, the rule) and the live tiers' ending state.
  2. **What cycle D builds** (spec §4 row D, restated) **and its raw material:** the lane document's draft, D8 retired
     Task 27 (`drafts/D8-tasks-24-28.md:1911-2453`: `docs/web-lane.md`'s eleven sections, the `CLAUDE.md` move script,
     the board's rewrite), corrected for what C changed; the seven screenshots' recipe (D8 Task 28's G, `:2800-2830`)
     against the DEPLOYED site; the polish items 5–9 with the paths of C's pictures; the phones, with the Device Hub
     iOS simulator (`mcp__mobile__*`) as the option the owner named; the investigation's inputs (the LEAVE lists of
     every close since M6.1, the M6.5c close's §7, this close's §7, the public door's preconditions, the AI lane's
     re-entry pointer `docs/plans/2026-09-11-ai-lane-reentry-handover.md`); **the merge and the tag recipe** (D8 Task
     28's I.2–I.3, `:2929-2990`, with the stack #31 → #42 and `delete_branch_on_merge` false: each PR retargeted by hand
     before its merge; the `v0.6.0` tag on master after the merge; `release.yml` read to green).
  3. **The rules D carries:** the standing set; the owner's authorization spent here and to be re-asked for D's merge
     and tag; the escape rule; no deploy in D unless a polish fix needs one (then the Worker's guard and the NAS recipe
     as C left them).
  4. **What cycle C does NOT claim** (the close's §7, one line each).
  5. **Every ruling taken in this cycle's ledger**, one line each, dated.
  6. **Cycle D's standing items this cycle created or found.**

  Then `docs/handover.md`'s "Updated" line and Track 3 point at the close and the handover and say cycle D is next;
  the orchestrator's memory (`yokohama-web-lane-status`, the index) records the cycle's end, what is live, and the
  PR number. Commit the handover and the board together; push.

---

## Appendix A — the drafts' line map, and what is dropped

The drafts were written for a 28-task plan against `1e16fe27` and refer to each other by THOSE numbers. Reading one,
translate:

| a draft says | it means here | the draft's lines |
|---|---|---|
| Tasks 1–8 (D1–D3) — the layer | cycle A, DONE | — |
| Tasks 9–16 (D4–D5) — the screens | cycle B, DONE | — |
| Task 17 (D6) — the two doors | **Task 1** | `D6-tasks-17-19.md:113-1574` |
| Task 18 (D6) — the open door's refusals | **Task 2** | `D6-tasks-17-19.md:1575-2388` |
| Task 19 (D6) — the refinish on the Worker | **Task 3** (+ the foreign-stamp cell, this plan's) | `D6-tasks-17-19.md:2389-3099` |
| Task 20 (D7) — the `table.ts` split | **DROPPED** (R-6.5c2.1 as amended); its names map back to `table.ts` | `D7-tasks-20-23.md:135-1019` |
| Task 21 (D7) — read once; the time unit; L-8; L-4; the memo | **Task 5**: the memo (its Steps 9–11), the time unit (Step 6), L-4 (Step 7), L-8 as R-C4; the read-once thread DROPPED | `D7-tasks-20-23.md:1020-1634` |
| Task 22 (D7) — the invariants, the imports, the figures | **Task 4** (+ R-C8's line; `delivery.ts` → `table.ts`) | `D7-tasks-20-23.md:1635-2595` |
| Task 23 (D7) — the scripts and the live tiers | **Task 7**: L-3, L-5, L-7, L-9 only; `dev-vars.mjs`, the origin, the replay's split, the shared rules module and the one precache reader DROPPED | `D7-tasks-20-23.md:2596-3956` |
| Task 24 (D8) — wasm-opt 133, CI, Docker, the gate | **Task 8** (+ R-C9's narrowed trigger) | `D8-tasks-24-28.md:98-896` |
| Task 25 (D8) — the version | **Task 9** | `D8-tasks-24-28.md:897-1162` |
| Task 26 (D8) — the hosts' documents | **Task 12** (minus the lane-document pointers) | `D8-tasks-24-28.md:1163-1910` |
| Task 27 (D8) — the lane document, `CLAUDE.md`, the board, the scrub | **cycle D**, except the address scrub and its row (Task 12) and the `CLAUDE.md` truth pass (Task 13) | `D8-tasks-24-28.md:1911-2453` |
| Task 28 (D8) — the close | **Task 13** (A–H), minus G's screenshots and I's merge and tag (cycle D) | `D8-tasks-24-28.md:2454-2990` |
| the Finished group; the carried fixes, polish 1–4 and the a11y pass | **Tasks 10, 11** — this plan's, no draft | — |
| the versioned refusal sentence; the run-id pin; the print scan | **Task 6** — this plan's, from cycle A's items | — |

**The drafts' base is `1e16fe27`**; cycles A and B moved most files they cite, and the drafts' stand-ins for cycle A's
outputs are superseded by the tree (the `Task 1`/`Task 2`/`Task 6`/`Task 7` references in D6 and D7 are cycle A's
real `User::parse`, `IdentityMode::Open`, `admit`, `PutError::Conflict { held }` and the five exports). Their test
people (`Ben Moore`, `Jeremy`) are replaced as each task says. Their "Expected" counts are the exports' readings and
are stated here as deltas; every implementer re-counts.

## Appendix B — the ledger, and how to use it

`.superpowers/sdd/2026-09-28-web-frontend-m6.5c2c-plan/` (gitignored):
- `progress.md` — the SDD identity line (`# SDD ledger — plan: docs/plans/2026-09-28-web-frontend-m6.5c2c-plan.md`),
  the standing rules, Step 0's baseline, one line per task with its readings and its report paths, one `Ruling:` line
  per deviation (R-C1–R-C9 recorded at the top).
- `task-<n>-brief.md`, `task-<n>-report.md`, `task-<n>-review.md` — every dispatched agent's brief and full report.
- `task-<n>-e2e.txt` — every Playwright run's output, written by the implementer and POLLED.
- `task-8-docker-build.log` — the image's first build since M6.5c.
- `task-<n>-look/` — the pictures for Tasks 10 and 11, named for the screen and width, beside a one-line `README`
  naming the mock each answers to and what differs.
- `move-section6.py` — Task 12's move script (never committed).
- `final-review.md`, `fix-wave-*.md`, `final-rereview.md`, `battery.md`, `close-sequence.sh`, `sequence.tsv`,
  `sequence/NN.log`, `tail21.json`, `pr-body.md`, `handover-draft.md` — the close's.
- The retired plan's workspace (`…/2026-09-26-web-frontend-m6.5c2-plan/`) holds the drafts, `review-plan-opus.md`
  and `drafter-artifacts/` (D6's and D7's export notes); cycle A's ledger (`…/2026-09-27-web-frontend-m6.5c2a-plan/`)
  holds Task 8's bounded-run report that Task 8 here reads; cycle B's (`…/2026-09-27-web-frontend-m6.5c2b-plan/`)
  holds `final-review.md` and `fix-wave-review.md`, whose carried items Task 11 closes, and the screens' pictures.
