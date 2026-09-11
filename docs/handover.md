# Handover — the owner's task board

**What this is.** The one screen to open before starting a session: the tracks in flight, the
next concrete step on each, and the rulings they are waiting on. It is a POINTER — every fact
here names the document that carries it, and it never outranks `guidingpath.md` (the authority),
`CLAUDE.md` (the operating facts), or a dated document in `docs/plans/`. Keep it current at every
session close: rewrite the stale lines, do not append history (the decision log is for history).

**Updated:** 2026-09-11, at the session that merged the web lane (`abe4eec`) and wrote the AI-lane
re-entry handover `docs/plans/2026-09-11-ai-lane-reentry-handover.md`.

## Where things stand

- **The bot:** V4, the sighted evaluator `v4-step9-draft1`, shipped 2026-08-30 as the production
  contestant and released as v0.5.0. The bar ("provably plays to win, two-player") is NOT claimed.
- **The two owner decisions** that close V4 — the D3 depth re-ask and the W6 knob-screening
  GO/WAIT — both HOLD until your `why`-scribed diagnostic game is in hand.
- **The web client** merged to master 2026-09-11 and is live at `yokohama.moore.nyc` behind
  Cloudflare Access, `yokohama-web:0.5.0-m5.5.4` on the NAS via Portainer. Nothing about it moves
  the bot. Its next milestone (M6, "the room") is unplanned.
- **Licensing:** Apache-2.0 on every crate and on `web/package.json` (checked 2026-09-11);
  `LICENSE` + `NOTICE` at the root; `ASSETS.md` carves out the game's own material.
- **Gates at HEAD:** all per-edit suites green (the counts are in `CLAUDE.md`); the fifteen-gate
  release roster last ran at the V4 merge head, 15/15 at 492.17s on the M5.

## Track 1 — V4 close-out: the diagnostic games and the two rulings (AI lane)

**Goal:** play the owed game(s) against the shipped bot, scribe every bot move with `why`, package
the findings, then rule D3 and W6 together. Entry: the re-entry handover §2–§5.

1. Check Forge is up (`http://127.0.0.1:3141/`) — the recipe is `docs/forge-cli-testing.md`,
   200×50 floor. If it fails to connect in a session, `/mcp` reconnect or a fresh session.
2. Saves on disk (gitignored `docs/temp/owner-games/`): `game4-v4.json` is COMPLETE (247 events;
   you won 82–81 per the 2026-08-30 session's record) and UNSCRIBED; `game5-v4.json` is at 40
   events. The games are played on master, whose bot is byte-identical to the V4 merge head's.
3. The shape you chose (2026-09-11): **the CLI game live in Forge** — the agent scribes by
   reconstruct-by-replay on autosave copies (`why <n>` at every bot event) and never advises,
   your comments anchored to event indices as you go; finish game 5 or start game 6. **The web
   game solo from your other desktop** — report the game id, it gets pulled with the Track 3
   recipe and scribed post hoc against your notes file (`docs/temp/owner-games/notes-<date>.md`,
   one line per flagged event). Game over is not a `why` refusal, so nothing is lost either way.
4. Package: a dated findings document on the Phase-B shape
   (`docs/plans/2026-08-17-phase-b-owner-games-handover.md` — event-anchored findings, the
   outcomes, what routes where), plus decision-log rows. Game 1's 82–81 is not in any tracked
   document until this lands.
5. Rule, together, with the scribe log in hand: **D3** (open the bounded cross-handoff diagnostic
   inside S1 under its six-clause fence, or leave the lane open-with-evidence) and **W6** (GO opens
   a new development identity via knob screening on b and m; WAIT keeps the shelf).
6. Then the ladder as the amendment wrote it: screening → the D1 identity freeze → W7 (the @V1
   instrument is blocking implementation work first, then the OD-5 pre-registration you sign, then
   ONE confirmatory battery) → OD-8 acceptance play → OD-11, the bar.

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

## Track 3 — The web client: operations now, M6 later (web lane)

**Live operations.** The deployment record and recipe are `deploy/RUNBOOK.md` (§3.5 the recipe,
§3.6 every deployment, §5 rollback). Deploys run through the Portainer MCP against environment 2
(`http://192.168.1.173:9000`); after every deploy, look at BOTH layouts on screen.

**Pulling a web game as a save file** — verified 2026-09-11:

- The server persists every game as the SAME `GameRecord` JSON the CLI's `save` writes (one
  function, `Session::to_record_json`), at `/data/games/<id>.json` inside the container
  (`YOKOHAMA_WEB_DIR=/data/games`, the named volume `yokohama-web-data`). No API route serves the
  raw record, and no route ever will carry the setup seed by default — so the pull is a Docker
  archive read through Portainer:

  ```bash
  # the token the Portainer MCP uses (never paste it anywhere): env, the portainer-mcp
  # checkout's .dev.vars, or Keychain — see ~/.local/bin/portainer-mcp-write
  curl -s -H "X-API-Key: $PORTAINER_MCP_TOKEN" \
    "http://192.168.1.173:9000/api/endpoints/2/docker/containers/yokohama-web/archive?path=/data/games/<id>.json" \
    | tar -xO > docs/temp/owner-games/web-<yyyy-mm-dd>-<id>.json
  ```

  Game ids come from `curl https://yokohama.moore.nyc/api/games` (WARP passes Access from the
  shell). The pulled file resumes in the CLI (`cargo run -p yokohama-cli -- <file>`) and `why`
  works on every bot move — checked on `sim-016f847457c957a6` (231 events; you beat the bot
  100–56 in that one).
- **Naming:** the browser cannot name a game (ids are random `sim-<16 hex>`), so name the PULLED
  copy `web-<date>-<id>.json` and let the findings document carry the purpose; CLI games keep
  `gameN-v4.json`. Everything under `docs/temp/owner-games/` is gitignored on purpose — a record
  carries the setup seed, which is the whole hidden set.
- **M6 ("the room")** — lobby, chat, switching, then Cloudflare Workers + Modal hosting — waits for a
  feature branch; its entry is `docs/plans/2026-09-06-web-frontend-m6-handover.md` (§3 the charter,
  §4 the LEAVE list, §5 the process you want).

## Owner rulings outstanding (one line each; detail in the re-entry handover §5)

1. D3 depth re-ask — HOLD (Track 1).
2. W6 GO/WAIT — HOLD (Track 1).
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
- Web lane: `docs/plans/2026-09-06-web-frontend-m6-handover.md`; operations `deploy/RUNBOOK.md`.
- Bridge (BGA capture → import): `docs/bridge-setup.md`; ledger
  `docs/plans/2026-08-08-phase3.5b-execution-ledger.md`.
- Everything: `CLAUDE.md`, then `guidingpath.md`'s Current state and the decision log's tail.
