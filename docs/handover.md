# Handover — the owner's task board

**What this is.** The one screen to open before starting a session: the tracks in flight, the
next concrete step on each, and the rulings they are waiting on. It is a POINTER — every fact
here names the document that carries it, and it never outranks `guidingpath.md` (the authority),
`CLAUDE.md` (the operating facts), or a dated document in `docs/plans/`. Keep it current at every
session close: rewrite the stale lines, do not append history (the decision log is for history).

**Updated:** 2026-09-22 (ninth session, the M6.5b2 EXECUTION, end to end): the session executed all
eleven tasks of `docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md` under your ruling that EVERY
task go subagent-driven with Opus the floor, took the Fable whole-branch review through ONE fix
wave and a scoped re-review, ran the spec's mutation battery (**22 of 22 caught, 0 survived**),
deployed, and wrote the close. **M6.5b2 "the front door, installable and offline" is LIVE** as
`yokohama-web:0.5.0-m6.5b2` at the NAS's new hostname `yokohama-nas.moore.nyc` — the first screen
is a door that asks the server for nothing, the whole release is precached under a digest of its
own bytes so the door and a full game against the bot work offline, the app installs to a home
screen, a game backs up to a file and restores on another device, and a game finished offline is
queued and uploaded when the connection returns. **The record of record is the close
`docs/plans/2026-09-22-web-frontend-m6.5b2-close.md`, which is also M6.5c's entry artifact**; its
§6.3 is the NINE owner items and §7 what the cycle does not claim. **Three of those want your
eye**: the path-scoped Access door is BUILT and proven locally but NOT deployed (you ruled the
hostname-wide policy stays through development), so the zero-login single-player claim is not shown
live; the HEADED walk of the deployed site is owed across TWO releases, because both deploy agents
found Claude-in-Chrome disconnected; and the phone exit is yours — nothing on a phone was measured
by this session. **Track 3 carries it.**

The previous update, 2026-09-21 (seventh session, the M6.5b EXECUTION, end to end): the session executed
all nine tasks of `docs/plans/2026-09-20-web-frontend-m6.5b-plan.md` under your ruling (1 and 9
inline, 2–8 subagent-driven with Opus), took the Fable whole-branch review through ONE fix wave and
its scoped re-review, ran spec §6.6's battery (19 caught, the one survivor closed by a test
assertion, one uncatchable by a ruling you will find in the close's §5), wrote the close
`docs/plans/2026-09-21-web-frontend-m6.5b-close.md`, deployed `0.5.0-m6.5b` (RUNBOOK §3.13),
pushed the branch and opened its PR, and wrote the M6.5b2 entry artifact
`docs/plans/2026-09-21-web-frontend-m6.5b2-handover.md` — **Track 3 carries it; M6.5b2 "installable
and offline" is next, brainstormed with you from that handover.** The previous update, 2026-09-20
(sixth session, at its handover for a clean context): the session
DEPLOYED **M6.5a** (`0.5.0-m6.5a`, RUNBOOK §3.12), brainstormed **M6.5b** with you, wrote its spec
and plan and carried them through FOUR review waves (spec §9–§10), recorded your Free-tier ruling
(R-6.5b.16), updated wrangler, and left the execution to a fresh session — **its entry artifact is
`docs/plans/2026-09-20-web-frontend-m6.5b-execution-handover.md`**, whose §2 records the three
things you RULED at the handover: SDD for Tasks 2–8 with Opus implementers and reviewers and inline
for Tasks 1 and 9, the two spec defaults standing, and offline mode as its own cycle "M6.5b2"
between M6.5b and M6.5c. The previous update the same day was at the DEPLOY of **M6.5a "the seam"** as
`yokohama-web:0.5.0-m6.5a` (`deploy/RUNBOOK.md` §3.12 — both archive sites read off the volume, the
archived record byte-identical to the store's and to the record route's body, the served record
loaded in the terminal) after a fresh whole-branch review of the close at HEAD, with the M6.5b
brainstorm following in the same session; the fifth session, earlier the same day, executed and
closed M6.5a (Track 3). The update before those was 2026-09-19 (fourth session), at the RE-CHARTER of **M6.5 "the hosting move"** as
WebAssembly + Durable Objects: the brainstorm with you, your thirteen rulings, and the umbrella spec
`docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` landed on the stacked branch `web-room-m6.5`
(Track 3 carries it; M6.5a "the seam" is next). The previous update, earlier the same day, was the
CLOSE of **M6.4 "chat"** — the wrap-up the paused session owed is
done: the scoped re-review of the fix wave (42 of 43, 0 open), the mutation battery (**22 of 22
caught, none survived**), the HEADED three-browser live review of the live site in both layouts
(**SHIP WITH FIXES** — 0 Critical / 1 Important / 4 Minor, all five fixed, re-reviewed SHIP and
redeployed as the point release **`yokohama-web:0.5.0-m6.4.1`**) and the Claude-in-Chrome pass under
your own profile, the counts in one sequence at `b16ccdd`, and the close
`docs/plans/2026-09-19-web-frontend-m6.4-close.md` with the M6.5 entry handover
`docs/plans/2026-09-19-web-frontend-m6.5-hosting-handover.md` beside it. **PR #34 is open** against
`web-room-m6.3` — **merge order is yours: #31 → #32 → #33 → #34** (Track 3). The previous update was
2026-09-16 (third session), at the RELEASE of **M6.3 "transport and spectating"** —
executed from the 2026-09-15 plan under subagent-driven development on branch `web-room-m6.3` in
the MAIN checkout, built at `9a4626d` and **SHIPPED AND LIVE as `yokohama-web:0.5.0-m6.3`** since
19:20:51Z: the three steps that were owed to you are done — the R-3.1 tunnel measurement was taken
both ways (**SSE ships**), the release is on the NAS, and the three-browser live review returned
**SHIP** (0 Critical / 0 Important / 3 Minor). Reviewed four ways and recorded in
`docs/plans/2026-09-16-web-frontend-m6.3-close.md`, **amended once at the release and now frozen**;
**PR #33 is open** and merge order is yours (Track 3). The previous update, earlier the
same day, was the close of **M6.2 "the lobby"** — shipped and live on the NAS as
`yokohama-web:0.5.0-m6.2.1`, reviewed four ways, recorded in
`docs/plans/2026-09-16-web-frontend-m6.2-close.md`. Before that, 2026-09-15 (second session),
recorded the M6.2 and M6.3 specs and plans; before that, earlier that day, the close of M6.1
"identity and seats" — shipped, live on the NAS, reviewed
clean against the live site, recorded in `docs/plans/2026-09-15-web-frontend-m6.1-close.md`. Before
that, 2026-09-14, followed the M6 "room" brainstorm producing the five-cycle plan, the M6.1 spec and the
M6.1 implementation plan. Before that, 2026-09-11 (second session) followed the V4 diagnostic games being
played and scribed and the owner-games LEDGER artifact being published for annotation (Track 1); before
that, the web-lane merge (`abe4eec`) with the AI-lane re-entry handover
`docs/plans/2026-09-11-ai-lane-reentry-handover.md`.

