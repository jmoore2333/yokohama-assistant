# M6.5c2 cycle B — the screens: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan
> task-by-task. The execution shape is §"Execution shape" below, which records the owner's standing answers: every
> task subagent-driven, an Opus implementer and an Opus reviewer per task, Fable for the whole-branch review at the
> cycle's close. Steps use checkbox (`- [ ]`) syntax for tracking. **Read the spec first** (revision 2.6, §4 is the
> cycle map; §2.1 "The client", §2.2, §2.3 and §2.8 are this cycle's mechanism); this plan argues from it and does not
> restate its reasons. **Read §"Couplings between tasks" before any task**: an implementer sees only its own task, and
> the couplings are what no single task section can carry. **Every screen task loads `frontend-design:frontend-design`
> before it writes a line** (R-6.5c2.18).

**Goal:** The web client's setup screens as M6.5c2 leaves them — the door, the name step and the identity states, the
lobby, the two forms, waiting and joining, the password screens — rebuilt in the approved look from the eleven mocks;
"Spectate" in every word a person reads; the bot's version shown wherever a bot is named; a game this build's bot
cannot continue listed, shown and removable, never opened; the client's small items from M6.5c; every rebuilt screen
proven at 1440 and 390 by a browser and LOOKED AT beside its mock — on the wire cycle A left, with nothing hand-authored.

