# M6.5c2 cycle D — the code: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan
> task-by-task. The execution shape is §"Execution shape" below: every task subagent-driven, the implementer's tier
> marked in each task's title (**Sonnet 5.5** for the implementation-focused tasks, **Opus** where judgment is needed,
> R-6.5c2.36), every task reviewer Opus, Fable for the whole-branch review at the close; every account step — a
> deploy, a live run — the ORCHESTRATOR's alone. Steps use checkbox (`- [ ]`) syntax. **Read the spec first**
> (revision 2.8: §10.9 is this cycle's rulings and §4's rows D and E as amended), then the dossier
> `docs/plans/2026-09-28-web-frontend-m6.5c2d-dossier.md` — **every task cites its rows there by number for WHERE the
> code is and what it reads NOW**, and carries the design, the tests and the steps itself. **Read §"Couplings between
> tasks" before any task**: an implementer sees only its own task.

**Goal:** The web lane's code as it will stay: every straw-poll row the owner marked "fix in D" fixed, batched by area —
the layer's defects and the NAS server's minors, the seat-email roster off the wire, chat delete and moderation and
the archive route's bounds, the R2 backoff and the retired held arm, the Worker refactors under a budget, the layer's
and the client's test debts, the client's defects by screen, the local host and its backup — with CI reshaped first
and the small debts closed; then the close: the review, the ONE recorded sequence, both hosts deployed, the live tiers
with three readings only a deploy affords, the close document, PR #42 and the handover into cycle E.

**Architecture:** Change runs from the wire inward and from the layer outward. CI first (Task 1), so the branch's own
pushes cost nothing. The small debts and the two records (2, 3) touch no behaviour. The layer's defects (4) and the
NAS server's (5) land before the roster (6), which reshapes every table payload and the chat's author, and before
chat delete (7), which adds routes beside it; both regenerate the corpus, which is why they precede everything the
corpus feeds. The Worker's two changes (8) and its refactors (9) come after the layer is quiet, because the pool's
2,209-row replay is their fence and it replays the corpus 6 and 7 leave. The layer's tests (10) pin what 4–7 built.
The client's three tasks (11–13) read the wire as 6 and 7 leave it; the client's tests (14) last, because every cell
they add reads the screens as 11–13 leave them. The close (15) is the orchestrator's.

**Tech Stack:** Rust 1.97.1 (`crates/yokohama-web`, `crates/yokohama-worker`, `crates/yokohama-wasm`; `wasm-bindgen`
0.2.128, `release-wasm`, `wasm-opt` 133); TypeScript in `worker/` (the shell, vitest node cells, the POOL inside workerd
in two projects) and in `web/` (React 18, vitest over jsdom, Playwright over Chromium against the release binary);
GitHub Actions; the owner's Cloudflare account and the NAS through Portainer (the close only).

**Spec:** `docs/plans/2026-09-26-web-frontend-m6.5c2-spec.md` at **revision 2.8** (`c5c29eaf` + `09095ebb`): §10.9 —
R-6.5c2.28 (the bar), .29 (two phases by area), .30 (the refactors under budget; the R2 ladder decided), .31 (CI),
.35 (the roster), .36 (the tiers) — and §4's rows D and E as amended. The straw poll
`docs/plans/2026-09-28-web-lane-straw-poll.md` numbers the rows; the triage page holds the owner's decisions; the
dossier is the file:line authority. **How this plan was built:** by the orchestrator alone, from the dossier, with a
stop rule — a task past ~250 lines is two tasks or E's, a plan past ~2,500 lines is split before it is finished.

---

## Global Constraints

