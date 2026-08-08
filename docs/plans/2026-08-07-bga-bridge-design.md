# Phase 3.5b — the BGA→CLI bridge: design

**Date:** 2026-08-07 · **Status:** APPROVED (owner, 2026-08-07 — "design doc looks good") **and numbered 3.5b by the same ruling** (the strength/automation lane formerly called 3.5b is now 3.5c; the close-handover §B draft is its kickoff material). The re-scoped Phase-2 live-tracking gate is claimed by this phase's acceptance run (§0.3). Implementation plan follows as its own dated doc; work runs on a bridge branch off master after PR #16 merges. · **Owner commission:** "shift our direction instead of the prep for 3.5b and build out the BGA>CLI hook … an extension in dev mode may be easier for the follow up hooks … we use that connection to test out the CLI for the final table game, then we re-run this process to prep for 3.5b when ready."

## 0. Owner rulings already made (2026-08-07, in-session)

1. **Sequencing:** PR #16 merges first, on the redesign's own completeness (the 2026-08-07 fix wave + independent review + owner acceptance). The bridge is its own branch/PR on master. **This re-scopes PR #16's merge criterion** — the Phase-2 live-tracking gate claim moves to the bridge's acceptance run. Gets a guidingpath decision-log row at the merge.
2. **Vehicle:** a Chrome extension loaded unpacked (dev mode) from day 1; the console-paste snippet is built first anyway, as the extension's own debugging harness.
3. **The gate run:** finish table **#895608640** tracked — import its full history (catch-up), then live-follow the remainder to game end. That run claims the Phase-2 gate; its record is the first bridge-produced `GameRecord`.

## 1. Foundation facts (verified live against table #895608640, 2026-08-07 — not assumptions)

- The game iframe (`/4/yokohama?table=…`) exposes `gameui.gamedatas`: a ~30 KB own-perspective snapshot — `players`, per-player `counters` (goods, yens, imports, gears, VP, flags per country, warehouse/hand/board piece counts, orders_done/orders_hand), `tiles`/`workers`/`agents` (board occupancy), `orders`/`techs` (every visible card **with its full requirement/reward vector**: `req_copper/silk/tea/fish`, `bonus_vp/yen/import/wh/move`, `location`, `player_id`, `completed`), `achievements`, `eog_triggers`, `gamestate`.
- `notificationHistory.html?table=…&from=N&privateinc=1&history=1` serves the complete structured move log. Session-token-guarded — reachable from the page's own context (the log pane consumes it), not from a bare fetch.
- **BGA's card numbering is a permutation of ours** (their 5→our O21, 10→O17, 11→O09, 21→O20, 28→O32). Numbers can never transfer. **All 36 of our order requirement+reward vectors are unique** (verified against `data.rs`), so the vector is a perfect identification key. The mechanism would have prevented both card mis-entries found in today's manual-tracking experiment (recorded O36/O24 match nothing BGA shows the owner holding).
- Visibility is own-perspective: the opponent's hand is absent from `gamedatas` — which is exactly the CLI's live-tracking hidden-information model.
- BGA has takebacks: the live table's log contains "konigsberg cancels their move". Cancelled attempts must collapse to the committed line (BGA's own replay does this via packet supersession) — **the top translation risk, and H0's first analysis task on real captured data.**

## 2. Architecture — three components

### 2.1 The extension (`bridge/extension/`, MV3, load-unpacked)
A content script in the game iframe with two jobs:
- **Snapshot:** serialize `gamedatas` on demand (toolbar action; also downloadable to a file — the paste-snippet path built in as the debug harness).
- **Follow:** pick up new notification packets while the tab is open and POST them to the local relay. v1 mechanism (marked fork, §5): poll `notificationHistory` with `from=<last seen>` on a few-second interval using the page's own session — dumb and robust — versus hooking the page's notification subscription (tighter coupling to BGA internals).

Read-only forever: the extension never submits a move to BGA, and it reads only what the owner's open, logged-in tab already has. Permissions minimal: the BGA host + `127.0.0.1`.

