# Handover — the owner's task board

**What this is.** The one screen to open before starting a session: the tracks in flight, the
next concrete step on each, and the rulings they are waiting on. It is a POINTER — every fact
here names the document that carries it, and it never outranks `guidingpath.md` (the authority),
`CLAUDE.md` (the operating facts), or a dated document in `docs/plans/`. Keep it current at every
session close: rewrite the stale lines, do not append history (the decision log is for history).

**Updated:** 2026-10-12 (the ninth security session) — S2a's SDD run is COMPLETE and LANDED on `security-s2` at `af7f30d5` (Track 4): the eight tasks, every task review and re-review, the after review's one wave; every per-edit suite and the web lane's rows green at the final head (`suites.sh`, 1,657 s), the fifteen-gate roster 487 s at the code head `4101fada`; PR #52 open, the merge S2's wrap. **NEXT: the first of S2's two windows — it opens by agreeing the two-identity block's slot with you (the tag `v0.6.2` and the release BEFORE either deploy, both hosts, the header reads, the Report-Only reading, the block) — then S2b's brainstorm and plan.** Two items for you: four parked comment sentences (Track 4's State; ten minutes, or S2b's), and the leftover scratch for you to remove.

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

## Track 4 — Security remediation: S1 LIVE (`v0.6.1`); S2a LANDED on `security-s2` (head `af7f30d5`, PR #52 open, 0.6.2 built; the tag at the window) — NEXT: the first of S2's two windows (your slot for the two-identity block), then S2b

