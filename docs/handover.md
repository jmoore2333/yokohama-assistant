# Handover — the owner's task board

**What this is.** The one screen to open before starting a session: the tracks in flight, the
next concrete step on each, and the rulings they are waiting on. It is a POINTER — every fact
here names the document that carries it, and it never outranks `guidingpath.md` (the authority),
`CLAUDE.md` (the operating facts), or a dated document in `docs/plans/`. Keep it current at every
session close: rewrite the stale lines, do not append history (the decision log is for history).

**Updated:** 2026-10-11 (eighth session, continued) — S2a's plan WRITTEN and REVIEWED (Track 4): a four-Sonnet + one-Opus Workflow executed the plan's blocks on exports and found 56 things — one Critical (Chromium logs a Report-Only CSP violation at console `info`, so the plan's local enforcement reading read nothing; D-9 rewritten) — every correction spliced at the source and six adjudications ruled; your four rulings taken (D-7 the tag on the branch; D-8 the two-identity block done LIVE with you, slotted at the window's start; D-2 and D-1 as proposed); the plan `docs/plans/2026-10-11-security-s2a-browser-fences-plan.md` (3,038 lines) still UNTRACKED beside this board's uncommitted rewrite; **NEXT: `git checkout -b security-s2 1e9c092d`, plan + board committed, the trimmed pre-flight, then S2a's SDD run from Task 1.** One item for you: the review's conduct and two live cookies at rest (Track 4's last lines).

## Gates at HEAD

The latest readings: security S1's wrap (guidingpath's 2026-10-11 wrap row; the S1c plan's §"Review record", its stack review and wrap entries) — the web lane's suites at `93c7a593`, the content of master `73469227` / `v0.6.1`: web 529 (lib 494), Worker 77, wasm 10, the pool 224, test:scripts 543, vitest 1,507, typecheck and workspace clippy clean, the wasm32 check clean; core 1,587 / 29, the CLI 862 / 2, oracle 191, difftest 52 / 5, bridge 81 + four node suites, eval_probe 214, solver 132 — unchanged since S1b's close (nothing under those crates moved in S1c or the stack review); **the fifteen-gate roster 484 s on the Mac at `04713bed`** (S1c's close; the evaluator gate 254 s; ≤ 420 s stays crossed — OD-6), not re-owed since. Before it, the core PR's 451 s and the M5 MacBook Air baseline (Darwin 27.0.0 arm64; the target ≤420 s and its restatement, OD-6, is yours). Re-count at your own HEAD.

## Track 1 — V4 close-out: the owner annotates the ledger, then the findings doc, then D3 + W6

**State (2026-09-11, second session):** three complete games against the shipped bot, all scribed
by reconstruct-by-replay (`why <n>` at every bot event on copies of the records, release build):

| game | file (gitignored `docs/temp/owner-games/`) | result | events | bot events / refusals |
|---|---|---|---|---|
| Game 4 (CLI, 2026-08-30, you started) | `game4-v4.FINAL.json` · `game4-why.log` | **you 82 – bot 81** | 247 | 119 / 0 |
| Game 6 (CLI in Forge, 2026-09-11, bot started) | `game6-v4.FINAL.json` · `game6-why.log` | **you 120 – bot 81** | 252 | 127 / 0 |
| Web game `sim-551da3b64523dbd1` (browser, 2026-09-11, you started) | `web-2026-09-11-sim-551da3b64523dbd1.json` · `web-2026-09-11-why.log` | **you 89 – bot 68** | 243 | 124 / 0 |

`game5-v4.json` (40 events) was abandoned by your choice at the table; `sim-016f847457c957a6`
(100–56, the first web game) stays an optional exhibit, unscribed. The scribe notes with the
setup echoes, self-read curves and event-anchored scribe-side reads for all three are
`docs/temp/owner-games/notes-2026-09-11.md` (its last two sections carry the reviewers'
corrections to the scribe's own claims — read them before quoting the earlier sections).

**THE LEDGER (where your annotation happens):**
`https://claude.ai/code/artifact/a57eb6f8-dd20-4266-b50b-1c94c5eb2591` — the three games turn by
turn: every move with its event index, the bot's own arithmetic for every decision (self-read,
stage, candidates, ties, nonzero feature rows, runner-ups, searched line), an Opus reviewer's
commentary on every turn (one reviewer per game, each having read the rules reference and the
Phase-B findings first), VP and self-read charts, key moments, and per-turn tag + note fields, a
verdict and a general field per game, and answer fields under the reviewer's questions. **Notes
save as you type into the artifact's database** (the status line top-right reads "notes saved";
if it reads "not connected", the note is in that tab only — reload and retry). The page's sources
and the scribe tooling are parked in `docs/temp/owner-games/ledger/` (README inside).