- **Branch `web-room-m6.5c2d`**, stacked on `web-room-m6.5c2c`; **the base for every code diff is `014433df`** (cycle
  C's branch head; its code is the reviewed head `ea4f174d`). The branch's first commits are the spec's revision 2.8,
  the two records and this plan. Every `file:line` in the dossier was read at `09095ebb`, the same tree: **find by
  content once an earlier task has moved a file.**
  - Stage with explicit paths, never `git add -A`. `git status` is clean after every commit.
  - **Nothing is pushed and nothing touches the account until Task 15.** The push and PR #42 are pre-authorized (spec
    §4, standing across the cycles). **The deploys and the live tiers are NOT**: Task 15.E asks the owner first
    (R-6.5c2.20's "on your word", each step separately).
- **Fences** (spec §5). At every commit this prints nothing:
  `git diff --stat 014433df -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli`.
  `SEMANTICS_REV` stays **7**. The CLI is not touched by any task (Task 4's `bga-human` fix reads the CLI's constant
  and changes nothing there). No bot behaviour moves.
- **The corpus moves in NAMED tasks only:** Task 6 (the roster: every payload with a seat, a creator or an author), Task
  7 (the chat's `deleted` field, the `chat_rev` counters, the two delete rows and their fixtures), Task 10 (two added
  fixtures), Task 8 (nothing: the events rows carry no query). Each regenerates with
  `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, READS the diff, and states in its report
  which files moved and why. Any other task that believes it needs a corpus change STOPS: a finding for the
  orchestrator.
- **Per-edit gates after EVERY task. Re-count at your own HEAD; the readings at `ea4f174d` (the C close §2) are the
  baseline every delta is stated against:** web 343 (lib 324 + pin 4 + gate 2 + `wire_fixtures` 13), wasm 7, worker 75,
  identity 8; `npm test` 1220 in 64, `test:wasm` 114 in 9, e2e 68 in 13; `worker/` `test:scripts` 387 in 19, the pool
  217 in 11 (two projects).
  - **Rust:** `cargo test -p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`; `cargo clippy --workspace
    --all-targets -- -D warnings`; `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`;
    `cargo test -p yokohama-web --no-default-features --features server --lib identity::` (8); `cargo build --profile
    release-wasm --target wasm32-unknown-unknown -p yokohama-worker`; `cargo doc -p yokohama-worker --no-deps` (0
    warnings) **and, from Task 3 on, `cargo doc -p yokohama-web --no-deps` (0 warnings) — R-D10.**
  - **`worker/`:** **`npm run build:module:debug` FIRST** whenever `crates/yokohama-worker` or the layer changed (Tasks
    4–8, 10), or the pool tests a stale module GREEN; then `npm run typecheck`, `npm run test:scripts`, `npm test`
    (BOTH projects).
  - **`web/`:** `npm run typecheck`, `npm test`, `npm run test:wasm` (after `npm run build:wasm` when the layer changed:
    Tasks 4–8, 10).
  - **`cd web && npm run e2e`** is owed by every task that changes what a browser DOES or SHOWS — **Tasks 6, 7, 8, 11,
    12, 13, 14** — SEQUENCED, never two Playwright runs at once, 6–8 min; run it **in the background with its output to
    a file in the ledger and POLL that file** (`tail -3` every minute or two) — **never a Monitor, never an event wait,
    never a re-arm.** It leaves the DEBUG module in `web/src/local/wasm/` and `web/dist/`; Task 15 rebuilds before any
    deploy. `plays_locally`'s quota cell is red on some seeds until Task 14 fixes it: before Task 14, re-run it alone,
    then believe the suite; after Task 14 a red is a finding.
  - **A red whole-game gate** (`cargo test -p yokohama-web --test play_a_whole_game`) is a FINDING, not a re-run; it
    reads 2 passed in ~25 s. **Check `df -h` first: ≥ 20 GiB free** before every e2e.
- **The strings rule holds** (the shell never `JSON.parse`s a record or a table document); **the two time bases hold.**
- **Test data never uses the owner's surname or first name** (R-6.5c2.16): `Ben Castle`, `Ada Bell`, or an address
  under `wire.test` / `e2e.test` / `t.test` / `readme.test`. **The owner's address goes into no document**; a
  deployment reading is recorded with "the owner's address" in the value's place.
- **Every INVISIBLE non-ASCII code point in code is a `\u` escape**; a visible glyph may be raw.
- **The ledger** is `.superpowers/sdd/2026-09-28-web-frontend-m6.5c2d-plan/` (gitignored). `progress.md` holds the SDD
  identity line first, one line per task and one `Ruling:` line per deviation from this plan. **Every agent writes its
  full report to a file there and replies with the PATH and three lines**; a reviewer's report is ordered Verdict →
  Issues → Assessment, each finding compact (`file:line`, defect, why, fix). **No agent spawns agents of its own**:
  an implementer or a reviewer that wants a helper asks the orchestrator.
- **Commit messages** quote the task's red runs in the body and end with the EXECUTING agent's own attribution lines.
- **A reviewer's mutants run on an EXPORT** (`git archive HEAD | tar -x -C <dir>`; ONE shared `CARGO_TARGET_DIR` under
  the session's scratchpad for every Rust export), never in the tree an implementer holds. While an implementer works,
  nothing runs in its tree and no timer is armed.
- **Entry for the executing session.** Read, in order: `CLAUDE.md`; `docs/handover.md`'s "Updated" line; the spec's
  §10.9 and §4; the D handover; the dossier's Summary; this plan WHOLE; the ledger's `progress.md`. Confirm `git
  branch --show-current` prints `web-room-m6.5c2d`, `git status --short` prints nothing, and `git log --oneline -1` is
  at or after this plan's commit. Then Task 1 Step 0, the baseline.

---

## Review Focus

The spec implies these five inputs, and no cell would meet them unless someone put it there. Each line says what a
person would expect; its test lives in the owning task and is marked **(Review Focus N)** there.

1. **A stranger reading a locked table, an open table, the "being played" list or the house chat.** Expected: not one
   address that is not the reader's own, in any payload, on either host — the seats read by name or "invited", the
   creator by name, an author by name or "you". (Task 6: the reader-aware leakage pin over every walked payload.)
2. **A deleted message, read by a client that fetched it before the delete.** Expected: the client learns of the
   deletion at the next frame without a reload, the message's number is never reused, and the deleter is its author
   or the table's creator and nobody else. (Task 7: the `chat_rev` cell and the three-caller refusal cell.)
3. **A request landing while R2 is down and a save is owed.** Expected: the request answers at its usual speed, the
   upload is attempted once per rung and not once per request, and the alarm's retry still climbs the ladder.
   (Task 8: the outage-burst pool cell counting puts.)
4. **The non-acting player posting a move at every turn of a whole game.** Expected: 403 naming whose turn it is, by
   seat, never the address, and the game unchanged. (Task 10: the gate's non-acting probe per turn.)
5. **Another tab holding the old IndexedDB version when "Back up everything" is pressed.** Expected: a sentence naming
   the other tab within seconds, and the button usable again; never a spinner for ever. (Task 12: the blocked-upgrade
   cell with a bound.)

---

## Execution shape (the owner's standing answers, and this plan's rulings)

- **Mode: SUBAGENT-DRIVEN, EVERY TASK.** Implementers: **Sonnet 5.5** for Tasks 1, 2, 3, 10, 14; **Opus** for Tasks
  4–9, 11–13. **Every task reviewer Opus.** Fable: the whole-branch review in Task 15. A Sonnet brief carries the
  judgment already made: what to change, what proves it, what not to touch; a Sonnet implementer that meets a design
  choice STOPS and asks.
- Tests are written first and RUN RED; the red run is quoted in the commit body and the ledger. Every commit passes
  every per-edit gate.
- Standing process rules: never follow up an implementer's DONE without reading HEAD first; a reviewed SHA is never
  amended — a fix is a NEW commit and the acceptance reply says "as a new commit"; while an implementer works, run
  nothing in its tree and arm no timers; wake a stalled agent by message; check the ledger within ~15 minutes of any
  long run; a reviewer's mutants run on an export.
- **The orchestrator edits no code.** Its steps: Task 1 Step 0's baseline; the triage page's decisions handed to Task 2
  as a file; the polish-free reading of Task 11's pictures; Task 15 whole.
- **Rulings at planning (2026-09-28, the orchestrator, under spec §10.9):**
  - **R-D1:** the spec's fourteen tasks are FIFTEEN: its Task 12 "the local host and backup" is two — 12 backup and
    restore, 13 the local host's actor — one reviewer's gate per concern.
  - **R-D2:** the base for every diff is `014433df`; the corpus is regenerated only by Tasks 6, 7 and 10.
  - **R-D3 — the roster's scope** (R-6.5c2.35, widened where the dossier found the pin would fail): a human seat crosses
    as `{kind: "human", name, you, invited}`; `created_by` becomes `mine: bool` + `creator: string | null`; chat
    authors cross as `{kind: "seat", seat, name, you}` / `{kind: "watcher", name, you}`; the two refusal sentences
    name a seat and "the table's creator"; `invited` (the address the creator typed) is served ONLY in a payload built
    for the creator and ONLY while the seat has no name. The stored documents keep `user` and `created_by`.
  - **R-D4 — chat delete's shape:** `DELETE …/chat/{seq}` by the author or the table's creator; a deleted message stays
    as a tombstone (`text: ""`, `deleted: true`) so `last` and every cursor hold; `Events` gains `chat_rev` and the
    house frame `rev`, +1 per append and per delete; the live runs delete their own posts; no operator route.
  - **R-D5 — the held arm is RETIRED** (row 86's option a): the NAS answers at once, `wait` and `after` stay parsed and
    ignored for an older release's probe, no host ever reports `"held"`, and the client's held arm and unions go.
  - **R-D6 — the R2 backoff** is a not-before gate on the REQUEST path only; game over's first attempt and the alarm
    path are unchanged (R-6.5c2.30's "a short backoff after a failure").
  - **R-D7 — the refactor budget:** ONE day of wall in Task 9, the order `table.ts` split → the four script dedups →
    the replay split; **methods stay on `Table`'s prototype** (mixins, not free functions), because the pool patches
    prototypes; whatever the budget leaves is a `Ruling:` line and a by-design row.
  - **R-D8 — a poisoned per-game `Mutex<Repl>` evicts the handle** through ONE helper; a poisoned table `Mutex<()>`
    is entered (`PoisonError::into_inner`).
  - **R-D9 — the livelock run** is 500 seeds in the RELEASE profile, in the background under `caffeinate -i`, its
    log polled; a failed seed is a finding, not a retry.
  - **R-D10 — `cargo doc -p yokohama-web --no-deps` joins the per-edit set** at 0 warnings from Task 3 on.
  - **R-D11 — the deploys are asked for at Task 15.E**, the push and PR #42 are not.

---

## Names shared across tasks

An implementer sees only its own task. **These names are the contract between tasks and are used verbatim.** A task
that finds the code forces a different shape STOPS and asks the ORCHESTRATOR, who amends this table with a `Ruling:`
line.

### What the tree holds at the base, consumed as it IS (read off the tree, never off this table)

| name | shape | tasks |
|---|---|---|
| `yokohama_web::table::{TableDoc, TableView, SeatDoc}`; `TableDoc::view(&self, user) -> TableView`; `TableDoc::seats_of(user) -> Vec<PlayerId>` (lowest first); `TableView { id, name, created_by, created_at, players, colors, start_seat, seats: Vec<SeatDoc>, chosen, my_seats, status, to_move, over, locked, winner, can_watch, bot }` | dossier Task 6 | 6, 11 |
| `yokohama_web::chat::{Message { seq, created_at, author, text }, Author::{Seat{seat,user,name}, Watcher{user,name}, Bot{seat,name}}, ChatDoc, ChatLimits}`; `api::author_of(table, viewer, user, name)`; `Games::{append_chat, house_lock, bump_table, bump_house_chat}` (dossier rows 82, 130) | as at the base | 6, 7 |
| `yokohama_web::api::{Events { events, to_move, status, your_turn, chat }, EventsQuery { after, wait }, MAX_WAIT_SECONDS, check_wait, events_at, routes, matches, undo_allowed, holder_of}`; `signal::house_frame(seq, chat)`; the NAS handler `lib.rs`'s `events` (dossier row 86) | as at the base | 4, 7, 8, 10 |
| `worker/src/table.ts`'s `class Table` and its method map (dossier row 127); `worker/src/slot.ts`'s `nextRetry`; `worker/src/r2.ts`'s `ArchiveOwed`, `executePlan`; `storage.ts`'s `getMeta`/`setMeta` | as at the base | 8, 9 |
| the pool's harness `worker/test/{helpers,gates,walk}.ts`; the corpus replays `worker/test/corpus.test.ts` (the pool), `web/src/local/corpus.node.test.ts` (the browser host), `worker/scripts/live-replay.mjs` (live) and their exact-set lists (dossier rows 20, 32) | as at the base | 6, 7, 9, 10, 14 |
| `web/src/types.ts` (`SeatDoc`, `TableView`, `ChatAuthor`, `ChatMessage`, `Events`); `web/src/wire/contract.ts`'s key sets `TABLE`, `CHAT_MESSAGE`, `CHAT_AUTHOR_SEAT`, `CHAT_AUTHOR_WATCHER`; `web/src/tables.ts`'s `seatName`, `holder`, `seatChips`; `web/src/signal.ts`'s arms (dossier rows 86, 42) | as at the base | 6, 7, 8, 11, 14 |
| `web/src/backup.ts`, `web/src/local/{worker,client,db,useLocalHold}.ts`, `web/src/{Abandon,OnThisDevice,BackupControls}.tsx` (dossier Tasks 12, 13) | as at the base | 12, 13 |
| `.github/workflows/{test,docker}.yml` as cycle C left them | the three web jobs' steps and comments, moved verbatim | 1 |

### What cycle D produces (the contract between its own tasks)

| name | shape | task |
|---|---|---|
| `.github/workflows/web.yml` (jobs `web-server`, `worker`, `web-client`; `on: workflow_dispatch` only); `test.yml` (job `test`; `on: pull_request` with `paths`, `workflow_dispatch`; `concurrency` with `cancel-in-progress`); `docker.yml` (`on: workflow_dispatch` only) | CI | 1 |
| `docs/plans/2026-09-28-web-lane-straw-poll.md` with the column `owner's disposition` and the column `phase` | the record | 2 |
| the per-edit line `cargo doc -p yokohama-web --no-deps` (0 warnings) | R-D10 | 3 |
| `api::UndoRefusal` (refused → 403) vs the replay's `GameError::Server` (500); `tables.rs`'s `seat_of_waiting(doc, user) -> PlayerId`; `bots::recorded` skipping `yokohama_cli::bga::CONTROLLER_BGA_HUMAN`; `client::without_version_drift` also dropping `warning: seat N records ` lines for a `bots::foreign` seat | the layer's defects | 4 |
| `games::lock_game(&Mutex<Repl>) -> Result<MutexGuard<Repl>, GameError>` (R-D8: poison → eviction + `GameError::Server("the game's handle was poisoned — reload")`); `api::admitted_with(table: &TableDoc, …)`; `archive::DirArchive`'s atomic marker write | the NAS server | 5 |
| `table::SeatView { kind, name: Option<String>, you: bool, invited: Option<String> }` (serde-tagged like `SeatDoc`, arms `human`/`bot`/`open`); `TableView { …, mine: bool, creator: Option<String>, seats: Vec<SeatView>, … }` (no `created_by`); `TableDoc::view(&self, user)` builds them; `chat::AuthorView::{Seat{seat,name,you}, Watcher{name,you}, Bot{seat,name}}` and `ChatDoc::page_for(reader) -> Vec<MessageView>`; the sentences `"it is seat {n}'s turn ({name-or-colour})"` and `"only the table's creator can delete this table"`; `web/src/types.ts`'s `SeatView`, `TableView.mine/creator`, `ChatAuthor` without `user`; `contract.ts`'s `SEAT_HUMAN`, `TABLE` (with `mine`, `creator`), the author key sets without `user`; `web/src/tables.ts`'s `seatName(seat)` → `seat.name ?? (seat.you ? "you" : seat.invited ?? "invited")` | the roster | 6 |
| the routes `DELETE /api/games/{id}/chat/{seq}` and `DELETE /api/chat/{seq}`; `chat::Message.deleted: bool` (`#[serde(default)]`), `ChatDoc::delete(seq, by: &User, creator: Option<&str>) -> Result<(), ChatRefusal>`, `ChatDoc.rev: u64`; `Events.chat_rev: u64`; `signal::house_frame(seq, chat, rev)`; `web/src/types.ts` `ChatMessage.deleted`, `Events.chat_rev`; `api.deleteChat(game, seq)` / `api.deleteHouseChat(seq)`; the fixtures `chat_delete.json`, `chat_delete_403.json`, `house_chat_delete.json`; the walk rows after each post; `live-replay.mjs`'s delete row; `api::ARCHIVE_BODY_LIMIT: usize = 1 << 20` | chat delete, the cap | 7 |
| `ArchiveOwed.not_before?: number` (ms); `Table.deliverArchive({ fromRequest })`; the NAS `events` handler without the hold; `Events`/`EventsQuery` unchanged in shape; `web/src/signal.ts` without `HELD_REASK_MS`/`HELD_WAIT_S`/the held arm; `Link`/`SignalKind` unions without `"held"` | the Worker's two, the held arm | 8 |
| `worker/src/table/{delivery,archiving,lifecycle,sockets}.ts` as mixins over `class Table`'s base; `worker/scripts/dev-vars.mjs` (+ `.d.mts`) or the twin-pin cell; `worker/scripts/replay/{credential,scripts,exemptions,send,judge}.mjs` if the budget reaches it | the refactors | 9 |
| the gate's `probe_non_acting(api, other, …)` per turn; `FailingWrites`' `Conflict` arm; the fixtures `deal_bot_table.json`, `table_two_open.json`; `session_routes()` with `/stream`, `/unlock`, `/record` | the layer's tests | 10 |
| `web/src/GameMenu.tsx`'s `<details>` closed on a switch; `Lobby.tsx`'s per-section `allSettled`; `Dealing.tsx` refetching on a `dealing` frame whose `events`/`chat` moved; `App.tsx`'s `open` restoring the previous game on a refusal and checking the route after each await; `OpenTable` taking the table read once | the client's defects | 11 |
| `backup.ts`'s `openDbBounded(ms)`; `backupEverything` over the union of the store's ids and the module's listing; `backup.rs`'s `envelope_version_of(text) -> Result<u32, String>` read BEFORE the whole parse; `db.ts`'s compat row re-derived against `semantics_rev`; `Abandon.tsx`'s "Abandon without saving" after a refused send | backup and restore | 12 |
| `local/worker.ts`'s `hold`/`ready`/`adopt` answering `{ kind: "unreadable", sentence }` instead of a trap; `web/e2e/caps.ts` exporting `CAP` and `REV` (read from `/api/health`) | the local host | 13 |
| `plays_locally.spec.ts`'s `actUntilUnsaved(page)`; `corpus.node.test.ts`'s present-and-differs rule; the e2e height sweep `heights_at_390(page, selectors)` in `setup_screens.spec.ts`; `test:wasm`'s release-module replay cell | the client's tests | 14 |

---

## File structure

**Created:** `.github/workflows/web.yml` (1); `worker/src/table/{delivery,archiving,lifecycle,sockets}.ts`,
`worker/scripts/dev-vars.mjs`, `worker/scripts/dev-vars.d.mts` (9, budget permitting); `web/src/wire/chat_delete.json`,
`chat_delete_403.json`, `house_chat_delete.json` (7); `web/src/wire/deal_bot_table.json`, `table_two_open.json` (10);
`web/e2e/caps.ts` (13); `docs/plans/<the day>-web-frontend-m6.5c2d-close.md`, `…-m6.5c2e-handover.md` (15); in the
ledger: the reports, `task-<n>-e2e.txt`, `task-10-gate-500.txt`, `task-11-look/`, `close-sequence.sh`, `sequence.tsv`.

**Modified** (principal files; each task's own list is the authority): `.github/workflows/{test,docker}.yml` (1);
`docs/plans/2026-09-28-web-lane-straw-poll.md` (2); `Cargo.toml`, `CLAUDE.md`, `deploy/README.md`, `worker/README.md`,
`worker/wrangler.jsonc`, `deploy/Dockerfile`, `worker/scripts/{read-deploy,module-figures}.mjs`, `web/e2e/exit_game.spec.ts`,
and about fifteen files' comments (3); `crates/yokohama-web/src/{api,tables,bots,client,grammar}.rs` (4);
`crates/yokohama-web/src/{tables,games,api,lib,chat,store,archive}.rs` (5); `crates/yokohama-web/src/{table,tables,api,chat,client}.rs`,
`tests/wire_fixtures.rs`, `web/src/{types,tables}.ts`, `web/src/wire/contract.ts`, `web/src/{Dealing,OpenTable,Lobby,App,Chat,TableFacts}.tsx`
and their tests, the corpus (6); `crates/yokohama-web/src/{api,chat,tables,signal,backup}.rs`, `tests/wire_fixtures.rs`,
`crates/yokohama-worker/src/{lib,events}.rs`, `worker/src/{table,house,frames}.ts`, `worker/scripts/live-replay.mjs`,
`web/src/{api,types,Chat}.tsx|ts`, `web/e2e/exit_game.spec.ts`, the corpus (7); `worker/src/{table,r2}.ts`,
`crates/yokohama-web/src/{api,lib}.rs`, `web/src/{signal,api,types}.ts`, `web/src/local/facts.ts` and their tests (8);
`worker/src/table.ts`, `worker/scripts/{live-replay,assemble-assets,read-deploy}.mjs`, `web/e2e/live.ts` (9);
`crates/yokohama-web/tests/play_a_whole_game.rs`, `src/{games,tables,lib}.rs` tests, `tests/wire_fixtures.rs`,
`web/src/wire/contract.ts` (10); `web/src/{Dealing,GameMenu,Lobby,App,OpenTable,Chat}.tsx`, `whoami.ts`, `types.ts`,
`styles.css`, `web/e2e/the_table.spec.ts` (11); `web/src/backup.ts`, `local/db.ts`, `Abandon.tsx`, `BackupControls.tsx`,
`crates/yokohama-web/src/backup.rs` (12); `web/src/local/{worker,client,useLocalHold}.ts`, `web/src/{App,api}.ts|tsx`,
`styles.css`, `crates/yokohama-wasm/src/lib.rs`, `web/e2e/*.spec.ts` (the `CAP` copies) (13); `web/e2e/{plays_locally,
setup_screens,offline,the_table}.spec.ts`, `web/src/local/corpus.node.test.ts`, `web/src/sw.node.test.ts`,
`web/src/Abandon.test.tsx`, `web/src/backup.test.ts`, `web/src/signal.test.ts` (14).

---

## Task order and why

```
1 ─► 2 ─► 3 ─► 4 ─► 5 ─► 6 ─► 7 ─► 8 ─► 9 ─► 10 ─► 11 ─► 12 ─► 13 ─► 14 ─► 15 (the close, ORCHESTRATOR)
```

1. **1 first:** so every later push of this branch runs no job; it also carries the cycle's baseline (Step 0).
2. **2 and 3 next:** no behaviour; the record and the small debts land before any file they cite moves.
3. **4 before 5:** both edit `api.rs` and `tables.rs`; 4's helpers (`seat_of_waiting`, the typed undo error) are what
   5's cells call.
4. **5 before 6:** 5's `lock_game` helper is what 6's new page builder and 7's delete route lock through.
5. **6 before 7:** 7 adds `AuthorView`-bearing payloads (the delete rows) to a corpus 6 has already reshaped, so the
   corpus is regenerated twice, each diff read once.
6. **7 before 8:** 8's held-arm retirement touches `signal.ts`, which 7's `chat_rev` re-read also touches; 7 first,
   8 finds by content.
7. **8 before 9:** 9 splits `table.ts` after 8's `deliverArchive` change lands in it.
8. **9 before 10:** 10's fixtures and the exact-set lists are replayed by the pool 9 leaves.
9. **10 before 11–14:** the layer is quiet; the client's tasks read a settled wire.
10. **11 → 12 → 13 → 14:** 11 edits the screens 14 walks; 12 and 13 share `backup.ts`/`local/*` — 12 the backup
    envelope and restore, 13 the actor's replies; 14 last.
11. **15 last.**

---

## Couplings between tasks (binding; the later task's implementer does what its item says)

- **Task 1 → Task 15:** the workflows are disabled on GitHub from 2026-09-28 through E's merge (R-6.5c2.31 as amended:
  the owner's minutes); this cycle's pull request runs NO job and nothing is dispatched; Task 15.H records that. The
  clean-checkout proof of the web lane is E's, after the re-enable in October.
- **Task 3 → Task 6:** the four `CLAUDE.md` sentences fixed in place; Task 6 changes no `CLAUDE.md` line (the web
  passages move to `docs/web-lane.md` in E).
- **Task 4 → Task 6:** `holder_of` (dossier row 6's neighbour) feeds the "it is seat N's turn" sentence; Task 4 leaves
  it, Task 6 rewrites the sentence to name the seat.
- **Task 5 → Tasks 6, 7:** every new lock take in 6 and 7 goes through `lock_game` (R-D8), never a bare `expect`.
- **Task 6 → Task 7:** 7's `AuthorView` is 6's; 7 adds `deleted` beside it and never re-adds `user`.
- **Task 6 → Task 11:** `mine`/`creator` replace every client `created_by === me`; row 17's seatless-creator Abandon
  keys on `mine`. Task 11 finds by content.
- **Task 7 → Task 8:** `signal.ts`'s house-frame parse gains `rev` in 7; 8 removes the held arm around it.
- **Task 7 → Task 14:** the two chat delete rows are in the walk; 14's corpus-replay cells' exact sets include the
  three new fixtures (10's two as well).
- **Task 8 → Task 9:** 9 moves `deliverArchive` into `archiving.ts` AFTER 8's gate is in it; find by content.
- **Task 8 → Task 14:** row 42's "held arm `connected`" item is MOOT once 8 lands; 14 skips it and says so.
- **Task 10 → Task 14:** `session_routes()` gains three routes; 14's structural cells read the list as 10 leaves it.
- **Task 12 → Task 13:** 12 owns `backup.ts` and `db.ts`; 13 owns `local/worker.ts`, `client.ts`, `useLocalHold.ts`;
  `Abandon.tsx` is 12's, `App.tsx` 13's.
- **Task 13 → Task 14:** `web/e2e/caps.ts`'s `CAP`/`REV` are what 14's specs import; 14 adds no copy.
- **Every task → Task 15:** each report names its catchers by cell name; Task 15's battery rows cite them.

---

### Task 1: The CI reshape — Sonnet 5.5 (R-6.5c2.31; straw poll rows 133, 134; dossier Task 3's `test.yml` lines)

**Files:**
- Modify: `.github/workflows/test.yml` (the core job alone), `.github/workflows/docker.yml` (dispatch only).
- Create: `.github/workflows/web.yml` (the three web jobs, moved verbatim).
- Test: `$L/task-1-ci-shape.py` (the ledger; a structure check, never committed).

**Interfaces:**
- Consumes: the three workflows as cycle C left them (the `web-server`, `worker` and `web-client` jobs with their
  comments, `test.yml:52-431`; `docker.yml`'s `image` job).
- Produces: the shape in §"Names" — `test.yml` on `pull_request` (the engine's input paths) + `workflow_dispatch`; `web.yml` on
  `workflow_dispatch` only; `docker.yml` on `workflow_dispatch` only; `concurrency` with `cancel-in-progress: true` on
  all three. The files are the shape CI takes when E re-enables it; nothing runs before then.

- [ ] **Step 0 (ORCHESTRATOR): the baseline.** `progress.md`'s first lines: the SDD identity line, the base `014433df`,
  and the C close §2's readings at `ea4f174d` as the baseline (the tree is byte-identical: `git diff --stat ea4f174d
  -- . ':!docs' ':!.github'` prints nothing — record that). No suite is re-run for the baseline.

- [ ] **Step 1: The structure check, red.** Write `$L/task-1-ci-shape.py`: with PyYAML (`python3 -c "import yaml"`;
  if absent, `pip3 install --user pyyaml` and say so in the report), load the three files and assert — `test.yml`:
  `on` has exactly the keys `pull_request` and `workflow_dispatch`, `pull_request.paths` equals `["crates/yokohama-core/**",
  "crates/gen-data/**", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".github/workflows/test.yml"]` (the inputs of
  the suite the job runs, so a web-only pull request runs nothing), its only job is `test`, and
  `concurrency["cancel-in-progress"] is True`; `web.yml`: `on` is exactly `workflow_dispatch`, its jobs are exactly
  `web-server`, `worker`, `web-client`, and each job's `steps` list DEEP-EQUALS the same job's `steps` in `test.yml`
  at `014433df` (read with `git show 014433df:.github/workflows/test.yml`), so the move is verbatim; `docker.yml`:
  `on` is exactly `workflow_dispatch`, `concurrency["cancel-in-progress"] is True`. Run it: red (the files are C's).

- [ ] **Step 2: `test.yml`.** Keep the `test` job, with its toolchain step changed from `dtolnay/rust-toolchain@stable`
  to `dtolnay/rust-toolchain@master` with `toolchain: 1.97.1` (row 134: the file pins the version the job then
  prints; a one-line comment says why). `on:` becomes `pull_request` with the six paths above plus
  `workflow_dispatch:`; add `permissions: contents: read` and `concurrency: { group: test-${{ github.event_name }}-${{
  github.ref }}, cancel-in-progress: true }`. **Rewrite the header comment** (it is the file's contract): what runs
  here (the engine crate's suite, on a pull request that changes a Rust input, and by hand); what does NOT and why —
  the web jobs are `web.yml`'s and run on dispatch alone (R-6.5c2.31, the owner's words quoted: *"what would the point
  be of having web even on merge into master - if we're running those full suites locally? The key to ensure
  consistency are the core crates"*); no `push` trigger, because in this repository a master push is a PR merge whose
  merge ref the PR run already tested, and push-plus-pull_request ran every job twice; the one coupling stated (the
  lint gate compiles the layer with the workspace, the corpus pins the engine's wire-visible words, both run locally by
  the per-edit rule); and the list of what stays local as C wrote it. Delete the three web jobs and their comments
  from this file.

- [ ] **Step 3: `web.yml`.** `name: web`; `on: workflow_dispatch:`; the same `permissions` and a `concurrency` group
  `web-${{ github.ref }}`; the three jobs pasted VERBATIM from `014433df`'s `test.yml`, comments included. A header
  comment: when to dispatch it — before a web deploy or a release, after a change under `crates/yokohama-web/**`,
  `crates/yokohama-wasm/**`, `crates/yokohama-worker/**`, `web/**` or `worker/**` as a clean-checkout proof, and
  whenever a core change moved what the layer compiles; `gh workflow run web.yml --ref <branch>`; the per-edit suites
  already run locally after every task, so this is a proof and never a gate a merge waits for.

- [ ] **Step 4: `docker.yml`.** `on:` becomes `workflow_dispatch:` alone (the two `paths` lists go). The header's
  trigger paragraph is rewritten in three sentences: dispatch only, by R-6.5c2.31; the cold/warm paragraph stays true
  and stays; R-C9's fifteen-minute rule is retired with the trigger (say so).

- [ ] **Step 5: Green.** `python3 $L/task-1-ci-shape.py` passes. `git diff --stat` lists exactly the three files.

- [ ] **Step 6: Note the GitHub side.** `test` and `docker` were DISABLED on GitHub on 2026-09-28 (`gh workflow
  disable`, the owner's minutes at 90% for the month) and stay so through E's merge (R-6.5c2.31 as amended); `web.yml`
  arrives dispatch-only and nobody dispatches it in D. The three headers say this in one sentence each. **No task
  and no close step of this cycle runs anything on GitHub.**

- [ ] **Step 7: Commit.**

```bash
git add .github/workflows/test.yml .github/workflows/web.yml .github/workflows/docker.yml
git commit -m "ci: the core job alone on pull requests that touch a Rust input; the three web jobs in web.yml and the image on dispatch only; cancel-in-progress everywhere (R-6.5c2.31)"
```

---

### Task 2: The straw-poll record — Sonnet 5.5 (R-6.5c2.28; spec §10.9 "the triage")

**Files:**
- Modify: `docs/plans/2026-09-28-web-lane-straw-poll.md`.
- Input (the ledger): `$L/triage-decisions.json` — written by the ORCHESTRATOR before dispatch from the triage page's
  store (one entry per row: `{n, decision, note}`; a row absent from the store takes the page's pre-filled call, which
  the file carries under `calls`), and `$L/phases.json` (row → `D`/`E`/`F`, the page's tags).
- Test: `$L/task-2-check.py` (every row 1–152 has a disposition; every `fix` row a phase; the tally line matches).

- [ ] **Step 1: The check, red.** The script parses the record's tables and fails: no `owner's disposition` column.

- [ ] **Step 2: The columns.** A Python script in the ledger rewrites each of the eight tables: after `rec` insert
  `owner's disposition` (the words `fix` / `public door` / `by design` / `not observable` / `the AI lane's` / `the
  owner's`) and `phase` (`D`, `E`, `F`, or blank). The tally line becomes the new counts. A section "**The premises
  the dossier corrected**" lists, from `docs/plans/2026-09-28-web-frontend-m6.5c2d-dossier.md`'s Summary, the rows
  false at HEAD (11, 24, 35) and the partly-false list, one line each with the dossier's reason. A section "**Notes
  from the triage**" lists every non-empty note, by row. The preamble's last sentence says the transcription is done
  and names this task.

- [ ] **Step 3: Green; read the diff** (`git diff --stat` lists one file; every table still has 152 rows in order).

- [ ] **Step 4: Commit.**

```bash
git add docs/plans/2026-09-28-web-lane-straw-poll.md
git commit -m "docs(plans): the straw poll carries the owner's disposition and the phase for every row, and the premises the dossier corrected"
```

---

### Task 3: The small debts — Sonnet 5.5 (straw poll rows 47, 48, 49, 50, 51, 9; the CI bullet; R-D10; dossier Task 3)

**Files:**
- Modify (row 47, the citations — `worker/RUNBOOK.md §N` for the old `deploy/RUNBOOK.md §6.N`, per the dossier's map):
  `.github/workflows/web.yml` (the two lines Task 1 moved: `deploy/RUNBOOK.md §6.7` → `worker/RUNBOOK.md §7`, `RUNBOOK
  §6.2` → `worker/RUNBOOK.md §2`), `deploy/Dockerfile:12`, `worker/wrangler.jsonc:6-7,61,86,91`,
  `worker/scripts/read-deploy.mjs:9`, `worker/scripts/read-deploy.test.mjs:53`, `web/e2e/exit_game.spec.ts:48,73`,
  `worker/README.md:1644`. The board's three are E's rewrite; the dated plans are frozen.
- Modify (row 48): `Cargo.toml:30-58` (both module paragraphs); `web/scripts/wasm-opt.mjs:5-7`, `web/src/precache.ts:122,144`,
  `deploy/RUNBOOK.md:127` (each becomes a CITATION of the paragraph, never a copy).
- Modify (row 51 + the CI bullet): `CLAUDE.md` lines 36, 44, 47, 118, 121; `deploy/README.md:211-212`.
- Modify (row 50): doc comments in `crates/yokohama-web/src/*.rs` (the real list comes from `cargo doc`).
- Modify (rows 49 and 9): comments only, in the files the dossier locates.

- [ ] **Step 1: Row 47, one commit.** Replace each citation as the dossier's map gives it. Then:

```bash
grep -rn -E 'RUNBOOK(\.md)?`? §6' --include='*.yml' --include='*.mjs' --include='*.ts' --include='*.jsonc' --include='Dockerfile*' . | grep -v 'deploy/RUNBOOK.md\|worker/RUNBOOK.md'
```

  prints nothing (the map's own lines in the two RUNBOOKs are the only survivors). Commit: `docs: the eleven RUNBOOK
  §6.x citations in code and configuration, and worker/README.md's one, name worker/RUNBOOK.md §N (from twelve at the
  D handover, eleven located in code at HEAD)`.

- [ ] **Step 2: Row 48.** In `Cargo.toml`'s browser paragraph: the release module reads **1,197,069 B** under
  `wasm-opt` 133 at `ea4f174d` (from 1,814,905 B before `wasm-opt`), "the C close §3.1 reading; gzip not re-read";
  the Worker paragraph: **1,258,106 B**, gzip 491,934 B, 18 pages, 170 data segments, 123,047 B of data, the section
  124,446 B, at `ea4f174d`. Both paragraphs keep the sentence that a figure is a reading. The three copies become
  "the figure is the root `Cargo.toml`'s `release-wasm` comment" with no number. Task 15.G re-reads both at the
  deploy build and updates the paragraph in the close's documents commit.

- [ ] **Step 3: Row 50.** Run `cargo doc -p yokohama-web --no-deps 2>&1 | tee $L/task-3-rustdoc-before.txt`; count
  the warnings. Fix each: a link to a private item becomes a plain code span (or the item's doc is moved to a public
  item it describes); a broken path gets the right path; `<name>` becomes `` `<name>` ``. Re-run: **0 warnings**;
  save the after-log. R-D10: the command joins the per-edit list in the ledger's `progress.md` header.

- [ ] **Step 4: Row 51 and the CI bullet, in `CLAUDE.md`.** Each is ONE clause in a long line; edit the clause, nothing
  else on the line: `:36` — the gate's vs-bot arm no longer livelocks: cycle A's placement probe and stuck-picker
  rule, `VS_BOT_CAP` 5,000 measured over 50 seeds (Task 10 raises it to 500), and the gate back in CI bounded — the
  CI sentence rewritten to Task 1's shape (`web.yml`, dispatch only); `:44` — a cap hit reports through `capReached`,
  never `deadEnd`; `:47` — the CI paragraph becomes: `test.yml` runs the core suite on a pull request that changes a
  Rust input and by hand; `web.yml` runs the three web jobs on dispatch only; `docker.yml` on dispatch only
  (R-6.5c2.31, two sentences); `:118` — a first visit reads "connecting…"; `:121` — the update offer renders on the
  door, and a waiting release activates by itself once every old page closes. `deploy/README.md:211-212` the same
  truth. `guide_sync` reads none of these files; still run `cargo test -p yokohama-cli guide_sync` once (18) as the
  cheap proof nothing else moved.

- [ ] **Step 5: Rows 49 and 9.** Every located minor in the dossier's row 49 list, comments only, each as the dossier
  words the fix; the four it could not locate are named in the report as "not located from tracked sources" and go
  to the close as by-design rows. Row 9: `receipt.rs:461-468` and `picker.rs:1033-1036` state the CLASS (a
  hand-made document whose value the layer's parse admits and the module's read refuses names the wrong reason; no
  layer write makes one) — R-C14's ruling, made a sentence.

- [ ] **Step 6: The gates.** `cargo doc -p yokohama-web --no-deps` 0; `cargo doc -p yokohama-worker --no-deps` 0;
  `cargo test -p yokohama-web --lib` and `-p yokohama-worker` (comments only, run anyway); clippy; `cd worker && npm run
  test:scripts` (the `module-figures` wording changed); `python3 $L/task-1-ci-shape.py` still green (Step 1 edited
  `web.yml`'s comments only — the DEEP-EQUAL is on `steps`; a comment is not a step).

- [ ] **Step 7: Commit** (the second of the task): `docs: the module figures cited from one place, rustdoc at zero
  warnings in the layer, four CLAUDE.md sentences and one README sentence made true, and C's deferred comment minors`.

---

### Task 4: The layer's defects — Opus (straw poll rows 5, 6, 7, 10, 143, 144; dossier Task 4)

**Files:**
- Modify: `crates/yokohama-web/src/api.rs` (`undo_allowed`, `matches`), `src/tables.rs` (the waiting caller's seat;
  the create probe), `src/bots.rs` (`recorded`), `src/client.rs` (the drift mask), `src/grammar.rs:110-111` (a doc
  line naming `api.rs`).

**Interfaces:**
- Consumes: `yokohama_core::state_at`, `GameError::{Server, BadRequest}`, `bots::foreign`, `client::without_version_drift`.
- Produces: `api::UndoRefusal` (typed: `Refused(String)` → 403, `Replay(String)` → 500 as `GameError::Server`);
  `tables::seat_of_waiting(doc: &TableDoc, user: &User) -> PlayerId`; `bots::recorded` skipping the CLI's `bga-human`
  controller; `without_version_drift` dropping `warning: seat {n} records ` lines for every seat `bots::foreign` names.

- [ ] **Step 1: The cells, red.**
  - `api.rs`: `undo_on_a_record_that_will_not_replay_is_a_500_not_a_403` — a hand-built record whose `rules_hash`
    is wrong (or whose events are truncated so `state_at` fails) at a two-person table; `undo` answers 500 with the
    replay's sentence; every existing undo cell unchanged.
  - `tables.rs`: `the_waiting_callers_set_table_is_drawn_for_their_own_seat` — BLUE (seat 1) has kept, RED has not;
    blue's `/deal` answers `frame.seat == 1`. Row 6.
  - `tables.rs`: `create_refuses_a_name_whose_record_is_unreadable` now asserts a **500** whose sentence names the
    store's error, not "already exists"; the refusal itself still stands (no 201, the record untouched).
  - `api.rs`: in `the_matcher_binds_segments_and_tells_a_wrong_method_from_no_route`, `/api/games//table` finds NO route.
  - `client.rs` or `games.rs`: `a_finished_same_name_re_point_shows_no_drift_warning` — the fixture pair re-pointed to
    `v4-step9-draft1` at another `version` (dossier row 143's shape): `ClientState.warnings == []`.
  - `bots.rs`: `a_bga_human_seat_is_a_person_not_a_foreign_bot` — a record with one seat's controller `bga-human`:
    `recorded` lists no bot for it, `foreign` is empty, and (`client.rs`) the CLI's "resumes as a HUMAN seat" line is
    masked for it.
  Red: six failures, quoted.

- [ ] **Step 2: The fixes.** `undo_allowed` returns `Result<bool, UndoRefusal>`; the caller maps the arms; if
  `GameEvent` names the acting seat, read the last event's seat instead of replaying (say which in the report; the
  replay stays if not). `seat_of_waiting` as in §"Names"; `first_human_seat` and its half-cell go if nothing else
  uses them. The create probe's `Err(e)` arm → `GameError::Server(format!("the store could not be read for {id:?}:
  {e}"))`. `matches` binds only a non-empty segment. `recorded` skips `"bga-human"` — through
  `yokohama_cli::bga::CONTROLLER_BGA_HUMAN` if the CLI exports it, else the literal with a comment citing
  `crates/yokohama-cli/src/bga/mod.rs` (the CLI is fenced: no edit there). `without_version_drift` gains the second
  mask; its doc records that R-C18's choice is reversed here and why (a finished re-point read "played against V4,
  which this server doesn't run" on a server running V4).

- [ ] **Step 3: Green; the corpus diff prints nothing** (`git diff --stat -- web/src/wire` empty; run `wire_fixtures`
  plain). `grammar.rs:110-111` names `api.rs`.

- [ ] **Step 4: The gates.** The Rust set; `cd worker && npm run build:module:debug && npm run typecheck && npm test`;
  `cd web && npm run build:wasm && npm run test:wasm`. No e2e (nothing a browser shows changed).

- [ ] **Step 5: Commit.** `fix(web): a replay failure under undo is a 500, the waiting caller's set table is their own
  seat, an unreadable store refuses by name, an empty path segment finds no route, a finished same-name re-point
  shows no drift line, and a bga-human seat is a person`.

---

### Task 5: The NAS server's defects and minors — Opus (straw poll rows 11, 128, 129, 130, and row 56's `archive.rs` items; R-D8; dossier Task 5, row 56 "Task 2")

**Files:**
- Modify: `crates/yokohama-web/src/games.rs` (`lock_game`, the `locks` prune, the doc at `:884-887`), `src/api.rs`
  (the thirteen `expect("a game")` sites; `admitted_with`; `events_at`; the hot-seat fence; `house_chat_post`;
  `chat_read`), `src/tables.rs` (the six table-lock sites; the tombstone doc; the create/delete leaf sentence;
  `read_chat`'s house branch; the 600-loop), `src/lib.rs` (the stream closure), `src/chat.rs` (whitespace text;
  `trim(Some(0))`; the refusal quotes; the non-UTF-8 value; `ChatConfig` docs; the normalized results), `src/store.rs`
  (`HOUSE` reserved by name), `src/archive.rs` (the marker's atomic write; `Failed` relative; the doc at `:93-95`),
  `tests/wire_fixtures.rs` (the frozen-clock cell narrowed).

**Interfaces:**
- Produces: `games::lock_game(lock: &Mutex<Repl>, id: &str) -> Result<MutexGuard<'_, Repl>, GameError>` — on poison,
  the handle is EVICTED (the next request rebuilds from the store) and the request answers `GameError::Server("the
  game's handle was poisoned; the next request rebuilds it")`; `api::admitted_with(table: &TableDoc, id, user) ->
  Result<(TableDoc, Viewer), ApiError>` (the read done once by the caller); the sentence `"a message needs some text"`.

- [ ] **Step 1: The cells, red** (each named in the report as a catcher):
  - `games.rs`: `a_poisoned_game_lock_evicts_the_handle_and_the_next_request_rebuilds_it` — a test that panics inside
    the guard (caught with `catch_unwind`), then a route answers 200 from the store; and
    `a_deleted_tables_lock_is_pruned` (`locks` no longer holds the id when its `Arc` is unshared).
  - `api.rs`/`lib.rs`: `events_at_reads_the_table_once` — a counting store double: one `Doc::Table` read per call.
  - `api.rs`: `a_hot_seat_creator_is_never_told_it_is_not_their_turn_between_two_lock_takes` — a store double whose
    table changes between the two takes; the fence reads the second.
  - `chat.rs`: whitespace-only text → 400 `"a message needs some text"`; `trim(Some(0))` keeps nothing and panics
    nowhere; the lobby-chat refusal quotes the value as typed; a non-UTF-8 `YOKOHAMA_WEB_CHAT_KEEP` is refused by the
    variable's name.
  - `tables.rs`: `a_landed_put_whose_stamp_failed_then_abandoned_lands_once_under_over` (row 11's residual as a
    positive: R-6.5b.13's path holds).
  - `archive.rs`: `the_marker_is_written_atomically` (no partial file is ever visible: write `.tmp`, then rename) and
    `a_failed_put_names_a_relative_path`.
  - `tests/wire_fixtures.rs`: `every_chat_message_carries_the_frozen_clock` narrowed to the chat pages by name.

- [ ] **Step 2: The fixes.** `lock_game` (R-D8) at every `expect("a game")` in `api.rs` (find by content; thirteen at
  the base) — one helper, one sentence; the six table `Mutex<()>` takes enter a poisoned lock
  (`unwrap_or_else(PoisonError::into_inner)`) with a comment saying why that is sound (the mutex guards no data).
  The `locks` prune: after `delete`, under the outer mutex, drop the entry when `Arc::strong_count == 1`; the field
  doc's by-design sentence is rewritten to the new rule. `admitted_with` and `events_at` reading once; the stream's
  per-frame closure runs `events_at` through `tokio::task::spawn_blocking` (a `.then`), the doc saying what it costs;
  the hot-seat fence recomputes `viewer_seat` under the second lock. The chat items as listed; `house_chat_post`
  through `author_of`; `chat_read` one table read (pass the doc into `chat_page`); `read_chat`'s dead `id` and the
  twice-stated house rule; the 600-loop becomes `keep + 1` with the reason; `HOUSE` reserved by a named check in
  `is_safe_id` (the doc says the underscore is the reservation and the name is pinned). Row 11: the two doc sentences
  made true (the residual: a stale mirror AND a lost stamp), no leaf change. The archive's marker written atomically;
  `Failed` carries paths relative to the archive root. The tombstone window (row 128's first item) is stated in
  `tables.rs`'s delete doc as by design: one refused move, never a lost one.

- [ ] **Step 3: Green.** Every cell of Step 1; every existing cell; **the corpus diff prints nothing** (no payload
  changed: run `wire_fixtures` plain and `git diff --stat -- web/src/wire`).

- [ ] **Step 4: The gates.** The Rust set; `cd worker && npm run build:module:debug && npm run typecheck && npm test`;
  `cd web && npm run build:wasm && npm run test:wasm`. No e2e.

- [ ] **Step 5: Commit.** `fix(web): a poisoned game lock evicts instead of killing the table, locks are pruned, the
  events route reads its table once and off the executor, the hot-seat fence reads the second take, the chat's
  whitespace and config edges refuse by name, the archive's marker is atomic and its failures relative`.

---

### Task 6: The seat-email roster off the wire — Opus (R-6.5c2.35; R-D3; straw poll row 82's roster half and row 104; dossier Task 6; Review Focus 1)

**Files:**
- Modify (the layer): `crates/yokohama-web/src/table.rs` (`SeatView`, `TableView`, `view`), `src/api.rs` (`holder_of`'s
  sentence at the input fence; `GamesList` rows; `get_table`), `src/tables.rs` (the delete refusal), `src/chat.rs`
  (`AuthorView`, `page_for`), `src/client.rs` (nothing: `ClientState` carries no address — assert), `tests/wire_fixtures.rs`
  (the new pin and its partner; the watcher pin's allowance).
- Modify (the client): `web/src/types.ts`, `web/src/wire/contract.ts`, `web/src/tables.ts`, `web/src/{Dealing,OpenTable,Lobby,App,Chat,TableFacts}.tsx`
  and their tests; `web/src/test/harness.tsx` where it builds a seat.
- Regenerate: the corpus (`UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`) — about twenty
  files; the diff is READ and listed in the report.

**Interfaces:**
- Consumes: `TableDoc` (`seats: Vec<SeatDoc>`, `created_by`, `seats_of`), `chat::ChatDoc`, `api::author_of`,
  `client::Viewer`, `games::lock_game` (Task 5).
- Produces: the shapes in §"Names" (`SeatView`, `TableView.mine/creator`, `AuthorView`, `ChatDoc::page_for`, the two
  sentences, `seatName`'s rule). **The stored documents do not change:** `SeatDoc::Human { user, name }`, `created_by`
  and `Author::{Seat,Watcher}.user` stay as stored; `TABLE_SCHEMA` stays 3; the Worker parses none of them.

**The rule, in one place (R-D3).** A payload built for reader R carries an address only where it is R's own: `/api/me`'s
`user`; and, in a payload built for the table's CREATOR, `seats[i].invited` for a human seat whose holder has not yet
given a name (the address the creator typed at create). Everywhere else a person is a `name`, a `you`, or the words
"invited" (a seat) / "someone" (an unnamed watcher). `creator` is the creator's seat name when they hold a seat, else
`null`; `mine` is whether R created the table.

- [ ] **Step 1: The layer's cells, red.** In `table.rs`: `the_view_names_no_address_but_the_readers_own` — a 3P table
  (red the creator, blue invited by address and not yet named, green an open seat): red's view has `seats[1].invited ==
  Some("blue@wire.test")`, `mine == true`; blue's view has `seats[1].you == true`, `invited == None` for every seat,
  and no `@` anywhere in its JSON; a stranger's view likewise no `@`; `creator == Some("Red")` once red has a name.
  In `api.rs`: the non-acting input refusal reads `it is seat 1's turn (Blue)` — never an address (the seat's name, or
  its colour when it has none). In `tables.rs`: a seated non-creator's delete reads `only the table's creator can
  delete this table`. In `chat.rs`: `page_for(reader)` marks `you` on the reader's own messages and carries no `user`.
  Red: four cells.

- [ ] **Step 2: The layer.** `SeatView` and `AuthorView` (serde-tagged, `rename_all = "snake_case"`, the same arm names
  as the documents'); `TableDoc::view` builds `seats` from `self.seats` with `you = user matches`, `invited = (creator
  == user && name.is_none()).then(|| address)`, `mine`, `creator`; every `TableView` producer goes through `view`
  (they do at the base). `chat_page` and the house page take the reader and answer `Vec<MessageView>` through
  `page_for`. The two sentences. `me.json`'s route is untouched.

- [ ] **Step 3: The corpus pin, red then green.** In `tests/wire_fixtures.rs` beside the seed pin:
  `no_payload_carries_an_address_that_is_not_the_readers_own` **(Review Focus 1)** — for every WALKED row (its reader
  is `ScriptRow.who`), walk the committed payload with the seed pin's tree walker: any string containing `@` must equal
  the reader's address, or sit at `seats[i].invited` in a payload whose `mine` is `true`; every CONSTRUCTED row carries
  no `@` at all (or is named in an allowlist with its reason, one line each — expected empty). Its partner:
  `the_stored_table_document_carries_every_seats_address` (a `Doc::Table` read from the store names blue's address
  beside red's, and the stored chat names each author's), so the pin cannot pass by absence. The watcher pin's
  allowance (`author.user` beside `kind: "watcher"`) is REMOVED: it now asserts no `user` on any author. Regenerate
  the corpus; **read the diff**: the report lists each changed file and the field that moved; `me.json`, `health.json`,
  `identity_cases.json` unchanged.

- [ ] **Step 4: The client.** `types.ts`: `SeatView`, `TableView` (`mine`, `creator`; `created_by` gone), `ChatAuthor`
  without `user` and with `you`. `contract.ts`: `TABLE` swaps `created_by` for `mine`, `creator`; a new `SEAT_HUMAN =
  ["kind", "name", "you", "invited"]` checked against the corpus's human seats; the two author sets drop `user`, add
  `you`. `tables.ts`: `seatName(seat)` → `seat.name ?? (seat.you ? "you" : seat.invited ?? "invited")`; `holder` and
  `seatChips` unchanged in shape. `Dealing.tsx:365` keys "you" on `seat.you`. `OpenTable.tsx`: the opener is
  `table.creator ?? "the creator"`, `Invite mine={table.mine}`, Abandon when `table.mine`. `Lobby.tsx:315` `yours =
  section === "mine" && t.mine`. `App.tsx:1474` `mayAbandon = local || (broken.table.mine && my_seats.length > 0)`.
  `Chat.tsx:326,331`: an author reads `a.name ?? (a.you ? "you" : a.kind === "watcher" ? "someone" : "seat " +
  a.seat)`. The jsdom cells keyed on `created_by` (`OpenTable.test.tsx:379,484`, `App.test.tsx:1874-1899`,
  `Lobby.test.tsx:129-132,327,488,579`) read `mine` from the regenerated corpus; a cell asserts a stranger's rendered
  open table shows "invited" and never an address.

- [ ] **Step 5: The gates.** The Rust set; `cd worker && npm run build:module:debug && npm run typecheck && npm test`
  (the corpus replay through the objects over the regenerated files); `cd web && npm run build:wasm && npm run
  typecheck && npm test && npm run test:wasm`; **`npm run e2e`** (the screens changed: the seat cards, the lobby rows,
  the chat labels), in the background, polled. Pictures at 1440 and 390 of the open table with an invited seat and of
  a chat with a watcher, into `$L/task-6-look/`.

- [ ] **Step 6: Commit.** `feat(web): a seat crosses the wire as a name and a "you", the creator as a flag and a name,
  a chat author without an address; the address the creator typed is served to the creator alone; pinned by a
  reader-aware leakage cell over every walked payload (R-6.5c2.35)`.

---

### Task 7: Chat delete and moderation, and the archive route's bounds — Opus (R-D4; straw poll rows 82's moderation half, 138, 81; dossier Task 7; Review Focus 2)

**Files:**
- Modify (the layer): `crates/yokohama-web/src/api.rs` (two routes, two dispatch arms, `chat_delete`,
  `house_chat_delete`, `Events.chat_rev`, `ARCHIVE_BODY_LIMIT`), `src/tables.rs` (`delete_chat` under the lock-then-bump
  order), `src/chat.rs` (`Message.deleted`, `ChatDoc.rev`, `ChatDoc::delete`, the refusal), `src/signal.rs`
  (`house_frame(seq, chat, rev)`), `src/backup.rs` (`check_pair_with`: the table's checks BEFORE the replay),
  `tests/wire_fixtures.rs` (the walk: three rows; the exact sets).
- Modify (the Worker): `crates/yokohama-worker/src/lib.rs` (the route table's two entries, if the module lists
  routes), `worker/src/{route,frames,house,table}.ts` (the two paths routed to their objects; the house frame's `rev`),
  the pool's chat cells; `worker/scripts/live-replay.mjs` (a delete after the house post; `HOST_FIELDS` if `rev` is
  host-specific — it is not).
- Modify (the client): `web/src/api.ts` (`deleteChat`, `deleteHouseChat`), `types.ts` (`ChatMessage.deleted`,
  `Events.chat_rev`), `contract.ts` (`CHAT_MESSAGE` + `deleted`; the events and house-frame sets), `Chat.tsx` (a Delete
  control on own messages, and on every message at a table the reader created; "message removed" for a tombstone; a
  page re-read when `chat_rev` moves), `web/e2e/exit_game.spec.ts` (x and y delete their posts at the end).

**Interfaces:**
- Produces: the routes and shapes in §"Names"; the refusal `only the message's author or the table's creator may
  delete it` (403); a deleted message `{ seq, created_at, author, text: "", deleted: true }`; `ChatDoc.rev` (+1 per
  append and per delete; `last` unchanged by a delete); `Events.chat_rev`; the house frame `{"seq":…,"chat":…,"rev":…}`;
  `api::ARCHIVE_BODY_LIMIT = 1 << 20` and the 413 `the record is larger than 1 MiB`; Task 6's `MessageView` gains
  `deleted`. **Row 8 rides here too:** under the `open` door a refused claim at `POST /api/archive` answers the Worker's
  **401** as the NAS does (R-C10 fixed: the shell's archive object maps the refusal through the layer's `error_body`
  arm rather than a 500), with a pool cell in the `open` project.

- [ ] **Step 1: The layer's cells, red.** `chat.rs`: `delete` by the author tombstones and bumps `rev`, keeps `seq` and
  `last`; by another user refuses; a second delete is idempotent. `tables.rs`/`api.rs`: the route answers 204 for the
  author and for the creator, 403 for a third seat and for a watcher on a seat's message **(Review Focus 2)**; the
  table's signal bumps AFTER the write (the gated-cell pattern of `signal.rs:222`); `Events.chat_rev` moves on a
  delete while `chat` does not; the house route: the author only; `house_frame` carries `rev`. `backup.rs`: a pair
  whose table disagrees with its record refuses on the table BEFORE any replay (a cell with a record whose replay
  would fail LATER and a table whose `players` is wrong: the sentence is the table's). `api.rs`: a body of
  `ARCHIVE_BODY_LIMIT + 1` bytes answers 413 before any parse (`the_archive_route_keeps_its_statuses_sentences_and_refusal_order`
  gains the row, first).

- [ ] **Step 2: The layer.** The routes in `routes()` (`DELETE /api/games/{id}/chat/{seq}`, `DELETE /api/chat/{seq}`);
  `chat_delete` admits the caller as `chat_post` does, takes the table's lock through `lock_game`'s sibling for tables,
  calls `ChatDoc::delete(seq, caller, creator)`, writes, then bumps; the house likewise under `house_lock`. `Events`
  gains `chat_rev` (the doc: a count for the text, a revision for the shape). The archive cap and the check order.

- [ ] **Step 3: The walk and the corpus.** Three rows after the existing chat rows: red deletes her own table message
  (`chat_delete.json`, 204 with an empty body — the fixture records status and headers as the walk's other 204s do),
  blue tries red's message (`chat_delete_403.json`), red deletes her house post (`house_chat_delete.json`); the
  events and house-frame fixtures gain `chat_rev`/`rev`. Regenerate; read the diff; the exact-set lists in
  `web/src/local/corpus.node.test.ts` and `worker/test/corpus.test.ts` gain the three names.

- [ ] **Step 4: The Worker.** The shell routes the two paths to the table and the house objects (find `route.ts`'s
  pattern list); the house frame's `rev` in `frames.ts`; the pool's corpus replay is green over the three rows; a
  pool cell for the 204 and the 403 through the objects. `live-replay.mjs`: after the house post row, its delete
  (the once-rule reads the second run's 404 or 204 as identical — decide and state); `worker/README.md` §10.4's
  "PERMANENT" sentence becomes the truth (the runs delete their posts).

- [ ] **Step 5: The client.** `api.ts`'s two calls; `Chat.tsx`: a "Delete" control on the reader's own messages and, at
  a table where `table.mine`, on every message; a tombstone renders as "message removed" in the author's row; the
  component re-reads the visible page when `chat_rev` moved (the `signal.ts` house-frame parse gains `rev`). jsdom
  cells for each; `exit_game.spec.ts` deletes x's and y's posts before the end. **`npm run e2e`** owed (a control
  appeared), plus one local cell where a browser deletes its message and the same page shows "message removed" after
  the frame (the spec that already posts chat locally: find by content).

- [ ] **Step 6: The gates** (the full set for a layer + Worker + client task) and **commit**: `feat(web): a chat message
  can be deleted by its author or the table's creator, as a tombstone the wire carries with a revision counter; the
  live runs delete their own posts; the archive route refuses a record over 1 MiB before parsing it and checks the
  table before replaying`.

---

### Task 8: The R2 backoff and the retired held arm — Opus (R-6.5c2.30; R-D5, R-D6; straw poll rows 95, 86; dossier Task 8; Review Focus 3)

**Files:**
- Modify (the Worker): `worker/src/table.ts` (`deliverArchive`, `retryArchive`, the commit site at `:1608`),
  `worker/src/r2.ts` (`ArchiveOwed.not_before?: number`), `worker/test/archive.test.ts` (the burst cell; `:1134`'s
  premise sentence).
- Modify (the layer): `crates/yokohama-web/src/lib.rs` (the `events` handler: no hold), `src/api.rs`
  (`EventsQuery`'s docs: `after` and `wait` accepted and ignored since R-D5; `check_wait` kept — a value above the cap
  stays a 400, the cells stay), `crates/yokohama-worker/src/events.rs` (its doc line).
- Modify (the client): `web/src/signal.ts` (the held arm, `HELD_REASK_MS`, `HELD_WAIT_S`, the `"held"` union member,
  `once` without `wait`), `web/src/api.ts` (`events()` without the `wait` parameter), `web/src/types.ts:484`,
  `web/src/local/facts.ts:35,52,147`, `web/src/signal.test.ts` (the three held cells).

- [ ] **Step 1: The pool cell, red.** `archive.test.ts`: under `failingPuts`, a game over owes a save; the first
  attempt fails (attempt 1, `not_before` set to the first rung); TWENTY requests inside the rung count ZERO further
  puts; the alarm at the rung puts once (attempt 2); a request after a SUCCESSFUL alarm put finds nothing owed
  **(Review Focus 3)**. The `:1134` cell's "resumes at the next request once on" still holds (the gate is set only by a
  failure). Red.

- [ ] **Step 2: The gate.** `deliverArchive({ fromRequest })`: on the request path, return early while `Date.now() <
  owed.not_before`; the alarm path ignores the field; `retryArchive` writes `not_before: this.retryAt(owed)` beside
  `attempt + 1`; `archiveOwedOf` tolerates a row without the field. The doc at `:1608` says what a request costs
  during an outage now: a read of the owed row, nothing else.

- [ ] **Step 3: The held arm, red then green.** `lib.rs`: `events_can_be_held_until_a_bump_and_the_wait_is_bounded`
  becomes `a_wait_is_accepted_and_answered_at_once` (a `?wait=25` answers within a bound with no bump); the handler's
  hold and its subscribe go; `EventsQuery`'s doc and `events.rs`'s say the keys are parsed and ignored (an older
  precached release's probe sends `wait=25`; `deny_unknown_fields` would otherwise 400 it — R-D5). `signal.ts`: the
  held arm and its constants removed; `SignalKind`/`Link`/`facts.ts`'s unions lose `"held"`; `once` calls
  `api.events(game)`; the three held cells are replaced by one asserting the socket arm's close probe sends no
  `wait`. `api.ts`'s `events` loses its third parameter.

- [ ] **Step 4: The gates.** `cd worker && npm run build:module:debug && npm run typecheck && npm run test:scripts &&
  npm test`; the Rust set; `cd web && npm run build:wasm && npm run typecheck && npm test && npm run test:wasm`; **the
  corpus diff prints nothing** (`git diff --stat -- web/src/wire` empty: the events rows carry no query); **`npm run
  e2e`** owed (the client's transport code moved), polled.

- [ ] **Step 5: Commit.** `fix(worker,web): an owed save is attempted once per rung and not once per request while R2
  is down; the held-request arm is retired on every host with its query keys still parsed for an older release`.

---

### Task 9: The Worker refactors under budget — Opus (R-6.5c2.30; R-D7; straw poll row 127; dossier Task 9)

**Files:**
- Modify: `worker/src/table.ts` → the base class plus `worker/src/table/{delivery,archiving,lifecycle,sockets}.ts`
  (mixins); `worker/scripts/live-replay.mjs`, `web/e2e/live.ts`, `worker/scripts/{assemble-assets,read-deploy}.mjs`,
  `worker/test/corpus.test.ts` (the dedups); `worker/scripts/replay/*.mjs` (the replay split, budget permitting);
  every test that names a moved file.

**The budget (R-D7): ONE day of wall, the order below, the fence at every step** — `npm run typecheck`, `npm run
test:scripts`, `npm test` (both projects, the 2,209-row replay among them), `npm run test:wasm` in `web/`; the module
untouched (`git diff --stat -- crates/` empty); the release module's bytes unchanged. At the budget's end, what is
undone is a `Ruling:` line in the ledger and a by-design row in the close.

- [ ] **Step 1: The `table.ts` split.** A structural node cell first (`worker/src/table.unit.test.ts`, or beside it):
  `Table.prototype` carries every method name in the dossier's row 127 map (the pool patches prototypes, so a method
  that stops being one breaks a cell silently). Then the split: `class TableBase` keeps the fields, `fetch`, `ready`,
  `answer`, `finish`, `wake`, `call`, `commit`, `owed`, `owedWrites`, `copyOf`, `batch`, `arm`, `pinned`, `reading`
  and the alarms; `withDelivery` (`deliverSlot`, `push`, `acknowledged`, `failed`, `recopy` + their helpers),
  `withArchiving` (`deliverArchive` … `archiveRung`, `owing`, `archiveOwedOf`), `withLifecycle` (`ended` … `force`,
  `copyFacts`), `withSockets` (`admit` … `webSocketMessage`, `requestedId`, `transportOf`, `isUpgrade`); `export class
  Table extends withSockets(withLifecycle(withArchiving(withDelivery(TableBase)))) {}`. The pool's patch sites keep
  resolving (the cell); nothing else in the pool changes. Each moved function keeps its comment.

- [ ] **Step 2: The four dedups** (dossier row 127's list). The `.dev.vars` reader as ONE module: try
  `worker/scripts/dev-vars.mjs` + `dev-vars.d.mts` imported by `web/e2e/live.ts`; if `web`'s `tsconfig.node.json`
  refuses the path, the fallback is a twin-pin cell in `web/e2e/harness.node.test.ts` that runs both readers on one
  input and asserts equality — say which in the report. The one origin, the shared field list and `order`, the one
  precache-list parser, likewise: shared where the packages allow, twin-pinned where not.

- [ ] **Step 3: The replay split** (only if the budget remains): `worker/scripts/replay/{credential,scripts,exemptions,
  send,judge}.mjs` with `live-replay.mjs`'s `replay` the one importer; `live-replay.test.mjs`'s cells import the
  modules they test.

- [ ] **Step 4: The gates and the commit(s)** — one commit per step, each green on the fence: `refactor(worker): table.ts
  as a base and four mixins, methods still on the prototype`, `refactor(worker,web): one .dev.vars reader, one origin,
  one field list, one precache parser`, `refactor(worker): the live replay in five modules`. The report states the
  hours per step and the budget's remainder.

---

### Task 10: The layer's test debts and the livelock run — Sonnet 5.5 (straw poll rows 31, 32, 39, 40, 33; R-D9; dossier Task 10; Review Focus 4)

**Files:**
- Modify: `crates/yokohama-web/tests/play_a_whole_game.rs` (the non-acting probe; the cap's doc and value),
  `src/games.rs` (the CAS conflict cell; the `NoSignal` and clock cells), `src/lib.rs` (`session_routes()`; the
  oversize-body cell), `src/tables.rs` (R-pre-3), `tests/wire_fixtures.rs` (two fixtures), `web/src/wire/contract.ts`,
  `web/src/local/corpus.node.test.ts` and `worker/test/corpus.test.ts` (the exact sets), `web/src/styles.css` (two
  comments naming each other).

- [ ] **Step 1: The gate's probe (Review Focus 4).** In `two_people_play_to_a_winner_and_each_sees_only_their_own_hand`,
  before each turn's input the OTHER identity posts the same line: 403, the sentence names the seat (Task 6's
  `it is seat N's turn (…)`), contains no `@`, and `events` is unchanged. Red first (the probe expects the seat's word;
  assert the count before the probe and after).

- [ ] **Step 2: The conflict arm.** A store double whose `write_if_version(Doc::Record, …)` answers
  `WriteError::Conflict` (copy `FailingWrites` at `games.rs:2038`, which answers `Io`): `persist` EVICTS the handle
  and the reply reads `APPLIED BUT NOT SAVED`, as the arm's doc says; the doc's "NO CELL DRIVES THE CONFLICT ARM"
  sentence is replaced by the cell's name.

- [ ] **Step 3: The two fixtures.** `deal_bot_table.json` (`GET …/deal` at the walk's bots table before finalize) and
  `table_two_open.json` (a 3P create with two open seats, `status: "open"`): emitted from the walk, added to
  `contract.ts`'s rows and to both exact-set lists; every leakage pin scans them (Task 6's pin included — the bot
  table's deal carries the reader's cards only).

- [ ] **Step 4: Rows 39 and 40.** `session_routes()` gains `/stream`, `/unlock`, `/record` and the router cell ties the
  list to the router; `unlock` bumps nothing (subscribe, unlock, `!rx.has_changed()`); the `@container (max-width:
  34rem)` rule and `--side: 36rem` name each other in comments (no instrument sees CSS; the coupling is stated).
  `every_bump_site_answers_under_no_signal` builds through `with_parts(.., Arc::new(NoSignal))` and its doc drops the
  "un-assertable" sentence; the clock cell through `with_parts`; a 2 MiB + 1 body answers `400 the body could not be
  read` (the adapter's sentence, pinned).

- [ ] **Step 5: The livelock run (R-D9).** In the background, logged and polled:

```bash
L=.superpowers/sdd/2026-09-28-web-frontend-m6.5c2d-plan
cargo build --release -p yokohama-web --tests 2>&1 | tail -1
( caffeinate -i bash -c 'for i in $(seq 1 500); do
  out=$(cargo test -q --release -p yokohama-web --test play_a_whole_game a_whole_two_player_game_against_the_bot -- --nocapture 2>&1); code=$?
  printf "%s\n" "$out" | grep "^gate: "
  [ "$code" -eq 0 ] || echo "gate: FAILED exit=$code"
done' > "$L/task-10-gate-500.txt" 2>&1 & )
```

  Poll `tail -3 "$L/task-10-gate-500.txt"` every few minutes (never a Monitor). The reading: `done`/`FAILED` counts,
  `inputs` min/median/max. **A `FAILED` line is a finding: STOP and report the seed.** Then `play_a_whole_game.rs:270-277`'s
  doc reads the 500-seed figures, and `VS_BOT_CAP` moves only if the max says so, in the same commit.

- [ ] **Step 6: The gates** (the Rust set; `cd worker && npm run build:module:debug && npm test`; `cd web && npm run
  build:wasm && npm run test:wasm` — the exact sets) and **commit**: `test(web): the acting-seat fence witnessed every
  turn of a whole game, the persist conflict arm driven, two fixtures the corpus lacked, three route and signal pins,
  and the vs-bot cap measured over 500 seeds`.

---

### Task 11: The client's defects by screen — Opus (straw poll rows 1, 2, 3, 4, 17, 38, 55, and row 130's client items; dossier Task 11, row 130 "Client")

**Files:**
- Modify: `web/src/{Dealing,GameMenu,Lobby,App,OpenTable,Chat}.tsx`, `web/src/whoami.ts`, `web/src/types.ts`
  (`KeepRequest` removed), `web/src/signal.ts:296`, `web/src/styles.css:1026`, `web/src/test/harness.tsx`, their
  tests; `web/e2e/the_table.spec.ts:160-180`; `crates/yokohama-web/tests/wire_fixtures.rs` (`freeze_clock`'s `next`
  arm by shape; the password partners).

**Interfaces:**
- Consumes: Task 6's `TableView.mine`/`creator`; Task 7's `chat_rev`.
- Produces: `GameMenu`'s `<details>` closed after a switch; `Lobby`'s per-section refusal strips; `Dealing`'s refetch on
  a `dealing` frame; `App`'s `open` restoring the previous game on a refusal and checking the route after each await;
  `OpenTable` taking `table` as a prop and reading it itself only when absent.

- [ ] **Step 1: The cells, red.** `Dealing.test.tsx`: a `dealing` frame whose `events` count moved (another seat kept)
  refreshes "waiting on"; a frame where only `chat` moved does not refetch the deal. `GameMenu.test.tsx`: after
  `onSwitch` resolves the summary's `details` is closed (assert `open === false`; jsdom sees the attribute). `whoami.test.ts`:
  a 401 landing during a deliberate sign-out's re-read sets no notice. `Lobby.test.tsx`: the first read with one
  scope refused shows the other two lists and one strip. `OpenTable.test.tsx`: a seatless creator sees Abandon. `App.test.tsx`:
  a refused `open` from the menu leaves the previous game drawn and the hash on it; Back during an in-flight open
  never draws the abandoned target; the open-table screen makes no second `…/table` request when given the doc.
  `Chat.test.tsx`: the deferred re-ask runs as a loop with a bound; a tick host makes no delta GET for its own post;
  exactly one chat read at mount. Red: the count quoted.

- [ ] **Step 2: The fixes**, each as the dossier's row words it. Row 3 is VERIFIED first: one headed look (the e2e
  harness's sign-out in a `--headed` Playwright run against the local server, recorded in the report); the guard in
  `whoami.ts` lands either way, since the window exists by reading. `usePolled` keeps its name; its doc says it is
  named for the poll it replaced (row 55). `.deal-pick.chosen` and `KeepRequest` go; `reread`'s `!` becomes a guard.
  Row 38: `isGamesList` at every `/api/games` matcher (twenty path-only, six `split("?")`); `freeze_clock`'s `next` arm
  keyed by shape, **the corpus byte-identical after regeneration** (assert `git diff --stat -- web/src/wire` empty);
  the two password needles gain partners that read the values from `web/src/wire-script/walk.json`; `join_403_locked`
  gets a render cell.

- [ ] **Step 3: The e2e cell.** `the_table.spec.ts:160-180` is rewritten for the fixed behaviour: after a switch the menu
  is CLOSED, and a summary click opens it (the cell used to depend on the defect).

- [ ] **Step 4: The gates.** `cd web && npm run typecheck && npm test`; the Rust `wire_fixtures` cell; **`npm run
  e2e`**, polled; pictures at 1440 and 390 of the lobby with one refused scope and of the dealing screen after another
  seat's keep, into `$L/task-11-look/` with a one-line README.

- [ ] **Step 5: Commit.** `fix(web): the dealing screen follows another seat's keep, the game menu closes on a switch, a
  deliberate sign-out shows no 401, one refused lobby scope leaves the others standing, a seatless creator can abandon,
  a refused open keeps the game, the table is read once, and the chat's re-ask is a loop`.

---

### Task 12: Backup and restore — Opus (straw poll rows 12, 13, 15, 16, 100; dossier Task 12; Review Focus 5)

**Files:**
- Modify: `web/src/backup.ts` (`openDbBounded`, `backupEverything`'s union, `unplayableIds`), `web/src/local/db.ts`
  (`restoreOne`'s compat row), `web/src/{BackupControls,Abandon,OnThisDevice}.tsx` and their tests, `web/src/backup.test.ts`,
  `web/src/local/db.test.ts`; `crates/yokohama-web/src/backup.rs` (`envelope_version_of`, `restore_read`).

**Interfaces:**
- Produces: `openDbBounded(ms = 3000)` rejecting `another tab holds this app's storage; close it and try again`;
  `backup::envelope_version_of(text: &str) -> Result<u32, String>` read BEFORE the whole parse, refusing `this is a
  version N backup; this build reads version 1`; `Abandon`'s "Abandon without saving" offered after a refused send;
  the device card's sentence `this device's games could not be listed` when the store is unreadable.

- [ ] **Step 1: The cells, red.** `backup.test.ts`: with fake-indexeddb holding v1 (row 27's staging, the pattern of
  `local/db.test.ts:223`), "Back up everything" answers the blocked sentence within the bound and `busy` clears
  **(Review Focus 5)**; a quota-refused create listed by the module and absent from the store is in the whole-device
  file (row 13). `backup.rs`: an envelope of version 2 with an extra key refuses by VERSION, not by the unknown field
  (row 15; the existing `an_envelope_of_another_version_is_refused_whole` changes its fixture to carry an extra key).
  `db.test.ts`: a compat row naming the CURRENT rev is dropped at the next restore and the game listed as playable
  (row 16). `Abandon.test.tsx`: after a refused (non-404) send the without-saving choice is offered; `:322` and `:335`
  keep their first answers. `OnThisDevice.test.tsx`: an unreadable store shows the sentence, not an empty list.

- [ ] **Step 2: The fixes**, then `cd web && npm run build:wasm` (the layer's `backup.rs` moved), the client's stubbed
  sentence (`BackupControls.test.tsx:184`, `backup.test.ts:524`) updated to the version sentence.

- [ ] **Step 3: The gates.** `cargo test -p yokohama-web` (+1); the wasm check; `cd worker && npm run build:module:debug
  && npm test`; `cd web && npm run typecheck && npm test && npm run test:wasm`; **`npm run e2e`** (Abandon's controls
  changed), polled.

- [ ] **Step 4: Commit.** `fix(web): a blocked backup names the other tab and recovers, the whole-device backup includes
  the games only the module lists, a newer envelope is refused by version, a compatibility row is re-derived at
  restore, and a refused send still lets a player abandon without saving`.

---

### Task 13: The local host's actor — Opus (straw poll rows 14, and row 56's local-host, wasm-host, corpus-replay and e2e items; dossier Task 13 rows 14, 56, 24)

**Files:**
- Modify: `web/src/local/{worker,client,useLocalHold}.ts` and their tests, `web/src/{App,Lobby}.tsx`, `web/src/api.ts:617-626`,
  `web/src/styles.css:1672-1694`, `crates/yokohama-wasm/src/lib.rs:206,243-244`, `crates/yokohama-web/tests/wire_fixtures.rs:178,233,3176-3212`,
  `crates/yokohama-web/tests/browser_played_pin.rs:299`, `web/src/local/corpus.node.test.ts:222`; the four `CAP =
  4000` copies and the two `REV` literals → `web/e2e/caps.ts`.

**Interfaces:**
- Produces: the actor's `hold`, `ready` and `adopt` replies gaining `{ kind: "unreadable", sentence }` (a
  `StoreUnreadable` is an answer, never a trap); `LocalHold`'s state `"unreadable"` drawn as the broken-store face with
  Reload; `web/e2e/caps.ts` exporting `CAP = 4000` and `REV` = `` `rev${health.semantics_rev}` `` read from
  `web/src/wire/health.json` at import.

- [ ] **Step 1: The cells, red.** `local/client.test.ts`: an unreadable store during `hold` answers `unreadable` and the
  page does NOT restart the host (the pattern of `exportPair.test.ts:153-180`); `useLocalHold.test`: the face is drawn;
  `client.test.ts`: `ask` leaves no pending entry when `postMessage` throws; `takeover` refuses while `held`; the
  restart gate lowers when `start()` throws inside `ask`; `api.test.ts`: `withUnsaved` keeps the warning when the reply
  has no `warnings` array; `App.test.tsx`: a local game's Game menu lists local games only in a multiplayer build;
  `crates/yokohama-wasm`: `set_clock` refuses a negative or NaN by name; `doc_of` before init is a typed refusal;
  `browser_played_pin.rs`: the `not_a_move` arm; `corpus.node.test.ts`: a non-JSON reply names the row.

- [ ] **Step 2: The fixes**, each as the dossier's row 56 words it (the local-host list: the pending leak, `takeover`'s
  `held`, the lost-handling stated once, the two dead switch arms, `reached.current` in an effect, the labels effect's
  cleanup, `.locked-out` sharing `.walking`'s rule, the hook releasing only what it held and `take`'s `live` guard);
  `hosts_own`'s reason and the drift message's offset in `wire_fixtures.rs`; `caps.ts` and its six importers. Row 24
  is FALSE at HEAD (a bot-first local game stores its first record at the human's turn): nothing to build; the
  parked clause in `plays_locally.spec.ts:393`'s comment says so, citing the dossier.

- [ ] **Step 3: The gates.** `cargo test -p yokohama-wasm` and `-p yokohama-web` (the pin, `wire_fixtures` plain — the
  corpus diff empty); `cd web && npm run build:wasm && npm run typecheck && npm test && npm run test:wasm`; **`npm run
  e2e`**, polled (the caps import and the local screens).

- [ ] **Step 4: Commit.** `fix(web): an unreadable store answers instead of trapping the local host, eleven local-host
  minors closed, the wasm host refuses a bad clock, the replay's error names its row and offset, and the e2e's cap and
  rev live in one file`.

---

### Task 14: The client's test debts — Sonnet 5.5 (straw poll rows 18, 19, 20, 23, 25, 26, 28, 30, 34, 41, 42; dossier Task 13)

**Files:**
- Modify: `web/e2e/{plays_locally,setup_screens,offline,the_table,every_spec_declares_its_worker}.spec.ts`,
  `web/src/local/corpus.node.test.ts:231-235`, `worker/test/corpus.test.ts:55-62` (its comment), `web/src/sw.node.test.ts`,
  `web/src/Abandon.test.tsx`, `web/src/backup.test.ts:624`, `web/src/signal.test.ts`, `web/src/route.test.ts`,
  `web/src/precache.ts:412`, `web/src/App.test.tsx:4736-4748`; a new node cell under `test:wasm` for the release module.

- [ ] **Step 1: Row 18.** `plays_locally.spec.ts:501-503` becomes the sibling's loop (`:589-596`): `act` until
  `unsaved.count() > 0`, twelve tries, named `actUntilUnsaved(page)` and shared by both cells. Run the cell alone five
  times (`npx playwright test plays_locally -g "APPLIED BUT NOT SAVED"`): five green.

- [ ] **Step 2: Row 19.** Five runs of `npm test` with the machine loaded (`yes > /dev/null &` ×4, killed after),
  outputs in `$L/task-14-jsdom-runs/`; every cell that fails once is named and hardened by the M6.5c method (a budget
  split, an awaited link, a `beforeEach` reset — the close names the three precedents). None failing in five runs is
  recorded as "not reproduced" with a `Ruling:` line.

- [ ] **Step 3: Rows 20, 41, 42.** `corpus.node.test.ts:231-235`: `expect(got).toHaveProperty(field)` then
  `not.toEqual` (the browser's rule is "present and differs": `multiplayer` is a boolean); the pool's comment names it
  the third statement. Row 41: a `channelOf` cell in `route.test.ts`; `precache.ts:412`'s regex explicit
  (`/digest:\s*"/g`); `every_spec_declares_its_worker.spec.ts:51` also matches `browser\.newPage\(`; `offline.spec.ts:431`
  gains its anti-vacuity partner; `App.test.tsx:4736`'s partner advances past the 5 s bound with the server answering.
  Row 35 is FALSE at HEAD (`hasMove` has one definition, `shared_table.ts:55`): nothing, said in the report.
  Row 42: `setup_screens.spec.ts:292`'s sweep filtered to the rows this cell opened; `backup.test.ts:624` reserializes
  the `RUN` slice alone; the held `connected` item is MOOT (Task 8) and the report says so.

- [ ] **Step 4: Rows 23, 25, 26, 28, 30, 34 — the new cells.** The height sweep at 390 over `.open-table` and
  `.on-this-device` controls (≥ 44 px) in `setup_screens.spec.ts`'s `height()` shape; `sw.node.test.ts` fake-timer
  cells for the deadline, the per-fetch timeout, `credentials`, the stamp-first order, `includeUncontrolled`, "took
  too long" and the abort-before-delete; `Abandon.test.tsx` cells for `backUp`'s and `abandonAnyway`'s catches (the
  Worker's trap/refusal split stays e2e-only: `worker.ts` imports the module — one sentence in the report); an
  `offline.spec.ts` cell whose tab is UNCONTROLLED when it clicks "Reload to use it" — if it cannot be staged within
  an hour, a `Ruling:` line records it not observable and why; a `test:wasm` node cell replaying the browser-played
  pair's bot moves through `web/wasm-node/` (the RELEASE module): the recorded moves reproduce; a `the_table.spec.ts`
  cell pressing a tile, Cancel, the picker closed, nothing applied; a `plays_locally` cell seeding a foreign-bot pair
  into IndexedDB and reading the foreign screen and a completed Remove (states A and B and the single-player build
  are recorded not observable locally, as the dossier's row 34 says).

- [ ] **Step 5: The gates.** `cd web && npm run typecheck && npm test && npm run test:wasm`; **`npm run e2e`**, polled —
  its count is the new reading; `cd worker && npm test` (the comment only; run anyway).

- [ ] **Step 6: Commit.** `test(web): the quota cell applies a move before it expects, the flakes named and hardened,
  the browser replay's presence rule, the 44 px sweep on two more screens, the service worker's decisions and Abandon's
  catches pinned, the release module replays the played pair, a Cancel walked, a foreign pair read`.

---

### Task 15: The cycle's close — ORCHESTRATOR (spec §3 as amended; §4 row D's "ends with"; R-D11; R-6.5c2.13)

No code. Every step is the orchestrator's. The order is C's, so a fix found in review never forces a second deploy or a
second live sequence: review → one fix wave → the scoped re-reviews → the battery → the ONE recorded sequence → **the
owner's word** → the deploys → the live tiers → the close document → the push and PR #42 → the handover into E. The
ledger is `L=.superpowers/sdd/2026-09-28-web-frontend-m6.5c2d-plan`.

**A. The branch is whole**

- [ ] **A.1** `git status` clean; `progress.md` holds a DONE line and a review line for each of Tasks 1–14; HEAD is
  Task 14's reviewed commit.
- [ ] **A.2 The fences**, output quoted in the close: the six fenced crates' diff against `014433df` prints nothing;
  `SEMANTICS_REV` 7; `git diff --stat 014433df -- web/src/wire web/src/wire-script` lists ONLY what Tasks 6, 7 and 10
  regenerated (their reports name the files); no bot behaviour moved (`git diff --stat 014433df -- crates/yokohama-core`
  empty is the proof).

**B. The review (Fable), one fix wave, the scoped re-reviews**

- [ ] **B.1** One **Fable** reviewer over `git diff 014433df..HEAD`, the spec at revision 2.8, the dossier, this plan and
  the ledger, in an EXPORT: the vacuity lens first (every new catcher reverted against its fix); the roster's pin tried
  with an address re-added under another key; the tombstone's `rev`; the backoff gate; the mixins' prototype; the
  sentences this plan and the tasks' reports wrote about themselves TRIED (the top yield every cycle); the fences.
  Report: `$L/final-review.md`.
- [ ] **B.2** ONE fix wave, each fix a NEW commit ("as a new commit" in every dispatch), Opus implementers, disjoint files
  if parallel; a fix that changes what a browser shows owes the e2e. A triage line per finding.
- [ ] **B.3** The scoped re-reviews of the wave — an Opus one and a Fable one, each in its own export. Their head is
  **the reviewed head** for everything below.

**C. The mutation battery**, at the reviewed head, an Opus agent on an EXPORT: each row applies the mutant, sees the
named catcher fail BY NAME, reverts, records the diff. The tasks' reports name the catchers; a catcher with no row
gets one; a row whose catcher does not fail is a FINDING. Report: `$L/battery.md`.

| # | the mutant | the catcher |
|---|---|---|
| 1 | `test.yml` regains a `push` trigger | Task 1's structure check |
| 2 | a web job's step changed in `web.yml` | Task 1's deep-equal |
| 3 | `undo_allowed`'s replay arm mapped to 403 | Task 4 |
| 4 | the waiting caller drawn for the lowest human seat | Task 4 (blue waits) |
| 5 | `matches` binds `""` | Task 4 |
| 6 | `recorded` counts `bga-human` | Task 4 |
| 7 | `lock_game` uses `expect` | Task 5's poison cell |
| 8 | `events_at` reads the table twice | Task 5's counting store |
| 9 | the hot-seat fence reads the first take | Task 5 |
| 10 | whitespace text accepted | Task 5 |
| 11 | the marker written in place | Task 5 |
| 12 | `SeatView` re-gains `user` | Task 6's pin (Review Focus 1) |
| 13 | `invited` served to a non-creator | Task 6's pin |
| 14 | an author re-gains `user` | Task 6's watcher pin |
| 15 | the input refusal names the address | Task 6 / Task 10's probe |
| 16 | a third seat may delete | Task 7 (Review Focus 2) |
| 17 | a delete leaves `rev` unchanged | Task 7 |
| 18 | a delete bumps `last` | Task 7 |
| 19 | the archive cap after the parse | Task 7's order row |
| 20 | the table checks after the replay | Task 7 |
| 21 | `deliverArchive` ignores `not_before` on a request | Task 8 (Review Focus 3) |
| 22 | the NAS `events` holds again | Task 8 |
| 23 | a mixin method moved off the prototype | Task 9's structural cell |
| 24 | the non-acting probe answered 200 | Task 10 (Review Focus 4) |
| 25 | `persist`'s conflict arm keeps the handle | Task 10 |
| 26 | `unlock` bumps the signal | Task 10 |
| 27 | the dealing frame ignored | Task 11 |
| 28 | the menu left open | Task 11's e2e cell |
| 29 | `Promise.all` back in the lobby's first read | Task 11 |
| 30 | `openDbBounded` without the bound | Task 12 (Review Focus 5) |
| 31 | the envelope parsed whole first | Task 12 |
| 32 | the compat row kept | Task 12 |
| 33 | `hold` answers `trapped` for an unreadable store | Task 13 |
| 34 | `set_clock` accepts NaN | Task 13 |
| 35 | the presence rule's `toHaveProperty` removed | Task 14 |
| 36 | the release-module replay fed the debug module | Task 14 |

**D. The ONE recorded sequence** (R-6.5c2.13), at the reviewed head: C's `close-sequence.sh` copied into this ledger
with `L=` set and **one row added — `cargo doc -p yokohama-web --no-deps` (R-D10)**; run in the background as ONE
uninterrupted run, machine-tagged, polled with `tail -n 3 "$L/sequence.tsv"`; every `rc` 0; the fifteen gates' walls
summed and set beside C's 461 s; a red row re-run ALONE before it is believed; each count set against Step 0's
baseline and the tasks' readings.

**E. ORCHESTRATOR — the owner's word, then the deploys** (R-D11; R-6.5c2.20)

- [ ] **E.1 ASK THE OWNER**, with the sequence's readings in hand: the Worker deploy, the NAS deploy, and the live tiers,
  each separately; and `YOKOHAMA_PROJECT_URL` for this release, or none. Nothing below runs without the answer.
- [ ] **E.2 Before anything live:** WARP (`curl -A 'Mozilla/5.0' https://yokohama.moore.nyc/api/me` names the owner;
  Access's 302 means the owner re-authorizes); `npx wrangler whoami`; `df -h`.
- [ ] **E.3 The release build, ONCE:** `cd web && npm run build` (with the link if given; `grep -c debug_trap
  src/local/wasm/yokohama_wasm.js` prints 0), `cd ../worker && npm run build`; the precache generator's four lines
  and the release name recorded; **the module figures re-read** (`node scripts/module-figures.mjs`; the browser
  module's size and a `gzip -9` reading) for the `Cargo.toml` paragraphs (Task 3's Step 2), updated in G.
- [ ] **E.4 The Worker, with a picker left open across it (row 110).** Open a throwaway table as `smoke@readme.test`
  through the dev door and start a two-step choice in a browser; then `node scripts/deploy-guard.mjs && npm run deploy`
  (`worker/RUNBOOK.md` §1–§2; the secrets are inherited); then the table's next request; read `picker-dropped` in the
  Workers Observability query (`worker/RUNBOOK.md` §7 names it — **row 137: the query API, not the tail, for every
  count in this close**). The readings of §5: `/api/health`, `/api/me` (recorded as "the owner's address"), `/sw.js`
  names E.3's release, the two secrets listed by name.
- [ ] **E.5 The NAS as ONE script (row 135).** `$L/nas-swap.sh` from `deploy/RUNBOOK.md` §3.17's four proxy calls
  (read the container, create with the same body and the image `yokohama-web:0.6.0-m6.5c2d`, start, read the
  listening line), the Portainer token from the portainer-mcp checkout's `.dev.vars`, the downtime read from the old
  exit to the new listening line beside E.6's 25.3 s and §3.16's 8.45 s. Then §3.2's readings: `/api/me` names the
  owner (a STOP otherwise: roll back by §5), `/` and `/sw.js` E.3's release.
- [ ] **E.6 A short game on the NAS under `cf-access` (row 107)**, through Claude-in-Chrome in the owner's signed-in
  browser: a 2P table against the bot, three moves, then read `run_id` (`web/server`) and the mirror's `bot` off the
  volume through the Portainer archive read (counts and fields, never an address). The first game on the NAS's
  `cf-access` arm by anyone.

**F. ORCHESTRATOR — the live tiers**

- [ ] **F.1** The quota guard (`worker/RUNBOOK.md` §7; HOLD above 80,000). **The replay's reset only if
  `archive-walk.json` moved** (A.2 says).
- [ ] **F.2** The live replay twice, sequentially; each HOLDS.
- [ ] **F.3 A FRESH tail attached and read FIRST, then** `cd web && YOKOHAMA_WEB_E2E_TARGET=live npx playwright test
  two_browsers joins_by_link` (row 109: a second unexplained watcher socket close is a finding; a clean run closes
  the item); the requests and rows-read for the game from the query API (row 137).
- [ ] **F.4 One large replay timed (row 117):** `POST /api/archive` on the Worker with the largest finished record in
  the tree (`crates/yokohama-web/tests/fixtures/browser-played-2p.json`, 54,103 B, paired) as a test identity; the
  wall and the object's CPU from the query API. Recorded, never asserted.

**G. The close document, the records, the current-state documents** — `docs/plans/<the day>-web-frontend-m6.5c2d-close.md`
on C's shape (the header; the headline and what is LIVE; §1 by task with §1.x the fences; §2 the sequence; §3 the
deployments, the picker-dropped reading, the NAS game, the live tiers, the timed replay, the quota; §4 the reviews
and the battery; §5 every ruling R-D1–R-D11 and every ledger `Ruling:` with its cost; §6 what E holds, restated from
the triage page as it stands, and the owner items; **§7 what this cycle does NOT claim** — at least: no public door;
the phones unseen; the roster's rule proven on the walked corpus and the stored documents, not on a live stranger; the
backoff proven in the pool, no live outage; the refactor budget's remainder; the livelock cap measured, not proven;
`picker-dropped` read once; the NAS game three moves long; §8 the handover). The records: `deploy/RUNBOOK.md` §3.18,
`worker/RUNBOOK.md` §18 (the twenty-fourth deployment), the `Cargo.toml` figures (E.3), `CLAUDE.md`'s counts in place
naming this close (the web section's move is E's), `guidingpath.md`'s ONE appended row (`M6.5c2 CYCLE D "THE CODE"
EXECUTED, CLOSED AND LIVE`, naming the close, the reviewed head, the deployments, R-6.5c2.28–.36 and R-D1–R-D11 by
number, the fences), the board, the spec's `§10.10 Corrections recorded in execution — cycle D`. Commit, documents
only. **Then the triage page** (the owner's phase-close view, never the record): one `ArtifactData` batch over its
`progress` collection — every D row `done` with its commit, the moot rows, the rows D pushed into E — read back once.

**H. The push, PR #42, and the handover into cycle E** (the push and the PR pre-authorized)

- [ ] **H.1** `git push -u origin web-room-m6.5c2d`; `gh pr create --base web-room-m6.5c2c --head web-room-m6.5c2d`
  with the cycle in one screen and the attribution line. **CI: nothing runs and nothing is dispatched** — `test` and
  `docker` are disabled on GitHub through E's merge and `web.yml` is dispatch-only (R-6.5c2.31 as amended); the PR
  page shows no checks, by design, and the close says so. `gh workflow list --all` in the ledger as the reading.
- [ ] **H.2** Nothing is merged and nothing is tagged: E's.
- [ ] **H.3** The handover `docs/plans/<the day>-web-frontend-m6.5c2e-handover.md`, under 200 lines, every fact off the
  TREE at the reviewed head: where D left the tree (the PR, every reading, both fences, what is LIVE); what E builds
  (spec §4 row E, restated: polish 5–9 and the owner's-eye calls with the paths of C's and D's pictures; paging,
  "Abandon and save", the arrival round trip; requests per move, bounded; the phones with the simulator recipe of
  this session — the bridge's agent install, `xcrun simctl boot`, the Android emulator headed with `-gpu host`, WARP
  passing identity; the public door's FULL precondition list; the documents with the D8 draft's paths and R-6.5c2.32's
  groupings; the merge and the tag under R-6.5c2.31's disabled workflows) and what D pushed into E (the budget's
  remainder, any `Ruling:`); the rules E carries; what D does NOT claim; every ruling of D's ledger; E's standing
  items. Then the board's "Updated" line and Track 3, the orchestrator's memory (`yokohama-web-lane-status`, the
  index), committed together and pushed.

---

## Appendix — the ledger

`.superpowers/sdd/2026-09-28-web-frontend-m6.5c2d-plan/` (gitignored): `progress.md` (the SDD identity line, the base,
Step 0's baseline, R-D1–R-D11, one line per task, one `Ruling:` per deviation); `triage-decisions.json` and
`phases.json` (Task 2's inputs, exported from the page by the orchestrator); `task-<n>-brief.md`, `-report.md`,
`-review.md`; `task-<n>-e2e.txt`; `task-1-ci-shape.py`; `task-10-gate-500.txt`; `task-14-jsdom-runs/`; `task-6-look/`,
`task-11-look/`; `final-review.md`, `fix-wave-*.md`, `battery.md`, `close-sequence.sh`, `sequence.tsv`, `sequence/NN.log`,
`nas-swap.sh`, `pr-body.md`, `handover-draft.md`. The C ledger (`…-m6.5c2c-plan/`) holds `close-sequence.sh`'s original,
the C pictures polish 5–9 are judged from, and the live tiers' logs the next readings sit beside.
