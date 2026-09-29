# M6.5c2 cycle E — the finish: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan
> task-by-task. The execution shape is §"Execution shape" below: every code and document task subagent-driven, the
> implementer's tier marked in each task's title (**Sonnet 5.5** for the implementation-focused tasks, **Opus** where
> judgment is needed, R-6.5c2.36), every task reviewer Opus, Fable for the whole-branch review; every account step — a
> deploy, a live run, an Access change, a rate-limit change — the ORCHESTRATOR's alone, at will (R-6.5c2.37). Steps use
> checkbox (`- [ ]`) syntax. **Read the spec first** (revision 2.9: §10.11 is this cycle's rulings and §4's row E as
> amended), then the dossier `docs/plans/2026-09-29-web-frontend-m6.5c2e-dossier.md` — **every task cites its items
> there by number for WHERE the code is and what it reads NOW** — and carries the design, the tests and the steps itself.
> **Read §"Couplings between tasks" before any task**: an implementer sees only its own task.

**Goal:** The web lane finished: the update path fixed first (the shell precached from a static asset, so a returning
browser is no longer pinned to its last release), D's leftovers closed, the door's three build values, the owner's-eye
polish off pictures, lobby paging and "Abandon and save" made true, the waiting-table expiry, requests per move
bounded, the door window's instruments; then, against E's own release, the update reading on both hosts, the live
tiers, the ten screenshots, the phones on the simulators, the rate-limit rule at a measured value, and the public
door vetted in two states then REVERTED; then the documents — the lane document, `CLAUDE.md`'s web passages to
pointers, the guidingpath archive by grouping, the board, the README in R-6.5c2.45's shape; then the close, PR #43,
and on the owner's word at the wrap the merge #31 → #43 and `v0.6.0`.

