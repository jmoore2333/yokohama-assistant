# M6.5c2 cycle F — the clean finish: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan
> task-by-task. The execution shape is §"Execution shape" below: every code and document task subagent-driven, the
> implementer's tier marked in each task's title (**Sonnet 5.5** for the implementation-focused tasks whose brief
> carries the judgment already made, **Opus** where judgment is needed), every task reviewer Opus, Fable for the
> whole-branch review; every account step — a deploy, a live run, an Access change, a zone rule — the ORCHESTRATOR's
> alone, at discretion (R-F7). Steps use checkbox (`- [ ]`) syntax. **Read the spec first**
> (`docs/plans/2026-09-30-web-frontend-m6.5c2f-spec.md`: §1 the rulings R-F1–R-F13, §2 the mechanism, §5 the fences,
> §6 what F does not do), then this plan's §"Names shared across tasks" and §"Couplings between tasks" — an
> implementer sees only its own task, and those two sections are how it learns what its neighbours produce. Every
> `file:line` here was read at `0d9dc90a` (the spec's commit on `web-room-m6.5c2f`); **find by content once an earlier
> task has moved a file.**

**Goal:** The web lane closed on the owner's goal of a deployment that will one day run on Cloudflare with no Access
barrier, on the Workers Free plan, with bots and scanners kept off the Worker: the door's no-click update path and a
release notice that ask the server nothing; the resume rule so a new bot version cuts off no game it can still play;
the self-host's table expiry; the door unmetered as a fence with a static 404 page, a guard, a probe and a ledger of
every metered action; Turnstile with the edge rule behind a switch, proven in a public window and reverted; the web
presence; E's minors; the documents; the close; then, on the owner's word, the merge #31 → #43 → F's PR and
`v0.6.0`.

**Architecture:** Three parts in order. **Part 1, the code** (Tasks 1–9), lands ONE release: the update path first
(1), since every later browser reading depends on a release that updates itself; the release notice (2) and the resume
rule (3) next, each self-contained; the sweep (4), the only server-side task; the 404 page with the guard's new checks,
the presence files and the protected-path change (5), which changes the deploy configuration every later task's
deploy reads; Turnstile's client half (6) then its Worker half (7), the switch's three names fixed in the names table
so 5's guard check and 7's config read agree; the instruments and the ledger (8), which read the client and the router
as 1–7 leave them; E's minors (9) last, small and disjoint; then the review, one fix wave, the battery and the ONE
recorded sequence (10). **Part 2, the live readings** (Task 11, the orchestrator's): both deploys, the update reading on
both hosts, the NAS sweep, then the public window LAST because it takes Access and the dev door down. **Part 3, the
documents** (Tasks 12–14), written from Part 2's readings, then the close and the wrap (15).

**Tech Stack:** Rust 1.97.1 (`crates/yokohama-web`, `crates/yokohama-worker`, `crates/yokohama-wasm`; tokio for the
sweep's interval, behind the `server` feature); TypeScript in `web/` (React 18, Vite, vitest over jsdom, Playwright over
Chromium) and `worker/` (the shell; vitest node cells; the POOL inside workerd); the owner's Cloudflare account through
the Cloudflare MCP (Access applications, WAF custom rules, the Observability query API, GraphQL) and `wrangler` 4.136.3
(deploys, `secret put`, `turnstile widget get`); the NAS through Portainer's Docker proxy; Cloudflare Turnstile
(`challenges.cloudflare.com/turnstile/v0/api.js`, siteverify, the published always-passing test pair).

**Spec:** `docs/plans/2026-09-30-web-frontend-m6.5c2f-spec.md` at `0d9dc90a`, whose §1 rulings are restated in the
M6.5c2 spec's §10.13. **How this plan was built:** by the orchestrator alone from the spec and the tree at `0d9dc90a`,
under D's stop rule — a task past ~250 lines is two tasks, a plan past what one agent reads whole is split before it is
finished (R-F11) — then reviewed adversarially by Opus against the spec, the plan and the approach, with one fix wave,
before the owner's final pass (§"Review record").

---

## Global Constraints

- **Branch `web-room-m6.5c2f`**, stacked on `web-room-m6.5c2e`; **the base for every code diff is `15de8117`** (E's
  branch head: the code head `c39da5a1`, E's documents commits on top). The branch's first commit is the spec
  (`0d9dc90a`); this plan is its second. Stage with explicit paths, never `git add -A`; `git status` is clean after
  every commit.
- **The push, F's PR, the deploys, the live tiers, the public window and its Access and zone changes are the
  orchestrator's at discretion** (R-F7). **The merge and the tag are NOT**: Task 15.H asks the owner at the wrap.
- **Fences** (spec §5). At every commit this prints nothing:
  `git diff --stat 15de8117 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli`.
  `SEMANTICS_REV` stays **7**. No bot behaviour moves: Task 3 changes which recorded bot the LAYER seats, never what
  a bot does. **The door unmetered is a fence of its own**: no task adds a request the door, the local game or an
  unknown path makes to the Worker; Task 8's cell and Task 11's readings are its proof.
- **The corpus moves in NAMED tasks only:** Task 3 (the foreign fixtures re-derived from a genuinely foreign identity;
  every foreign sample), Task 4 only if `health.json` changes (it should not: the corpus host sets no TTL — the task
  says so), Task 7 (`/enter`'s new refusal samples, if the corpus carries `/enter` at all — read it first). Each
  regenerates with `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, READS the diff, and names
  in its report which files moved and why; `git diff --stat 15de8117 -- web/src/wire web/src/wire-script` at the close
  lists only those. **The corpus's seed pin, password pins and watcher pin stay green at every commit.**
- **The per-edit gates, after EVERY task** (`CLAUDE.md`'s Commands; the readings the close re-counts; the baseline is
  Task 1 Step 0's): `cargo test -p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`, the eight `identity::`
  cells, the `wasm32` check, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo doc -p yokohama-web
  --no-deps` and `-p yokohama-worker --no-deps` at 0 warnings; in `web/`: `npm run build:wasm` then `npm run
  typecheck`, `npm test`, `npm run test:wasm`; in `worker/`: `npm run build:module:debug` FIRST, then `npm run
  typecheck`, `npm run test:scripts`, `npm test` (the pool). **`npm run e2e` is owed by every task that changes what a
  browser does or shows** — Tasks 1, 2, 3, 5, 6, 8 and 9 — run ONE at a time, in the foreground or polled, never under
  a Monitor; a red cell re-runs ALONE before it is believed.
- **Every invisible non-ASCII code point in code is a `\u` escape.** No document spells the owner's address: readings
  say "the owner's address". **No secret value is printed, copied or committed**: the credential in `.dev.vars`,
  `DEV_DOOR_CLIENT_ID`, `YOKOHAMA_WEB_ADMINS`, `TURNSTILE_SECRET` (a line of that NAME sits in the repository root's
  `.dev.vars` — inert for the Worker, never read by any task), the Portainer token, any cookie. **The Turnstile site
  key is public but stays out of every document**: a build passes it through the environment, and the ledger records
  "the site key" by name.
- **Sonnet 5.5 implements a task whose brief carries the judgment already made** (Tasks 2, 5, 9, 14); a Sonnet
  implementer that meets a design choice STOPS and asks (NEEDS_CONTEXT to the orchestrator). **Opus implements Tasks 1,
  3, 4, 6, 7, 8, 12, 13.** Every reviewer is Opus, mutants on an EXPORT (`git archive`), never in the implementer's
  tree. Fable reviews the whole branch (Task 10).
- **Reports to files**, never the notification: `$L/task-<n>-report.md`, three lines in the reply. **A fix is a NEW
  commit** — once a SHA is REPORTED, every change is a new commit. **Nothing runs in an implementer's tree** while it
  works, not even read-only git. **No Monitor, no event waits**: a long run is polled by reading its output file; the
  orchestrator checks the ledger within ~15 minutes of any long run. **No `rm -rf`, `rm -r` or bulk `rm -f` in any
  dispatch**: exports and screenshot trees stay in place, named in the report.
- **The wire corpus's rule for the client tasks that render** (1, 2, 6): every payload a jsdom cell renders comes from
  `web/src/wire/`; nothing is hand-authored, and the fake Worker answers from the corpus. A FEED fixture (Task 2) is
  not a wire payload: it is the feed host's shape, held beside the cell.

---

## Review Focus

The spec implies these five inputs, and no cell would meet them unless someone put it there. Each line says what a
person would expect; its test lives in the owning task and is marked **(Review Focus N)** there.

1. **A player with the door open in one tab and a game in another when a release lands.** Expected: the door tab
   reloads itself once into the new release; the game tab keeps playing on its old bundle, shows the one line, and
   nothing in it reloads until the click. (Task 1: the auto-apply cell that a second, non-door page is not reloaded,
   and the once-per-page-life cell; Task 11.D's reading with two tabs.)
2. **A V4 game left mid-play the day a V5 bot ships, and a V4 game whose knobs a hand-edit changed.** Expected: the
   first opens and V4 moves; the second refuses naming the knobs. (Task 3: the flipped `lookahead-v1` fixture and the
   knob-mismatch cell.)
3. **A self-host with a TTL set whose archive directory is unset, and one whose first sweep finds forty idle tables.**
   Expected: the first refuses to start naming both variables; the second logs forty lines before ending any, and
   every ended game with moves is in the archive. (Task 4: the start refusal cell and the listing-before-ending cell.)
4. **A visitor who arrives by a table link under the no-Access door with no clearance, and one whose clearance
   lapses mid-game.** Expected: the widget once, then the table; and the widget once at the next fetch, then play
   continues. (Task 6: the fetch handler's retry cell and the second-challenge cell.)
5. **A scanner asking the Worker for `/.env` and a browser navigating to `/nope`.** Expected: both get the 404 page,
   neither invokes the Worker, and a controlled browser's navigation still gets the shell. (Task 5: the axum 404 cell
   and the precache exclusion; Task 8's fence cell; Task 11.G's probe.)

---

## Execution shape (the owner's standing answers, and this plan's rulings)

- **Mode: SUBAGENT-DRIVEN, EVERY TASK.** Implementers as the Global Constraints say. **Every task reviewer Opus.**
  Fable: the whole-branch review (Task 10). A Sonnet brief carries the judgment already made: what to change, what
  proves it, what not to touch.
- Tests are written first and RUN RED; the red run is quoted in the commit body and the ledger. Every commit passes
  every per-edit gate.
- **The orchestrator edits no code.** Its steps: Task 1 Step 0's baseline; Task 10 and Task 11 whole; the pictures
  the notice and the line are judged from; Task 15 whole.
- **Rulings at planning (2026-09-30, the orchestrator, under the F spec):**
  - **R-FP1 — the numbering:** nine code tasks (1–9); 10 the review and the sequence; 11 the live readings; 12–14 the
    document tasks; 15 the close and the wrap. The base for every diff is `15de8117`.
  - **R-FP2 — the three build values and the switch keep the spec's names verbatim**: `YOKOHAMA_RELEASE_FEED`,
    `YOKOHAMA_SITE_URL`, `YOKOHAMA_TURNSTILE_SITEKEY` (client build values, defined as `__YOKOHAMA_RELEASE_FEED__`,
    `__YOKOHAMA_SITE_URL__`, `__YOKOHAMA_TURNSTILE_SITEKEY__` beside `__YOKOHAMA_VERSION__`); `YOKOHAMA_TURNSTILE`
    (a Worker var, `on` | `off`); `TURNSTILE_SECRET` (a Worker secret).
  - **R-FP3 — the build-values stamp**: the client build writes `web/dist/build-values.json`, a JSON object of
    BOOLEANS naming which build values were set (`{"project_url": true, "blurb": true, "notice": true,
    "release_feed": true, "site_url": false, "turnstile_sitekey": false}`), never a value; it is not precached, it
    travels into the Worker's assets tree, and the deploy guard's Turnstile consistency check reads it. It is the ONE
    way the guard can know what the bundle carries without knowing the key.
  - **R-FP4 — one build per door state** (spec §2.6): production (Access on) is built WITHOUT the site key and deployed
    with `YOKOHAMA_TURNSTILE: "off"`; the open configuration deploys a build WITH the key and `"on"`; the guard refuses
    the other two combinations. The NAS build carries no key.
  - **R-FP5 — the release feed's three states** are spelled by the checker: unset derives from a GitHub project link
    (`https://github.com/<owner>/<repo>` with an optional trailing slash → `https://api.github.com/repos/<owner>/<repo>/releases/latest`),
    a URL is that feed, the literal `off` is none; any other project link with the variable unset is none.
  - **R-FP6 — the sweep's activity read** is a new `Store` method `last_written(id) -> Result<Option<u64>, String>`
    (unix seconds, the latest of the three documents' modification times); `MemStore` stamps writes with a counter a
    cell can set (`stamp_for_test`). The decision is `sweep::ending`, pure; the task is `sweep::run`, `server`-gated.
  - **R-FP7 — siteverify has a seam**: `worker/src/turnstile.ts` exports `siteverify(secret, token, remoteIp, expectedHost, fetchImpl)`,
    unit-tested in node with a fake fetch; the pool exercises `/enter` under `off` and the `on`-without-secret 500;
    the real verification is read in the window (Task 11.H). No pool cell reaches `challenges.cloudflare.com`.
  - **R-FP8 — the edge challenge's handling lives at ONE site**: `api.ts`'s `call` retries a request once after the
    challenge handler resolves; the handler is a module-level seam `setChallengeHandler` the App wires to the overlay;
    the outbox and the button use the same seam.
  - **R-FP9 — the passive line's places** are the lobby's `lobby-actions` block and the game screen's `banner` header
    (`App.tsx:2118`); the release notice sits under the door's blurb and in the lobby's `lobby-door` block. Both are
    judged from a before-and-after picture in the task's report (R-6.5c2.38's rule); the orchestrator may move either
    with a `Ruling:` line.
  - **R-FP10 — the window runs on ONE UTC day after the deploys**, the quota guard before it and after it; the probe
    burst is 200 requests, well inside the rate-limit rule's window arithmetic (paced at two a second).
  - **R-FP11 — the sequence** is E's `close-sequence.sh` with `L=` set, one row added for `npm run e2e`'s per-spec
    walls (Task 9's minor), and runs ONCE at the FINAL reviewed head (Task 15.A), as E's R-E17 ruled and for the same
    reason.

---

## Names shared across tasks

An implementer sees only its own task. **These names are the contract between tasks and are used verbatim.** A task
that finds the code forces a different shape STOPS and asks the ORCHESTRATOR, who amends this table with a `Ruling:`
line.

### What the tree holds at the base, consumed as it IS (read off the tree, never off this table)

| name | where | tasks |
|---|---|---|
| `useReadiness`, `applyUpdate`, `readinessOf`, `SwHost`, `RegistrationLike`, `ReadinessStatus` | `web/src/swRegister.ts:29–380` | 1 |
| `Readiness` (the door's release line) | `web/src/OnThisDevice.tsx:223–280`; passed as `readiness={readiness}` from `App.tsx:1602` | 1, 2 |
| `useRoute`, `channelOf`, `parseRoute`, `Route` (`{ kind: "door" }` …) | `web/src/route.ts:21–140` | 1, 2 |
| the door's friends card (`<a className="primary-link" href="/enter">Go to the lobby</a>`), `multiplayerOffered()` | `web/src/FrontDoor.tsx:41–47, 142–156` | 6 |
| `call`, `settle`, `isAccessChallenge`, `ACCESS_CHALLENGE`, `onUnauthorized`, `ensureArchiveFact` | `web/src/api.ts:116, 262–275, 563, 636–660` | 6 |
| `drain`, `attempt`, `startOutbox`, `useOutbox`; `PendingReason` = `"offline" \| "signed-out" \| "unknown" \| "refused"` | `web/src/outbox.ts:443–470, 546–571`; `web/src/local/db.ts:87` | 6 |
| `BUILD_VALUES`, `checkProjectUrl`, `checkText`; the three `define`s | `web/scripts/build-values.mjs:19–64`; `web/vite.config.ts:38–45`; `web/src/vite-env.d.ts` (the three `declare const`s); `web/src/buildValues.ts` | 2, 5, 6 |
| the precache walk (`distFiles`, excluding `sw.js` and the shell copy) | `web/scripts/precache.mjs:95–108` | 5 |
| `deployGuard`, `configFailures`, `assetsBlock` (the assets keys and values), `SECRETS`, `secretsInVars`, `doorRules`, `siblingDrift`, `staleAssets` | `worker/scripts/deploy-guard.mjs:133–160, 215–240, 297–330, 371–395, 399–440, 441–490`; the cells `deploy-guard.test.mjs` | 1, 5, 7 |
| the assets blocks (`not_found_handling: "none"`, `run_worker_first: ["/api/*", "/enter"]`) | `worker/wrangler.jsonc:37–43`; `wrangler.test.jsonc:21–26`; `wrangler.open.test.jsonc:23–28`; `wrangler.open.jsonc:33–38`; `wrangler.gate.jsonc:38–43` | 5 |
| `Env`, `shellConfig`, `ShellConfig`, `onOff`, `refuse`, `openDoorRefusals`, `moduleConfig` | `worker/src/env.ts:40–70`; `worker/src/config.ts:37–51, 66–72, 189–192, 209–230, 250–300` | 7 |
| `route()`, `Target` (`enter`, `not-allowed`, `missing`, …), `handle()` (the `enter` 302 at `index.ts:114`; `bounded(req.body)` at :127) | `worker/src/route.ts:9–80`; `worker/src/index.ts:96–140` | 7 |
| `accessPaths`, `ROUTER_FILES`, `segments.delete("health")` | `worker/scripts/access-paths.mjs:1–60`; its cell `access-paths.test.mjs` (the nine destinations by hand) | 5 |
| `bots::foreign`, `bots::table_bot`, `bots::production`, `bots::label`, `bots::cannot_continue`, `bots::distinguishing_labels`, `BotDescriptor`, `TableBot { label, runs_here }` | `crates/yokohama-web/src/bots.rs:40–55, 70–87, 199–218, 293–318` | 3 |
| the foreign-bot rule's ONE site; the fixtures `foreign_pair(...)` | `crates/yokohama-web/src/games.rs:691–715, 1337–1420` | 3 |
| `explain::foreign_why`, `FOREIGN_BOT_KIND`, `Why::foreign` | `crates/yokohama-web/src/explain.rs:443–475` | 3 |
| `BotSeats::from_provenance`, `eligible` (`pub(crate)`), `identity_drift_warnings` | `crates/yokohama-cli/src/bot.rs:255–306, 377, 482–507` (the CLI is FENCED: read, never edited) | 3 |
| `Contestant::from_parts`, `Contestant::descriptor` | `crates/yokohama-core/src/game/arena/stats.rs:637, 817` (fenced) | 3 |
| `Store` (`list`, `read`, `write_if_version`, `delete`), `FsStore` (`path`, `read_locked`, `publish`), `MemStore` | `crates/yokohama-web/src/store.rs:121–142, 240–420, 436–505` | 4 |
| `Games::delete(id, &User)` / `delete_locked`, `Games::with_log`, `Games::with_clock`, `Games::archive_on`, the `Log` trait | `crates/yokohama-web/src/tables.rs:1875–1960`; `games.rs:485–518`; `log.rs` | 4 |
| `TableDoc { created_by, seats, status, turn, archived_at }`, `TableStatus::{Open, Dealing, Playing}`, `is_creator` | `crates/yokohama-web/src/table.rs:222–270, 417` | 4 |
| `Host { table_ttl_days, open_table_ttl_days, … }`, `Health`, `health()`; `AppState::new`, `with_multiplayer`, `app()`, `enter()`, `ENTER_PATH` | `crates/yokohama-web/src/api.rs:60–80, 360–392`; `lib.rs:351–470, 543, 895–935` | 4, 5 |
| `cache_control`, `serve_path`, `serve` (the SPA fallback for route-like paths) | `crates/yokohama-web/src/assets.rs:113–140` | 5 |
| `platform_calls_stay_behind_the_server_feature`, `platform::CALLS` | `crates/yokohama-web/src/lib.rs:7555`; `platform.rs` | 4 |
| the lane document's cells (`map_rows`, the three map cells) and the variables cell | `crates/yokohama-web/tests/lane_document.rs:69–170`; `worker/scripts/lane-variables.test.mjs:79–150` | 8, 12 |
| `TALLY_VAR`, the tally listener; `tally-report.mjs` | `web/e2e/harness.ts:913–940`; `web/scripts/tally-report.mjs` | 8 |
| `plays_locally.spec.ts`'s cells (the no-`/api/` game at :236; the API-refused first screen at :822; the foreign-bot "Will not open here" cell at :1205); `offline.spec.ts`'s second-release trick (`execFileSync("node", ["scripts/precache.mjs"] …)` at :821 and :1449) | `web/e2e/` | 1, 3, 8 |
| E's ledger instruments: `pinned-browsers.mjs`, `close-sequence.sh`, `nas-build.sh`, `nas-swap.sh`, `worker-deploy.sh`, `worker-readings.sh`, `state1-readings.mjs`, `state2-negative.mjs`, `after-revert-clean.mjs` | `.superpowers/sdd/2026-09-29-web-frontend-m6.5c2e-plan/` (gitignored) | 8, 11, 15 |

### What cycle F produces (the contract between its own tasks)

| name | shape | task |
|---|---|---|
| `RECHECK_MS = 15 * 60 * 1000` (`swRegister.ts`); `useReadiness` re-checking on `visibilitychange` and on the interval; `useDoorAutoApply(isDoor: boolean, status: ReadinessStatus): void` (once per page life; only with `updateWaiting`; only while `isDoor`); the passive line `<p className="muted update-line">A new version is ready. <button className="linkish">Reload to use it</button></p>` rendered by ONE component `UpdateLine({ status })` (`web/src/UpdateLine.tsx`) used by the door, the lobby and the game banner; the guard's check 10 `cacheBlock(name, value, failures)` | the update path | 1 |
| `web/src/releaseFeed.ts`: `defaultFeedFor(projectUrl: string \| null): string \| null`, `releaseFeedUrl(): string \| null` (the define), `parseVersion(s: string): [number, number, number] \| null`, `isNewer(tag: string, own: string): boolean`, `useReleaseNotice(fetchImpl?: typeof fetch): { tag: string; url: string } \| null` (once per page life; the hourly throttle under the storage key `yokohama.release.checked`); `web/src/ReleaseNotice.tsx` (`<p className="muted release-notice">A newer release is available: <a>v0.7.0</a>. This site runs 0.6.0</p>`); the build value `YOKOHAMA_RELEASE_FEED` in `BUILD_VALUES` with `checkReleaseFeed(raw, projectUrl)`; `__YOKOHAMA_RELEASE_FEED__: string \| null` and `__YOKOHAMA_VERSION__: string`; the Dockerfile `ARG` | the release notice | 2 |
| `bots::foreign(record) -> Vec<(u8, BotDescriptor, Foreignness)>` where `pub enum Foreignness { Unknown(String), Ineligible(String), Knobs(String), Drift { recorded: u32, runs: u32 } }`; `bots::runs_here(d: &BotDescriptor) -> Result<(), Foreignness>`; `bots::cannot_continue(played, runs, why: &Foreignness) -> String` ("This game was played against V4 (lookahead-v1). This server can't continue it: …"); `TableBot.runs_here` from `runs_here(...).is_ok()`; the regenerated corpus | the resume rule | 3 |
| `crates/yokohama-web/src/sweep.rs`: `pub struct Ttls { pub table: Option<u64>, pub open: Option<u64> }` with `Ttls::from_env() -> Result<Self, String>` and `Ttls::none()`; `pub enum Ending { Expired { days: u64 }, Waited { days: u64 } }`; `pub fn ending(status: TableStatus, last_active: u64, now: u64, ttls: &Ttls) -> Option<Ending>`; `pub fn pass(games: &Games, ttls: &Ttls, now: u64) -> Vec<(String, Ending, Result<(), String>)>` (lists, logs each candidate, then ends); `#[cfg(feature = "server")] pub fn run(state: Arc<AppState>, ttls: Ttls, every: Duration, first_after: Duration)` (a spawned task); `Store::last_written(&self, id: &str) -> Result<Option<u64>, String>`; `MemStore::stamp_for_test(id, secs)`; `Host.table_ttl_days`/`open_table_ttl_days` filled from `Ttls` on the axum host; the refusal sentences (below) | the sweep | 4 |
| `web/public/404.html`; `not_found_handling: "404-page"` in all five configurations; the guard's `assetsBlock` amended, check 11 `notFoundPage(tree, failures)`, check 12 `turnstileConsistency(name, value, tree, failures)` reading `build-values.json`; `web/dist/build-values.json` (R-FP3) written by a tiny Vite plugin in `vite.config.ts`, copied by `assemble-assets.mjs`; `web/scripts/presence.mjs` writing `dist/robots.txt`, `dist/llms.txt`, `dist/sitemap.xml` (the last only with `YOKOHAMA_SITE_URL`); the precache walk excluding `404.html`, `robots.txt`, `llms.txt`, `sitemap.xml`, `build-values.json`; the shell's `<meta name="description">`, Open Graph, Twitter and JSON-LD through `transformIndexHtml`; `YOKOHAMA_SITE_URL` in `BUILD_VALUES` with `checkSiteUrl`; `accessPaths` no longer deleting `health` (ten destinations); the axum 404 page for a miss | the 404 page, the presence, the guard | 5 |
| `web/src/turnstile.ts`: `turnstileSiteKey(): string \| null`, `loadTurnstile(): Promise<TurnstileApi>` (the script injected once, `?render=explicit`), `setChallengeHandler(fn: (() => Promise<void>) \| null)`, `passChallenge(): Promise<void>`; `web/src/Challenge.tsx` (the overlay rendering the widget into `<div className="challenge-widget">`, resolving on the callback); `api.ts`: `isEdgeChallenge(res): boolean` (`status === 403 && cf-mitigated === "challenge"`), `call` retrying once after `passChallenge()`, the error sentence `EDGE_CHECK = "This site asked for a check this build cannot show."` when no key; `FrontDoor.tsx`'s friends card as a `<form method="post" action="/enter">` with `<input type="hidden" name="cf-turnstile-response">` when a key is set; `PendingReason` gains `"challenged"`; `outbox.ts`'s `drain` excluding `challenged` rows on the `online` path and `drainVisible()` (launch, click) retrying them through the handler | Turnstile's client half | 6 |
| `Env.YOKOHAMA_TURNSTILE?`, `Env.TURNSTILE_SECRET?`; `ShellConfig.turnstile: boolean`; the refusal `YOKOHAMA_TURNSTILE=on needs the TURNSTILE_SECRET secret`; `Target` gains `{ kind: "enter-post" }` (POST `/enter`); `worker/src/turnstile.ts`: `siteverify(secret, token, remoteIp, expectedHost, fetchImpl): Promise<{ ok: true } \| { ok: false; why: string }>` and `tokenOf(formBody: string): string \| null`; `handle()`: under `on`, GET `/enter` → 403 `{"error": "go through the door's Play with friends"}`, POST `/enter` → siteverify then 302, else 403 `{"error": "the check did not pass — try again from the door"}`; under `off`, GET 302 and POST 405 as today; `SECRETS` gains `TURNSTILE_SECRET`; `wrangler.open.jsonc` carries `"YOKOHAMA_TURNSTILE": "on"`, production and the gate `"off"`, both pool files `"off"` | Turnstile's Worker half | 7 |
| `docs/web-lane.md`'s section "The metered-action ledger" (a table whose first column is the route as the API map spells it); `crates/yokohama-web/tests/lane_document.rs`'s cell `the_ledger_names_every_route`; `web/e2e/door_unmetered.spec.ts` (three cells); `worker/scripts/probe-burst.mjs` (orchestrator-only; prints per-request status, `cf-mitigated`, `cf-cache-status`, `cf-ray`); `web/scripts/pinned-browsers.mjs` (E's instrument, the `createRequire` path relative, the state directory gitignored) | the instruments | 8 |

---

## File structure

**Created:** `web/src/UpdateLine.tsx`, `web/src/UpdateLine.test.tsx` (1); `web/src/releaseFeed.ts`, `web/src/releaseFeed.test.ts`,
`web/src/ReleaseNotice.tsx`, `web/src/ReleaseNotice.test.tsx` (2); `crates/yokohama-web/src/sweep.rs` (4); `web/public/404.html`,
`web/scripts/presence.mjs`, `web/scripts/presence.node.test.ts` (5); `web/src/turnstile.ts`, `web/src/turnstile.test.ts`,
`web/src/Challenge.tsx`, `web/src/Challenge.test.tsx` (6); `worker/src/turnstile.ts`, `worker/src/turnstile.unit.test.ts` (7);
`web/e2e/door_unmetered.spec.ts`, `worker/scripts/probe-burst.mjs`, `web/scripts/pinned-browsers.mjs` (8); `deploy/update.sh`
(14); `docs/plans/<the day>-web-frontend-m6.5c2f-close.md` (15); in the ledger: the reports, the reviews, the pictures
(`task-1-look/`, `task-2-look/`, `task-6-look/`), `probe/`, `window.md`, `live.md`, `nas-sweep.md`, `sequence.tsv`.

**Modified** (principal files; each task's own list is the authority): `web/src/swRegister.ts`, `swRegister.test.ts`,
`App.tsx`, `App.test.tsx`, `OnThisDevice.tsx`, `Lobby.tsx`, `styles.css`, `worker/scripts/deploy-guard.mjs` and its cells (1);
`web/scripts/build-values.mjs`, `web/vite.config.ts`, `web/src/vite-env.d.ts`, `web/src/buildValues.ts`, `FrontDoor.tsx`,
`FrontDoor.test.tsx`, `Lobby.tsx`, `deploy/Dockerfile`, `deploy/README.md` §3, `web/scripts/e2e-build.mjs` (2);
`crates/yokohama-web/src/{bots,games,explain,tables}.rs`, `web/src/wire/` (the foreign samples), `web/src/App.test.tsx`,
`web/src/local/*.ts` where the "Will not open here" predicate lives (3); `crates/yokohama-web/src/{store,lib,main,api,log}.rs`,
`deploy/README.md` §4 and §8 (4); `worker/wrangler*.jsonc` (five files), `worker/scripts/{deploy-guard,assemble-assets,access-paths}.mjs`
and their cells, `worker/static/_headers`, `web/scripts/precache.mjs`, `web/index.html`, `web/vite.config.ts`,
`crates/yokohama-web/src/assets.rs` (5); `web/src/api.ts`, `api.test.ts`, `FrontDoor.tsx`, `outbox.ts`, `outbox.test.ts`,
`local/db.ts`, `App.tsx`, `styles.css` (6); `worker/src/{env,config,route,index}.ts`, `worker/src/config.unit.test.ts`,
`worker/test/worker.test.ts`, `worker/scripts/deploy-guard.mjs`, `worker/wrangler.open.jsonc` (7); `docs/web-lane.md`,
`crates/yokohama-web/tests/lane_document.rs`, `.gitignore` (8); the three replay-scrubber cells, `web/e2e/state1_game.spec.ts`,
`crates/yokohama-web/src/api.rs:2189–2196`, `README.md`, `docs/tools/guidingpath-relocate.py` (9); `docs/web-lane.md` (12);
`CLAUDE.md`, `docs/handover.md`, `guidingpath.md`, `docs/plans/2026-09-11-ai-lane-reentry-handover.md` (13); `README.md`,
`deploy/README.md`, `deploy/RUNBOOK.md`, `worker/RUNBOOK.md`, `worker/README.md` (14); the spec's §10.14, the RUNBOOKs' records (15).

---

## Task order and why

```
1 ─► 2 ─► 3 ─► 4 ─► 5 ─► 6 ─► 7 ─► 8 ─► 9 ─► 10 (review, wave, battery, code-head gates) ─► 11 (the live readings, the window)
                                                                                            └─► 12 ─► 13 ─► 14 ─► 15
```

1. **1 first:** the update path is what every later browser reading rides, and it touches `swRegister.ts`, which no
   other task touches.
2. **2 before 5 and 6:** 2 extends the build-values checker and the `define` shape once; 5 (the site URL) and 6 (the
   site key) add their values in the same shape and find the checker as 2 leaves it.
3. **3 and 4 are self-contained** and land before 5 so the corpus regeneration (3) is done before the configuration
   files move.
4. **5 before 6 and 7:** 5 writes the build-values stamp and the guard's check 12, whose names 6 (the client's key) and
   7 (the Worker's switch) honour; 5 also changes the assets block that 7's `wrangler.open.jsonc` edit finds as 5
   leaves it.
5. **6 before 7:** the client posts the form 7 answers; 7's pool cells use the client's field name.
6. **8 after 1–7:** the ledger and the fence cell read the client's requests as the cycle leaves them; the pinned
   instrument is committed once the update path is final.
7. **9 last of the code:** small, disjoint files.
8. **10, then 11:** the review before the deploys, so a fix wave never forces a second live sequence.
9. **12 → 14 after 11:** every document records Part 2's readings; 12 (the lane document) before 13 (`CLAUDE.md`, the
   board, guidingpath point into it) before 14 (the README and the guides link to every one).
10. **15 last.**

---

## Couplings between tasks (binding; the later task's implementer does what its item says)

- **Task 1 → Task 8:** `pinned-browsers.mjs` asserts the door's exact sentences; Task 1's report names them ("Ready
  to play offline"; the line's text).
- **Task 1 → Task 11:** 11.D's reading with two tabs uses Task 1's once-per-page-life rule; a door that reloads twice
  is a STOP.
- **Task 2 → Task 5:** `BUILD_VALUES` gains `YOKOHAMA_RELEASE_FEED` in 2 and `YOKOHAMA_SITE_URL` in 5; 5 finds the
  checker's shape as 2 leaves it and adds one entry.
- **Task 2 → Task 6:** the same for `YOKOHAMA_TURNSTILE_SITEKEY`.
- **Task 3 → Task 8:** the ledger's `/api/games/<id>/why` row cites the thirteenth refusal as 3 leaves it.
- **Task 4 → Task 14:** the self-host guide's rows and §8 quote the exact refusal sentences and log lines Task 4's
  report gives.
- **Task 5 → Task 7:** the guard's check 12 reads `vars.YOKOHAMA_TURNSTILE` and `build-values.json`'s
  `turnstile_sitekey`; 7 adds the var to the five configurations exactly as check 12 expects (`"off"` everywhere but
  `wrangler.open.jsonc`).
- **Task 5 → Task 11:** 11's probe runs against the deploy that carries the 404 page; the control read proves the
  instrument.
- **Task 6 → Task 7:** the form field is `cf-turnstile-response`; 7's `tokenOf` reads that name.
- **Task 6 → Task 8:** the fence cell counts the door's requests with the key UNSET (the e2e build); the widget script
  must not be among them.
- **Task 7 → Task 11:** the window's deploy is `npm run deploy:open` over a build WITH the key (R-FP4); the guard's
  door line and its check-12 line are quoted.
- **Task 8 → Task 12:** the ledger section's table is the one Task 12 completes with Part 2's figures; its cell holds
  every route to a row from Task 8 on.
- **Task 11 → Tasks 12–14:** every figure in the documents is Part 2's reading, named with its date.
- **Every task → Task 10:** each report names its catchers by cell name; the battery rows cite them.

---

### Task 1: The update path — the re-check on focus and every 15 minutes, the door applying itself, the one passive line, the guard's cache check — Opus (R-F1, R-F3; spec §2.1; Review Focus 1)

**Files:**
- Modify: `web/src/swRegister.ts:184–296` (`useReadiness`: the `visibilitychange` listener and the interval, both on
  `undo`; `registration.current?.update()` guarded by `live`), the module doc's "three rules" paragraph (now four:
  the re-check); `web/src/swRegister.test.ts` (the cells below); `web/src/App.tsx:191` (`useDoorAutoApply(route.kind
  === "door", readiness)` beside `useReadiness`), `:1602` (the door), the lobby's `lobby-actions` block in
  `web/src/Lobby.tsx:615–633` and the game's `<header className="banner">` at `App.tsx:2118` (each renders
  `<UpdateLine status={readiness} />`); `web/src/OnThisDevice.tsx:223–280` (`Readiness` keeps the install and failure
  lines and delegates the update line to `UpdateLine`); `web/src/styles.css` (`.update-line`, one line, the linkish
  button as today); `worker/scripts/deploy-guard.mjs:399–440` (`configFailures` gains check 10) and
  `worker/scripts/deploy-guard.test.mjs`; `web/e2e/offline.spec.ts:445` (the update cell's "nothing switches until
  the click" holds for a NON-door page; the door's auto-apply gets its own cell).
- Create: `web/src/UpdateLine.tsx`, `web/src/UpdateLine.test.tsx`.
- Do NOT touch: `web/src/sw.ts`, `swRoute.ts`, `swInstall.ts` (spec §2.1: the worker half is untouched — a diff
  there is a review finding).

**Interfaces:**
- Consumes: `useReadiness`'s `applyUpdate` and `updateWaiting` as they are; `RegistrationLike.update()`.
- Produces: `RECHECK_MS`; `useDoorAutoApply(isDoor, status)`; `UpdateLine({ status })`; `cacheBlock(name, value,
  failures)` in the guard.

- [ ] **Step 0 (the ORCHESTRATOR, before this task is dispatched): the baseline.** E's `step0-baseline.sh` copied into
  `$L` and run at `0d9dc90a` + this plan's commit: every per-edit gate's count and wall into `$L/baseline.md`
  (cargo, the eight identity cells, clippy, both rustdocs, the client's four, the Worker's four, `npm run e2e` with
  its per-spec walls). Every later reading is set against it.
- [ ] **Step 1: The re-check cells, red** (`web/src/swRegister.test.ts`, the fake `SwHost` and `RegistrationLike` the
  file already drives): (a) after `register` resolves, a `visibilitychange` to `visible` calls `update()` exactly once;
  a change to `hidden` calls nothing; (b) under fake timers, `update()` is called at `RECHECK_MS` and again at
  `2 * RECHECK_MS`, and never after the hook unmounts; (c) `update()` rejecting leaves `readiness` as it was and
  throws nothing. Run: `cd web && npx vitest run src/swRegister.test.ts` → FAIL (`RECHECK_MS` undefined; no
  `update` call).
- [ ] **Step 2: The re-check.** In the effect, after `adoptRegistration`: a `visibilitychange` listener on `document`
  (`if (document.visibilityState === "visible" && live) void reg.update().catch(() => undefined)`) and a
  `setInterval` at `RECHECK_MS`, both pushed onto `undo` (`clearInterval`, `removeEventListener`). The `SwHost` gains
  nothing: `document` and timers are the page's own, as `window.location.reload()` is. Step 1 → PASS.
- [ ] **Step 3: The auto-apply cells, red** (`swRegister.test.ts`): (a) `useDoorAutoApply(true, status)` with
  `status.updateWaiting` true calls `status.applyUpdate` once, and NOT again when the hook re-renders with the same
  status **(Review Focus 1)**; (b) with `isDoor` false it calls nothing even with `updateWaiting` true; (c) with
  `updateWaiting` false it calls nothing; (d) a status that goes `updateWaiting: false` then `true` while `isDoor`
  is true applies once. Run → FAIL.
- [ ] **Step 4: `useDoorAutoApply`.** A hook in `swRegister.ts`: a `useRef(false)` `applied`; an effect on
  `[isDoor, status.updateWaiting]` that, when both hold and `!applied.current`, sets it and calls
  `status.applyUpdate()`. Wired in `App.tsx` beside `useReadiness`. Step 3 → PASS.
- [ ] **Step 5: `UpdateLine`, red then green.** `UpdateLine.test.tsx`: renders nothing when `status.updateWaiting` is
  false or `status` is undefined; renders the sentence and the button when true; the click calls `applyUpdate`. Then
  the component; `OnThisDevice.tsx`'s `Readiness` delegates its update branch to it (the door's existing cell at
  `FrontDoor.test.tsx:287` "offers the update and switches only when it is pressed" stays green — it is the same
  element); the lobby and the game banner render it (`App.test.tsx`: a game screen with `updateWaiting` true shows
  the line and nothing reloads until the click; a lobby the same). The report carries a before-and-after picture of
  the lobby and the game banner at 1280 and 390 px (`$L/task-1-look/`; R-FP9).
- [ ] **Step 6: The e2e cell over two releases** (`web/e2e/offline.spec.ts`, beside the update cell at :445, using the
  second-release trick at :821): a browser on the DOOR at release A, release B written, a plain reload → within the
  install's time the door reloads ITSELF and reads "Ready to play offline" under B with NO click, and the caches show
  A retained beside B; a browser on a LOCAL GAME at release A when B lands shows the line, the game keeps playing,
  and the click continues the same game (the move count preserved) under B. Run: `npm run e2e` → PASS.
- [ ] **Step 7: The guard's check 10, red** (`worker/scripts/deploy-guard.test.mjs`): a configuration with
  `"cache": { "enabled": true }` at the top level, under `env.production`, or under `exports.default` is refused with
  the sentence `<file> enables Workers Cache (cache.enabled is not false) — with it on, static-asset requests are
  billed at the Worker request rate (Workers pricing, footnote 4); remove the block`; `"cache": { "enabled": false }`
  passes; no block passes. Then `cacheBlock(name, value, failures)` in `configFailures`, walking the three places.
  Run: `cd worker && npm run test:scripts` → PASS.
- [ ] **Step 8: The per-edit gates and the commit.**

```bash
git add web/src/swRegister.ts web/src/swRegister.test.ts web/src/App.tsx web/src/App.test.tsx web/src/OnThisDevice.tsx web/src/Lobby.tsx web/src/UpdateLine.tsx web/src/UpdateLine.test.tsx web/src/styles.css web/e2e/offline.spec.ts worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs
git commit -m "feat(web): the release is re-checked on focus and every 15 minutes, the door applies a ready release itself, one passive line elsewhere; the deploy guard refuses Workers Cache (R-F3, spec §2.1)"
```

---

### Task 2: The release notice — the feed build value, the browser-side check, the line on the door and the lobby — Sonnet 5.5 (R-F2; spec §2.2)

**Files:**
- Modify: `web/scripts/build-values.mjs:19–64` (`BUILD_VALUES` gains `YOKOHAMA_RELEASE_FEED`; a checker
  `checkReleaseFeed(raw, projectUrl)` implementing R-FP5: `undefined`/`""` → `defaultFeedFor(projectUrl)`; `"off"`
  → `null`; else an absolute `https://` URL checked as `checkProjectUrl` checks, at most 300 characters);
  `web/scripts/build-values.node.test.ts`; `web/vite.config.ts:38–45` (`__YOKOHAMA_RELEASE_FEED__` and
  `__YOKOHAMA_VERSION__` from `package.json`'s `version`); `web/src/vite-env.d.ts` (the two `declare const`s);
  `web/src/buildValues.ts` (`releaseFeed(): string | null`, `buildVersion(): string`); `web/src/FrontDoor.tsx`
  (`<ReleaseNotice />` under the blurb, above the link) and `FrontDoor.test.tsx:85` (the zero-server-request cell:
  `fetch` is stubbed to THROW for same-origin URLs and to answer the feed fixture for the feed URL — the cell now
  asserts no same-origin call); `web/src/Lobby.tsx:608–613` (`<ReleaseNotice />` in `lobby-door`); `deploy/Dockerfile:202–204`
  (`ARG YOKOHAMA_RELEASE_FEED`); `deploy/README.md:126–143` (the build arguments table gains the row with the three
  states); `web/scripts/e2e-build.mjs:14–21` (sets `YOKOHAMA_RELEASE_FEED=off` so no e2e cell reaches the network).
- Create: `web/src/releaseFeed.ts`, `web/src/releaseFeed.test.ts`, `web/src/ReleaseNotice.tsx`,
  `web/src/ReleaseNotice.test.tsx`.

**Interfaces:**
- Consumes: `checkProjectUrl` as it is; `projectLink()`; `readItem`/`writeItem` (`web/src/browserStorage.ts`) for
  the hourly throttle.
- Produces: the names table's row for this task, verbatim.

**The judgment already made (do not re-decide):** the feed answers GitHub's release shape (`{ "tag_name": "v0.7.0",
"html_url": "…" }`); the comparison is `isNewer(tag, own)` over `parseVersion` (`v` optional, three numbers, anything
else `null` → no notice); the fetch is `fetch(url, { cache: "no-store", headers: { accept: "application/vnd.github+json" } })`
through `fetchImpl` (default `fetch`), once per page life, and skipped when `yokohama.release.checked` holds a
timestamp under an hour old (every storage access wrapped; a broken storage means "check"); any failure → `null`,
nothing retried; the request leaves the origin, so the service worker leaves it alone and the door's same-origin rule
holds.

- [ ] **Step 1: The checker cells, red** (`build-values.node.test.ts`): `checkReleaseFeed(undefined, "https://github.com/o/r")`
  → `https://api.github.com/repos/o/r/releases/latest`; with a trailing slash the same; with `"https://example.org/x"`
  → `null`; with `null` → `null`; `checkReleaseFeed("off", …)` → `null`; `checkReleaseFeed("https://feed.example/latest", …)`
  → that URL; `checkReleaseFeed("ftp://x", …)` throws naming `YOKOHAMA_RELEASE_FEED`. Run: `cd web && npm run
  test:wasm -- build-values` → FAIL.
- [ ] **Step 2: The checker, the define, the declarations, the reader.** Step 1 → PASS; `npm run typecheck` green.
- [ ] **Step 3: The feed cells, red** (`releaseFeed.test.ts`): `parseVersion("v0.7.0")` → `[0,7,0]`, `"0.7.0"` the
  same, `"v0.7"` → `null`, `"latest"` → `null`; `isNewer("v0.7.0", "0.6.0")` true, equal false, older false,
  unparseable false; `useReleaseNotice` (rendered through a tiny harness) with a fixture fetch answering `{tag_name:
  "v0.7.0", html_url: "https://example/r"}` → `{ tag: "v0.7.0", url: … }`; answering `v0.6.0` → `null`; answering a
  500 → `null`; throwing → `null`; with `releaseFeed()` null → the fetch is NEVER called; a second render in the
  same page → no second fetch; a `yokohama.release.checked` under an hour old → no fetch. Run → FAIL.
- [ ] **Step 4: `releaseFeed.ts`.** As the judgment says. Step 3 → PASS.
- [ ] **Step 5: `ReleaseNotice`, red then green** (`ReleaseNotice.test.tsx`): nothing when the hook answers `null`; the
  sentence "A newer release is available: v0.7.0. This site runs 0.6.0" with the tag as a link (`target="_blank"
  rel="noopener"`) when it answers. Then the door and the lobby render it; `FrontDoor.test.tsx:85` narrowed as the
  Files say; a before-and-after picture of the door and the lobby at 1280 and 390 px in `$L/task-2-look/` (R-FP9).
- [ ] **Step 6: The Dockerfile, the guide's row, `e2e-build.mjs`; `npm run e2e` (the door's cells; nothing reaches the
  network since the e2e build sets `off`).**
- [ ] **Step 7: The per-edit gates and the commit.**

```bash
git add web/scripts/build-values.mjs web/scripts/build-values.node.test.ts web/vite.config.ts web/src/vite-env.d.ts web/src/buildValues.ts web/src/releaseFeed.ts web/src/releaseFeed.test.ts web/src/ReleaseNotice.tsx web/src/ReleaseNotice.test.tsx web/src/FrontDoor.tsx web/src/FrontDoor.test.tsx web/src/Lobby.tsx deploy/Dockerfile deploy/README.md web/scripts/e2e-build.mjs
git commit -m "feat(web): the door and the lobby say when the release feed names a newer release — YOKOHAMA_RELEASE_FEED, derived from a GitHub project link, off with the literal off; no server involved (R-F2, spec §2.2)"
```

---

### Task 3: The resume rule — a build that reconstructs the exact recorded bot plays on — Opus (R-F4; spec §2.3; Review Focus 2)

**Files:**
- Modify: `crates/yokohama-web/src/bots.rs:199–218` (`table_bot`, `foreign`), :293–318 (the sentence), the module
  doc (the rule restated); `crates/yokohama-web/src/games.rs:691–715` (the one site: the refusal carries the
  `Foreignness`), :1337–1420 (the fixtures: `foreign_pair("lookahead-v1", …)` becomes the OPENS case; a new
  `foreign_pair_at_knobs("v4-step9-draft1", b=8)` and a `foreign_pair_drifted(...)` are the refusals);
  `crates/yokohama-web/src/explain.rs:443–475` (`foreign_why` over the new `foreign`; the sentence carries the
  reason); `crates/yokohama-web/src/tables.rs` where `runs_here` is worded for the lobby (find by content: the
  "can't continue it" or `runs_here` sites); the browser list's predicate (find by content in `web/src/local/` and
  `web/src/BackupControls.tsx:400`: "Will not open here" is decided from `TableView.bot.runs_here`, which the module
  answers — if the client re-derives it, STOP and ask); the corpus (`web/src/wire/`: every sample built from a
  foreign identity — `why_foreign.json` and the table/list samples with `runs_here: false` — regenerated from the
  new genuinely-foreign fixture, the diff READ and named in the report); `web/src/App.test.tsx` and
  `web/e2e/plays_locally.spec.ts:1205` (the "Will not open here" e2e: its fixture must be a genuinely foreign identity
  now — a knob-mismatched named identity — or it goes green for the wrong reason).
- Do NOT touch: `crates/yokohama-cli/`, `crates/yokohama-core/` (fenced); `web/src/wire/production_bot.json`.

**Interfaces:**
- Consumes: `Contestant::from_parts(name, params)`, `Contestant::descriptor()`, `yokohama_cli::bot::eligible` — it is
  `pub(crate)` in the CLI (`bot.rs:377`); the layer needs it `pub`. **The CLI is fenced: STOP and ask.** The
  orchestrator's standing answer, given here so the task need not wait: **a one-line visibility change from
  `pub(crate)` to `pub` on `eligible` is a behaviour-preserving seam change the fence allows (CLAUDE.md: "the CLI
  touched only by behaviour-preserving seam changes")**; make it in its own commit, named in the report, and the
  fence line in Global Constraints reads that one file's diff as the seam. If `eligible`'s doc says it must stay
  crate-private for a reason, STOP.
- Produces: the names table's row for this task, verbatim: `Foreignness`, `runs_here(&BotDescriptor)`, the widened
  `foreign`, the sentence with its reason.

- [ ] **Step 1: The predicate cells, red** (`bots.rs` tests): `runs_here(&descriptor_of(Contestant::LookaheadV1 {…
  default knobs…}))` → `Ok(())`; the production descriptor → `Ok(())`; a `v4-step9-draft1` descriptor with `b: 8` →
  `Err(Foreignness::Knobs(_))` whose text names `b`; the production descriptor with `version + 1` →
  `Err(Foreignness::Drift { recorded, runs })`; controller `"nope-v9"` → `Err(Foreignness::Unknown(_))`; an
  `atomic-diag-v1` descriptor → `Err(Foreignness::Ineligible(_))`; `foreign(record)` over a record whose seat is
  `lookahead-v1` → empty. Run: `cargo test -p yokohama-web --lib bots` → FAIL.
- [ ] **Step 2: The predicate.** `runs_here`: `from_parts` → `Unknown`; `eligible` → `Ineligible`; the reconstructed
  `descriptor()`'s version ≠ recorded → `Drift`; knobs are already checked by `from_parts` for named identities and
  for the knobbed controllers the reconstructed params must equal the recorded ones → `Knobs` otherwise. `foreign`
  filters `recorded` through it. `table_bot`'s `runs_here: runs_here(&bot).is_ok()`. Step 1 → PASS.
- [ ] **Step 3: The site's cells** (`games.rs`): the flipped fixture — a `lookahead-v1` game at `BOT_TO_MOVE_AT` OPENS
  under the production build and `bring_alive`/the next input makes the bot's move (the seat's recorded controller
  moves, asserted through `run.seats`) **(Review Focus 2)**; the knob-mismatched fixture REFUSES with
  `GameError::ForeignBot { played, runs, why }` whose sentence names the knobs **(Review Focus 2)**; the drifted
  fixture refuses naming both versions; the finished foreign game opens as today (`a_finished_game_against_a_foreign_bot_opens`
  keeps passing with the new fixture); `an_unfinished_foreign_game_is_refused_by_the_retry_and_cleared_by_abandon`
  on the new fixture. Red first for the flipped and the two new; then the site carries `why` and the sentence is
  `cannot_continue(played, runs, &why)`: `This game was played against {played}. This server can't continue it:
  {why}` where `{why}` is one of `it does not know the bot {controller}` / `{controller} is a diagnostic and never
  seats` / `the record's knobs are not the bot's ({detail})` / `the bot's version drifted from {recorded} to {runs}`.
- [ ] **Step 4: `why` and the lobby.** `explain.rs`'s `foreign_why` uses the new `foreign`; a cell: `why` on a move by
  a `lookahead-v1` seat answers the reconstruction (no refusal); on a knob-mismatched seat the authored refusal with
  the reason. The lobby's row wording where `runs_here` is false carries the same sentence.
- [ ] **Step 5: The corpus.** `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`; READ the diff:
  every foreign sample now comes from the knob-mismatched named identity, `runs_here: false` rows keep their shape,
  the `why_foreign.json` sentence carries the reason, and NOTHING else moved. The client cells that render those
  samples (`App.test.tsx`, `OnThisDevice.test.tsx:109–150`, `BackupControls`) stay green or are adjusted to the new
  sentence only. The e2e at `plays_locally.spec.ts:1205` on the new fixture → PASS.
- [ ] **Step 6: The per-edit gates, the corpus pins, `npm run e2e`, and the commit** (the `eligible` visibility change
  as its own prior commit if made).

```bash
git add crates/yokohama-web/src/bots.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/explain.rs crates/yokohama-web/src/tables.rs web/src/wire web/src/App.test.tsx web/src/OnThisDevice.test.tsx web/e2e/plays_locally.spec.ts
git commit -m "feat(web): a game opens under any build that reconstructs its exact recorded bot; only an unknown, ineligible, re-knobbed or drifted bot refuses, naming why (R-F4 amending R-6.5c2.10; spec §2.3)"
```

---

### Task 4: The self-host sweep — the two TTLs, the hourly pass through the creator's delete, the archive-required refusal — Opus (R-F9, R-F10; spec §2.4; Review Focus 3)

**Files:**
- Create: `crates/yokohama-web/src/sweep.rs`.
- Modify: `crates/yokohama-web/src/store.rs:121–142` (the trait gains `last_written`), :341–420 (`FsStore`: the
  latest `modified()` over the three paths that exist, as unix seconds; absent all → `Ok(None)`; the house is refused
  as `path` refuses it), :436–505 (`MemStore`: a `stamps: Mutex<BTreeMap<GameId, u64>>` bumped by a counter on every
  `write_if_version`, `stamp_for_test(id, secs)`, removed on `delete`); `crates/yokohama-web/src/lib.rs:351–470`
  (`AppState` gains `ttls: sweep::Ttls`; `AppState::new` reads `Ttls::from_env()` and refuses TTL-without-archive
  BEFORE building the store), the `app()`/adapter site that builds `api::Host` (`table_ttl_days: state.ttls.table`,
  `open_table_ttl_days: state.ttls.open`); `crates/yokohama-web/src/main.rs:135–156` (after `AppState::new`:
  `tokio::spawn(sweep::run(state.clone(), state.ttls.clone(), Duration::from_secs(3600), Duration::from_secs(60)))`
  when `ttls.table.is_some()`); `crates/yokohama-web/src/log.rs` (three new events: `sweep-listed`, `sweep-ended`,
  `sweep-failed`, each with the id, the reason and the days); `deploy/README.md:144–184` (§4's variables table gains
  the two rows) and :224–265 (§8 rewritten: the sweep, the listing, the archive rule, the log lines; the by-hand
  recipe kept for the unset case); `docs/web-lane.md:444–445` (the two rows' host column: "both"; the drift cell
  `lane-variables.test.mjs` sees the names in `crates/yokohama-web/src/sweep.rs`).
- Do NOT touch: `worker/src/*` (the Worker's expiry is its own; the names and meanings are copied, the code is not).

**Interfaces:**
- Consumes: `Games::delete(id, &User)` (the creator's own delete, `tables.rs:1875`); `TableDoc` (`created_by`,
  `status`), read through `Games::read_table` or the store; `Games::log`; `AppState::with_store_chat_archive_and_log`.
- Produces: the names table's row for this task, verbatim. **The refusal sentences**: `YOKOHAMA_WEB_TABLE_TTL_DAYS
  must be a whole number of days above zero — got "x"`; `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS must be a whole number of
  days above zero — got "x"`; `YOKOHAMA_WEB_TABLE_TTL_DAYS is set but YOKOHAMA_WEB_ARCHIVE_DIR is not — a sweep
  without an archive throws games away; set both or neither`. **The log lines**: `sweep-listed {id, status, idle_days,
  reason}` once per candidate BEFORE any delete; `sweep-ended {id, reason, kept: "archive" | "nothing"}`;
  `sweep-failed {id, reason, error}`.

**The decision, so the implementer builds the Worker's rule and not a new one** (`worker/src/marker.ts:99–129`):
with `ttls.table = None` → nothing; `open` and `now >= last_active + open_ttl_days` (open defaulting to 7) →
`Waited { days: open }`; any status and `now >= last_active + table_ttl_days` → `Expired { days: table }`; `dealing`
and `playing` wait the table TTL; a day is 86,400 seconds, whole days.

- [ ] **Step 1: The decision cells, red** (`sweep.rs` tests, pure): the seven cases above plus the day boundary (one
  second short → `None`; exactly the deadline → `Some`) and `open` past both TTLs → `Waited` (the shorter wins, as
  the Worker's `expiryReason` orders it). Run: `cargo test -p yokohama-web --lib sweep` → FAIL (no module).
- [ ] **Step 2: `ending`, `Ttls`.** `Ttls::from_env()` reads the two variables with the Worker's refusal shape (a
  whole number above zero; unset table → `Ttls::none()` whatever the open value; unset open → 7 when the table is
  set). Step 1 → PASS.
- [ ] **Step 3: The store cells, red** (`store.rs`'s `both(...)` harness over `FsStore` and `MemStore`):
  `last_written` is `None` for an unknown id; after a record write it is set; a later chat write moves it forward;
  the house id is refused; after `delete` it is `None`. Then the two implementations. → PASS.
- [ ] **Step 4: The pass cells, red** (`sweep.rs` tests over `Games` on a `MemStore` with `with_clock` and a
  recording `Log`): three tables — one `playing` with moves idle 100 days under `Ttls { table: 90, open: 7 }`, one
  `open` idle 8 days, one `playing` idle 10 days — `pass(...)` logs `sweep-listed` for the first two BEFORE any
  `sweep-ended` (the log's order is asserted) **(Review Focus 3)**, ends both through the creator's delete (the
  archive receives the first's record once under `over/` or `abandoned/` per its mirror; the second files nothing,
  `kept: "nothing"`), leaves the third; a finished game already stamped is not put again; a table whose archive put
  FAILS (a failing `Archive`) is logged `sweep-failed` and is still there; the house chat is never listed. Then
  `pass`. → PASS.
- [ ] **Step 5: The start refusal, red** (`lib.rs` tests over `AppState::new`-like construction, or `Ttls` +
  `DirArchive::from_env` composed in a function `sweep::check_archive(ttls, archive_on) -> Result<(), String>`):
  a TTL with no archive → the sentence naming both **(Review Focus 3)**; a TTL with an archive → `Ok`; no TTL, no
  archive → `Ok`. Then `AppState::new` calls it and `main.rs` exits 1 naming the variables. → PASS.
- [ ] **Step 6: Health and the task.** `Host`'s two fields from `state.ttls` (a lib cell: with `Ttls { table: Some(90),
  open: Some(7) }` the health JSON reads 90 and 7; with `none()` both `null` — the existing cell at `lib.rs:6729`
  stays). `sweep::run` behind `#[cfg(feature = "server")]`: `tokio::time::sleep(first_after)`, then a loop of
  `pass(...)` + `tokio::time::sleep(every)`, `now` from `SystemTime` inside the gated file — the scan
  `platform_calls_stay_behind_the_server_feature` names any escape (`sweep.rs` is NOT whole-file gated: `ending`,
  `Ttls` and `pass` build for wasm; `run` and the clock read carry the attribute). `cargo check --no-default-features
  --target wasm32-unknown-unknown -p yokohama-web` green.
- [ ] **Step 7: The documents** (the guide's §4 rows and §8; the lane document's two host columns). `npm run
  test:scripts` (the variables cell) green.
- [ ] **Step 8: The per-edit gates (the corpus's `health.json` must NOT move — the corpus host sets no TTL; say so in
  the report) and the commit.**

```bash
git add crates/yokohama-web/src/sweep.rs crates/yokohama-web/src/store.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/main.rs crates/yokohama-web/src/api.rs crates/yokohama-web/src/log.rs deploy/README.md docs/web-lane.md
git commit -m "feat(web-server): the self-host sweeps idle tables under YOKOHAMA_WEB_TABLE_TTL_DAYS and YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS, hourly, through the creator's own delete, listing before ending; a TTL without an archive refuses to start (R-F9, R-F10; spec §2.4)"
```

---

### Task 5: The 404 page, the guard's checks 11 and 12, the build-values stamp, the web presence files, health into the protected list — Sonnet 5.5 (R-F5, R-F12; spec §2.5, §2.7; Review Focus 5)

**Files:**
- Create: `web/public/404.html`; `web/scripts/presence.mjs`, `web/scripts/presence.node.test.ts`.
- Modify: `worker/wrangler.jsonc:37–43`, `wrangler.test.jsonc:21–26`, `wrangler.open.test.jsonc:23–28`,
  `wrangler.open.jsonc:33–38`, `wrangler.gate.jsonc:38–43` (`"not_found_handling": "404-page"`);
  `worker/scripts/deploy-guard.mjs:133–160` (`assetsBlock` expects `404-page`), :399–440 (`configFailures` gains
  check 11 `notFoundPage(tree, failures)` — the assets tree holds `404.html` — and check 12 `turnstileConsistency(name,
  value, tree, failures)`: `vars.YOKOHAMA_TURNSTILE` read as `on`/`off`/absent (absent = off, the Worker's rule),
  `build-values.json`'s `turnstile_sitekey` read as a boolean; `on` without `true` → `<file> sets YOKOHAMA_TURNSTILE
  on but the assets tree's build carries no site key (build-values.json) — build the client with
  YOKOHAMA_TURNSTILE_SITEKEY set`; `off`/absent with `true` → `<file> leaves YOKOHAMA_TURNSTILE off but the assets
  tree's build carries a site key — the widget would render under Access; build the client without the key or set
  the var on`; a tree with no `build-values.json` → refused naming the file), and `deploy-guard.test.mjs`;
  `worker/scripts/assemble-assets.mjs` (copies `build-values.json`, `404.html`, `robots.txt`, `llms.txt`, and
  `sitemap.xml` when present, into the tree; they are NOT precache entries, so `verifyTree` skips them — read how it
  treats `_headers` and do the same); `worker/static/_headers` (`/robots.txt`, `/llms.txt`, `/sitemap.xml`:
  `Cache-Control: public, max-age=3600`); `web/scripts/precache.mjs:108` (the walk excludes the five names);
  `web/src/precache.node.test.ts` (none of the five is listed); `web/vite.config.ts` (the `writeBundle` plugin
  writing `dist/build-values.json`, R-FP3; `transformIndexHtml` injecting the metadata; `YOKOHAMA_SITE_URL` through
  `checkSiteUrl`); `web/scripts/build-values.mjs` (`YOKOHAMA_SITE_URL`, `checkSiteUrl`: an absolute `https://`
  origin with no path beyond `/`, at most 200 characters); `web/src/vite-env.d.ts`, `web/src/buildValues.ts`
  (`siteUrl(): string | null`); `web/index.html:1–33` (a placeholder comment where the transform inserts);
  `web/package.json:12` (`build:client` runs `presence.mjs` after `precache.mjs`); `worker/scripts/access-paths.mjs:56`
  (`segments.delete("health")` removed; the header comment corrected) and `access-paths.test.mjs` (ten
  destinations); `crates/yokohama-web/src/assets.rs:113–140` (`cache_control`: the three presence files get
  `public, max-age=3600`; `serve_path`'s miss for a non-route path answers `404.html`'s bytes with status 404 and
  `text/html`); `deploy/Dockerfile:202–204` (`ARG YOKOHAMA_SITE_URL`); `deploy/README.md:126–143` (the row).
- Do NOT touch: `web/src/sw.ts`, `swRoute.ts` (a controlled browser never sees the 404 page: rule 2 answers every
  navigation from the shell — the cell below proves it without changing the worker).

**Interfaces:**
- Consumes: `BUILD_VALUES` and the checker shape as Task 2 leaves them; `assetsBlock`'s key list; `readPrecacheList`.
- Produces: the names table's row for this task, verbatim.

**The texts, given so no wording is decided in the task.** `404.html`: a page in the app's own colours (`--bg`
`#0d1013`, the typefaces are not needed) with `<meta name="robots" content="noindex">`, the title "Nothing here",
one sentence "There's nothing at this address." and a link "Go to the door" to `/`. `robots.txt`:

```
User-agent: *
Allow: /
Disallow: /api/
Disallow: /enter
Disallow: /cdn-cgi/
```

plus `Sitemap: <site url>/sitemap.xml` when the site URL is set. `llms.txt` (the llmstxt.org shape; the values from
the build):

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
metadata in the shell: `<meta name="description" content="<the blurb, or: Yokohama, the board game, in your browser. Play the bot here, or play with friends.">`,
`<link rel="canonical" href="<site url>/">` (with the URL only), `og:title` "Yokohama", `og:description` the same
description, `og:type` `website`, `og:url` (with the URL only), `og:image` `<site url>/icons/icon-512.png` when the URL
is set else `/icons/icon-512.png` (check the icon's real name in `web/public/icons/`), `twitter:card` `summary`, and
one `<script type="application/ld+json">` with `{"@context":"https://schema.org","@type":"WebApplication","name":"Yokohama","applicationCategory":"GameApplication","operatingSystem":"Any","isAccessibleForFree":true,"license":"https://www.apache.org/licenses/LICENSE-2.0","description":"<the description>","url":"<site url or omitted>","codeRepository":"<project link or omitted>"}`.

- [ ] **Step 1: The guard cells, red** (`deploy-guard.test.mjs`): the five committed files pass with `404-page`; a
  file with `none` is refused by `assetsBlock`'s sentence; a tree without `404.html` is refused naming it; the four
  Turnstile combinations as the sentences above (a fixture tree with a `build-values.json` of each boolean). Run:
  `cd worker && npm run test:scripts` → FAIL.
- [ ] **Step 2: The configurations and the guard.** The five files; `assetsBlock`; `notFoundPage`;
  `turnstileConsistency`. Step 1 → PASS. `node scripts/deploy-guard.mjs` reads green on the committed files (without
  deploying) once the stamp exists (Step 4).
- [ ] **Step 3: The presence cells, red** (`presence.node.test.ts`, run over a scratch `dist`): `robots.txt` carries the
  three `Disallow` lines and the `Sitemap:` line only with a site URL; `llms.txt` carries the repository section only
  with a link and never an `@`; `sitemap.xml` exists only with a site URL and names the one page; `404.html` is in
  `dist` and carries `noindex`; `build-values.json` is the six booleans and no other key; `precache.node.test.ts`:
  none of the five names is a precache entry, and the entry count is what it was. Run: `cd web && npm run test:wasm
  -- presence precache` → FAIL.
- [ ] **Step 4: The files.** `presence.mjs` (reads the checked values through `build-values.mjs`; writes the three
  files), the Vite plugin writing the stamp, `checkSiteUrl`, the precache exclusion, the `_headers` lines,
  `assemble-assets.mjs`. Step 3 → PASS.
- [ ] **Step 5: The shell's metadata, red then green** (a node cell over the built `dist/index.html`): each tag exactly
  once; the JSON-LD parses and carries `codeRepository` only when the link is set; the canonical and `og:url` only
  with the site URL; then `transformIndexHtml`. `npm run build:client` twice, once with and once without the two
  values; both trees checked.
- [ ] **Step 6: The axum 404, red then green** (`assets.rs` tests over the router with a scratch `dist` holding
  `404.html`): `GET /nope.json` and `GET /wp-login.php` answer 404 with `404.html`'s bytes and `text/html`;
  `GET /` and a route-like `GET /t/abc` still answer the shell **(Review Focus 5)**; `GET /robots.txt` answers
  `text/plain` with `max-age=3600`. Then the code.
- [ ] **Step 7: The protected list.** `access-paths.mjs` stops deleting `health`; the cell's list gains
  `yokohama.moore.nyc/api/health` (ten); the comment says why (spec §2.5). `npm run test:scripts` → PASS.
- [ ] **Step 8: The e2e** (`npm run e2e`: the offline cells' entry counts unchanged; the door's cells green).
- [ ] **Step 9: The per-edit gates and the commit.**

```bash
git add web/public/404.html web/scripts/presence.mjs web/scripts/presence.node.test.ts web/scripts/build-values.mjs web/scripts/build-values.node.test.ts web/scripts/precache.mjs web/src/precache.node.test.ts web/vite.config.ts web/src/vite-env.d.ts web/src/buildValues.ts web/index.html web/package.json worker/wrangler.jsonc worker/wrangler.test.jsonc worker/wrangler.open.test.jsonc worker/wrangler.open.jsonc worker/wrangler.gate.jsonc worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs worker/scripts/assemble-assets.mjs worker/scripts/assemble-assets.test.mjs worker/scripts/access-paths.mjs worker/scripts/access-paths.test.mjs worker/static/_headers crates/yokohama-web/src/assets.rs deploy/Dockerfile deploy/README.md
git commit -m "feat(web,worker): a static 404 page answers every miss on both hosts (not_found_handling 404-page); the deploy guard requires it, refuses a Turnstile switch that disagrees with the build's stamp; robots.txt, llms.txt, a sitemap and the page's metadata with YOKOHAMA_SITE_URL; health joins the protected paths (R-F5, R-F12; spec §2.5, §2.7)"
```

---

### Task 6: Turnstile's client half — the button's widget, the edge-challenge handler at the one API call site, the outbox's "challenged" row — Opus (R-F6; spec §2.6; Review Focus 4)

**Files:**
- Create: `web/src/turnstile.ts`, `web/src/turnstile.test.ts`, `web/src/Challenge.tsx`, `web/src/Challenge.test.tsx`.
- Modify: `web/scripts/build-values.mjs` (`YOKOHAMA_TURNSTILE_SITEKEY` in `BUILD_VALUES`, checked as plain text of at
  most 100 characters, no whitespace; unset or empty → `null`); `web/vite.config.ts` (`__YOKOHAMA_TURNSTILE_SITEKEY__`;
  the stamp's `turnstile_sitekey` boolean from it — Task 5's plugin reads the checked values, so this is one entry);
  `web/src/vite-env.d.ts`; `web/src/buildValues.ts` (`turnstileSiteKey()` lives in `turnstile.ts` and reads the
  define; `buildValues.ts` re-exports nothing new); `web/src/api.ts:262–275` (`isEdgeChallenge` beside
  `isAccessChallenge`; `EDGE_CHECK`), :636–660 (`call` gains the retry, R-FP8; an `opts?: { challenge?: boolean }`
  parameter threaded through the `api` helpers the outbox uses — find them by content in `api.ts`: the archive post
  and the health read); `web/src/api.test.ts`; `web/src/FrontDoor.tsx:142–156` (the friends card's two shapes);
  `web/src/FrontDoor.test.tsx`; `web/src/local/db.ts:87` (`PendingReason` gains `"challenged"`; a stored row with it
  reads back — the IndexedDB shape is a string, no migration); `web/src/outbox.ts:443–470` (`drain` from the `online`
  event and the launch's background path excludes `"challenged"` rows as it excludes `"refused"`, and calls the API
  with `{ challenge: false }` so a challenge MARKS the row instead of popping a widget in a hidden tab;
  `drainVisible()` — called at the door's launch when `document.visibilityState === "visible"` and by the pending
  row's button — includes them and lets the handler run), the pending row's wording in the door's outbox list
  (`"waiting for a human check"` with a button "Check now" that calls `drainVisible`); `web/src/outbox.test.ts`;
  `web/src/App.tsx` (mounts `<Challenge />` after `<App />`'s tree when `turnstileSiteKey()` is set — as `Footer`
  is mounted in `main.tsx`, put it there); `web/src/styles.css` (`.challenge-overlay`, `.challenge-widget`,
  `.enter-form`).
- Do NOT touch: `web/src/sw.ts` and `swRoute.ts` (the widget script is cross-origin; the worker leaves it alone by
  its same-origin rule — a cell asserts the script's origin is `challenges.cloudflare.com`, nothing more).

**Interfaces:**
- Consumes: `BUILD_VALUES` as Tasks 2 and 5 leave it; `call`/`settle`; `PendingReason`; `readItem`/`writeItem`.
- Produces: the names table's row for this task, verbatim. **The field name `cf-turnstile-response`** (Task 7 reads
  it). **The script URL** `https://challenges.cloudflare.com/turnstile/v0/api.js?onload=__yokohamaTurnstile&render=explicit`,
  injected ONCE per page life by `loadTurnstile()`, never at module load. **The overlay's words**: "Checking that
  this is a person…" and a "Cancel" button. **The offline sentence** on the button: "The lobby needs a connection."

**The judgment already made:** `challengeOnce(container)` renders the widget explicitly (`turnstile.render(container,
{ sitekey, callback, "error-callback", "expired-callback" })`), resolves the token from `callback`, rejects on the
two error callbacks or after 90 s, and removes the widget (`turnstile.remove(id)`) either way. `passChallenge()`
resolves when the overlay's `challengeOnce` resolves (the pre-clearance cookie is then set by the widget; the token
is discarded — the cookie is what the retry rides). `call`'s retry is ONE: a second `cf-mitigated: challenge` on the
same request is settled as the refusal it is (a 403 with the sentence `EDGE_CHECK_REPEATED = "The site's check did not
pass. Try again from the door."`). With no site key in the build, `passChallenge` rejects with `EDGE_CHECK` and the
refusal carries that sentence. The stream openers (`signal.ts`) are NOT changed: a challenged stream fails as any
failure and the existing reconnect takes it after the next `call` has passed the check.

- [ ] **Step 1: The loader and challenge cells, red** (`turnstile.test.ts`, jsdom, a fake `window.turnstile`):
  `turnstileSiteKey()` is `null` under the test define and a string when set (through `vi.stubGlobal` on the
  define); `loadTurnstile()` injects the script tag ONCE across two calls with the exact URL, and resolves when the
  fake `onload` global fires; `challengeOnce(el)` resolves the token the fake `callback` gives and calls `remove`;
  rejects on `error-callback` and on the timeout under fake timers; `passChallenge()` with no handler rejects with
  `EDGE_CHECK`. Run: `cd web && npx vitest run src/turnstile.test.ts` → FAIL.
- [ ] **Step 2: `turnstile.ts`.** Step 1 → PASS.
- [ ] **Step 3: The handler cells, red** (`api.test.ts`): a fixture `Response` 403 with `cf-mitigated: challenge`
  followed by a 200 → the handler is called once, the second response is returned **(Review Focus 4)**; two
  challenges in a row → the handler once, then the `EDGE_CHECK_REPEATED` refusal, no third fetch **(Review Focus
  4)**; no handler → `EDGE_CHECK`, no retry; `{ challenge: false }` → no handler call, the refusal at once; a plain 403
  without the header → today's refusal path untouched. Then `isEdgeChallenge`, the retry loop in `call`, the option.
  → PASS.
- [ ] **Step 4: The overlay, red then green** (`Challenge.test.tsx`): mounts hidden; `passChallenge()` shows it and
  renders the widget into `.challenge-widget`; resolves on the fake callback and hides; "Cancel" rejects and hides.
- [ ] **Step 5: The button, red then green** (`FrontDoor.test.tsx`): with no key the anchor `href="/enter"` as today
  and the loader is NEVER called; with a key the form with the hidden input and the submit button, the loader not
  called at MOUNT, called once on the click, the token written into the input and the form submitted (spy on
  `HTMLFormElement.prototype.submit`); offline (`navigator.onLine` false) the sentence and no loader call. The
  door's zero-server-request cell (Task 2's shape) still passes: no same-origin request.
- [ ] **Step 6: The outbox, red then green** (`outbox.test.ts`): an `online` drain whose archive post meets a
  challenge marks the row `"challenged"`, the handler is NOT called; `drainVisible()` calls the handler, retries and
  clears the row on 200; the door lists the row with the wording and the button; a `"challenged"` row is not retried
  by the next `online` event.
- [ ] **Step 7: `npm run e2e`** (the e2e build has no key: every existing cell green; the door's requests unchanged).
  A before-and-after picture of the friends card with a key (a local build with the always-passing test site key)
  and of the overlay, in `$L/task-6-look/`.
- [ ] **Step 8: The per-edit gates and the commit.**

```bash
git add web/src/turnstile.ts web/src/turnstile.test.ts web/src/Challenge.tsx web/src/Challenge.test.tsx web/src/api.ts web/src/api.test.ts web/src/FrontDoor.tsx web/src/FrontDoor.test.tsx web/src/local/db.ts web/src/outbox.ts web/src/outbox.test.ts web/src/main.tsx web/src/styles.css web/scripts/build-values.mjs web/scripts/build-values.node.test.ts web/vite.config.ts web/src/vite-env.d.ts
git commit -m "feat(web): Turnstile behind a build value — the widget on the multiplayer button's click, the edge challenge handled once at the API call site with one retry, the outbox marking a challenged send for a visible tab (R-F6; spec §2.6)"
```

---

### Task 7: Turnstile's Worker half — the switch, the secret, POST /enter with siteverify, the pool and the guard — Opus (R-F6; spec §2.6)

**Files:**
- Create: `worker/src/turnstile.ts`, `worker/src/turnstile.unit.test.ts`.
- Modify: `worker/src/env.ts:40–70` (`YOKOHAMA_TURNSTILE?: string`, `TURNSTILE_SECRET?: string`, each with its doc:
  the var `on` | `off`, absent = off; the secret set with `wrangler secret put`, never a committed value);
  `worker/src/config.ts:37–51` (`ROWS`), the `ShellConfig` type (`turnstile: boolean`), :250–275 (`shellConfig`:
  `turnstile: onOff("YOKOHAMA_TURNSTILE", text(env, "YOKOHAMA_TURNSTILE"))`, then the refusal
  `YOKOHAMA_TURNSTILE=on needs the TURNSTILE_SECRET secret` when `on` and the secret is absent or empty);
  `worker/src/config.unit.test.ts`; `worker/src/route.ts:9–80` (`Target` gains `{ kind: "enter-post" }`; `route`:
  `/enter` → GET/HEAD `enter`, POST `enter-post`, else `not-allowed`); `worker/src/index.ts:96–140` (the `enter`
  arm MOVES below `shellConfig` — it needs the switch now; under `off`: `enter` → 302, `enter-post` → 405; under
  `on`: `enter` → 403 `{"error":"go through the door's Play with friends"}` in the API's error shape, `enter-post` →
  the bounded body, `tokenOf`, `siteverify(secret, token, req.headers.get("CF-Connecting-IP"), url.hostname,
  fetch)` → 302 to `/#/lobby`, else 403 `{"error":"the check did not pass — try again from the door"}`; a missing
  token is the same 403); `worker/test/worker.test.ts:180–290` (the pool cells); `worker/scripts/deploy-guard.mjs:215–240`
  (`SECRETS` gains `TURNSTILE_SECRET`: "the Turnstile secret, set with wrangler secret put") and :275–295
  (`siblingDrift`'s allowed differences for `wrangler.open.jsonc` gain `YOKOHAMA_TURNSTILE: "on"` beside the door);
  `worker/wrangler.jsonc` (`"YOKOHAMA_TURNSTILE": "off"` with a comment), `wrangler.gate.jsonc` (`"off"`),
  `wrangler.test.jsonc` and `wrangler.open.test.jsonc` (`"off"`), `wrangler.open.jsonc` (`"on"`);
  `worker/README.md`'s variables table and `worker/RUNBOOK.md:170–260` §3 (the var's row and the secret's row: what
  absence means).
- Do NOT touch: `crates/yokohama-worker/` (the module knows nothing of Turnstile; `/enter` never reaches it).

**Interfaces:**
- Consumes: `bounded(req.body)` (`index.ts:127`); the API error shape (`errorBody`, `respond`); Task 6's field name.
- Produces: the names table's row for this task, verbatim.

**The judgment already made** (R-FP7): `siteverify` posts `application/x-www-form-urlencoded` with `secret`,
`response` and, when present, `remoteip` to `https://challenges.cloudflare.com/turnstile/v0/siteverify`, with
`AbortSignal.timeout(10_000)`, through `fetchImpl`; it reads `{ success, hostname, "error-codes" }`; `ok` iff
`success === true` and `hostname === expectedHost`; any throw, non-JSON or timeout → `{ ok: false, why: "siteverify did
not answer" }`; a hostname mismatch names both; error codes are joined into `why`. **The secret is never logged, never
in an error sentence, never in a `why`.** `tokenOf(body)` reads `cf-turnstile-response` from the form body
(`URLSearchParams`), non-empty and at most 2048 characters, else `null`.

- [ ] **Step 1: The unit cells, red** (`turnstile.unit.test.ts`, node): the six `siteverify` outcomes above over a
  fake `fetchImpl` that records the request (the URL, the content type, the three fields; the secret appears in the
  BODY and nowhere in any returned `why`); `tokenOf` for a good body, an empty field, a missing field, a 3,000-char
  token. Run: `cd worker && npm run test:scripts -- turnstile` (or the unit project the file's name selects) → FAIL.
- [ ] **Step 2: `turnstile.ts`.** → PASS.
- [ ] **Step 3: The config cells, red** (`config.unit.test.ts`): absent → `turnstile: false`; `off` → false; `on`
  with the secret → true; `on` without → the refusal sentence; `maybe` → `onOff`'s refusal naming the var;
  `moduleConfig` unchanged (the module never sees it — a cell asserts the eight keys stay eight). → then the code →
  PASS.
- [ ] **Step 4: The pool cells, red** (`worker.test.ts`): under `off` (the pool's default): GET `/enter` 302, POST
  405, HEAD 302 (today's cells stay); under an env with `YOKOHAMA_TURNSTILE: "on"` and `TURNSTILE_SECRET: "x"`: GET
  `/enter` 403 with the sentence, POST with no body 403 with the sentence, POST with a token → the request reaches
  `siteverify` — which in the pool CANNOT be stubbed and must not reach the network — so this cell asserts only the
  shape BEFORE the verification: a body over the bound answers 400, a body without the field 403, and the
  with-token case is marked `test.skip` with the sentence "read in the window (R-FP7)"; under `on` without the
  secret every request answers 500 naming it. Then the route arm and `handle`. → PASS.
- [ ] **Step 5: The guard and the five files** (`deploy-guard.test.mjs`: `TURNSTILE_SECRET` in any `vars` is refused
  with its why; the open file's `on` is not sibling drift; a pool file with `on` is). `node scripts/deploy-guard.mjs`
  on the committed files reads green with the current assets tree (which carries `turnstile_sitekey: false` — check
  12 accepts production's `off`).
- [ ] **Step 6: The documents** (the Worker README's table, the RUNBOOK's §3 rows; the lane document's §12 rows are
  Task 12's, but the variables DRIFT cell reads the names from code now: `npm run test:scripts` must stay green — if
  the cell requires the document row FIRST, add the two rows to `docs/web-lane.md` §12 in this task with the
  spec's wording and say so).
- [ ] **Step 7: The per-edit gates (`npm run build:module:debug` before the pool) and the commit.**

```bash
git add worker/src/turnstile.ts worker/src/turnstile.unit.test.ts worker/src/env.ts worker/src/config.ts worker/src/config.unit.test.ts worker/src/route.ts worker/src/index.ts worker/test/worker.test.ts worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs worker/wrangler.jsonc worker/wrangler.gate.jsonc worker/wrangler.test.jsonc worker/wrangler.open.test.jsonc worker/wrangler.open.jsonc worker/README.md worker/RUNBOOK.md docs/web-lane.md
git commit -m "feat(worker): YOKOHAMA_TURNSTILE on|off with the TURNSTILE_SECRET secret — POST /enter verifies the widget's token through siteverify before the 302; off keeps /enter as it was; the guard refuses the secret in vars (R-F6; spec §2.6)"
```

---

### Task 8: The instruments and the ledger — the fence cell, the ledger section and its cell, the probe script, the pinned-browser instrument — Opus (R-F5; spec §2.5; Review Focus 5)

**Files:**
- Create: `web/e2e/door_unmetered.spec.ts`; `worker/scripts/probe-burst.mjs`, `worker/scripts/probe-burst.test.mjs`;
  `web/scripts/pinned-browsers.mjs` (E's ledger copy, the `createRequire` path relative, a usage header; its state
  directories under `web/scripts/pinned/` gitignored).
- Modify: `docs/web-lane.md` (a new last section, "## 16. The metered-action ledger", the table's SHAPE with every
  route as a row and the columns filled from the code and E's figures; Task 12 completes it with Part 2's
  readings); `crates/yokohama-web/tests/lane_document.rs:69–170` (the cell `the_ledger_names_every_route`: every
  route of the layer's `routes()` table and the two hosts' own routes appears in §16's route column, reusing
  `map_rows`'s parser over the new section); `.gitignore` (`web/scripts/pinned/`); `web/e2e/every_spec_declares_its_worker.spec.ts`
  (the new spec declares its worker as the convention requires — read that spec first).

**Interfaces:**
- Consumes: the release's precache list (`web/dist/sw.js` through `worker/scripts/precache-list.mjs`'s
  `readPrecacheList`); `TALLY_VAR`; the router facts `access-paths.mjs` reads.
- Produces: the names table's row for this task, verbatim. **The ledger's columns**: `route` (as the API map spells
  it) · `trigger` (the screen and the action) · `purpose` · `per session / per applied move` (E's figures where they
  exist: 7.9 a move in the two-browser tier, 8.6 an event with the setup in; "F: —" where Part 2 will fill) · `before
  a clearance?` (yes/no: whether the request can be made by a browser that has passed no widget) · `script-able
  without a browser?` · `instrument` (the cell or the reading that measures it). **The section's first paragraph**
  carries the harness caveat in the spec's words.

- [ ] **Step 1: The fence cells, red** (`door_unmetered.spec.ts`, the worker ON, no sign-in): (a) the door loaded
  cold: every same-origin request's path is in the release's precache list or is one of `/`, `/sw.js`,
  `/shell.bin`; none starts `/api/` and none is `/enter`; (b) a local bot game played to game over with no sign-in:
  no `/api/` request until the screen says the game is over, and after it at most `/api/health` and `/api/archive`
  (the upload's own two); (c) against the local binary, `fetch("/nope.json")` from the page answers 404 with the
  404 page's title, and a navigation to `/nope` in a context the worker CONTROLS answers the shell (the worker's
  rule 2), while in a fresh uncontrolled context it answers the 404 page **(Review Focus 5)**. Run: `cd web &&
  npx playwright test door_unmetered` → FAIL (the spec does not exist).
- [ ] **Step 2: The spec.** Written on `plays_locally.spec.ts:822`'s shape (the API-refused first screen) and
  `offline.spec.ts`'s controlled-context helpers (`controlled(page)`). → PASS; `npm run e2e` whole → PASS.
- [ ] **Step 3: The ledger's cell, red** (`lane_document.rs`): the section is absent → FAIL. Then §16 written with
  every route as a row (the layer's `routes()`, the Worker's own `/enter` GET and POST, `/api/art`, the streams
  `/api/games/<id>/stream` and `/api/stream`, `/api/admin/*`, and the presence files and the 404 page as
  NON-metered rows marked "static"), the columns as the Interfaces say. → PASS.
- [ ] **Step 4: The probe script and its cells** (`probe-burst.test.mjs`, no network): argument parsing (`<origin>
  [--count N] [--pace ms] [--paths a,b,c]`), the two header shapes (a browser User-Agent with `Sec-Fetch-Mode:
  navigate` and `Accept: text/html`; a bare one), the TSV row formatter (`path`, `shape`, `status`, `cf-mitigated`,
  `cf-cache-status`, `cf-ray`, `content-type`, `bytes`) over a fake response; the default paths `/.env`,
  `/wp-login.php`, `/admin`, `/xmlrpc.php`, `/robots.txt`, a random `/<hex>.json`; redirects never followed. The
  script is orchestrator-only and says so in its header; it sends nothing in any cell.
- [ ] **Step 5: The pinned instrument.** Copied from E's ledger; `createRequire(new URL("../package.json",
  import.meta.url))`; the usage header; a node cell that running it with no arguments exits 2 with the usage line.
  `.gitignore` gains `web/scripts/pinned/`.
- [ ] **Step 6: The per-edit gates and the commit.**

```bash
git add web/e2e/door_unmetered.spec.ts web/e2e/every_spec_declares_its_worker.spec.ts worker/scripts/probe-burst.mjs worker/scripts/probe-burst.test.mjs web/scripts/pinned-browsers.mjs .gitignore docs/web-lane.md crates/yokohama-web/tests/lane_document.rs
git commit -m "test(web,docs): the door-unmetered fence cell (the door and a local game make no server request before the upload; a miss is the 404 page); the metered-action ledger with its route-coverage cell; the probe and pinned-browser instruments committed (R-F5; spec §2.5)"
```

---

### Task 9: E's deferred minors — Sonnet 5.5 (spec §2.8; E's close §6)

**Files:**
- Modify: the three replay-scrubber cells (`grep -n scrubber web/src/App.test.tsx web/src/Surfaces.test.tsx
  web/src/Board.test.tsx` — E's ledger names the three; each `{ timeout: 5_000 }` or `5000` on those cells becomes
  `10_000`, and a comment says why: 5.9–6.1 s under the sequence's load, green alone); `web/e2e/state1_game.spec.ts`
  (E's close §6's four: m-4 the e2e temp directory made at config load → made in `test.beforeAll`; m-5 the lobby's
  unreachable-server card → an explicit assertion that it is NOT drawn after sign-in; m-6 the final asserts that
  cannot fail → real assertions over the archived record's event count; m-7 the `afterEach` fallback DELETE sent
  anonymously under the open door → sent with the claimed name the walk used); `crates/yokohama-web/src/api.rs:2189–2196`
  (the comment's "five in `CLAUDE.md`" → "a third count in an earlier document"; **this re-stamps the Worker's picker
  code, by design** — say so in the report); `README.md:271–275` ("an open seat for anyone with the link" → "an open
  seat for anyone who has the link and can pass the deployment's door") and its two long PROSE lines over 200
  characters (`awk 'length>200' README.md` — table rows and link lines are not prose; wrap the prose ones);
  `docs/tools/guidingpath-relocate.py:202–245` (`check`'s docstring and the `Fail` message: "a line past the plan's
  end — expected once the tree has moved on after the relocation; the proof is `check --at 054b1e4f`").
- Do NOT touch: anything else. A Sonnet implementer that finds a fourth scrubber cell or a fifth state-1 item STOPS
  and asks.

- [ ] **Step 1: The scrubber cells** raised; `cd web && npx vitest run src/App.test.tsx src/Surfaces.test.tsx
  src/Board.test.tsx` green.
- [ ] **Step 2: The state-1 spec's four**, each red-first where a behaviour is asserted (m-5, m-6, m-7); `npx playwright
  test state1_game --list` compiles; a local dry run (`YOKOHAMA_WEB_STATE1_GAME=1` against the local binary, as E's
  ruling 10-c did) green.
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
(Task 15.A), as E's R-E17 ruled: the document tasks add doc-pinning cells after the deploys, and the fences prove the
gates' inputs byte-identical at every commit. The ledger is `L=.superpowers/sdd/2026-09-30-web-frontend-m6.5c2f-plan`.

**A. The branch is whole**

- [ ] **A.1** `git status` clean; `progress.md` holds a DONE line and a review line for each of Tasks 1–9; HEAD is Task
  9's reviewed commit.
- [ ] **A.2 The fences**, output quoted in the close: the six fenced crates' diff against `15de8117` prints nothing but
  Task 3's one-line `eligible` visibility change in the CLI, if made (named as the seam it is); `SEMANTICS_REV` 7;
  `git diff --stat 15de8117 -- web/src/wire web/src/wire-script` lists ONLY what Task 3 regenerated; no bot behaviour
  moved; **the door-unmetered fence**: Task 8's spec green.

**B. The review (Fable), one fix wave, the scoped re-reviews**

- [ ] **B.1** One **Fable** reviewer over `git diff 15de8117..HEAD`, the spec, this plan and the ledger, in an EXPORT:
  the vacuity lens first (every new catcher reverted against its fix); **the door's auto-apply tried on a non-door
  route** (the cell must catch it); the resume predicate tried with drift ignored; the sweep tried with the listing
  after the delete; the 404 page tried out of the assets tree; the guard's check 12 tried with `on` and a `false`
  stamp; the fetch handler tried with the retry unbounded; the feed comparison tried with `>=`; the sentences this
  plan and the tasks' reports wrote about themselves TRIED (the top yield every cycle); the fences. Report:
  `$L/final-review.md`.
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
| 2 | the door auto-applies off the door | Task 1's `isDoor` cell (Review Focus 1) |
| 3 | the auto-apply fires twice in one page life | Task 1's once cell (Review Focus 1) |
| 4 | the guard accepts `cache.enabled: true` under `env` | Task 1's check-10 cell |
| 5 | `isNewer` uses `>=` | Task 2's equal-version cell |
| 6 | the feed fetched with `releaseFeed()` null | Task 2's never-fetched cell |
| 7 | `runs_here` ignores version drift | Task 3's drift cell (Review Focus 2) |
| 8 | `runs_here` compares to production again | Task 3's flipped `lookahead-v1` cell (Review Focus 2) |
| 9 | the sweep deletes before listing | Task 4's order cell (Review Focus 3) |
| 10 | a TTL without an archive starts | Task 4's refusal cell (Review Focus 3) |
| 11 | `open` waits the table TTL | Task 4's `Waited` cell |
| 12 | `404.html` dropped from the tree | Task 5's check-11 cell |
| 13 | `on` accepted with `turnstile_sitekey: false` | Task 5's check-12 cell |
| 14 | `robots.txt` allows `/api/` | Task 5's presence cell |
| 15 | `404.html` precached | Task 5's exclusion cell |
| 16 | `health` deleted from the destinations again | Task 5's access-paths cell |
| 17 | the fetch handler retries without bound | Task 6's two-challenges cell (Review Focus 4) |
| 18 | the loader called at mount | Task 6's button cell |
| 19 | the `online` drain pops the widget | Task 6's outbox cell |
| 20 | `siteverify` accepts a hostname mismatch | Task 7's unit cell |
| 21 | `on` accepted without the secret | Task 7's config cell |
| 22 | POST `/enter` under `off` answers 302 | Task 7's pool cell |
| 23 | the door makes one `/api/health` call | Task 8's fence cell (Review Focus 5) |
| 24 | a route missing from §16 | Task 8's ledger cell |

**D. The per-edit set and the e2e at the code head**, in the foreground or polled: every Rust suite, the `identity::`
cells, the `wasm32` check, clippy, both rustdocs, the four bridge node suites, the Worker's four commands (the debug
module first), the client's four; `npm run e2e` last. Every reading against Task 1 Step 0's baseline and the tasks'
readings; a red cell re-runs ALONE before it is believed. The readings go to `$L/code-head-readings.md`.

---

### Task 11: The live readings against F's release — the deploys, the update reading, the NAS sweep, the tally, then the public window and the revert — ORCHESTRATOR (R-F7; spec §3.3, §2.6; R-FP4, R-FP10)

No code. Every step is the orchestrator's, at discretion (R-F7), over WARP as the owner where a signed-in reading is
needed and through the credential where a claimed identity is (`worker/RUNBOOK.md` §4's header file, mode 600,
deleted after). ONE UTC day (R-FP10). The transcript of every account step is `$L/live.md` and `$L/window.md`; every
count from the Workers Observability query API is a `calculations` `count` over a named minute window
(`worker/RUNBOOK.md` §7; every ad-hoc body needs a `queryId`); a fresh `wrangler tail --format json` per live run,
its `.err` read for "Reconnecting" before a count is believed.

- [ ] **A. Pre-flight.** `curl -A 'Mozilla/5.0' https://yokohama.moore.nyc/api/me` names the owner; `cd worker && npx
  wrangler whoami`; `df -h`; **the quota guard's first reading** (HOLD above 80,000). **The widget confirmed,
  metadata only**: `WRANGLER_LOG_SANITIZE=true npx wrangler turnstile widget get "$YOKOHAMA_TURNSTILE_SITEKEY" --json |
  jq '{mode, domains, clearance_level, bot_fight_mode}'` (the key from the environment, never typed into the
  transcript; the secret field NEVER selected) — mode `managed`, the Worker's hostname among the domains,
  pre-clearance at `managed`; anything else is set right in the dashboard before G (the orchestrator says what it
  found). **Prepare the two pinned browsers BEFORE any deploy**: `node web/scripts/pinned-browsers.mjs worker
  https://yokohama.moore.nyc web/scripts/pinned/worker` and the same for the NAS — each at E's release until "Ready to
  play offline"; and a THIRD context with a local game open (Review Focus 1's game tab). They stay open.
- [ ] **B. The production release build, ONCE, WITHOUT the site key** (R-FP4): `cd web && YOKOHAMA_PROJECT_URL='https://github.com/jmoore2333/yokohama-assistant/'
  YOKOHAMA_SITE_NOTICE='Not affiliated with Asmodee' YOKOHAMA_SITE_BLURB='<the blurb both hosts carry>'
  YOKOHAMA_SITE_URL='https://yokohama.moore.nyc' npm run build` (the feed derived; `grep -c debug_trap … ` 0; the
  precache's lines recorded; `dist/build-values.json` read: `turnstile_sitekey: false`, `site_url: true`,
  `release_feed: true`), then `cd ../worker && npm run build`. **The NAS build is its own** (`YOKOHAMA_SITE_URL='https://yokohama-nas.moore.nyc'`,
  no key) through E's `nas-build.sh` with the tag `yokohama-web:0.6.0-m6.5c2f`.
- [ ] **C. The Worker's deployment** (`npm run deploy`, the tail listening): the guard's door line, its check-10, -11
  and -12 lines, the assets uploaded (`404.html`, `robots.txt`, `llms.txt`, `sitemap.xml`, `build-values.json` among
  them), the `vars` with `YOKOHAMA_TURNSTILE ("off")`, the secrets inherited. §5's readings: `/api/health`,
  `/api/me`, `/sw.js` naming F's release, `/robots.txt` 200 `text/plain` `max-age=3600`, `/llms.txt`, `/sitemap.xml`,
  `/404.html` 200 with `noindex`, `/nope.json` — under Access this meets the login; recorded as such (the 404 reading
  is the window's).
- [ ] **D. THE UPDATE READING on both hosts** (Review Focus 1): in each pinned context, ONE plain reload of the door
  → the new `sw.js`, the install from `/shell.bin`, the new worker waiting, then the DOOR RELOADING ITSELF ONCE (the
  instrument's `after-1.json` shows the reload without a `/click` file) and "Ready to play offline" under F's
  release, the old generation retained; the third context (the game tab) after a `visibilitychange`: the line
  shown, the game unchanged, no reload; then its click → the same game continues under F. The NAS's reading after
  E: the same, in the NAS context. A failure is a STOP.
- [ ] **E. The NAS's sweep reading, then its seventeenth deployment.** `$L/nas-sweep.md`: a copy of the volume
  (`docs/handover.md`'s pull recipe: the Portainer archive endpoint) into a scratch volume; a scratch container from
  the new image with `YOKOHAMA_WEB_TABLE_TTL_DAYS=1`, `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS=1`,
  `YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive`, `YOKOHAMA_WEB_IDENTITY=open`, bound to loopback on the NAS only; after its
  first pass (one minute) the log's `sweep-listed` lines BEFORE its `sweep-ended` lines, the archive directory's new
  files, `/api/health` reading `1` and `1`; the container and the scratch volume removed BY NAME. Then the swap
  (E's `nas-swap.sh` with the tag and the two TTLs at the Worker's values, 90 and 7 — a `Ruling:` line if the owner
  names others): the readings (`/api/me` names the owner — a STOP otherwise; `/api/health` with the TTLs;
  `/robots.txt`; `/nope.json` 404 with the page); the first real pass's listing recorded verbatim.
- [ ] **F. The live tiers under Access with the dev door** (the quota guard first): the live replay TWICE, sequentially;
  a FRESH tail, then `YOKOHAMA_WEB_TALLY=$L/tally.jsonl YOKOHAMA_WEB_E2E_TARGET=live npx playwright test two_browsers
  joins_by_link`; `node web/scripts/tally-report.mjs $L/tally.jsonl` → the per-route figures for the ledger; the
  query API's requests and rows for the window.
- [ ] **G. THE PUBLIC WINDOW** (R-F7; spec §2.6; E's flip recipe extended), in this order, each step a dated line:
  1. the quota guard;
  2. **a client build WITH the site key** (`YOKOHAMA_TURNSTILE_SITEKEY` from the shell's environment, the other
     values as B) and `cd worker && npm run build`; `build-values.json` reads `turnstile_sitekey: true`;
  3. the operator secret DELETED (the `open` shell refuses to start beside it); **`TURNSTILE_SECRET` put from the
     repository root's `.dev.vars` line through stdin, never printed**: `grep '^TURNSTILE_SECRET=' ../.dev.vars |
     cut -d= -f2- | npx wrangler secret put TURNSTILE_SECRET`; `wrangler secret list` names it;
  4. `npm run deploy:open` — the guard's door line, its check-12 line (`on` with a keyed build), the version;
  5. the Access application's destinations PUT WITHOUT `yokohama.moore.nyc` (the three PUTs' discipline: the `aud`
     read back unchanged);
  6. **the WAF rule** created through the API in the zone's `http_request_firewall_custom` phase, expression
     `(http.host eq "yokohama.moore.nyc" and (http.request.uri.path eq "/enter" or starts_with(http.request.uri.path, "/api/")))`,
     action `managed_challenge`, description `yokohama — the no-Access door's edge check (F)`, ENABLED; its id
     recorded;
  7. the readings, each in its own named minute so the query API can slice it:
     - **G.a the probe burst** (Review Focus 5): `node worker/scripts/probe-burst.mjs https://yokohama.moore.nyc
       --count 200 --pace 500 > $L/probe/burst.tsv`; every unknown path 404 with the page's bytes (`cf-mitigated`
       absent, `cf-cache-status` recorded); the query API's stateless count for that minute → **zero**; then the
       control: one `/api/health` from a cleared headed browser → the count for its minute reads one;
     - **G.b the edge**: `curl -sD - -o /dev/null https://yokohama.moore.nyc/enter`, `…/api/health`, `-X POST …/api/archive`
       → 403 with `cf-mitigated: challenge`, zero invocations in that minute;
     - **G.c the owner's path** (headed Chromium on the workstation, no WARP; the owner's own Edge with WARP off as
       the second reading): the door → "Go to the lobby" → the widget → the lobby's name step → a table → a game of
       a few moves; the query API shows the invocations of that game and nothing before the widget;
     - **G.d the link arrival**: a fresh headed context opening the table's link → the overlay once → the table;
     - **G.e the upload**: a fresh headed context, a local bot game to game over → the overlay → `archive-landed`
       in the tail;
     - **G.f the outbox**: a fresh headed context, a local game finished OFFLINE (`context.setOffline(true)`), the
       network restored → the `online` drain marks the row ("waiting for a human check" on the door) → the door's
       launch (a reload) → the overlay → landed;
     - **G.g the update reading under the open door** in a fresh pinned context (static, no login): as D.
  8. **the revert**: the rule DISABLED by its id (kept for the public end state); a client build WITHOUT the key and
     `npm run build`; `npm run deploy` (the worker door, `off`, the dev door on); `YOKOHAMA_WEB_ADMINS` put back
     from `git config user.email` through stdin; the three hostnames back on the application, the `aud` unchanged;
     the after-check (`/api/me` over WARP names the owner; a clean headed browser without WARP meets Access's login;
     `wrangler secret list` names the three secrets); ONE live replay HOLDS; the quota after.
- [ ] **H. The figures** for Tasks 12–14: the ledger's per-route counts (F), the probe's zero and its control, the
  window's minutes, the NAS's listing, the update readings' sentences.

---

### Task 12: The lane document, `docs/web-lane.md` — Opus (spec §2.9; R-F5, R-F6, R-F10)

- [ ] §1's state paragraph (F's release on both hosts; the window vetted and reverted; the rule disabled by id); §2's
  counts at F's code head (Task 10.D's readings, named); §5's traps gain: the `cf-mitigated: challenge` fetch (a
  challenge page cannot be solved by a fetch; the handler is the client's one site), the probe (a miss without a
  404 page is an invocation), one build per door state (the guard's check 12); §8's preconditions table: Turnstile
  and the rule as the no-Access shape with the window's readings, `/api/health` closed, rows 108 and 119 still owed;
  §10's operations: the window's recipe (the eight steps of Task 11.G with the rule's id), the probe and
  pinned-browser instruments, the NAS sweep reading; §11 F's row; §12's variables: the five new names with their kinds
  and readers, the two TTL rows' host column (if Task 4 or 7 did not already); §13's map: `POST /enter`; §15 by
  design: no NAS marker, the TTL-without-archive refusal, the in-game click line, one build per door state, the
  feed's silence while the repository is private; **§16 the ledger completed** with Part 2's figures and the harness
  caveat. The two drift cells green. Commit: `docs(web-lane): cycle F — …`.

---

### Task 13: `CLAUDE.md`, the board, guidingpath, the AI lane's re-entry line — Opus (spec §2.9; R-F4)

- [ ] `CLAUDE.md`: the counts in place naming F's reading; in the web lane's fence sentence a fourth clause, "the door
  unmetered (a door visit, a local game and any miss cost zero Worker invocations; the first metered request is
  `/enter`, a link arrival's first read or a finished local game's upload)"; in "Two facts cross into the AI lane" a
  third: "a build that reconstructs a game's exact recorded bot plays it on; only an unknown, ineligible, re-knobbed
  or drifted identity refuses (R-F4)"; the pointer to F's close beside E's. `docs/handover.md`: the Updated line;
  Track 3 at F (the merge and the tag on the owner's word); the entry-artifacts line. `guidingpath.md`: F's roadmap
  row and the decision-log rows (R-F4 amending R-6.5c2.10 forward, with its identity-freeze reason; the
  door-unmetered fence; Turnstile as the no-Access shape behind a switch; the version stays 0.6.0).
  `docs/plans/2026-09-11-ai-lane-reentry-handover.md`: one line under its open items: "a V5 ship no longer cuts off V4
  games on the web (F, R-F4); a NEW `Contestant` or `NamedIdentity` variant still needs its one-line label in
  `bots.rs`". Commit, documents only.

---

### Task 14: The README, the self-host guide with `deploy/update.sh`, both runbooks' records — Sonnet 5.5 (spec §2.9; R-F2, R-F9, R-F12)

- [ ] `README.md`: the getting-started and defaults from the lane document's §14; one sentence on the resume rule; the
  web presence (what a public deployment gets for free); the self-host update pointer; the release line left
  conditional for Task 15.H. `deploy/README.md`: §3's build arguments table with the five values (the feed's three
  states; the site URL's effect; the site key's "leave it blank behind Access"); §4's rows as Task 4 left them; a new
  "§9 Updating" — fetch the tag, rebuild, smoke, swap, what players then see (the door updates itself; a game in
  play shows the line) — and `deploy/update.sh`: `set -euo pipefail`; `TAG=${1:?a release tag}`; `git fetch --tags
  && git checkout "$TAG"`; `docker build --platform linux/amd64 -f deploy/Dockerfile <the --build-args from an env
  file the operator names in YOKOHAMA_WEB_BUILD_ENV> -t "yokohama-web:$TAG" .`; `bash deploy/smoke-image.sh
  "yokohama-web:$TAG"`; then `docker stop`/`rm`/`run` with `--env-file "$YOKOHAMA_WEB_ENV"` and the volume; a
  `--dry-run` that prints the commands; never `rm -rf`. `deploy/RUNBOOK.md` §3.20 (the seventeenth deployment from
  Task 11.E's transcript) and `worker/RUNBOOK.md` §20 (F's deployments, the window with the rule's recipe and id, the
  probe's figures, the secret by name) and its §3 rows if Task 7 left any; `worker/README.md`'s table. Commit,
  documents only.

---

### Task 15: The close, F's PR, and the wrap — ORCHESTRATOR (spec §4; R-F7, R-F13; R-FP11)

- [ ] **A. The ONE recorded sequence at the FINAL reviewed head** (R-FP11): Tasks 12–14 reviewed and landed; E's
  `close-sequence.sh` copied into `$L` with `L=` set and the e2e per-spec row added; ONE uninterrupted run under
  `caffeinate -i`, machine-tagged, polled with `tail -n 3 "$L/sequence.tsv"`; every `rc` 0; the fifteen gates' walls
  summed beside E's 2,881 s; a red row re-run ALONE before it is believed; each count set against Task 10.D's readings.
- [ ] **B. The close document** `docs/plans/<the day>-web-frontend-m6.5c2f-close.md` on E's shape: the header; the
  headline and what is LIVE; §1 by task with the fences; §2 the sequence; §3 the deployments, the update readings, the
  NAS sweep, the tally, the window (the probe with its control, the edge readings, the six browser readings, the
  revert's after-check), the quota; §4 the reviews and the battery; §5 every ruling R-F1–R-F13, R-FP1–R-FP11 and every
  ledger `Ruling:` with its cost; §6 what the lane leaves; **§7 what this cycle does NOT claim** — at least: the fully
  open end state (vetted, reverted); the real widget outside the window; the feed's notice live; a public load; the
  handsets; rows 108 and 119; §8 the wrap. **Then** the spec's `§10.14 Corrections recorded in execution — cycle F`;
  the guidingpath row's execution half; `CLAUDE.md`'s counts if the final head moved them; the board's Updated line;
  the lane document's §11 row's close pointer. Commit, documents only.
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
`15de8117`, Step 0's baseline, R-FP1–R-FP11, one line per task, one `Ruling:` per deviation); `task-<n>-brief.md`,
`-report.md`, `-review.md`; `task-<n>-e2e.txt`; `task-1-look/`, `task-2-look/`, `task-6-look/` (the pictures at 1280
and 390 px); `task-9-e2e-walls.txt`; `final-review.md`, `fix-wave-*.md`, `battery.md`, `code-head-readings.md`;
`live.md`, `window.md`, `nas-sweep.md`, `probe/`, `tally.jsonl`; `nas-build.sh`, `nas-swap.sh`, `worker-deploy.sh`,
`close-sequence.sh`, `sequence.tsv`, `sequence/NN.log`; `pr-body.md`. E's ledger (`…-m6.5c2e-plan/`) holds the
scripts' originals and the pinned instrument's origin.

---

## Review record

- **The self-review at writing (2026-09-30, the orchestrator):** spec coverage — §2.1 Task 1, §2.2 Task 2, §2.3 Task 3,
  §2.4 Task 4, §2.5 Tasks 5 and 8 (the probe in 11.G), §2.6 Tasks 6, 7 and 11.G, §2.7 Task 5, §2.8 Task 9, §2.9 Tasks
  12–14, §4's wrap Task 15; placeholders none; the names table is the type contract (`Foreignness`, `Ttls`, `Ending`,
  the guard's three checks, the client's seams) and every task cites it; Review Focus 1–5 each pinned to a cell in its
  owning task. One coupling found and written while reviewing: Task 7's `enter` arm must move BELOW `shellConfig` in
  `handle()`, since the switch is read from the configuration, which the pool cell for `on`-without-secret
  (every request 500) exercises.
- **The adversarial Opus review of the spec, this plan and the approach, and its fix wave (2026-09-30):** recorded
  below by the orchestrator once run — the findings by severity, each with its disposition (fixed in the spec, fixed
  here, or declined with the reason), and the reviewer's own "does NOT claim" list.
- **The owner's review:** pending.
