# M6.5c2 cycle C — the hosts and the close: the close (2026-09-28)

**Branch** `web-room-m6.5c2c`, stacked on `web-room-m6.5c2b` (cycle B) over `web-room-m6.5c2a` (cycle A) over
`web-room-m6.5c` — PR #41 against `web-room-m6.5c2b`. **Base** `3ed27f07` (cycle B's reviewed code head; its close commit
`23735672` is the branch's first). **Authority** the spec `docs/plans/2026-09-26-web-frontend-m6.5c2-spec.md` at revision
2.7 (`a855b379`; its dated §10.8 carries every correction this execution found). **Plan**
`docs/plans/2026-09-28-web-frontend-m6.5c2c-plan.md` (`676d72c1`; thirteen tasks from drafts D6–D8 as revision 2.7 amends
them). **Execution shape** subagent-driven for every task — one Opus implementer and one Opus reviewer each, one green
commit with its red run quoted, the fix rounds as new commits — with Fable orchestrating, reviewing the whole branch and
running the close; the record is the gitignored ledger `.superpowers/sdd/2026-09-28-web-frontend-m6.5c2c-plan/progress.md`.
**The reviewed head is `ea4f174d`**: 28 commits over `3ed27f07` (B's close, the spec's revision, the plan, the twelve tasks'
20, the ONE fix wave's 5); 117 files, +10,399 / −2,590. The documents commit that carries this close follows it.

**Headline.** Cycle C brings M6.5c2 to both hosts and closes the last engineering cycle of the web lane before its
wrap-up. **What is LIVE, all of it deployed at this close on 2026-09-28:**

- **The Worker `yokohama-web` at `yokohama.moore.nyc` — the twenty-first, twenty-second and twenty-third deployments**
  (`worker/RUNBOOK.md` §15–§17): versions `ce923774-aa81-4f2b-a8dc-b0fcb5a05f70` (16:48 UTC — the two doors with the
  `worker` door deployed, the refinish rule, the memo, the module's invariants, the bot's version on every save, `0.6.0`,
  and **`DEV_DOOR_CLIENT_ID` put as a SECRET in the same deploy that dropped it from `vars`**), `43822b48-da53-48ab-9be9-96f6cb16a2a7`
  (17:26 UTC, the exit's window open) and **`36038818-4aa8-4544-961a-0bec2674782c` (17:32 UTC, the window closed — the
  version this close leaves live)**. The release both hosts serve is `yokohama-release-605fe69f17100c46`.
- **The NAS `yokohama-web:0.6.0-m6.5c2` at `yokohama-nas.moore.nyc` on `cf-access`** — its fourteenth deployment
  (`deploy/RUNBOOK.md` §3.17; container `10feaa56a4fe…`, listening 16:54:51 UTC), the first deployment of the layer's
  `CfAccess` arm on any host: **`/api/me` names the owner** over WARP, and the credential reads `null` and 401 (state B).
- **The bucket's `test/` lifecycle rule** `expire-test-after-30-days` (R-6.5c2.12): every save under `test/` expires 30
  days after it was written; the owner's paths are out of its reach by construction.

**The lane's fence held at every commit and was re-read at `ea4f174d`:** the five fenced crates and the CLI byte-identical
to `3ed27f07` (0 lines), `SEMANTICS_REV` 7, no bot behaviour moved, the layer's `src` moved in exactly its six named
places, the corpus in exactly Task 9's three files by the version alone (§1.13).

---

## 1. What shipped, by task in execution order

Each task's readings are its final ones; the fixes named are fix rounds landed as NEW commits before the task's re-review.
Deltas are against Step 0's baseline (§2) or the task before. Every task's report, review and re-review sit in the ledger
by name (`task-<n>-report.md`, `task-<n>-review.md`, `task-<n>-rereview.md`).

- **Task 1 — the Worker's two doors** (`676d72c1..15157a18`; one fix round). `YOKOHAMA_WEB_IDENTITY` with NO default on the
  Worker: an unset door is the host-written 500 at `handle`, `worker` deploys, `open` reads the claim from the header on a
  fetch and from `claimed=` on an upgrade (URL-decoded; a non-ASCII value is no caller, never a platform 500); the objects
  read their door's ONE header; the pool gains its second project (`wrangler.open.test.jsonc`). The value is trimmed and
  matched EXACTLY, as the layer's `from_str` matches it on the NAS (R-C11). Readings: worker 67 (+3), the pool 203 in 10
  (+6; `worker` 199 in 9, `open` 4 in 1), `test:scripts` 366 in 18 (+14), `npm test` 1198 in 64 (+3); the e2e 68 in 13.
- **Task 2 — what the open door refuses** (`15157a18..b498da6c`). Under `open` the operator surface is off (the layer's
  404 whatever name is claimed); `YOKOHAMA_WEB_ADMINS` or `YOKOHAMA_DEV_DOOR=on` beside `open` is the 500 naming both; a
  plain name never files under `test/` (an `@` required); the deploy guard reads the door and rules per door, and refuses
  `DEV_DOOR_CLIENT_ID` in `vars` beside the operator list. Readings: worker 68, `test:scripts` 379 in 18, the pool 206 in
  10.
- **Task 3 — the refinish rule on the Worker** (`b498da6c..d431dbc1`; two fix rounds). A conflict hands its held record
  to the module; the layer stamps a same-game conflict (`archive-kept-earlier-completion`) and every other conflict is
  FINAL; a refused stamp is `stamp-refused {id, status}` and retried, never thrown; the race is pinned both ways in the
  pool. Readings: worker 70, the pool 210 in 10, `test:scripts` 382 in 18.
- **Task 4 — the module's invariants and the imports** (`d431dbc1..83b52710`). The picker's rule on every call (a
  watch around `Games::handle`, `events_inner` inside it); the scrub fenced to the seat and the mirror; one plan per call
  (`SECOND_PLAN`); both hosts' restatements replaced by the layer's five exports; both hosts' platform-call scans
  switched to the layer's `calls_in`; the Worker module's figures measured in the root `Cargo.toml` beside the browser
  module's (1,256,770 B at that tree). Readings: worker 75 (+5), `test:scripts` 384 in 19.
- **Task 5 — the shell's small debts** (`83b52710..82280fd4`). The `hasTables` memo — **a warm request asks
  `sqlite_master` nothing: 32 rows read → 2, MEASURED in the pool**; the operator listing's `last_active` in seconds; a
  one-row record check in `deleting` (3 reads → 2 on a cold path); a force that flags itself (`meta.forced`);
  `removal-ended` for the markerless removals older code left. Readings: the pool 216 in 11.
- **Task 6 — the layer's small debts** (`82280fd4..eb949e68`; one fix round). The versioned refusal sentence
  (`distinguishing_labels`: a tie adds `.<controller version>`, a second tie the parameters, `({controller})` when the map
  is empty; the three callers routed through it and a same-name re-point cell pinning them — R-C5, R-C15–R-C17);
  `persist` re-stamps `run_id` from the host and the run-id pin; the print scan's `writeln!` probe. The layer's `src` moved
  in exactly `api.rs`, `bots.rs`, `explain.rs`, `games.rs`, `lib.rs`, `log.rs`. Readings: web 342 (+3).
- **Task 7 — the scripts' LOW findings** (`eb949e68..97f222c8`; one fix round). L-3 `deployedRelease` names, never
  echoes; L-5 the host-field exemption must be SPENT (a `no-string` verdict on an absent field); L-7 `dealAnswers` capped
  at one 409 a page; L-9 `awaitCreate` with its structural cell (`live.node.test.ts`) refusing any live create outside it;
  the null id's SAFE reason (a status or an error's name, never its message). Readings: `test:scripts` 387 in 19, the
  pool 217 in 11, `test:wasm` 114 in 9.
