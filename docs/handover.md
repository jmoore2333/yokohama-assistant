# Handover — the owner's task board

**What this is.** The one screen to open before starting a session: the tracks in flight, the
next concrete step on each, and the rulings they are waiting on. It is a POINTER — every fact
here names the document that carries it, and it never outranks `guidingpath.md` (the authority),
`CLAUDE.md` (the operating facts), or a dated document in `docs/plans/`. Keep it current at every
session close: rewrite the stale lines, do not append history (the decision log is for history).

**Updated:** 2026-09-15, on branch `web-room` (worktree `.claude/worktrees/web-room`), at the close
of M6.1 "identity and seats" — shipped, live on the NAS, reviewed clean against the live site, and
recorded in `docs/plans/2026-09-15-web-frontend-m6.1-close.md` (Track 3). The previous update,
2026-09-14, followed the M6 "room" brainstorm producing the five-cycle plan, the M6.1 spec and the
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
- **The web client** merged to master 2026-09-11 and is live at `yokohama.moore.nyc` behind
  Cloudflare Access, now `yokohama-web:0.5.0-m6.1` on the NAS via Portainer. Nothing about it moves
  the bot. **M6 "the room" is PLANNED (2026-09-14) on branch `web-room` and its FIRST cycle has
  SHIPPED**: M6.1 "identity and seats" went live 2026-09-15, closed in
  `docs/plans/2026-09-15-web-frontend-m6.1-close.md`. **M6.2, the lobby, is the next step**
  (Track 3), from the room plan `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §2. The branch
  `web-room` is not yet merged to master.
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

## Track 3 — The web client: M6.1 shipped, M6.2 (the lobby) next (web lane)

**M6.1 "identity and seats" is DONE and LIVE** — planned 2026-09-14, shipped 2026-09-15 as
`yokohama-web:0.5.0-m6.1` on the NAS with `YOKOHAMA_WEB_IDENTITY=claimed`. You sign in, you hold
named seats at named tables, each seat is dealt into its own browser, and every payload the server
builds is built for the caller. The five fenced crates stayed byte-identical, `SEMANTICS_REV` is 7,
no bot behaviour moved, and the CLI moved by 24 behaviour-preserving lines. Reviewed clean against
the live site from two browsers as two people (**SHIP AS IS**, 3 cosmetic minors) and by the spec's
own eight-mutation battery (**8/8 caught**).

**The record of record is the close, `docs/plans/2026-09-15-web-frontend-m6.1-close.md`** — §5 the
thirty rulings with their costs, §6 the LEAVE list (inherited and new), §7 what is not
claimed, §8 the handover into M6.2. It is also M6.2's entry artifact.

**Two things owed to you on the NAS:**

- Delete the review's game, id **`m6.1-live-review`** — the only game the live review created.
- Nothing else: the named volume rode the swap untouched, and records from before M6 stay unlisted
  and unserved by design (R-P7).

**Next step — M6.2, the lobby.** Charter: `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §2.
Open tables and join links, taking a seat, start when full, a display name beside the email, "my
games" with whose turn it is, abandon by the creator — the front door proper, so the first screen a
person sees is the lobby rather than a form. Exit criterion: someone holding only a link joins a
table and plays. It starts from M6.1's table document (which gains `open` before `dealing`), the
identity seam (a join link is the first place a person arrives without being named in advance) and
the bot-at-2P rule; the close's §8 says what each of those already gives it. Same session shape as
M6.1: brainstorm → spec → plan → execution under R-P11 → deploy → one clean live review → close.

**Live operations.** The deployment record and recipe are `deploy/RUNBOOK.md` (§3.5 the recipe,
§3.7 the M6.1 deployment, §5 rollback). Deploys run through the Portainer MCP against environment 2
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

## Entry artifacts by lane

- AI lane: `docs/plans/2026-09-11-ai-lane-reentry-handover.md`, then the V4 close
  `docs/plans/2026-08-30-phase3.5c-v4-close.md` §3.
- Web lane: `docs/plans/2026-09-15-web-frontend-m6.1-close.md` (M6.2's entry), then the room plan
  `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §0 and §2; what M6 inherited is
  `docs/plans/2026-09-06-web-frontend-m6-handover.md`; operations `deploy/RUNBOOK.md`.
- Bridge (BGA capture → import): `docs/bridge-setup.md`; ledger
  `docs/plans/2026-08-08-phase3.5b-execution-ledger.md`.
- Everything: `CLAUDE.md`, then `guidingpath.md`'s Current state and the decision log's tail.
