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
5. **2P dummy player (finding D1, rulebook p.9 NOTE):** model the setup-placed dummy
   assistants on church faith-7 and customs space-4 as REAL slot occupants of a sentinel
   owner (not eligibility blocks): space blocking, the 5-total trigger arithmetic (= 4 real
   pawns, unchanged game behavior), and — the previously-unmodeled rule — dummy participation
   in the church/customs MAJORITY bonuses (6/3, 8/4, rightmost tie-break) then all fall out of
   one representation. Scoring + oracle scoring + invariants + CLI display updated together;
   2P final scores change where the dummy denies/takes a place (SEMANTICS_REV 2 covers it).
6. Fuzz/census/selfplay re-baselines (game length and branching will shift — more edges ⇒ more
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
2. **Decision-surface optimizations** (review §9.14 + standing candidates): placement fee
   preview on `a`/`b` grammar echo, Employment Agency cost preview, Church faith preview
   (power + selected goods → total faith before committing).
3. **Expanded UX mandate (owner-directed 2026-07-21, superseding "graphical polish TBD"):**
   the CLI should become graphically attractive and beta-testable — colorized, well-labeled,
   ASCII-art-rich where it aids comprehension, and **grammar-optional** (the `a`/`b`/`m`/`r`
   mini-language stays as a shortcut but must never be the only usable path). Concrete
   backlog with captured evidence: **§7**. Jeremy prioritizes the §7 items at kickoff — ask
   once, early; the staggered renderer (item 1) and decision-surface items above are core
   regardless.
4. Guide pair + `guide_sync` updated for every user-visible change.

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
6. **Forge visual sign-off** (`docs/forge-cli-testing.md`): every rendering/UX task ends with
   the agent driving the CLI in a Forge PTY and reading the actual screen — staggered board
   shapes eyeballed at 2P/3P/4P, canal edge display, placement/movement decision surfaces,
   wizard flow; the PR report includes captured screens as evidence. Complements the
   automated gates (which remain the merge bar), never replaces them.

## 5. Explicitly out of scope

MCTS/search work (Phase 3 proper); the factored/lazy action interface (Phase 3's first task,
runs against the harness on the corrected topology); computer vision.

## 6. Owner gates recorded

- Tile arrangement: **CONFIRMED** (Jeremy, 2026-07-21, from the physical edition).
- Canal transit: **RULED any-to-any** (Jeremy, 2026-07-21) — no box check needed.
- 2P track trigger: **CLOSED** (p.9 NOTE — dummy occupies and counts; 4 real pawns ends a 2P
  game; engine confirmed). **NEW finding D1 rides in this plan:** the dummy player scores in
  2P majority bonuses (previously unmodeled — see §3.1 item 5).
- Card country distributions: **BOX-CONFIRMED** (orders NL5/FR8/US8/GB8/DE7; techs
  GB9/US7/NL5/FR5/DE4 — exact YAML matches, 2026-07-21).

---

## 7. Addendum (2026-07-21): Forge-driven visual verification + grounded UX backlog

**Tooling adopted:** the Forge MCP (PTY terminal server, `mcp__forge__*` tools) lets the
executing agent RUN the CLI in a real TTY and read back both the raw ANSI stream and the
rendered screen — verified end-to-end 2026-07-21 (wizard scripted in one write, game driven
move-by-move, color confirmed live). Full recipe, setup facts, and the `posix_spawnp`
chmod fix: **`docs/forge-cli-testing.md`**. Jeremy can watch any agent-driven game live at
`http://127.0.0.1:3141/` — beta-testing observation in parallel with development.

**Process rule for this PR:** every §3.3 task ends with a Forge visual check of the affected
screens (gate §4.6). Renderer work without looking at the render is how the lattice board
shipped in the first place.

**UX backlog** (evidence captured live pre-fix, listed in `docs/forge-cli-testing.md`;
Jeremy prioritizes at kickoff — items 1–2 are core):

1. **Staggered board renderer** (§3.3.1) — offset rows, widest on top, canal transit edges
   distinguishable from plain adjacency (the free-edge-governs rule needs the distinction
   anyway).
2. **Grammar-optional guided pickers** — first Placement today is 186 flat options over 10
   pages; a guided flow (pick Option A/B, then toggle areas by id off the board view, fee
   preview inline) makes the `a`/`b` grammar a shortcut instead of a necessity. Same
   treatment for assistant-move (`m`) targets; retreat already has its guided flow.
3. **Color the render layer** — today only player names + the phase line carry SGR; board
   cells, option lists, and views are monochrome code-strings (`sh ---- | th -`). Per-player
   color on assistants/presidents/buildings everywhere they appear; colored goods letters;
   `NO_COLOR`/piped-plain guarantees unchanged.
4. **Ceremony reduction** — 4 of 7 inputs in a typical opening turn are forced single-option
   no-ops (End-AA ×2, Skip 5-Power, Skip building). Candidate: bare **Enter** applies the
   sole option (still recorded honestly); explicit auto-advance is Jeremy's call.
5. **Threshold context on previews** — `Produce copper (power 2 -> +0 copper)` is faithful
   but reads like a bug; append the next threshold (`next: +1 at power 3`). Same spirit as
   the §3.3.2 fee/cost/faith previews.
6. **D1 dummy display** — `tracks` shows the 2P dummy as a bare `X`; once it scores in
   majorities (§3.1.5) it must render as a named sentinel occupant wherever majority state
   is shown.
7. **ASCII-art richness where it aids comprehension** (taste-gated by Jeremy): title banner,
   tile glyphs, legible build marks replacing the `sh ---- | th -` codes.

Honesty constraints pinned for ALL of the above: the engine stays the sole legality
authority (pickers assemble moves that must match the legal list, exactly like the
grammars); piped output stays byte-identical plain; `NO_COLOR` honored; guide pair +
`guide_sync` updated with every change; no display may claim state the engine doesn't hold.

## 7.1 Kickoff prioritization (owner-answered 2026-07-21, execution session)

Jeremy's kickoff answers, recorded at execution start:

- **§7 optional items ALL IN**: guided pickers (item 2), colorized render layer (item 3),
  threshold context on previews (item 5).
- **Ceremony reduction (item 4): bare-Enter variant** — pressing Enter on a forced
  single-option prompt applies the sole option, still recorded honestly. No silent
  auto-advance.
- **ASCII art (item 7): FULL treatment** — title banner + tile glyphs + legible build marks.
- **NEW item 8 (owner-directed at kickoff): composed gamestate frontend.** Owner's words in
  substance: a better-arranged gamestate layout — a proper ASCII CLI frontend, not the
  rolling text built for internal testing. A side window/box shows the current player's
  resources and the other players' public information; a hide/unhide flag guards private
  information (e.g. private orders) so the laptop can be passed around during a real
  tracked game; that panel/screen is toggleable on and off. Owner-stated constraint: this
  must NOT change how the CLI interfaces with the engine (engine stays the sole legality
  authority; the CLI stays a thin renderer over engine legal moves). Purpose: human
  beta-testing of live tracking before Phase 3, with Forge as the creative-design loop.
  The §7 honesty constraints apply unchanged (piped output byte-identical plain,
  `NO_COLOR` honored, guide pair + `guide_sync` per change).
