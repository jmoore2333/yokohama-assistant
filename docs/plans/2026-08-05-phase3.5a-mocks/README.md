# 3.5a mock round 1 — two candidate surfaces at the raised floor (200×50)

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

**Status: round 1, awaiting the owner's rulings** on: the floor size · tile-box content · dashboard
composition · the mini-card rendering and by-flag grouping · the decision zone. Rulings land in the
3.5a spec §0; implementation starts only after mock sign-off.
