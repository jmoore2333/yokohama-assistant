# 3.5a mocks — round 2 (round-1 rulings applied)

Design artifacts for the CLI redesign (brief: `../2026-08-05-phase3.5a-cli-redesign-brief.md`).
Both frames render the owner's actual 2026-08-05 live game (seed …964555000) at turn 1,
byte-honest to the recorded setup. **Mocks, not implementation** — nothing here touches the CLI.

- `main-frame.ans` — the play frame: full-name tile boxes on the staggered board · persistent
  dashboard (players / tracks with the dummy / achievements by category / your hand as a rendered
  card) · Port A as six rendered order cards · priced lab line · decision zone (phase chain with
  the current phase lit, a "YOU ARE DECIDING" line, numbered options, standing hints). 200×48.
- `order-index.ans` — the identification surface (brief R8): all 36 orders on one screen grouped
  by flag, selection by bracketed number, engine ids as small grey annotations (never required),
  already-recorded cards dimmed with their location. Shown in its setup-recording role. 200×45.

**View in a terminal (authoritative):** `cat docs/plans/2026-08-05-phase3.5a-mocks/main-frame.ans`
in a window at least 200 columns wide. **Browser review copy:** the round-1 artifact page
(https://claude.ai/code/artifact/6d1aded6-7492-4178-9435-401f595b970f) — colors approximate ANSI.

**Regenerate / iterate:** `python3 docs/plans/2026-08-05-phase3.5a-mocks/mockgen.py` (from the repo
root). Every emitted line is asserted ≤ 200 visible columns.

**Round-1 rulings (brief §3.1):** the order index APPROVED; main-frame deltas applied in round 2.
**Round 2 adds:** `tracks-view.ans` (the `view` pattern — church + customs, every space as a card
with cost/VP/rider/occupant, dummy visible, majority bonuses stated) and `frame-4p.ans` (4-player
layout study: same 200-col floor, dashboard moves BELOW the board; illustrative tile arrangement,
site numbers are placeholders). Main frame rev 2 carries building zones per player, track reward
summaries with customs costed in IMPORTS (round 1's yen was wrong), and the visible undo/↑-history
hints. **Awaiting round-2 rulings** on: building-zone split · the view pattern · 4P posture
(dashboard-below vs wider per-count floor) · track-summary balance. Then the 3.5a spec §0.