**Architecture:** The layer (`crates/yokohama-web`) is FINISHED for this cycle: cycle A committed every corpus file and
type the screens render from (handover §2), and cycle B touches Rust in exactly one place, Task 1's one test. Change
runs outward from the shared pieces: the components and tokens every screen uses (Task 2) before the first screen that
draws them (Task 3); the band and the identity corner (Task 4) before the lobby's lists under them (Task 5); the forms
(Task 6) before the table screens that share their frame (Task 7); the words and the foreign-game surfaces (Task 8);
the small items last (Task 9), because nothing depends on them. Every screen keeps the DOM contract the browser walks
read (`.your-games`, one link per row, `header.banner` as the GAME's alone), so the eleven existing e2e files keep
passing with word changes only.

**Tech Stack:** React 18 + TypeScript in `web/` (vitest over jsdom for the component cells; Playwright over Chromium
against the real release binary for the browser cells; `tsc -b` in four projects); the app's own stylesheet
(`web/src/styles.css`, `:root` tokens, no new colour role); Vite (one new `define`); Rust 1.97.1 for Task 1's one cell.

**Spec:** `docs/plans/2026-09-26-web-frontend-m6.5c2-spec.md` at **revision 2.6** — §2.1 "The client" (the name step,
the corner, states A–E, R-N3), §2.2 whole, §2.3 whole, §2.4's client items (the version tag, "Will not open here", the
game screen's foreign refusal), §2.8 "Task 8's items", §3 "The screens" / "The seed" / "The project link", §4 row B,
§10.5's tail (the eight spec-gap answers, each built as stated). The entry artifact is
`docs/plans/2026-09-27-web-frontend-m6.5c2b-handover.md` (§2 the wire as it IS, §3 the raw material and the known
corrections, §4 the rules, §5 what A does not claim). The mocks are `docs/plans/2026-09-26-web-frontend-m6.5c2-mocks/01…11`.

**How this plan was built, and what is measured.** It is assembled from the retired single plan's drafts D4 (retired
Tasks 9–12) and D5 (retired Tasks 13–16), under the gitignored
`.superpowers/sdd/2026-09-26-web-frontend-m6.5c2-plan/drafts/`, renumbered here (Appendix A is the map). **What was
measured:** D4 ran its four tasks in a `web/`-only export over stand-ins for cycle A's outputs — every red and green
count below marked "D4" is that export's; its rendered screens (`…/d4-look/`, eighteen PNGs at 1440 and 390) were read
beside mocks 01, 02, 03, 05 and 11. D5 ran its four likewise, with D4's `Band`, `Chips` and `SeatCard` copied in and
running — counts marked "D5". **Neither ran an e2e cell** (they needed cycle A's server); every e2e count here is a
prediction stated as a delta, and each task's implementer is the first to run it. **Cycle A's real outputs differ
from the drafts' stand-ins in four places, each carried in its task**: the seed table is EMITTED already (A's Task 7),
so Task 6 consumes it and emits nothing; `identity_cases.json`'s rows are A's 41, not D4's list (Task 4's required set
is copied from the committed file); the test-person rule is `Ben Castle` in every spelling (the retired review's P-2),
so D5's "Ben Moore", "b3n_m-o.re", "Jeremy" and "Jem" are replaced; and the game screen's version tag, which no draft
drew, is Task 8's. The three tasks that remain unmeasured by anyone at the seams — Task 8's foreign screen against
A's real 409, Task 9's `run` pin against A's real `run_id`, and every 390 cell — are named in Review Focus and their
reviewers measure them first.

**A plan sized to be read whole.** Cycle B's code is React whose contract is the mock, so the drafts' component code
is NOT restated here: each step names the draft's section and line range that holds it, and states what to CHANGE
from the draft. Code appears inline only where it defines a contract another task or a reviewer reads — the R-A19
cell, the interfaces, the DOM contract, the accessible names, the e2e helpers' shapes.

---

## Global Constraints

- **Branch `web-room-m6.5c2b`**, stacked on `web-room-m6.5c2a`; **the base for every code diff is `f2a3dd61`**
  (cycle A's close commit, whose code is the reviewed head `2e8e4d91` byte for byte). This plan's own commit is the
  branch's first; every `file:line` below was read at that base unless it says "D4/D5 base", which means `1e16fe27`
  (the drafts' base). Cycle A touched these client files, so a D4/D5 anchor in them may sit a few lines off: `App.tsx`
  (+4 lines), `Door.tsx`, `SignIn.tsx`, `OnThisDevice.tsx`, `whoami.ts`, `signal.ts`, `api.ts`, `types.ts`, `sse.ts`,
  `ws.ts`, `wire/contract.ts`, `e2e/harness.ts` (+97), `playwright.config.ts`, and five specs. **Find by content.**
  - Stage with explicit paths, never `git add -A`. `git status` is clean after every commit.
  - **Nothing is pushed until the close (Task 10).** The push and PR #40 against `web-room-m6.5c2a` are
    **PRE-AUTHORIZED** (the owner, 2026-09-27: *"push and PR are pre-approved at the orchestrator's discretion"*).
- **Fences** (spec §5). At every commit this prints nothing:
  `git diff --stat f2a3dd61 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli`
  — and so does `git diff --stat f2a3dd61 -- crates/yokohama-web/src crates/yokohama-wasm crates/yokohama-worker worker/`:
  **this cycle changes no Rust source and no Worker code.** The ONE Rust edit is Task 1's test file. `SEMANTICS_REV`
  stays **7**; the workspace version stays `0.5.0` (the bump is cycle C's). A task that finds it needs a layer change
  STOPS and asks the ORCHESTRATOR, who rules with a `Ruling:` line in the ledger.
- **The committed corpus is NOT regenerated this cycle.** Every payload a component renders comes from `web/src/wire/`
  as cycle A left it (the M2.5 rule; nothing hand-authored — a derived row is a corpus row with ONE field moved, and
  the cell says which), and the fake Worker (`web/src/test/fakeWorker.ts`) answers from the same corpus. `wire_fixtures`
  stays at **13** and its files unchanged; a task that believes it needs a new corpus file is a FINDING for the
  orchestrator. The two contract files the screens are pinned to are `identity_cases.json` (41 rows) and
  `seed_cases.json` (26 rows); `production_bot.json` is IMPORTED by the components directly (a Vite JSON import), never
  through `contract.ts`.
- **Every task that changes what the client shows** loads `frontend-design:frontend-design` first and builds against
  the mocks in the app's own stylesheet: the `:root` tokens only, NO new colour role (Task 2 adds two GROUND shades with
  a comment, the one addition), Fraunces for names and titles, JetBrains Mono for data; **no card art and no `<img>` on
  any setup screen** (`ASSETS.md`); the door asks the server NOTHING (R-6.5b2.7, a cell); `:focus-visible` on every new
  control; `prefers-reduced-motion` honoured; nothing wider than the window at 390 px.
- **The DOM contract the browser walks read is kept**: `header.banner` is the GAME's header alone (the setup screens
  use `header.lobby-band`); the lobby's lists are `section.your-games` / `.open-tables` / `.playing-tables` with `li`
  rows whose ONE link is the table's name; `data-busy` on the app root and `data-event` on each `TurnAccount` row stay.
- **Per-edit gates after EVERY task. Re-count at your own HEAD; a figure in any document is a reading, not a constant.
  Counts below are stated as DELTAS over your own HEAD's reading; the readings at `f2a3dd61` are handover §1's.**
  - **In `web/`:** `npm run typecheck`, `npm test`, `npm run test:wasm` (needs `npm run build:wasm` once if
    `src/local/wasm/yokohama_wasm.js` is absent; no task here changes Rust the module compiles, so one build serves
    the cycle).
  - **`cd web && npm run e2e`** is owed by every task that changes what a browser DOES or SHOWS — Tasks 3, 4, 5, 6, 7,
    8 and 9, each saying why. Its run is **SEQUENCED**: never two Playwright runs at once, and no other agent's
    Playwright while yours runs. It takes 6.5–15 min; run it **in the background with its output to a file in the
    ledger and POLL that file** (`tail -3` every minute or two) until the summary line appears — **never a Monitor,
    never an event wait, never a re-arm**: an implementer in cycle A parked on one and stalled five hours. The DEBUG
    module it builds is left in `web/src/local/wasm/` and `web/dist/` afterwards, which nothing in this cycle reads.
  - **The Rust and `worker/` gates** run at every task anyway and MUST NOT MOVE: `cargo test -p yokohama-web`,
    `-p yokohama-wasm`, `-p yokohama-worker`; `cargo clippy --workspace --all-targets -- -D warnings`; in `worker/`,
    `npm run typecheck`, `npm run test:scripts`, `npm test` (no module rebuild is owed: no task changes Rust a module
    compiles, which the second fence command proves). Task 1 alone moves one count (`browser_played_pin` 3 → 4).
  - **A red whole-game gate** (`cargo test -p yokohama-web --test play_a_whole_game`) is a FINDING now, not a re-run:
    cycle A fixed its livelock and it reads 2 passed in ~25 s.
- **The headed look is a STEP with an artefact, in every screen task**: the task's browser cell writes its screenshots
  at 1440 and 390 through the `look` helper (Task 3 defines it) into the run's `test-results/`; the implementer COPIES
  them into the ledger under `task-<n>-look/`, names each in the report beside the mock it answers to, and says what
  differs; **the reviewer and the orchestrator read them beside the mock before acceptance** (the Read tool renders a
  PNG). No jsdom cell sees a stylesheet, so a task with no pictures is a task whose look nobody checked.
- **Test data never uses the owner's surname or first name** (the retired review's P-2; R-6.5c2.16): every test person
  is `Ben Castle` in some spelling (`"  Ben   Castle "`, `"ben castle"`, `"BEN"`, `"b3n_c-a.stle"`), every second
  person `Ada Bell` or an address under `wire.test` / `e2e.test`. The hostnames under `moore.nyc` are real and stay.
  **The owner's email address goes into no document.**
- **The ledger** is `.superpowers/sdd/2026-09-27-web-frontend-m6.5c2b-plan/` (gitignored). `progress.md` holds the SDD
  identity line first, one line per task and one `Ruling:` line per deviation from this plan. **Every agent writes its
  full report to a file there and replies with the PATH and three lines**; a reviewer's report is ordered Verdict →
  Issues → Assessment, each finding compact (`file:line`, defect, why, fix).
- **Commit messages** quote the task's red runs in the body and end with the EXECUTING agent's own attribution lines.
- **A reviewer's mutants run on an EXPORT** (`git archive HEAD | tar -x -C <dir>`; for Task 1, ONE shared
  `CARGO_TARGET_DIR` under the session's scratchpad), never in the tree an implementer holds; a mutated file is
  restored with `cp` and `touch`, never `mv` from an older copy (cargo keeps the mutant's fingerprint otherwise — one
  false reading in cycle A). While an implementer works, nothing runs in its tree and no timer is armed.
- **No step in this cycle touches the owner's account**: no deploy, no secret, no bucket, no live tier. The two live
  specs (`two_browsers`, `joins_by_link`) and the exit spec change WORDS here and must type-check; their live runs are
  cycle C's close. The NAS is NOT redeployed from this branch.
- **Entry for the executing session.** Read, in order: `CLAUDE.md`; `docs/handover.md`'s "Updated" line; the spec's
  §4 and §10.6; the handover; this plan WHOLE; the ledger's `progress.md`. Confirm `git branch --show-current` prints
  `web-room-m6.5c2b`, `git status --short` prints nothing, and `git log --oneline -1` is at or after this plan's
  commit. Then Task 1 Step 0, which records the baseline every delta in this plan is read against.

---

## Review Focus

The spec implies these five inputs, and no cell would meet them unless someone put it there. Each line says what a
person would expect; its test lives in the owning task and is marked **(Review Focus N)** there.

1. **A name typed with capitals and extra spaces at the name step, against the REAL server** (`"  Ben   Castle "`).
   Expected: the header carries `ben castle`, `/api/me` answers it, the corner shows `Ben Castle`, and the same person
   typing `ben castle` in another browser reaches the same seats. (Task 4: `nameRule.test.ts` over all 41 rows, and
   the `setup_screens.spec.ts` name-step cell.)
2. **Seed text the server accepts that a naive check refuses, and the reverse** (`"+007 "`, `" 7"`, `"007"`,
   `"\u000112"` with U+0085 accepted; `"７"`, `"1e3"`, `"-1"`, `"18446744073709551616"`, `"﻿12"` refused).
   Expected: the fold says exactly what the server would, in place, and holds the button. (Task 6: `seedRule.test.ts`
   over all 26 rows, `NewGame.test.tsx`'s table cell, and `the_forms.spec.ts`'s in-fold refusal.)
3. **An expired Access session, which fails BOTH reads** (the hostname-wide application fronts `/api/health` too).
   Expected: state A, "Your sign-in has run out", with the turnstile — never "Can't reach the server", which would send
   a player at their network. (Task 4: the A-before-C cell, and `Door`'s discriminator order.)
4. **Arriving at a table by a link, or opening a link preview.** Expected: nobody is seated until a button is pressed —
   one click, never zero — and with one free seat there is exactly one button. (Task 7: "seats nobody on arrival" with a
   faked clock, and `joins_by_link`'s request log.)
5. **A 409 on the game route.** Expected: a body carrying `foreign_bot` is a SCREEN (the sentence, Abandon for the
   creator, the way back); a 409 without it (the deal's own) keeps the bounce to the lobby; and the screen opens no
   stream for the refused game — the GET's body decides, never the transport (handover §5: the SSE stream writes a
   comment and stays open, the socket sends nothing). (Task 8: the three route cells and the no-stream assertion.)

---

## Execution shape (the owner's standing answers)

- **Mode: SUBAGENT-DRIVEN, EVERY TASK** (the owner, 2026-09-21, standing; R-6.5c2.18). Each task gets a fresh **Opus**
  implementer and a fresh **Opus** task reviewer; **Fable** does the whole-branch review in Task 10. Folding two small
  tasks into one execution unit is the orchestrator's call (spec §9); each task keeps its own green commit either way.
- Tests are written first and RUN RED; the red run is quoted in the commit body and in the ledger. Every commit passes
  every per-edit gate.
- Standing process rules: never follow up an implementer's DONE without reading HEAD first; a reviewed SHA is never
  amended — a fix is a NEW commit, and the acceptance reply that asks for it says "as a new commit"; while an
  implementer works, run nothing in its tree and arm no timers; wake a stalled agent by message rather than spawning
  another; check the ledger within ~15 minutes of any long run; a reviewer's mutants run on an export.
- **The orchestrator edits no code.** Its steps are Task 10's (the review dispatches, the push and the PR —
  pre-authorized — the handover into cycle C, the board and the memory), and the headed look's reading at every task.
- **Rulings taken at planning (2026-09-27, the owner):** R-B1 the R-A19 witness is Task 1 of this cycle; R-B2 the name
  step keeps SERVER PARITY (an `@`-shaped name is admitted under `open` exactly as `User::parse` admits it; the hint
  copy reads the plain-name rule as mock 03 shows); R-B3 L-6 (`outbox.ts`) is Task 9's, against spec §4 row C's
  placement, because it is `web/src` code beside Task 8's items and D5 drafted it with its cells; R-B4 the disk was
  cleared by the owner (~83 GiB free at planning).

---

## Names shared across tasks

An implementer sees only its own task. **These names are the contract between tasks and are used verbatim.** A task
that finds the code forces a different shape STOPS and asks the ORCHESTRATOR, who amends this table with a `Ruling:`
line. It never renames on its own.

### What cycle A produced, consumed here as it IS (read off the tree, never off this table)

| name | shape | where |
|---|---|---|
| `web/src/wire/identity_cases.json` | 41 rows `{"display": str \| null, "key": str \| null, "raw": str}`; `wire.identityCases: readonly IdentityCase[]`, `IdentityCase` from `./wire/contract` | Task 4 |
| `web/src/wire/seed_cases.json` | 26 rows `{"ok": bool, "raw": str}`; `wire.seedCases: readonly {raw: string; ok: boolean}[]` | Task 6 |
| `web/src/wire/production_bot.json` | `{"identity": "v4-step9-draft1", "label": "V4"}`; IMPORTED directly: `import productionBot from "./wire/production_bot.json"` | Tasks 3, 5, 6, 8 |
| `TableView.bot: TableBot \| null`, `TableBot { label: string; runs_here: boolean }` (`types.ts`) | every listing row; `wire.tableBot` (an undealt vs-bot table, `runs_here: true`), `wire.tableForeign` (`{label: "V2", runs_here: false}`, `status: "playing"`, `over: false`, `created_by: "local@browser"`) | Tasks 3, 5, 8 |
| `ClientState.bots: SeatBot[]`, `SeatBot { seat: PlayerId; label: string }` | the record's bot seats, labelled; `wire.watching.bots` is `[{seat: 0, label: "V4"}]` | Task 8 |
| `ForeignBotErrorBody { error, foreign_bot: {played, runs} }`; `wire.foreignBot409` | the 409 every game route answers for a not-over game whose bot this build does not run: `"This game was played against V2. This server now runs V4 and can't continue it."` | Task 8 |
| `wire.whyForeign` | `why`'s thirteenth refusal, `kind: "foreign_bot"` — already rendered by `Explain.tsx`'s generic refusal path; no B task touches it | — |
| `Me.mode: "open" \| "cf-access" \| "worker"`; `wire.me` (`{mode: "open", user: "red@wire.test"}`), `wire.meSignedOut` | the door's answer; `useIdentityMode()` (`api.ts:511`) reads `/api/health`'s `identity` | Tasks 4, 6 |
| `api.ts`: `CLAIMED_USER_KEY`, `DISPLAY_NAME_KEY`, `setClaimedUser`, `clearClaimedUser`, `displayName`, `setDisplayName`, `clearDisplayName`, `ACCESS_CHALLENGE`, `isSessionExpired`, `BOUND_MS`, `bounded`, `LOCAL_USER`, `lobbyChat`, `api.{create, join, remove, recordText, me, health}` | as at `f2a3dd61` | Tasks 2–9 |
| `web/src/test/harness.tsx`: `stubApi`, `gate`, `openStream`, `fakeStream`, `isGamesList`, `gamesNamed`, `flush`, `until`, `gameMenu`, `renderApp`; `web/src/test/fakeWorker.ts`: `stubWorker`, `currentWorker` | the jsdom harness | Tasks 2–9 |
| `web/e2e/harness.ts`: `signIn(page, email?, url?)`, `newGameForm`, `startGame`, `openFromList`, `readOverflow`, `blame`, `watchForBreakage`, `WALKER`; `web/e2e/sw.ts`: `test`, `expect` (every new spec imports `test` from `./sw`, or `every_spec_declares_its_worker` fails it) | the browser harness | Tasks 3–9 |

### What cycle B produces (the contract between its own tasks)

| name | shape | task |
|---|---|---|
| `web/src/BoardMark.tsx`: `<BoardMark width={number} />`, `export const TILES`, `type Trade`, `interface Tile` | the door's original SVG drawing: ten tiles, 4/3/2/1, `aria-hidden`, classes `.board-mark`, `.bm-tile.<trade>`, `.bm-president`, `.bm-cube` | 2 |
| `web/src/Band.tsx`: `<LobbyBand crumbs={string[]} corner={ReactNode} />`, `<IdentityCorner me={Me} onChange={() => void} />` | `header.lobby-band` with `h1` "Yokohama"; **the LAST crumb is the page's `<h2>`**, earlier crumbs plain `span.crumb`; the corner is `.band-corner`. `IdentityCorner`: "Playing as <b title={me.user}>{display ?? user}</b>"; under `open` the one button is **"switch player"** (clears the display name, calls `onChange`); under an edge door **"change name"** (edits the display name, calls nothing); a `.identity-pill` disclosure at phone width over the SAME menu; renders nothing when `me.user === null` | 2 |
| `web/src/Chips.tsx`: `<SeatChip color label open? toMove? />`, `<PasswordTag />`, `<Padlock size? />`, `<VersionTag label />` | `.seat-chip` (+ `.open`, `.to-move`), the dot `i.seat-dot.seat-<colour>` with `role="img"` named for the colour; `.password-tag` `role="img"` **named "password required"**, text "Password", a drawn `svg.padlock`; `.version-tag` with `title="the bot's version"` | 2 |
| `web/src/SeatCard.tsx`: `<SeatCard color caption who note? action? open? />` | `section.seat-card.seat-<colour>` (+ `.open`), a **region named by its caption** | 2 |
| `styles.css` tokens `--ground-glow: #16304f`, `--ground-deep: #0f1c2c`; classes `.raised-card`, `.card-band` (`.yen` / `.vp` / `.warn`), `.card-in`, `.card-cap`, `.lede` under a card, `button.primary.yen`, `a.primary-link` | the shared frame; Tasks 3–7's stylesheets do NOT restate them | 2 |
| `web/scripts/project-url.mjs`: `checkProjectUrl(raw: string \| undefined): string \| null`, `PROJECT_URL_VAR`, `PROJECT_URL_MAX`; `project-url.d.mts` beside it | the build's check; `vite.config.ts` `define: { __YOKOHAMA_PROJECT_URL__ }` | 3 |
| `web/src/projectLink.ts`: `type ProjectLink = {href; text}`, `linkFor(url)`, `projectLink()` | the door's one reader of the define | 3 |
| `FrontDoor` props `multiplayer`, `notice?`, `readiness?`, `link?: ProjectLink \| null`; `OnThisDevice` prop `readiness?`; `BackUpEverything` prop `label?`; `Help` prop `shots?: boolean` (default `true`) | | 3 |
| `tables.ts`: `deviceWord(t: TableView): string` | the door's row words ("vs V4 · your move") | 3 |
| `web/e2e/setup_screens.spec.ts` with helpers `noSidewaysScroll(page, what)`, `look(page, name)`, `images(page)`, `height(page, role, name)`, constants `DESKTOP = {1440×900}`, `PHONE = {390×844}` | ONE spec for the door (3), the name step (4) and the lobby (5); the width rule written once | 3 |
| `web/src/nameRule.ts`: `normalizeName(raw): {ok: true; key; display} \| {ok: false; reason}`, `NAME_MAX = 32`, `type NameVerdict` | Rust's trim, ASCII lowercase, the plain-name grammar, the email shape — pinned to all 41 rows | 4 |
| `whoami.ts`: `type DoorFailure = "challenge" \| "network" \| "refused"`, `failureOf(e)`, `Door.failure: DoorFailure \| null`; `doorSentence(mode: Exclude<Me["mode"], "open">)` | | 4 |
| `Door.tsx`: `Door({door, serverUp: boolean \| null, onSignedIn, onRetry})`, `export function Unreachable({onRetry})` | states A–D + the refusal card + the name step + B, in the order §2.1 tests them (A first) | 4 |
| `signal.ts`: `Link = "connecting" \| "live" \| "refreshing" \| "reconnecting"` | R-N3 | 4 |
| `Lobby` props: `who` → `corner?: ReactNode`; `onRetry?: () => void` | the band's corner and state C's retry | 4 |
| `App.tsx`: `retry`, `healthTick`; `serverCorner` (the identity corner node every SERVER setup screen's band takes) | | 4 (`retry`), 6 (`serverCorner`) |
| `tables.ts`: `type Section = "mine" \| "open" \| "playing"`, `ago(createdAt, now?)`, `rowMeta(t, section, now?)`, `seatChips(t): ChipModel[]`, `rowAction(t, section): RowAction`, `type ChipModel`, `type RowAction` (`go` / `copy` / `stranded`) | the lobby row's words; action buttons named `"<label>: <table name>"` | 5 |
| `web/src/seedRule.ts`: `checkSeed(raw): {ok: true} \| {ok: false; reason}`, `trimSeed(raw)` | Rust's `str::trim` (the `White_Space` property), then `^\+?[0-9]+$` and a `BigInt` bound — pinned to all 26 rows | 6 |
| `NewGame` props: `onCancel: () => void`; `export function drawSeat(n)`; the two forms' accessible names: `"<Colour> seat"` (combobox), `"<Colour> player's name"` / `"…address"`, groups "Players" / "First player" (radios), "Name", "Ask for a password to join or spectate", "Table password", "Seed" under `details.fold` "Advanced", buttons "Deal" / "Open table" / "Cancel"; every "New table" heading query takes **`level: 3`** | | 6 |
| `web/e2e/harness.ts`: `nameThisBrowser(page, name)` | writes `localStorage["yokohama.display_name"]` | 6 |
| `web/e2e/the_forms.spec.ts` | the two forms at 1280 and 390, the seed refused in its fold | 6 |
| `OpenTable`'s accessible names: "Join as <Colour>" (one free seat), "Sit here as <Colour>" (several), "Invite link", "Copy link" → "Copied", "Table password"; `Unlock`'s "Spectate", "Table password" | | 7 |
| `leaveLabel?: string` on `LockScreen`, `Dealing`, `GameMenu` (default `"Lobby"`); `App.tsx`: `localExit` | the four single-player controls | 8 |
| `backup.ts`: `removeForeign(id)`; `Unplayable` props `foreign?: readonly TableView[]`, `onRemoved?` | "Will not open here" for a foreign row, through the module's fenced delete | 8 |
| `Dashboard` prop `bots?: readonly SeatBot[]` | the Players panel's version tag | 8 |
| `web/src/test/fakeSocket.ts`: `export class FakeSocket` | the ONE fake socket | 9 |

---

## File structure

**Created**
- `web/src/BoardMark.tsx`, `Band.tsx`, `Chips.tsx`, `SeatCard.tsx` and their `.test.tsx` (Task 2)
- `web/scripts/project-url.mjs`, `project-url.d.mts`, `project-url.node.test.ts`; `web/src/projectLink.ts`,
  `projectLink.test.ts`; `web/e2e/setup_screens.spec.ts` (Task 3)
- `web/src/nameRule.ts`, `nameRule.test.ts`, `Door.test.tsx` (Task 4)
- `web/src/seedRule.ts`, `seedRule.test.ts`; `web/e2e/the_forms.spec.ts` (Task 6)
- `web/src/test/fakeSocket.ts`; `web/src/devProxy.node.test.ts` (Task 9)
- in the ledger (gitignored): `task-<n>-look/*.png` for Tasks 3–8, `task-<n>-e2e.txt` for every e2e run

**Modified** (principal files; each task's own list is the authority)
- `crates/yokohama-web/tests/browser_played_pin.rs` (Task 1 — the cycle's one Rust edit)
- `web/src/styles.css` (Tasks 2–7: one appended block per task, superseded rules deleted by selector)
- `web/src/FrontDoor.tsx`, `OnThisDevice.tsx`, `BackupControls.tsx`, `Help.tsx`, `tables.ts`, `vite.config.ts`,
  `vite-env.d.ts`, `vitest.node.config.ts`, `tsconfig.node.json`, `deploy/Dockerfile` (Task 3)
- `web/src/SignIn.tsx`, `Door.tsx`, `whoami.ts`, `signal.ts`, `Lobby.tsx`, `App.tsx`, `Help.tsx` (Task 4)
- `web/src/Lobby.tsx`, `tables.ts`, `Help.tsx` (Task 5)
- `web/src/NewGame.tsx`, `App.tsx`, `Help.tsx` (Task 6)
- `web/src/OpenTable.tsx`, `Unlock.tsx`, `LockScreen.tsx`, `App.tsx`, `Help.tsx` (Task 7)
- `web/src/App.tsx`, `GameMenu.tsx`, `Dealing.tsx`, `LockScreen.tsx`, `Chat.tsx`, `Help.tsx`, `OnThisDevice.tsx`,
  `BackupControls.tsx`, `backup.ts`, `Dashboard.tsx` (Task 8)
- `web/src/signal.ts`, `api.ts`, `App.tsx`, `Abandon.tsx`, `outbox.ts`, `ws.test.ts`, `signal.test.ts`,
  `web/vite.config.ts` (Task 9)
- The e2e: `harness.ts`, `plays_locally.spec.ts`, `joins_by_link.spec.ts`, `two_browsers.spec.ts`, `exit_game.spec.ts`,
  `the_table.spec.ts` (P-7, Task 6), each in the task that changes its words

---

## Task order and why

```
1 ─► 2 ─► 3 ─► 4 ─► 5 ─► 6 ─► 7 ─► 8 ─► 9 ─► 10 (the close, ORCHESTRATOR)
```

1. **1 first, alone:** a measured layer cell, one commit, no client dependency — it closes cycle A's parked residue
   before any screen work stacks on it (R-B1).
2. **2 before 3:** the shared pieces exist, with their own cells and a "no existing element restyled" check, before the
   first screen draws them; Task 3 is the first headed look.
3. **3 before 4:** Task 4's `App.tsx` edits sit below Task 3's (the door gets `link`), and `setup_screens.spec.ts`'s
   helpers are Task 3's.
4. **4 before 5:** the band, the corner and the actions row frame the lists Task 5 rebuilds; Task 5 edits the fragment
   Task 4 leaves.
5. **4 before 6, 6 before 7:** the forms take the corner Task 4 builds (hoisted as `serverCorner` in Task 6) and the
   `.band-screen` / `.form-field` / `.quiet-link` rules Task 6 adds; Task 7's screens share all three. Task 6 also
   defines `nameThisBrowser`, which Task 7's walks use.
6. **5 before 8:** Task 8's help text and cells read "Being played", the name Task 5 gives the third list.
7. **7 before 8:** Task 8's `leaveLabel` lands on the `LockScreen` shape Task 7 restyles.
8. **9 last:** the small items depend on nothing later; `signal.ts`'s `lost()` (Task 4) is the line Task 9's battery-37
   mutant reorders, so 9 follows 4.
9. **10:** the orchestrator's close.

---

## Couplings between tasks (binding; the later task's implementer does what its item says)

- **Task 2 → every screen task: the band's LAST crumb is the page's `<h2>`.** So a card under a band titles itself
  `<h3>`, and every jsdom query or Playwright locator for "New table", a table's name or "Choose a name" as a heading
  takes **`level: 3`** where the band also carries the word — D5 measured `Found multiple elements with the role
  "heading" and name "New table"` without it. No band ends in "Lobby" but the lobby's: `App.test` and every walk wait
  for the lobby by that heading (the open table's loading face reads `["Lobby", "Table"]`).
- **Task 2 → Task 3: `.raised-card`, `.card-band`, `.card-cap`, `.lede`, `button.primary.yen`, `a.primary-link`** are
  Task 2's; Task 3's door stylesheet uses them and restates none.
- **Task 3 → Task 4: `App.tsx`'s door call gains `link={projectLink()}`**, moving the lines below `:1349` by +6; Task
  4's anchors below it are D4-base numbers — find by content. Task 3's `Help.tsx` edits (the `shots` prop's doc, the
  door's sentence) move Task 4's Help anchors by +4.
- **Task 3 → Task 4 → Task 5: `setup_screens.spec.ts`.** Task 3 creates it with the door's cell and the four helpers;
  Task 4 ADDS the name-step cell; Task 5 ADDS the lobby cell and the `PRODUCTION_LABEL` reader. Each task appends; none
  rewrites another's cell.
- **Task 4 → Task 5: `Lobby.tsx`'s frame.** Task 4 rewrites the imports, the props (`corner`, `onRetry`), the band, the
  actions row and state C, and leaves the three sections and the chat (`:375-438` at the D4 base) INSIDE its fragment
  untouched; Task 5 rebuilds exactly that fragment's tail. Task 4's `toDevice` doc says "An ANCHOR for `lobbyRow`'s
  reason"; Task 5 deletes `lobbyRow` and makes it "An ANCHOR (R-14)".
- **Task 4 → Task 6: `serverCorner`.** Task 4 builds the identity corner INLINE in `lobbyScreen`
  (`corner={signedIn && door.me ? <IdentityCorner me={door.me} onChange={signOut} /> : undefined}`); Task 6 HOISTS it
  to `const serverCorner = …` above `lobbyScreen`, points `lobbyScreen` and the table form at it, and Task 7 points the
  open table and the unlock face at it. One statement of it, never two.
- **Task 4 → Task 9: `signal.ts`'s `setLink(lost())`** at the own-close site (`:431` at the D4 base) is the line Task
  9's battery-37 mutant moves ABOVE `if (mine.signal.aborted) return;`; the expected red reads
  `expected 'reconnecting' to be 'live'`.
- **Task 5 → Task 8: "Being played".** Task 5 renames the lobby's third section and `Help.tsx`'s two mentions
  (`:184`, `:234` at the D4 base); Task 8's Help cells assert `toContain("Being played")` and its whole-panel cell
  asserts no `\bwatch` anywhere — Task 8 checks Task 5 did the rename and edits only if it did not.
- **Task 6 → Task 7: `nameThisBrowser`, `.band-screen`, `.form-field`, `.quiet-link`, `level: 3`.** Task 6 defines
  them; Task 7 uses them. Task 6 KEEPS both `.field.seat-row` rules (the open table still draws them until Task 7);
  Task 7 deletes them.
- **Task 6 → Task 8: `Help.tsx`.** Task 6 deletes the "(It is not the display name on the form below…)" sentence and
  the assertion Task 4 rewrote for it; Task 8 replaces the "Watching a game" section whole. Both edit the same file:
  find by text.
- **Task 7 → Task 8: `LockScreen`'s standalone shape** (band + `.notice-card`) is Task 7's; Task 8 adds `leaveLabel`
  to it and changes the leave button's text to `{leaveLabel}`. Task 7 leaves the word "Lobby" there.
- **Task 8 → Task 9: `App.tsx`'s `sendToArchive` catch and `Abandon.tsx`'s `sendFirst` catch** are Task 9's sites;
  Task 8's `App.tsx` edits sit elsewhere (`open`'s catch, the `broken` screen, `localExit`, the Game menu). Both tasks
  edit `App.tsx`: Task 9 finds by content.
- **Task 1 → nobody.** It touches one Rust test and moves `browser_played_pin` 3 → 4; every later task's Rust gate
  reads 4 there.

---

### Task 1: The parked witness — `ClientState.warnings` at the FIELD (cycle A's R-A19; handover §5 and §7; spec §2.6 "The version drift line is filtered", review R2-m2)

**Files:**
- Modify: `crates/yokohama-web/tests/browser_played_pin.rs` — one new cell, beside
  `a_record_from_another_release_opens_with_no_warning_and_keeps_its_version` (`:146-194`), which it mirrors.

**Interfaces:**
- Consumes: the file's own helpers — `host_with_the_pair() -> (Games, String)` (`:59`), `HOST` (`:54`), `LOCAL_USER`
  (`:47`), `dispatch`, `ApiRequest` (from `yokohama_web::api`), `Doc` and the `Store` trait
  (`yokohama_web::store`), `User` (`yokohama_web::identity`); `yokohama_core::{GameRecord, Rules}`.
- Produces: nothing another task reads. `cargo test -p yokohama-web --test browser_played_pin` reads **4**, and stays
  4 through the cycle.

**Why it exists** (cycle A's `final-rereview.md`, "Important 1, precisely"): the fix wave's cell pins the FILTER
(`without_foreign_conversion` drops the conversion line and nothing else), and a `warnings: Vec::new()` forced at the
field's initializer (`crates/yokohama-web/src/client.rs:538`) still ships GREEN through lib 320, pin 3 and
`wire_fixtures` 13. The reviewer measured the smallest closer: a `rules_hash` derived to differ makes
`Session::from_record` warn exactly once, and that line must reach the wire. A `rules_hash` line, not an
`engine_version` one, because the version line is what the second filter drops by design.

- [ ] **Step 0: The baseline** (ORCHESTRATOR or the implementer, recorded in `progress.md` before any edit). Confirm the
  branch and a clean tree (Global Constraints), then read and record: `cargo test -p yokohama-web` (lib, pin, gate,
  `wire_fixtures`), `cargo test -p yokohama-wasm`, `cargo test -p yokohama-worker`; in `web/`, `npm test` (cells and
  files), `npm run test:wasm`; in `worker/`, `npm run test:scripts` and the pool; the two fence commands printing
  nothing. Handover §1 has the expected figures (web 338 = 320 + 3 + 2 + 13; wasm 7; worker 64; `npm test` 1067 in
  56; `test:wasm` 95 in 7; scripts 352; pool 197). A different reading is recorded, not argued with.

- [ ] **Step 1: The failing cell.** In `crates/yokohama-web/tests/browser_played_pin.rs`, directly below
  `a_record_from_another_release_opens_with_no_warning_and_keeps_its_version`:

```rust
/// **A CLI warning reaches the wire** — the field-level witness for
/// `ClientState.warnings`, which the two filter cells cannot give: a chain
/// that emptied every game's warnings passes them and fails here. A
/// `rules_hash` derived to differ from the running one makes
/// `Session::from_record` warn exactly once, and the game screen's payload
/// carries that line. (Cycle A's R-A19, closed here as cycle B's first task.)
#[test]
fn a_rules_hash_drift_reaches_the_wire_as_a_warning() {
    use yokohama_web::store::Store;
    let (games, id) = host_with_the_pair();
    let store: Arc<dyn Store> = Arc::clone(games.store());
    let (json, version) = store.read(Doc::Record, &id).expect("a readable store").expect("the pair's record");
    let mut record: yokohama_core::GameRecord = serde_json::from_str(&json).expect("the committed record parses");
    let running = yokohama_core::Rules::standard().content_hash();
    record.rules_hash = format!("{running}-another");
    let rewritten = serde_json::to_string_pretty(&record).expect("a record serializes");
    store.write_if_version(Doc::Record, &id, &rewritten, Some(version)).expect("the rewrite lands");
    let user = User::parse(LOCAL_USER).expect("the local address parses");
    let res = dispatch(
        &games,
        &HOST,
        Ok(&user),
        &ApiRequest { method: "GET", path: &format!("/api/games/{id}"), query: "", content_type: None, body: b"" },
    );
    assert_eq!(res.status, 200, "{}", res.body);
    let state: serde_json::Value = serde_json::from_str(&res.body).expect("a ClientState");
    let warnings = state["warnings"].as_array().expect("an array");
    assert_eq!(warnings.len(), 1, "exactly the rules_hash line: {warnings:?}");
    assert!(
        warnings[0].as_str().unwrap().starts_with("warning: GameRecord.rules_hash="),
        "the CLI's own line reaches the wire: {warnings:?}",
    );
}
```

  If `games.store()`, `Store::read`'s tuple shape or `write_if_version`'s signature differ at HEAD from the reviewer's
  reading, follow the sibling cell's idiom at `:146-194` (it reads and rewrites the same pair) — the CONTRACT is the
  two assertions, not the plumbing.

- [ ] **Step 2: Watch it fail under the survivor, and pass at HEAD.** First the mutant, in your own tree (an
  implementer may mutate its own tree; only a REVIEWER's mutants must run on an export): in
  `crates/yokohama-web/src/client.rs:538`, replace the `warnings:` expression with `warnings: Vec::new(),`. Then:

```bash
cargo test -p yokohama-web --test browser_played_pin a_rules_hash_drift
```

  Expected: FAIL, `exactly the rules_hash line: []`. Quote it. Then `git checkout -- crates/yokohama-web/src/client.rs`
  and confirm `git status --short` names ONLY the test file. Run the command again: PASS. Then the whole binary:

```bash
cargo test -p yokohama-web --test browser_played_pin
```

  Expected: **4 passed** (3 → 4).

- [ ] **Step 3: The per-edit gates.** `cargo test -p yokohama-web` (+1, in the pin binary; lib 320, gate 2,
  `wire_fixtures` 13 unmoved), `cargo clippy --workspace --all-targets -- -D warnings`, and BOTH fence commands
  printing nothing (the second: `crates/yokohama-web/src` is untouched — the mutant was reverted). The `web/` and
  `worker/` suites are unaffected and run anyway: no count moves.

- [ ] **Step 4: Commit.**

```bash
git add crates/yokohama-web/tests/browser_played_pin.rs
git commit -m "test(web): a rules_hash drift reaches the wire as a warning — the field-level witness for ClientState.warnings (R-A19)"
```

  The body quotes Step 2's red line and names the mutant (`client.rs:538`, `warnings: Vec::new()`).

---

### Task 2: The shared pieces (spec §2.2 "The shared parts"; R-6.5c2.18; mocks 01, 02, 08, 09, 10, 11) — from D4 retired Task 9

**Files:**
- Create: `web/src/BoardMark.tsx`, `web/src/Band.tsx`, `web/src/Chips.tsx`, `web/src/SeatCard.tsx`.
- Create: `web/src/BoardMark.test.tsx`, `web/src/Band.test.tsx`, `web/src/Chips.test.tsx`, `web/src/SeatCard.test.tsx`.
- Modify: `web/src/styles.css` — two ground tokens at the end of `:root` (after `--wide-card-aspect`, `:124` at
  base), and a new block at the END of the file, "M6.5c2: the setup screens' shared parts".

**Interfaces:**
- Consumes: `Me` (`types.ts`; `mode` is `"open" | "cf-access" | "worker"` since cycle A); `displayName`,
  `setDisplayName`, `clearDisplayName` (`api.ts:173-201`); `seatClass` (`Cards.tsx:25`); `PlayerColor` (`types.ts`);
  `wire.me`, `wire.meSignedOut` (the corpus's open-door payloads; the Access door is `wire.me` with `mode` moved —
  `App.test.tsx`'s own precedent).
- Produces (verbatim, the shared-names table): `<BoardMark width />` + `TILES`; `<LobbyBand crumbs corner />`;
  `<IdentityCorner me onChange />`; `<SeatChip color label open? toMove? />`; `<PasswordTag />`; `<Padlock size? />`;
  `<VersionTag label />`; `<SeatCard color caption who note? action? open? />`; the tokens `--ground-glow`,
  `--ground-deep`; the classes `.raised-card`, `.card-band` (`.yen`/`.vp`/`.warn`), `.card-in`, `.card-cap`, `.lede`
  under a card, `button.primary.yen`, `a.primary-link`, `.board-mark`/`.bm-*`, `.lobby-band`/`.band-*`/`.crumb*`,
  `.identity-*`, `.seat-chip`/`.seat-dot`, `.password-tag`/`.padlock`, `.version-tag`, `.seat-card*`.
- **Nothing renders these yet**: Task 3 is the first screen that does. So no e2e, and no headed look here; the
  reviewer reads the components against the mocks' HTML (`…/mocks-html/door-v3-lobby-v2-name.html`: the board SVG,
  the chips, the password tag; `phone.html`: the pill).

**The design plan** (frontend-design; the mocks pin the direction, so this is a translation into the tokens, not a new
look): the `:root` grounds and inks; the seat colours on seats; `--yen` for "yours / to move / the bot and this
device"; `--vp` for the server; `--warn` for the password. **The one addition is two GROUND shades** for the hero's and
the band's navy glow, defined as tokens with a comment (D4 measured that no `color-mix` of `--vp` or `--blue` into
`--bg` reproduces the approved stops — the per-channel fractions disagree). Fraunces for the product name, titles,
crumbs and row names; JetBrains Mono for everything read as data; the mocks' small-caps captions kept.

**Rules the corner encodes** (spec §2.1 "The client"; D4 amendment 2): under `open` the name IS the identity, so the
one action is **switch player** — it clears the display name and calls `onChange` (App passes `signOut`, which drops
the claimed name and re-asks `/api/me`); under an edge door the identity is Access's, so the one action is **change
name**, which edits the display name and calls NOTHING (a `signOut` there would sign the player out). The corner shows
`me.user`'s answer, never what this browser stores under `yokohama.claimed_user`. The 64-character cap on the edit
field is the server's own seat-name rule (`tables.rs`'s `check_name`).

- [ ] **Step 1: Load the design skill and read the mocks.** Invoke `frontend-design:frontend-design`. Read mocks 01,
  02, 08, 09, 10 and 11 as images, and the two HTML sources named above.

- [ ] **Step 2: The failing cells** — the four test files exactly as D4 writes them,
  `drafts/D4-tasks-09-12.md:144-447` (`BoardMark.test.tsx` 5 cells: the 4/3/2/1 rows, the half-tile offset, two
  presidents and five cubes in two seats' colours, decoration with no `<img>`/`<image>`, every trade class coloured by
  a rule in `styles.css` read from source; `Band.test.tsx` 7 cells: the band's `h1`/last-crumb `h2`/`aria-hidden`
  separators/`header.lobby-band` not `header.banner`, switch player under `open`, the identity shown when no display
  name, change name behind Access editing only the display name and calling nothing, the emptied field clearing it
  and cancel writing nothing, the pill's disclosure over ONE menu, nothing drawn for `me.user === null`;
  `Chips.test.tsx` 5 cells: the dot named for the colour with the chip's text the label alone, open and to-move
  classes, the drawn padlock named "password required" with no U+1F512, the padlock `aria-hidden`, the version tag's
  title; `SeatCard.test.tsx` 2 cells: a region named by its caption with the seat class, an open card dashed with its
  action inside). The test person in `Band.test.tsx` is `"  Captain Red "` → keep; nothing here names a surname.

```bash
cd web && npx vitest run src/BoardMark.test.tsx src/Band.test.tsx src/Chips.test.tsx src/SeatCard.test.tsx
```

  Expected: all four files fail to load — `Failed to resolve import "./BoardMark" from "src/BoardMark.test.tsx"` (and
  `./Band`, `./Chips`, `./SeatCard`).

- [ ] **Step 3: The four components** — `drafts/D4-tasks-09-12.md:460-902` (`BoardMark.tsx` :460-576: the ten `TILES`
  with their trades and alphas, `viewBox 0 0 420 296`, `INDENT = [0, 54, 108, 162]`, two presidents, five cubes;
  `Chips.tsx` :578-655; `SeatCard.tsx` :657-703; `Band.tsx` :705-902: `ICON_TILES` from `web/public/icons/icon.svg`'s
  geometry, `LobbyBand`, `IdentityCorner` with its `useId` menu and the edit form). Take them as written.

- [ ] **Step 4: The stylesheet** — the two tokens and the appended block, `drafts/D4-tasks-09-12.md:907-1082`. Take it
  as written; it is scoped so that every rule names a class this block introduces.

- [ ] **Step 5: Watch them pass, then the whole suite.**

```bash
cd web && npx vitest run src/BoardMark.test.tsx src/Band.test.tsx src/Chips.test.tsx src/SeatCard.test.tsx
cd web && npm test && npm run typecheck
```

  Expected: the four files green — **5 + 7 + 5 + 2 = 19 new cells** (D4) — and `npm test` up by exactly +19 cells and
  +4 files over Step 0's reading.

- [ ] **Step 6: No existing element was restyled.** Print the new block's selectors and READ them:

```bash
cd web && sed -n '/M6.5c2: the setup screens. shared parts/,$p' src/styles.css | grep '{' | sed 's/ *{.*//'
```

  Expected: every selector names at least one class this block introduces, or is `button.primary.yen` (a new
  compound: no element carries both classes before Task 3), or is the `:focus-visible` rule scoped under
  `.front-door` / `.lobby`. A selector made only of pre-existing names (`.ghost`, `.open`, `.seat-red`, `.chip`…)
  would restyle the game screen: a FINDING. Paste the list in the report.

- [ ] **Step 7: The per-edit gates** (Global Constraints): `npm run typecheck`, `npm test` (+19 / +4 files),
  `npm run test:wasm` (unmoved); the Rust and `worker/` gates unmoved; both fences empty. **No e2e** (nothing draws
  these yet; Step 6 shows no existing element restyled — the one rule that reaches today's screens is the focus outline,
  which moves no layout and which no e2e cell reads).

- [ ] **Step 8: Commit.**

```bash
git add web/src/BoardMark.tsx web/src/BoardMark.test.tsx web/src/Band.tsx web/src/Band.test.tsx \
  web/src/Chips.tsx web/src/Chips.test.tsx web/src/SeatCard.tsx web/src/SeatCard.test.tsx web/src/styles.css
git commit -m "feat(web): the setup screens' shared pieces — the board mark, the band and its identity corner, the chips and the seat card"
```

---

### Task 3: The front door and the project link (spec §2.2 "The door", §2.3; R-6.5c2.6, R-6.5c2.4 state E; mocks 01, 06, 10, 11) — from D4 retired Task 10

**Files:**
- Create: `web/scripts/project-url.mjs`, `web/scripts/project-url.d.mts`, `web/scripts/project-url.node.test.ts`;
  `web/src/projectLink.ts`, `web/src/projectLink.test.ts`; `web/e2e/setup_screens.spec.ts`.
- Modify: `web/vite.config.ts` (import `checkProjectUrl`, the `define`, a header paragraph); `web/src/vite-env.d.ts`
  (declare `__YOKOHAMA_PROJECT_URL__`); `web/vitest.node.config.ts:36` and `web/tsconfig.node.json`'s `include` (the
  node project takes `scripts/**/*.node.test.ts`).
- Modify: `web/src/FrontDoor.tsx` — rewritten whole (160 lines); `web/src/OnThisDevice.tsx` — the header, the imports,
  the signature (`readiness?`) and the render; the readiness line and `oneStop`/`Readiness` MOVE in from
  `FrontDoor.tsx:108-160`; `InstallOffer`'s available arm reads "Install app" (the iOS instruction arm is UNCHANGED:
  WebKit has no programmatic install, spec §10.5); `StorageLine`, `Outbox`, `OutboxRow`, `Remedy` untouched.
- Modify: `web/src/BackupControls.tsx` (`BackUpEverything` takes `label`, its NAME stays "Back up everything";
  `Restore`'s picker goes out of sight behind the word "Restore", its input's name stays "Restore from a backup");
  `web/src/tables.ts` (+ `deviceWord`); `web/src/Help.tsx` (`shots` prop; the five `<Shot>`s wrapped in
  `{shots && …}`; the door's sentence names the three cards); `web/src/App.tsx` (`doorScreen` passes
  `link={projectLink()}`); `web/src/styles.css` (a door block appended; the superseded door rules deleted by selector);
  `deploy/Dockerfile` (`ARG YOKOHAMA_PROJECT_URL` directly above `RUN npm run build:client`, `:172`).
- Test: `web/src/FrontDoor.test.tsx` (the first describe rewritten, 11 cells; the readiness describe gets a
  `stubWorker` `beforeEach`), `web/src/OnThisDevice.test.tsx` (one cell out, one query renamed, one added),
  `web/src/Help.test.tsx` (one reworded, one added), `web/src/tables.test.ts` (+2), `web/src/App.test.tsx` (five door
  strings), `web/e2e/harness.ts` (`createLocal`, `:399`: the button is "New game"), `web/e2e/plays_locally.spec.ts`
  (`:439`, `:795`, `:814-817`, `:1075`: "Play the bot here" → "New game"; "Play with others" → "Go to the lobby").

**Interfaces:**
- Consumes: Task 2's `BoardMark`, `VersionTag`, `.raised-card`/`.card-band`/`.card-cap`/`.lede`/`button.primary.yen`/
  `a.primary-link`; `production_bot.json` imported directly; `TableView.bot`; `ReadinessStatus` (`swRegister.ts:42`);
  `Refusal` (takes `className`); `navigate` (`route.ts`); `stubWorker`, `resetLocalHostForTests`.
- Produces: `checkProjectUrl`, `PROJECT_URL_VAR`, `PROJECT_URL_MAX`; `linkFor`, `projectLink`, `type ProjectLink`;
  `FrontDoor`'s `link` prop; `OnThisDevice`'s `readiness` prop; `BackUpEverything`'s `label`; `Help`'s `shots`;
  `deviceWord(t: TableView): string` ("vs V4 · your move", "vs V4 · you won", "vs V4 · choose a card", …; no "vs"
  where the listing carries no `bot`); `setup_screens.spec.ts` with `noSidewaysScroll`, `look`, `images`, `height`,
  `DESKTOP`, `PHONE`. The door's DOM: `.front-door > header.door-hero` (the `h1` "Yokohama" and `.project-link`),
  `main.door-main > .door-cards > section.door-card` ×3 (`.bot-card`, `.friends-card`, `.on-this-device`), `.two-up`
  under state E, `.door-foot` with the "How to play" disclosure, `section.door-help` holding `<Help shots={false} />`.

**What the door is** (mock 01; phone 11): the title, the optional project link, the drawn board; three raised cards
in the mock's order — the bot (yen band, "2 PLAYERS", the version tag in the heading, "New game"), with friends (vp
band, "2 TO 4 PLAYERS", `<a class="primary-link" href="/enter">Go to the lobby</a>` — an `<a>` and never a fetch,
because Access answers a navigation with a 302 to another origin), on this device (the rows as `deviceWord`, the
install offer, ONE foot line: readiness · Back up · Restore, then "Will not open here"); "How to play" opens the help
IN PLACE without its five screenshots (spec §10.5: they are photographs of the card art). State E
(`VITE_MULTIPLAYER=false`, mock 06) draws no friends card and the other two widen (`.two-up`). The door asks the
server NOTHING (R-6.5b2.7): a cell holds `fetch` to the setup file's throwing stub; the browser cell reads every
request the page made.

- [ ] **Step 1: Load the design skill; read the mocks.** `frontend-design:frontend-design`; mocks 01, 06, 10, 11;
  `door-v3-lobby-v2-name.html` §1 and `phone.html`'s door for the spacing.

- [ ] **Step 2: The build variable's cells, failing** — `drafts/D4-tasks-09-12.md:1172-1226`: the node project's
  `include` gains `"scripts/**/*.node.test.ts"` (both files, with the one comment bullet), then
  `web/scripts/project-url.node.test.ts` (8 cells: unset and EMPTY read as no link — a Docker `ARG` nobody passed
  reaches the build as `""`; `https`/`http` pass through unchanged; six refusals each naming the variable: no scheme,
  another scheme, a space, a control character, over 200 characters, no host).

```bash
cd web && npx vitest run -c vitest.node.config.ts scripts/ && npx tsc -p tsconfig.node.json
```

  Expected: `Failed to load url ./project-url.mjs`; `TS2307: Cannot find module './project-url.mjs'`.

- [ ] **Step 3: The check and its declaration** — `drafts/D4-tasks-09-12.md:1238-1293` (`project-url.mjs` and
  `project-url.d.mts` as written: TypeScript resolves `./project-url.mjs` to the `.d.mts` under
  `moduleResolution: "bundler"`, measured). Re-run Step 2's command: **8 passed**, `tsc` clean.

- [ ] **Step 4: The door's reader, failing then green** — `drafts/D4-tasks-09-12.md:1301-1412`: `projectLink.test.ts`
  (2 cells; **every cell stubs the global**: under vitest 4.1.11 a Vite `define` is a runtime GLOBAL, so
  `vi.stubGlobal("__YOKOHAMA_PROJECT_URL__", …)` reaches it and a `YOKOHAMA_PROJECT_URL` exported in the shell would
  otherwise leak into the run — D4 measured both); then `vite.config.ts`'s import, header paragraph, `const projectUrl
  = checkProjectUrl(process.env.YOKOHAMA_PROJECT_URL);` and `define: { __YOKOHAMA_PROJECT_URL__:
  JSON.stringify(projectUrl) }`; `vite-env.d.ts`'s `declare const __YOKOHAMA_PROJECT_URL__: string | null;`;
  `projectLink.ts`. Expected: 2 passed; `npx tsc -b --noEmit && npx tsc -p tsconfig.node.json` clean.

- [ ] **Step 5: The door's cells, failing** — `drafts/D4-tasks-09-12.md:1414-1717`:
  - `FrontDoor.test.tsx`: the header, the imports and the first describe REPLACED (11 cells: the turnstile link by
    `href`; the product named in `.door-hero` and not `header.banner`; the three card titles in the mock's order with
    the version tag in the first; state E's `.two-up` with two cards; the device rows drawn with `fetch` never called;
    New game opens `#/new/local`; the version tag from the contract file; the notice; the project link only when the
    build names one, `target="_blank"`, `rel="noopener noreferrer"`; "How to play" opens a region with the help's
    words and zero `img, figure.shot`, focuses its heading, and Close removes it; `multiplayerOffered()` reads the
    build value — `"false"` alone turns it off); the readiness describe's `beforeEach`/`afterEach` (`stubWorker`,
    `resetLocalHostForTests`), because the line is the device card's now.
  - `OnThisDevice.test.tsx`: DELETE "takes you to the local form, which needs no door" (the button moved to the bot
    card); "Install app" in the prompt cell; ADD "names each game's bot and what it is waiting on, and draws no per-row
    backup" (a corpus `TableView` with seat 0 made `{kind: "bot"}`, `to_move: 1`, `bot: {label: productionBot.label,
    runs_here: true}` — the ONE-field-moved rule, stated in the cell; expects `vs V4 · your move`, no per-row "Back
    up", the foot's "Back up everything" reading "Back up").
  - `Help.test.tsx`: "names the door's ways on…" reworded (contains "You against the bot", "Play with friends", "On
    this device", "link back to the door"; NOT "Play with others", "Play on this device", "Sign in first"); ADD "draws
    its screenshots unless its host asks for the words alone" (5 `figure.shot` by default; 0 with `shots={false}`).
  - `tables.test.ts`: the `deviceWord` describe (2 cells, the six states and the no-`bot` case).
  - `App.test.tsx`: "Play with others" → "Go to the lobby" (2 sites, `href` stays `/enter`); "Play the bot here" →
    "New game" (3 sites).

```bash
cd web && npx vitest run src/FrontDoor.test.tsx src/OnThisDevice.test.tsx src/Help.test.tsx src/tables.test.ts src/App.test.tsx
```

  Expected: red at the named queries (no "Go to the lobby", no `.door-hero`, no "How to play"; no `vs …` row, no
  "Install app"; "Play with others" still present and 5 figures under `shots={false}`; `deviceWord is not a function`;
  the five App door queries).

- [ ] **Step 6: The words, the help, the backup controls, the door, the device card, App** —
  `drafts/D4-tasks-09-12.md:1719-2158`: `tables.ts`'s `deviceWord` (:1721-1742); `Help.tsx`'s `shots` prop, the
  wrapped `<Shot>`s, the door's sentence (:1744-1764); `BackupControls.tsx`'s `BackUpEverything` label and `Restore`'s
  hidden picker (:1766-1816); `FrontDoor.tsx` whole (:1818-1992); `OnThisDevice.tsx`'s header, imports, signature,
  render, `DeviceRow`, the moved `oneStop`/`Readiness` (root `div.offline-state`, the ready line "Ready to play
  offline" with no period), `InstallOffer`'s available arm (:1994-2139); `App.tsx`'s `projectLink` import and
  `doorScreen`'s `link` (:2141-2158).

```bash
cd web && npx vitest run src/FrontDoor.test.tsx src/OnThisDevice.test.tsx src/Help.test.tsx src/tables.test.ts src/App.test.tsx src/projectLink.test.ts
cd web && npm test && npm run typecheck
```

  Expected: green; `npm test` **+9 cells, +1 file** over Task 2's reading (FrontDoor +4, OnThisDevice ±0, Help +1,
  tables +2, projectLink +2 new) — D4's figure; re-count and state both.

- [ ] **Step 7: The door's stylesheet** — `drafts/D4-tasks-09-12.md:2171-2283`: the appended door block (the hero's
  radial glow on the two ground tokens, the 76rem column, the cards rising into the hero's foot by `.door-main`'s
  negative margin, the device card's rows and its ONE-line grid foot, the help in place, one column below 900px, the
  phone rules: board above the title, full-width 44px primaries); and DELETE by selector the superseded door rules —
  the "the front door (M6.5b2, R-6.5b2.1)" block (`.front-door .with-others …`, `.door-banner …`, `.front-door
  .offline-state …`), the `.front-door .saved li a.row-link` halves (keep the `.lobby` halves for Task 5),
  `.on-this-device { margin-top }`, and `.restore .restore-field` + its `input[type="file"]` rule. Re-run `npm test`.

- [ ] **Step 8: The build's two readings, and the image's ARG.**

```bash
cd web && D=$(mktemp -d) && YOKOHAMA_PROJECT_URL="ftp://example.test/x" npx vite build --outDir "$D"; echo "exit $?"
cd web && D=$(mktemp -d) && YOKOHAMA_PROJECT_URL="https://example.test/project" npx vite build --outDir "$D" && grep -c "example.test/project" "$D"/assets/*.js && ! grep -l "__YOKOHAMA_PROJECT_URL__" "$D"/assets/*.js
```

  Expected: the first prints `Error: YOKOHAMA_PROJECT_URL must begin with https:// or http://: "ftp://example.test/x"`
  and `exit 1` (D4 measured); the second builds, one bundle counts the URL, no bundle keeps the identifier. Both write
  to a temp directory: `web/dist` is untouched. Then `deploy/Dockerfile`: the five-line comment and `ARG
  YOKOHAMA_PROJECT_URL` directly above `RUN npm run build:client` (`drafts/D4-tasks-09-12.md:2297-2306`). The image
  is not built this cycle (cycle C's); the line is what makes spec §2.3's `--build-arg` reach the build.

- [ ] **Step 9: The browser walk** — `drafts/D4-tasks-09-12.md:2308-2426`: `harness.ts:399` (`createLocal` clicks
  "New game", `exact: true`); `plays_locally.spec.ts`'s four sites; then CREATE `web/e2e/setup_screens.spec.ts` as
  written (:2315-2426): `test` from `./sw`, the skip line, `DESKTOP`/`PHONE`, the four helpers, and the door's cell —
  at BOTH widths: the title visible, three cards, the turnstile `href`, `noSidewaysScroll`, zero `<img>`; at 1440 one
  row of cards left-to-right and the board RIGHT of the title; at 390 one column top-to-bottom, the board ABOVE the
  title, "New game" and "Go to the lobby" ≥ 44px; `look(page, "door-1440")` / `"door-390"`; then the help opened at
  1440 with zero images and `look("door-help-1440")`; and **every request the page made**: nothing under `/api/`.

```bash
cd web && npm run e2e > ../.superpowers/sdd/2026-09-27-web-frontend-m6.5c2b-plan/task-3-e2e.txt 2>&1 &
```

  POLL the file (`tail -3` every minute or two) until the summary line; never a Monitor. Expected: green, **one cell
  and one file larger** than Step 0's reading (61 in 11 → 62 in 12). A red elsewhere is read before it is believed:
  the walks that create a local game go through `createLocal`, which this step changed.

- [ ] **Step 10: The headed look.** Copy `web/test-results/*/door-1440.png`, `door-390.png` and `door-help-1440.png`
  into the ledger's `task-3-look/`; read them (the Read tool renders a PNG) beside mocks 01 and 11 — the title's size
  and weight, the board's place and scale, the cards' overlap of the hero's foot, the device card's rows and its
  one-line foot, the full-width buttons at 390 — and correct the CSS until the three read as the mock does; re-run the
  spec after any CSS change. State E (mock 06) needs a `VITE_MULTIPLAYER=false` build and is carried by the jsdom
  cell's structure: say so in the report. The report lists the three PNG paths and what differs.

- [ ] **Step 11: The per-edit gates.** `npm run typecheck`; `npm test` (+9 / +1 file); **`npm run test:wasm` +8 cells /
  +1 file** (the build variable's node cells); the e2e (Step 9); the Rust and `worker/` gates unmoved; both fences
  empty.

- [ ] **Step 12: Commit.**

```bash
git add web/scripts/project-url.mjs web/scripts/project-url.d.mts web/scripts/project-url.node.test.ts \
  web/vitest.node.config.ts web/tsconfig.node.json web/vite.config.ts web/src/vite-env.d.ts \
  web/src/projectLink.ts web/src/projectLink.test.ts web/src/FrontDoor.tsx web/src/FrontDoor.test.tsx \
  web/src/OnThisDevice.tsx web/src/OnThisDevice.test.tsx web/src/BackupControls.tsx web/src/tables.ts \
  web/src/tables.test.ts web/src/Help.tsx web/src/Help.test.tsx web/src/App.tsx web/src/App.test.tsx \
  web/src/styles.css web/e2e/harness.ts web/e2e/plays_locally.spec.ts web/e2e/setup_screens.spec.ts deploy/Dockerfile