- **Task 8 — build and CI** (`97f222c8..11d52122`; one fix round). `deploy/Dockerfile` installs the same `wasm-opt` 133
  tarball CI pins; the `worker` job its own; the whole-game gate back in `web-server`, LAST and bounded (`timeout-minutes:
  20`, `--nocapture`, `YOKOHAMA_WEB_GATE_BOT_SEED`); a test-binary guard off `cargo metadata`; the refusing-build step's
  `set +e`; `docker.yml` building and smoking the self-host image under each door (`deploy/smoke-image.sh`), on web-lane
  inputs only (R-C9); the image built in 230 s, `smoke: PASS` twice. No compiled file moved.
- **Task 9 — the 0.6.0 bump** (`11d52122..0d34ec60`). The workspace and both npm packages at `0.6.0`; the pin cell
  holding the two packages to the crate's version; the corpus regenerated in EXACTLY three files by the version alone
  (`health.json` 2 lines, `archive-walk.json` 14, `backup-envelope.json` 2; the proof `task-9-corpus-proof.txt`); the
  four cells that go red with `without_version_drift` reverted (R-C22). Readings: web 343; every crate's count equals Step
  0's.
- **Task 10 — the lobby's Finished group** (`0d34ec60..105a196b`). `section.finished-tables > details.fold`, rendered
  only when a finished game exists, "View" on its rows, the padlock kept, newest first, the marker after the count; the
  Help's sentences; the browser witness on `plays_a_whole_game.spec.ts`; pictures at 1440 and 390 beside mocks 02 and 11.
  Readings: `npm test` 1206 in 64 (+8); the e2e 68 in 13.
