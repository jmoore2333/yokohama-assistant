# Handover — the owner's task board

**What this is.** The one screen to open before starting a session: the tracks in flight, the
next concrete step on each, and the rulings they are waiting on. It is a POINTER — every fact
here names the document that carries it, and it never outranks `guidingpath.md` (the authority),
`CLAUDE.md` (the operating facts), or a dated document in `docs/plans/`. Keep it current at every
session close: rewrite the stale lines, do not append history (the decision log is for history).

**Updated:** 2026-09-30, M6.5c2 cycle E "the finish" CLOSED on `web-room-m6.5c2e` (the code head `c39da5a1`; the final reviewed head `3107eb8b`; the record [close](plans/2026-09-30-web-frontend-m6.5c2e-close.md), its §7 what the cycle does NOT claim). Both hosts run E's release; the door was vetted in two states and reverted; PR #43 is open against `web-room-m6.5c2d`. **By your word at the close (2026-09-30), the stack is NOT merged now: cycle F "the clean finish" follows, from [its handover](plans/2026-09-30-web-frontend-m6.5c2f-handover.md) — the door's update path unmetered, the self-host's table TTL sweep, Turnstile as a brainstorm item; the merge #31 → #43 → F's PR and the `v0.6.0` tag are F's last step, on your word again** (Track 3). After them the project refocuses on the computer player (Track 1).