## Where things stand

- **The bot:** V4, the sighted evaluator `v4-step9-draft1`, shipped 2026-08-30 as the production
  contestant and released as v0.5.0. The bar ("provably plays to win, two-player") is NOT claimed.
- **The two owner decisions** that close V4 — the D3 depth re-ask and the W6 knob-screening
  GO/WAIT — still HOLD. **The games are now PLAYED AND SCRIBED (three of them, 370 bot events
  reconstructed, zero refusals)**; what is outstanding is YOUR annotation of them in the ledger
  artifact, then the findings document, then the two rulings together (Track 1).
- **The web client** merged to master 2026-09-11 and is live behind Cloudflare Access, now
  **`yokohama-web:0.5.0-m6.5b2`** on the NAS via Portainer (since 2026-09-22 19:08:27Z,
  `deploy/RUNBOOK.md` §3.15) — and since 2026-09-21 at **`yokohama-nas.moore.nyc`**, because
  `yokohama.moore.nyc` is now HELD for the Cloudflare Worker that has not been deployed.
  Nothing about it moves the bot. **M6 "the room" is PLANNED (2026-09-14) and its first FOUR cycles have SHIPPED**:
  M6.1 "identity and seats" (live 2026-09-15, `docs/plans/2026-09-15-web-frontend-m6.1-close.md`),
  M6.2 "the lobby" (live 2026-09-16, `docs/plans/2026-09-16-web-frontend-m6.2-close.md`),
  M6.3 "transport and spectating" (live 2026-09-16,
  `docs/plans/2026-09-16-web-frontend-m6.3-close.md`) and **M6.4 "chat" (live 2026-09-17, the
  point release 2026-09-20, `docs/plans/2026-09-19-web-frontend-m6.4-close.md`)**: two players talk
  during a game, a third reads and answers, the lobby talks, and the first HEADED review of the live
  site found — and the point release fixed — what four headless reviews could not see. **SEVEN pull
  requests now**, a stack, nothing merged and **merge order is yours**: `web-room` PR #31 against
  master, `web-room-m6.2` #32, `web-room-m6.3` #33, `web-room-m6.4` #34, `web-room-m6.5` #35,
  `web-room-m6.5b` #36 against `web-room-m6.5`, and `web-room-m6.5b2` PR-NUMBER-HERE against
  `web-room-m6.5b`. **M6.5 (the hosting move) was
  RE-CHARTERED on 2026-09-19 as WebAssembly + Durable Objects** — its authority is the umbrella spec
  `docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` on branch `web-room-m6.5` (stacked on
  `web-room-m6.4` at `faf1a88`, your call), three sub-cycles a/b/c. **All three of M6.5a "the seam" (2026-09-20), M6.5b
  "single-player in the browser" (2026-09-21) and M6.5b2 "the front door, installable and offline"
  (2026-09-22) are CLOSED AND LIVE; M6.5c "the objects" is next** (Track 3).