### 2.2 The relay (localhost listener)
One job: append every received packet to a per-table NDJSON capture file — the bridge's raw truth, auditable and replayable. CORS pinned to the extension; binds `127.0.0.1` only. **Capture works from day one, before any translation exists** — H0 banks the current table's full history immediately, so nothing is lost however fast the game finishes. (Shape fork, §5: standalone tiny binary vs a `yokohama-cli` subcommand.)

### 2.3 The importer/translator (inside `yokohama-cli` — the substantial piece)
- **Setup import:** `gamedatas` + history → the same `ObservedBoard`/starting-order/achievement captures the live wizard fills. The bridge answers the wizard's questions; it never bypasses the engine.
- **Card mapper:** orders by unique vector; techs by a checked static table (BGA `tech_id` → our `TechCardId`, built in H0's discovery pass). Ambiguity or a missing match is a hard error, never a guess.
- **Move translator:** BGA notifications → engine `Move`s / choice resolutions / reveal captures. Every translated move goes through the same checked `apply_move` boundary as typed entry. A translation the engine refuses is a **STOP with a state diff** — never a skip. Catch-up and live-follow are the same translator on two feeds (history batch; live poll).
- **Output:** a normal live-mode `GameRecord` (opponent hand hidden), autosaved, with bridge provenance noted. The map §1.5/§1.6 comparison-only fence applies unchanged.

**Honesty dividend:** at game end the engine's final scoring is cross-checked against BGA's — every bridged game is a differential test of our rules implementation against BGA's, the Phase-2.9 oracle discipline pointed at an independent external implementation.

## 3. What the bridge will NOT do

- Never submits a move, chat, or any write to BGA.
- Never touches `yokohama-core`, `yokohama-oracle`, or `yokohama-difftest` (the engine fence; `SEMANTICS_REV` unmoved).
- Never reads beyond the owner's own logged-in session and own games. Personal-use, read-only posture; not a scale tool.

## 4. Phasing (each phase a task set under the standing cadence, bridge branch off master post-#16)

| Phase | Content | Size |
|---|---|---|
| **H0 — capture first** | Extension skeleton + relay + full capture of table #895608640's notification history and `gamedatas`; notification-type inventory written from the real data; tech mapping table drafted; the cancel/supersession semantics analyzed | ~1 day |
| **H1 — setup importer + card mapper** | Vector matching, checked tech table, wizard-answer synthesis; today's O36/O24 mis-entries as test fixtures | 1–2 days |
| **H2 — catch-up translator** | Captured history → engine-legal replay to the current position; state cross-checked against BGA `counters` (goods/VP/flags per player) at checkpoints | 2–3 days |
| **H3 — live-follow + the gate run** | Poll loop → incremental apply → spectate frame; follow table #895608640 to completion; final-score cross-check; gate claim recorded (guidingpath rows: the re-scope ruling + the run) | 1–2 days |

## 5. Open implementation forks (marked for the plan, owner may veto any)

1. **Follow mechanism:** poll `notificationHistory` (v1 recommendation) vs hook the page's subscription.
2. **Relay shape:** standalone binary vs `yokohama-cli bridge --listen` subcommand.
3. **Repo placement:** in-repo `bridge/` (recommended — one review cadence, guide documents it) vs separate repo.
4. **Spectate rendering during follow:** reuse the live frame as a read-only ticker vs minimal line output in v1.

## 6. Risks, named

- **Cancel/takeback semantics** (top risk) — analyzed in H0 on captured data before H2 commits to a translation.
- **Private-info completeness** — verify `privateinc=1` carries the owner's own hand events; the opponent's stay hidden (H0 check).
- **The table finishing early** — H0's capture-first ordering protects the history; live-follow needs remaining moves to exist (table at 37% progression, async timers are long).
- **BGA markup/schema drift** — the bridge pins itself to the data shapes captured in H0 and hard-errors on surprises (never guesses); a drift is a maintenance task, not silent corruption.

## 7. Relationship to the roadmap

3.5b prep (the close-handover §B draft) stays parked; "we re-run this process to prep for 3.5b when ready" (owner, 2026-08-07). The bridge additionally feeds 3.5b when it resumes: bridged records are comparison-corpus material (agreement rates, blunder finding, scenario harvesting — never tuning inputs), and the replay viewer item gains real human-game records to view.