git commit -m "feat(web): the front door rebuilt — the hero and its board, three cards, the bot's version, the project link as a checked build value"
```

---

### Task 4: The identity states, the name step and the lobby's band (spec §2.1 "The client", R-6.5c2.3–.5, R-N3; mocks 03, 04, 05, 11; Review Focus 1 and 3) — from D4 retired Task 11

**Files:**
- Create: `web/src/nameRule.ts`, `web/src/nameRule.test.ts`, `web/src/Door.test.tsx`.
- Modify: `web/src/SignIn.tsx` — rewritten whole as the name step; `web/src/Door.tsx` — rewritten whole (states A–D,
  the refusal card, B's operator fold); `web/src/whoami.ts` (`DoorFailure`, `failureOf`, `Door.failure`, `useMe` keeps
  the failure's kind, `doorSentence` becomes the operator's words); `web/src/signal.ts` (`Link` gains `"connecting"`;
  a `connected` flag and `lost()` inside the effect; the six `setLink` sites); `web/src/Lobby.tsx` (imports; props
  `who` → `corner`, `+ onRetry`; the name field's state DELETED; the band, the actions row and state C — **the three
  lists and the chat stay inside the fragment untouched**, Task 5's); `web/src/App.tsx` (`IdentityCorner` import;
  `healthTick` and `retry`; the health effect re-runs on the tick; `doorContent` hands `serverUp` and `retry` to
  `Door`; `lobbyScreen` passes `corner` and `onRetry`); `web/src/Help.tsx` (the "who you are" clauses of "Starting a
  game" and the Talking section's name); `web/src/styles.css` (a block appended; the superseded lobby and sign-in rules
  deleted by selector).
- Test: `web/src/SignIn.test.tsx` (rewritten, 7 cells), `web/src/whoami.test.ts` (+2 and one assertion),
  `web/src/signal.test.ts` (+1; eight "reconnecting" expectations in six cells become "connecting"),
  `web/src/Lobby.test.tsx` (the name-field cell deleted; the back-link cells reworded; the C cell rewritten; +1 band
  cell), `web/src/App.test.tsx` (the door strings; +2: Review Focus 3's stale-name cell and switch player),
  `web/src/Help.test.tsx` (two cells reworded), `web/e2e/harness.ts` (`signIn` types into the name step),
  `web/e2e/plays_locally.spec.ts` (`:821-823`: the C heading), `web/e2e/joins_by_link.spec.ts` (`:91-177`: the
  who-strip block becomes the corner's phone reading), `web/e2e/setup_screens.spec.ts` (+ the name-step cell).

**Interfaces:**
- Consumes: `identity_cases.json` via `wire.identityCases` (41 rows; the `display` rule: the input trimmed with runs
  of U+0020 collapsed, case KEPT, for an accepted plain name; the trimmed input, case kept, for an accepted email;
  `null` where `key` is `null` — VERIFIED at `f2a3dd61`: `"  Ben   Castle "` → `Ben Castle` / `ben castle`); `Me.mode`;
  Task 2's `LobbyBand`, `IdentityCorner`; `ACCESS_CHALLENGE`, `isSessionExpired`, `ApiError`, `setClaimedUser`,
  `setDisplayName`, `CLAIMED_USER_KEY`, `DISPLAY_NAME_KEY` (`api.ts`); `wire.error` (a refusal body).
- Produces: `normalizeName`, `NAME_MAX`, `NameVerdict`; `DoorFailure`, `failureOf`, `Door.failure`;
  `Door({door, serverUp, onSignedIn, onRetry})`, `Unreachable({onRetry})`; `Link` with `"connecting"`; `Lobby`'s
  `corner` and `onRetry`; App's `retry`. The states' DOM: `section.raised-card.state-card` with a card band (`vp` for
  A, `warn` for B and the refusal, plain for C), the name step `section.raised-card.state-card.name-step` (heading
  "Choose a name", field labelled "Name", button "Continue", the hint "Letters, numbers, spaces, dots, dashes and
  underscores. Up to 32 characters.", the `.fine` trust sentence), state D `div.connecting-state` with
  `role="status"` "Connecting to the server…" over `.skeleton-rows` (3), B's `details.operator-note` "For the
  operator". The lobby's frame: `div.lobby > header.lobby-band + div.lobby-body`, `.lobby-actions` (New table, the
  "Games on this device" link — in the band's corner while nobody is signed in — and the `.link-word` status),
  `.lobby-door` around the door's content.

**The discriminators, in the order `Door` tests them, and the order is the rule** (spec §2.1; the spec-gap answer):
**A** `door.failure === "challenge"` (the edge's challenge — FIRST, because the hostname-wide Access application
fronts `/api/health` too, so an expired session fails BOTH reads) → **C** `door.failure === "network"` or
(`serverUp === false && door.me === null`) → the server's own refusal (`door.failure === "refused"`, today's `Refusal`
inside a card) → **D** `door.me === null` → the name step (`mode === "open"`, no user) → **B** (`cf-access` or
`worker`, no user) → nothing (a user). **State D is the DOOR's alone**: a signed-in lobby waiting for its lists has
passed `/api/me` and reads "loading…" over skeleton rows under Your tables (Task 5) — D4 tried drawing D there and it
is a false sentence about a reachable server that breaks fourteen cells.

**The name rule is Rust's, character for character** (D4 measured against rustc): `str::trim` strips the Unicode
`White_Space` property (Rust trims U+0085 and keeps U+FEFF; JavaScript's `trim` the reverse); `to_ascii_lowercase`
folds A–Z only (`toLowerCase` folds the Kelvin sign U+212A into `k`); `char::is_control` is Cc. **R-B2: server
parity** — an `@`-shaped input takes the email half of the grammar exactly as `User::parse` does, so the pin stays
two-way over every row; the hint copy reads the plain-name rule as mock 03 shows.

- [ ] **Step 1: Load the design skill; read the mocks.** `frontend-design:frontend-design`; mocks 03, 04, 05, 11;
  `identity-states.html` and `door-v3-lobby-v2-name.html` §3.

- [ ] **Step 2: The name rule's cells, failing** — `drafts/D4-tasks-09-12.md:2509-2576`, with ONE change: the
  `REQUIRED` list is **copied from the committed table's `raw` column**, not from D4 (D4's list predates cycle A and
  carries `"b3n_m-o.re"`, a surname; A's row is `"b3n_c-a.stle"`). The four cells: every row gets the layer's own
  verdict (`key === null` ⇒ refused; else `{ok: true, key, display}` equal); the table carries every REQUIRED input,
  an accepted address, an accepted plain name and a refusal (anti-vacuity); **(Review Focus 1)** `"  Ben   Castle "`
  → `{key: "ben castle", display: "Ben Castle"}` and `"ben castle"` → itself; the reasons in a player's words (`""` →
  "Type a name to play under."; `".."` → "A name begins with a letter or a digit."; 33 × `a` → "A name is at most 32
  characters."; `"ANONYMOUS"` matches /means nobody/; `"bén"` matches /letters, numbers, spaces/; `"a@b@c"` matches
  /exactly one @/).

  The committed `raw` column at `f2a3dd61` (41): `"red@wire.test"`, `"Red@Wire.TEST"`, `" Red@Wire.TEST "`,
  `"a+b@x.test"`, `"ben castle"`, `"  Ben   Castle "`, `"Ben"`, `"BEN"`, `"b3n_c-a.stle"`, `"a.test"`, `"7 wonders"`,
  `"x"`, 32 × `a`, 33 × `a`, `"."`, `".."`, `". ."`, `"-"`, `".x"`, `"_x"`, `"x."`, `"anonymous"`, `"ANONYMOUS"`,
  `"anonymous1"`, `""`, `"   "`, `"Bén"`, `"ben!"`, `"a/b"`, `"ｂｅｎ"` (full width), `"Kelvin"`, `"a@b@c"`,
  `"@x.test"`, `"a@"`, `"a b@x.test"`, `"ben\t"`, `"ben\tcastle"`, `"ben\r"`, `" Ben"`, `"\u0085ben"`,
  `"﻿ben"`. Write every code point outside ASCII as a `\u` escape, never as the character.

```bash
cd web && npx vitest run src/nameRule.test.ts
```

  Expected: `Failed to resolve import "./nameRule"`.

- [ ] **Step 3: The rule** — `drafts/D4-tasks-09-12.md:2585-2672`, as written (`WHITE` = the `White_Space` code points,
  `IS_CONTROL` = Cc, `asciiLower`, `ADDRESS_MAX_BYTES = 254`, `address()` and `plainName()`). Expected: 4 passed. **A
  red first cell is the two rules disagreeing on a row** — the row's `raw`, the server's key and this file's verdict
  are all in the message; fix it HERE only if this file misreads Rust. If the table's `display` differs from the rule
  stated above, STOP and ask the orchestrator.

- [ ] **Step 4: The door's failure kinds, failing then green** — `drafts/D4-tasks-09-12.md:2682-2794`:
  `whoami.test.ts` (+ `failure === "refused"` in the refusal cell; "says HOW the door failed" over a thrown
  `TypeError` and a 401 with `WWW-Authenticate: Cloudflare-Access`; the `failureOf` describe: the edge's challenge vs
  this API's OWN 401 vs no answer), then `whoami.ts` (`DoorFailure`, `failureOf`, the `failure` state kept beside
  `error`, `doorSentence(mode: Exclude<Me["mode"], "open">)` reading the two operator sentences that name
  `Cf-Access-Authenticated-User-Email` and `X-Yokohama-User`).

- [ ] **Step 5: The door's states and the name step, failing** — `drafts/D4-tasks-09-12.md:2796-2991`: `Door.test.tsx`
  (7 cells: **(Review Focus 3)** A before C — `failure: "challenge"` with `serverUp: false` draws "Your sign-in has
  run out" and NOT "Can't reach the server"; C from the door's own network failure OR from `serverUp === false` with
  `me === null`, "Try again" calls `onRetry`, the platform's `TypeError` text absent, "Games on this device" links to
  `#/`; the server's refusal in its own words inside "The server refused" with no turnstile; D's `role="status"` over
  three skeleton rows; the open door with no name is "Choose a name"; B's card with the turnstile and the closed
  "For the operator" fold naming the header; nothing once the server names the caller) and `SignIn.test.tsx` whole
  (7 cells: **(Review Focus 1, on the screen)** `"  Ben   Castle "` stores `ben castle` under `CLAIMED_USER_KEY`,
  `Ben Castle` under `DISPLAY_NAME_KEY`, and hands up the key; an address keyed as the server keys it; `".."` refused
  with its sentence and nothing stored; an empty field asks rather than refuses; the notice above the form as
  `role="status"`; no status without a notice; the trust sentence and "Up to 32 characters").