- **Licensing:** Apache-2.0 on every crate and on `web/package.json` (checked 2026-09-11);
  `LICENSE` + `NOTICE` at the root; `ASSETS.md` carves out the game's own material.
- **Gates at HEAD:** all per-edit suites green (the counts are in `CLAUDE.md`); the fifteen-gate
  release roster last ran at the V4 merge head, 15/15 at 492.17s on the M5.

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

## Track 3 — The web client: M6.5b2 "the front door, installable and offline" CLOSED AND LIVE 2026-09-22 (`yokohama-web:0.5.0-m6.5b2` from `43d54a5` on `web-room-m6.5b2`, RUNBOOK §3.15, at the NAS's new hostname `yokohama-nas.moore.nyc`; its PR open against `web-room-m6.5b`); **M6.5c "the objects" is next, and this cycle's close is its entry artifact**

**M6.5b2 — DONE (2026-09-22).** The record of record is the close
`docs/plans/2026-09-22-web-frontend-m6.5b2-close.md`: §1 what shipped by task in execution order,
§2 the counts in one sequence, §3 the corpus diff (ONE key on ONE payload), §4 the reviews (ten
Opus task reviews with a fix round each where an Important was found, the Fable whole-branch
review's 0 Critical / 5 Important / 12 Minor and its TWELVE false or stale sentences, the one fix
wave, the scoped re-review, the battery at 22 of 22, the deploy), **§5 every ruling taken on your
behalf with its cost** — including the six commits mis-attributed to the orchestrator's model by the
orchestrator's own dispatch and left unamended, and the interim state the mid-cycle deploy knowingly shipped — §6
the LEAVE list, the **nine OWNER ITEMS** and the two frozen-spec sentences recorded as amendments,
§7 what the cycle does NOT claim, and §8 the eight opening items for M6.5c.

**What wants YOU, shortest first.** (1) **The phone**: install to the home screen, airplane mode,
play, close the browser entirely, reopen, resume, finish, export — then an update and a restore on
a fresh profile; the Android handset is the second reading. Both are recorded, not gated, and both
are M6.5c opening items. (2) **The headed walk of the deployed site**, owed across two releases
now: the door and the lobby's back-link at 390 px, the door's new backup and restore controls
(which no width gate covers), and the five error strips the Refusal sweep touched mid-flow — the
deal, the open table, the create form, the unlock screen and the chat panel. jsdom cannot see a
stylesheet and thirteen screens changed. (3) **Two product calls**: a finished LOCAL game now
uploads itself to the archive at game over with nobody clicking, and on the first discovery of an
archive every finished local game on the device goes up — spec-conformant, and a player can notice
it; and under a single-player build four "Lobby"-labelled controls on local screens lead to a lobby
whose game routes do not exist. (4) **When the public Worker's configuration is decided**
(R-6.5b2.6), it decides whether the card art becomes public.

**What is deliberately NOT done, so you are not surprised by it.** The path-scoped Access
application of spec §3.2 is BUILT and proven locally and is NOT deployed — you ruled the
hostname-wide policy stays throughout development, so no path on the site is public and the plan's
"public paths answer 200 with no session" readings were deferred and recorded as the 302s they are.
`claimed` is still the NAS's identity mode; what changed is that a public build can now refuse it
by name.