- **Task 11 — the client's carried fixes, polish 1–4 and the a11y pass** (`105a196b..8c572f4e`; one fix round). The
  foreign-server and local-broken screens as lock-screen kin; "Will not open here" with export and Remove; `isStranded`
  filtering both menus; focus returning from every panel; 44 px on every pressable setup control; each refusal beside its
  control (R-C24: Abandon's own strip); the door's project link; the `failed` class keyed on failure (R-C25); pictures at
  1440 and 390 beside mocks 01, 02, 08, 09, 11 and B's lock screen. Readings: `npm test` 1220 in 64 (+14); the e2e 68 in
  13. **The code head is `8c572f4e`.**
- **Task 12 — the host documents** (`8c572f4e..0ac89dbd`; one fix round; documents only). `deploy/RUNBOOK.md` §6 MOVED
  verbatim to `worker/RUNBOOK.md` (§1–§14; a map left behind; the move script reproduces it byte for byte, then twelve
  dated corrections); `deploy/README.md`, the self-host guide, with the open door's own section; the address scrub —
  0 hits for the address, its local part and its transliteration across the tracked tree, the two frozen plans and the
  five RUNBOOK lines reading "the owner's address" under the owner-ruled exception (R-6.5c2.16; the decision-log row).
- **Task 13 — this close** (B.1–H.3): §2–§8.

### 1.13 The fences, shown to hold

At `0ac89dbd` (`fences-at-0ac89dbd.txt`) and again at `ea4f174d` and at this close's own head: `git diff --stat 3ed27f07 --
crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data
crates/yokohama-cli` prints NOTHING; `SEMANTICS_REV` 7 (`semantics.rs:180`); the layer's `src` = `api.rs` +6/−2, `bots.rs`
+116, `explain.rs` +10/−2, `games.rs` +173/−12, `lib.rs` +36, `log.rs` +7/−1 (Tasks 6 and 9's named places; R-C15 reads
`games.rs::live_handle` as the foreign-bot sentence site) plus `tests/browser_played_pin.rs`'s doc by Task 9's mandate;
`crates/yokohama-wasm/src/lib.rs` in Task 4 alone; the corpus = `archive-walk.json` 14, `backup-envelope.json` 2,
`health.json` 2 — nine lines removed, nine added, each pair equal with `0.5.0` and `0.6.0` masked; **101** JSON files under
`web/src/wire/`; `Cargo.lock` moved in nine `version` lines; no bot behaviour moved.

---

## 2. The counts at the close HEAD, in ONE sequence

Every command below ran at the reviewed head `ea4f174d` on the Mac workstation (Darwin 27.0.0 arm64, macOS 27.0; the pinned
1.97.1 toolchain; node 26.10.0; `wasm-opt` 133; wrangler 4.136.3) as ONE uninterrupted script (`close-sequence.sh`, R-6.5c2.13)
from **16:19:05 UTC** — the per-edit suites of every crate and package, the browser walk among them, then the fifteen
release gates with their binaries pre-built — to its `done` row at **16:40:15 UTC: 1,270 s (21.2 min) over 41 rows**. The
wall is each command's own; the log per row is the evidence (`sequence/NN.log`). **Every count equals Step 0's baseline
(the plan's commit, code byte-identical to `3ed27f07`) or the owning task's final reading.**

| # | command | reading at `ea4f174d` | wall |
|---|---|---|---|
| 1 | `cargo test -p yokohama-core` | 1577 passed + 29 ignored (22 result lines: the twenty-one binaries + the doc-test) | 196 s |
| 2 | `cargo test -p yokohama-cli` | 851 + 2 ignored (823 lib + 9 `bga_import` + 18 `guide_sync` + 1 `piped_dogfood`) | 36 s |
| 3 | `cargo test -p yokohama-oracle` | 191 | 3 s |
| 4 | `cargo test -p yokohama-difftest` | 52 + 5 ignored | 21 s |
| 5 | `cargo test -p yokohama-core --example eval_probe` | 214 | 49 s |
| 6 | `cargo test -p yokohama-core --example solver` | 132 | 3 s |
| 7 | `cargo test -p yokohama-bridge` | 81 | 7 s |
| 8 | `cargo test -p yokohama-web` (the whole-game gate included) | **343** = lib 324 + `browser_played_pin` 4 + `play_a_whole_game` 2 + `wire_fixtures` 13 | 111 s |
| 9 | `cargo test -p yokohama-wasm` | 7 | 2 s |
| 10 | `cargo test -p yokohama-worker` | 75 | 8 s |
| 11 | the refusing build's `--lib identity::` | 8 | 1 s |
| 12 | `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown` | ok, 0 warnings | 0 s |
| 13 | `cargo clippy --workspace --all-targets -- -D warnings` | ok | 1 s |
| 14 | `cargo doc -p yokohama-worker --no-deps` | 0 warnings | 0 s |
| 15 | `cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm` | ok | 1 s |
| 16 | `cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker` | ok | 0 s |
| 17 | the four bridge node suites | PASS ×4 | 9 s |
| 18 | `cd worker && npm run build:module:debug` | ok | 0 s |
| 19 | `cd worker && npm run typecheck` | clean (two projects) | 2 s |
| 20 | `cd worker && npm run test:scripts` | 387 in 19 files | 1 s |
| 21 | `cd worker && npm test` (the pool, both projects) | 217 in 11 files | 8 s |
| 22 | `cd web && npm run build:wasm` | `wasm-opt` 133: 1,814,905 → 1,197,069 B, both modules | 2 s |
| 23 | `cd web && npm run typecheck` | clean (four projects) | 5 s |
| 24 | `cd web && npm test` | 1220 in 64 files | 13 s |
| 25 | `cd web && npm run test:wasm` | 114 in 9 files | 2 s |
| 26 | `cd web && npm run e2e` | **67 passed of 68** in 4.4 min — the one red `plays_locally.spec.ts:476` "a quota failure is APPLIED BUT NOT SAVED" (§2.1); **re-run ALONE at the same head right after: 68 passed in 5.1 min, the cell in 1.1 s** | 267 s |
| 27 | release gate 1 — `fuzz_full_games` | 1 | 45 s |
| 28 | release gate 2 — `choice_encoding` | 1 | 1 s |
| 29 | release gate 3 — `observation_honesty` | 2 | 3 s |
| 30 | release gate 4 — `action_source_equivalence` | 2 | 26 s |
| 31 | release gate 5 — `action_source_worstcase` | 3 | 1 s |
| 32 | release gate 6 — `arena_gate` (single-threaded) | 8 | 20 s |
| 33 | release gate 7 — `policy_diversity` | 4 | 6 s |
| 34 | release gate 8 — `evaluator_gate` (single-threaded) | 5 | 259 s |
| 35 | release gate 9 — `architecture_gate` (single-threaded) | 3 | 47 s |
| 36 | release gate 10 — the core lib's ONE `#[ignore]` (the recovery falsifier; exactly 1 test ran) | 1 | 9 s |
| 37 | release gate 11 — `piped_dogfood` | 2 | 12 s |
| 38 | release gate 12 — `differential` | 1 | 25 s |
| 39 | release gate 13 — `routes3_differential` | 1 | 2 s |
| 40 | release gate 14 — `scoring_differential` | 1 | 2 s |
| 41 | release gate 15 — `routes3_worstcase` | 2 | 3 s |

**The fifteen-gate roster's wall in this sequence: 461 s** (7.7 min) — the Mac workstation's reading, recorded BESIDE
M6.5c's 533 s on the same machine and the M5-era ≤ 420 s target, never against them (every gate's count as M6.5c's). The
per-edit rows 1–26 summed 748 s. The "warnings: N" column of `sequence.tsv` is a word count of test names, not compiler
warnings; clippy's row is the lint gate and read 0.

### 2.1 The e2e's one red, read at its trace

The cell `playUntilAnswerable`s until an enabled option exists, arms the quota failure, clicks the first enabled option
and expects "APPLIED BUT NOT SAVED" within 10 s. Its Playwright trace (unzipped in the scratchpad) shows the click
returning in **31 ms** and the page, at the timeout, holding the destination sub-picker "Assistant destinations from hand:
[1]…[10] > pick a number, or 'cancel'" at Turn 1 — **the option clicked was a two-step choice's FIRST step**, so no move was
applied, the store was never asked to put, and the warning could never appear. The premise "a move to make" is weaker than
the cell needs ("an option that APPLIES a move"), and which shape the OS-entropy local game reaches is seed-conditional.
**The M6.5c close's sequence recorded this SAME cell red once** ("not found within 10 s under the sequence's load", green
alone twice, "recorded for the cleanup cycle"); this trace revises that reading — at least this red was a premise, not a
slow reply. Re-run once alone, green; not retried further (R-C30's sibling rule for the sequence: a red row is re-run alone
before it is believed, never retried into green). The hardening is cycle D's (§6.2).

---

## 3. The deployments, the live tiers, the F-4 reading, the quota and the lifecycle rule

Every deploy was the orchestrator's, from the reviewed head and a clean tree, `npm run build` in `web/` then `worker/`
first (ONE release for both hosts), the deploy guard passing, `wrangler tail` listening before the Worker's first, the
credential presented only through a mode-600 header file that was deleted after, and the owner's address in no reading
that is written down. The records of record are `worker/RUNBOOK.md` §15–§17 and `deploy/RUNBOOK.md` §3.17; this section
is their summary with the readings that decide the cycle's claims.

### 3.1 The release build, once (E.2)

`cd web && npm run build` after the close's e2e re-run: `debug_trap` **0**; `wasm-opt version 133` **1,814,905 →
1,197,069 B (−34.0%)** over both browser modules (§3.16's 1,175,223 under 132: the layer's own growth rides in the module);
the precache generator: **153 entries, 10,010,662 B; the module at `/assets/yokohama_wasm_bg-Ciy339-r.wasm`; the release
`yokohama-release-605fe69f17100c46` → `dist/sw.js` (29,045 B), 131 art files**. `cd worker && npm run build`: the module
**1,901,547 → 1,258,106 B** (the root `Cargo.toml` paragraph reads 1,256,770 at `d431dbc1`; +1,336 B since — a reading to
transcribe there in D), gzip 491,934 B, 18 pages, 170 segments; `factory-glue: 16 exports (a release module: no
debug_trap)`; `assemble-assets: 153 precache entries verified`; `smoke-release: … /api/health 200` at `0.6.0`.

