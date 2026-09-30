# M6.5c2 cycle F — the clean finish: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan
> task-by-task. The execution shape is §"Execution shape" below: every code and document task subagent-driven, the
> implementer's tier marked in each task's title (**Sonnet 5.5** for the implementation-focused tasks whose brief
> carries the judgment already made, **Opus** where judgment is needed), every task reviewer Opus, Fable for the
> whole-branch review; every account step — a deploy, a live run, an Access change, a zone rule — the ORCHESTRATOR's
> alone, at discretion (R-F7). Steps use checkbox (`- [ ]`) syntax. **Read the spec first**
> (`docs/plans/2026-09-30-web-frontend-m6.5c2f-spec.md`: §1 the rulings R-F1–R-F13, §2 the mechanism — §2.3, §2.5
> and §2.6 as corrected after the adversarial review, §9 — §5 the fences, §6 what F does not do), then this plan's
> §"Names shared across tasks" and §"Couplings between tasks" — an implementer sees only its own task, and those two
> sections are how it learns what its neighbours produce. Every `file:line` here was read at `a63b48e1` and corrected
> by the review; **find by content once an earlier task has moved a file.**

**Goal:** The web lane closed on the owner's goal of a deployment that will one day run on Cloudflare with no Access
barrier, on the Workers Free plan, with bots and scanners kept off the Worker: the door's no-click update path and a
release notice that ask the server nothing; the resume rule so a new bot version cuts off no game the build can still
play; the self-host's table expiry; the door unmetered as a fence with a static 404 page, a guard, a probe and a
ledger of every metered action; ONE edge rule that keeps every request outside the release's static names off the
Worker, with Turnstile's pre-clearance as the way a person passes it, behind a switch, proven in a public window and
reverted; the web presence; E's minors; the documents; the close; then, on the owner's word, the merge #31 → #43 →
F's PR and `v0.6.0`.