**M6.5b — DONE (2026-09-21).** The record of record is the close
`docs/plans/2026-09-21-web-frontend-m6.5b-close.md`: §1 what shipped by task, §2 the counts in
one sequence, §4 the reviews (seven Opus task reviews with one fix round each where an Important
was found, the Fable whole-branch review's 0 Critical / 3 Important / 11 Minor and its ten-entry
vacuity list, the one fix wave `ee552cf`, the battery), **§5 every ruling taken on your behalf
with its cost** — read the Task 3 detour (the self-host Dockerfile, cancelled on "we're not using
Modal" and reinstated on "a user running locally in Docker is a goal") and the Task 8 PARKED ruling
(the cold-pump arm of persistence-by-outcome is unreachable through the shipped client because
every table seats you first; "who starts" on the local form is the owner item) — §6.3 the OWNER
ITEMS (the module's 758 KB gzip and no release profile; the first-screen boot instantiating the
module every visit; `walk.json`'s 419 KB pretty-printed; "who starts"; the archive route handing
an untrusted record to the engine's setup path; the invisible no-Web-Locks fallback; nothing in CI
building the self-host image), and §7 what it does NOT claim. **Your Web Locks probe on the iOS 27
simulator is the R-6.5b.5 measurement** (PASS; §1.4). **M6.5b2 followed and is CLOSED** (the block above); what it inherited is
`docs/plans/2026-09-21-web-frontend-m6.5b2-handover.md`.

**The previous state of this track (2026-09-20), kept for the record:** M6.5a "the seam" CLOSED AND LIVE 2026-09-20 (`yokohama-web:0.5.0-m6.5a` from `8a6b391` on `web-room-m6.5`; PR #35 open); M6.5b "single-player in the browser" was next.

**Pre-execution review (owner-requested, 2026-09-20) — DISPOSED:**
[Free-tier, offline, and save review](plans/2026-09-20-web-frontend-m6.5b-review-free-tier-offline-saves.md)
recorded eight corrections; every one was verified against the code and TAKEN into the spec's
fourth wave and the plan's revision (spec §10 is the disposition table); the Free-tier target is
now R-6.5b.16 (you confirmed it), and the offline/restore work is the level-3 charter in spec §8,
recommended as its own cycle "M6.5b2" between M6.5b and M6.5c. **Execution starts from
`plans/2026-09-20-web-frontend-m6.5b-execution-handover.md`.**

**M6.5b's SPEC and PLAN are WRITTEN on the stacked branch `web-room-m6.5b`; execution is next.** The spec `docs/plans/2026-09-20-web-frontend-m6.5b-single-player-spec.md` (R-6.5b.1–15, at `54a9a1f`) came out of the brainstorm with you and was revised three times — an Opus adversarial read (2C/10I/13M, 41 claims verified true), that reviewer's scoped re-read of the fix wave (one new Critical: the archive route's table check would have refused every real send, since a bot seat's keep lives in the record), and your own lightweight-agent read (the run route's 404 boundary one position too tight against the client's own `to = next + 1`; the lock lifecycle; a stored game that no longer rebuilds; the handover's superseded sentences) — its §9 traces every change. The plan `docs/plans/2026-09-20-web-frontend-m6.5b-plan.md` is nine tasks in dependency order (the run route → the archive route → the wasm crate → the Worker, the SDD candidate → the first screen → the local game screen → the corpus replay → the e2e and the pin → the close), each ending in the spec's own cells. **You confirmed the Free-tier target (R-6.5b.16, amending umbrella R-6.5.12; the decision-log row of 2026-09-20 carries it and M6.5c inherits the consequences), ruled the execution mode (SDD for Tasks 2–8, inline for 1 and 9), kept both spec defaults, and made offline mode its own cycle "M6.5b2". The two defaults, for the record:** `labels` is read from the server first and falls back to the Worker (the alternative wakes the Worker on every page load); the archive's caller directory is a readable transliteration (`jamoore_at_gmail.com`) that can collide, with a hash the one-line alternative. **M6.5a is DONE, DEPLOYED and reviewed.** (1) **The deploy is done:**
`yokohama-web:0.5.0-m6.5a`, built from `8a6b391` (the close's docs commit over `42832f8`), live in
container `579cd1ff…` since **14:04:35Z** with BOTH `Env` entries — `deploy/RUNBOOK.md` §3.12 is the
record. `/api/health` reads `"archive": "dir"`; a throwaway table abandoned after one move left
`/data/archive/rev7/abandoned/<id>.json` + `.table.json` (schema 3, no lock, no roster, the stamp set)
and a throwaway game played to the end through the API left the pair under `over/`, its record
**byte-identical** to the store's copy and to the body `GET …/record` served (one sha256 for all
three), and that served record loaded in the terminal with `why` reconstructing the bot's last move
with no refusal. Both throwaways were deleted afterwards; the volume is back to your one table, which
is a schema-2 document the new binary reads with every new field at its default. (2) **PR #35** is
open against `web-room-m6.4` (https://github.com/jmoore2333/yokohama-assistant/pull/35; CI green on
all three jobs) — the stack is #31 → #32 → #33 → #34 → #35 and merge order is yours. (3) **The
fresh review before the deploy found nothing that blocks it and two things to carry forward**, both
recorded here and in the RUNBOOK rather than changed on the reviewed branch: **a schema rollback
caveat** — `TABLE_SCHEMA` is 3, only `create` stamps it, and every binary before `0.5.0-m6.5a`
REFUSES a schema-3 document (the lobby drops the row, its reads fail) until the newer binary is back
(RUNBOOK §3.12, §5); and **a LEAVE for the M6.5b spec** — `delete` chooses the archive leaf by
STATUS, so a FINISHED game with no stamp (finished before this release, or one whose game-over put
failed) is archived under `abandoned/` at its delete: the leaf lies, the archived table's `turn.over`
tells the truth, and the one-line fix is to choose `Why::Over` when the mirror says over — a spec
sentence for M6.5b, not a hot change. (A third, cosmetic: an already-archived finished game's lobby
row still reads "Abandon and save" while nothing more is saved — beside the close's owner item 4.)
**Next: open M6.5b** from `docs/plans/2026-09-20-web-frontend-m6.5b-handover.md` — its spec is
written at its start (R-6.5.7), and §3 of the handover lists the decisions that spec takes first (the
local identity, the JS clock, the Worker, the two tabs, the bot seed, a local abandon). What shipped,
in one breath: the table layer is out from under axum — the crate builds for `wasm32-unknown-unknown`
without its `server` half (CI checks it; a source-scanning cell ENFORCES one `#[cfg(feature =
"server")]` per item), every route is a typed `pub fn` behind one table and one `dispatch` in
`api.rs` whose 400 sentences are axum's word for word and pinned against axum, the signal/clock/archive
are seams on `Games`, finished and abandoned games are archived once under `rev7/over/` and
`rev7/abandoned/`, a seat holder can download a finished record as the store's exact bytes, and the
client saves it without parsing it. **The record of record is the close
`docs/plans/2026-09-20-web-frontend-m6.5a-close.md`** — §2 the counts in one sequence (web 259 · CLI
851 + 2 · vitest 640 in 38 · e2e 39 in 8 · corpus 83 · the five fenced crates and the CLI untouched ·
`SEMANTICS_REV` 7; re-read at `8a6b391` before the deploy: web 259, clippy, the wasm check, typecheck,
vitest 640 — all green), §4 the reviews (two per-task, one mid-point, one whole-branch, each with a
fix wave; the fifteen-mutation battery at 15 of 15), §5 the rulings with their costs, §6 the LEAVE
list and FIVE owner items (the first, the deploy, now done), §7 what it does NOT claim (nothing RUNS
in wasm; one host). **You ruled mid-cycle** that small tasks execute INLINE with broad review rather
than per-task SDD; the close records it and the memory carries it.

**M6.5 was re-chartered on 2026-09-19** in a brainstorm with you, and the umbrella spec
`docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` is its authority (branch `web-room-m6.5`,
stacked on `web-room-m6.4` at `faf1a88` by your ruling). What changed: the engine, the production bot
and the whole seam the web crate consumes were MEASURED to compile to `wasm32-unknown-unknown`
unchanged (1.30 MB raw / 387 KB gzip with the `Repl` seam; the bot ~1.5 ms per decision native), so
the container, Postgres and the origin-poked notifier are gone — hosted multiplayer runs inside Durable
Objects (one object per table IS the store, the lock and the signal; a house object is the lobby's
projection), single-player runs entirely in the browser as a primary channel, and the axum binary in
Docker stays exactly as it is as the self-host path. Your thirteen rulings are the spec's §1
(R-6.5.1–R-6.5.13): R-P5 amended (the object's SQLite replaces Postgres); the Access allow-list door
(the email roster, the password oracles and moderation stay LEAVE items on it); the R2 archive in the
file store's own layout with **abandon-and-save** (once per game id, never versions; a directory on
the NAS so saves are collected during development); single-player entirely local with a strict-
replay-verified "Send to the archive"; the order a → b → c; **test in the live setup during
development**, not only under `wrangler dev`. **The adversarial read of the spec against HEAD is DONE
and its fix wave landed** (4 HIGH / 11 MEDIUM / 12 MINOR / 10 OPEN, 71 claims verified true; the
report and the cut-surface survey sit in the gitignored `.superpowers/sdd/2026-09-19-web-frontend-m6.5-hosting/`):
the four HIGHs were real — the wasm check line cannot see a `SystemTime` call (the clock gets its own
scanning fence), "82 payloads on all three hosts" contradicted the browser host (the ruling now
reads "every payload a host SERVES"), Worker-level Access does not support WebSockets (the door is a
HOSTNAME-based Access application and `ctx.access` is recorded unavailable), and an open sub-picker
dies at every hibernation (named, with M6.5c to choose the fix) — and the survey found the layering
inverted (the cut runs THROUGH `Games`, §2.1 rewritten). **A second wave landed from the merged
standing review you brought (16 items), each proven or refuted against the mechanism first**: the
placement reservoir's `usize` draw really is width-dependent (reproduced: the same stream draws
206 native / 281 wasm) and really is unreached at 2P (the code's own pin), so it is O-11, owed
only if 3–4P bots come in scope; seventeen worlds / 7,826 moves played natively and under Node's
WASI came out byte-identical (§0.3, support for M6.5b's pin); `replay_strict` skips the digest
when absent (a fourth check); input gates open during a house push or R2 put (revisions, a
pending-deliveries list, delete ordering); the SQLite row cap is 2 MB (~20,000 events of
headroom); the DO CPU limit reads 30 s unqualified; `workers.dev` off. **The M6.5a spec is
WRITTEN** — `docs/plans/2026-09-19-web-frontend-m6.5a-seam-spec.md`, sixteen rulings R-6.5a.1–16:
the cut through `Games`, `ApiError` on the `http` crate, ONE route table in a new `api.rs` with
axum reduced to an adapter (the corpus and the 74 route cells then prove the dispatcher before a
second host exists), the rejection sentences reproduced and pinned against axum's, the `server`
feature over optional axum/tokio, the clock seam with its scanning fence, the `Signal` trait, the
archive trait with a directory implementation and one layout function, `archived_at` at schema 3,
the two archive sites with their two failure rules, the record route, "Save game file", and the
corpus rules — plus five pre-plan flags from a second review, each verified (HEAD as GET, the
per-mode 401 carried, the trait's four `&str` methods, `eprintln!` probed on the target). **The
M6.5a PLAN is WRITTEN** — `docs/plans/2026-09-19-web-frontend-m6.5a-plan.md`, eleven tasks in
dependency order (signal trait → clock → chat on `Games` → the `server` feature + scanning fence →
`api.rs` + the four public routes → every route through the adapter → the archive module → the two
archive sites → the record route → the client → the deploy note), six plan-level rulings, the close
checklist. **Next: execution under R-P11 in a FRESH session, subagent-driven, from
`docs/plans/2026-09-19-web-frontend-m6.5a-execution-handover.md`** (its §4 is the resume point:
the SDD workspace, Task 1 to an Opus implementer, the close checklist), then the deploy as
`0.5.0-m6.5a` and the close; the M6.5b and M6.5c specs are written at their cycles' starts from
M6.5a's close. Nothing of the plan is executed yet — every commit on the branch is a document. The spec's §6 is what it does
NOT claim and §6.1 the ten open items; nothing hosted is verified yet.

**M6.4 "chat" is CLOSED and LIVE** as `yokohama-web:0.5.0-m6.4.1` (container `a62b17ad…` since
00:38:01Z on 2026-09-20; `deploy/RUNBOOK.md` §3.10 is the `0.5.0-m6.4` record and §3.11 the point
release) — executed 2026-09-17 under subagent-driven development from
`docs/plans/2026-09-16-web-frontend-m6.4-plan.md` and wrapped up 2026-09-19/20: the scoped re-review
of the fix wave (42 of 43, 0 open), the mutation battery (**22 of 22 caught, none survived**), the
HEADED three-browser live review you put back in that morning (**SHIP WITH FIXES** — 0 Critical /
1 Important / 4 Minor: a phone-width chat-row squeeze, the who-strip's wrapping, one delta GET per
own post, the setup heading rules leaking into the panel, and a favicon 404 only a HEADED browser
logs — all five fixed in one Opus wave, re-reviewed SHIP 8/8, re-counted at `b16ccdd` and shipped
under your standing authorization as `0.5.0-m6.4.1`, then re-checked headed: **all five FIXED, zero
console errors**), the Claude-in-Chrome pass under your own profile (the panel, composer, prefilled
name and chips fit at 1440 and at Chrome's 500 px floor; two inherited cosmetic wobbles LEAVE), and
the counts in one sequence (web 230 · CLI 851 + 2 · vitest 634 in 37 · e2e 39 · corpus 82 · the five
fenced crates and the CLI untouched · `SEMANTICS_REV` 7). **The record of record is the close
`docs/plans/2026-09-19-web-frontend-m6.4-close.md`** — §4.5 the headed review and its re-check, §5
the rulings with their costs, §6.3 SEVEN owner items (the vs-bot gate livelock, the e2e `CAP`, the
disk, the house chat's undeletable lines — five sit there now — the lobby's three reads per house
message, the per-process chat state a multi-replica host breaks, and WARP not staying on by itself),
§7 what it does NOT claim. **M6.5's entry artifact was
`docs/plans/2026-09-19-web-frontend-m6.5-hosting-handover.md`**, now superseded by the umbrella spec
above (its §0.5 says exactly where); the handover's §5 questions on the roster and the password
oracles are LEAVE items on the Access door by R-6.5.3. **PR #34 is open** and merge order is yours —
#31 → #32 → #33 → #34; M6.5 is stacked on top by your ruling, so a merge of the stack at any point
only retargets its PR. The SDD workspace (gitignored) keeps the 117 review photographs.


**M6.3 "transport and spectating" is SHIPPED AND LIVE** as `yokohama-web:0.5.0-m6.3` (container
`f0c371c7…` since 19:20:51Z on 2026-09-16) — planned 2026-09-15, executed
2026-09-16 under subagent-driven development (nine Opus implementer dispatches, nine Opus per-task
reviews, six fix rounds, one Fable whole-branch review and its single three-commit fix wave, one
Opus mutation battery), landing twenty-two execution commits on `web-room-m6.3` at **`9a4626d`**.
Nothing polls: the browser holds ONE server-sent-event connection per visible tab and the
opponent's move arrives on it and walks. A signed-in stranger with a link — or a row in the lobby's
new *Games in progress* — can WATCH a playing game after presenting its password once: every hand a
count, no answer bar, no undo, no suggest, and nothing they can send. The five fenced crates stayed
byte-identical, `SEMANTICS_REV` is 7, and the CLI moved by ONE behaviour-preserving widening in
`repl.rs` (+39/−7). The mutation battery: **15/15 caught, none survived**.

**The record of record is the close, `docs/plans/2026-09-16-web-frontend-m6.3-close.md`** — §1 what
shipped, §2 the counts with **§2.1 the tunnel measurement, now taken both ways**, §3 the corpus
diff, §4 the reviews with **§4.5 the live review against the deployed site**, §5 the twenty-one
execution rulings with their costs, §6 the LEAVE list and four owner items, §7 what is NOT claimed,
§8 the handover. **It was amended once at the release and is now FROZEN.** The deployment itself is
`deploy/RUNBOOK.md` §3.9.

### The three steps are DONE — 2026-09-16, on your in-session authorization

- [x] **1. The tunnel measurement (plan Task 3) — SSE SHIPS.** The rc step was folded into the
      release (you authorized it and nobody was playing), so the reading was taken against the
      deployed stream. **Reading A**, with the 25 s heartbeat: a watcher held `…/stream` for ten
      minutes, 23 heartbeat comments at 25 s, a move made at minute nine arrived in under a second,
      and the connection was still open at minute ten — ended by `curl`'s own timer, not the edge.
      **Reading B**, on a throwaway `-rc2` with the heartbeat disabled and removed afterwards: the
      idle stream was reset at **125 s**. So `SIGNAL_KIND` stays `"sse"` and the held-request arm
      stays dark. Close §2.1.
- [x] **2. The release (plan Task 10).** `yokohama-web:0.5.0-m6.3`, built at `74a0e80`, live from
      19:04:04Z and — after the rc2 reading swapped twice — from **19:20:51Z** in container
      `f0c371c7…`, same address, no cloudflared restart, the volume untouched. Verified through the
      tunnel: `/api/health` carries `"signal":"sse"`, a bare `/api/games` is 401, all three scopes
      answer, the bundle is this build's. Recorded in `deploy/RUNBOOK.md` §3.9, and the Yokohama
      ingress comment in `deploy/cloudflared-config.yml` now carries the heartbeat sentence.
- [x] **3. The live review (plan Task 11 step 2) — SHIP.** Three browsers, three people, both
      layouts: 0 Critical, 0 Important, **3 Minor**; 25 claims tried and true; zero console errors
      on all three screens. **The exit criterion is shown on the NAS**: the watcher asked
      `/events` **zero** times and held **one** stream through two full turns, a 444 s game of
      1,471 events and a whole game against the bot. No point release was needed. Close §4.5.

**The three Minors, all LEAVE.** A watcher on a FINISHED game is not told when you abandon it (the
stream closes at game over, so the sentence has nobody to reach — one of two one-line fixes closes
it, and the same fix covers the third); every arrival by link or reload draws the lobby for one
round trip before the table lands; and a player whose turn it is holds no stream, so an abandon
under them is learned on their next post (suspected, not reproduced). None is on the wire. They
are candidates for the start of M6.4 if you want them closed.

### Also for you

- **Three pull requests, and the merge order is the one thing M6.3 now waits on.** `web-room` →
  master is PR #31 (M6.1); `web-room-m6.2` → `web-room` is M6.2's; **PR #33 is
  `web-room-m6.3` → `web-room-m6.2`** (https://github.com/jmoore2333/yokohama-assistant/pull/33).
  Nothing is merged; the live site is served from the branch's build, not from master.
- **The email roster reaches any signed-in caller in one page.** The lobby's new `playing` scope and
  `GET …/{id}/table` at playing and over hand every such table's full seat roster — emails — to
  anyone signed in. Behind Cloudflare Access that is your own allow-list; on the hosted path (M6.5)
  it is a decision. The per-seat display name M6.2 added exists for exactly this if you would rather
  the listing showed names alone. Close §6.3 item 1.
- **The whole-game gate's vs-BOT arm livelocks on some seeds, and it is now diagnosed** — the arm's
  driving policy places nothing and leans on the bot to end the game, so a bad seed runs to the
  20,000-input cap (1,813 s in the counts sequence; the same binary re-run alone passed in 26 s).
  Candidate fix: the placement probe the two-people arm already has. Close §6.3 item 2.
- **This machine's disk sits at ~9.4 GiB free and a FAILED Playwright run writes a multi-GB trace**
  (`web/test-results/`, 2.2 GB once), which is why both failed e2e runs this cycle hit ENOSPC on the
  artefact that would have explained them. Worth clearing before the next e2e run. Close §6.3 item 4.
- **M6.2's ruling #13** — the delete control on the lobby's Your-games rows — you approved
  mid-cycle ("Ruling 13 is ok") and it needs nothing further.

**M6.4 "chat" — BRAINSTORMED, SPECIFIED AND PLANNED on 2026-09-16, on branch `web-room-m6.4`
(off `2194b57`, PR #33's tip).** The brainstorm settled sixteen rulings in your words, numbered
`R-6.4.1`–`R-6.4.16` (spec §1): seats AND watchers post under the one signed-in identity; the label
is the lobby's rule (name, else the address) with a "watching" marker; "Your name" at the door and
beside the address, stored per browser; the stream HELD at every visible status; chat on the game,
dealing, open-table and lobby screens with `YOKOHAMA_WEB_LOBBY_CHAT=on|off`; retention as launch
variables (`YOKOHAMA_WEB_CHAT_KEEP` 500|all, `YOKOHAMA_WEB_CHAT_TEXT` 500) reported on health; a
count on the signal and a fetch for the text; the panel LAST in the side column and FOLDED
everywhere; a tagged author with a `bot` arm no route produces, so an LLM-driven bot can talk later
(fed its own VIEW, never the record); the live review HEADED. **The spec is
`docs/plans/2026-09-16-web-frontend-m6.4-chat-spec.md`**, corrected by an Opus adversarial read
against HEAD (8 HIGH, 10 MEDIUM, 9 MINOR, 10 OPEN, all folded; 80 claims verified true; its report
is under the gitignored `.superpowers/sdd/2026-09-16-web-frontend-m6.4-spec/`). **The plan is
`docs/plans/2026-09-16-web-frontend-m6.4-plan.md`**: ten tasks, the store's third kind first and the
headed live review last, seven plan-level rulings. **You ruled (2026-09-16, at the plan's handoff): execute in a FRESH session.** That session
starts at the plan's Task 0 (re-read every citation, re-count, open the ledger at
`.superpowers/sdd/2026-09-16-web-frontend-m6.4-plan/progress.md`) under R-P11 — Opus implementers
and per-task reviewers, Fable orchestrating, every agent's report to a file, never a follow-up
without reading HEAD — through Task 8; then it stops for your authorization at the release (Task 9,
`0.5.0-m6.4`) and runs the HEADED three-browser review (Task 10), which needs nothing changed on the
NAS (WARP stays on; the claimed door makes three contexts three people). The whole-game gate stays
out of CI. Fresh-session read order: this board → the spec's §12 → the plan.

**Live operations.** The deployment record and recipe are `deploy/RUNBOOK.md` (§3.5 the recipe,
§3.9 the M6.3 deployment with the R-3.1 readings, §5 rollback — and note that rolling back behind
M6.3 takes BOTH halves, because the M6.3 client streams and the M6.2 server has no `/stream`). Deploys run through the Portainer MCP against environment 2
(`http://192.168.1.173:9000`); after every deploy, look at BOTH layouts on screen. Two operating
facts worth having in hand: the workstation's LAN address changes between sessions and must be read
from `ifconfig` for the context-tarball fetch, and a Cloudflare "Just a moment…" challenge page from
`curl` means WARP is off, not a failed deploy.

**Pulling a web game as a save file** — verified 2026-09-11:

- The server persists every game as the SAME `GameRecord` JSON the CLI's `save` writes (one
  function, `Session::to_record_json`), at `/data/games/<id>.json` inside the container
  (`YOKOHAMA_WEB_DIR=/data/games`, the named volume `yokohama-web-data`). No API route serves the
  raw record, and no route ever will carry the setup seed — so the pull is a Docker archive read
  through Portainer:

  ```bash
  # the token the Portainer MCP uses (never paste it anywhere): env, the portainer-mcp
  # checkout's .dev.vars, or Keychain — see ~/.local/bin/portainer-mcp-write
  curl -s -H "X-API-Key: $PORTAINER_MCP_TOKEN" \
    "http://192.168.1.173:9000/api/endpoints/2/docker/containers/yokohama-web/archive?path=/data/games/<id>.json" \
    | tar -xO > docs/temp/owner-games/web-<yyyy-mm-dd>-<id>.json
  ```

  Game ids come from `curl https://yokohama.moore.nyc/api/games` (WARP passes Access from the
  shell) — **which now needs an identity header**: add
  `-H "X-Yokohama-Claimed-User: <your email>"` and the list is the tables that email sits at.
  Since M6.1 a browser-created game carries the name you typed rather than a random `sim-` id, but
  `<id>.table.json` beside the record is a second file and the pull above takes only the record.
  The pulled file resumes in the CLI (`cargo run -p yokohama-cli -- <file>`) and `why` works on
  every bot move — checked on `sim-016f847457c957a6` (231 events; you beat the bot 100–56 in that
  one).
- **Naming:** name the PULLED copy `web-<date>-<id>.json` and let the findings document carry the
  purpose; CLI games keep `gameN-v4.json`. Everything under `docs/temp/owner-games/` is gitignored
  on purpose — a record carries the setup seed, which is the whole hidden set.

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
10. **M6.2 ruling #13** — the creator's Abandon on the lobby's Your-games rows: keep or strike
    (the close §5; one control either way).

## Entry artifacts by lane

- AI lane: `docs/plans/2026-09-11-ai-lane-reentry-handover.md`, then the V4 close
  `docs/plans/2026-08-30-phase3.5c-v4-close.md` §3.
- Web lane: **`docs/plans/2026-09-22-web-frontend-m6.5b2-close.md`** (the newest close and M6.5c's
  entry artifact — §6.3 the owner items, §7 what is not claimed, §8 the eight opening items), then
  `docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` (M6.5's authority — the umbrella;
  §0 what is true at HEAD, §1 the rulings, §3 the three sub-cycles), then the handover it supersedes
  `docs/plans/2026-09-19-web-frontend-m6.5-hosting-handover.md` (§2 the seams as M6.4 left them, §6 the
  process), then the M6.4 close `docs/plans/2026-09-19-web-frontend-m6.4-close.md` (the record of what
  shipped; the M6.3, M6.2 and M6.1 closes are the three before it), then the room plan
  `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §0 and §2; what M6.4 inherited is
  `docs/plans/2026-09-16-web-frontend-m6.4-chat-handover.md` and what M6 inherited is
  `docs/plans/2026-09-06-web-frontend-m6-handover.md`; operations `deploy/RUNBOOK.md`.
- Bridge (BGA capture → import): `docs/bridge-setup.md`; ledger
  `docs/plans/2026-08-08-phase3.5b-execution-ledger.md`.
- Everything: `CLAUDE.md`, then `guidingpath.md`'s Current state and the decision log's tail.