```bash
cd web && npx vitest run src/Door.test.tsx src/SignIn.test.tsx
```

  Expected: red — no new cards (today's `Door` draws a `Refusal` line, "connecting…", the old form or the header
  sentence); no field labelled "Name", no "Continue".

- [ ] **Step 6: The name step and the door** — `drafts/D4-tasks-09-12.md:2993-3240`: `SignIn.tsx` whole (stores
  BEFORE the callback, because the callback re-reads `/api/me` and `api.ts` attaches the header from storage) and
  `Door.tsx` whole (`StateCard`, `Unreachable` EXPORTED, `Connecting`, `Door` in the discriminator order). Expected:
  7 + 7 passed; `npx tsc -b --noEmit` FAILS at App's `Door` call site ("Property 'serverUp' is missing") until Step 10.

- [ ] **Step 7: The first connection's word (R-N3), failing then green** — `drafts/D4-tasks-09-12.md:3249-3331`: in
  `signal.test.ts` the eight expectations in six cells that describe a signal that has NEVER connected become
  `"connecting"` (D4 names each by line at its base: `:302/:317/:327`, `:412`, `:737`, `:1176`, `:1201/:1214/:1219`,
  `:1257` — find by content; the four that follow a connection that OPENED stay "reconnecting"), plus the new cell
  "says connecting until its first connection, and reconnecting only once one is lost (R-N3)". Expected red: 7
  failed. Then `signal.ts`: the `Link` doc and type; `let connected = false; const lost = (): Link => (connected ?
  "reconnecting" : "connecting");` after the `controller` declaration inside the effect; `connected = true` at
  `onConnected`'s first line after the `stopped` guard and in the held arm's answer; every `setLink("reconnecting")`
  → `setLink(lost())` (six sites); the `word === "live" ? lost() : word` form at the lost-link site. Expected: green.

- [ ] **Step 8: The lobby's frame, failing then green** — `drafts/D4-tasks-09-12.md:3333-3512`: `Lobby.test.tsx` (the
  `./api` import loses the name key; `show()` renders `corner={<span className="identity-corner">{ME}</span>}`; the
  chat describe retitled and its "offers Your name…" cell DELETED; "carries one way back to the door, beside New table
  and above the lists"; the door-content cell's link found in `header.lobby-band`; the C cell rewritten around the
  heading "Can't reach the server", "Try again" → `retry`, and the listing STILL going out; +1 "draws the band — the
  product, the lobby, and the corner it is handed"). Then `Lobby.tsx`'s imports, the two props with their docs, the
  name field's state deleted, the `toDevice` anchor, and the `return` down to the actions row — **the three sections
  and the chat stay exactly as they are inside the fragment** (Task 5's) — and the two prop docs' one-sentence
  changes. Expected: green.

- [ ] **Step 9: App's cells, failing** — `drafts/D4-tasks-09-12.md:3514-3621`: the `DISPLAY_NAME_KEY` import; the
  door describe's first cell on the name step (heading "Choose a name", label "Name", "Continue", then the corner's
  `[title]` equal to `wire.me.user`); every `{ name: "Sign in" }` heading query → "Choose a name" (ten sites); the
  "not connected" texts → the heading "Can't reach the server" (`:2012`, `:3740`, `:3747`, `:3776-3797`, `:3833`,
  `:3842-3870` at the D4 base — find by content); `/Your session has expired/` → the heading "Your sign-in has run
  out"; B's heading asserted; "Play on this device" → "Games on this device"; and TWO new cells: **the stale stored
  name behind Access** (`CLAIMED_USER_KEY` = `stale@old.test`, `/api/me` answering `{...wire.me, mode: "cf-access"}`,
  `/api/health` `{...wire.health, identity: "cf-access"}`: the corner names `wire.me.user`, offers "change name", no
  "switch player", the stale name nowhere in the body, and the request log shows the stale header WAS sent and
  ignored) and **switch player under the open door** (drops both keys and lands on "Choose a name"). Expected: 3
  failed (the first door cell and the two new ones); `tsc` still fails at the `Door` call site.