### 3.2 The Worker — deployments 21, 22 and 23 (E.3–E.5, F.5)

| # | when (UTC) | version | what it brought | the readings that matter |
|---|---|---|---|---|
| **21** | 16:48:09 | `ce923774-aa81-4f2b-a8dc-b0fcb5a05f70` | the whole of M6.5c2 on the Worker (§1); **`DEV_DOOR_CLIENT_ID` a SECRET, put by `--secrets-file` from a mode-600 file written from `.dev.vars` and deleted after** | the guard PASSED every check, the door printed first; 6 assets uploaded, 148 already there; upload 1386.14 KiB / gzip 526.03 KiB; startup 2 ms; `secret list` → `DEV_DOOR_CLIENT_ID`, `YOKOHAMA_WEB_ADMINS`; `/api/health` 200 `0.6.0`/`cf-access`/`ws`/`r2`; `/api/me` the owner; `/sw.js` 29,045 B naming `605fe69f17100c46`, byte-equal to `web/dist/sw.js`; **the dev door works with the secret** (the credential + `X-Yokohama-Claimed-User: smoke@readme.test` → `user: "smoke@readme.test"`; the credential alone → `null`); `picker-dropped` 0 (no table object stored under earlier code existed to be asked); the F-4 burst (§3.5); the lifecycle rule (§3.7); the replay twice (§3.4); the live Playwright tier (§3.4) |
| **22** | 17:26:40 | `43822b48-da53-48ab-9be9-96f6cb16a2a7` | the same code; **the exit's window OPEN** (`--var YOKOHAMA_WEB_COUNT_WAKES:on`, after the guard) | 172 assets read, none re-uploaded; startup 1 ms; the bindings show `YOKOHAMA_WEB_COUNT_WAKES ("(hidden)")`; both secrets inherited; the exit game and the wake witness (§3.4) |
| **23** | 17:32:48 | **`36038818-4aa8-4544-961a-0bec2674782c`** | the same code; **the window CLOSED** (`npm run deploy`) | the guard's every check; none re-uploaded; startup 3 ms; `YOKOHAMA_WEB_COUNT_WAKES` gone from the bindings; both secrets |

By design every picker row stored under the twentieth is dropped at its table's next request under this code (I-85,
I-91); none was observed, because no such table existed (§7).

### 3.3 The NAS — the fourteenth deployment (E.6–E.7)