**Architecture:** Three parts in order, because each later part reads what the earlier one leaves. **Part 1, the
code** (Tasks 1–8), lands ONE release: the service worker's shell fetch first (1), since every later browser reading
depends on a release that installs; the leftovers (2) and the build values (3) before the polish tasks (4a, 4b), which
photograph the screens as 3 leaves them; the wire changes (5, 6) before the requests task (7), which counts the fetches
5 and 6 leave; the window's instruments (8) last, since they read the router and the configurations as the cycle leaves
them. Then the review, one fix wave, the battery and the ONE recorded sequence (9). **Part 2, the live readings**
(Task 10, the orchestrator's), against the deployed release, the door window LAST because it takes the dev door down.
**Part 3, the documents** (Tasks 11–15), written from Part 2's readings, then the close and the wrap (16).

**Tech Stack:** Rust 1.97.1 (`crates/yokohama-web`, `crates/yokohama-worker`, `crates/yokohama-wasm`; `wasm-bindgen`
0.2.128, `release-wasm`, `wasm-opt` 133); TypeScript in `web/` (React 18, Vite, vitest over jsdom, Playwright over
Chromium) and `worker/` (the shell; vitest node cells; the POOL inside workerd, two projects); the owner's Cloudflare
account through the Cloudflare MCP and `wrangler` 4.136.3; the NAS through Portainer's Docker proxy; the iOS
simulators and the Android emulator through the mobile bridge (`mcp__mobile__*`).

**Spec:** `docs/plans/2026-09-26-web-frontend-m6.5c2-spec.md` at **revision 2.9** (`97c69909` + `e93ab965`): §10.11 —
the update-path defect as measured; R-6.5c2.37 (the authorization), .38 (the polish is the orchestrator's call), .39
(the door in two states then reverted), .40 (the three build values), .41 (the shell as a static asset), .42 (rows 74,
83, 91 by design; the waiting-table expiry), .43 (the rate limit at a measured value), .44 (the Access changes
automated), .45 (the README's shape) — and §4's row E as amended. The E handover
`docs/plans/2026-09-29-web-frontend-m6.5c2e-handover.md` is the entry; the straw poll
`docs/plans/2026-09-28-web-lane-straw-poll.md` numbers the rows; the dossier is the file:line authority. **How this
plan was built:** by the orchestrator alone, from the dossier and the records, with D's stop rule — a task past ~250
lines is two tasks, a plan past ~2,500 lines is split before it is finished.

---

## Global Constraints

- **Branch `web-room-m6.5c2e`**, stacked on `web-room-m6.5c2d`; **the base for every code diff is `c06c4b36`** (cycle
  D's branch head: its code is the reviewed head `4212f6a0`, its two documents commits on top). The branch's first
  commits are the spec's revision 2.9 and this plan (with the dossier). Every `file:line` in the dossier was read at
  `e93ab965`, the same tree: **find by content once an earlier task has moved a file.**
  - Stage with explicit paths, never `git add -A`. `git status` is clean after every commit.
  - **The push, PR #43, the deploys, the live tiers and the Access and rate-limit changes are the orchestrator's at
    will** (R-6.5c2.37). **The merge and the tag are NOT**: Task 16.H asks the owner at the wrap.
- **Fences** (spec §5). At every commit this prints nothing:
  `git diff --stat c06c4b36 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli`.
  `SEMANTICS_REV` stays **7**. No bot behaviour moves.
- **The corpus moves in NAMED tasks only:** Task 2 (the house-frame fixture), Task 5 (`TableView.saves`, every payload
  carrying a table), Task 6 (`health.json`'s two TTL fields). Each regenerates with
  `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, READS the diff, and states in its report
  which files moved and why; `git diff --stat c06c4b36 -- web/src/wire web/src/wire-script` at the close lists only
  those. **The corpus's seed pin, password pins and watcher pin stay green at every commit.**
- **The per-edit gates, after EVERY task** (`CLAUDE.md`'s Commands, the readings the close re-counts): `cargo test -p
  yokohama-web` (393 + the task's own), `-p yokohama-wasm` (9), `-p yokohama-worker` (75), the eight `identity::`
  cells, the `wasm32` check, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo doc -p yokohama-web
  --no-deps` and `-p yokohama-worker --no-deps` at 0 warnings; in `web/`: `npm run build:wasm` then `npm run typecheck`,
  `npm test` (1292 in 64), `npm run test:wasm` (134 in 11); in `worker/`: `npm run build:module:debug` FIRST, then
  `npm run typecheck`, `npm run test:scripts` (396 in 21), `npm test` (the pool, 221 in 11). **`npm run e2e` (72 in
  13) is owed by every task that changes what a browser does or shows** — Tasks 1, 3, 4a, 4b, 5, 7 and 8 — run ONE at
  a time, in the foreground or polled, never under a Monitor; a red cell re-runs ALONE before it is believed.
- **Every invisible non-ASCII code point in code is a `\u` escape** (M6.5c2). No document spells the owner's address
  (R-6.5c2.16): readings say "the owner's address". No secret value is printed, copied or committed: the credential
  in `.dev.vars`, `DEV_DOOR_CLIENT_ID`, `YOKOHAMA_WEB_ADMINS`, the Portainer token, any cookie.
- **Sonnet 5.5 implements a task whose brief carries the judgment already made** (Tasks 2, 3, 8, 14, 15 and the phones);
  a Sonnet implementer that meets a design choice STOPS and asks (NEEDS_CONTEXT to the orchestrator). Opus implements
  Tasks 1, 4a, 4b, 5, 6, 7, 11, 12 and 13. **Every reviewer is Opus**, mutants on an EXPORT (`git archive`), never in
  the implementer's tree. Fable reviews the whole branch (Task 9).
- **Reports to files**, never the notification: `$L/task-<n>-report.md`, three lines in the reply. **A fix is a NEW
  commit** — once a SHA is REPORTED, every change is a new commit, reviewed or not, pushed or not. **Nothing runs in an
  implementer's tree** while it works, not even read-only git. **No Monitor, no event waits**: a long run is polled
  by reading its output file; the orchestrator checks the ledger within ~15 minutes of any long run.
- **The wire corpus's rule, restated for the two client tasks that render** (4a, 4b, 5): every payload a jsdom cell
  renders comes from `web/src/wire/`; nothing is hand-authored, and the fake Worker answers from the corpus.

---

## Review Focus

The spec implies these five inputs, and no cell would meet them unless someone put it there. Each line says what a
person would expect; its test lives in the owning task and is marked **(Review Focus N)** there.

1. **A returning visitor whose browser is controlled by D's release and whose clearance cookie is stale, opening E's
   release.** Expected: the new release installs, the door shows the link and the blurb, and nothing says "Try again".
   (Task 1: the inventory's no-HTML-entry cell and the navigation-answer cell; Task 10.B's update reading on both
   hosts is the live proof.)
2. **A table that waited seven days with a seat still open, beside a playing table idle seven days.** Expected: the
   waiting one is gone with a marker that says it waited for players; the playing one is untouched until its ninety
   days. (Task 6: the two-deadline pool cell.)
3. **A house with twenty-one open tables, or twenty-one games being played.** Expected: the twenty-first is reachable
   through "more" under its own list, and the poll never re-fetches a page already held. (Task 5: the paging cells
   for both scopes.)
4. **Under state 1, a stranger typing another person's address as their name.** Expected: under `open` that name IS
   them — the open door's trust model, R-6.5c2.3 — and it grants no operator right, because the operator secret is
   not there and the shell refuses to start with it. (Task 8: the `wrangler.open.jsonc` guard cells; Task 10.G's
   reading.)
5. **The game screen at 390 px with the footer set.** Expected: no horizontal overflow, the table never covered, the
   footer under the dashboards, one line. (Task 3: the 390 px overflow cell with the notice set, and the 44 px sweep
   unchanged.)

---

## Execution shape (the owner's standing answers, and this plan's rulings)

- **Mode: SUBAGENT-DRIVEN, EVERY TASK.** Implementers: **Sonnet 5.5** for Tasks 2, 3, 8, 14, 15 and the three phone
  agents; **Opus** for Tasks 1, 4a, 4b, 5, 6, 7, 11, 12, 13. **Every task reviewer Opus.** Fable: the whole-branch
  review (Task 9). A Sonnet brief carries the judgment already made: what to change, what proves it, what not to touch.
- Tests are written first and RUN RED; the red run is quoted in the commit body and the ledger. Every commit passes
  every per-edit gate.
- **The orchestrator edits no code.** Its steps: Task 1 Step 0's baseline; the pictures handed to 4a and 4b as files;
  the requests target ratified at Task 7's NEEDS_CONTEXT; Task 9 and Task 10 whole; Task 16 whole.
- **Rulings at planning (2026-09-29, the orchestrator, under spec §10.11):**
  - **R-E1:** the spec's nine code tasks keep their numbers (1, 2, 3, 4a, 4b, 5, 6, 7, 8); Task 9 is the review and the
    sequence; Task 10 the live readings; Tasks 11–15 the five document tasks; Task 16 the close and the wrap.
  - **R-E2:** the base for every diff is `c06c4b36`; the corpus is regenerated only by Tasks 2, 5 and 6.
  - **R-E3 — the shell asset:** `/shell.bin`, `application/octet-stream` on both hosts by extension; the inventory lists
    it in place of `/index.html`; the worker answers a navigation from its cached bytes with `content-type: text/html;
    charset=utf-8` set by itself; `/index.html` stays the page every host serves for a navigation, unchanged.
  - **R-E4 — the three build values** share one checker module and one `define` shape: `YOKOHAMA_PROJECT_URL` as today,
    `YOKOHAMA_SITE_BLURB` (plain text, at most 400 characters), `YOKOHAMA_SITE_NOTICE` (plain text, at most 200); unset
    or empty draws nothing; a control character or an overrun fails the BUILD naming the variable, as the link does.
  - **R-E5 — `TableView.saves: bool`**, computed by the layer from the SAME condition its `delete` files a save on
    (the mirror gains `events`, the fact the condition needs), is what the client renders "Abandon and save" from for a
    hosted table; for a local game the client's own rule adds `events > 0`.
  - **R-E6 — paging:** the lobby pages the `open` and `playing` scopes exactly as it pages `mine` — a cursor per
    scope, a "more" per scope while `next` is non-null (the `playing` scope's after the Finished fold, since Being
    played and Finished share it), the poll replacing page one only.
  - **R-E7 — the arrival:** a cold load of `#/t/<id>` draws an "opening…" screen while the table is fetched and then
    the table's screen, the lobby never (the M6.3 close's own fix); the lists load only where the lobby is the screen.
  - **R-E8 — the waiting-table TTL** is `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS`, default `7`, refused like the first TTL.
    The status is not readable when the alarm is armed (the strings rule), so the alarm is ARMED at the shorter
    deadline and DECIDED at the alarm by the status `copyFacts` reads (the one sanctioned parse): `open` expires with
    the marker head that says the table waited for players; anything else re-arms at the long deadline. `/api/health`
    carries `table_ttl_days` and `open_table_ttl_days` (null on the NAS, which keeps no TTL, R-6.5c2.2), which both
    replays' `HOST_FIELDS` exempt.
  - **R-E9 — requests per move:** the target is set from Task 7's by-route breakdown and ratified by the orchestrator at
    the task's NEEDS_CONTEXT, never before; the same tally yields the per-10-second peaks R-6.5c2.43 needs.
  - **R-E10 — the window's configurations:** a committed `worker/wrangler.open.jsonc` (the `open` door, no dev door, no
    Access values) under the guard's own door rules and its drift checks; state 2's "dev door off" is a `--var` deploy
    if `wrangler deploy --dry-run` shows the flag overriding the file's value, else a committed `wrangler.gate.jsonc`.
  - **R-E11 — the protected path list** is derived from the shell's router and printed by a script, with a cell that
    fails when the router gains a prefix the list does not name; `/api/health` is the one public API path.
  - **R-E12 — the screenshot generator** is a THIRD opt-in live spec, gated like the exit's, under the live tier's rules
    (`liveRequest`, no `toBeOK`, tracing off), writing all TEN of `worker/README.md` §1.4's pictures into
    `worker/docs/img/` — the seven the spec names and the three beside them, at the same cost — after a local dry run.
  - **R-E13 — the phones** are three Sonnet agents — the iPhone and the iPad in parallel, Android after — each with a
    brief carrying the device id, the bridge's tool names and coordinate rule, the checklist, the capture rules and its
    report path; nothing else runs on this Mac while the two iOS agents run.
  - **R-E14 — the sequence** is D's `close-sequence.sh` with `L=` set and no row added; the e2e inside it.
  - **R-E15 — Part 2 runs across TWO UTC days**: day 1 the deploys, the update reading, the live replay, the live tier,
    the screenshots and the phones; day 2 the rate-limit rule and the door window; the quota guard before each block.
  - **R-E16 — the merge and the tag** are Task 16.H, asked for at the wrap; if the owner defers them, Task 16 ends at
    the close and the handover, and the merge is a short follow-on session from the same recipe.

---

## Names shared across tasks

An implementer sees only its own task. **These names are the contract between tasks and are used verbatim.** A task
that finds the code forces a different shape STOPS and asks the ORCHESTRATOR, who amends this table with a `Ruling:`
line.

### What the tree holds at the base, consumed as it IS (read off the tree, never off this table)

| name | shape | tasks |
|---|---|---|
| `web/scripts/precache.mjs` and `web/src/precache.ts` (`buildPrecache`, `expectedType`, the ceilings), `worker/scripts/precache-list.mjs` (`readPrecacheList(text, "built" | "deployed")`, the ONE reader), `web/src/sw.ts` (`installOne`, `fromCache`, `fromRetained`), `web/src/swRoute.ts`, `web/src/swInstall.ts` (`checkHead`, `checkDigest`), `crates/yokohama-web/src/assets.rs` (`cache_control`), `worker/static/_headers` — dossier items 1–3 | as at the base | 1, 8 |
| `worker/scripts/{assemble-assets,deploy-guard,read-deploy}.mjs` and their `.test.mjs` cells — dossier items 4, 24 | as at the base | 1, 8 |
| `web/scripts/project-url.mjs` (`checkProjectUrl`, `PROJECT_URL_VAR`, `PROJECT_URL_MAX`), `web/vite.config.ts`'s `define`, `web/src/projectLink.ts` (`linkFor`, `projectLink`), `web/src/FrontDoor.tsx`'s `link` prop — dossier item 12 | as at the base | 3, 4a |
| `crates/yokohama-web/src/api.rs`'s `TableView`, `Health`, `ListQuery`, `GamesList`; `tables.rs`'s `Scope`, `list`, `delete`/`delete_locked`; `web/src/types.ts`'s `TableView`, `Health`; `web/src/wire/contract.ts`'s `TABLE` and `HEALTH` key sets — dossier items 17, 18, 20 | as at the base | 5, 6 |
| `worker/src/table/*.ts` (the expiry's mixin), `crates/yokohama-worker/src/lib.rs`'s `expire_inner`, `worker/src/config.ts`'s `Config` and its refusals — dossier items 20, 24 | as at the base | 6, 8 |
| `web/e2e/two_browsers.spec.ts:115–131, :200–219` (the request listeners), `web/e2e/harness.ts` (`capReached`, `noSidewaysScroll`), `web/e2e/live.ts` (`LIVE`, `LIVE_SPECS`, `EXIT`, `liveContextOptions`, `liveRequest` at :251), `web/playwright.config.ts` — dossier items 23, Part 2 | as at the base | 7, 8 |

### What cycle E produces (the contract between its own tasks)

| name | shape | task |
|---|---|---|
| the inventory entry `{ path: "/shell.bin", digest, type: "application/octet-stream" }` in place of `/index.html`; `dist/shell.bin` byte-equal to `dist/index.html`; `sw.ts`'s `shellResponse(body)` (or the name the implementer gives, ONE function) answering a navigation with `content-type: text/html; charset=utf-8`; `assets.rs::cache_control("/shell.bin") == Some("no-store")` and the same rule in `worker/static/_headers`; `read-deploy.mjs install` matching `Ready to play offline` (no period) and `cache.match("/shell.bin")`; the `assets` mode with no EDGE allowance | the update path | 1 |
| `web/src/wire/house_frame.json` (a constructed fixture in BOTH exact-set lists); `types.ts`'s `HouseFrame`; `contract.ts`'s `HOUSE_FRAME`; the lobby cells reading it; `log::HOUSE_SEED_UNREAD` (the sixth event) and the seed's `Err` arm logging it | D's leftovers | 2 |
| `web/scripts/build-values.mjs` (+ `.d.mts`) exporting `checkProjectUrl` (unchanged), `checkText(name, raw, max)`, `BUILD_VALUES` (the three names with their maxima); `vite.config.ts` defining `__YOKOHAMA_PROJECT_URL__`, `__YOKOHAMA_SITE_BLURB__`, `__YOKOHAMA_SITE_NOTICE__`; `web/src/buildValues.ts` exporting `projectLink()`, `siteBlurb(): string \| null`, `siteNotice(): string \| null`; `web/src/Footer.tsx` (`<footer className="site-notice">`, mounted in `main.tsx` after `<App/>`, drawn only when the notice is set, adding `has-notice` to `<html>` while drawn); the CSS variable `--notice-h`; the door's `<p className="blurb">` under the title; the lobby's `<a className="quiet-link">`; `deploy/Dockerfile`'s `ARG YOKOHAMA_SITE_BLURB` and `ARG YOKOHAMA_SITE_NOTICE` | the build values | 3 |
| `TurnMirror.events: u64`; `TableView.events: u64` and `TableView.saves: bool` (`table.rs`; `types.ts`; `contract.ts`'s `TABLE`); `tables::saves_on_abandon(doc, archive_on)`; the lobby's `paged` keyed by scope (`{ mine, open, playing }`), a "more" per scope; App's "opening…" screen while a table route is pending, the lobby never mounted on a cold `#/t/<id>` load | paging, "Abandon and save", the arrival | 5 |
| `ShellConfig.openTtlDays: number` (default 7) read from `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS`; the alarm ARMED at the shorter deadline from SQL and DECIDED at the alarm by the status `copyFacts` reads; the marker reason `waited` and its head; `moduleConfig`'s `table_ttl_days` and `open_table_ttl_days`; `Health.table_ttl_days: Option<u64>`, `Health.open_table_ttl_days: Option<u64>` (`api.rs`; `types.ts`; `contract.ts`'s `HEALTH`; `health.json` regenerated; `HOST_FIELDS` on both replays) | the waiting-table expiry | 6 |
| `web/e2e/harness.ts`'s `tally(page, label)` called from `two_browsers.spec.ts`'s listeners (`YOKOHAMA_WEB_TALLY=<path>`: one JSON line per request — `t`, `page`, `method`, `route` with ids replaced by `<id>`, `status`), and `web/scripts/tally-report.mjs` printing requests per move by route and the peak per 10-second window per route and on the oracle paths; the client's refetch policy as Task 7 leaves it, its target ratified | requests per move | 7 |
| `worker/wrangler.open.jsonc`; `worker/scripts/deploy-guard.mjs` taking `--config <file>` (its five `"wrangler.jsonc"` sites reading the CLI's choice); `npm run deploy:open`; `worker/scripts/access-paths.mjs` printing the seven destinations; its structural cell over `route.ts` and `admin.ts`; `web/e2e/readme_shots.spec.ts` under `YOKOHAMA_WEB_README_SHOTS=1` writing the ten pictures | the window's instruments | 8 |

---

## File structure

**Created:** `web/src/Footer.tsx`, `web/src/Footer.test.tsx`, `web/src/buildValues.ts`, `web/src/buildValues.test.ts`,
`web/scripts/build-values.mjs`, `web/scripts/build-values.d.mts`, `web/scripts/build-values.node.test.mjs` (3); `web/src/wire/house_frame.json` (2); `web/scripts/tally-report.mjs`, `web/scripts/tally-report.node.test.mjs` (7); `worker/wrangler.open.jsonc`,
`worker/scripts/access-paths.mjs`, `worker/scripts/access-paths.test.mjs`, `web/e2e/readme_shots.spec.ts` (8);
`docs/web-lane.md` (11); `guidingpath-archive-phase3.md`, `guidingpath-archive-web.md`,
`docs/plans/<the day>-guidingpath-consolidation.md` (13); `docs/plans/<the day>-web-frontend-m6.5c2e-close.md` (16); in
the ledger: the reports, the reviews, the pictures (`task-4a-look/`, `task-4b-look/`), `phones-iphone/`, `phones-ipad/`,
`phones-android/`, `close-sequence.sh`, `sequence.tsv`, the door window's transcript `door-window.md`.

**Modified** (principal files; each task's own list is the authority): `web/scripts/precache.mjs`, `web/src/sw.ts`,
`web/src/sw.node.test.ts`, `web/src/precache.node.test.ts`, `worker/scripts/{assemble-assets,read-deploy}.mjs` and their
cells (1); `crates/yokohama-web/src/{bots,lib}.rs`, `tests/wire_fixtures.rs`, `web/src/Lobby.test.tsx`, `worker/src/ws.test.ts`,
the comments the dossier's item 10 lists (2); `web/vite.config.ts`, `web/src/{FrontDoor,App,Lobby}.tsx`, `web/src/styles.css`,
`web/src/vite-env.d.ts`, `deploy/Dockerfile`, `deploy/README.md`, `worker/README.md`, `worker/RUNBOOK.md` §1 (3);
`web/src/styles.css` and the screens the dossier's item 15 names (4a, 4b); `crates/yokohama-web/src/{api,tables}.rs`,
`web/src/{types,api}.ts`, `web/src/wire/contract.ts`, `web/src/{Lobby,Abandon,App}.tsx` and their tests, the corpus (5);
`worker/src/config.ts`, `worker/src/env.ts`, `worker/src/marker.ts`, `worker/src/table.ts`, `worker/src/table/lifecycle.ts`, `crates/yokohama-worker/src/lib.rs`,
`crates/yokohama-web/src/api.rs`, `web/src/types.ts`, `contract.ts`, `worker/README.md`, `deploy/README.md`, the corpus (6);
`web/src/{App,TableSections,Chat,Lobby}.tsx`, `web/src/{usePolled,signal}.ts`, `web/e2e/harness.ts` (7);
`worker/scripts/deploy-guard.mjs`, `worker/package.json`, `web/e2e/live.ts`, `web/playwright.config.ts` (8); `CLAUDE.md`
(12); `guidingpath.md` (13); `docs/handover.md` (14); `README.md`, `deploy/RUNBOOK.md` §3.6, `worker/README.md` §1.4 and
`worker/docs/img/` (15); the RUNBOOKs' records, the spec's §10.12, the triage page (16).

---

## Task order and why

```
1 ─► 2 ─► 3 ─► 4a ─► 4b ─► 5 ─► 6 ─► 7 ─► 8 ─► 9 (review, wave, battery, sequence) ─► 10 (the live readings, two days)
                                                                                         └─► 11 ─► 12 ─► 13 ─► 14 ─► 15 ─► 16
```

1. **1 first:** the release must install for a returning browser before any browser reading of this cycle means
   anything; it touches the precache, the worker and the Worker's scripts, and nothing else touches them.
2. **2 next:** no behaviour but a log line; the fixture and the comments land before the files they cite move.
3. **3 before 4a/4b:** the polish tasks photograph the door and every screen with the blurb and the footer on.
4. **4a before 4b:** both edit `styles.css`; 4a the door, the lobby and the device card, 4b the game and the table; 4b
   finds by content.
5. **5 before 6:** both move the corpus; 5's `TableView` change reshapes every table payload, 6 adds two health fields;
   two regenerations, each diff read once.
6. **6 before 7:** 7 counts the fetches the lobby and the game screen make as 5 and 6 leave them.
7. **7 before 8:** 8's screenshot spec and tally reading ride the harness 7 extended.
8. **9, then 10:** the review before the deploys, so a fix wave never forces a second live sequence.
9. **11 → 15 after 10:** every document records Part 2's readings; 11 (the lane document) before 12 (`CLAUDE.md`'s
   pointers into it) before 13 (guidingpath cites both) before 14 (the board points at all three) before 15 (the README
   links to every one).
10. **16 last.**

---

## Couplings between tasks (binding; the later task's implementer does what its item says)

- **Task 1 → Task 8:** `read-deploy.mjs install`'s door check is Task 1's; Task 8 changes no line of that script.
- **Task 1 → Task 10:** the update reading (10.B) uses a browser controlled by D's release with `cf_clearance` cleared,
  then E's `sw.js`; Task 1's report names the exact door sentence the reading asserts.
- **Task 2 → Task 5:** `house_frame.json` is a constructed fixture; Task 5's regeneration leaves it untouched (it carries
  no table) and its report says so.
- **Task 3 → Tasks 4a, 4b:** the pictures are taken with `YOKOHAMA_SITE_BLURB` and `YOKOHAMA_SITE_NOTICE` SET (the
  orchestrator's values in the brief) and once with both unset; 4a keeps the footer to one line at 390 px.
- **Task 3 → Task 15:** the README's variables table names the three build values with Task 3's maxima.
- **Task 5 → Task 6:** the corpus is regenerated twice; 6 finds `health.json` and `contract.ts`'s `HEALTH` as 5 leaves
  them and touches no `TABLE` key.
- **Task 5 → Task 7:** 7's tally is read over the lobby as 5 leaves it (three pagers); 7 changes no paging code.
- **Task 6 → Task 8:** `wrangler.jsonc` and both pool files carry `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS: "7"` after 6; 8's
  `wrangler.open.jsonc` carries it too, and the guard's drift checks compare the files as 6 leaves them.
- **Task 6 → Task 10:** both replays' `HOST_FIELDS` exempt the two TTL fields, so 10.F's live replay compares them as
  host fields and HOLDS.
- **Task 7 → Task 10:** 10.F widens the rate-limit rule at the value 7's report gives (the peak per 10 seconds on the
  oracle paths, with the replay's own row rate added as 10.D reads it).
- **Task 8 → Task 10:** 10.G uses `npm run deploy:open`, the `--var` or `wrangler.gate.jsonc` shape 8 measured, and
  `access-paths.mjs`'s printed list VERBATIM for state 2's destinations.
- **Task 8 → Task 15:** the README's API map is checked against the same route facts `access-paths.mjs` reads.
- **Task 10 → Tasks 11–15:** every figure in the documents is Part 2's reading, named with its date; no document is
  written from this plan's expectations.
- **Task 11 → Task 12:** `CLAUDE.md`'s five pointers name `docs/web-lane.md`'s anchors as Task 11 wrote them.
- **Task 11 → Task 13:** the guidingpath row cites the lane document's records table for every cycle's close.
- **Every task → Task 9:** each report names its catchers by cell name; the battery rows cite them.

---

### Task 1: The shell precached from a static asset, its cache header on both hosts, and the install reader — Opus (R-6.5c2.41; R-E3; spec §10.11's defect; dossier items 1–6; Review Focus 1)

**Files:**
- Modify: `web/src/precache.ts:296–372` (`buildPrecache`: the `need("index.html", …)` at :326 stays — the file is the
  SOURCE — and the entry it lists becomes `/shell.bin` on the default `application/octet-stream` arm of `expectedType`
  :199–235), `web/scripts/precache.mjs:91` (the walk of `dist` minus `sw.js` also skips `shell.bin`, since
  `offline.spec.ts:820` and :1441–1447 re-run the generator over its own output) and the write of `dist/shell.bin` as
  a byte copy of `dist/index.html` before the digest; `web/src/sw.ts:364` (the navigation answer), :375–383
  (`fromCache`); `crates/yokohama-web/src/assets.rs:104–109` (`cache_control`: `/shell.bin` → `no-store`, as `/sw.js`
  is) and its cell :246–280 (`hashed_assets_are_immutable_the_worker_is_no_store_and_nothing_else_carries_a_header`,
  which asserts no other path carries a header — it gains `shell.bin` and keeps its name or takes a truer one);
  `worker/static/_headers:1–7` (the same rule restated for Workers Assets; guard check 9 compares the assembled copy);
  `worker/scripts/read-deploy.mjs:284`, :302 (`cache.match("/index.html")` → `/shell.bin`), :296, :309 (the period),
  :118–124 and :182–196 (the `assets` mode's EDGE allowance for a `text/html` entry: REMOVED — no HTML entry remains,
  every entry is byte-exact); `worker/scripts/read-deploy.test.mjs:81–128` (fixtures naming `/index.html`);
  `web/e2e/offline.spec.ts:172` (`cacheHas(…, "/index.html")` → `/shell.bin`) and the comment at :611.
- Test: `web/src/precache.test.ts:125–128` (asserts `/index.html` → `text/html`; becomes `/shell.bin` →
  `application/octet-stream`), :163 (the refusal's sentence, if it names the entry); `web/src/precache.node.test.ts:108`,
  :143–144; `web/src/sw.node.test.ts:674–681` (a navigation answered with the shell — `.text()` only today; it gains
  the content-type assertion), :872–880, :887; `assets.rs`'s cell; a structural pin in `read-deploy.test.mjs`.
- Do NOT touch: `web/src/OnThisDevice.tsx:257` (the door's "Ready to play offline", no period — the reader moves to
  it); `deploy/Dockerfile:222–224` and `deploy/Dockerfile.runtime:40–42` (they copy the whole `dist`; `shell.bin`
  travels — dossier item 6); `worker/scripts/assemble-assets.mjs` (it verifies every listed entry and names no
  `/index.html` — dossier item 4; check that `shell.bin` is verified by running it, and say so).

**Interfaces:**
- Consumes: `readPrecacheList(swText, "built" | "deployed")` (`worker/scripts/precache-list.mjs:39–46`, the ONE reader,
  whose `ENTRY` regex requires `path`, a 64-hex `digest` and `type`); `installOne`, `checkHead`, `checkDigest` as they
  are; `swRoute.ts:91`'s navigation classification as it is.
- Produces: the inventory entry `{ path: "/shell.bin", digest: <sha256 of dist/index.html>, type: "application/octet-stream" }`
  in place of `/index.html`; `dist/shell.bin` byte-equal to `dist/index.html`; `sw.ts`'s navigation answer built
  from the `/shell.bin` cache entry with `content-type: text/html; charset=utf-8` (ONE function, `shellResponse(body)`
  or the name the implementer gives, used by the navigation arm alone); `assets.rs::cache_control("/shell.bin") ==
  Some("no-store")`; `_headers` with `/shell.bin` `Cache-Control: no-store`; `read-deploy.mjs install` expecting
  `Ready to play offline` and `cache.match("/shell.bin")`.

**The mechanism, so the implementer builds the right thing.** The zone's JavaScript Detections injects a per-request
`<script>` into every HTML response at the edge, on both hosts (spec §10.11: 3,325 B against the release's 2,387; a
different digest each time; the origin untouched). The worker's install digests `/index.html` against the inventory
and refuses. A fresh browser passes because its page view issued a `cf_clearance` cookie the worker's fetch carries; a
browser CONTROLLED by a release never reaches the edge as HTML again, its clearance goes stale, and every later
release is refused. The fix: the worker never fetches HTML. The shell's bytes are precached from `/shell.bin`, a
non-HTML asset the edge does not inject into, and the worker answers a navigation from those bytes with the HTML
content type it sets itself. **And the edge's CACHE is the second half (dossier item 3):** `.bin` is on Cloudflare's
default cached-extensions list, and a 200 with no `cache-control` stays at the edge for 120 minutes, so a
`/shell.bin` served bare would be the previous release's for up to two hours after a NAS deploy, and every install in
that window would fail the digest check — today's defect in a new form. `cache: "reload"` bypasses the browser's
cache, not the edge's. So `/shell.bin` is served `no-store` on both hosts, exactly as `/sw.js` is, and Task 10.C and
10.E read its `cf-cache-status` after each deploy (`DYNAMIC` or `BYPASS`, never `HIT`). The first navigation still gets
and runs the snippet; the digest check is unchanged; `/index.html` is served exactly as today.

- [ ] **Step 1: The inventory cells, red.** `web/src/precache.test.ts` (the pure half, corpus-free): in the cell at
  :125–128, the shell entry is `/shell.bin` typed `application/octet-stream`, `/index.html` is listed by no entry, and
  no entry's type starts `text/html`; a `dist` without `index.html` is still refused by name (:163). In
  `web/src/precache.node.test.ts` (a real `dist`; `npm run build:client` first if stale), at :108: the same three facts
  over the built `sw.js` through `readPrecacheList`, plus `readFileSync("dist/shell.bin").equals(readFileSync("dist/index.html"))`
  and the entry's digest equal to `createHash("sha256").update(index).digest("hex")`. Run: `cd web && npx vitest run
  src/precache.test.ts` and `npx vitest run -c vitest.node.config.ts src/precache.node.test.ts` → FAIL.
- [ ] **Step 2: The generator.** `precache.ts`'s `buildPrecache` lists the shell as `/shell.bin` (its bytes are
  `index.html`'s; the default type arm); `precache.mjs` writes `dist/shell.bin` from `dist/index.html` before the walk
  and skips it in the walk as it skips `sw.js`; the printed "shell" figure counts it. The ceilings (:126, :135, :152,
  :160–178) are unchanged; the release grows by one shell's bytes. Step 1 → PASS; the four `precache:` lines quoted
  in the report.
- [ ] **Step 3: The navigation cell, red.** `web/src/sw.node.test.ts:674–681` gains: the answer's
  `headers.get("content-type")` is `text/html; charset=utf-8`, its body the shell's text, and the network was not
  asked (the file's own fake-fetch counter); the inventory the cell boots carries `/shell.bin` typed
  `application/octet-stream` and no `/index.html`. Run → FAIL.
- [ ] **Step 4: The worker.** `sw.ts:364`: the navigation arm answers from `/shell.bin` through `shellResponse`, which
  builds `new Response(body, { status: 200, headers: { "content-type": "text/html; charset=utf-8" } })` over the cached
  entry's body — the stored entry keeps the server's headers (`installOne` :202–205 unchanged), and the navigation's
  response carries the type the worker sets. `installOne`, `checkHead`, `checkDigest`, the `/__release__` stamp, the
  update offer and `fromRetained` are untouched; a non-navigation GET of `/index.html` keeps routing to `fromRetained`
  (dossier item 2). The header comment gains one paragraph on the mechanism (the date, R-6.5c2.41, the edge's cache).
  Step 3 → PASS; the file's other cells (:286–404, :872–880, :887) green.
- [ ] **Step 5: The cache header, red first.** `assets.rs`'s cell at :246–280: `cache_control("/shell.bin")` reads
  `Some("no-store")` and every other unhashed `dist` path still reads `None` → FAIL; then the arm beside `/sw.js`'s →
  PASS. `worker/static/_headers` gains the same rule (the file's shape as it is); `cd worker && npm run build:assets`
  (or the assembler's cell) shows the copy verified.
- [ ] **Step 6: The reader's cells, red.** `worker/scripts/read-deploy.test.mjs`: (a) the fixtures at :81–128 name
  `/shell.bin` typed `application/octet-stream` and no `/index.html`, and the `assets` mode's pure half reads a byte-exact
  list with no EDGE line; (b) a STRUCTURAL pin: read `web/src/OnThisDevice.tsx`, take the text of the paragraph
  `className="muted ready"` (:257), read `read-deploy.mjs`, take the string its `install` mode passes to
  `innerText.includes(`, assert equality. Run: `cd worker && npm run test:scripts -- read-deploy` → FAIL.
- [ ] **Step 7: The reader.** `read-deploy.mjs`: :284 and :302 match `/shell.bin`; :296 and :309 lose the period; the
  EDGE allowance at :118–124 and the lines :182–196 go, with the header comment rewritten (the inventory holds no HTML;
  `/index.html` is still checked to answer 200 and never a redirect, as the navigation's page). Step 6 → PASS.
- [ ] **Step 8: The e2e** (owed: what a browser does changed): `offline.spec.ts:172` reads `/shell.bin`; the comment at
  :611 corrected; `cd web && npm run e2e` → 72 in 13 (or the count Task 1 leaves); the cells at :300 and :354 are the
  ones that ride the new path offline; a red cell re-runs ALONE.
- [ ] **Step 9: The per-edit gates** (`npm run test:wasm` carries both node files; `cargo test -p yokohama-web`
  carries `assets.rs`'s cell; `cd worker && npm run test:scripts`).
- [ ] **Step 10: Commit.**

```bash
git add web/src/precache.ts web/scripts/precache.mjs web/src/sw.ts web/src/precache.test.ts web/src/precache.node.test.ts web/src/sw.node.test.ts crates/yokohama-web/src/assets.rs worker/static/_headers worker/scripts/read-deploy.mjs worker/scripts/read-deploy.test.mjs web/e2e/offline.spec.ts
git commit -m "fix(sw): the shell is precached from /shell.bin, served no-store on both hosts, so a returning browser installs the new release past the edge's snippet and its cache (R-6.5c2.41)"
```

  The body quotes Steps 1, 3, 5 and 6's red runs and the mechanism in four lines, the edge's cache among them.

---

### Task 2: D's leftovers — the `bots.rs` clause, the house seed's log line, the house-frame fixture, the review citations, the hostname cell — Sonnet 5.5 (D's rulings 22, 36, 48, 49; the E handover §3; dossier items 7–11)

**Files:**
- Modify: `crates/yokohama-web/src/bots.rs:414–420` (one comment line — dossier item 7); `crates/yokohama-web/src/lib.rs:491–493`
  (the seed's `if let Ok(counts)` — dossier item 8) and `crates/yokohama-web/src/log.rs:64–77` (a SIXTH event constant)
  with its cell at :116–137 (`the_events_are_five_names_and_none_is_the_shells`, renamed to six) and :144–176
  (`the_layer_prints_only_through_its_log`, unchanged: no `eprintln!`); `crates/yokohama-web/tests/wire_fixtures.rs:2150–2325`
  (`constructed()`) and :2361–2380 (`corpus()`); `web/src/local/corpus.node.test.ts:149` (`NOT_SERVED`, 29 → 30) and
  `worker/test/corpus.test.ts:96` (`THIRD_COLUMN`, 17 → 18) — BOTH exact-set lists (D's ruling 31); `web/src/types.ts`
  (a `HouseFrame` type: `{ seq: number; chat: number; rev: number }`) and `web/src/wire/contract.ts` (imports :112–125,
  `KEYS` :788+: `HOUSE_FRAME`); `web/src/Lobby.test.tsx:586` (`{ seq: 1, chat: 0 }`, no `rev`), :794, :798, :845, :849,
  :920, :955 (the hand-built frames; the reason at :769–775); `web/src/ws.test.ts:44–51` (the hostname at :49–50);
  the ~thirty D-written label citations the dossier's item 10 lists, each re-pointed to the close.
- Create: `web/src/wire/house_frame.json` (a NAMED corpus move, R-E2).
- Do NOT touch: `crates/yokohama-worker/src/picker.rs:368`, :377, `receipt.rs:396` (production) and `receipt.rs:1173`
  (a test) — the `expect("a game")` sites are by design on a single-threaded instance; Task 11 states it.

**Interfaces:**
- Consumes: `yokohama_web::signal::house_frame(seq, chat, rev)` (`signal.rs:81–91`, pinned at :372–378 and in the
  Worker at `house.rs:614–618`); the log seam `trait Log { fn line(&self, event: &'static str, fields: Value) }`
  (`log.rs:33–37`), called as `games.rs:1037` does: `self.log.line(crate::log::OVER_TABLE_GONE, serde_json::json!({ "id": id }))`,
  field values strings.
- Produces: `house_frame.json` — `signal::house_frame` for the house as it stands after the walk's last house post;
  `types.ts`'s `HouseFrame`; `contract.ts`'s `HOUSE_FRAME`; the lobby cells reading the fixture; `log::HOUSE_SEED_UNREAD`
  (the sixth constant) and the line `games.log().line(HOUSE_SEED_UNREAD, json!({ "error": e.to_string() }))` in the
  seed's `Err` arm — through whatever accessor `Games` exposes for its log (the implementer reads `games.rs` for the
  field's visibility and names the accessor it used).

- [ ] **Step 1: The fixture, red.** In `wire_fixtures.rs`'s `constructed()` add `house_frame.json` built from
  `signal::house_frame(seq, chat, rev)` with the house counters the walk leaves (the implementer names the row after
  which they are read); register its contract in `corpus()` beside the other constructed files. Run `cargo test -p
  yokohama-web --test wire_fixtures` → FAIL (the committed corpus lacks the file). Then `UPDATE_WIRE_FIXTURES=1 cargo
  test -p yokohama-web --test wire_fixtures`, READ the diff — ONE new file, no other change — and run again → PASS.
  Add the file to `NOT_SERVED` and `THIRD_COLUMN` (their own cells refuse a wrong entry; run `npm run test:wasm`'s
  corpus replay and the pool's `corpus.test.ts` to see the two counts move by one). `types.ts` gains `HouseFrame`;
  `contract.ts` gains `HOUSE_FRAME`; `npm run typecheck` green.
- [ ] **Step 2: The lobby cells read the fixture.** Each of the seven hand-built frames in `Lobby.test.tsx` loads
  `house_frame.json` through the file's own corpus loader and derives its moved frame from it
  (`{ ...frame, chat: frame.chat + 1 }`, `{ ...frame, rev: frame.rev + 1 }`); the reason comment at :769–775 is
  rewritten to say the frame comes from the corpus. A mutant — the fixture's key `chat` renamed — must fail these cells
  AND `contract.ts`'s check; the report records the run. `npm test` green.
- [ ] **Step 3: The seed's log line, red first.** A lib cell in `lib.rs` beside :7403 (the `Ok` arm's cell): a store
  double whose house read FAILS (the file's own failing-store double, or `FailingWrites`' read arm) makes
  `AppState::with_store_chat_and_archive` emit ONE `HOUSE_SEED_UNREAD` line through a collecting `Log` double, and the
  frame's counters start at 0, 0. Run → FAIL (no constant). Then the sixth constant in `log.rs` (its doc: a transient
  boot-time read failure of the house document leaves that process counting from 0 until the next post — D's ruling
  49), the six-names cell renamed and extended, the arm `Err(e) => games.log().line(HOUSE_SEED_UNREAD, json!({ "error": e.to_string() }))`.
  → PASS; `the_layer_prints_only_through_its_log` still green.
- [ ] **Step 4: The `bots.rs` clause** at :414–420: "only when one controller is unparseable" is false — the rung aliases
  `retreat_averse` and `family_weighted` are PARSEABLE (`crates/yokohama-core/src/game/policy.rs:143–144`, read, never
  edited) and reach Row 4 too; reword the clause to what the arm does. One line, no code.
- [ ] **Step 5: The citations** — every site the dossier's item 10 lists, re-pointed to the tracked record it names
  (`docs/plans/2026-09-29-web-frontend-m6.5c2d-close.md` §4, §5's ruling number, or §1's task row): `lib.rs:483`,
  :7388, :4119; `signal.rs:38`, :125, :166; `tables.rs:1155`, :1186; `games.rs:2514`, :2517; `web/src/OnThisDevice.tsx:138`;
  `OnThisDevice.test.tsx:207`, :298; `web/src/local/db.ts:413`; `web/src/tables.ts:195`; `Lobby.test.tsx:477`, :859;
  `OpenTable.test.tsx:535`; `App.tsx:753`; `Dealing.test.tsx:568`; `Chat.tsx:391`; `Chat.test.tsx:387`, :395, :434;
  `Lobby.tsx:245`; `worker/src/table/delivery.ts:88`; `worker/src/table/sockets.ts:38`; `worker/scripts/replay/judge.mjs:111`;
  `crates/yokohama-web/tests/browser_played_pin.rs:269` (find each by content). No sentence changes its claim. The
  ~200 older label citations of earlier cycles are OUT of scope and the report says so.
- [ ] **Step 6: The hostname.** `web/src/ws.test.ts:49–50` uses `https://example.test` in place of the site's domain;
  the assertion unchanged.
- [ ] **Step 7: The per-edit gates** (the corpus pins green after the regeneration; `npm test`; `cd worker && npm run
  build:module:debug && npm test` — `THIRD_COLUMN` moved), then commit:

```bash
git add crates/yokohama-web/src/bots.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/log.rs crates/yokohama-web/tests/wire_fixtures.rs web/src/wire/house_frame.json web/src/wire/contract.ts web/src/types.ts web/src/Lobby.test.tsx web/src/local/corpus.node.test.ts worker/test/corpus.test.ts web/src/ws.test.ts <the citation files>
git commit -m "chore(web): cycle D's leftovers - the house frame as a corpus fixture, a log line on the house seed's read failure, the bots.rs clause, the review citations re-pointed at the close, the ws.test.ts hostname"
```

---

### Task 3: The door's three build values — the blurb, the footer, the lobby's link — Sonnet 5.5 (R-6.5c2.40; R-E4; dossier items 12–14; Review Focus 5)

**Files:**
- Create: `web/scripts/build-values.mjs` and `web/scripts/build-values.d.mts` (the checker; `project-url.mjs:15–39` and
  `project-url.d.mts` are DELETED once `vite.config.ts` and the node cell import the new module),
  `web/scripts/build-values.node.test.mjs` (the cells of `project-url.node.test.ts:9+` moved and extended),
  `web/src/buildValues.ts` (`web/src/projectLink.ts:9–21` DELETED once `App.tsx:1518–1525` and `FrontDoor.tsx` import
  from it; `projectLink.test.ts:12–25` moves to `buildValues.test.ts`), `web/src/Footer.tsx`, `web/src/Footer.test.tsx`.
- Modify: `web/vite.config.ts:24–36`, `web/src/vite-env.d.ts:37` (three globals), `web/src/main.tsx:16–18` (the footer
  mounts HERE, after `<App/>`: `App` has about fifteen `return` sites and no wrapper — dossier item 12),
  `web/src/FrontDoor.tsx:89–111` (the blurb between `<h1>Yokohama</h1>` :92 and the link :93–107), `web/src/Lobby.tsx:517–528`
  (`.lobby-actions`: the quiet link), `web/src/styles.css` (`.blurb` near the hero :1946–1966; `.site-notice`; `.quiet-link`;
  the ≥1201 px `.app` grid :228–245), `web/src/FrontDoor.test.tsx:115–128`, `web/package.json:11–12` (the `e2e` script's
  defaults), `deploy/Dockerfile:194–200` (two more `ARG`s), `.github/workflows/docker.yml:99` (passes the two beside the
  first), `deploy/README.md:119–135`, `worker/README.md` (names none today: the three named in "The two doors" or its
  build paragraph), `worker/RUNBOOK.md:90–121` (§1's build command names the three).

**Interfaces:**
- Consumes: `checkProjectUrl` (moved verbatim); `FrontDoor`'s `link` prop (:68–70).
- Produces: `checkText(name, raw, max): string | null` (unset or empty → `null`; whitespace-only → throw
  `<name> holds no words`; a control character → throw `<name> holds a control character`; over `max` → throw
  `<name> is longer than <max> characters`); `BUILD_VALUES = { YOKOHAMA_PROJECT_URL: 200, YOKOHAMA_SITE_BLURB: 400,
  YOKOHAMA_SITE_NOTICE: 200 }`; the defines `__YOKOHAMA_PROJECT_URL__`, `__YOKOHAMA_SITE_BLURB__`, `__YOKOHAMA_SITE_NOTICE__`
  (each `string | null`, `vitest.config.ts` merging them as it merges the first); `projectLink()`, `siteBlurb()`,
  `siteNotice()`; `<Footer/>` rendering `<footer className="site-notice">{notice}</footer>` and, when drawn, adding
  the class `has-notice` to `document.documentElement` (removed when not drawn); the CSS variable `--notice-h: 1.75rem`.

**The layout rule (dossier item 12).** At ≥1201 px the game screen's `.app` is a grid of `height: 100vh` with rows
`auto auto auto minmax(0, 1fr)`, and the fit rule reads `.table`'s own height (`100cqh`). A footer mounted outside
`.app` would sit below the viewport; inside it there is no wrapper to mount in. So the footer mounts in `main.tsx`
after `<App/>`, and `.has-notice .app` at ≥1201 px reads `height: calc(100vh - var(--notice-h))`, which the fit rule
follows — the table shrinks by one text line, never covered. Below 1201 px the page flows and the footer ends it.

- [ ] **Step 1: The checker's cells, red.** `build-values.node.test.mjs`: the six `checkText` cases of the Interfaces,
  and `checkProjectUrl`'s existing cells moved. Run `cd web && npx vitest run -c vitest.node.config.ts
  scripts/build-values.node.test.mjs` → FAIL (no module).
- [ ] **Step 2: The checker.** `build-values.mjs` and its `.d.mts`; the header paragraph of `project-url.mjs` moved and
  widened to three values. Step 1 → PASS.
- [ ] **Step 3: The defines and the readers.** `vite.config.ts`'s `define` carries the three (the comment: THREE build
  values, one checker); `vite-env.d.ts` declares the two new globals beside :37; `buildValues.ts` exports
  `linkFor`/`projectLink` (moved) and the two readers; `projectLink.test.ts`'s cells move to `buildValues.test.ts` with
  two more (`siteBlurb`/`siteNotice` read the globals; `""` never reaches them since the checker made it `null`).
  Delete the three old files; `npm run typecheck` green.
- [ ] **Step 4: The footer's cells, red.** `Footer.test.tsx`: with `siteNotice` mocked to `"Not affiliated with
  Asmodee"`, `render(<Footer/>)` shows `footer.site-notice` with that text and `document.documentElement` carries
  `has-notice`; with `null`, no `footer` and no class; unmounting removes the class. Run → FAIL.
- [ ] **Step 5: The footer.** `Footer.tsx` as the Interfaces say; `main.tsx` renders `<Footer/>` after `<App/>`;
  `styles.css`: `.site-notice { padding: .4rem 16px; font-size: .78rem; line-height: 1.3; color: var(--fg-dim);
  text-align: center }`, `:root { --notice-h: 1.75rem }`, and in the ≥1201 px block `.has-notice .app { height:
  calc(100vh - var(--notice-h)) }`. Step 4 → PASS.
- [ ] **Step 6: The blurb and the lobby's link, cells red.** `FrontDoor.test.tsx:115–128` gains: with `siteBlurb`
  mocked to two sentences, `p.blurb` sits between the `h1` and the link (`h1 + .blurb` matches; the link follows);
  with `null`, no `.blurb`. `Lobby.test.tsx`: with `projectLink` mocked, `a.quiet-link` inside `.lobby-actions` with
  the link's `href` and `text`, `rel="noopener"`; with `null`, none. Run → FAIL.
- [ ] **Step 7: The two renders.** `FrontDoor.tsx`: `{blurb && <p className="blurb">{blurb}</p>}` after the `h1`;
  `.blurb { max-width: 60ch; color: var(--fg-dim); margin: .25rem 0 .5rem; font-size: .95rem }` — **and the
  title-never-meets-the-board cell at `setup_screens.spec.ts:100–137` (861 and 700 px, text measured by `Range`)
  must stay green with the blurb in the box** (dossier item 14), so the blurb wraps under the title's column, never
  beside the board. `Lobby.tsx`: the quiet link in `.lobby-actions` (`font-size: .8rem; color: var(--fg-dim);
  text-decoration: underline dotted`). Step 6 → PASS.
- [ ] **Step 8: The e2e sees them.** `web/package.json`'s `e2e` script runs `build:client` with
  `YOKOHAMA_SITE_NOTICE="${YOKOHAMA_SITE_NOTICE:-Not affiliated with Asmodee}"` and
  `YOKOHAMA_SITE_BLURB="${YOKOHAMA_SITE_BLURB:-Yokohama, the board game, in your browser. Play the bot here, or play with friends.}"`
  (sh's `${VAR:-default}`), so every e2e build carries both. Then `npm run e2e`: `noSidewaysScroll` (`harness.ts:876`)
  and the 44 px reads (`setup_screens.spec.ts:370–374`) hold on every screen with the footer present **(Review Focus
  5)**; the 861/700 px title cell green; the game screen's `.table` height at 1440×900 read before and after into
  the report (the footer's cost).
- [ ] **Step 9: The build arguments and the documents.** `deploy/Dockerfile:199`'s neighbours: `ARG YOKOHAMA_SITE_BLURB`,
  `ARG YOKOHAMA_SITE_NOTICE`, the comment widened (an ARG with no `--build-arg` is unset; `--build-arg X=` is the empty
  string, which the checker reads as none); `docker.yml:99` passes both beside the first; `deploy/README.md:119–135`
  becomes "The three build values" with each name, maximum and meaning; `worker/README.md` names them where it
  explains the door; `worker/RUNBOOK.md` §1's `cd web && npm run build` line shows the three set.
- [ ] **Step 10: The per-edit gates**, then commit:

```bash
git add web/scripts/build-values.mjs web/scripts/build-values.d.mts web/scripts/build-values.node.test.mjs web/src/buildValues.ts web/src/buildValues.test.ts web/src/Footer.tsx web/src/Footer.test.tsx web/src/main.tsx web/vite.config.ts web/src/vite-env.d.ts web/src/App.tsx web/src/FrontDoor.tsx web/src/FrontDoor.test.tsx web/src/Lobby.tsx web/src/Lobby.test.tsx web/src/styles.css web/package.json deploy/Dockerfile .github/workflows/docker.yml deploy/README.md worker/README.md worker/RUNBOOK.md
git rm web/scripts/project-url.mjs web/scripts/project-url.d.mts web/scripts/project-url.node.test.ts web/src/projectLink.ts web/src/projectLink.test.ts
git commit -m "feat(door): three build values - the blurb under the title, the notice as a one-line footer on every screen, the repository link on the lobby (R-6.5c2.40)"
```

---

### Task 4a: The owner's-eye polish — the door, the lobby and the device card — Opus (R-6.5c2.38; straw poll rows 58, 59, 62, 63, 64, 68, 21's create-form and open-table strips, the device card's row; dossier items 15–16)

**Files:**
- Modify: `web/src/styles.css:922–926` and :1810–1814 (row 58; the ≤600 px widenings :2083–2085, :2158–2160), :1973–1974
  and :2059–2061 (row 59: `.two-up`, `max-width: 61rem` = 976 px), :955–965 and :1998 (row 62), :1820–1828 (the marks'
  hues), :2344–2348 and :2548–2587 (row 63), :2181–2218 and :2226–2233 (row 64: `.row-abandon button.ghost`
  `min-height: 24px`), :1985–1997 (the device row); `web/src/FrontDoor.tsx:120` (row 59 — the `two-up` state is the door
  with `VITE_MULTIPLAYER=false`, state E); `web/src/OnThisDevice.tsx:300–343` (`StorageLine`), :178–191 (the foot),
  :386–388 (the iOS note), :201–214 (the device row); `web/src/BoardMark.tsx:36–47` (the three mark tones: customs
  `imports`, laboratory `faint`, employment agency `neutral`); `web/src/Lobby.tsx:612–624` (`<details className="fold">`),
  :406–466 (`row()`; `.row-abandon` :460–462), :438–448 (the copy button with `aria-live="polite"` AND a changing
  `aria-label`); `web/src/OpenTable.tsx:457–460` (the other copy button); `web/e2e/setup_screens.spec.ts:505–516` (the
  device row's exemption); `web/src/NewGame.tsx:441`, :555 and `OpenTable.tsx:235`, :362, :402 (the two strips) with
  their cells `NewGame.test.tsx:449`, `OpenTable.test.tsx:248`.
- The instrument: pictures BEFORE and AFTER, at 390, 768, 1024 and 1280 (and 900 for row 59), into
  `$L/task-4a-look/before/` and `/after/`: the setup screens through `setup_screens.spec.ts`'s own `look(page, …)`
  captures (:98, :135, :377, :396 — dossier item 16; `gates.spec.ts` shoots the game screens only) and a short
  Playwright script for the rest, with the blurb and the notice SET (Task 3's e2e defaults) and, for the door, once
  unset; row 59's state needs a client built with `VITE_MULTIPLAYER=false` (one extra `build:client` into a scratch
  directory). The review reads the pictures beside the diff.

**Interfaces:**
- Consumes: Task 3's `.blurb`, `.site-notice`, `.quiet-link` (kept as they are).
- Produces: the rules the items name; Task 4b finds `styles.css` by content.

**The items, in plain words, each with its own cell or picture** (the orchestrator's calls, R-6.5c2.38; a row whose
picture shows nothing to fix is STRUCK in the report with its picture, never silently):

1. **Row 58** — the door's link styled as a button (`a.primary-link`, padding `0.5rem 1.05rem`) is not the box of the
   real `button.primary` (`0.45rem 1.1rem`): about 1.6 px taller and narrower. ONE shared rule for both; a structural
   CSS cell (`styles.css` read as text: the two selectors share one declaration block); the 1280 picture.
2. **Row 59** — the multiplayer-off door's `.two-up` goes two-column at 976 px (`max-width: 61rem`) where mock 06
   (`docs/plans/2026-09-26-web-frontend-m6.5c2-mocks/06-door-multiplayer-off.webp`) meant about 868. `max-width:
   54.25rem`; picture at 900 in the `VITE_MULTIPLAYER=false` build.
3. **Row 62** — `StorageLine`'s sentence (ending " Back them up to hold a copy you can restore.") and the foot's
   backup note: one sentence each in the door's voice, no sentence twice on the card; the cell that renders the
   card asserts the new text. The three board-mark tones (`BoardMark.tsx:36–47`): LOOK at the 1280 picture of a set
   table; if `imports`, `faint` and `neutral` read as one hue, separate them in `styles.css:1820–1828` by lightness
   steps a reader can tell apart (≥ 12% apart); else strike with the picture.
4. **Row 63** — the closed Finished fold (`<details className="fold">`) shows no cue at 1×. The `<summary>` gains a
   `::before` chevron (`content: "▸"`, rotated 90° when `[open]`) and its text names the count ("Finished (3)");
   a jsdom cell over the corpus's lobby that the summary's text carries the count.
5. **Row 64 and the device row** — at 390 px a lobby row you created stands about 146 px tall because each control
   pads to 44 px, and the device card's game row is under 44 px. The row itself becomes the hit box (the whole row one
   link at ≥ 44 px, the Abandon a 44 px square at its end), and the device row reaches 44 px; the sweep's exemption at
   `setup_screens.spec.ts:505–516` is REMOVED first (red), then the CSS (green).
6. **Row 68** — "Copied" is announced twice because the copy BUTTON is the live region and both its text and its
   `aria-label` change (`Lobby.tsx:438–448`, `OpenTable.tsx:457–460`). The button's label stays constant and ONE
   `role="status"` element beside it carries "Copied"; a jsdom cell that exactly one status element in the row
   carries the copy state and the button's `aria-label` does not change.
7. **Row 21, two of the four strips** — the create form's refusal (`NewGame.tsx:441`, :555: a bad seed) and the open
   table's (`OpenTable.tsx:235`, :362, :402: a wrong password), provoked at 390 px headed, pictured; any overflow fixed
   with the 390 cell as the pin; the `Refusal` component (`Refusal.tsx:30+`) keeps its `error` class.

- [ ] **Step 1: The before pictures.**
- [ ] **Step 2: Items 5 and 6 red first**, then green.
- [ ] **Step 3: Items 1–4 and 7**, each with its cell or its picture; every row FIXED (before and after) or STRUCK
  (the picture and one sentence) in the report.
- [ ] **Step 4: The after pictures**; `npm run e2e` green (owed); `npm test` green.
- [ ] **Step 5: Commit** (one commit per row is fine; the last message names the set):
  `git commit -m "polish(web): the door, the lobby and the device card - rows 58, 59, 62, 63, 64, 68 and 21's two strips, off the pictures (R-6.5c2.38)"`.

---

### Task 4b: The owner's-eye polish — the game and the table — Opus (R-6.5c2.38; straw poll rows 60, 61, 66, 67, 71, 72, the game header's "You are ben", 21's deal and chat strips; dossier items 15–16)

**Files:**
- Modify: `web/src/Band.tsx:67–88` (`LobbyBand`) and :101 (`IdentityCorner`) with `styles.css:1835–1856`, :1872–1878,
  :1923–1937, :2521 (row 60); `web/src/Dashboard.tsx:198–210` (`.seat-head`), `web/src/Chips.tsx:68–72`,
  `styles.css:1896–1900` (`.version-tag` in `--yen`) and :538 (`.chip.yen`) (row 61); `styles.css:2514` and :2520 (row
  66, both ≤600 px; the comment :2508); `web/src/OpenTable.tsx:368–383` and `styles.css:2391–2396` (row 67; mock
  `…-m6.5c2-mocks/08-waiting-for-players.webp`); `styles.css:160–185`, :1052–1054, :1063–1064 and `web/src/App.tsx:2054–2058`
  (row 71) with the gate `web/e2e/the_screen_itself.spec.ts:952–969`; `web/src/Dealing.tsx:304` with `styles.css:1038–1042`,
  :1510–1512, and `web/src/NewGame.tsx:271`, :314, :494, `web/src/Index.tsx:295` (row 72); `web/src/App.tsx:1446`,
  :1494–1502 (`signedIn = door.me?.user`, the parsed key; `whoStrip`) against `web/src/SeatCard.tsx:33–38` (the seat's
  typed `name`, `api.ts:169–179`); `web/src/Dealing.tsx:310`, :476 and `web/src/Chat.tsx:533` (the two strips) with
  `Dealing.test.tsx:510`, `Chat.test.tsx:493`.
- The instrument: as Task 4a, at 390, 500, 560, 768, 1024 and 1280 into `$L/task-4b-look/`; the game screens through
  `gates.spec.ts` (1440×900, 1710×869, 2560×1440 — dossier item 16) plus the script for the other widths.

**Interfaces:**
- Consumes: `styles.css` as Task 4a leaves it (find by content).
- Produces: the rules the items name.

**The items, in plain words:**

1. **Row 60** — at 390 px the band's crumbs wrap under the pill (`styles.css:1923–1937`: crumbs 16 px, pill 36 px).
   The pill first, the crumbs on their own line under it, never beside it; picture at 390.
2. **Row 61** — in `.seat-head` the bot's gold `V4` tag (`.version-tag`, `--yen`) sits beside the gold `¥` chip
   (`.chip.yen`, the same gold). The tag moves into the seat's name and takes the door's muted outline (not `--yen`);
   a jsdom cell over the corpus's game payload that the tag is inside the name element.
3. **Row 66** — the two `1lh` declarations at :2514 and :2520 get a `line-height`-based fallback declared BEFORE each
   (an older Safari ignores `1lh` and keeps the fallback); a structural CSS cell that every `1lh` line is preceded
   by its fallback within two lines.
4. **Row 67** — the open table's chat sits in the MAIN column (`OpenTable.tsx:368–383`, "not mock 08's rail"). At
   ≥ 976 px it moves to the rail mock 08 draws; under, it stays; pictures at 390 and 1280.
5. **Row 71** — at 1024 px the banner (`styles.css:160–185` wraps) takes two rows, and at 1280 the "reconnecting…"
   strip (`App.tsx:2054–2058`) wraps it. One row at 1024 (the title's tracking and the menu's padding), the strip on
   its own line under the banner; the gate at `the_screen_itself.spec.ts:952–969` ("one row at 1440 and 1280") gains
   1024 (red first), and the jsdom cell that renders `reconnecting` asserts the strip's own row.
6. **Row 72** — the who-strip survives on the deal screen alone (`Dealing.tsx:304`; wraps at ≤ 480 px); the
   placeholders at `NewGame.tsx:271`, :314, :494 and `Index.tsx:295` truncate at ≤ 500 px. Pictures at 500 and 560;
   fix what shows (a wrap rule; shorter placeholders), strike what does not, with the pictures.
7. **"You are ben" against "Ben"** — the game header's `whoStrip` reads the parsed KEY (`door.me?.user`) while the seat
   card reads the seat's typed `name`. The header reads the display name where one is set (`api.ts:169–179`'s
   `display_name`) and the key otherwise; a jsdom cell over the corpus's game payload that the header and the seat
   card show the same string.
8. **Row 21, the other two strips** — the deal's refusal (`Dealing.tsx:310`, :476: the designed 409 after a flip) and
   the chat panel's (`Chat.tsx:533`: a message over the limit), provoked at 390 px headed, pictured; any overflow fixed
   with the 390 cell as the pin.

- [ ] **Step 1: The before pictures.**
- [ ] **Step 2: Items 2, 3, 5 and 7 red first** (the four cells), then green.
- [ ] **Step 3: Items 1, 4, 6 and 8** with their pictures; every row FIXED or STRUCK in the report.
- [ ] **Step 4: The after pictures**; `npm run e2e` green (owed; `the_screen_itself` with 1024); `npm test` green.
- [ ] **Step 5: Commit**: `git commit -m "polish(web): the game and the table - rows 60, 61, 66, 67, 71, 72, the header's name and 21's two strips, off the pictures (R-6.5c2.38)"`.

---

### Task 5: Lobby paging for Open and Being played, "Abandon and save" made true, the arrival by link — Opus (straw poll rows 84, 90, 73; R-E5, R-E6, R-E7; dossier items 17–19; Review Focus 3)

**Files:**
- Modify: `crates/yokohama-web/src/table.rs:94–117` (`TurnMirror` gains `events: u64`, `#[serde(default)]`), :409–436
  (`TableDoc::view` fills `events` from the mirror and `saves` from what the route hands it), :504–555 (`TableView`
  gains `events: u64` and `saves: bool`); `crates/yokohama-web/src/tables.rs:1884–1935` (the abandon rule factored into
  `pub(crate) fn saves_on_abandon(doc: &TableDoc, archive_on: bool) -> bool` — archive on, `status == Playing`, no
  `archived_at`, `mirror.events > 0` — called by `delete_locked` and by the view's builder) and `persist`'s mirror
  refresh (it stamps `events` beside `to_move`/`over`/`winner`; the mirror is a cache whose conflict is ignored);
  `crates/yokohama-web/src/api.rs` (every `TableView` the routes build passes `games.archive_on()` — the accessor the
  implementer names — so `saves` is the route's fact; `TableDoc::view` alone reads `false`); `web/src/types.ts:573`
  (`TableView.events`, `TableView.saves`); `web/src/wire/contract.ts:658–662` (`TABLE` gains both); `web/src/Abandon.tsx:94`,
  :112–115, :250–257 (the label reads `table.saves` for a hosted table; for a LOCAL id the client keeps
  `archiveDisposition(archive) === "send" && table.status === "playing"` and adds `&& table.events > 0`, since the
  browser host's layer has `NoArchive` and the upload is the client's own); `web/src/Lobby.tsx:91–101` (`fetchLists`),
  :218–226 (`paged`), :342–355 (`page`), :468–476 (`nextCursor`), :558–562 ("more"); `web/src/App.tsx:820–860` (the
  route effect), :654 (`open`), :1722 and :1810 (the screen selection); `web/src/route.ts:45–57`; the cells
  `Lobby.test.tsx:210`, :237, `Abandon.test.tsx`, the routing cell (a new `App.route.test.tsx` beside `App`'s cells, or
  the file the implementer finds them in); the corpus: the 14 table-carrying files
  (`deal.json`, `deal_waiting.json`, `deal_bot_table.json`, `games.json`, `games_next.json`, `games_open.json`,
  `games_playing.json`, `table.json`, `table_open.json`, `table_dealing.json`, `table_watchable.json`, `table_bot.json`,
  `table_foreign.json`, `table_two_open.json`).

**Interfaces:**
- Consumes: `tables::list(user, scope, limit, before)` (`tables.rs:1741–1805`: newest first then `id`, strictly after
  the cursor `<created_at>,<id>`, `next` only when a row is left; limits 20 and 100 at :754–755) and
  `GamesList { games, next }` — every scope's reply already carries `next`; `Scope::Playing` = playing or over
  (:713–743), which the lobby splits into Being played and Finished by `over` (`Lobby.tsx:473–475`), so paging the
  `playing` scope pages both.
- Produces: `TurnMirror.events`, `TableView.events`, `TableView.saves`, `tables::saves_on_abandon`; the lobby's
  `paged: Record<"mine" | "open" | "playing", { games, next } | null>`, `page(scope, cursor)`, a "more" under Your
  tables, under Open tables, and under the Being played / Finished pair (the `playing` scope's, placed after the
  Finished fold); the "opening…" screen App draws while a table route is pending (the M6.3 close's own fix, §6.2
  :635–640) so the lobby is never mounted on a cold `#/t/<id>` load.

- [ ] **Step 1: `saves` and `events`, red.** Lib cells beside the `TableView` cells in `table.rs` and the abandon cells
  in `tables.rs` (:2886, :2922, :2984–3007): a dealing table's view reads `events: 0, saves: false`; a playing table
  with one event and the archive on reads `events: 1, saves: true`; the same with the archive off reads `false`; a
  finished, already-archived table reads `false` (the delete files nothing twice — the existing cell at :2984 says so);
  `saves_on_abandon` and `delete_locked` agree on every one of those four (the delete's own put count). Run `cargo
  test -p yokohama-web --lib saves` → FAIL (no field). Then the fields, the factoring and `persist`'s stamp → PASS.
  A mirror stored WITHOUT `events` (an older document) reads `0` (a `serde(default)` cell).
- [ ] **Step 2: The corpus.** `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`; READ the diff:
  the 14 files gain `"events"` and `"saves"` and nothing else, the values as Step 1's rule predicts for each file
  (the report lists them: which files read `saves: true`); `contract.ts`'s `TABLE` gains both keys; `types.ts`; `npm
  run typecheck`; the corpus pins green.
- [ ] **Step 3: The client renders it, red.** `Abandon.test.tsx`: a corpus payload with `saves: true` reads "Abandon and
  save", one with `saves: false` reads "Abandon" (both from the corpus by the files Step 2 named); a local id with
  `events: 0` reads "Abandon" even when the disposition is `send`. Run → FAIL. Then `Abandon.tsx` → PASS.
- [ ] **Step 4: Paging, red.** `Lobby.test.tsx`: a lobby whose `open` list carries `next` shows "more" under Open
  tables; clicking it fetches `scope=open&before=<cursor>` ONCE (the fake Worker's call log) and appends; a poll that
  replaces page one keeps the appended page (the shape of the `mine` cell at :237); the same for `playing`, whose
  appended rows fall into Being played or Finished by `over` **(Review Focus 3)**. Run → FAIL.
- [ ] **Step 5: The pager per scope.** `Lobby.tsx`: `paged` keyed by scope; `page(scope, cursor)` calls
  `api.list(scope, cursor)` (`api.ts:868–877`, unchanged); `nextCursor(scope)`; the three "more" buttons; the
  drop-on-abandon rule (:361) per scope. → PASS.
- [ ] **Step 6: The arrival, red.** The routing cell: render `App` with `location.hash = "#/t/<id>"` and the fake
  Worker answering `me` and the table; assert the lobby's list routes are NEVER called (no `scope=` in the fake's
  call log), `/api/stream` is never opened, and an element reading "opening…" is on screen until the table answers,
  then the table's screen. Run → FAIL (the lobby mounts for one round trip — `App.tsx:1810`'s fall-through while
  `open(id)` awaits).
- [ ] **Step 7: The route.** `App.tsx`: while a table route is pending (the route effect at :820–860 has fired and
  `open` has not resolved), the screen is `opening…`, never `<Lobby>`; the lobby's `fetchLists` and the house stream
  start only where the lobby is the screen (R-E7). → PASS. Then `npm run e2e` (owed): `joins_by_link.spec.ts:75`
  green, and the whole run.
- [ ] **Step 8: The per-edit gates**, then commit:

```bash
git add crates/yokohama-web/src/table.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/src/api.rs web/src/types.ts web/src/wire/contract.ts web/src/wire/ web/src/Lobby.tsx web/src/Lobby.test.tsx web/src/Abandon.tsx web/src/Abandon.test.tsx web/src/App.tsx web/src/route.ts <the routing cell>
git commit -m "feat(web): 'Abandon and save' is the layer's fact (TableView.saves over the mirror's event count), Open and Being played page, and a table link opens the table (rows 90, 84, 73)"
```

---

### Task 6: The waiting-table expiry, default 7 days, both TTLs on health, and the finished-table pool cell — Opus (R-6.5c2.42; R-E8; dossier items 20–21; Review Focus 2)

**Files:**
- Modify: `worker/src/config.ts:12–24` (`Env`), :37–51 (`ROWS`, held to `Env` by a cell), :125–134 (`ttlDays`; the
  second reader beside it with the same refusal shape: "YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS must be a whole number of days
  above zero — got …"), :262–273 (`moduleConfig` carries `table_ttl_days` and `open_table_ttl_days`); `worker/src/env.ts:61`;
  `worker/src/marker.ts:25` (`Reason` gains `waited`), :35–40 (`TAILS`), :48–62 (`markerSentence`: the waited head),
  :75–83 (`ttlDaysOf` stays the ONE reader of the first; `openTtlDaysOf` beside it); `worker/src/table.ts:842–853`
  (`expiryDue`: the alarm is set at `lastActive + min(openTtl, ttl)` from SQL alone) and :251–263 (`onAlarm`: before
  expiring, `copyFacts` (`lifecycle.ts:537–555`, the ONE sanctioned table parse) gives the status: `open` → expire
  with `waited`; otherwise, if `lastActive + ttl` has not passed → re-arm at it, else expire as today);
  `worker/src/table/lifecycle.ts:255–290` (`expire` carries the reason); `crates/yokohama-worker/src/receipt.rs:473–494`
  (`expire_inner` unchanged unless the reason must reach the layer's DELETE — the marker is the shell's);
  `crates/yokohama-web/src/api.rs:310–359` (`Health` gains `table_ttl_days: Option<u64>`, `open_table_ttl_days: Option<u64>`,
  filled from the layer's configuration — the module's `init` receives them from `moduleConfig`; the NAS passes
  `None`), :361–373 (`health()`); `crates/yokohama-worker/src/lib.rs` (the `init` configuration's two fields);
  `web/src/types.ts:480` (`Health`), `web/src/wire/contract.ts:653–656` (`HEALTH`); `web/src/wire/health.json` (both
  `null`, regenerated); `worker/scripts/corpus-walk.mjs:19` (`HOST_FIELDS.health` gains the two — the pool and the live
  replay) and `web/src/local/corpus.node.test.ts:103–104` (its own `HOST_FIELDS`; each refuses an exemption the hosts
  agree on, :296–301 — the Worker reads 90 and 7, the browser host `null`, so the exemption is real);
  `worker/wrangler.jsonc:89–92` (`YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS: "7"` beside the first, so the guard's drift checks
  and the pool files (both :42) carry it); `worker/RUNBOOK.md:178`, :227, :245 and `worker/README.md:1333`, :1594
  (the variables tables); `deploy/README.md` (one line: the NAS reads no TTL); the cells `worker/test/expiry.test.ts:117–444`,
  :445–555, `marker.unit.test.ts`, `config.unit.test.ts`, and `worker/test/archive.test.ts` (the finished-table cell).

**Interfaces:**
- Consumes: `lastActive(sql)` (`delivery.ts:244–250`, `max(written_at)` over `record`, `table`, `chat`);
  `copyFacts` (`lifecycle.ts:537–555`, off `mod.house_copy(id)`, the seed zeroed — the strings rule's ONE permitted
  parse); `expiryDeadline` and the backoff as they are.
- Produces: `ShellConfig.openTtlDays: number` (default 7 when the variable is absent; the FIRST TTL keeps its own
  absence rule — no expiry at all — and with no first TTL the second is inert); the alarm rule of R-E8 as amended
  here: ARMED at the shorter deadline from SQL, DECIDED at the alarm by the status `copyFacts` reads; the marker head
  `this table waited {N days} for players and was closed on {YYYY-MM-DD}` + the existing tail; `Health`'s two fields;
  `moduleConfig`'s two fields; `HOST_FIELDS` on both replays.

- [ ] **Step 1: The two-deadline cell, red** (`worker/test/expiry.test.ts`, the pool, the clock frozen): a table
  created `open` under `YOKOHAMA_WEB_TABLE_TTL_DAYS=90` and `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS=7`: its alarm is 7 days
  out; advanced 7 days and run, it is gone, its marker reads the waited head, `expired` is logged with the reason; a
  table dealt and moved (`playing`) has its alarm 7 days out too (armed from SQL), and at that alarm it is NOT
  expired — the alarm is re-armed at 90 days from its last activity; a third table created `open` and filled on day
  6 survives day 7 and expires on day 96 **(Review Focus 2)**. Run: `cd worker && npm run build:module:debug && npx
  vitest run --project worker test/expiry.test.ts` → FAIL.
- [ ] **Step 2: The config, the alarm, the marker.** `config.ts` (`Env`, `ROWS`, the reader, `ShellConfig`); `marker.ts`
  (the reason, the head, `openTtlDaysOf`); `table.ts`'s `expiryDue` and `onAlarm` as the Interfaces say; `lifecycle.ts`'s
  `expire` takes the reason. Step 1 → PASS; `marker.unit.test.ts` and `config.unit.test.ts` extended with the head and
  the refusal sentence (red first for each).
- [ ] **Step 3: Health, red.** A layer cell (`cargo test -p yokohama-web --lib health`): the NAS's `Health` serializes
  `"table_ttl_days": null, "open_table_ttl_days": null` → FAIL; a pool cell: the Worker's `/api/health` reads `90` and
  `7` → FAIL. Then the two `Option<u64>` fields with `#[serde(default)]`, the module's `init` configuration carrying
  them from `moduleConfig`; `UPDATE_WIRE_FIXTURES=1 …` → `health.json` gains the two `null`s and nothing else (READ the
  diff); `HEALTH` gains the keys; `HOST_FIELDS` in `corpus-walk.mjs:19` and `corpus.node.test.ts:103–104` gain both
  (their own anti-vacuity cells at :296–301 hold: the hosts differ on them). Both cells → PASS; the pool's corpus
  replay and `test:wasm`'s replay green.
- [ ] **Step 4: The finished-table pool cell** (dossier item 21: the layer's cell at `tables.rs:2984–3007` and the
  pool's refinish-race cell at `archive.test.ts:892` exist; missing is the PLAIN case): a finished table archived at
  game over (`archive-landed` once) is abandoned by its creator → 204, no second `archive-landed`, nothing under
  `abandoned/`, the marker names the archive. Red first only if it fails; else a pin, and the report says which.
- [ ] **Step 5: The documents.** The RUNBOOK's variables table (:165–248) and both READMEs' gain the row; `wrangler.jsonc`
  and both pool files carry `"7"`; the deploy guard's drift checks pass (`node scripts/deploy-guard.mjs` reads green
  on the file, without deploying).
- [ ] **Step 6: The per-edit gates** (`npm run build:module:debug` before the pool; the corpus pins; `npm run
  test:scripts` for the guard), then commit:

```bash
git add worker/src/config.ts worker/src/env.ts worker/src/marker.ts worker/src/table.ts worker/src/table/lifecycle.ts crates/yokohama-worker/src/lib.rs crates/yokohama-web/src/api.rs web/src/types.ts web/src/wire/contract.ts web/src/wire/health.json worker/scripts/corpus-walk.mjs web/src/local/corpus.node.test.ts worker/test/ worker/wrangler.jsonc worker/wrangler.test.jsonc worker/wrangler.open.test.jsonc worker/RUNBOOK.md worker/README.md deploy/README.md
git commit -m "feat(worker): a table still waiting for players expires after YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS (default 7), decided at the alarm by the table's status; health reports both TTLs (R-6.5c2.42)"
```

---

### Task 7: Requests per move — the tally, the refetch policy, the target, the per-10-second peaks — Opus (straw poll row 80; R-6.5c2.43; R-E9; dossier items 22–23)

**Files:**
- Modify: `web/e2e/harness.ts` (a `tally(page, label)` helper the spec's listeners call — the listeners are in
  `web/e2e/two_browsers.spec.ts:115–131` and :200–219, not the harness; dossier item 23), `web/e2e/two_browsers.spec.ts`
  (wires the tally when `YOKOHAMA_WEB_TALLY` is set), `web/src/App.tsx:1140–1200` (`arrive`), :487–500 (`landed`),
  :89–97 (`fetchRun`), :1372–1387; `web/src/TableSections.tsx:87–104` (per `state` change, not while `scrubbing`:
  `api.ports`, `api.labs`, `api.areas`); `web/src/signal.ts:354–357`, :536–559; `web/src/Chat.tsx:345–348`;
  `web/src/Lobby.tsx:328–340` (`once: fetchLists` — three listings per house frame, and every move bumps the house);
  `web/src/OpenTable.tsx:149–165` (`api.table` per frame); their cells.
- Create: `web/scripts/tally-report.mjs`, `web/scripts/tally-report.node.test.mjs`.

**Interfaces:**
- Consumes: the fetches as dossier item 22 lists them (a watcher's move today costs the state, the run's pages, the
  log and up to three sections; an open lobby anywhere adds three listings per move).
- Produces: `YOKOHAMA_WEB_TALLY=<path>` — one JSON line per request the browsers make:
  `{ t: <ms since the run's start>, page: "a" | "b" | "watcher", method, route, status }`, `route` normalized by ONE
  regex (`/api/games/<id>/…`, the query dropped); `node scripts/tally-report.mjs <path> --moves <n>` printing (a)
  requests per move by route and in total, (b) the peak count in any 10-second window per route and in total, (c)
  the same restricted to the oracle paths (`…/join`, `…/unlock`, `/api/archive`); the client's refetch policy as this
  task leaves it, with the target ratified.

- [ ] **Step 1: The report's cell, red.** `tally-report.node.test.mjs`: six lines over two routes at 0, 1, 2, 11, 12,
  13 s read `2 per move` for one route at `--moves 3`, a 10-second peak of `3`, an oracle subset of `0`; a line with
  `/api/games/sim-abc/join?x=1` normalizes to `/api/games/<id>/join`. Run → FAIL.
- [ ] **Step 2: The tally and the report.** `harness.ts`'s `tally` (a sync `appendFileSync` per request when the
  variable is set; the normalizing regex exported for the cell); the spec's two listeners call it; `tally-report.mjs`
  as the Interfaces say. Step 1 → PASS.
- [ ] **Step 3: MEASURE before.** `cd web && YOKOHAMA_WEB_TALLY=$L/task-7-tally/before.jsonl npx playwright test
  two_browsers` (local, the release binary, ONE run); `node scripts/tally-report.mjs $L/task-7-tally/before.jsonl
  --moves <the game's applied moves>` → the by-route table into the report. **STOP: NEEDS_CONTEXT to the
  orchestrator** with the table and a proposed target — the routes a frame fetches without reading the result, named,
  and the number the policy below reaches — and the levers you propose among these: (a) gate `TableSections`' three
  reads on a change the run's steps show (an event naming a port, a lab or an area); (b) gate `OpenTable`'s `api.table`
  on the frame's `events`/`chat` counters; (c) the lobby's three listings only when the house frame's `seq` moved AND
  the lobby is the screen; (d) as a last resort, ONE additive layer route `GET …/sections` returning ports, labs and
  areas together, with its own corpus file — an additive route only, no existing payload changing shape, named in the
  report. **The orchestrator ratifies the target and the levers as a `Ruling:` line (R-E9).**
- [ ] **Step 4: The policy, red first.** A jsdom cell per lever taken: on a frame whose counters did not move, the
  panel fetches nothing (the fake Worker's call log); on a frame where they moved, it fetches once. Run → FAIL. Then
  the levers; `TableSections`' `scrubbing` guard kept. → PASS.
- [ ] **Step 5: MEASURE after**, the same command into `after.jsonl`; the report beside the before; the target met or
  the gap named with its reason. `npm run e2e` (owed) green — the walk still walks, the watcher holds one stream.
- [ ] **Step 6: The peaks for R-6.5c2.43** — from the after run, the oracle-path peak per 10 seconds and the whole-API
  peak, both in the report; the orchestrator adds the replay's peak in Task 10.J.
- [ ] **Step 7: The per-edit gates**, then commit:

```bash
git add web/e2e/harness.ts web/e2e/two_browsers.spec.ts web/scripts/tally-report.mjs web/scripts/tally-report.node.test.mjs web/src/App.tsx web/src/TableSections.tsx web/src/Chat.tsx web/src/Lobby.tsx web/src/OpenTable.tsx web/src/signal.ts <their cells; the corpus file if (d) was taken>
git commit -m "perf(web): a frame fetches only what moved - requests per move measured by route before and after (row 80); the tally is the rate-limit value's instrument (R-6.5c2.43)"
```

---

### Task 8: The door window's instruments — `wrangler.open.jsonc`, the guard's `--config`, the protected path list, the screenshot generator — Sonnet 5.5 (R-6.5c2.39, .44; R-E10, R-E11, R-E12; dossier items 24–25, Part 2; Review Focus 4)

**Files:**
- Create: `worker/wrangler.open.jsonc`, `worker/scripts/access-paths.mjs`, `worker/scripts/access-paths.test.mjs`,
  `web/e2e/readme_shots.spec.ts`.
- Modify: `worker/scripts/deploy-guard.mjs:187`, :286, :348, :354, :393 (the five sites that name `"wrangler.jsonc"`
  take the configuration the CLI at :421–437 was given by `--config <file>`, default `wrangler.jsonc`; check 2's
  directory and checks 3–7 judge the named file; checks 5 and 6 hold it to production's blocks — dossier item 24),
  `worker/scripts/deploy-guard.test.mjs:415–545`, `worker/package.json:18` (`deploy:open`), `web/e2e/live.ts:61–76`
  (`README_SHOTS` beside `EXIT`; the spec's name beside `EXIT_SPEC`), `web/playwright.config.ts:163–173` (collected only
  under the switch, like the exit's), `web/e2e/live.node.test.ts` (the new spec counts as live: its `request.*` through
  `liveRequest`, `web/e2e/live.ts:251`), `worker/README.md` §"The two doors" (the open deployment's paragraph).

**Interfaces:**
- Consumes: `doorRules` (`deploy-guard.mjs:248–282`), `POOLS` (:180), `openDoorRefusals` (`config.ts:188–218`); the
  shell's router `worker/src/route.ts:69–84` (`route()`), `admin.ts:77–89`, `index.ts:109`; the live tier's rules.
- Produces: `wrangler.open.jsonc` — `wrangler.jsonc` with `YOKOHAMA_WEB_IDENTITY: "open"`, no `YOKOHAMA_DEV_DOOR`, no
  `ACCESS_TEAM_DOMAIN`/`ACCESS_AUD`, `workers_dev: false`, `preview_urls: false`, the same route, `assets`,
  `durable_objects`, `migrations`, `r2_buckets`, `observability` and other `vars` (the archive, both TTLs, the chat
  three); `npm run deploy:open` = `node scripts/deploy-guard.mjs --config wrangler.open.jsonc && wrangler deploy
  --config wrangler.open.jsonc`; `node scripts/access-paths.mjs` printing one destination per line —
  `yokohama.moore.nyc/api/games`, `/api/chat`, `/api/stream`, `/api/archive`, `/api/me`, `/api/admin`, `/enter` —
  derived from `route.ts`'s and `admin.ts`'s prefixes less the three the client needs signed out (`health`, `art`,
  `labels`; dossier item 25: the art manifest is read as a static file and labels only when signed in);
  `readme_shots.spec.ts` writing the TEN pictures of `worker/README.md` §1.4 under their names.

- [ ] **Step 1: The guard's cells, red.** `deploy-guard.test.mjs`: `--config wrangler.open.jsonc` reads THAT file's door
  and prints it (`… deploys the open door — no identity service …`, `DOORS.open`'s sentence); a copy with
  `YOKOHAMA_DEV_DOOR: "on"` is refused **(Review Focus 4)**; one with `YOKOHAMA_WEB_ADMINS` in `vars` is refused; one
  whose `assets` block differs from `wrangler.jsonc`'s is refused; the plain run still judges `wrangler.jsonc`; the
  committed-files cell (:532–545) covers the new file. Run: `cd worker && npm run test:scripts -- deploy-guard` → FAIL.
- [ ] **Step 2: The guard and the file.** The five sites read the CLI's configuration; the drift checks compare
  `[wrangler.jsonc, wrangler.open.jsonc, ...POOLS]` to production; `wrangler.open.jsonc` as the Interfaces say, with a
  header comment (the state-1 configuration of R-6.5c2.39, deployed for the window and never left live; the operator
  secret must be deleted before it deploys, since the shell refuses `open` beside it, `config.ts:188–218`);
  `package.json`'s `deploy:open`. Step 1 → PASS. Then `npx wrangler deploy --config wrangler.open.jsonc --dry-run`
  prints the bindings (`YOKOHAMA_WEB_IDENTITY: "open"`, no dev door) — quoted in the report, no upload.
- [ ] **Step 3: `--var` measured** (R-E10): `npx wrangler deploy --dry-run --var YOKOHAMA_DEV_DOOR:off` against
  `wrangler.jsonc`: does the printed binding read `off`? YES or NO in the report; on NO, add `wrangler.gate.jsonc` (the
  worker door with `YOKOHAMA_DEV_DOOR: "off"`, otherwise `wrangler.jsonc`) under the same guard rules and a
  `deploy:gate` script, and its own guard cell.
- [ ] **Step 4: The path list, red.** `access-paths.test.mjs`: the printed list equals the seven; a STRUCTURAL cell reads
  `route.ts` and `admin.ts` with the script's own regex for `/api/<segment>` prefixes and asserts each is `health`,
  `art`, `labels` or in the list, so a new prefix fails until the list names it (and a prefix the router dropped
  fails too). Run → FAIL (no script).
- [ ] **Step 5: The script.** `access-paths.mjs` reads the two files and prints the destinations. Step 4 → PASS.
- [ ] **Step 6: The screenshot generator, gated.** `readme_shots.spec.ts` under `YOKOHAMA_WEB_README_SHOTS=1` AND the
  live target (otherwise skipped out loud — a cell in `live.node.test.ts`'s style pins the gate); three contexts through
  the dev door as `alice@readme.test`, `ben@readme.test` and `carol@readme.test` (`liveContextOptions`); the ten
  scenes of `worker/README.md` §1.4 (:223–278), each at 1280×832 unless said: `door-desktop.png` (Alice's door, signed
  in — the blurb, the link, the footer), `door-phone.png` (390×844), `lobby-desktop.png` (Ben's lobby: Alice's open
  table "Friday game", 2P, locked, one seat open; the Finished fold; the house chat folded), `open-table-desktop.png`
  (Alice's own open table), `join-desktop.png` (Ben joining with the password), `deal-desktop.webp` (Alice choosing
  her card), `game-desktop-player.webp` (Alice's first turn), `game-desktop-watcher.webp` (Carol: "You are
  spectating"), `game-phone.webp` (Alice's game at 390×844), `marker-404.png` (Ben's lobby after Alice abandons: the
  marker line) — the seven the spec names and the three beside them, since the generator's cost is the same. Every
  `request.*` through `liveRequest`; `expect(res.ok())` never `toBeOK()`; the tables abandoned in `afterEach`; **a
  local DRY run first** — `YOKOHAMA_WEB_README_SHOTS=1 YOKOHAMA_WEB_README_SHOTS_DIR=<scratch> npx playwright test
  readme_shots` against the local server (the `.test` identities through the open door) writes the same ten files
  into the scratch directory, so the spec is proven before it spends the quota.
- [ ] **Step 7: The document.** `worker/README.md` §"The two doors": the open deployment's recipe in one paragraph
  (the secret deleted first, `npm run deploy:open`, what a visitor then sees, the revert), pointing at
  `worker/RUNBOOK.md` §19 for the window's record.
- [ ] **Step 8: The per-edit gates** (`npm run test:scripts`; `npm run typecheck` in `web/`; the dry run's ten files
  looked at), then commit:

```bash
git add worker/wrangler.open.jsonc worker/scripts/access-paths.mjs worker/scripts/access-paths.test.mjs worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs worker/package.json web/e2e/readme_shots.spec.ts web/e2e/live.ts web/playwright.config.ts web/e2e/live.node.test.ts worker/README.md
git commit -m "chore(worker): the door window's instruments - the open configuration under the guard, the protected path list derived from the router, the README screenshot generator (R-6.5c2.39)"
```

---

### Task 9: The review, one fix wave, the battery, and the per-edit set at the code head — ORCHESTRATOR (spec §3; R-6.5c2.13; R-E14, R-E17)

No code. The order is D's: review → one fix wave → the scoped re-reviews → the battery → the per-edit set and the
e2e at the code head. **R-E17 (a ruling of this plan):** the fifteen release gates and the ONE recorded sequence run
ONCE, at the FINAL reviewed head in Task 16.A, because Part 3's document tasks add two doc-pinning cells after the
deploys, and the fences prove the gates' inputs (the engine, the CLI, the oracle) byte-identical at every commit, so
their readings cannot differ between the code head and the final head; the deploys need the per-edit set and the
e2e green, which this task runs. The ledger is `L=.superpowers/sdd/2026-09-29-web-frontend-m6.5c2e-plan`.

**A. The branch is whole**

- [ ] **A.1** `git status` clean; `progress.md` holds a DONE line and a review line for each of Tasks 1–8; HEAD is Task
  8's reviewed commit.
- [ ] **A.2 The fences**, output quoted in the close: the six fenced crates' diff against `c06c4b36` prints nothing;
  `SEMANTICS_REV` 7; `git diff --stat c06c4b36 -- web/src/wire web/src/wire-script` lists ONLY what Tasks 2, 5 and 6
  regenerated (their reports name the files); no bot behaviour moved.

**B. The review (Fable), one fix wave, the scoped re-reviews**

- [ ] **B.1** One **Fable** reviewer over `git diff c06c4b36..HEAD`, the spec at revision 2.9, the dossier, this plan and
  the ledger, in an EXPORT: the vacuity lens first (every new catcher reverted against its fix); **the inventory tried
  with `/index.html` re-added** (the cell must refuse); the navigation answer's content type tried with the header
  dropped; `saves` tried true on a dealing table; the two-deadline expiry tried with the statuses swapped; the
  build-value checker tried with a control character; the sentences this plan and the tasks' reports wrote about
  themselves TRIED (the top yield every cycle); the fences. Report: `$L/final-review.md`.
- [ ] **B.2** ONE fix wave, each fix a NEW commit ("as a new commit" in every dispatch), Opus implementers, disjoint files
  if parallel; a fix that changes what a browser shows owes the e2e. A triage line per finding.
- [ ] **B.3** The scoped re-reviews of the wave — an Opus one and a Fable one, each in its own export. Their head is
  **the code head** for everything below; Task 16.A's sequence runs at the final head.

**C. The mutation battery**, at the code head, an Opus agent on an EXPORT: each row applies the mutant, sees the named
catcher fail BY NAME, reverts, records the diff. The tasks' reports name the catchers; a catcher with no row gets one;
a row whose catcher does not fail is a FINDING. Report: `$L/battery.md`.

| # | the mutant | the catcher |
|---|---|---|
| 1 | the inventory lists `/index.html` again | Task 1's no-HTML-entry cell (Review Focus 1) |
| 2 | the navigation answer served with the cached body's own content type | Task 1's navigation cell |
| 3 | `/shell.bin` served with no cache header on the NAS | Task 1's `assets.rs` cell |
| 4 | the reader expects the period again | Task 1's reader cell |
| 5 | the house seed's `Err` arm silent again | Task 2's log cell |
| 6 | the lobby cells' frame hand-built again | Task 2's fixture-reading cell |
| 7 | `checkText` accepts 401 characters | Task 3's checker cell |
| 8 | the footer drawn with the notice unset | Task 3's footer cell |
| 9 | the blurb drawn above the title | Task 3's door cell |
| 10 | a polish rule reverted (one per row, from 4a's and 4b's reports) | the named cell or picture |
| 11 | `saves` true on a dealing table | Task 5's `saves` cell |
| 12 | the open-scope pager dropped | Task 5's paging cell (Review Focus 3) |
| 13 | a cold `#/t/<id>` load draws the lobby first | Task 5's arrival cell |
| 14 | the waiting-table deadline ignores the status | Task 6's two-deadline cell (Review Focus 2) |
| 15 | `open_table_ttl_days` missing from health | Task 6's health cell |
| 16 | a finished, stamped table's abandon files a second save | Task 6's plain pool cell |
| 17 | the tally's route normalization dropped | Task 7's tally cell |
| 18 | a per-frame fetch re-added | Task 7's counting cell |
| 19 | `wrangler.open.jsonc` names the dev door on | Task 8's guard cell (Review Focus 4) |
| 20 | the router gains a prefix the list lacks | Task 8's access-paths cell |
| 21 | `readme_shots` collected in a plain run | Task 8's gating cell |

**D. The per-edit set and the e2e at the code head**, in the foreground or polled: every Rust suite, the `identity::`
cells, the `wasm32` check, clippy, both rustdocs, the four bridge node suites, the Worker's four commands (the debug
module first), the client's four; `npm run e2e` last. Every reading against Task 1 Step 0's baseline and the tasks'
readings; a red cell re-runs ALONE before it is believed. The readings go to `$L/code-head-readings.md`.

---

### Task 10: The live readings against E's release — ORCHESTRATOR (spec §4 row E's Part 2; R-6.5c2.33, .37, .39, .41, .43, .44; R-E13, R-E15)

No code. Every step is the orchestrator's, at will (R-6.5c2.37), over WARP as the owner and through the credential
where a claimed identity is needed (`worker/RUNBOOK.md` §4's header file, mode 600, deleted after). **Two UTC days**
(R-E15). The transcript of every account step is `$L/door-window.md` (day 2) and `$L/live-day1.md` (day 1); every
count from the Workers Observability query API is a `calculations` `count` and every shell line is found by the
`needle` (`worker/RUNBOOK.md` §7); a fresh `wrangler tail --format json` per live run, its `.err` read for
"Reconnecting" before a count is believed.

**Day 1**

- [ ] **A. Pre-flight.** `curl -A 'Mozilla/5.0' https://yokohama.moore.nyc/api/me` names the owner (Access's 302 means
  the owner re-authorizes WARP); `cd worker && npx wrangler whoami`; `df -h`; **the quota guard's first reading**
  (GraphQL, one dataset per call: the day's Worker requests and Durable Object rows written; HOLD above 80,000).
  **Prepare the two pinned browsers BEFORE any deploy** (Review Focus 1): two Playwright persistent contexts, one per
  host, each loading `/` at D's release until the service worker is `activated` and the door reads "Ready to play
  offline"; then `cf_clearance` cleared in each. They stay open.
- [ ] **B. The release build, ONCE**, with the three build values: `YOKOHAMA_PROJECT_URL='https://github.com/jmoore2333/yokohama-assistant/'`,
  `YOKOHAMA_SITE_NOTICE='Not affiliated with Asmodee'`, and `YOKOHAMA_SITE_BLURB` as the OWNER gives it in the
  planning session's follow-up (blank if none is given): `cd web && … npm run build` (`grep -c debug_trap
  src/local/wasm/yokohama_wasm.js` prints 0; the precache's four lines recorded — the shell line now names `shell.bin`),
  then `cd ../worker && npm run build`; the module figures re-read for the `Cargo.toml` paragraphs.
- [ ] **C. The Worker's twenty-fifth deployment** (`worker/RUNBOOK.md` §1–§2; the worker door, the dev door on):
  `npm run deploy` with the tail listening; the guard's door line and its checks; the assets uploaded (`shell.bin`
  among them); the nine `vars` unchanged; the secrets inherited (`wrangler secret list` names both). §5's readings:
  `/api/health` (now with `table_ttl_days: 90` and `open_table_ttl_days: 7`), `/api/me`, `/sw.js` naming B's release,
  `/shell.bin` 200 `application/octet-stream` at the release's bytes (its digest equals the inventory's) with
  `cache-control: no-store` and a `cf-cache-status` that is never `HIT` — read twice, a minute apart — `/index.html`
  200 with the edge's snippet (unchanged, by design).
- [ ] **D. THE UPDATE READING on the Worker** (Review Focus 1; the proof R-6.5c2.41 names): in the pinned Worker context
  from A, reload `/` ONCE (a plain reload, not Shift+Reload): the new `sw.js` is fetched, its install fetches
  `/shell.bin`, the registration reaches `activated` with ONE new cache named B's release, the old generation
  RETAINED beside it, and the door — after the update offer, or after every old page has closed, whichever the
  spec's rule gives — reads the new release: the blurb, the link, "Ready to play offline". Recorded with the
  caches' names and the door's text. A failure here is a STOP: nothing else deploys until Task 1's mechanism is
  re-read.
- [ ] **E. The NAS's sixteenth deployment as ONE script** (`deploy/RUNBOOK.md` §2; D's `nas-build.sh` and `nas-swap.sh`
  copied into this ledger with the tag `yokohama-web:0.6.0-m6.5c2e` and this session's scratchpad path): the context's
  size, sha256, entry count and `._` count 0; `/.dev.vars` 404 before the POST; the image's size and layers; the swap's
  downtime beside D's 1.50 s; §3.2's readings (`/api/me` names the owner — a STOP otherwise; `/sw.js` B's release;
  `/shell.bin` as on the Worker: `no-store`, `cf-cache-status` never `HIT`, the release's bytes). Then **the update
  reading on the NAS** in the pinned NAS context, as D above — and, since this host sits behind the zone's cache, a
  second install reading in a FRESH profile inside the first two hours of the deploy (dossier item 3's window).
- [ ] **F. The live tiers**, the dev door still on: the quota guard; the live replay TWICE, sequentially (the reset only if
  `archive-walk.json`'s records moved — A.2 of Task 9 says); a FRESH tail, then `cd web && YOKOHAMA_WEB_E2E_TARGET=live
  npx playwright test two_browsers joins_by_link`; the game's requests and rows from the query API (the per-move
  figures beside D's ~13 / ~2 / ~52).
- [ ] **G. The screenshots** (R-E12; R-6.5c2.32): `cd web && YOKOHAMA_WEB_README_SHOTS=1 YOKOHAMA_WEB_E2E_TARGET=live
  npx playwright test readme_shots` writes the ten files of `worker/README.md` §1.4 into `worker/docs/img/`
  (`door-desktop.png`, `door-phone.png`, `lobby-desktop.png`, `open-table-desktop.png`, `join-desktop.png`,
  `deal-desktop.webp`, `game-desktop-player.webp`, `game-desktop-watcher.webp`, `game-phone.webp`, `marker-404.png`);
  **look at every picture** (the caption Task 15 writes must be true of it); the spec's own tables abandoned in its
  `afterEach`.
- [ ] **H. The phones** (R-6.5c2.33; R-E13). `xcrun simctl boot 5CB920DD-1CCC-4E7A-869A-B0B83F8C628B` (the iPhone 17 Pro,
  iOS 26.4) and the iPad Pro 13-inch (M5) `FE5B3632-72B4-4B3F-B41C-84B917A93530` (iOS 27.0), `open -a Simulator`;
  `mcp__mobile__mobile_list_available_devices` lists both `online`. Then TWO Sonnet agents in parallel, one per device,
  each with this brief, then ONE for Android (`emulator -avd Medium_Phone_API_36.1 -gpu host`, headed) after both
  report. Nothing else runs on this Mac meanwhile (no Playwright, no cargo).

  > **The phones brief** (device id `<id>`, the report `$L/phones-<device>.md`, pictures under `$L/phones-<device>/`).
  > You drive ONE simulator through the mobile bridge — `mcp__mobile__mobile_open_url`, `mobile_list_elements_on_screen`
  > (read the screen by refs and coordinates, never by screenshots alone), `mobile_click_on_screen_at_coordinates`,
  > `mobile_type_keys`, `mobile_swipe_on_screen`, `mobile_press_button` (HOME, the lock button), `mobile_take_screenshot`
  > (one per screen, named `<nn>-<screen>.png`), `mobile_get_device_logs` where a screen misbehaves; pass `<id>` to every
  > call; coordinates are the screen's logical points as `mobile_get_screen_size` reports them. This Mac's WARP passes
  > the owner's identity, so both hosts open signed in — `https://yokohama.moore.nyc` (the Worker) and
  > `https://yokohama-nas.moore.nyc` (the NAS) — with no login step; if a Cloudflare login page appears, STOP and report
  > it. The three questions, each answered with what you saw and a picture: (1) **the socket and the wake** — on the
  > Worker, open a table you create against the bot, make two moves, press HOME for 60 seconds, return: does the game
  > screen read "live" again and does the bot's next turn walk? then lock the screen for 60 seconds and return: the same
  > question; (2) **persistent storage** — on either host, play the bot locally ("Play the bot here"), make one move, note
  > whether the browser asks to keep storage, reload the page: is the game listed on the door? add the site to the Home
  > Screen (Share → Add to Home Screen), open it from there: is the same game listed, or does the home-screen app hold its
  > own storage? (3) **anything that blocks** — a control you cannot reach or tap, a picker that does not open, text you
  > cannot read, a screen that does not fit. Then one picture per screen for the layout: the door, the lobby, a new
  > table, the set table, the game (portrait and landscape), the chat open, the final score if reached. Rules: no
  > subagents; no Monitor or event waits; do not touch the other simulator; never print a cookie or a token; the owner's
  > address is "the owner's address" in your report; write the report as you go, and reply with its path and three lines.

- [ ] **I. Day 1's records**: `$L/live-day1.md` complete; the quota after; the pinned contexts closed.

**Day 2** (the quota guard first; the live tiers and the screenshots are done, so the dev door may go down)

- [ ] **J. The rate-limit rule at the measured value** (R-6.5c2.43). The value: Task 7's peak per 10-second window on
  the oracle paths (join, unlock, `/api/archive`) from the local tally, and the live replay's own peak on those paths
  from day 1's replay log (rows per 10 s on the oracle paths), the larger of the two times two, rounded up to a ten.
  The change, through the Cloudflare MCP (`PUT /zones/<zone>/rulesets/phases/http_ratelimit/entrypoint` with the ONE
  rule kept by its id, `0cbb21be3dc44896bb780926f35e4639`): the expression becomes
  `(http.request.uri.path eq "/") or (http.host eq "yokohama.moore.nyc" and (http.request.uri.path ends_with "/join" or http.request.uri.path ends_with "/unlock" or http.request.uri.path eq "/api/archive"))`
  and `requests_per_period` the value; `period` 10, `mitigation_timeout` 10, `characteristics` `["ip.src",
  "cf.colo.id"]`, `action` `block` as today. Read back. **The burst:** through the dev door, `value + 5` `POST …/join`
  attempts with a wrong password in under 10 seconds from this Mac → the first `value` answer the layer's 403, the rest
  the edge's block (429 or 403 with `cf-mitigated`), and 10 seconds later a join answers the layer again. Then the live
  replay ONCE under the rule → HOLDS (the rule never fires on the walk). The rule STAYS (the owner: "enable it so we
  can know it's tested/vetted"); the value goes into the lane document as the Worker-side default (Task 11).
- [ ] **K. State 1 — no door** (R-6.5c2.39). In `worker/`: `npx wrangler secret delete YOKOHAMA_WEB_ADMINS` (the shell
  refuses to start with it beside `open`); `npm run deploy:open` (Task 8's configuration: the `open` door, no dev door;
  the guard's door line reads `open`); while Access still fronts the hostname, from the owner's browser: `/api/me`
  reads `{"mode":"open", …}` and the lobby shows the name step. **Then the Access PUT, state 1**, through the MCP on the
  ONE application `11cbcf68-96cd-4310-baee-f2ade812155d` (rehearsed on a throwaway on 2026-09-29: the audience did not
  change): `PUT /accounts/<account>/access/apps/<id>` with the application read in full first and the body carrying
  `name`, `type: "self_hosted"`, `domain: "yokohama-nas.moore.nyc"`, `self_hosted_domains: ["yokohama-nas.moore.nyc",
  "yokohama-cli.moore.nyc"]`, `destinations` the same two as `{type: "public", uri}`, `session_duration`,
  `app_launcher_visible`, `allowed_idps`, `auto_redirect_to_identity`, `http_only_cookie_attribute` as read, and
  `policies` as `[{id, precedence}]` for its three; read back: the two hostnames, the `aud` unchanged.
  **The readings:** a clean Chromium (no cookies, a plain profile): `/` 200 with the door, the blurb, the link and the
  footer; the name step; a table created and played by TWO clean contexts with two typed names for ten moves each,
  with the tail attached — **row 114's plain two-person game**: its requests, rows written and rows read per move from
  the query API; **the operator surface**: `GET /api/admin/tables` as either name answers the layer's 404 (no secret,
  no operator); a typed name that is the owner's address (Review Focus 4): the same, by design (R-6.5c2.3 — recorded,
  and the owner's address never printed); the burst of J from a clean context → blocked at the value; the archive:
  the stranger's finished game lands under `rev7/` in the bucket (the tail's `archive-landed`). Every reading into
  `$L/door-window.md` with its time.
- [ ] **L. State 2 — the gate on the paths.** `printf '%s' "$(git config user.email)" | npx wrangler secret put
  YOKOHAMA_WEB_ADMINS` (the value never in a document); the worker door with the dev door OFF — Task 8's measured
  shape (`node scripts/deploy-guard.mjs && npx wrangler deploy --var YOKOHAMA_DEV_DOOR:off`, or `--config
  wrangler.gate.jsonc`); the guard's door line reads `worker`. **The Access PUT, state 2**: the same body with
  `self_hosted_domains` and `destinations` = the two hostnames plus `access-paths.mjs`'s printed list VERBATIM
  (`yokohama.moore.nyc/api/games`, `/api/chat`, `/api/stream`, `/api/archive`, `/api/me`, `/api/admin`, `/enter`);
  read back. **The readings:** a clean Chromium: `/` 200 (no login), the door, a local game against the bot for five
  moves with the tail reading ZERO invocations for that visitor (single player costs nothing, R-6.5b2.7), "Play with
  others" → `/enter` → Access's login page; **the owner's second identity** (the owner signs in a second address on
  the allow-list, in a clean profile): `/enter` once, then the lobby, a table, its stream, its chat, the archive fact
  and `/api/me` with NO second login — the single-audience reading (row 108); `GET /api/admin/tables` as the second
  identity → the operator gate's refusal of an unlisted person (row 119); a third context with no session: every
  path on the list answers 302 (a navigation) or 401 with `WWW-Authenticate` (an `X-Requested-With` request), `/` and
  `/shell.bin` 200 — the negative reading of the M6.5b2 spec §3.2. Every reading into `$L/door-window.md`.
- [ ] **M. The revert.** The Access PUT with the three hostnames as read at K's start (the exact body saved in the
  transcript, the `aud` unchanged); `npm run deploy` (the plain deploy restores `wrangler.jsonc`'s vars: the worker
  door, the dev door on); `wrangler secret list` names both secrets. **The after-check:** `/api/me` over WARP names
  the owner; a clean context at `/` meets Access's login; the credential + the claimed header through the dev door
  names its caller; the live replay ONCE → HOLDS; the quota after. The flip recipe — the three PUT bodies and the two
  deploy commands, in order, with their read-backs — is written into `worker/RUNBOOK.md` §19 as a dated record AND a
  recipe, so the day the repository goes public it is one section to follow.
- [ ] **N. Day 2's records**: `worker/RUNBOOK.md` §19 (the twenty-fifth and the window's deploys, each a dated
  sub-section: what changed, the readings, what it does NOT claim, the rollback); `deploy/RUNBOOK.md` §3.19 (the
  sixteenth); the quota readings; the pinned-browser readings; the phones' three reports; the screenshots' names; the
  rate-limit value and its burst reading. Nothing here is a claim beyond its reading.

---

### Task 11: The lane document, `docs/web-lane.md` — Opus (R-6.5c2.32, .45; spec §2.11; the D8 draft; dossier Part 3, items 26, 27)

**Files:**
- Create: `docs/web-lane.md`.
- Create: `crates/yokohama-web/tests/lane_document.rs` (the API map's drift cell).
- Create: `worker/scripts/lane-variables.test.mjs` (the variables table's drift cell; it runs under `npm run test:scripts`).
- Read: `.superpowers/sdd/2026-09-26-web-frontend-m6.5c2-plan/drafts/D8-tasks-24-28.md:1909–2453` (the draft's template,
  its move script and its Step 3 corrections list — written for a single-plan era, so every sentence is RE-READ against
  the tree at HEAD, never copied on trust), `CLAUDE.md:16, 35–56, 107–150` (the five passages the document takes IN,
  verbatim, corrected), Task 10's records, the dossier's variable inventory (item 26) and route list (item 27).

**Interfaces:**
- Consumes: Task 10's readings (`$L/live-day1.md`, `$L/door-window.md`, the phones' reports); Task 7's report (the
  requests figure and the per-10-second peaks); Task 8's `access-paths.mjs` list.
- Produces: the anchors Task 12's pointers name — `#2-the-suites-and-the-gates`, `#4-the-architecture-and-the-standing-rules`,
  `#7-the-release-version-rule`, `#11-the-records-of-record`, `#12-every-variable`, `#13-the-api-map`,
  `#14-getting-started`; the by-design list Task 14 and Task 15 cite.

- [ ] **Step 1: The API map's cell, red first.** `crates/yokohama-web/tests/lane_document.rs`: read `docs/web-lane.md`,
  find the section `## 13. The API map`, parse every table row whose first cell is `` `METHOD /path` ``, and assert the
  set of `(method, pattern)` for rows marked host `layer` equals the layer's own route table (`api::routes()` as the
  dossier's item 27 names it), and that every row marked `axum` or `worker` or `browser` names a route the dossier's
  item 27 lists (a static list in the test, one line each, with a comment that it is the document's own pin). Run:
  `cargo test -p yokohama-web --test lane_document` → FAIL (no document). The cell stays in the per-edit set.
- [ ] **Step 2: The variables cell, red first.** `worker/scripts/lane-variables.test.mjs`: grep the tree's code for every
  `YOKOHAMA_[A-Z0-9_]+` and `DEV_DOOR_CLIENT_ID` and `VITE_[A-Z_]+` name (the same directories the dossier's item 26
  scanned, listed in the cell), and assert each appears in `docs/web-lane.md`'s `## 12. Every variable` table's first
  column, and that every name in the table is read somewhere in that code (a name in the table that nothing reads is a
  failure too). Run: `cd worker && npx vitest run -c vitest.node.config.ts scripts/lane-variables.test.mjs` → FAIL.
- [ ] **Step 3: Write `docs/web-lane.md`** from the D8 draft's template, in this order, every count and reading the
  close's (Task 10's, Task 9.D's) and never this plan's: **§1** what the lane is and where it stands (the two hosts at
  E's release, the door reverted, the lane quiet after E); **§2** the suites and the gates (CLAUDE.md's passages A and B
  moved VERBATIM, then corrected: the counts at the final head, `shell.bin` in the precache line, the reader's period,
  the two new doc cells named); **§3** the testing strategy table (the D8 draft's rows re-read: the phones row now the
  simulators over WARP with the three questions; the deploy readings row gains the UPDATE reading with a pinned
  browser; the live tiers as run; the door window's states); **§4** the architecture and the standing rules (passage C
  moved verbatim, corrected — the shell asset, the three build values, the waiting-table TTL, `saves`, the paging, the
  arrival, the count hosts); **§5** the traps — the 27 by reference plus M6.5c2's 28–32 from the draft and E's: 33 the
  edge's JavaScript Detections snippet and the `cf_clearance` trap (the update-path defect as §10.11 measured it); 34 a
  reader's expected sentence drifts with the screen (the period); 35 the Free plan's ONE rate-limit rule per zone,
  shared across the zone's sites; 36 `wrangler deploy --var` and the file's vars (as Task 8 measured); 37 the operator
  secret refuses `open` (delete before an open deploy, put after); **§6** what a save file says (the draft's, checked);
  **§7** the release-version rule (the draft's, checked); **§8** the public door's preconditions — the FULL list
  (spec §2.10's six and the umbrella's three) with each one's STATE after E (met in D / vetted in E's window / recorded
  with its condition), the window's readings by date, the rate-limit value as the Worker-side default, and the flip
  recipe's pointer (`worker/RUNBOOK.md` §19); **§9** how the documents are built (the draft's, plus the two doc cells);
  **§10** operating facts (the draft's, plus the phones recipe, the pinned-browser update reading, the query API's
  `needle` rule); **§11** the records of record (passage D moved verbatim; E's row added); **§12 Every variable** —
  one table from the dossier's item 26: name · kind (build value / launch variable / Worker var / secret / test-only /
  orchestrator-only) · default · what it means · which host reads it · where (file); **§13 The API map** — one table:
  `METHOD /path` · host (`layer` / `axum` / `worker` / `browser`) · who may call it · what it answers, in a few words,
  every route from the dossier's item 27; **§14 Getting started** — three short paths: play at the hosted site (an
  Access login, the lobby, a table), self-host in Docker (`deploy/README.md`), play the bot in a browser with no
  server; and **the defaults both deployments run** as one small table (the NAS: `cf-access`, `dir` archive, `sse`,
  no TTL; the Worker: `worker` door, the dev door on behind Access, `r2`, `ws`, TTL 90 and 7, chat 500/500, the three
  build values as E's release set them); **the by-design list** (rows 74, 83, 91 with their reasons; the four
  `expect("a game")` sites; the NAS's waiting tables). Every `CLAUDE.md`-moved passage keeps its words except where a
  fact moved, and the report lists every corrected sentence.
- [ ] **Step 4: The two cells green** (Steps 1 and 2 re-run) and the links resolve (D's link script from
  `.superpowers/sdd/2026-09-26-web-frontend-m6.5c2-plan/` Task 26 Step 15, or a five-line equivalent: every
  `](…#anchor)` in the file resolves to a heading's slug).
- [ ] **Step 5: The per-edit gates**, then commit: `git add docs/web-lane.md crates/yokohama-web/tests/lane_document.rs
  worker/scripts/lane-variables.test.mjs` and `git commit -m "docs(lane): the web lane's operating record, with the
  API map and every variable pinned by a cell"`.

---

### Task 12: `CLAUDE.md`'s five web passages to pointers, the read-order line, the counts in place — Opus (R-6.5c2.32; spec §2.11)

**Files:**
- Modify: `CLAUDE.md` — the five passages (found by their first words, since Task 3 and Task 6 may have edited
  lines): **E** the "Where things stand" web bullet (`:16`); **A** the web suites (`:35–56`, from `- Web (server):`
  to before `- The BGA BRIDGE's browser half`); **B** the card-art command (`:87`); **C** the browser lane's
  architecture (`:107–126`); **D** the records table (`:128–153`); the read-order sentence of the Project section
  (line 5) naming the three archive groupings; the Conventions' "add ONE row to the web lane's records table" phrase
  (`:172`).
- Read: `docs/web-lane.md` as Task 11 wrote it (the anchors); the D8 draft's `REPLACE` texts
  (`D8-tasks-24-28.md:2205–2260`) as the STARTING point, re-read against the tree.

**Interfaces:**
- Consumes: Task 11's anchors verbatim.
- Produces: `CLAUDE.md` under 65,000 bytes with every engine and AI-lane rule byte-identical (Step 4 proves it).

- [ ] **Step 1: Measure before.** `wc -c CLAUDE.md` → the reading (110,258 at `e93ab965`); save a copy to
  `$L/claude-before.md`.
- [ ] **Step 2: Replace the five passages** with pointer texts of at most six lines each, on the D8 draft's `REPLACE`
  shape, each true of the tree at HEAD: E — the lane is finished, two hosts live at E's release behind Access, single
  player in the browser, the operating detail in `docs/web-lane.md`, the lane's fence in one sentence; A — the suites
  and their order rules are `docs/web-lane.md#2-the-suites-and-the-gates`, run them whenever a change reaches what
  the layer compiles; B — the card-art command with its pointer; C — the layer ROUTES, SCOPES and SERIALIZES, the two
  facts that cross into the AI lane (the bot label in `bots.rs`; `PROVISIONAL_DEFAULT_BOT` and `production_bot.json`;
  the release-version rule), the pointer to `#4-the-architecture-and-the-standing-rules`; D — one sentence pointing at
  `#11-the-records-of-record`.
- [ ] **Step 3: The read-order line and the Conventions phrase.** Line 5's read order names the groupings: `guidingpath.md`
  (the live file: mission, current state, roadmap, the decision log's index and its rows from D's close on),
  `guidingpath-archive.md` (the engine era, 2026-07-16 → 2026-08-08), `guidingpath-archive-phase3.md` (the computer
  player's road to V4, 2026-08-09 → 2026-08-30, with the origins), `guidingpath-archive-web.md` (the web lane,
  2026-08-31 → 2026-09-28) — the files Task 13 creates, named here first. The Conventions phrase becomes "add ONE row
  to the web lane's records table in `docs/web-lane.md` §11".
- [ ] **Step 4: Prove nothing else moved.** `diff <(grep -v -n -F -f $L/moved-lines.txt $L/claude-before.md) <(…)` is
  not the instrument; this is: write `$L/claude-diff-check.py` that reads the before and after files, removes the five
  replaced spans and the two edited lines from each, and asserts the remainders are byte-identical; run it; quote its
  output in the report. Then `wc -c CLAUDE.md` (the target under 65,000 bytes) and every `](docs/web-lane.md#…)`
  anchor resolves (Task 11's link check).
- [ ] **Step 5: The per-edit gates** (`guide_sync` reads only the CLI guide pair; nothing here touches it), then commit:
  `git add CLAUDE.md` and `git commit -m "docs(claude): the web lane's five passages become pointers into docs/web-lane.md; the read-order line names the archive groupings"`.

---

### Task 13: The guidingpath archive by grouping, the live file nearly fresh, the relocation record — Opus (R-6.5c2.32)

**Files:**
- Create: `guidingpath-archive-phase3.md`, `guidingpath-archive-web.md`, `docs/plans/<the day>-guidingpath-consolidation.md`,
  `docs/tools/guidingpath-relocate.py` (the byte-check script, committed so the method is reproducible).
- Modify: `guidingpath.md` (232 lines, 412,353 B; the sections at `:9` mission, `:21` current state, `:39` why the
  stall, `:52` evidence base, `:63` roadmap with Phase 3 at `:105–134`, `:140` the decision log — the pointer row
  `:144`, the rows dated 2026-08-09 → 2026-08-30 at `:145–204`, the rows dated 2026-08-31 onward at `:205–232`, two of
  them out of date order at `:228–229` and kept in their stored order).
- Read: `docs/plans/2026-08-19-docs-consolidation.md` and `docs/plans/2026-09-23-claude-md-consolidation.md` (the two
  relocation records' shape and byte-check method), `guidingpath-archive.md` (untouched; its header's wording for the
  new archives' headers).

**Interfaces:**
- Consumes: Task 12's file names for the groupings (verbatim).
- Produces: the live `guidingpath.md` with a decision-log INDEX table; the row Task 16 appends goes after D's.

- [ ] **Step 1: The script, red first.** `docs/tools/guidingpath-relocate.py` takes the live file and a relocation map
  (which line ranges go to which new file, in order) and (a) writes each new file with a header naming its range and
  its reason, the moved lines VERBATIM; (b) writes the live file with the ranges removed and the replacements inserted;
  (c) CHECKS, byte for byte, that every moved line appears exactly once across the new files and the live file's
  remainder, and that no other line of the live file changed; it exits 1 naming the first line that fails. A dry run
  against the tree with an empty map exits 0 changing nothing; a run with a map naming a range that overlaps another
  exits 1 — both are the script's own `--self-test`.
- [ ] **Step 2: The groupings.** `guidingpath-archive-phase3.md` opens with the origins — the sections "Why the Dec-2025
  stall happened" and "Evidence base (July 2026 research)" moved whole, then the Phase 3 roadmap subsection's landed
  notes (`:105–134`), then the decision-log rows dated 2026-08-09 through 2026-08-30 — with a header: the computer
  player's road to V4, superseded for the current state by the V4 close (the owner's words from spec §10.9 R-6.5c2.32,
  quoted). `guidingpath-archive-web.md` holds the rows dated 2026-08-31 through 2026-09-28 (the personal-data row
  among them), with a header naming the lane's records table in `docs/web-lane.md` §11 as the record of record. Both
  archives are append-closed, like the first, and say so.
- [ ] **Step 3: The live file.** The mission (`:9–20`) kept with ONE dated line added on why the web lane exists, in the
  owner's words ("we pivoted to the web lane to make continued vetting and save game gathering easier and testing of
  future lanes easier than the CLI while still allowing automated play to be straightforward in continued
  development", spec §10.9 R-6.5c2.32, 2026-09-28); a one-screen current state written fresh at E's close (the engine
  at rev 7, V4 shipped, the AI lane's re-entry pointer and its two HOLDs, the web lane finished at E with its records
  in `docs/web-lane.md`); the roadmap as ONE table — one row per phase and sub-phase: status, date, gate, record of
  record — the Phase 3 subsection's prose replaced by its rows; the decision log opening with an INDEX table (date
  range → file → what it holds: the three archives and this file) and holding the rows from 2026-09-29 (D's) on. The
  archived rows remain authoritative where cited, and the index says so.
- [ ] **Step 4: Run the script** with the map; quote its check line (`N lines moved, byte-identical; M lines kept`);
  `wc -c` before and after for all four files; every `[[…]]` or `](…)` link in the four files resolves.
- [ ] **Step 5: The relocation record** `docs/plans/<the day>-guidingpath-consolidation.md` on the 2026-09-23 record's
  shape: §1 what moved where (the map, by line range and date range), §2 what was rewritten in the live file (the
  new current state and the roadmap table, with the old text's location in the archive), §3 the byte-check's output,
  §4 the reviews (filled by Task 9's shape at the close).
- [ ] **Step 6: The per-edit gates**, then commit the five files with `git commit -m "docs(guidingpath): the archive by
  grouping - the engine era, the computer player's road to V4, the web lane - and a live file that starts nearly fresh
  (R-6.5c2.32)"`.

---

### Task 14: The board — the history as dated pointers, the stale lines, Track 3 at E — Sonnet 5.5 (straw poll row 52; R-6.5c2.32; dossier Part 3)

**Files:**
- Modify: `docs/handover.md` (551 lines): the "Updated" line (`:9`) and every "previous update" paragraph (`:11–100`),
  "Where things stand" (`:102–114`), Track 3 (`:194–524`), the stale sentences row 52 names — found by CONTENT, since
  the straw poll's line numbers drifted in D (the dossier's Part 3 quotes each): the web bullet's version sentence
  (`:110`), the roster figure, any "stays at 132" sentence (grep `132`), "No API route serves the raw record"
  (`:501–502`), the M6.2 ruling #13 "keep or strike" item, the `deploy/RUNBOOK.md` "§6 the Worker" pointer, the
  doubled phones item at `:204`, and "Entry artifacts by lane" (`:544–551`, which names D's plan).

**Interfaces:**
- Consumes: Task 11's `docs/web-lane.md` and Task 13's live `guidingpath.md` (the pointers' targets).
- Produces: a board Task 16 updates by ONE "Updated" line at the close.

- [ ] **Step 1: The history as pointers.** Replace the "Updated" block and every "previous update" paragraph with the D8
  draft's dated list (`D8-tasks-24-28.md:2330–2360`), extended with 2026-09-28 (C's close), 2026-09-29 (D's close and E's
  planning session, spec §10.11) and E's own close line (left as `<the day>` for Task 16), each a one-line pointer to its
  record.
- [ ] **Step 2: The stale sentences**, each found by content: the web bullet (`:110`) → one sentence at E's release;
  the roster figure → "the fifteen gates last ran at E's close, <Task 16's figure> on the Mac beside D's 446 s"; any
  "stays at 132" → 133 since C (or struck if D already did); "No API route serves the raw record" (`:501–502`) →
  `GET …/record` since M6.5a and the pull recipe in `docs/web-lane.md` §10; the M6.2 ruling #13 item → approved at
  M6.3, struck from the outstanding list; the `deploy/RUNBOOK.md` "§6 the Worker" pointer → `worker/RUNBOOK.md`; the
  doubled phones item at `:204` → one line pointing at E's phones readings; "Entry artifacts by lane" → E's plan and
  close.
- [ ] **Step 3: Track 3 at E.** The block becomes: the state (E's code on `web-room-m6.5c2e`, PR #43, both hosts at E's
  release, the door reverted), what the lane is and how it runs (a pointer to `docs/web-lane.md`), and "What wants
  YOU" — the merge #31 → #43 and the `v0.6.0` tag on your word (R-6.5c2.37), the handset's optional ten minutes, the
  blurb's words if you want them changed.
- [ ] **Step 4: "Where things stand"'s web bullet** → three sentences: live on two hosts behind Access at E's release,
  single player in the browser, the lane finished and its record `docs/web-lane.md` §11; the gates bullet's counts
  pointer names `docs/web-lane.md` §2 for the web.
- [ ] **Step 5:** every link resolves (Task 11's check over `docs/handover.md`); the address sweep prints nothing
  (`git grep -n -i -F "$(git config user.email | cut -d@ -f1)" -- docs/handover.md`); commit `git add docs/handover.md`
  and `git commit -m "docs(board): the history as dated pointers, the stale lines at 0.6.0, Track 3 at cycle E"`.

---

### Task 15: The README in R-6.5c2.45's shape, the runbook's two lines, the seven captions — Sonnet 5.5 (straw poll rows 45, 46, 53; R-6.5c2.32, .45; dossier Part 3)

**Files:**
- Modify: `README.md` (725 lines; the sections at `:17` See it running, `:199` The browser client, `:294–438` Hosting,
  `:439` Status, `:587` Workspace crates, `:601` Download and run, `:633` Build from source, `:687` Data provenance,
  `:705` Where truth lives; the false lines the straw poll's §C names, verified by the dossier at HEAD: `:9–10`,
  `:213–215`, `:331–333`, `:340–341`, `:388`, `:399–401`, `:415–417`, `:428–437`); `deploy/RUNBOOK.md:531` and `:563`
  (the two "The live build is …" lines in §3.6, a record section); `worker/README.md` §1.4 (`:223–278`: the sentence
  that the pictures "are recaptured in cycle D" is false; TEN captions) and its `:1735`, `:1831–1833`, `:1845`.
  `deploy/README.md:209–213` is already true (fixed in D) — nothing there.
- Read: the ten pictures Task 10.G wrote (look at each before writing its caption); `docs/web-lane.md` §12–§14
  (the tables the README links to, never copies).

**Interfaces:**
- Consumes: Task 11's anchors `#12-every-variable`, `#13-the-api-map`, `#14-getting-started`; Task 10's screenshots.
- Produces: the README Task 16 leaves alone but for the release line at the tag.

- [ ] **Step 1: Section links at the top.** After the opening paragraph and the release note, a contents list: one link
  per `##` heading (the slugs GitHub derives), so a reader jumps to the CLI, the browser client, the hosting, the
  status, the crates, the downloads, the build, the provenance and where truth lives.
- [ ] **Step 2: The web section restructured** (`:199–438` becomes): **Getting started** — the three paths in a short
  numbered list (the hosted site behind Access, a self-host in Docker with the guide's link, the bot in any browser
  with no server), each two sentences; **What both deployments run** — the defaults table copied from
  `docs/web-lane.md` §14 by LINK, with the six settings a player notices named in one line each (the door, the archive,
  the transport, the TTLs, the chat limits, the three build values); **Every variable and the API** — two sentences
  and the two links (`docs/web-lane.md#12-every-variable`, `#13-the-api-map`); **See it** — the door, the lobby and
  the watcher's game screen from `worker/docs/img/` (three of the seven, with captions true of the pictures); **The
  three hosts** — the present bullets for the self-host guide, the Worker and the browser, kept and made true at
  0.6.0 (`:331–333`'s "the cleanup cycle after it, M6.5c2, starts from its handover" → the cycle closed in E, its
  records in the lane document; `:340–341`'s "operating detail … is in CLAUDE.md" → `docs/web-lane.md`); **the room
  cycles' history** (`:356–437`) reduced to the closes list, with C, D and E added (`:428–437`) and the false
  sentences (`:388` the first screen is the door; `:399–401` "Being played" and "You are spectating"; `:415–417` the
  name lives in the corner's "Playing as") corrected where a sentence survives.
- [ ] **Step 3: The release line** (`:9–10`): "current release **v0.5.0**" → the release this tree builds is v0.6.0, cut
  from the M6.5c2 close; the download line points at the Releases page for the newest build (Task 16.H's tag makes
  it true; the sentence is written so it is true before and after).
- [ ] **Step 4: The other three files.** `deploy/RUNBOOK.md:531` and `:563`: each "The live build is …" sentence gains a
  dated note in place — "(a record of 2026-09-05; the live build is §3.19's)" — nothing else in the section moves.
  `worker/README.md` §1.4: the false "recaptured in cycle D" sentence corrected to E, and each of the TEN captions
  rewritten to what its new picture shows (the door with the blurb and the link; the phone door; the lobby's "Playing
  as" corner, "Being played", the Finished fold; the open table; the join; the deal; the player's turn; the watcher's
  "You are spectating"; the phone game; the marker), and `:1735` ("kept for good") → the 30-day rule, `:1831–1833` →
  the lane document, `:1845` → `docs/web-lane.md`.
- [ ] **Step 5:** every link in the four files resolves; the address sweep prints nothing; the per-edit gates; commit
  `git add README.md deploy/RUNBOOK.md worker/README.md worker/docs/img/` and `git commit -m "docs(readme): section links, getting started, the defaults, every variable and the API map by link, the ten pictures recaptured (R-6.5c2.45)"`.

---

### Task 16: The close, PR #43, and the wrap — ORCHESTRATOR (spec §4 row E's "ends with"; R-6.5c2.37; R-E16, R-E17)

- [ ] **A. The ONE recorded sequence at the FINAL reviewed head** (R-6.5c2.13; R-E14, R-E17): Tasks 11–15 reviewed and
  landed; D's `close-sequence.sh` copied into `$L` with `L=` set, no row added; ONE uninterrupted run under `caffeinate
  -i`, machine-tagged, polled with `tail -n 3 "$L/sequence.tsv"`; every `rc` 0; the fifteen gates' walls summed beside
  D's 446 s; a red row re-run ALONE before it is believed; each count set against Task 9.D's readings and the tasks'.
  The e2e inside it leaves the DEBUG module in `web/dist` — no deploy follows, by design.
- [ ] **B. The close document** `docs/plans/<the day>-web-frontend-m6.5c2e-close.md` on D's shape: the header; the
  headline and what is LIVE; §1 by task with §1.x the fences; §2 the sequence; §3 the deployments, the update reading on
  both hosts, the live tiers, the screenshots, the phones' answers to the three questions, the rate-limit value and its
  burst, the door window's three states with their readings and the revert's after-check, the quota; §4 the reviews and
  the battery; §5 every ruling R-E1–R-E17 and every ledger `Ruling:` with its cost; §6 what the lane leaves (the
  by-design list; the triage page's E rows done); **§7 what this cycle does NOT claim** — at least: the handsets; the
  public door as an end state; a public load; the rate-limit threshold as tuned; the 15-minute window unless the timer
  arm ran; the NAS's waiting tables; CI dark until the merge; the exit spec; §8 the wrap. **Then** the spec's `§10.12
  Corrections recorded in execution — cycle E`; `guidingpath.md`'s ONE appended row (`M6.5c2 CYCLE E "THE FINISH"
  EXECUTED, CLOSED AND LIVE`, naming the close, the final head, both deployments, R-6.5c2.37–.45 and R-E1–R-E17 by
  number, the fences); `CLAUDE.md`'s counts in place at the final head; the board's "Updated" line and Track 3's state
  line; `docs/web-lane.md` §2's counts and §11's E row; the `Cargo.toml` figures from 10.B. Commit, documents only.
- [ ] **C. The triage page** (the owner's view, never the record): one `ArtifactData` batch over `progress` — every E row
  `done` with its task and commits (21, 43–46, 52, 53, 58–64, 66–68, 71–73, 76, 77, 79, 80, 81, 84, 90, 106, 108, 114,
  119; 132 `in progress` until H), read back once.
- [ ] **D. The memory**: `yokohama-web-lane-status` rewritten to the close (E DONE; what is live; the wrap's state), the
  index line; `project-web-lane-wrap-then-ai` gains the E update.
- [ ] **E. The push and PR #43** (pre-authorized): `git push -u origin web-room-m6.5c2e`; `gh pr create --base
  web-room-m6.5c2d --head web-room-m6.5c2e` with the cycle in one screen and the attribution line. No checks run
  (R-6.5c2.31 as amended); `gh workflow list --all` in the ledger as the reading.
- [ ] **F. The AI lane's re-entry**: the board's Track 1 pointer and `docs/plans/2026-09-11-ai-lane-reentry-handover.md`
  are the entry; the close's §8 says so in three lines. No new handover document.
- [ ] **G. STOP and ask the owner**: the merge and the tag now, or as a follow-on session (R-6.5c2.37; R-E16).
- [ ] **H. THE WRAP, on the owner's word.** The merge #31 → #43, bottom-up, one at a time, each read before the next:
  for #31, `gh pr merge 31 --merge` (a merge commit on master); then for each of #32 … #43 in order, `gh pr edit <n>
  --base master`, `gh pr view <n> --json commits` to see ONLY that cycle's commits, then `gh pr merge <n> --merge`; the
  workflows stay disabled and nothing is dispatched; after #43, `git checkout master && git pull` and the fences read
  against `master`. Then the tag: `git tag -a v0.6.0 -m "Yokohama CLI v0.6.0 — the web lane finished: two hosts, the
  cleanup, the update path"`, `git push origin v0.6.0`; `release.yml` runs on the tag — `gh run list --workflow
  release.yml` read to green and the release's three assets listed. Then `gh workflow enable test.yml` and
  `gh workflow enable docker.yml` (October). Then the memory rewritten (D above, amended: merged and tagged), the
  board's "Updated" line, `README.md:9–10`'s release line if Task 15 left a conditional, committed on master as one
  documents commit. The refocus on the computer player is the next session's, from the re-entry handover.

---

## Appendix — the ledger

`.superpowers/sdd/2026-09-29-web-frontend-m6.5c2e-plan/` (gitignored): `progress.md` (the SDD identity line, the base,
Step 0's baseline, R-E1–R-E17, one line per task, one `Ruling:` per deviation); `task-<n>-brief.md`, `-report.md`,
`-review.md`; `task-<n>-e2e.txt`; `task-4a-look/`, `task-4b-look/` (the before and after pictures, four widths);
`task-7-tally/` (the tally files and the report's output); `final-review.md`, `fix-wave-*.md`, `battery.md`,
`code-head-readings.md`; `live-day1.md`, `door-window.md`, `phones-iphone.md`, `phones-ipad.md`, `phones-android.md`
with their picture directories; `nas-build.sh`, `nas-swap.sh`, `close-sequence.sh`, `sequence.tsv`, `sequence/NN.log`;
`pr-body.md`. D's ledger (`…-m6.5c2d-plan/`) holds the scripts' originals and the pictures polish is judged from.