**Architecture:** Three parts in order. **Part 1, the code** (Tasks 1–9), lands ONE release: the update path first
(1), since every later browser reading depends on a release that updates itself; the three build values' checker and
the release notice (2); the resume rule (3) and the sweep (4), each self-contained; the 404 page, the build-values
stamp, the guard's three new checks and the protected-path change (5a), then the presence files and the page's
metadata (5b); Turnstile's client half (6) then the edge rule's script, the deploy declaration and the smoke (7),
their names fixed in the names table; the instruments and the ledger (8), which read the client and the router as
1–7 leave them; E's minors (9) last; then the review, one fix wave, the battery and the ONE recorded sequence (10).
**Part 2, the live readings** (Task 11, the orchestrator's): both deploys, the update readings, the NAS sweep, then
the public window LAST because it takes Access and the dev door down. **Part 3, the documents** (Tasks 12–14),
written from Part 2's readings, then the close and the wrap (15).

**Tech Stack:** Rust 1.97.1 (`crates/yokohama-web`, `crates/yokohama-worker`, `crates/yokohama-wasm`; tokio for the
sweep's interval, behind the `server` feature); TypeScript in `web/` (React 19, Vite, vitest over jsdom, Playwright over
Chromium) and `worker/` (the shell; vitest node cells; the POOL inside workerd); the owner's Cloudflare account through
the Cloudflare MCP (Access applications, WAF custom rules, the Observability query API, GraphQL) and `wrangler` 4.136.3
(deploys, `turnstile widget get`); the NAS through Portainer's Docker proxy; Cloudflare Turnstile
(`challenges.cloudflare.com/turnstile/v0/api.js`, pre-clearance; no siteverify).

**Spec:** `docs/plans/2026-09-30-web-frontend-m6.5c2f-spec.md` as corrected after the review (its §9), whose §1
rulings are restated, with their amendments, in the M6.5c2 spec's §10.13. **How this plan was built:** by the
orchestrator alone from the spec and the tree, under D's stop rule — a task past ~250 lines is two tasks, a plan
past what one agent reads whole is split before it is finished (R-F11) — then reviewed adversarially by Opus against
the spec, the plan and the approach (32 findings, three confirmed by probes) and rewritten in one fix wave
(§"Review record"), before the owner's final pass.

---

## Global Constraints

- **Branch `web-room-m6.5c2f`**, stacked on `web-room-m6.5c2e`; **the base for every code diff is `15de8117`** (E's
  branch head). The branch's first commits are the spec, this plan and their fix wave. Stage with explicit paths,
  never `git add -A`; `git status` is clean after every commit.
- **The push, F's PR, the deploys, the live tiers, the public window and its Access and zone changes are the
  orchestrator's at discretion** (R-F7). **The merge and the tag are NOT**: Task 15.G asks the owner at the wrap.
- **Fences** (spec §5). At every commit this prints nothing:
  `git diff --stat 15de8117 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli`.
  `SEMANTICS_REV` stays **7**. No bot behaviour moves and NO CLI file moves: Task 3 decides which recorded bot the
  layer seats by calling the CLI's public resume, never by editing it. **The door unmetered is a fence of its own**:
  no task adds a request the door with nothing owed, the local game or an unknown path makes to the Worker; Task 8's
  cell and Task 11's readings are its proof.
- **The corpus moves in ONE named task:** Task 3 regenerates every foreign sample for the new refusal sentence and
  nothing else (the samples are already built from `v2-padded`, the V5 shape). `UPDATE_WIRE_FIXTURES=1 cargo test -p
  yokohama-web --test wire_fixtures`, the diff READ and named in the report; `git diff --stat 15de8117 -- web/src/wire
  web/src/wire-script` at the close lists only those files. **The corpus's seed pin, password pins and watcher pin stay
  green at every commit.**
- **The per-edit gates, after EVERY task** (`CLAUDE.md`'s Commands; the baseline is Task 1 Step 0's): `cargo test -p
  yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`, the eight `identity::` cells, the `wasm32` check, `cargo
  clippy --workspace --all-targets -- -D warnings`, `cargo doc -p yokohama-web --no-deps` and `-p yokohama-worker
  --no-deps` at 0 warnings; in `web/`: `npm run build:wasm` then `npm run typecheck`, `npm test`, `npm run test:wasm`;
  in `worker/`: `npm run build:module:debug` FIRST, then `npm run typecheck`, `npm run test:scripts`, `npm test` (the
  pool). **The variables drift cell** (`worker/scripts/lane-variables.test.mjs`, inside `test:scripts`) scans every
  code file for `YOKOHAMA_*` and `VITE_*` names and fails on a name without a row in `docs/web-lane.md` §12: **every
  task that adds a name adds its §12 row in the same commit** (Tasks 2, 5b, 7) and lists the lane document among its
  files; Task 12 completes the wording. **`npm run e2e` is owed by every task that changes what a browser does or
  shows** — Tasks 1, 2, 3, 5a, 5b, 6, 8 and 9 — run ONE at a time, in the foreground or polled, never under a Monitor;
  a red cell re-runs ALONE before it is believed.
- **Every invisible non-ASCII code point in code is a `\u` escape.** No document spells the owner's address: readings
  say "the owner's address". **No secret value is printed, copied or committed**: the credential in `.dev.vars`,
  `DEV_DOOR_CLIENT_ID`, `YOKOHAMA_WEB_ADMINS`, the Portainer token, any cookie; the `TURNSTILE_SECRET` line in the
  repository root's `.dev.vars` is UNUSED by F (no siteverify is built) and no task reads it. **The Turnstile site key
  is public but stays out of every document**: a build passes it through the environment; the ledger says "the site
  key".
- **Sonnet 5.5 implements a task whose brief carries the judgment already made** (Tasks 2, 5b, 9, 14); a Sonnet
  implementer that meets a design choice STOPS and asks (NEEDS_CONTEXT to the orchestrator). **Opus implements Tasks
  1, 3, 4, 5a, 6, 7, 8, 12, 13.** Every reviewer is Opus, mutants on an EXPORT (`git archive`), never in the
  implementer's tree. Fable reviews the whole branch (Task 10).
- **Reports to files**, never the notification: `$L/task-<n>-report.md`, three lines in the reply. **A fix is a NEW
  commit** — once a SHA is REPORTED, every change is a new commit. **Nothing runs in an implementer's tree** while it
  works, not even read-only git. **No Monitor, no event waits**: a long run is polled by reading its output file; the
  orchestrator checks the ledger within ~15 minutes of any long run. **No `rm -rf`, `rm -r` or bulk `rm -f` in any
  dispatch**: exports and screenshot trees stay in place, named in the report.
- **The wire corpus's rule for the client tasks that render** (1, 2, 6): every payload a jsdom cell renders comes from
  `web/src/wire/`; nothing is hand-authored, and the fake Worker answers from the corpus. A FEED fixture (Task 2) and
  a fake widget (Task 6) are not wire payloads: they are the third party's shapes, held beside their cells.

---

## Review Focus

The spec implies these five inputs, and no cell would meet them unless someone put it there. Each line says what a
person would expect; its test lives in the owning task and is marked **(Review Focus N)** there.

1. **A player with the door open in one tab and a game in another tab of the SAME browser when a release lands.**
   Expected: the door tab reloads itself once; the game tab, handed to the new worker by that apply, keeps playing on
   its old bundle out of the retained generation and shows the line, whose click reloads it; nothing in it reloads
   by itself. (Task 1: the two-hooks-over-one-registration cell; Task 11.D's reading with a second TAB of the pinned
   context.)
2. **A V4 game left mid-play the day a V5 bot ships, before the V5 ship grandfathers V4 in the CLI.** Expected: the
   web refuses it EXACTLY as the CLI would, naming the CLI's reason — and the same code plays it on the day the CLI
   admits V4, with no web change. (Task 3: `v2-padded` refuses with `eligible`'s own sentence; a genuine
   `lookahead-v1` record opens and moves; the pin on the CLI's sentence shapes.)
3. **A self-host with a TTL set whose archive directory is unset, and one whose first sweep finds forty idle tables.**
   Expected: the first refuses to start naming both variables; the second logs forty lines before ending any, and
   every ended game with moves is in the archive. (Task 4: the start refusal cell and the listing-before-ending cell.)
4. **A visitor who arrives by a table link under the no-Access door with no clearance, and a WATCHER whose clearance
   lapses while only the stream is open.** Expected: the widget once, then the table; and for the watcher the widget
   once at the stream's failure, then the stream reconnects. (Task 6: the fetch handler's retry cell at each site and
   the stream's one-`call` cell.)
5. **A scanner asking the Worker for `/.env` with no browser headers, and a browser navigating to `/wp-login.php`.**
   Expected: under the no-Access door the scanner is challenged at the edge and invokes nothing; the browser gets the
   404 page; a controlled browser's navigation still gets the shell. (Task 5a: the axum 404 cell and the precache
   exclusion; Task 7: the rule's expression cell; Task 8's fence cell; Task 11.G's probe in three shapes.)

---

## Execution shape (the owner's standing answers, and this plan's rulings)

- **Mode: SUBAGENT-DRIVEN, EVERY TASK.** Implementers as the Global Constraints say. **Every task reviewer Opus.**
  Fable: the whole-branch review (Task 10). A Sonnet brief carries the judgment already made: what to change, what
  proves it, what not to touch.
- Tests are written first and RUN RED; the red run is quoted in the commit body and the ledger. Every commit passes
  every per-edit gate.
- **The orchestrator edits no code.** Its steps: Task 1 Step 0's baseline; Task 10 and Task 11 whole; the pictures
  the notice and the line are judged from; Task 15 whole.
- **Rulings at planning (2026-09-30, the orchestrator, under the F spec; R-FP12–R-FP15 from the fix wave):**
  - **R-FP1 — the numbering:** nine code tasks (1, 2, 3, 4, 5a, 5b, 6, 7, 8, 9); 10 the review and the sequence; 11 the
    live readings; 12–14 the document tasks; 15 the close and the wrap. The base for every diff is `15de8117`.
  - **R-FP2 — the build values and the switch keep the spec's names verbatim**: `YOKOHAMA_RELEASE_FEED`,
    `YOKOHAMA_TURNSTILE_SITEKEY` (both checked and defined by Task 2 as `__YOKOHAMA_RELEASE_FEED__`,
    `__YOKOHAMA_TURNSTILE_SITEKEY__`, beside `__YOKOHAMA_VERSION__`), `YOKOHAMA_SITE_URL` (Task 5b,
    `__YOKOHAMA_SITE_URL__`); `YOKOHAMA_TURNSTILE` (`on` | `off`, a deploy DECLARATION in the configuration's `vars`,
    read by the guard and the smoke, declared in `Env` and `ROWS` as a string the shell never reads). No secret.
  - **R-FP3 — the build-values stamp**: the client build writes `web/dist/build-values.json`, a JSON object of SIX
    BOOLEANS naming which build values were set (`project_url`, `blurb`, `notice`, `release_feed`, `site_url`,
    `turnstile_sitekey`), never a value; it is not precached, it lands in the Worker's assets tree with the rest of
    `dist`, and the guard's check 12 reads it. Task 5a writes it with all six keys, since Task 2 has defined the site
    key's value by then; `site_url` reads `false` until Task 5b adds the value.
  - **R-FP4 — one build per door state** (spec §2.6): production (Access on) is built WITHOUT the site key and deployed
    with `YOKOHAMA_TURNSTILE: "off"`; the open configuration deploys a build WITH the key and `"on"`; the guard refuses
    the other two combinations. The NAS build carries no key.
  - **R-FP5 — the release feed's three states** are spelled by the checker: unset derives from a GitHub project link
    (`https://github.com/<owner>/<repo>` with an optional trailing slash → `https://api.github.com/repos/<owner>/<repo>/releases/latest`),
    a URL is that feed, the literal `off` is none; any other project link with the variable unset is none.
  - **R-FP6 — the sweep's activity read** is a `Store` method `last_written(&self, id: &str) -> Result<Option<u64>, String>`
    (unix seconds, the latest of the three documents' modification times) **with a default body `Ok(None)`**,
    "activity unknown: never a candidate", so the crate's twenty test doubles keep compiling; `FsStore` and `MemStore`
    override it, `MemStore` through `stamp_for_test(id, secs)`. `Ttls::from_lookup` is pure; `main.rs` passes
    `std::env::var`; the gated `run` sits in one closed column-0 region at the end of `sweep.rs`.
  - **R-FP7 — no siteverify, no secret, `/enter` unchanged on both hosts.** The edge rule is the fence: `worker/scripts/edge-rule.mjs`
    prints the rule's JSON from the assets tree, a drift cell pins its allow-list to the tree's top-level names, and
    `--verify` compares a rule read from the zone; `worker/scripts/smoke-release.mjs` gains the `on` reading (a
    cookie-less `GET /api/health` must read 403 `cf-mitigated: challenge`).
  - **R-FP8 — the edge challenge is detected by ONE predicate at every site that talks to the server**: `api.ts`'s
    `call`, the record download (`api.ts:820`), the stream reader (`sse.ts:117`), and the WebSocket's failure path
    (`ws.ts:107`); the retry is one; the widget is one overlay behind a module-level seam (`setChallengeHandler`).
  - **R-FP9 — the lines' places**: the update line in the lobby's `lobby-actions` block and the game screen's `banner`
    header (`App.tsx:2118`), and on the door where the offer stands; the release notice under the door's blurb and in
    the lobby's `lobby-actions` block beside the project link (the signed-in block, F-12). Each judged from a
    before-and-after picture; the orchestrator may move either with a `Ruling:` line.
  - **R-FP10 — the window runs on ONE UTC day after the deploys**, the quota guard before and after; the rule goes ON
    before the hostname leaves Access, and Access comes BACK before the rule goes off; the probe burst is 200
    requests in three shapes, paced at two a second.
  - **R-FP11 — the sequence** is E's `close-sequence.sh` with `L=` set, one row added for `npm run e2e`'s per-spec
    walls (Task 9's minor), and runs ONCE at the FINAL reviewed head (Task 15.A), as E's R-E17 ruled.
  - **R-FP12 — the resume predicate calls the CLI's resume**: `bots::runs_here(record) -> Vec<(u8, BotDescriptor, Runs)>`
    over ONE `BotSeats::from_provenance(record.run.as_ref())`; `Runs::Yes` when `contestant_at(seat)` is `Some` and
    no warning names `seat {i}`; `Runs::No(reason)` otherwise, the reason being the CLI's warning with its resume
    tail cut (" — it resumes as a HUMAN seat, so you are now playing it" / " — the seat still plays, under the
    CURRENT build") and its "warning: seat {i} " prefix cut. A cell holds the three CLI sentence shapes verbatim.
  - **R-FP13 — the guard's checks 10, 11 and 12 live in `deployGuard`** beside checks 2 and 9, over the judged
    configuration's own assets tree; check 10 walks the top level, every `env`, and EVERY `exports` name.
  - **R-FP14 — the two-tab rule**: a tab that hears a `controllerchange` it did not ask for sets `staleTab` and draws
    the same line with a plain `location.reload()` behind its click; the door's auto-apply waits while a restore or a
    send is in flight (`setApplyBusy(true|false)` from the backup controls and the outbox's button).
  - **R-FP15 — Task 5 is two tasks**: 5a the 404 page, the stamp, the guard's checks and the protected list (Opus); 5b
    the presence files and the metadata with `YOKOHAMA_SITE_URL` (Sonnet 5.5).

---

## Names shared across tasks

An implementer sees only its own task. **These names are the contract between tasks and are used verbatim.** A task
that finds the code forces a different shape STOPS and asks the ORCHESTRATOR, who amends this table with a `Ruling:`
line.

### What the tree holds at the base, consumed as it IS (read off the tree, never off this table)

| name | where | tasks |
|---|---|---|
| `useReadiness`, `applyUpdate`, `readinessOf`, `SwHost`, `RegistrationLike`, `ReadinessStatus` | `web/src/swRegister.ts:29–380` | 1 |
| `Readiness` (the door's release line, its update branch at :269–271) | `web/src/OnThisDevice.tsx:247–280`; passed as `readiness={readiness}` from `App.tsx:1602` | 1, 2 |
| `useRoute`, `channelOf`, `parseRoute`, `Route` (`{ kind: "door" }` …) | `web/src/route.ts:21–140` | 1, 2 |
| the door's friends card (`<a className="primary-link" href="/enter">Go to the lobby</a>`), `multiplayerOffered()`; the other `/enter` links (`OnThisDevice.tsx:477` `Remedy`, `Door.tsx:113, :137`, `Refusal.tsx:50`) — all stay plain navigations | `web/src/FrontDoor.tsx:41–47, 142–156` | 6 |
| `call` (:636), the record download's own `fetch` (:820), `settle`, `isAccessChallenge` (:262), `ACCESS_CHALLENGE`, `onUnauthorized`, `ensureArchiveFact` (:563); the stream reader `sse.ts:117`; the socket `ws.ts:107` | `web/src/api.ts`, `web/src/sse.ts`, `web/src/ws.ts` | 6 |
| `drain` (:452), `attempt`, `noteGameOver` (:429–438), `startOutbox` (:546), `useOutbox` (:571); `PendingReason` = `"offline" \| "signed-out" \| "unknown" \| "refused"` | `web/src/outbox.ts`; `web/src/local/db.ts:87` | 6 |
| `BUILD_VALUES`, `checkProjectUrl`, `checkText`; the three `define`s; the three `declare const`s | `web/scripts/build-values.mjs:19–64`; `web/vite.config.ts:38–45`; `web/src/vite-env.d.ts`; `web/src/buildValues.ts`; `web/scripts/e2e-build.mjs:14–21` | 2, 5b |
| the precache walk (`distFiles`, excluding `sw.js` and the shell copy) | `web/scripts/precache.mjs:95–108` | 5a |
| `deployGuard` (:441–490, the tree known), `configFailures` (:399–440, no tree), `assetsBlock` (:133–160), `SECRETS`/`secretsInVars` (:215–240), `doorRules` (:297–330), `siblingDrift` + the open file's allowed differences (:275–295), `staleAssets` (:371–395); `worker/scripts/smoke-release.mjs`; `worker/scripts/assemble-assets.mjs` (copies `web/dist` WHOLE and refuses a path two sources provide: no edit) | `worker/scripts/` and their `.test.mjs` cells | 1, 5a, 7 |
| the assets blocks (`not_found_handling: "none"`, `run_worker_first: ["/api/*", "/enter"]`) | `worker/wrangler.jsonc:37–43`; `wrangler.test.jsonc:21–26`; `wrangler.open.test.jsonc:23–28`; `wrangler.open.jsonc:33–38`; `wrangler.gate.jsonc:38–43` | 5a |
| `Env` (the interface opens at :23), `shellConfig`, `ShellConfig`, `ROWS` (:37–51), `strings(env)`, `onOff` (:189), `moduleConfig` (:287) | `worker/src/env.ts`; `worker/src/config.ts` | 7 |
| `route()`, `Target`, `handle()` (the `enter` 302 at `index.ts:114`, above `shellConfig`; `bounded(req.body)` at :125) — **unchanged by F** | `worker/src/route.ts:9–80`; `worker/src/index.ts:96–140` | 7 (read only) |
| `accessPaths`, `ROUTER_FILES`, `segments.delete("health")` (:55) | `worker/scripts/access-paths.mjs`; its cell (the nine destinations by hand) | 5a |
| `bots::foreign` (:243), `bots::table_bot` (:224), `bots::production` (:70), `bots::label` (:82), `bots::cannot_continue` (:316), `bots::distinguishing_labels` (:293), `BotDescriptor`, `TableBot { label, runs_here }` | `crates/yokohama-web/src/bots.rs` | 3 |
| the foreign-bot rule's ONE site (:691–715); the fixtures `foreign_pair(...)` (:1337–1420, the builder at :3354–3390 — a RENAMED seat keeping its version and knobs) | `crates/yokohama-web/src/games.rs` | 3 |
| `explain::foreign_why`, `FOREIGN_BOT_KIND`, `Why::foreign` | `crates/yokohama-web/src/explain.rs:443–475` | 3 |
| `BotSeats::from_provenance(run: Option<&RunProvenance>) -> (BotSeats, Vec<String>)` (:255), `BotSeats::contestant_at(seat) -> Option<Contestant>` (:169), `is_bot` (:159); `eligible` (`pub(crate)`, :377–403: ONE named identity admitted, by equality with `Contestant::PRODUCTION`); `identity_drift_warnings` (:482–507: the version and EVERY param, the weights digest included); the three warning shapes (:275–302, :486–503) | `crates/yokohama-cli/src/bot.rs` (FENCED: read, never edited) | 3 |
| `Contestant::from_parts` (:637), `Contestant::descriptor` (:817), `Contestant::PRODUCTION` (:558) | `crates/yokohama-core/src/game/arena/stats.rs` (fenced) | 3 |
| the wire corpus's foreign samples, built from `v2-padded` (`wire_fixtures.rs:1897–1898`); the wire's `foreign_bot` pinned to `{played, runs}` (`web/src/wire/contract.ts:677`) | `crates/yokohama-web/tests/wire_fixtures.rs`; `web/src/wire/` | 3 |
| `Store` (`list`, `read`, `write_if_version`, `delete`; :121–142), `FsStore` (:240–420), `MemStore` (:436–505), and the twenty test doubles (`store.rs:535`; `games.rs:2105, 2245, 2362, 2782, 3000, 3311`; `lib.rs:4182, 6334, 7110, 7503`; `tables.rs:2774, 2954, 3413, 3701, 4443, 4713, 5151`) | `crates/yokohama-web/src/` | 4 |
| `Games::delete(id, &User)` / `delete_locked` (:1875–1960: the seat and the creator re-checked, not idleness), `Games::with_log`, `Games::with_clock`, `Games::archive_on`, the `Log` trait; `User::parse` | `crates/yokohama-web/src/tables.rs`; `games.rs:485–518`; `log.rs`; `identity.rs:134–165` | 4 |
| `TableDoc { created_by, seats, status, turn, archived_at }`, `TableStatus::{Open, Dealing, Playing}`, `is_creator` (:417) | `crates/yokohama-web/src/table.rs:222–270` | 4 |
| `Host { table_ttl_days, open_table_ttl_days, … }` (:54–76), `Health` (:318), `health()`; `AppState::new`, `with_multiplayer` (:543), `app()`, `enter()`, `ENTER_PATH` (:895–935) | `crates/yokohama-web/src/api.rs`; `lib.rs` | 4, 5a |
| `cache_control` (:113), `serve_path` (:135), the SPA fallback (:160–186: a miss WITH an extension is 404; without one, the shell) | `crates/yokohama-web/src/assets.rs` | 5a |
| `platform_calls_stay_behind_the_server_feature` (:7555–7656: column-0 items only; a region open until `// ---- end server ----`), `platform::CALLS` | `crates/yokohama-web/src/lib.rs`; `platform.rs`; the convention at `store.rs:27, 433, 515` | 4 |
| the lane document's cells (`map_rows` :83–112 asserts the second column is a host; the three map cells :115–170) and the variables cell (:43–150) | `crates/yokohama-web/tests/lane_document.rs`; `worker/scripts/lane-variables.test.mjs` | 8, 12 |
| `TALLY_VAR`, the tally listener; `tally-report.mjs` | `web/e2e/harness.ts:913–940`; `web/scripts/tally-report.mjs` | 8 |
| `plays_locally.spec.ts`'s cells (:236 the no-`/api/` game; :822 the API-refused first screen; :1205 "Will not open here"); `offline.spec.ts`'s second release (`shipRelease` :1443–1447 CHANGES a file, then re-runs the generator at :821/:1449) | `web/e2e/` | 1, 3, 8 |
| E's ledger instruments: `pinned-browsers.mjs`, `close-sequence.sh`, `nas-build.sh`, `nas-swap.sh`, `worker-deploy.sh`, `worker-readings.sh`, `state1-readings.mjs`, `state2-negative.mjs`, `after-revert-clean.mjs` | `.superpowers/sdd/2026-09-29-web-frontend-m6.5c2e-plan/` (gitignored) | 8, 11, 15 |

### What cycle F produces (the contract between its own tasks)

| name | shape | task |
|---|---|---|
| `RECHECK_MS = 15 * 60 * 1000`; `useReadiness` re-checking on `visibilitychange` and on the interval; `ReadinessStatus.staleTab: boolean` (set on a `controllerchange` this tab did not ask for); `useDoorAutoApply(isDoor: boolean, status: ReadinessStatus): void` (once per page life; only with `updateWaiting`; only while `isDoor`; deferred while `applyBusy()`); `setApplyBusy(busy: boolean)` / `applyBusy()` (module-level, `swRegister.ts`); ONE component `UpdateLine({ status })` (`web/src/UpdateLine.tsx`) rendering `<p className="muted update-line">A new version is ready. <button className="linkish">Reload to use it</button></p>` when `updateWaiting`, and the same sentence with a plain reload when `staleTab`; the guard's check 10 `cacheBlock(name, value, failures)` | the update path | 1 |
| `web/src/releaseFeed.ts`: `defaultFeedFor(projectUrl: string \| null): string \| null`, `releaseFeedUrl(): string \| null`, `parseVersion(s): [number, number, number] \| null`, `isNewer(tag, own): boolean`, `useReleaseNotice(fetchImpl?): { tag: string; url: string } \| null` (once per page life; the stored answer `{checkedAt, tag, url}` under `yokohama.release.checked`, drawn from within the hour); `web/src/ReleaseNotice.tsx`; `YOKOHAMA_RELEASE_FEED` and `YOKOHAMA_TURNSTILE_SITEKEY` in `BUILD_VALUES` with `checkReleaseFeed(raw, projectUrl)` and `checkSiteKey(raw)` (plain text, at most 100 characters, no whitespace); `__YOKOHAMA_RELEASE_FEED__`, `__YOKOHAMA_TURNSTILE_SITEKEY__`, `__YOKOHAMA_VERSION__`; `buildValues.ts`'s `releaseFeed()`, `turnstileSiteKey()`, `buildVersion()`; the two Dockerfile `ARG`s; the two §12 rows | the release notice and the two values | 2 |
| `pub enum Runs { Yes, No(String) }`; `bots::runs_here(record: &GameRecord) -> Vec<(u8, BotDescriptor, Runs)>` (R-FP12); `bots::foreign(record) -> Vec<(u8, BotDescriptor, String)>` (the `No` seats with their reasons); `TableBot.runs_here` from it; `bots::cannot_continue(played: &str, runs: &str, why: &str) -> String` = `This game was played against {played}. This server can't continue it: {why}`; `GameError::ForeignBot { played, runs, why }` (the wire's `foreign_bot` keeps exactly `{played, runs}`; `why` rides the sentence only); the corpus regenerated for the sentence | the resume rule | 3 |
| `crates/yokohama-web/src/sweep.rs`: `pub struct Ttls { pub table: Option<u64>, pub open: Option<u64> }`, `Ttls::from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, String>`, `Ttls::none()`; `pub enum Ending { Expired { days: u64 }, Waited { days: u64 } }`; `pub fn ending(status: TableStatus, last_active: u64, now: u64, ttls: &Ttls) -> Option<Ending>`; `pub fn check_archive(ttls: &Ttls, archive_on: bool) -> Result<(), String>`; `pub fn pass(games: &Games, ttls: &Ttls, now: u64) -> Vec<(String, Ending, Result<(), String>)>`; `#[cfg(feature = "server")] pub fn run(state: Arc<AppState>, ttls: Ttls, every: Duration, first_after: Duration)` in the closed region; `Store::last_written` (R-FP6); `MemStore::stamp_for_test(id, secs)`; `AppState.ttls`; the refusal sentences and log lines (Task 4) | the sweep | 4 |
| `web/public/404.html`; `not_found_handling: "404-page"` in all five configurations; `assetsBlock` amended; check 11 `notFoundPage(tree, failures)`; check 12 `turnstileConsistency(config, value, tree, failures)` reading `vars.YOKOHAMA_TURNSTILE` and `build-values.json`'s `turnstile_sitekey`; `web/dist/build-values.json` (R-FP3, a Vite `writeBundle` plugin in `vite.config.ts`); the precache walk excluding `404.html`, `robots.txt`, `llms.txt`, `sitemap.xml`, `build-values.json`; `accessPaths` no longer deleting `health` (ten destinations); the axum 404 page for a miss with an extension | the 404 page and the guard | 5a |
| `web/scripts/presence.mjs` writing `dist/robots.txt`, `dist/llms.txt`, `dist/sitemap.xml` (the last only with `YOKOHAMA_SITE_URL`); the shell's metadata through `transformIndexHtml`; `YOKOHAMA_SITE_URL` in `BUILD_VALUES` with `checkSiteUrl` (an absolute `https://` origin, no path beyond `/`, at most 200 characters); `__YOKOHAMA_SITE_URL__`; `buildValues.ts`'s `siteUrl()`; the Dockerfile `ARG`; the `_headers` lines; its §12 row | the presence | 5b |
| `web/src/turnstile.ts`: `loadTurnstile(): Promise<TurnstileApi>` (the script injected once, `?render=explicit`), `challengeOnce(container): Promise<void>` (resolves when the widget's callback fires; the token discarded — the cookie is the point), `setChallengeHandler(fn \| null)`, `passChallenge(): Promise<void>`, `isEdgeChallenge(res: Response): boolean` (`status === 403 && cf-mitigated === "challenge"`), `EDGE_CHECK`, `EDGE_CHECK_REPEATED`; `web/src/Challenge.tsx` (the overlay); `call` and the record download retrying once; `sse.ts` and `ws.ts` making one `call` on a challenged failure before their reconnect; `FrontDoor.tsx`'s friends card: with a key, a `<button className="primary-link">Go to the lobby</button>` that renders the widget then `location.assign("/enter")`; without, the `<a>`; `PendingReason` gains `"challenged"`; `outbox.ts`: `drain({ interactive: false })` marks every waiting row `challenged` on a challenged health read or send, `drainVisible()` retries them through the handler; the nameless visitor under `identity: "open"` marked `signed-out` locally, no post | Turnstile's client half | 6 |
| `worker/scripts/edge-rule.mjs`: `allowList(tree) -> { files: string[], prefixes: string[] }` from the assets tree's top level, `expression(allow, host)`, `ruleJson(allow, host)` (`expression`, `action: "managed_challenge"`, `description: "yokohama — the no-Access door's edge check (F)"`), `verify(ruleFromZone, allow, host) -> string[]` (differences), the CLI (`node scripts/edge-rule.mjs [--verify < rule.json]`); its cells; `YOKOHAMA_TURNSTILE` in the five configurations (`"off"` everywhere but `wrangler.open.jsonc`), in `Env` and `ROWS` as an unread string, in the open file's allowed differences; `smoke-release.mjs`'s `on` reading; the §12 row | the edge rule, the declaration, the smoke | 7 |
| `docs/web-lane.md`'s section "## 16. The metered-action ledger" (route \| trigger \| purpose \| per session / per applied move \| before a clearance? \| script-able without a browser? \| instrument); `lane_document.rs`'s `map_rows` parameterised and the cell `the_ledger_names_every_route`; `web/e2e/door_unmetered.spec.ts` (three cells); `worker/scripts/probe-burst.mjs` (three shapes); `web/scripts/pinned-browsers.mjs` | the instruments | 8 |

---

## File structure

**Created:** `web/src/UpdateLine.tsx`, `web/src/UpdateLine.test.tsx` (1); `web/src/releaseFeed.ts`, `web/src/releaseFeed.test.ts`,
`web/src/ReleaseNotice.tsx`, `web/src/ReleaseNotice.test.tsx` (2); `crates/yokohama-web/src/sweep.rs` (4); `web/public/404.html`
(5a); `web/scripts/presence.mjs`, `web/scripts/presence.node.test.ts` (5b); `web/src/turnstile.ts`, `web/src/turnstile.test.ts`,
`web/src/Challenge.tsx`, `web/src/Challenge.test.tsx` (6); `worker/scripts/edge-rule.mjs`, `worker/scripts/edge-rule.test.mjs` (7);
`web/e2e/door_unmetered.spec.ts`, `worker/scripts/probe-burst.mjs`, `worker/scripts/probe-burst.test.mjs`,
`web/scripts/pinned-browsers.mjs` (8); `deploy/update.sh` (14); `docs/plans/<the day>-web-frontend-m6.5c2f-close.md` (15); in
the ledger: the reports, the reviews, the pictures (`task-1-look/`, `task-2-look/`, `task-6-look/`), `probe/`, `window.md`,
`live.md`, `nas-sweep.md`, `sequence.tsv`.

**Modified** (principal files; each task's own list is the authority): `web/src/swRegister.ts`, `swRegister.test.ts`,
`App.tsx`, `App.test.tsx`, `OnThisDevice.tsx`, `FrontDoor.test.tsx` (a cell renamed), `BackupControls.tsx`, `Lobby.tsx`,
`styles.css`, `web/e2e/offline.spec.ts`, `worker/scripts/deploy-guard.mjs` and its cells (1); `web/scripts/build-values.mjs`
and its cells, `web/vite.config.ts`, `web/src/vite-env.d.ts`, `web/src/buildValues.ts`, `FrontDoor.tsx`, `Lobby.tsx`,
`deploy/Dockerfile`, `deploy/README.md` §3, `web/scripts/e2e-build.mjs`, `docs/web-lane.md` §12 (2);
`crates/yokohama-web/src/{bots,games,explain,tables}.rs`, `web/src/wire/` (the foreign samples' sentence), the client cells
that render them (3); `crates/yokohama-web/src/{store,lib,main,api,log}.rs`, `deploy/README.md` §4 and §8, `docs/web-lane.md`
§12 and §15 (4); `worker/wrangler*.jsonc` (five), `worker/scripts/{deploy-guard,access-paths}.mjs` and their cells,
`worker/scripts/assemble-assets.test.mjs` (a cell only), `web/scripts/precache.mjs`, `web/src/precache.node.test.ts`,
`web/vite.config.ts`, `crates/yokohama-web/src/assets.rs` (5a); `web/index.html`, `web/vite.config.ts`, `web/package.json`,
`web/scripts/build-values.mjs`, `web/src/vite-env.d.ts`, `web/src/buildValues.ts`, `worker/static/_headers`, `deploy/Dockerfile`,
`deploy/README.md`, `docs/web-lane.md` §12 (5b); `web/src/{api,sse,ws,outbox,FrontDoor,main}.ts(x)` and their cells,
`web/src/local/db.ts`, `styles.css` (6); `worker/wrangler*.jsonc` (five), `worker/src/{env,config}.ts`, `config.unit.test.ts`,
`worker/scripts/{deploy-guard,smoke-release}.mjs` and their cells, `worker/README.md`, `worker/RUNBOOK.md` §3,
`docs/web-lane.md` §12 (7); `docs/web-lane.md` §16, `crates/yokohama-web/tests/lane_document.rs`, `.gitignore`,
`web/e2e/every_spec_declares_its_worker.spec.ts` (8); the three replay-scrubber cells, `web/e2e/state1_game.spec.ts`,
`crates/yokohama-web/src/api.rs:2189–2196`, `README.md`, `docs/tools/guidingpath-relocate.py` (9); `docs/web-lane.md` (12);
`CLAUDE.md`, `docs/handover.md`, `guidingpath.md`, `docs/plans/2026-09-11-ai-lane-reentry-handover.md` (13); `README.md`,
`deploy/README.md`, `deploy/RUNBOOK.md`, `worker/RUNBOOK.md`, `worker/README.md` (14); the spec's §10.14, the RUNBOOKs' records (15).

---

## Task order and why

```
1 ─► 2 ─► 3 ─► 4 ─► 5a ─► 5b ─► 6 ─► 7 ─► 8 ─► 9 ─► 10 (review, wave, battery, code-head gates) ─► 11 (the live readings, the window)
                                                                                                    └─► 12 ─► 13 ─► 14 ─► 15
```

1. **1 first:** the update path is what every later browser reading rides, and it touches `swRegister.ts`, which no
   other task touches.
2. **2 before 5a, 5b and 6:** 2 extends the build-values checker once and defines the site key's value; 5a's stamp
   needs that value to exist; 5b adds the site URL in the same shape; 6 reads the key.
3. **3 and 4 are self-contained** and land before the configuration files move.
4. **5a before 5b, 6 and 7:** 5a writes the stamp and the guard's checks whose names 6 (the key) and 7 (the
   declaration) honour; 5b's presence files are excluded from the precache by 5a's walk.
5. **6 before 7:** 7's smoke reads the challenge the client of 6 is built to pass.
6. **8 after 1–7:** the ledger and the fence cell read the client's requests as the cycle leaves them.
7. **9 last of the code:** small, disjoint files.
8. **10, then 11:** the review before the deploys, so a fix wave never forces a second live sequence.
9. **12 → 14 after 11:** every document records Part 2's readings.
10. **15 last.**

---

## Couplings between tasks (binding; the later task's implementer does what its item says)

- **Task 1 → Task 8:** `pinned-browsers.mjs` asserts the door's exact sentences; Task 1's report names them.
- **Task 1 → Task 11:** 11.D's reading opens the game as a second TAB of the pinned context; a game tab that loses its
  line, or a door that reloads twice, is a STOP.
- **Task 2 → Task 5a:** the stamp's `turnstile_sitekey` boolean comes from `checkSiteKey`'s result; 5a reads it through
  `build-values.mjs`, never the raw environment.
- **Task 2 → Task 5b:** `BUILD_VALUES` gains `YOKOHAMA_SITE_URL` in 5b in the shape 2 leaves.
- **Task 2 → Task 6:** `turnstileSiteKey()` is 2's reader; 6 imports it.
- **Task 3 → Task 8:** the ledger's `…/why` row cites the thirteenth refusal as 3 leaves it.
- **Task 3 → Task 13:** the AI lane's re-entry line is the V5 requirement Task 3's report states in one sentence.
- **Task 4 → Task 14:** the self-host guide quotes the exact refusal sentences and log lines Task 4's report gives.
- **Task 5a → Task 7:** check 12 reads `vars.YOKOHAMA_TURNSTILE` and `build-values.json`'s `turnstile_sitekey`; 7 adds
  the var to the five configurations exactly as check 12 expects.
- **Task 5a → Task 11:** the probe runs with the rule ON; the 404 page is what a browser-shaped navigation reads.
- **Task 6 → Task 7:** the smoke's `on` reading is the request shape 6's handler meets (a cookie-less fetch, 403,
  `cf-mitigated: challenge`).
- **Task 6 → Task 8:** the fence cell counts the door's requests with the key UNSET (the e2e build); the widget script
  must not be among them.
- **Task 7 → Task 11:** the window's rule is `edge-rule.mjs`'s JSON verbatim; `--verify` reads it back after creation.
- **Task 8 → Task 12:** the ledger section's table is the one Task 12 completes with Part 2's figures.
- **Task 11 → Tasks 12–14:** every figure in the documents is Part 2's reading, named with its date.
- **Every task → Task 10:** each report names its catchers by cell name; the battery rows cite them.

---

### Task 1: The update path — the re-check on focus and every 15 minutes, the door applying itself, the two-tab line, the guard's cache check — Opus (R-F1, R-F3; spec §2.1; Review Focus 1)

**Files:**
- Modify: `web/src/swRegister.ts:184–296` (`useReadiness`: the `visibilitychange` listener and the interval on `undo`;
  `staleTab` set in the existing `controllerchange` listener when `asked.current` is `null`; `setApplyBusy`/`applyBusy`;
  `useDoorAutoApply`), the module doc; `web/src/swRegister.test.ts`; `web/src/App.tsx:191` (`useDoorAutoApply(route.kind
  === "door", readiness)`), `:1602`, the lobby's `lobby-actions` (`Lobby.tsx:615–633`) and the game's `<header
  className="banner">` (`App.tsx:2118`) rendering `<UpdateLine status={readiness} />`; `web/src/OnThisDevice.tsx:247–280`
  (`Readiness` delegates its update branch to `UpdateLine`); `web/src/BackupControls.tsx` (`setApplyBusy(true)` around
  a restore, `false` in `finally`) and the outbox's "Send now"/"Check now" button (the same); `web/src/FrontDoor.test.tsx:285–296`
  (the cell renamed "the door's release line switches only when it is pressed, and the App applies by itself" — its
  assertion unchanged); `web/src/styles.css` (`.update-line`); `worker/scripts/deploy-guard.mjs:441–490` (check 10 in
  `deployGuard`, R-FP13) and `deploy-guard.test.mjs`; `web/e2e/offline.spec.ts:445` (the update cell's "nothing switches
  until the click" holds for a NON-door page).
- Create: `web/src/UpdateLine.tsx`, `web/src/UpdateLine.test.tsx`.
- Do NOT touch: `web/src/sw.ts`, `swRoute.ts`, `swInstall.ts` (the worker half is untouched — a diff there is a review
  finding).

**Interfaces:**
- Consumes: `applyUpdate`, `updateWaiting`, the `controllerchange` listener as it is (`:243–251`); `RegistrationLike.update()`.
- Produces: the names table's row 1, verbatim.

- [ ] **Step 0 (the ORCHESTRATOR, before this task is dispatched): the baseline.** E's `step0-baseline.sh` copied into
  `$L` and run at this plan's commit: every per-edit gate's count and wall into `$L/baseline.md` (cargo, the eight
  identity cells, clippy, both rustdocs, the client's four, the Worker's four, `npm run e2e` with its per-spec walls).
- [ ] **Step 1: The re-check cells, red** (`swRegister.test.ts`, the fake `SwHost` and `RegistrationLike` the file already
  drives): (a) after `register` resolves, a `visibilitychange` to `visible` calls `update()` once; to `hidden`, nothing;
  (b) under fake timers `update()` is called at `RECHECK_MS` and `2 * RECHECK_MS`, never after unmount; (c) `update()`
  rejecting leaves `readiness` as it was and throws nothing. Run: `cd web && npx vitest run src/swRegister.test.ts` →
  FAIL.
- [ ] **Step 2: The re-check.** In the effect, after `adoptRegistration`: the listener and the interval, both on `undo`.
  Step 1 → PASS.
- [ ] **Step 3: The auto-apply and two-tab cells, red**: (a) `useDoorAutoApply(true, status)` with `updateWaiting` true
  calls `applyUpdate` once and not again on a re-render; (b) `isDoor` false → nothing; (c) `updateWaiting` false →
  nothing; (d) `applyBusy()` true → deferred until `setApplyBusy(false)`; (e) **two hooks over ONE fake registration**:
  the door hook applies, the fake platform activates the worker and fires `controllerchange` in both; the door reloads
  once; the game hook reloads nothing, its `updateWaiting` is now false, and its `staleTab` is true **(Review Focus
  1)**; its click reloads. Run → FAIL.
- [ ] **Step 4: `useDoorAutoApply`, `staleTab`, the busy flag.** Step 3 → PASS.
- [ ] **Step 5: `UpdateLine`, red then green** (`UpdateLine.test.tsx`): nothing when neither `updateWaiting` nor `staleTab`;
  the sentence and the button when either; the click calls `applyUpdate` (waiting) or `reload` (stale). Then the
  component; `Readiness` delegates; the lobby and the banner render it (`App.test.tsx`: a game screen with
  `updateWaiting` shows the line and nothing reloads until the click; a lobby the same); the renamed door cell green.
  A before-and-after picture of the lobby and the game banner at 1280 and 390 px in `$L/task-1-look/` (R-FP9).
- [ ] **Step 6: The e2e cell over two releases** (`offline.spec.ts`, beside :445, with `shipRelease`'s file change then the
  generator): a browser on the DOOR at release A, release B shipped, a plain reload → the door reloads ITSELF and reads
  "Ready to play offline" under B with NO click, A retained beside B; a second TAB of the same context on a LOCAL GAME
  at A when the door applies: its line appears (stale), the game keeps playing, and its click continues the same game
  (the move count preserved) under B. Run: `npm run e2e` → PASS.
- [ ] **Step 7: The guard's check 10, red** (`deploy-guard.test.mjs`): a configuration with `"cache": { "enabled": true }`
  at the top level, under any `env`, or under ANY `exports` name is refused with the sentence `<file> enables Workers
  Cache (cache.enabled is not false) — with it on, static-asset requests are billed at the Worker request rate (Workers
  pricing, footnote 4); remove the block`; `enabled: false` passes; no block passes. Then `cacheBlock` called from
  `deployGuard`. `cd worker && npm run test:scripts` → PASS.
- [ ] **Step 8: The per-edit gates and the commit.**

```bash
git add web/src/swRegister.ts web/src/swRegister.test.ts web/src/App.tsx web/src/App.test.tsx web/src/OnThisDevice.tsx web/src/BackupControls.tsx web/src/FrontDoor.test.tsx web/src/Lobby.tsx web/src/UpdateLine.tsx web/src/UpdateLine.test.tsx web/src/styles.css web/e2e/offline.spec.ts worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs
git commit -m "feat(web): the release is re-checked on focus and every 15 minutes, the door applies a ready release itself, a tab handed over by another tab's apply keeps the line; the deploy guard refuses Workers Cache (R-F3, spec §2.1)"
```

---

### Task 2: The release notice — the feed and site-key build values, the browser-side check, the line on the door and the lobby — Sonnet 5.5 (R-F2; spec §2.2)

**Files:**
- Modify: `web/scripts/build-values.mjs:19–64` (`BUILD_VALUES` gains `YOKOHAMA_RELEASE_FEED` and `YOKOHAMA_TURNSTILE_SITEKEY`;
  `checkReleaseFeed(raw, projectUrl)` implementing R-FP5 — `undefined`/`""` → `defaultFeedFor(projectUrl)`; `"off"`
  → `null`; else an absolute `https://` URL checked as `checkProjectUrl` checks, at most 300 characters;
  `checkSiteKey(raw)` — unset or empty → `null`, else plain text with no whitespace or control character, at most
  100); `web/scripts/build-values.node.test.ts`; `web/vite.config.ts:38–45` (`__YOKOHAMA_RELEASE_FEED__`,
  `__YOKOHAMA_TURNSTILE_SITEKEY__`, `__YOKOHAMA_VERSION__` from `package.json`'s `version`); `web/src/vite-env.d.ts`;
  `web/src/buildValues.ts` (`releaseFeed()`, `turnstileSiteKey()`, `buildVersion()`); `web/src/FrontDoor.tsx`
  (`<ReleaseNotice />` under the blurb, above the link) and `FrontDoor.test.tsx:81–90` (the zero-server-request cell:
  `fetch` throws for same-origin URLs and answers the feed fixture for the feed URL; the cell asserts no same-origin
  call); `web/src/Lobby.tsx:615–633` (`<ReleaseNotice />` in `lobby-actions` beside the project link, R-FP9);
  `deploy/Dockerfile:202–204` (the two `ARG`s); `deploy/README.md:126–143` (two rows); `web/scripts/e2e-build.mjs:14–21`
  (`YOKOHAMA_RELEASE_FEED=off`, no site key); `docs/web-lane.md` §12 (two rows: `YOKOHAMA_RELEASE_FEED` — build value —
  the three states; `YOKOHAMA_TURNSTILE_SITEKEY` — build value — blank draws the plain link).
- Create: `web/src/releaseFeed.ts`, `web/src/releaseFeed.test.ts`, `web/src/ReleaseNotice.tsx`, `web/src/ReleaseNotice.test.tsx`.

**Interfaces:**
- Consumes: `checkProjectUrl`; `projectLink()`; `readItem`/`writeItem` (`web/src/browserStorage.ts`).
- Produces: the names table's row 2, verbatim.

**The judgment already made:** the feed answers GitHub's release shape (`{ "tag_name": "v0.7.0", "html_url": "…" }`);
`isNewer(tag, own)` over `parseVersion` (`v` optional, three numbers, anything else `null` → no notice); the fetch is
`fetch(url, { cache: "no-store", headers: { accept: "application/vnd.github+json" } })` through `fetchImpl` (default
`fetch`), once per page life; the stored answer `{checkedAt, tag, url}` under `yokohama.release.checked` (every access
wrapped; a broken storage means "check") is DRAWN FROM when under an hour old, with no fetch; any failure → `null`,
nothing retried.

- [ ] **Step 1: The checker cells, red** (`build-values.node.test.ts`): the feed's seven cases (R-FP5 and a bad scheme
  naming the variable); the site key's four (unset, empty, a plain key, whitespace refused naming it). Run: `cd web &&
  npm run test:wasm -- build-values` → FAIL.
- [ ] **Step 2: The checkers, the defines, the declarations, the readers.** → PASS; `npm run typecheck` green.
- [ ] **Step 3: The feed cells, red** (`releaseFeed.test.ts`): `parseVersion` and `isNewer` cases (equal false, older
  false, unparseable false); `useReleaseNotice` over a fixture fetch (newer → the answer; equal → `null`; a 500 → `null`;
  a throw → `null`; `releaseFeed()` null → never fetched; a second render → no second fetch; **a stored newer answer
  under an hour old → the notice with no fetch** (F-13); a stored answer over an hour old → one fetch). Run → FAIL.
- [ ] **Step 4: `releaseFeed.ts`.** → PASS.
- [ ] **Step 5: `ReleaseNotice`, red then green**: nothing when `null`; the sentence with the tag as a link (`target="_blank"
  rel="noopener"`). The door and the lobby's actions block render it; the door cell narrowed; a before-and-after picture
  of the door and the signed-in lobby at 1280 and 390 px in `$L/task-2-look/`.
- [ ] **Step 6: The Dockerfile, the guide's rows, `e2e-build.mjs`, the two §12 rows; `npm run test:scripts` (the variables
  cell) green; `npm run e2e`.**
- [ ] **Step 7: The per-edit gates and the commit.**

```bash
git add web/scripts/build-values.mjs web/scripts/build-values.node.test.ts web/vite.config.ts web/src/vite-env.d.ts web/src/buildValues.ts web/src/releaseFeed.ts web/src/releaseFeed.test.ts web/src/ReleaseNotice.tsx web/src/ReleaseNotice.test.tsx web/src/FrontDoor.tsx web/src/FrontDoor.test.tsx web/src/Lobby.tsx deploy/Dockerfile deploy/README.md web/scripts/e2e-build.mjs docs/web-lane.md
git commit -m "feat(web): the door and the lobby say when the release feed names a newer release — YOKOHAMA_RELEASE_FEED derived from a GitHub project link, off with the literal off, the answer kept an hour; YOKOHAMA_TURNSTILE_SITEKEY checked and defined (R-F2, spec §2.2)"
```

---

### Task 3: The resume rule — the web decides by the CLI's own resume; a build that reconstructs the exact recorded bot plays on — Opus (R-F4; spec §2.3; R-FP12; Review Focus 2)

**Files:**
- Modify: `crates/yokohama-web/src/bots.rs:224–250` (`table_bot`, `foreign` → over `runs_here`), :293–318 (the sentence
  with its reason), the module doc (the rule: the CLI's resume decides); `crates/yokohama-web/src/games.rs:691–715` (the
  one site: `GameError::ForeignBot { played, runs, why }`), :1337–1420 (the fixtures: `foreign_pair("lookahead-v1", …)`
  becomes the DRIFT refusal case; a new `genuine_pair(Contestant::LookaheadV1 { …default knobs… })` builds a record whose
  seat carries that contestant's FULL descriptor and is the opens case; `foreign_pair("v2-padded", …)` the ineligible
  case); `crates/yokohama-web/src/explain.rs:443–475` (`foreign_why` over the new `foreign`); `crates/yokohama-web/src/tables.rs`
  where `runs_here` is worded (find by content); the wire corpus (`UPDATE_WIRE_FIXTURES=1 …`: every foreign sample's
  sentence gains its reason; `contract.ts`'s `foreign_bot` keys stay `{played, runs}`); the client cells that render
  those samples (`App.test.tsx`, `OnThisDevice.test.tsx:109–150`, `BackupControls`) adjusted to the new sentence only;
  `web/e2e/plays_locally.spec.ts:1205` (its fixture must stay a GENUINELY foreign identity — read it).
- Do NOT touch: `crates/yokohama-cli/`, `crates/yokohama-core/` (fenced; **no `pub(crate)` → `pub` change**: the rule
  reads the public `from_provenance`); `web/src/wire/production_bot.json`.

**Interfaces:**
- Consumes: `BotSeats::from_provenance`, `contestant_at`, `is_bot`; the three CLI warning shapes (`bot.rs:275–302`,
  `:486–503`); `Contestant::descriptor()`.
- Produces: the names table's row 3, verbatim (R-FP12).

**The judgment already made:** ONE `from_provenance` call per record; a warning names a seat by the literal prefix
`warning: seat {i} ` — the seed-mismatch warning included, which makes a two-seed record foreign (no web record carries
two); the reason is the warning after that prefix with its resume tail cut; `contestant_at(i)` `None` for a recorded
bot seat → `No(reason)` where the reason is the CLI's "was played by controller … but {why}" clause with the tail cut;
a seat with `Some` and no warning → `Yes`. The pin cell holds the three tails and the prefix verbatim, so a CLI wording
change fails here first.

- [ ] **Step 1: The predicate cells, red** (`bots.rs` tests): over records built from full descriptors —
  `LookaheadV1 { default }` → `Yes`; `Contestant::PRODUCTION` → `Yes`; `NamedTable(V2Padded)` → `No("v2-padded is a
  named research identity for arena weight comparisons, not available as a CLI bot")` **(Review Focus 2)**; the
  production descriptor with `version + 1` → `No(…"records controller … at version …, but this build runs version …")`;
  the production descriptor with its `weights` digest moved by one → `No(…"records weights=…, but this build
  derives weights=…")`; a `v4-step9-draft1` seat at `b: 8` → `No(…from_parts' sentence…)`; controller `"nope-v9"` →
  `No(…"which this build does not know how to run"…)`; `AtomicDiag` → `No(…"is a diagnostic"…)`; the pin cell over the
  three tails and the prefix. Run: `cargo test -p yokohama-web --lib bots` → FAIL.
- [ ] **Step 2: `runs_here`, `foreign`, `table_bot`, the sentence.** Step 1 → PASS.
- [ ] **Step 3: The site's cells** (`games.rs`): the genuine `lookahead-v1` record OPENS under the production build and
  the bot makes the next move (asserted through the applied event's seat) **(Review Focus 2)**; the mongrel
  (`foreign_pair("lookahead-v1")`) REFUSES with `why` naming `weights`; `foreign_pair("v2-padded")` refuses with the
  CLI's sentence; the finished foreign game opens; `an_unfinished_foreign_game_is_refused_by_the_retry_and_cleared_by_abandon`
  on `v2-padded`. Red first for the genuine and the mongrel; then the site carries `why`.
- [ ] **Step 4: `why` and the lobby.** A cell: `why` on a move by the genuine `lookahead-v1` seat reconstructs (no refusal);
  on a `v2-padded` seat the authored refusal with the reason. The lobby's wording where `runs_here` is false carries the
  sentence.
- [ ] **Step 5: The corpus.** Regenerate; READ the diff: every foreign sample's sentence changed and NOTHING else; the
  client cells adjusted to the sentence; the e2e at `plays_locally.spec.ts:1205` green on a genuinely foreign identity.
- [ ] **Step 6: The report states the V5 requirement in one sentence** for Task 13: "a V5 build plays V4 games on the
  web exactly when the V5 ship admits `v4-step9-draft1` in the CLI's `eligible`; until then the web refuses them with
  `eligible`'s own sentence, as the CLI does."
- [ ] **Step 7: The per-edit gates, the corpus pins, `npm run e2e`, and the commit.**

```bash
git add crates/yokohama-web/src/bots.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/explain.rs crates/yokohama-web/src/tables.rs web/src/wire web/src/App.test.tsx web/src/OnThisDevice.test.tsx web/e2e/plays_locally.spec.ts
git commit -m "feat(web): a game opens under any build whose CLI resume reconstructs its exact recorded bot; the refusal names the CLI's own reason (R-F4 amending R-6.5c2.10; spec §2.3; the V5 ship owes one CLI line)"
```

---

### Task 4: The self-host sweep — the two TTLs, the hourly pass through the creator's delete, the archive-required refusal — Opus (R-F9, R-F10; spec §2.4; R-FP6; Review Focus 3)

**Files:**
- Create: `crates/yokohama-web/src/sweep.rs` (the pure half first; the gated region LAST, closed by `// ---- end server ----`).
- Modify: `crates/yokohama-web/src/store.rs:121–142` (`last_written` with the default body `Ok(None)` and its doc
  "activity unknown: never a candidate"), :341–420 (`FsStore`: the latest `modified()` over the three paths that exist,
  as unix seconds; absent all → `Ok(None)`; the house refused as `path` refuses it), :436–505 (`MemStore`: `stamps`
  bumped on every `write_if_version`, `stamp_for_test`, cleared on `delete`); `crates/yokohama-web/src/lib.rs:351–470`
  (`AppState.ttls`; `AppState::new` calls `Ttls::from_lookup(|k| std::env::var(k).ok())` and `check_archive` BEFORE the
  store), the adapter site building `api::Host` (`table_ttl_days: state.ttls.table`, `open_table_ttl_days`);
  `crates/yokohama-web/src/main.rs:135–156` (after `AppState::new`: `tokio::spawn(sweep::run(state.clone(),
  state.ttls.clone(), Duration::from_secs(3600), Duration::from_secs(60)))` when `ttls.table.is_some()`);
  `crates/yokohama-web/src/log.rs` (`sweep-listed`, `sweep-ended`, `sweep-failed`); `deploy/README.md:144–184` (§4's two
  rows) and :224–265 (§8 rewritten); `docs/web-lane.md:444–445` (the host column "both") and §15 (the race sentence:
  a move landing between the pass's read and its delete is refused out loud and the game archived, not lost).
- Do NOT touch: `worker/src/*`; the twenty test doubles (the default body keeps them compiling — a cell asserts a
  double answering `Ok(None)` is never ended).

**Interfaces:**
- Consumes: `Games::delete(id, &User)`; `TableDoc`; `Games::log`; `User::parse`.
- Produces: the names table's row 4, verbatim. **The refusal sentences**: `YOKOHAMA_WEB_TABLE_TTL_DAYS must be a whole
  number of days above zero — got "x"`; the same for the second; `YOKOHAMA_WEB_TABLE_TTL_DAYS is set but
  YOKOHAMA_WEB_ARCHIVE_DIR is not — a sweep without an archive throws games away; set both or neither`. **The log
  lines**: `sweep-listed {id, status, idle_days, reason}` BEFORE any delete; `sweep-ended {id, reason, kept}`;
  `sweep-failed {id, reason, error}`.

**The decision** (the Worker's `expiryReason`, `worker/src/marker.ts:99–129`): `ttls.table = None` → nothing; `open`
and `now >= last_active + open_days * 86400` (open defaulting to 7) → `Waited`; any status past the table TTL →
`Expired`; `dealing` and `playing` wait the table TTL; the shorter wins for `open`.

- [ ] **Step 1: The decision cells, red** (pure): the cases above, the day boundary, `open` past both. Run: `cargo test
  -p yokohama-web --lib sweep` → FAIL.
- [ ] **Step 2: `ending`, `Ttls::from_lookup`, `check_archive`.** → PASS.
- [ ] **Step 3: The store cells, red** (`store.rs`'s `both(...)` harness): `None` for an unknown id; set after a record
  write; moved by a chat write; the house refused; `None` after `delete`; a double with the default body answers
  `Ok(None)`. → then the two implementations → PASS.
- [ ] **Step 4: The pass cells, red** (`Games` on a `MemStore` with `with_clock` and a recording `Log`): the three tables
  as spec §2.4 lays them out; `sweep-listed` lines for both candidates BEFORE any `sweep-ended` (order asserted)
  **(Review Focus 3)**; the finished-stamped table not put twice; a failing `Archive` → `sweep-failed`, the table kept;
  the house never listed; a table whose store answers `Ok(None)` for `last_written` never ended. → then `pass` → PASS.
- [ ] **Step 5: The start refusal, red**: `check_archive` (a TTL with no archive → the sentence **(Review Focus 3)**; with →
  `Ok`; none → `Ok`); `AppState::new` calls it; `main.rs` exits 1. → PASS.
- [ ] **Step 6: Health and the task.** The two `Host` fields from `state.ttls` (a lib cell: 90 and 7 through; `none()` →
  both `null`, the existing cell at `lib.rs:6729` stays); `run` in the closed gated region with its `SystemTime` read;
  `platform_calls_stay_behind_the_server_feature` green (F-11's layout: the gated `impl`/`fn` items LAST, the region
  closed); `cargo check --no-default-features --target wasm32-unknown-unknown -p yokohama-web` green.
- [ ] **Step 7: The documents; the corpus's `health.json` unmoved (say so).**
- [ ] **Step 8: The per-edit gates and the commit.**

```bash
git add crates/yokohama-web/src/sweep.rs crates/yokohama-web/src/store.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/main.rs crates/yokohama-web/src/api.rs crates/yokohama-web/src/log.rs deploy/README.md docs/web-lane.md
git commit -m "feat(web-server): the self-host sweeps idle tables under YOKOHAMA_WEB_TABLE_TTL_DAYS and YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS, hourly, through the creator's own delete, listing before ending; a TTL without an archive refuses to start (R-F9, R-F10; spec §2.4)"
```

---

### Task 5a: The 404 page, the build-values stamp, the guard's checks 11 and 12, health into the protected list — Opus (R-F5; spec §2.5; R-FP3, R-FP13; Review Focus 5)

**Files:**
- Create: `web/public/404.html` (the app's own colours, `<meta name="robots" content="noindex">`, the title "Nothing
  here", "There's nothing at this address." and a link "Go to the door" to `/`).
- Modify: the five `wrangler*.jsonc` assets blocks (`"not_found_handling": "404-page"`); `worker/scripts/deploy-guard.mjs:133–160`
  (`assetsBlock` expects `404-page`), :441–490 (`deployGuard` gains check 11 `notFoundPage(tree, failures)` — the tree
  holds `404.html` — and check 12 `turnstileConsistency(config, value, tree, failures)`: `vars.YOKOHAMA_TURNSTILE` read as
  `on`/`off`/absent (absent = off), `build-values.json`'s `turnstile_sitekey` as a boolean; `on` without `true` →
  `<file> sets YOKOHAMA_TURNSTILE on but the assets tree's build carries no site key (build-values.json) — build the
  client with YOKOHAMA_TURNSTILE_SITEKEY set`; `off`/absent with `true` → `<file> leaves YOKOHAMA_TURNSTILE off but
  the assets tree's build carries a site key — the widget would render under Access; build the client without the key
  or set the var on`; no stamp in the tree → refused naming the file), and its cells; `worker/scripts/assemble-assets.test.mjs`
  (a cell: `404.html` and `build-values.json` land in the tree — no edit to the assembler); `web/scripts/precache.mjs:108`
  (the walk excludes `404.html`, `robots.txt`, `llms.txt`, `sitemap.xml`, `build-values.json`); `web/src/precache.node.test.ts`;
  `web/vite.config.ts` (a `writeBundle` plugin writing `dist/build-values.json` with the six booleans from the checked
  values — `site_url` false until Task 5b); `worker/scripts/access-paths.mjs:55` (`segments.delete("health")` removed;
  the header corrected) and its cell (ten destinations); `crates/yokohama-web/src/assets.rs:135–186` (a miss WITH an
  extension answers `404.html`'s bytes, status 404, `text/html`; the extension-less fallback to the shell stays).
- Do NOT touch: `web/src/sw.ts`, `swRoute.ts`.

**Interfaces:**
- Consumes: the checked values as Task 2 leaves them (`build-values.mjs`); `assetsBlock`'s key list; `readPrecacheList`.
- Produces: the names table's row 5a, verbatim.

- [ ] **Step 1: The guard cells, red**: the five files pass with `404-page`; `none` refused; a tree without `404.html`
  refused naming it; the four Turnstile combinations with their sentences over fixture trees; a tree with no stamp
  refused. Run: `cd worker && npm run test:scripts` → FAIL.
- [ ] **Step 2: The configurations and the guard** (checks 11 and 12 in `deployGuard`, R-FP13). → PASS.
- [ ] **Step 3: The stamp and the exclusion, red then green** (`precache.node.test.ts`: none of the five names is an entry
  and the count is what it was; a node cell: `dist/build-values.json` is exactly the six booleans). `node scripts/deploy-guard.mjs`
  on the committed files reads green once the stamp exists.
- [ ] **Step 4: The axum 404, red then green** (`assets.rs` tests over the router with a scratch `dist` holding `404.html`):
  `GET /nope.json` and `GET /wp-login.php` answer 404 with the page and `text/html`; `GET /` and `GET /t/abc` still the
  shell **(Review Focus 5)**.
- [ ] **Step 5: The protected list** (ten destinations; the comment says why, spec §2.5). `npm run test:scripts` → PASS.
- [ ] **Step 6: `npm run e2e`** (the offline counts unchanged).
- [ ] **Step 7: The per-edit gates and the commit.**

```bash
git add web/public/404.html web/scripts/precache.mjs web/src/precache.node.test.ts web/vite.config.ts worker/wrangler.jsonc worker/wrangler.test.jsonc worker/wrangler.open.test.jsonc worker/wrangler.open.jsonc worker/wrangler.gate.jsonc worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs worker/scripts/assemble-assets.test.mjs worker/scripts/access-paths.mjs worker/scripts/access-paths.test.mjs crates/yokohama-web/src/assets.rs
git commit -m "feat(web,worker): a static 404 page for a browser's miss on both hosts (not_found_handling 404-page); the build-values stamp; the guard requires the page and holds the Turnstile declaration to the stamp; health joins the protected paths (R-F5; spec §2.5)"
```

---

### Task 5b: The web presence — robots.txt, llms.txt, the sitemap, the page's metadata, the site URL — Sonnet 5.5 (R-F12; spec §2.7)

**Files:**
- Create: `web/scripts/presence.mjs`, `web/scripts/presence.node.test.ts`.
- Modify: `web/scripts/build-values.mjs` (`YOKOHAMA_SITE_URL`, `checkSiteUrl`) and its cells; `web/vite.config.ts`
  (`__YOKOHAMA_SITE_URL__`; the stamp's `site_url` from it; `transformIndexHtml` injecting the metadata);
  `web/src/vite-env.d.ts`; `web/src/buildValues.ts` (`siteUrl()`); `web/index.html:1–33` (a placeholder comment where
  the transform inserts); `web/package.json:12` (`build:client` runs `presence.mjs` after `precache.mjs`);
  `worker/static/_headers` (`/robots.txt`, `/llms.txt`, `/sitemap.xml`: `Cache-Control: public, max-age=3600`);
  `crates/yokohama-web/src/assets.rs:113–118` (the same for the three); `deploy/Dockerfile:202–204` (`ARG YOKOHAMA_SITE_URL`);
  `deploy/README.md:126–143` (the row); `docs/web-lane.md` §12 (the row).
- Do NOT touch: `web/src/sw.ts`; `worker/scripts/assemble-assets.mjs`.

**The texts, given so no wording is decided in the task** — spec §2.7's; copied here verbatim for the implementer:
`robots.txt`:

```
User-agent: *
Allow: /
Disallow: /api/
Disallow: /enter
Disallow: /cdn-cgi/
```

plus `Sitemap: <site url>/sitemap.xml` when the site URL is set. `llms.txt`:

```
# Yokohama

> Yokohama, the board game by Hisashi Hayashi, in your browser: a bot game that runs entirely in the browser, and multiplayer behind a door. An independent open-source project, not affiliated with the game's designer or publisher.

- The blurb: <YOKOHAMA_SITE_BLURB, when set>
- The notice: <YOKOHAMA_SITE_NOTICE, when set>

## Code

- [The repository](<YOKOHAMA_PROJECT_URL>): Apache-2.0; the game's own material carries no grant (ASSETS.md in the repository).

## What a visitor can do here

- Play the bot with no account: everything runs in the browser and works offline once installed.
- Play with friends behind the door at /enter, which is not for automated clients.
- The API under /api/ serves the game's own client and is not a public API.
```

(the "Code" section only when the link is set). `sitemap.xml`: one `<url><loc><site url>/</loc></url>`. The
metadata: `<meta name="description">` from the blurb or the default sentence "Yokohama, the board game, in your
browser. Play the bot here, or play with friends."; `<link rel="canonical">` and `og:url` with the URL only; `og:title`
"Yokohama", `og:description` the description, `og:type` `website`, `og:image` the app's largest icon in
`web/public/icons/` (read its real name), absolute with the URL else root-relative; `twitter:card` `summary`; one
`<script type="application/ld+json">` with `{"@context":"https://schema.org","@type":"WebApplication","name":"Yokohama","applicationCategory":"GameApplication","operatingSystem":"Any","isAccessibleForFree":true,"license":"https://www.apache.org/licenses/LICENSE-2.0","description":"<the description>","url":"<site url or omitted>","codeRepository":"<project link or omitted>"}`.

- [ ] **Step 1: The checker cells, red**: `checkSiteUrl` (unset → `null`; an origin passes; a path beyond `/` refused;
  `http://` refused; over 200 refused; naming the variable). → then the checker → PASS.
- [ ] **Step 2: The presence cells, red** (`presence.node.test.ts` over a scratch `dist`): the three `Disallow` lines; the
  `Sitemap:` line only with a URL; `llms.txt`'s repository section only with a link and never an `@`; the sitemap only
  with a URL; `404.html` present with `noindex` (Task 5a's file, read only). → then `presence.mjs` → PASS.
- [ ] **Step 3: The shell's metadata, red then green** (a node cell over the built `dist/index.html`): each tag exactly once;
  the JSON-LD parses; `codeRepository` only with a link; the canonical and `og:url` only with a URL. `npm run
  build:client` twice, with and without the two values; both trees checked; the stamp's `site_url` reads true/false.
- [ ] **Step 4: The headers on both hosts** (an `assets.rs` cell for the three paths' cache header; the `_headers` lines);
  the Dockerfile, the guide's row, the §12 row (`npm run test:scripts` green).
- [ ] **Step 5: `npm run e2e`** (the offline counts unchanged).
- [ ] **Step 6: The per-edit gates and the commit.**

```bash
git add web/scripts/presence.mjs web/scripts/presence.node.test.ts web/scripts/build-values.mjs web/scripts/build-values.node.test.ts web/vite.config.ts web/src/vite-env.d.ts web/src/buildValues.ts web/index.html web/package.json worker/static/_headers crates/yokohama-web/src/assets.rs deploy/Dockerfile deploy/README.md docs/web-lane.md
git commit -m "feat(web): robots.txt (the metered paths disallowed), llms.txt, a sitemap and the page's metadata, with YOKOHAMA_SITE_URL for the canonical address (R-F12; spec §2.7)"
```

---

### Task 6: Turnstile's client half — the button's widget, the edge-challenge predicate at every site, the outbox's rules — Opus (R-F6; spec §2.6; R-FP8; Review Focus 4)

**Files:**
- Create: `web/src/turnstile.ts`, `web/src/turnstile.test.ts`, `web/src/Challenge.tsx`, `web/src/Challenge.test.tsx`.
- Modify: `web/src/api.ts:262–275` (`isEdgeChallenge` lives in `turnstile.ts`; `api.ts` imports it), :636–660 (`call`
  gains the one retry after `passChallenge()`, and an `opts?: { interactive?: boolean }` parameter, default true,
  threaded through the helpers the outbox uses — the archive post and the health read, find by content), :820 (the
  record download: the same predicate and one retry); `web/src/sse.ts:117` and `web/src/ws.ts:107` (a failure whose
  response — or, for the socket, whose probe — is an edge challenge makes ONE `call` to `/api/health` with the default
  options, which passes the check, before the existing reconnect ladder continues); `web/src/api.test.ts`,
  `sse.test.ts`, `ws.test.ts`; `web/src/FrontDoor.tsx:142–156` (with a key: `<button type="button"
  className="primary-link">Go to the lobby</button>` — on the click, offline → the sentence "The lobby needs a
  connection."; else `challengeOnce(container)` in the friends card, then `location.assign("/enter")`; without a key
  the `<a>` as today) and `FrontDoor.test.tsx`; `web/src/local/db.ts:87` (`PendingReason` gains `"challenged"`);
  `web/src/outbox.ts:429–470` (`noteGameOver`: under a known `identity: "open"` with no stored claimed name, the row
  is marked `signed-out` locally and nothing is posted; `drain({ interactive })`: the non-interactive drains — the
  `online` event, a background launch — call the API with `interactive: false`, and a challenge met by the health
  read OR a send marks EVERY waiting row `challenged`; `drainVisible()` — the door's launch when the document is
  visible, and the pending row's "Check now" button — retries `challenged` rows through the handler); the door's
  pending row wording "waiting for a human check" with the button; `web/src/outbox.test.ts`; `web/src/main.tsx`
  (`<Challenge />` mounted after `<App />` when `turnstileSiteKey()` is set — as `Footer` is); `web/src/styles.css`.
- Do NOT touch: `web/src/sw.ts`, `swRoute.ts`; the five other `/enter` links (they stay plain navigations, spec §2.6).

**Interfaces:**
- Consumes: `turnstileSiteKey()` (Task 2); `call`/`settle`; `PendingReason`; `readItem`/`writeItem`; the archive
  fact's `identity` (`web/src/local/facts.ts`).
- Produces: the names table's row 6, verbatim. **The script URL** `https://challenges.cloudflare.com/turnstile/v0/api.js?onload=__yokohamaTurnstile&render=explicit`,
  injected ONCE per page life by `loadTurnstile()`, never at module load. **The overlay's words**: "Checking that
  this is a person…" and a "Cancel" button. `EDGE_CHECK = "This site asked for a check this build cannot show."`;
  `EDGE_CHECK_REPEATED = "The site's check did not pass. Try again from the door."`.

**The judgment already made:** `challengeOnce(container)` renders explicitly (`turnstile.render(container, { sitekey,
callback, "error-callback", "expired-callback" })`), resolves when `callback` fires (the token is discarded — the
pre-clearance cookie the widget set is the point), rejects on the two error callbacks or after 90 s, and removes the
widget either way. `passChallenge()` resolves when the overlay's `challengeOnce` resolves; with no handler mounted it
rejects with `EDGE_CHECK`. The retry at each site is ONE; a second challenge is settled as the refusal
`EDGE_CHECK_REPEATED`. `interactive: false` never calls the handler: the challenge becomes the refusal at once, which
the outbox turns into the `challenged` mark.

- [ ] **Step 1: The loader and challenge cells, red** (`turnstile.test.ts`, jsdom, a fake `window.turnstile`): the script
  tag injected ONCE across two calls with the exact URL, resolving on the fake `onload` global; `challengeOnce(el)`
  resolves on the fake `callback` and calls `remove`; rejects on `error-callback` and on the timeout under fake timers;
  `passChallenge()` with no handler rejects with `EDGE_CHECK`; `isEdgeChallenge` over a 403 with and without the
  header, and a 401. Run: `cd web && npx vitest run src/turnstile.test.ts` → FAIL.
- [ ] **Step 2: `turnstile.ts`.** → PASS.
- [ ] **Step 3: The four sites' cells, red**: `api.test.ts` — a fixture 403 with the header then a 200 → the handler once,
  the second response returned; two challenges → the handler once, then `EDGE_CHECK_REPEATED`, no third fetch **(Review
  Focus 4)**; no handler → `EDGE_CHECK`, no retry; `interactive: false` → no handler, the refusal; a plain 403 → today's
  path; the record download the same two cases; `sse.test.ts` — a challenged open makes one `call` then reconnects
  **(Review Focus 4)**; `ws.test.ts` — the same on the socket's probe. Then the predicate and the retries. → PASS.
- [ ] **Step 4: The overlay, red then green** (`Challenge.test.tsx`): hidden at mount; shown by `passChallenge()` with the
  widget in `.challenge-widget`; hidden on resolve; "Cancel" rejects.
- [ ] **Step 5: The button, red then green** (`FrontDoor.test.tsx`): no key → the anchor and the loader never called; a
  key → the button, the loader not called at MOUNT, called once on the click, then `location.assign("/enter")` (spied);
  offline → the sentence and no loader call. The door's same-origin cell (Task 2's shape) still passes.
- [ ] **Step 6: The outbox, red then green** (`outbox.test.ts`): a non-interactive drain whose HEALTH read meets a
  challenge marks every waiting row `challenged` (F-15) and calls no handler; one whose send meets it, the same;
  `drainVisible()` calls the handler, retries and clears on 200; the door lists the row with the wording and the
  button; a `challenged` row is not retried by the next `online` event; a finished game under `identity: "open"` with
  no claimed name is marked `signed-out` with NO request (F-14).
- [ ] **Step 7: `npm run e2e`** (the e2e build has no key: every existing cell green; the door's requests unchanged). A
  picture of the friends card and the overlay from a local build with Cloudflare's always-passing test site key, in
  `$L/task-6-look/`.
- [ ] **Step 8: The per-edit gates and the commit.**

```bash
git add web/src/turnstile.ts web/src/turnstile.test.ts web/src/Challenge.tsx web/src/Challenge.test.tsx web/src/api.ts web/src/api.test.ts web/src/sse.ts web/src/sse.test.ts web/src/ws.ts web/src/ws.test.ts web/src/FrontDoor.tsx web/src/FrontDoor.test.tsx web/src/local/db.ts web/src/outbox.ts web/src/outbox.test.ts web/src/main.tsx web/src/styles.css
git commit -m "feat(web): Turnstile behind a build value — the widget on the multiplayer button's click for pre-clearance, the edge challenge met once at every fetch site and the socket with one retry, the outbox marking challenged rows for a visible tab, no post for a nameless visitor (R-F6; spec §2.6)"
```

---

### Task 7: The edge rule's script, the deploy declaration and the post-deploy smoke — Opus (R-F6; spec §2.5, §2.6; R-FP7)

**Files:**
- Create: `worker/scripts/edge-rule.mjs`, `worker/scripts/edge-rule.test.mjs`.
- Modify: `worker/src/env.ts` (`YOKOHAMA_TURNSTILE?: string`, documented as a deploy declaration the shell never
  reads); `worker/src/config.ts:37–51` (`ROWS` gains it as a string; NO reader — a cell asserts `shellConfig` and
  `moduleConfig` ignore it and the module's eight keys stay eight); `worker/src/config.unit.test.ts`; the five
  `wrangler*.jsonc` (`"YOKOHAMA_TURNSTILE": "off"` with a comment in production, the gate and both pool files; `"on"` in
  `wrangler.open.jsonc`); `worker/scripts/deploy-guard.mjs:275–295` (the open file's allowed differences gain
  `YOKOHAMA_TURNSTILE: "on"`; a pool file with `on` is drift); `worker/scripts/smoke-release.mjs` (when the deployed
  configuration's `vars.YOKOHAMA_TURNSTILE` is `on`: a cookie-less `GET /api/health` with a browser User-Agent must
  answer 403 with `cf-mitigated: challenge`, else the smoke FAILS naming the rule; when `off`, no such line) and its
  cell; `worker/README.md`'s variables table and `worker/RUNBOOK.md:170–260` §3 (the declaration's row: what `on` means
  and what it does not); `docs/web-lane.md` §12 (the row).
- Do NOT touch: `worker/src/index.ts`, `route.ts` (`/enter` is unchanged); `crates/yokohama-worker/`.

**Interfaces:**
- Consumes: the assets tree's layout as `assemble-assets.mjs` produces it; check 12 as Task 5a leaves it.
- Produces: the names table's row 7, verbatim. **The expression**, for `host = "yokohama.moore.nyc"` and the tree's
  top level (files `F`, prefixes `P`):
  `(http.host eq "<host>" and not (http.request.uri.path in {"/" "/index.html" "/sw.js" "/shell.bin" "/manifest.webmanifest" "/404.html" "/robots.txt" "/llms.txt" "/sitemap.xml" "/build-values.json"} or starts_with(http.request.uri.path, "/assets/") or starts_with(http.request.uri.path, "/art/") or starts_with(http.request.uri.path, "/icons/") or starts_with(http.request.uri.path, "/fonts/") or starts_with(http.request.uri.path, "/help/") or starts_with(http.request.uri.path, "/cdn-cgi/")))`
  — the set and the prefixes DERIVED from the tree (every top-level file as `"/<name>"`, `/` for `index.html`, every
  top-level directory as a prefix, `/cdn-cgi/` always added, `_headers` and `_redirects` never listed); the action
  `managed_challenge`; the description `yokohama — the no-Access door's edge check (F)`.

- [ ] **Step 1: The script's cells, red** (`edge-rule.test.mjs`, a fixture tree): `allowList` reads the files and the
  directories, skips `_headers`/`_redirects`, always adds `/cdn-cgi/`; `expression` is byte-exact for the fixture; the
  drift cell: `allowList(worker/assets)` over the COMMITTED tree equals the list written out by hand in the cell, so a
  new top-level name fails until the cell and the zone's rule move together; `verify` reports a differing expression,
  action or description and nothing for an equal rule; the CLI prints the JSON and exits 0, `--verify` exits 1 on a
  difference. Run: `cd worker && npm run test:scripts` → FAIL.
- [ ] **Step 2: `edge-rule.mjs`.** → PASS.
- [ ] **Step 3: The declaration, red then green**: `config.unit.test.ts` — `on`, `off` and absent leave `shellConfig`'s
  fields and `moduleConfig`'s eight keys unchanged (the var is declared, never read); the guard's cells — the open
  file's `on` is not drift, a pool file's `on` is; the five files; `node scripts/deploy-guard.mjs` on the committed
  files reads green (the assets tree's stamp says `false`, production says `off`).
- [ ] **Step 4: The smoke, red then green** (`smoke-release.test.mjs` over a fixture response): `on` + a 403 with the
  header passes; `on` + a 200 FAILS naming the rule; `off` reads nothing. Then the reading in `smoke-release.mjs`.
- [ ] **Step 5: The documents** (the Worker README's row; the RUNBOOK's §3 row; the §12 row; `npm run test:scripts` green).
- [ ] **Step 6: The per-edit gates (`npm run build:module:debug` before the pool) and the commit.**

```bash
git add worker/scripts/edge-rule.mjs worker/scripts/edge-rule.test.mjs worker/src/env.ts worker/src/config.ts worker/src/config.unit.test.ts worker/wrangler.jsonc worker/wrangler.gate.jsonc worker/wrangler.test.jsonc worker/wrangler.open.test.jsonc worker/wrangler.open.jsonc worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs worker/scripts/smoke-release.mjs worker/scripts/smoke-release.test.mjs worker/README.md worker/RUNBOOK.md docs/web-lane.md
git commit -m "feat(worker): the no-Access door's edge rule printed and verified from the assets tree (edge-rule.mjs); YOKOHAMA_TURNSTILE as a deploy declaration the guard holds to the build and the smoke checks against the edge (R-F6; spec §2.6)"
```

---

### Task 8: The instruments and the ledger — the fence cell, the ledger section and its cell, the probe script, the pinned-browser instrument — Opus (R-F5; spec §2.5; Review Focus 5)

**Files:**
- Create: `web/e2e/door_unmetered.spec.ts`; `worker/scripts/probe-burst.mjs`, `worker/scripts/probe-burst.test.mjs`;
  `web/scripts/pinned-browsers.mjs` (E's ledger copy, the `createRequire` path relative — `new URL("../package.json",
  import.meta.url)` — a usage header; state directories under `web/scripts/pinned/` gitignored).
- Modify: `docs/web-lane.md` (a new last section, "## 16. The metered-action ledger", every route a row, the columns
  filled from the code and E's figures; Task 12 completes it with Part 2's); `crates/yokohama-web/tests/lane_document.rs:83–112`
  (`map_rows` parameterised by its column set, so the ledger's second column, `trigger`, parses) and the cell
  `the_ledger_names_every_route` (every route of the layer's `routes()` and each host's own routes appears in §16's
  route column); `.gitignore`; `web/e2e/every_spec_declares_its_worker.spec.ts` (the new spec declares its worker as
  the convention requires — read that spec first).

**Interfaces:**
- Consumes: the release's precache list (`web/dist/sw.js` through `worker/scripts/precache-list.mjs`'s
  `readPrecacheList`); `TALLY_VAR`; the router facts `access-paths.mjs` reads; `offline.spec.ts`'s `controlled(page)`.
- Produces: the names table's row 8, verbatim. **The ledger's columns**: `route` · `trigger` · `purpose` · `per
  session / per applied move` (E's figures where they exist; "F: —" where Part 2 fills) · `before a clearance?` ·
  `script-able without a browser?` · `instrument`. **The first paragraph** carries the harness caveat in the spec's
  words and the nameless-visitor row's path (spec §2.5).

- [ ] **Step 1: The fence cells, red** (`door_unmetered.spec.ts`, the worker ON, no sign-in): (a) the door loaded cold:
  every same-origin request's path is in the release's precache list or is one of `/`, `/sw.js`, `/shell.bin`; none
  starts `/api/` and none is `/enter`; (b) a local bot game played to game over with no sign-in: no `/api/` request
  until the screen says the game is over, and after it at most `/api/health` and `/api/archive`; (c) against the local
  binary, `fetch("/wp-login.php")` from the page answers 404 with the page's title, a navigation to `/wp-login.php` in
  a fresh uncontrolled context answers the 404 page, and the same navigation in a context the worker CONTROLS answers
  the shell (rule 2) **(Review Focus 5)**. Run: `cd web && npx playwright test door_unmetered` → FAIL.
- [ ] **Step 2: The spec** (on `plays_locally.spec.ts:822`'s shape and `offline.spec.ts`'s helpers). → PASS; `npm run
  e2e` whole → PASS.
- [ ] **Step 3: The ledger's cell, red** (`lane_document.rs`): the section is absent → FAIL. Then `map_rows` parameterised
  (the three map cells unchanged in behaviour) and §16 written with every route as a row — the layer's `routes()`,
  the Worker's own `/enter`, `/api/art`, the streams, `/api/admin/*`; the presence files, the 404 page and the stamp
  as NON-metered rows marked "static". → PASS.
- [ ] **Step 4: The probe script and its cells** (`probe-burst.test.mjs`, no network): argument parsing (`<origin>
  [--count N] [--pace ms] [--paths a,b,c]`), the THREE header shapes (bare; a browser User-Agent with `Sec-Fetch-Mode:
  navigate` and `Accept: text/html`; a browser User-Agent with `Sec-Fetch-Mode: no-cors` and `Accept: */*`), the TSV
  row formatter (`path`, `shape`, `status`, `cf-mitigated`, `cf-cache-status`, `cf-ray`, `content-type`, `bytes`) over
  a fake response; the default paths `/.env`, `/wp-login.php`, `/admin`, `/xmlrpc.php`, a random `/<hex>.json`;
  redirects never followed. Orchestrator-only, said in its header; nothing sent in any cell.
- [ ] **Step 5: The pinned instrument** and its node cell (no arguments → exit 2 with the usage line); `.gitignore`.
- [ ] **Step 6: The per-edit gates and the commit.**

```bash
git add web/e2e/door_unmetered.spec.ts web/e2e/every_spec_declares_its_worker.spec.ts worker/scripts/probe-burst.mjs worker/scripts/probe-burst.test.mjs web/scripts/pinned-browsers.mjs .gitignore docs/web-lane.md crates/yokohama-web/tests/lane_document.rs
git commit -m "test(web,docs): the door-unmetered fence cell; the metered-action ledger with its route-coverage cell; the probe (three shapes) and pinned-browser instruments committed (R-F5; spec §2.5)"
```

---

### Task 9: E's deferred minors — Sonnet 5.5 (spec §2.8; E's close §6)

**Files:**
- Modify: the three replay-scrubber cells (`grep -n scrubber web/src/App.test.tsx web/src/Surfaces.test.tsx
  web/src/Board.test.tsx` — E's ledger names the three; each `{ timeout: 5_000 }` or `5000` on those cells becomes
  `10_000`, with a comment: 5.9–6.1 s under the sequence's load, green alone); `web/e2e/state1_game.spec.ts` (E's
  close §6's four: m-4 the e2e temp directory made at config load → made in `test.beforeAll`; m-5 the lobby's
  unreachable-server card → an explicit assertion that it is NOT drawn after sign-in; m-6 the final asserts that
  cannot fail → real assertions over the archived record's event count; m-7 the `afterEach` fallback DELETE sent
  anonymously under the open door → sent with the claimed name the walk used); `crates/yokohama-web/src/api.rs:2189–2196`
  (the comment's "five in `CLAUDE.md`" → "a third count in an earlier document"; **this re-stamps the Worker's picker
  code, by design** — say so in the report); `README.md:271–275` ("an open seat for anyone with the link" → "an open
  seat for anyone who has the link and can pass the deployment's door") and its two long PROSE lines over 200
  characters (`awk 'length>200' README.md`; table rows and link lines are not prose); `docs/tools/guidingpath-relocate.py:202–245`
  (`check`'s docstring and the `Fail` message: "a line past the plan's end — expected once the tree has moved on after
  the relocation; the proof is `check --at 054b1e4f`").
- Do NOT touch: anything else. A fourth scrubber cell or a fifth state-1 item is a STOP and a question.

- [ ] **Step 1: The scrubber cells** raised; `cd web && npx vitest run src/App.test.tsx src/Surfaces.test.tsx
  src/Board.test.tsx` green.
- [ ] **Step 2: The state-1 spec's four**, red-first where a behaviour is asserted (m-5, m-6, m-7); `npx playwright test
  state1_game --list` compiles; a local dry run (`YOKOHAMA_WEB_STATE1_GAME=1` against the local binary, as E's ruling
  10-c did) green.
- [ ] **Step 3: The comment, the README, the script** (`python3 docs/tools/guidingpath-relocate.py --self-test` green).
- [ ] **Step 4: The e2e per-spec walls**: `npx playwright test --reporter=list 2>&1 | tee $L/task-9-e2e-walls.txt`; the
  report names the three slowest specs and their walls; a spec over five minutes is NAMED, not fixed.
- [ ] **Step 5: The per-edit gates and the commit.**

```bash
git add web/src/App.test.tsx web/src/Surfaces.test.tsx web/src/Board.test.tsx web/e2e/state1_game.spec.ts crates/yokohama-web/src/api.rs README.md docs/tools/guidingpath-relocate.py
git commit -m "chore(web,docs): E's deferred minors — the scrubber cells' 10 s timeout, the state-1 spec's four fixes, the api.rs comment, the README nits, the relocation check's sentence (spec §2.8)"
```

---

### Task 10: The review, one fix wave, the battery, and the per-edit set at the code head — ORCHESTRATOR (spec §3.4; R-FP11)

No code. The order is E's: review → one fix wave → the scoped re-reviews → the battery → the per-edit set and the
e2e at the code head. The fifteen release gates and the ONE recorded sequence run ONCE, at the FINAL reviewed head
(Task 15.A), as E's R-E17 ruled. The ledger is `L=.superpowers/sdd/2026-09-30-web-frontend-m6.5c2f-plan`.

**A. The branch is whole**

- [ ] **A.1** `git status` clean; `progress.md` holds a DONE line and a review line for each of Tasks 1–9; HEAD is Task
  9's reviewed commit.
- [ ] **A.2 The fences**, output quoted in the close: the six fenced crates' diff against `15de8117` prints NOTHING (no
  CLI change in F); `SEMANTICS_REV` 7; `git diff --stat 15de8117 -- web/src/wire web/src/wire-script` lists ONLY the
  foreign samples Task 3 regenerated; no bot behaviour moved; **the door-unmetered fence**: Task 8's spec green.

**B. The review (Fable), one fix wave, the scoped re-reviews**

- [ ] **B.1** One **Fable** reviewer over `git diff 15de8117..HEAD`, the spec, this plan and the ledger, in an EXPORT:
  the vacuity lens first (every new catcher reverted against its fix); **the door's auto-apply tried on a non-door
  route**; the two-tab cell tried with `staleTab` never set; the resume predicate tried with the drift warnings ignored;
  the sweep tried with the listing after the delete; the 404 page tried out of the assets tree; the guard's check 12
  tried with `on` and a `false` stamp; the fetch handler tried with the retry unbounded; the stream tried with no
  `call` before its reconnect; the feed comparison tried with `>=`; `edge-rule.mjs` tried with `/cdn-cgi/` dropped;
  the sentences this plan and the tasks' reports wrote about themselves TRIED (the top yield every cycle); the
  fences. Report: `$L/final-review.md`.
- [ ] **B.2** ONE fix wave, each fix a NEW commit ("as a new commit" in every dispatch), Opus implementers, disjoint
  files if parallel; a fix that changes what a browser shows owes the e2e. A triage line per finding.
- [ ] **B.3** The scoped re-reviews of the wave — an Opus one and a Fable one, each in its own export. Their head is
  **the code head** for everything below.

**C. The mutation battery**, at the code head, an Opus agent on an EXPORT: each row applies the mutant, sees the named
catcher fail BY NAME, reverts, records the diff. A catcher with no row gets one; a row whose catcher does not fail is
a FINDING. Report: `$L/battery.md`.

| # | the mutant | the catcher |
|---|---|---|
| 1 | `RECHECK_MS` never fires | Task 1's interval cell |
| 2 | the door auto-applies off the door | Task 1's `isDoor` cell |
| 3 | the auto-apply fires twice in one page life | Task 1's once cell |
| 4 | `staleTab` never set on a foreign `controllerchange` | Task 1's two-hook cell (Review Focus 1) |
| 5 | the auto-apply ignores `applyBusy()` | Task 1's busy cell |
| 6 | the guard accepts `cache.enabled: true` under an `exports` name | Task 1's check-10 cell |
| 7 | `isNewer` uses `>=` | Task 2's equal-version cell |
| 8 | the feed fetched with `releaseFeed()` null | Task 2's never-fetched cell |
| 9 | a stored answer under an hour old not drawn | Task 2's stored-answer cell |
| 10 | `runs_here` ignores a drift warning | Task 3's weights-drift cell |
| 11 | `runs_here` compares to production again | Task 3's genuine `lookahead-v1` cell (Review Focus 2) |
| 12 | `runs_here` admits an ineligible named identity | Task 3's `v2-padded` cell (Review Focus 2) |
| 13 | the CLI's tail cut from the wrong sentence | Task 3's pin cell |
| 14 | the sweep deletes before listing | Task 4's order cell (Review Focus 3) |
| 15 | a TTL without an archive starts | Task 4's refusal cell (Review Focus 3) |
| 16 | `open` waits the table TTL | Task 4's `Waited` cell |
| 17 | a `last_written` of `Ok(None)` ended | Task 4's unknown-activity cell |
| 18 | `404.html` dropped from the tree | Task 5a's check-11 cell |
| 19 | `on` accepted with `turnstile_sitekey: false` | Task 5a's check-12 cell |
| 20 | `404.html` precached | Task 5a's exclusion cell |
| 21 | `health` deleted from the destinations again | Task 5a's access-paths cell |
| 22 | `robots.txt` allows `/api/` | Task 5b's presence cell |
| 23 | the JSON-LD names the repository with no link set | Task 5b's metadata cell |
| 24 | the fetch handler retries without bound | Task 6's two-challenges cell (Review Focus 4) |
| 25 | the stream reconnects with no `call` after a challenge | Task 6's stream cell (Review Focus 4) |
| 26 | the loader called at mount | Task 6's button cell |
| 27 | the non-interactive drain pops the widget | Task 6's outbox cell |
| 28 | a challenged health read marks nothing | Task 6's health-read cell |
| 29 | the nameless visitor's game posted | Task 6's `signed-out` cell |
| 30 | `/cdn-cgi/` dropped from the allow-list | Task 7's expression cell |
| 31 | a top-level directory added to the tree | Task 7's drift cell |
| 32 | the smoke passes `on` over a 200 | Task 7's smoke cell |
| 33 | the door makes one `/api/health` call | Task 8's fence cell (Review Focus 5) |
| 34 | a route missing from §16 | Task 8's ledger cell |

**D. The per-edit set and the e2e at the code head**, in the foreground or polled: every Rust suite, the `identity::`
cells, the `wasm32` check, clippy, both rustdocs, the four bridge node suites, the Worker's four commands (the debug
module first), the client's four; `npm run e2e` last. Every reading against Task 1 Step 0's baseline and the tasks'
readings; a red cell re-runs ALONE before it is believed. The readings go to `$L/code-head-readings.md`.

---

### Task 11: The live readings against F's release — the deploys, the update readings, the NAS sweep, the tally, then the public window and the revert — ORCHESTRATOR (R-F7; spec §3.3, §2.5, §2.6; R-FP4, R-FP10)

No code. Every step is the orchestrator's, at discretion (R-F7), over WARP as the owner where a signed-in reading is
needed and through the credential where a claimed identity is (`worker/RUNBOOK.md` §4's header file, mode 600,
deleted after). ONE UTC day (R-FP10). The transcript of every account step is `$L/live.md` and `$L/window.md`; every
count from the Workers Observability query API is a `calculations` `count` over a named minute window
(`worker/RUNBOOK.md` §7; every ad-hoc body needs a `queryId`); a fresh `wrangler tail --format json` per live run,
its `.err` read for "Reconnecting" before a count is believed.

- [ ] **A. Pre-flight.** `curl -A 'Mozilla/5.0' https://yokohama.moore.nyc/api/me` names the owner; `cd worker && npx
  wrangler whoami`; `df -h`; **the quota guard's first reading** (HOLD above 80,000). **The widget confirmed, metadata
  only**: `WRANGLER_WRITE_LOGS=false WRANGLER_LOG=log WRANGLER_LOG_SANITIZE=true npx wrangler turnstile widget get
  "$YOKOHAMA_TURNSTILE_SITEKEY" --json | jq '{mode, domains, clearance_level, bot_fight_mode}'` (the key from the
  environment, never typed into the transcript; the secret field NEVER selected) — mode `managed`, the Worker's
  hostname among the domains, pre-clearance at `managed`; anything else is set right in the dashboard before G. **The
  zone's settings read** (read-only, through the API): Bot Fight Mode, Challenge Passage (recorded; the end state's
  value is the owner's ruling), managed `robots.txt`. **Prepare the pinned browsers BEFORE any deploy**: `node
  web/scripts/pinned-browsers.mjs worker https://yokohama.moore.nyc web/scripts/pinned/worker` and the same for the NAS,
  each at E's release until "Ready to play offline"; in the Worker's context open a SECOND TAB with a local game a few
  moves in (Review Focus 1). They stay open.
- [ ] **B. The production release build, ONCE, WITHOUT the site key** (R-FP4): `cd web && YOKOHAMA_PROJECT_URL='https://github.com/jmoore2333/yokohama-assistant/'
  YOKOHAMA_SITE_NOTICE='Not affiliated with Asmodee' YOKOHAMA_SITE_BLURB='<the blurb both hosts carry>'
  YOKOHAMA_SITE_URL='https://yokohama.moore.nyc' npm run build` (the feed derived; `grep -c debug_trap …` 0; the
  precache's lines recorded; `dist/build-values.json` read: `turnstile_sitekey: false`, `site_url: true`,
  `release_feed: true`), then `cd ../worker && npm run build`. **The NAS build is its own** (`YOKOHAMA_SITE_URL='https://yokohama-nas.moore.nyc'`,
  no key) through E's `nas-build.sh` with the tag `yokohama-web:0.6.0-m6.5c2f`.
- [ ] **C. The Worker's deployment** (`npm run deploy`, the tail listening): the guard's door line, its check-10, -11 and
  -12 lines, the assets uploaded (`404.html`, `robots.txt`, `llms.txt`, `sitemap.xml`, `build-values.json` among them),
  the `vars` with `YOKOHAMA_TURNSTILE ("off")`, the secrets inherited; the smoke reads no edge line under `off`. §5's
  readings: `/api/health`, `/api/me`, `/sw.js` naming F's release, `/robots.txt` 200 `text/plain` `max-age=3600` — the
  SERVED body compared to the committed one (managed robots.txt prepends if on), `/llms.txt`, `/sitemap.xml`,
  `/404.html` 200 with `noindex`; `/wp-login.php` under Access meets the login (the 404 and the probe are the window's).
- [ ] **D. THE UPDATE READINGS on both hosts** (Review Focus 1): in each pinned context, ONE plain reload of the door →
  the new `sw.js`, the install from `/shell.bin`, the new worker waiting, then the DOOR RELOADING ITSELF ONCE (the
  instrument's `after-1.json` shows the reload with no `/click` file) and "Ready to play offline" under F's release,
  the old generation retained; **the second TAB** (the local game): its line appears (stale), the game unchanged, no
  reload; its click → the same game continues under F. A door that reloads twice, or a game tab with no line, is a
  STOP. The NAS's reading after E: the same.
- [ ] **E. The NAS's sweep reading, then its seventeenth deployment.** `$L/nas-sweep.md`: a copy of the volume (the
  Portainer archive endpoint) into a scratch volume; **`stat` three table documents in the copy against the originals
  — the modification times must match, or the copy resets them and the reading is vacuous (F-24)**; a scratch
  container from the new image with `YOKOHAMA_WEB_TABLE_TTL_DAYS=1`, `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS=1`,
  `YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive`, `YOKOHAMA_WEB_IDENTITY=open`, bound to loopback on the NAS only; after its
  first pass (one minute) the log's `sweep-listed` lines BEFORE its `sweep-ended` lines, the archive directory's new
  files, `/api/health` reading `1` and `1`; **a pass that lists nothing is a STOP**; the container and the scratch
  volume removed BY NAME. Then the swap (E's `nas-swap.sh` with the tag and the two TTLs at the Worker's values, 90 and
  7 — a `Ruling:` line if the owner names others): the readings (`/api/me` names the owner — a STOP otherwise;
  `/api/health` with the TTLs; `/robots.txt`; `/wp-login.php` 404 with the page); the first real pass's listing
  recorded verbatim.
- [ ] **F. The live tiers under Access with the dev door** (the quota guard first): the live replay TWICE, sequentially;
  a FRESH tail, then `YOKOHAMA_WEB_TALLY=$L/tally.jsonl YOKOHAMA_WEB_E2E_TARGET=live npx playwright test two_browsers
  joins_by_link`; `node web/scripts/tally-report.mjs $L/tally.jsonl` → the per-route figures for the ledger; the
  query API's requests and rows for the window.
- [ ] **G. THE PUBLIC WINDOW** (R-F7; spec §2.6; E's flip recipe extended), in THIS order, each step a dated line:
  1. the quota guard;
  2. **a client build WITH the site key** (`YOKOHAMA_TURNSTILE_SITEKEY` from the shell's environment, the other values
     as B) and `cd worker && npm run build`; `build-values.json` reads `turnstile_sitekey: true`;
  3. the operator secret DELETED (the `open` shell refuses to start beside it); `npm run deploy:open` — the guard's door
     line and its check-12 line (`on` with a keyed build), the version; the smoke's edge line is EXPECTED TO FAIL here
     (no rule yet) and is read again after step 5;
  4. **the WAF rule**: `node scripts/edge-rule.mjs > $L/edge-rule.json`; the rule created through the API in the
     zone's `http_request_firewall_custom` phase from that JSON, ENABLED, its id recorded; the zone's rule read back and
     `node scripts/edge-rule.mjs --verify < rule-from-zone.json` exits 0;
  5. the Worker's hostname taken off the Access application's destinations (the three PUTs' discipline: the `aud` read
     back unchanged); **the smoke's edge line now PASSES** (a cookie-less `GET /api/health` → 403 `cf-mitigated:
     challenge`); WARP off on the owner's machine for the owner's readings;
  6. the readings, each in its own named minute so the query API can slice it:
     - **G.a the probe burst** (Review Focus 5): `node worker/scripts/probe-burst.mjs https://yokohama.moore.nyc --count
       200 --pace 500 > $L/probe/burst.tsv`; every request in all three shapes answered at the edge (`cf-mitigated:
       challenge`, 403) — the navigation shape's answer recorded too, since the 404 page is what a browser would then
       get; the query API's stateless count for that minute → **zero**; then the control: one `/api/health` from a
       cleared headed browser → the count for its minute reads one;
     - **G.b the edge**: `curl -sD - -o /dev/null https://yokohama.moore.nyc/enter`, `…/api/health`, `-X POST
       …/api/archive`, `-I …/api/health` (HEAD), `-X OPTIONS …/api/games` → each 403 with `cf-mitigated: challenge`,
       zero invocations in that minute;
     - **G.c the owner's path** (headed Chromium on the workstation, no WARP; the owner's own Edge with WARP off as the
       second reading): the door → "Go to the lobby" → the widget → the 302 → the lobby's name step → a table → a game
       of a few moves; **the WebSocket upgrade read as a 101 under the clearance**; the query API shows the invocations
       of that game and nothing before the widget;
     - **G.d the link arrival**: a fresh headed context opening the table's link → the overlay once → the table;
     - **G.e the upload**: a fresh headed context, the lobby's name step FIRST (a nameless visitor's upload is refused
       by the layer, F-4/F-14), then a local bot game to game over → the overlay → `archive-landed` in the tail;
     - **G.f the outbox**: a fresh headed context, a name chosen, a local game finished OFFLINE (`context.setOffline(true)`),
       the network restored → the `online` drain's health read is challenged → the row reads "waiting for a human
       check" on the door → a reload (the door's launch) → the overlay → landed;
     - **G.g the update reading under the open door** in a fresh pinned context (static, no login): as D.
  7. **the revert**, in THIS order: the three hostnames back on the Access application FIRST, the `aud` unchanged; THEN
     the rule DISABLED by its id (kept for the public end state); a client build WITHOUT the key and `npm run build`;
     `npm run deploy` (the worker door, `off`, the dev door on); `YOKOHAMA_WEB_ADMINS` put back from `git config
     user.email` through stdin; the after-check (`/api/me` over WARP names the owner; a clean headed browser without
     WARP meets Access's login; `wrangler secret list` names the two secrets); ONE live replay HOLDS; the quota after.
- [ ] **H. The figures** for Tasks 12–14: the ledger's per-route counts (F), the probe's zero and its control, the
  window's minutes, the rule's id, the NAS's listing, the update readings' sentences, the zone settings read in A.

---

### Task 12: The lane document, `docs/web-lane.md` — Opus (spec §2.9; R-F5, R-F6, R-F10)

- [ ] §1's state paragraph (F's release on both hosts; the window vetted and reverted; the rule disabled by id); §2's
  counts at F's code head (Task 10.D's readings, named); §5's traps gain: a challenge page cannot be solved by a fetch
  (the `cf-mitigated` predicate at every site); a non-navigation miss invokes the Worker whatever the 404 page says
  (the edge rule is the fence); one build per door state (the guard's check 12); a volume copy that resets file times
  makes a sweep reading vacuous; §8's preconditions table: the edge rule and pre-clearance as the no-Access shape with
  the window's readings, `/api/health` closed, rows 108 and 119 still owed, the Challenge Passage value as the
  owner's; §10's operations: the window's recipe (Task 11.G's seven steps with the rule's id and `edge-rule.mjs`), the
  probe and pinned-browser instruments, the NAS sweep reading; §11 F's row; §12's variables: the four new names with
  their kinds and readers (`YOKOHAMA_RELEASE_FEED`, `YOKOHAMA_SITE_URL`, `YOKOHAMA_TURNSTILE_SITEKEY`,
  `YOKOHAMA_TURNSTILE`), the two TTL rows' host column; §13's map unchanged (`/enter` did not change); §15 by design:
  no NAS marker, the TTL-without-archive refusal, the sweep-versus-return race, the in-game click line, the two-tab
  stale line, one build per door state, the feed's silence while the repository is private, **the V5 ship's one CLI
  line** (Task 3's sentence), no siteverify; **§16 the ledger completed** with Part 2's figures and the harness caveat.
  The two drift cells green. Commit: `docs(web-lane): cycle F — …`.

---

### Task 13: `CLAUDE.md`, the board, guidingpath, the AI lane's re-entry line — Opus (spec §2.9; R-F4)

- [ ] `CLAUDE.md`: the counts in place naming F's reading; in the web lane's fence sentence a fourth clause, "the door
  unmetered (a door visit with nothing owed, a local game and, under the no-Access door's edge rule, any miss cost zero
  Worker invocations; the first metered request is `/enter`, a link arrival's first read or a finished local game's
  upload)"; in "Two facts cross into the AI lane" a third, Task 3's sentence: "a game opens under any build whose CLI
  resume reconstructs its exact recorded bot; a V5 build plays V4 games on the web exactly when the V5 ship admits
  `v4-step9-draft1` in the CLI's `eligible` (R-F4)"; the pointer to F's close beside E's. `docs/handover.md`: the
  Updated line; Track 3 at F (the merge and the tag on the owner's word); the entry-artifacts line; **Track 1 gains the
  V5 line as an owed item**. `guidingpath.md`: F's roadmap row and the decision-log rows (R-F4 amending R-6.5c2.10
  forward with its identity-freeze reason and the V5 requirement; the door-unmetered fence with the edge rule; the
  version stays 0.6.0). `docs/plans/2026-09-11-ai-lane-reentry-handover.md`: one line under its open items, Task 3's
  sentence, plus "a NEW `Contestant` or `NamedIdentity` variant still needs its one-line label in `bots.rs`".
  Commit, documents only.

---

### Task 14: The README, the self-host guide with `deploy/update.sh`, both runbooks' records — Sonnet 5.5 (spec §2.9; R-F2, R-F9, R-F12)

- [ ] `README.md`: the getting-started and defaults from the lane document's §14; one sentence on the resume rule (Task
  3's); the web presence (what a public deployment gets for free); the self-host update pointer; the release line left
  conditional for Task 15.G. `deploy/README.md`: §3's build arguments table with the five values (the feed's three
  states; the site URL's effect; the site key's "leave it blank behind Access — the widget is the no-Access Worker's");
  §4's rows as Task 4 left them; a new "§9 Updating" — fetch the tag, rebuild, smoke, swap, what players then see (the
  door updates itself; a game in play shows the line) — and `deploy/update.sh`: `set -euo pipefail`; `TAG=${1:?a
  release tag}`; `git fetch --tags && git checkout "$TAG"`; `docker build --platform linux/amd64 -f deploy/Dockerfile`
  with the `--build-arg`s read from the env file the operator names in `YOKOHAMA_WEB_BUILD_ENV`, `-t "yokohama-web:$TAG"
  .`; `bash deploy/smoke-image.sh "yokohama-web:$TAG"`; then `docker stop`/`rm`/`run` with `--env-file
  "$YOKOHAMA_WEB_ENV"` and the volume; a `--dry-run` that prints the commands; never `rm -rf`. `deploy/RUNBOOK.md` §3.20
  (the seventeenth deployment from Task 11.E's transcript) and `worker/RUNBOOK.md` §20 (F's deployments, the window
  with the rule's recipe and id, the probe's figures, the zone settings read) and its §3 row if Task 7 left any;
  `worker/README.md`'s table. Commit, documents only.

---

### Task 15: The close, F's PR, and the wrap — ORCHESTRATOR (spec §4; R-F7, R-F13; R-FP11)

- [ ] **A. The ONE recorded sequence at the FINAL reviewed head** (R-FP11): Tasks 12–14 reviewed and landed; E's
  `close-sequence.sh` copied into `$L` with `L=` set and the e2e per-spec row added; ONE uninterrupted run under
  `caffeinate -i`, machine-tagged, polled with `tail -n 3 "$L/sequence.tsv"`; every `rc` 0; the fifteen gates' walls
  summed beside E's 2,881 s; a red row re-run ALONE before it is believed; each count set against Task 10.D's readings.
- [ ] **B. The close document** `docs/plans/<the day>-web-frontend-m6.5c2f-close.md` on E's shape: the header; the
  headline and what is LIVE; §1 by task with the fences; §2 the sequence; §3 the deployments, the update readings, the
  NAS sweep, the tally, the window (the probe in three shapes with its control, the edge readings, the six browser
  readings with the 101 line, the revert's after-check), the quota, the zone settings; §4 the reviews and the battery;
  §5 every ruling R-F1–R-F13, R-FP1–R-FP15 and every ledger `Ruling:` with its cost; §6 what the lane leaves; **§7 what
  this cycle does NOT claim** — at least: the fully open end state (vetted, reverted); the real widget outside the
  window; the feed's notice live; a public load; the handsets; rows 108 and 119; the V5 ship's CLI line (owed, not
  done); a cookie-keyed rate limit on Free (unread); §8 the wrap. **Then** the spec's `§10.14 Corrections recorded in
  execution — cycle F`; the guidingpath row's execution half; `CLAUDE.md`'s counts if the final head moved them; the
  board's Updated line; the lane document's §11 row's close pointer. Commit, documents only.
- [ ] **C. The memory**: `yokohama-web-lane-status` rewritten to the close; `project-web-lane-wrap-then-ai` gains the F
  update; the index lines.
- [ ] **D. The push and F's PR** (R-F7): `git push`; `gh pr create --base web-room-m6.5c2e --head web-room-m6.5c2f` with
  the cycle in one screen and the attribution lines. No checks run; `gh workflow list --all` in the ledger as the
  reading.
- [ ] **E. The AI lane's re-entry**: the board's Track 1 pointer and the re-entry handover are the entry; the close's §8
  says so.
- [ ] **F. STOP and ask the owner**: the merge and the tag now, or as a follow-on session (R-F7).
- [ ] **G. THE WRAP, on the owner's word** (E's recipe, F's PR added): the merge #31 → #43 → F's PR, bottom-up, one at
  a time, each read before the next: for #31, `gh pr merge 31 --merge`; then for each later PR in order, `gh pr edit
  <n> --base master`, `gh pr view <n> --json commits` to see ONLY that cycle's commits, then `gh pr merge <n> --merge`;
  the workflows stay disabled and nothing is dispatched; after F's PR, `git checkout master && git pull` and the
  fences read against `master`. Then the tag: `git tag -a v0.6.0 -m "Yokohama CLI v0.6.0 — the web lane finished: two
  hosts, the cleanup, the update path, the clean finish"`, `git push origin v0.6.0`; `release.yml` read to green and
  the release's three assets listed. Then `gh workflow enable test.yml` and `gh workflow enable docker.yml`. Then
  the memory (C above, amended: merged and tagged), the board's Updated line, the README's release line, `CLAUDE.md`'s
  pointer, committed on master as one documents commit. The refocus on the computer player is the next session's,
  from the re-entry handover.

---

## Appendix — the ledger

`.superpowers/sdd/2026-09-30-web-frontend-m6.5c2f-plan/` (gitignored): `progress.md` (the SDD identity line, the base
`15de8117`, Step 0's baseline, R-FP1–R-FP15, one line per task, one `Ruling:` per deviation); `task-<n>-brief.md`,
`-report.md`, `-review.md`; `task-<n>-e2e.txt`; `task-1-look/`, `task-2-look/`, `task-6-look/` (the pictures at 1280
and 390 px); `task-9-e2e-walls.txt`; `final-review.md`, `fix-wave-*.md`, `battery.md`, `code-head-readings.md`;
`live.md`, `window.md`, `nas-sweep.md`, `probe/`, `edge-rule.json`, `tally.jsonl`; `nas-build.sh`, `nas-swap.sh`,
`worker-deploy.sh`, `close-sequence.sh`, `sequence.tsv`, `sequence/NN.log`; `pr-body.md`. E's ledger (`…-m6.5c2e-plan/`)
holds the scripts' originals and the pinned instrument's origin. The adversarial review's report and its probes are in
the planning session's scratch (`f-review/`), copied into this ledger at Task 1 Step 0.

---

## Review record

- **The self-review at writing (2026-09-30, the orchestrator):** spec coverage — §2.1 Task 1, §2.2 Task 2, §2.3 Task 3,
  §2.4 Task 4, §2.5 Tasks 5a and 8 (the probe in 11.G), §2.6 Tasks 6, 7 and 11.G, §2.7 Task 5b, §2.8 Task 9, §2.9 Tasks
  12–14, §4's wrap Task 15; placeholders none; the names table is the type contract and every task cites it; Review
  Focus 1–5 each pinned to a cell in its owning task.
- **The adversarial Opus review (2026-09-30, over `a63b48e1`, the plan's first commit; the report `f-review/report.md`;
  32 findings — 1 BLOCKER, 6 HIGH, 16 MEDIUM, 9 LOW — three confirmed by probes run in an export: a vitest probe over
  `useReadiness` with two hooks, the platform scan over a mock `sweep.rs`, a cargo probe over the CLI's
  `from_provenance`) and the ONE fix wave, all in this revision:**
  - **F-1 (BLOCKER)** — the CLI's `eligible` admits only the production named identity, so the rule as first written
    reproduced the V5 cutoff, and the flagship cell proved a different branch → Task 3 rewritten: the predicate calls
    `from_provenance` (R-FP12), no CLI edit, the V5 requirement stated in the report and carried to Task 13; Review
    Focus 2 rewritten.
  - **F-2, F-20, F-21 (HIGH, MEDIUM, MEDIUM)** — the 404 page does not stop a non-navigation miss (Cloudflare's routing
    diagram); the fence cell's `/nope` clause contradicted axum's extension-less fallback; the probe could not tell the
    router's answer from Bot Fight Mode's → the edge rule is the fence (Task 7, R-FP7); the 404 page a courtesy (5a);
    `/wp-login.php` in the fence cell; three probe shapes and the zone's settings read (8, 11).
  - **F-3, F-4, F-22, F-26 (HIGH, HIGH, MEDIUM, LOW)** — the Worker half protected nothing, made dead ends of the five
    other `/enter` links, had no success-path cell, and the window's order left the API briefly open → no siteverify,
    no secret, `/enter` unchanged; `YOKOHAMA_TURNSTILE` a declaration held to the stamp and to the smoke (7); the
    window's order corrected (R-FP10, 11.G).
  - **F-5, F-6, F-8, F-9 (HIGH, HIGH, MEDIUM, MEDIUM)** — drift compared on the version alone; the mongrel fixture; the
    self-contradicting classification; the corpus already the V5 shape → all inside R-FP12's predicate (the CLI's drift
    warnings compare every param); the genuine record as the opens case; the corpus regenerated for the sentence only.
  - **F-7, F-28 (HIGH, LOW)** — the door's apply strips the game tab's line; two unnamed edges → R-FP14 (`staleTab`, the
    busy flag); Review Focus 1 rewritten; the door cell renamed.
  - **F-10, F-11, F-24, F-25 (MEDIUM, MEDIUM, LOW, LOW)** — twenty test doubles; the platform scan's layout; a vacuous
    volume copy; the sweep-versus-return race → R-FP6's default body and `from_lookup`; the closed region; the `stat`
    and the STOP in 11.E; the race sentence in the lane document.
  - **F-12, F-13 (MEDIUM)** — the notice in a block a signed-in lobby never draws; the flickering notice → R-FP9; the
    stored answer.
  - **F-14, F-15, F-16 (MEDIUM)** — the nameless visitor's two invocations; the challenged health read; three fetch sites
    and a socket → Task 6's outbox rules and R-FP8's four sites; Review Focus 4 rewritten.
  - **F-17, F-18, F-19, F-29, F-30 (MEDIUM, MEDIUM, MEDIUM, LOW, LOW)** — the variables drift cell; the checks' home; Task
    5's size and the stamp's sixth key; check 10's `exports` walk and the assembler; the ledger's parser → the §12 rule
    in Global Constraints; R-FP13; R-FP15 (5a/5b) with the site key's value in Task 2; every `exports` name; no
    assembler edit; `map_rows` parameterised.
  - **F-23, F-27, F-31 (MEDIUM, LOW, LOW)** — Challenge Passage; the three logging variables; the fence's wording and
    three residuals → 11.A reads the zone's settings; the pre-flight's variables; "a door visit with nothing owed" and
    the residuals in the spec's §2.6.
  - **F-32 (LOW)** — anchors and sentences off at HEAD → corrected in the names table and the tasks.
  - **Declined:** none. **Left as readings owed** (the reviewer's PLAUSIBLE items): the array form of `run_worker_first`
    and navigation misses (11.G.a reads it); the WebSocket under a challenge (11.G.c's 101 line); a cookie-keyed rate
    limit on Free (the close's §7).
- **The owner's review:** pending.
