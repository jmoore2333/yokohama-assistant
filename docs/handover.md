# Handover — the owner's task board

**What this is.** The one screen to open before starting a session: the tracks in flight, the
next concrete step on each, and the rulings they are waiting on. It is a POINTER — every fact
here names the document that carries it, and it never outranks `guidingpath.md` (the authority),
`CLAUDE.md` (the operating facts), or a dated document in `docs/plans/`. Keep it current at every
session close: rewrite the stale lines, do not append history (the decision log is for history).

**Updated:** 2026-10-09 (third session) — S1b's plan written against the stack's head with your knob choice and the comment rule, untracked pending your notes (Track 4). 2026-10-09 (second session) — S1a's plan written on the branch `security-s1` and reviewed by you with three rulings (Track 4); the A2/B12 figures measured. 2026-10-09 — the security remediation's core PR MERGED (PR #48, `9ab8c430`; guidingpath's 2026-10-09 row); the remainder of S1, then S2 and S3, stack on ONE new branch (your ruling). 2026-10-08 (second session) — the core PR plan's "before" review done and PR #47 open (Track 4). 2026-10-08 — the security review of the web lane and its remediation specification (Track 4; guidingpath's 2026-10-08 row). 2026-10-04 — the instruction files split by role (guidingpath's 2026-10-04 row): a session starts here, and CLAUDE.md holds standing rules only. The web lane is complete (`v0.6.0`, 2026-10-01) and the V5 ship's one CLI line has landed (the hotfix row). **NEXT: fold your notes into S1b's plan, commit it with this board, then S1b's pre-flight and Task 1's dispatch (Track 4), in a FRESH session on `security-s1`; the computer player (Track 1) resumes after the security cycles.**

## Gates at HEAD

The latest readings: security S1a's close (guidingpath's 2026-10-09 S1a row; the plan
[`plans/2026-10-09-security-s1a-leaks-plan.md`](plans/2026-10-09-security-s1a-leaks-plan.md) §"Review record") — every per-edit suite at
`8b7c24dd` on `security-s1` (core 1,587 passed / 29 ignored; the CLI 857 / 2 — 855 + A1's two digest cells; the web lane's rows whole: web 460,
wasm 10, Worker 77, vitest 1,505, test:wasm 353, test:scripts 537, the pool 224, e2e 79 (78 green in the full run; the banner-width spec red on a deal-pick timeout under load, green alone on re-run); the fifteen-gate roster NOT owed — core and the
version unmoved), and before it the core PR's: the fifteen release gates as one sequence, **451 s** on the Mac (Darwin 27.0.0 arm64 — the M5 MacBook Air; cycle F read 534 s; the target is
≤420 s and its restatement, OD-6, is yours). Re-count at your own HEAD.

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

## Track 4 — Security remediation: S1a landed on the stack (`security-s1`, PR #49, merge left undone); S1b "the cost bounds" is PLANNED — its pre-flight and dispatch are next