The musl cross-compile in 17.55 s (four crates recompiled at `0.6.0`): the binary **6,747,440 B**, sha256 `a5d4ebcd…`; the
context **11,338,490 B, 174 entries, 0 `._`**, served from a directory of its own on `192.168.1.221:8899` with `/.dev.vars`
answering **404 before the POST** (P-1); the image `yokohama-web:0.6.0-m6.5c2` (`sha256:ff05bda4…`, **24,672,200 B**, the
binary layer new, the dist layer +73,690, the art layer shared); the swap — the old container exited 16:54:26.604 UTC (exit
0), the new one `10feaa56a4fe…` created with **`Env` exactly `["YOKOHAMA_WEB_IDENTITY=cf-access", "YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive"]`**
listening at 16:54:51.905 UTC; **downtime 25.3 s** (§3.16's 8.45 s: four proxy round trips here, not one script). **Verified
through the tunnel at 16:55:20 UTC:** `/api/health` 200 `0.6.0`/`cf-access`/`sse`/`dir`; **`/api/me` →
`{"mode":"cf-access","user":"<the owner's address>"}` — the load-bearing reading, no STOP, no rollback**; `?scope=mine`
the owner's one table (`sim-54b139c22c136cb0`, playing, its copy carrying no bot descriptor — a pre-deploy document); the
credential → `/api/me` `null` and `/api/games` **401** (state B); `/enter` 302 → `/#/lobby`; `/` naming
`index-BdDFy9mU.js`; `/sw.js` naming `605fe69f17100c46`, byte-equal; the module 1,197,069 B; the art 200. The tables on the
volume as counts: 1 table document (the owner's, a bot seat, playing, schema 2), 9 records, 2 chats. Memory 11.2 MB. **The
owner's own browser** (Claude-in-Chrome, connected mid-close by the owner) opened the lobby: the corner reads "Playing as"
the Access identity beside "change name", no stored claimed name anywhere; the one table listed; no Finished group drawn
(none exists) — B's Review Focus 3, live.

### 3.4 The live tiers — the ending state

- **The replay's reset** (F.1): the four keys under `test/browser/red_at_wire.test/rev7/{over,abandoned}/` deleted
  (Task 9's bump had changed the walk's records).
- **The live corpus replay, twice, sequentially** (F.3): run 1 (16:57–17:00 UTC) forces 204 ×3 before and after,
  **2,209 of 2,209 rows run, 86 named rows compared, both first sends 201, mismatches none, HOLDS**; run 2 (17:00–17:03)
  the same 2,209/86, both first sends **200 with the once-rule's `identical: true`**, mismatches none, HOLDS. The
  host-field exemption spent (`me_signed_out.mode: "open" → "cf-access"`); ~2,190 requests and ~21,800 rows read a run.
- **The live Playwright tier** (F.4). **`joins_by_link` PASSED** (5.0 s; 6 moves across two browsers in 4 s; the deal's
  409 forgiven a 2 / b 0; the beacon's pair a 2 / b 2). **`two_browsers` read RED once, then PASSED:** run 1 (17:04 UTC,
  both specs in one invocation at `workers: 1`) stopped at 13.9 s on "the watcher never asks /events while a stream is
  open" — ONE `/events` ask by the watcher at the twentieth watch check, which on this host is the socket arm's one probe
  after a close the client did not make (`web/src/signal.ts`; a hidden tab or a teardown probes nothing), so the
  watcher's socket CLOSED once, unexplained, within the first twenty applied moves; `wrangler tail` had lost its
  connection at 17:04:02 and carried nothing of that table, so the server side is UNOBSERVED; the workstation's load read
  10.9 on 10 cores (the battery beside it). **R-C30**: re-run ONCE, ALONE, with a fresh tail attached — run 2 (17:16–17:25
  UTC): **game `sim-a34bc7df08d68d31` finished in 1,795 moves, 535 s, 0 idle looks; the watcher was held to 1,394 applied
  moves, asked `/events` 0 times and opened `/stream` ONCE — 1 at every one of 69 checkpoints**; the players' streams a 2 /
  b 2 (both before the first move); the deal's 409 forgiven 0/0/0; the beacon's pair 2/2/2; every console clean; no `Call
  log`; the tail's 1,065 sampled events of that table: `picker-dropped` 0, `object-failed` 0, `frame-failed` 0, `trap` 0,
  exceptions 0, the object's `/input` CPU median 4 ms, max 26. Run 1's one probe stands as recorded and unexplained (§7).
- **The headed exit** (F.5, inside the window): **PASSED, 1 passed in 5.3 min.** Table `sim-d67dacbd33f2a287`: seating
  live (x locked, y joined through the lobby's Open-tables link with the password, w unlocked and watched with no hand);
  three windows in a 2×2 grid under a 38 px inset, visible at every checkpoint; **40 moves in 311 s; four pauses of
  60.0 s after moves 5, 15, 25 and 35, nothing asked meanwhile, each followed by the move's POST in 84 / 243 / 81 / 75 ms
  against an awake median of 80 ms**; table chat x 10 / y 10 (the watcher's panel 20), lobby chat x 1 / y 1; the abandon →
  404 "its game is in the archive", x, y and w told; forgiven the beacon's pair 2/2/2 and the deal's 409 on x twice; every
  console clean. **The wake witness off the tail: FOUR `wake` lines, `wakes` 1 → 2 → 3 → 4** at 17:28:10, 17:29:28, 17:30:48
  and 17:32:05 UTC, each on the first POST after a pause, **at 9–12 ms of CPU against a 2 ms median (max 22)** over 32
  sampled object-side move POSTs — the object hibernated in every pause and came back on the next move (M6.5c's exit read
  6–9 ms); `slot-pushed` 7 / `slot-acked` 6, `archive-landed` 1; `trap` 0. The window was open six minutes.

### 3.5 The F-4 reading — rows read, on the pool and live

In the pool (Task 5): a warm request **32 → 2 rows**, a cold request's documents read twice, not three times. Live, the
day's Durable Objects analytics around a counted burst on one throwaway table (`sim-248e605df587a848`: a create, one cold
`/table` read at 95 ms, **50 warm `/table` reads in 6 s**, a delete): **+225 rows read for +60 Worker requests** — about 4
a request all-in, consistent with 2 a warm table request once the cold read, the create, the delete and the house's reads
are set aside. Live, the two-browser game's window: **+101,236 rows read over 1,795 moves ≈ 56 a move, against M6.5c's ~470
(an eightfold reduction)**; rows written +4,570 ≈ 2.5 a move and requests +24,815 ≈ 13.8 a move, both unchanged — as spec
§6 says, requests per move did not move, and they still bind the free plan first (the lever is the client's refetch on
every frame; rows read now binds last). The exit's 40 paced moves with three pages and chat: ≈ 63 rows read and ≈ 16
requests a move.

### 3.6 The quota guard (the owner's line: 80,000 a UTC day for Worker requests and for rows written)

| read at (UTC) | Worker requests, account | `yokohama-web` | rows written | rows read |
|---|---|---|---|---|
| 16:42, before any deploy | 12,568 (35 scripts) | 3 | 221 | 266 |
| 16:56, after deploy 21's readings and the burst | 12,657 | 63 | 253 | 491 |
| 17:03, after both replays | 17,061 | 4,447 | 7,713 | 44,129 |
| 17:25, after the two-browser runs | 42,640 | 29,918 | 12,283 | 145,365 |
| 17:32, after the exit | **43,334** | **30,564** | **12,443** | **147,887** |

Both guarded figures stayed under the line all day; the day held two replays, a short failed game, a 1,795-move game and
the exit with room for one more such game.

### 3.7 The `test/` lifecycle rule (E.5; R-6.5c2.12)

`npx wrangler r2 bucket lifecycle add yokohama-archive expire-test-after-30-days test/ --expire-days 30` — a prefix
given, so no whole-bucket question; `--force` never passed. `lifecycle list` reads back the default multipart-abort rule
(all prefixes, 7 days) and **`expire-test-after-30-days`, enabled, prefix exactly `test/`, "Expire objects after 30 days"**.
No expiring rule reads "(all prefixes)".

### 3.8 The two instruments that misread this close, recorded

`wrangler tail` DROPPED its connection under the live tier's burst ("Tail connection lost. Reconnecting"), carried nothing
of the run that mattered most, and SAMPLES a busy game (457 `/input` rows of 1,795) — a tail is not a log, and what it did
not carry is unobserved, not absent. And a `security find-generic-password` that the morning's session used answered
"could not be found" in this one; the Portainer token's other place (the portainer-mcp checkout's `.dev.vars`) served.

---

## 4. The reviews and the battery

- **B.1 — the whole-branch review** (Fable, in an export of `0ac89dbd`; `final-review.md`): **With fixes — 0 CRITICAL,
  0 HIGH, 3 MEDIUM, 12 MINOR, 4 OPEN.** Ten mutants RE-RUN across seven tasks and three languages, ten agreements with the
  reports; every picture read beside its mock and passed; the fences re-read and held; every deployment sentence in the
  documents in plan tense (O-2, the list this close's truth pass rewrote); `CLAUDE.md`'s false sentences listed (O-3,
  rewritten in the same pass). The three MEDIUMs were false sentences a person follows: `docker.yml`'s "read at the push"
  (the trigger is master-only; the cold reading is the pull request's), the self-host guide's loopback remedy (it did not
  close the path it named), and the Help's "every game already over" (untrue of the caller's own).
- **B.2 — ONE fix wave**, five NEW commits on `0ac89dbd` by an Opus implementer on the review's triage adopted as written
  (R-C27): `933104f1` (the two workflows' sentences and counts; the cold set complete), `95be9a9c` (`Help.tsx`, the
  closed fold's marker in `--fg-dim`, the hover under `@media (hover: hover)`; pictures re-cut; the e2e 68 in 13),
  `f70d283d` (`deploy/RUNBOOK.md`'s antecedent), `1c7d8767` (the two thrown sentences repointed to `worker/RUNBOOK.md`
  §4), `ea4f174d` (`deploy/README.md`'s loopback paragraph in R-C28's words — the review's own closing clause was wrong).
  Nothing compiled moved; the fences and the corpus unmoved.
- **B.3 — the scoped re-reviews**, an Opus one and a Fable one in parallel over the wave's diff, each in its own export:
  **both APPROVED**, every item ADDRESSED with `file:line`, no new Critical or Important; the marker's colour MEASURED
  ((123,135,149) → (139,151,166), confined to its box); out of scope for D: the cue still quiet at 1×, `test.yml:210`'s
  "to the clock", the twelve remaining `§6.x` citations. **`ea4f174d` is the reviewed head.**
- **C — the mutation battery** (an Opus agent in an export of `ea4f174d` under this session's scratchpad with the warm
  target directory, concurrent with the deploys by R-C29; `battery.md`, 2,600 lines with every applied diff;
  `battery-logs/results.tsv`): **132 mutants, 132 CAUGHT, 0 findings, 0 not run**, in 53 minutes — the plan's 48 rows as
  51 lines (rows 2, 16 and 23 each have two readings), the seven extra rows as 13 (X2 two, X4 six), and **61 catchers the
  agent found in the reports with no row of their own, run as 68 lines** (Task 4's five frame and watch mutants, Task 5's
  force and copy mutants, Task 7's harness cells, Task 8's four CI structure mutants, Task 10's five, Task 11's twenty).
  The door and refinish rows went first; row 22 and its three siblings were read at the compiler as the brief allows;
  row 40 (the image with a default door) was BUILT — `docker buildx build --platform linux/amd64 --load`, since the
  Dockerfile pins the x86_64 binaryen tarball and the host is arm64, as Task 8 built it — and `deploy/smoke-image.sh`'s
  step 1 refused it ("the image is still serving 30 s after starting with no `YOKOHAMA_WEB_IDENTITY`"); the pristine
  Dockerfile's image read `smoke: PASS` as the control. Every mutant was restored by copy, `touch` and `cmp`; every file's
  catcher read GREEN after its last revert (the whole jsdom suite 1220 in 64, the pool 217 in 11, `test:scripts` 387 in 19,
  worker 75, web lib 324 + pin 4, wasm 7, the two e2e node files 72); the export's `diff -rq` against the pristine copy
  showed only build products. **Observations that are not findings:** Task 11's "10 cells" for the refusal-outside-the-card
  mutant read 6 (one site, in `App.tsx`); row 48's catchers read 76 cells in six files, not 1 + 74 in five; row 32's
  catcher is a probe on the scan, not a standing cell (as Task 6 recorded it); the gate step without `set +e` still
  prints cargo's line and loses only its `::error::` explanation; two cells' line numbers drifted since their reports
  (`worker.test.ts:392`, `games.rs:1318`); the stylesheet rules of Tasks 10 and 11 have NO mutation reading — their
  catchers are the browser reds taken against HEAD's client, which the battery's no-browser rule kept out. **This is the
  cycle's proof that its catchers catch: 132 of 132, and the reports' catcher tables read trustworthy, twice over (the
  whole-branch review's ten re-runs, then the battery's 132).**

---

## 5. Every ruling taken in execution, with its cost

**The owner's** (spec §10.7, revision 2.7): R-6.5c2.20–.27 — one cycle C then a lighter D; the bot's version by ingestion;
the Finished group; the polish standard and split; CI lean; the lane document to D; the dropped refactors stay dropped;
the phones wait for the deploy — executed as written.

**At planning (the plan's rulings):** **R-C1** thirteen tasks (the spec's item 5 split into the shell's and the layer's);
**R-C2** every diff's base is `3ed27f07`; **R-C3** L-9's helpers in `web/e2e/live.ts`, L-5's presence rule stated beside
the replay's; **R-C4** L-8 is `hasDoc(sql, kind)`; **R-C5** the versioned refusal sentence's three arms; **R-C6** the
Finished group as `section.finished-tables > details.fold`; **R-C7** its browser witness on `plays_a_whole_game`; **R-C8**
the `events` two-defect line; **R-C9** `docker.yml` on web-lane inputs only, with a fifteen-minute cold-wall fallback.
Costs: a plan shape; each reversible by a task.

**In execution** (the ledger's `Ruling:` lines, in order; each with what it costs if wrong):

- **R-C10** (Task 1): under `open`, a claim the grammar refuses on `POST /api/archive` reaches an Archive object whose
  `init` refuses it — a 500 with the layer's own sentence, not the 401. Accepted as measured: the layer judges the name,
  the client never sends such a claim, no open-door Worker is deployed. *Cost: a hand-made request to an open-door
  Worker reads a truthful 500 instead of a 401; fixable in the module's init path.*
- **R-C11** (Task 1): `YOKOHAMA_WEB_IDENTITY`'s value trimmed and matched EXACTLY on both hosts. *Cost: an operator typing
  `Open` reads a 500 naming the value.*
- **R-C12** (Task 1): the module's `open-door` feature is inert at compile time and STAYS as §5's declaration. *Cost:
  none.*
- **R-C13** (Task 3): `stamp-refused {id, status}` keeps its shape; the reason is on the wake's `restore-refused` line.
  *Cost: an operator follows one more line.*
- **R-C14** (Task 4, amended at review): `expire_inner`'s no-creator arm CAN reach a rebuild on a hand-made document
  (`1e400` in an unknown key; nesting past 128; a lone surrogate); accepted, no code change — no layer write makes one.
  *Cost: a hand-crafted row reads a 500 with the wrong reason.*
- **R-C15** (Task 6): the foreign-bot sentence site is `games.rs::live_handle`, where the tree holds it. *Cost: none — a
  file attribution.*
- **R-C16** (Task 6): `api.rs`'s `undo_allowed` routed through the pair's first half. *Cost: a one-line revert.*
- **R-C17** (Task 6): the three callers' routing PINNED by one same-name re-point cell; `({controller})` when the map is
  empty. *Cost: one cell.*
- **R-C18** (Task 6): a FINISHED same-name re-point shows the CLI's drift warning — left; D's item. *Cost: one stray
  warning on such a screen.*
- **R-C19** (Task 7): `type Opening = { page: Page; id: … }`; `sent()` stays dropped. *Cost: one field on a harness type.*
- **R-C20** (Task 7): the L-5 hole survives in the browser host's replay — left, recorded forward. *Cost: an absent host
  field in the browser replay passes unread until D.*
- **R-C21** (Task 8, amended twice): `docker.yml`'s cold set stated whole; R-C9's wall read only from a run whose log
  shows `[server 10/10]` executing, **at PR #41's `pull_request` run** (the `push` trigger is master-only). *Cost: the
  fallback judged on a warm wall.*
- **R-C22** (Task 9): FOUR cells go red with `without_version_drift` reverted, not two — a stronger guard. *Cost: none.*
- **R-C23** (Task 11): six places beyond the brief's letter, each the item's rule applied consistently. *Cost: small
  reverts, each a named hunk.*
- **R-C24** (Task 11): Abandon's refusal gets its OWN strip beside its control. *Cost: one strip element.*
- **R-C25** (Task 11): the `failed` class keyed on failure. *Cost: none.*
- **R-C26** (Task 12): the scrub's rule read as "prints nothing outside the plan's own command quote". *Cost: none.*
- **R-C27** (B.2): the review's triage adopted as written, plus m-4's two THROWN sentences. *Cost: a D item done early.*
- **R-C28** (B.2): the review's own loopback clause was FALSE (`worker` trusts its header unverified); the implementer's
  true limit adopted verbatim. *Cost: one paragraph of a guide.*
- **R-C29** (D): the ONE recorded sequence FIRST, on the quiet machine; the battery after it, concurrent with the
  deploys. *Cost: a battery row finding a product defect after the deploy would cost a second deploy — judged unlikely
  after ten re-run mutants agreed with every report.*
- **R-C30** (F.4, amended): the red live run re-run ONCE, ALONE, with a fresh tail attached — the attached tail the
  load-bearing condition, not a quiet machine — both outcomes the record. *Cost: one more live game's quota; or a finding
  believed one run late.*

The plan's corrections that needed no ruling — Task 1's Step 4/5 reds, Task 2's guard checks, Task 3's five readings,
Task 8's six departures, Task 9's Files line, Task 10's four facts, Task 11's heading level, Task 12's "three times" — are
the spec's §10.8 rows.

---

## 6. The LEAVE list and the owner items

### 6.1 Inherited and not touched

The M6.5c close's §6 and the cycle A and B handovers' standing items, where this cycle did not take them: the public
door's preconditions (Access on both hosts; the path-scoped application built, not deployed; the dev door ON in
production's variables for the live tiers; the art unpublished); the phones; polish items 5–9; the lane document and the
top-level refresh (R-6.5c2.25); the seven screenshots; the merge and the tag (R-6.5c2.20).

### 6.2 This cycle's own — deferred, each with its reason (cycle D's list)

- **The `plays_locally` quota cell's premise** (§2.1): step through a sub-picker, or assert the click applied a move before
  arming the expect. Reason: a test's premise, seed-conditional, red in two closes' sequences.
- **The `two_browsers` live probe** (§3.4): one unexplained socket close under a loaded workstation with the tail down;
  a re-run read none over 1,795 moves. Reason: unexplained, not reproduced; the next live run reads the tail first.
- **The rows-read figure's transcription**: the Worker module's 1,258,106 B into the root `Cargo.toml` paragraph
  (1,256,770 at `d431dbc1`); the browser module's 1,197,069 B beside its 1,175,223.
- **The fourteen `RUNBOOK §6.x` citations** the fix wave left (twelve after the two thrown sentences): a D sweep, in
  one commit, naming the count it starts from (the review's E-2).
- **The closed Finished fold's cue** stays quiet at 1× after the `--fg-dim` marker; a larger glyph needs the owner's eye.
- **The lobby's density at 390** (a creator's row ~146 px from the 44 px rule; a hit box around a visually small Abandon
  is the alternative); the crumbs wrapping under the pill at 390 (polish 7); `1lh` centring on old Safari; no jsdom cell
  for the 44 px on the open table and the device card (the one-off log witnesses it).
- **The deferred minors by task**, as the ledger lists them and the whole-branch review triaged them (D or drop): the
  two `deploy-guard` comments; `deploy-guard.mjs`'s success line; `archive.test.ts`'s three written-out closures (drop);
  `identity_inner`'s hand-built 401; `receipt.rs`/`picker.rs`'s "one document" class; `module-figures.mjs`'s LEB128,
  section bounds and gzip wording; `lib.rs:507`'s "only the two calls"; the `status` arm comparing the layer with itself;
  the scan partners' comment; `SECOND_PLAN`'s unnamed premise; `acknowledged`'s doc; the memo's unstated invariant
  (`forgetTables` in `batch`'s catch, or the sentence); two long comment lines (drop); `operator.test.ts`'s optional
  `DELETE` (drop); `bots.rs:333`'s "only when"; `read-deploy.mjs:178`'s twin; `live.ts:322`'s omitted reason; `test.yml:210`'s
  "to the clock"; `docker.yml:49`'s warm example; `record.rs:181`'s "never-bumped" (FENCED — the AI lane's, never a web
  commit); `plays_locally.spec.ts:335`'s detached-HEAD read; `styles.css:2473`'s shared selector; `setup_screens.spec.ts:331`'s
  ~18 px; `OpenTable.tsx:82`'s error sources; `styles.css:2487`'s grouping; R-C10's 500; R-C14's class; R-C18's warning;
  R-C20's L-5 hole; the 34 rustdoc warnings in `yokohama-web` (pre-existing); two jsdom cells that failed once in five
  runs under load 5.9 (a flake candidate).
- **The E.6 downtime**: 25.3 s where one script would read ~8 s — the recipe as four proxy calls; D may script the swap.
- **The Keychain item** `portainer-mcp` that the morning's session had and this one did not: the token's places are the
  helper's; the runbook's E.6 now names the checkout's `.dev.vars` first.

### 6.3 OWNER ITEMS

1. **The phones** — the iPhone exit and the Android handset against BOTH hosts at `0.6.0`, now that the deploy exists
   (R-6.5c2.27); D may layer in the Device Hub iOS simulator.
2. **The merge order and the tag** — the stack #31 → #41 (and D's #42) with merge commits, each PR retargeted by hand
   before its merge (`delete_branch_on_merge` is false), then `v0.6.0` on master and its `release.yml` run — D's, on your
   word (R-6.5c2.20).
3. **The public door's preconditions**, recorded and not taken: the path-scoped Access application; the dev door OFF
   (`YOKOHAMA_DEV_DOOR`, and the secret deleted); the art's publication; requests per move (~13.8) as the free plan's
   first binding limit.
4. **The polish calls that want your eye**: the closed Finished fold's cue; the lobby's density at 390; the crumbs at
   390; a "connecting" word on a first visit (the M6.5c handover's item).
5. **The `two_browsers` probe** (§3.4): whether one unexplained watcher socket close in a first run, with a clean 1,795-move
   re-run, is a finding to chase in D (the next live run reads a fresh tail first) or a recorded limit.
6. **The `wrangler tail` instrument**: it drops under a burst and samples; whether D's live runs should read the Workers
   Observability query API instead (the runbook's §7 names it).
7. **CI refocusing — your direction at this close (2026-09-28), taken into D** (the D handover §6 item 11): the web
   jobs are "heavy handed CI for a segment … nearly ready to freeze"; path-filter `test.yml`'s three web jobs to the web
   lane's inputs as `docker.yml` is, keep `workflow_dispatch`, and let a core-only push pay the core `test` job alone. The
   one coupling, stated so it is not feared: the committed corpus pins the engine's wire-visible words, so a core change
   that moved them fails `cargo test -p yokohama-web` — run locally after every task by the per-edit rule already.

---

## 7. What this cycle does NOT claim

- **No public door, and no open-door Worker is deployed.** Access covers both hostnames; the `open` door is proven in the
  pool's second project, the local e2e and `docker.yml`'s smoke; its one refusal that differs from the NAS's (R-C10) is
  recorded, not fixed.
- **CI has not run this branch at this close's HEAD.** The `worker` job, the three additions to `web-server`, the
  test-binary guard and `docker.yml` are proven by structure checks, stub runs and a clean-export rehearsal on one
  workstation; their first runs, `docker.yml`'s cold wall (read at PR #41's `pull_request` run) and build-push v7's
  build-record upload under read-only permissions are the push's readings, recorded in §8 once read.
- **The NAS's `cf-access` was read ONCE, as the owner, and no other person.** No game was played on it; `run_id` and the
  mirror's `bot` sit on no document there yet (the one table predates the deploy); its update path and the edge's
  `index.html` rewrite were not re-read.
- **No not-over foreign-bot game exists on either host**, so the versioned refusal sentence and the foreign-bot 409 are
  cell-read, never live; a finished same-name re-point still shows the CLI's drift warning (R-C18).
- **The livelock bound is measured over recorded seeds, not proven**; the gate is back in CI bounded, not unbounded.
- **Overlapping uploads' pair consistency is best-effort**; production R2's create-if-absent under a real race was not
  watched; a refused stamp after a `landed` upload retries hourly until the creator's Abandon or the expiry, by design.
- **The refinish rule is proven in the pool** (two race cells and the module's held-record cells); no live game met a
  conflict.
- **Requests per move did not move** (~13.8, M6.5c's figure); what fell is rows read (~470 → ~56, one harness game's
  reading, three pages refetching on every frame); a plain two-person game without the harness was not measured.
- **One unexplained watcher socket close** (§3.4) stands beside one clean re-run; the tail was disconnected at that
  moment, so what closed it is not known.
- **`picker-dropped` was not observed**: no table object stored under earlier code existed to be asked; the rule is the
  pool's.
- **The memo's premise** (one `SqlStorage` per instance, a new one after eviction) rests on a throwaway pool cell; a reset
  under `blockConcurrencyWhile` was not measured; its failure would cost one extra scan per read, never a wrong answer.
- **The L-5 presence rule is stated in the two Worker replays, not the browser's** (R-C20).
- **The bot's version by ingestion**: the versioned refusal sentence lands; no key and no record changed (R-6.5c2.21).
- **The pictures are Chromium's**, headless, at 1440 and 390, plus one look at the NAS lobby in the owner's own browser;
  WebKit and the phones are unseen.
- **The `test/` rule's first expiry has not happened**: the rule reads back as set; an object 30 days old is October's
  reading.
- **The address is in the git history**; what a public release does about history is not this cycle's.
- **The seven screenshots, the lane document, the top-level refresh, the merge and the tag are cycle D's** (R-6.5c2.20,
  .25); the phones are the owner's.

---

## 8. The handover

Cycle D starts from **`docs/plans/2026-09-28-web-frontend-m6.5c2d-handover.md`**, written at this close's push: where the
tree stands (the PR, CI's first readings), what D builds (spec §4 row D, restated with its raw material — the lane
document's draft in D8's retired Task 27, the screenshots' recipe against the DEPLOYED site, polish 5–9 with the paths of
C's pictures, the phones, the investigation of what remains, the merge and tag recipe), the rules D carries, what C does
not claim, every ruling of this ledger, and D's standing items (§6.2, §6.3). After D the lane goes quiet and the AI lane
re-enters from `docs/plans/2026-09-11-ai-lane-reentry-handover.md`.

### 8.1 CI's first readings of this branch (H.1; the addendum §7 promised)

The push of `web-room-m6.5c2c` at 17:42 UTC (the close's documents commit `e39ceee2` over `ea4f174d`) fired `test.yml`
alone, and PR #41 fired `test.yml` AND `docker.yml` — as R-C21 (amended) says, since `docker.yml`'s `push` trigger is
master-only. **Every job green.**

- **`docker.yml`, run 36460020302 — the COLD wall R-C9 is judged on: the `image` job 7 min 26 s** (17:42:29 → 17:49:55
  UTC). The log shows the build COLD: `importing cache manifest from gha`, then every server-stage step EXECUTED —
  `[server 1/10] FROM rust:1-slim-trixie` through `[server 10/10] RUN --mount=type=cache … cargo build` (17:45:49 →
  ~17:49:30, the `cargo build` itself about 3 min 40 s), NO `CACHED` server line; `wasm-opt version 133 (version_133)`
  installed in the stage; `smoke: PASS — yokohama-web:ci` under no door, `cf-access` and `open`; the second build WITH
  the project link reused every server layer from the cache. **Under fifteen minutes: R-C9's fallback (to master and
  dispatch only) is NOT invoked.** `docker/build-push-action@v7`'s build-record upload under `permissions: contents:
  read` raised no error (O-1's second half).
- **`test.yml`, run 36460020190 (the pull request): 15 min 13 s** — `web-client` 3 min 00 s; `worker` 4 min 23 s (its
  first run on a runner: `npm ci`, the debug module, the typecheck, `test:scripts`, the pool's two projects, `cargo test
  -p yokohama-worker`, the `release-wasm` build — O-1's first half, green); the core `test` job 12 min 36 s;
  **`web-server` 15 min 07 s** (`wasm-bindgen-cli` from source, binaryen 133, the module, the typecheck, the layer's
  three test binaries with `browser_played_pin`, the test-binary guard, the refusing build's eight cells, `test:wasm`,
  and the whole-game gate LAST and bounded). The push's run (36460015068) ran the same four jobs a minute behind.
- **The owner's direction, read at these figures** (§6.3 item 7): this weight is not wanted for a segment about to be
  frozen; D path-filters the three web jobs to the web lane's inputs, so a core-only push pays the core `test` job alone.
  These are the last full-run figures before that filter.
