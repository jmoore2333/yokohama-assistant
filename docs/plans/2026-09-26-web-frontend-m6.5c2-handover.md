# M6.5c2 — the cleanup cycle: entry handover

**Written 2026-09-24 to 2026-09-26 at the close of M6.5c "the objects", for the session that BRAINSTORMS, SPECS
and PLANS the cleanup cycle** — the ONE cycle the owner placed between M6.5c and the refocus on the
computer player (R-6.5c.4: "the NAS/Docker host is reconciled with the Worker in a cleanup cycle
AFTER M6.5c, and then the project refocuses on the computer player"). This document is the record
of what M6.5c leaves, in the owner's own words where the owner gave them. **It is NOT a spec: every
product decision below is the brainstorm's to take**, and where it and a frozen artifact disagree,
the artifact wins.

---

## 0. Read order

1. **This file.**
2. **The M6.5c close**, `docs/plans/2026-09-26-web-frontend-m6.5c-close.md` — §3 (the deployments,
   the live tiers' ending state, the exit, what a game costs), §5 (every ruling with its cost), §6
   (the LEAVE list and the OWNER ITEMS), §7 (what the cycle does NOT claim).
3. **The M6.5c spec**, `docs/plans/2026-09-22-web-frontend-m6.5c-objects-spec.md` — §1 (R-6.5c.1–16,
   R-6.5c.4's host-local rule above all), §2 (the mechanism), and **§10, the corrections recorded in
   execution**: read §0–§9 through it.
4. **`deploy/RUNBOOK.md`** — §6 (the Worker: build, deploy, variables, the credential, reading it,
   rollback, the deployment records §6.8 on) beside §0–§3 and §5 (the NAS). Both halves are what
   this cycle arranges.
5. **The whole-branch review's four reports**, gitignored in
   `.superpowers/sdd/2026-09-23-web-frontend-m6.5c-plan/` — `final-review-fable-notes.md`,
   `final-review-r1-report.md` (its "For the M6.5c2 cleanup cycle"), `final-review-r2-report.md`
   (the battery), `final-review-r3-report.md` (its §7) — read for their method and for §4.6 below.
6. **The umbrella** `docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` §1 (R-6.5.1–13):
   what "the NAS stays" meant when the hosting move was chartered.
7. **`CLAUDE.md`** — the operating facts and every gate command. **Re-count every suite at your own
   HEAD.**

---

## 1. Where things stand

- **Branch `web-room-m6.5c`** at the docs commit that follows `754b80ae`, **PR #{{TBD: number}}** against
  `web-room-m6.5b2`: eight pull requests stacked from #31 against master, nothing merged, **the merge
  order is the owner's**. The code close head is `754b80ae`; its shipped code is `bd8ce497`'s, since
  the three commits after that (`f2c27a3f`, `528b1285`, `754b80ae`) move only the exit spec and its
  harness. Where this cycle's branch starts — stacked on
  `web-room-m6.5c`, or after merges — is the owner's call, asked at the brainstorm.
- **LIVE on Cloudflare:** `yokohama-web` version `6da906a5-a088-442e-bc43-d94904630982`, the twentieth
  deployment (2026-09-26 06:38:27 UTC), at `yokohama.moore.nyc`, behind the hostname-wide `yokohama` Access application — archive `r2` (the bucket `yokohama-archive`), TTL
  90 days, the operator list set as a secret, the dev door ON, `YOKOHAMA_WEB_COUNT_WAKES` off.
- **LIVE on the NAS, untouched by M6.5c:** `yokohama-web:0.5.0-m6.5b2.1` at
  `yokohama-nas.moore.nyc` (RUNBOOK §3.16) — `claimed`, SSE, the directory archive, no TTL.
- **The bucket** holds this cycle's `test/` saves, which the owner KEPT (§4.5); as far as the ledger
  records, every save M6.5c made sits under `test/`.
- **The fences held**: the five fenced crates, the CLI and `yokohama-wasm` byte-identical to
  `2621fbe`; the layer changed by exactly two items; the corpus byte-identical; `SEMANTICS_REV` 7.
- **The live tiers' ending state** is the close's §3.2: nothing is owed. The two-browser re-run on
  the fixed client PASSED on 2026-09-26, and the exit's third run PASSED with the object hibernating in
  each of its four pauses (§3 below).
- **A two-day pause** (2026-09-24 ~10:58 UTC → 2026-09-26 ~06:04 UTC, the account's weekly limit)
  stopped every agent mid-task; each was woken by message at the reset, never respawned, as the owner
  asked. After it, the WARP client read Connected but its device's Access login had EXPIRED, so every
  request from the workstation without the credential answered Access's 302 ("Please authenticate via
  the warp client"); the owner re-authorized it on the 26th.

---

## 2. The owner's direction (quoted, so it is searchable)

- **2026-09-24 ~07:00 UTC, the scope of this cycle:** *"tidying up, arranging the docker vs
  cloudflare setup in the repo, finalizing that documentation and construction, and testing
  strategy to perhaps start with that at the right time to handle the full test suite as we wrap
  this up with the whole set of modules and docs and repo strategy and frontend design tweaking."*
- **The same message, on the live tier:** *"If we won't be using the full test now we should know
  the consequences and still be comfortable through either the headed test or otherwise our ending
  state in the report."* — answered by the close's §3.2; the testing strategy (§4.4) is where it
  becomes standing practice.
- **2026-09-24 ~07:50 UTC:** the `test/` saves of M6.5c REMAIN in the bucket, with no lifecycle
  rule now; this handover carries the choice — *a periodic script, or an R2 expiry after N days on
  the `test/` prefix* — with a best-practice recommendation, decided here (§4.5). And: *"Otherwise
  seems this is ready to move forward."*
- **2026-09-26 ~06:3x UTC, at the close-out:** clear to deploy, push and open the PR. On CI: *"CI
  will likely need to be updated, optimizing for what's reasonable to have in github vs being part
  of the local work."* On the README: *"a comprehensive update to the readme.md … or a detailed
  readme for the cloudflare version that is linked going through in high detail the reasoning and
  mechanics of the durable object/WASM/cloudflare worker optimization."* A docs and CI agent was
  dispatched at the close for `worker/README.md`, the root `README.md` and
  `.github/workflows/test.yml`; this cycle starts from what that work leaves (§4.1, §4.4).
- **2026-09-22, the lane's direction** (the M6.5c brainstorm, recorded in R-6.5c.4): *"we are trying
  to not make changes to other modules to not increase debt and complexity where it's unneeded"* —
  refined the same day: *"yokohama-web is new within the broader cycle and isn't being used anywhere
  in production, so if we find an architectural issue that we should or need to address, we can."*
  After this cycle, `CLAUDE.md` and `guidingpath.md` are refreshed and the project refocuses on the
  computer player.

---

## 3. What M6.5c's live tiers left: nothing owed

The one live step M6.5c deferred, the `two_browsers` re-run on the fixed client (I-82), **ran on
2026-09-26 and PASSED** (06:12–06:22 UTC, deployment 19). Game `sim-1a8263683952c0b4` finished in
2,510 moves in 601 s. The watcher held to 1,886 applied moves, asked `/events` 0 times and opened
`/stream` ONCE at every checkpoint; each player opened the table's stream twice, both before the
first move. No page logged a console error but the analytics beacon's two forgiven lines: the first
live run's `CLOSING` lines are gone, and no 426 appeared. The exit's third run passed the same
morning: forty paced moves and four `wake` lines, one after each 60 s pause, at 6–9 ms of CPU each.
The records are the M6.5c close's §3.2–§3.3 and the RUNBOOK's record of the nineteenth deployment.
**This cycle starts with no live step owed.** A live run in it follows §4.4's rules: WARP's device
login checked first, the quota guard, and one Playwright run at a time.

---

## 4. The scope, by the owner's list

### 4.1 Tidying the repository

The lane's own footprint, not the public split (the board's Track 2 has its own brainstorm):
- **Two hosts, two directories, one runbook**: `deploy/` holds the NAS/Docker host (`Dockerfile`,
  `Dockerfile.runtime`, `Dockerfile.dockerignore`, `cloudflared-config.yml`) AND the whole
  `RUNBOOK.md`, whose §6 is the Worker's; `worker/` holds the Worker's package, its scripts and its
  two wrangler configurations, but none of its operating document (§4.2).
- **Stale sentences outside the frozen records**: at M6.5c's code head, README's Hosting paragraph
  named `yokohama.moore.nyc` as the NAS's and `0.5.0-m6.4` as the build, and `guidingpath.md`
  carried no M6.5c row. The owner's close-out direction (§2) sent a docs agent to the root
  `README.md` and a new `worker/README.md`: read both as that work leaves them, and check
  `guidingpath.md` for an M6.5c row before writing one.
- **The board's history**: `docs/handover.md` still appends "previous update" paragraphs back to
  2026-09-11; the refresh after this cycle was always to rewrite them into pointers (the CLAUDE.md
  consolidation record says so).
- **Test code the reviews named for tidying** — §4.6's list.

### 4.2 The Docker/NAS setup against the Cloudflare one: what lives where, what each host owns

| concern | the NAS / Docker host | the Cloudflare Worker | shared |
|---|---|---|---|
| the code | `crates/yokohama-web` with its `server` feature (axum); `deploy/Dockerfile*` | `worker/` (the TypeScript shell) over `crates/yokohama-worker` (the module) | the layer — `crates/yokohama-web` without `server` — and the `web/` client, one `dist` for both |
| identity | `claimed` (the `dev-identity` feature) behind Access | `cf-access`: the Access assertion VERIFIED; the dev door (the credential plus a claimed header) | inside the objects the layer's `worker` mode reads `X-Yokohama-User` |
| transport | SSE (`signal: "sse"`) | hibernatable WebSockets (`"ws"`) carrying the SSE frames' data | the frames |
| storage | `FsStore` on a Docker volume | SQLite in each Durable Object | the layer's `Store` trait |
| the archive | `DirArchive` under `YOKOHAMA_WEB_ARCHIVE_DIR` | R2 through the receipt, behind `YOKOHAMA_WEB_ARCHIVE` | `archive::paths`' layout: the CLI loads either |
| expiry and the operator | none; Portainer and the volume | TTL 90 and the marker; `/api/admin` behind `YOKOHAMA_WEB_ADMINS` (a secret) | — |
| configuration | the container's environment | `worker/wrangler.jsonc`'s `vars` and the secrets; `wrangler.test.jsonc` for the pool | the chat variables' names and defaults |
| hostname and door | `yokohama-nas.moore.nyc` through cloudflared | `yokohama.moore.nyc`, a custom domain | the ONE `yokohama` Access application |
| operating document | RUNBOOK §0–§3, §5 | RUNBOOK §6 | — |
| tests | `cargo test -p yokohama-web`; the local e2e against the axum binary | `cargo test -p yokohama-worker`; `worker/`'s node cells and pool; the live tiers | `wire_fixtures` and the corpus; `test:wasm`'s node replay |

**Questions this cycle's spec takes first** (questions, not answers):
1. **What the NAS is for now.** A deployed host beside the Worker, or the self-host path only (M6.5b's
   "a user running locally in Docker is a goal")? Spec §3.4 says the cleanup "reconciles the two hosts,
   and may give the NAS a TTL".
2. **Which of the Worker's additions the NAS takes**, if any: a TTL and a marker, an operator
   surface, `cf-access` in place of `claimed`. Each is a layer-or-host question under R-6.5c.4.
3. **The runbook's shape**: split per host (R3's §7 — about 2,900 lines covering two hosts) —
   e.g. the NAS's beside `deploy/` and the Worker's beside `worker/` — with the deployment records
   kept verbatim wherever they land.
4. **The two images**: the self-host image was not rebuilt during M6.5c although the workspace gained a
   member (R3's E-3); whether this cycle rebuilds and re-measures it.
5. **The public door's preconditions**, recorded for the step after: the dev door off; the path-scoped
   Access application; the art's publication; `DEV_DOOR_CLIENT_ID` as a secret (§5 item 9).

### 4.3 Finalizing the documentation and its construction

- **`CLAUDE.md`**: about 36 KB of its 92 KB is the web lane's operating detail; the consolidation
  record's option — move it into a lane document once the lane goes quiet — is this cycle's to
  take or leave. Every count line keeps naming its reading.
- **`guidingpath.md`** (398 KB) and the board: the refresh after this cycle, as the owner ordered it.
- **One place for each figure.** The browser module's sizes live in the root `Cargo.toml`'s
  `release-wasm` comment; the WORKER module has no such place — `worker/src/table.ts`'s `copyOf`
  doc carries release readings that module growth will make false silently (R1's debt 6).
- **The NAS-era RUNBOOK §3 spells the owner's address** in five lines (§5 item 4).
- **Construction**: which documents are generated or checked (the guide pair's `guide_sync`; the
  wire corpus), which are dated records frozen at their close, which are current-state pointers —
  written down once, so the next session does not re-derive it.

### 4.4 A testing strategy: when each tier runs, what it proves, what skipping it costs

The owner asked for this "to perhaps start with that at the right time to handle the full test
suite as we wrap this up". The proposal: **run the WHOLE suite once, as one recorded and
machine-tagged sequence, at the START of this cycle** (the baseline every refactor below is measured
against) **and again at its END** (the lane's closing record) — per-edit gates, the release roster,
the local e2e, both live tiers on a fresh UTC day, and the headed game.

| tier | command(s) | what it proves | when it runs | what skipping it costs |
|---|---|---|---|---|
| Rust, per-edit | `cargo test -p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`; clippy; the `wasm32` check; both wasm builds | the layer and both hosts' module logic, natively; the wire contract (`wire_fixtures`) | every task; CI | a layer or module regression reaches the pool or the browser first, where it is slower to read |
| client, per-edit | `cd web && npm run typecheck && npm test` | the client against the committed corpus, over jsdom (no CSS, no layout) | every task; CI's `web-client` | a client regression against the wire |
| builds, per-edit | `npm run test:wasm` | the corpus through the COMPILED browser module in Node; the service worker; the precache; the live tier's pure cells | every task; CI's `web-server` | a module or precache regression the jsdom cells cannot see |
| the Worker, per-edit | `cd worker && npm run build:module:debug` FIRST, then `npm run typecheck`, `npm run test:scripts`, `npm test` | the shell inside workerd — real SQLite, alarms, sockets, a local R2 — and the whole corpus through the objects | every task touching the Worker; CI's `web-server` | the batch, alarm, slot and receipt rules untested anywhere else; **without `build:module:debug` the pool tests a stale module GREEN** |
| the local e2e | `cd web && npm run e2e` (6.5–15 min; 61 cells) | real Chromium against the axum binary: images, focus, layout, CSS, the service worker, a game to a winner | any task that changes what a browser DOES | a layout, CSS or service-worker regression; not in CI |
| the release roster | the fifteen release gates (`CLAUDE.md`) | the engine, the bot, the oracle, the differential harness | before merges and phase closes; the owner's call for the web lane's merge | nothing while the fence holds — a statement about the fence, not a run; it last ran at the V4 merge head |
| the deploy readings | `worker/scripts/read-deploy.mjs` and the curls (RUNBOOK §6) | the deployed Worker's identity, assets, service worker, protection and CPU | every deploy | a broken deploy found by a player |
| the live replay | `cd worker && node scripts/live-replay.mjs`, twice, sequentially, over WARP (~4,600 requests for two runs) | the whole corpus byte-identical through the LIVE Worker, and repeatable | after a deploy that changes what the Worker serves; the cycle's start and end | a host-only payload difference — `/api/art` was one — seen by nothing but the pool's model of the platform |
| the live Playwright tier | `cd web && YOKOHAMA_WEB_E2E_TARGET=live npx playwright test two_browsers joins_by_link` (measured: ~32,000–38,000 Worker requests and ~1.2–1.34 M object rows read per two-browser game, about 13 requests a move) | two browsers play to a winner on the live Worker, one socket each, no unknown console error | the quota guard first; before a release that changes the client or the transport | the real edge's behaviour — live run 1 found three findings no local run could |
| the headed game | `cd web && YOKOHAMA_WEB_EXIT=1 YOKOHAMA_WEB_E2E_TARGET=live npx playwright test exit_game --headed` (`WAKE_PAUSE_MS`, default 45000, and 60000 in M6.5c's exit runs; `EXIT_SCREEN`, the screen's logical size, default 1280x832 — three windows side by side where three of Chromium's narrowest fit, else a 2×2 grid under the menu bar's inset, every window's bounds read back and any overlap refused), the tail open, the window opened by `--var YOKOHAMA_WEB_COUNT_WAKES:on` and closed by the plain deploy, no other deploy meanwhile | hibernation and the wake OBSERVED live, with chat and a watcher; the per-move wall and CPU; the lobby's Open-tables join, the lobby chat and the watcher's unlock | a cycle's exit; after a change to the wake path | hibernation is never observed live |
| the mutation battery | the spec's rows, at the reviewed head, each reverted | that the named catchers catch | a cycle's close | a catcher that cannot fail stays in the suite |
| the phones | the owner's own devices | WebKit, installation, offline on a phone | the owner's | "installs and runs offline" stays a Chromium claim |

**Two CI facts to settle here**: the Worker's suites ride inside the `web-server` job — a job of
their own would let a Worker-only change run only what it needs; and CI runs the deploy guard's
checks 3–7 over the committed configurations but builds no release assembly, so checks 1, 2, 8 and
9 — the release module, the assets tree and the two freshness checks M6.5c's fix wave added — run
only on the workstation that deploys. And `test:scripts` needs `cargo` on `PATH` since the
freshness digest's `cargo metadata` cell. The live tiers stay local, because they need the
credential. **The owner asked for this at the close-out** (§2): CI updated, *"optimizing for what's
reasonable to have in github vs being part of the local work."* A first pass at
`.github/workflows/test.yml` was dispatched then, so read the workflow as that work leaves it before
settling these facts.

### 4.5 The `test/` prefix's lifecycle — the choice, with a recommendation

**The choice**: (a) a periodic script that deletes the old `test/` objects, or (b) an R2 object
lifecycle rule that expires every object under `test/` N days after it was written.

**Recommendation: (b), an R2 lifecycle rule on the `test/` prefix, N = 30 days.** It is Cloudflare's
own pattern for a TTL'd prefix (its Sandbox backup guide: "add an R2 object lifecycle rule on the
`backups/` prefix"), and this bucket already carries one lifecycle rule (the default seven-day
multipart abort). Why it fits this project:
- **No code, no credential, no invocations.** The rule runs on Cloudflare's side; a script needs a
  WRITE-capable credential — the owner's R2 key is read-only by design — and a scheduler, which is
  either a cron Worker (code, tests and invocations against the shared quota) or a workstation
  habit.
- **The owner's training data is out of reach by construction.** The rule names the `test/` prefix;
  the owner's games sit at the bucket's top (`rev7/…`, `browser/…`). The rule should be read back
  after it is set, so the prefix is seen to be exactly `test/`.
- **Declarative and visible** in the bucket's own configuration, and set once (wrangler's
  `r2 bucket lifecycle` commands or the dashboard; the cycle confirms the exact syntax and the
  deletion latency on R2's "Object lifecycles" page before running it).

**The trade-off.** A lifecycle rule deletes by AGE, not by meaning: it cannot keep "the last K runs"
or a named exhibit; a game worth keeping must be copied out of `test/` before N days. It does not
replace the live replay's RESET RECIPE (the close's §7): a change to `wire`'s records makes the
replay's first sends answer 409 until those four keys go, whatever their age. And deletion is
asynchronous, so "N days" is a lower bound. A script is the right answer only if a retention rule
richer than age is wanted.

### 4.6 The design debts the reviews listed for this cycle

By area; each carries its source. The M6.5c fix wave closed the deploy guard's freshness, the three
jsdom flakes, the socket arm's no-poll pin, the replay's reset, the operator list's echo and the
crate's docs sweep; what it left is below.

**The Worker's shell (`worker/src`)**
- `table.ts` is 2,082 lines with four separable parts: the transport helpers beside `frames.ts`; the
  slot half (~330 lines) as a `delivery.ts`; the archive half (~380 lines) behind a narrow host
  interface; and three copies of the guarded-call pattern (Task 10's M5, Task 12's M9, Task 13's m4).
- **The rows-read cost** (F-4): the three documents are re-read up to three times per request —
  ~470 rows read per move live, the second Free-plan figure to bind, after requests (13.4 a move). Read once per
  request and thread the result.
- The operator listing carries `created_at` in the layer's seconds beside `last_active` in the
  shell's milliseconds.

**The module (`crates/yokohama-worker`) — R1's list**
- The picker invariant is not structural: three exports reach `Games::handle` outside
  `picker::around` — route them through it, or clear the list whenever the record changes.
- The scrub's compile-time fence is one level deep (`TableDoc`, `Lock`, not `SeatDoc::Human` or
  `TurnMirror`).
- `ReceiptArchive.pending` keeps only the LAST deferred plan — a `Vec` or a refusal makes one plan
  per call structural.
- The restatements — `error_body`, `MAX_WAIT_SECONDS`, the house frame, the clock, the platform
  scan (rulings R-02, R-03) — each pinned, one pinned to the layer's source TEXT.
- The layer's `eprintln!` is mute on this host: a logging seam in the layer would let
  `archive_over_retry` report a failed stamp.
- The worker module has no one place for its figures.
- The layer's own `create` accepts a positional-array body (spec §2.2 left it for this cycle); the
  module's `init` refuses the array form since the fix wave.
- No cell reaches `archive_over_retry`'s second `NotFound` arm (Task 1's M6): its home is the layer's
  `games.rs`, fenced during M6.5c.

**The scripts, the live tiers and the documents — R3's §7 and Task 15's deferrals**
- `.dev.vars` is parsed three times (`web/e2e/live.ts`, `worker/scripts/live-replay.mjs`, RUNBOOK
  §6.4's recipe); two origin constants; the replay's five parts (the forces, the base read, the rows,
  the judging, the report) in one 649-line script; `HOST_FIELDS` and `order()` and the credential's
  key names each stated twice; two parsers of the one precache list `web/scripts/precache.mjs`
  prints.
- The POOL's host-field exemption can be spent on an absent field (the presence rule is in the live
  replay only).
- `dealAnswers` is uncapped (ruled); a cap of about two per page keeps the forgiveness exact in count
  as well as shape.
- After the first 404, the outbox's drain goes on posting the remaining rows.
- `read-deploy.mjs` echoes up to 200 characters of a non-200 `/sw.js` body (R3's L-3).
- A live create whose reply the spec never read leaves a table in the shared lobby (R3's L-9).
- The structural live-request cell can still be passed by a computed key held in a variable
  (`page[k].get(…)`) — contrived, and left.

**The build and the pins**
- **`wasm-opt`**: 133 on the workstation that deploys, 132 in CI and the Dockerfile (I-87). Align
  them — 132 on the workstation the way CI installs it, or CI and the Dockerfile to 133 with its
  checksum — and make the build say which it used.
- The Worker's own CI job (§4.4).

**The client**: the first connection's word is "reconnecting…", since `Link` has no "connecting"
(R3's R-N3; a frontend tweak, §4.7). **Carried from Task 8**: a probe answer landing after its
attempt was aborted is still handed over; the link word is never read while a tab is hidden; the two fake sockets could be one;
the two self-posting sites (`App.tsx`, `Abandon.tsx`) do not remember "off" on a 404; the probe is
unbounded; the Vite dev proxy has no `ws: true`.

### 4.7 The frontend design tweaks

The owner named "frontend design tweaking" as part of this cycle and gave no list in M6.5c's
session: **bring the owner's list to the brainstorm.** The lane's standing direction for the table's
look is physical-table fidelity — the site card on its area tile, pawns and cubes on the art in seat
colour, the port and the lab as strips of six cards under the board, centered and GROWING to the
room — and the owner decides faster from a mock than from a question. The M6.5c exit's headed walk at
390 px — clean, no layout defect seen, one of the five error strips provoked in the browser, the unlock strip — the
open table's password refusal for a stranger (the close's §3.3) — and the phones (§5) are inputs, as is the first connection's word (R-N3). A tweak that changes what a browser DOES owes the
local e2e; one that changes only what it SHOWS still wants a headed look, because no jsdom cell sees
a stylesheet.

---

## 5. The owner items carried

The M6.5c close's §6.3 in full; in short:
1. **The `test/` lifecycle** — §4.5.
2. **The refinish consequence**: on the Worker, and never on the NAS, an undo during a game-over
   upload can strand a one-person table (its Abandon refused for good, its expiry `held` until an
   operator's force), and two overlapping uploads can leave one completion's record beside
   another's table. A stamp-on-conflict-after-first-completion rule is a SPEC decision.
3. **The phones** — the iPhone exit and the Android handset, against the Worker; recorded, not gated.
   not taken this session; the owner's, recorded not gated
4. **The NAS-era RUNBOOK §3 spells the owner's address** in five lines (at M6.5c's code head, 572 and
   581 in §3.7, 658 in §3.8, 1413 and 1417 in §3.14's Step 4b) — scrub them before the repository
   goes public, or keep them.
5. **The PR merge order** — eight stacked PRs.
6. **What the picker's code stamp costs**: a deploy that changes the engine's, the CLI's or the
   layer's sources drops every open picker on the Worker (I-85, I-91) — safe, and logged.
7. **A rollback across M6.5c's fix wave is a GENERATION-ORDER rollback** (I-89; RUNBOOK §6.6): back
   to the eighteenth deployment or earlier, a re-created table goes unlisted until its retired
   generation is pruned, and every open picker the newer code stored is dropped.
8. **The first connection's word** is "reconnecting…" on both hosts for a browser with no remembered
   health answer (R-N3) — a frontend tweak (§4.7).
9. **Pool fact 8**: `ctx.id.name` inside a production `alarm()` is still unread live; record the
   first real alarm's line when one fires.
10. **`DEV_DOOR_CLIENT_ID`** committed in `worker/wrangler.jsonc` — a secret instead, at no code
    cost; **the dev door** is ON in production's vars and goes off at the public door.
11. **The Origin check** — a ruling the owner can reverse.
12. **Carried from earlier cycles**: the whole-game gate's vs-bot livelock; the e2e's `CAP = 4000`;
    the clean-browser single-audience reading; R-F6's Access re-login; the "Lobby" controls under a
    single-player build; the no-Web-Locks fallback and background suspension.

An owner NOTE, not an item: the WARP client's device login expired during the two-day pause, and the
owner re-authorized it on the 26th (trap 25).

---

## 6. What NOT to re-derive (measured traps M6.5c paid for)

1. **The stock wasm-bindgen glue gives an isolate ONE instance**; the factory transform makes each
   call its own, and refuses a glue whose shape changed.
2. **The asset layer's defaults break the service worker**: `html_handling` and `not_found_handling`
   are both `none`, and a missing file then invokes the Worker, which answers an empty 404.
3. **The test pool's facts** (the plan's Pool facts): patch the PROTOTYPE of an RPC target, never
   the instance; restore every patch in `finally` and before any alarm that follows an abort; a stub
   taken before `abortAllDurableObjects()` is dead; an eviction drops everything on the live
   instance; storage is per FILE; a close reason over 123 UTF-8 bytes throws; and the pool CAN deliver
   a nameless alarm (Pool fact 8, narrowed).
4. **`npm test` in `worker/` rebuilds no module**: `npm run build:module:debug` first after any change
   to `crates/yokohama-worker`, or the pool tests a stale module green; `npm run typecheck`'s
   `pretypecheck` hook writes `test/fixtures.gen.ts`.
5. **The strings rule**: the record and the table document are STRINGS end to end — a seed is a
   `u64` that a JavaScript number rounds past 2^53; the Rust-made fixtures reach the pool as text.
6. **Two time bases**: every time the shell keeps is `Date.now()` in milliseconds; the module's
   pinned reading, in whole seconds, feeds only what the layer stamps; a generation's time is the
   shell's (F-2).
7. **A GET can write** — a cold handle pumps an owed bot move — so the shell compares documents
   after EVERY module call, and ANY reply's archive plan is owed.
8. **The finalizing keep emits two bumps**; frames are one per socket per REPLY.
9. **`wrangler tail --format json` writes pretty-printed JSON**: parse it by whole objects; a
   line-by-line parse once printed nothing for 3,080 lines.
10. **Bot Fight Mode answers a curl-shaped request with a 403 challenge** (`cf-mitigated: challenge`,
    a ~5 KB "Just a moment…" page) BEFORE Access, on both hosts; a browser's user agent with
    `Accept: text/html` reads Access's 302.
11. **The analytics beacon's CORS error is a harness artifact**: custom headers on its cross-origin
    request force a preflight its host does not answer. A real browser logs none.
12. **Playwright's request routing does not reach a page-opened WebSocket upgrade**; context headers
    do.
13. **A thrown Playwright API call prints every request header, the cookie included**, to the
    reporter and to `web/test-results/<test>/error-context.md`, and a trace records an API
    context's headers: every live `request.*` goes through `liveRequest`, tracing is off under the
    live target, and a live response never meets `toBeOK()`.
14. **Access sets `CF_Authorization` on a service-token response** — to `/api/health` and to `/` —
    and that cookie's identity outranks WARP's on the same machine.
15. **A deploy stops every object and closes every socket 1006**: hibernation is read only in a
    window with no deploy.
16. **No alarm fires on the happy path**: every acknowledgement deletes or re-arms it.
17. **The deal's designed 409** is logged by Chromium as a console error, and the race resolves
    differently on the two hosts.
18. **What a game costs live**: ~38,000 Worker requests, ~7,000 rows written and ~1.34 M rows read
    for one 2,862-move three-page game, and ~32,000 requests for the 2,510-move re-run; the exit's
    paced game ran about 35 requests a move; the quota is the UTC day's and shared with the account's
    other Workers; the GraphQL recipe reads one dataset per `try`, because the MCP throws on the
    first GraphQL error; the Workers Observability query API reads a past run that had no tail.
19. **The live replay's reset recipe** when `wire`'s records change (the close's §7).
20. **The credential's key names** in `.dev.vars` are `CF-Access-Client-Id` and
    `CF-Access-Client-Secret`, the read-only R2 key's `R2_ACCESS_KEY_ID` and `R2_SECRET_ACCESS_KEY`;
    no value is ever printed, and a shell presents the credential through a header file of mode 600
    (RUNBOOK §6.4).
21. **`web/dist` holds the DEBUG module after a local e2e**: `cd web && npm run build` before any
    Worker deploy, and `grep -c debug_trap` must print 0.
22. **The deploy guard refuses a deploy without a rebuild**: its nine checks include two FRESHNESS
    checks — `gen/` built from the sources as they are, and `worker/assets` assembled from the
    current `web/dist` and `worker/static` — so `npm run build` in `web/`, then in `worker/`, comes
    before every `npm run deploy`.
23. **The exit's wake window is two deploys**: `node scripts/deploy-guard.mjs && npx wrangler deploy
    --var YOKOHAMA_WEB_COUNT_WAKES:on` opens it, and the plain `npm run deploy` closes it (a `--var`
    sits beside the file's `vars` and replaces none; wrangler shows its value as "(hidden)").
24. **macOS marks a COVERED headed window hidden** whatever Chromium's
    `--disable-backgrounding-occluded-windows` and `--disable-renderer-backgrounding` read (both
    Playwright 1.62.1 defaults), and a hidden page lets its socket go by design (R-P8): the exit spec
    places its windows through the browser protocol and asserts visibility as its own precondition.
    **Chromium widens a window to 500 px at least**: asked for 420, it reads back 500, so three windows
    overlap on a 1280-px screen. The spec therefore lays a 2×2 grid under the menu bar's inset (38 px
    on the workstation), reads every window's bounds back, and refuses an overlap or a window off the
    screen. The exit's second run stopped on exactly this, before its first pause; its third passed on
    the grid.
25. **WARP's device login can EXPIRE across a long pause**: after the two-day stop `warp-cli status`
    still read Connected, but every request from the workstation without the credential answered
    Access's 302 ("Please authenticate via the warp client"), and a curl-shaped one met Bot Fight
    Mode's challenge first. Only the owner re-authorizes it, in a browser. Check the login before any
    step that needs the owner's identity — an operator reading, a `?force`; the credential's runs, the
    deploys (wrangler's own login) and the bucket sync need no WARP. A `text/html` 403 of about
    5,259 B is the zone's, not the Worker's.
26. **The account's weekly limit stops every agent mid-task.** At the reset each is WOKEN by message
    and resumes as it was — never respawned (the owner, 2026-09-26). A long session also re-arms its
    keep-awake (`caffeinate`) and re-reads the tail's state after such a stop.
27. **`npm run test:scripts` needs `cargo` on `PATH`**: the freshness digest's `cargo metadata` cell
    walks the module's path dependencies.

---

## 7. An execution order, proposed

1. **The brainstorm with the owner**: what the NAS is for (§4.2's questions), the `test/` rule
   (§4.5), the frontend tweak list (§4.7), and the testing cadence (§4.4). Then the spec, and the
   plan, with the lane's review of each.
2. **The baseline**: ONE full sequence of every tier (§4.4), machine-tagged, before the first
   change — the net the refactors below fall into.
3. **The build's integrity**: the `wasm-opt` pins and the Worker's CI job (the deploy guard's
   freshness landed in M6.5c's fix wave).
4. **The Worker's design debts** (§4.6), behaviour-preserving, with the pool and the live replay as
   the net: `table.ts`'s split, the rows-read cost, the restatements.
5. **The host arrangement** (§4.2): the runbook's split, the NAS's role, whatever the spec rules
   for the NAS.
6. **The frontend tweaks** (§4.7), with the local e2e and a headed look.
7. **The `test/` lifecycle rule** (§4.5), set once and read back.
8. **The documentation** (§4.3): `CLAUDE.md`, `guidingpath.md`, the board, README — last, so they
   describe the final state.
9. **The closing sequence, the close** — and the refocus on the computer player, from
   `docs/plans/2026-09-11-ai-lane-reentry-handover.md`.

---

## 8. Process (the lane's standing rules)

- Subagent-driven, Opus the floor for implementers and reviewers, Fable the whole-branch review; the
  orchestrator runs every step that touches the account — deploys, the bucket, secrets, `wrangler
  tail`, the live runs — and never edits code.
- ONE green commit per task with its red run quoted; a reviewed SHA is never amended — a fix is a new
  commit; an acceptance reply that asks for changes after a DONE is a fix-round dispatch and says "as
  a new commit".
- Every agent writes its full report to a file in the ledger and replies with the path and three
  lines; ledger writes go by absolute path.
- While an implementer works, run nothing in its tree and arm no timers; a reviewer's mutants run on
  an export (`git archive`) unless the tree has been handed to it.
- The quota guard before every live run, on the owner's line; an anomaly — a trap, `exceededCpu`, a
  hang, an unexplained 5xx — goes to a dedicated research subagent before it comes to the owner.
- An agent the account's weekly limit stops is woken by message at the reset, never respawned; an
  UNREPORTED commit may be amended before its report, a reported one never.
- The owner's email address goes into no document; write "the owner" or "the operator".