**History, by record.** This board keeps no running history; each session's account is its record of record:
- 2026-09-30 — cycle E "the finish" closed ([close](plans/2026-09-30-web-frontend-m6.5c2e-close.md), the last row of `web-lane.md` §11; the plan [`2026-09-29-web-frontend-m6.5c2e-plan.md`](plans/2026-09-29-web-frontend-m6.5c2e-plan.md)).
- 2026-09-29 — cycle D "the code" closed and live ([close](plans/2026-09-29-web-frontend-m6.5c2d-close.md)); cycle E planned (spec §10.11 in [the spec](plans/2026-09-26-web-frontend-m6.5c2-spec.md), [the plan](plans/2026-09-29-web-frontend-m6.5c2e-plan.md)).
- 2026-09-28 — cycle C "the hosts and the close" closed and live ([close](plans/2026-09-28-web-frontend-m6.5c2c-close.md)); cycle D planned ([plan](plans/2026-09-28-web-frontend-m6.5c2d-plan.md)); cycle B "the screens" closed ([C's handover](plans/2026-09-28-web-frontend-m6.5c2c-handover.md) §1 restates what it shipped).
- 2026-09-27 — cycle A "the layer" closed ([B's handover](plans/2026-09-27-web-frontend-m6.5c2b-handover.md) restates what it shipped).
- 2026-09-26 — M6.5c "the objects" closed ([close](plans/2026-09-26-web-frontend-m6.5c-close.md)); M6.5c2 specified ([spec](plans/2026-09-26-web-frontend-m6.5c2-spec.md)).
- 2026-09-23 — M6.5c specified and planned ([spec](plans/2026-09-22-web-frontend-m6.5c-objects-spec.md), [plan](plans/2026-09-23-web-frontend-m6.5c-plan.md)).
- 2026-09-22 — M6.5b2 closed ([close](plans/2026-09-22-web-frontend-m6.5b2-close.md)), and the PR #37 fix wave was live as `0.5.0-m6.5b2.1` ([record](plans/2026-09-22-web-frontend-m6.5b2-pr37-fixes.md) §5).
- 2026-09-21 — M6.5b closed ([close](plans/2026-09-21-web-frontend-m6.5b-close.md)).
- 2026-09-20 — M6.5a closed and live ([close](plans/2026-09-20-web-frontend-m6.5a-close.md)); M6.5b specified ([spec](plans/2026-09-20-web-frontend-m6.5b-single-player-spec.md)).
- 2026-09-19 — M6.4 closed ([close](plans/2026-09-19-web-frontend-m6.4-close.md)); M6.5 re-chartered ([umbrella](plans/2026-09-19-web-frontend-m6.5-hosting-spec.md)).
- 2026-09-16 — M6.2 and M6.3 closed ([M6.2](plans/2026-09-16-web-frontend-m6.2-close.md), [M6.3](plans/2026-09-16-web-frontend-m6.3-close.md)).
- 2026-09-15 — M6.1 closed ([close](plans/2026-09-15-web-frontend-m6.1-close.md)).
- 2026-09-14 — the M6 room plan ([plan](plans/2026-09-14-web-frontend-m6-room-plan.md)).
- 2026-09-11 — the web lane merged (`abe4eec`), and the AI lane's [re-entry handover](plans/2026-09-11-ai-lane-reentry-handover.md); the V4 diagnostic games scribed (Track 1).

## Where things stand

- **The bot:** V4, the sighted evaluator `v4-step9-draft1`, shipped 2026-08-30 as the production
  contestant and released as v0.5.0. The bar ("provably plays to win, two-player") is NOT claimed.
- **The two owner decisions** that close V4 — the D3 depth re-ask and the W6 knob-screening
  GO/WAIT — still HOLD. **The games are now PLAYED AND SCRIBED (three of them, 370 bot events
  reconstructed, zero refusals)**; what is outstanding is YOUR annotation of them in the ledger
  artifact, then the findings document, then the two rulings together (Track 1).
- **The web client** is live on two hosts behind one Cloudflare Access application at E's release `0.6.0`: the Worker at `yokohama.moore.nyc` and the NAS at `yokohama-nas.moore.nyc`. Single player runs in the browser with no server. The lane is finished at cycle E and its record is [`web-lane.md`](web-lane.md) §11; the merge of its stack of pull requests waits on your word (Track 3).
- **Licensing:** Apache-2.0 on every crate and on `web/package.json` (checked 2026-09-11);
  `LICENSE` + `NOTICE` at the root; `ASSETS.md` carves out the game's own material.
- **Gates at HEAD:** all per-edit suites green (the counts are in `CLAUDE.md`, and the web lane's in [`web-lane.md`](web-lane.md) §2); the fifteen gates last ran at E's close, `2881 s` on the Mac beside D's 446 s.

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

**You decide:** D3 · W6 · OD-6 (restate the ≤420s roster target — the M5 read 492.17s) · OD-8's
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

## Track 3 — The web client: M6.5c2 cycle E "the finish", the lane's last cycle

**State:** E's code is on `web-room-m6.5c2e` (E's PR, to be opened at the close), on top of a stack of pull requests, #31 against master through #42, none merged. Both hosts run E's release `0.6.0` behind the hostname-wide Access application: the Worker at `yokohama.moore.nyc` (`worker/RUNBOOK.md` §19) and the NAS at `yokohama-nas.moore.nyc` (`deploy/RUNBOOK.md` §3.19). The public door was vetted in two states on 2026-09-30 and reverted the same hour. What the lane is and how it runs (the hosts, the suites, the testing strategy, what a save file says, the release-version rule, the public door's preconditions, how to pull a game, §10) is [`web-lane.md`](web-lane.md); its records are §11.

**What wants YOU, shortest first.**
1. **The merge and the tag**: the stack #31 → E's PR in order with merge commits, and the `v0.6.0` tag on master, on your word (R-6.5c2.37). CI stays disabled on GitHub through the merge.
2. **The handset's optional ten minutes** (R-6.5c2.33): your own phone against either host, to confirm what the simulators read — the wake after the Home screen, the storage, anything that blocks ([`web-lane.md`](web-lane.md) §3's phones row; the readings `worker/RUNBOOK.md` §19's phones bullet). The same bullet records three owner's-eye layout readings, not fixed: the game header's share of the iPhone's portrait height, the iPad's tiles, the chat input under Android's keyboard.
3. **The blurb's words**, if you want them changed: the placeholder text E shipped is quoted in [`web-lane.md`](web-lane.md) §14.

After the merge the project refocuses on the computer player (Track 1).

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
10. **M6.2 ruling #13** — the creator's Abandon on the lobby's Your-games rows: DONE, approved at M6.3; nothing outstanding.
11. **The refinish rule** — RULED (R-6.5c2.11) and LIVE since cycle C (2026-09-28): a same-game conflict stamps, every other conflict is final.
12. **The `test/` prefix's lifecycle** — RULED (R-6.5c2.12) and SET at cycle C's close (2026-09-28): the R2 rule `expire-test-after-30-days` on `test/`.

## Entry artifacts by lane

- **F's brainstorm, then its spec and plan** — the six items of [the F handover](plans/2026-09-30-web-frontend-m6.5c2f-handover.md) §7 (Turnstile's place and cost; the wording path; the NAS marker; the sweep's first run). The merge and the tag moved to F's end.
- AI lane: `docs/plans/2026-09-11-ai-lane-reentry-handover.md`, then the V4 close
  `docs/plans/2026-08-30-phase3.5c-v4-close.md` §3.
- Web lane: [`web-lane.md`](web-lane.md) (the lane's operating record; §11 its records of record), then cycle E's plan [`plans/2026-09-29-web-frontend-m6.5c2e-plan.md`](plans/2026-09-29-web-frontend-m6.5c2e-plan.md) and E's close (the last row of `web-lane.md` §11, written at the close); operations `deploy/RUNBOOK.md` (§0–§3 and §5 the NAS) and `worker/RUNBOOK.md` (the Worker; `deploy/RUNBOOK.md` §6 points there).
- Bridge (BGA capture → import): `docs/bridge-setup.md`; ledger
  `docs/plans/2026-08-08-phase3.5b-execution-ledger.md`.
- Everything: `CLAUDE.md`, then `guidingpath.md`'s Current state and the decision log's tail.