**State (2026-10-09, the S1b planning session):** **S1a "the leaks" is COMPLETE on `security-s1` at `8b7c24dd`** (the close's docs `604d7269`), PR #49 against master, its merge LEFT UNDONE (your note of 2026-10-09: S1b and S1c stack on the same branch and PR; the merge is S1's wrap after S1c). S1a's record of record is `plans/2026-10-09-security-s1a-leaks-plan.md` §"Review record"; guidingpath's 2026-10-09 S1a row; the ledger with A1, B1, B2, B3, B7, D5, D10, D11 `fixed`. **S1b's plan is WRITTEN and UNTRACKED, awaiting your reading notes:** `plans/2026-10-09-security-s1b-cost-bounds-plan.md` (3,059 lines, nine tasks: 1 A2 at `check_pair` + the body cap + a retry answered before any replay + D29; 2 D25; 3 B12's per-caller share and the outbox's 507; 4 D6 + the two-permit replay gate; 5 the CLI `session.rs` checkpoint ladder + the predicate fold + D27 + the record route's lock-scope pin; 6 A3's fold on the wire + `options_omitted` + the corpus once; 7 the live corpus replay as the production-door walk; 8 the documents, Sonnet; 9 the orchestrator). **Your two rulings of 2026-10-09 are transcribed in it:** the recommended table (R-SEC8(f): 4,096 events / 512 KiB / 256 records and 128 MiB a caller / 2 permits with a 2 s wait then 503), and the comment rule — every knob constant carries a concise in-code comment defending it from real-game measurements and saying the upload path is not designed for worst-case imports, because the spec and plans may not be in the public repository. **Eight decisions the plan takes are recorded in its §"The decisions this plan takes" for your eye** — the three to read first: D-3 (the CLI is touched in `session.rs` alone, the ladder, because the session is the record's one owner; `screen_model.rs` untouched); D-5 (the quota refuses with a 507 the outbox keeps, shows and retries — the outbox ends a row on 400/409 alone, a 403 is the edge's challenge, a 429 the zone's rate limit); D-6 (the live corpus replay narrows to the production-door walk: `wire` and `wire2` are seeded creates beside an open seat, which a production host answers 400 under your S1a rule, and 2,180 of the walk's 2,201 rows sit at them — the replay sends those creates expecting the 400, skips their rows and the lobby pages, and runs the rest; the whole walk stays verified through the pool and the browser module; the `--var` diagnostic-deploy alternative is recorded and refused). Nine spec corrections go to §9.4 (Task 8). The gitignored S1a material stays under `docs/temp/security-review-2026-10-08/s1a-sdd/`; S1b's ledger goes under `s1b-sdd/`.

**Next session, in a FRESH context, in order:**
1. `git checkout security-s1`; read the board, then the plan whole (its header names what to read first: the spec's §1 R-SEC2/R-SEC7/R-SEC8(e)(f), §2.1's rows, §6, §9.2, §9.3; then the plan's §"Names shared across tasks"). Fold your reading notes into the plan, then commit the plan and this board on `security-s1` as ONE docs commit (S1a's precedent).
2. **Task 9 Step 1 — the pre-flight, the orchestrator's own**, on a `git archive` export of the branch head: cut the spread fixture from the review's worst-case body (the plan's D-2: 4,096 events, `local-spread4096`, four `local@browser` seats, the provenance file with `spread_decision_at`) and COMMIT it before any dispatch; measure every bound the plan states as "measured" and write each reading into the plan's constants and its review record; execute every cell of Tasks 1–7 against scratch implementations lifted from the plan's blocks (never retyped), read every mutant M1–M33 red at its cell and nowhere else; count the corpus files `options_omitted` moves; print the live replay's kept-row list for Task 7's brief. Corrections at the source, as a new docs commit, before Task 1 is dispatched.
3. **Task 1's dispatch** (Opus implementer, Opus reviewer, SDD; the ledger under `docs/temp/security-review-2026-10-08/s1b-sdd/progress.md`; the probe-and-name-concerns paragraph in every brief; the heartbeat for any wait past an hour), then Tasks 2–8 in order, the after review (Task 9 Step 5), the suites with the web lane's rows and `npm run e2e` (Step 4), the close (Step 6: the ledger's seven rows, guidingpath's row, this board, at most one CLAUDE.md sentence), PR #49's description appended (Step 7), the live tier if a window holds (Step 8), the hand-off into S1c (Step 9).
4. Then S1c "the plumbing" (B8, B9, D1, D7, D12, D26, D28, D29's `is_token` sentence, with M-3 and the served `why` chart's pin) on the same branch; the merge at S1's wrap on your word; S2's brainstorm from the ledger after; S3.

**Five configuration changes need no code and are reversible today** (spec §2.2 B4, B6):
the Access cookie's SameSite to Lax, the application session to a day, `wrangler.gate.jsonc` deployed so
the dev door is off outside live tiers, `chmod 600 .dev.vars` with its unused `TURNSTILE_SECRET` line
removed, and the stopped `cloudflare-d` container removed with its token rotated — the orchestrator's,
recorded in the runbook when made.

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