- [ ] **Step 10: App wired** — `drafts/D4-tasks-09-12.md:3623-3679`: `import { IdentityCorner } from "./Band";`;
  `healthTick` and `retry` after `serverUp`'s state (`:186`); `}, [channel, healthTick]);` on the health effect
  (`:885` at the D4 base); `doorContent` becomes `signedIn ? null : <Door door={door} serverUp={serverUp}
  onSignedIn={() => refresh()} onRetry={retry} />` — the `unreachable` const goes with it (state C is `Door`'s now);
  `lobbyScreen` passes `corner={signedIn && door.me ? <IdentityCorner me={door.me} onChange={signOut} /> : undefined}`
  and `onRetry={retry}`. **`who` and `signOutButton` stay** for the screens that still draw the "You are …" strip (the
  form until Task 6, the deal, the open table until Task 7, the game).

```bash
cd web && npx vitest run src/App.test.tsx && npx tsc -b --noEmit
```

  Expected: green (D4: 118 = 116 + 2), and the app project clean.

- [ ] **Step 11: The help's words about who you are** — `drafts/D4-tasks-09-12.md:3681-3748`: the "Starting a game"
  paragraph's identity clause ("on a server with no sign-in of its own you choose a name once…; behind Cloudflare
  Access it is your Access sign-in"), the corner paragraph ("playing as", "switch player", "change name", "this browser
  remembers it", "leave it blank and your address is used" — and, until Task 6 removes it, the "(It is not the display
  name on the form below…)" sentence STAYS), the Talking section's "The name in the lobby's corner…"; the two Help
  cells reworded as D4 writes them. Expected: 22 passed (the file's count after Task 3; none added).

- [ ] **Step 12: The stylesheet** — `drafts/D4-tasks-09-12.md:3750-3830`: the appended block (`.lobby-body`'s 72rem
  column, `.lobby-actions`, `.lobby-door`, `.state-card`, `.operator-note`, `.name-step`, `.connecting-state` with its
  pulsing dots under `prefers-reduced-motion`, `.skeleton-rows`, the phone rules); DELETE by selector `.lobby a.to-door`
  + `:hover`, `.lobby > .setup`, the four `.sign-in …` rules, `.who-strip .your-name` + its input. `.who-strip` itself
  and `.door-error` STAY.

- [ ] **Step 13: The browser walk** — `drafts/D4-tasks-09-12.md:3832-3931`: `harness.ts`'s `signIn` (the name step's
  heading, the field "Name", "Continue"; signed in reads EITHER the corner's `[title]` or the "You are …" strip, because
  the other screens still draw the strip); `plays_locally.spec.ts:821-823` → the C heading; `joins_by_link.spec.ts`'s
  who-strip block (`:91-177` at the D4 base) → the corner's phone reading (desktop: "Playing as" and the action
  visible, the pill hidden; 390: the pill visible, the action hidden behind it, no sideways scroll open or closed;
  **the action is "change name" under `LIVE` and "switch player" locally** — this spec runs live in cycle C's close);
  and `setup_screens.spec.ts` gains the name-step cell (**Review Focus 1 against the real server**: at 390 the heading,
  the trust sentence, no sideways scroll, "Continue" ≥ 44px, zero images, `look("name-step-390")`; then
  `"  Ben   Castle "` typed → the pill reads "Ben Castle", the corner's `[title="ben castle"]` attached,
  `localStorage["yokohama.claimed_user"] === "ben castle"`, `look("lobby-signed-in-390")`).

  Run the e2e in the background to `task-4-e2e.txt` and POLL. Expected: green, one cell larger than Task 3's.

- [ ] **Step 14: The headed look.** `name-step-390.png` and `lobby-signed-in-390.png` into `task-4-look/`, read beside
  mocks 03 and 11 (the pill); correct the CSS and re-run. **States A–D are not reachable against the local server
  without faking the network**: take four screenshots by hand in a one-off Playwright `page.route` session in an
  export (never committed) — `/api/me` answered 401 with `WWW-Authenticate: Cloudflare-Access …` (A),
  `{"mode":"cf-access","user":null}` (B), aborted (C), held (D) — into `task-4-look/state-{A,B,C,D}-1440.png`, read
  beside mock 05, and named in the report. D4's own renders of these four are in `…/d4-look/` for comparison.

- [ ] **Step 15: The per-edit gates.** `npm run typecheck`; `npm test` **+17 cells, +2 files** (D4: nameRule +4 new,
  Door +7 new, SignIn +1, whoami +2, signal +1, App +2, Lobby ±0, Help ±0); `test:wasm` unmoved; the e2e; the Rust and
  `worker/` gates unmoved; both fences empty.

- [ ] **Step 16: Commit.**

```bash
git add web/src/nameRule.ts web/src/nameRule.test.ts web/src/SignIn.tsx web/src/SignIn.test.tsx \
  web/src/Door.tsx web/src/Door.test.tsx web/src/whoami.ts web/src/whoami.test.ts web/src/signal.ts \
  web/src/signal.test.ts web/src/Lobby.tsx web/src/Lobby.test.tsx web/src/App.tsx web/src/App.test.tsx \
  web/src/Help.tsx web/src/Help.test.tsx web/src/styles.css web/e2e/harness.ts web/e2e/plays_locally.spec.ts \
  web/e2e/joins_by_link.spec.ts web/e2e/setup_screens.spec.ts
git commit -m "feat(web): the identity states A-D, the open door's name step pinned to User::parse, the lobby's band and corner, and a first connection that reads connecting"
```

---

### Task 5: The lobby's lists (spec §2.2 "The lobby"; R-6.5c2.7; R-6.5c2.10's lobby row; mocks 02, 10, 11) — from D4 retired Task 12

**Files:**
- Modify: `web/src/Lobby.tsx` — the header comment, the imports, `lobbyRow` DELETED (its other caller,
  `OnThisDevice`, draws its own rows since Task 3; `TableFacts` stays for the Game menu and the unlock face), a
  `notMine` filter, a `copied` state, `copyLink`, `row(t, section)`, and the fragment's tail (the three sections and
  the chat) rebuilt as `.lobby-cols > .lobby-lists + .lobby-rail`.
- Modify: `web/src/tables.ts` (+ `Section`, `ago`, `rowMeta`, `seatChips`, `rowAction`, `ChipModel`, `RowAction`);
  `web/src/Help.tsx` (the lists' names and where the chat sits, five sites); `web/src/styles.css` (a block appended;
  the superseded list rules deleted by selector).
- Test: `web/src/Lobby.test.tsx` ("Your games" → "Your tables" and "Games in progress" → "Being played" EVERYWHERE;
  two cells rewritten; +7), `web/src/tables.test.ts` (+4), `web/src/App.test.tsx` ("Your games" → "Your tables",
  five sites), `web/src/Help.test.tsx` (four needles), `web/e2e/joins_by_link.spec.ts:478` (`/your move|to move/`)
  and three comments, `web/e2e/two_browsers.spec.ts:181` and `web/e2e/harness.ts:272` (comments),
  `web/e2e/setup_screens.spec.ts` (+ the lobby cell).

**Interfaces:**
- Consumes: Task 2's `SeatChip`, `PasswordTag`, `Padlock`, `LobbyBand`; Task 4's frame of `Lobby` (band, corner,
  actions row, `Unreachable`, `.skeleton-rows`); `TableView.bot`; `production_bot.json` (the cells only); `Abandon`
  (`web/src/Abandon.tsx`), `Chat`, `usePolled`, `lobbyChat`.
- Produces: `tables.ts` — `Section`, `ago(createdAt, now?)`, `rowMeta(t, section, now?)`, `seatChips(t)`,
  `rowAction(t, section)`, `ChipModel`, `RowAction`. **The lobby's DOM contract the walks read**: `.lobby`,
  `section.lobby-section.your-games` / `.open-tables` / `.playing-tables` holding `ul.lobby-rows > li.lobby-row`
  (+ `.mine`) whose ONE link is `a.row-link` (the table's name), `.row-title` / `.row-meta` / `.row-seats` /
  `.row-action`, the action buttons named `"<label>: <table name>"` (`"Play: wire"`, `"Copy link: wire2"` →
  `"Copied: wire2"`, `"Take a seat: wire2"`, `"Spectate: wire"`, `"View: done"`), the creator's `Abandon` beside them,
  `span.stranded` "can't continue here", `section.lobby-rail.lobby-chat`.

**What a row is** (mock 02): `name [PASSWORD] / size · age-or-state` · a chip per seat · one action. The action by
section — Your tables: **Copy link** while the table fills, **Play** (gold, `.primary.yen`, when your move or your
card is owed) while it deals or plays, **View** once over; Open tables: **Take a seat**; Being played: **Spectate**.
The padlock rides the action where a password is still wanted (a locked open table; a game with `can_watch` false).
A game NOT over whose bot this build does not run (`bot.runs_here === false`) offers no action and says "can't
continue here" — every game route refuses it — and its creator keeps Abandon (the spec-gap answer: the lobby is the
ONLY place a dealing or playing server table can be abandoned, R-13). A table you sit at is under Your tables ONLY
(`my_seats` non-empty). Row actions are BUTTONS and the name is the one link (`exit_game.spec.ts` counts one).

- [ ] **Step 1: Load the design skill; read the mocks.** `frontend-design:frontend-design`; mocks 02, 10, 11;
  `door-v3-lobby-v2-name.html` §2 and `phone.html`'s lobby.

- [ ] **Step 2: The row's words, failing then green** — `drafts/D4-tasks-09-12.md:4008-4200`: `tables.test.ts`'s
  describe "the lobby row" (4 cells: `ago`'s eight readings including a clock behind the table and a `NaN`;
  `rowMeta` by section; `seatChips` in seat order — the bot chip `the bot · V4 · to move`, `the bot · to move` where
  the listing carries no `bot`, `open seat`, no turn while dealing or over; `rowAction` — copy for yours filling, gold
  Play when owed, View when over, Take a seat with the padlock on a locked open table, Spectate with the padlock until
  `can_watch`, `{kind: "stranded"}` for a not-over foreign game and View once it is over). Then `tables.ts` as written
  (:4094-4194).

- [ ] **Step 3: The lobby's cells, failing** — `drafts/D4-tasks-09-12.md:4202-4405`: the two list names renamed
  everywhere in `Lobby.test.tsx` (`grep -c 'Your games\|Games in progress' web/src/Lobby.test.tsx` then prints 0);
  the "games in progress" cell reads "2 players" and the Spectate button; "says what each table is doing and who is
  at it: its size, then a chip per seat" and "marks whose turn it is in gold, and says who won" rewritten; the chat
  cell's selector `section.lobby-chat`; the seven new cells (one action per row and the click going where the name
  goes with nothing fetched and ONE link per row; the gold edge on yours only; the padlock on the action only where a
  password is still wanted; the bot's version and "can't continue here" with Abandon kept and the finished foreign
  game opening as ever; Copy link → Copied through `userEvent`'s clipboard; a table you sit at under Your tables only;
  skeleton rows and "loading…" until the lists land, never "none yet"). `App.test.tsx`'s five "Your games" sites.

```bash
cd web && npx vitest run src/Lobby.test.tsx src/App.test.tsx
```

  Expected (D4): 22 failed — twenty in `Lobby.test.tsx`, two in `App.test.tsx`.

- [ ] **Step 4: The lobby** — `drafts/D4-tasks-09-12.md:4407-4649`: the header comment, the imports (`Padlock,
  PasswordTag, SeatChip` from `./Chips`; `rowAction, rowMeta, seatChips, type Section` from `./tables`), `lobbyRow`
  deleted, `notMine`, the `copied` state, `copyLink` (a refused clipboard puts the address in the error strip),
  `row(t, section)` (the `Abandon` for `section === "mine" && t.created_by === me`), `mine`/`open`/`playing`/
  `nextCursor`/`rail`, and the fragment's tail (`.lobby-cols` → `.lobby-lists` with the three `section.lobby-section`s
  — Your tables with its "loading…" + `.skeleton-rows` while `lists === null && error === null`, Open tables "take a
  free seat", Being played "spectate a game" — and the `.lobby-rail.lobby-chat` with `<Chat scope={{kind: "house"}}
  tick={tick} sendName heading="h3" />`). Task 4's `toDevice` doc: "An ANCHOR for `lobbyRow`'s reason" → "An ANCHOR
  (R-14)".

```bash
cd web && npx vitest run src/Lobby.test.tsx src/App.test.tsx src/tables.test.ts && npx tsc -b --noEmit
```

  Expected: green (D4: 160 = 29 + 118 + 13).

- [ ] **Step 5: The help's list names** — `drafts/D4-tasks-09-12.md:4657-4669`: "Your tables" (two sites), "Being
  played, the ones you can spectate" and "Being played" (two sites), the chat "beside the three lists (under them on a
  phone)"; the four `Help.test.tsx` needles. Expected: 22 passed.

- [ ] **Step 6: The browser walk's words, and the lobby cell** — `drafts/D4-tasks-09-12.md:4671-4798`:
  `joins_by_link.spec.ts:478`'s `/your move|to move/` and the three list-name comments (`:463`, `:470`;
  `two_browsers.spec.ts:181`; `harness.ts:272`; `harness.ts:296`'s is history and stays); then `setup_screens.spec.ts`
  gains the `PRODUCTION_LABEL` reader (`offline.spec.ts`'s committed-file idiom: the package is an ES module, so a bare
  JSON import needs an attribute) and the lobby cell: three tables created over the API as `WALKER` and `OTHER`
  (`someone@e2e.test`), one locked, one against the bot, one somebody else's; at BOTH widths the rows, the password tag,
  `the bot · V4` from `TableView.bot`, "Take a seat: Someone else's", no sideways scroll, zero images; at 1440 the rail
  RIGHT of the lists, the pill hidden, "Playing as" visible, name/seats/action on ONE line (the three middles within
  12px); at 390 the rail BELOW the lists, the pill, "switch player" hidden then visible after a tap with no sideways
  scroll, "Copy link: Width check" and "New table" ≥ 44px; `look("lobby-1440")` / `("lobby-390")`; the three tables
  deleted in `finally`.

- [ ] **Step 7: The suites.** `npm test && npm run typecheck`: **+11 cells**, no new file (Lobby +7, tables +4).

- [ ] **Step 8: The stylesheet** — `drafts/D4-tasks-09-12.md:4808-4862`: the appended block (`.lobby-cols` 1fr +
  18.75rem, `.lobby-section`, `.lobby-row`'s three-column grid with `.mine`'s gold left edge, `.row-*`, the padlock in
  `--warn` on an action, `.stranded`, one column below 860px, the phone's two-column row with the seats wrapping under);
  DELETE by selector the "lobby's two lists ARE the setup screen's `.saved` sections" block (`.lobby .lock`, `.lobby
  .open-tables li .table-facts`, `.lobby .saved li a.row-link` + `:hover`), `.lobby .lobby-chat { margin-top }`, and
  `.saved ul` / `.saved li` / `.saved li .table-facts` — after this task no element carries `saved`
  (`grep -n 'className=[^>]*saved' web/src/*.tsx` prints nothing; run it and paste).

- [ ] **Step 9: The browser walk.** Background to `task-5-e2e.txt`, POLL. Expected: green, one cell larger than Task
  4's. `exit_game.spec.ts` (opt-in, live) counts ONE link per table, which the row keeps; its run is cycle C's.

- [ ] **Step 10: The headed look.** `lobby-1440.png` and `lobby-390.png` into `task-5-look/`, read beside mocks 02 and
  11: the rows' three columns, the gold edge, the PASSWORD tag after the name and the padlock on the action, the chips'
  gold for the turn, the rail beside then below, the pill. Correct and re-run. D4's own renders (`d4-look/lobby-*.png`)
  are the comparison.

- [ ] **Step 11: The per-edit gates.** `npm run typecheck`; `npm test` (+11); `test:wasm` unmoved; the e2e; the Rust
  and `worker/` gates unmoved; both fences empty.

- [ ] **Step 12: Commit.**

```bash
git add web/src/Lobby.tsx web/src/Lobby.test.tsx web/src/tables.ts web/src/tables.test.ts web/src/App.test.tsx \
  web/src/Help.tsx web/src/Help.test.tsx web/src/styles.css web/e2e/joins_by_link.spec.ts web/e2e/two_browsers.spec.ts \
  web/e2e/harness.ts web/e2e/setup_screens.spec.ts
git commit -m "feat(web): the lobby as three lists of rows — name and age, a chip per seat, one action; the password flag, the bot's version, the chat in a rail"
```

---

### Task 6: The forms and the seed check (spec §2.2 "The forms", "The seed"; R-6.5c2.8; mock 07, mock 11; Review Focus 2; the retired review's P-7) — from D5 retired Task 13, MINUS the emission cycle A did

**What cycle A already did, and this task must NOT redo:** D5's Steps 1–6 (the Rust cell `the_seed_table_holds_the_
verdicts_the_spec_names`, `SEED_INPUTS`, `seed_cases()`, the regeneration, the two third-column inserts) and Step 9's
`contract.ts` entries landed in cycle A's Task 7. At `f2a3dd61` `web/src/wire/seed_cases.json` has 26 rows,
`contract.ts` imports it, types it (`_seedCases: readonly {raw; ok}[]`) and exports `wire.seedCases`; `NOT_SERVED` and
`THIRD_COLUMN` name it. **This task edits no Rust, no corpus file, no `contract.ts`, no third column**; a diff touching
any of them is a FINDING.

**Files:**
- Create: `web/src/seedRule.ts`, `web/src/seedRule.test.ts`; `web/e2e/the_forms.spec.ts`.
- Modify: `web/src/NewGame.tsx` — rewritten whole (370 lines); `web/src/NewGame.test.tsx` — rewritten whole;
  `web/src/App.tsx` (`onCancel` on both forms; `serverCorner` hoisted above `lobbyScreen`; the table form takes
  `who={serverCorner}`); `web/src/Help.tsx` (the form's paragraph; the "(It is not the display name on the form
  below…)" sentence DELETED); `web/src/styles.css` (a block appended; `.field.password-field` and `.field .check`
  deleted; **both `.field.seat-row` rules KEPT** until Task 7).
- Test: `web/src/App.test.tsx` (the form's words; +2: Cancel, the band's corner), `web/src/Help.test.tsx` (the
  display-name assertion and its comment deleted), `web/e2e/harness.ts` (`newGameForm` `level: 3`; `startGame`'s
  seats by colour; `createLocal`'s heading; + `nameThisBrowser`), `web/e2e/joins_by_link.spec.ts:179-195`,
  `web/e2e/two_browsers.spec.ts:134-146`, `web/e2e/exit_game.spec.ts:338-342`, `web/e2e/plays_locally.spec.ts:1076`
  (the form's words), **`web/e2e/the_table.spec.ts:430-433` (P-7: the 4P cell drives the OLD form's `selectOption`
  on "Players" and "Seat N" — it takes the segment "4" and the three colour comboboxes' "me", exactly as `startGame`
  does)**.

**Interfaces:**
- Consumes: Task 2's `LobbyBand`, `VersionTag`, `.raised-card`/`.card-band.vp`/`.yen`/`.card-in`/`.seat-dot`/
  `button.primary.yen`; Task 4's `normalizeName` and the corner `lobbyScreen` builds; `production_bot.json` directly;
  `useIdentityMode()` (`"open"` under the local server); `wire.seedCases`; `displayName`, `LOCAL_USER`,
  `NewGameRequest`, `SeatRequest`, `api.create(req, host?)`; `patchServerFacts`, `resetFactsForTest`
  (`local/facts.ts`); `stubWorker`, `currentWorker`.
- Produces: `seedRule.ts` — `checkSeed(raw)`, `trimSeed(raw)`, `type SeedVerdict`; `NewGame`'s `onCancel`;
  `export function drawSeat(n)`; the forms' accessible names (the shared-names table); App's `serverCorner`; e2e
  `nameThisBrowser(page, name)`; the `.band-screen`, `.form-card`, `.form-field`, `.form-seat`, `.seg`, `.toggle`,
  `.fold`, `.form-foot`, `.quiet-link` rules; the request body's key set for a bare create:
  `["colors", "players", "seats", "start_seat"]`.

**The seed rule has two statements, and the table holds them together** (spec §2.2, §3): the server reads
`str::trim` then `u64::from_str` (`WireSeed`); the form checks the same rule AS IT IS TYPED so a bad value explains
itself in the fold instead of arriving as the create's 400. `trimSeed` is `/^\p{White_Space}+|\p{White_Space}+$/gu`,
NOT `String.prototype.trim` (D5 measured the two disagree on exactly U+FEFF and U+0085 — the mutant the table catches).
A seed is never a number on this side: text from the keystroke to the wire, the bound compared as a `BigInt`. Blank is
the FORM's question — "the server draws one", never sent — while the table says `false` for `""`.

**What the forms are** (mock 07): the table form under the band `["Lobby", "New table"]` with the corner — a raised
card titled `<h3>New table</h3>` (level 3: the band's crumb is the `h2`), the name field whose placeholder "ID shown
in the lobby if left blank" is the whole explanation (no subtext), Players as segments 2/3/4, seats as colour rows
in the engine's order each with a `<select aria-label="<Colour> seat">` of You / Open seat / A specific person / The
bot (two players only, with the version tag beside it and the note "offered at two players only"), a specific
person's field labelled by NAME on an open door and ADDRESS behind Access (`type="email"`), First player as segments
(the colours + Random, drawn at SEND from `crypto.getRandomValues` by rejection sampling), the password toggle "Ask
for a password to join or spectate" revealing "Table password" with no placeholder, the seed under `details.fold`
"Advanced" (held open while refused; `aria-invalid`, `aria-describedby="seed-error"`), Cancel (a quiet link) and
Deal / Open table (the word says which). The local form under `["On this device"]` with NO corner: a yen card titled
"New game against the bot" with the tag beside the heading, "Two players: you and the bot.", Name "Optional", the two
seats as a LIST (not controls), First player You / The bot / Random, Cancel and a yen Deal. "Your name" is on neither
(the corner owns it; a create sends the name this browser KEEPS, read at send).

- [ ] **Step 1: The failing client cell (Review Focus 2)** — `drafts/D5-tasks-13-16.md:348-410`, `seedRule.test.ts` as
  written (4 cells: every verdict the server gives, with the anti-vacuity that the table has both verdicts; the
  named inputs present; the trim is Rust's — the two premises `"﻿12".trim() === "12"` and
  `"\u008512".trim() === "\u008512"` asserted, then `trimSeed`'s opposite answers; the two refusal sentences and
  `" +007 "` accepted). Write the two non-ASCII characters as `\u` escapes.

```bash
cd web && npx vitest run src/seedRule.test.ts
```

  Expected: `Failed to resolve import "./seedRule"`.

- [ ] **Step 2: The rule** — `drafts/D5-tasks-13-16.md:423-470`, `seedRule.ts` as written. Then:

```bash
cd web && npx vitest run src/seedRule.test.ts && npm run typecheck
```

  Expected: 4 passed, both projects clean. (D5's Step 9 `contract.ts` edits are ALREADY in the tree — do not add them.)

- [ ] **Step 3: Load the design skill; read the mocks.** `frontend-design:frontend-design`; mocks 07 and 11;
  `tables-forms-v2.html`.

- [ ] **Step 4: The failing form cells** — `drafts/D5-tasks-13-16.md:505-1000`, `NewGame.test.tsx` whole (32 cells:
  the table form's seats — one per colour, the bot at two players only with its tag, a specific person by NAME on the
  open door normalized to the key, by ADDRESS behind Access, held while blank or unkeyable, the hot seat asked for, the
  bot back at two, Deal vs Open table; the first player — the colour chosen, Blue as seat 1, Random drawn at SEND, a
  first player the new count lacks falls back to Red; `drawSeat`'s rejection sampling; the name field's placeholder
  with no subtext, no "Your name" and `display_name` from storage read at send, none when none kept, the password
  toggle's label and no placeholder, the password sent only when ticked and typed; the seed — in the fold as the exact
  text typed (a `u64` a JSON number cannot hold, premise asserted), no seed for blank, a bad seed explained IN the fold
  with the button held, **(Review Focus 2)** every committed input's verdict through `fireEvent.change`; sending — the
  key set, one create however fast, a refused create's sentence, Cancel; the local form — its heading and tag, the
  seats as a list named "Seats", the Worker create seated by `local@browser` with `start_seat` chosen, Random at
  send, the names carried, Cancel), with these SUBSTITUTIONS: `"  Ben   Moore "` → `"  Ben   Castle "` and its key
  `"ben moore"` → `"ben castle"`; `"  Jem  "` → `"  Ben  "` and `"Jem"` → `"Ben"`; and **every "New table" heading
  query takes `level: 3`**.

```bash
cd web && npx vitest run src/NewGame.test.tsx
```

  Expected (D5, with the stubs): **26 of 32 fail**, each at its first query (`… role "heading" and name "New table"`,
  `… role "group" and name "Players"`, `… role "combobox" and name "Blue seat"`); the six behaviours the old form
  already has stay green (no `display_name` when none kept; no seed for blank; the key set; one create for a double
  click; a refused create's sentence; the local form's two names).

- [ ] **Step 5: The form** — `drafts/D5-tasks-13-16.md:1019-1566`, `NewGame.tsx` whole as written (`COLORS`,
  `BOT_LABEL`, `SeatRow`, `First`, `defaultSeats`, `drawSeat`, `Segmented`, `useSubmit` — a ref decides whether the
  request happens, the state only disables the button — `keptName`, `NewGame`, `TableForm`, `LocalForm`).

```bash
cd web && npx vitest run src/NewGame.test.tsx src/seedRule.test.ts
```

  Expected: 36 passed.

- [ ] **Step 6: Cancel, the band's corner, and App's form cells** — `drafts/D5-tasks-13-16.md:1574-1660`: `App.tsx`'s
  two forms take `onCancel` (`toDoor()` local, `toLobby()` server); **`serverCorner` HOISTED** directly above
  `lobbyScreen` (`const serverCorner = signedIn && door.me ? <IdentityCorner me={door.me} onChange={signOut} /> :
  undefined;`), `lobbyScreen`'s `corner={serverCorner}`, the table form's `who={serverCorner}`; `App.test.tsx`'s form
  words ("New table" with `level: 3` at three sites and an unlevelled ABSENCE at one; the Players segment with the
  premise asserted; the local form's `queryByRole("group", {name: "Players"})` wait; `playsHere`'s "New game against
  the bot" and the seats list scoped), and the two new cells ("leaves the table form for the lobby and the local form
  for the door"; "draws the identity corner in the new-table form's band"). **Mutation-proven** (D5): with the table
  form's `who={serverCorner}` put back to `who={who}` the corner cell fails `Unable to find an element with the text:
  /Playing as/`; restored, it passes. Quote it.

- [ ] **Step 7: The help panel's form paragraph** — `drafts/D5-tasks-13-16.md:1662-1692`: the second paragraph of
  "Starting a game" replaced (one row per seat in the colours' order; the bot at two players with the version it plays
  as; who plays first or a draw; a name; a password anyone joining or spectating types; Advanced's seed checked as you
  type; Deal vs Open table); **and the display-name sentence Task 4 kept is DELETED**, with `Help.test.tsx`'s
  `expect(text).toContain("not the display name on the form below");` and its one comment line. `grep -n "display
  name" web/src/Help.tsx` then prints nothing about the form. Expected: the Help suite green; "says the bot is offered
  at two players only" still reads the new sentence.

- [ ] **Step 8: The stylesheet** — `drafts/D5-tasks-13-16.md:1703-1818`: the appended block (`.band-screen > .setup`,
  `.form-card`, `.form-field`, the inputs and selects by class never bare `input`, `.field-error`, `.seg` with the
  radio transparent over its label and `:has(input:focus-visible)`, `.form-seat`, `.seat-note`, `.toggle` with
  `.switch-track` — never `.track`, the dashboard's own class — `.fold` with a drawn marker, `.form-foot`,
  `.quiet-link`, reduced motion, the 480px rules with 2.75rem controls); DELETE `.field.password-field` and
  `.field .check`; **KEEP both `.field.seat-row` rules**. No Task 2 class is restated.

```bash
cd web && npm run typecheck && npm test
```

  Expected: **+16 cells, +1 file** over Task 5's reading (D5: seedRule +4 new, NewGame +10 net, App +2).

- [ ] **Step 9: The browser walks on the new form** — `drafts/D5-tasks-13-16.md:1831-1916`: `harness.ts`
  (`newGameForm`'s heading `{ name: "New table", exact: true, level: 3 }`; `startGame` above two players checks the
  Players radio and selects "me" in each `"<Colour> seat"` combobox, `exact`; `createLocal`'s heading "New game
  against the bot"; `nameThisBrowser` added below `signIn`); `joins_by_link.spec.ts:179-195` (the Blue seat combobox
  → "open", Name "Link test", `nameThisBrowser(a, "Annabelle the Cartographer")` — the LONG name the chat row is
  measured under, live review I-1 — the toggle checked, "Table password" filled); `two_browsers.spec.ts:134-146` (the
  Blue seat → "person", `/^Blue player's (name|address)$/` filled with `b@e2e.test`, the toggle and `WATCH_PASSWORD`);
  `exit_game.spec.ts:338-342` and `plays_locally.spec.ts:1076`; **and P-7, `the_table.spec.ts:430-433`**: the 4P
  cell's `selectOption("4")` on "Players" becomes the Players group's radio "4" checked, and its three `Seat ${i}`
  `selectOption("me")` become `getByRole("combobox", { name: \`${color} seat\`, exact: true }).selectOption("me")` for
  Blue, Yellow, Green — the same four lines `startGame` now runs.

- [ ] **Step 10: The forms' own width cells (Review Focus 2)** — `drafts/D5-tasks-13-16.md:1921-2027`, CREATE
  `web/e2e/the_forms.spec.ts` as written: `test` from `./sw`, the skip line, `atWidth`, `fits`; for 1280 and 390 the
  new-table form AT ITS WIDEST (4 players, Green a specific person, the password open, the fold open — a width claim
  about the narrowest form proves nothing) refusing `18446744073709551616` INSIDE the fold's own box with the button
  held, then `"+007 "` letting it go; the local form fitting with Random checked; at 390 the submit ≥ 44px with its
  right edge on the screen; a screenshot per cell (`new-table-<w>.png`, `new-local-<w>.png`).

  Run the e2e in the background to `task-6-e2e.txt`, POLL. Expected: green, **+4 cells, +1 file** over Task 5's;
  `the_forms.spec.ts` picked up by both meta-specs with no change to either; `the_table`'s 4P cell green on the new
  form (P-7 closed).

- [ ] **Step 11: The headed look.** The four pictures from `web/test-results/the_forms-*/` into `task-6-look/`, read
  beside mocks 07 and 11 — the segments, the seat rows, the toggle, the fold's refusal in `--warn`, the yen local card —
  correct and re-run; the report says what differs.

- [ ] **Step 12: The per-edit gates.** `npm run typecheck`; `npm test` (+16 / +1 file); `test:wasm` unmoved; the e2e
  (+4 / +1 file); the Rust and `worker/` gates unmoved (`git diff --stat f2a3dd61 -- crates worker` prints nothing);
  both fences empty.

- [ ] **Step 13: Commit.**

```bash
git add web/src/seedRule.ts web/src/seedRule.test.ts web/src/NewGame.tsx web/src/NewGame.test.tsx \
  web/src/App.tsx web/src/App.test.tsx web/src/Help.tsx web/src/Help.test.tsx web/src/styles.css \
  web/e2e/harness.ts web/e2e/joins_by_link.spec.ts web/e2e/two_browsers.spec.ts web/e2e/exit_game.spec.ts \
  web/e2e/plays_locally.spec.ts web/e2e/the_table.spec.ts web/e2e/the_forms.spec.ts
