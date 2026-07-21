# Board topology correction + CLI graphical optimizations — mini-plan

**Status:** QUEUED — the next PR after PR #3 merges, and the last engineering work before the
Phase-3 (MCTS/computer player) spec. Owner-directed bundling (Jeremy, 2026-07-21): topology fix
and CLI graphical work land together.
**Origin:** 11-lens review of PR #3 (Blocker 1), independently confirmed by the orchestrator
against rulebook p.3 and by Jeremy from the physical edition ("this tile arrangement is the
only way the game works").

---

## 1. The defect

`board_gen.rs` builds **rectangular grids with orthogonal-only adjacency** (2P 2×5, 3P 2×7,
4P 3×6). Rulebook p.3 step 2 prints **staggered pyramids**:

| Players | Printed rows (Jeremy's edition, top→bottom) | Tiles | Physical edges (review) | Engine today |
|---|---|---|---|---|
| 2 | 4 / 3 / 2 / 1 | 10 ✓ | 18 | 2×5, 13 edges |
| 3 | 5 / 4 / 3 / 2 | 14 ✓ | 28 | 2×7, 19 edges |
| 4 | 6 / 5 / 4 / 3 | 18 ✓ | 38 | 3×6, 27 edges |

Rows are brick-offset: a tile touches up to **six** neighbors (2 lateral + 2 in the row above
+ 2 in the row below). The engine both misses ~30% of real edges and invents vertical edges
between tiles that only touch at corners. Tile counts match both models — which is why every
count-based check passed — and the graph is shared *input* to engine and oracle, so the entire
differential campaign was structurally blind to it (the documented method boundary, final
report §7.1). Orientation note: the PDF's diagram and Jeremy's edition print mirror-image
orientations of the same pyramid; the adjacency **graph** is identical, and the graph is what
the engine models. Rendering follows Jeremy's edition (widest row on top).

**Blast radius:** movement legality and president routes, pass-through fees, Telegram adjacent
placement, canal crossing derivation, live-board reconciliation, CLI board rendering and the
`sites` adjacency lines, and the difftest worst-case battery boards.

## 2. Primary sources (pinned)

- **Rulebook p.3, step 2** — the three layout diagrams (verified by orchestrator 2026-07-21;
  confirmed physically by Jeremy same day).
- **Rulebook p.3, step 1 — the removal table** (re-verify `board_gen`'s per-count lists in this
  plan): 2P returns Laboratory B, Port B, Chinatown, **Canal**, Copper Mine ×1, Silk Mill ×1,
  Tea Plantation ×1, Fishing Port ×1; 3P returns Laboratory B, Copper Mine ×1, Silk Mill ×1,
  Tea Plantation ×1; 4P returns nothing. **Note: the 2-player board has NO Canal** — verify the
  engine agrees; if it does not, that is a second fidelity defect fixed here.
- **Canal transit — RULED 2026-07-21 (supersedes the re-open):** ANY-TO-ANY. The Canal
  connects any of its adjacent tiles to any other for the 1-Yen bank toll (Jeremy, on the
  staggered-neighborhood diagram: "you can go from any of the tiles you've indicated to any of
  the other ones"), including neighbor pairs that are also directly adjacent (legal, dominated,
  self-resolving — owner-noted). The colinear-pairs model and the bent-transit prohibition are
  superseded; derivation becomes all-neighbor-pairs, which is simpler than colinearity
  geometry. The Canal remains unplaceable and unstoppable.

## 3. Scope of work

### 3.1 Engine (`yokohama-core`) — **SEMANTICS_REV bumps to 2** (adjacency changes movement
legality — the bump contract applies; old records become strict-rejected, correctly)

1. `board_gen.rs`: staggered-row layout model (per-count row lengths above), adjacency derived
   from brick-offset contact (lateral + two upper + two lower where tiles exist), removal lists
   re-verified against the p.3 table, canal-crossing derivation re-done for the staggered
   geometry (colinear through-lines: the lateral pair + the two diagonal pairs, each only where
   both flank tiles exist — no vertical pair can exist).
2. `types/board.rs`: whatever row-structure metadata rendering needs (row lengths/offsets);
   `BoardLayout.adjacency` remains the single graph consumers read.
3. `reconcile_setup`: tile re-typing over the fixed **staggered** cells; the multiset tripwire
   unchanged.
4. Invariants: structural checks re-verified on the new generator (symmetry, crossing⊆adjacency
   hold by construction); add a **layout-fidelity pin**: per player count, assert the generated
   row shape and the exact undirected edge count (18/28/38) so the topology can never silently
   regress to a lattice.
5. Fuzz/census/selfplay re-baselines (game length and branching will shift — more edges ⇒ more
   routes; document new numbers with provenance per the I6 convention).

### 3.2 Oracle + difftest

- The oracle consumes `state.board_layout` by construction (spec §4) — **no oracle logic
  change**; its test-support boards and the difftest worst-case battery are rebuilt on staggered
  geometry (canal cells re-derived to the any-to-any neighbor-pair model).
- **The whole differential suite re-runs on the corrected graph** — this is what makes the fix
  safe: enumeration, routes3 three-way (pruneless reference re-validates pruning on the denser
  graph), scoring, plus the per-edit smoke. Convergence on the new topology is the acceptance
  gate.

### 3.3 CLI (the bundled graphical work)

1. **Staggered board rendering** — replace the rectangular box-grid with offset rows matching
   Jeremy's edition (widest row on top); keep TTY color/`NO_COLOR`/piped-plain guarantees and
   the width-warning behavior; `sites`/adjacency lines re-derived from the corrected graph.
2. **Decision-surface optimizations** (review §9.14 + standing candidates, Jeremy prioritizes
   at execution): placement fee preview on `a`/`b` grammar echo, Employment Agency cost preview,
   Church faith preview (power + selected goods → total faith before committing), and any
   further graphical polish Jeremy names at kickoff.
3. Guide pair + `guide_sync` updated for every user-visible change.

### 3.4 Docs

- Decision-log rows: topology adjudication (owner-confirmed), canal re-derivation outcome
  (owner-confirmed at the box), removal-table verification, SEMANTICS_REV 2 rationale.
- Final-report addendum: Blocker 1 resolution recorded against the method-boundary framing.
- KB: board-layout section corrected; back-port of load-bearing DL rulings into the KB (review
  §5 recommendation) rides here.

## 4. Verification plan (acceptance gates)

1. Layout-fidelity pins green (rows + edge counts per player count).
2. Full debug matrix + **all five release gates green on the corrected topology** — the
   differential convergence is the point of having built the oracle.
3. Live-reconciliation suite green on staggered boards.
4. Owner confirmations: canal transit RULED any-to-any (done, 2026-07-21); the 2P track
   trigger 4-vs-5 answer (pending — a `track_trigger` question, independent of topology).
5. Re-baselines published: moves/sec, game length, branching per phase, organic route-label
   max — all provenance-stamped.

## 5. Explicitly out of scope

MCTS/search work (Phase 3 proper); the factored/lazy action interface (Phase 3's first task,
runs against the harness on the corrected topology); computer vision.

## 6. Owner gates recorded

- Tile arrangement: **CONFIRMED** (Jeremy, 2026-07-21, from the physical edition).
- Canal transit: **RULED any-to-any** (Jeremy, 2026-07-21) — no box check needed.
- 2P track trigger (4 vs 5 real pawns): **pending Jeremy's distinguishing answer** (the
  dummy-is-a-blocker clarification re-opened it; engine stays at 4 until answered).
- Card country distributions: **BOX-CONFIRMED** (orders NL5/FR8/US8/GB8/DE7; techs
  GB9/US7/NL5/FR5/DE4 — exact YAML matches, 2026-07-21).
