# Handover — the owner's task board

**What this is.** The one screen to open before starting a session: the tracks in flight, the
next concrete step on each, and the rulings they are waiting on. It is a POINTER — every fact
here names the document that carries it, and it never outranks `guidingpath.md` (the authority),
`CLAUDE.md` (the operating facts), or a dated document in `docs/plans/`. Keep it current at every
session close: rewrite the stale lines, do not append history (the decision log is for history).

**Updated:** 2026-10-11 (sixth session) — S1c "the plumbing" RUN AND CLOSED on `security-s1c` at `30088995` (Track 4): the pre-flight stopped at your direction after Task 1's cells, every task Opus/Opus, eight tasks landed and reviewed, one pre-existing defect found and fixed (a HEAD ran a write arm — the ledger's B13), version 0.6.1, the roster 484 s, the wave done, the push and PR #51 done; the battery landed after the close (77 mutants, 75 red at the named cells, T4-M19 green by design, ONE survivor — T1-M15, a false doc sentence, parked for Step 8) and PR #49's S1 summary appended; **NEXT (at the top of the next session, before S2's brainstorm — your direction at 94% context): the battery's report read and any survivor ruled, then the four-agent stack review, the merges and the tag, the deploys at will (both windows open), the hand-off to S2 (Track 4).** 2026-10-10 (fifth session) — S1c "the plumbing" PLANNED on `security-s1b` at `680737e5` (Track 4): your seven rulings taken, the plan `docs/plans/2026-10-10-security-s1c-plumbing-plan.md` written (untracked, for your read), the stale residual pair found; **NEXT: commit the plan and the board on `security-s1c`, run the pre-flight, then the SDD run and the wrap of S1 (Track 4).** 2026-10-10 — S1b "the cost bounds" COMPLETE on `security-s1b` at `17085e72` (Track 4): seven ledger rows fixed, the corpus moved once, every suite green at the head, the run's five mechanism holes closed by review, PR #50 (`security-s1b` → `security-s1`, stacked under PR #49) open; S1c is next, planned in a fresh context. **NEXT: S1c's plan (Track 4), in a FRESH session on `security-s1b`; the computer player (Track 1) resumes after the security cycles.** 2026-10-09 (fourth session) — S1b's plan committed and PRE-FLIGHTED: the before review executed every cell of Tasks 1–7 against the plan's own blocks in an export, read every mutant over the whole lib, measured every bound, cut and committed the spread fixture, and corrected the plan at the source (twenty-eight findings, three load-bearing — Track 4); your direction mid-session: the SDD run starts in a CLEAN context. 2026-10-09 (third session) — S1b's plan written against the stack's head with your knob choice and the comment rule (Track 4). 2026-10-09 (second session) — S1a's plan written on the branch `security-s1` and reviewed by you with three rulings (Track 4); the A2/B12 figures measured. 2026-10-09 — the security remediation's core PR MERGED (PR #48, `9ab8c430`; guidingpath's 2026-10-09 row); the remainder of S1, then S2 and S3, stack on ONE new branch (your ruling). 2026-10-08 (second session) — the core PR plan's "before" review done and PR #47 open (Track 4). 2026-10-08 — the security review of the web lane and its remediation specification (Track 4; guidingpath's 2026-10-08 row). 2026-10-04 — the instruction files split by role (guidingpath's 2026-10-04 row): a session starts here, and CLAUDE.md holds standing rules only. The web lane is complete (`v0.6.0`, 2026-10-01) and the V5 ship's one CLI line has landed (the hotfix row).

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

## Track 4 — Security remediation: S1 COMPLETE, MERGED AND LIVE on both hosts (`v0.6.1`, 2026-10-11); S2 next — its brainstorm from the ledger