git commit -m "feat(web): the two forms in the approved look, and the seed checked as it is typed against the server's own table"
```

---

### Task 7: Waiting, joining, and the password screens (spec §2.2 "Waiting and joining", "Spectate"; R-6.5c2.9; mocks 08, 09, 11; Review Focus 4) — from D5 retired Task 14

**Files:**
- Modify: `web/src/OpenTable.tsx` — the render rewritten; `join` reads the kept name AT THE CLICK; the `name` and
  `copied` states go (the copied state moves into `Invite`); `web/src/Unlock.tsx` — restyled under the band, "watch"
  → "spectate"; `web/src/LockScreen.tsx` — the standalone shape restyled (band + `.notice-card`), its `who` prop and
  `ReactNode` import removed; `web/src/App.tsx` (`who={serverCorner}` on `<OpenTable>` and `<Unlock>`; the lock
  screen's `who={who}` goes; the server deal's `who` STAYS — `Dealing` draws no band); `web/src/Help.tsx` (the joining
  paragraph); `web/src/styles.css` (a block appended; `.field.seat-row` ×2, `.open-table .seats`, the old
  `.open-table-controls`, `.field.link-field` ×2, `.unlock .unlock-controls` deleted; `.locked-out .unlock-controls`
  dropped from the `:1844` selector list — `.broken-save .unlock-controls` KEPT).
- Test: `web/src/OpenTable.test.tsx` (the two faces rewritten; a several-seats describe added; the chat cell's two
  queries), `web/src/Unlock.test.tsx` (the words; `level: 3` on the table's heading), `web/src/App.test.tsx` ("sit here"
  → "Join as Blue" ×5; the unlock face's "Watch" → "Spectate" ×4 and "Password" → "Table password" ×6; +1 the corner on
  both faces), `web/e2e/joins_by_link.spec.ts` (A's waiting face, the 390 block, B's arrival), `web/e2e/two_browsers.spec.ts`
  (`:192`, `:221`), `web/e2e/exit_game.spec.ts` (`:371-375`, `:395-398`).

**Interfaces:**
- Consumes: Task 2's `LobbyBand`, `SeatCard`, `PasswordTag`; Task 6's `nameThisBrowser`, `serverCorner`,
  `.band-screen`/`.form-field`/`.quiet-link`; `displayName()`; `api.join(game, {seat, name?, password?})`;
  `hashOf`; `Chat`, `Abandon`, `TableFacts`, `usePolled`.
- Produces: no new names. The accessible names change: "sit here" → **"Join as <Colour>"** (one free seat) or
  **"Sit here as <Colour>"** (each open card, several free); "Link" → **"Invite link"**; "copy" → **"Copy link"** →
  **"Copied"**; the joiner's "Password" → **"Table password"**; the unlock face's "Watch" → **"Spectate"**, its
  "Password" → **"Table password"**. The open table's DOM: `.band-screen > header.lobby-band + .setup.open-table`,
  `h3.page-title`, `p.table-status` (+ `PasswordTag`), `.join-password` ABOVE `ol.seat-cards[aria-label="Seats"]`, the
  one `button.primary.join` BELOW the cards, `section.invite` (the read-only `input[aria-label="Invite link"]`, the
  Copy button, the `.note`), the chat in the MAIN column under the invite (the spec-gap answer, not mock 08's rail),
  `.open-table-controls` (Abandon as the outlined danger, "← Lobby" as a quiet link).

**What changes, in one list** (spec §2.2; R-6.5c2.9; the spec-gap answers): seat cards replace the seat lines — a
taken seat solid by its holder's NAME with the note "seated" (mock 08's "seated by you when you opened the table" is
not derivable: a seat bound at create and one joined later are the same `SeatDoc`), the caller's own "You" with the
table's name for them, an open one dashed ("yours if you join" / "anyone in the lobby, with the password"), the first
player marked in the caption; the joiner types no name and types the password ABOVE the seats (the spec's rule
wins over mock 09's box) and gets ONE "Join as <Colour>" under the seats when one seat is free, "Sit here" in each
open card when two or more are; **arriving seats nobody — one click, never zero** (Review Focus 4); every seated
caller gets "Invite people" with the password line worded for the creator ("…the password you set…") or a seated
guest ("…the table's password."); Abandon the creator's, outlined in `--warn`.

- [ ] **Step 1: Load the design skill; read the mocks.** `frontend-design:frontend-design`; mocks 08, 09, 11;
  `waiting.html`, `tables-forms-v2.html`.

- [ ] **Step 2: The failing open-table cells** — `drafts/D5-tasks-13-16.md:2116-2441`: the `fourWithTwoOpen` builder
  (the corpus's tables with only the SIZE moved, the cell says so), `cards()`; the describe "the open table, to a
  stranger with one seat to take" (11 cells: the table asked for once; ONE way in with the password above the seats and
  the cards' words; no password at an unlocked table; **(Review Focus 4)** "seats nobody on arrival" under a faked clock
  with `flush()` and no `/join` posted; the seat, the kept name and the password posted — with `"  Jeremy  "` →
  `"  Ben  "` and `"Jeremy"` → `"Ben"`; no name and no password where none; a wrong password's sentence keeping what
  was typed; the seat taken meanwhile re-reads once and shows no way in; hand-over on 409-not-open and on the
  last-seat join; one join however fast); the describe "…with a choice of seats" (2 cells: "Sit here as Yellow/Green",
  the password once above, THE seat chosen posted); the describe "…to somebody sitting at it" (10 cells: the creator's
  invite and password line, a seated guest's line, nothing at an unlocked table, the seat cards, Copied through
  `userEvent`'s clipboard, an escaped id, Abandon for the creator only, leaving after abandon, the way back); the chat
  cell's "Invite link", `.invite`, "Join as Blue".

```bash
cd web && npx vitest run src/OpenTable.test.tsx
```

  Expected (D5): **21 of 27 fail, 6 stay green** (the three watch cells, Abandon for the creator, leaving after
  abandon, the way back) — each red at a first query, "seats nobody on arrival" as `the table was drawn never became
  true`.

- [ ] **Step 3: The two faces** — `drafts/D5-tasks-13-16.md:2456-2687`: the imports; the `name` and `copied` states
  deleted (`noUnusedLocals` refuses the orphan, measured); `join` reading `displayName()?.trim()` at the click and
  spreading `name` and `password` only when present (every request body is `deny_unknown_fields`); the render from
  the `corner` node through `Invite`. Expected: `npx vitest run src/OpenTable.test.tsx` green.

- [ ] **Step 4: The unlock face** — `drafts/D5-tasks-13-16.md:2698-2775`: `Unlock.test.tsx`'s words ("Table password"
  ×4, "Spectate" ×6, `spectate` variable, "a would-be SPECTATOR", the table's heading with `level: 3`, the whole-body
  `not.toMatch(/\bwatch/i)` and "password required to spectate"); expected red 5 of 6 (the "goes back to the lobby"
  cell stays green); then `Unlock.tsx`'s header word, the `LobbyBand`/`PasswordTag` imports and the returned tree
  (`.band-screen`, the title row with the tag, `TableFacts` + "password required to spectate", `.spectate-box`, the
  quiet Lobby link). Expected: green.

- [ ] **Step 5: The lock screen, restyled** — `drafts/D5-tasks-13-16.md:2777-2810`: the `LobbyBand` import; the
  standalone return under `["On this device"]` with NO corner and a `.notice-card`; the `who` prop, its doc, the
  `ReactNode` import and App's `who={who}` on the lock screen REMOVED. The word "Lobby" stays here (Task 8 makes it
  `leaveLabel`). `App.test.tsx:3334-3345`'s words ("This game is open in another tab", "Take over") hold.

- [ ] **Step 6: App's cells and the help** — `drafts/D5-tasks-13-16.md:2812-2863`: "sit here" → "Join as Blue" (five
  sites), the unlock face's six label and four button sites, and the new cell "draws the identity corner in the open
  table's band and the unlock face's"; `App.tsx`'s `who={serverCorner}` on `<OpenTable>` and `<Unlock>` (**mutation-
  proven**, D5: either put back to `who={who}` fails `Unable to find an element with the text: /Playing as/`); the help's
  joining paragraph (Invite people, Join as its colour when one seat is left, Sit here when more, Open tables without a
  link, the deal begins by itself). `Help.test.tsx` needs no edit (its needles `/link/i`, `/password/i` hold).

- [ ] **Step 7: The stylesheet** — `drafts/D5-tasks-13-16.md:2865-2942`: the deletions by selector (after which
  `grep -rn "seat-row" web/src --include='*.tsx'` prints nothing — run it and paste), then the appended block
  (`h3.page-title`, `.notice-card h3.page-title` smaller, `.table-status`, the three inputs BY CLASS — never
  `.open-table input`, which would restyle the chat's own box — `.join-password`, `.seat-cards` auto-fit grid,
  `button.primary.join`, `.invite` with its caption an `h3.caption` drawn as a caption, `.spectate-box`,
  `.notice-card`, `.notice-actions`, the 480px rules: one column of cards, full-width primaries, 2.75rem controls).

```bash
cd web && npm run typecheck && npm test
```

  Expected: **+7 cells** over Task 6's (D5: OpenTable +6 from 21 to 27, App +1; Unlock holds 6).

- [ ] **Step 8: The browser walks** — `drafts/D5-tasks-13-16.md:2961-3065`: `joins_by_link.spec.ts` — A's invite
  (`"Invite link"` by value, the creator's password reminder visible), the 390 block ("Copy link", "Invite link", the
  seat cards in ONE column at 390 each under the last), B's arrival REPLACED (a `request` listener counting every
  `/join` B's page makes; `signIn(b, "b@e2e.test", …)`; the heading "Link test"; the password tag; ONE "Join as Blue"
  and zero "Sit here"; **`expect(joinsByB).toEqual([])` — arriving by the link seated nobody (Review Focus 4)**; the
  joiner's screen at 390: the button ≥ 44px with its right edge on the screen and no sideways scroll;
  `nameThisBrowser(b, "Bob")`; the wrong password's sentence with the field keeping what was typed; then "sesame" and
  the join), and the file's top comment; `two_browsers.spec.ts` (`spectateButton` "Spectate", "Table password");
  `exit_game.spec.ts` (`:371-375` the join by "Join as Blue" after `nameThisBrowser(y, "y (exit test)")`; `:395-398`
  the "Spectate" button and "Table password"; `nameThisBrowser` imported). Background to `task-7-e2e.txt`, POLL.
  Expected: green at Task 6's count (no new spec file). `exit_game` only type-checks here (it runs live, cycle C's).

- [ ] **Step 9: The headed look.** The walk's own pictures from `web/test-results/joins_by_link-*` — where the run
  left none, take two with a one-off `page.screenshot` in an export (never committed) — A's waiting face and B's
  joining face at 1280 and 390, into `task-7-look/`, read beside mocks 08, 09 and 11; correct and re-run.

- [ ] **Step 10: The per-edit gates.** `npm run typecheck`; `npm test` (+7); `test:wasm` unmoved; the e2e (count
  unchanged); the Rust and `worker/` gates unmoved; both fences empty.

- [ ] **Step 11: Commit.**

```bash
git add web/src/OpenTable.tsx web/src/OpenTable.test.tsx web/src/Unlock.tsx web/src/Unlock.test.tsx \
  web/src/LockScreen.tsx web/src/App.tsx web/src/App.test.tsx web/src/Help.tsx web/src/styles.css \
  web/e2e/joins_by_link.spec.ts web/e2e/two_browsers.spec.ts web/e2e/exit_game.spec.ts