**State (2026-10-12, the ninth security session — S2a's run):** **S2a "the browser's fences" is LANDED on `security-s2`** — the branch cut from master `1e9c092d` (X1; S2b and S2c stack on it; ONE merge at S2's wrap), the plan and this board `5a03a609`, the pre-flight `5a4c9444` (Tasks 1 and 2's cells green on the plan's blocks; the debug-client e2e under the policy ENFORCED **81/81 with no CSP line** — the wrap's D-9 input; the mutant pass stopped at your word), Tasks 1–8 `61db24e0`…`4101fada` (the bump to 0.6.2), the after review's wave `ee2f4868`/`52939c39`/`af7f30d5`; head `af7f30d5`. The ledger `docs/security-ledger.md`: B4, D13, B9's Worker half, D14, D15, D18 and C3 `fixed` with their catchers, D16 `accepted`; guidingpath's 2026-10-12 row is the record; the plan's §"Review record" carries the pre-flight, the run and the after review; the SDD ledger `docs/temp/security-review-2026-10-08/s2a-sdd/progress.md` every ruling. **The run:** every task's cells green on the plan's blocks at first run; every task but 4 and 8 reported a concern and needed one fix round, each corrected IN THE PLAN first then transcribed (the readers made exact rather than narrowed three times; the guard's `_headers` arm outside its loop; the 401 row; a CSP line never swallowed by the status filter; `wrangler login`, not `whoami`; D-8's W-1 citation §20; the README's `workers.dev` bullet); the after review re-derived every row from the code, the battery confirmed all 78 named mutants, the wave took sixteen deferred minors, re-reviewed clean. **The readings:** the per-edit suites and the lane's rows at `af7f30d5` (core 1,587/29, CLI 862/2, oracle 191, difftest 52/5, eval_probe 214, solver 132, bridge 81 + 4 node, clippy clean; web 534, wasm 10, worker 77, vitest 1,511/73, test:wasm 372/16, test:scripts 561/26, pool 226/11, the e2e 81), the roster 487 s at `4101fada` (beside 484 s), the eight fenced crates byte-identical, `SEMANTICS_REV` 7. **For you:** (1) four PARKED comment sentences with their true forms in the SDD ledger (`headers.mjs` ×2 — the ruling's own wording about "every read"; `lib.rs` ×2 — an echo of B3's cause) and D2's `console.info` label unpinned — ten minutes as one commit before the tag, or S2b's; (2) the leftover scratch, bulk deletes being forbidden to agents: this session's exports under `/private/tmp/claude-501/…/99b051bb-…/scratchpad/` (~10 GB: `export-1e9c092d`, the `review-*`/`rereview-*`/`battery-*` exports), `$TMPDIR`'s 105 `yokohama-headers-*` and 209 `secrets-guard-shim-*` trees from before the cells learned to clean up, and the earlier sessions' 83 GB of scratchpads; (3) conduct: one Task 6 reviewer's probe command carried a stray `rm -f /dev/null` (refused by the harness, `/dev/null` intact — the same slip the plan review's documents lens made); (4) S1's two pinned profiles' Access cookies lapsed 2026-10-12 ~11:05Z. **State of the tree:** `web/dist` holds the e2e's DEBUG client (the window's `npm run build` restores the release one before the Worker's assembly); `worker/gen`/`worker/assets` are the release builds of the live-shape measurement.

**Next (Task 9 Steps 8–10 of the plan — the window needs you at the keyboard):** (1) **Step 8, the tag and the release BEFORE either deploy** — on `af7f30d5` (or the head after the parked sentences, if you take them): `git tag -a v0.6.2 -m "Yokohama CLI v0.6.2 — security S2a: the browser's fences, the header set in Report-Only" && git push origin v0.6.2`; `release.yml` runs on any `v*` tag; read the release's assets on GitHub before the window. (2) **Step 9, the window** — it OPENS by naming the two-identity block (d) and agreeing its slot; WARP off for that slot only; every `curl` over WARP with the smoke's `BROWSER_UA`; the quota guard first (`worker/RUNBOOK.md` §7); the live replay's reset recipe (§7 step 6) BEFORE the replay (Task 8 moved the archive fixture's records); `cd web && npm run build` (with the site URL) and `cd worker && npm run build` first. (a) The NAS: the update reading pinned FIRST (`node web/scripts/pinned-browsers.mjs nas https://yokohama-nas.moore.nyc web/scripts/pinned/nas-s2a --go-wait 90`), the runtime image from the tagged commit (`deploy/RUNBOOK.md` §1–§3, the tag `0.6.2-s2a`) through Portainer, health `server_version: 0.6.2`, the four header reads live after the health read (`x-frame-options: DENY`; the policy under `content-security-policy-report-only`; `/api/health`'s `cache-control: no-store` and `x-content-type-options: nosniff`), then `go`, then `done`. (b) The Worker: the update reading pinned (`worker https://yokohama.moore.nyc web/scripts/pinned/worker-s2a --go-wait 90`), `npm run deploy` (the dev door on for the live tiers; the secrets guard's first live run prints its sentence — the one call to the account this cycle), health 0.6.2, `go`/`done`; the live corpus replay TWICE after its reset, HOLDING; the live Playwright tier once (`two_browsers`, `joins_by_link`), its `reportOnlyForgiven` printed; the quota guard again; the update readings' `verdict.json`, `log.txt` (`cookies: cleared before close`) and empty `Cookies`; `npm run deploy:gate` last. (c) The live acceptance: the response headers of `/`, `/shell.bin`, the entry chunk (its hash is new in 0.6.2) and `/api/health` on BOTH hosts against spec correction 10's set, the 404 page's style hash; B4's framed page in Chrome; D13's SW-served navigation on a FRESH profile; **the Report-Only reading** — a fresh profile's first visit to each host, the console at EVERY level (two `info` lines of Cloudflare's detection script expected, anything else a finding), the real line's text recorded against `REPORT_ONLY_INLINE_SCRIPT`; B9's Worker half on a table answer; D14's `#/t/..`; A1 and B3 re-read. (d) The two-identity block: profile A yours, profile B `myfreemovies@gmail.com`, on the NAS hostname (and the Worker's for §8's rows), every probe a page-context `fetch`/`EventSource` from the profile's own tab through Claude in Chrome, the cookie never extracted; D1 at an open table A fills itself; B1's withheld `why` at a bot-first table; `docs/web-lane.md` §8's rows 108 and 119 as B on the Worker; every probe table deleted; WARP back on. (e) The ledger's live column, the deploy records (`deploy/RUNBOOK.md` §3.22, `worker/RUNBOOK.md` §22), the rollback bullets (`wrangler rollback` to `fd34c477`; the NAS's `yokohama-web:0.6.1-s1`). (3) **Step 10, the hand-off into S2b:** this Track rewritten with the window's readings; S2b's brainstorm and plan against `security-s2`'s head (B5, B6's `.test` arm, C1 with D24, C2's `ratelimits` figures from measured calls per move, D3's socket cap, B12's Worker quota, D23, the escaped-key/48-cut/crafted-name cells, an unlisted WORKER-door `--config` file's sibling rule (F-40), S4's first-deploy path past the secrets guard; the Report-Only reading as the wrap's input with the four shapes). **Operating notes that stand:** a live tier that needs the dev door needs `npm run deploy` before it and `npm run deploy:gate` after; `warp-cli debug access-reauth` when Access answers "Please authenticate via the warp client"; off WARP a `curl` is no transport (W-1); the secrets guard makes every deploy script a network step. **Residuals S2b and S2c inherit** stay as listed before, plus S2a's: the deferred minors the wave did not take (the SDD ledger's "not taken" list), the §9.5 gap, the older `curl` recipes without `BROWSER_UA`.

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