**State (2026-10-11, the seventh security session — the wrap):** **S1 is DONE.** The whole-stack review (S1c Task 9 Step 8; four agents — two Opus, two Sonnet — over `286d5985` against master: 36 findings, 25 after the dedup, no Critical, every one verified by the orchestrator), its fix wave (one Opus implementer, six commits `71b5e329` … `93c7a593`, every catcher red under its mutant on the committed file; the HI-F1 label echo grew into `bots::echo_record_values`, the `why` refusal's message is capped at the boundary, LINE_MAX/MESSAGE_MAX/the gate/OPTIONS_CAP/UNLOCKED_CAP/the archive share pinned at their figures, B12's race cell, the guard's parsed keys, the outbox's 413, the four parked items and T1-M15's catcher) and the scoped re-review (one Sonnet agent, 61 claims, 59 held, its two failures closed by the sixth commit and re-verified under both mutants on an export) — then **the merges, each a merge commit: PR #50 `55641e3b`, PR #51 `aefc64f1`, PR #49 → master `73469227`; the tag `v0.6.1`; `release.yml` published the release (run 38133636216, 11:00Z) BEFORE either deploy, your order.** The record of record is the S1c plan's §"Review record" (the stack review entry and the wrap entry) and the two runbooks' records.

**Both hosts are live on 0.6.1 (the window 10:55–11:17Z, one window, `deploy/RUNBOOK.md` §3.21 and `worker/RUNBOOK.md` §21):** the NAS `yokohama-web:0.6.1-s1` (`50dad60b`, the four `Env` entries) swapped with **the archive's rename's NAS half between stop and start** (one legacy folder, `live-review_at_m65b.test`, moved by the guarded `mv`, exit 0; the shape check clean); the Worker `ee8f7777` through the guard, **the R2 half after it** (eight keys moved and read back identical — `jamoore_at_gmail.com`, the bare `window-reader-g` onto its own derived segment, `smoke_at_readme.test` — and the replay's four legacy keys DELETED, not moved, the stack review's AH-F3), the live replay twice HOLDING (the first sends 201 then the once-rule's 200), then **the gate `fd34c477` (the dev door off) last**; both update readings 11 of 11 (fresh profiles, `web/scripts/pinned/{nas,worker}-s1`). **The live acceptance** on both hosts: A1 (no seed field in `suggest`), A2 (the 1 MiB review body → 413 in 0.35 s), B3, B8 (1,000 × 404 in 11 s, memory flat but for page cache), B9 (`no-store` / `DYNAMIC` on the NAS; the Worker's half is S2's), B13, D12, D26 (the rename and the replay's landing under the digested key), D28 — **D1 and B1's WITHHELD `why` were NOT read live** (one Access identity on the NAS; the Worker streams over WebSocket; the placement picker's selection is the client's) and stand on their cells. **The five configuration changes are made and read back:** the Access cookie `SameSite=Lax`, the application session 24h, the gate deployed, `.dev.vars` 600 without its Turnstile line, the stopped `cloudflare-d` removed and `bdvc-llc`'s tunnel secret rotated. The ledger's live column is filled where a probe was taken; its `Updated:` line says so.

**Next: S2's brainstorm, from the ledger (`docs/security-ledger.md`) and the stack review's record:** B4, B5, B6, B10, C1, C2, C3, D3, D8, D13–D16, D18, D23, D24; **B9's Worker half** (`respond.ts`'s API answers carry no `Cache-Control` — the stack review's DC-F4); **B12's Worker object quota**; the stack review's UNVERIFIED list (the withheld-`why` timing channel; a wrangler secret named like the switch; the request-body read timeout, D3's; the Worker suites under the figure mutants; a crafted bare name equal to another address's current segment under the open door; wrangler's inferred Content-Type on a put; the 48-cut in WASM against native; README's 0.6.0 lines); HI-F2's design option (refuse `undo` past game over once the record or a public account was served — the solo table's exception is a document fence today); the two live readings not taken (D1; B1's withheld `why` — a second Access identity, or the client's picker driven by Playwright); and the Access configuration changes R-SEC8(g) still the orchestrator's. **Residuals S2 also inherits** (the ledger's deferred minors): cell (a)'s 500/405/HEAD rows and its DELETE order (S1c Task 3's O5); the e2e harness's one segment function (O7); the three pre-existing unbounded joins MG hangs (`tables.rs:4466`, `:3140`, `games.rs:3003`); the Worker's `identity_inner` 401 outside the cap's layers; the browser restore report's own ids now echoed (the wave's O1); `absent`'s constant beside `User::parse`'s sentence in §15; `Games::lock_for`'s `.expect` (a poison there is unreachable short of allocation failure — recorded by the fix wave, not changed). **Two operating notes for the next window:** a live tier that needs the dev door's claimed identities now needs `npm run deploy` before it and `npm run deploy:gate` after; and the workstation's WARP identity can lapse while the client reads `Connected` — `warp-cli debug access-reauth` before any bare `curl`.

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