git commit -m "feat(web): waiting and joining as seat cards — one free seat is one Join button — and the password screens say spectate"
```

---

### Task 8: "Spectate", the single-player controls, "Will not open here", the game screen's foreign refusal, and the Players panel's version tag (spec §2.2 "Spectate" and the single-player "Lobby" controls; §2.4 the local foreign row, the game screen's refusal and the version tag; R-6.5c2.7, R-6.5c2.10; Review Focus 5) — from D5 retired Task 15, plus the tag no draft drew

**Files:**
- Modify: `web/src/App.tsx` — the banner strip ("You are spectating"); `localExit` beside `toLobby`/`toDoor`;
  `isForeignBot(body)` beside `saveGameFile`; the foreign 409 arm in `open`'s catch; the `broken` state's doc and its
  screen made host-aware; `onThisDevice`'s condition; the lock screen's, the local deal's and the Game menu's calls;
  the game screen's `<Dashboard>` gains `bots={state.bots}`.
- Modify: `web/src/LockScreen.tsx`, `web/src/Dealing.tsx`, `web/src/GameMenu.tsx` — a `leaveLabel?: string` prop
  (default `"Lobby"`); `web/src/Chat.tsx` (the marker's word; the CLASS `watching` stays — no one reads a class and
  `Chat.test.tsx:96` pins it); `web/src/Help.tsx` ("Spectating a game" replaces "Watching a game" whole; the Talking
  section's two words; `grep -in watch web/src/Help.tsx` then prints nothing); `web/src/OnThisDevice.tsx` (the
  `foreign` filter; `<Unplayable foreign onRemoved>`); `web/src/BackupControls.tsx` (`Unplayable` lists foreign rows
  beside the cross-rev ones, `foreignWords`, `removeRefused`); `web/src/backup.ts` (+ `removeForeign`);
  **`web/src/Dashboard.tsx` (the Players panel: `bots?: readonly SeatBot[]` threaded to `Players`; a bot seat's tag
  reads "the bot" + `<VersionTag label />` where `bots` names the seat, else today's bare "bot")**.
- Test: `web/src/App.test.tsx` (the banner ×2; +5 single-player: the multiplayer partner and four "in a
  single-player build"; +3 the foreign route screen; +1 the local foreign screen; **+1 no stream for the refused
  game**), `web/src/Chat.test.tsx` (`:101`, `:355-359`), `web/src/Help.test.tsx` (the spectating describe replaced by
  5 cells; the Talking label cell), `web/src/OnThisDevice.test.tsx` (+3), `web/src/backup.test.ts` (+3),
  **`web/src/Dashboard.test.tsx` (+2)**, `web/e2e/two_browsers.spec.ts` (`:223` "You are spectating", `:244-252` the
  marker " · spectating"), `web/e2e/exit_game.spec.ts` (`:399`, the comment at `:390-391`).

**Interfaces:**
- Consumes: `TableView.bot`; `wire.foreignBot409` (`ForeignBotErrorBody`), `wire.tableForeign`, `wire.dealing409`
  (a 409 WITHOUT `foreign_bot`); `wire.watching.bots` (`[{seat: 0, label: "V4"}]`); `production_bot.json`; Task 7's
  `LockScreen` shape; Task 5's "Being played"; `multiplayerOffered()` (`FrontDoor.tsx`); `leave`, `toDoor`, `toLobby`
  (`App.tsx:507-519`); `hold`, `release` (`local/client.ts:450`, `:465`); `api.remove`; `forgetUnplayable`,
  `listUnplayable` (`backup.ts`); Task 2's `VersionTag`; `SeatBot` (`types.ts`).
- Produces: `removeForeign(id)`; `leaveLabel?` on the three; `Unplayable`'s `foreign?` and `onRemoved?`;
  `localExit: {to: "lobby" | "door"; label: string}`; `Dashboard`'s `bots?`. The foreign screen's DOM: the `broken`
  screen (`.setup.broken-save`) with the server's sentence through `Refusal`, "Save game file" (LOCAL only), `Abandon`
  for the creator (local, or `created_by === signedIn && my_seats.length > 0`), the way back reading "Lobby" or
  `localExit.label`.

**Four things, one task, because they share `App.tsx`'s screens and the "Being played" words:**
1. **"Spectate" in every word a person reads** (R-6.5c2.7): the banner ("You are spectating"), the chat marker
   (" · spectating"), `Help.tsx` whole. The wire's `watching`, `watcher`, `can_watch` are protocol keys and stay.
2. **The four "Lobby" controls on LOCAL screens** (M6.5b2 close §6.3 item 5): the Game menu's first button, the
   other-tab lock screen (local-only by `lockId`), the local deal, the screen for a save that no longer rebuilds. Under
   `VITE_MULTIPLAYER=false` each leads to the DOOR and says "On this device"; a build that serves a lobby keeps all
   four (the anti-vacuity partners press "Lobby" today).
3. **A game this build's bot cannot continue** (spec §2.4; R-6.5c2.10). On the game route the 409 carrying
   `foreign_bot` is a SCREEN, not a bounce (the broken save's reason: what is left is still the player's — the
   sentence, and the Abandon that archives it); a 409 WITHOUT the field keeps the bounce; the arm reads the FIELD, never
   the status alone (`OpenTable`'s `hasStatus` rule). **Review Focus 5's third half: the refused game opens NO
   stream** — `show(null)` leaves no game state to stream for, and the cell asserts no request to
   `/api/games/wire/stream` or `…/events` after the 409 (handover §5: the SSE arm would write a comment and stay open;
   the socket arm sends nothing; the GET's body decides). In the browser the local listing's not-over foreign row goes
   under **"Will not open here"** with Back up and a two-click Remove — built from `TableView.bot.runs_here`, never a
   parse of the record (`db.ts` forbids it) — and Remove takes the MODULE's fenced delete (`hold` → `DELETE` →
   `release`), not `forgetUnplayable`'s unfenced IndexedDB path: a foreign row is the module's LISTING, loaded into
   every tab's Worker at boot, and a delete behind the Worker's back would leave it listed. Export works because it
   reads documents and never rebuilds. A FINISHED foreign game opens as ever (R-6.5c2.10).
4. **The Players panel's version tag** (spec §2.2: the game screen changes only by the words "and the version tag in
   §2.4"; mock 10's third panel "seat 1 · the bot V4"): `Dashboard` takes `bots`, `Players` renders, for `p.is_bot`,
   `<span className="tag">the bot <VersionTag label={label} /></span>` where `bots.find(b => b.seat === p.seat)` names
   it, else today's `<span className="tag">bot</span>`. The label is the RECORD's (`ClientState.bots`), so a finished
   foreign game shows its own bot's label.

- [ ] **Step 1: "Spectate" — failing cells** — `drafts/D5-tasks-13-16.md:3171-3245`: `App.test.tsx` (`:1727` and the
  spectator describe's two cells → "You are spectating"); `Chat.test.tsx` (`:101` → "· spectating" with the separator in
  the needle; `:355-359`); `Help.test.tsx` (check Task 5 renamed "Being played" in "names all three of the lobby's
  lists" — edit only if it did not; the describe "the help panel and watching (R-8.1)" REPLACED WHOLE by "the help
  panel and spectating (R-8.1; M6.5c2 R-6.5c2.7)" — 5 cells, the last the whole-panel `not.toMatch(/\bwatch/i)` and
  `toMatch(/\bspectat/i)`; the Talking label cell's word).

```bash
cd web && npx vitest run src/App.test.tsx src/Chat.test.tsx src/Help.test.tsx
```

  Expected (D5): 9 or 10 cells fail — two App, one Chat, the Help describe's five plus the Talking cell, and "names
  all three…" only where Task 5 did not rename.

- [ ] **Step 2: The words** — `drafts/D5-tasks-13-16.md:3264-3312`: `App.tsx:1308` and `:1328`; `Chat.tsx:14` and
  `:332`; `Help.tsx`'s "Spectating a game" section (the two paragraphs as written) and the Talking section's
  `a spectator's` / `<em>spectating</em>`. Expected: green.

- [ ] **Step 3: The four controls — failing, then built** — `drafts/D5-tasks-13-16.md:3320-3475`: in "a local game",
  the multiplayer partner "keeps a local game's Game-menu way out to the lobby in a build that serves one" (passes
  already) and the describe "in a single-player build" (`vi.stubEnv("VITE_MULTIPLAYER", "false")`; `leavesForTheDoor`
  asserts no "Lobby" button and "On this device" landing on `#/`; four cells: the Game menu, the other tab's lock
  screen via `hooks: {hold: () => false}`, the broken save via a 500 with the SEMANTICS_REV sentence, the local deal
  via the corpus's dealing table re-addressed). Expected red: all four at `no control says Lobby`. Then `GameMenu.tsx`,
  `Dealing.tsx` and `LockScreen.tsx` gain `leaveLabel = "Lobby"` (the prop, its doc, the button text); `App.tsx` gains
  `localExit` below `toDoor` and threads it to the lock screen, the broken save's button (superseded by Step 5's
  host-aware screen, same behaviour), the local deal, and the Game menu's `onLobby` / `leaveLabel`
  (`isLocal(state.id) ? localExit… : "lobby"`). Expected: green; `GameMenu.test.tsx:60` and `Dealing.test.tsx:346`
  still find "Lobby" (the default).

- [ ] **Step 4: The game screen's foreign refusal — failing cells (Review Focus 5)** —
  `drafts/D5-tasks-13-16.md:3477-3575`, plus ONE cell: in "the route", the describe "a game this build's bot cannot
  continue" (`dealt = {...wire.table, bot: wire.tableForeign.bot}`; `answers(table)` serving `/table` and a 409 with
  `wire.foreignBot409` on the GET; three cells: the screen with the sentence, `abandon wire` for the creator and
  "Lobby" leaving for `#/lobby`, the hash STAYING `#/t/wire`; no Abandon for a non-creator; **the anti-vacuity**: a 409
  with `wire.dealing409` bounces to `#/lobby`); and in "a local game" the local arm (the module answering 409 on the
  GET: the sentence, `#/t/local-1`, "Save game file", `/^abandon /`). **Add a fourth route cell**:

```tsx
    // **Review Focus 5's third half: the refused game opens no stream.** The
    // GET's 409 body decides the screen; the transports say nothing useful
    // about a foreign game (the SSE arm writes a comment and stays open, the
    // socket sends nothing), so no stream or events read is made for it.
    it("opens no stream and asks no events for a game it will not open", async () => {
      window.location.hash = "#/t/wire";
      const { stub } = app(answers(dealt));
      expect(await screen.findByText(refusal.error)).toBeInTheDocument();
      await flush();
      const streams = stub.calls.filter(
        (c) => c.path.startsWith("/api/games/wire/") && /\/(stream|events)(\?|$)/.test(c.path),
      );
      expect(streams, "no stream and no events for the refused game").toEqual([]);
    });
```

  (`app()` returns the stub the file's other route cells read; `flush` is the harness's. If the file's `app` helper
  hands back no stub, read `stubApi`'s handler calls the way the sibling cells do — the CONTRACT is the empty list.)

```bash
cd web && npx vitest run src/App.test.tsx -t "bot cannot continue"
```

  Expected: the two foreign-screen cells, the local one and the no-stream cell FAIL because the screen bounced (D5:
  `expected '#/lobby' to be '#/t/wire'`; the second lands on the lobby with no "Lobby" button; the local one loses the
  sentence to the door); the anti-vacuity cell passes already.

- [ ] **Step 5: The screen** — `drafts/D5-tasks-13-16.md:3577-3667`: `isForeignBot` beside `saveGameFile`; the arm in
  `open`'s catch below the local-500 arm (`e instanceof ApiError && e.status === 409 && isForeignBot(e.body)` →
  `show(null)`, `setBroken({id, table: doc, sentence: e.message})`, `tableOnScreen.current = id`, return); the
  `broken` doc's last paragraph; `onThisDevice`'s `(broken !== null && isLocal(broken.id)) ||`; the screen host-aware
  (`local`, `mayAbandon`, "Save game file" local only, `Abandon` with `onDone` → door or lobby, the way back reading
  `localExit.label` or "Lobby"). Expected: `npx vitest run src/App.test.tsx` green, the no-stream cell included — if
  it is RED, the game screen is opening a stream for a state that `show(null)` should have cleared: find the effect
  that opens the stream on `tableOnScreen` and gate it on a shown state, and say so in the report (a FINDING about
  the drafts, worth a `Ruling:` line).

- [ ] **Step 6: "Will not open here" — failing cells, then built** — `drafts/D5-tasks-13-16.md:3675-3960`:
  `backup.test.ts`'s describe "removing a game this build's bot cannot continue" (3 cells: `hold` → `DELETE` →
  `release` with the stored pair untouched; another tab's hold refused in a sentence with nothing sent; the hold given
  back on a refused delete); `OnThisDevice.test.tsx`'s describe (3 cells over `wire.tableForeign` re-addressed with
  `name: null` and only `over` moved: a not-over row under "Will not open here" with `played against V2`, Back up,
  `remove local-9`, and NO link; a FINISHED one an ordinary link; the two-click remove through the module with the
  steps `["hold", "DELETE /api/games/local-9", "release"]`). Expected red: `removeForeign is not a function`; the first
  and third OnThisDevice cells. Then `backup.ts`'s `removeForeign` (with its doc), `BackupControls.tsx`'s
  `PRODUCTION_LABEL`, `foreignWords`, `Unplayable` rewritten (`foreign`, `onRemoved`, `removeRefused`, the foreign
  rows first), `OnThisDevice.tsx`'s `foreign` filter and `<Unplayable reload foreign onRemoved={() => void read()} />`.
  Expected: green.