**Next session, in order:**

1. **Pull your notes from the ledger** (Artifact tool, `action: read_db`, the URL above,
   `db_op: list`, `collection: notes` → docs `game4`/`game6`/`web`; shape in the ledger README).
   Every note is anchored to a turn number; the datasets map turns to event ranges.
2. **Write the findings package on the Phase-B shape** (`docs/plans/2026-08-17-phase-b-owner-games-handover.md`
   is the precedent): a dated tracked document with event-anchored findings that pair YOUR line
   with the scribe/reviewer grounding, the three outcomes, the routing per D7's branches; plus
   decision-log rows for the three outcomes (game 4's 82–81 is in NO tracked document yet).
   Scribe-side headline findings already on file (notes file + the ledger's reviews): (a) threat
   blindness is now measurable in the V4 columns — game 6 e109 read +951 with your three
   fulfillable orders in its table at weight 0 (`held_order_value` is weighted 0 until the end
   trigger; `executable_orders`/`matched_production_access` 0 always; loading a threat RAISES its
   read), and the web game's e124–e148 customs landing (+22) was priced only through
   `imported_goods` w6; (b) game 4 e241 is terminal-exact and the shipped beam (b=4, m=8) had 12
   lines tied at lose-by-1 while `driven table …:241 --contestant lookahead-v1:b=16,m=32
   --decision-seed 1013458669601587549` finds "Place 2 in Bank" one VP better (82–82, still lost on
   the clockwise tie-break) — a search-breadth miss, weights irrelevant; the game-4 reviewer adds
   that the point was really lost at the Church (2-to-2 rightmost tie-break, a 3-point swing);
   (c) produce-and-hoard live in game 6 (7 tea held e68→end, one order all game); (d) the web
   game's final round HAD intent (e214–221) where game 6's last turn was dead; (e) instrument
   note: the stage proxy jumped 750000→1000000 inside one bot turn (game 6 e217→e219), and the
   forced turn-start node consistently reads worse than the real root one event later (both
   reviewers) — those reads are not part of the self-read curve.
3. **Rule D3 and W6 together** with the package in hand (close doc §3.2: a word suffices on each).
   The scribe's recommendation, offered 2026-09-11 and NOT a ruling: **D3 YES, W6 WAIT**, with a
   V5 (weights) cycle entering only through its own pre-registration after the D3 diagnostic read
   — because two of the three headline findings are representation (weights), the breadth exhibit
   is a strictly-better-not-winning line, and the close's own sentence is "W6 tunes the
   THEN-SETTLED table". A GO is a fully defensible reading of "beats the previous bot at 0.69".
4. Then the ladder as the amendment wrote it (§3 Phases D–F): W6 screening → D1 identity freeze →
   W7 (the @V1 instrument is blocking implementation work, then the OD-5 pre-registration you
   sign, then ONE confirmatory battery) → OD-8 acceptance play → OD-11, the bar. A weight change
   is a DIFFERENT contestant (D1), never a tune.

**The V5 ship's one CLI line is DONE (R-F4; the hotfix, 2026-10-01):** a V5 build plays V4 games
on the web exactly when it still derives V4's exact descriptor, `LOOKAHEAD_IMPL_VERSION` and
`EVAL_IMPL_VERSION` unmoved — a weights-only V5; the admission is in place since 2026-10-01
(`GRANDFATHERED_NAMED_IDENTITIES` in `crates/yokohama-cli/src/bot.rs`). **What the re-point owes
this line is the append:** the V5 ship adds its identity to that list and never removes one. A V5
that moves either refuses V4 games as DRIFT (the re-entry handover §5, item 12;
[`web-lane.md`](web-lane.md) §15; [the spec](plans/2026-10-01-cli-grandfathered-identity-hotfix-spec.md)).

**You decide:** D3 · W6 · OD-6 (restate the ≤420 s roster target — the latest sequence read 534 s at cycle F's close) · OD-8's
format (game count, rubric — its inherited "brake behind" line contradicts canon ruling 15).

## Track 2 — Repo reorganization, licensing, the public split (needs a brainstorm session)

**State:** the code is Apache-2.0 end to end; no per-file SPDX headers (deliberate for now — the
root `LICENSE` + `NOTICE` carry the grant; headers are a public-split-day decision). `ASSETS.md`
names the four excluded paths and says a public release is a FRESH-HISTORY repository (the art has
been tracked since commit 32). The mixed assets stay in this private repo for now, by your call.

**Open questions for the brainstorm** (bring your notes): the public split's shape and timing;
the root layout (`analysis-1/`, `tts-assets/`, `bridge/`, `deploy/`, `docs/temp/`); whether the two
rulebook PDFs stay (the owner's edition is `tts-assets/pdf/…`; `analysis-1/`'s is the outlier);
what a public README claims. Session shape: brainstorm → spec → plan, in its own branch.

## Track 3 — The web client: complete

**State:** WRAPPED 2026-10-01 — merged to master (`b1052bed`, fourteen merge commits), tagged `v0.6.0`, the record the decision log's row of that day; [`web-lane.md`](web-lane.md) is current-state as F left it, with the day's corrections recorded in the row rather than edited. Both hosts run F's release behind Access; the edge rule stays disabled, Bot Fight Mode on; the open-door recipe (`web-lane.md` §10) is the public-split day's.

**Optional, if you want them:**
1. **The handset's optional ten minutes** (R-6.5c2.33): your own phone against either host, to confirm what the simulators read — the wake after the Home screen, the storage, anything that blocks ([`web-lane.md`](web-lane.md) §3's phones row; the readings `worker/RUNBOOK.md` §19's phones bullet). The same bullet records three owner's-eye layout readings, not fixed: the game header's share of the iPhone's portrait height, the iPad's tiles, the chat input under Android's keyboard.
2. **The blurb's words**, if you want them changed: the placeholder text E shipped is quoted in [`web-lane.md`](web-lane.md) §14.

## Track 4 — Security remediation: S1 LIVE (`v0.6.1`, 2026-10-11); S2 PLANNED as three cycles; S2a's plan written AND reviewed, its corrections spliced — NEXT: commit on `security-s2`, the trimmed pre-flight, then S2a's SDD run

**State (2026-10-11, the eighth security session — S2's brainstorm, S2a's plan, and the plan's review):** **S1 is DONE and LIVE on both hosts**; the wrap's record stands as the seventh session left it (`deploy/RUNBOOK.md` §3.21, `worker/RUNBOOK.md` §21, the S1c plan's §"Review record", the ledger's live column, guidingpath's 2026-10-11 wrap row). **S2 is THREE cycles, cut between the hosts** (your ruling, 2026-10-11, after the count: grouped by mechanism, two cycles put fifteen tasks in the second): **S2a "the browser's fences"** — B4, D13, the CSP in Report-Only, B9's Worker half (DC-F4), D14, D15, D16 `accepted`, D18, C3; the bump to 0.6.2; the first of S2's two windows; **S2b "the Worker's door"** — B5, B6's `.test` arm (the token's duration `accepted` by you), C1 with D24, C2 (the GA `ratelimits` binding, figures from the measured calls per move for a couple of games at once), D3's socket cap, B12's Worker quota, D23, the escaped-key, 48-cut and crafted-name cells, an unlisted WORKER-door `--config` file's sibling rule (the review's F-40), S4's first-deploy path past the secrets guard; **S2c "the NAS's identity"** — B10 under your dependency fence (minimal, Apache-2.0-compatible crates, `cargo tree` recorded), D3's stream cap and the body read timeout, D8, the explain surface's two rules, the inherited residuals, README's 0.6.0 lines, the wrap (0.6.3 enforcing the policy from the Report-Only reading — four shapes now: accept the refusal, a fixed nonce, `'unsafe-inline'`, or `Cache-Control: no-transform`, under which Cloudflare injects no script; the policy carries no `report-to`, so the reading is the window's own sessions — the stack review, ONE merge to master, the second window). **Your twelve rulings** are transcribed in the plan's first section (X1–X6, A1–A6), and **your four rulings at the plan review** beside them: **D-7 taken** (the tag `v0.6.2` cut on the deployed `security-s2` commit, the release published before either deploy; `docs/web-lane.md` §7 records the exception); **D-8 done LIVE with you** (the two Access sign-ins are yours, so the window OPENS by naming the two-identity block and agreeing its slot, before any deploy); **D-2 and D-1 as proposed**. **S2a's plan is WRITTEN and REVIEWED:** `docs/plans/2026-10-11-security-s2a-browser-fences-plan.md` (3,038 lines after the review; nine tasks; the mutants M1–M38 with their lettered variants; eleven decisions D-1–D-11; twelve spec corrections for §9.6) — still UNTRACKED, for your read; this board's rewrite is uncommitted beside it. **The plan review (your shape: "two to four sonnet agents, with an opus agent to assemble"):** four Sonnet lenses on `git archive` exports of `1e9c092d`, the plan's blocks lifted and EXECUTED, every platform claim measured; one Opus assembler; the packet and the four reports under `docs/temp/security-review-2026-10-08/s2a-plan-review/` (gitignored); the record is the plan's §"Review record". 86 findings → 56 (1 Critical, 25 Important, 22 Minor, 8 Information). **The Critical:** Chromium 143–153 and Chrome 155 log a Report-Only CSP violation at console type `info` — no `[Report Only]` prefix — and the harness read `error` alone, so the plan's "local enforcement reading" would have read nothing and its live forgiveness matched no real line (I re-ran the probe myself; it holds). **Ruled (AD-1): both halves** — the e2e build rewrites its own `dist/headers.json` under the enforcing name (`web/scripts/e2e-build.mjs`; the shipped `CSP_MODE` stays `report-only`), and the live tier reads a CSP line at any level, forgiving exactly the measured line at the deployment's origin. The other five: `headers.json` never copied into the Worker's assets (an input of the assembly — else the WAF edge rule's drift cell reds); **the fifteen-gate roster RUNS at S2a's code head after the bump** (spec §5.2, as S1c's did at `04713bed`); the NAS's header reading is taken LIVE after the health read (its image is assembled by Portainer — no local image to smoke); the block's probes run INSIDE each signed-in Chrome profile through Claude in Chrome (the cookie is `HttpOnly`, and W-1 says a `curl` off WARP is stopped whatever its User-Agent — your UA rule stands for every `curl` over WARP), a `umask 077` cookie file the named fallback; the pass-through carries `Cache-Control: no-store`. The load-bearing Importants, each corrected in the plan: the framed e2e cell frames from a second loopback origin (Chromium's Local Network Access check blocks an `about:blank` framer of 127.0.0.1 before the host answers — also re-run by me); `main.tsx`'s refusal gets its e2e catcher; the D15 cell is a recording fake over an exported `leaveNoSession` (the inserted `clearCookies()` lost `verdict.json` on a dead browser); the dockerignore patterns are `**/` (BuildKit let `web/.env*` and `worker/.dev.vars` in); the secrets guard gains command cells with a fake `npx` and refuses an unread listing; `tempfile` is not a dependency; the empty-set notice prints from `main.rs`, never the lib; Task 5 adds `id_cases` to `NOT_SERVED` and `THIRD_COLUMN` by hand; Tasks 2 and 6 share `deploy-guard.mjs` and run one after the other; the update readings are pinned BEFORE the deploys; the quota guard precedes the live tiers. **Conduct, for you:** two reviewers broke the brief and said so — the documents lens ran the secrets guard in its export, which asked YOUR Cloudflare account for the Worker's secret names (read-only, two names, no value) in a review the brief made offline, and typed a stray `rm -rf /dev/null` (no effect); the client lens removed a scratch directory of its own with `rm -rf`. No repository file was written and no host touched; every brief from here says "no call to the Cloudflare account, read-only included". **Also for you:** S1's two pinned profiles (`web/scripts/pinned/nas-s1`, `worker-s1`) hold live Access cookies until 2026-10-12 near 11:05Z — left to lapse unless you want the two `Cookies` files removed by hand.

**Next (the next session, at the top):** (1) `git checkout -b security-s2 1e9c092d`, the plan and this board committed as its first commit (any last note of yours folded in first); (2) Task 9 Step 1's TRIMMED pre-flight on an export of `1e9c092d`, Rust on a private `CARGO_TARGET_DIR` — the review did the rest — the corrected Tasks 1 and 2 cells executed once more with their mutants, the DEBUG-client `npm run e2e` under `enforce` (what enforcement blocks today — the wrap's input) and then under `report-only` with the widened filter, the `--no-default-features --features server` lint of Task 3's gated `get`; every finding corrected in the plan before Task 1 is dispatched, the findings file under `docs/temp/security-review-2026-10-08/s2a-before/`; (3) the SDD run from Task 1 (Opus/Opus; the ledger under `docs/temp/security-review-2026-10-08/s2a-sdd/`): Tasks 2 and 3 in parallel after Task 1's review, Task 4 after Task 3, Task 5 (Sonnet/Opus) beside them, **Task 6 (Sonnet/Opus) after Task 2's commit**, Task 7, Task 8 last; (4) the per-edit suites AND the fifteen-gate roster once at the code head; (5) the after review on an export, the close's documents (the ledger's eight rows, guidingpath's row, this board, `docs/web-lane.md` §8, at most one CLAUDE.md sentence), PR #52 (`security-s2` → master, left open), the tag `v0.6.2` and the release BEFORE the deploys, **the window** — it opens by agreeing the two-identity block's slot with you; the quota guard first; both hosts at 0.6.2 with the update readings pinned BEFORE each deploy; the NAS's four header reads live after its health read; the header reads on `/`, `/shell.bin`, the entry chunk, `/api/health` and the 404 page on both hosts; the Report-Only console read on a FRESH profile's first visit at every console level (two `info` lines of Cloudflare's detection script expected, nothing else); B4's framed page and D13's SW-served headers (a fresh profile) in Chrome; the live replay twice; the live Playwright tier once; the block in its slot (D1 at an open table A fills itself; B1's withheld `why` at a bot-first table; web-lane §8's rows 108 and 119 as the second identity on the Worker host), WARP off, every probe from the profile's own tab; the gate last — and the hand-off to S2b. **Operating notes that stand:** a live tier that needs the dev door needs `npm run deploy` before it and `npm run deploy:gate` after; `warp-cli debug access-reauth` before any bare `curl` when Access answers "Please authenticate via the warp client"; every `curl` at either host OVER WARP carries a browser `User-Agent` or meets Bot Fight Mode's 403 (your note), and off WARP a `curl` is no transport at all (W-1); reviewers run Rust on a private `CARGO_TARGET_DIR` per export. **Residuals S2b and S2c inherit** stay as the seventh session listed them: cell (a)'s 500/405/HEAD rows and its DELETE order (O5); the e2e harness's one segment function (O7); the three pre-existing unbounded joins MG hangs (`tables.rs:4466`, `:3140`, `games.rs:3003`); the Worker's `identity_inner` 401 outside the cap's layers; the browser restore report's own ids echoed (O1); `absent`'s constant beside `User::parse`'s sentence in §15; `Games::lock_for`'s `.expect` (recorded, not changed); HI-F2's design option (refuse `undo` past game over once the record or a public account was served — S2c's explain rules, take or accept).

## Owner rulings outstanding (one line each; detail in the re-entry handover §5)

1. D3 depth re-ask — HOLD; the games are in hand, your ledger annotation is the last input (Track 1).
2. W6 GO/WAIT — HOLD, ruled together with D3 (Track 1).
3. OD-6 roster-target restatement — ripe.
4. OD-8 acceptance format — never set.
5. CLI branch-4 register (`docs/plans/2026-08-18-cli-branch4-touchup-spec.md` §5): decisions 11,
   13, and 4/9.
6. The 5-power port-bonus timing divergence (BGA books at placement, the engine at the take) —
   a D-row candidate you said you would research in the rulebook.
7. The sim-seat / named-identity bot seat option (where "difficulty levels" would live).
8. D10's NAS sidecar sync — no row says it was done.
9. The web lane's V5 notes (a sighted Keep; per-good requirement weights) — bot-behaviour changes
   owing their own pre-registration when a cycle takes them up.

## Entry artifacts by lane

- AI lane: the re-entry handover `docs/plans/2026-09-11-ai-lane-reentry-handover.md`; the ratified scoping amendment
  `docs/plans/2026-08-11-phase3.5c-scoping-amendment.md` (the forward authority for the rest of 3.5c) and the value-architecture
  checkpoint `docs/plans/2026-08-20-phase3.5c-value-architecture-checkpoint.md` (ratified 2026-08-20, the V4 forward authority);
  the V4 close `docs/plans/2026-08-30-phase3.5c-v4-close.md` §3. Phase 3 as a whole is governed by the accepted map
  `docs/plans/2026-07-25-phase3-map.md`, whose §1 settled inputs are owner direction.
- Web lane: [`web-lane.md`](web-lane.md) (the lane's operating record; §11 its records of record), then cycle F's plan [`plans/2026-09-30-web-frontend-m6.5c2f-plan.md`](plans/2026-09-30-web-frontend-m6.5c2f-plan.md) and F's close [`plans/2026-10-01-web-frontend-m6.5c2f-close.md`](plans/2026-10-01-web-frontend-m6.5c2f-close.md), beside E's [close](plans/2026-09-30-web-frontend-m6.5c2e-close.md) (the last two rows of `web-lane.md` §11); operations `deploy/RUNBOOK.md` (§0–§3 and §5 the NAS) and `worker/RUNBOOK.md` (the Worker; `deploy/RUNBOOK.md` §6 points there).
- Bridge (BGA capture → import): `docs/bridge-setup.md`; ledger
  `docs/plans/2026-08-08-phase3.5b-execution-ledger.md`.
- Everything: `CLAUDE.md`, then `guidingpath.md`'s Current state and the decision log's tail.
