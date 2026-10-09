# Handover — the owner's task board

**What this is.** The one screen to open before starting a session: the tracks in flight, the
next concrete step on each, and the rulings they are waiting on. It is a POINTER — every fact
here names the document that carries it, and it never outranks `guidingpath.md` (the authority),
`CLAUDE.md` (the operating facts), or a dated document in `docs/plans/`. Keep it current at every
session close: rewrite the stale lines, do not append history (the decision log is for history).

**Updated:** 2026-10-09 (second session) — S1a's plan written on the branch `security-s1` and reviewed by you with three rulings (Track 4); the A2/B12 figures measured. 2026-10-09 — the security remediation's core PR MERGED (PR #48, `9ab8c430`; guidingpath's 2026-10-09 row); the remainder of S1, then S2 and S3, stack on ONE new branch (your ruling). 2026-10-08 (second session) — the core PR plan's "before" review done and PR #47 open (Track 4). 2026-10-08 — the security review of the web lane and its remediation specification (Track 4; guidingpath's 2026-10-08 row). 2026-10-04 — the instruction files split by role (guidingpath's 2026-10-04 row): a session starts here, and CLAUDE.md holds standing rules only. The web lane is complete (`v0.6.0`, 2026-10-01) and the V5 ship's one CLI line has landed (the hotfix row). **NEXT: S1a's "before" review and Task 1's dispatch (Track 4), in a FRESH session on `security-s1`; the computer player (Track 1) resumes after the security cycles.**

## Gates at HEAD

The latest readings: the core PR plan's §"Review record", its 2026-10-09 entry
([`plans/2026-10-08-security-s1-core-replay-plan.md`](plans/2026-10-08-security-s1-core-replay-plan.md)) — every per-edit suite at
`de8ad8ed` (core 1,587 passed / 29 ignored; the CLI 855 / 2; the web lane's rows unchanged from cycle F's close §2), and the fifteen
release gates as one sequence, **451 s** on the Mac (Darwin 27.0.0 arm64 — the M5 MacBook Air; cycle F read 534 s; the target is
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

## Track 4 — Security remediation: the core PR landed; S1a's plan is written on `security-s1`; S1b, S1c, S2 and S3 stack on it (S4 is the public set, Track 2)

**State (2026-10-09):** the core PR is MERGED — **PR #48, master `9ab8c430`** (guidingpath's 2026-10-09 row; the plan
`plans/2026-10-08-security-s1-core-replay-plan.md`, its §"Review record" the record of record, R-1–R-18): `is_listed_move`,
`MoveCheck` and `replay_strict_with(.., Predicate)` with the 7-cell proof binary `crates/yokohama-core/tests/replay_check_equivalence.rs`;
`SEMANTICS_REV` 7, the oracle and the checked boundary untouched; the readings in "Gates at HEAD" above. **The measurement S1 sizes
from:** the review's 4P worst-case record (23,606 events, 1 MiB) replays in **54 ms** under the predicate check against **28.5 s** under
`replay_strict`, the same digest. The entry documents: the spec `plans/2026-10-08-security-remediation-spec.md` (§1 the ten rulings,
§2.1 S1's rows, §3, §5 the acceptance, §9 the corrections — §9.2 your branching ruling); the ledger `security-ledger.md` (A2 `open`
with its core half landed; every other row `open`); the gitignored report and the four worst-case bodies under
`docs/temp/security-review-2026-10-08/` (`report.md`, `reviewer-reports/`, `worst-case/`, `core-pr-sdd/` — the core PR's briefs,
reports and probe sources); read them there, never commit them.

**Your ruling at the merge (2026-10-09):** the core set had unique treatment — its own PR, reviewed before and after — because it
modified the protected core files. The remainder does not touch them, so **S1 proper, then S2, then S3 run STACKED on ONE new branch
cut from this master and merge back to master from there, not per cycle**; the per-cycle whole-branch reviews stand, and each cycle's
plan is written against the stack's head.

**S1 is THREE cycles (the M6.5c2 lesson), each planned against the stack's head:** S1a "the leaks" (A1, B1, B2,
B3, B7, D5, D10, D11), S1b "the cost bounds" (A2, A3, B12, D6, D25, D27, D29's `archive_post` sentence; it carries
the core PR's two notes — the CLAUDE.md parenthetical to drop, the "`replay_strict` itself is unedited" snapshot),
S1c "the plumbing" (B8, B9, D1, D7, D12, D26, D28, D29's `is_token` sentence). **S1a's plan is
[`plans/2026-10-09-security-s1a-leaks-plan.md`](plans/2026-10-09-security-s1a-leaks-plan.md)** on the branch
`security-s1`, cut from master `e9824ec9` (the code of `9ab8c430` plus the core PR's close documents) — seven
tasks in the core PR's shape, its §"Spec corrections" the seven forward corrections Task 6 records as the spec's
§9.3. **Your rulings on it (2026-10-09):** reviewers are Opus for every task (the core PR's Fable reviews were the
protected core files' exception; Sonnet/Opus and Opus/Opus are this sequence's tiers); B1 is served WITHHELD while
the bot's starting order is hidden from the requester — you asked whether that is the opening round only: it is
not, the kept card stays in the bot's hand, masked from every other seat, until the bot fulfils it, mid-game in
every recorded game, and the review named it from the first bot move's feature values alone; B3 is the named
`SeedPolicy::Diagnostic` opt-in for the fixtures and the Worker's pool, the production rule `SoloOnly`; the corpus
regenerates once per wire-changing cycle, each diff read (a forward correction to spec §6). **The A2/B12 figures
are measured** (the plan's §"The S1 figures"; raw logs `docs/temp/security-review-2026-10-08/s1a-measure/`):
people's and the bot's games are 250–600 events under 60 KB pretty-printed, random play 15k–19k events and
1.5–1.9 MB; the recommended knobs (a 4,096-event cap, 512 KiB body cap, 256 records / 128 MiB per caller, a
2-permit semaphore with a 2 s bounded wait) are yours to confirm at S1b's plan (R-SEC8(f)).

**Next session, in a FRESH context, in order:**
1. `git checkout security-s1`; read the plan whole, its §"Names shared across tasks" first.
2. **Task 7 Step 1, the "before" review:** execute Tasks 1–5's cells and the TS cells against scratch
   implementations of the plan's own blocks in a `git archive` export of `e9824ec9`, every mutant M1–M9 red at
   its cell; three cells are written expecting to teach something (Task 2's drive words, Task 5(b)'s
   reachability, Task 4(e)'s seat control); corrections at the SOURCE in the plan, as a new commit on the branch,
   recorded in the plan's §"Review record".
3. Dispatch Task 1 (Opus implementer, Opus reviewer on an export), the SDD ledger under
   `docs/temp/security-review-2026-10-08/s1a-sdd/` (full reports to files, three-line replies, the
   probe-and-name-concerns paragraph, no `rm -r`, no Monitor); then Tasks 2–6.
4. The "after" whole-branch review, the per-edit suites with the web lane's rows and `npm run e2e` (the client
   moved; no fifteen-gate roster — core and the version are unmoved), the close: the ledger's eight rows,
   guidingpath's row and current state, this board.
5. S1b's plan against the stack's head (transcribing the figures, with your knob choice), then S1c; S2's
   brainstorm from the ledger after S1 lands; S3; the merge to master after S3, on your word.

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