- [ ] **Step 7: The Players panel's version tag — failing cells, then built.** In `web/src/Dashboard.test.tsx`, a
  describe of its own (the file's `show(state, colors)` helper renders `state.frame.dashboard`; pass `bots` beside it):

```tsx
// ---- M6.5c2: the bot's version on the Players panel (spec §2.2, §2.4; mock 10)
//
// **Shown, never chosen** (R-6.5c2.10): the record's own bot seats, labelled
// (`ClientState.bots`), so a finished game against V2 says V2 while this build
// runs V4. `wire.watching` is the corpus's one state whose record has a bot
// seat; a state with no `bots` row for the seat keeps today's bare word.
describe("the bot's version on the Players panel", () => {
  it("labels the bot's seat with the record's own version", () => {
    expect(wire.watching.bots).toEqual([{ seat: 0, label: "V4" }]);
    render(
      <Dashboard
        panels={wire.watching.frame.dashboard}
        art={wire.artManifest}
        labels={wire.labels}
        colors={COLORS}
        only={["Players"]}
        bots={wire.watching.bots}
      />,
    );
    const seat = document.querySelectorAll(".panel .seat")[0]!;
    expect(seat.querySelector(".tag .version-tag")).toHaveTextContent("V4");
    expect(seat.querySelector(".tag")).toHaveTextContent("the bot");
    // The other seat is a person: no tag of either kind.
    expect(document.querySelectorAll(".panel .seat")[1]!.querySelector(".version-tag")).toBeNull();
  });

  it("keeps the bare word where the state names no version for the seat", () => {
    render(
      <Dashboard
        panels={wire.watching.frame.dashboard}
        art={wire.artManifest}
        labels={wire.labels}
        colors={COLORS}
        only={["Players"]}
        bots={[]}
      />,
    );
    const seat = document.querySelectorAll(".panel .seat")[0]!;
    expect(seat.querySelector(".version-tag")).toBeNull();
    expect(seat.querySelector(".tag")).toHaveTextContent("bot");
  });
});
```

  The premise, VERIFIED at `f2a3dd61`: `wire.watching.frame.dashboard` is four one-key panels (`Players`, `Tracks`,
  `Achievements`, `Hand`), and its Players panel reads `[(seat 0, is_bot: true), (seat 1, is_bot: false)]` beside
  `bots: [{seat: 0, label: "V4"}]`. The file's own `playersOf(wire.watching)` (`web/src/test/panels.ts:19`) is the
  reader to assert it with, in the cell's first line, beside the `bots` assertion above. Expected red: `bots` is an
  unknown prop and no `.version-tag` is drawn. Then `Dashboard.tsx`: the prop `bots?: readonly SeatBot[]` with its
  doc, threaded to `Players`; at `:186` the `is_bot` tag becomes the labelled form when
  `bots?.find((b) => b.seat === p.seat)` is found; `VersionTag` imported from `./Chips`. `App.tsx`'s `<Dashboard …>`
  (the side column, `:2029`) gains **`bots={state.bots}`** — `state`, the live `ClientState`, and NOT `view`
  (`const view = walking ? stepNow.after : scrubbing ? past : state;`, `:1577`): the record's bot seats do not change
  with the position walked, and a replayed frame is not where they live. Expected: green, and `Dashboard.test.tsx` +2.

- [ ] **Step 8: The suites.**

```bash
cd web && npm run typecheck && npm test
```

  Expected: **+18 cells** over Task 7's (D5's +16: App +5 single-player, +3 foreign, +1 local foreign; Help +1;
  OnThisDevice +3; backup +3 — plus this plan's +1 no-stream and +2 Dashboard; the total is a prediction; re-count).

- [ ] **Step 9: The browser walks** — `drafts/D5-tasks-13-16.md:3978-4003`: `two_browsers.spec.ts:223` and
  `exit_game.spec.ts:399` ("You are spectating"; the comment at `:390-391`); `two_browsers.spec.ts:252`'s
  `.toContainText("· spectating")` with its comment (C's typed message "watching you both" stays: a person's words,
  not the client's; `joins_by_link.spec.ts:311`'s "eleven" is a recorded measurement and stays). Background to
  `task-8-e2e.txt`, POLL. Expected: green at Task 7's count. No walk builds a single-player bundle, so the four
  controls are held by Step 3's jsdom cells — recorded here rather than implied.

- [ ] **Step 10: The headed look.** Two pictures by hand in an export: the game screen's Players panel on a vs-bot
  local game (the tag beside the bot's seat, mock 10's third panel) and the door's "Will not open here" block — the
  latter needs a foreign row, which only a stubbed listing gives (a `page.route` on `/api/games` in the export);
  into `task-8-look/`, named in the report. The foreign screen on the game route is jsdom-held (the real server
  cannot deal a foreign bot).

- [ ] **Step 11: The per-edit gates.** `npm run typecheck`; `npm test` (+18); `test:wasm` unmoved; the e2e; the Rust
  and `worker/` gates unmoved; both fences empty.

- [ ] **Step 12: Commit.**

```bash
git add web/src/App.tsx web/src/App.test.tsx web/src/GameMenu.tsx web/src/Dealing.tsx web/src/LockScreen.tsx \
  web/src/Chat.tsx web/src/Chat.test.tsx web/src/Help.tsx web/src/Help.test.tsx \
  web/src/OnThisDevice.tsx web/src/OnThisDevice.test.tsx web/src/BackupControls.tsx web/src/backup.ts \
  web/src/backup.test.ts web/src/Dashboard.tsx web/src/Dashboard.test.tsx \
  web/e2e/two_browsers.spec.ts web/e2e/exit_game.spec.ts
git commit -m "feat(web): spectate in every word, local screens leave for the door in a single-player build, foreign-bot games listed and shown rather than opened, and the bot's version on the Players panel"
```

---

### Task 9: The client's small items (spec §2.8 "Task 8's items"; L-6 by R-B3; §2.4's browser-side `run` pins) — from D5 retired Task 16

**Files:**
- Create: `web/src/test/fakeSocket.ts` (the ONE fake socket, hoisted from `ws.test.ts:20-134`);
  `web/src/devProxy.node.test.ts`.
- Modify: `web/src/ws.test.ts` (the header; the class removed and imported); `web/src/signal.test.ts` (the second fake
  removed and imported; +2 cells in "the signal, over a WebSocket (M6.5c)"; battery 37 reads the word while hidden);
  `web/src/signal.ts` (the probe bounded, and dropped when its attempt was aborted); `web/src/api.ts` (`bounded`
  clears its timer); `web/src/api.test.ts` (+2); `web/src/App.tsx` (`sendToArchive`'s catch remembers "off" on 404)
  and `web/src/Abandon.tsx` (`sendFirst`'s catch likewise), with a cell each; `web/src/outbox.ts` (L-6: `attempt`
  stops after the first 404; the header's rule 8 amended) and `web/src/outbox.test.ts` (+2); `web/vite.config.ts`
  (`ws: true` on `/api`); `web/src/backup.test.ts` (+4: the `run` block through the browser's three save paths).

**Interfaces:**
- Consumes: `rememberArchiveOff`, `archiveAnswer` (`local/facts.ts:301`, `:224`); `BOUND_MS`, `bounded`
  (`api.ts:283-294`); `FakeSocket` as `ws.test.ts` defines it; `RECORD_TEXT`, `putPair`, `readBack`, `tableFor`,
  `backupOne`, `restorePairs`, `Envelope` (`backup.test.ts`'s own helpers); Task 4's `setLink(lost())` at the
  own-close site (the battery-37 mutant).
- Produces: `web/src/test/fakeSocket.ts`'s `export class FakeSocket`; `bounded(p, ms)` keeps its signature and its
  timer no longer outlives `p`; `web/vite.config.ts`'s `/api` entry `{target, changeOrigin: false, ws: true}`.

**Where the `run` pin runs, decided** (spec §2.4, §3): vitest, with the fake Worker and `fake-indexeddb`. The three
browser save paths are PAGE code that never parses a record: this database (`db.ts`), "Save game file"
(`api.recordText` → `localPair`), the backup envelope (`buildEnvelope` → `restorePairs`). The module's own write
(`run_id: "web/browser"`) is cycle A's Task 5 pin. The committed envelope's record already carries a `run` block with
two u64s past 2^53 (`agent_seed` 10851594032870736878, `weights` 6957643967275421640), so the needle is the corpus's.

- [ ] **Step 1: ONE fake socket** — `drafts/D5-tasks-13-16.md:4058-4096`: create `fakeSocket.ts` with `ws.test.ts`'s
  class verbatim under the header given, exported; `ws.test.ts` and `signal.test.ts` import it and delete their
  copies (keep `signal.test.ts`'s section comment). `npx vitest run src/ws.test.ts src/signal.test.ts && npm run
  typecheck`: every cell passes at the SAME count (`grep -c "^  it(" …` before and after — a refactor proves itself by
  moving nothing).

- [ ] **Step 2: A probe answer after its attempt was aborted — failing** — `drafts/D5-tasks-13-16.md:4098-4138`: the
  cell "drops a probe answer that lands after a hidden tab aborted its attempt" (D5 expected red: `a hidden tab is
  handed nothing: expected [ 4, 5 ] to deeply equal [ 4 ]`).

- [ ] **Step 3: The probe bounded — failing** — `drafts/D5-tasks-13-16.md:4140-4213`: the signal cell "bounds the
  probe: one that never answers climbs the ladder after BOUND_MS" (red: `socket 2 was opened never became true`) and
  `api.test.ts`'s `bounded` describe (the first cell red: `expected 1 to be +0`, the timer outliving the read; the
  second passes already).

- [ ] **Step 4: The probe** — `drafts/D5-tasks-13-16.md:4215-4269`: `api.ts`'s `bounded` clearing its timer on either
  settlement; `signal.ts`'s probe `await bounded(src.current.once(), BOUND_MS); if (stopped || mine.signal.aborted)
  return;` with the comment. Expected: `npx vitest run src/signal.test.ts src/api.test.ts` green; battery 23 holds
  `vi.getTimerCount() === 0`.

- [ ] **Step 5: The link word while the tab is hidden** — `drafts/D5-tasks-13-16.md:4271-4296`: battery 37's two
  assertions ("hidden, the word is still live"; "and still live a minute later"). **Prove the mutant**: move Task 4's
  `setLink(lost())` at the own-close site ABOVE `if (mine.signal.aborted) return;`, run `-t "battery 37"`, expected
  `hidden, the word is still live: expected 'reconnecting' to be 'live'`; revert; `git diff web/src/signal.ts` shows only
  Step 4's edit. Quote the red.

- [ ] **Step 6: The two self-posting sites remember "off" — failing, then built** —
  `drafts/D5-tasks-13-16.md:4298-4389`: `Abandon.test.tsx`'s "remembers the archive OFF when the send meets the
  route's 404, and keeps the game" and `App.test.tsx`'s "…when Send to the archive meets the route's 404, and stops
  offering it" (both red at `expected 'dir' to be 'off'`); then `if (e instanceof ApiError && e.status === 404)
  rememberArchiveOff();` in `Abandon.tsx`'s `sendFirst` catch and `App.tsx`'s `sendToArchive` catch — on the STATUS,
  never the sentence (the outbox's own rule).

- [ ] **Step 7: L-6 — failing, then built (R-B3)** — `drafts/D5-tasks-13-16.md:4391-4457`: `outbox.test.ts`'s "stops
  posting after the first 404 in a drain, and forgets every waiting row" and "…in the sweep too" (red on the upload
  count, 2 for 1); then `attempt`'s `if (kind === "off" || archiveAnswer() === "off") { await forget(db, id); return;
  }` with its comment, and rule 8's sentence appended.

- [ ] **Step 8: The dev proxy's upgrades — failing, then built** — `drafts/D5-tasks-13-16.md:4459-4510`: create
  `devProxy.node.test.ts` (resolves the exported config for `serve`/`development` and expects `/api` to match
  `{target: "http://127.0.0.1:8080", ws: true}`; red naming `"ws": true`); then `vite.config.ts`'s `/api` entry with
  its comment (Task 3's `define` does not touch the proxy).

- [ ] **Step 9: `run` through the browser's three save paths** — `drafts/D5-tasks-13-16.md:4512-4583`:
  `backup.test.ts`'s describe (4 cells: the premise — two u64s no JavaScript number holds and a parse-and-reserialize
  would change the bytes; the database byte for byte; `api.recordText` is the stored text; a backup and a restore onto
  a fresh device — a new `IDBFactory`, `resetLocalHostForTests`, `stubWorker` — byte for byte). They pass at HEAD.
  **Prove both mutants** (D5 measured each fails exactly its cell): `buildEnvelope`'s `record: p.record` →
  `JSON.stringify(JSON.parse(p.record), null, 2)` fails the backup cell; `api.ts`'s `recordText` local arm `return text`
  → the same round trip fails the Save game file cell. Revert both; `git diff web/src/backup.ts web/src/api.ts` shows
  only Step 4's `bounded` edit and Task 8's `removeForeign`. Quote both reds.

- [ ] **Step 10: The suites.**

```bash
cd web && npm run typecheck && npm test && npm run test:wasm
```

  Expected: `npm test` **+12** (D5: signal +2, api +2, Abandon +1, App +1, outbox +2, backup +4; `ws.test.ts` and
  `signal.test.ts` hold their counts); `test:wasm` **+1 cell, +1 file** (`devProxy.node.test.ts`).

- [ ] **Step 11: The browser suite.** Background to `task-9-e2e.txt`, POLL. Expected: green at Task 8's count. The
  local walk serves "sse", so it proves no regression rather than the bounded probe itself (the socket arm is held by
  Steps 2–5's cells; its live reading is cycle C's).

- [ ] **Step 12: The per-edit gates.** The Rust suites untouched (`git diff --stat f2a3dd61 -- crates` prints only
  Task 1's test file); `npm run typecheck`, `npm test` (+12), `test:wasm` (+1/+1), the e2e; the `worker/` gates unmoved;
  both fences empty.

- [ ] **Step 13: Commit.**

```bash
git add web/src/test/fakeSocket.ts web/src/ws.test.ts web/src/signal.ts web/src/signal.test.ts \
  web/src/api.ts web/src/api.test.ts web/src/App.tsx web/src/App.test.tsx web/src/Abandon.tsx web/src/Abandon.test.tsx \
  web/src/outbox.ts web/src/outbox.test.ts web/vite.config.ts web/src/devProxy.node.test.ts web/src/backup.test.ts
git commit -m "fix(web): M6.5c Task 8's minors — the probe bounded and dropped when aborted, the hidden tab's word, the two 404 sites, one fake socket, ws on the dev proxy — L-6, and the run block pinned through every browser save path"
```

  The body quotes every red run: the aborted probe, the unbounded probe, `bounded`'s timer, the two 404 sites, L-6's
  two counts, `ws`, battery 37's mutant, and the `run` pin's two mutants.

---

### Task 10: The cycle's close (ORCHESTRATOR)

No code. The orchestrator's steps, in order. **The push and the PR are PRE-AUTHORIZED** (the owner, 2026-09-27:
*"push and PR are pre-approved at the orchestrator's discretion"*); nothing here waits on the owner. **Step 5's
handover is the ENTRY for cycle C's planning session**: a fresh orchestrator writes cycle C's plan from it, the spec's
§4 row C, drafts D6–D8 and the standing items, against this branch's real tree — so it is written for that reader.

- [ ] **Step 1: The Fable whole-branch review.** One Fable reviewer over `git diff f2a3dd61..HEAD`, the spec, this plan,
  the mocks and the ledger, working in an EXPORT: the vacuity lens first (every new catcher reverted against its fix —
  the name rule's Rust-trim rows, the seed rule's two trim rows, `isForeignBot`'s field test, the no-stream cell, the
  one-click join, `removeForeign`'s hold, the `run` pin's two mutants, battery 37's order); **the pictures**: every
  `task-<n>-look/*.png` read beside its mock, and any screen the tasks could not photograph (the four identity states,
  the single-player build, the foreign screen) rendered once by the reviewer in the export and read; the DOM contract
  the walks read; nothing hand-authored (every rendered payload traced to a corpus file or a one-field derivation the
  cell names); the fences; the sentences this plan writes about itself tried rather than re-read (the top yield, every
  cycle). Its report goes to the ledger.
- [ ] **Step 2: ONE fix wave**, each fix a NEW commit by an Opus implementer, reviewed by an Opus reviewer, then a scoped
  Fable re-review of the fixes. A reviewed SHA is never amended. A fix that changes what a browser shows owes the e2e,
  sequenced.
- [ ] **Step 3: The per-edit suites at the reviewed head**, recorded in the ledger with the machine tag: `cargo test
  -p yokohama-web` (the pin at 4), `-p yokohama-wasm`, `-p yokohama-worker`; `cargo clippy --workspace --all-targets --
  -D warnings`; `web/`'s four commands, the e2e ONE MORE TIME at the reviewed head (68 in 13 is the prediction: 61 +
  the door, the name step, the lobby, the four forms cells); `worker/`'s three; BOTH fence commands printing nothing.
  **No release gates, no deploy, no live tier** (R-6.5c2.13: the full sequence is cycle C's).
- [ ] **Step 4: The push and the PR, at the orchestrator's discretion.** `git push -u origin web-room-m6.5c2b`, then
  `gh pr create --base web-room-m6.5c2a --head web-room-m6.5c2b` with a description that is the cycle in one screen:
  what landed by task, the readings at the reviewed head, the eleven mocks and which task answers to each, what the
  cycle does NOT claim, ending with the attribution line. Record the PR's number in the ledger and on the board; the
  spec's §4 table says "expected #40", and the number GitHub gives is the record.
- [ ] **Step 5: The handover into cycle C — its planning session's entry.** Write
  `docs/plans/<date>-web-frontend-m6.5c2c-handover.md`, under 200 lines, every fact read off the TREE at the reviewed
  head, never off this plan. Its sections, in order:
  1. **Where cycle B left the tree:** the branch, its HEAD, the PR; every suite's reading at the reviewed head,
     machine-tagged; both fence checks' (empty) output; the e2e runs' lines (one per task and the close's).
  2. **The client as it IS**, which cycle C ships and deploys: the screens by file with the mock each answers to; the
     accessible names the live specs and the exit spec now read (`"Join as Blue"`, `"Spectate"`, `"Table password"`,
     `"Invite link"`, `"Copy link"`, `"New table"` at level 3, `"New game against the bot"`, `"Choose a name"`,
     `"Playing as"`, `"switch player"` / `"change name"`, `"You are spectating"`, `" · spectating"`) — so C's live tier
     and headed exit are run against the words the walks were rewritten to; `nameThisBrowser` and the display-name
     key; `YOKOHAMA_PROJECT_URL` as a build variable the Worker's `cd web && npm run build` and the Docker `ARG` take
     (C's RUNBOOK recipe names the value used); `production_bot.json` as the one place the door's label comes from
     (a re-point regenerates it through `wire_fixtures`).
  3. **What cycle C builds** (spec §4 row C, restated in one paragraph) **and its raw material:** drafts D6 (retired
     Tasks 17–19), D7 (20–23) and D8 (24–28) under the same `drafts/` directory, with the assembly's notes and
     `review-plan-opus.md`; the retired plan review's P-9 (the secret's hand-off: ONE `wrangler deploy --secrets-file`
     at the close, nothing before) and P-10; cycle A's §7 standing items VERBATIM, each marked done here or still open
     (the R-A19 witness is DONE, Task 1); the client's `claimed=` upgrade query parameter under `open` (§2.1 "The
     claim's carriers"), which is the ONE client change C makes; and the words the live specs read (item 2), because C
     runs them.
  4. **The rules cycle C's plan carries:** the standing set (handover §4, minus the screen rules), plus: every account
     step is the orchestrator's; the quota guard before any live tier; the live replay's reset recipe BEFORE its first
     run (owed since A's Task 5 and owed again by the bump); the two pool projects for the two doors; the deploy
     guard's per-door rules; the close in M6.5c's order (spec §3).
  5. **What cycle B does NOT claim:** the four identity states, the single-player build's four controls and the
     foreign screens are jsdom-held and hand-photographed, not walked (no local server reaches them); the bounded probe
     is cell-held (the local walk serves "sse"); the live specs and the exit spec were REWORDED and type-checked, not
     run; `exit_game`'s "one link per table" is read at C's live run; the mocks are a design, not a pixel contract, and
     each task's report says where its screen differs; whatever the whole-branch review left open, named.
  6. **Every ruling taken in this cycle's ledger**, one line each, dated (R-B1–R-B4 and every `Ruling:` line).
  7. **Cycle C's standing items this cycle created or found**, appended to A's §7 list: anything a task's report or
     the review named for C (a stream opened for a refused game, if Task 8's Step 5 found one; a corpus row a screen
     wanted and derived instead; a walk that measures under the old words).

  Then: `docs/handover.md`'s "Updated" line and Track 3 point at the handover and say cycle C is next; the
  orchestrator's memory (`yokohama-web-lane-status`) records the cycle's end and the PR number. Commit the handover
  and the board together; push (pre-authorized).
- [ ] **Step 6: Nothing else.** The documents, CLAUDE.md's counts, the decision-log rows, the deploys and the version
  bump are cycle C's.

---

## Appendix A — the drafts' line map, and the renumbering

The drafts were written for a 28-task plan and refer to each other by THOSE numbers. Reading a draft, translate:

| a draft says | it means here | the draft's lines |
|---|---|---|
| Task 1 (D1) — the key grammar, `identity_cases.json` | cycle A, DONE (`60b94fad`) | — |
| Task 2 (D1) — the rename, the mode rule, `Me.mode` | cycle A, DONE (`b27acf11`) | — |
| Task 3 (D2) — `TableView.bot`, `ClientState.bots`, `production_bot.json`, `table_bot.json` | cycle A, DONE (`f1e80e46`, `46103a72`) | — |
| Task 4 (D2) — the foreign 409, `table_foreign.json`, `why_foreign.json` | cycle A, DONE (`860c00d5`) | — |
| Task 5 (D2) — `run_id` | cycle A, DONE (`552906bf`) | — |
| Task 7 (D3) — `seed_cases.json` | cycle A, DONE (`605c3ddd`) | — |
| Task 9 (D4) — the shared pieces | **Task 2** | `D4-tasks-09-12.md:109-1130` |
| Task 10 (D4) — the door and the project link | **Task 3** | `D4-tasks-09-12.md:1132-2463` |
| Task 11 (D4) — the identity states, the name step, the band | **Task 4** | `D4-tasks-09-12.md:2465-3968` |
| Task 12 (D4) — the lobby's lists | **Task 5** | `D4-tasks-09-12.md:3970-4891` |
| Task 13 (D5) — the forms and the seed check | **Task 6** (Steps 7–23 of the draft; its Steps 1–6 and Step 9's `contract.ts` edits are cycle A's) | `D5-tasks-13-16.md:84-2061` |
| Task 14 (D5) — waiting, joining, the password screens | **Task 7** | `D5-tasks-13-16.md:2063-3098` |
| Task 15 (D5) — Spectate, the four controls, "Will not open here", the foreign screen | **Task 8** (+ the Players panel's tag and the no-stream cell, this plan's) | `D5-tasks-13-16.md:3100-4018` |
| Task 16 (D5) — the client's small items | **Task 9** (+ L-6 by R-B3, which the draft already includes) | `D5-tasks-13-16.md:4020-4630` |
| "final Task 10 / 11 / 12" (the retired merge map) | Tasks 6 / 7 / 8+9 | — |
| Task 24 (D8), Task 28 (D8) — the Docker build, the close's live tiers | cycle C | — |

**The drafts' base is `1e16fe27`**; cycle A moved the client files named in Global Constraints. **Their stand-ins for
cycle A's outputs are superseded by the tree**: where a draft's cell stubs `Me.mode`, `TableView.bot`,
`production_bot.json`, `identity_cases.json` or `seed_cases.json`, the tree has the real thing, and the plan's
Interfaces name it. **Their test people**: `Ben Moore` / `b3n_m-o.re` / `Jeremy` / `Jem` are replaced as each task
says. **Their "Expected" counts** are the export's readings and are stated here as deltas; every implementer re-counts.

The rendered screens D4's code drew — `…/d4-look/{door,lobby,name,state-A..D,states}-{1440,390}.png` — are the closest
thing to a pre-run headed look for Tasks 3, 4 and 5; read them beside the mocks before the task's own pictures exist.
The mocks' HTML sources and the two font files are in `…/mocks-html/`; the brainstorm's nineteen `check-*.png` review
shots (the iterations the owner approved from) in `.superpowers/brainstorm/`.

## Appendix B — the ledger, and how to use it

`.superpowers/sdd/2026-09-27-web-frontend-m6.5c2b-plan/` (gitignored):
- `progress.md` — the SDD identity line (`# SDD ledger — plan: docs/plans/2026-09-27-web-frontend-m6.5c2b-plan.md`),
  the standing rules, Step 0's baseline, one line per task with its readings and its report paths, one `Ruling:` line
  per deviation (R-B1–R-B4 recorded at the top).
- `task-<n>-brief.md`, `task-<n>-report.md`, `task-<n>-review.md` — every dispatched agent's brief and full report.
- `task-<n>-e2e.txt` — every Playwright run's output, written by the implementer and POLLED; the file is the evidence
  of the run's count and wall.
- `task-<n>-look/` — the headed look's pictures for Tasks 3–8, named for the screen and width, beside a one-line
  `README` naming the mock each answers to and what differs.
- `final-review.md`, `fix-wave-*.md`, `final-rereview.md`, `pr-body.md`, `handover-draft.md` — the close's.
- Cycle A's ledger (`…/2026-09-27-web-frontend-m6.5c2a-plan/`) holds `final-rereview.md`, whose "Important 1,
  precisely" is Task 1's specification and measured red; and the retired plan's workspace
  (`…/2026-09-26-web-frontend-m6.5c2-plan/`) holds the drafts, `review-plan-opus.md`, `d4-look/`, `mocks-html/` and
  `drafter-artifacts/` (D4's component files under `d4files/`, D5's seed crate under `seedcrate/`, `nameRule.mjs`).
